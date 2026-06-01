import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
	createEventBus,
	createExtensionRuntime,
	DefaultPackageManager,
	DefaultResourceLoader,
	SettingsManager,
	type AgentSessionRuntimeDiagnostic,
	type AgentSessionServices,
	type AuthStorage,
	type ExtensionFactory,
	type LoadExtensionsResult,
	type ModelRegistry,
	type ResourceLoader,
} from "@earendil-works/pi-coding-agent";

type ResourceExtensionPaths = Parameters<ResourceLoader["extendResources"]>[0];
type CachedJiti = {
	import: (id: string, options: { default: true }) => Promise<unknown>;
};
type CreateJiti = (
	filename: string,
	options: { moduleCache: boolean; alias: Record<string, string> },
) => CachedJiti;
type LoadExtensionFromFactory = (
	factory: ExtensionFactory,
	cwd: string,
	eventBus: unknown,
	runtime: LoadExtensionsResult["runtime"],
	extensionPath: string,
) => Promise<LoadExtensionsResult["extensions"][number]>;

type RuntimeResourceOptions = {
	cwd: string;
	agentDir: string;
	authStorage: AuthStorage;
	modelRegistry: ModelRegistry;
	enableExtensions: boolean;
};

type ProviderRegistration = {
	name: string;
	config: Parameters<ModelRegistry["registerProvider"]>[1];
	extensionPath: string;
};

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

function cacheKey(options: RuntimeResourceOptions): string {
	return [
		options.cwd,
		options.agentDir,
		options.enableExtensions ? "extensions" : "no-extensions",
	].join("\0");
}

export class RuntimeResourceCache {
	private readonly factoryCache: ExtensionFactoryCache;
	private readonly entries = new Map<string, RuntimeResourceEntry>();

	constructor(resolvePiAgentRoot: () => Promise<string>) {
		this.factoryCache = new ExtensionFactoryCache(resolvePiAgentRoot);
	}

	async createServices(
		options: RuntimeResourceOptions,
	): Promise<AgentSessionServices> {
		const key = cacheKey(options);
		let entry = this.entries.get(key);
		if (!entry) {
			entry = new RuntimeResourceEntry(
				options.cwd,
				options.agentDir,
				options.enableExtensions,
				this.factoryCache,
			);
			this.entries.set(key, entry);
		}
		return entry.createServices(options.authStorage, options.modelRegistry);
	}

	clear(): void {
		this.entries.clear();
		this.factoryCache.clear();
	}
}

class RuntimeResourceEntry {
	private readonly settingsManager: SettingsManager;
	private readonly eventBus = createEventBus();
	private readonly packageManager: DefaultPackageManager;

	constructor(
		private readonly cwd: string,
		private readonly agentDir: string,
		private readonly enableExtensions: boolean,
		private readonly factoryCache: ExtensionFactoryCache,
	) {
		this.settingsManager = SettingsManager.create(cwd, agentDir);
		this.packageManager = new DefaultPackageManager({
			cwd,
			agentDir,
			settingsManager: this.settingsManager,
		});
	}

	async createServices(
		authStorage: AuthStorage,
		modelRegistry: ModelRegistry,
	): Promise<AgentSessionServices> {
		const baseLoader = new DefaultResourceLoader({
			cwd: this.cwd,
			agentDir: this.agentDir,
			settingsManager: this.settingsManager,
			eventBus: this.eventBus,
			noExtensions: true,
		});
		await baseLoader.reload();
		const extensionsResult = await this.loadExtensions(false);
		const resourceLoader = new CachedResourceLoader(
			baseLoader,
			extensionsResult,
			(force) => this.loadExtensions(force),
		);
		const diagnostics = this.collectDiagnostics(
			extensionsResult,
			modelRegistry,
		);

		return {
			cwd: this.cwd,
			agentDir: this.agentDir,
			authStorage,
			settingsManager: this.settingsManager,
			modelRegistry,
			resourceLoader,
			diagnostics,
		};
	}

	private async loadExtensions(
		forceModuleReload: boolean,
	): Promise<LoadExtensionsResult> {
		if (!this.enableExtensions) {
			return {
				extensions: [],
				errors: [],
				runtime: createExtensionRuntime(),
			};
		}

		const resolved = await this.packageManager.resolve();
		const extensionPaths = resolved.extensions
			.filter((resource) => resource.enabled)
			.map((resource) => resource.path);
		return this.factoryCache.load(
			extensionPaths,
			this.cwd,
			this.eventBus,
			forceModuleReload,
		);
	}

	private collectDiagnostics(
		extensionsResult: LoadExtensionsResult,
		modelRegistry: ModelRegistry,
	): AgentSessionRuntimeDiagnostic[] {
		const diagnostics: AgentSessionRuntimeDiagnostic[] =
			extensionsResult.errors.map((error) => ({
				type: "error",
				message: `Extension "${error.path}" failed to load: ${error.error}`,
			}));
		const runtime =
			extensionsResult.runtime as typeof extensionsResult.runtime & {
				pendingProviderRegistrations?: ProviderRegistration[];
			};
		for (const registration of runtime.pendingProviderRegistrations ?? []) {
			try {
				modelRegistry.registerProvider(registration.name, registration.config);
			} catch (error) {
				diagnostics.push({
					type: "error",
					message: `Extension "${registration.extensionPath}" error: ${errorMessage(error)}`,
				});
			}
		}
		runtime.pendingProviderRegistrations = [];
		return diagnostics;
	}
}

class CachedResourceLoader implements ResourceLoader {
	constructor(
		private readonly base: DefaultResourceLoader,
		private extensionsResult: LoadExtensionsResult,
		private readonly reloadExtensions: (
			forceModuleReload: boolean,
		) => Promise<LoadExtensionsResult>,
	) {}

	getExtensions(): LoadExtensionsResult {
		return this.extensionsResult;
	}

	getSkills(): ReturnType<ResourceLoader["getSkills"]> {
		return this.base.getSkills();
	}

	getPrompts(): ReturnType<ResourceLoader["getPrompts"]> {
		return this.base.getPrompts();
	}

	getThemes(): ReturnType<ResourceLoader["getThemes"]> {
		return this.base.getThemes();
	}

	getAgentsFiles(): ReturnType<ResourceLoader["getAgentsFiles"]> {
		return this.base.getAgentsFiles();
	}

	getSystemPrompt(): string | undefined {
		return this.base.getSystemPrompt();
	}

	getAppendSystemPrompt(): string[] {
		return this.base.getAppendSystemPrompt();
	}

	extendResources(paths: ResourceExtensionPaths): void {
		this.base.extendResources(paths);
	}

	async reload(): Promise<void> {
		await this.base.reload();
		this.extensionsResult = await this.reloadExtensions(true);
	}
}

class ExtensionFactoryCache {
	private readonly factories = new Map<string, ExtensionFactory>();
	private jiti: CachedJiti | undefined;
	private loadExtensionFromFactory: LoadExtensionFromFactory | undefined;

	constructor(private readonly resolvePiAgentRoot: () => Promise<string>) {}

	async load(
		extensionPaths: string[],
		cwd: string,
		eventBus: unknown,
		forceModuleReload: boolean,
	): Promise<LoadExtensionsResult> {
		if (forceModuleReload) {
			this.clear();
		}

		const runtime = createExtensionRuntime();
		const extensions: LoadExtensionsResult["extensions"] = [];
		const errors: LoadExtensionsResult["errors"] = [];
		const loadExtensionFromFactory = await this.getLoadExtensionFromFactory();
		for (const extensionPath of extensionPaths) {
			try {
				const factory = await this.loadFactory(extensionPath);
				extensions.push(
					await loadExtensionFromFactory(
						factory,
						cwd,
						eventBus,
						runtime,
						extensionPath,
					),
				);
			} catch (error) {
				errors.push({
					path: extensionPath,
					error: `Failed to load extension: ${errorMessage(error)}`,
				});
			}
		}
		return { extensions, errors, runtime };
	}

	clear(): void {
		this.factories.clear();
		this.jiti = undefined;
	}

	private async loadFactory(extensionPath: string): Promise<ExtensionFactory> {
		const cached = this.factories.get(extensionPath);
		if (cached) return cached;
		const jiti = await this.getJiti();
		const module = await jiti.import(extensionPath, { default: true });
		if (typeof module !== "function") {
			throw new Error(
				`Extension does not export a valid factory function: ${extensionPath}`,
			);
		}
		const factory = module as ExtensionFactory;
		this.factories.set(extensionPath, factory);
		return factory;
	}

	private async getJiti(): Promise<CachedJiti> {
		if (this.jiti) return this.jiti;
		const root = await this.resolvePiAgentRoot();
		const requireFromPiAgent = createRequire(
			pathToFileURL(join(root, "dist/core/extensions/loader.js")),
		);
		const jitiModule = (await import(
			pathToFileURL(requireFromPiAgent.resolve("jiti")).href
		)) as {
			createJiti?: CreateJiti;
			default?: CreateJiti | { createJiti?: CreateJiti };
		};
		const createJiti =
			jitiModule.createJiti ??
			(typeof jitiModule.default === "function"
				? jitiModule.default
				: jitiModule.default?.createJiti);
		if (!createJiti) {
			throw new Error("Unable to load jiti for cached Pi extension loading");
		}
		this.jiti = createJiti(import.meta.url, {
			moduleCache: true,
			alias: await this.extensionAliases(root),
		});
		return this.jiti;
	}

	private async getLoadExtensionFromFactory(): Promise<LoadExtensionFromFactory> {
		if (this.loadExtensionFromFactory) return this.loadExtensionFromFactory;
		const root = await this.resolvePiAgentRoot();
		const module = (await import(
			pathToFileURL(join(root, "dist/core/extensions/loader.js")).href
		)) as { loadExtensionFromFactory?: LoadExtensionFromFactory };
		if (!module.loadExtensionFromFactory) {
			throw new Error("Pi extension factory loader is unavailable");
		}
		this.loadExtensionFromFactory = module.loadExtensionFromFactory;
		return this.loadExtensionFromFactory;
	}

	private async extensionAliases(
		root: string,
	): Promise<Record<string, string>> {
		const piAgentCore = this.resolveNestedOrPackage(
			root,
			"node_modules/@earendil-works/pi-agent-core/dist/index.js",
			"@earendil-works/pi-agent-core",
		);
		const piTui = this.resolveNestedOrPackage(
			root,
			"node_modules/@earendil-works/pi-tui/dist/index.js",
			"@earendil-works/pi-tui",
		);
		const piAi = this.resolveNestedOrPackage(
			root,
			"node_modules/@earendil-works/pi-ai/dist/index.js",
			"@earendil-works/pi-ai",
		);
		const piAiOauth = this.resolveNestedOrPackage(
			root,
			"node_modules/@earendil-works/pi-ai/dist/oauth.js",
			"@earendil-works/pi-ai/oauth",
		);
		const typebox = this.resolveNestedOrPackage(
			root,
			"node_modules/typebox/build/index.mjs",
			"typebox",
		);
		const typeboxCompile = this.resolveNestedOrPackage(
			root,
			"node_modules/typebox/build/compile/index.mjs",
			"typebox/compile",
		);
		const typeboxValue = this.resolveNestedOrPackage(
			root,
			"node_modules/typebox/build/value/index.mjs",
			"typebox/value",
		);
		const piCodingAgent = join(root, "dist/index.js");

		return {
			"@earendil-works/pi-coding-agent": piCodingAgent,
			"@mariozechner/pi-coding-agent": piCodingAgent,
			"@earendil-works/pi-agent-core": piAgentCore,
			"@mariozechner/pi-agent-core": piAgentCore,
			"@earendil-works/pi-tui": piTui,
			"@mariozechner/pi-tui": piTui,
			"@earendil-works/pi-ai": piAi,
			"@mariozechner/pi-ai": piAi,
			"@earendil-works/pi-ai/oauth": piAiOauth,
			"@mariozechner/pi-ai/oauth": piAiOauth,
			typebox,
			"typebox/compile": typeboxCompile,
			"typebox/value": typeboxValue,
			"@sinclair/typebox": typebox,
			"@sinclair/typebox/compile": typeboxCompile,
			"@sinclair/typebox/value": typeboxValue,
		};
	}

	private resolveNestedOrPackage(
		root: string,
		relativePath: string,
		specifier: string,
	): string {
		const nested = join(root, relativePath);
		if (existsSync(nested)) return nested;
		const requireFromPiAgent = createRequire(
			pathToFileURL(join(root, "dist/index.js")),
		);
		return requireFromPiAgent.resolve(specifier);
	}
}

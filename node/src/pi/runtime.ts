import { join } from "node:path";
import type {
	AuthStorage,
	ModelRegistry,
	SessionManager,
	AgentSessionRuntime,
	CreateAgentSessionRuntimeFactory,
} from "@earendil-works/pi-coding-agent";
import type { FauxProviderRegistration, Model } from "@earendil-works/pi-ai";
import type {
	AuthFlowUpdate,
	BridgeCommand,
	BridgeResponse,
	CoreStateSnapshot,
	InitCommand,
	ModelDescriptor,
	ModelSelection,
	OAuthLoginMethod,
	PackageSearchResponse,
	ProviderAuthStatus,
	ThinkingLevel,
	ToolDescriptor,
} from "../generated/protocol.js";
import { BridgeCommandError } from "../bridge/errors.js";
import { eventEnvelope } from "../bridge/envelope.js";
import { emitEvent } from "../bridge/native.js";
import { NativeExtensionUi } from "./extension_ui.js";
import {
	installPackageSource,
	listInstalledPackages,
	removePackageSource,
} from "./package_manager.js";
import {
	DEFAULT_PACKAGE_LIMIT,
	searchPackageRegistry,
} from "./package_registry.js";
import { RuntimeResourceCache } from "./resource_cache.js";
import {
	loadPiAgentRuntime,
	loadPiAiRuntime,
	piAgentRoot,
} from "./runtime_loader.js";
import {
	asJson,
	mapImages,
	openExternalUrl,
	queueMode,
} from "./runtime_helpers.js";

type PiAgentRuntime = typeof import("@earendil-works/pi-coding-agent");

type RuntimeServices = {
	runtime: AgentSessionRuntime;
	ui: NativeExtensionUi;
	unsubscribe?: () => void;
	sessionKeys?: Set<string>;
	pendingMessageUpdate?: unknown;
	pendingTextDelta?: string | undefined;
	messageUpdateTimer?: ReturnType<typeof setTimeout> | undefined;
};

const MESSAGE_UPDATE_FLUSH_MS = 50;

function errorMessage(error: unknown): string {
	if (error instanceof Error) return error.message;
	return String(error);
}

export class PiRuntimeBackend {
	private services: RuntimeServices | undefined;
	private sessionRuntimes = new Map<string, RuntimeServices>();
	private createRuntimeFactory: CreateAgentSessionRuntimeFactory | undefined;
	private runtimeCwd: string | undefined;
	private runtimeAgentDir: string | undefined;
	private authStorage: AuthStorage | undefined;
	private modelRegistry: ModelRegistry | undefined;
	private authAgentDir: string | undefined;
	private authTestMode = false;
	private faux: FauxProviderRegistration | undefined;
	private selectedModel: Model<string> | undefined;
	private enableExtensions = true;
	private resourceCache: RuntimeResourceCache | undefined;

	async dispatch(command: BridgeCommand): Promise<BridgeResponse> {
		switch (command.type) {
			case "init":
				return this.init(command.payload);
			case "shutdown":
				await this.dispose();
				emitEvent(eventEnvelope({ type: "shutdown" }));
				return { type: "ack" };
			case "reload":
				await this.requireRuntime().runtime.session.reload();
				return { type: "ack" };
			case "prompt": {
				const target = await this.runtimeForSessionPath(
					command.payload.sessionPath ?? undefined,
				);
				void target.runtime.session
					.prompt(command.payload.text, {
						images: mapImages(command.payload.images),
						...(command.payload.streamingBehavior
							? { streamingBehavior: command.payload.streamingBehavior }
							: {}),
					})
					.catch((error: unknown) => {
						this.cancelPendingMessageUpdate(target);
						this.emitSessionRunError(target, errorMessage(error));
					});
				return { type: "ack" };
			}
			case "steer":
				await this.requireRuntime().runtime.session.steer(
					command.payload.text,
					mapImages(command.payload.images),
				);
				return { type: "ack" };
			case "followUp":
				await this.requireRuntime().runtime.session.followUp(
					command.payload.text,
					mapImages(command.payload.images),
				);
				return { type: "ack" };
			case "sendUserMessage":
				await this.requireRuntime().runtime.session.sendUserMessage(
					command.payload.text,
					{ deliverAs: "followUp" },
				);
				return { type: "ack" };
			case "sendCustomMessage":
				await this.requireRuntime().runtime.session.sendCustomMessage(
					{
						customType: command.payload.customType,
						content: command.payload.content as string,
						display: command.payload.display,
						details: command.payload.details,
					},
					{
						triggerTurn: command.payload.triggerTurn,
						...(command.payload.deliverAs
							? { deliverAs: command.payload.deliverAs }
							: {}),
					},
				);
				return { type: "ack" };
			case "abort":
				await this.requireRuntime().runtime.session.abort();
				return { type: "ack" };
			case "clearQueue":
				return {
					type: "json",
					payload: {
						value: this.requireRuntime().runtime.session.clearQueue(),
					},
				};
			case "getState":
				return { type: "state", payload: { state: this.snapshot() } };
			case "getSessionState": {
				const target = await this.runtimeForSessionPath(
					command.payload.sessionPath,
				);
				return { type: "state", payload: { state: this.snapshotFor(target) } };
			}
			case "getMessages":
				return {
					type: "messages",
					payload: {
						messages:
							this.requireRuntime().runtime.session.messages.map(asJson),
					},
				};
			case "getSessionStats":
				return {
					type: "sessionStats",
					payload: {
						stats: asJson(
							this.requireRuntime().runtime.session.getSessionStats(),
						),
					},
				};
			case "getAuthStatus":
				await this.ensureAuthServices();
				return {
					type: "authStatus",
					payload: { statuses: this.authStatuses(command.payload.provider) },
				};
			case "setApiKey":
				await this.ensureAuthServices();
				this.setApiKey(
					command.payload.provider,
					command.payload.apiKey,
					command.payload.persist,
				);
				return { type: "ack" };
			case "oauthLogin":
				await this.ensureAuthServices();
				await this.oauthLogin(command.payload.provider, command.payload.method);
				return { type: "ack" };
			case "removeAuth":
				await this.ensureAuthServices();
				this.removeAuth(command.payload.provider);
				return { type: "ack" };
			case "searchPackages":
				return {
					type: "json",
					payload: {
						value: asJson(
							await this.searchPackages(
								command.payload.query,
								command.payload.limit,
							),
						),
					},
				};
			case "listPackages":
				return {
					type: "json",
					payload: {
						value: asJson(await this.listPackages(command.payload.cwd)),
					},
				};
			case "installPackage":
				await this.installPackage(
					command.payload.source,
					command.payload.project,
					command.payload.cwd,
				);
				return {
					type: "json",
					payload: {
						value: asJson(await this.listPackages(command.payload.cwd)),
					},
				};
			case "removePackage":
				await this.removePackage(
					command.payload.source,
					command.payload.project,
					command.payload.cwd,
				);
				return {
					type: "json",
					payload: {
						value: asJson(await this.listPackages(command.payload.cwd)),
					},
				};
			default:
				return this.dispatchSessionOrUi(command);
		}
	}

	snapshot(): CoreStateSnapshot {
		if (!this.services) {
			return this.emptySnapshot();
		}
		return this.snapshotFor(this.services);
	}

	private snapshotFor(services: RuntimeServices): CoreStateSnapshot {
		const session = services.runtime.session;
		return {
			initialized: true,
			cwd: services.runtime.cwd,
			sessionId: session.sessionId,
			sessionFile: session.sessionFile ?? null,
			sessionName: session.sessionName ?? null,
			isStreaming: session.isStreaming,
			isCompacting: session.isCompacting,
			isRetrying: session.isRetrying,
			isBashRunning: session.isBashRunning,
			model: session.model ? this.modelDescriptor(session.model) : null,
			thinkingLevel: session.thinkingLevel as ThinkingLevel,
			activeTools: session.getActiveToolNames(),
			queue: {
				steering: [...session.getSteeringMessages()],
				followUp: [...session.getFollowUpMessages()],
			},
			messages: session.messages.map(asJson),
			diagnostics: services.runtime.diagnostics.map((diagnostic) => ({
				level: diagnostic.type,
				message: diagnostic.message,
			})),
		};
	}

	private async dispatchSessionOrUi(
		command: BridgeCommand,
	): Promise<BridgeResponse> {
		const current = this.requireRuntime();
		const runtime = current.runtime;
		switch (command.type) {
			case "newSession":
				return {
					type: "cancelled",
					payload: await runtime.newSession({
						...(command.payload.parentSession
							? { parentSession: command.payload.parentSession }
							: {}),
					}),
				};
			case "switchSession":
				return {
					type: "cancelled",
					payload: await runtime.switchSession(command.payload.sessionPath, {
						...(command.payload.cwdOverride
							? { cwdOverride: command.payload.cwdOverride }
							: {}),
					}),
				};
			case "fork":
				return {
					type: "json",
					payload: {
						value: asJson(
							await runtime.fork(command.payload.entryId, {
								position: command.payload.position,
							}),
						),
					},
				};
			case "navigateTree":
				return {
					type: "json",
					payload: {
						value: asJson(
							await runtime.session.navigateTree(command.payload.targetId, {
								summarize: command.payload.summarize,
								replaceInstructions: command.payload.replaceInstructions,
								...(command.payload.customInstructions
									? { customInstructions: command.payload.customInstructions }
									: {}),
								...(command.payload.label
									? { label: command.payload.label }
									: {}),
							}),
						),
					},
				};
			case "importJsonl":
				return {
					type: "cancelled",
					payload: await runtime.importFromJsonl(
						command.payload.inputPath,
						command.payload.cwdOverride ?? undefined,
					),
				};
			case "exportHtml":
				return {
					type: "path",
					payload: {
						path: await runtime.session.exportToHtml(
							command.payload.outputPath ?? undefined,
						),
					},
				};
			case "exportJsonl":
				return {
					type: "path",
					payload: {
						path: runtime.session.exportToJsonl(
							command.payload.outputPath ?? undefined,
						),
					},
				};
			case "setSessionName": {
				const target = await this.runtimeForSessionPath(
					command.payload.sessionPath ?? undefined,
				);
				target.runtime.session.setSessionName(command.payload.name);
				return { type: "ack" };
			}
			case "getAvailableModels":
				return { type: "models", payload: { models: this.availableModels() } };
			case "setModel":
				await runtime.session.setModel(
					this.resolveModel(command.payload.model),
				);
				return { type: "ack" };
			case "cycleModel":
				return {
					type: "json",
					payload: {
						value: asJson(
							await runtime.session.cycleModel(command.payload.direction),
						),
					},
				};
			case "setThinkingLevel":
				runtime.session.setThinkingLevel(command.payload.level);
				return { type: "ack" };
			case "cycleThinkingLevel":
				return {
					type: "json",
					payload: { value: runtime.session.cycleThinkingLevel() ?? null },
				};
			case "getTools":
				return { type: "tools", payload: { tools: this.tools() } };
			case "setActiveTools":
				runtime.session.setActiveToolsByName(command.payload.toolNames);
				return { type: "ack" };
			case "setSteeringMode":
				runtime.session.setSteeringMode(queueMode(command.payload.mode));
				return { type: "ack" };
			case "setFollowUpMode":
				runtime.session.setFollowUpMode(queueMode(command.payload.mode));
				return { type: "ack" };
			case "setAutoCompaction":
				runtime.session.setAutoCompactionEnabled(command.payload.enabled);
				return { type: "ack" };
			case "setAutoRetry":
				runtime.session.setAutoRetryEnabled(command.payload.enabled);
				return { type: "ack" };
			case "compact":
				return {
					type: "json",
					payload: {
						value: asJson(
							await runtime.session.compact(
								command.payload.customInstructions ?? undefined,
							),
						),
					},
				};
			case "abortCompaction":
				runtime.session.abortCompaction();
				return { type: "ack" };
			case "abortRetry":
				runtime.session.abortRetry();
				return { type: "ack" };
			case "executeBash":
				return {
					type: "json",
					payload: {
						value: asJson(
							await runtime.session.executeBash(
								command.payload.command,
								(chunk) => {
									emitEvent(
										eventEnvelope({ type: "bashChunk", payload: { chunk } }),
									);
								},
								{ excludeFromContext: command.payload.excludeFromContext },
							),
						),
					},
				};
			case "abortBash":
				runtime.session.abortBash();
				return { type: "ack" };
			case "uiResponse":
				return {
					type: "json",
					payload: {
						value: current.ui.handleUiResponse(
							command.payload.requestId,
							command.payload.response,
						),
					},
				};
			case "autocomplete":
				return {
					type: "autocomplete",
					payload: {
						items: await current.ui.autocomplete(
							command.payload.text,
							command.payload.cursor,
						),
					},
				};
			case "terminalInput":
				return {
					type: "json",
					payload: {
						value: current.ui.handleTerminalInput(command.payload.data),
					},
				};
			case "componentInput":
				current.ui.handleComponentInput(
					command.payload.handle,
					command.payload.data,
				);
				return { type: "ack" };
			case "renderComponent":
				return {
					type: "componentRender",
					payload: {
						render: {
							handle: command.payload.handle,
							lines: current.ui.renderComponent(
								command.payload.handle,
								command.payload.width,
							),
						},
					},
				};
			case "setEditorText":
				current.ui.setEditorText(command.payload.text);
				return { type: "ack" };
			case "getEditorText":
				return { type: "text", payload: { text: current.ui.getEditorText() } };
			case "pasteToEditor":
				current.ui.pasteToEditor(command.payload.text);
				return { type: "ack" };
			case "setTheme":
				return {
					type: "json",
					payload: {
						value: current.ui.setTheme(
							command.payload.theme as Parameters<
								NativeExtensionUi["setTheme"]
							>[0],
						),
					},
				};
			case "getTheme":
				return {
					type: "json",
					payload: {
						value: asJson(current.ui.getTheme(command.payload.name) ?? null),
					},
				};
			case "getAllThemes":
				return {
					type: "json",
					payload: { value: asJson(current.ui.getAllThemes()) },
				};
			case "setToolsExpanded":
				current.ui.setToolsExpanded(command.payload.enabled);
				return { type: "ack" };
			default:
				throw new BridgeCommandError(
					"invalidCommand",
					`Unhandled command: ${command.type}`,
				);
		}
	}

	private async ensureAuthServices(
		command?: InitCommand,
	): Promise<PiAgentRuntime> {
		const piAgent = await loadPiAgentRuntime();
		const agentDir =
			command?.agentDir ?? this.authAgentDir ?? piAgent.getAgentDir();
		const testMode = command?.testMode != null;
		const shouldRecreate =
			!this.authStorage ||
			!this.modelRegistry ||
			(command != null &&
				!this.services &&
				(this.authAgentDir !== agentDir || this.authTestMode !== testMode));
		if (shouldRecreate) {
			this.authStorage = command?.testMode
				? piAgent.AuthStorage.inMemory()
				: piAgent.AuthStorage.create(join(agentDir, "auth.json"));
			this.modelRegistry = piAgent.ModelRegistry.inMemory(this.authStorage);
			this.authAgentDir = agentDir;
			this.authTestMode = testMode;
			if (command?.testMode) {
				this.authStorage.setRuntimeApiKey("faux", "pi-gpui-faux-key");
			}
		}
		return piAgent;
	}

	private async init(command: InitCommand): Promise<BridgeResponse> {
		if (this.services) {
			throw new BridgeCommandError(
				"alreadyInitialized",
				"Pi runtime is already initialized",
			);
		}
		const piAgent = await this.ensureAuthServices(command);
		const cwd = command.cwd;
		const agentDir = command.agentDir ?? piAgent.getAgentDir();
		if (command.model) {
			this.selectedModel = this.resolveModel(command.model);
		}
		this.enableExtensions = command.enableExtensions;
		const authStorage = this.authStorage;
		const modelRegistry = this.modelRegistry;
		if (!authStorage || !modelRegistry) {
			throw new BridgeCommandError(
				"notInitialized",
				"Pi auth services failed to initialize",
			);
		}
		const initialSelectedModel = this.selectedModel;
		const initialTools = command.tools ?? undefined;
		const testMode = command.testMode ?? undefined;
		const createRuntime: CreateAgentSessionRuntimeFactory = async (options) => {
			const services = await this.resources().createServices({
				cwd: options.cwd,
				agentDir: options.agentDir,
				authStorage,
				modelRegistry,
				enableExtensions: this.enableExtensions,
			});
			let runtimeModel = initialSelectedModel;
			if (testMode) {
				const piAi = await loadPiAiRuntime();
				this.faux?.unregister();
				const faux = piAi.registerFauxProvider({
					tokensPerSecond: testMode.tokensPerSecond ?? 0,
				});
				faux.setResponses(
					Array.from({ length: 32 }, () =>
						piAi.fauxAssistantMessage(testMode.fauxResponse),
					),
				);
				this.faux = faux;
				runtimeModel = faux.getModel();
				authStorage.setRuntimeApiKey(runtimeModel.provider, "pi-gpui-faux-key");
			}
			return {
				...(await piAgent.createAgentSessionFromServices({
					services,
					sessionManager: options.sessionManager,
					...(options.sessionStartEvent
						? { sessionStartEvent: options.sessionStartEvent }
						: {}),
					...(runtimeModel ? { model: runtimeModel } : {}),
					...(initialTools ? { tools: initialTools } : {}),
				})),
				services,
				diagnostics: services.diagnostics,
			};
		};
		this.createRuntimeFactory = createRuntime;
		this.runtimeCwd = cwd;
		this.runtimeAgentDir = agentDir;
		const sessionManager = this.createSessionManager(command, piAgent);
		const runtime = await piAgent.createAgentSessionRuntime(createRuntime, {
			cwd,
			agentDir,
			sessionManager,
			sessionStartEvent: { type: "session_start", reason: "startup" },
		});
		const ui = new NativeExtensionUi();
		this.services = { runtime, ui };
		this.configureRuntimeServices(this.services);
		await this.bindSession(this.services);
		this.emitSnapshot();
		return { type: "state", payload: { state: this.snapshot() } };
	}

	private async bindSession(current: RuntimeServices): Promise<void> {
		current.unsubscribe?.();
		this.cancelPendingMessageUpdate(current);
		this.rememberRuntime(current);
		current.unsubscribe = current.runtime.session.subscribe((event) => {
			const eventType = (event as { type?: string }).type;
			if (eventType === "message_update") {
				this.handleMessageUpdate(current, event);
				return;
			}
			this.flushPendingMessageUpdate(current);
			if (eventType === "agent_start") {
				this.emitSessionRunStarted(current);
				return;
			}
			if (eventType === "agent_end") {
				this.emitSessionRunFinished(current);
				return;
			}
			this.emitSessionEvent(current, event);
			if (eventType === "queue_update") {
				const queueEvent = event as unknown as {
					steering: string[];
					followUp: string[];
				};
				emitEvent(
					eventEnvelope({
						type: "queueUpdate",
						payload: {
							sessionId: current.runtime.session.sessionId ?? null,
							sessionFile: current.runtime.session.sessionFile ?? null,
							queue: {
								steering: queueEvent.steering,
								followUp: queueEvent.followUp,
							},
						},
					}),
				);
			}
		});
		if (this.enableExtensions) {
			await current.runtime.session.bindExtensions({
				uiContext: current.ui,
				onError: (error) => {
					emitEvent(
						eventEnvelope({
							type: "log",
							payload: {
								level: "error",
								message: `${error.extensionPath}: ${error.error}`,
							},
						}),
					);
				},
			});
		}
	}

	private emitSessionEvent(services: RuntimeServices, event: unknown): void {
		emitEvent(
			eventEnvelope({
				type: "piSessionEvent",
				payload: {
					sessionId: services.runtime.session.sessionId ?? null,
					sessionFile: services.runtime.session.sessionFile ?? null,
					event: asJson(event),
				},
			}),
		);
	}

	private emitSessionRunStarted(services: RuntimeServices): void {
		emitEvent(
			eventEnvelope({
				type: "sessionRunStarted",
				payload: {
					sessionId: services.runtime.session.sessionId ?? null,
					sessionFile: services.runtime.session.sessionFile ?? null,
				},
			}),
		);
	}

	private emitSessionRunFinished(services: RuntimeServices): void {
		emitEvent(
			eventEnvelope({
				type: "sessionRunFinished",
				payload: {
					sessionId: services.runtime.session.sessionId ?? null,
					sessionFile: services.runtime.session.sessionFile ?? null,
				},
			}),
		);
	}

	private emitSessionRunError(services: RuntimeServices, message: string): void {
		emitEvent(
			eventEnvelope({
				type: "sessionRunError",
				payload: {
					sessionId: services.runtime.session.sessionId ?? null,
					sessionFile: services.runtime.session.sessionFile ?? null,
					message,
				},
			}),
		);
	}

	private emitSessionTextDelta(services: RuntimeServices, delta: string): void {
		emitEvent(
			eventEnvelope({
				type: "sessionTextDelta",
				payload: {
					sessionId: services.runtime.session.sessionId ?? null,
					sessionFile: services.runtime.session.sessionFile ?? null,
					delta,
				},
			}),
		);
	}

	private handleMessageUpdate(services: RuntimeServices, event: unknown): void {
		const assistantEvent = (
			event as {
				assistantMessageEvent?: { type?: string; delta?: unknown };
			}
		).assistantMessageEvent;
		if (
			assistantEvent?.type === "text_delta" &&
			typeof assistantEvent.delta === "string"
		) {
			services.pendingTextDelta = `${services.pendingTextDelta ?? ""}${assistantEvent.delta}`;
			this.scheduleMessageUpdateFlush(services);
			return;
		}
		this.flushPendingMessageUpdate(services);
		services.pendingMessageUpdate = event;
		this.scheduleMessageUpdateFlush(services);
	}

	private scheduleMessageUpdateFlush(services: RuntimeServices): void {
		if (services.messageUpdateTimer) return;
		services.messageUpdateTimer = setTimeout(() => {
			services.messageUpdateTimer = undefined;
			this.flushPendingMessageUpdate(services);
		}, MESSAGE_UPDATE_FLUSH_MS);
	}

	private flushPendingMessageUpdate(services: RuntimeServices): void {
		const delta = services.pendingTextDelta;
		if (delta) {
			services.pendingTextDelta = undefined;
			this.emitSessionTextDelta(services, delta);
		}
		const event = services.pendingMessageUpdate;
		if (!event) return;
		services.pendingMessageUpdate = undefined;
		this.emitSessionEvent(services, event);
	}

	private cancelPendingMessageUpdate(services: RuntimeServices): void {
		if (services.messageUpdateTimer) {
			clearTimeout(services.messageUpdateTimer);
			services.messageUpdateTimer = undefined;
		}
		services.pendingMessageUpdate = undefined;
		services.pendingTextDelta = undefined;
	}

	private configureRuntimeServices(services: RuntimeServices): void {
		services.runtime.setRebindSession(async () => this.bindSession(services));
		services.runtime.setBeforeSessionInvalidate(() => services.ui.dispose());
		this.rememberRuntime(services);
	}

	private rememberRuntime(services: RuntimeServices): void {
		for (const key of services.sessionKeys ?? []) {
			if (this.sessionRuntimes.get(key) === services) {
				this.sessionRuntimes.delete(key);
			}
		}
		const keys = new Set<string>();
		const sessionFile = services.runtime.session.sessionFile;
		const sessionId = services.runtime.session.sessionId;
		if (sessionFile) keys.add(sessionFile);
		if (sessionId) keys.add(sessionId);
		for (const key of keys) this.sessionRuntimes.set(key, services);
		services.sessionKeys = keys;
	}

	private async runtimeForSessionPath(
		sessionPath?: string,
	): Promise<RuntimeServices> {
		if (!sessionPath) return this.requireRuntime();
		const existing = this.sessionRuntimes.get(sessionPath);
		if (existing) return existing;

		const createRuntime = this.createRuntimeFactory;
		const cwd = this.runtimeCwd;
		const agentDir = this.runtimeAgentDir;
		if (!createRuntime || !cwd || !agentDir) {
			throw new BridgeCommandError(
				"notInitialized",
				"Pi runtime has not been initialized",
			);
		}
		const piAgent = await loadPiAgentRuntime();
		const sessionManager = piAgent.SessionManager.open(
			sessionPath,
			undefined,
			cwd,
		);
		const runtime = await piAgent.createAgentSessionRuntime(createRuntime, {
			cwd,
			agentDir,
			sessionManager,
			sessionStartEvent: { type: "session_start", reason: "resume" },
		});
		const services = { runtime, ui: new NativeExtensionUi() };
		this.configureRuntimeServices(services);
		await this.bindSession(services);
		return services;
	}

	private createSessionManager(
		command: InitCommand,
		piAgent: PiAgentRuntime,
	): SessionManager {
		const session = command.session;
		if (!session || session.type === "new")
			return piAgent.SessionManager.create(command.cwd);
		if (session.type === "continueRecent")
			return piAgent.SessionManager.continueRecent(command.cwd);
		return piAgent.SessionManager.open(
			session.payload.path,
			undefined,
			command.cwd,
		);
	}

	private requireRuntime(): RuntimeServices {
		if (!this.services) {
			throw new BridgeCommandError(
				"notInitialized",
				"Pi runtime has not been initialized",
			);
		}
		return this.services;
	}

	private resources(): RuntimeResourceCache {
		this.resourceCache ??= new RuntimeResourceCache(piAgentRoot);
		return this.resourceCache;
	}

	private resolveModel(selection: ModelSelection): Model<string> {
		const fauxModel = this.faux?.getModel(selection.modelId);
		if (fauxModel && fauxModel.provider === selection.provider)
			return fauxModel;
		const model = this.modelRegistry?.find(
			selection.provider,
			selection.modelId,
		);
		if (!model) {
			throw new BridgeCommandError(
				"invalidPayload",
				`Unknown model ${selection.provider}/${selection.modelId}`,
			);
		}
		return model as Model<string>;
	}

	private authStatuses(provider?: string | null): ProviderAuthStatus[] {
		const providers = provider
			? [provider]
			: Array.from(
					new Set(
						(this.modelRegistry?.getAll() ?? []).map((model) => model.provider),
					),
				).sort();
		return providers.map((providerName) => {
			const status = this.modelRegistry?.getProviderAuthStatus(
				providerName,
			) ?? {
				configured: false,
			};
			return {
				provider: providerName,
				displayName:
					this.modelRegistry?.getProviderDisplayName(providerName) ??
					providerName,
				configured:
					status.configured || this.authStorage?.hasAuth(providerName) || false,
				source: status.source ? this.authSource(status.source) : null,
				label: status.label ?? null,
			};
		});
	}

	private authSource(source: string): ProviderAuthStatus["source"] {
		switch (source) {
			case "stored":
			case "runtime":
			case "environment":
			case "fallback":
				return source;
			case "models_json_key":
				return "modelsJsonKey";
			case "models_json_command":
				return "modelsJsonCommand";
			default:
				return null;
		}
	}

	private async searchPackages(
		query: string,
		limit = DEFAULT_PACKAGE_LIMIT,
	): Promise<PackageSearchResponse> {
		return searchPackageRegistry(query, limit);
	}

	private async listPackages(cwd: string) {
		return listInstalledPackages(this.packageManagerOptions(cwd));
	}

	private async installPackage(
		source: string,
		project: boolean,
		cwd: string,
	): Promise<void> {
		await installPackageSource(
			source,
			project,
			this.packageManagerOptions(cwd),
		);
		this.resourceCache?.clear();
	}

	private async removePackage(
		source: string,
		project: boolean,
		cwd: string,
	): Promise<void> {
		await removePackageSource(source, project, this.packageManagerOptions(cwd));
		this.resourceCache?.clear();
	}

	private packageManagerOptions(cwd: string) {
		return {
			cwd,
			authAgentDir: this.authAgentDir,
			loadPiAgentRuntime,
		};
	}

	private async oauthLogin(
		provider: string,
		method: OAuthLoginMethod | null,
	): Promise<void> {
		const authStorage = this.authStorage;
		if (!authStorage) {
			throw new BridgeCommandError(
				"notInitialized",
				"Pi auth storage is not initialized",
			);
		}
		const oauthProvider = authStorage
			.getOAuthProviders()
			.find((candidate) => candidate.id === provider);
		if (!oauthProvider) {
			throw new BridgeCommandError(
				"invalidPayload",
				`Provider ${provider} does not support OAuth login`,
			);
		}
		this.emitAuthFlowUpdate({
			provider,
			message: `Starting ${oauthProvider.name} authentication…`,
			url: null,
			userCode: null,
		});
		await authStorage.login(provider, {
			onAuth: (info) => {
				this.emitAuthFlowUpdate({
					provider,
					message:
						info.instructions ??
						"Complete authentication in the browser window.",
					url: info.url,
					userCode: null,
				});
				openExternalUrl(info.url);
			},
			onDeviceCode: (info) => {
				this.emitAuthFlowUpdate({
					provider,
					message: "Enter the device code in the opened browser window.",
					url: info.verificationUri,
					userCode: info.userCode,
				});
				openExternalUrl(info.verificationUri);
			},
			onPrompt: async (prompt) => {
				this.emitAuthFlowUpdate({
					provider,
					message: prompt.message,
					url: null,
					userCode: null,
				});
				if (prompt.allowEmpty) return "";
				throw new Error(
					`${prompt.message} Manual entry is not available in the desktop auth drawer yet.`,
				);
			},
			onProgress: (message) => {
				this.emitAuthFlowUpdate({
					provider,
					message,
					url: null,
					userCode: null,
				});
			},
			onSelect: async (prompt) => {
				const desired =
					method === "deviceCode"
						? "device_code"
						: method === "browser"
							? "browser"
							: undefined;
				return (
					prompt.options.find((option) => option.id === desired)?.id ??
					prompt.options[0]?.id
				);
			},
		});
		this.refreshModelsAfterAuthChange();
		this.emitAuthFlowUpdate({
			provider,
			message: `${oauthProvider.name} authentication complete.`,
			url: null,
			userCode: null,
		});
	}

	private emitAuthFlowUpdate(update: AuthFlowUpdate): void {
		emitEvent(
			eventEnvelope({
				type: "authFlowUpdate",
				payload: { update },
			}),
		);
	}

	private setApiKey(provider: string, apiKey: string, persist: boolean): void {
		if (persist) {
			this.authStorage?.set(provider, { type: "api_key", key: apiKey });
		} else {
			this.authStorage?.setRuntimeApiKey(provider, apiKey);
		}
		this.refreshModelsAfterAuthChange();
	}

	private removeAuth(provider: string): void {
		this.authStorage?.removeRuntimeApiKey(provider);
		this.authStorage?.remove(provider);
		this.refreshModelsAfterAuthChange();
	}

	private refreshModelsAfterAuthChange(): void {
		if (!this.faux) this.modelRegistry?.refresh();
	}

	private availableModels(): ModelDescriptor[] {
		const models = this.modelRegistry?.getAvailable() ?? [];
		const fauxModels = this.faux?.models ?? [];
		return [...fauxModels, ...models].map((model) =>
			this.modelDescriptor(model),
		);
	}

	private modelDescriptor(model: Model<any>): ModelDescriptor {
		return {
			provider: model.provider,
			id: model.id,
			name: model.name,
			reasoning: model.reasoning,
			contextWindow: model.contextWindow ?? null,
		};
	}

	private tools(): ToolDescriptor[] {
		const session = this.requireRuntime().runtime.session;
		const active = new Set(session.getActiveToolNames());
		return session.getAllTools().map((tool) => ({
			name: tool.name,
			description: tool.description,
			active: active.has(tool.name),
			source: tool.sourceInfo.source ?? null,
			parameters: asJson(tool.parameters),
		}));
	}

	private emitSnapshot(): void {
		emitEvent(
			eventEnvelope({
				type: "stateSnapshot",
				payload: { state: this.snapshot() },
			}),
		);
	}

	private emptySnapshot(): CoreStateSnapshot {
		return {
			initialized: false,
			cwd: null,
			sessionId: null,
			sessionFile: null,
			sessionName: null,
			isStreaming: false,
			isCompacting: false,
			isRetrying: false,
			isBashRunning: false,
			model: null,
			thinkingLevel: null,
			activeTools: [],
			queue: { steering: [], followUp: [] },
			messages: [],
			diagnostics: [],
		};
	}

	private async dispose(): Promise<void> {
		const uniqueServices = new Set(this.sessionRuntimes.values());
		if (this.services) uniqueServices.add(this.services);
		for (const services of uniqueServices) {
			this.flushPendingMessageUpdate(services);
			this.cancelPendingMessageUpdate(services);
			services.unsubscribe?.();
			services.ui.dispose();
			await services.runtime.dispose();
		}
		this.sessionRuntimes.clear();
		this.faux?.unregister();
		this.services = undefined;
		this.createRuntimeFactory = undefined;
		this.runtimeCwd = undefined;
		this.runtimeAgentDir = undefined;
		this.faux = undefined;
		this.selectedModel = undefined;
	}
}

export const piRuntimeBackend = new PiRuntimeBackend();

export function piVersion(): string {
	return "0.78.0";
}

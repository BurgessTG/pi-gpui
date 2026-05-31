import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type {
	AuthStorage,
	ModelRegistry,
	SessionManager,
	AgentSessionRuntime,
	CreateAgentSessionRuntimeFactory,
} from "@earendil-works/pi-coding-agent";
import type { FauxProviderRegistration, Model } from "@earendil-works/pi-ai";
import type {
	BridgeCommand,
	BridgeResponse,
	CoreStateSnapshot,
	ImageAttachment,
	InitCommand,
	JsonValue,
	ModelDescriptor,
	ModelSelection,
	ProviderAuthStatus,
	QueueMode,
	ThinkingLevel,
	ToolDescriptor,
} from "../generated/protocol.js";
import { BridgeCommandError } from "../bridge/errors.js";
import { eventEnvelope } from "../bridge/envelope.js";
import { emitEvent } from "../bridge/native.js";
import { NativeExtensionUi } from "./extension_ui.js";

type RuntimeServices = {
	runtime: AgentSessionRuntime;
	ui: NativeExtensionUi;
	unsubscribe?: () => void;
};

function asJson(value: unknown): JsonValue {
	const serialized = JSON.stringify(value);
	return serialized === undefined
		? null
		: (JSON.parse(serialized) as JsonValue);
}

function mapImages(
	images: ImageAttachment[],
): Array<{ type: "image"; data: string; mimeType: string }> {
	return images.map((image) => ({
		type: "image",
		data: image.dataBase64,
		mimeType: image.mediaType,
	}));
}

function queueMode(mode: QueueMode): "all" | "one-at-a-time" {
	return mode === "oneAtATime" ? "one-at-a-time" : "all";
}

type PiAgentRuntime = typeof import("@earendil-works/pi-coding-agent");
type PiAiRuntime = typeof import("@earendil-works/pi-ai");
let piAgentRuntimePromise: Promise<PiAgentRuntime> | undefined;
let piAiRuntimePromise: Promise<PiAiRuntime> | undefined;

async function piAgentRoot(): Promise<string> {
	const agentEntry = fileURLToPath(
		import.meta.resolve("@earendil-works/pi-coding-agent"),
	);
	return dirname(dirname(agentEntry));
}

async function loadPiAgentRuntime(): Promise<PiAgentRuntime> {
	piAgentRuntimePromise ??= (async () => {
		const root = await piAgentRoot();
		const moduleUrls = [
			"config.js",
			"core/auth-storage.js",
			"core/model-registry.js",
			"core/session-manager.js",
			"core/sdk.js",
		].map((path) => pathToFileURL(join(root, "dist", path)).href);
		const modules = await Promise.all(moduleUrls.map((url) => import(url)));
		return Object.assign({}, ...modules) as PiAgentRuntime;
	})();
	return piAgentRuntimePromise;
}

async function loadPiAiRuntime(): Promise<PiAiRuntime> {
	piAiRuntimePromise ??= (async () => {
		const nestedPiAi = pathToFileURL(
			join(
				await piAgentRoot(),
				"node_modules/@earendil-works/pi-ai/dist/index.js",
			),
		).href;
		return import(nestedPiAi) as Promise<PiAiRuntime>;
	})();
	return piAiRuntimePromise;
}

export class PiRuntimeBackend {
	private services: RuntimeServices | undefined;
	private authStorage: AuthStorage | undefined;
	private modelRegistry: ModelRegistry | undefined;
	private faux: FauxProviderRegistration | undefined;
	private selectedModel: Model<string> | undefined;
	private enableExtensions = true;

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
			case "prompt":
				await this.requireRuntime().runtime.session.prompt(
					command.payload.text,
					{
						images: mapImages(command.payload.images),
						...(command.payload.streamingBehavior
							? { streamingBehavior: command.payload.streamingBehavior }
							: {}),
					},
				);
				return { type: "ack" };
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
				return {
					type: "authStatus",
					payload: { statuses: this.authStatuses(command.payload.provider) },
				};
			case "setApiKey":
				this.setApiKey(
					command.payload.provider,
					command.payload.apiKey,
					command.payload.persist,
				);
				return { type: "ack" };
			case "removeAuth":
				this.removeAuth(command.payload.provider);
				return { type: "ack" };
			default:
				return this.dispatchSessionOrUi(command);
		}
	}

	snapshot(): CoreStateSnapshot {
		if (!this.services) {
			return this.emptySnapshot();
		}
		const session = this.services.runtime.session;
		return {
			initialized: true,
			cwd: this.services.runtime.cwd,
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
			diagnostics: this.services.runtime.diagnostics.map((diagnostic) => ({
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
			case "setSessionName":
				runtime.session.setSessionName(command.payload.name);
				return { type: "ack" };
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

	private async init(command: InitCommand): Promise<BridgeResponse> {
		if (this.services) {
			throw new BridgeCommandError(
				"alreadyInitialized",
				"Pi runtime is already initialized",
			);
		}
		const piAgent = await loadPiAgentRuntime();
		const cwd = command.cwd;
		const agentDir = command.agentDir ?? piAgent.getAgentDir();
		this.authStorage = command.testMode
			? piAgent.AuthStorage.inMemory()
			: piAgent.AuthStorage.create(join(agentDir, "auth.json"));
		this.modelRegistry = piAgent.ModelRegistry.inMemory(this.authStorage);
		if (command.testMode) {
			this.authStorage.setRuntimeApiKey("faux", "pi-gpui-faux-key");
		}
		if (command.model) {
			this.selectedModel = this.resolveModel(command.model);
		}
		this.enableExtensions = command.enableExtensions;
		const authStorage = this.authStorage;
		const modelRegistry = this.modelRegistry;
		const initialSelectedModel = this.selectedModel;
		const initialTools = command.tools ?? undefined;
		const testMode = command.testMode ?? undefined;
		const createRuntime: CreateAgentSessionRuntimeFactory = async (options) => {
			const services = await piAgent.createAgentSessionServices({
				cwd: options.cwd,
				agentDir: options.agentDir,
				authStorage,
				modelRegistry,
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
		const sessionManager = this.createSessionManager(command, piAgent);
		const runtime = await piAgent.createAgentSessionRuntime(createRuntime, {
			cwd,
			agentDir,
			sessionManager,
			sessionStartEvent: { type: "session_start", reason: "startup" },
		});
		const ui = new NativeExtensionUi();
		this.services = { runtime, ui };
		runtime.setRebindSession(async () => this.bindSession());
		runtime.setBeforeSessionInvalidate(() => ui.dispose());
		await this.bindSession();
		this.emitSnapshot();
		return { type: "state", payload: { state: this.snapshot() } };
	}

	private async bindSession(): Promise<void> {
		const current = this.requireRuntime();
		current.unsubscribe?.();
		current.unsubscribe = current.runtime.session.subscribe((event) => {
			emitEvent(
				eventEnvelope({
					type: "piSessionEvent",
					payload: { event: asJson(event) },
				}),
			);
			if ((event as { type?: string }).type === "queue_update") {
				const queueEvent = event as unknown as {
					steering: string[];
					followUp: string[];
				};
				emitEvent(
					eventEnvelope({
						type: "queueUpdate",
						payload: {
							queue: {
								steering: queueEvent.steering,
								followUp: queueEvent.followUp,
							},
						},
					}),
				);
			}
			this.emitSnapshot();
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
		this.services?.unsubscribe?.();
		this.services?.ui.dispose();
		await this.services?.runtime.dispose();
		this.faux?.unregister();
		this.services = undefined;
		this.faux = undefined;
		this.selectedModel = undefined;
	}
}

export const piRuntimeBackend = new PiRuntimeBackend();

export function piVersion(): string {
	return "0.78.0";
}

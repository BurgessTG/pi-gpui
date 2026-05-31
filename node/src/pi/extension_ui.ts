import { randomUUID } from "node:crypto";
import type {
	ExtensionUIContext,
	ExtensionUIDialogOptions,
	WorkingIndicatorOptions,
} from "@earendil-works/pi-coding-agent";
import type { AutocompleteProvider, Component } from "@earendil-works/pi-tui";
import type {
	AutocompleteItem,
	ComponentContent,
	ExtensionUiRequest,
	ExtensionUiResponse,
	ExtensionUiUpdate,
	JsonValue,
	WorkingUpdate,
} from "../generated/protocol.js";
import { eventEnvelope } from "../bridge/envelope.js";
import { emitEvent } from "../bridge/native.js";

type PendingUi = {
	resolve(value: ExtensionUiResponse): void;
};

type StoredComponent = Component & { dispose?(): void };

function asJson(value: unknown): JsonValue {
	const serialized = JSON.stringify(value);
	return serialized === undefined
		? null
		: (JSON.parse(serialized) as JsonValue);
}

function workingUpdate(patch: Partial<WorkingUpdate>): WorkingUpdate {
	return {
		message: null,
		visible: null,
		indicatorFrames: null,
		hiddenThinkingLabel: null,
		...patch,
	};
}

class EmptyAutocompleteProvider implements AutocompleteProvider {
	async getSuggestions(): Promise<null> {
		return null;
	}

	applyCompletion(
		lines: string[],
		cursorLine: number,
		cursorCol: number,
	): { lines: string[]; cursorLine: number; cursorCol: number } {
		return { lines, cursorLine, cursorCol };
	}
}

export class NativeExtensionUi implements ExtensionUIContext {
	readonly theme: ExtensionUIContext["theme"] =
		{} as ExtensionUIContext["theme"];
	private readonly pending = new Map<string, PendingUi>();
	private readonly components = new Map<string, StoredComponent>();
	private readonly terminalHandlers = new Set<
		(data: string) => { consume?: boolean; data?: string } | undefined
	>();
	private autocompleteProvider: AutocompleteProvider =
		new EmptyAutocompleteProvider();
	private editorText = "";
	private toolsExpanded = true;

	async select(
		title: string,
		options: string[],
		opts?: ExtensionUIDialogOptions,
	): Promise<string | undefined> {
		const response = await this.requestUi(
			{
				type: "select",
				payload: {
					id: randomUUID(),
					title,
					options,
					timeoutMs: opts?.timeout ?? null,
				},
			},
			opts,
		);
		return response.type === "selected"
			? (response.payload.value ?? undefined)
			: undefined;
	}

	async confirm(
		title: string,
		message: string,
		opts?: ExtensionUIDialogOptions,
	): Promise<boolean> {
		const response = await this.requestUi(
			{
				type: "confirm",
				payload: {
					id: randomUUID(),
					title,
					message,
					timeoutMs: opts?.timeout ?? null,
				},
			},
			opts,
		);
		return response.type === "confirmed" ? response.payload.value : false;
	}

	async input(
		title: string,
		placeholder?: string,
		opts?: ExtensionUIDialogOptions,
	): Promise<string | undefined> {
		const response = await this.requestUi(
			{
				type: "input",
				payload: {
					id: randomUUID(),
					title,
					placeholder: placeholder ?? null,
					timeoutMs: opts?.timeout ?? null,
				},
			},
			opts,
		);
		return response.type === "text"
			? (response.payload.value ?? undefined)
			: undefined;
	}

	notify(message: string, type: "info" | "warning" | "error" = "info"): void {
		this.emitUpdate({ type: "notify", payload: { message, level: type } });
	}

	onTerminalInput(
		handler: (data: string) => { consume?: boolean; data?: string } | undefined,
	): () => void {
		this.terminalHandlers.add(handler);
		return () => this.terminalHandlers.delete(handler);
	}

	setStatus(key: string, text: string | undefined): void {
		this.emitUpdate({ type: "status", payload: { key, text: text ?? null } });
	}

	setWorkingMessage(message?: string): void {
		this.emitUpdate({
			type: "working",
			payload: workingUpdate({ message: message ?? null }),
		});
	}

	setWorkingVisible(visible: boolean): void {
		this.emitUpdate({
			type: "working",
			payload: workingUpdate({ visible }),
		});
	}

	setWorkingIndicator(options?: WorkingIndicatorOptions): void {
		this.emitUpdate({
			type: "working",
			payload: workingUpdate({ indicatorFrames: options?.frames ?? null }),
		});
	}

	setHiddenThinkingLabel(label?: string): void {
		this.emitUpdate({
			type: "working",
			payload: workingUpdate({ hiddenThinkingLabel: label ?? null }),
		});
	}

	setWidget(
		key: string,
		content:
			| string[]
			| ((tui: never, theme: never) => StoredComponent)
			| undefined,
		options?: { placement?: "aboveEditor" | "belowEditor" },
	): void {
		const placement = options?.placement ?? "aboveEditor";
		this.emitUpdate({
			type: "widget",
			payload: { key, placement, content: this.componentContent(content) },
		});
	}

	setFooter(
		factory:
			| ((tui: never, theme: never, footerData: never) => StoredComponent)
			| undefined,
	): void {
		this.emitUpdate({
			type: "footer",
			payload: { content: this.componentContent(factory) },
		});
	}

	setHeader(
		factory: ((tui: never, theme: never) => StoredComponent) | undefined,
	): void {
		this.emitUpdate({
			type: "header",
			payload: { content: this.componentContent(factory) },
		});
	}

	setTitle(title: string): void {
		this.emitUpdate({ type: "title", payload: { title } });
	}

	async custom<T>(
		factory: (
			tui: never,
			theme: never,
			keybindings: never,
			done: (result: T) => void,
		) => StoredComponent | Promise<StoredComponent>,
		options?: { overlay?: boolean; overlayOptions?: unknown },
	): Promise<T> {
		const id = randomUUID();
		const handle = randomUUID();
		let resolved = false;
		return new Promise<T>((resolve) => {
			const done = (result: T): void => {
				if (!resolved) {
					resolved = true;
					resolve(result);
				}
			};
			Promise.resolve(
				factory({} as never, this.theme as never, {} as never, done),
			)
				.then((component) => {
					this.components.set(handle, component);
					this.pending.set(id, {
						resolve: (response) => {
							if (response.type === "custom") done(response.payload.value as T);
						},
					});
					this.emitRequest({
						type: "customComponent",
						payload: {
							id,
							handle,
							overlay: options?.overlay ?? false,
							overlayOptions:
								options?.overlayOptions === undefined
									? null
									: asJson(options.overlayOptions),
						},
					});
				})
				.catch((error: unknown) => {
					done(undefined as T);
					this.notify(
						error instanceof Error ? error.message : String(error),
						"error",
					);
				});
		});
	}

	pasteToEditor(text: string): void {
		this.editorText += text;
		this.emitUpdate({ type: "editorText", payload: { text: this.editorText } });
	}

	setEditorText(text: string): void {
		this.editorText = text;
		this.emitUpdate({ type: "editorText", payload: { text } });
	}

	getEditorText(): string {
		return this.editorText;
	}

	async editor(title: string, prefill?: string): Promise<string | undefined> {
		const response = await this.requestUi({
			type: "editor",
			payload: { id: randomUUID(), title, prefill: prefill ?? null },
		});
		return response.type === "text"
			? (response.payload.value ?? undefined)
			: undefined;
	}

	addAutocompleteProvider(
		factory: (current: AutocompleteProvider) => AutocompleteProvider,
	): void {
		this.autocompleteProvider = factory(this.autocompleteProvider);
	}

	setEditorComponent(_factory: unknown): void {
		this.notify(
			"Custom editor components are registered in the backend and require frontend host attachment.",
			"info",
		);
	}

	getEditorComponent(): undefined {
		return undefined;
	}

	getAllThemes(): { name: string; path: string | undefined }[] {
		return [{ name: "native-gpui-placeholder", path: undefined }];
	}

	getTheme(name: string): ExtensionUIContext["theme"] | undefined {
		return name === "native-gpui-placeholder" ? this.theme : undefined;
	}

	setTheme(theme: Parameters<ExtensionUIContext["setTheme"]>[0]): {
		success: boolean;
		error?: string;
	} {
		this.emitUpdate({ type: "theme", payload: { theme: asJson(theme) } });
		return { success: true };
	}

	getToolsExpanded(): boolean {
		return this.toolsExpanded;
	}

	setToolsExpanded(expanded: boolean): void {
		this.toolsExpanded = expanded;
		this.emitUpdate({ type: "toolsExpanded", payload: { expanded } });
	}

	handleUiResponse(requestId: string, response: ExtensionUiResponse): boolean {
		const pending = this.pending.get(requestId);
		if (!pending) return false;
		this.pending.delete(requestId);
		pending.resolve(response);
		return true;
	}

	handleTerminalInput(data: string): { consume: boolean; data: string } {
		let current = data;
		let consume = false;
		for (const handler of this.terminalHandlers) {
			const result = handler(current);
			if (!result) continue;
			if (result.consume) consume = true;
			if (typeof result.data === "string") current = result.data;
		}
		return { consume, data: current };
	}

	handleComponentInput(handle: string, data: string): void {
		this.components.get(handle)?.handleInput?.(data);
	}

	renderComponent(handle: string, width: number): string[] {
		const component = this.components.get(handle);
		if (!component) return [];
		return component.render(Math.max(1, width));
	}

	async autocomplete(
		text: string,
		cursor: number,
	): Promise<AutocompleteItem[]> {
		const beforeCursor = text.slice(0, cursor);
		const lines = beforeCursor.split("\n");
		const cursorLine = lines.length - 1;
		const cursorCol = lines[cursorLine]?.length ?? 0;
		const suggestions = await this.autocompleteProvider.getSuggestions(
			lines,
			cursorLine,
			cursorCol,
			{
				signal: new AbortController().signal,
				force: true,
			},
		);
		return (
			suggestions?.items.map((item) => ({
				label: item.label,
				detail: item.description ?? null,
				replacement: item.value,
			})) ?? []
		);
	}

	dispose(): void {
		for (const component of this.components.values()) {
			component.dispose?.();
		}
		this.components.clear();
		this.pending.clear();
		this.terminalHandlers.clear();
	}

	private async requestUi(
		request: ExtensionUiRequest,
		opts?: ExtensionUIDialogOptions,
	): Promise<ExtensionUiResponse> {
		if (opts?.signal?.aborted) return { type: "cancelled" };
		this.emitRequest(request);
		return new Promise<ExtensionUiResponse>((resolve) => {
			const id = request.payload.id;
			const abort = (): void => {
				this.pending.delete(id);
				resolve({ type: "cancelled" });
			};
			opts?.signal?.addEventListener("abort", abort, { once: true });
			this.pending.set(id, {
				resolve: (response) => {
					opts?.signal?.removeEventListener("abort", abort);
					resolve(response);
				},
			});
		});
	}

	private emitRequest(request: ExtensionUiRequest): void {
		emitEvent(
			eventEnvelope({ type: "extensionUiRequest", payload: { request } }),
		);
	}

	private emitUpdate(update: ExtensionUiUpdate): void {
		emitEvent(
			eventEnvelope({ type: "extensionUiUpdate", payload: { update } }),
		);
	}

	private componentContent(
		content: string[] | ((...args: never[]) => StoredComponent) | undefined,
	): ComponentContent | null {
		if (!content) return null;
		if (Array.isArray(content))
			return { type: "lines", payload: { lines: content } };
		const handle = randomUUID();
		try {
			const component = content({} as never, this.theme as never, {} as never);
			this.components.set(handle, component);
			return { type: "handle", payload: { handle } };
		} catch (error) {
			this.notify(
				error instanceof Error ? error.message : String(error),
				"error",
			);
			return null;
		}
	}
}

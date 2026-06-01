import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

type PiAgentRuntime = typeof import("@earendil-works/pi-coding-agent");
type PiAiRuntime = typeof import("@earendil-works/pi-ai");

let piAgentRuntimePromise: Promise<PiAgentRuntime> | undefined;
let piAiRuntimePromise: Promise<PiAiRuntime> | undefined;

export async function piAgentRoot(): Promise<string> {
	const agentEntry = fileURLToPath(
		import.meta.resolve("@earendil-works/pi-coding-agent"),
	);
	return dirname(dirname(agentEntry));
}

export async function loadPiAgentRuntime(): Promise<PiAgentRuntime> {
	piAgentRuntimePromise ??= (async () => {
		const root = await piAgentRoot();
		const moduleUrls = [
			"config.js",
			"core/auth-storage.js",
			"core/model-registry.js",
			"core/package-manager.js",
			"core/session-manager.js",
			"core/settings-manager.js",
			"core/sdk.js",
		].map((path) => pathToFileURL(join(root, "dist", path)).href);
		const modules = await Promise.all(moduleUrls.map((url) => import(url)));
		return Object.assign({}, ...modules) as PiAgentRuntime;
	})();
	return piAgentRuntimePromise;
}

export async function loadPiAiRuntime(): Promise<PiAiRuntime> {
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

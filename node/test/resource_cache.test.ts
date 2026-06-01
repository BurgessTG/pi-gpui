import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { AuthStorage, ModelRegistry } from "@earendil-works/pi-coding-agent";
import { describe, expect, it } from "vitest";
import { RuntimeResourceCache } from "../src/pi/resource_cache.js";

type CacheTestGlobal = typeof globalThis & {
	__piResourceCacheImportCount?: number;
	__piResourceCacheFactoryCount?: number;
};

async function piAgentRoot(): Promise<string> {
	const agentEntry = fileURLToPath(
		new URL(
			"../node_modules/@earendil-works/pi-coding-agent/dist/index.js",
			import.meta.url,
		),
	);
	return dirname(dirname(agentEntry));
}

describe("RuntimeResourceCache", () => {
	it("imports extension modules once but creates fresh extension registrations per service", async () => {
		const root = mkdtempSync(join(tmpdir(), "pi-resource-cache-"));
		const agentDir = join(root, "agent");
		const extensionsDir = join(agentDir, "extensions");
		mkdirSync(extensionsDir, { recursive: true });
		writeFileSync(
			join(agentDir, "settings.json"),
			'{"extensions":["extensions/cached-extension.js"]}',
		);
		writeFileSync(
			join(extensionsDir, "cached-extension.js"),
			`globalThis.__piResourceCacheImportCount = (globalThis.__piResourceCacheImportCount ?? 0) + 1;
export default (pi) => {
  globalThis.__piResourceCacheFactoryCount = (globalThis.__piResourceCacheFactoryCount ?? 0) + 1;
  pi.on('session_start', () => undefined);
};
`,
		);

		const globals = globalThis as CacheTestGlobal;
		globals.__piResourceCacheImportCount = 0;
		globals.__piResourceCacheFactoryCount = 0;

		const cache = new RuntimeResourceCache(piAgentRoot);
		const authStorage = AuthStorage.inMemory();
		const modelRegistry = ModelRegistry.inMemory(authStorage);
		const options = {
			cwd: root,
			agentDir,
			authStorage,
			modelRegistry,
			enableExtensions: true,
		};

		const first = await cache.createServices(options);
		const second = await cache.createServices(options);

		expect(first.resourceLoader.getExtensions().extensions).toHaveLength(1);
		expect(second.resourceLoader.getExtensions().extensions).toHaveLength(1);
		expect(globals.__piResourceCacheImportCount).toBe(1);
		expect(globals.__piResourceCacheFactoryCount).toBe(2);
	});
});

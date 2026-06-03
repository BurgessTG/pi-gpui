import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import type { InstalledPackage } from "../generated/protocol.js";
import { BridgeCommandError } from "../bridge/errors.js";

type PiAgentRuntime = typeof import("@earendil-works/pi-coding-agent");
type RuntimeLoader = () => Promise<PiAgentRuntime>;

type PackageManagerOptions = {
	cwd: string;
	authAgentDir?: string | undefined;
	loadPiAgentRuntime: RuntimeLoader;
};

export async function listInstalledPackages(
	options: PackageManagerOptions,
): Promise<InstalledPackage[]> {
	const manager = await packageManager(options);
	return manager.listConfiguredPackages().map((pkg) => {
		const metadata = readPackageMetadata(pkg.installedPath);
		return {
			source: pkg.source,
			displayName: displayNameForSource(pkg.source),
			scope: pkg.scope,
			filtered: pkg.filtered,
			installedPath: pkg.installedPath ?? null,
			version: metadata.version ?? null,
			description: metadata.description ?? null,
			canvasNodes: metadata.canvasNodes ?? [],
		};
	});
}

export async function installPackageSource(
	source: string,
	project: boolean,
	options: PackageManagerOptions,
): Promise<void> {
	const manager = await packageManager(options);
	await manager.installAndPersist(normalizePackageSource(source), {
		local: project,
	});
}

export async function removePackageSource(
	source: string,
	project: boolean,
	options: PackageManagerOptions,
): Promise<void> {
	const manager = await packageManager(options);
	const removed = await manager.removeAndPersist(
		normalizePackageSource(source),
		{
			local: project,
		},
	);
	if (!removed) {
		throw new BridgeCommandError(
			"invalidPayload",
			`No installed package matched ${source}`,
		);
	}
}

async function packageManager(options: PackageManagerOptions) {
	const piAgent = await options.loadPiAgentRuntime();
	const agentDir = options.authAgentDir ?? piAgent.getAgentDir();
	const settingsManager = piAgent.SettingsManager.create(options.cwd, agentDir);
	return new piAgent.DefaultPackageManager({
		cwd: options.cwd,
		agentDir,
		settingsManager,
	});
}

function normalizePackageSource(source: string): string {
	const trimmed = source
		.trim()
		.replace(/^pi\s+install\s+(?:-l\s+)?/i, "")
		.trim();
	if (!trimmed) {
		throw new BridgeCommandError(
			"invalidPayload",
			"Package source is required",
		);
	}
	return /^(npm:|git:|https?:|ssh:|git:|\.|\/)/.test(trimmed)
		? trimmed
		: `npm:${trimmed}`;
}

function displayNameForSource(source: string): string {
	const npm = source.match(/^npm:(@?[^@]+\/[^@]+|[^@/]+)(?:@.+)?$/);
	if (npm) return npm[1];
	return basename(source.replace(/\/$/, ""));
}

function readPackageMetadata(installedPath?: string): {
	version?: string;
	description?: string;
	canvasNodes?: InstalledPackage["canvasNodes"];
} {
	if (!installedPath) return { canvasNodes: [] };
	try {
		const packageJson = JSON.parse(
			readFileSync(join(installedPath, "package.json"), "utf8"),
		) as {
			version?: string;
			description?: string;
			pi?: { canvasNodes?: unknown };
			piCanvasNodes?: unknown;
		};
		return Object.fromEntries(
			Object.entries({
				version: packageJson.version,
				description: packageJson.description,
				canvasNodes: normalizeCanvasNodeManifests(
					packageJson.pi?.canvasNodes ?? packageJson.piCanvasNodes,
				),
			}).filter(([, value]) => value !== undefined),
		) as {
			version?: string;
			description?: string;
			canvasNodes?: InstalledPackage["canvasNodes"];
		};
	} catch {
		return { canvasNodes: [] };
	}
}

function normalizeCanvasNodeManifests(
	value: unknown,
): InstalledPackage["canvasNodes"] {
	if (!Array.isArray(value)) return [];
	return value.flatMap((entry) => {
		const manifest = entry as {
			id?: unknown;
			label?: unknown;
			runtime?: unknown;
			renderMode?: unknown;
		};
		if (typeof manifest.id !== "string" || typeof manifest.label !== "string") {
			return [];
		}
		const id = manifest.id.trim();
		const label = manifest.label.trim();
		if (!id || !label) return [];
		return [
			{
				id,
				label,
				runtime: normalizeCanvasNodeRuntime(manifest.runtime),
				renderMode: normalizeCanvasNodeRenderMode(manifest.renderMode),
			},
		];
	});
}

function normalizeCanvasNodeRuntime(
	value: unknown,
): InstalledPackage["canvasNodes"][number]["runtime"] {
	return value === "piSession" || value === "workerProcess" ? value : "none";
}

function normalizeCanvasNodeRenderMode(
	value: unknown,
): InstalledPackage["canvasNodes"][number]["renderMode"] {
	return value === "sceneOnly" ? "sceneOnly" : "gpuiIsland";
}

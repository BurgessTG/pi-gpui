import type { PackageSearchResponse } from "../generated/protocol.js";
import { BridgeCommandError } from "../bridge/errors.js";

const NPM_SEARCH_ENDPOINT = "https://registry.npmjs.org/-/v1/search";
export const DEFAULT_PACKAGE_LIMIT = 20;

interface NpmRegistrySearchResponse {
	total?: number;
	objects?: NpmRegistrySearchObject[];
}

interface NpmRegistrySearchObject {
	downloads?: { monthly?: number };
	updated?: string;
	package?: {
		name?: string;
		version?: string;
		description?: string;
		date?: string;
		keywords?: string[];
		publisher?: { username?: string };
		links?: {
			npm?: string;
			repository?: string;
			homepage?: string;
		};
	};
	score?: { final?: number };
	searchScore?: number;
}

export async function searchPackageRegistry(
	query: string,
	limit = DEFAULT_PACKAGE_LIMIT,
): Promise<PackageSearchResponse> {
	const response = await fetch(buildPackageSearchUrl(query, limit), {
		headers: { accept: "application/json" },
	});
	if (!response.ok) {
		throw new BridgeCommandError(
			"piSdkError",
			`npm package search failed with ${response.status} ${response.statusText}`,
		);
	}
	const payload = (await response.json()) as NpmRegistrySearchResponse;
	const results = (payload.objects ?? []).map((item) => {
		const name = item.package?.name ?? "unknown-package";
		const keywords = item.package?.keywords ?? [];
		return {
			name,
			version: item.package?.version ?? "0.0.0",
			description: item.package?.description ?? "No description provided.",
			publisher: item.package?.publisher?.username ?? null,
			monthlyDownloads: item.downloads?.monthly ?? null,
			updated: item.updated ?? item.package?.date ?? null,
			resourceTypes: resourceTypesFromKeywords(keywords),
			installCommand: `pi install npm:${name}`,
			npmUrl:
				item.package?.links?.npm ??
				`https://www.npmjs.com/package/${encodeURIComponent(name)}`,
			repositoryUrl: item.package?.links?.repository ?? null,
			homepageUrl: item.package?.links?.homepage ?? null,
			score: item.score?.final ?? item.searchScore ?? 0,
		};
	});
	return {
		query: query.trim(),
		limit: Math.min(Math.max(Math.trunc(limit), 1), 50),
		total: payload.total ?? results.length,
		results,
	};
}

function buildPackageSearchUrl(query: string, limit: number): URL {
	const url = new URL(NPM_SEARCH_ENDPOINT);
	const trimmed = query.trim();
	url.searchParams.set(
		"text",
		trimmed ? `keywords:pi-package ${trimmed}` : "keywords:pi-package",
	);
	url.searchParams.set("size", String(Math.min(Math.max(limit, 1), 50)));
	return url;
}

function resourceTypesFromKeywords(keywords: string[] | undefined): string[] {
	const keywordMap: Array<[string, string]> = [
		["pi-extension", "extension"],
		["pi-skill", "skill"],
		["pi-prompt", "prompt"],
		["pi-theme", "theme"],
		["pi-canvas-node", "canvas node"],
	];
	const found = keywordMap
		.filter(([keyword]) => keywords?.includes(keyword))
		.map(([, label]) => label);
	return found.length > 0 ? found : ["package"];
}

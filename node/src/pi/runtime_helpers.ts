import { execFile } from "node:child_process";
import type {
	ImageAttachment,
	JsonValue,
	QueueMode,
} from "../generated/protocol.js";

export function asJson(value: unknown): JsonValue {
	const serialized = JSON.stringify(value);
	return serialized === undefined
		? null
		: (JSON.parse(serialized) as JsonValue);
}

export function mapImages(
	images: ImageAttachment[],
): Array<{ type: "image"; data: string; mimeType: string }> {
	return images.map((image) => ({
		type: "image",
		data: image.dataBase64,
		mimeType: image.mediaType,
	}));
}

export function queueMode(mode: QueueMode): "all" | "one-at-a-time" {
	return mode === "oneAtATime" ? "one-at-a-time" : "all";
}

export function openExternalUrl(url: string): void {
	const command =
		process.platform === "darwin"
			? "open"
			: process.platform === "win32"
				? "cmd"
				: "xdg-open";
	const args = process.platform === "win32" ? ["/c", "start", "", url] : [url];
	try {
		execFile(command, args, () => undefined);
	} catch {
		// The auth URL is still emitted to the frontend for manual opening.
	}
}

import { createInterface } from "node:readline";

process.env.PI_GPUI_PROCESS = "1";
process.env.TERMUX_VERSION ??= "pi-gpui-disable-native-clipboard";
console.log = (...args: unknown[]) => console.error(...args);

function writeJsonLine(value: unknown): void {
	process.stdout.write(`${JSON.stringify(value)}\n`);
}

globalThis.__PI_GPUI_NATIVE = {
	emitEvent: (json: string) => writeJsonLine(JSON.parse(json) as unknown),
	emitResponse: (json: string) => writeJsonLine(JSON.parse(json) as unknown),
};

const [
	{ bridgeDispatcher },
	{ eventEnvelope, PROTOCOL_VERSION },
	{ piVersion },
] = await Promise.all([
	import("./bridge/dispatcher.js"),
	import("./bridge/envelope.js"),
	import("./pi/runtime.js"),
]);

globalThis.__piBridge = bridgeDispatcher;

writeJsonLine(
	eventEnvelope({
		type: "ready",
		payload: {
			nodeVersion: process.versions.node,
			piVersion: piVersion(),
			protocolVersion: PROTOCOL_VERSION,
		},
	}),
);

const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });

lines.on("line", (line) => {
	void dispatchLine(line);
});

async function dispatchLine(line: string): Promise<void> {
	const trimmed = line.trim();
	if (!trimmed) return;
	try {
		const envelope = JSON.parse(trimmed) as Parameters<
			typeof bridgeDispatcher.dispatch
		>[0];
		writeJsonLine(await bridgeDispatcher.dispatch(envelope));
		if (envelope.command.type === "shutdown") process.exit(0);
	} catch (error) {
		writeJsonLine(
			eventEnvelope({
				type: "fatalError",
				payload: {
					error: {
						code: "nodeRuntimeError",
						message:
							error instanceof Error && error.stack
								? error.stack
								: String(error),
						details: null,
						retryable: false,
					},
				},
			}),
		);
	}
}

process.on("SIGTERM", () => {
	void bridgeDispatcher
		.dispatch({
			version: PROTOCOL_VERSION,
			requestId: "shutdown",
			command: { type: "shutdown" },
		})
		.finally(() => process.exit(0));
});

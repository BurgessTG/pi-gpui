import { eventEnvelope, PROTOCOL_VERSION } from "./bridge/envelope.js";
import { emitEvent } from "./bridge/native.js";

declare global {
	var __piBridge: typeof bridgeDispatcher | undefined;
}

process.env.PI_GPUI_EMBEDDED = "1";
process.env.TERMUX_VERSION ??= "pi-gpui-disable-native-clipboard";

const [{ bridgeDispatcher }, { piVersion }] = await Promise.all([
	import("./bridge/dispatcher.js"),
	import("./pi/runtime.js"),
]);

globalThis.__piBridge = bridgeDispatcher;

emitEvent(
	eventEnvelope({
		type: "ready",
		payload: {
			nodeVersion: process.versions.node,
			piVersion: piVersion(),
			protocolVersion: PROTOCOL_VERSION,
		},
	}),
);

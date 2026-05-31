import type { BridgeEventEnvelope, BridgeResponseEnvelope } from '../generated/protocol.js'

type NativeBridge = {
  emitEvent(json: string): void
  emitResponse(json: string): void
}

declare global {
  var __PI_GPUI_NATIVE: NativeBridge | undefined
}

export function nativeBridge(): NativeBridge {
  const native = globalThis.__PI_GPUI_NATIVE
  if (!native) {
    throw new Error('Native Pi GPUI bridge is not installed')
  }
  return native
}

export function emitEvent(event: BridgeEventEnvelope): void {
  nativeBridge().emitEvent(JSON.stringify(event))
}

export function emitResponse(response: BridgeResponseEnvelope): void {
  nativeBridge().emitResponse(JSON.stringify(response))
}

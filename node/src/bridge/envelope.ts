import type { BridgeEvent, BridgeEventEnvelope, BridgeResponse, BridgeResponseEnvelope, RequestId } from '../generated/protocol.js'

export const PROTOCOL_VERSION = 3

export function ok(requestId: RequestId, value: BridgeResponse): BridgeResponseEnvelope {
  return {
    version: PROTOCOL_VERSION,
    requestId,
    response: { status: 'ok', value }
  }
}

export function eventEnvelope(event: BridgeEvent): BridgeEventEnvelope {
  return { version: PROTOCOL_VERSION, event }
}

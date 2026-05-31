import type { BridgeCommandEnvelope, BridgeResponseEnvelope } from '../generated/protocol.js'
import { normalizeError } from './errors.js'
import { ok, PROTOCOL_VERSION } from './envelope.js'
import { piRuntimeBackend } from '../pi/runtime.js'

export class BridgeDispatcher {
  async dispatch(envelope: BridgeCommandEnvelope): Promise<BridgeResponseEnvelope> {
    if (envelope.version !== PROTOCOL_VERSION) {
      return {
        version: PROTOCOL_VERSION,
        requestId: envelope.requestId,
        response: {
          status: 'error',
          error: {
            code: 'protocolVersionMismatch',
            message: `Unsupported protocol version ${envelope.version}`,
            details: null,
            retryable: false
          }
        }
      }
    }
    try {
      return ok(envelope.requestId, await piRuntimeBackend.dispatch(envelope.command))
    } catch (error) {
      return {
        version: PROTOCOL_VERSION,
        requestId: envelope.requestId,
        response: { status: 'error', error: normalizeError(error, 'piSdkError') }
      }
    }
  }
}

export const bridgeDispatcher = new BridgeDispatcher()

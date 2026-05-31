import type { BridgeError, BridgeErrorCode } from '../generated/protocol.js'

export class BridgeCommandError extends Error {
  readonly code: BridgeErrorCode
  readonly details: string | null
  readonly retryable: boolean

  constructor(code: BridgeErrorCode, message: string, options: { details?: string; retryable?: boolean } = {}) {
    super(message)
    this.name = 'BridgeCommandError'
    this.code = code
    this.details = options.details ?? null
    this.retryable = options.retryable ?? false
  }
}

export function normalizeError(error: unknown, fallback: BridgeErrorCode = 'internal'): BridgeError {
  if (error instanceof BridgeCommandError) {
    return {
      code: error.code,
      message: error.message,
      details: error.details,
      retryable: error.retryable
    }
  }
  if (error instanceof Error) {
    return {
      code: fallback,
      message: error.message,
      details: error.stack ?? null,
      retryable: false
    }
  }
  return {
    code: fallback,
    message: String(error),
    details: null,
    retryable: false
  }
}

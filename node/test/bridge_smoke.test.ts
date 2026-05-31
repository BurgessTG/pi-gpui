import { describe, expect, it, vi } from 'vitest'
import type { BridgeEventEnvelope, BridgeResponseEnvelope } from '../src/generated/protocol.js'

describe('bootstrap bridge', () => {
  it('installs a global dispatcher and emits ready', async () => {
    const events: BridgeEventEnvelope[] = []
    const responses: BridgeResponseEnvelope[] = []
    globalThis.__PI_GPUI_NATIVE = {
      emitEvent: (json: string) => events.push(JSON.parse(json) as BridgeEventEnvelope),
      emitResponse: (json: string) => responses.push(JSON.parse(json) as BridgeResponseEnvelope)
    }
    vi.resetModules()
    await import('../src/bootstrap.js')
    expect(globalThis.__piBridge).toBeTruthy()
    expect(events.some((event) => event.event.type === 'ready')).toBe(true)
    expect(responses).toHaveLength(0)
  })
})

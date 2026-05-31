import { describe, expect, it } from 'vitest'
import type { BridgeEventEnvelope } from '../src/generated/protocol.js'
import { NativeExtensionUi } from '../src/pi/extension_ui.js'

describe('native extension UI bridge', () => {
  it('emits select requests and resolves responses', async () => {
    const events: BridgeEventEnvelope[] = []
    globalThis.__PI_GPUI_NATIVE = {
      emitEvent: (json: string) => events.push(JSON.parse(json) as BridgeEventEnvelope),
      emitResponse: () => undefined
    }
    const ui = new NativeExtensionUi()
    const promise = ui.select('Pick one', ['a', 'b'])
    const request = events.find((event) => event.event.type === 'extensionUiRequest')
    expect(request?.event.type).toBe('extensionUiRequest')
    if (request?.event.type !== 'extensionUiRequest') throw new Error('missing request')
    const id = request.event.payload.request.payload.id
    ui.handleUiResponse(id, { type: 'selected', payload: { value: 'b' } })
    await expect(promise).resolves.toBe('b')
  })

  it('stores and renders JS-owned components', () => {
    const events: BridgeEventEnvelope[] = []
    globalThis.__PI_GPUI_NATIVE = {
      emitEvent: (json: string) => events.push(JSON.parse(json) as BridgeEventEnvelope),
      emitResponse: () => undefined
    }
    const ui = new NativeExtensionUi()
    ui.setWidget('demo', () => ({
      render: (width: number) => [`width:${width}`],
      handleInput: () => undefined,
      invalidate: () => undefined
    }))
    const update = events.find((event) => event.event.type === 'extensionUiUpdate')
    expect(update?.event.type).toBe('extensionUiUpdate')
    if (update?.event.type !== 'extensionUiUpdate') throw new Error('missing update')
    const payload = update.event.payload.update.payload
    if (!('content' in payload) || payload.content?.type !== 'handle') throw new Error('missing handle')
    expect(ui.renderComponent(payload.content.payload.handle, 12)).toEqual(['width:12'])
  })
})

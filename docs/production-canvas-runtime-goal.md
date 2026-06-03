# Production Canvas Runtime Goal

## Goal

Build Pi Workspaces as an Excalidraw-class infinite canvas where users can create extensible node types, run many agent sessions simultaneously, and keep pan/zoom/click/scroll interactions fluid while all offscreen and onscreen work preserves state.

## Non-negotiable architecture principles

1. **GPUI process is UI-first**: agent runtimes, SDK loops, blocking tools, and heavy stream processing must not run on the GPUI/UI thread.
2. **Worker-process runtime isolation**: active agent sessions run in external worker processes or process groups with explicit IPC, backpressure, and lifecycle control.
3. **Typed delta protocol**: streaming output crosses the bridge as compact typed deltas, not full growing messages or raw SDK event blobs at token rate.
4. **Retained scene canvas**: the canvas stores unlimited objects, but each frame only resolves visible/active/selected objects through spatial indexes and level-of-detail rules.
5. **GPUI active islands**: full GPUI views are reserved for visible/focused/interactive panels; offscreen or zoomed-out nodes render as cheap scene objects/snapshots/status shells.
6. **Extensible node system**: node definitions should be pluggable like packages/extensions, with typed manifests, capabilities, render/interaction contracts, and worker/runtime requirements.
7. **State never depends on viewport**: moving away from an area must not stop agents or lose UI/runtime state; viewport only controls what is materialized/rendered.
8. **Measured performance budgets**: frame time, input latency, event queue depth, worker count, bridge throughput, and render counts must be observable without a UI overlay.

## Acceptance criteria

- Two, ten, and many concurrent agent runs do not block canvas pan/zoom/click interactions.
- Streaming nodes update through bounded/coalesced UI work; final markdown quality is preserved after completion.
- Agent workers can be created, monitored, cancelled, and cleaned up outside the GPUI process.
- The bridge protocol uses typed run/session/node deltas with backpressure and no token-rate full-message JSON crossing.
- Canvas object count can grow well beyond visible nodes; only visible/active objects are materialized as GPUI views.
- Node creation is extension/package friendly: new node kinds can be registered without hardcoding every node type in the main canvas renderer.
- Validation includes automated checks plus render/runtime instrumentation evidence.

## Current first production slice

- Pi Desktop now starts the Node runtime through an external `node/dist/process_host.js` JSONL process host by default.
- The prior embedded libnode host remains in the codebase for tests and fallback work, but it is no longer the default desktop runtime boundary.
- Streaming assistant text now crosses into Rust as coalesced compact `assistant_text_delta` events instead of repeated full growing assistant message snapshots; final message events still preserve markdown/tool fidelity.
- Prompt commands now acknowledge run submission immediately; session run start/finish/error lifecycle crosses the bridge as typed events so long-running prompts no longer hold request/response IPC slots open.
- The external Node worker host now fails pending requests and emits a fatal backend event if worker stdout closes or fails, and process-host shutdown exits cleanly after disposal.

## Next milestones

1. Replace raw `serde_json::Value` session events with typed compact delta events.
2. Convert prompts from long-lived request/response calls into run-start acknowledgements plus run lifecycle events.
3. Add a worker supervisor and per-session/process lifecycle model.
4. Introduce a node type registry/manifest model for extensible canvas nodes.
5. Split canvas rendering into retained scene layers and GPUI active islands with LOD/snapshot shells.
6. Add performance instrumentation thresholds and regression tests for stream throughput and UI frame pacing.

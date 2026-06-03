# Progress

## Status
Complete locally; validated end-to-end.

## Tasks Completed
- Implemented targeted protocol routing for concurrent chat sessions (`sessionPath`, `getSessionState`, and session/node metadata on stream events).
- Refactored Node runtime into an in-process multi-session runtime pool keyed by session id/path, with targeted snapshots and event emission.
- Replaced global prompt serialization with per-session backend locks and multi-node frontend streaming state.
- Preserved same-node prompt ordering with a local queued prompt path while allowing different nodes to stream concurrently.
- Added bottom-right zoom-aware resize grip affordance with hover highlight and diagonal resize cursor.
- Added dynamic minimap world bounds for distant/unbounded canvas content.
- Added node spatial indexing, drawing broadphase culling, cached drawing path geometry, and block-level markdown view caching.
- Preserved entity/cached GPUI rendering architecture, transcript virtualization, pinned panels, world-space zoom, animations, markdown, and env-gated render tracing.
- Fixed the stale `node/dist` failure mode by auto-building the embedded Node backend when TypeScript sources are newer than the generated runtime bundle, and bumped the bridge protocol version to catch mismatched bundles clearly.
- Fixed scroll ownership so node hover/scroll stops wheel propagation to canvas zoom, while canvas wheel zoom remains active on bare canvas.
- Reduced streaming/runtime jank by batching backend stream events per frame and removing redundant per-session-event global state snapshots from the Node bridge.
- Added a second performance pass after live lag reports: coalesced SDK `message_update` events in Node before JSON crossing, grouped Rust bridge events by session target across each batch, and deferred markdown rendering for actively streaming assistant text until completion.
- Started the production architecture goal by moving Pi Desktop's default backend from embedded libnode to an external Node process host over JSONL stdio, keeping GPUI as a UI-only process boundary for the first worker-process runtime slice.
- Replaced token-rate full assistant message updates during streaming with coalesced compact `assistant_text_delta` events; final message events still preserve complete markdown/tool fidelity.
- Converted prompt submission to immediate ACK plus typed session run lifecycle events (`sessionRunStarted`, `sessionRunFinished`, `sessionRunError`), so long-running agent turns no longer keep bridge requests open.
- Hardened the external Node worker lifecycle: worker stdout failure now fails pending requests and emits a fatal backend event, and process-host shutdown exits after runtime disposal.
- Added the first canvas node registry scaffold for package/extensible node definitions, with built-in Pi session nodes carrying runtime and render-mode metadata.
- Added env-gated Node worker bridge instrumentation for request/response/event rates, byte totals, stderr lines, invalid stdout, stdout closure, and max pending request depth.
- Converted tool execution start/update/end transport to typed session tool bridge events and bumped the bridge protocol to v3.
- Added a first retained-scene canvas materialization plan for session nodes with visible indexed queries and zoomed-out low-detail shells.
- Added package canvas-node manifest transport from installed package `package.json` metadata (`pi.canvasNodes` / `piCanvasNodes`) and surfaced node counts in the package settings table.
- Added bounded Node worker IPC backpressure via `NodeProcessHostConfig::max_pending_requests` (default 256) to prevent unbounded pending request growth.
- Wired installed package canvas-node manifests into Pi Desktop's canvas node registry on backend data refresh.

## Validation
- `cargo fmt --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd node && npm run build`
- `cd node && npm run typecheck`
- `cd node && npm test`
- `cd node && npm run check-protocol`
- `git diff --check`
- LSP diagnostics on workspace: clean.
- `timeout 8s cargo run -p pi-desktop` compiled and launched the app, then was intentionally terminated by timeout.

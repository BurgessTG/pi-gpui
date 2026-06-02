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

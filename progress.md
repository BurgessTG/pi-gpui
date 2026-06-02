# Progress

## Status
Complete locally; validated.

## Tasks Completed
- Refactored workspace canvas, chat nodes, bottom dock, status bar/tabs, and pinned panels toward persistent GPUI `Entity` views.
- Added safe `AnyView::cached(...)` boundaries with uncached same-frame rendering when synced props change.
- Added transcript virtualization with stable `ChatMessageView` entities and bottom-aligned streaming scroll preservation.
- Added Hyprland-style pinned panel rendering through chat node entities.
- Added env-gated non-UI render tracing (`PI_WORKSPACES_RENDER_TRACE`, `PI_WORKSPACES_RENDER_TRACE_MS`).
- Replaced fixed canvas repaint timer for interactions with frame-aligned `window.on_next_frame` scheduling.
- Added cached drawing bounds plus a real broadphase spatial index for drawing render culling, hit testing, and erasing.
- Filtered canvas/pinned child maps to active-workspace-local keys to avoid cloning/comparing unrelated workspaces.
- Split canvas tests into `workspace/canvas_tests.rs` to keep core canvas code smaller.

## Validation
- `cargo fmt --check`
- `cargo check -p pi-desktop`
- `cargo check --workspace --all-targets`
- `cargo clippy -p pi-desktop --all-targets -- -D warnings`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pi-desktop`
- `cargo test --workspace`
- `cd node && npm run typecheck`
- `cd node && npm test`
- `cd node && npm run check-protocol`
- `git diff --check`
- LSP diagnostics on touched Rust files: clean.

## Remaining Optional Work
- Manual UX smoke test for pin/unpin, pinned resize/focus, shortcuts, streaming, pan/zoom/draw/erase/text boxes, tabs, and bottom dock.
- Measure with render trace before attempting riskier canvas-state entity/revision extraction or path geometry caching.
- Optional backend routing for multiple active sessions.

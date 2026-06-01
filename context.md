# Code Context

## Files Retrieved
1. `crates/pi-desktop/src/app.rs` (lines 71-134, 247-270) - root `PiDesktop` state, chat/transcript maps, render throttle flags.
2. `crates/pi-desktop/src/app/render.rs` (lines 6-24, 94-333, 385-544) - root render and workspace/canvas/pinned composition.
3. `crates/pi-desktop/src/app/backend_flow.rs` (lines 1-283) - backend subscriptions, bridge event application, render throttling.
4. `crates/pi-desktop/src/app/chat_actions.rs` (lines 16-120) - chat submit/stream lifecycle.
5. `crates/pi-desktop/src/app/canvas_actions.rs` (lines 162-367) - canvas drag/draw/zoom updates and throttled render requests.
6. `crates/pi-desktop/src/components/workspace_canvas.rs` (lines 1-929) - canvas element tree, node visibility, drawing/grid/text-box rendering.
7. `crates/pi-desktop/src/components/chat_node.rs` (lines 1-777) - chat node, transcript body, markdown/tool/composer rendering.
8. `crates/pi-desktop/src/components/pinned_panels.rs` (lines 17-54) - pinned node panel rendering path.
9. `crates/pi-desktop/src/workspace/canvas.rs` (lines 1-687) - `CanvasState` model and mutation operations.
10. `crates/pi-desktop/src/workspace/state.rs` (lines 1-488) - workspace/tab wrappers around active canvas and pinned layout.
11. `crates/pi-desktop/src/chat/transcript.rs` (lines 1-418) - transcript storage and bridge event parsing.

## Key Code
- Root state is monolithic: `PiDesktop` owns `workspace_state`, input entity maps, `chat_transcripts: HashMap<(usize, usize), ChatTranscript>`, `streaming_node`, `event_render_scheduled`, and `canvas_render_scheduled` (`app.rs:83-134`). The shared throttle is `FRAME_RENDER_INTERVAL = 8ms` (`app.rs:71-72`).
- Backend stream events are drained in batches up to 128 (`backend_flow.rs:109-151`), then each `PiSessionEvent` mutates `chat_transcripts[streaming_node]`, updates status, and calls `request_event_render` (`backend_flow.rs:172-185`). `request_event_render` schedules a root `cx.notify()` after 8ms (`backend_flow.rs:254-264`).
- Root render rebuilds the app (`render.rs:6-24`). Workspace render scans all active nodes, reconciles input/subscriptions (`render.rs:120-189`), scans drawings for text boxes (`render.rs:191-251`), then calls `workspace_canvas(...)` with the full transcript map (`render.rs:271-293`). Pinned mode also rebuilds `pinned_panel_region` (`render.rs:295-323`).
- Canvas render composes grid, drawings, text boxes, marker layers, all visible unpinned nodes, minimap, and toolbar (`workspace_canvas.rs:168-229`). Visible nodes call `chat_node::chat_node(...)` with their transcript (`workspace_canvas.rs:191-215`). Pinned panels separately find pinned nodes and call `pinned_chat_node_panel` (`pinned_panels.rs:31-54`).
- Chat body obtains `entries`, `streaming`, `revision`, auto-scrolls on each revision, then renders every entry (`chat_node.rs:291-360`). Assistant entries call `TextView::markdown(..., content.to_owned(), ...)` every render (`chat_node.rs:431-459`).
- Transcript updates replace full assistant text: `message_update` calls `update_assistant_from_message` (`transcript.rs:149-157`), which computes `message_text`, compares whole strings, assigns the new whole string, and bumps `revision` (`transcript.rs:237-248`). `content_text` clones/joins text parts (`transcript.rs:339-378`); tool updates can pretty-print/truncate JSON (`transcript.rs:343-397`).

## Architecture
Current data flow for one streamed node:
1. `submit_chat_node` clears the input, pushes a user message, sets `streaming_node = Some(key)`, sets `pending = true`, and root-notifies (`chat_actions.rs:16-69`).
2. Backend bridge events arrive through the subscription loop, are batch-drained, and are applied to the single `streaming_node` transcript (`backend_flow.rs:109-185`).
3. A coalesced timer root-notifies the entire `PiDesktop` every ~8ms while events keep arriving (`backend_flow.rs:254-264`).
4. Root render rebuilds workspace UI, canvas layers, visible nodes, and pinned panels (`render.rs:94-333`; `workspace_canvas.rs:168-229`).
5. The streaming chat node renders the full transcript list and the growing assistant markdown body (`chat_node.rs:291-360`, `431-459`).

Hot paths / likely sluggishness causes:
- **Root-level invalidation for token updates.** A streaming token dirties `PiDesktop`, not a node-local view, so unchanged tabs/status/canvas/nodes/text boxes are re-evaluated (`backend_flow.rs:178-185`, `254-264`; `render.rs:120-293`).
- **Full markdown/text work per frame.** Each render passes the whole assistant text into `TextView::markdown`; each message update also reconstructs and compares the whole assistant string. This is likely O(total streamed text) per event/render, possibly O(n²) over a long response (`chat_node.rs:431-459`; `transcript.rs:237-248`, `339-378`).
- **All transcript entries render every time.** No transcript virtualization or stable per-message entities; historical entries and tool blocks are rebuilt during each root render (`chat_node.rs:347-357`, `476-530`).
- **Auto-scroll mutates keyed state during render.** `scroll_to_bottom` plus `scroll_revision.update` runs inside `render_body` on every transcript revision, adding layout/scroll work to the render path (`chat_node.rs:302-319`).
- **Canvas work happens even for chat-only updates.** `workspace_canvas` recreates grid/drawing/textbox/node children; drawings are filtered and cloned (`workspace_canvas.rs:234-269`), pen paths are rebuilt point-by-point during paint (`workspace_canvas.rs:386-429`), and grid lines are repainted (`workspace_canvas.rs:803-859`).
- **Event render requests are unconditional for session events.** `observe_session_event` returns no changed flag, and `request_event_render` is called for every `PiSessionEvent` while streaming, even if the event produced no transcript change (`backend_flow.rs:178-185`; `transcript.rs:78-97`).

## Start Here
Start with `crates/pi-desktop/src/app/backend_flow.rs`: it is the highest-leverage choke point because it turns backend stream events into root-wide 8ms invalidations. Then inspect `crates/pi-desktop/src/components/chat_node.rs` and `crates/pi-desktop/src/chat/transcript.rs` for the full-text markdown/render work.

## Refactor Recommendations
1. Split root state into smaller GPUI entities: `WorkspaceCanvasView`, `ChatNodeView`, and per-node `TranscriptState`. Backend events should update/notify only the affected transcript/node, not `PiDesktop`.
2. Make `observe_session_event` return a change/delta result; batch all envelopes first, then schedule one render only if visible transcript/status changed.
3. Cache or incrementally render assistant content: store chunks/rope or diff suffixes, render streaming text cheaply, and parse markdown on completion or at a lower cadence.
4. Move auto-scroll out of render into a post-update/effect path and throttle it.
5. Virtualize or key transcript entries so old messages/tools do not rebuild on every token.
6. Separate canvas layers from chat updates; cache drawing paths/visibility by drawing+viewport revision and avoid cloning visible drawings each render.
7. Consider raising stream UI cadence to 16-33ms after scoping invalidation; keep canvas interaction cadence separate.

Open questions: whether `TextView::markdown` internally caches by id/revision, and whether backend `message_update` can provide deltas instead of full message content.
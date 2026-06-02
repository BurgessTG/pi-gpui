# Code Context

## Files Retrieved
1. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/README.md` (lines 42-50) - GPUI’s intended registers: entities, views, elements.
2. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/docs/contexts.md` (lines 1-33) - App/Context/Entity/Window roles.
3. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/_ownership_and_data_flow.rs` (lines 1-72) - entity ownership/update/notify examples.
4. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/element.rs` (lines 1-32) - immediate element tree lifecycle and when custom elements are appropriate.
5. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/view.rs` (lines 99-294) - `AnyView::cached` mechanics and cache invalidation.
6. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/app.rs` (lines 753-833, 1198-1255, 2033-2054) - window invalidator tracking, flush effects, notify propagation.
7. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/window.rs` (lines 116-124, 1303-1318, 1366-1371, 1914-1995, 2577-2625) - dirty view marking, refresh, draw/access tracking, keyed state.
8. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/elements/animation.rs` (lines 158-178) and `window.rs` (lines 1648-1658) - animation frames notify the current view.
9. `/home/burgess/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/src/elements/canvas.rs` (lines 8-74) - `canvas` is low-level callback painting, no inherent caching/id.
10. `vendor/gpui-component/README.md` (lines 7-19) - component library claims virtualized list/table and markdown support.
11. `vendor/gpui-component/src/text/text_view.rs` (lines 401-517, 599-755) - markdown state uses `use_keyed_state`, debounced parse, parent notify, selection.
12. `vendor/gpui-component/src/scroll/scrollable.rs` (lines 111-130) - scroll wrappers use `use_keyed_state` for handles.
13. `vendor/gpui-component/src/virtual_list.rs` (lines 1-13, 116-145) - virtual list only renders visible ranges.
14. `vendor/gpui-component/src/resizable/panel.rs` (lines 126-160, 260-317, 377-419) and `vendor/gpui-component/src/resizable/mod.rs` (lines 95-142, 197-263, 265-285) - resizable group/state behavior.
15. `crates/pi-desktop/src/app.rs` (lines 72-138, 190-283, 293-319) - root entity fields, render throttle constants, child entities/maps, transcript notify helper.
16. `crates/pi-desktop/src/app/render.rs` (lines 6-39, 94-369, 394-464) - root render, UI state creation during render, canvas/pin layout, settings animations.
17. `crates/pi-desktop/src/components/workspace_canvas.rs` (lines 44-230, 233-277, 385-430, 616-759, 797-814) - canvas event handlers, custom paint, culling, text boxes, grid.
18. `crates/pi-desktop/src/components/chat_node.rs` (lines 24-140, 390-467, 537-565, 680-765) - chat body/message entities, cached body, scroll, markdown, repeated animations.
19. `crates/pi-desktop/src/app/canvas_actions.rs` (lines 177-232, 308-357) - high-frequency canvas/minimap/zoom updates use coalesced render requests.
20. `crates/pi-desktop/src/app/backend_flow.rs` (lines 110-203, 205-247, 286-314) - backend event batching and root/canvas render throttles.
21. `crates/pi-desktop/src/components/pinned_panels.rs` (lines 16-74) and `crates/pi-desktop/src/app/render.rs` (lines 304-369) - pinned panel composition and shell sizing.
22. `crates/pi-desktop/src/design/theme.rs` (lines 212-289) - component theme application and `window.refresh()` use.
23. `crates/pi-desktop/src/main.rs` (lines 48-60) - app/root construction; PiDesktop is passed to gpui-component `Root` uncached.

## Key Code

GPUI model:
```rust
// AnyView::cached: reuses prepaint/paint if bounds/mask/text style match,
// the view is not dirty, and window.refresh is not active.
// gpui view.rs:99-103, 208-223, 273-281
```
```rust
// notify: if a rendered window has accessed an entity, GPUI invalidates that
// entity; Window later marks the rendered view path dirty.
// gpui app.rs:2033-2054; window.rs:1303-1318
```
```rust
// use_keyed_state observes the created state entity and notifies current_view
// when that state notifies.
// gpui window.rs:2577-2598
```
```rust
// AnimationElement calls window.request_animation_frame(); that notifies the
// current view next frame.
// gpui elements/animation.rs:172-176; window.rs:1653-1657
```

Pi desktop hot spots:
```rust
// Root entity owns broad state + per-node child entities.
// crates/pi-desktop/src/app.rs:84-138
workspace_state: WorkspaceState,
pin_shell_state: Entity<ResizableState>,
chat_transcripts: HashMap<(usize, usize), Entity<ChatTranscript>>,
chat_body_views: HashMap<(usize, usize), Entity<chat_node::ChatBodyView>>,
```
```rust
// render_workspace_content creates/retains InputState, ChatTranscript, ChatBodyView
// while rendering, keyed by (workspace_id, node_id) or drawing index.
// crates/pi-desktop/src/app/render.rs:120-260
```
```rust
// Only chat body is cached today.
// crates/pi-desktop/src/components/chat_node.rs:405-425
.child(AnyView::from(body_view).cached(StyleRefinement::default().size_full()))
```
```rust
// ChatBodyView observes transcript, syncs message view entities, then notifies itself.
// crates/pi-desktop/src/components/chat_node.rs:75-117
```
```rust
// Canvas custom paint clones visible drawings and rebuilds paths on paint.
// crates/pi-desktop/src/components/workspace_canvas.rs:233-277, 385-430
```
```rust
// High-frequency canvas motion mutates WorkspaceState, then schedules one root notify after 8ms.
// crates/pi-desktop/src/app/canvas_actions.rs:177-232; backend_flow.rs:301-314
```

## Architecture

Current architecture is **partially aligned** with GPUI’s performance model:

- Good alignment: persistent state lives in entities where it matters (`InputState`, `ResizableState`, `ChatTranscript`, `ChatBodyView`, `ChatMessageView`); backend events are batched; canvas drag repaint is coalesced; markdown uses gpui-component `TextView` with stable IDs and debounced parsing; `window.refresh()` appears limited to global theme changes.
- Main mismatch: `PiDesktop` is a large root view owning `WorkspaceState` by value. Canvas/node/pin mutations generally dirty `PiDesktop`, so immediate canvas layers, node shells, minimap, toolbar, and status all rebuild together. The single `AnyView::cached` chat body protects message content from unrelated root notifies, but the canvas side is still mostly immediate-mode every redraw.
- `AnyView::cached` is used correctly for `ChatBodyView` because the wrapper is full-size and the cache style is `size_full`. It will skip transcript body layout/paint when root state changes but transcript/body did not. It does not help the outer chat node, canvas grid, drawings, minimap, or repeated loading/working animations.
- Notify propagation is mostly sound. `update_chat_transcript` only notifies when transcript revision changes, and `ChatBodyView` observes it. One risk: `apply_session_events` updates `self.status` without directly notifying `PiDesktop`; it relies on transcript/body invalidation to repaint. If status changes but transcript revision does not, status may not update promptly.
- Animations are featureful but broad. Repeated chat loading bars/dots are rendered inside the `PiDesktop` view path, so their `request_animation_frame` likely notifies `PiDesktop` every frame while visible. Short drawer/tool fade animations are acceptable; repeated streaming indicators can drive full root/canvas rebuilds unless more subtrees are cached/split.
- Scroll/markdown quality is good for modest transcript sizes. `TextView::markdown` uses keyed state and background debounce; selection is preserved/handled by TextView. The chat transcript itself is a plain `v_flex` of all message views inside a scroll div, not a virtual list.
- Resizable panels are aligned with gpui-component design: explicit `ResizableState` entities are supplied, and the component also supports `use_keyed_state` fallback. Current pinned panel sizes are index-based, not node-ID-based, so swaps/unpins preserve slot sizes rather than per-node sizes.
- Custom `canvas` usage is appropriate for short-term/simple paint, but for many drawings it becomes the main scaling risk: visible drawings are cloned and paths rebuilt every paint, and background grids repaint on unrelated invalidations.

Concrete improvements that keep quality/features/animations:

1. **Split heavy UI into cached view entities.** Add `WorkspaceCanvasView` / `ChatNodeView` / maybe `CanvasLayerView` entities and render them through `AnyView::cached` with exact outer layout styles. Keep the root as coordinator; make subviews observe or receive narrowly-scoped state/revisions.
2. **Cache message rows too.** Render `ChatMessageView` entities through `AnyView::cached` inside `ChatBodyView`, so a streaming assistant update does not re-layout unchanged prior messages. Keep `TextView::markdown` and selection.
3. **Virtualize long transcripts when needed.** For high message counts, replace the all-children `v_flex` with gpui/gpui-component list/virtual list while retaining stable message entities and bottom-scroll behavior. `virtual_list.rs` is designed for visible-range rendering.
4. **Move canvas state toward entity/revision granularity.** Make active canvas/tab state an entity or maintain explicit `viewport_revision`, `drawings_revision`, `nodes_revision`, `selection_revision`. Notify only the layer/view whose inputs changed.
5. **Cache/precompute drawing geometry.** Store world-space drawing bounds and maybe path data keyed by drawing revision/stroke/tool; rebuild only changed drawings or when viewport/stroke changes. For very large drawing sets, implement a custom `Element` with `with_element_state` instead of plain callback `canvas`.
6. **Use frame scheduling instead of fixed timer where practical.** `request_canvas_render` is a good coalescer, but an 8ms `Timer` is detached from the actual window frame. Passing `window` through drag/zoom updates and using `window.request_animation_frame()`/`on_next_frame` for canvas repaint would align with GPUI’s frame model.
7. **Isolate repeated indicators.** Keep loading bars/dots, but put long-running repeat animations in small child views and cache unaffected siblings. This reduces repeated full canvas/chat rebuilds during streaming.
8. **Consider status as its own entity.** Status changes are frequent and low-cost but currently dirty root; a `StatusBarView`/status entity would avoid root invalidation and fix the `apply_session_events` status-notify edge case.
9. **Refine pinned resizable state if UX requires.** Persist sizes by node ID or panel ID if swaps/unpins should carry size with content; add explicit `size_range` constraints for pinned shell/panels. Current index-based `ResizableState` is acceptable but not semantic.
10. **Avoid adding more `window.refresh()`.** Current theme refresh is appropriate because global theme functions are not entity-tracked and refresh intentionally bypasses caches. Normal state changes should remain entity `notify`-driven.

## Start Here
Open `crates/pi-desktop/src/components/chat_node.rs` first. It shows the best-aligned pattern already present: model entity (`ChatTranscript`) -> observed view entity (`ChatBodyView`) -> cached `AnyView` -> markdown `TextView`. Use that pattern as the template for canvas layers and chat node shells before touching lower-level paint code.

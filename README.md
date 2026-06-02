# Pi Workspaces

Pi Workspaces is a native desktop workspace for Pi. It turns Pi coding-agent sessions into live, draggable chat nodes on a high-performance GPUI canvas so multiple sessions can stay visible, active, and organized side by side.

The app is built for a desktop-first workflow: open a project folder, create or resume Pi sessions as canvas nodes, stream responses with inline tool activity, sketch/annotate the workspace, and pin important sessions into resizable panels without leaving the native UI.

## What this repository contains

- **Native GPUI desktop app** — `crates/pi-desktop` is the Rust/GPUI frontend.
- **Embedded Pi runtime** — the app embeds Node/libnode through the workspace-owned `vendor/edon` fork and hosts `@earendil-works/pi-coding-agent` directly.
- **Typed Rust ↔ Node bridge** — `crates/pi-bridge-types`, `crates/pi-sdk-bridge`, and `node/src` define and implement the protocol between GPUI and the embedded Pi SDK runtime.
- **Canvas workspace model** — workspaces, tabs, session nodes, drawings, minimap, pinned panels, and stable UI state live under `crates/pi-desktop/src/workspace` and `crates/pi-desktop/src/components`.
- **Protocol/codegen checks** — TypeScript bindings are generated from Rust bridge types and verified in CI-style local checks.

Stock Pi RPC sidecar mode is intentionally not used in authored source; Pi Workspaces runs the SDK directly inside the embedded runtime.

## Current capabilities

- Folder-backed and blank Pi workspaces.
- Workspace tabs with stable workspace IDs.
- Infinite-canvas style GPUI workspace with pan, zoom, minimap, grid, drawing tools, text boxes, and markers.
- Pi session nodes for:
  - new sessions
  - forked sessions
  - resumed/switched sessions
- Full-quality live chat nodes with:
  - editable session titles
  - streaming assistant responses
  - markdown rendering
  - inline tool cards
  - composer input
  - stable transcript/body view state
- World-space zoom: at 100% canvas objects render at normal size; zooming out/in scales visible nodes, drawings, text boxes, and markers like an infinite workspace.
- Hyprland-style pin workflow:
  - pin/unpin nodes from the canvas
  - pinned canvas markers
  - resizable pinned panel region
  - focused pinned panel state
  - keyboard panel controls
- Provider authentication and settings UI.
- Package search/install/remove UI for Pi packages.
- Theme/appearance settings.

## Performance design

Pi Workspaces is optimized around keeping visible nodes active and high quality without making one busy node slow down the whole canvas.

Important design points:

- Streaming events update node-local transcript entities instead of forcing root workspace rerenders.
- Transcript bodies and message rows are stable GPUI entities.
- Backend session events are batched before transcript mutation.
- Canvas zoom scales canvas objects consistently while pinned panels remain full-size UI.
- Canvas rendering uses culling for drawings, text boxes, markers, and nodes.
- Grid rendering is painted instead of built from many UI elements.
- Pinned panels reuse the same full-quality chat node body path.

## Repository layout

```text
crates/
  pi-desktop/        Native GPUI desktop application
  pi-bridge-types/   Shared protocol types and TS binding export tests
  pi-sdk-bridge/     Rust client for the embedded SDK bridge
  pi-node-host/      Embedded Node/libnode host integration
  pi-core/           Core reducer/state helpers
  pi-edon/           Edon/libnode integration wrapper
node/
  src/               Embedded TypeScript runtime and bridge dispatcher
  test/              Node bridge/runtime tests
vendor/
  edon/              Workspace-owned edon fork
  gpui-component/    Patched gpui-component dependency
scripts/             Protocol sync and libnode helper scripts
assets/, icons/      App icons, provider logos, and UI assets
```

## Requirements

- Rust toolchain from `rust-toolchain.toml`.
- Node.js `>=22.19.0`.
- npm.
- Linux desktop environment supported by GPUI.
- Network access for first-time dependency/libnode fetches.

## Setup

Install Node dependencies and build the embedded runtime:

```bash
cd node
npm install
npm run build
cd ..
```

Fetch libnode:

```bash
./scripts/fetch-libnode.sh .
```

By default this downloads libnode to:

```text
.libnode/v24.4.1/libnode.so
```

## Run the desktop app

```bash
PI_GPUI_LIBNODE=$PWD/.libnode/v24.4.1/libnode.so cargo run -p pi-desktop
```

Useful runtime overrides:

- `PI_GPUI_BOOTSTRAP` — path to `node/dist/bootstrap.js`.
- `PI_GPUI_LIBNODE` — path to `libnode.so`, `libnode.dylib`, or `libnode.dll`.
- `EDON_LIBNODE_PATH` — alternative libnode path consumed by edon.
- `PI_GPUI_LIBNODE_VERSION` — version used by `scripts/fetch-libnode.sh`.
- `PI_GPUI_LIBNODE_PLATFORM` — platform archive used by `scripts/fetch-libnode.sh`.

## Development checks

Run Rust checks:

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run Node/protocol checks:

```bash
cd node
npm run typecheck
npm test
npm run build
npm run check-protocol
```

If Rust protocol types change, regenerate TypeScript bindings with:

```bash
cd node
npm run sync-protocol
```

## Project status

Pi Workspaces is an active native frontend for Pi. The current focus is a fast, full-quality GPUI canvas/node workflow where many sessions can remain visible and active while preserving smooth pan, zoom, streaming, resizing, and pinned-panel interactions.

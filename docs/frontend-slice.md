# First GPUI frontend slice

The first native frontend is `crates/pi-desktop`.

## Included

- Native GPUI app window using `gpui` and `gpui-component`.
- Official Pi SVG mark from `pi.dev/logo-auto.svg` stored at `assets/pi-logo-auto.svg`.
- Fresh-load workspace landing screen with centered Pi mark and a single `Open Workspace` action.
- `Open Workspace` flow with an internal tree-based folder picker.
- Workspace tabs for folder-backed workspaces.
- Workspace canvas with a plain grid, left-drag panning, mouse-wheel zoom, bottom-left minimap, floating square +/- zoom controls, and floating zoom percentage.
- Minimap supports node symbols plus click-to-jump and frame-drag viewport control.
- GPUI Component right-click context menu for session-node creation.
- Session nodes are backed only by Pi primitives: `NewSession`, `Fork`, and resume via `SwitchSession`.
- Session nodes render as large, solid draggable chat panels with saved-title headers, Idle/Working status tags, Pi-backed composers, scrollable response streams, markdown assistant output, and inline tool usage cards.
- Compact bottom segmented dock with a single Settings icon aligned to the right.
- Settings screen for provider auth flow:
  - provider picker with auth status and auth source labels
  - masked API-key input
  - save persisted API key
  - use runtime-only API key
  - remove provider auth
  - model selector for authenticated/available models
  - effort selector backed by Pi thinking levels
- Embedded backend startup from the desktop app without stock Pi RPC.
- Typed Rust bridge helpers for Pi session lifecycle commands.

## Runtime paths

By default, the desktop app looks for:

- `node/dist/bootstrap.js`
- `.libnode/v24.4.1/libnode.so`

These can be overridden with:

- `PI_GPUI_BOOTSTRAP`
- `PI_GPUI_LIBNODE` or `EDON_LIBNODE_PATH`

## Run

```bash
npm --prefix node run build
./scripts/fetch-libnode.sh .
PI_GPUI_LIBNODE=$PWD/.libnode/v24.4.1/libnode.so cargo run -p pi-desktop
```

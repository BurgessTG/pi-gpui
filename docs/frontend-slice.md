# First GPUI frontend slice

The first native frontend is `crates/pi-desktop`.

## Included

- Native GPUI app window using `gpui` and `gpui-component`.
- Official Pi SVG mark from `pi.dev/logo-auto.svg` stored at `assets/pi-logo-auto.svg`.
- Landing screen with the Pi mark centered slightly above the horizontal median.
- Slogan: “There are many agent ochestrators, but this one is yours.”
- Bottom chat bar with multi-line input and send action.
- Settings screen for provider auth flow:
  - provider picker with auth status and auth source labels
  - masked API-key input
  - save persisted API key
  - use runtime-only API key
  - remove provider auth
  - model selector for authenticated/available models
  - effort selector backed by Pi thinking levels
- Embedded backend startup from the desktop app without stock Pi RPC.

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

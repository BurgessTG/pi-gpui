# Backend foundation status

This workspace was built backend-first. The embedded Pi runtime, bridge protocol,
and state reducers are now stable enough that the first GPUI frontend slice lives
in `crates/pi-desktop`.

## Architecture

- `vendor/edon`: workspace-owned fork of edon/libnode embedding code, kept
  formatted and warning-free under normal and all-feature Rust gates.
- `pi-edon`: safe boundary around the vendored edon/libnode crate. All embedded
  Node loading and N-API module registration enters through this crate.
- `pi-node-host`: async Rust host for the embedded Node runtime. It loads the JS
  bootstrap, exposes a native module, dispatches typed bridge commands, and
  streams typed events back to Rust.
- `node/src`: TypeScript bootstrap that imports `@earendil-works/pi-coding-agent`
  directly and owns `AgentSessionRuntime` lifecycle.
- `pi-bridge-types`: versioned Rust protocol for commands, responses, events,
  state snapshots, and extension UI requests.
- `pi-sdk-bridge`: ergonomic Rust client over the typed protocol.
- `pi-core`: GPUI-independent reducers/state for later frontend rendering.

Stock Pi RPC is not used. The forbidden-RPC gate checks for RPC mode symbols.

## Verified backend capabilities

- Embedded libnode smoke tests.
- Native Rust callback from JavaScript through N-API.
- Embedded Node loads the TypeScript-compiled bootstrap.
- Pi SDK runtime initializes with `createAgentSessionRuntime` and faux provider.
- Prompt command streams through the real Pi SDK and persists state.
- Provider auth is part of the bridge: status queries, runtime API-key injection,
  persisted API-key storage, removal, model listing, and model selection all use
  typed Rust-generated commands/responses.
- Core command surface is covered with embedded tests: provider auth/model
  controls, thinking/tool controls, queue modes, editor text, theme state, bash
  execution, compaction controls, exports, messages, stats, aborts, session
  import/switch/fork/tree navigation, shutdown, and post-shutdown errors.
- Extension UI backend is covered through a real loaded Pi extension: updates,
  select/confirm/input/editor/custom dialogs, terminal input, autocomplete, and
  JS-owned component render/input all round-trip through embedded Node.
- Protocol TypeScript is generated from Rust `ts-rs` bindings and checked for
  drift by `scripts/sync-protocol.sh --check`.
- The real-provider harness is not ignored. It no-ops only when live-provider
  env is absent. When `PI_GPUI_REAL_PROVIDER` and `PI_GPUI_REAL_MODEL` are set,
  missing provider auth is a test failure; otherwise it initializes the real
  model, checks auth status, optionally injects `PI_GPUI_REAL_API_KEY`, sends a
  real prompt, and verifies a real response. `PI_GPUI_REAL_PROMPT` and
  `PI_GPUI_REAL_EXPECT` may override the default live prompt/assertion.
- Core reducers consume backend events without depending on GPUI.

## Validation gates

Run from the workspace root:

```bash
npm --prefix node install
npm --prefix node run build
./scripts/fetch-libnode.sh .
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features
npm --prefix node run typecheck
npm --prefix node test
PI_GPUI_LIBNODE=$PWD/.libnode/v24.4.1/libnode.so \
EDON_LIBNODE_PATH=$PWD/.libnode/v24.4.1/libnode.so \
  cargo test --workspace --all-features -- --test-threads=1
npm --prefix node run check-protocol
./scripts/check-loc.sh .
./scripts/forbid-stock-pi-rpc.sh .
```

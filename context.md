# Code Context

## Files Retrieved
1. `crates/pi-desktop/src/ui.rs` (lines 24-45) - provider env hint and current provider-logo slug/file mapping.
2. `crates/pi-desktop/src/ui.rs` (lines 47-59) - initials fallback for providers without a logo.
3. `crates/pi-desktop/src/app.rs` (lines 431-527) - auth settings renders provider icon grid and calls `provider_logo`.
4. `crates/pi-desktop/src/app.rs` (lines 528-741) - selected provider auth flow, browser-auth special cases, and `provider_logo` SVG/fallback logic.
5. `crates/pi-desktop/src/backend.rs` (lines 13-18, 27-45, 80-83) - desktop initializes Node backend and loads auth statuses shown in UI.
6. `crates/pi-bridge-types/src/state.rs` (lines 19-26, 95-101) - Rust bridge types carrying provider slugs/display names.
7. `node/src/pi/runtime.ts` (lines 480-541, 640-687, 704-717) - Node backend creates `ModelRegistry`, derives auth provider list from models, and maps model descriptors.
8. `node/node_modules/@earendil-works/pi-coding-agent/dist/core/model-registry.js` (lines 344-360, 383-521, 626-635) - SDK model registry pulls built-ins from pi-ai, accepts custom providers, and resolves display names.
9. `node/node_modules/@earendil-works/pi-coding-agent/node_modules/@earendil-works/pi-ai/dist/models.js` (lines 1-18) - `getProviders()` returns keys of generated `MODELS`.
10. `node/node_modules/@earendil-works/pi-coding-agent/node_modules/@earendil-works/pi-ai/dist/models.generated.js` (lines 1-20, 16220-16251; first provider occurrences listed below) - generated built-in provider/model source.
11. `node/node_modules/@earendil-works/pi-coding-agent/dist/core/provider-display-names.js` (lines 1-32) - SDK built-in display names for most providers.
12. `node/node_modules/@earendil-works/pi-coding-agent/node_modules/@earendil-works/pi-ai/dist/utils/oauth/index.js` (lines 20-68), `.../github-copilot.js` (lines 240-268), `.../openai-codex.js` (lines 444-459) - OAuth providers add display names for `github-copilot` and `openai-codex`.
13. `crates/pi-sdk-bridge/tests/mock_transport.rs` (lines 73-99) - bridge auth command test uses `openai` only.
14. `crates/pi-node-host/tests/embedded_pi.rs` (lines 103-214, 725-805) - embedded tests cover auth/status for faux/openai and optional real provider.
15. `Cargo.toml` (lines 1-48), `node/package.json` (lines 1-24), `xtask/src/main.rs` (lines 6-35) - validation command entry points.

## Key Code

### Runtime provider slugs
The desktop does not hard-code the provider list. It asks `BackendSession::collect_data()` for `client.auth_status(None)` (`crates/pi-desktop/src/backend.rs:80-83`). Node handles that with:

```ts
// node/src/pi/runtime.ts:640-657
const providers = provider
  ? [provider]
  : Array.from(new Set((this.modelRegistry?.getAll() ?? []).map((model) => model.provider))).sort();
```

`ModelRegistry` loads built-ins from `@earendil-works/pi-ai` and merges custom `models.json` providers (`model-registry.js:344-360`, `383-521`). Therefore custom providers may have arbitrary slugs and will fall back to initials unless mapped.

Current built-in SDK slugs from `getProviders()` (verified with Node import of `pi-ai/dist/models.js`):

- `amazon-bedrock` (first generated occurrence line 9)
- `anthropic` (1557)
- `azure-openai-responses` (1974)
- `cerebras` (2714)
- `cloudflare-ai-gateway` (2767)
- `cloudflare-workers-ai` (3380)
- `deepseek` (3598)
- `fireworks` (3638)
- `github-copilot` (3856)
- `google` (4259)
- `google-vertex` (4542)
- `groq` (4769)
- `huggingface` (5078)
- `kimi-coding` (5476)
- `minimax` (5514)
- `minimax-cn` (5550)
- `mistral` (5586)
- `moonshotai` (6064)
- `moonshotai-cn` (6192)
- `openai` (6320)
- `openai-codex` (7060)
- `opencode` (7170)
- `opencode-go` (7900)
- `openrouter` (8113)
- `together` (12664)
- `vercel-ai-gateway` (13005)
- `xai` (15729)
- `xiaomi` (15850)
- `xiaomi-token-plan-ams` (15942)
- `xiaomi-token-plan-cn` (16016)
- `xiaomi-token-plan-sgp` (16090)
- `zai` (16164)

### Current logo mapping
`crates/pi-desktop/src/ui.rs:31-45`:

```rust
pub fn provider_logo_path(provider: &str) -> Option<String> {
    let file = match provider {
        "anthropic" => "anthropic.svg",
        "openai" | "azure-openai-responses" => "openai.svg",
        "amazon-bedrock" => "aws.svg",
        "cerebras" => "cerebras.svg",
        "cloudflare-ai-gateway" | "cloudflare-workers-ai" => "cloudflare.svg",
        "deepseek" => "deepseek.svg",
        "fireworks" => "fireworks.svg",
        "github-copilot" => "github.svg",
        _ => return None,
    };
    let path = workspace_root().join("assets/provider-logos").join(file);
    path.exists().then(|| path.display().to_string())
}
```

`provider_logo` only renders SVG when that path exists; otherwise it renders initials (`crates/pi-desktop/src/app.rs:722-741`).

### Current asset coverage
Actual files in `assets/provider-logos/`:

- `anthropic.svg`
- `cloudflare.svg`
- `deepseek.svg`
- `github.svg`

Mapped and working now:
- `anthropic` -> `anthropic.svg`
- `cloudflare-ai-gateway`, `cloudflare-workers-ai` -> `cloudflare.svg`
- `deepseek` -> `deepseek.svg`
- `github-copilot` -> `github.svg`

Mapped but currently missing asset, so UI falls back to initials:
- `openai`, `azure-openai-responses` -> missing `openai.svg`
- `amazon-bedrock` -> missing `aws.svg`
- `cerebras` -> missing `cerebras.svg`
- `fireworks` -> missing `fireworks.svg`

Built-in SDK slugs with no mapping at all:
- `google`, `google-vertex`, `groq`, `huggingface`, `kimi-coding`, `minimax`, `minimax-cn`, `mistral`, `moonshotai`, `moonshotai-cn`, `openai-codex`, `opencode`, `opencode-go`, `openrouter`, `together`, `vercel-ai-gateway`, `xai`, `xiaomi`, `xiaomi-token-plan-ams`, `xiaomi-token-plan-cn`, `xiaomi-token-plan-sgp`, `zai`.

### Existing tests / gaps
- No direct tests for `provider_logo_path`, logo asset existence, or SDK slug coverage were found.
- Existing auth bridge tests only exercise `openai` auth commands (`crates/pi-sdk-bridge/tests/mock_transport.rs:73-99`) and embedded provider auth/status (`crates/pi-node-host/tests/embedded_pi.rs:103-214`, `725-805`).
- `provider_supports_browser_auth` only matches `openai` and `github-copilot` (`crates/pi-desktop/src/app.rs:672-689`); `openai-codex` has SDK OAuth/display-name support but no desktop browser-auth special case.

## Architecture
Desktop UI receives `Vec<ProviderAuthStatus>` from Rust backend. Rust backend calls bridge `auth_status(None)`. Node runtime builds statuses from unique `model.provider` values in SDK `ModelRegistry.getAll()`, plus display names from registered providers/OAuth providers/built-in display-name table. `provider_logo_path` is purely desktop-side and maps slug -> SVG filename, but it suppresses mapped logos when the file is absent. Unmapped or absent assets render initials using SDK display name.

## Start Here
Open `crates/pi-desktop/src/ui.rs` first. It contains the only provider slug -> logo file mapping, and any completion work should align it with the 32 built-in SDK provider slugs plus current `assets/provider-logos/` files.

## Validation commands
Focused/available:
- `cargo check -p pi-desktop --all-targets`
- `cargo test -p pi-sdk-bridge`
- `cargo test -p pi-node-host --test embedded_pi` (requires libnode/bootstrap; some tests skip if config missing)
- `npm --prefix node run typecheck`
- `npm --prefix node run build`
- `npm --prefix node test`

Full existing CI entry point:
- `cargo run -p xtask -- ci`

Useful slug audit command used during recon:
- `node --input-type=module -e "import { getProviders } from './node/node_modules/@earendil-works/pi-coding-agent/node_modules/@earendil-works/pi-ai/dist/models.js'; console.log(getProviders().sort().join('\\n'))"`

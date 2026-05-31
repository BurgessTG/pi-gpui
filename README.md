# pi-gpui native frontend

Backend-first foundation plus the first native GPUI frontend slice for Pi.

The architecture embeds Node/libnode via the workspace-owned `vendor/edon` fork
and hosts `@earendil-works/pi-coding-agent` directly in that embedded runtime.
Stock Pi RPC sidecar mode is forbidden in authored source.

The backend crates expose a stable typed contract for the GPUI shell, with
embedded faux-provider, provider-auth/model-selection, real-provider-gated, and
extension-UI integration tests proving the Rust ↔ Node ↔ Pi SDK path.

The first desktop crate is `crates/pi-desktop`. It opens a native GPUI window,
starts the embedded Pi backend, renders the centered Pi landing mark and slogan,
and provides a settings flow for provider auth, model selection, effort selection,
and the first chat-bar prompt path.

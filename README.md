# pi-gpui native frontend

Backend-first foundation plus the first native GPUI frontend slice for Pi.

The architecture embeds Node/libnode via the workspace-owned `vendor/edon` fork
and hosts `@earendil-works/pi-coding-agent` directly in that embedded runtime.
Stock Pi RPC sidecar mode is forbidden in authored source.

The backend crates expose a stable typed contract for the GPUI shell, with
embedded faux-provider, provider-auth/model-selection, real-provider-gated, and
extension-UI integration tests proving the Rust ↔ Node ↔ Pi SDK path.



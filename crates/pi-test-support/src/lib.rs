#![allow(clippy::module_name_repetitions)]

use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub fn node_bootstrap_dist() -> PathBuf {
    workspace_root().join("node/dist/bootstrap.js")
}

pub fn optional_libnode_path() -> Option<PathBuf> {
    std::env::var_os("PI_GPUI_LIBNODE")
        .or_else(|| std::env::var_os("EDON_LIBNODE_PATH"))
        .map(PathBuf::from)
}

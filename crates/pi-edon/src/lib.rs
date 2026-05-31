#![allow(clippy::module_name_repetitions)]

mod error;
mod runtime;

pub use edon::napi;
pub use error::{EdonBoundaryError, Result};
pub use runtime::{EmbeddedNode, EmbeddedNodeConfig};

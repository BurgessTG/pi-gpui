#![allow(clippy::module_name_repetitions)]

mod client;
mod error;
mod transport;

pub use client::BridgeClient;
pub use error::{BridgeClientError, Result};
pub use transport::{
    BridgeTransport, NodeHostTransport, NodeProcessTransport, NodeWorkerPoolTransport,
};

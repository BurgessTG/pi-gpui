#![allow(clippy::module_name_repetitions)]

mod reducer;
mod state;

pub use reducer::{ApplyEvent, ReducerError};
pub use state::{BackendState, TranscriptItem};

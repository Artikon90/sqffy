//! Autocomplete engine + schema cache.
//!
//! Clause/scope scanning lives in [`context`]; ranking in [`engine`].

mod cache;
mod context;
mod engine;

pub use cache::SchemaCache;
pub use engine::Completer;

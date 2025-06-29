//! Read-only MCP server (stdio standalone + HTTP alongside REPL).

mod guard;
mod protocol;
mod server;
mod tools;

pub use server::{run_http_background, run_stdio};
pub use tools::ToolContext;

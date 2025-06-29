//! SQffy — fast SQL CLI (sqf).

pub mod cli;
pub mod complete;
pub mod config;
pub mod connections;
pub mod db;
pub mod error;
pub mod generate;
pub mod i18n;
pub mod mcp;
pub mod meta;
pub mod picker;
pub mod pool;
pub mod provision;
pub mod render;
pub mod repl;
pub mod sql;
pub mod ui;

pub use cli::Cli;
pub use config::Config;
pub use db::{ConnectionSpec, Database, Driver, DriverRegistry};
pub use error::{Error, Result};

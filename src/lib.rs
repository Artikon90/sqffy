//! SQffy — fast SQL CLI (`sqf`).

pub mod cli;
pub mod complete;
pub mod config;
pub mod db;
pub mod error;
pub mod i18n;
pub mod meta;
pub mod render;
pub mod sql;
pub mod ui;
pub use cli::Cli;
pub use config::Config;
pub use db::{ConnectionSpec, Database, Driver, DriverRegistry};
pub use error::{Error, Result};

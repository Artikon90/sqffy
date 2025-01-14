use clap::{Parser, Subcommand};

/// SQffy — fast SQL CLI with saved connections (`sqf`).
#[derive(Debug, Parser)]
#[command(name = "sqf", version, about = "SQffy — fast SQL CLI with saved connections", long_about = None)]
pub struct Cli {
    /// SQLite path or connection URL (skips the interactive picker).
    pub target: Option<String>,

    /// Force driver id (`sqlite`, `postgres`, `mysql`).
    #[arg(long, short = 'd', value_name = "NAME")]
    pub driver: Option<String>,

    /// List built-in drivers and exit.
    #[arg(long, default_value_t = false)]
    pub list_drivers: bool,

    /// UI language (`en`, `ru`). Overrides system locale / SQFFY_LANG.
    #[arg(long, value_name = "LANG")]
    pub lang: Option<String>,

    /// Max rows to print for query results (0 = unlimited).
    #[arg(long, default_value_t = 100)]
    pub limit: usize,

    /// Disable the built-in pager.
    #[arg(long, default_value_t = false)]
    pub no_pager: bool,

    /// Execute SQL / meta commands and exit (non-interactive).
    #[arg(short = 'c', long = "command")]
    pub command: Option<String>,

    /// Skip saving a direct CLI connection into the catalog.
    #[arg(long, default_value_t = false)]
    pub no_save: bool,

    /// Start read-only MCP HTTP server on 127.0.0.1:PORT alongside the REPL.
    #[arg(long, value_name = "PORT")]
    pub mcp_port: Option<u16>,

    #[command(subcommand)]
    pub subcommand: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run MCP server only (stdio by default; use --port for HTTP).
    Mcp {
        /// Listen with HTTP JSON-RPC on 127.0.0.1:PORT instead of stdio.
        #[arg(long, value_name = "PORT")]
        port: Option<u16>,
    },
}

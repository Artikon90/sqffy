use clap::Parser;
use sqffy::cli::Cli;
use sqffy::{app, error::Result};

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Err(err) = app::run(cli) {
        app::open_error(err);
    }
    Ok(())
}

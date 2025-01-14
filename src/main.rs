use clap::Parser;
use sqffy::cli::Cli;

fn main() {
    let cli = Cli::parse();
    if cli.list_drivers {
        println!("(drivers arrive in a later milestone)");
        return;
    }
    println!("SQffy scaffold — pass a target once the database layer lands.");
}

use anyhow::Result;
use clap::Parser;
use gmap::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.execute()
}

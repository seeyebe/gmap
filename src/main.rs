use anyhow::Result;
use gmap::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.execute()
}
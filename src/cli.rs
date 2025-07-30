use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gmap")]
#[command(about = "Git repository analysis tool for churn, heatmap, and exports")]
#[command(version)]
pub struct Cli {
    #[clap(flatten)]
    pub common: CommonArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Clone)]
pub struct CommonArgs {
    #[arg(long, help = "Path to git repository")]
    pub repo: Option<PathBuf>,

    #[arg(long, help = "Path to cache database")]
    pub cache: Option<PathBuf>,

    #[arg(long, help = "Include merge commits", default_value_t = true)]
    pub include_merges: bool,

    #[arg(long, help = "Include binary files", default_value_t = false)]
    pub binary: bool,

    #[arg(long, help = "Start from this commit or date (RFC3339, YYYY-MM-DD, or natural language)")]
    pub since: Option<String>,

    #[arg(long, help = "End at this commit or date (RFC3339, YYYY-MM-DD, or natural language)")]
    pub until: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Churn {
        #[arg(long, help = "Output as JSON")]
        json: bool,

        #[arg(long, help = "Output as NDJSON")]
        ndjson: bool,

        #[arg(long, help = "Directory depth for aggregation")]
        depth: Option<u32>,

        #[arg(help = "Path prefix to analyze")]
        path: Option<String>,
    },
    Heat {
        #[arg(long, help = "Output as JSON")]
        json: bool,

        #[arg(long, help = "Output as NDJSON")]
        ndjson: bool,

        #[arg(long = "interactive", alias = "tui", alias = "ui", help = "Enable interactive terminal UI")]
        interactive: bool,

        #[arg(help = "Path prefix to analyze")]
        path: Option<String>,
    },
    Export {
        #[arg(long, help = "Output as JSON")]
        json: bool,

        #[arg(long, help = "Output as NDJSON")]
        ndjson: bool,
    },
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    pub fn execute(self) -> Result<()> {
        match self.command {
            Commands::Churn { json, ndjson, depth, path } => {
                crate::churn::exec(self.common, depth, json, ndjson, path)
            }
            Commands::Heat { json, ndjson, interactive, path } => {
                if interactive {
                    crate::tui::run(&self.common, path).map_err(|e| anyhow::anyhow!(e))
                } else {
                    crate::heat::exec(self.common, json, ndjson, path)
                }
            }
            Commands::Export { json, ndjson } => {
                crate::export::exec(self.common, json, ndjson)
            }
        }
    }
}

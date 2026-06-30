use clap::Parser;
use std::path::PathBuf;

/// Generate agent skills from crate documentation.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CliArgs {
    /// Crate to fetch, e.g. `clap` or `clap@4.5`. Omit to search interactively.
    pub crate_spec: Option<String>,

    /// Override the base output path (default: .agents/skills)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

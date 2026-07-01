use clap::Parser;
use std::path::PathBuf;

/// Generate agent skills from crate documentation.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CliArgs {
    /// Crate to fetch, e.g. `clap` or `clap@4.5`. Omit to search interactively.
    pub crate_spec: Option<String>,

    /// Override the base output path
    #[arg(short, long, default_value = ".agents/skills")]
    pub output: PathBuf,

    /// Count tokens in generated skill files instead of generating
    #[arg(long)]
    pub count: bool,
}

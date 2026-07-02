use clap::Parser;
use std::path::PathBuf;

/// Generate agent skills from package documentation.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CliArgs {
    /// Package to fetch, e.g. `clap` or `clap@4.5`. Omit to search interactively.
    pub spec: Option<String>,

    /// Override the base output path
    #[arg(short, long, default_value = ".agents/skills")]
    pub output: PathBuf,

    /// Count tokens in generated skill files instead of generating
    #[arg(long)]
    pub count: bool,

    /// Language to generate documentation for. Prompted interactively if not set.
    #[arg(short, long)]
    pub language: Option<Language>,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    #[cfg(feature = "rust")]
    Rust,
    #[cfg(feature = "csharp")]
    Csharp,
}

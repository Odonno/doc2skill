use clap::Parser;
use color_eyre::Result;

/// Generate agent skills from crate documentation.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Crate to fetch, e.g. `clap` or `clap@4.5`
    crate_spec: String,

    /// Override the base output path (default: .agents/skills)
    #[arg(short, long)]
    output: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let (name, version) = parse_spec(&args.crate_spec);
    println!("crate: {name}, version: {version:?}");

    let base = args
        .output
        .unwrap_or_else(|| std::path::PathBuf::from(".agents/skills"));
    println!("output: {}", base.display());

    Ok(())
}

fn parse_spec(spec: &str) -> (&str, Option<&str>) {
    match spec.split_once('@') {
        Some((name, ver)) => (name, Some(ver)),
        None => (spec, None),
    }
}

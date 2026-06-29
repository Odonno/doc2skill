mod fetch;
mod search;
mod write;

use clap::Parser;
use color_eyre::Result;
use fetch::fetch_crate;
use search::select_crate;
use std::path::PathBuf;
use write::write_skill;

/// Generate agent skills from crate documentation.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Crate to fetch, e.g. `clap` or `clap@4.5`. Omit to search interactively.
    crate_spec: Option<String>,

    /// Override the base output path (default: .agents/skills)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    // Interactive selection runs before the async runtime starts so that
    // reqwest::blocking (used inside the autocomplete) has no outer Tokio
    // runtime to conflict with.
    let spec = match args.crate_spec {
        Some(s) => s,
        None => select_crate()?,
    };

    let target = parse_spec(&spec);
    let base = args
        .output
        .unwrap_or_else(|| PathBuf::from(".agents/skills"));

    tokio::runtime::Runtime::new()?.block_on(async {
        let client = reqwest::Client::new();
        let info = fetch_crate(&client, &target).await?;
        write_skill(&info, &base)?;

        println!("name:        {}", info.name);
        println!("version:     {}", info.version);
        println!("license:     {}", info.license);
        println!("description: {}", info.description);
        if info.references.len() > 0 {
            println!(
                "pages:       {} ({} references)",
                info.references.len() + 1,
                info.references.len()
            );
        }
        println!("output:      {}", base.display());

        Ok(())
    })
}

pub struct CrateTarget {
    pub name: String,
    pub version: Option<String>,
}

fn parse_spec(spec: &str) -> CrateTarget {
    match spec.split_once('@') {
        Some((name, version)) => CrateTarget {
            name: name.to_string(),
            version: Some(version.to_string()),
        },
        None => CrateTarget {
            name: spec.to_string(),
            version: None,
        },
    }
}

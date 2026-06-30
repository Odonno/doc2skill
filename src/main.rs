mod cargo;
mod fetch;
mod search;
mod write;

use clap::Parser;
use color_eyre::Result;
use fetch::fetch_crate;
use inquire::MultiSelect;
use search::select_crate;
use std::path::{Path, PathBuf};
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
    let base = args
        .output
        .unwrap_or_else(|| PathBuf::from(".agents/skills"));

    match args.crate_spec {
        Some(spec) => run_single(&spec, &base)?,
        None if Path::new("Cargo.toml").exists() => run_multiple(&base)?,
        None => {
            // Interactive selection runs before the async runtime starts so that
            // reqwest::blocking (used inside the autocomplete) has no outer Tokio runtime to conflict with.
            let spec = select_crate()?;
            run_single(&spec, &base)?;
        }
    }

    Ok(())
}

fn run_single(spec: &str, base: &Path) -> Result<()> {
    let target = parse_spec(spec);
    tokio::runtime::Runtime::new()?.block_on(async {
        let client = reqwest::Client::new();
        let info = fetch_crate(&client, &target).await?;
        write_skill(&info, base)?;

        println!("name:        {}", info.name);
        println!("version:     {}", info.version);
        println!("license:     {}", info.license);
        println!("description: {}", info.description);
        if !info.references.is_empty() {
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

fn run_multiple(base: &Path) -> Result<()> {
    let all_crates = cargo::read_cargo_deps()?;
    let defaults: Vec<usize> = (0..all_crates.len()).collect();
    let selected = MultiSelect::new("Select crates to generate skills for:", all_crates)
        .with_default(&defaults)
        .prompt()?;

    let rt = tokio::runtime::Runtime::new()?;
    let client = reqwest::Client::new();
    let total = selected.len();
    let mut ok = 0usize;

    for name in &selected {
        let target = parse_spec(name);
        match rt.block_on(async {
            let info = fetch_crate(&client, &target).await?;
            write_skill(&info, base)?;
            Ok::<_, color_eyre::Report>(base.join(&info.name))
        }) {
            Ok(path) => {
                println!("✓ {}", path.display());
                ok += 1;
            }
            Err(e) => println!("✗ {} — {e}", name),
        }
    }

    println!("\n{ok}/{total} skills generated");

    Ok(())
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

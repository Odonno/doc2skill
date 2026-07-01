mod cargo;
mod cli;
#[cfg(feature = "tokens")]
mod count;
mod crate_target;
mod fetch;
mod search;
mod write;

use clap::Parser;
use color_eyre::Result;
use fetch::fetch_crate;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use search::select_crate;
use std::path::Path;
use write::write_skill;

use crate::{cli::CliArgs, crate_target::CrateTarget};

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = CliArgs::parse();
    let base = args.output;

    if args.count {
        #[cfg(feature = "tokens")]
        return count::run(args.crate_spec.as_deref(), &base);
        #[cfg(not(feature = "tokens"))]
        {
            eprintln!("error: built without 'tokens' feature, recompile with --features tokens");
            std::process::exit(1);
        }
    }

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
    let target = CrateTarget::parse(spec);
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

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("[{pos}/{len}] {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    for name in &selected {
        pb.set_message(format!("fetching {name}…"));
        let target = CrateTarget::parse(name);
        match rt.block_on(async {
            let info = fetch_crate(&client, &target).await?;
            write_skill(&info, base)?;
            Ok::<_, color_eyre::Report>(base.join(&info.name))
        }) {
            Ok(path) => {
                pb.println(format!("✓ {}", path.display()));
                ok += 1;
            }
            Err(e) => pb.println(format!("✗ {name} — {e}")),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    println!("{ok}/{total} skills generated");

    Ok(())
}

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

use crate::{cli::CliArgs, crate_target::CrateTarget, fetch::CrateInfo};

#[derive(Debug)]
enum SkillWarning {
    NoContent,
    #[cfg(feature = "tokens")]
    TooManyTokens(usize),
}

fn collect_warnings(info: &CrateInfo) -> Vec<SkillWarning> {
    let mut warnings = vec![];
    if info.page.markdown.trim().is_empty() {
        warnings.push(SkillWarning::NoContent);
    }
    #[cfg(feature = "tokens")]
    if let Ok(tokens) = count::count_text_tokens(&info.page.markdown) {
        if tokens > count::SKILL_TOKEN_WARN_THRESHOLD {
            warnings.push(SkillWarning::TooManyTokens(tokens));
        }
    }
    warnings
}

fn print_warnings(name: &str, warnings: &[SkillWarning]) {
    for w in warnings {
        let msg = match w {
            SkillWarning::NoContent => format!("\x1b[33m⚠ {name}: no content\x1b[0m"),
            #[cfg(feature = "tokens")]
            SkillWarning::TooManyTokens(tokens) => {
                format!(
                    "\x1b[33m⚠ {name}: skill content too large ({tokens} tokens > {})\x1b[0m",
                    count::SKILL_TOKEN_WARN_THRESHOLD
                )
            }
        };
        println!("{msg}");
    }
}

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
        let warnings = collect_warnings(&info);
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
        print_warnings(&info.name, &warnings);

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

    let mut all_warnings: Vec<(String, Vec<SkillWarning>)> = vec![];

    for target in &selected {
        pb.set_message(format!("fetching {}…", target.name));
        match rt.block_on(async {
            let info = fetch_crate(&client, target).await?;
            let warnings = collect_warnings(&info);
            write_skill(&info, base)?;
            Ok::<_, color_eyre::Report>((base.join(&info.name), info.name.clone(), warnings))
        }) {
            Ok((path, name, warnings)) => {
                pb.println(format!("✓ {}", path.display()));
                if !warnings.is_empty() {
                    all_warnings.push((name, warnings));
                }
                ok += 1;
            }
            Err(e) => pb.println(format!("✗ {} — {e}", target.name)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    println!("{ok}/{total} skills generated");
    for (name, warnings) in &all_warnings {
        print_warnings(name, warnings);
    }

    Ok(())
}

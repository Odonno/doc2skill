mod cli;
mod core;
#[cfg(feature = "csharp")]
mod csharp;
mod detection;
mod providers;
mod run;
#[cfg(feature = "rust")]
mod rust;

use clap::Parser;
use cli::{CliArgs, Language};
use color_eyre::Result;
use detection::detect_all_deps;
use providers::build_providers;
use run::{run_multiple_mixed, run_with};

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = CliArgs::parse();
    let base = args.output;

    if args.count {
        #[cfg(feature = "tokens")]
        return core::count::run(args.spec.as_deref(), &base);
        #[cfg(not(feature = "tokens"))]
        {
            eprintln!("error: built without 'tokens' feature, recompile with --features tokens");
            std::process::exit(1);
        }
    }

    let providers = build_providers();

    // Explicit language flag: single-provider path as before
    if let Some(lang) = args.language {
        return run_with(args.spec.as_deref(), &base, &providers[&lang]);
    }

    // No explicit language, no spec: detect project deps across all providers
    if args.spec.is_none() {
        let detected = detect_all_deps(&providers);
        if !detected.is_empty() {
            return run_multiple_mixed(detected, &providers, &base);
        }
    }

    // Fallback: prompt user for language
    let lang = select_language()?;
    run_with(args.spec.as_deref(), &base, &providers[&lang])
}

fn select_language() -> Result<Language> {
    let available: &[(&str, Language)] = &[
        #[cfg(feature = "rust")]
        ("Rust", Language::Rust),
        #[cfg(feature = "csharp")]
        ("C#", Language::Csharp),
    ];

    if available.len() == 1 {
        // ponytail: only one language compiled in — skip the prompt
        return Ok(available[0].1.clone());
    }

    let names: Vec<&str> = available.iter().map(|(n, _)| *n).collect();
    let selected = inquire::Select::new("Select language:", names).prompt()?;
    Ok(available
        .iter()
        .find(|(n, _)| *n == selected)
        .map(|(_, l)| l.clone())
        .unwrap())
}

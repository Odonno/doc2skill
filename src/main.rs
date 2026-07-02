mod cli;
mod core;
#[cfg(feature = "rust")]
mod rust;

use clap::Parser;
use cli::{CliArgs, Language};
use color_eyre::Result;
use core::{collect_warnings, print_warnings, write_skill, LanguageProvider, SkillWarning};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use std::path::Path;

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

    let language = select_language(args.language)?;

    match language {
        #[cfg(feature = "rust")]
        Language::Rust => {
            let provider = rust::RustProvider::new();
            run(args.spec.as_deref(), &base, &provider)?;
        }
    }

    Ok(())
}

fn select_language(lang: Option<Language>) -> Result<Language> {
    if let Some(l) = lang {
        return Ok(l);
    }

    // Build list of available language names at compile time
    let available: &[(&str, Language)] = &[
        #[cfg(feature = "rust")]
        ("Rust", Language::Rust),
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

fn run<P: LanguageProvider>(spec: Option<&str>, base: &Path, provider: &P) -> Result<()> {
    match (spec, provider.read_project_deps()) {
        (Some(spec), _) => run_single(spec, base, provider),
        (None, Some(deps)) => run_multiple(deps?, base, provider),
        (None, None) => {
            let spec = provider.search_interactive()?;
            run_single(&spec, base, provider)
        }
    }
}

fn run_single<P: LanguageProvider>(spec: &str, base: &Path, provider: &P) -> Result<()> {
    tokio::runtime::Runtime::new()?.block_on(async {
        let info = provider.fetch_info(spec).await?;
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

fn run_multiple<P: LanguageProvider>(deps: Vec<String>, base: &Path, provider: &P) -> Result<()> {
    let defaults: Vec<usize> = (0..deps.len()).collect();
    let selected = MultiSelect::new("Select packages to generate skills for:", deps)
        .with_default(&defaults)
        .prompt()?;

    let rt = tokio::runtime::Runtime::new()?;
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

    for spec in &selected {
        let name = spec.split_once('@').map(|(n, _)| n).unwrap_or(spec);
        pb.set_message(format!("fetching {}…", name));
        match rt.block_on(async {
            let info = provider.fetch_info(spec).await?;
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
            Err(e) => pb.println(format!("✗ {name} — {e}")),
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

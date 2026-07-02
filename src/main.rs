mod cli;
mod core;
#[cfg(feature = "csharp")]
mod csharp;
#[cfg(feature = "rust")]
mod rust;

use clap::Parser;
use cli::{CliArgs, Language};
use color_eyre::Result;
use core::{collect_warnings, print_warnings, write_skill, LanguageProvider, SkillWarning};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use std::{collections::BTreeMap, path::Path};

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

// --- provider dispatch ---

enum AnyProvider {
    #[cfg(feature = "rust")]
    Rust(rust::RustProvider),
    #[cfg(feature = "csharp")]
    Csharp(csharp::CSharpProvider),
}

impl AnyProvider {
    async fn fetch_info(&self, spec: &str) -> Result<core::SkillInfo> {
        match self {
            #[cfg(feature = "rust")]
            Self::Rust(p) => p.fetch_info(spec).await,
            #[cfg(feature = "csharp")]
            Self::Csharp(p) => p.fetch_info(spec).await,
        }
    }

    fn read_project_deps(&self) -> Option<Result<Vec<String>>> {
        match self {
            #[cfg(feature = "rust")]
            Self::Rust(p) => p.read_project_deps(),
            #[cfg(feature = "csharp")]
            Self::Csharp(p) => p.read_project_deps(),
        }
    }

    fn search_interactive(&self) -> Result<String> {
        match self {
            #[cfg(feature = "rust")]
            Self::Rust(p) => p.search_interactive(),
            #[cfg(feature = "csharp")]
            Self::Csharp(p) => p.search_interactive(),
        }
    }
}

fn build_providers() -> BTreeMap<Language, AnyProvider> {
    let mut map = BTreeMap::new();
    #[cfg(feature = "rust")]
    map.insert(Language::Rust, AnyProvider::Rust(rust::RustProvider::new()));
    #[cfg(feature = "csharp")]
    map.insert(
        Language::Csharp,
        AnyProvider::Csharp(csharp::CSharpProvider::new()),
    );
    map
}

// --- detection ---

struct PackageDetected {
    name: String,
    language: Language,
}

fn detect_all_deps(providers: &BTreeMap<Language, AnyProvider>) -> Vec<PackageDetected> {
    let mut all = Vec::new();
    for (lang, provider) in providers {
        if let Some(Ok(deps)) = provider.read_project_deps() {
            for dep in deps {
                all.push(PackageDetected {
                    name: dep,
                    language: lang.clone(),
                });
            }
        }
    }
    all
}

fn language_tag(lang: &Language) -> &'static str {
    match lang {
        #[cfg(feature = "rust")]
        Language::Rust => "rust",
        #[cfg(feature = "csharp")]
        Language::Csharp => "csharp",
    }
}

// --- runners ---

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

fn run_with(spec: Option<&str>, base: &Path, provider: &AnyProvider) -> Result<()> {
    match (spec, provider.read_project_deps()) {
        (Some(spec), _) => run_single(spec, base, provider),
        (None, Some(deps)) => {
            let items = build_single_provider_batch(deps?, provider)?;
            run_batch(items, base)
        }
        (None, None) => {
            let spec = provider.search_interactive()?;
            run_single(&spec, base, provider)
        }
    }
}

fn build_single_provider_batch(
    deps: Vec<String>,
    provider: &AnyProvider,
) -> Result<Vec<(String, &AnyProvider)>> {
    let defaults: Vec<usize> = (0..deps.len()).collect();
    let selected = MultiSelect::new("Select packages to generate skills for:", deps)
        .with_default(&defaults)
        .prompt()?;
    Ok(selected.into_iter().map(|s| (s, provider)).collect())
}

fn run_multiple_mixed(
    detected: Vec<PackageDetected>,
    providers: &BTreeMap<Language, AnyProvider>,
    base: &Path,
) -> Result<()> {
    let display: Vec<String> = detected
        .iter()
        .map(|d| format!("[{}] {}", language_tag(&d.language), d.name))
        .collect();

    let defaults: Vec<usize> = (0..display.len()).collect();
    let chosen: std::collections::HashSet<String> =
        MultiSelect::new("Select packages to generate skills for:", display.clone())
            .with_default(&defaults)
            .prompt()?
            .into_iter()
            .collect();

    let items: Vec<(String, &AnyProvider)> = detected
        .iter()
        .zip(display.iter())
        .filter(|(_, disp)| chosen.contains(disp.as_str()))
        .map(|(det, _)| (det.name.clone(), &providers[&det.language]))
        .collect();

    run_batch(items, base)
}

fn run_single(spec: &str, base: &Path, provider: &AnyProvider) -> Result<()> {
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

fn run_batch(items: Vec<(String, &AnyProvider)>, base: &Path) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let total = items.len();
    let mut ok = 0usize;

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("[{pos}/{len}] {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    let mut all_warnings: Vec<(String, Vec<SkillWarning>)> = vec![];

    for (spec, provider) in &items {
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

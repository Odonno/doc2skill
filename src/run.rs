use color_eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use std::{collections::BTreeMap, path::Path};

use crate::cli::Language;
use crate::core::{SkillWarning, collect_warnings, print_warnings, write_skill};
use crate::detection::{PackageDetected, language_tag};
use crate::providers::AnyProvider;

pub fn run_with(spec: Option<&str>, base: &Path, provider: &AnyProvider) -> Result<()> {
    match (spec, provider.read_project_deps()) {
        (Some(spec), _) => run_single(spec, base, provider),
        (None, Some(deps)) => {
            let items = prompt_single_provider_batch(deps?, provider)?;
            run_batch(items, base)
        }
        (None, None) => {
            let spec = provider.search_interactive()?;
            run_single(&spec, base, provider)
        }
    }
}

pub fn run_multiple_mixed(
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

fn prompt_single_provider_batch(
    deps: Vec<String>,
    provider: &AnyProvider,
) -> Result<Vec<(String, &AnyProvider)>> {
    let defaults: Vec<usize> = (0..deps.len()).collect();
    let selected = MultiSelect::new("Select packages to generate skills for:", deps)
        .with_default(&defaults)
        .prompt()?;
    Ok(selected.into_iter().map(|s| (s, provider)).collect())
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

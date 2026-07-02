use color_eyre::{eyre::eyre, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use tiktoken_rs::{cl100k_base, CoreBPE};

use crate::crate_target::CrateTarget;

pub const SKILL_TOKEN_WARN_THRESHOLD: usize = 5000;

struct FileEntry {
    skill_name: String,
    skill_md: PathBuf,
    references: Vec<PathBuf>,
}

pub struct RefTokenCount {
    pub name: String,
    pub tokens: usize,
}

pub struct SkillTokenCount {
    pub name: String,
    pub skill_tokens: usize,
    pub references: Vec<RefTokenCount>,
}

pub fn run(crate_spec: Option<&str>, base: &Path) -> Result<()> {
    let dirs = skill_dirs(base, crate_spec)?;
    let entries = collect_entries(&dirs);

    let total_files: u64 = entries.iter().map(|e| 1 + e.references.len() as u64).sum();
    let pb = make_progress_bar(total_files);

    let bpe = cl100k_base().map_err(|e| eyre!("{e}"))?;
    let results = count_all(&entries, &bpe, &pb);

    pb.finish_and_clear();
    print_tree(base, &results);

    Ok(())
}

fn skill_dirs(base: &Path, crate_spec: Option<&str>) -> Result<Vec<PathBuf>> {
    if let Some(spec) = crate_spec {
        let name = CrateTarget::parse(spec).name;
        let dir = base.join(&name);
        if !dir.is_dir() {
            return Err(eyre!("no skill found for '{name}'"));
        }
        return Ok(vec![dir]);
    }

    let mut dirs: Vec<PathBuf> = fs::read_dir(base)
        .map_err(|_| eyre!("cannot read output directory: {}", base.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn collect_entries(dirs: &[PathBuf]) -> Vec<FileEntry> {
    dirs.iter()
        .filter_map(|dir| {
            let skill_md = dir.join("SKILL.md");
            if !skill_md.exists() {
                return None;
            }
            let refs_dir = dir.join("references");
            let mut references: Vec<PathBuf> = if refs_dir.is_dir() {
                fs::read_dir(&refs_dir)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
                    .collect()
            } else {
                vec![]
            };
            references.sort();
            let name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            Some(FileEntry {
                skill_name: name,
                skill_md,
                references,
            })
        })
        .collect()
}

fn make_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("[{pos}/{len}] {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

fn count_all(entries: &[FileEntry], bpe: &CoreBPE, pb: &ProgressBar) -> Vec<SkillTokenCount> {
    entries
        .iter()
        .map(|fe| {
            pb.set_message(format!("{}/SKILL.md", fe.skill_name));
            let skill_tokens = count_file_tokens(&fe.skill_md, bpe);
            pb.inc(1);

            let references = fe
                .references
                .iter()
                .map(|ref_path| {
                    let name = ref_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    pb.set_message(format!("{}/references/{name}", fe.skill_name));
                    let tokens = count_file_tokens(ref_path, bpe);
                    pb.inc(1);
                    RefTokenCount { name, tokens }
                })
                .collect();

            SkillTokenCount {
                name: fe.skill_name.clone(),
                skill_tokens,
                references,
            }
        })
        .collect()
}

fn count_file_tokens(path: &Path, bpe: &CoreBPE) -> usize {
    let text = fs::read_to_string(path).unwrap_or_default();
    bpe.encode_with_special_tokens(&text).len()
}

pub fn count_text_tokens(text: &str) -> Result<usize> {
    let bpe = cl100k_base().map_err(|e| eyre!("{e}"))?;
    Ok(bpe.encode_with_special_tokens(text).len())
}

fn print_tree(base: &Path, results: &[SkillTokenCount]) {
    println!("/{}", base.display());
    for skill in results {
        println!("  /{}", skill.name);
        let skill_line = format!("    SKILL.md - {} tokens", skill.skill_tokens);
        println!(
            "{}",
            colorize(&skill_line, skill.skill_tokens > SKILL_TOKEN_WARN_THRESHOLD)
        );
        if !skill.references.is_empty() {
            println!("    references");
            for r in &skill.references {
                println!("      {} - {} tokens", r.name, r.tokens);
            }
        }
    }
}

fn colorize(text: &str, red: bool) -> String {
    if red {
        format!("\x1b[31m{text}\x1b[0m")
    } else {
        format!("\x1b[32m{text}\x1b[0m")
    }
}

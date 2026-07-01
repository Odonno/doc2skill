use std::path::{Path, PathBuf};

use crate::crate_target::CrateTarget;

struct FileEntry {
    skill_name: String,
    skill_md: PathBuf,
    references: Vec<PathBuf>,
}

// token counts: (skill_name, skill_tokens, Vec<(ref_name, ref_tokens)>)
type TokenCountsResult = (String, usize, Vec<(String, usize)>);

pub fn run(crate_spec: Option<&str>, base: &Path) -> color_eyre::Result<()> {
    use color_eyre::eyre::eyre;
    use indicatif::{ProgressBar, ProgressStyle};
    use std::fs;
    use tiktoken_rs::cl100k_base;

    // Collect (skill_dir_name, skill_md_path, reference_md_paths)
    let entries: Vec<PathBuf> = if let Some(spec) = crate_spec {
        let target = CrateTarget::parse(spec);
        let dir = base.join(target.name.to_string());
        if !dir.is_dir() {
            return Err(eyre!("no skill found for '{}'", target.name));
        }
        vec![dir]
    } else {
        let mut dirs: Vec<_> = fs::read_dir(base)
            .map_err(|_| eyre!("cannot read output directory: {}", base.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        dirs
    };

    // Collect all files to count upfront so we can size the progress bar.
    let mut skills: Vec<FileEntry> = Vec::new();
    for dir in &entries {
        let skill_md = dir.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let refs_dir = dir.join("references");
        let mut references: Vec<PathBuf> = if refs_dir.is_dir() {
            let mut r: Vec<_> = fs::read_dir(&refs_dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
                .collect();
            r.sort();
            r
        } else {
            vec![]
        };
        references.sort();
        let name = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        skills.push(FileEntry {
            skill_name: name,
            skill_md,
            references,
        });
    }

    let total_files: u64 = skills.iter().map(|s| 1 + s.references.len() as u64).sum();

    let bpe = cl100k_base().map_err(|e| eyre!("{e}"))?;

    let pb = ProgressBar::new(total_files);
    pb.set_style(
        ProgressStyle::with_template("[{pos}/{len}] {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    let mut results: Vec<TokenCountsResult> = Vec::new();

    for fe in &skills {
        pb.set_message(format!("{}/SKILL.md", fe.skill_name));
        let text = fs::read_to_string(&fe.skill_md).unwrap_or_default();
        let skill_tokens = bpe.encode_with_special_tokens(&text).len();
        pb.inc(1);

        let mut ref_counts = Vec::new();
        for ref_path in &fe.references {
            let ref_name = ref_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            pb.set_message(format!("{}/references/{}", fe.skill_name, ref_name));
            let text = fs::read_to_string(ref_path).unwrap_or_default();
            let tokens = bpe.encode_with_special_tokens(&text).len();
            ref_counts.push((ref_name, tokens));
            pb.inc(1);
        }

        results.push((fe.skill_name.clone(), skill_tokens, ref_counts));
    }

    pb.finish_and_clear();

    // Print tree
    println!("/{}", base.display());
    for (skill_name, skill_tokens, ref_counts) in &results {
        println!("  /{skill_name}");
        let skill_line = format!("    SKILL.md - ~ {skill_tokens} tokens");
        if *skill_tokens > 5000 {
            // red
            println!("\x1b[31m{skill_line}\x1b[0m");
        } else {
            // green
            println!("\x1b[32m{skill_line}\x1b[0m");
        }
        if !ref_counts.is_empty() {
            println!("    references");
            for (ref_name, tokens) in ref_counts {
                println!("      {ref_name} - ~ {tokens} tokens");
            }
        }
    }

    Ok(())
}

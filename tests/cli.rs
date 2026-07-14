use assert_cmd::Command;
use color_eyre::Result;
use predicates::prelude::*;
use temp_dir::TempDir;

fn cmd(args: &[&str], dir: &TempDir) -> assert_cmd::assert::Assert {
    Command::cargo_bin("doc2skill")
        .unwrap()
        .args(args)
        .arg("--language")
        .arg("rust")
        .arg("--output")
        .arg(dir.path())
        .assert()
}

#[test]
fn skill_md_is_created() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["color-eyre"], &dir).success();

    assert!(dir.path().join("color-eyre/SKILL.md").exists());

    Ok(())
}

#[test]
fn skill_md_clap_snapshot() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["clap@4.6.1"], &dir).success();

    let content = std::fs::read_to_string(dir.path().join("clap/SKILL.md"))?;
    insta::assert_snapshot!(content);

    Ok(())
}

#[test]
fn skill_md_has_frontmatter() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["color-eyre"], &dir).success();

    let content = std::fs::read_to_string(dir.path().join("color-eyre/SKILL.md"))?;
    assert!(content.starts_with("---\n"), "missing frontmatter opening");
    for field in ["name: color-eyre", "metadata:", "  version:"] {
        assert!(content.contains(field), "missing field: {field}");
    }

    Ok(())
}

#[test]
fn references_dir_is_created() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["color-eyre"], &dir).success();
    assert!(dir.path().join("color-eyre/references").is_dir());

    Ok(())
}

#[test]
fn pinned_version_is_used() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["color-eyre@0.6"], &dir).success();

    let content = std::fs::read_to_string(dir.path().join("color-eyre/SKILL.md"))?;
    assert!(
        content.contains("version: \"0.6"),
        "expected 0.6.x in frontmatter"
    );

    Ok(())
}

#[test]
fn unknown_crate_fails() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["this-crate-does-not-exist-xyz-999"], &dir)
        .failure()
        .stderr(predicate::str::is_empty().not());

    Ok(())
}

#[test]
fn skill_md_reference_links_use_md_paths() -> Result<()> {
    let dir = TempDir::new()?;
    cmd(&["clap"], &dir).success();

    let content = std::fs::read_to_string(dir.path().join("clap/SKILL.md"))?;
    assert!(
        !content.contains("](struct.Command.html"),
        "SKILL.md must not contain raw .html links for pages that exist as references"
    );
    assert!(
        content.contains("](references/struct.Command.md"),
        "SKILL.md must rewrite known reference links to references/<slug>.md"
    );

    Ok(())
}

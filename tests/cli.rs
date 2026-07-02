use assert_cmd::Command;
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
fn skill_md_is_created() {
    let dir = TempDir::new().unwrap();
    cmd(&["color-eyre"], &dir).success();
    assert!(dir.path().join("color-eyre/SKILL.md").exists());
}

#[test]
fn skill_md_has_frontmatter() {
    let dir = TempDir::new().unwrap();
    cmd(&["color-eyre"], &dir).success();
    let content = std::fs::read_to_string(dir.path().join("color-eyre/SKILL.md")).unwrap();
    assert!(content.starts_with("---\n"), "missing frontmatter opening");
    for field in ["name: color-eyre", "metadata:", "  version:"] {
        assert!(content.contains(field), "missing field: {field}");
    }
}

#[test]
fn references_dir_is_created() {
    let dir = TempDir::new().unwrap();
    cmd(&["color-eyre"], &dir).success();
    assert!(dir.path().join("color-eyre/references").is_dir());
}

#[test]
fn pinned_version_is_used() {
    let dir = TempDir::new().unwrap();
    cmd(&["color-eyre@0.6"], &dir).success();
    let content = std::fs::read_to_string(dir.path().join("color-eyre/SKILL.md")).unwrap();
    assert!(
        content.contains("version: \"0.6"),
        "expected 0.6.x in frontmatter"
    );
}

#[test]
fn unknown_crate_fails() {
    let dir = TempDir::new().unwrap();
    cmd(&["this-crate-does-not-exist-xyz-999"], &dir)
        .failure()
        .stderr(predicate::str::is_empty().not());
}

use super::SkillInfo;
use color_eyre::Result;
use std::{fs, path::Path};

pub fn build_frontmatter(info: &SkillInfo) -> String {
    let skill_name = info.name.replace('_', "-");
    let description = info
        .description
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let mut lines = vec![
        "---".to_string(),
        format!("name: {}", skill_name),
        format!("description: {}", description),
    ];

    if !info.license.is_empty() {
        lines.push(format!("license: {}", info.license));
    }

    lines.push("metadata:".to_string());

    if !info.author.is_empty() {
        lines.push(format!("  author: {}", info.author));
    }

    lines.push(format!("  version: \"{}\"", info.version));
    lines.push("---".to_string());

    format!("{}\n\n", lines.join("\n"))
}

pub fn write_skill(info: &SkillInfo, base: &Path) -> Result<()> {
    let skill_dir = base.join(&info.name);
    fs::create_dir_all(&skill_dir)?;

    let frontmatter = build_frontmatter(info);
    fs::write(
        skill_dir.join("SKILL.md"),
        format!("{}{}", frontmatter, info.page.markdown),
    )?;

    fs::write(
        skill_dir.join("doc.skill"),
        format!(
            "name = \"{}\"\nversion = \"{}\"\ngen = \"v1\"\n",
            info.name, info.version
        ),
    )?;

    if !info.references.is_empty() {
        let refs_dir = skill_dir.join("references");
        fs::create_dir_all(&refs_dir)?;
        for r in info.references.values() {
            fs::write(refs_dir.join(format!("{}.md", r.slug)), &r.markdown)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::SkillPage;
    use std::collections::BTreeMap;

    fn make_info(license: &str, author: &str) -> SkillInfo {
        SkillInfo {
            name: "my_crate".to_string(),
            version: "1.0.0".to_string(),
            description: "A test crate".to_string(),
            license: license.to_string(),
            author: author.to_string(),
            page: SkillPage {
                slug: "index".to_string(),
                title: "".to_string(),
                markdown: "".to_string(),
            },
            references: BTreeMap::new(),
        }
    }

    #[test]
    fn all_fields_present() {
        let info = make_info("MIT", "alice");
        let fm = build_frontmatter(&info);
        assert!(fm.contains("name: my-crate"));
        assert!(fm.contains("description: A test crate"));
        assert!(fm.contains("license: MIT"));
        assert!(fm.contains("  author: alice"));
        assert!(fm.contains("  version: \"1.0.0\""));
    }

    #[test]
    fn skips_license_when_empty() {
        let info = make_info("", "alice");
        let fm = build_frontmatter(&info);
        assert!(
            !fm.contains("license:"),
            "license line should be absent when empty"
        );
    }

    #[test]
    fn skips_author_when_empty() {
        let info = make_info("MIT", "");
        let fm = build_frontmatter(&info);
        assert!(
            !fm.contains("author:"),
            "author line should be absent when empty"
        );
        assert!(
            fm.contains("metadata:"),
            "metadata block must still exist for version"
        );
    }
}

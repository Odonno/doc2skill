use color_eyre::Result;
use std::{collections::BTreeMap, fs, path::Path};

use super::{SkillInfo, SkillPage};

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

/// Rewrites relative `slug.html` links to `references/slug.md` for known reference slugs.
fn rewrite_reference_links(markdown: &str, references: &BTreeMap<String, SkillPage>) -> String {
    let mut result = markdown.to_string();
    for slug in references.keys() {
        result = result.replace(
            &format!("({}.html", slug),
            &format!("(references/{}.md", slug),
        );
    }
    result
}

fn build_items_section(items: &[(String, Vec<(String, String)>)]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut s = "\n### Items\n".to_string();
    for (category, entries) in items {
        s.push_str(&format!("\n#### {}\n\n", category));
        for (name, slug) in entries {
            s.push_str(&format!("* [{}](references/{}.md)\n", name, slug));
        }
    }
    s
}

pub fn write_skill(info: &SkillInfo, base: &Path) -> Result<()> {
    let skill_dir = base.join(&info.name);
    fs::create_dir_all(&skill_dir)?;

    let frontmatter = build_frontmatter(info);
    let skill_markdown = rewrite_reference_links(&info.page.markdown, &info.references);
    let items_section = build_items_section(&info.items);
    fs::write(
        skill_dir.join("SKILL.md"),
        format!("{}{}{}", frontmatter, skill_markdown, items_section),
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
            items: Vec::new(),
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

    fn make_reference(slug: &str) -> SkillPage {
        SkillPage {
            slug: slug.to_string(),
            title: String::new(),
            markdown: String::new(),
        }
    }

    #[test]
    fn rewrites_known_reference_links() {
        let mut refs = BTreeMap::new();
        refs.insert(
            "struct.Command".to_string(),
            make_reference("struct.Command"),
        );

        let input = r#"* Builder [tutorial](_tutorial/index.html "mod clap::_tutorial") and [reference](struct.Command.html "struct clap::Command")"#;
        let output = rewrite_reference_links(input, &refs);

        assert!(
            output.contains("(references/struct.Command.md"),
            "known reference should be rewritten to references/"
        );
        assert!(
            output.contains("(_tutorial/index.html"),
            "unknown link should be left unchanged"
        );
    }

    #[test]
    fn leaves_non_reference_links_unchanged() {
        let refs: BTreeMap<String, SkillPage> = BTreeMap::new();
        let input = "See [docs](https://docs.rs/clap/latest/clap/) for details.";
        assert_eq!(rewrite_reference_links(input, &refs), input);
    }

    #[test]
    fn build_items_section_generates_links() {
        let items = vec![
            (
                "Structs".to_string(),
                vec![
                    ("Command".to_string(), "struct.Command".to_string()),
                    ("builder::Arg".to_string(), "struct.builder.Arg".to_string()),
                ],
            ),
            (
                "Enums".to_string(),
                vec![("ColorChoice".to_string(), "enum.ColorChoice".to_string())],
            ),
        ];
        let s = build_items_section(&items);
        assert!(s.contains("### Items"));
        assert!(s.contains("#### Structs"));
        assert!(s.contains("* [Command](references/struct.Command.md)"));
        assert!(s.contains("* [builder::Arg](references/struct.builder.Arg.md)"));
        assert!(s.contains("#### Enums"));
        assert!(s.contains("* [ColorChoice](references/enum.ColorChoice.md)"));
    }

    #[test]
    fn build_items_section_empty_returns_empty_string() {
        assert_eq!(build_items_section(&[]), "");
    }
}

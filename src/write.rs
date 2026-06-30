use crate::fetch::CrateInfo;
use color_eyre::Result;
use std::{fs, path::Path};

pub fn write_skill(info: &CrateInfo, base: &Path) -> Result<()> {
    let skill_dir = base.join(&info.name);
    fs::create_dir_all(&skill_dir)?;

    let frontmatter = format!(
        "---\nname: {}\ndescription: {}\nlicense: {}\nmetadata:\n  author: {}\n  version: \"{}\"\n---\n\n",
        info.name, info.description, info.license, info.author, info.version
    );
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

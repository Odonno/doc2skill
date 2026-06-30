use color_eyre::Result;
use std::{collections::BTreeSet, fs};

/// Reads `Cargo.toml` in the current directory and returns a sorted list of
/// actual crate names from both `[dependencies]` and `[dev-dependencies]`.
/// Handles renamed deps: `alias = { package = "real-name", ... }`.
pub fn read_cargo_deps() -> Result<Vec<String>> {
    let content = fs::read_to_string("Cargo.toml")?;
    let manifest: toml::Value = toml::from_str(&content)?;

    let mut names = BTreeSet::new();
    for section in &["dependencies", "dev-dependencies"] {
        if let Some(table) = manifest.get(section).and_then(|v| v.as_table()) {
            for (alias, value) in table {
                let name = value
                    .get("package")
                    .and_then(|v| v.as_str())
                    .unwrap_or(alias)
                    .to_owned();
                names.insert(name);
            }
        }
    }

    Ok(names.into_iter().collect())
}

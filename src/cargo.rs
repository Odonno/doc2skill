use color_eyre::Result;
use std::{collections::BTreeMap, fs};

use crate::crate_target::CrateTarget;

/// Reads `Cargo.toml` in the current directory and returns a sorted list of
/// crate targets (name + version) from both `[dependencies]` and `[dev-dependencies]`.
/// Handles renamed deps: `alias = { package = "real-name", ... }`.
pub fn read_cargo_deps() -> Result<Vec<CrateTarget>> {
    let content = fs::read_to_string("Cargo.toml")?;
    let manifest: toml::Value = toml::from_str(&content)?;

    let mut map: BTreeMap<String, Option<String>> = BTreeMap::new();
    for section in &["dependencies", "dev-dependencies"] {
        if let Some(table) = manifest.get(section).and_then(|v| v.as_table()) {
            for (alias, value) in table {
                let name = value
                    .get("package")
                    .and_then(|v| v.as_str())
                    .unwrap_or(alias)
                    .to_owned();
                let version = match value {
                    toml::Value::String(s) => parse_version_req(s),
                    toml::Value::Table(_) => value
                        .get("version")
                        .and_then(|v| v.as_str())
                        .and_then(parse_version_req),
                    _ => None,
                };
                map.entry(name).or_insert(version);
            }
        }
    }

    Ok(map
        .into_iter()
        .map(|(name, version)| CrateTarget { name, version })
        .collect())
}

/// Strips leading operator chars (^, ~, =, >, <, spaces) and returns the first
/// version number. E.g. `"^0.9.8"` → `Some("0.9.8")`, `"*"` → `None`.
fn parse_version_req(req: &str) -> Option<String> {
    let stripped = req.trim_start_matches(|c: char| !c.is_ascii_digit());
    if stripped.is_empty() {
        return None;
    }
    Some(
        stripped
            .split(|c: char| c.is_whitespace() || c == ',')
            .next()
            .unwrap()
            .to_owned(),
    )
}

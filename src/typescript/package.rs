use color_eyre::Result;
use std::{collections::BTreeMap, fs};

use super::target::NpmTarget;

/// Reads `package.json` in the current directory and returns a sorted list of
/// npm package specs from both `dependencies` and `devDependencies`.
/// Filters out `@types/*` packages (redundant with their base package).
pub fn read_package_deps() -> Result<Vec<NpmTarget>> {
    let content = fs::read_to_string("package.json")?;
    let manifest: serde_json::Value = serde_json::from_str(&content)?;

    let mut map: BTreeMap<String, Option<String>> = BTreeMap::new();
    for section in &["dependencies", "devDependencies"] {
        if let Some(obj) = manifest.get(section).and_then(|v| v.as_object()) {
            for (name, version_val) in obj {
                if name.starts_with("@types/") {
                    continue;
                }
                let version = version_val
                    .as_str()
                    .and_then(parse_version_spec)
                    .map(|s| s.to_owned());
                map.entry(name.clone()).or_insert(version);
            }
        }
    }

    Ok(map
        .into_iter()
        .map(|(name, version)| NpmTarget { name, version })
        .collect())
}

/// Strips leading semver operators and returns the first concrete version.
/// `^19.0.0` → `19.0.0`, `~1.2.3` → `1.2.3`, `workspace:*` / `*` → `None`.
fn parse_version_spec(spec: &str) -> Option<&str> {
    let stripped = spec
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit());
    if stripped.is_empty() {
        return None;
    }
    let end = stripped
        .find(|c: char| c.is_whitespace() || c == ',')
        .unwrap_or(stripped.len());
    let v = stripped[..end].trim_end_matches('.');
    if v.is_empty() { None } else { Some(v) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_caret() {
        assert_eq!(parse_version_spec("^19.0.0"), Some("19.0.0"));
    }

    #[test]
    fn strips_tilde() {
        assert_eq!(parse_version_spec("~1.2.3"), Some("1.2.3"));
    }

    #[test]
    fn wildcard_is_none() {
        assert_eq!(parse_version_spec("*"), None);
    }

    #[test]
    fn workspace_is_none() {
        assert_eq!(parse_version_spec("workspace:*"), None);
    }

    #[test]
    fn exact_version_unchanged() {
        assert_eq!(parse_version_spec("19.2.7"), Some("19.2.7"));
    }
}

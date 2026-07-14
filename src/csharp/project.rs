use color_eyre::Result;
use std::{collections::BTreeMap, fs, path::Path};

use super::target::PackageTarget;

/// Scans the current dir (and one level down) for `.csproj` files and
/// `Directory.Packages.props`, returns a deduplicated sorted list of package specs.
pub fn read_project_deps() -> Result<Vec<PackageTarget>> {
    // ponytail: simple attribute scan instead of a full XML parser; add quick-xml if malformed input matters
    let mut map: BTreeMap<String, Option<String>> = BTreeMap::new();

    for path in collect_project_files() {
        if let Ok(xml) = fs::read_to_string(&path) {
            let tag = if path.ends_with("Directory.Packages.props") {
                "PackageVersion"
            } else {
                "PackageReference"
            };
            for (id, version) in extract_package_entries(&xml, tag) {
                map.entry(id).or_insert(version);
            }
        }
    }

    Ok(map
        .into_iter()
        .map(|(name, version)| PackageTarget { name, version })
        .collect())
}

/// Returns project file paths to inspect: `Directory.Packages.props` in the
/// current directory, then all `*.csproj` in the current dir and one level down.
fn collect_project_files() -> Vec<String> {
    let mut paths = vec![];

    if Path::new("Directory.Packages.props").exists() {
        paths.push("Directory.Packages.props".to_owned());
    }

    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("csproj") {
                if let Some(s) = p.to_str() {
                    paths.push(s.to_owned());
                }
            } else if p.is_dir() {
                // one level down
                if let Ok(sub) = fs::read_dir(&p) {
                    for sub_entry in sub.flatten() {
                        let sp = sub_entry.path();
                        if sp.extension().and_then(|e| e.to_str()) == Some("csproj")
                            && let Some(s) = sp.to_str()
                        {
                            paths.push(s.to_owned());
                        }
                    }
                }
            }
        }
    }

    paths
}

/// Extracts (id, version) pairs from XML for the given element tag name.
/// Handles both self-closing and paired tags; attributes may be in any order.
fn extract_package_entries(xml: &str, tag: &str) -> Vec<(String, Option<String>)> {
    let prefix = format!("<{}", tag);
    let mut results = Vec::new();

    for segment in xml.split(&prefix) {
        // Find the closing `>` of the opening tag
        let end = match segment.find('>') {
            Some(i) => i,
            None => continue,
        };
        let attrs = &segment[..end];
        if let Some(id) = extract_attr(attrs, "Include") {
            let version = extract_attr(attrs, "Version")
                .as_deref()
                .and_then(parse_version_spec)
                .map(|s| s.to_owned());
            results.push((id, version));
        }
    }

    results
}

/// Returns the value of `Name="value"` from an attribute string.
fn extract_attr(attrs: &str, name: &str) -> Option<String> {
    let key = format!("{}=\"", name);
    let start = attrs.find(&key)? + key.len();
    let end = attrs[start..].find('"')?;
    Some(attrs[start..start + end].to_owned())
}

/// Strips range/floating operators and returns the first concrete version segment.
/// `[1.0, 2.0)` → `1.0`, `1.0.*` → `1.0`, `^13.0.3` → `13.0.3`, `*` → `None`.
fn parse_version_spec(spec: &str) -> Option<&str> {
    let stripped = spec
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit());
    if stripped.is_empty() {
        return None;
    }
    let end = stripped
        .find(|c: char| c.is_whitespace() || c == ',' || c == ')' || c == ']' || c == '*')
        .unwrap_or(stripped.len());
    let v = stripped[..end].trim_end_matches('.');
    if v.is_empty() { None } else { Some(v) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_reference() {
        let xml = r#"<PackageReference Include="Newtonsoft.Json" Version="13.0.3" />"#;
        let entries = extract_package_entries(xml, "PackageReference");
        assert_eq!(
            entries,
            vec![("Newtonsoft.Json".to_owned(), Some("13.0.3".to_owned()))]
        );
    }

    #[test]
    fn parses_package_version() {
        let xml = r#"<PackageVersion Include="Serilog" Version="3.1.0" />"#;
        let entries = extract_package_entries(xml, "PackageVersion");
        assert_eq!(
            entries,
            vec![("Serilog".to_owned(), Some("3.1.0".to_owned()))]
        );
    }

    #[test]
    fn version_range_stripped() {
        let xml = r#"<PackageReference Include="Foo" Version="[1.0, 2.0)" />"#;
        let entries = extract_package_entries(xml, "PackageReference");
        assert_eq!(entries[0].1, Some("1.0".to_owned()));
    }

    #[test]
    fn wildcard_version_is_none() {
        assert_eq!(parse_version_spec("*"), None);
        assert_eq!(parse_version_spec("1.0.*"), Some("1.0"));
    }
}

use crate::core::{SkillInfo, SkillPage};
use color_eyre::{Result, eyre::eyre};
use reqwest::Client;
use std::collections::BTreeMap;

use super::target::PackageTarget;

pub async fn fetch_package(client: &Client, target: &PackageTarget) -> Result<SkillInfo> {
    let id = target.id_lower();

    // Resolve concrete version first (needed for subsequent URLs).
    let version = match &target.version {
        Some(spec) => resolve_version(client, &id, spec).await?,
        None => fetch_latest_version(client, &id).await?,
    };

    let (canonical_name, description, license, author) =
        fetch_metadata(client, &id, &version).await?;
    let readme = fetch_readme(client, &id, &version).await?;

    let markdown = readme.unwrap_or_else(|| description.clone());

    Ok(SkillInfo {
        name: canonical_name,
        version,
        description,
        license,
        author,
        page: SkillPage {
            slug: "index".to_owned(),
            title: String::new(),
            markdown,
        },
        references: BTreeMap::new(), // ponytail: no references for now
        items: Vec::new(),           // ponytail: no item scanning for C#
    })
}

/// Returns all versions from the flat-container index (oldest → newest).
async fn fetch_versions(client: &Client, id: &str) -> Result<Vec<String>> {
    let resp: serde_json::Value = client
        .get(format!(
            "https://api.nuget.org/v3-flatcontainer/{}/index.json",
            id
        ))
        .header(
            "User-Agent",
            "doc2skill/0.1 (https://github.com/odonno/doc2skill)",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    resp["versions"]
        .as_array()
        .map(|vs| {
            vs.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .ok_or_else(|| eyre!("nuget: missing versions array for {}", id))
}

async fn fetch_latest_version(client: &Client, id: &str) -> Result<String> {
    let versions = fetch_versions(client, id).await?;
    // Prefer stable; fall back to latest pre-release if no stable exists.
    versions
        .iter()
        .rev()
        .find(|v| !v.contains('-'))
        .or_else(|| versions.last())
        .cloned()
        .ok_or_else(|| eyre!("nuget: no versions found for {}", id))
}

/// Finds the highest version that starts with `spec` (e.g. "13" matches "13.0.3").
async fn resolve_version(client: &Client, id: &str, spec: &str) -> Result<String> {
    let versions = fetch_versions(client, id).await?;
    versions
        .into_iter()
        .rev()
        .find(|v| v == spec || v.starts_with(&format!("{}.", spec)))
        .ok_or_else(|| eyre!("nuget: no version matching '{}' for {}", spec, id))
}

/// Returns (canonical_id, description, license, author) from the registration leaf + catalog.
async fn fetch_metadata(
    client: &Client,
    id: &str,
    version: &str,
) -> Result<(String, String, String, String)> {
    // Step 1: registration leaf — catalogEntry is a URL, not an inline object
    let leaf: serde_json::Value = client
        .get(format!(
            "https://api.nuget.org/v3/registration5-semver1/{}/{}.json",
            id, version
        ))
        .header(
            "User-Agent",
            "doc2skill/0.1 (https://github.com/odonno/doc2skill)",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let catalog_url = leaf["catalogEntry"]
        .as_str()
        .ok_or_else(|| eyre!("nuget: missing catalogEntry for {}/{}", id, version))?;

    // Step 2: catalog entry — contains the actual metadata
    let entry: serde_json::Value = client
        .get(catalog_url)
        .header(
            "User-Agent",
            "doc2skill/0.1 (https://github.com/odonno/doc2skill)",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let canonical_id = entry["id"].as_str().unwrap_or(id).to_owned();

    let description = entry["description"].as_str().unwrap_or("").to_owned();

    let license = entry["licenseExpression"]
        .as_str()
        .or_else(|| entry["licenseUrl"].as_str())
        .unwrap_or("")
        .to_owned();

    let author = entry["authors"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();

    Ok((canonical_id, description, license, author))
}

/// Returns the README markdown, or `None` if the package has no README.
async fn fetch_readme(client: &Client, id: &str, version: &str) -> Result<Option<String>> {
    let resp = client
        .get(format!(
            "https://api.nuget.org/v3-flatcontainer/{}/{}/readme",
            id, version
        ))
        .header(
            "User-Agent",
            "doc2skill/0.1 (https://github.com/odonno/doc2skill)",
        )
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let text = resp.error_for_status()?.text().await?;
    Ok(Some(text))
}

use crate::core::{SkillInfo, SkillPage};
use color_eyre::{Result, eyre::eyre};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{BTreeMap, HashMap};

use super::target::NpmTarget;

pub async fn fetch_package(client: &Client, target: &NpmTarget) -> Result<SkillInfo> {
    let version_spec = target.version.as_deref().unwrap_or("latest");
    let (version, description, license, author, registry_readme) =
        fetch_metadata(client, &target.name, version_spec).await?;

    // CDN README is preferred; fall back to the readme field in the registry response.
    let readme = fetch_readme(client, &target.name, &version)
        .await
        .unwrap_or(registry_readme);

    // npmx.dev docs are best-effort; silently skip on failure.
    let (references, items) = fetch_npmx_docs(client, &target.name, &version)
        .await
        .unwrap_or_default();

    Ok(SkillInfo {
        name: target.name.clone(),
        version,
        description,
        license,
        author,
        page: SkillPage {
            slug: "index".to_owned(),
            title: String::new(),
            markdown: readme,
        },
        references,
        items,
    })
}

async fn fetch_metadata(
    client: &Client,
    name: &str,
    version_spec: &str,
) -> Result<(String, String, String, String, String)> {
    let resp: serde_json::Value = client
        .get(format!(
            "https://registry.npmjs.org/{}/{}",
            name, version_spec
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

    let version = resp["version"]
        .as_str()
        .ok_or_else(|| eyre!("npm: missing version for {}", name))?
        .to_owned();

    let description = resp["description"].as_str().unwrap_or("").to_owned();

    let license = resp["license"]
        .as_str()
        .or_else(|| resp["license"]["type"].as_str())
        .unwrap_or("unknown")
        .to_owned();

    let author = [
        resp["author"]["name"].as_str(),
        resp["author"].as_str(),
        resp["maintainers"][0]["name"].as_str(),
    ]
    .iter()
    .filter_map(|&s| s)
    .find(|s| !s.is_empty())
    .unwrap_or("unknown")
    .to_owned();

    let readme = resp["readme"].as_str().unwrap_or("").to_owned();

    Ok((version, description, license, author, readme))
}

async fn fetch_readme(client: &Client, name: &str, version: &str) -> Result<String> {
    let text = client
        .get(format!(
            "https://cdn.jsdelivr.net/npm/{}@{}/README.md",
            name, version
        ))
        .header("User-Agent", "doc2skill/0.1")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(text)
}

/// Item kinds to extract from npmx.dev, in display order.
/// `(api_kind, slug_prefix, display_category)`
const TS_ITEM_KINDS: &[(&str, &str, &str)] = &[
    ("function", "fn", "Functions"),
    ("class", "class", "Classes"),
    ("interface", "interface", "Interfaces"),
    ("variable", "variable", "Variables"),
];

async fn fetch_npmx_docs(
    client: &Client,
    name: &str,
    version: &str,
) -> Result<(
    BTreeMap<String, SkillPage>,
    Vec<(String, Vec<(String, String)>)>,
)> {
    let resp: serde_json::Value = client
        .get(format!(
            "https://npmx.dev/api/registry/docs/{}/v/{}",
            name, version
        ))
        .header("User-Agent", "doc2skill/0.1")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let html_str = resp["html"].as_str().unwrap_or("");
    if html_str.is_empty() {
        return Ok(Default::default());
    }

    let doc = Html::parse_fragment(html_str);
    let article_sel = Selector::parse("article[id]").unwrap();
    let sig_sel = Selector::parse(".docs-signature").unwrap();
    let desc_sel = Selector::parse(".docs-description").unwrap();

    let mut references: BTreeMap<String, SkillPage> = BTreeMap::new();
    let mut items_map: HashMap<&str, Vec<(String, String)>> = HashMap::new();

    for article in doc.select(&article_sel) {
        let id = article.value().attr("id").unwrap_or("");
        let Some(dash) = id.find('-') else { continue };
        let kind = &id[..dash];
        let item_name = &id[dash + 1..];

        let Some(&(_, prefix, category)) = TS_ITEM_KINDS.iter().find(|(k, _, _)| *k == kind) else {
            continue;
        };

        let slug = format!("{}.{}", prefix, item_name);

        // Signature: raw text from the shiki pre block (span styles stripped by .text()).
        let signature: String = article
            .select(&sig_sel)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join(""))
            .unwrap_or_default();

        // Description: convert inner HTML to markdown to preserve <code> and <br>.
        let description_md: String = article
            .select(&desc_sel)
            .next()
            .map(|e| {
                htmd::convert(&e.inner_html())
                    .unwrap_or_else(|_| e.text().collect::<Vec<_>>().join(""))
            })
            .unwrap_or_default();

        let mut markdown = String::new();
        if !signature.is_empty() {
            markdown.push_str("```typescript\n");
            markdown.push_str(signature.trim());
            markdown.push_str("\n```\n\n");
        }
        if !description_md.is_empty() {
            markdown.push_str(description_md.trim());
            markdown.push('\n');
        }

        references.insert(
            slug.clone(),
            SkillPage {
                slug: slug.clone(),
                title: item_name.to_owned(),
                markdown,
            },
        );

        items_map
            .entry(category)
            .or_default()
            .push((item_name.to_owned(), slug));
    }

    // Preserve category display order from TS_ITEM_KINDS.
    let items: Vec<(String, Vec<(String, String)>)> = TS_ITEM_KINDS
        .iter()
        .filter_map(|(_, _, cat)| {
            items_map
                .remove(*cat)
                .filter(|v| !v.is_empty())
                .map(|v| (cat.to_string(), v))
        })
        .collect();

    Ok((references, items))
}

use crate::core::{SkillInfo, SkillPage};
use color_eyre::{Result, eyre::eyre};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::collections::{BTreeMap, HashSet};

use super::target::CrateTarget;

pub async fn fetch_crate(client: &Client, target: &CrateTarget) -> Result<SkillInfo> {
    let ((version, description, license), author) = tokio::try_join!(
        fetch_metadata(client, target),
        fetch_author(client, &target.name)
    )?;
    let (page, references) = fetch_docs(client, &target.name, &version).await?;
    Ok(SkillInfo {
        name: target.name.clone(),
        version,
        description,
        license,
        author,
        page,
        references,
    })
}

async fn fetch_metadata(client: &Client, target: &CrateTarget) -> Result<(String, String, String)> {
    let resp: serde_json::Value = client
        .get(format!("https://crates.io/api/v1/crates/{}", target.name))
        .header(
            "User-Agent",
            "doc2skill/0.1 (https://github.com/odonno/doc2skill)",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let description = resp["crate"]["description"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let newest = resp["crate"]["newest_version"]
        .as_str()
        .ok_or_else(|| eyre!("crates.io: missing newest_version for {}", target.name))?
        .to_owned();
    let version = target.version.clone().unwrap_or(newest);

    let license = resp["versions"]
        .as_array()
        .and_then(|vs| {
            vs.iter().find(|v| {
                v["num"]
                    .as_str()
                    .map(|n| version_matches(n, &version))
                    .unwrap_or(false)
            })
        })
        .and_then(|v| v["license"].as_str())
        .unwrap_or("unknown")
        .to_owned();

    Ok((version, description, license))
}

async fn fetch_author(client: &Client, name: &str) -> Result<String> {
    let resp: serde_json::Value = client
        .get(format!("https://crates.io/api/v1/crates/{}/owners", name))
        .header(
            "User-Agent",
            "doc2skill/0.1 (https://github.com/odonno/doc2skill)",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let author = resp["users"]
        .as_array()
        .and_then(|us| us.first())
        .and_then(|u| u["login"].as_str())
        .unwrap_or("unknown")
        .to_owned();
    Ok(author)
}

/// Matches exact version or prefix (e.g. "4.5" matches "4.5.40").
// ponytail: no semver crate; add it if pre-release or build-metadata matching is ever needed.
fn version_matches(num: &str, spec: &str) -> bool {
    num == spec || num.starts_with(&format!("{}.", spec))
}

async fn fetch_docs(
    client: &Client,
    name: &str,
    version: &str,
) -> Result<(SkillPage, BTreeMap<String, SkillPage>)> {
    let crate_module = name.replace('-', "_");
    let index_url = format!(
        "https://docs.rs/{}/{}/{}/index.html",
        name, version, crate_module
    );

    let resp = client
        .get(&index_url)
        .header("User-Agent", "doc2skill/0.1")
        .send()
        .await?
        .error_for_status()?;

    let final_url = resp.url().clone();
    let html = resp.text().await?;
    let crate_base = final_url.join("./").unwrap();

    let (title, markdown, links) = extract_page(&html, &final_url, &crate_base)?;
    let page = SkillPage {
        slug: "index".to_owned(),
        title,
        markdown,
    };

    let mut references = BTreeMap::new();
    for link in links {
        let slug = link
            .path_segments()
            .and_then(|mut s| s.next_back())
            .and_then(|s| s.strip_suffix(".html"))
            .unwrap_or("unknown")
            .to_owned();
        let resp = client
            .get(link.as_str())
            .header("User-Agent", "doc2skill/0.1")
            .send()
            .await?
            .error_for_status()?;
        let page_url = resp.url().clone();
        let html = resp.text().await?;
        let (title, markdown, _) = extract_page(&html, &page_url, &crate_base)?;
        references.insert(
            slug.clone(),
            SkillPage {
                slug,
                title,
                markdown,
            },
        );
    }

    Ok((page, references))
}

/// Post-processes converted markdown:
/// - Bare opening fences (```) default to ```rust
/// - Strips leading line numbers from scraped-example code blocks
fn postprocess_markdown(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut in_fence = false;
    for line in markdown.lines() {
        if line.starts_with("```") {
            if in_fence {
                out.push_str(line);
                in_fence = false;
            } else {
                let after_ticks = line.trim_start_matches('`');
                if after_ticks.is_empty() {
                    out.push_str("```rust");
                } else {
                    out.push_str(line);
                }
                in_fence = true;
            }
        } else if in_fence {
            out.push_str(line.trim_start_matches(|c: char| c.is_ascii_digit()));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn extract_page(
    html: &str,
    page_url: &Url,
    crate_base: &Url,
) -> Result<(String, String, Vec<Url>)> {
    let doc = Html::parse_document(html);

    let title = doc
        .select(&Selector::parse("title").unwrap())
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();

    let content_html: String = doc
        .select(&Selector::parse("#main-content .docblock").unwrap())
        .map(|e| e.html())
        .collect::<Vec<_>>()
        .join("\n");

    let markdown = htmd::convert(&content_html).map_err(|e| eyre!("htmd: {e}"))?;
    let markdown = postprocess_markdown(&markdown);

    let mut seen = HashSet::new();
    let mut links = Vec::new();
    for el in doc.select(&Selector::parse("#main-content .docblock a[href]").unwrap()) {
        let href = el.value().attr("href").unwrap_or("");
        if let Ok(resolved) = page_url.join(href) {
            let s = resolved.as_str().to_owned();
            if s.starts_with(crate_base.as_str()) && s.ends_with(".html") && seen.insert(s) {
                links.push(resolved);
            }
        }
    }

    Ok((title, markdown, links))
}

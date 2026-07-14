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

/// Strips inline tags (like `<span>`) from HTML, preserving structural tags
/// (`<code>`, `</code>`) and all text content (including `\n`).
/// htmd needs `<code>` inside `<pre>` to produce fenced code blocks.
fn strip_inline_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(tag_start) = rest.find('<') {
        out.push_str(&rest[..tag_start]);
        rest = &rest[tag_start..];
        let Some(tag_end) = rest.find('>') else { break };
        let tag_inner = &rest[1..tag_end];
        // Keep <code ...> and </code>; discard span and all other inline tags
        if tag_inner.starts_with("code") || tag_inner == "/code" {
            out.push_str(&rest[..tag_end + 1]);
        }
        rest = &rest[tag_end + 1..];
    }
    out.push_str(rest);
    out
}

/// Replaces each `<pre>...</pre>` block with a tag-stripped version so that
/// `\n` text nodes between syntax-highlighting spans are preserved after
/// htmd conversion (htmd loses them when spans are adjacent with no space).
fn preprocess_pre_blocks(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(pre_start) = rest.find("<pre") {
        out.push_str(&rest[..pre_start]);
        rest = &rest[pre_start..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        let Some(close_start) = rest.find("</pre>") else {
            break;
        };
        let opening_tag = &rest[..open_end + 1]; // preserve <pre class="..."> for htmd language detection
        let inner = &rest[open_end + 1..close_start];
        out.push_str(opening_tag);
        out.push_str(&strip_inline_tags(inner));
        out.push_str("</pre>");
        rest = &rest[close_start + "</pre>".len()..];
    }
    out.push_str(rest);
    out
}

/// Removes `[§](#anchor)` section-link patterns docs.rs injects into headings.
fn strip_anchor_links(line: &str) -> String {
    let marker = "[\u{00a7}](#";
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find(marker) {
        out.push_str(&rest[..start]);
        rest = &rest[start + marker.len()..];
        if let Some(end) = rest.find(')') {
            rest = &rest[end + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// Post-processes converted markdown:
/// - Bare opening fences (```) default to ```rust
/// - Strips leading line numbers from scraped-example code blocks
/// - Strips `[§](#anchor)` links from heading lines
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
        } else if line.starts_with('#') {
            out.push_str(&strip_anchor_links(line));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_inline_tags_keeps_code_element_and_newlines() {
        assert_eq!(
            strip_inline_tags(
                "<code class=\"language-rust\"><span>foo</span>\n<span>bar</span></code>"
            ),
            "<code class=\"language-rust\">foo\nbar</code>"
        );
    }

    #[test]
    fn preprocess_pre_blocks_keeps_code_element() {
        let input = "<p>text</p><pre class=\"rust\"><code class=\"rust\"><span>/// Comment</span>\n<span>#[derive(Debug)]</span></code></pre><p>after</p>";
        let result = preprocess_pre_blocks(input);
        assert!(
            result.contains("<pre class=\"rust\"><code class=\"rust\">/// Comment\n#[derive(Debug)]</code></pre>"),
            "got: {result}"
        );
        assert!(result.contains("<p>text</p>"));
        assert!(result.contains("<p>after</p>"));
    }

    #[test]
    fn preprocess_pre_blocks_preserves_opening_tag_class() {
        let input = "<pre class=\"language-console\"><code class=\"language-console\"><span>$ cargo add clap</span></code></pre>";
        let result = preprocess_pre_blocks(input);
        assert_eq!(
            result,
            "<pre class=\"language-console\"><code class=\"language-console\">$ cargo add clap</code></pre>",
            "got: {result}"
        );
    }

    #[test]
    fn strip_anchor_links_removes_section_anchors() {
        assert_eq!(
            strip_anchor_links("### [\u{00a7}](#aspirations)Aspirations"),
            "### Aspirations"
        );
    }

    #[test]
    fn strip_anchor_links_leaves_plain_headings_unchanged() {
        assert_eq!(
            strip_anchor_links("### Normal Heading"),
            "### Normal Heading"
        );
    }

    #[test]
    fn postprocess_strips_heading_anchors() {
        let input = "### [\u{00a7}](#example)Example\n\nsome text\n";
        let result = postprocess_markdown(input);
        assert_eq!(result, "### Example\n\nsome text\n");
    }

    #[test]
    fn postprocess_preserves_code_fence_language() {
        let input = "```rust\nfn main() {}\n```\n";
        let result = postprocess_markdown(input);
        assert_eq!(result, "```rust\nfn main() {}\n```\n");
    }

    #[test]
    fn postprocess_defaults_bare_fence_to_rust() {
        let input = "```\nfn main() {}\n```\n";
        let result = postprocess_markdown(input);
        assert_eq!(result, "```rust\nfn main() {}\n```\n");
    }
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

    let content_html = preprocess_pre_blocks(&content_html);
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

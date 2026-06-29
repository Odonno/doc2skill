# doc2skill — Design Decisions

| Concern | Decision |
|---|---|
| Language / stack | Rust, tokio, clap |
| Input | `<crate>` or `<crate>@<version>` |
| Output | `.agents/skills/<name>/SKILL.md` + `references/*.md` |
| `--output` | Overrides base path |
| Crawl | Main page + 1 level deep, description section only (no sidebar, no module/struct/trait index) |
| HTML→MD | `scraper` (target DOM) + `htmd` (convert) |
| Frontmatter `name`/`description`/`license` | Pulled from crates.io API |
| Same-crate links | → `references/<page>.md` |
| Cross-crate links | → inline `use skill \`/<crate>\`` |
| External links | Keep as-is |
| Version pinning | `clap@4.5` → specific; omitted → latest |
| LLM | Dropped (YAGNI) |
| Errors | Fail fast, no partial output |

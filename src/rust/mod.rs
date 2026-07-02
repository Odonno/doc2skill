mod cargo;
mod fetch;
mod search;
mod target;

use crate::core::{LanguageProvider, SkillInfo};
use color_eyre::Result;
use target::CrateTarget;

pub struct RustProvider {
    client: reqwest::Client,
}

impl RustProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl LanguageProvider for RustProvider {
    fn language_name(&self) -> &'static str {
        "rust"
    }

    async fn fetch_info(&self, spec: &str) -> Result<SkillInfo> {
        let target = CrateTarget::parse(spec);
        fetch::fetch_crate(&self.client, &target).await
    }

    fn search_interactive(&self) -> Result<String> {
        search::select_crate()
    }

    fn read_project_deps(&self) -> Option<Result<Vec<String>>> {
        if !std::path::Path::new("Cargo.toml").exists() {
            return None;
        }
        Some(cargo::read_cargo_deps().map(|deps| deps.into_iter().map(|t| t.to_string()).collect()))
    }
}

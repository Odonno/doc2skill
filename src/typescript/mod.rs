mod fetch;
mod package;
mod search;
mod target;

use crate::core::{LanguageProvider, SkillInfo};
use color_eyre::Result;
use target::NpmTarget;

pub struct TypeScriptProvider {
    client: reqwest::Client,
}

impl TypeScriptProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl LanguageProvider for TypeScriptProvider {
    fn language_name(&self) -> &'static str {
        "typescript"
    }

    async fn fetch_info(&self, spec: &str) -> Result<SkillInfo> {
        let target = NpmTarget::parse(spec);
        fetch::fetch_package(&self.client, &target).await
    }

    fn search_interactive(&self) -> Result<String> {
        search::select_package()
    }

    fn read_project_deps(&self) -> Option<Result<Vec<String>>> {
        if !std::path::Path::new("package.json").exists() {
            return None;
        }
        Some(
            package::read_package_deps()
                .map(|deps| deps.into_iter().map(|t| t.to_string()).collect()),
        )
    }
}

mod fetch;
mod project;
mod search;
mod target;

use crate::core::{LanguageProvider, SkillInfo};
use color_eyre::Result;
use target::PackageTarget;

pub struct CSharpProvider {
    client: reqwest::Client,
}

impl CSharpProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl LanguageProvider for CSharpProvider {
    fn language_name(&self) -> &'static str {
        "csharp"
    }

    async fn fetch_info(&self, spec: &str) -> Result<SkillInfo> {
        let target = PackageTarget::parse(spec);
        fetch::fetch_package(&self.client, &target).await
    }

    fn search_interactive(&self) -> Result<String> {
        search::select_package()
    }

    fn read_project_deps(&self) -> Option<Result<Vec<String>>> {
        // Check for any .csproj or Directory.Packages.props nearby
        let has_csproj = std::fs::read_dir(".")
            .ok()?
            .flatten()
            .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("csproj"));
        let has_props = std::path::Path::new("Directory.Packages.props").exists();

        if !has_csproj && !has_props {
            return None;
        }

        Some(
            project::read_project_deps()
                .map(|deps| deps.into_iter().map(|t| t.to_string()).collect()),
        )
    }
}

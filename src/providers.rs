use color_eyre::Result;
use std::collections::BTreeMap;

use crate::cli::Language;
use crate::core::{LanguageProvider, SkillInfo};

pub enum AnyProvider {
    #[cfg(feature = "rust")]
    Rust(crate::rust::RustProvider),
    #[cfg(feature = "csharp")]
    Csharp(crate::csharp::CSharpProvider),
}

impl AnyProvider {
    pub async fn fetch_info(&self, spec: &str) -> Result<SkillInfo> {
        match self {
            #[cfg(feature = "rust")]
            Self::Rust(p) => p.fetch_info(spec).await,
            #[cfg(feature = "csharp")]
            Self::Csharp(p) => p.fetch_info(spec).await,
        }
    }

    pub fn read_project_deps(&self) -> Option<Result<Vec<String>>> {
        match self {
            #[cfg(feature = "rust")]
            Self::Rust(p) => p.read_project_deps(),
            #[cfg(feature = "csharp")]
            Self::Csharp(p) => p.read_project_deps(),
        }
    }

    pub fn search_interactive(&self) -> Result<String> {
        match self {
            #[cfg(feature = "rust")]
            Self::Rust(p) => p.search_interactive(),
            #[cfg(feature = "csharp")]
            Self::Csharp(p) => p.search_interactive(),
        }
    }
}

pub fn build_providers() -> BTreeMap<Language, AnyProvider> {
    let mut map = BTreeMap::new();
    #[cfg(feature = "rust")]
    map.insert(
        Language::Rust,
        AnyProvider::Rust(crate::rust::RustProvider::new()),
    );
    #[cfg(feature = "csharp")]
    map.insert(
        Language::Csharp,
        AnyProvider::Csharp(crate::csharp::CSharpProvider::new()),
    );
    map
}

use std::collections::BTreeMap;

use crate::cli::Language;
use crate::providers::AnyProvider;

pub struct PackageDetected {
    pub name: String,
    pub language: Language,
}

pub fn detect_all_deps(providers: &BTreeMap<Language, AnyProvider>) -> Vec<PackageDetected> {
    let mut all = Vec::new();
    for (lang, provider) in providers {
        if let Some(Ok(deps)) = provider.read_project_deps() {
            for dep in deps {
                all.push(PackageDetected {
                    name: dep,
                    language: lang.clone(),
                });
            }
        }
    }
    all
}

pub fn language_tag(lang: &Language) -> &'static str {
    match lang {
        #[cfg(feature = "rust")]
        Language::Rust => "rust",
        #[cfg(feature = "csharp")]
        Language::Csharp => "csharp",
        #[cfg(feature = "typescript")]
        Language::Typescript => "typescript",
    }
}

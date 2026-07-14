#[cfg(feature = "tokens")]
pub mod count;
pub mod warn;
pub mod write;

pub use warn::{SkillWarning, collect_warnings, print_warnings};
pub use write::write_skill;

use color_eyre::Result;
use std::collections::BTreeMap;

pub struct SkillPage {
    pub slug: String,
    #[allow(dead_code)]
    pub title: String,
    pub markdown: String,
}

pub struct SkillInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub author: String,
    pub page: SkillPage,
    pub references: BTreeMap<String, SkillPage>,
    /// Items grouped by category in declaration order (Structs, Enums, Traits, …).
    /// Each entry is `(display_name, slug)` where slug is the reference file stem.
    pub items: Vec<(String, Vec<(String, String)>)>,
}

pub trait LanguageProvider {
    #[allow(dead_code)]
    fn language_name(&self) -> &'static str;
    async fn fetch_info(&self, spec: &str) -> Result<SkillInfo>;
    fn search_interactive(&self) -> Result<String>;
    /// Returns `None` if no project file is found in the current directory.
    fn read_project_deps(&self) -> Option<Result<Vec<String>>>;
}

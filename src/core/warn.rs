use super::SkillInfo;
#[cfg(feature = "tokens")]
use super::count::{SKILL_TOKEN_WARN_THRESHOLD, count_text_tokens};

#[derive(Debug)]
pub enum SkillWarning {
    NoContent,
    #[cfg(feature = "tokens")]
    TooManyTokens(usize),
}

pub fn collect_warnings(info: &SkillInfo) -> Vec<SkillWarning> {
    let mut warnings = vec![];
    if info.page.markdown.trim().is_empty() {
        warnings.push(SkillWarning::NoContent);
    }
    #[cfg(feature = "tokens")]
    if let Ok(tokens) = count_text_tokens(&info.page.markdown)
        && tokens > SKILL_TOKEN_WARN_THRESHOLD
    {
        warnings.push(SkillWarning::TooManyTokens(tokens));
    }
    warnings
}

pub fn print_warnings(name: &str, warnings: &[SkillWarning]) {
    for w in warnings {
        let msg = match w {
            SkillWarning::NoContent => format!("\x1b[33m⚠ {name}: no content\x1b[0m"),
            #[cfg(feature = "tokens")]
            SkillWarning::TooManyTokens(tokens) => {
                format!(
                    "\x1b[33m⚠ {name}: skill content too large ({tokens} tokens > {})\x1b[0m",
                    SKILL_TOKEN_WARN_THRESHOLD
                )
            }
        };
        println!("{msg}");
    }
}

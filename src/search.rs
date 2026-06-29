use color_eyre::Result;
use inquire::{Autocomplete, CustomUserError, Text};
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(10);

/// Runs the interactive crate search prompt, returns the selected crate name.
pub fn select_crate() -> Result<String> {
    let selected = Text::new("Search crates.io:")
        .with_autocomplete(CrateSearch::new())
        .prompt()?;

    // Extract name from "name — description"
    let name = selected
        .split_once(" — ")
        .map(|(n, _)| n)
        .unwrap_or(&selected)
        .trim()
        .to_owned();

    Ok(name)
}

#[derive(Clone)]
struct CrateSearch {
    last_changed: Instant,
    cached: Vec<String>,
    fetched_for: String,
    client: reqwest::blocking::Client,
}

impl CrateSearch {
    fn new() -> Self {
        Self {
            // subtract DEBOUNCE so the very first keystroke is eligible to fetch
            last_changed: Instant::now() - DEBOUNCE,
            cached: Vec::new(),
            fetched_for: String::new(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn fetch(&self, input: &str) -> Vec<String> {
        let Ok(resp) = self
            .client
            .get("https://crates.io/api/v1/crates")
            .query(&[("q", input), ("per_page", "20")])
            .header("User-Agent", "doc2skill/0.1")
            .send()
        else {
            return vec![];
        };
        let Ok(json) = resp.json::<serde_json::Value>() else {
            return vec![];
        };
        json["crates"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|c| {
                let name = c["name"].as_str().unwrap_or("");
                let desc = c["description"].as_str().unwrap_or("").trim();
                format!("{name} — {desc}")
            })
            .collect()
    }
}

impl Autocomplete for CrateSearch {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        if input.is_empty() {
            return Ok(vec![]);
        }
        let now = Instant::now();
        let since_last = now.duration_since(self.last_changed);
        self.last_changed = now;

        // ponytail: fires on first keystroke after a 10ms pause, not true trailing debounce
        // (true trailing debounce needs a background thread; add if UX demands it)
        if input != self.fetched_for && since_last >= DEBOUNCE {
            self.cached = self.fetch(input);
            self.fetched_for = input.to_owned();
        }

        Ok(self.cached.clone())
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted: Option<String>,
    ) -> Result<Option<String>, CustomUserError> {
        Ok(highlighted)
    }
}

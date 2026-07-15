use color_eyre::Result;
use inquire::{Autocomplete, CustomUserError, Text};
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(10);

/// Runs the interactive npm package search prompt, returns the selected package name.
pub fn select_package() -> Result<String> {
    let selected = Text::new("Search npm:")
        .with_autocomplete(NpmSearch::new())
        .prompt()?;

    let name = selected
        .split_once(" — ")
        .map(|(n, _)| n)
        .unwrap_or(&selected)
        .trim()
        .to_owned();

    Ok(name)
}

#[derive(Clone)]
struct NpmSearch {
    last_changed: Instant,
    cached: Vec<String>,
    fetched_for: String,
    client: reqwest::blocking::Client,
}

impl NpmSearch {
    fn new() -> Self {
        Self {
            last_changed: Instant::now() - DEBOUNCE,
            cached: Vec::new(),
            fetched_for: String::new(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn fetch(&self, input: &str) -> Vec<String> {
        let Ok(resp) = self
            .client
            .get("https://registry.npmjs.org/-/v1/search")
            .query(&[("text", input), ("size", "20")])
            .header("User-Agent", "doc2skill/0.1")
            .send()
        else {
            return vec![];
        };
        let Ok(json) = resp.json::<serde_json::Value>() else {
            return vec![];
        };
        json["objects"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|c| {
                let name = c["package"]["name"].as_str().unwrap_or("");
                let desc = c["package"]["description"].as_str().unwrap_or("").trim();
                format!("{name} — {desc}")
            })
            .collect()
    }
}

impl Autocomplete for NpmSearch {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        if input.is_empty() {
            return Ok(vec![]);
        }
        let now = Instant::now();
        let since_last = now.duration_since(self.last_changed);
        self.last_changed = now;

        // ponytail: fires on first keystroke after a 10ms pause, not true trailing debounce
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

use color_eyre::Result;
use inquire::{Autocomplete, CustomUserError, Text};
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(10);

pub fn select_package() -> Result<String> {
    let selected = Text::new("Search NuGet:")
        .with_autocomplete(PackageSearch::new())
        .prompt()?;

    // Extract ID from "ID — description"
    let id = selected
        .split_once(" — ")
        .map(|(n, _)| n)
        .unwrap_or(&selected)
        .trim()
        .to_owned();

    Ok(id)
}

#[derive(Clone)]
struct PackageSearch {
    last_changed: Instant,
    cached: Vec<String>,
    fetched_for: String,
    client: reqwest::blocking::Client,
}

impl PackageSearch {
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
            .get("https://azuresearch-usnc.nuget.org/query")
            .query(&[("q", input), ("take", "20")])
            .header("User-Agent", "doc2skill/0.1")
            .send()
        else {
            return vec![];
        };
        let Ok(json) = resp.json::<serde_json::Value>() else {
            return vec![];
        };
        json["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|p| {
                let id = p["id"].as_str().unwrap_or("");
                let desc = p["description"].as_str().unwrap_or("").trim();
                format!("{id} — {desc}")
            })
            .collect()
    }
}

impl Autocomplete for PackageSearch {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        if input.is_empty() {
            return Ok(vec![]);
        }
        let now = Instant::now();
        let since_last = now.duration_since(self.last_changed);
        self.last_changed = now;

        // ponytail: same naive debounce as the Rust provider — add background thread if UX demands it
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

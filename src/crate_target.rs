pub struct CrateTarget {
    pub name: String,
    pub version: Option<String>,
}

impl CrateTarget {
    pub fn parse(spec: &str) -> CrateTarget {
        match spec.split_once('@') {
            Some((name, version)) => CrateTarget {
                name: name.to_string(),
                version: Some(version.to_string()),
            },
            None => CrateTarget {
                name: spec.to_string(),
                version: None,
            },
        }
    }
}

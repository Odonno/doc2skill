use std::fmt;

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

impl fmt::Display for CrateTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}@{}", self.name, v),
            None => write!(f, "{}", self.name),
        }
    }
}

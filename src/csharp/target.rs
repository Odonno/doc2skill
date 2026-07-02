use std::fmt;

pub struct PackageTarget {
    pub name: String,
    pub version: Option<String>,
}

impl PackageTarget {
    pub fn parse(spec: &str) -> Self {
        match spec.split_once('@') {
            Some((name, version)) => Self {
                name: name.to_string(),
                version: Some(version.to_string()),
            },
            None => Self {
                name: spec.to_string(),
                version: None,
            },
        }
    }

    /// NuGet API requires lowercase IDs for flat-container endpoints.
    pub fn id_lower(&self) -> String {
        self.name.to_lowercase()
    }
}

impl fmt::Display for PackageTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}@{}", self.name, v),
            None => write!(f, "{}", self.name),
        }
    }
}

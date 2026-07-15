use std::fmt;

pub struct NpmTarget {
    pub name: String,
    pub version: Option<String>,
}

impl NpmTarget {
    pub fn parse(spec: &str) -> Self {
        // Scoped packages: @scope/name[@version] — the version @ comes after the first slash.
        if let Some(rest) = spec.strip_prefix('@') {
            if let Some(at_in_rest) = rest.find('@') {
                let name_end = 1 + at_in_rest; // position in `spec` of the version @
                return Self {
                    name: spec[..name_end].to_string(),
                    version: Some(spec[name_end + 1..].to_string()),
                };
            }
            return Self {
                name: spec.to_string(),
                version: None,
            };
        }
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
}

impl fmt::Display for NpmTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}@{}", self.name, v),
            None => write!(f, "{}", self.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain() {
        let t = NpmTarget::parse("react");
        assert_eq!(t.name, "react");
        assert_eq!(t.version, None);
    }

    #[test]
    fn parses_versioned() {
        let t = NpmTarget::parse("react@19.0.0");
        assert_eq!(t.name, "react");
        assert_eq!(t.version, Some("19.0.0".to_owned()));
    }

    #[test]
    fn parses_scoped() {
        let t = NpmTarget::parse("@angular/core");
        assert_eq!(t.name, "@angular/core");
        assert_eq!(t.version, None);
    }

    #[test]
    fn parses_scoped_versioned() {
        let t = NpmTarget::parse("@angular/core@17.0.0");
        assert_eq!(t.name, "@angular/core");
        assert_eq!(t.version, Some("17.0.0".to_owned()));
    }
}

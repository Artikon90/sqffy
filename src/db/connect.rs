use crate::error::{Error, Result};
use crate::i18n::tr;

/// Parsed connection request — driver-agnostic.
///
/// Accepted inputs:
/// - plain path → SQLite file (`./voice.db`)
/// - `:memory:` → SQLite in-memory
/// - URL `sqlite:…` / `sqlite://…` / `file:…`
/// - URL `postgres://…` / `postgresql://…`
/// - `--driver <id>` + positional target
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionSpec {
    pub driver: String,
    /// Path, db name, or opaque target for the driver.
    pub target: String,
    /// Original URL if the user passed one (drivers may prefer it).
    pub url: Option<String>,
}

impl ConnectionSpec {
    pub fn parse(target: Option<&str>, driver_override: Option<&str>) -> Result<Self> {
        let Some(raw) = target.map(str::trim).filter(|s| !s.is_empty()) else {
            let driver = driver_override.unwrap_or("sqlite").to_ascii_lowercase();
            return Ok(Self {
                driver,
                target: ":memory:".into(),
                url: None,
            });
        };

        if let Some(spec) = parse_url(raw)? {
            if let Some(over) = driver_override {
                return Ok(Self {
                    driver: over.to_ascii_lowercase(),
                    target: spec.target,
                    url: spec.url,
                });
            }
            return Ok(spec);
        }

        let driver = driver_override.unwrap_or("sqlite").to_ascii_lowercase();
        Ok(Self {
            driver,
            target: raw.to_string(),
            url: None,
        })
    }
}

fn parse_url(raw: &str) -> Result<Option<ConnectionSpec>> {
    let lower = raw.to_ascii_lowercase();

    if lower == ":memory:" || lower == "sqlite::memory:" {
        return Ok(Some(ConnectionSpec {
            driver: "sqlite".into(),
            target: ":memory:".into(),
            url: Some(raw.to_string()),
        }));
    }

    if let Some(rest) = strip_scheme(&lower, &["sqlite://", "sqlite:", "file:"]) {
        // Preserve original path casing from `raw` after the scheme.
        let original_rest = strip_scheme_raw(raw, &["sqlite://", "sqlite:", "file:"])
            .unwrap_or(rest);
        let target = normalize_sqlite_path(original_rest);
        return Ok(Some(ConnectionSpec {
            driver: "sqlite".into(),
            target,
            url: Some(raw.to_string()),
        }));
    }

    if strip_scheme(&lower, &["postgres://", "postgresql://"]).is_some() {
        return Ok(Some(ConnectionSpec {
            driver: "postgres".into(),
            target: raw.to_string(),
            url: Some(raw.to_string()),
        }));
    }

    if strip_scheme(&lower, &["mysql://"]).is_some() {
        return Ok(Some(ConnectionSpec {
            driver: "mysql".into(),
            target: raw.to_string(),
            url: Some(raw.to_string()),
        }));
    }

    // Unknown scheme → leave to plain-path handling.
    if raw.contains("://") {
        let scheme = raw.split("://").next().unwrap_or("");
        return Err(Error::msg(tr(
            "drivers.unknown_scheme",
            &[("scheme", scheme)],
        )));
    }

    Ok(None)
}

fn strip_scheme<'a>(lower: &'a str, schemes: &[&str]) -> Option<&'a str> {
    for scheme in schemes {
        if let Some(rest) = lower.strip_prefix(scheme) {
            return Some(rest);
        }
    }
    None
}

fn strip_scheme_raw<'a>(raw: &'a str, schemes: &[&str]) -> Option<&'a str> {
    let lower = raw.to_ascii_lowercase();
    for scheme in schemes {
        if lower.starts_with(scheme) {
            return Some(&raw[scheme.len()..]);
        }
    }
    None
}

fn normalize_sqlite_path(rest: &str) -> String {
    // sqlite:///C:/path or sqlite:///absolute
    if let Some(stripped) = rest.strip_prefix("//") {
        if stripped.starts_with('/') {
            return stripped.to_string();
        }
        // sqlite://relative → treat as relative path
        return stripped.to_string();
    }
    rest.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_path_defaults_to_sqlite() {
        let spec = ConnectionSpec::parse(Some("data/voice.db"), None).unwrap();
        assert_eq!(spec.driver, "sqlite");
        assert_eq!(spec.target, "data/voice.db");
    }

    #[test]
    fn postgres_url() {
        let spec =
            ConnectionSpec::parse(Some("postgres://u:p@localhost:5432/app"), None).unwrap();
        assert_eq!(spec.driver, "postgres");
    }

    #[test]
    fn driver_override() {
        let spec = ConnectionSpec::parse(Some("localhost/app"), Some("postgres")).unwrap();
        assert_eq!(spec.driver, "postgres");
    }
}

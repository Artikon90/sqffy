//! Lightweight localization.
//!
//! Locales live in `/locales/{lang}.toml` (nested tables) and are embedded at
//! compile time. Keys are flattened to dotted paths (`app.name`, `picker.title`).
//!
//! Language resolution (first wins):
//! 1. explicit `init(Some(lang))` / `--lang`
//! 2. `SQFFY_LANG`
//! 3. `LANG` / `LC_ALL` / `LC_MESSAGES`
//! 4. `en`

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static CATALOG: OnceLock<RwLock<Catalog>> = OnceLock::new();

#[derive(Debug)]
struct Catalog {
    lang: String,
    strings: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Catalog {
    fn lookup(&self, key: &str) -> String {
        self.strings
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}

fn flatten_toml(value: &toml::Value, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_toml(child, &next, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

fn load_embedded(lang: &str) -> HashMap<String, String> {
    let raw = match lang {
        "ru" => include_str!("../locales/ru.toml"),
        "zh" => include_str!("../locales/zh.toml"),
        _ => include_str!("../locales/en.toml"),
    };
    let value = match raw.parse::<toml::Value>() {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let mut out = HashMap::new();
    flatten_toml(&value, "", &mut out);
    out
}

fn normalize_lang(raw: &str) -> String {
    let primary = raw
        .split(['.', '_', '-'])
        .next()
        .unwrap_or("en")
        .to_ascii_lowercase();
    match primary.as_str() {
        "ru" => "ru".into(),
        "zh" | "zhcn" | "zhtw" | "cn" => "zh".into(),
        _ => "en".into(),
    }
}

/// Detect language from explicit value or environment.
pub fn detect(explicit: Option<&str>) -> String {
    if let Some(lang) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return normalize_lang(lang);
    }
    if let Ok(lang) = std::env::var("SQFFY_LANG") {
        if !lang.trim().is_empty() {
            return normalize_lang(&lang);
        }
    }
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(lang) = std::env::var(key) {
            if !lang.trim().is_empty() && lang != "C" && !lang.eq_ignore_ascii_case("POSIX") {
                return normalize_lang(&lang);
            }
        }
    }
    "en".into()
}

/// Initialize global catalog. Call once at startup before printing UI.
pub fn init(explicit: Option<&str>) {
    let lang = detect(explicit);
    let catalog = Catalog {
        lang: lang.clone(),
        strings: load_embedded(&lang),
        fallback: load_embedded("en"),
    };
    if let Some(lock) = CATALOG.get() {
        if let Ok(mut guard) = lock.write() {
            *guard = catalog;
        }
    } else {
        let _ = CATALOG.set(RwLock::new(catalog));
    }
}

fn with_catalog<R>(f: impl FnOnce(&Catalog) -> R) -> R {
    let cell = CATALOG.get_or_init(|| {
        let lang = detect(None);
        RwLock::new(Catalog {
            lang: lang.clone(),
            strings: load_embedded(&lang),
            fallback: load_embedded("en"),
        })
    });
    let guard = cell.read().expect("i18n catalog poisoned");
    f(&guard)
}

pub fn lang() -> String {
    with_catalog(|c| c.lang.clone())
}

/// Translate a key.
pub fn t(key: &str) -> String {
    with_catalog(|c| c.lookup(key))
}

/// Translate and replace `{name}` placeholders.
pub fn tr(key: &str, args: &[(&str, &str)]) -> String {
    let mut out = t(key);
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_langs() {
        assert_eq!(normalize_lang("ru_RU.UTF-8"), "ru");
        assert_eq!(normalize_lang("en-US"), "en");
        assert_eq!(normalize_lang("zh_CN.UTF-8"), "zh");
        assert_eq!(normalize_lang("zh-Hans"), "zh");
    }

    #[test]
    fn locale_catalogs_resolve() {
        // Sequential: catalog is process-global.
        init(Some("en"));
        assert_eq!(t("app.name"), "SQffy");
        assert!(t("picker.title").contains("Select"), "{}", t("picker.title"));

        init(Some("ru"));
        assert!(t("picker.title").contains("Выберите"), "{}", t("picker.title"));

        init(Some("zh"));
        assert!(t("picker.title").contains("选择"), "{}", t("picker.title"));
    }
}

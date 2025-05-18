//! Persistent connection catalog (`connections.toml`).

use crate::db::ConnectionSpec;
use crate::error::{Error, Result};
use crate::i18n::{t, tr};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConnection {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<u64>,
}

impl SavedConnection {
    pub fn to_spec(&self) -> ConnectionSpec {
        ConnectionSpec {
            driver: self.driver.clone(),
            target: self.target.clone(),
            url: self.url.clone(),
        }
    }

    pub fn touch(&mut self) {
        self.last_used = Some(now_secs());
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStore {
    #[serde(default)]
    pub connections: Vec<SavedConnection>,
}

impl ConnectionStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)?;
        let store: Self = toml::from_str(&text).map_err(|e| {
            Error::msg(tr(
                "connect.invalid_store",
                &[("path", &path.display().to_string()), ("error", &e.to_string())],
            ))
        })?;
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        let path = store_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|e| Error::msg(tr("connect.serialize_error", &[("error", &e.to_string())])))?;
        fs::write(&path, text)?;
        Ok(())
    }

    pub fn upsert(&mut self, conn: SavedConnection) {
        if let Some(existing) = self.connections.iter_mut().find(|c| c.id == conn.id) {
            *existing = conn;
        } else if let Some(existing) = self
            .connections
            .iter_mut()
            .find(|c| c.name.eq_ignore_ascii_case(&conn.name))
        {
            *existing = conn;
        } else {
            self.connections.push(conn);
        }
    }

    pub fn mark_used(&mut self, id: &str) -> Result<()> {
        if let Some(conn) = self.connections.iter_mut().find(|c| c.id == id) {
            conn.touch();
            self.save()?;
        }
        Ok(())
    }

    pub fn sorted(&self) -> Vec<&SavedConnection> {
        let mut items: Vec<_> = self.connections.iter().collect();
        items.sort_by(|a, b| b.last_used.cmp(&a.last_used).then(a.name.cmp(&b.name)));
        items
    }
}

/// Override root for config/data (`connections.toml`, `databases/`).
/// Used by e2e tests so the real user catalog is never touched.
fn home_override() -> Option<PathBuf> {
    std::env::var_os("SQFFY_HOME").map(PathBuf::from)
}

pub fn store_path() -> Result<PathBuf> {
    if let Some(home) = home_override() {
        return Ok(home.join("connections.toml"));
    }
    let dirs = ProjectDirs::from("dev", "SQffy", "sqffy")
        .ok_or_else(|| Error::msg(t("connect.config_dir_error")))?;
    Ok(dirs.config_dir().join("connections.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    let path = if let Some(home) = home_override() {
        home.join("databases")
    } else {
        let dirs = ProjectDirs::from("dev", "SQffy", "sqffy")
            .ok_or_else(|| Error::msg(t("connect.data_dir_error")))?;
        dirs.data_dir().join("databases")
    };
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn make_id(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        format!("db-{}", now_secs())
    } else {
        format!("{slug}-{}", now_secs() % 10_000)
    }
}

pub fn new_saved(
    name: impl Into<String>,
    driver: impl Into<String>,
    target: impl Into<String>,
    url: Option<String>,
) -> SavedConnection {
    let name = name.into();
    SavedConnection {
        id: make_id(&name),
        name,
        driver: driver.into(),
        target: target.into(),
        url,
        last_used: Some(now_secs()),
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn resolve_sqlite_path(input: &str) -> PathBuf {
    let path = PathBuf::from(input);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

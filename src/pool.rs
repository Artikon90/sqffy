//! Shared named database connections for REPL + MCP.

use crate::db::{default_registry, ConnectionSpec, Database, DriverRegistry};
use crate::error::{Error, Result};
use crate::connections::ConnectionStore;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ConnectionMeta {
    pub name: String,
    pub driver: String,
    pub label: String,
}

pub struct PooledConnection {
    pub meta: ConnectionMeta,
    pub db: Mutex<Box<dyn Database>>,
}

/// Thread-safe pool of open databases.
#[derive(Clone, Default)]
pub struct ConnectionPool {
    inner: Arc<Mutex<HashMap<String, Arc<PooledConnection>>>>,
    default: Arc<Mutex<Option<String>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, name: impl Into<String>, db: Box<dyn Database>) {
        let name = name.into();
        let meta = ConnectionMeta {
            name: name.clone(),
            driver: db.driver_id().to_string(),
            label: db.display_label().to_string(),
        };
        let pooled = Arc::new(PooledConnection {
            meta,
            db: Mutex::new(db),
        });
        let mut map = self.inner.lock().expect("pool lock");
        map.insert(name.clone(), pooled);
        let mut def = self.default.lock().expect("default lock");
        if def.is_none() {
            *def = Some(name);
        }
    }

    pub fn set_default(&self, name: &str) -> Result<()> {
        let map = self.inner.lock().expect("pool lock");
        if !map.contains_key(name) {
            return Err(Error::msg(format!("unknown connection: {name}")));
        }
        *self.default.lock().expect("default lock") = Some(name.to_string());
        Ok(())
    }

    pub fn get(&self, name: Option<&str>) -> Result<Arc<PooledConnection>> {
        let map = self.inner.lock().expect("pool lock");
        let key = match name {
            Some(n) => n.to_string(),
            None => self
                .default
                .lock()
                .expect("default lock")
                .clone()
                .ok_or_else(|| Error::msg("no open connections"))?,
        };
        map.get(&key)
            .cloned()
            .ok_or_else(|| Error::msg(format!("unknown connection: {key}")))
    }

    pub fn list(&self) -> Vec<ConnectionMeta> {
        let map = self.inner.lock().expect("pool lock");
        let def = self.default.lock().expect("default lock").clone();
        let mut items: Vec<_> = map.values().map(|p| p.meta.clone()).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        // Mark default by putting it first
        if let Some(d) = def {
            if let Some(pos) = items.iter().position(|m| m.name == d) {
                let def_item = items.remove(pos);
                items.insert(0, def_item);
            }
        }
        items
    }

    pub fn len(&self) -> usize {
        self.inner.lock().expect("pool lock").len()
    }

    /// Open all saved connections from the store (best-effort; failures skipped).
    pub fn load_saved(&self, store: &ConnectionStore, registry: &DriverRegistry) -> Vec<String> {
        let mut errors = Vec::new();
        for conn in &store.connections {
            match registry.open(&conn.to_spec()) {
                Ok(db) => self.insert(&conn.name, db),
                Err(err) => errors.push(format!("{}: {err}", conn.name)),
            }
        }
        errors
    }

    pub fn open_spec(&self, name: &str, spec: &ConnectionSpec) -> Result<()> {
        let registry = default_registry();
        let db = registry.open(spec)?;
        self.insert(name, db);
        Ok(())
    }
}

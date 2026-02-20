use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::traits::Database;
use crate::error::{Error, Result};
use crate::i18n::tr;
use std::collections::HashMap;
use std::sync::Arc;

/// Lookup table of available backends.
#[derive(Clone, Default)]
pub struct DriverRegistry {
    by_id: HashMap<&'static str, Arc<dyn Driver>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, driver: Arc<dyn Driver>) {
        let id = driver.id();
        for alias in driver.aliases() {
            self.by_id.insert(*alias, Arc::clone(&driver));
        }
        self.by_id.insert(id, driver);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Driver>> {
        self.by_id.get(id)
    }

    pub fn list(&self) -> Vec<(&'static str, &'static str)> {
        let mut seen = HashMap::new();
        for driver in self.by_id.values() {
            seen.insert(driver.id(), driver.description());
        }
        let mut items: Vec<_> = seen.into_iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        items
    }

    pub fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        let driver = self.get(&spec.driver).ok_or_else(|| {
            let known: Vec<_> = self.list().into_iter().map(|(id, _)| id).collect();
            Error::msg(tr(
                "drivers.unknown",
                &[("driver", &spec.driver), ("known", &known.join(", "))],
            ))
        })?;
        driver.open(spec)
    }
}

/// Built-in drivers enabled by Cargo features.
pub fn default_registry() -> DriverRegistry {
    let mut registry = DriverRegistry::new();

    #[cfg(feature = "sqlite")]
    registry.register(Arc::new(super::sqlite::SqliteDriver));

    #[cfg(feature = "postgres")]
    registry.register(Arc::new(super::postgres::PostgresDriver));

    #[cfg(feature = "mysql")]
    registry.register(Arc::new(super::mysql::MysqlDriver));

    #[cfg(feature = "mariadb")]
    registry.register(Arc::new(super::mariadb::MariadbDriver));

    #[cfg(feature = "clickhouse")]
    registry.register(Arc::new(super::clickhouse::ClickhouseDriver));

    #[cfg(feature = "oracle")]
    registry.register(Arc::new(super::oracle::OracleDriver));

    registry
}

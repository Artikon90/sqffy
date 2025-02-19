use super::connect::ConnectionSpec;
use super::provision::{LocalStatus, ProvisionInfo};
use super::traits::Database;
use crate::error::Result;

/// Factory for a database backend.
///
/// To add a database:
/// 1. Implement [`Database`] for your session type.
/// 2. Implement [`Driver`] that opens it from [`ConnectionSpec`].
/// 3. Register it in [`super::registry::default_registry`] (or at runtime).
pub trait Driver: Send + Sync {
    /// Canonical id used in `--driver` and URLs (`sqlite`, `postgres`, …).
    fn id(&self) -> &'static str;

    /// Extra names that resolve to this driver (`postgresql` → `postgres`).
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn description(&self) -> &'static str;

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>>;

    /// How to create / provision this database type locally.
    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo::default()
    }

    /// Probe whether a usable server is already on this machine.
    fn probe_local(&self) -> LocalStatus {
        LocalStatus::Missing
    }
}

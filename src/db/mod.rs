//! Pluggable database backends.
//!
//! ```text
//! ConnectionSpec ──▶ DriverRegistry ──▶ Box<dyn Database>
//!                         ▲
//!            SqliteDriver ┤
//!          PostgresDriver ┤  (feature-gated)
//!             MysqlDriver ┤
//!           MariadbDriver ┤
//!        ClickhouseDriver ┤
//!            OracleDriver ┘
//! ```
//!
//! App / meta / render / complete only depend on [`Database`].
//! To add a backend, see `docs/DRIVERS.md`.

mod connect;
mod driver;
pub(crate) mod probe;
mod provision;
mod registry;
mod traits;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(any(feature = "mysql", feature = "mariadb"))]
mod mysql;

#[cfg(feature = "mariadb")]
mod mariadb;

#[cfg(feature = "clickhouse")]
mod clickhouse;

#[cfg(feature = "oracle")]
mod oracle;

pub use connect::ConnectionSpec;
pub use driver::Driver;
pub use provision::{CreateKind, DockerSpec, LocalStatus, ProvisionInfo};
pub use registry::{default_registry, DriverRegistry};
pub use traits::{
    Capabilities, ColumnInfo, ConstraintInfo, Database, IndexInfo, QueryResult, Row, TableInfo,
    TableSchema,
};

#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteDb, SqliteDriver};

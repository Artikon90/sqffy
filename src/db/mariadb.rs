//! MariaDB driver — MySQL wire protocol with MariaDB provision recipes.

use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::mysql::open_mysql_url;
use super::probe;
use super::provision::{CreateKind, DockerSpec, LocalStatus, ProvisionInfo};
use super::traits::Database;
use crate::error::Result;

pub struct MariadbDriver;

impl Driver for MariadbDriver {
    fn id(&self) -> &'static str {
        "mariadb"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["maria"]
    }

    fn description(&self) -> &'static str {
        "MariaDB"
    }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        open_mysql_url(spec, "mariadb")
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::Server,
            default_port: Some(3306),
            docker: Some(DockerSpec {
                image: "mariadb:11.4",
                env: &[
                    ("MARIADB_ROOT_PASSWORD", "sqffy"),
                    ("MARIADB_DATABASE", "sqffy"),
                    ("MARIADB_USER", "sqffy"),
                    ("MARIADB_PASSWORD", "sqffy"),
                ],
                ports: &[(3306, 3306)],
                name_prefix: "sqffy-mariadb",
                url_template: "mariadb://sqffy:sqffy@{host}:{port}/sqffy",
            }),
            download_url: Some("https://mariadb.org/download/"),
            winget_id: Some("MariaDB.Server"),
            help_key: "provision.help.mariadb",
        }
    }

    fn probe_local(&self) -> LocalStatus {
        // Same default port family as MySQL; probe still helps when something listens.
        probe::tcp_localhost(3306)
    }
}

//! Oracle Database driver (ODPI-C via the `oracle` crate).
//!
//! Requires Oracle Instant Client (or a full client) on the machine at runtime.

use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::probe;
use super::provision::{CreateKind, DockerSpec, LocalStatus, ProvisionInfo};
use super::traits::{
    Capabilities, ColumnInfo, ConstraintInfo, Database, IndexInfo, QueryResult, TableInfo,
    TableSchema,
};
use crate::error::{Error, Result};
use oracle::Connection;
use std::cell::RefCell;

pub struct OracleDriver;

impl Driver for OracleDriver {
    fn id(&self) -> &'static str {
        "oracle"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["oracledb", "ora"]
    }

    fn description(&self) -> &'static str {
        "Oracle Database"
    }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        let raw = spec.url.clone().unwrap_or_else(|| spec.target.clone());
        let (user, password, connect) = parse_oracle_url(&raw)?;
        let conn = Connection::connect(&user, &password, &connect).map_err(ora_err)?;
        Ok(Box::new(OracleDb {
            conn: RefCell::new(conn),
            label: redact_label(&user, &connect),
            user,
        }))
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::Server,
            default_port: Some(1521),
            docker: Some(DockerSpec {
                image: "gvenzl/oracle-xe:21-slim",
                env: &[
                    ("ORACLE_PASSWORD", "sqffy"),
                    ("APP_USER", "sqffy"),
                    ("APP_USER_PASSWORD", "sqffy"),
                ],
                ports: &[(1521, 1521)],
                name_prefix: "sqffy-oracle",
                url_template: "oracle://sqffy:sqffy@{host}:{port}/XEPDB1",
            }),
            download_url: Some(
                "https://www.oracle.com/database/technologies/instant-client.html",
            ),
            winget_id: None,
            help_key: "provision.help.oracle",
        }
    }

    fn probe_local(&self) -> LocalStatus {
        probe::tcp_localhost(1521)
    }
}

pub struct OracleDb {
    conn: RefCell<Connection>,
    label: String,
    user: String,
}

impl Database for OracleDb {
    fn driver_id(&self) -> &'static str {
        "oracle"
    }

    fn display_label(&self) -> &str {
        &self.label
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            schemas: true,
            table_ddl: false,
            quote_char: '"',
        }
    }

    fn execute_sql(&mut self, sql: &str, row_limit: usize) -> Result<QueryResult> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Ok(QueryResult::empty_affected(0));
        }
        let conn = self.conn.borrow();
        if looks_like_query(trimmed) {
            let mut stmt = conn.statement(trimmed).build().map_err(ora_err)?;
            let rows = stmt.query(&[]).map_err(ora_err)?;
            let columns: Vec<String> = rows
                .column_info()
                .iter()
                .map(|c| c.name().to_string())
                .collect();
            let column_types: Vec<String> = rows
                .column_info()
                .iter()
                .map(|c| format!("{:?}", c.oracle_type()))
                .collect();
            let mut out_rows = Vec::new();
            let mut truncated = false;
            for (i, item) in rows.enumerate() {
                let row = item.map_err(ora_err)?;
                if row_limit > 0 && i >= row_limit {
                    truncated = true;
                    break;
                }
                let mut cells = Vec::with_capacity(columns.len());
                for j in 0..columns.len() {
                    cells.push(cell_to_string(&row, j));
                }
                out_rows.push(cells);
            }
            Ok(QueryResult {
                columns,
                column_types,
                rows: out_rows,
                truncated,
                rows_affected: None,
            })
        } else {
            let affected = conn
                .execute(trimmed, &[])
                .map_err(ora_err)?
                .row_count()
                .map_err(ora_err)?;
            let _ = conn.commit();
            Ok(QueryResult::empty_affected(affected))
        }
    }

    fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let conn = self.conn.borrow();
        let owner = self.user.to_ascii_uppercase();
        let mut out = Vec::new();
        let rows = conn
            .query_as::<String>("SELECT table_name FROM user_tables ORDER BY table_name", &[])
            .map_err(ora_err)?;
        for item in rows {
            out.push(TableInfo {
                name: item.map_err(ora_err)?,
                kind: "table".into(),
                schema: Some(owner.clone()),
            });
        }
        let views = conn
            .query_as::<String>("SELECT view_name FROM user_views ORDER BY view_name", &[])
            .map_err(ora_err)?;
        for item in views {
            out.push(TableInfo {
                name: item.map_err(ora_err)?,
                kind: "view".into(),
                schema: Some(owner.clone()),
            });
        }
        Ok(out)
    }

    fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let name = table
            .split('.')
            .next_back()
            .unwrap_or(table)
            .to_ascii_uppercase();
        validate_ident(&name)?;
        let conn = self.conn.borrow();
        let sql = "SELECT column_name, data_type, nullable, data_default
                   FROM user_tab_columns
                   WHERE table_name = :1
                   ORDER BY column_id";
        let rows = conn
            .query_as::<(String, String, String, Option<String>)>(sql, &[&name])
            .map_err(ora_err)?;
        let mut cols = Vec::new();
        for item in rows {
            let (cname, ty, nullable, def) = item.map_err(ora_err)?;
            cols.push(ColumnInfo {
                name: cname,
                type_name: ty,
                not_null: nullable == "N",
                default_value: def.map(|s| s.trim().to_string()),
                primary_key: false,
                comment: None,
            });
        }
        if cols.is_empty() {
            return Err(Error::msg(format!("table not found: {table}")));
        }
        let pk_sql = "SELECT column_name FROM user_cons_columns
                      WHERE constraint_name IN (
                        SELECT constraint_name FROM user_constraints
                        WHERE table_name = :1 AND constraint_type = 'P'
                      )";
        let pk_rows = conn.query_as::<String>(pk_sql, &[&name]).map_err(ora_err)?;
        let mut pks = Vec::new();
        for item in pk_rows {
            pks.push(item.map_err(ora_err)?);
        }
        for c in &mut cols {
            if pks.iter().any(|p| p == &c.name) {
                c.primary_key = true;
            }
        }
        Ok(cols)
    }

    fn table_ddl(&self, _table: &str) -> Result<Option<String>> {
        Ok(None)
    }

    fn inspect_table(&self, table: &str) -> Result<TableSchema> {
        let columns = self.describe_table(table)?;
        let mut constraints = Vec::new();
        let pk: Vec<_> = columns
            .iter()
            .filter(|c| c.primary_key)
            .map(|c| c.name.as_str())
            .collect();
        if !pk.is_empty() {
            constraints.push(ConstraintInfo {
                kind: "PRIMARY KEY".into(),
                name: "PK".into(),
                definition: pk.join(", "),
            });
        }
        Ok(TableSchema {
            name: table.to_string(),
            kind: "table".into(),
            columns,
            constraints,
            indexes: Vec::<IndexInfo>::new(),
            ddl: None,
        })
    }
}

fn parse_oracle_url(raw: &str) -> Result<(String, String, String)> {
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("oracle://")
        || lower.starts_with("oracledb://")
        || lower.starts_with("ora://")
    {
        let scheme_len = raw.find("://").map(|i| i + 3).unwrap_or(0);
        let rest_orig = &raw[scheme_len..];
        let (auth_host, service) = rest_orig
            .split_once('/')
            .map(|(a, b)| (a, b.split('?').next().unwrap_or(b)))
            .unwrap_or((rest_orig, "XEPDB1"));
        let (user, password, hostport) = if let Some((creds, hp)) = auth_host.split_once('@') {
            let (u, p) = creds.split_once(':').unwrap_or((creds, ""));
            (u.to_string(), p.to_string(), hp)
        } else {
            return Err(Error::msg(
                "oracle URL needs user:password@host:port/service",
            ));
        };
        let connect = if hostport.starts_with("//") {
            format!("{hostport}/{service}")
        } else {
            format!("//{hostport}/{service}")
        };
        return Ok((user, password, connect));
    }

    if let Some((creds, connect)) = raw.split_once('@') {
        let (user, password) = creds.split_once('/').unwrap_or((creds, ""));
        return Ok((user.into(), password.into(), connect.into()));
    }

    Err(Error::msg(format!(
        "unrecognized oracle connect string: {raw}"
    )))
}

fn looks_like_query(sql: &str) -> bool {
    let u = sql.trim_start().to_ascii_uppercase();
    u.starts_with("SELECT")
        || u.starts_with("WITH")
        || u.starts_with("SHOW")
        || u.starts_with("DESCRIBE")
        || u.starts_with("DESC ")
        || u.starts_with("EXPLAIN")
}

fn cell_to_string(row: &oracle::Row, idx: usize) -> String {
    if let Ok(v) = row.get::<_, Option<String>>(idx) {
        return v.unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.get::<_, Option<i64>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.get::<_, Option<f64>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    "NULL".into()
}

fn validate_ident(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '#')
    {
        return Err(Error::msg(format!("invalid identifier: {name}")));
    }
    Ok(())
}

fn redact_label(user: &str, connect: &str) -> String {
    format!("oracle://{user}@{connect}")
}

fn ora_err(err: impl std::fmt::Display) -> Error {
    Error::backend("oracle", err.to_string())
}

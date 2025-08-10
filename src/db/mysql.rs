//! MySQL / MariaDB driver (sync `mysql` crate).

use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::probe;
use super::provision::{CreateKind, DockerSpec, LocalStatus, ProvisionInfo};
use super::traits::{
    Capabilities, ColumnInfo, ConstraintInfo, Database, IndexInfo, QueryResult, Row, TableInfo,
    TableSchema,
};
use crate::error::{Error, Result};
use mysql::prelude::*;
use mysql::{Opts, Pool, Row as MyRow, Value};
use std::cell::RefCell;

pub struct MysqlDriver;

impl Driver for MysqlDriver {
    fn id(&self) -> &'static str {
        "mysql"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["mariadb"]
    }

    fn description(&self) -> &'static str {
        "MySQL / MariaDB"
    }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        let url = spec.url.clone().unwrap_or_else(|| spec.target.clone());
        let opts = Opts::from_url(&url).map_err(my_err)?;
        let pool = Pool::new(opts).map_err(my_err)?;
        let mut conn = pool.get_conn().map_err(my_err)?;
        let schema: String = conn
            .query_first("SELECT DATABASE()")
            .map_err(my_err)?
            .unwrap_or_default();
        Ok(Box::new(MysqlDb {
            conn: RefCell::new(conn),
            label: redact_url(&url),
            schema,
        }))
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::Server,
            default_port: Some(3306),
            docker: Some(DockerSpec {
                image: "mysql:8.4",
                env: &[
                    ("MYSQL_ROOT_PASSWORD", "sqffy"),
                    ("MYSQL_DATABASE", "sqffy"),
                    ("MYSQL_USER", "sqffy"),
                    ("MYSQL_PASSWORD", "sqffy"),
                ],
                ports: &[(3306, 3306)],
                name_prefix: "sqffy-mysql",
                url_template: "mysql://sqffy:sqffy@{host}:{port}/sqffy",
            }),
            download_url: Some("https://dev.mysql.com/downloads/mysql/"),
            winget_id: Some("Oracle.MySQL"),
            help_key: "provision.help.mysql",
        }
    }

    fn probe_local(&self) -> LocalStatus {
        probe::tcp_localhost(3306)
    }
}

pub struct MysqlDb {
    conn: RefCell<mysql::PooledConn>,
    label: String,
    schema: String,
}

impl Database for MysqlDb {
    fn driver_id(&self) -> &'static str {
        "mysql"
    }

    fn display_label(&self) -> &str {
        &self.label
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            schemas: true,
            table_ddl: true,
            quote_char: '`',
        }
    }

    fn execute_sql(&mut self, sql: &str, row_limit: usize) -> Result<QueryResult> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Ok(QueryResult::empty_affected(0));
        }
        let mut conn = self.conn.borrow_mut();
        let result = conn.query_iter(trimmed).map_err(my_err)?;

        let mut columns = Vec::new();
        let mut column_types = Vec::new();
        for c in result.columns().as_ref().iter() {
            columns.push(String::from_utf8_lossy(c.name_ref()).into_owned());
            column_types.push(format!("{:?}", c.column_type()).to_ascii_uppercase());
        }

        if columns.is_empty() {
            let affected = result.affected_rows();
            return Ok(QueryResult::empty_affected(affected));
        }

        let mut rows = Vec::new();
        let mut truncated = false;
        for (i, item) in result.enumerate() {
            let row = item.map_err(my_err)?;
            if row_limit > 0 && i >= row_limit {
                truncated = true;
                break;
            }
            rows.push(mysql_row_to_strings(&row, columns.len()));
        }
        Ok(QueryResult {
            columns,
            column_types,
            rows,
            truncated,
            rows_affected: None,
        })
    }

    fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let mut conn = self.conn.borrow_mut();
        let schema = effective_schema(&mut conn, &self.schema)?;
        validate_ident(&schema)?;
        let sql = format!(
            "SELECT table_name, table_type FROM information_schema.tables
             WHERE table_schema = '{schema}'
             ORDER BY table_type, table_name"
        );
        let rows: Vec<(String, String)> = conn.query(sql).map_err(my_err)?;
        Ok(rows
            .into_iter()
            .map(|(name, kind_raw)| TableInfo {
                name,
                kind: if kind_raw.to_ascii_uppercase().contains("VIEW") {
                    "view".into()
                } else {
                    "table".into()
                },
                schema: Some(schema.clone()),
            })
            .collect())
    }

    fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let mut conn = self.conn.borrow_mut();
        let default_schema = effective_schema(&mut conn, &self.schema)?;
        let (schema, name) = split_qual(table, &default_schema);
        validate_ident(&schema)?;
        validate_ident(&name)?;
        let sql = format!(
            "SELECT column_name, column_type, is_nullable, column_default,
                    column_key, IFNULL(column_comment, '')
             FROM information_schema.columns
             WHERE table_schema = '{schema}' AND table_name = '{name}'
             ORDER BY ordinal_position"
        );
        let rows: Vec<(String, String, String, Option<String>, String, String)> =
            conn.query(sql).map_err(my_err)?;
        if rows.is_empty() {
            return Err(Error::msg(format!("table not found: {table}")));
        }
        Ok(rows
            .into_iter()
            .map(|(name, ty, nullable, def, key, comment)| ColumnInfo {
                name,
                type_name: ty,
                not_null: nullable.eq_ignore_ascii_case("NO"),
                default_value: def,
                primary_key: key.contains('P'),
                comment: if comment.is_empty() {
                    None
                } else {
                    Some(comment)
                },
            })
            .collect())
    }

    fn table_ddl(&self, table: &str) -> Result<Option<String>> {
        let (_, name) = split_qual(table, &self.schema);
        validate_ident(&name)?;
        let mut conn = self.conn.borrow_mut();
        let row: Option<(String, String)> = conn
            .query_first(format!("SHOW CREATE TABLE `{name}`"))
            .map_err(my_err)?;
        Ok(row.map(|(_, ddl)| ddl))
    }

    fn inspect_table(&self, table: &str) -> Result<TableSchema> {
        let columns = self.describe_table(table)?;
        let ddl = self.table_ddl(table)?;
        let mut conn = self.conn.borrow_mut();
        let default_schema = effective_schema(&mut conn, &self.schema)?;
        let (schema, name) = split_qual(table, &default_schema);
        let indexes = list_indexes(&mut conn, &schema, &name)?;
        let fks = foreign_keys(&mut conn, &schema, &name)?;
        let mut constraints = Vec::new();
        let pk: Vec<_> = columns
            .iter()
            .filter(|c| c.primary_key)
            .map(|c| c.name.as_str())
            .collect();
        if !pk.is_empty() {
            constraints.push(ConstraintInfo {
                kind: "PRIMARY KEY".into(),
                name: "PRIMARY".into(),
                definition: pk.join(", "),
            });
        }
        constraints.extend(fks);
        Ok(TableSchema {
            name: table.to_string(),
            kind: "table".into(),
            columns,
            constraints,
            indexes,
            ddl,
        })
    }
}

fn effective_schema(conn: &mut mysql::PooledConn, fallback: &str) -> Result<String> {
    if !fallback.is_empty() {
        return Ok(fallback.to_string());
    }
    Ok(conn
        .query_first("SELECT DATABASE()")
        .map_err(my_err)?
        .unwrap_or_default())
}

fn list_indexes(
    conn: &mut mysql::PooledConn,
    schema: &str,
    table: &str,
) -> Result<Vec<IndexInfo>> {
    validate_ident(schema)?;
    validate_ident(table)?;
    let sql = format!(
        "SELECT index_name, non_unique, group_concat(column_name ORDER BY seq_in_index)
         FROM information_schema.statistics
         WHERE table_schema = '{schema}' AND table_name = '{table}'
         GROUP BY index_name, non_unique"
    );
    let rows: Vec<(String, i64, Option<String>)> = conn.query(sql).map_err(my_err)?;
    Ok(rows
        .into_iter()
        .map(|(name, non_unique, cols)| {
            let cols = cols.unwrap_or_default();
            IndexInfo {
                unique: non_unique == 0,
                origin: if name == "PRIMARY" {
                    "pk"
                } else if non_unique == 0 {
                    "u"
                } else {
                    "c"
                }
                .into(),
                columns: cols.split(',').map(|s| s.to_string()).collect(),
                name,
            }
        })
        .collect())
}

fn foreign_keys(
    conn: &mut mysql::PooledConn,
    schema: &str,
    table: &str,
) -> Result<Vec<ConstraintInfo>> {
    validate_ident(schema)?;
    validate_ident(table)?;
    let sql = format!(
        "SELECT constraint_name, column_name, referenced_table_name, referenced_column_name
         FROM information_schema.key_column_usage
         WHERE table_schema = '{schema}' AND table_name = '{table}'
           AND referenced_table_name IS NOT NULL"
    );
    let rows: Vec<(String, String, String, String)> = conn.query(sql).map_err(my_err)?;
    Ok(rows
        .into_iter()
        .map(|(cname, from, to_table, to_col)| ConstraintInfo {
            kind: "FOREIGN KEY".into(),
            name: cname,
            definition: format!("{from} → {to_table}.{to_col}"),
        })
        .collect())
}

fn mysql_row_to_strings(row: &MyRow, len: usize) -> Row {
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let v: Option<Value> = row.get(i);
        out.push(match v {
            None | Some(Value::NULL) => "NULL".into(),
            Some(Value::Bytes(b)) => String::from_utf8_lossy(&b).into_owned(),
            Some(Value::Int(n)) => n.to_string(),
            Some(Value::UInt(n)) => n.to_string(),
            Some(Value::Float(n)) => n.to_string(),
            Some(Value::Double(n)) => n.to_string(),
            Some(other) => format!("{other:?}"),
        });
    }
    out
}

fn split_qual(table: &str, default_schema: &str) -> (String, String) {
    if let Some((s, n)) = table.split_once('.') {
        (s.to_string(), n.to_string())
    } else {
        (default_schema.to_string(), table.to_string())
    }
}

fn validate_ident(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        return Err(Error::msg(format!("invalid identifier: {name}")));
    }
    Ok(())
}

fn redact_url(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let creds = &url[scheme_end + 3..at];
            let user = creds.split(':').next().unwrap_or(creds);
            return format!("{scheme}{user}@{}", &url[at + 1..]);
        }
    }
    url.to_string()
}

fn my_err(err: impl std::fmt::Display) -> Error {
    Error::backend("mysql", err.to_string())
}

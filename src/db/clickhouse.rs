//! ClickHouse driver over the HTTP interface (sync `ureq`).

use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::probe;
use super::provision::{CreateKind, DockerSpec, LocalStatus, ProvisionInfo};
use super::traits::{
    Capabilities, ColumnInfo, Database, QueryResult, Row, TableInfo, TableSchema,
};
use crate::error::{Error, Result};
use std::io::Read;

pub struct ClickhouseDriver;

impl Driver for ClickhouseDriver {
    fn id(&self) -> &'static str {
        "clickhouse"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["ch", "click"]
    }

    fn description(&self) -> &'static str {
        "ClickHouse"
    }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        let raw = spec.url.clone().unwrap_or_else(|| spec.target.clone());
        let cfg = HttpConfig::parse(&raw)?;
        // Smoke query
        let _ = cfg.query("SELECT 1", 1)?;
        Ok(Box::new(ClickhouseDb {
            cfg,
            label: redact_url(&raw),
        }))
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::Server,
            default_port: Some(8123),
            docker: Some(DockerSpec {
                image: "clickhouse/clickhouse-server:24.8",
                env: &[
                    ("CLICKHOUSE_USER", "sqffy"),
                    ("CLICKHOUSE_PASSWORD", "sqffy"),
                    ("CLICKHOUSE_DB", "sqffy"),
                    ("CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT", "1"),
                ],
                ports: &[(8123, 8123), (9000, 9000)],
                name_prefix: "sqffy-ch",
                url_template: "clickhouse://sqffy:sqffy@{host}:8123/sqffy",
            }),
            download_url: Some("https://clickhouse.com/docs/en/install"),
            winget_id: None,
            help_key: "provision.help.clickhouse",
        }
    }

    fn probe_local(&self) -> LocalStatus {
        probe::tcp_localhost(8123)
    }
}

pub struct ClickhouseDb {
    cfg: HttpConfig,
    label: String,
}

impl Database for ClickhouseDb {
    fn driver_id(&self) -> &'static str {
        "clickhouse"
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
        if looks_like_result_set(trimmed) {
            self.cfg.query(trimmed, row_limit)
        } else {
            self.cfg.exec(trimmed)?;
            Ok(QueryResult::empty_affected(0))
        }
    }

    fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let db = self.cfg.database.clone();
        validate_ident(&db)?;
        let sql = format!(
            "SELECT name, engine FROM system.tables WHERE database = '{db}' ORDER BY name"
        );
        let result = self.cfg.query(&sql, 0)?;
        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| {
                let name = row.first()?.clone();
                let engine = row.get(1).cloned().unwrap_or_default();
                let kind = if engine.to_ascii_lowercase().contains("view") {
                    "view"
                } else {
                    "table"
                };
                Some(TableInfo {
                    name,
                    kind: kind.into(),
                    schema: Some(db.clone()),
                })
            })
            .collect())
    }

    fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let (db, name) = split_qual(table, &self.cfg.database);
        validate_ident(&db)?;
        validate_ident(&name)?;
        let sql = format!(
            "SELECT name, type, default_kind, default_expression, comment \
             FROM system.columns WHERE database = '{db}' AND table = '{name}' \
             ORDER BY position"
        );
        let result = self.cfg.query(&sql, 0)?;
        if result.rows.is_empty() {
            return Err(Error::msg(format!("table not found: {table}")));
        }
        Ok(result
            .rows
            .into_iter()
            .map(|row| {
                let default_kind = row.get(2).cloned().unwrap_or_default();
                let default_expr = row.get(3).cloned().unwrap_or_default();
                let default_value = if default_kind.is_empty() || default_kind == "DEFAULT" {
                    if default_expr.is_empty() {
                        None
                    } else {
                        Some(default_expr)
                    }
                } else {
                    Some(format!("{default_kind} {default_expr}"))
                };
                let comment = row.get(4).cloned().filter(|c| !c.is_empty());
                ColumnInfo {
                    name: row.first().cloned().unwrap_or_default(),
                    type_name: row.get(1).cloned().unwrap_or_else(|| "?".into()),
                    not_null: true,
                    default_value,
                    primary_key: false,
                    comment,
                }
            })
            .collect())
    }

    fn table_ddl(&self, table: &str) -> Result<Option<String>> {
        let (db, name) = split_qual(table, &self.cfg.database);
        validate_ident(&db)?;
        validate_ident(&name)?;
        let sql = format!("SHOW CREATE TABLE `{db}`.`{name}`");
        let result = self.cfg.query(&sql, 1)?;
        Ok(result.rows.first().and_then(|r| r.first().cloned()))
    }

    fn inspect_table(&self, table: &str) -> Result<TableSchema> {
        let columns = self.describe_table(table)?;
        let ddl = self.table_ddl(table)?;
        Ok(TableSchema {
            name: table.to_string(),
            kind: "table".into(),
            columns,
            constraints: Vec::new(),
            indexes: Vec::new(),
            ddl,
        })
    }
}

#[derive(Clone)]
struct HttpConfig {
    base: String,
    user: Option<String>,
    password: Option<String>,
    database: String,
}

impl HttpConfig {
    fn parse(raw: &str) -> Result<Self> {
        let lower = raw.to_ascii_lowercase();
        let rest = if lower.starts_with("clickhouse://")
            || lower.starts_with("ch://")
            || lower.starts_with("http://")
            || lower.starts_with("https://")
        {
            let scheme_len = raw.find("://").map(|i| i + 3).unwrap_or(0);
            &raw[scheme_len..]
        } else if raw.contains("://") {
            return Err(Error::msg(format!("unsupported clickhouse URL: {raw}")));
        } else {
            raw
        };

        let (auth_host, path_q) = if let Some((a, b)) = rest.split_once('/') {
            (a, b)
        } else {
            (rest, "")
        };

        let (user, password, hostport) = if let Some((creds, hp)) = auth_host.split_once('@') {
            let (u, p) = creds.split_once(':').unwrap_or((creds, ""));
            (Some(u.to_string()), Some(p.to_string()), hp)
        } else {
            (None, None, auth_host)
        };

        let (path, _) = path_q.split_once('?').unwrap_or((path_q, ""));
        let database = if path.is_empty() {
            "default".into()
        } else {
            path.trim_matches('/').to_string()
        };

        let use_https = lower.starts_with("https://");
        let scheme = if use_https { "https" } else { "http" };
        let hostport = if hostport.contains(':') {
            hostport.to_string()
        } else {
            format!("{hostport}:8123")
        };

        Ok(Self {
            base: format!("{scheme}://{hostport}/"),
            user,
            password,
            database,
        })
    }

    fn agent(&self) -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(60))
            .build()
    }

    fn apply_auth(&self, mut req: ureq::Request) -> ureq::Request {
        if let Some(user) = &self.user {
            req = req.set(
                "X-ClickHouse-User",
                user,
            );
        }
        if let Some(pass) = &self.password {
            req = req.set("X-ClickHouse-Key", pass);
        }
        req
    }

    fn exec(&self, sql: &str) -> Result<()> {
        let url = format!("{}?database={}", self.base, urlencoding_db(&self.database));
        let req = self.apply_auth(self.agent().post(&url));
        let resp = req.send_string(sql).map_err(ch_err)?;
        if resp.status() >= 400 {
            let mut body = String::new();
            let _ = resp.into_reader().read_to_string(&mut body);
            return Err(Error::backend("clickhouse", body));
        }
        Ok(())
    }

    fn query(&self, sql: &str, row_limit: usize) -> Result<QueryResult> {
        let mut q = sql.trim().to_string();
        if !q.to_ascii_uppercase().contains("FORMAT ") {
            q.push_str(" FORMAT TabSeparatedWithNamesAndTypes");
        }
        let url = format!("{}?database={}", self.base, urlencoding_db(&self.database));
        let req = self.apply_auth(self.agent().post(&url));
        let resp = req.send_string(&q).map_err(ch_err)?;
        if resp.status() >= 400 {
            let mut body = String::new();
            let _ = resp.into_reader().read_to_string(&mut body);
            return Err(Error::backend("clickhouse", body));
        }
        let mut body = String::new();
        resp.into_reader()
            .read_to_string(&mut body)
            .map_err(ch_err)?;
        parse_tsv_names_types(&body, row_limit)
    }
}

fn parse_tsv_names_types(body: &str, row_limit: usize) -> Result<QueryResult> {
    let mut lines = body.lines();
    let Some(header) = lines.next() else {
        return Ok(QueryResult::empty_affected(0));
    };
    if header.is_empty() && body.trim().is_empty() {
        return Ok(QueryResult::empty_affected(0));
    }
    let columns: Vec<String> = header.split('\t').map(|s| s.to_string()).collect();
    let types_line = lines.next().unwrap_or("");
    let column_types: Vec<String> = if types_line.is_empty() {
        columns.iter().map(|_| "?".into()).collect()
    } else {
        types_line.split('\t').map(|s| s.to_string()).collect()
    };

    let mut rows = Vec::new();
    let mut truncated = false;
    for (i, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        if row_limit > 0 && i >= row_limit {
            truncated = true;
            break;
        }
        let row: Row = line.split('\t').map(|s| s.to_string()).collect();
        rows.push(row);
    }
    Ok(QueryResult {
        columns,
        column_types,
        rows,
        truncated,
        rows_affected: None,
    })
}

fn looks_like_result_set(sql: &str) -> bool {
    let u = sql.trim_start().to_ascii_uppercase();
    u.starts_with("SELECT")
        || u.starts_with("WITH")
        || u.starts_with("SHOW")
        || u.starts_with("DESCRIBE")
        || u.starts_with("DESC ")
        || u.starts_with("EXISTS")
        || u.starts_with("EXPLAIN")
}

fn split_qual(table: &str, default_db: &str) -> (String, String) {
    if let Some((s, n)) = table.split_once('.') {
        (s.to_string(), n.to_string())
    } else {
        (default_db.to_string(), table.to_string())
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

fn urlencoding_db(db: &str) -> String {
    db.replace(' ', "%20")
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

fn ch_err(err: impl std::fmt::Display) -> Error {
    Error::backend("clickhouse", err.to_string())
}

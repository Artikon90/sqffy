//! PostgreSQL driver (sync `postgres` crate).

use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::provision::{CreateKind, DockerSpec, LocalStatus, ProvisionInfo};
use super::traits::{
    Capabilities, ColumnInfo, ConstraintInfo, Database, IndexInfo, QueryResult, Row, TableInfo,
    TableSchema,
};
use super::probe;
use crate::error::{Error, Result};
use postgres::types::Type;
use postgres::{Client, NoTls, Row as PgRow};
use std::cell::RefCell;

pub struct PostgresDriver;

impl Driver for PostgresDriver {
    fn id(&self) -> &'static str {
        "postgres"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["postgresql", "pg"]
    }

    fn description(&self) -> &'static str {
        "PostgreSQL"
    }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        let url = spec.url.clone().unwrap_or_else(|| spec.target.clone());
        let client = Client::connect(&url, NoTls).map_err(pg_err)?;
        Ok(Box::new(PostgresDb {
            client: RefCell::new(client),
            label: redact_url(&url),
        }))
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::Server,
            default_port: Some(5432),
            docker: Some(DockerSpec {
                image: "postgres:16-alpine",
                env: &[
                    ("POSTGRES_USER", "sqffy"),
                    ("POSTGRES_PASSWORD", "sqffy"),
                    ("POSTGRES_DB", "sqffy"),
                ],
                ports: &[(5432, 5432)],
                name_prefix: "sqffy-pg",
                url_template: "postgres://sqffy:sqffy@{host}:{port}/sqffy",
            }),
            download_url: Some("https://www.postgresql.org/download/"),
            winget_id: Some("PostgreSQL.PostgreSQL.16"),
            help_key: "provision.help.postgres",
        }
    }

    fn probe_local(&self) -> LocalStatus {
        probe::tcp_localhost(5432)
    }
}

pub struct PostgresDb {
    client: RefCell<Client>,
    label: String,
}

impl Database for PostgresDb {
    fn driver_id(&self) -> &'static str {
        "postgres"
    }

    fn display_label(&self) -> &str {
        &self.label
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            schemas: true,
            table_ddl: true,
            quote_char: '"',
        }
    }

    fn execute_sql(&mut self, sql: &str, row_limit: usize) -> Result<QueryResult> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Ok(QueryResult::empty_affected(0));
        }

        let mut client = self.client.borrow_mut();
        match client.query(trimmed, &[]) {
            Ok(rows) => Ok(pg_rows_to_result(&rows, row_limit)),
            Err(first_err) => match client.simple_query(trimmed) {
                Ok(messages) => Ok(simple_query_to_result(messages, row_limit)),
                Err(_) => Err(pg_err(first_err)),
            },
        }
    }

    fn list_tables(&self) -> Result<Vec<TableInfo>> {
        let mut client = self.client.borrow_mut();
        let rows = client
            .query(
                "SELECT table_schema, table_name, table_type
                 FROM information_schema.tables
                 WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
                 ORDER BY table_schema, table_name",
                &[],
            )
            .map_err(pg_err)?;
        let mut out = Vec::new();
        for row in rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let kind_raw: String = row.get(2);
            let kind = if kind_raw.eq_ignore_ascii_case("VIEW") {
                "view"
            } else {
                "table"
            };
            out.push(TableInfo {
                name,
                kind: kind.into(),
                schema: Some(schema),
            });
        }
        Ok(out)
    }

    fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let (schema, name) = split_qual(table);
        let mut client = self.client.borrow_mut();
        let rows = client
            .query(
                "SELECT column_name, data_type, is_nullable, column_default
                 FROM information_schema.columns
                 WHERE table_schema = $1 AND table_name = $2
                 ORDER BY ordinal_position",
                &[&schema, &name],
            )
            .map_err(pg_err)?;
        if rows.is_empty() {
            return Err(Error::msg(format!("table not found: {table}")));
        }
        let pk = primary_key_cols(&mut client, &schema, &name)?;
        let mut cols = Vec::new();
        for row in rows {
            let col_name: String = row.get(0);
            let type_name: String = row.get(1);
            let nullable: String = row.get(2);
            let default_value: Option<String> = row.get(3);
            cols.push(ColumnInfo {
                name: col_name.clone(),
                type_name,
                not_null: nullable.eq_ignore_ascii_case("NO"),
                default_value,
                primary_key: pk.iter().any(|c| c == &col_name),
                comment: None,
            });
        }
        Ok(cols)
    }

    fn table_ddl(&self, table: &str) -> Result<Option<String>> {
        // Approximate DDL from columns — full pg_dump parity is out of scope.
        let cols = self.describe_table(table)?;
        let mut parts = Vec::new();
        for c in &cols {
            let mut line = format!("  \"{}\" {}", c.name, c.type_name);
            if c.not_null {
                line.push_str(" NOT NULL");
            }
            if let Some(d) = &c.default_value {
                line.push_str(&format!(" DEFAULT {d}"));
            }
            parts.push(line);
        }
        let pk: Vec<_> = cols
            .iter()
            .filter(|c| c.primary_key)
            .map(|c| format!("\"{}\"", c.name))
            .collect();
        if !pk.is_empty() {
            parts.push(format!("  PRIMARY KEY ({})", pk.join(", ")));
        }
        Ok(Some(format!(
            "CREATE TABLE {} (\n{}\n);",
            quote_ident(table),
            parts.join(",\n")
        )))
    }

    fn inspect_table(&self, table: &str) -> Result<TableSchema> {
        let columns = self.describe_table(table)?;
        let ddl = self.table_ddl(table)?;
        let (schema, name) = split_qual(table);
        let mut client = self.client.borrow_mut();
        let constraints = foreign_keys(&mut client, &schema, &name)?;
        let indexes = list_indexes(&mut client, &schema, &name)?;
        let mut all_constraints = Vec::new();
        let pk: Vec<_> = columns
            .iter()
            .filter(|c| c.primary_key)
            .map(|c| c.name.as_str())
            .collect();
        if !pk.is_empty() {
            all_constraints.push(ConstraintInfo {
                kind: "PRIMARY KEY".into(),
                name: "pk".into(),
                definition: pk.join(", "),
            });
        }
        all_constraints.extend(constraints);
        Ok(TableSchema {
            name: table.to_string(),
            kind: "table".into(),
            columns,
            constraints: all_constraints,
            indexes,
            ddl,
        })
    }
}

fn primary_key_cols(client: &mut Client, schema: &str, table: &str) -> Result<Vec<String>> {
    let rows = client
        .query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             JOIN pg_class c ON c.oid = i.indrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE i.indisprimary AND n.nspname = $1 AND c.relname = $2
             ORDER BY a.attnum",
            &[&schema, &table],
        )
        .map_err(pg_err)?;
    Ok(rows.iter().map(|r| r.get(0)).collect())
}

fn foreign_keys(client: &mut Client, schema: &str, table: &str) -> Result<Vec<ConstraintInfo>> {
    let rows = client
        .query(
            "SELECT tc.constraint_name, kcu.column_name, ccu.table_name, ccu.column_name
             FROM information_schema.table_constraints AS tc
             JOIN information_schema.key_column_usage AS kcu
               ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
             JOIN information_schema.constraint_column_usage AS ccu
               ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
             WHERE tc.constraint_type = 'FOREIGN KEY'
               AND tc.table_schema = $1 AND tc.table_name = $2",
            &[&schema, &table],
        )
        .map_err(pg_err)?;
    let mut out = Vec::new();
    for row in rows {
        let cname: String = row.get(0);
        let from: String = row.get(1);
        let to_table: String = row.get(2);
        let to_col: String = row.get(3);
        out.push(ConstraintInfo {
            kind: "FOREIGN KEY".into(),
            name: cname,
            definition: format!("{from} → {to_table}.{to_col}"),
        });
    }
    Ok(out)
}

fn list_indexes(client: &mut Client, schema: &str, table: &str) -> Result<Vec<IndexInfo>> {
    let rows = client
        .query(
            "SELECT i.relname, ix.indisunique,
                    array_to_string(ARRAY(
                      SELECT a.attname FROM pg_attribute a
                      WHERE a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
                      ORDER BY a.attnum
                    ), ', ')
             FROM pg_class t
             JOIN pg_namespace n ON n.oid = t.relnamespace
             JOIN pg_index ix ON t.oid = ix.indrelid
             JOIN pg_class i ON i.oid = ix.indexrelid
             WHERE n.nspname = $1 AND t.relname = $2 AND NOT ix.indisprimary",
            &[&schema, &table],
        )
        .map_err(pg_err)?;
    let mut out = Vec::new();
    for row in rows {
        let name: String = row.get(0);
        let unique: bool = row.get(1);
        let cols: String = row.get(2);
        out.push(IndexInfo {
            name,
            unique,
            origin: if unique { "u" } else { "c" }.into(),
            columns: cols
                .split(", ")
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        });
    }
    Ok(out)
}

fn pg_rows_to_result(rows: &[PgRow], row_limit: usize) -> QueryResult {
    if rows.is_empty() {
        return QueryResult {
            columns: vec![],
            column_types: vec![],
            rows: vec![],
            truncated: false,
            rows_affected: None,
        };
    }
    let cols = rows[0].columns();
    let columns: Vec<String> = cols.iter().map(|c| c.name().to_string()).collect();
    let column_types: Vec<String> = cols.iter().map(|c| type_name(c.type_())).collect();
    let mut out = Vec::new();
    let mut truncated = false;
    for (i, row) in rows.iter().enumerate() {
        if row_limit > 0 && i >= row_limit {
            truncated = true;
            break;
        }
        let mut vals = Vec::with_capacity(columns.len());
        for (j, _) in columns.iter().enumerate() {
            vals.push(pg_cell(row, j));
        }
        out.push(vals);
    }
    QueryResult {
        columns,
        column_types,
        rows: out,
        truncated,
        rows_affected: None,
    }
}

fn simple_query_to_result(
    messages: Vec<postgres::SimpleQueryMessage>,
    row_limit: usize,
) -> QueryResult {
    let mut affected = 0u64;
    let mut saw_row = false;
    let mut columns = Vec::new();
    let mut column_types = Vec::new();
    let mut out_rows: Vec<Row> = Vec::new();
    let mut truncated = false;

    for msg in messages {
        match msg {
            postgres::SimpleQueryMessage::Row(row) => {
                saw_row = true;
                if columns.is_empty() {
                    for i in 0..row.len() {
                        columns.push(
                            row.columns()
                                .get(i)
                                .map(|c| c.name().to_string())
                                .unwrap_or_else(|| format!("col{i}")),
                        );
                        column_types.push("?".into());
                    }
                }
                if row_limit > 0 && out_rows.len() >= row_limit {
                    truncated = true;
                    continue;
                }
                let mut vals = Vec::with_capacity(row.len());
                for i in 0..row.len() {
                    vals.push(row.get(i).unwrap_or("NULL").to_string());
                }
                out_rows.push(vals);
            }
            postgres::SimpleQueryMessage::CommandComplete(n) => {
                affected = affected.saturating_add(n);
            }
            _ => {}
        }
    }

    if saw_row {
        QueryResult {
            columns,
            column_types,
            rows: out_rows,
            truncated,
            rows_affected: None,
        }
    } else {
        QueryResult::empty_affected(affected)
    }
}

fn pg_cell(row: &PgRow, idx: usize) -> String {
    if let Ok(v) = row.try_get::<_, Option<&str>>(idx) {
        return v.unwrap_or("NULL").to_string();
    }
    if let Ok(v) = row.try_get::<_, Option<i64>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<_, Option<f64>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<_, Option<bool>>(idx) {
        return v
            .map(|b| b.to_string())
            .unwrap_or_else(|| "NULL".into());
    }
    "NULL".into()
}

fn type_name(t: &Type) -> String {
    t.name().to_ascii_uppercase()
}

fn split_qual(table: &str) -> (String, String) {
    if let Some((s, n)) = table.split_once('.') {
        (s.to_string(), n.to_string())
    } else {
        ("public".into(), table.to_string())
    }
}

fn quote_ident(name: &str) -> String {
    if name.contains('.') {
        name.split('.')
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(".")
    } else {
        format!("\"{name}\"")
    }
}

fn redact_url(url: &str) -> String {
    // postgres://user:pass@host/db → postgres://user@host/db
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

fn pg_err(err: postgres::Error) -> Error {
    Error::backend("postgres", err.to_string())
}

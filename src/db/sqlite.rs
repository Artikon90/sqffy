use super::connect::ConnectionSpec;
use super::driver::Driver;
use super::provision::{CreateKind, LocalStatus, ProvisionInfo};
use super::traits::{Capabilities, ColumnInfo, Database, QueryResult, Row, TableInfo};
use crate::error::{Error, Result};
use crate::i18n::tr;
use rusqlite::{Connection, types::ValueRef};
use std::path::Path;

pub struct SqliteDriver;

impl Driver for SqliteDriver {
    fn id(&self) -> &'static str {
        "sqlite"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["file"]
    }

    fn description(&self) -> &'static str {
        "SQLite (file or :memory:)"
    }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        let db = if spec.target == ":memory:" {
            SqliteDb::open_memory()?
        } else {
            SqliteDb::open(Path::new(&spec.target))?
        };
        Ok(Box::new(db))
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::File,
            default_port: None,
            docker: None,
            download_url: None,
            winget_id: None,
            help_key: "provision.help.sqlite",
        }
    }

    fn probe_local(&self) -> LocalStatus {
        // File engine — always "ready" to create.
        LocalStatus::Ready {
            host: "local".into(),
            port: 0,
        }
    }
}

pub struct SqliteDb {
    conn: Connection,
    label: String,
}

impl SqliteDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(sqlite_err)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            ",
        )
        .map_err(sqlite_err)?;
        Ok(Self {
            conn,
            label: path.display().to_string(),
        })
    }

    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(sqlite_err)?;
        Ok(Self {
            conn,
            label: ":memory:".into(),
        })
    }
}

impl Database for SqliteDb {
    fn driver_id(&self) -> &'static str {
        "sqlite"
    }

    fn display_label(&self) -> &str {
        &self.label
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            schemas: false,
            table_ddl: true,
            quote_char: '"',
        }
    }

    fn execute_sql(&mut self, sql: &str, row_limit: usize) -> Result<QueryResult> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Ok(QueryResult::empty_affected(0));
        }

        let mut stmt = self
            .conn
            .prepare(trimmed)
            .map_err(|e| sqlite_err_sql(e, trimmed))?;
        let column_count = stmt.column_count();

        if column_count == 0 {
            drop(stmt);
            let rows_affected = self
                .conn
                .execute(trimmed, [])
                .map_err(|e| sqlite_err_sql(e, trimmed))? as u64;
            return Ok(QueryResult::empty_affected(rows_affected));
        }

        let columns: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        // Prefer declared types from the statement; refine unknowns from first row.
        let mut column_types: Vec<String> = stmt
            .columns()
            .iter()
            .map(|col| {
                col.decl_type()
                    .map(str::to_string)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default()
            })
            .collect();
        if column_types.len() < column_count {
            column_types.resize(column_count, String::new());
        }

        let mut rows: Vec<Row> = Vec::new();
        let mut truncated = false;
        let mut rows_iter = stmt
            .query([])
            .map_err(|e| sqlite_err_sql(e, trimmed))?;
        let mut inferred = false;

        while let Some(row) = rows_iter.next().map_err(|e| sqlite_err_sql(e, trimmed))? {
            if !inferred {
                for i in 0..column_count {
                    if column_types[i].is_empty() {
                        column_types[i] = infer_type_name(
                            row.get_ref(i).map_err(|e| sqlite_err_sql(e, trimmed))?,
                        )
                        .to_string();
                    }
                }
                inferred = true;
            }
            if row_limit > 0 && rows.len() >= row_limit {
                truncated = true;
                break;
            }
            let mut out = Vec::with_capacity(column_count);
            for i in 0..column_count {
                out.push(value_to_string(
                    row.get_ref(i).map_err(|e| sqlite_err_sql(e, trimmed))?,
                ));
            }
            rows.push(out);
        }

        for ty in &mut column_types {
            if ty.is_empty() {
                *ty = "?".into();
            }
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
        let mut stmt = self
            .conn
            .prepare(
                "SELECT name, type FROM sqlite_master
             WHERE type IN ('table', 'view')
               AND name NOT LIKE 'sqlite_%'
             ORDER BY type, name",
            )
            .map_err(sqlite_err)?;
        let iter = stmt
            .query_map([], |row| {
                Ok(TableInfo {
                    name: row.get(0)?,
                    kind: row.get(1)?,
                    schema: None,
                })
            })
            .map_err(sqlite_err)?;
        let mut tables = Vec::new();
        for item in iter {
            tables.push(item.map_err(sqlite_err)?);
        }
        Ok(tables)
    }

    fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        validate_ident(table)?;
        let sql = format!("PRAGMA table_info({table})");
        let mut stmt = self.conn.prepare(&sql).map_err(sqlite_err)?;
        let iter = stmt
            .query_map([], |row| {
                Ok(ColumnInfo {
                    name: row.get(1)?,
                    type_name: row.get::<_, String>(2).unwrap_or_default(),
                    not_null: row.get::<_, i64>(3)? != 0,
                    default_value: row.get(4)?,
                    primary_key: row.get::<_, i64>(5)? != 0,
                    comment: None,
                })
            })
            .map_err(sqlite_err)?;
        let mut cols = Vec::new();
        for item in iter {
            cols.push(item.map_err(sqlite_err)?);
        }
        if cols.is_empty() {
            return Err(Error::msg(tr("meta.table_not_found", &[("table", table)])));
        }
        Ok(cols)
    }

    fn table_ddl(&self, table: &str) -> Result<Option<String>> {
        validate_ident(table)?;
        let mut stmt = self
            .conn
            .prepare("SELECT sql FROM sqlite_master WHERE name = ?1 AND sql IS NOT NULL")
            .map_err(sqlite_err)?;
        let mut rows = stmt.query([table]).map_err(sqlite_err)?;
        if let Some(row) = rows.next().map_err(sqlite_err)? {
            Ok(Some(row.get(0).map_err(sqlite_err)?))
        } else {
            Ok(None)
        }
    }

    fn inspect_table(&self, table: &str) -> Result<super::TableSchema> {
        validate_ident(table)?;
        let columns = self.describe_table(table)?;
        let ddl = self.table_ddl(table)?;
        let kind = self.object_kind(table)?.unwrap_or_else(|| "table".into());
        let mut constraints = build_column_constraints(&columns);
        constraints.extend(self.foreign_key_constraints(table)?);
        constraints.extend(extract_check_constraints(ddl.as_deref().unwrap_or("")));
        let indexes = self.list_indexes(table)?;
        // UNIQUE indexes that aren't PK also appear as constraints
        for idx in &indexes {
            if idx.unique && idx.origin != "pk" {
                constraints.push(super::ConstraintInfo {
                    kind: "UNIQUE".into(),
                    name: idx.name.clone(),
                    definition: idx.columns.join(", "),
                });
            }
        }
        Ok(super::TableSchema {
            name: table.to_string(),
            kind,
            columns,
            constraints,
            indexes,
            ddl,
        })
    }
}

impl SqliteDb {
    fn object_kind(&self, name: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT type FROM sqlite_master WHERE name = ?1")
            .map_err(sqlite_err)?;
        let mut rows = stmt.query([name]).map_err(sqlite_err)?;
        if let Some(row) = rows.next().map_err(sqlite_err)? {
            Ok(Some(row.get(0).map_err(sqlite_err)?))
        } else {
            Ok(None)
        }
    }

    fn foreign_key_constraints(&self, table: &str) -> Result<Vec<super::ConstraintInfo>> {
        let sql = format!("PRAGMA foreign_key_list({table})");
        let mut stmt = self.conn.prepare(&sql).map_err(sqlite_err)?;
        let mut rows = stmt.query([]).map_err(sqlite_err)?;
        // Group by id
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<i64, (String, Vec<(String, String)>, String, String)> =
            BTreeMap::new();
        while let Some(row) = rows.next().map_err(sqlite_err)? {
            let id: i64 = row.get(0).map_err(sqlite_err)?;
            let seq: i64 = row.get(1).map_err(sqlite_err)?;
            let parent: String = row.get(2).map_err(sqlite_err)?;
            let from: String = row.get(3).map_err(sqlite_err)?;
            let to: String = row.get(4).map_err(sqlite_err)?;
            let on_update: String = row.get(5).map_err(sqlite_err)?;
            let on_delete: String = row.get(6).map_err(sqlite_err)?;
            let entry = groups.entry(id).or_insert_with(|| {
                (parent, Vec::new(), on_update.clone(), on_delete.clone())
            });
            // Keep seq order
            let _ = seq;
            entry.1.push((from, to));
            entry.2 = on_update;
            entry.3 = on_delete;
        }
        let mut out = Vec::new();
        for (id, (parent, cols, on_update, on_delete)) in groups {
            let froms: Vec<_> = cols.iter().map(|(f, _)| f.as_str()).collect();
            let tos: Vec<_> = cols.iter().map(|(_, t)| t.as_str()).collect();
            let mut def = format!(
                "FOREIGN KEY ({}) REFERENCES {} ({})",
                froms.join(", "),
                parent,
                tos.join(", ")
            );
            if !on_update.is_empty() && on_update != "NO ACTION" {
                def.push_str(&format!(" ON UPDATE {on_update}"));
            }
            if !on_delete.is_empty() && on_delete != "NO ACTION" {
                def.push_str(&format!(" ON DELETE {on_delete}"));
            }
            out.push(super::ConstraintInfo {
                kind: "FOREIGN KEY".into(),
                name: format!("fk_{id}"),
                definition: def,
            });
        }
        Ok(out)
    }

    fn list_indexes(&self, table: &str) -> Result<Vec<super::IndexInfo>> {
        let sql = format!("PRAGMA index_list({table})");
        let mut stmt = self.conn.prepare(&sql).map_err(sqlite_err)?;
        let mut rows = stmt.query([]).map_err(sqlite_err)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(sqlite_err)? {
            let name: String = row.get(1).map_err(sqlite_err)?;
            let unique: i64 = row.get(2).map_err(sqlite_err)?;
            let origin: String = row.get(3).map_err(sqlite_err)?;
            let columns = self.index_columns(&name)?;
            out.push(super::IndexInfo {
                name,
                unique: unique != 0,
                origin,
                columns,
            });
        }
        Ok(out)
    }

    fn index_columns(&self, index: &str) -> Result<Vec<String>> {
        validate_ident(index)?;
        let sql = format!("PRAGMA index_info({index})");
        let mut stmt = self.conn.prepare(&sql).map_err(sqlite_err)?;
        let mut rows = stmt.query([]).map_err(sqlite_err)?;
        let mut cols = Vec::new();
        while let Some(row) = rows.next().map_err(sqlite_err)? {
            let name: Option<String> = row.get(2).map_err(sqlite_err)?;
            cols.push(name.unwrap_or_else(|| "«expr»".into()));
        }
        Ok(cols)
    }
}

fn build_column_constraints(columns: &[super::ColumnInfo]) -> Vec<super::ConstraintInfo> {
    let mut out = Vec::new();
    let pk: Vec<&str> = columns
        .iter()
        .filter(|c| c.primary_key)
        .map(|c| c.name.as_str())
        .collect();
    if !pk.is_empty() {
        out.push(super::ConstraintInfo {
            kind: "PRIMARY KEY".into(),
            name: "pk".into(),
            definition: pk.join(", "),
        });
    }
    for col in columns {
        if col.not_null && !col.primary_key {
            out.push(super::ConstraintInfo {
                kind: "NOT NULL".into(),
                name: col.name.clone(),
                definition: col.name.clone(),
            });
        }
    }
    out
}

fn extract_check_constraints(ddl: &str) -> Vec<super::ConstraintInfo> {
    let mut out = Vec::new();
    let upper = ddl.to_ascii_uppercase();
    let mut search_from = 0;
    let mut n = 0;
    while let Some(rel) = upper[search_from..].find("CHECK") {
        let abs = search_from + rel;
        let after = ddl[abs + 5..].trim_start();
        if let Some(rest) = after.strip_prefix('(') {
            if let Some(end) = find_matching_paren(rest) {
                let expr = rest[..end].trim();
                n += 1;
                out.push(super::ConstraintInfo {
                    kind: "CHECK".into(),
                    name: format!("check_{n}"),
                    definition: expr.to_string(),
                });
                search_from = abs + 5 + (after.len() - rest.len()) + end + 1;
                continue;
            }
        }
        search_from = abs + 5;
    }
    out
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn sqlite_err(err: rusqlite::Error) -> Error {
    match err {
        rusqlite::Error::SqlInputError {
            msg,
            sql,
            offset,
            ..
        } if offset >= 0 => Error::backend_at("sqlite", msg, sql, offset as usize),
        other => Error::backend("sqlite", other.to_string()),
    }
}

fn sqlite_err_sql(err: rusqlite::Error, sql: &str) -> Error {
    match err {
        rusqlite::Error::SqlInputError {
            msg,
            sql: err_sql,
            offset,
            ..
        } if offset >= 0 => {
            let source = if err_sql.is_empty() {
                sql.to_string()
            } else {
                err_sql
            };
            Error::backend_at("sqlite", msg, source, offset as usize)
        }
        other => {
            let message = other.to_string();
            if let Some(offset) = near_token_offset(sql, &message) {
                Error::backend_at("sqlite", message, sql, offset)
            } else {
                Error::backend("sqlite", message)
            }
        }
    }
}

/// Best-effort byte offset from common SQLite message shapes.
fn near_token_offset(sql: &str, message: &str) -> Option<usize> {
    let token = extract_error_token(message)?;
    find_ident(sql, token)
}

fn extract_error_token(message: &str) -> Option<&str> {
    let lower = message.to_ascii_lowercase();
    if let Some(idx) = lower.find("near ") {
        let rest = message[idx + 5..].trim_start();
        return Some(quoted_or_word(rest));
    }
    for prefix in ["no such table:", "no such column:", "no such view:"] {
        if let Some(idx) = lower.find(prefix) {
            return Some(message[idx + prefix.len()..].trim());
        }
    }
    None
}

fn quoted_or_word(rest: &str) -> &str {
    if let Some(stripped) = rest.strip_prefix('"') {
        stripped.split('"').next().unwrap_or("")
    } else {
        rest.split(|c: char| c == ':' || c.is_whitespace())
            .next()
            .unwrap_or("")
    }
}

fn find_ident(sql: &str, token: &str) -> Option<usize> {
    if token.is_empty() {
        return None;
    }
    let lower_sql = sql.to_ascii_lowercase();
    let lower_tok = token.to_ascii_lowercase();
    let mut start = 0;
    while let Some(rel) = lower_sql[start..].find(&lower_tok) {
        let abs = start + rel;
        let before_ok = abs == 0
            || !sql.as_bytes()[abs - 1].is_ascii_alphanumeric() && sql.as_bytes()[abs - 1] != b'_';
        let after = abs + token.len();
        let after_ok = after >= sql.len()
            || !sql.as_bytes()[after].is_ascii_alphanumeric() && sql.as_bytes()[after] != b'_';
        if before_ok && after_ok {
            return Some(abs);
        }
        start = abs + 1;
    }
    None
}

fn value_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".into(),
        ValueRef::Integer(v) => v.to_string(),
        ValueRef::Real(v) => v.to_string(),
        ValueRef::Text(v) => String::from_utf8_lossy(v).into_owned(),
        ValueRef::Blob(v) => format!("<blob {} bytes>", v.len()),
    }
}

fn infer_type_name(value: ValueRef<'_>) -> &'static str {
    match value {
        ValueRef::Null => "?",
        ValueRef::Integer(_) => "INTEGER",
        ValueRef::Real(_) => "REAL",
        ValueRef::Text(_) => "TEXT",
        ValueRef::Blob(_) => "BLOB",
    }
}

fn validate_ident(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(Error::msg(format!("invalid identifier: {name}")));
    }
    Ok(())
}

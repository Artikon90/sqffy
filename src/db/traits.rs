use crate::error::Result;

/// One logical row as displayable strings (typed rendering comes later).
pub type Row = Vec<String>;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    /// Declared / inferred SQL types aligned with [`columns`] (may be empty/`?`).
    pub column_types: Vec<String>,
    pub rows: Vec<Row>,
    /// True when `rows` was cut by the configured limit.
    pub truncated: bool,
    pub rows_affected: Option<u64>,
}

impl QueryResult {
    pub fn empty_affected(rows_affected: u64) -> Self {
        Self {
            columns: vec![],
            column_types: vec![],
            rows: vec![],
            truncated: false,
            rows_affected: Some(rows_affected),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub kind: String,
    /// Optional schema/namespace (Postgres `public`, …). Empty for SQLite.
    pub schema: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
    /// Column comment when the backend exposes one (often empty on SQLite).
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    pub kind: String,
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub unique: bool,
    pub origin: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub kind: String,
    pub columns: Vec<ColumnInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub indexes: Vec<IndexInfo>,
    pub ddl: Option<String>,
}

/// What a backend can do — meta/REPL adapt instead of assuming SQLite.
#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    /// Supports SQL schemas / namespaces (Postgres).
    pub schemas: bool,
    /// Can return `CREATE` DDL text for `.schema <table>`.
    pub table_ddl: bool,
    /// Identifier quoting style hint for future SQL rewrite helpers.
    pub quote_char: char,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            schemas: false,
            table_ddl: true,
            quote_char: '"',
        }
    }
}

/// Open session used by meta commands and the REPL.
///
/// Implement this for each database. Register a matching [`super::Driver`]
/// so `App` can open connections without knowing the concrete type.
pub trait Database: Send {
    fn driver_id(&self) -> &'static str;

    /// Human-readable connection label for the prompt.
    fn display_label(&self) -> &str;

    fn capabilities(&self) -> Capabilities;

    fn execute_sql(&mut self, sql: &str, row_limit: usize) -> Result<QueryResult>;

    fn list_tables(&self) -> Result<Vec<TableInfo>>;

    fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>>;

    /// Raw `CREATE` SQL for a table/view, if the backend supports it.
    fn table_ddl(&self, table: &str) -> Result<Option<String>>;

    /// Columns + constraints + indexes for `.schema <table>`.
    /// Default: columns only (from [`describe_table`]).
    fn inspect_table(&self, table: &str) -> Result<TableSchema> {
        let columns = self.describe_table(table)?;
        Ok(TableSchema {
            name: table.to_string(),
            kind: "table".into(),
            columns,
            constraints: Vec::new(),
            indexes: Vec::new(),
            ddl: self.table_ddl(table).ok().flatten(),
        })
    }
}

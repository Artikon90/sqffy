use crate::db::{ColumnInfo, Database, TableInfo};
use crate::error::Result;
use std::collections::HashMap;

use super::context::TableRef;

#[derive(Debug, Default, Clone)]
pub struct SchemaCache {
    tables: Vec<TableInfo>,
    columns: HashMap<String, Vec<ColumnInfo>>,
}

impl SchemaCache {
    pub fn from_parts(
        tables: Vec<TableInfo>,
        columns: HashMap<String, Vec<ColumnInfo>>,
    ) -> Self {
        Self { tables, columns }
    }

    pub fn refresh(db: &dyn Database) -> Result<Self> {
        let tables = db.list_tables()?;
        let mut columns = HashMap::new();
        for table in &tables {
            if table.kind == "table" {
                if let Ok(cols) = db.describe_table(&table.name) {
                    columns.insert(table.name.clone(), cols);
                }
            }
        }
        Ok(Self { tables, columns })
    }

    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.tables.iter().map(|t| t.name.as_str())
    }

    /// Resolve `alias_or_table` against scope, then schema.
    pub fn resolve_table<'a>(
        &'a self,
        qualifier: &str,
        scope: &'a [TableRef],
    ) -> Option<&'a str> {
        for r in scope {
            if r.alias
                .as_deref()
                .is_some_and(|a| a.eq_ignore_ascii_case(qualifier))
            {
                return Some(r.table.as_str());
            }
            if r.table.eq_ignore_ascii_case(qualifier) {
                return Some(r.table.as_str());
            }
        }
        self.columns
            .keys()
            .find(|k| k.eq_ignore_ascii_case(qualifier))
            .map(|s| s.as_str())
            .or_else(|| {
                self.tables
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(qualifier))
                    .map(|t| t.name.as_str())
            })
    }

    pub fn column_names_ci<'a>(&'a self, table: &str) -> Option<impl Iterator<Item = &'a str> + 'a> {
        self.columns
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(table))
            .map(|(_, cols)| cols.iter().map(|c| c.name.as_str()))
    }

    /// Columns for tables in scope (deduped). Empty scope → all columns.
    pub fn columns_for_scope(&self, scope: &[TableRef]) -> Vec<String> {
        if scope.is_empty() {
            return self
                .columns
                .values()
                .flat_map(|cols| cols.iter().map(|c| c.name.clone()))
                .collect();
        }

        let mut out = Vec::new();
        for r in scope {
            if let Some(cols) = self.column_names_ci(&r.table) {
                for name in cols {
                    if !out.iter().any(|c: &String| c.eq_ignore_ascii_case(name)) {
                        out.push(name.to_string());
                    }
                }
            }
        }
        out
    }

    pub fn columns_of(&self, table: &str) -> Vec<String> {
        self.column_names_ci(table)
            .map(|it| it.map(str::to_string).collect())
            .unwrap_or_default()
    }
}

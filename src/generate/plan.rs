use super::DEFAULT_BATCH_SIZE;
use crate::db::{ColumnInfo, TableSchema};

#[derive(Debug, Clone)]
pub struct FkRef {
    pub column: String,
    pub ref_table: String,
    pub ref_column: String,
    pub pool: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratePlan {
    pub table: String,
    pub columns: Vec<String>,
    pub types: Vec<String>,
    pub fks: Vec<FkRef>,
    pub batch_size: usize,
}

impl GeneratePlan {
    /// Build a plan from schema; skip auto-increment / serial PK columns.
    pub fn from_schema(schema: &TableSchema, skip_pk: bool) -> Self {
        let mut columns = Vec::new();
        let mut types = Vec::new();
        for col in &schema.columns {
            if skip_pk && col.primary_key && looks_auto(col) {
                continue;
            }
            columns.push(col.name.clone());
            types.push(col.type_name.clone());
        }
        Self {
            table: schema.name.clone(),
            columns,
            types,
            fks: Vec::new(),
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    pub fn add_fk(&mut self, column: &str, ref_table: &str, ref_column: &str) {
        self.fks.push(FkRef {
            column: column.to_string(),
            ref_table: ref_table.to_string(),
            ref_column: ref_column.to_string(),
            pool: Vec::new(),
        });
    }

    pub fn fk_for(&self, column: &str) -> Option<&FkRef> {
        self.fks.iter().find(|f| f.column.eq_ignore_ascii_case(column))
    }
}

fn looks_auto(col: &ColumnInfo) -> bool {
    let t = col.type_name.to_ascii_uppercase();
    t.contains("SERIAL")
        || t.contains("IDENTITY")
        || col
            .default_value
            .as_deref()
            .is_some_and(|d| d.to_ascii_lowercase().contains("nextval"))
}

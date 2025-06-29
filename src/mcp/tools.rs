use super::guard::is_readonly_sql;
use crate::db::default_registry;
use crate::error::{Error, Result};
use crate::pool::ConnectionPool;
use serde_json::{json, Value};

pub struct ToolContext {
    pub pool: ConnectionPool,
}

impl ToolContext {
    pub fn call(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "list_connections" => self.list_connections(),
            "list_tables" => self.list_tables(args),
            "describe_table" => self.describe_table(args),
            "inspect_table" => self.inspect_table(args),
            "execute_readonly" => self.execute_readonly(args),
            "list_drivers" => self.list_drivers(),
            other => Err(Error::msg(format!("unknown tool: {other}"))),
        }
    }

    pub fn catalog() -> Value {
        json!([
            {
                "name": "list_connections",
                "description": "List open database connections in this SQffy session (multi-DB)",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "list_tables",
                "description": "List tables/views for a connection",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "connection": { "type": "string", "description": "Connection name (default: active)" }
                    }
                }
            },
            {
                "name": "describe_table",
                "description": "Describe columns of a table",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "table": { "type": "string" },
                        "connection": { "type": "string" }
                    },
                    "required": ["table"]
                }
            },
            {
                "name": "inspect_table",
                "description": "Columns, constraints, indexes for a table",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "table": { "type": "string" },
                        "connection": { "type": "string" }
                    },
                    "required": ["table"]
                }
            },
            {
                "name": "execute_readonly",
                "description": "Run a read-only SQL query (SELECT/WITH/SHOW/EXPLAIN/PRAGMA)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sql": { "type": "string" },
                        "limit": { "type": "integer", "default": 100 },
                        "connection": { "type": "string" }
                    },
                    "required": ["sql"]
                }
            },
            {
                "name": "list_drivers",
                "description": "List built-in database drivers",
                "inputSchema": { "type": "object", "properties": {} }
            }
        ])
    }

    fn list_connections(&self) -> Result<Value> {
        let items: Vec<Value> = self
            .pool
            .list()
            .into_iter()
            .enumerate()
            .map(|(i, m)| {
                json!({
                    "name": m.name,
                    "driver": m.driver,
                    "label": m.label,
                    "default": i == 0,
                })
            })
            .collect();
        Ok(json!({ "connections": items, "count": items.len() }))
    }

    fn list_tables(&self, args: &Value) -> Result<Value> {
        let conn_name = args.get("connection").and_then(|v| v.as_str());
        let pooled = self.pool.get(conn_name)?;
        let db = pooled.db.lock().map_err(|_| Error::msg("connection lock"))?;
        let tables = db.list_tables()?;
        Ok(json!(tables
            .into_iter()
            .map(|t| json!({
                "name": t.name,
                "kind": t.kind,
                "schema": t.schema,
            }))
            .collect::<Vec<_>>()))
    }

    fn describe_table(&self, args: &Value) -> Result<Value> {
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::msg("missing table"))?;
        let conn_name = args.get("connection").and_then(|v| v.as_str());
        let pooled = self.pool.get(conn_name)?;
        let db = pooled.db.lock().map_err(|_| Error::msg("connection lock"))?;
        let cols = db.describe_table(table)?;
        Ok(json!(cols
            .into_iter()
            .map(|c| json!({
                "name": c.name,
                "type": c.type_name,
                "not_null": c.not_null,
                "default": c.default_value,
                "pk": c.primary_key,
                "comment": c.comment,
            }))
            .collect::<Vec<_>>()))
    }

    fn inspect_table(&self, args: &Value) -> Result<Value> {
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::msg("missing table"))?;
        let conn_name = args.get("connection").and_then(|v| v.as_str());
        let pooled = self.pool.get(conn_name)?;
        let db = pooled.db.lock().map_err(|_| Error::msg("connection lock"))?;
        let schema = db.inspect_table(table)?;
        Ok(serde_json::to_value(schema_json(&schema)).unwrap_or(json!({})))
    }

    fn execute_readonly(&self, args: &Value) -> Result<Value> {
        let sql = args
            .get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::msg("missing sql"))?;
        if !is_readonly_sql(sql) {
            return Err(Error::msg(
                "MCP is read-only: only SELECT/WITH/SHOW/EXPLAIN/PRAGMA allowed",
            ));
        }
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
        let conn_name = args.get("connection").and_then(|v| v.as_str());
        let pooled = self.pool.get(conn_name)?;
        let mut db = pooled.db.lock().map_err(|_| Error::msg("connection lock"))?;
        let result = db.execute_sql(sql, limit)?;
        Ok(json!({
            "columns": result.columns,
            "column_types": result.column_types,
            "rows": result.rows,
            "truncated": result.truncated,
            "rows_affected": result.rows_affected,
        }))
    }

    fn list_drivers(&self) -> Result<Value> {
        let items: Vec<Value> = default_registry()
            .list()
            .into_iter()
            .map(|(id, desc)| json!({ "id": id, "description": desc }))
            .collect();
        Ok(json!({ "drivers": items }))
    }
}

fn schema_json(schema: &crate::db::TableSchema) -> Value {
    json!({
        "name": schema.name,
        "kind": schema.kind,
        "columns": schema.columns.iter().map(|c| json!({
            "name": c.name,
            "type": c.type_name,
            "not_null": c.not_null,
            "default": c.default_value,
            "pk": c.primary_key,
            "comment": c.comment,
        })).collect::<Vec<_>>(),
        "constraints": schema.constraints.iter().map(|c| json!({
            "kind": c.kind,
            "name": c.name,
            "definition": c.definition,
        })).collect::<Vec<_>>(),
        "indexes": schema.indexes.iter().map(|i| json!({
            "name": i.name,
            "unique": i.unique,
            "origin": i.origin,
            "columns": i.columns,
        })).collect::<Vec<_>>(),
        "ddl": schema.ddl,
    })
}

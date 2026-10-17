use super::cache::SchemaCache;
use super::context::{self, Clause, CompletionContext, StatementKind};

const META_COMMANDS: &[&str] = &[
    ".help",
    ".h",
    ".tables",
    ".t",
    ".schema",
    ".s",
    ".describe",
    ".d",
    ".generate",
    ".gen",
    ".quit",
    ".q",
    ".exit",
];

/// Produces ranked completion candidates for the current buffer.
#[derive(Debug, Clone)]
pub struct Completer {
    cache: SchemaCache,
}

impl Completer {
    pub fn new(cache: SchemaCache) -> Self {
        Self { cache }
    }

    pub fn set_cache(&mut self, cache: SchemaCache) {
        self.cache = cache;
    }

    /// Candidates for the token being typed at the end of `line`.
    pub fn candidates(&self, line: &str) -> Vec<String> {
        let trimmed = line.trim_start();
        if trimmed.starts_with('.') {
            return self.meta_candidates(trimmed);
        }

        let ctx = context::analyze(line);
        self.rank(&ctx)
    }

    fn rank(&self, ctx: &CompletionContext) -> Vec<String> {
        let mut columns = Vec::new();
        let mut tables = Vec::new();
        let mut keywords = Vec::new();

        // `alias.` / `table.` → only that relation's columns
        if let Some(qual) = ctx.qualifier.as_deref() {
            if let Some(table) = self.cache.resolve_table(qual, &ctx.scope) {
                for col in self.cache.columns_of(table) {
                    if prefix_match(&col, &ctx.prefix) {
                        columns.push(col);
                    }
                }
            } else {
                // Unknown qualifier — still try as bare table name via all columns of matching tables
                for col in self.cache.columns_of(qual) {
                    if prefix_match(&col, &ctx.prefix) {
                        columns.push(col);
                    }
                }
            }
            return finalize(columns, Vec::new(), Vec::new());
        }

        match ctx.clause {
            Clause::Start => {
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
            }
            Clause::SelectList
            | Clause::On
            | Clause::Where
            | Clause::Having
            | Clause::Set
            | Clause::GroupBy
            | Clause::OrderBy => {
                for col in self.cache.columns_for_scope(&ctx.scope) {
                    if prefix_match(&col, &ctx.prefix) {
                        columns.push(col);
                    }
                }
                // Bare table/alias names are useful in SELECT (t.…) and ON
                if matches!(ctx.clause, Clause::SelectList | Clause::On) {
                    self.push_scope_relations(&mut tables, ctx);
                    self.push_matching_tables(&mut tables, &ctx.prefix);
                }
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
            }
            Clause::From
            | Clause::Join
            | Clause::Into
            | Clause::UpdateTable
            | Clause::TableName => {
                self.push_matching_tables(&mut tables, &ctx.prefix);
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
            }
            Clause::ObjectType
            | Clause::ColName
            | Clause::ColTypeTail
            | Clause::ColTail
            | Clause::ColKey
            | Clause::ColNot => {
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
            }
            Clause::ColType => {
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
                for name in crate::sql::type_names_matching(&ctx.prefix) {
                    push_unique(&mut keywords, name.to_string());
                }
            }
            Clause::ColRef => {
                self.push_matching_tables(&mut tables, &ctx.prefix);
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
            }
            Clause::Values | Clause::Limit => {
                push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
            }
        }

        // Empty start with empty prefix: still offer starters
        if ctx.statement == StatementKind::Empty
            && ctx.clause == Clause::Start
            && keywords.is_empty()
        {
            push_kw(&mut keywords, context::clause_keywords(ctx), &ctx.prefix);
        }

        finalize(columns, tables, keywords)
    }

    fn meta_candidates(&self, line: &str) -> Vec<String> {
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let rest = parts.next();

        if rest.is_none() && !line.contains(char::is_whitespace) {
            let lower = cmd.to_ascii_lowercase();
            return META_COMMANDS
                .iter()
                .filter(|c| c.starts_with(&lower))
                .map(|c| (*c).to_string())
                .collect();
        }

        let cmd_lower = cmd.to_ascii_lowercase();
        if matches!(cmd_lower.as_str(), ".schema" | ".s" | ".describe" | ".d") {
            let prefix = rest.unwrap_or("");
            let mut tables = Vec::new();
            self.push_matching_tables(&mut tables, prefix);
            return finalize(Vec::new(), tables, Vec::new());
        }

        Vec::new()
    }

    fn push_matching_tables(&self, out: &mut Vec<String>, prefix: &str) {
        for name in self.cache.table_names() {
            if prefix_match(name, prefix) {
                push_unique(out, name.to_string());
            }
        }
    }

    fn push_scope_relations(&self, out: &mut Vec<String>, ctx: &CompletionContext) {
        for r in &ctx.scope {
            if let Some(alias) = &r.alias {
                if prefix_match(alias, &ctx.prefix) {
                    push_unique(out, alias.clone());
                }
            }
            if prefix_match(&r.table, &ctx.prefix) {
                push_unique(out, r.table.clone());
            }
        }
    }
}

fn push_kw(out: &mut Vec<String>, kws: &[&str], prefix: &str) {
    let upper = prefix.to_ascii_uppercase();
    for kw in kws {
        if kw.starts_with(&upper) {
            push_unique(out, (*kw).to_string());
        }
    }
}

fn prefix_match(value: &str, prefix: &str) -> bool {
    prefix.is_empty()
        || value
            .to_ascii_lowercase()
            .starts_with(&prefix.to_ascii_lowercase())
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !out.iter().any(|c| c.eq_ignore_ascii_case(&value)) {
        out.push(value);
    }
}

/// Order: columns → tables → keywords.
/// Names are alphabetical. Keywords keep the likelihood order from `clause_keywords`.
fn finalize(
    mut columns: Vec<String>,
    mut tables: Vec<String>,
    keywords: Vec<String>,
) -> Vec<String> {
    let sort_ci = |v: &mut Vec<String>| v.sort_by_key(|s| s.to_ascii_lowercase());
    sort_ci(&mut columns);
    sort_ci(&mut tables);

    let mut out = Vec::with_capacity(columns.len() + tables.len() + keywords.len());
    for c in columns {
        push_unique(&mut out, c);
    }
    for t in tables {
        push_unique(&mut out, t);
    }
    for k in keywords {
        push_unique(&mut out, k);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ColumnInfo, TableInfo};
    use std::collections::HashMap;

    fn sample_cache() -> SchemaCache {
        let mut columns = HashMap::new();
        columns.insert("users".into(), vec![col("id"), col("name"), col("email")]);
        columns.insert(
            "orders".into(),
            vec![col("id"), col("user_id"), col("total")],
        );
        SchemaCache::from_parts(
            vec![
                TableInfo {
                    name: "users".into(),
                    schema: None,
                    kind: "table".into(),
                },
                TableInfo {
                    name: "orders".into(),
                    schema: None,
                    kind: "table".into(),
                },
            ],
            columns,
        )
    }

    fn col(name: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            type_name: "TEXT".into(),
            not_null: false,
            default_value: None,
            primary_key: false,
            comment: None,
        }
    }

    #[test]
    fn where_prefers_scoped_columns() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SELECT * FROM users WHERE n");
        assert!(got.iter().any(|s| s == "name"));
        // `total` is only on orders — should not appear when scope is users
        assert!(!got.iter().any(|s| s == "total"));
        // clause keyword still available, after the column
        let name_at = got.iter().position(|s| s == "name").unwrap();
        let not_at = got.iter().position(|s| s == "NOT").unwrap();
        assert!(name_at < not_at, "{got:?}");
    }

    #[test]
    fn where_lists_columns_then_keywords() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SELECT * FROM users WHERE ");
        let expected = [
            "email", "id", "name", "NOT", "EXISTS", "AND", "OR", "IS", "IN", "LIKE", "BETWEEN",
            "NULL", "TRUE", "FALSE", "GROUP", "ORDER", "LIMIT", "HAVING", "JOIN",
        ];
        assert_eq!(
            got.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn from_suggests_tables_not_all_columns() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SELECT * FROM u");
        assert!(got.iter().any(|s| s == "users"));
        assert!(!got.iter().any(|s| s == "email"));
    }

    #[test]
    fn alias_dot_resolves_columns() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SELECT * FROM users u WHERE u.e");
        assert_eq!(got, vec!["email".to_string()]);
    }

    #[test]
    fn join_on_sees_both_tables_columns() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SELECT * FROM users u JOIN orders o ON ");
        assert!(got.iter().any(|s| s == "user_id"));
        assert!(got.iter().any(|s| s == "email"));
    }

    #[test]
    fn update_set_scoped() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("UPDATE users SET n");
        assert!(got.iter().any(|s| s == "name"));
        assert!(!got.iter().any(|s| s == "total"));
    }

    #[test]
    fn start_suggests_select() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SEL");
        assert!(got.iter().any(|s| s == "SELECT"));
        assert!(!got.iter().any(|s| s == "users"));
    }

    #[test]
    fn meta_tables() {
        let c = Completer::new(sample_cache());
        let got = c.candidates(".ta");
        assert!(got.iter().any(|s| s == ".tables"));
    }

    #[test]
    fn select_list_offers_from_keyword() {
        let c = Completer::new(sample_cache());
        let got = c.candidates("SELECT id F");
        assert!(got.iter().any(|s| s == "FROM"));
    }

    #[test]
    fn create_table_suggests_type_and_primary_key() {
        let c = Completer::new(sample_cache());

        let types = c.candidates("CREATE TABLE aboba(id ");
        assert!(types.iter().any(|s| s == "TEXT"), "{types:?}");
        assert!(types.iter().any(|s| s == "INTEGER"), "{types:?}");

        let partial = c.candidates("CREATE TABLE aboba(id TE");
        assert!(partial.iter().any(|s| s == "TEXT"), "{partial:?}");

        let keys = c.candidates("CREATE TABLE aboba(id TEXT PRIMAR");
        assert!(keys.iter().any(|s| s == "PRIMARY KEY"), "{keys:?}");

        let after_primary = c.candidates("CREATE TABLE aboba(id TEXT PRIMARY ");
        assert_eq!(after_primary, vec!["KEY".to_string()]);
    }
}

//! Lightweight clause/scope scanner for unfinished SQL.
//!
//! Not a full parser — intentionally tolerant of incomplete input so Tab
//! completion still works mid-statement.

use crate::sql::{is_keyword, split_tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementKind {
    Empty,
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clause {
    /// Beginning of a statement (or after `;`).
    Start,
    SelectList,
    From,
    Join,
    On,
    Where,
    GroupBy,
    OrderBy,
    Having,
    Limit,
    Set,
    Values,
    Into,
    /// Right after `UPDATE` — expecting table name.
    UpdateTable,
    /// After `CREATE`/`DROP` — expecting TABLE/VIEW/INDEX.
    ObjectType,
    /// After `CREATE TABLE` / `DROP TABLE` — expecting object name.
    TableName,
    /// Inside `CREATE TABLE (` — column name, or a table-level constraint.
    ColName,
    /// After a column name — expecting a type (`INTEGER`, `TEXT`, …).
    ColType,
    /// Multi-word type still open (`DOUBLE`, `TIMESTAMP WITH`, …).
    ColTypeTail,
    /// After the type — `PRIMARY KEY`, `NOT NULL`, …
    ColTail,
    /// After `PRIMARY` / `FOREIGN`.
    ColKey,
    /// After `NOT`.
    ColNot,
    /// After `REFERENCES` — expecting a table.
    ColRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRef {
    pub table: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionContext {
    pub statement: StatementKind,
    pub clause: Clause,
    pub scope: Vec<TableRef>,
    /// Token currently being typed (may be empty after a space).
    pub prefix: String,
    /// `alias` / `table` when completing `qualifier.prefix`.
    pub qualifier: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    None,
    Table,
    /// Saw table; next non-keyword ident is alias (or `AS`).
    OptionalAlias,
    AliasAfterAs,
    /// After `GROUP` / `ORDER` — waiting for `BY`.
    By,
}

/// Analyze `line` (text before cursor) into a completion context.
pub fn analyze(line: &str) -> CompletionContext {
    let segment = last_statement(line);
    let (complete, prefix, qualifier) = split_active_token(segment);

    let mut statement = StatementKind::Empty;
    let mut clause = Clause::Start;
    let mut scope: Vec<TableRef> = Vec::new();
    let mut expect = Expect::None;
    let mut paren: i32 = 0;
    let mut create_paren: Option<i32> = None;
    let mut nested: i32 = 0;
    let mut col_phase = ColPhase::Name;

    for raw in significant_tokens(&complete) {
        let upper = raw.to_ascii_uppercase();
        let upper_s = upper.as_str();

        if upper_s == "(" {
            paren += 1;
            if create_paren.is_none()
                && clause == Clause::TableName
                && expect != Expect::Table
                && statement == StatementKind::Create
            {
                create_paren = Some(paren);
                col_phase = ColPhase::Name;
                clause = Clause::ColName;
                expect = Expect::None;
            } else if create_paren.is_some() {
                nested += 1;
            }
            continue;
        }
        if upper_s == ")" {
            if nested > 0 {
                nested -= 1;
                if nested == 0 && create_paren.is_some() {
                    col_phase = ColPhase::Tail;
                    clause = Clause::ColTail;
                }
            } else if create_paren == Some(paren) {
                create_paren = None;
                clause = Clause::Start;
                expect = Expect::None;
            }
            paren = paren.saturating_sub(1);
            continue;
        }
        if create_paren.is_some() {
            if nested == 0 {
                if upper_s == "," {
                    col_phase = ColPhase::Name;
                    clause = Clause::ColName;
                } else if is_word(raw) {
                    apply_col_phase(upper_s, &mut col_phase, &mut clause);
                }
            }
            continue;
        }

        // Join modifiers don't change expect/clause by themselves.
        if matches!(
            upper_s,
            "LEFT" | "RIGHT" | "INNER" | "OUTER" | "FULL" | "CROSS" | "NATURAL"
        ) {
            continue;
        }

        if expect == Expect::By {
            if upper_s == "BY" {
                expect = Expect::None;
                continue;
            }
            expect = Expect::None;
            // fall through — unexpected token after GROUP/ORDER
        }

        match upper_s {
            "SELECT" => {
                statement = StatementKind::Select;
                clause = Clause::SelectList;
                expect = Expect::None;
            }
            "INSERT" => {
                statement = StatementKind::Insert;
                clause = Clause::Start;
                expect = Expect::None;
            }
            "INTO" => {
                if statement == StatementKind::Empty {
                    statement = StatementKind::Insert;
                }
                clause = Clause::Into;
                expect = Expect::Table;
            }
            "UPDATE" => {
                statement = StatementKind::Update;
                clause = Clause::UpdateTable;
                expect = Expect::Table;
            }
            "DELETE" => {
                statement = StatementKind::Delete;
                clause = Clause::Start;
                expect = Expect::None;
            }
            "CREATE" => {
                statement = StatementKind::Create;
                clause = Clause::ObjectType;
                expect = Expect::None;
            }
            "DROP" => {
                statement = StatementKind::Drop;
                clause = Clause::ObjectType;
                expect = Expect::None;
            }
            "TABLE" | "VIEW" | "INDEX"
                if matches!(statement, StatementKind::Create | StatementKind::Drop) =>
            {
                clause = Clause::TableName;
                expect = Expect::Table;
            }
            "IF" | "TEMPORARY" | "TEMP" | "UNLOGGED" | "GLOBAL" | "LOCAL"
                if matches!(clause, Clause::ObjectType | Clause::TableName) => {}
            "NOT" | "EXISTS" if clause == Clause::TableName && expect == Expect::Table => {}
            "FROM" => {
                clause = Clause::From;
                expect = Expect::Table;
            }
            "JOIN" => {
                clause = Clause::Join;
                expect = Expect::Table;
            }
            "ON" => {
                clause = Clause::On;
                expect = Expect::None;
            }
            "WHERE" => {
                clause = Clause::Where;
                expect = Expect::None;
            }
            "SET" => {
                clause = Clause::Set;
                expect = Expect::None;
            }
            "VALUES" => {
                clause = Clause::Values;
                expect = Expect::None;
            }
            "HAVING" => {
                clause = Clause::Having;
                expect = Expect::None;
            }
            "GROUP" => {
                clause = Clause::GroupBy;
                expect = Expect::By;
            }
            "ORDER" => {
                clause = Clause::OrderBy;
                expect = Expect::By;
            }
            "LIMIT" | "OFFSET" => {
                clause = Clause::Limit;
                expect = Expect::None;
            }
            "," if matches!(clause, Clause::From | Clause::Join | Clause::Into) => {
                expect = Expect::Table;
            }
            "AS" if expect == Expect::OptionalAlias => {
                expect = Expect::AliasAfterAs;
            }
            _ => {
                if is_ident(raw) {
                    match expect {
                        Expect::Table => {
                            // Object name after CREATE/DROP TABLE — don't put in query scope
                            if clause == Clause::TableName {
                                expect = Expect::None;
                            } else {
                                scope.push(TableRef {
                                    table: raw.to_string(),
                                    alias: None,
                                });
                                expect = Expect::OptionalAlias;
                            }
                        }
                        Expect::OptionalAlias | Expect::AliasAfterAs => {
                            if is_clause_starter(upper_s) {
                                // Re-process as keyword — recurse one step via match logic
                                // by applying clause transition without consuming as alias.
                                expect = Expect::None;
                                apply_clause_keyword(
                                    upper_s,
                                    &mut statement,
                                    &mut clause,
                                    &mut expect,
                                );
                            } else if !is_keyword(raw) || upper_s == "AS" {
                                if let Some(last) = scope.last_mut() {
                                    last.alias = Some(raw.to_string());
                                }
                                expect = Expect::None;
                            } else {
                                expect = Expect::None;
                            }
                        }
                        Expect::None | Expect::By => {}
                    }
                } else if raw == "," && matches!(clause, Clause::From | Clause::Join | Clause::Into)
                {
                    expect = Expect::Table;
                }
            }
        }
    }

    // If the user is still typing the table name, don't leave expect stuck for
    // candidate ranking — clause already tells us what we need.
    let _ = expect;

    // Typing a qualifier without finishing the table token: `FROM u` — scope
    // may already contain partial "u" if it was scanned. Strip trailing partial
    // table that equals the active prefix when still expecting a table name.
    if qualifier.is_none()
        && !prefix.is_empty()
        && matches!(
            clause,
            Clause::From | Clause::Join | Clause::Into | Clause::UpdateTable | Clause::TableName
        )
    {
        if let Some(last) = scope.last() {
            if last.alias.is_none() && last.table.eq_ignore_ascii_case(&prefix) {
                scope.pop();
            }
        }
    }

    CompletionContext {
        statement,
        clause,
        scope,
        prefix,
        qualifier,
    }
}

fn apply_clause_keyword(
    upper: &str,
    statement: &mut StatementKind,
    clause: &mut Clause,
    expect: &mut Expect,
) {
    match upper {
        "FROM" => {
            *clause = Clause::From;
            *expect = Expect::Table;
        }
        "JOIN" => {
            *clause = Clause::Join;
            *expect = Expect::Table;
        }
        "ON" => {
            *clause = Clause::On;
            *expect = Expect::None;
        }
        "WHERE" => {
            *clause = Clause::Where;
            *expect = Expect::None;
        }
        "SET" => {
            *clause = Clause::Set;
            *expect = Expect::None;
        }
        "HAVING" => {
            *clause = Clause::Having;
            *expect = Expect::None;
        }
        "GROUP" => {
            *clause = Clause::GroupBy;
            *expect = Expect::By;
        }
        "ORDER" => {
            *clause = Clause::OrderBy;
            *expect = Expect::By;
        }
        "SELECT" => {
            *statement = StatementKind::Select;
            *clause = Clause::SelectList;
            *expect = Expect::None;
        }
        _ => {}
    }
}

fn last_statement(line: &str) -> &str {
    line.rsplit(';').next().unwrap_or(line).trim_start()
}

fn significant_tokens(sql: &str) -> Vec<&str> {
    split_tokens(sql)
        .into_iter()
        .filter(|t| {
            let t = t.trim();
            !t.is_empty()
                && !t.chars().all(|c| c.is_ascii_whitespace())
                && !t.starts_with("--")
                && !t.starts_with("/*")
        })
        .collect()
}

fn is_ident(tok: &str) -> bool {
    let mut chars = tok.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn is_clause_starter(upper: &str) -> bool {
    matches!(
        upper,
        "SELECT"
            | "FROM"
            | "WHERE"
            | "JOIN"
            | "ON"
            | "GROUP"
            | "ORDER"
            | "HAVING"
            | "LIMIT"
            | "OFFSET"
            | "SET"
            | "VALUES"
            | "UNION"
            | "EXCEPT"
            | "INTERSECT"
            | "RETURNING"
    )
}

/// Split off the token being typed. Returns (complete_sql, prefix, qualifier).
fn split_active_token(line: &str) -> (String, String, Option<String>) {
    let trimmed_end = line.trim_end();
    if trimmed_end.is_empty() {
        return (String::new(), String::new(), None);
    }

    // Ended with whitespace → nothing being typed.
    if line.ends_with(|c: char| c.is_ascii_whitespace()) {
        return (trimmed_end.to_string(), String::new(), None);
    }

    // Trailing `table.` / `alias.`
    if trimmed_end.ends_with('.') {
        let before_dot = &trimmed_end[..trimmed_end.len() - 1];
        let qual = last_ident(before_dot).unwrap_or("");
        let complete = before_dot
            .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
            .to_string();
        return (complete, String::new(), Some(qual.to_string()));
    }

    // `table.col` mid-token
    if let Some(dot) = trimmed_end.rfind('.') {
        let after = &trimmed_end[dot + 1..];
        if !after.is_empty() && after.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            let before_dot = &trimmed_end[..dot];
            let qual = last_ident(before_dot).unwrap_or("");
            let complete = before_dot
                .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
                .to_string();
            return (complete, after.to_string(), Some(qual.to_string()));
        }
    }

    // Plain identifier prefix
    let prefix = last_ident(trimmed_end).unwrap_or("").to_string();
    if prefix.is_empty() {
        return (trimmed_end.to_string(), String::new(), None);
    }
    let complete = trimmed_end[..trimmed_end.len() - prefix.len()].to_string();
    (complete, prefix, None)
}

fn last_ident(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 {
        let c = bytes[start - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            start -= 1;
        } else {
            break;
        }
    }
    if start == end {
        None
    } else {
        Some(&s[start..end])
    }
}

/// Keywords suggested for the current clause (not the full SQL keyword list).
pub fn clause_keywords(ctx: &CompletionContext) -> &'static [&'static str] {
    match ctx.clause {
        Clause::Start => match ctx.statement {
            StatementKind::Empty => &[
                "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "EXPLAIN", "WITH",
                "BEGIN", "COMMIT", "ROLLBACK", "PRAGMA",
            ],
            StatementKind::Insert => &["INTO"],
            StatementKind::Delete => &["FROM"],
            _ => &[],
        },
        Clause::SelectList => &[
            "FROM", "AS", "DISTINCT", "CASE", "WHEN", "THEN", "ELSE", "END", "AND", "OR", "NOT",
            "NULL", "TRUE", "FALSE",
        ],
        Clause::From => &[
            "AS", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "WHERE", "GROUP",
            "ORDER", "LIMIT", "UNION",
        ],
        Clause::Join => &[
            "AS", "ON", "USING", "JOIN", "LEFT", "RIGHT", "INNER", "WHERE",
        ],
        // Declaration order is the suggestion order (most likely first).
        // Columns are ranked ahead of this list by the completer.
        Clause::On | Clause::Where | Clause::Having => &[
            "NOT", "EXISTS", "AND", "OR", "IS", "IN", "LIKE", "BETWEEN", "NULL", "TRUE", "FALSE",
            "GROUP", "ORDER", "LIMIT", "HAVING", "JOIN",
        ],
        Clause::GroupBy => &["BY", "HAVING", "ORDER", "LIMIT"],
        Clause::OrderBy => &["BY", "ASC", "DESC", "LIMIT", "OFFSET"],
        Clause::Limit => &["OFFSET"],
        Clause::Set => &["WHERE", "AND", "OR", "NULL", "DEFAULT"],
        Clause::Values => &[],
        Clause::Into => &["VALUES", "SELECT", "(", ")"],
        Clause::UpdateTable => &["SET", "AS"],
        Clause::ObjectType => &[
            "TABLE",
            "VIEW",
            "INDEX",
            "IF",
            "EXISTS",
            "TEMPORARY",
            "TEMP",
        ],
        Clause::TableName => &["IF", "NOT", "EXISTS", "AS"],
        Clause::ColName => &[
            "PRIMARY KEY",
            "CONSTRAINT",
            "UNIQUE",
            "FOREIGN KEY",
            "CHECK",
        ],
        Clause::ColType => &[
            "INTEGER",
            "INT",
            "TEXT",
            "REAL",
            "BLOB",
            "NUMERIC",
            "BOOLEAN",
            "BOOL",
            "VARCHAR",
            "CHAR",
            "DATE",
            "DATETIME",
            "TIMESTAMP",
            "JSON",
            "UUID",
            "FLOAT",
            "DOUBLE",
            "DECIMAL",
            "BIGINT",
            "SMALLINT",
            "SERIAL",
        ],
        Clause::ColTypeTail => &[
            "PRECISION",
            "VARYING",
            "WITH",
            "WITHOUT",
            "TIME",
            "ZONE",
            "UNSIGNED",
            "PRIMARY KEY",
            "NOT NULL",
            "NULL",
            "UNIQUE",
            "DEFAULT",
            "REFERENCES",
        ],
        Clause::ColTail => &[
            "PRIMARY KEY",
            "NOT NULL",
            "NULL",
            "UNIQUE",
            "DEFAULT",
            "CHECK",
            "REFERENCES",
            "CONSTRAINT",
            "FOREIGN KEY",
            "COLLATE",
            "AUTOINCREMENT",
            "GENERATED",
            "ON",
            "DELETE",
            "UPDATE",
            "CASCADE",
            "SET",
        ],
        Clause::ColKey => &["KEY"],
        Clause::ColNot => &["NULL"],
        Clause::ColRef => &["ON"],
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColPhase {
    Name,
    Type,
    TypeCont,
    Tail,
    AfterPrimary,
    AfterNot,
    AfterRef,
}

fn apply_col_phase(upper: &str, phase: &mut ColPhase, clause: &mut Clause) {
    match *phase {
        ColPhase::Name => {
            if is_constraint_start(upper) {
                enter_constraint(upper, phase, clause);
            } else {
                *phase = ColPhase::Type;
                *clause = Clause::ColType;
            }
        }
        ColPhase::Type | ColPhase::TypeCont => {
            if is_constraint_word(upper) {
                enter_constraint(upper, phase, clause);
            } else if type_expects_more(upper) {
                *phase = ColPhase::TypeCont;
                *clause = Clause::ColTypeTail;
            } else {
                *phase = ColPhase::Tail;
                *clause = Clause::ColTail;
            }
        }
        ColPhase::Tail | ColPhase::AfterPrimary | ColPhase::AfterNot | ColPhase::AfterRef => {
            if type_expects_more(upper) && !is_constraint_word(upper) {
                *phase = ColPhase::TypeCont;
                *clause = Clause::ColTypeTail;
            } else {
                enter_constraint(upper, phase, clause);
            }
        }
    }
}

fn enter_constraint(upper: &str, phase: &mut ColPhase, clause: &mut Clause) {
    match upper {
        "PRIMARY" | "FOREIGN" => {
            *phase = ColPhase::AfterPrimary;
            *clause = Clause::ColKey;
        }
        "NOT" => {
            *phase = ColPhase::AfterNot;
            *clause = Clause::ColNot;
        }
        "REFERENCES" => {
            *phase = ColPhase::AfterRef;
            *clause = Clause::ColRef;
        }
        _ => {
            *phase = ColPhase::Tail;
            *clause = Clause::ColTail;
        }
    }
}

fn is_constraint_start(upper: &str) -> bool {
    matches!(
        upper,
        "CONSTRAINT" | "PRIMARY" | "UNIQUE" | "FOREIGN" | "CHECK"
    )
}

fn is_constraint_word(upper: &str) -> bool {
    is_constraint_start(upper)
        || matches!(
            upper,
            "NOT"
                | "NULL"
                | "KEY"
                | "DEFAULT"
                | "REFERENCES"
                | "COLLATE"
                | "AUTOINCREMENT"
                | "AUTO_INCREMENT"
                | "GENERATED"
                | "ALWAYS"
                | "IDENTITY"
                | "STORED"
                | "VIRTUAL"
                | "ON"
                | "DELETE"
                | "UPDATE"
                | "CASCADE"
                | "RESTRICT"
                | "SET"
                | "NO"
                | "ACTION"
        )
}

fn type_expects_more(upper: &str) -> bool {
    matches!(
        upper,
        "DOUBLE"
            | "FLOAT"
            | "CHARACTER"
            | "CHAR"
            | "NCHAR"
            | "NATIONAL"
            | "BIT"
            | "TIMESTAMP"
            | "LONG"
            | "WITH"
            | "WITHOUT"
            | "TIME"
            | "LARGE"
    )
}

fn is_word(token: &str) -> bool {
    is_ident(token) || matches!(token.as_bytes().first(), Some(b'"' | b'`' | b'['))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_where_scopes_table() {
        let ctx = analyze("SELECT * FROM users u WHERE ");
        assert_eq!(ctx.clause, Clause::Where);
        assert_eq!(ctx.scope.len(), 1);
        assert_eq!(ctx.scope[0].table, "users");
        assert_eq!(ctx.scope[0].alias.as_deref(), Some("u"));
        assert!(ctx.prefix.is_empty());
    }

    #[test]
    fn qualifier_dot_prefix() {
        let ctx = analyze("SELECT u.na");
        assert_eq!(ctx.qualifier.as_deref(), Some("u"));
        assert_eq!(ctx.prefix, "na");
        assert_eq!(ctx.clause, Clause::SelectList);
    }

    #[test]
    fn from_partial_table_not_in_scope() {
        let ctx = analyze("SELECT * FROM use");
        assert_eq!(ctx.clause, Clause::From);
        assert!(ctx.scope.is_empty());
        assert_eq!(ctx.prefix, "use");
    }

    #[test]
    fn join_adds_second_table() {
        let ctx = analyze("SELECT * FROM a JOIN b ON ");
        assert_eq!(ctx.clause, Clause::On);
        assert_eq!(ctx.scope.len(), 2);
        assert_eq!(ctx.scope[0].table, "a");
        assert_eq!(ctx.scope[1].table, "b");
    }

    #[test]
    fn update_set_clause() {
        let ctx = analyze("UPDATE users SET ");
        assert_eq!(ctx.statement, StatementKind::Update);
        assert_eq!(ctx.clause, Clause::Set);
        assert_eq!(ctx.scope[0].table, "users");
    }

    #[test]
    fn create_table_column_type_then_constraint() {
        let ctx = analyze("CREATE TABLE aboba(id ");
        assert_eq!(ctx.clause, Clause::ColType);
        assert!(ctx.prefix.is_empty());

        let ctx = analyze("CREATE TABLE aboba(id TEXT PRIMAR");
        assert_eq!(ctx.clause, Clause::ColTail);
        assert_eq!(ctx.prefix, "PRIMAR");

        let ctx = analyze("CREATE TABLE aboba(id TEXT PRIMARY ");
        assert_eq!(ctx.clause, Clause::ColKey);
        assert!(ctx.prefix.is_empty());
    }

    #[test]
    fn create_table_if_not_exists_still_reaches_columns() {
        let ctx = analyze("CREATE TABLE IF NOT EXISTS aboba(id TEXT ");
        assert_eq!(ctx.clause, Clause::ColTail);
        assert!(ctx.prefix.is_empty());
    }
}

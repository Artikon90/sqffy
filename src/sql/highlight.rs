use super::tokens::{is_keyword, is_type_modifier, is_type_name, split_tokens};
use crate::ui::{color_enabled, paint_cyan, paint_dim, paint_green, paint_yellow};
use owo_colors::OwoColorize;

/// Colorize SQL for terminal output / rustyline highlighter.
///
/// Keywords stay cyan. Table names are yellow, column names blue, type names magenta.
pub fn colorize(sql: &str) -> String {
    if !color_enabled() {
        return sql.to_string();
    }

    let tokens = split_tokens(sql);
    let roles = classify(&tokens);
    let mut out = String::with_capacity(sql.len().saturating_mul(2));
    for (token, role) in tokens.iter().zip(roles) {
        out.push_str(&paint(token, role));
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Keyword,
    Type,
    Table,
    Column,
    String,
    Number,
    Comment,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Top,
    WantRel,
    AfterRel,
    PostRel,
    ColName,
    ColType,
    ColTail,
    TypeArgs,
    NameList,
    InsertCols,
    SelectList,
    Expr,
    WantAlias,
    WantColAlias,
    WantCast,
    AfterAdd,
    AlterDrop,
    Rename,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stmt {
    #[default]
    Other,
    Ddl,
    Alter,
    Select,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Obj {
    #[default]
    None,
    Table,
    View,
    Index,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RelKind {
    #[default]
    None,
    /// Relation introduced by CREATE / DROP TABLE|VIEW|INDEX.
    Ddl,
    /// FROM / JOIN / INTO / UPDATE.
    Query,
    /// CTE name after WITH.
    Cte,
    /// Table after REFERENCES.
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    /// Restore `phase` captured when the paren opened.
    Restore,
    /// Type arguments: `VARCHAR(255)`, `Nullable(String)`.
    TypeArgs {
        after: Phase,
    },
    /// Column name list: `PRIMARY KEY (id)`, index columns, `USING (id)`.
    NameList {
        after: Phase,
    },
    InsertCols,
    CreateCols,
}

struct Frame {
    kind: FrameKind,
    phase: Phase,
    from_list: bool,
    cte: bool,
    rel: RelKind,
    obj: Obj,
    stmt: Stmt,
    expect_type: bool,
    /// This paren is the argument list of CAST/CONVERT.
    cast: bool,
    /// This paren is the argument list of EXTRACT (`EXTRACT(YEAR FROM …)`).
    extract: bool,
}

struct Hl {
    phase: Phase,
    stmt: Stmt,
    obj: Obj,
    rel: RelKind,
    from_list: bool,
    cte: bool,
    expect_type: bool,
    stack: Vec<Frame>,
    prev_word: String,
    resume: Phase,
    pending_cast: bool,
}

impl Default for Hl {
    fn default() -> Self {
        Self {
            phase: Phase::Top,
            stmt: Stmt::Other,
            obj: Obj::None,
            rel: RelKind::None,
            from_list: false,
            cte: false,
            expect_type: true,
            stack: Vec::new(),
            prev_word: String::new(),
            resume: Phase::Top,
            pending_cast: false,
        }
    }
}

fn classify(tokens: &[&str]) -> Vec<Role> {
    let mut hl = Hl::default();
    let mut roles = Vec::with_capacity(tokens.len());
    for i in 0..tokens.len() {
        roles.push(hl.token(tokens, i));
    }
    roles
}

impl Hl {
    fn token(&mut self, tokens: &[&str], i: usize) -> Role {
        let token = tokens[i];
        if is_ws(token) {
            return Role::Plain;
        }
        if is_comment(token) {
            return Role::Comment;
        }
        if token.starts_with('\'') {
            return Role::String;
        }
        if looks_like_number(token) {
            return Role::Number;
        }
        if token == "(" {
            return self.open_paren();
        }
        if token == ")" {
            return self.close_paren();
        }
        if token == "," {
            return self.comma();
        }
        if token == ";" {
            *self = Self::default();
            return Role::Plain;
        }
        if token == "." {
            return Role::Plain;
        }
        if token == ":" {
            return self.colon(tokens, i);
        }
        if let Some(key) = ident_key(token) {
            return self.word(tokens, i, &key, is_quoted(token));
        }
        Role::Plain
    }

    fn word(&mut self, tokens: &[&str], i: usize, key: &str, quoted: bool) -> Role {
        if self.pending_cast {
            return self.finish_cast(tokens, i, key);
        }

        let next = next_sig(tokens, i);
        let role = match self.phase {
            Phase::WantCast => self.cast_type(tokens, i, key),
            Phase::WantAlias => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                self.phase = Phase::PostRel;
                Role::Table
            }
            Phase::WantColAlias => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                self.phase = Phase::SelectList;
                Role::Column
            }
            Phase::WantRel => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                if next != Some(".") {
                    self.phase = Phase::AfterRel;
                }
                Role::Table
            }
            Phase::ColName => {
                if !quoted && is_constraint_start(key) {
                    return self.keyword(key);
                }
                self.phase = if self.expect_type {
                    Phase::ColType
                } else {
                    Phase::ColTail
                };
                Role::Column
            }
            Phase::ColType => {
                if !quoted && self.type_ends(key) {
                    return self.keyword(key);
                }
                self.after_type_word(key, next);
                Role::Type
            }
            Phase::TypeArgs => {
                if !quoted && is_keyword(key) && !is_type_name(key) && !is_type_modifier(key) {
                    return self.keyword(key);
                }
                Role::Type
            }
            Phase::NameList | Phase::InsertCols => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                Role::Column
            }
            Phase::AfterRel => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                self.phase = Phase::PostRel;
                Role::Table
            }
            Phase::AfterAdd => {
                if key == "COLUMN" && !quoted {
                    self.expect_type = true;
                    self.phase = Phase::ColName;
                    Role::Keyword
                } else if !quoted && (key == "CONSTRAINT" || is_constraint_start(key)) {
                    self.phase = Phase::ColTail;
                    Role::Keyword
                } else if quoted || !is_keyword(key) {
                    self.expect_type = true;
                    self.phase = Phase::ColType;
                    Role::Column
                } else {
                    return self.keyword(key);
                }
            }
            Phase::AlterDrop => {
                if key == "COLUMN" && !quoted {
                    self.expect_type = false;
                    self.phase = Phase::ColName;
                    Role::Keyword
                } else {
                    return self.keyword(key);
                }
            }
            Phase::Rename => {
                if key == "COLUMN" && !quoted {
                    self.expect_type = false;
                    self.phase = Phase::ColName;
                    Role::Keyword
                } else if key == "TO" && !quoted {
                    self.phase = Phase::WantRel;
                    self.rel = RelKind::Reference;
                    Role::Keyword
                } else {
                    return self.keyword(key);
                }
            }
            Phase::SelectList | Phase::Expr => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                expr_role(next)
            }
            Phase::ColTail | Phase::PostRel | Phase::Top => {
                if !quoted && is_keyword(key) {
                    return self.keyword(key);
                }
                if !quoted && (is_type_name(key) || is_type_modifier(key)) {
                    Role::Type
                } else {
                    Role::Plain
                }
            }
        };
        self.prev_word = key.to_string();
        role
    }

    fn cast_type(&mut self, tokens: &[&str], i: usize, key: &str) -> Role {
        let next = next_sig(tokens, i);
        if next == Some("(") {
            // Stay in WantCast so `(` opens type arguments.
        } else if continues_type(key, next_word(tokens, i).as_deref()) {
            // `CAST(x AS DOUBLE PRECISION)`, `AS TIMESTAMP WITH TIME ZONE`.
        } else {
            self.phase = self.resume;
        }
        self.prev_word = key.to_string();
        Role::Type
    }

    fn finish_cast(&mut self, tokens: &[&str], i: usize, key: &str) -> Role {
        let next = next_sig(tokens, i);
        self.pending_cast = false;
        self.prev_word = key.to_string();
        if next == Some("(") {
            let resume = self.phase;
            self.resume = resume;
            self.phase = Phase::WantCast;
        } else if continues_type(key, next_word(tokens, i).as_deref()) {
            self.pending_cast = true;
        }
        Role::Type
    }

    fn after_type_word(&mut self, key: &str, next: Option<&str>) {
        if continues_type(key, next_word_from(next)) || next == Some("(") {
            return;
        }
        self.phase = Phase::ColTail;
    }

    /// Keyword (or other word) that closes a type and belongs to the column tail.
    fn type_ends(&self, key: &str) -> bool {
        if is_type_name(key) || is_type_modifier(key) {
            return false;
        }
        if matches!(
            key,
            "WITH" | "WITHOUT" | "TIME" | "ZONE" | "LARGE" | "OBJECT"
        ) {
            return false;
        }
        is_keyword(key)
    }

    fn keyword(&mut self, key: &str) -> Role {
        match key {
            "SELECT" => {
                self.stmt = Stmt::Select;
                self.phase = Phase::SelectList;
                self.from_list = false;
                self.obj = Obj::None;
                self.rel = RelKind::Query;
            }
            "FROM" | "JOIN" => {
                let in_extract =
                    key == "FROM" && self.stack.last().is_some_and(|frame| frame.extract);
                if !in_extract && self.phase != Phase::TypeArgs {
                    self.phase = Phase::WantRel;
                    self.from_list = true;
                    self.rel = RelKind::Query;
                }
            }
            "WHERE" | "HAVING" | "SET" | "RETURNING" => {
                if !self.in_column_clause() {
                    self.phase = Phase::Expr;
                    self.from_list = false;
                }
            }
            "ON" => {
                if self.stmt == Stmt::Ddl && self.obj == Obj::Index {
                    self.phase = Phase::WantRel;
                    self.rel = RelKind::Ddl;
                } else if !self.in_column_clause() {
                    self.phase = Phase::Expr;
                    self.from_list = false;
                }
            }
            "UPDATE" if !self.in_column_clause() => {
                self.stmt = Stmt::Update;
                self.phase = Phase::WantRel;
                self.rel = RelKind::Query;
                self.from_list = false;
                self.obj = Obj::None;
            }
            "INSERT" if !self.in_column_clause() => {
                self.stmt = Stmt::Insert;
                self.phase = Phase::Top;
                self.obj = Obj::None;
                self.from_list = false;
            }
            "INTO" => {
                if self.stmt == Stmt::Other {
                    self.stmt = Stmt::Insert;
                }
                self.phase = Phase::WantRel;
                self.rel = RelKind::Query;
            }
            "DELETE" if !self.in_column_clause() => {
                self.stmt = Stmt::Delete;
                self.phase = Phase::Top;
                self.from_list = false;
            }
            "CREATE" if self.at_boundary() => {
                self.stmt = Stmt::Ddl;
                self.phase = Phase::Top;
                self.obj = Obj::None;
                self.rel = RelKind::None;
                self.from_list = false;
            }
            "ALTER" if self.at_boundary() => {
                self.stmt = Stmt::Alter;
                self.phase = Phase::Top;
                self.obj = Obj::None;
                self.rel = RelKind::None;
                self.from_list = false;
            }
            "DROP" if self.at_boundary() => {
                self.stmt = Stmt::Ddl;
                self.phase = Phase::Top;
                self.obj = Obj::None;
                self.rel = RelKind::None;
                self.from_list = false;
            }
            "DROP" if self.stmt == Stmt::Alter => {
                self.phase = Phase::AlterDrop;
            }
            "TABLE" if self.ddl_object_slot() => {
                self.obj = Obj::Table;
                self.phase = Phase::WantRel;
                self.rel = RelKind::Ddl;
            }
            "VIEW" if self.ddl_object_slot() => {
                self.obj = Obj::View;
                self.phase = Phase::WantRel;
                self.rel = RelKind::Ddl;
            }
            "INDEX" if self.ddl_object_slot() => {
                self.obj = Obj::Index;
                self.phase = Phase::WantRel;
                self.rel = RelKind::Ddl;
            }
            "VALUES" => {
                if !self.in_column_clause() {
                    self.phase = Phase::Top;
                    self.from_list = false;
                }
            }
            "AS" => self.on_as(),
            "ADD" if self.stmt == Stmt::Alter => {
                self.phase = Phase::AfterAdd;
            }
            "COLUMN" if matches!(self.stmt, Stmt::Alter | Stmt::Ddl) => {
                self.expect_type = self.prev_word != "DROP";
                self.phase = Phase::ColName;
            }
            "RENAME" if self.stmt == Stmt::Alter => {
                self.phase = Phase::Rename;
            }
            "TO" if self.stmt == Stmt::Alter
                && matches!(self.phase, Phase::ColTail | Phase::ColName) =>
            {
                self.expect_type = false;
                self.phase = Phase::ColName;
            }
            "TO" if self.stmt == Stmt::Alter => {
                self.phase = Phase::WantRel;
                self.rel = RelKind::Reference;
            }
            "REFERENCES" => {
                self.phase = Phase::WantRel;
                self.rel = RelKind::Reference;
            }
            "WITH" if self.at_boundary() => {
                self.cte = true;
                self.phase = Phase::WantRel;
                self.rel = RelKind::Cte;
            }
            "USING" => {
                if !self.in_column_clause() {
                    self.phase = Phase::PostRel;
                }
            }
            "GROUP" | "ORDER" | "LIMIT" | "OFFSET" | "UNION" | "EXCEPT" | "INTERSECT" => {
                if !self.in_column_clause() {
                    self.from_list = false;
                    self.phase = Phase::Expr;
                }
            }
            "CONSTRAINT" | "PRIMARY" | "UNIQUE" | "FOREIGN" | "CHECK" | "DEFAULT"
                if matches!(
                    self.phase,
                    Phase::ColName | Phase::ColType | Phase::AfterAdd
                ) =>
            {
                self.phase = Phase::ColTail;
            }
            "NOT" | "NULL" if self.phase == Phase::ColType => {
                self.phase = Phase::ColTail;
            }
            _ => {}
        }
        self.prev_word = key.to_string();
        Role::Keyword
    }

    fn on_as(&mut self) {
        if self.stack.last().is_some_and(|frame| frame.cast) {
            self.resume = self.phase;
            self.phase = Phase::WantCast;
            return;
        }
        match self.phase {
            Phase::AfterRel | Phase::PostRel if matches!(self.rel, RelKind::Ddl | RelKind::Cte) => {
                self.phase = Phase::Top;
            }
            Phase::AfterRel | Phase::PostRel => {
                self.phase = Phase::WantAlias;
            }
            Phase::SelectList | Phase::Expr => {
                self.phase = Phase::WantColAlias;
            }
            _ => {}
        }
    }

    fn open_paren(&mut self) -> Role {
        let prev = self.prev_word.clone();
        match self.phase {
            Phase::ColType | Phase::WantCast => {
                let after = if self.phase == Phase::WantCast {
                    self.resume
                } else {
                    Phase::ColTail
                };
                self.push(FrameKind::TypeArgs { after });
                self.phase = Phase::TypeArgs;
            }
            Phase::AfterRel | Phase::PostRel
                if self.rel == RelKind::Ddl && self.obj == Obj::Table =>
            {
                self.expect_type = true;
                self.push(FrameKind::CreateCols);
                self.phase = Phase::ColName;
            }
            Phase::AfterRel | Phase::PostRel
                if self.rel == RelKind::Ddl && self.obj == Obj::Index =>
            {
                self.push(FrameKind::NameList {
                    after: Phase::PostRel,
                });
                self.phase = Phase::NameList;
            }
            Phase::AfterRel | Phase::PostRel if self.rel == RelKind::Reference => {
                self.push(FrameKind::NameList {
                    after: Phase::ColTail,
                });
                self.phase = Phase::NameList;
            }
            Phase::AfterRel | Phase::PostRel
                if self.stmt == Stmt::Insert && self.rel == RelKind::Query =>
            {
                self.push(FrameKind::InsertCols);
                self.phase = Phase::InsertCols;
            }
            Phase::WantRel => {
                // Subquery: `(SELECT ...)`. Afterwards an alias is allowed.
                self.phase = Phase::AfterRel;
                self.push(FrameKind::Restore);
                self.phase = Phase::Top;
            }
            _ if opens_name_list(&prev) => {
                let after = if self.in_column_clause() {
                    Phase::ColTail
                } else {
                    Phase::PostRel
                };
                self.push(FrameKind::NameList { after });
                self.phase = Phase::NameList;
            }
            Phase::ColTail | Phase::ColName => {
                self.push(FrameKind::Restore);
                self.phase = Phase::Expr;
            }
            _ => {
                self.push(FrameKind::Restore);
            }
        }
        if let Some(frame) = self.stack.last_mut() {
            if matches!(
                prev.as_str(),
                "CAST" | "CONVERT" | "TRY_CAST" | "SAFE_CAST" | "TRY_CONVERT"
            ) {
                frame.cast = true;
            }
            if prev == "EXTRACT" {
                frame.extract = true;
            }
        }
        Role::Plain
    }

    fn close_paren(&mut self) -> Role {
        self.pop_frame();
        Role::Plain
    }

    fn comma(&mut self) -> Role {
        match self.phase {
            Phase::TypeArgs | Phase::NameList | Phase::InsertCols => {}
            Phase::ColName | Phase::ColType | Phase::ColTail => {
                self.phase = Phase::ColName;
                self.expect_type = true;
            }
            Phase::AfterRel | Phase::PostRel | Phase::WantAlias if self.inside_create() => {
                self.phase = Phase::ColName;
                self.expect_type = true;
            }
            Phase::AfterRel | Phase::PostRel | Phase::WantRel | Phase::WantAlias
                if self.from_list =>
            {
                self.phase = Phase::WantRel;
                self.rel = RelKind::Query;
            }
            Phase::Top | Phase::PostRel | Phase::AfterRel if self.cte && self.stack.is_empty() => {
                self.phase = Phase::WantRel;
                self.rel = RelKind::Cte;
            }
            _ => {}
        }
        Role::Plain
    }

    fn colon(&mut self, tokens: &[&str], i: usize) -> Role {
        if next_sig(tokens, i) == Some(":") {
            self.pending_cast = false;
            self.prev_word = ":".to_string();
        } else if self.prev_word == ":" {
            self.pending_cast = true;
        }
        Role::Plain
    }

    fn push(&mut self, kind: FrameKind) {
        self.stack.push(Frame {
            kind,
            phase: self.phase,
            from_list: self.from_list,
            cte: self.cte,
            rel: self.rel,
            obj: self.obj,
            stmt: self.stmt,
            expect_type: self.expect_type,
            cast: false,
            extract: false,
        });
    }

    fn pop_frame(&mut self) {
        let Some(frame) = self.stack.pop() else {
            return;
        };
        self.from_list = frame.from_list;
        self.cte = frame.cte;
        self.rel = frame.rel;
        self.obj = frame.obj;
        self.stmt = frame.stmt;
        self.expect_type = frame.expect_type;
        self.phase = match frame.kind {
            FrameKind::Restore => frame.phase,
            FrameKind::TypeArgs { after } => after,
            FrameKind::NameList { after } => after,
            FrameKind::InsertCols => Phase::Top,
            FrameKind::CreateCols => Phase::PostRel,
        };
    }

    fn at_boundary(&self) -> bool {
        self.stack.is_empty() && matches!(self.phase, Phase::Top | Phase::PostRel)
    }

    fn ddl_object_slot(&self) -> bool {
        matches!(self.stmt, Stmt::Ddl | Stmt::Alter)
            && matches!(self.phase, Phase::Top | Phase::WantRel | Phase::PostRel)
    }

    fn in_column_clause(&self) -> bool {
        matches!(
            self.phase,
            Phase::ColName
                | Phase::ColType
                | Phase::ColTail
                | Phase::TypeArgs
                | Phase::NameList
                | Phase::AfterAdd
                | Phase::AlterDrop
                | Phase::Rename
        )
    }

    fn inside_create(&self) -> bool {
        self.stack.iter().any(|f| f.kind == FrameKind::CreateCols)
    }
}

fn expr_role(next: Option<&str>) -> Role {
    if next == Some("(") {
        Role::Keyword
    } else if next == Some(".") {
        Role::Table
    } else {
        Role::Column
    }
}

fn is_constraint_start(key: &str) -> bool {
    matches!(
        key,
        "CONSTRAINT" | "PRIMARY" | "UNIQUE" | "FOREIGN" | "CHECK"
    )
}

fn opens_name_list(prev: &str) -> bool {
    matches!(prev, "KEY" | "UNIQUE" | "USING" | "INDEX")
}

fn continues_type(current: &str, next: Option<&str>) -> bool {
    let Some(next) = next else {
        return false;
    };
    matches!(
        (current, next),
        ("DOUBLE", "PRECISION")
            | ("FLOAT", "PRECISION")
            | ("CHARACTER", "VARYING")
            | ("CHAR", "VARYING")
            | ("NCHAR", "VARYING")
            | ("NATIONAL", "CHARACTER")
            | ("NATIONAL", "CHAR")
            | ("NATIONAL", "VARCHAR")
            | ("BIT", "VARYING")
            | ("TIMESTAMP", "WITH")
            | ("TIMESTAMP", "WITHOUT")
            | ("TIME", "WITH")
            | ("TIME", "WITHOUT")
            | ("WITH", "TIME")
            | ("WITHOUT", "TIME")
            | ("TIME", "ZONE")
            | ("CHARACTER", "LARGE")
            | ("LARGE", "OBJECT")
            | ("LONG", "RAW")
            | ("LONG", "VARCHAR")
            | ("LONG", "TEXT")
            | ("LONG", "BLOB")
            | (_, "UNSIGNED")
            | (_, "SIGNED")
            | (_, "ZEROFILL")
    )
}

fn next_word_from(next: Option<&str>) -> Option<&str> {
    next.filter(|t| ident_key(t).is_some())
}

fn paint(token: &str, role: Role) -> String {
    match role {
        Role::Keyword => paint_cyan(token),
        Role::Type => token.bright_magenta().to_string(),
        Role::Table => token.bright_yellow().bold().to_string(),
        Role::Column => token.bright_blue().to_string(),
        Role::String => paint_green(token),
        Role::Number => paint_yellow(token),
        Role::Comment => paint_dim(token),
        Role::Plain => {
            if is_quoted(token) {
                paint_yellow(token)
            } else {
                token.to_string()
            }
        }
    }
}

fn is_ws(token: &str) -> bool {
    token.chars().all(|c| c.is_ascii_whitespace())
}

fn is_comment(token: &str) -> bool {
    token.starts_with("--") || token.starts_with("/*")
}

fn is_quoted(token: &str) -> bool {
    matches!(token.as_bytes().first(), Some(b'"' | b'`' | b'['))
}

fn ident_key(token: &str) -> Option<String> {
    if let Some(inner) = quoted_inner(token) {
        return Some(inner.to_ascii_uppercase());
    }
    if is_bare_ident(token) {
        return Some(token.to_ascii_uppercase());
    }
    None
}

fn quoted_inner(token: &str) -> Option<&str> {
    let bytes = token.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let close = match bytes[0] {
        b'"' => b'"',
        b'`' => b'`',
        b'[' => b']',
        _ => return None,
    };
    if *bytes.last()? == close {
        Some(&token[1..token.len() - 1])
    } else {
        Some(&token[1..])
    }
}

fn is_bare_ident(token: &str) -> bool {
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn next_sig<'a>(tokens: &[&'a str], i: usize) -> Option<&'a str> {
    tokens[i + 1..]
        .iter()
        .copied()
        .find(|t| !is_ws(t) && !is_comment(t))
}

fn next_word(tokens: &[&str], i: usize) -> Option<String> {
    next_sig(tokens, i).and_then(ident_key)
}

fn looks_like_number(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let mut chars = token.chars();
    let first = chars.next().unwrap();
    if first == '.' {
        return token.len() > 1 && token[1..].bytes().all(|b| b.is_ascii_digit());
    }
    if !first.is_ascii_digit() {
        return false;
    }
    token.bytes().all(|b| {
        b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E' || b == b'+' || b == b'-'
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marked(sql: &str) -> String {
        let tokens = split_tokens(sql);
        let roles = classify(&tokens);
        let mut out = String::new();
        for (token, role) in tokens.iter().zip(roles) {
            if is_ws(token) || is_punct(token) {
                continue;
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(token);
            out.push('=');
            out.push_str(role_tag(role));
        }
        out
    }

    fn is_punct(token: &str) -> bool {
        token.chars().count() == 1 && !token.chars().next().unwrap().is_ascii_alphanumeric()
    }

    fn role_tag(role: Role) -> &'static str {
        match role {
            Role::Keyword => "kw",
            Role::Type => "ty",
            Role::Table => "tbl",
            Role::Column => "col",
            Role::String => "str",
            Role::Number => "num",
            Role::Comment => "com",
            Role::Plain => "plain",
        }
    }

    #[test]
    fn create_table_names_types_and_columns() {
        assert_eq!(
            marked(
                "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT, adress TEXT);"
            ),
            "CREATE=kw TABLE=kw IF=kw NOT=kw EXISTS=kw users=tbl id=col INTEGER=ty PRIMARY=kw KEY=kw name=col TEXT=ty adress=col TEXT=ty"
        );
    }

    #[test]
    fn create_table_types_with_args_and_quotes() {
        assert_eq!(
            marked(
                "CREATE TABLE \"users\" (\n  \"id\" VARCHAR(255) NOT NULL,\n  price DECIMAL(10, 2),\n  ts TIMESTAMP WITH TIME ZONE,\n  qty DOUBLE PRECISION,\n  tag CHARACTER VARYING(50),\n  PRIMARY KEY (\"id\")\n);"
            ),
            "CREATE=kw TABLE=kw \"users\"=tbl \"id\"=col VARCHAR=ty 255=num NOT=kw NULL=kw price=col DECIMAL=ty 10=num 2=num ts=col TIMESTAMP=ty WITH=ty TIME=ty ZONE=ty qty=col DOUBLE=ty PRECISION=ty tag=col CHARACTER=ty VARYING=ty 50=num PRIMARY=kw KEY=kw \"id\"=col"
        );
    }

    #[test]
    fn select_from_where_join() {
        assert_eq!(
            marked(
                "SELECT u.id, COUNT(*) FROM public.users u JOIN orders o ON u.id = o.user_id WHERE u.name = 'ada'"
            ),
            "SELECT=kw u=tbl id=col COUNT=kw FROM=kw public=tbl users=tbl u=tbl JOIN=kw orders=tbl o=tbl ON=kw u=tbl id=col o=tbl user_id=col WHERE=kw u=tbl name=col 'ada'=str"
        );
    }

    #[test]
    fn insert_update_delete() {
        assert_eq!(
            marked("INSERT INTO users (id, name) VALUES (1, 'a')"),
            "INSERT=kw INTO=kw users=tbl id=col name=col VALUES=kw 1=num 'a'=str"
        );
        assert_eq!(
            marked("UPDATE users SET name = 'x' WHERE id = 1"),
            "UPDATE=kw users=tbl SET=kw name=col 'x'=str WHERE=kw id=col 1=num"
        );
        assert_eq!(
            marked("DELETE FROM users WHERE id = 1"),
            "DELETE=kw FROM=kw users=tbl WHERE=kw id=col 1=num"
        );
    }

    #[test]
    fn cast_and_colon_cast() {
        assert_eq!(
            marked("SELECT CAST(id AS INTEGER), id::text FROM t"),
            "SELECT=kw CAST=kw id=col AS=kw INTEGER=ty id=col text=ty FROM=kw t=tbl"
        );
        assert_eq!(
            marked("SELECT CAST(id AS VARCHAR(20)) FROM t"),
            "SELECT=kw CAST=kw id=col AS=kw VARCHAR=ty 20=num FROM=kw t=tbl"
        );
    }

    #[test]
    fn clickhouse_nested_type_and_index() {
        assert_eq!(
            marked("CREATE TABLE t (id Nullable(String), tags Array(UInt32))"),
            "CREATE=kw TABLE=kw t=tbl id=col Nullable=ty String=ty tags=col Array=ty UInt32=ty"
        );
        assert_eq!(
            marked("CREATE INDEX idx ON users (id, name)"),
            "CREATE=kw INDEX=kw idx=tbl ON=kw users=tbl id=col name=col"
        );
    }

    #[test]
    fn alter_add_and_drop_column() {
        assert_eq!(
            marked("ALTER TABLE users ADD COLUMN email TEXT"),
            "ALTER=kw TABLE=kw users=tbl ADD=kw COLUMN=kw email=col TEXT=ty"
        );
        assert_eq!(
            marked("ALTER TABLE users DROP COLUMN email"),
            "ALTER=kw TABLE=kw users=tbl DROP=kw COLUMN=kw email=col"
        );
    }

    #[test]
    fn references_and_on_delete_stay_in_the_column_list() {
        assert_eq!(
            marked("CREATE TABLE t (id INT REFERENCES other(id) ON DELETE CASCADE, name TEXT)"),
            "CREATE=kw TABLE=kw t=tbl id=col INT=ty REFERENCES=kw other=tbl id=col ON=kw DELETE=kw CASCADE=kw name=col TEXT=ty"
        );
    }

    #[test]
    fn create_or_replace_view_and_nested_cast() {
        assert_eq!(
            marked("CREATE OR REPLACE VIEW v AS SELECT id FROM t"),
            "CREATE=kw OR=kw REPLACE=kw VIEW=kw v=tbl AS=kw SELECT=kw id=col FROM=kw t=tbl"
        );
        assert_eq!(
            marked("SELECT CAST(lower(id) AS INTEGER) AS name FROM t"),
            "SELECT=kw CAST=kw lower=kw id=col AS=kw INTEGER=ty AS=kw name=col FROM=kw t=tbl"
        );
    }

    #[test]
    fn comments_stay_dim_and_second_statement_resets() {
        assert_eq!(
            marked("SELECT id FROM t; -- tail\nCREATE TABLE u (n INT)"),
            "SELECT=kw id=col FROM=kw t=tbl -- tail=com CREATE=kw TABLE=kw u=tbl n=col INT=ty"
        );
    }
}

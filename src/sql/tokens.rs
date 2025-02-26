/// Keywords used for highlighting and autocomplete.
pub const KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL",
    "CROSS", "ON", "USING", "GROUP", "BY", "ORDER", "ASC", "DESC", "LIMIT", "OFFSET",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "VIEW",
    "INDEX", "DROP", "ALTER", "ADD", "COLUMN", "AND", "OR", "NOT", "NULL", "IS",
    "IN", "LIKE", "BETWEEN", "EXISTS", "AS", "DISTINCT", "HAVING", "UNION", "ALL",
    "EXCEPT", "INTERSECT", "CASE", "WHEN", "THEN", "ELSE", "END", "WITH", "RECURSIVE",
    "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "CHECK", "DEFAULT",
    "CONSTRAINT", "CASCADE", "REPLACE", "IF", "BEGIN", "COMMIT", "ROLLBACK",
    "PRAGMA", "ATTACH", "DETACH", "ANALYZE", "EXPLAIN", "VACUUM", "TRUE", "FALSE",
];

pub fn is_keyword(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    KEYWORDS.iter().any(|kw| *kw == upper)
}

/// Split SQL into whitespace / identifiers / strings / punctuation tokens.
pub fn split_tokens(sql: &str) -> Vec<&str> {
    let chars: Vec<char> = sql.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_whitespace() {
            let begin = i;
            while i < chars.len() && chars[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(slice(sql, &chars, begin, i));
            continue;
        }
        if chars[i].is_ascii_alphanumeric() || chars[i] == '_' {
            let begin = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(slice(sql, &chars, begin, i));
            continue;
        }
        if chars[i] == '\'' {
            let begin = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    i += 1;
                    if i < chars.len() && chars[i] == '\'' {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            tokens.push(slice(sql, &chars, begin, i));
            continue;
        }
        if chars[i] == '"' || chars[i] == '`' || chars[i] == '[' {
            let begin = i;
            let closer = match chars[i] {
                '"' => '"',
                '`' => '`',
                _ => ']',
            };
            i += 1;
            while i < chars.len() && chars[i] != closer {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            tokens.push(slice(sql, &chars, begin, i));
            continue;
        }
        if chars[i] == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            let begin = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(slice(sql, &chars, begin, i));
            continue;
        }
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let begin = i;
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            tokens.push(slice(sql, &chars, begin, i));
            continue;
        }
        tokens.push(slice(sql, &chars, i, i + 1));
        i += 1;
    }
    tokens
}

fn slice<'a>(sql: &'a str, chars: &[char], start: usize, end: usize) -> &'a str {
    let byte_start: usize = chars[..start].iter().map(|c| c.len_utf8()).sum();
    let byte_end: usize = chars[..end].iter().map(|c| c.len_utf8()).sum();
    &sql[byte_start..byte_end]
}

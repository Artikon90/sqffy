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
    "TEMPORARY", "TEMP", "UNLOGGED", "NATURAL", "MATERIALIZED", "WITHOUT",
    "GENERATED", "ALWAYS", "IDENTITY", "STORED", "VIRTUAL", "AUTOINCREMENT",
    "AUTO_INCREMENT", "COLLATE", "ENGINE",
];

pub fn is_keyword(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    KEYWORDS.iter().any(|kw| *kw == upper)
}

/// Builtin SQL type names (not keywords — highlighted separately).
const TYPES: &[&str] = &[
    "AGGREGATEFUNCTION", "ARRAY", "BIGINT", "BIGSERIAL", "BINARY", "BINARY_DOUBLE",
    "BINARY_FLOAT", "BIT", "BLOB", "BOOL", "BOOLEAN", "BPCHAR", "BYTEA", "CHAR",
    "CHARACTER", "CLOB", "CITEXT", "DATE", "DATE32", "DATETIME", "DATETIME2",
    "DATETIME64", "DEC", "DECIMAL", "DECIMAL128", "DECIMAL256", "DECIMAL32", "DECIMAL64",
    "DOUBLE", "ENUM", "ENUM16", "ENUM8", "FIXEDSTRING", "FLOAT", "FLOAT32", "FLOAT4",
    "FLOAT64", "FLOAT8", "GEOGRAPHY", "GEOMETRY", "HSTORE", "IMAGE", "INET", "INT",
    "INT128", "INT16", "INT2", "INT256", "INT32", "INT4", "INT64", "INT8", "INTEGER",
    "INTERVAL", "IPV4", "IPV6", "JSON", "JSONB", "LONGBLOB", "LONGTEXT", "LOWCARDINALITY",
    "MACADDR", "MACADDR8", "MAP", "MEDIUMBLOB", "MEDIUMINT", "MEDIUMTEXT", "MONEY",
    "NCHAR", "NCLOB", "NESTED", "NOTHING", "NULLABLE", "NUMBER", "NUMERIC", "NVARCHAR",
    "NVARCHAR2", "OID", "POINT", "RAW", "REAL", "REGCLASS", "SERIAL", "SERIAL2",
    "SERIAL4", "SERIAL8", "SIMPLEAGGREGATEFUNCTION", "SMALLDATETIME", "SMALLINT",
    "SMALLMONEY", "SMALLSERIAL", "STRING", "TEXT", "TIME", "TIMESTAMP", "TIMESTAMPTZ",
    "TIMESTAMP_LTZ", "TIMESTAMP_NTZ", "TIMESTAMP_TZ", "TINYBLOB", "TINYINT", "TINYTEXT",
    "TUPLE", "UINT128", "UINT16", "UINT256", "UINT32", "UINT64", "UINT8", "UUID",
    "VARBINARY", "VARCHAR", "VARCHAR2", "XML", "XMLTYPE", "YEAR",
];

pub fn is_type_name(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    TYPES.iter().any(|ty| *ty == upper)
}

/// Type names that start with `prefix` (case-insensitive). Empty prefix returns none
/// so callers can show a short common list instead of every dialect type.
pub fn type_names_matching(prefix: &str) -> Vec<&'static str> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let upper = prefix.to_ascii_uppercase();
    TYPES
        .iter()
        .copied()
        .filter(|ty| ty.starts_with(&upper))
        .collect()
}

/// Second (or later) word of a multi-word type: `DOUBLE PRECISION`, `INT UNSIGNED`.
pub fn is_type_modifier(token: &str) -> bool {
    matches!(
        token.to_ascii_uppercase().as_str(),
        "PRECISION" | "VARYING" | "UNSIGNED" | "SIGNED" | "ZEROFILL" | "ZONE" | "NATIONAL"
    )
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

/// Returns true if SQL is allowed in read-only MCP mode.
pub fn is_readonly_sql(sql: &str) -> bool {
    let trimmed = sql.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    // Strip leading comments
    let mut s = trimmed;
    loop {
        if s.starts_with("--") {
            s = s.split_once('\n').map(|(_, r)| r).unwrap_or("").trim_start();
            continue;
        }
        if s.starts_with("/*") {
            s = s
                .split_once("*/")
                .map(|(_, r)| r)
                .unwrap_or("")
                .trim_start();
            continue;
        }
        break;
    }
    let upper = s.to_ascii_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("WITH")
        || upper.starts_with("SHOW")
        || upper.starts_with("EXPLAIN")
        || upper.starts_with("DESCRIBE")
        || upper.starts_with("DESC ")
        || upper.starts_with("PRAGMA")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_select() {
        assert!(is_readonly_sql("SELECT 1"));
        assert!(is_readonly_sql("  with x as (select 1) select * from x"));
    }

    #[test]
    fn rejects_dml() {
        assert!(!is_readonly_sql("DELETE FROM t"));
        assert!(!is_readonly_sql("INSERT INTO t VALUES (1)"));
        assert!(!is_readonly_sql("DROP TABLE t"));
    }
}

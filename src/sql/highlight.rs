use super::tokens::{is_keyword, split_tokens};
use crate::ui::{color_enabled, paint_cyan, paint_dim, paint_green, paint_yellow};

/// Colorize SQL for terminal output / rustyline highlighter.
pub fn colorize(sql: &str) -> String {
    if !color_enabled() {
        return sql.to_string();
    }

    let mut out = String::with_capacity(sql.len().saturating_mul(2));
    for token in split_tokens(sql) {
        if token.chars().all(|c| c.is_ascii_whitespace()) {
            out.push_str(token);
        } else if token.starts_with("--") || token.starts_with("/*") {
            out.push_str(&paint_dim(token));
        } else if token.starts_with('\'') {
            out.push_str(&paint_green(token));
        } else if token.starts_with('"') || token.starts_with('`') || token.starts_with('[') {
            out.push_str(&paint_yellow(token));
        } else if is_keyword(token) {
            out.push_str(&paint_cyan(token));
        } else if looks_like_number(token) {
            out.push_str(&paint_yellow(token));
        } else {
            out.push_str(token);
        }
    }
    out
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
    token
        .bytes()
        .all(|b| b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E' || b == b'+' || b == b'-')
}

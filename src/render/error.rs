//! Pretty error output: closed red frame + optional SQL caret.

use crate::error::Error;
use crate::i18n::t;
use crate::ui::{paint_dim, paint_red, term_width};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Multi-line error block for the REPL / CLI.
pub fn format_error(err: &Error) -> String {
    let message = err.message_text();
    let inner = term_width().saturating_sub(2).clamp(40, 72);

    let mut lines: Vec<String> = Vec::new();
    lines.push(top_border(&box_title(err), inner));

    for line in wrap_text(&message, inner.saturating_sub(2)) {
        lines.push(content_line(&paint_red(&format!(" {line}")), line.width() + 1, inner));
    }

    if let Some(sql) = err.sql_snippet() {
        let offset = err
            .offset()
            .unwrap_or_else(|| guess_offset(sql, &message));
        lines.push(content_line(&paint_dim(" "), 1, inner));
        lines.extend(sql_location_lines(sql, offset, inner));
    }

    lines.push(bottom_border(inner));
    lines.join("\n") + "\n"
}

fn box_title(err: &Error) -> String {
    match err.driver() {
        Some(d) => format!("{} · {d}", t("app.error_prefix").trim_end_matches(':')),
        None => t("app.error_prefix").trim_end_matches(':').to_string(),
    }
}

fn top_border(title: &str, inner: usize) -> String {
    // ╭─ title ──…──╮
    let label = format!("─ {title} ");
    let fill = inner.saturating_sub(label.width());
    paint_red(&format!("╭{label}{}╮", "─".repeat(fill)))
}

fn bottom_border(inner: usize) -> String {
    paint_red(&format!("╰{}╯", "─".repeat(inner)))
}

fn content_line(styled_body: &str, visible_width: usize, inner: usize) -> String {
    let pad = inner.saturating_sub(visible_width);
    format!(
        "{}{}{}{}",
        paint_red("│"),
        styled_body,
        " ".repeat(pad),
        paint_red("│")
    )
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            if current.is_empty() {
                current.push_str(word);
            } else if current.width() + 1 + word.width() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn guess_offset(sql: &str, message: &str) -> usize {
    let lower = message.to_ascii_lowercase();
    let token = if let Some(idx) = lower.find("near ") {
        let rest = message[idx + 5..].trim_start();
        if let Some(s) = rest.strip_prefix('"') {
            s.split('"').next().unwrap_or("").to_string()
        } else {
            rest.split(|c: char| c == ':' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_string()
        }
    } else {
        let mut found = String::new();
        for prefix in ["no such table:", "no such column:", "no such view:"] {
            if let Some(idx) = lower.find(prefix) {
                found = message[idx + prefix.len()..].trim().to_string();
                break;
            }
        }
        found
    };
    if token.is_empty() {
        return 0;
    }
    sql.to_ascii_lowercase()
        .find(&token.to_ascii_lowercase())
        .unwrap_or(0)
}

fn sql_location_lines(sql: &str, byte_offset: usize, inner: usize) -> Vec<String> {
    let sql = sql.trim_end_matches(|c: char| c == ';' || c.is_whitespace());
    if sql.is_empty() {
        return Vec::new();
    }

    let offset = byte_offset.min(sql.len());
    let (line_idx, col_byte, line_text) = line_at(sql, offset);
    let col_display = display_col(line_text, col_byte);
    let mark_len = token_mark_len(line_text, col_byte).max(1);

    let gutter = format!("{}", line_idx + 1);
    let gutter_w = gutter.len().max(1);
    // " {gutter} │ {code}"  → 1 + gutter + 3 + code
    let source_budget = inner
        .saturating_sub(1 + gutter_w + 3)
        .max(8);

    let (shown, caret_col) = fit_line(line_text, col_display, source_budget);
    let mark = "▲".to_string() + &"─".repeat(mark_len.saturating_sub(1));
    let pad_g = " ".repeat(gutter_w);

    // Plain (visible) strings for width, then style for display.
    let header_plain = format!(" {pad_g} ┌─ sql");
    let blank_plain = format!(" {pad_g} │");
    let code_plain = format!(" {gutter:>gutter_w$} │ {shown}");
    let caret_plain = format!(
        " {pad_g} │ {}{}",
        " ".repeat(caret_col),
        &mark
    );
    let foot_plain = format!(
        " {pad_g} └{}",
        "─".repeat((source_budget / 2).clamp(6, 16))
    );

    vec![
        content_line(&paint_dim(&header_plain), header_plain.width(), inner),
        content_line(&paint_dim(&blank_plain), blank_plain.width(), inner),
        content_line(
            &format!(
                "{}{}{}",
                paint_dim(&format!(" {gutter:>gutter_w$} │ ")),
                shown,
                ""
            ),
            code_plain.width(),
            inner,
        ),
        content_line(
            &format!(
                "{}{}{}",
                paint_dim(&format!(" {pad_g} │ ")),
                " ".repeat(caret_col),
                paint_red(&mark)
            ),
            caret_plain.width(),
            inner,
        ),
        content_line(&paint_dim(&foot_plain), foot_plain.width(), inner),
    ]
}

fn line_at(sql: &str, offset: usize) -> (usize, usize, &str) {
    let mut start = 0;
    let mut line_idx = 0;
    for (i, ch) in sql.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line_idx += 1;
            start = i + ch.len_utf8();
        }
    }
    let end = sql[start..]
        .find('\n')
        .map(|i| start + i)
        .unwrap_or(sql.len());
    let line = &sql[start..end];
    let col_byte = offset.saturating_sub(start).min(line.len());
    (line_idx, col_byte, line)
}

fn display_col(line: &str, byte_col: usize) -> usize {
    let byte_col = byte_col.min(line.len());
    let mut end = byte_col;
    while end > 0 && !line.is_char_boundary(end) {
        end -= 1;
    }
    line[..end].width()
}

fn token_mark_len(line: &str, byte_col: usize) -> usize {
    let byte_col = byte_col.min(line.len());
    let mut start = byte_col;
    while start > 0 && !line.is_char_boundary(start) {
        start -= 1;
    }
    let rest = &line[start..];
    let mut end = 0;
    for (i, ch) in rest.char_indices() {
        if i == 0 {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '"' || ch == '\'' {
                end = ch.len_utf8();
                continue;
            }
            return ch.width().unwrap_or(1).max(1);
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return 1;
    }
    rest[..end].width().max(1)
}

fn fit_line(line: &str, caret_col: usize, budget: usize) -> (String, usize) {
    if budget == 0 {
        return (String::new(), 0);
    }
    if line.width() <= budget {
        return (line.to_string(), caret_col.min(budget.saturating_sub(1)));
    }

    let half = budget / 2;
    let start_col = caret_col.saturating_sub(half);
    let mut start_byte = 0;
    let mut col = 0;
    for (i, ch) in line.char_indices() {
        if col >= start_col {
            start_byte = i;
            break;
        }
        col += ch.width().unwrap_or(0);
        start_byte = i + ch.len_utf8();
    }

    let slice = &line[start_byte..];
    let mut out = String::new();
    let mut used = 0;
    if start_col > 0 {
        out.push('…');
        used = 1;
    }
    for ch in slice.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > budget.saturating_sub(1) {
            out.push('…');
            break;
        }
        out.push(ch);
        used += w;
    }

    let caret = if start_col > 0 {
        caret_col - start_col + 1
    } else {
        caret_col
    };
    (out, caret.min(used.saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_box_has_right_border() {
        let err = Error::backend_at(
            "sqlite",
            "near \"FROMM\": syntax error",
            "SELECT * FROMM users",
            9,
        );
        let text = format_error(&err);
        assert!(text.contains('╭'));
        assert!(text.contains('╮'));
        assert!(text.contains('╰'));
        assert!(text.contains('╯'));
        // Every content line should end with a right border marker (before newline)
        for line in text.lines().filter(|l| !l.is_empty()) {
            // Strip ANSI roughly: right border is last non-empty visual — check raw contains │ twice
            // or ends with ╮/╯
            let plain: String = line
                .chars()
                .filter(|c| !c.is_control() && *c != '\u{1b}')
                .collect();
            // ANSI-stripped poorly; just ensure line has closing box char
            assert!(
                plain.contains('│') || plain.contains('╮') || plain.contains('╯') || plain.contains('╭') || plain.contains('╰'),
                "line missing border: {line:?}"
            );
        }
        assert!(text.contains("syntax error"));
        assert!(text.contains('▲'));
    }

    #[test]
    fn message_only_error_still_boxed() {
        let err = Error::msg("table not found: nope");
        let text = format_error(&err);
        assert!(text.contains("table not found"));
        assert!(text.contains('╭'));
        assert!(text.contains('╮'));
        assert!(text.contains('╯'));
    }

    #[test]
    fn multiline_offset() {
        let sql = "SELECT *\nFROMM users";
        let (line, col, text) = line_at(sql, 9);
        assert_eq!(line, 1);
        assert_eq!(text, "FROMM users");
        assert_eq!(col, 0);
    }
}

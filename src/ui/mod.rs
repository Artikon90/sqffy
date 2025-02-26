//! Shared CLI chrome — logo, frames, menu rows.
//!
//! Classic REPL (not a TUI), with deliberate visual hierarchy:
//! brand accent, muted meta, bold actions, framed focus area.
//! Honors `NO_COLOR`.

mod frame;
mod logo;
mod progress;
mod select;
mod table;

pub use frame::{panel, panel_to_string, rule, term_width, MenuItem, Panel};
pub use logo::{print_banner, print_compact_brand};
pub use progress::{spin_frame, ProgressBar};
pub use select::{interactive_filter_select, interactive_select, FilterOption, SelectOutcome};
pub use table::{
    footnote, section_heading, sql_value_cell, title_banner, CellStyle, DataTable, StyledCell,
};

use owo_colors::OwoColorize;
use std::io::{self, Write};

pub fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

pub fn paint_cyan(text: &str) -> String {
    if color_enabled() {
        text.cyan().to_string()
    } else {
        text.to_string()
    }
}

/// Brand accent for identifiers (column names, etc.).
pub fn paint_accent(text: &str) -> String {
    if color_enabled() {
        text.cyan().bold().to_string()
    } else {
        text.to_string()
    }
}

pub fn paint_bold(text: &str) -> String {
    if color_enabled() {
        text.bold().to_string()
    } else {
        text.to_string()
    }
}

pub fn paint_dim(text: &str) -> String {
    if color_enabled() {
        text.dimmed().to_string()
    } else {
        text.to_string()
    }
}

pub fn paint_green(text: &str) -> String {
    if color_enabled() {
        text.green().to_string()
    } else {
        text.to_string()
    }
}

pub fn paint_yellow(text: &str) -> String {
    if color_enabled() {
        text.yellow().to_string()
    } else {
        text.to_string()
    }
}

pub fn paint_red(text: &str) -> String {
    if color_enabled() {
        text.red().bold().to_string()
    } else {
        text.to_string()
    }
}

/// Styled prompt: cyan caret + bold label.
pub fn prompt_line(label: &str) -> io::Result<Option<String>> {
    let prefix = format!("{} {}", paint_cyan("›"), paint_bold(label));
    print!("{prefix} ");
    io::stdout().flush()?;
    let mut line = String::new();
    let n = io::stdin().read_line(&mut line)?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

pub fn truncate_end(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    if max < 2 {
        return chars.into_iter().take(max).collect();
    }
    let mut out: String = chars.iter().take(max - 1).collect();
    out.push('…');
    out
}

pub fn truncate_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    if max < 5 {
        return truncate_end(s, max);
    }
    let head = (max - 1) / 2;
    let tail = max - 1 - head;
    let mut out: String = chars.iter().take(head).collect();
    out.push('…');
    out.extend(chars.iter().rev().take(tail).rev());
    out
}

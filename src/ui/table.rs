//! Unified tabular rendering for the whole app.
//!
//! All SQL results, `.describe`, `.help`, object lists, etc. should go through
//! [`DataTable`] so borders, colors, and truncation stay consistent.

use super::{paint_accent, paint_bold, paint_cyan, paint_dim, paint_yellow, term_width, truncate_end};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use unicode_width::UnicodeWidthStr;

/// Visual role of a cell — mapped once to comfy-table styling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CellStyle {
    #[default]
    Plain,
    /// Primary identifiers / commands (cyan).
    Accent,
    /// Secondary / NULL (dim grey).
    Muted,
    /// Attention (yellow) — e.g. PK.
    Warn,
    Bold,
    /// Text already contains ANSI; do not wrap with Cell colors.
    Precolored,
}

#[derive(Debug, Clone)]
pub struct StyledCell {
    pub text: String,
    pub style: CellStyle,
}

impl StyledCell {
    pub fn new(text: impl Into<String>, style: CellStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(text, CellStyle::Plain)
    }

    pub fn accent(text: impl Into<String>) -> Self {
        Self::new(text, CellStyle::Accent)
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self::new(text, CellStyle::Muted)
    }

    pub fn warn(text: impl Into<String>) -> Self {
        Self::new(text, CellStyle::Warn)
    }
}

impl<T: Into<String>> From<T> for StyledCell {
    fn from(value: T) -> Self {
        Self::plain(value)
    }
}

/// One consistent table builder used everywhere.
#[derive(Debug, Default, Clone)]
pub struct DataTable {
    headers: Vec<StyledCell>,
    rows: Vec<Vec<StyledCell>>,
    /// Soft-truncate cell text longer than this (0 = no truncate).
    max_cell_width: usize,
}

impl DataTable {
    pub fn new() -> Self {
        Self {
            max_cell_width: 64,
            ..Default::default()
        }
    }

    pub fn max_cell_width(mut self, width: usize) -> Self {
        self.max_cell_width = width;
        self
    }

    pub fn headers<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.headers = cols
            .into_iter()
            .map(|c| StyledCell::new(c, CellStyle::Bold))
            .collect();
        self
    }

    pub fn headers_styled<I>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = StyledCell>,
    {
        self.headers = cols.into_iter().collect();
        self
    }

    /// Column name (cyan/bold) on the first line, SQL type (yellow) on the second.
    pub fn headers_with_types<I, N, T>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = (N, T)>,
        N: Into<String>,
        T: Into<String>,
    {
        self.headers = cols
            .into_iter()
            .map(|(name, ty)| {
                let name = name.into();
                let ty = ty.into();
                let name_painted = paint_accent(&name);
                let text = if ty.is_empty() || ty == "?" {
                    name_painted
                } else {
                    format!("{name_painted}\n{}", paint_yellow(&ty))
                };
                StyledCell::new(text, CellStyle::Precolored)
            })
            .collect();
        self
    }

    pub fn row<I, S>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<StyledCell>,
    {
        self.rows
            .push(cells.into_iter().map(Into::into).collect());
        self
    }

    pub fn push_row<I, S>(&mut self, cells: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<StyledCell>,
    {
        self.rows
            .push(cells.into_iter().map(Into::into).collect());
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Render using the global SQffy table preset.
    pub fn render(&self) -> String {
        let mut table = base_table();
        if !self.headers.is_empty() {
            table.set_header(self.headers.iter().map(|c| style_header(c)));
        }
        for row in &self.rows {
            table.add_row(row.iter().map(|c| style_body(c, self.max_cell_width)));
        }
        table.to_string()
    }

    /// Title banner + table (used by help / describe).
    pub fn render_with_title(&self, title: &str) -> String {
        let mut out = title_banner(title);
        out.push('\n');
        out.push_str(&self.render());
        out.push('\n');
        out
    }
}

fn base_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
}

fn style_header(cell: &StyledCell) -> Cell {
    if cell.style == CellStyle::Precolored {
        return Cell::new(&cell.text);
    }
    let mut c = Cell::new(&cell.text).add_attribute(Attribute::Bold);
    match cell.style {
        CellStyle::Accent => c = c.fg(Color::Cyan),
        CellStyle::Warn => c = c.fg(Color::Yellow),
        CellStyle::Muted => c = c.fg(Color::DarkGrey),
        CellStyle::Plain | CellStyle::Bold | CellStyle::Precolored => {}
    }
    c
}

fn style_body(cell: &StyledCell, max_width: usize) -> Cell {
    let text = if cell.style != CellStyle::Precolored
        && max_width > 0
        && cell.text.chars().count() > max_width
    {
        let truncated: String = cell.text.chars().take(max_width.saturating_sub(1)).collect();
        format!("{truncated}…")
    } else {
        cell.text.clone()
    };

    let mut c = Cell::new(text);
    match cell.style {
        CellStyle::Accent => c = c.fg(Color::Cyan),
        CellStyle::Muted => c = c.fg(Color::DarkGrey),
        CellStyle::Warn => c = c.fg(Color::Yellow),
        CellStyle::Bold => c = c.add_attribute(Attribute::Bold),
        CellStyle::Plain | CellStyle::Precolored => {}
    }
    c
}

/// Rounded cyan title strip — shared chrome above tables/sections.
pub fn title_banner(title: &str) -> String {
    let width = term_width();
    let inner = width.saturating_sub(2);
    let label = format!("  {title}  ");
    let pad = inner.saturating_sub(label.width());

    let mut out = String::new();
    out.push_str(&paint_cyan(&format!(
        "╭{}╮\n",
        "─".repeat(inner)
    )));
    out.push_str(&format!(
        "{}{}{}{}\n",
        paint_cyan("│"),
        paint_bold(&label),
        " ".repeat(pad),
        paint_cyan("│")
    ));
    out.push_str(&paint_cyan(&format!("╰{}╯", "─".repeat(inner))));
    out
}

pub fn section_heading(title: &str) -> String {
    paint_bold(title)
}

pub fn footnote(text: &str) -> String {
    paint_dim(&format!("  ◆  {}", truncate_end(text, term_width().saturating_sub(6))))
}

/// Value cell helper for SQL result grids.
pub fn sql_value_cell(value: &str) -> StyledCell {
    if value == "NULL" {
        StyledCell::muted("NULL")
    } else {
        StyledCell::plain(value)
    }
}

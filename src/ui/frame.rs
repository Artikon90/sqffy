use super::{
    color_enabled, paint_bold, paint_cyan, paint_dim, truncate_end, truncate_middle,
};
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

/// Prefer `$COLUMNS`, else a comfortable default.
pub fn term_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(72)
        .clamp(52, 100)
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub key: String,
    pub label: String,
    pub hint: Option<String>,
    pub emphasis: bool,
}

impl MenuItem {
    pub fn action(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            hint: None,
            emphasis: true,
        }
    }

    pub fn connection(
        key: impl Into<String>,
        label: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            hint: Some(hint.into()),
            emphasis: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Panel {
    pub title: String,
    pub subtitle: Option<String>,
    pub items: Vec<MenuItem>,
    pub footer: Option<String>,
}

impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    pub fn items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self
    }

    pub fn footer(mut self, text: impl Into<String>) -> Self {
        self.footer = Some(text.into());
        self
    }
}

fn border_line(left: &str, fill: char, right: &str, inner: usize) -> String {
    let mid: String = std::iter::repeat(fill).take(inner).collect();
    paint_cyan(&format!("{left}{mid}{right}"))
}

fn bordered_line(text: &str, style: fn(&str) -> String, inner: usize) -> String {
    let pad = inner.saturating_sub(text.width());
    format!(
        "{}{}{}{}",
        paint_cyan("│"),
        style(text),
        " ".repeat(pad),
        paint_cyan("│")
    )
}

/// Render panel to a multi-line string (no trailing newline after last line).
pub fn panel_to_string(p: &Panel, selected: Option<usize>) -> String {
    let width = term_width();
    let inner = width.saturating_sub(2);
    let mut out = Vec::new();

    out.push(border_line("╭", '─', "╮", inner));
    out.push(bordered_line(&format!("  {}  ", p.title), paint_bold, inner));

    if let Some(sub) = &p.subtitle {
        let truncated = truncate_end(sub, inner.saturating_sub(4));
        out.push(bordered_line(
            &format!("  {truncated}"),
            paint_dim,
            inner,
        ));
    }

    out.push(border_line("├", '─', "┤", inner));

    if p.items.is_empty() {
        out.push(bordered_line("  —", paint_dim, inner));
    } else {
        for (idx, item) in p.items.iter().enumerate() {
            out.push(menu_row_line(item, inner, selected == Some(idx)));
        }
    }

    if let Some(footer) = &p.footer {
        out.push(border_line("├", '─', "┤", inner));
        let truncated = truncate_middle(footer, inner.saturating_sub(4));
        out.push(bordered_line(
            &format!("  {truncated}"),
            paint_dim,
            inner,
        ));
    }

    out.push(border_line("╰", '─', "╯", inner));
    out.join("\n")
}

/// Draw a rounded Unicode panel (no selection highlight).
pub fn panel(p: &Panel) {
    println!("{}", panel_to_string(p, None));
}

fn menu_row_line(item: &MenuItem, inner: usize, selected: bool) -> String {
    let marker = if selected { "❯" } else { " " };
    let key_plain = format!("{:>2})", item.key);

    let hint_plain = item
        .hint
        .as_ref()
        .map(|h| {
            let budget = inner
                .saturating_sub(1) // marker
                .saturating_sub(1) // space
                .saturating_sub(key_plain.width())
                .saturating_sub(1)
                .saturating_sub(item.label.width())
                .saturating_sub(2);
            if budget < 8 {
                String::new()
            } else {
                format!("  {}", truncate_middle(h, budget))
            }
        })
        .unwrap_or_default();

    let plain = format!("{marker}{key_plain} {}{hint_plain}", item.label);
    let pad = inner.saturating_sub(plain.width());

    if selected && color_enabled() {
        let body = format!("{plain}{}", " ".repeat(pad));
        // Inverse accent row — full inner width.
        let painted = body.on_bright_black().cyan().bold().to_string();
        return format!("{}{}{}", paint_cyan("│"), painted, paint_cyan("│"));
    }

    let key_styled = if item.emphasis || selected {
        paint_cyan(&key_plain)
    } else {
        paint_dim(&key_plain)
    };
    let marker_styled = if selected {
        paint_cyan(marker)
    } else {
        " ".into()
    };

    format!(
        "{}{}{} {}{}{}{}",
        paint_cyan("│"),
        marker_styled,
        key_styled,
        paint_bold(&item.label),
        paint_dim(&hint_plain),
        " ".repeat(pad),
        paint_cyan("│")
    )
}

pub fn rule() {
    let width = term_width();
    println!("{}", paint_dim(&"─".repeat(width)));
}

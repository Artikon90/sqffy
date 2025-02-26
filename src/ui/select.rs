//! Interactive list selection — arrows / WASD / hjkl / digits / Enter / q.
//! Filterable variant: type to narrow the list.

use super::frame::{panel_to_string, MenuItem, Panel};
use super::{paint_dim, paint_yellow};
use crate::i18n::t;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{cursor, execute};
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

pub enum SelectOutcome {
    Confirmed(usize),
    Quit,
}

/// One option for [`interactive_filter_select`].
#[derive(Debug, Clone)]
pub struct FilterOption {
    pub id: String,
    pub label: String,
    pub hint: Option<String>,
}

impl FilterOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

/// Interactive selection over `panel.items`.
///
/// Controls:
/// - ↑↓ / W S / K J — move
/// - Enter — confirm
/// - `1`…`9` — jump to / confirm numbered item
/// - `q` / Esc — quit
///
/// Falls back to classic typed choice when stdin is not a TTY.
pub fn interactive_select(panel: &Panel, mut selected: usize) -> io::Result<SelectOutcome> {
    if panel.items.is_empty() {
        return Ok(SelectOutcome::Quit);
    }
    selected = selected.min(panel.items.len() - 1);

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return fallback_line_select(panel);
    }

    let mut stdout = io::stdout();
    let _raw = RawModeGuard::enter()?;
    let mut first = true;
    let mut last_lines = 0usize;

    loop {
        if !first {
            if last_lines > 0 {
                execute!(
                    stdout,
                    cursor::MoveUp(last_lines as u16),
                    Clear(ClearType::FromCursorDown)
                )?;
            }
        }
        first = false;

        let rendered = panel_to_string(panel, Some(selected));
        last_lines = rendered.lines().count();
        // raw mode: `\n` must be `\r\n` for correct cursor geometry on Windows.
        for line in rendered.lines() {
            write!(stdout, "{line}\r\n")?;
        }
        write!(
            stdout,
            "{}\r\n",
            paint_dim(&format!("  {}", t("picker.nav_hint")))
        )?;
        last_lines += 1;
        stdout.flush()?;

        loop {
            if !event::poll(Duration::from_millis(500))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Char('k') => {
                    if selected == 0 {
                        selected = panel.items.len() - 1;
                    } else {
                        selected -= 1;
                    }
                    break;
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('j') => {
                    selected = (selected + 1) % panel.items.len();
                    break;
                }
                KeyCode::Home | KeyCode::Char('a') | KeyCode::Char('A') => {
                    selected = 0;
                    break;
                }
                KeyCode::End | KeyCode::Char('d') | KeyCode::Char('D') => {
                    selected = panel.items.len() - 1;
                    break;
                }
                KeyCode::Enter => return Ok(SelectOutcome::Confirmed(selected)),
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    return Ok(SelectOutcome::Quit);
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap_or(0) as usize;
                    if digit == 0 {
                        continue;
                    }
                    if let Some(idx) = panel
                        .items
                        .iter()
                        .position(|item| item.key == digit.to_string())
                    {
                        return Ok(SelectOutcome::Confirmed(idx));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Filterable picker: type to filter, ↑↓ to move, Enter to confirm, Esc to quit.
///
/// Returns the index into the **original** `options` slice (not the filtered view).
pub fn interactive_filter_select(
    title: &str,
    subtitle: &str,
    options: &[FilterOption],
) -> io::Result<SelectOutcome> {
    if options.is_empty() {
        return Ok(SelectOutcome::Quit);
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return fallback_filter_select(title, options);
    }

    let mut filter = String::new();
    let mut selected = 0usize;
    let mut stdout = io::stdout();
    let _raw = RawModeGuard::enter()?;
    let mut first = true;
    let mut last_lines = 0usize;

    loop {
        let visible = filtered_indices(options, &filter);
        if visible.is_empty() {
            selected = 0;
        } else if selected >= visible.len() {
            selected = visible.len() - 1;
        }

        if !first && last_lines > 0 {
            execute!(
                stdout,
                cursor::MoveUp(last_lines as u16),
                Clear(ClearType::FromCursorDown)
            )?;
        }
        first = false;

        let panel = build_filter_panel(title, subtitle, options, &visible, &filter);
        let rendered = panel_to_string(&panel, if visible.is_empty() { None } else { Some(selected) });
        last_lines = rendered.lines().count();
        for line in rendered.lines() {
            write!(stdout, "{line}\r\n")?;
        }
        stdout.flush()?;

        loop {
            if !event::poll(Duration::from_millis(500))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Up => {
                    if !visible.is_empty() {
                        selected = if selected == 0 {
                            visible.len() - 1
                        } else {
                            selected - 1
                        };
                    }
                    break;
                }
                KeyCode::Down => {
                    if !visible.is_empty() {
                        selected = (selected + 1) % visible.len();
                    }
                    break;
                }
                KeyCode::Enter => {
                    if let Some(&orig) = visible.get(selected) {
                        return Ok(SelectOutcome::Confirmed(orig));
                    }
                }
                KeyCode::Esc => {
                    if !filter.is_empty() {
                        filter.clear();
                        selected = 0;
                        break;
                    }
                    return Ok(SelectOutcome::Quit);
                }
                KeyCode::Backspace => {
                    filter.pop();
                    selected = 0;
                    break;
                }
                KeyCode::Char(c) if !c.is_control() => {
                    // Digits jump when filter empty and matches a key number
                    if filter.is_empty() && c.is_ascii_digit() {
                        let digit = c.to_digit(10).unwrap_or(0) as usize;
                        if digit >= 1 && digit <= options.len() {
                            return Ok(SelectOutcome::Confirmed(digit - 1));
                        }
                    }
                    filter.push(c);
                    selected = 0;
                    break;
                }
                _ => {}
            }
        }
    }
}

fn filtered_indices(options: &[FilterOption], filter: &str) -> Vec<usize> {
    if filter.is_empty() {
        return (0..options.len()).collect();
    }
    let q = filter.to_ascii_lowercase();
    options
        .iter()
        .enumerate()
        .filter(|(_, opt)| {
            opt.id.to_ascii_lowercase().contains(&q)
                || opt.label.to_ascii_lowercase().contains(&q)
                || opt
                    .hint
                    .as_deref()
                    .is_some_and(|h| h.to_ascii_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}

fn build_filter_panel(
    title: &str,
    subtitle: &str,
    options: &[FilterOption],
    visible: &[usize],
    filter: &str,
) -> Panel {
    let items: Vec<MenuItem> = if visible.is_empty() {
        vec![MenuItem::action("—", t("picker.filter_empty"))]
    } else {
        visible
            .iter()
            .enumerate()
            .map(|(i, &orig)| {
                let opt = &options[orig];
                let key = (i + 1).to_string();
                match &opt.hint {
                    Some(hint) => MenuItem::connection(key, opt.label.clone(), hint.clone()),
                    None => MenuItem::action(key, opt.label.clone()),
                }
            })
            .collect()
    };
    let filter_footer = if filter.is_empty() {
        format!("{}  ·  {}", t("picker.filter_placeholder"), t("picker.filter_hint"))
    } else {
        format!("› {filter}_  ·  {}", t("picker.filter_hint"))
    };
    Panel::new(title)
        .subtitle(subtitle)
        .items(items)
        .footer(filter_footer)
}

fn fallback_filter_select(title: &str, options: &[FilterOption]) -> io::Result<SelectOutcome> {
    println!("{}", paint_dim(title));
    for (i, opt) in options.iter().enumerate() {
        let hint = opt
            .hint
            .as_deref()
            .map(|h| format!(" — {h}"))
            .unwrap_or_default();
        println!("  {}) {}{}", i + 1, opt.label, hint);
    }
    print!("{} ", paint_dim(&format!("› {}", t("picker.prompt"))));
    io::stdout().flush()?;
    let mut line = String::new();
    let n = io::stdin().read_line(&mut line)?;
    if n == 0 {
        return Ok(SelectOutcome::Quit);
    }
    let trimmed = line.trim();
    if trimmed.eq_ignore_ascii_case("q") {
        return Ok(SelectOutcome::Quit);
    }
    if let Ok(n) = trimmed.parse::<usize>() {
        if n >= 1 && n <= options.len() {
            return Ok(SelectOutcome::Confirmed(n - 1));
        }
    }
    let q = trimmed.to_ascii_lowercase();
    if let Some(idx) = options.iter().position(|o| {
        o.id.eq_ignore_ascii_case(trimmed) || o.label.to_ascii_lowercase().contains(&q)
    }) {
        return Ok(SelectOutcome::Confirmed(idx));
    }
    println!("{}", paint_yellow(&t("picker.invalid")));
    Ok(SelectOutcome::Quit)
}

fn fallback_line_select(panel: &Panel) -> io::Result<SelectOutcome> {
    let rendered = panel_to_string(panel, None);
    println!("{rendered}");
    println!("{}", paint_dim(&format!("  {}", t("picker.nav_hint"))));
    print!("{} ", paint_dim(&format!("› {}", t("picker.prompt"))));
    io::stdout().flush()?;

    let mut line = String::new();
    let n = io::stdin().read_line(&mut line)?;
    if n == 0 {
        return Ok(SelectOutcome::Quit);
    }
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower == "q" || lower == "quit" || lower == "exit" {
        return Ok(SelectOutcome::Quit);
    }
    if let Ok(n) = trimmed.parse::<usize>() {
        if let Some(idx) = panel.items.iter().position(|item| item.key == n.to_string()) {
            return Ok(SelectOutcome::Confirmed(idx));
        }
    }
    println!("{}", paint_yellow(&t("picker.invalid")));
    Ok(SelectOutcome::Quit)
}

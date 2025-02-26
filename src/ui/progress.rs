//! Terminal progress bar for downloads / docker pull.

use super::{paint_cyan, paint_dim, paint_green, term_width};
use std::io::{self, Write};

pub struct ProgressBar {
    label: String,
    width: usize,
}

impl ProgressBar {
    pub fn new(label: impl Into<String>) -> Self {
        let inner = term_width().saturating_sub(24).clamp(16, 40);
        Self {
            label: label.into(),
            width: inner,
        }
    }

    /// `fraction` in 0.0..=1.0; `detail` optional right-hand text.
    pub fn render(&self, fraction: f64, detail: &str) -> io::Result<()> {
        let fraction = fraction.clamp(0.0, 1.0);
        let filled = ((self.width as f64) * fraction).round() as usize;
        let empty = self.width.saturating_sub(filled);
        let bar = format!(
            "{}{}",
            "█".repeat(filled),
            "░".repeat(empty)
        );
        let pct = (fraction * 100.0).round() as u32;
        let line = format!(
            "\r{} {} {}% {}",
            paint_dim(&self.label),
            paint_cyan(&format!("│{bar}│")),
            pct,
            paint_dim(detail)
        );
        let mut out = io::stderr();
        write!(out, "{line}")?;
        // clear rest of line
        write!(out, "\x1b[K")?;
        out.flush()
    }

    pub fn finish(&self, message: &str) -> io::Result<()> {
        let mut out = io::stderr();
        write!(out, "\r\x1b[K")?;
        writeln!(out, "{}", paint_green(message))?;
        out.flush()
    }

    pub fn fail(&self, message: &str) -> io::Result<()> {
        let mut out = io::stderr();
        write!(out, "\r\x1b[K")?;
        writeln!(out, "{}", super::paint_red(message))?;
        out.flush()
    }
}

/// Indeterminate spinner frames for unknown-length ops.
pub fn spin_frame(i: usize) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[i % FRAMES.len()]
}

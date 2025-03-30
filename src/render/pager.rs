use crate::i18n::t;
use std::io::{self, Write};

/// Minimal pager v1 — full scrollable Esc/q pager comes later.
pub fn page_if_needed(text: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "{text}")?;
    if !text.ends_with('\n') {
        writeln!(out)?;
    }
    writeln!(out, "\n{}", t("pager.end"))?;
    out.flush()?;

    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    Ok(())
}

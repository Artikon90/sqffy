use crate::complete::Completer;
use crate::sql;
use crate::ui::{color_enabled, paint_cyan};
use rustyline::completion::{Completer as RlCompleter, Pair};
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow;

pub struct ReplHelper {
    completer: Completer,
}

impl ReplHelper {
    pub fn new(completer: Completer) -> Self {
        Self { completer }
    }

    pub fn set_completer(&mut self, completer: Completer) {
        self.completer = completer;
    }
}

impl Helper for ReplHelper {}
impl Hinter for ReplHelper {
    type Hint = String;
}
impl Validator for ReplHelper {}

impl RlCompleter for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        let start = completion_start(before);
        let candidates = self.completer.candidates(before);
        let pairs = candidates
            .into_iter()
            .map(|c| Pair {
                display: c.clone(),
                replacement: c,
            })
            .collect();
        Ok((start, pairs))
    }
}

impl Highlighter for ReplHelper {
    /// Color the prompt here — never put ANSI in the string passed to `readline`,
    /// or backspace/redraw will leave visual gaps (rustyline counts bytes ≠ columns).
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if color_enabled() {
            Cow::Owned(paint_cyan(prompt))
        } else {
            Cow::Borrowed(prompt)
        }
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if color_enabled() && !line.is_empty() {
            Cow::Owned(sql::colorize(line))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, kind: CmdKind) -> bool {
        !matches!(kind, CmdKind::MoveCursor)
    }
}

/// Byte index where the token being completed starts.
///
/// `table.col` starts just after `.`, so only the column is replaced.
/// A meta-command (`.sche`) starts at the dot itself: candidates already
/// include it (`.schema`), and replacing only the tail would produce `..schema`.
fn completion_start(before: &str) -> usize {
    let bytes = before.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        let c = bytes[i - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            i -= 1;
            continue;
        }
        if c == '.' {
            let dot = i - 1;
            if before[..dot].chars().all(|ch| ch.is_ascii_whitespace()) {
                return dot;
            }
            return i;
        }
        break;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::completion_start;
    use crate::complete::{Completer, SchemaCache};

    #[test]
    fn meta_command_replaces_the_leading_dot() {
        let line = ".sche";
        let start = completion_start(line);
        assert_eq!(start, 0);

        let got = Completer::new(SchemaCache::default()).candidates(line);
        assert!(got.iter().any(|c| c == ".schema"), "{got:?}");
        for candidate in &got {
            let applied = format!("{}{candidate}", &line[..start]);
            assert_eq!(applied, candidate.as_str(), "{candidate}");
        }
    }

    #[test]
    fn qualified_column_still_starts_after_the_dot() {
        assert_eq!(completion_start("users.na"), "users.".len());
        assert_eq!(completion_start("  .ta"), 2);
        assert_eq!(completion_start(".schema use"), ".schema ".len());
    }
}

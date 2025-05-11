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
/// For `table.col` the start is right after `.` so only the column is replaced.
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
            return i;
        }
        break;
    }
    i
}

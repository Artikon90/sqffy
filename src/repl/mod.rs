//! Interactive line editor (rustyline).
//!
//! Intentionally not a TUI event loop — one prompt, one line (or continued
//! lines), then hand off to `app` for execution/rendering.

mod helper;

use crate::complete::Completer;
use crate::error::Result;
use helper::ReplHelper;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Cmd, CompletionType, Config as RlConfig, Editor, KeyEvent};

pub struct Repl {
    editor: Editor<ReplHelper, DefaultHistory>,
}

pub enum ReadOutcome {
    Line(String),
    Interrupt,
    Eof,
}

impl Repl {
    pub fn new(completer: Completer) -> Result<Self> {
        let config = RlConfig::builder()
            .completion_type(CompletionType::List)
            // Ensure colored prompts go through Highlighter (not raw ANSI in prompt).
            .build();
        let mut editor = Editor::with_config(config)?;
        editor.set_helper(Some(ReplHelper::new(completer)));
        editor.bind_sequence(KeyEvent::from('\t'), Cmd::Complete);
        // Force a clean cooked terminal after crossterm raw-mode picker.
        let _ = crossterm::terminal::disable_raw_mode();
        Ok(Self { editor })
    }

    pub fn refresh_completer(&mut self, completer: Completer) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.set_completer(completer);
        }
    }

    pub fn read_line(&mut self, prompt: &str) -> Result<ReadOutcome> {
        match self.editor.readline(prompt) {
            Ok(line) => {
                let _ = self.editor.add_history_entry(line.as_str());
                Ok(ReadOutcome::Line(line))
            }
            Err(ReadlineError::Interrupted) => Ok(ReadOutcome::Interrupt),
            Err(ReadlineError::Eof) => Ok(ReadOutcome::Eof),
            Err(err) => Err(err.into()),
        }
    }
}

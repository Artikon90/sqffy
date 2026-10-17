//! Shared SQL tokenization helpers (highlight + completion context).

mod highlight;
mod tokens;

pub use highlight::colorize;
pub use tokens::{KEYWORDS, is_keyword, split_tokens, type_names_matching};

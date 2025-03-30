//! Output formatting for stdout (and optional pager).
//!
//! Keep this layer free of database I/O — it only turns structured data into text.

mod ddl;
mod error;
mod help;
mod pager;
mod schema;
mod table;

pub use ddl::{ddl_block, schema_index, tables_list};
pub use error::format_error;
pub use help::help_text;
pub use pager::page_if_needed;
pub use schema::table_schema;
pub use table::{describe_table, query_result};

use crate::config::Config;
use crate::db::QueryResult;

/// Something ready to print (and optionally page).
#[derive(Debug, Clone)]
pub struct Rendered {
    pub text: String,
    /// Hint for the pager: prefer paging when true.
    pub tall: bool,
}

impl Rendered {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tall: false,
        }
    }

    pub fn tall(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tall: true,
        }
    }
}

pub fn print_rendered(rendered: &Rendered, config: &Config) -> std::io::Result<()> {
    if config.pager && rendered.tall {
        page_if_needed(&rendered.text)
    } else {
        print!("{}", rendered.text);
        if !rendered.text.ends_with('\n') {
            println!();
        }
        Ok(())
    }
}

pub fn from_query(result: &QueryResult, config: &Config) -> Rendered {
    query_result(result, config)
}

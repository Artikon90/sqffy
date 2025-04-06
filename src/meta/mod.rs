//! Dot-commands (`.tables`, `.schema`, `.describe`, …).

use crate::db::Database;
use crate::error::{Error, Result};
use crate::i18n::{t, tr};
use crate::render::{self, Rendered};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaCommand {
    Help,
    Tables,
    Schema { table: Option<String> },
    Describe { table: String },
    /// `.generate <table> <count> [--yes] [--dry-run] [col=ref_table.ref_col …]`
    Generate {
        table: String,
        count: usize,
        yes: bool,
        dry_run: bool,
        /// column → (ref_table, ref_column)
        fks: Vec<(String, String, String)>,
    },
    Quit,
}

impl MetaCommand {
    pub fn parse(line: &str) -> Result<Self> {
        let line = line.trim();
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        match cmd {
            ".help" | ".h" | "?" => Ok(Self::Help),
            ".tables" | ".t" => Ok(Self::Tables),
            ".schema" | ".s" => Ok(Self::Schema {
                table: parts.next().map(str::to_string),
            }),
            ".describe" | ".d" => {
                let table = parts
                    .next()
                    .ok_or_else(|| Error::msg(t("meta.usage_describe")))?
                    .to_string();
                Ok(Self::Describe { table })
            }
            ".generate" | ".gen" => parse_generate(parts),
            ".quit" | ".q" | ".exit" => Ok(Self::Quit),
            other => Err(Error::msg(tr("meta.unknown", &[("cmd", other)]))),
        }
    }
}

fn parse_generate<'a>(mut parts: impl Iterator<Item = &'a str>) -> Result<MetaCommand> {
    let table = parts
        .next()
        .ok_or_else(|| Error::msg(t("meta.usage_generate")))?
        .to_string();
    let count: usize = parts
        .next()
        .ok_or_else(|| Error::msg(t("meta.usage_generate")))?
        .parse()
        .map_err(|_| Error::msg(t("meta.usage_generate")))?;
    let mut yes = false;
    let mut dry_run = false;
    let mut fks = Vec::new();
    for tok in parts {
        match tok {
            "--yes" | "-y" => yes = true,
            "--dry-run" => dry_run = true,
            other if other.contains('=') => {
                let (col, rest) = other.split_once('=').unwrap();
                let (rt, rc) = rest.split_once('.').ok_or_else(|| {
                    Error::msg(t("meta.usage_generate"))
                })?;
                fks.push((col.to_string(), rt.to_string(), rc.to_string()));
            }
            _ => return Err(Error::msg(t("meta.usage_generate"))),
        }
    }
    Ok(MetaCommand::Generate {
        table,
        count,
        yes,
        dry_run,
        fks,
    })
}

pub fn execute(db: &dyn Database, command: &MetaCommand) -> Result<Option<Rendered>> {
    match command {
        MetaCommand::Help => Ok(Some(render::help_text())),
        MetaCommand::Tables => {
            let tables = db.list_tables()?;
            Ok(Some(render::tables_list(&tables)))
        }
        MetaCommand::Schema { table: None } => {
            let tables = db.list_tables()?;
            Ok(Some(render::schema_index(&tables)))
        }
        MetaCommand::Schema { table: Some(name) } => {
            let schema = db.inspect_table(name)?;
            Ok(Some(render::table_schema(&schema)))
        }
        MetaCommand::Describe { table } => {
            let cols = db.describe_table(table)?;
            Ok(Some(render::describe_table(table, &cols)))
        }
        MetaCommand::Generate { .. } => {
            // Handled in App with &mut Database.
            Ok(None)
        }
        MetaCommand::Quit => Ok(None),
    }
}

use super::Rendered;
use crate::db::TableInfo;
use crate::i18n::{t, tr};
use crate::sql;
use crate::ui::{footnote, section_heading, DataTable, StyledCell};
use owo_colors::OwoColorize;

pub fn tables_list(tables: &[TableInfo]) -> Rendered {
    if tables.is_empty() {
        return Rendered::plain(t("meta.no_tables"));
    }

    let mut table = DataTable::new().headers([t("meta.col.kind"), t("meta.col.name")]);
    for tinfo in tables {
        let name = match &tinfo.schema {
            Some(schema) if !schema.is_empty() => format!("{schema}.{}", tinfo.name),
            _ => tinfo.name.clone(),
        };
        table.push_row([
            StyledCell::muted(&tinfo.kind),
            StyledCell::accent(name),
        ]);
    }

    let mut out = table.render();
    out.push('\n');
    out.push_str(&tr(
        "meta.objects",
        &[("count", &tables.len().to_string())],
    ));
    out.push('\n');
    Rendered::plain(out)
}

pub fn schema_index(tables: &[TableInfo]) -> Rendered {
    let mut out = String::new();
    out.push_str(&section_heading(&t("meta.use_schema_table")));
    out.push('\n');

    if tables.is_empty() {
        out.push_str(&t("meta.no_tables"));
        out.push('\n');
        return Rendered::plain(out);
    }

    let mut table = DataTable::new().headers([t("meta.col.name"), t("meta.col.kind")]);
    for tinfo in tables {
        let name = match &tinfo.schema {
            Some(schema) if !schema.is_empty() => format!("{schema}.{}", tinfo.name),
            _ => tinfo.name.clone(),
        };
        table.push_row([
            StyledCell::accent(name),
            StyledCell::muted(&tinfo.kind),
        ]);
    }
    out.push_str(&table.render());
    out.push('\n');
    out.push_str(&footnote(&t("meta.schema_hint")));
    out.push('\n');
    Rendered::plain(out)
}

pub fn ddl_block(table: &str, ddl: &str) -> Rendered {
    let mut out = String::new();
    out.push_str(&format!("-- {}\n", table.bold()));
    out.push_str(&sql::colorize(ddl));
    if !ddl.ends_with('\n') {
        out.push('\n');
    }
    Rendered::plain(out)
}

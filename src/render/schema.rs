//! Rich `.schema <table>` output: columns, constraints, indexes.

use super::Rendered;
use crate::db::TableSchema;
use crate::i18n::{t, tr};
use crate::sql;
use crate::ui::{footnote, section_heading, title_banner, DataTable, StyledCell};

pub fn table_schema(schema: &TableSchema) -> Rendered {
    let mut out = title_banner(&tr(
        "meta.table_header",
        &[("table", &schema.name)],
    ));
    out.push('\n');
    out.push_str(&footnote(&tr(
        "meta.schema_kind",
        &[("kind", &schema.kind)],
    )));
    out.push('\n');
    out.push('\n');

    out.push_str(&section_heading(&t("meta.section.columns")));
    out.push('\n');
    out.push_str(&columns_table(schema).render());
    out.push('\n');

    out.push('\n');
    out.push_str(&section_heading(&t("meta.section.constraints")));
    out.push('\n');
    if schema.constraints.is_empty() {
        out.push_str(&footnote(&t("meta.none")));
        out.push('\n');
    } else {
        out.push_str(&constraints_table(schema).render());
        out.push('\n');
    }

    out.push('\n');
    out.push_str(&section_heading(&t("meta.section.indexes")));
    out.push('\n');
    if schema.indexes.is_empty() {
        out.push_str(&footnote(&t("meta.none")));
        out.push('\n');
    } else {
        out.push_str(&indexes_table(schema).render());
        out.push('\n');
    }

    if let Some(ddl) = schema.ddl.as_deref().filter(|s| !s.is_empty()) {
        out.push('\n');
        out.push_str(&section_heading(&t("meta.section.ddl")));
        out.push('\n');
        out.push_str(&sql::colorize(ddl));
        if !ddl.ends_with('\n') {
            out.push('\n');
        }
    }

    Rendered::plain(out)
}

fn columns_table(schema: &TableSchema) -> DataTable {
    let mut grid = DataTable::new().headers([
        t("meta.column"),
        t("meta.type"),
        t("meta.null"),
        t("meta.default"),
        t("meta.pk"),
        t("meta.comment"),
    ]);

    for col in &schema.columns {
        grid.push_row([
            StyledCell::accent(&col.name),
            StyledCell::plain(if col.type_name.is_empty() {
                "?"
            } else {
                &col.type_name
            }),
            StyledCell::plain(if col.not_null { "NO" } else { "YES" }),
            StyledCell::muted(col.default_value.as_deref().unwrap_or("—")),
            if col.primary_key {
                StyledCell::warn("PK")
            } else {
                StyledCell::muted("—")
            },
            StyledCell::muted(col.comment.as_deref().unwrap_or("—")),
        ]);
    }
    grid
}

fn constraints_table(schema: &TableSchema) -> DataTable {
    let mut grid = DataTable::new().headers([
        t("meta.constraint.kind"),
        t("meta.constraint.name"),
        t("meta.constraint.definition"),
    ]);
    for c in &schema.constraints {
        grid.push_row([
            StyledCell::warn(&c.kind),
            StyledCell::accent(&c.name),
            StyledCell::plain(&c.definition),
        ]);
    }
    grid
}

fn indexes_table(schema: &TableSchema) -> DataTable {
    let mut grid = DataTable::new().headers([
        t("meta.index.name"),
        t("meta.index.unique"),
        t("meta.index.origin"),
        t("meta.index.columns"),
    ]);
    for idx in &schema.indexes {
        grid.push_row([
            StyledCell::accent(&idx.name),
            if idx.unique {
                StyledCell::warn("YES")
            } else {
                StyledCell::muted("NO")
            },
            StyledCell::muted(origin_label(&idx.origin)),
            StyledCell::plain(idx.columns.join(", ")),
        ]);
    }
    grid
}

fn origin_label(origin: &str) -> &str {
    match origin {
        "pk" => "primary key",
        "u" => "unique",
        "c" => "create index",
        other => other,
    }
}

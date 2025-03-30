use super::Rendered;
use crate::config::Config;
use crate::db::{ColumnInfo, QueryResult};
use crate::i18n::{t, tr};
use crate::ui::{sql_value_cell, title_banner, DataTable, StyledCell};

pub fn query_result(result: &QueryResult, config: &Config) -> Rendered {
    if let Some(affected) = result.rows_affected {
        return Rendered::plain(tr(
            "meta.rows_affected",
            &[("count", &affected.to_string())],
        ));
    }
    if result.columns.is_empty() {
        return Rendered::plain(String::new());
    }

    let types = if result.column_types.len() == result.columns.len() {
        result.column_types.clone()
    } else {
        vec!["?".into(); result.columns.len()]
    };

    let mut table = DataTable::new()
        .max_cell_width(config.max_cell_width)
        .headers_with_types(
            result
                .columns
                .iter()
                .cloned()
                .zip(types.into_iter()),
        );

    for row in &result.rows {
        table.push_row(row.iter().map(|v| sql_value_cell(v)));
    }

    let mut text = table.render();
    text.push('\n');
    text.push_str(&tr(
        "meta.rows_count",
        &[("count", &result.rows.len().to_string())],
    ));
    if result.truncated {
        text.push_str(&tr(
            "meta.truncated",
            &[("limit", &config.row_limit.to_string())],
        ));
    }
    text.push('\n');

    let tall = result.rows.len() > 30 || text.lines().count() > 40;
    if tall {
        Rendered::tall(text)
    } else {
        Rendered::plain(text)
    }
}

pub fn describe_table(table: &str, cols: &[ColumnInfo]) -> Rendered {
    let mut grid = DataTable::new().headers([
        t("meta.column"),
        t("meta.type"),
        t("meta.null"),
        t("meta.default"),
        t("meta.pk"),
    ]);

    for col in cols {
        grid.push_row([
            StyledCell::accent(&col.name),
            StyledCell::plain(if col.type_name.is_empty() {
                "?"
            } else {
                &col.type_name
            }),
            StyledCell::plain(if col.not_null { "NO" } else { "YES" }),
            StyledCell::muted(col.default_value.as_deref().unwrap_or("")),
            if col.primary_key {
                StyledCell::warn("PK")
            } else {
                StyledCell::plain("")
            },
        ]);
    }

    let mut text = title_banner(&tr("meta.table_header", &[("table", table)]));
    text.push('\n');
    text.push_str(&grid.render());
    text.push('\n');
    Rendered::plain(text)
}

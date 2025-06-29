//! Synthetic row generator with SQL preview and mandatory batch inserts.

mod plan;
mod values;

pub use plan::{GeneratePlan, FkRef};
pub use values::gen_value;

use crate::db::Database;
use crate::error::{Error, Result};
use crate::i18n::{t, tr};
use rand::Rng;

#[cfg(test)]
mod batch_tests {
    use super::*;

    #[test]
    fn preview_caps_at_ten() {
        let plan = GeneratePlan {
            table: "t".into(),
            columns: vec!["id".into()],
            types: vec!["INTEGER".into()],
            fks: vec![],
            batch_size: 200,
        };
        let rows: Vec<Vec<String>> = (0..10).map(|i| vec![i.to_string()]).collect();
        let text = preview_sql(&plan, &rows, 100_000, '"');
        assert!(text.contains("99990 more") || text.contains("and 99990"));
        let value_lines = text.lines().filter(|l| l.contains('(')).count();
        assert!(value_lines <= 12, "{text}");
    }

    #[test]
    fn batch_sql_multi_values() {
        let rows = vec![vec!["1".into()], vec!["2".into()], vec!["3".into()]];
        let sql = build_insert_sql("t", &["id".into()], &rows, '"');
        assert!(sql.contains("VALUES"));
        assert_eq!(sql.matches("),").count() + if sql.contains(')') { 1 } else { 0 }, 3);
    }
}

pub const PREVIEW_ROWS: usize = 10;
pub const DEFAULT_BATCH_SIZE: usize = 200;

/// Build INSERT SQL for `rows` (already generated value vectors).
pub fn build_insert_sql(table: &str, columns: &[String], rows: &[Vec<String>], quote: char) -> String {
    let cols = columns
        .iter()
        .map(|c| format!("{quote}{c}{quote}"))
        .collect::<Vec<_>>()
        .join(", ");
    let values = rows
        .iter()
        .map(|r| format!("({})", r.join(", ")))
        .collect::<Vec<_>>()
        .join(",\n  ");
    format!("INSERT INTO {quote}{table}{quote} ({cols}) VALUES\n  {values};")
}

/// Preview text: at most [`PREVIEW_ROWS`] sample value tuples + note about the rest.
pub fn preview_sql(plan: &GeneratePlan, sample_rows: &[Vec<String>], total: usize, quote: char) -> String {
    let shown = sample_rows.len().min(PREVIEW_ROWS);
    let sql = build_insert_sql(&plan.table, &plan.columns, &sample_rows[..shown], quote);
    let mut out = sql;
    if total > shown {
        out.push_str(&format!(
            "\n-- … and {} more row(s) (batched inserts of {})\n",
            total - shown,
            plan.batch_size
        ));
    }
    out
}

/// Generate `count` rows according to plan (FK pools must be filled first).
pub fn generate_rows(plan: &GeneratePlan, count: usize, rng: &mut impl Rng) -> Vec<Vec<String>> {
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        let mut row = Vec::with_capacity(plan.columns.len());
        for (i, col) in plan.columns.iter().enumerate() {
            if let Some(fk) = plan.fk_for(col) {
                if fk.pool.is_empty() {
                    row.push("NULL".into());
                } else {
                    let pick = &fk.pool[rng.gen_range(0..fk.pool.len())];
                    row.push(sql_literal(pick));
                }
            } else {
                let ty = plan.types.get(i).map(|s| s.as_str()).unwrap_or("TEXT");
                row.push(gen_value(ty, rng));
            }
        }
        rows.push(row);
    }
    rows
}

pub fn sql_literal(raw: &str) -> String {
    if raw.eq_ignore_ascii_case("NULL") {
        return "NULL".into();
    }
    if raw.parse::<i64>().is_ok() || raw.parse::<f64>().is_ok() {
        return raw.to_string();
    }
    format!("'{}'", raw.replace('\'', "''"))
}

/// Load FK id pools from referenced tables.
pub fn fill_fk_pools(db: &mut dyn Database, plan: &mut GeneratePlan) -> Result<()> {
    let random_fn = match db.driver_id() {
        "mysql" => "RAND()",
        _ => "RANDOM()",
    };
    let quote = db.capabilities().quote_char;
    for fk in &mut plan.fks {
        let sql = format!(
            "SELECT {q}{col}{q} FROM {q}{table}{q} ORDER BY {random_fn} LIMIT 500",
            q = quote,
            col = fk.ref_column,
            table = fk.ref_table,
        );
        let result = db.execute_sql(&sql, 500)?;
        fk.pool = result
            .rows
            .into_iter()
            .filter_map(|r| r.into_iter().next())
            .filter(|v| v != "NULL")
            .collect();
        if fk.pool.is_empty() {
            return Err(Error::msg(tr(
                "generate.empty_fk",
                &[("table", &fk.ref_table), ("column", &fk.ref_column)],
            )));
        }
    }
    Ok(())
}

/// Apply plan: preview, then batched inserts. Returns rows inserted.
pub fn apply(
    db: &mut dyn Database,
    plan: &GeneratePlan,
    count: usize,
    dry_run: bool,
    confirm: bool,
) -> Result<u64> {
    let mut rng = rand::thread_rng();
    let preview_n = count.min(PREVIEW_ROWS);
    let sample = generate_rows(plan, preview_n, &mut rng);
    let quote = db.capabilities().quote_char;
    println!("{}", preview_sql(plan, &sample, count, quote));

    if dry_run {
        println!("{}", t("generate.dry_run"));
        return Ok(0);
    }

    if !confirm && count >= 100 {
        return Err(Error::msg(t("generate.need_yes")));
    }

    let mut inserted = 0u64;
    let mut remaining = count;
    while remaining > 0 {
        let chunk = remaining.min(plan.batch_size);
        let rows = generate_rows(plan, chunk, &mut rng);
        let sql = build_insert_sql(&plan.table, &plan.columns, &rows, quote);
        let result = db.execute_sql(&sql, 0)?;
        inserted += result.rows_affected.unwrap_or(chunk as u64);
        remaining -= chunk;
    }
    Ok(inserted)
}

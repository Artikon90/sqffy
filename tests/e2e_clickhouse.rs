//! ClickHouse e2e — enabled when `SQFFY_E2E_CLICKHOUSE` is set.

mod common;

use common::{assert_contains, TestEnv};

fn url() -> Option<String> {
    std::env::var("SQFFY_E2E_CLICKHOUSE").ok().filter(|s| !s.is_empty())
}

#[test]
#[ignore = "requires SQFFY_E2E_CLICKHOUSE (see e2e/)"]
fn clickhouse_crud_and_meta() {
    let Some(url) = url() else {
        eprintln!("skip: SQFFY_E2E_CLICKHOUSE not set");
        return;
    };
    let env = TestEnv::new();
    let table = format!("e2e_ch_{}", std::process::id());

    env.run_ok(&[
        "--no-pager",
        "--no-save",
        "--lang",
        "en",
        &url,
        "-c",
        &format!(
            r#"
DROP TABLE IF EXISTS {table};
CREATE TABLE {table} (
  id UInt32,
  title String
) ENGINE = MergeTree ORDER BY id;
INSERT INTO {table} VALUES (1, 'alpha'), (2, 'beta');
SELECT title FROM {table} ORDER BY id;
.tables
.schema {table}
.describe {table}
"#
        ),
    ]);

    let count = env.run_ok(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("SELECT count() AS c FROM {table};"),
    ]);
    assert_contains(&TestEnv::combined(&count), "2");

    let _ = env.run(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("DROP TABLE IF EXISTS {table};"),
    ]);
}

#[test]
#[ignore = "requires SQFFY_E2E_CLICKHOUSE (see e2e/)"]
fn clickhouse_sql_error_box() {
    let Some(url) = url() else {
        eprintln!("skip: SQFFY_E2E_CLICKHOUSE not set");
        return;
    };
    let env = TestEnv::new();
    let out = env.run(&[
        "--no-pager",
        "--no-save",
        "--lang",
        "en",
        &url,
        "-c",
        "SELECT * FROM definitely_missing_table_e2e;",
    ]);
    assert!(!out.status.success());
    assert_contains(&TestEnv::combined(&out), "╭");
}

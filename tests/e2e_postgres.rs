//! PostgreSQL e2e — enabled when `SQFFY_E2E_POSTGRES` is set (Docker/CI).

mod common;

use common::{assert_contains, TestEnv};

fn pg_url() -> String {
    std::env::var("SQFFY_E2E_POSTGRES").expect("SQFFY_E2E_POSTGRES")
}

#[test]
#[ignore = "requires SQFFY_E2E_POSTGRES (see e2e/)"]
fn postgres_crud_schema_generate() {
    let url = pg_url();
    let env = TestEnv::new();
    let suffix = std::process::id();
    let table = format!("e2e_users_{suffix}");

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
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  email TEXT
);
INSERT INTO {table} (name, email) VALUES ('Ada', 'ada@example.com');
SELECT name FROM {table};
.tables
.schema {table}
.describe {table}
"#
        ),
    ]);

    let generated = env.run_ok(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!(".generate {table} 15 --yes"),
    ]);
    assert_contains(&TestEnv::combined(&generated), "15");

    let count = env.run_ok(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("SELECT COUNT(*) AS c FROM {table};"),
    ]);
    // 1 seed + 15 generated
    assert_contains(&TestEnv::combined(&count), "16");

    let _ = env.run(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("DROP TABLE IF EXISTS {table};"),
    ]);
}

#[test]
#[ignore = "requires SQFFY_E2E_POSTGRES (see e2e/)"]
fn postgres_sql_error_box() {
    let url = pg_url();
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

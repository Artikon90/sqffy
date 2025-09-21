//! MySQL e2e — enabled when `SQFFY_E2E_MYSQL` is set (Docker/CI).

mod common;

use common::{assert_contains, TestEnv};

fn mysql_url() -> String {
    std::env::var("SQFFY_E2E_MYSQL").expect("SQFFY_E2E_MYSQL")
}

#[test]
#[ignore = "requires SQFFY_E2E_MYSQL (see e2e/)"]
fn mysql_crud_schema_generate() {
    let url = mysql_url();
    let env = TestEnv::new();
    let suffix = std::process::id();
    let table = format!("e2e_items_{suffix}");

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
  id INT AUTO_INCREMENT PRIMARY KEY,
  title VARCHAR(64) NOT NULL,
  qty INT
);
INSERT INTO {table} (title, qty) VALUES ('seed', 1);
SELECT title FROM {table};
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
        &format!(".generate {table} 12 --yes"),
    ]);
    assert_contains(&TestEnv::combined(&generated), "12");

    let count = env.run_ok(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("SELECT COUNT(*) AS c FROM {table};"),
    ]);
    assert_contains(&TestEnv::combined(&count), "13");

    let _ = env.run(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("DROP TABLE IF EXISTS {table};"),
    ]);
}

#[test]
#[ignore = "requires SQFFY_E2E_MYSQL (see e2e/)"]
fn mysql_sql_error_box() {
    let url = mysql_url();
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

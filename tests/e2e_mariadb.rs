//! MariaDB e2e — enabled when `SQFFY_E2E_MARIADB` is set.

mod common;

use common::{assert_contains, TestEnv};

fn url() -> Option<String> {
    std::env::var("SQFFY_E2E_MARIADB").ok().filter(|s| !s.is_empty())
}

#[test]
#[ignore = "requires SQFFY_E2E_MARIADB (see e2e/)"]
fn mariadb_crud_schema_generate() {
    let Some(url) = url() else {
        eprintln!("skip: SQFFY_E2E_MARIADB not set");
        return;
    };
    let env = TestEnv::new();
    let table = format!("e2e_maria_{}", std::process::id());

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
        &format!(".generate {table} 8 --yes"),
    ]);
    assert_contains(&TestEnv::combined(&generated), "8");

    let count = env.run_ok(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("SELECT COUNT(*) AS c FROM {table};"),
    ]);
    assert_contains(&TestEnv::combined(&count), "9");

    let _ = env.run(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("DROP TABLE IF EXISTS {table};"),
    ]);
}

#[test]
#[ignore = "requires SQFFY_E2E_MARIADB (see e2e/)"]
fn mariadb_sql_error_box() {
    let Some(url) = url() else {
        eprintln!("skip: SQFFY_E2E_MARIADB not set");
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

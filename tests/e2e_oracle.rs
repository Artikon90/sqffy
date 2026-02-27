//! Oracle e2e — enabled when `SQFFY_E2E_ORACLE` is set.
//! Also requires Oracle Instant Client on the runner host.

mod common;

use common::{assert_contains, TestEnv};

fn url() -> Option<String> {
    std::env::var("SQFFY_E2E_ORACLE").ok().filter(|s| !s.is_empty())
}

#[test]
#[ignore = "requires SQFFY_E2E_ORACLE + Instant Client (see e2e/)"]
fn oracle_crud_and_meta() {
    let Some(url) = url() else {
        eprintln!("skip: SQFFY_E2E_ORACLE not set");
        return;
    };
    let env = TestEnv::new();
    let table = format!("E2E_ORA_{}", std::process::id());

    env.run_ok(&[
        "--no-pager",
        "--no-save",
        "--lang",
        "en",
        &url,
        "-c",
        &format!(
            r#"
BEGIN EXECUTE IMMEDIATE 'DROP TABLE {table}'; EXCEPTION WHEN OTHERS THEN NULL; END;
CREATE TABLE {table} (
  id NUMBER PRIMARY KEY,
  title VARCHAR2(64) NOT NULL
);
INSERT INTO {table} (id, title) VALUES (1, 'alpha');
INSERT INTO {table} (id, title) VALUES (2, 'beta');
SELECT title FROM {table} ORDER BY id;
.tables
.describe {table}
"#
        ),
    ]);

    let count = env.run_ok(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("SELECT COUNT(*) AS c FROM {table}"),
    ]);
    assert_contains(&TestEnv::combined(&count), "2");

    let _ = env.run(&[
        "--no-pager",
        "--no-save",
        &url,
        "-c",
        &format!("DROP TABLE {table}"),
    ]);
}

#[test]
#[ignore = "requires SQFFY_E2E_ORACLE + Instant Client (see e2e/)"]
fn oracle_sql_error_box() {
    let Some(url) = url() else {
        eprintln!("skip: SQFFY_E2E_ORACLE not set");
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
        "SELECT * FROM definitely_missing_table_e2e",
    ]);
    assert!(!out.status.success());
    assert_contains(&TestEnv::combined(&out), "╭");
}

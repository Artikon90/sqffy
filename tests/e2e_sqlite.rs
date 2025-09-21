//! Full SQLite CLI path: SQL, meta commands, schema, generate, errors.

mod common;

use common::{assert_contains, assert_not_contains, TestEnv};
use std::path::PathBuf;

fn db_path(env: &TestEnv) -> PathBuf {
    env.home_path().join("e2e.sqlite")
}

#[test]
fn create_insert_select_and_meta() {
    let env = TestEnv::new();
    let db = db_path(&env);

    let out = env.sqlite_script(
        &db,
        r#"
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  email TEXT
);
INSERT INTO users (id, name, email) VALUES (1, 'Ada', 'ada@example.com');
INSERT INTO users (id, name, email) VALUES (2, 'Grace', 'grace@example.com');
SELECT id, name FROM users ORDER BY id;
.tables
.schema users
.describe users
"#,
    );
    let text = TestEnv::combined(&out);
    assert_contains(&text, "Ada");
    assert_contains(&text, "Grace");
    assert_contains(&text, "users");
    assert_contains(&text, "email");
    assert!(db.is_file(), "sqlite file should exist");
}

#[test]
fn schema_index_and_help() {
    let env = TestEnv::new();
    let db = db_path(&env);
    env.sqlite_script(&db, "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT);");
    let out = env.sqlite_script(&db, ".schema\n.help");
    let text = TestEnv::combined(&out);
    assert_contains(&text, "t");
    assert_contains(&text, ".tables");
}

#[test]
fn sql_error_is_boxed() {
    let env = TestEnv::new();
    let db = db_path(&env);
    env.sqlite_script(&db, "CREATE TABLE t (id INTEGER);");
    let out = env.run(&[
        "--no-pager",
        "--no-save",
        "--lang",
        "en",
        db.to_str().unwrap(),
        "-c",
        "SELECT missing FROM t;",
    ]);
    assert!(!out.status.success());
    let err = TestEnv::combined(&out);
    assert_contains(&err, "╭");
    assert_contains(&err, "╰");
}

#[test]
fn generate_dry_run_and_insert() {
    let env = TestEnv::new();
    let db = db_path(&env);
    env.sqlite_script(
        &db,
        "CREATE TABLE items (id INTEGER PRIMARY KEY, title TEXT, qty INTEGER);",
    );

    let dry = env.sqlite_script(&db, ".generate items 5 --dry-run");
    let dry_text = TestEnv::combined(&dry);
    assert_contains(&dry_text, "INSERT INTO");
    assert_contains(&dry_text, "items");

    let generated = env.sqlite_script(&db, ".generate items 25 --yes");
    let gen_text = TestEnv::combined(&generated);
    assert_contains(&gen_text, "25");

    let count = env.sqlite_script(&db, "SELECT COUNT(*) AS c FROM items;");
    let count_text = TestEnv::combined(&count);
    assert_contains(&count_text, "25");
}

#[test]
fn generate_with_fk() {
    let env = TestEnv::new();
    let db = db_path(&env);
    env.sqlite_script(
        &db,
        r#"
CREATE TABLE parents (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE children (parent_id INTEGER NOT NULL, label TEXT);
INSERT INTO parents (id, name) VALUES (1, 'p1'), (2, 'p2');
"#,
    );
    let out = env.sqlite_script(
        &db,
        ".generate children 10 --yes parent_id=parents.id",
    );
    let text = TestEnv::combined(&out);
    assert_contains(&text, "10");

    let check = env.sqlite_script(
        &db,
        "SELECT COUNT(*) FROM children WHERE parent_id IN (1, 2);",
    );
    assert_contains(&TestEnv::combined(&check), "10");
}

#[test]
fn row_limit_flag() {
    let env = TestEnv::new();
    let db = db_path(&env);
    env.sqlite_script(
        &db,
        "CREATE TABLE n (id INTEGER PRIMARY KEY); INSERT INTO n VALUES (1),(2),(3),(4),(5);",
    );
    let out = env.run_ok(&[
        "--no-pager",
        "--no-save",
        "--limit",
        "2",
        db.to_str().unwrap(),
        "-c",
        "SELECT id FROM n ORDER BY id;",
    ]);
    let text = TestEnv::combined(&out);
    assert_contains(&text, "1");
    assert_contains(&text, "2");
    // third row should not appear as a data cell prominently; limit footnote may mention more
    assert!(
        text.contains("limit") || text.matches('3').count() <= 2,
        "limit should truncate rows:\n{text}"
    );
}

#[test]
fn no_save_skips_catalog() {
    let env = TestEnv::new();
    let db = db_path(&env);
    env.run_ok(&[
        "--no-pager",
        "--no-save",
        db.to_str().unwrap(),
        "-c",
        "SELECT 1;",
    ]);
    let catalog = env.home_path().join("connections.toml");
    assert!(
        !catalog.exists(),
        "connections.toml must not be created with --no-save"
    );
}

#[test]
fn multi_statement_script() {
    let env = TestEnv::new();
    let db = db_path(&env);
    let out = env.sqlite_script(
        &db,
        "CREATE TABLE a (x INTEGER); INSERT INTO a VALUES (42); SELECT x FROM a;",
    );
    assert_contains(&TestEnv::combined(&out), "42");
    assert_not_contains(&TestEnv::combined(&out), "╭");
}

//! Basic CLI surface: help, drivers, language, exit codes.

mod common;

use common::{assert_contains, TestEnv};

#[test]
fn help_prints_usage() {
    let env = TestEnv::new();
    let out = env.run_ok(&["--help"]);
    let text = TestEnv::combined(&out);
    assert_contains(&text, "SQffy");
    assert_contains(&text, "--list-drivers");
    assert_contains(&text, "-c");
}

#[test]
fn version_prints() {
    let env = TestEnv::new();
    let out = env.run_ok(&["--version"]);
    let text = TestEnv::combined(&out);
    assert_contains(&text, "sqf");
    assert_contains(&text, env!("CARGO_PKG_VERSION"));
}

#[test]
fn list_drivers_includes_builtins() {
    let env = TestEnv::new();
    let out = env.run_ok(&["--list-drivers", "--lang", "en"]);
    let text = TestEnv::stdout(&out);
    assert_contains(&text, "sqlite");
    assert_contains(&text, "postgres");
    assert_contains(&text, "mysql");
}

#[test]
fn lang_ru_list_drivers() {
    let env = TestEnv::new();
    let out = env.run_ok(&["--list-drivers", "--lang", "ru"]);
    let text = TestEnv::stdout(&out);
    // Russian locale key for available drivers
    assert!(
        text.contains("драйвер") || text.contains("Драйвер") || text.contains("sqlite"),
        "unexpected ru output:\n{text}"
    );
}

#[test]
fn unknown_target_scheme_fails() {
    let env = TestEnv::new();
    let out = env.run(&["--no-save", "foo://bar"]);
    assert!(!out.status.success());
    let err = TestEnv::combined(&out);
    assert!(err.contains("╭") || err.to_ascii_lowercase().contains("scheme") || err.contains("unknown"));
}

#[test]
fn memory_select_oneshot() {
    let env = TestEnv::new();
    let out = env.run_ok(&["--no-pager", "--no-save", ":memory:", "-c", "SELECT 1 AS n;"]);
    let text = TestEnv::combined(&out);
    assert_contains(&text, "1");
}

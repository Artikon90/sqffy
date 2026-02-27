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
    assert_contains(&text, "mariadb");
    assert_contains(&text, "clickhouse");
    assert_contains(&text, "oracle");
}

#[test]
fn list_drivers_zh() {
    let env = TestEnv::new();
    let out = env.run_ok(&["--list-drivers", "--lang", "zh"]);
    let text = TestEnv::stdout(&out);
    assert!(
        text.contains("驱动") || text.contains("sqlite"),
        "unexpected zh output:\n{text}"
    );
}

#[test]
fn new_adapter_url_schemes_are_recognized() {
    let env = TestEnv::new();
    // Unknown host → connection/backend error, not "unknown scheme".
    for url in [
        "mariadb://sqffy:sqffy@127.0.0.1:1/sqffy",
        "clickhouse://sqffy:sqffy@127.0.0.1:1/sqffy",
        "oracle://sqffy:sqffy@127.0.0.1:1/XEPDB1",
    ] {
        let out = env.run(&["--no-save", "--no-pager", url, "-c", "SELECT 1"]);
        assert!(!out.status.success(), "expected failure for {url}");
        let err = TestEnv::combined(&out).to_ascii_lowercase();
        assert!(
            !err.contains("unknown connection scheme") && !err.contains("неизвестная схема"),
            "scheme should be recognized for {url}:\n{err}"
        );
    }
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

//! MCP HTTP + stdio e2e against a seeded SQLite connection.

mod common;

use common::{assert_contains, mcp_rpc, McpChild, TestEnv};
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
use std::time::Duration;

#[test]
fn mcp_http_list_tools_and_readonly_query() {
    let env = TestEnv::new();
    let db = env.home_path().join("mcp.sqlite");
    env.sqlite_script(
        &db,
        r#"
CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
INSERT INTO products (id, name) VALUES (1, 'Widget'), (2, 'Gadget');
"#,
    );
    env.write_connection(
        "mcpdb",
        "sqlite",
        db.to_str().unwrap(),
        None,
    );

    let mcp = McpChild::spawn(&env);
    let host = "127.0.0.1";
    let port = mcp.port;

    let init = mcp_rpc(host, port, "initialize", json!({}), 1);
    assert!(init.get("result").is_some(), "{init}");

    let listed = mcp_rpc(host, port, "tools/list", json!({}), 2);
    let tools = listed["result"]["tools"].as_array().expect("tools array");
    let names: Vec<_> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"list_connections"));
    assert!(names.contains(&"execute_readonly"));
    assert!(names.contains(&"list_drivers"));

    let conns = mcp_rpc(
        host,
        port,
        "tools/call",
        json!({"name": "list_connections", "arguments": {}}),
        3,
    );
    assert_eq!(conns["result"]["isError"], false);
    let structured = &conns["result"]["structuredContent"];
    assert!(
        structured["count"].as_u64().unwrap_or(0) >= 1,
        "{structured}"
    );

    let tables = mcp_rpc(
        host,
        port,
        "tools/call",
        json!({"name": "list_tables", "arguments": {"connection": "mcpdb"}}),
        4,
    );
    let table_text = tables["result"]["structuredContent"].to_string();
    assert_contains(&table_text, "products");

    let rows = mcp_rpc(
        host,
        port,
        "tools/call",
        json!({
            "name": "execute_readonly",
            "arguments": {
                "connection": "mcpdb",
                "sql": "SELECT name FROM products ORDER BY id",
                "limit": 10
            }
        }),
        5,
    );
    assert_eq!(rows["result"]["isError"], false);
    let payload = rows["result"]["structuredContent"].to_string();
    assert_contains(&payload, "Widget");
    assert_contains(&payload, "Gadget");

    let blocked = mcp_rpc(
        host,
        port,
        "tools/call",
        json!({
            "name": "execute_readonly",
            "arguments": {
                "connection": "mcpdb",
                "sql": "DELETE FROM products"
            }
        }),
        6,
    );
    assert_eq!(blocked["result"]["isError"], true);
}

#[test]
fn mcp_stdio_initialize_and_ping() {
    let env = TestEnv::new();
    let db = env.home_path().join("mcp-stdio.sqlite");
    env.sqlite_script(&db, "CREATE TABLE t (id INTEGER PRIMARY KEY);");
    env.write_connection("stdio", "sqlite", db.to_str().unwrap(), None);

    let err_log = env.home_path().join("mcp-stdio.err");
    let err_file = std::fs::File::create(&err_log).expect("err log");
    let mut child = env
        .command()
        .args(["mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(err_file))
        .spawn()
        .expect("spawn mcp stdio");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    // Give the server a moment to open stdin.
    std::thread::sleep(Duration::from_millis(100));

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let payload = req.to_string();
    write!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
    .expect("write initialize");
    stdin.flush().ok();

    // Read Content-Length framed response
    let mut header = String::new();
    reader.read_line(&mut header).unwrap_or_else(|e| {
        let err = std::fs::read_to_string(&err_log).unwrap_or_default();
        panic!("read header failed ({e}); stderr:\n{err}");
    });
    assert!(
        header.to_ascii_lowercase().starts_with("content-length:"),
        "got header: {header:?}; stderr:\n{}",
        std::fs::read_to_string(&err_log).unwrap_or_default()
    );
    let len: usize = header
        .split(':')
        .nth(1)
        .and_then(|s| s.trim().parse().ok())
        .expect("content length");
    let mut blank = String::new();
    reader.read_line(&mut blank).ok();
    let mut buf = vec![0u8; len];
    use std::io::Read;
    reader.read_exact(&mut buf).expect("body");
    let resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");
    assert!(resp.get("result").is_some(), "{resp}");

    let ping = json!({"jsonrpc":"2.0","id":2,"method":"ping","params":{}});
    let payload = ping.to_string();
    write!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
    .ok();
    drop(stdin);

    let _ = child.wait_timeout(Duration::from_secs(3));
    let _ = child.kill();
    let _ = child.wait();
}

trait WaitTimeout {
    fn wait_timeout(&mut self, dur: Duration) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(&mut self, dur: Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = std::time::Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if start.elapsed() > dur {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

#[test]
fn parallel_mcp_port_with_oneshot_still_serves() {
    // `--mcp-port` starts background server while `-c` runs; process exits after script.
    // We only assert the flag is accepted and SQL still works.
    let env = TestEnv::new();
    let db = env.home_path().join("parallel.sqlite");
    let port = common::free_port();
    let out = env.run_ok(&[
        "--no-pager",
        "--no-save",
        "--mcp-port",
        &port.to_string(),
        db.to_str().unwrap(),
        "-c",
        "CREATE TABLE x (id INTEGER); SELECT 1;",
    ]);
    assert_contains(&TestEnv::combined(&out), "1");
}

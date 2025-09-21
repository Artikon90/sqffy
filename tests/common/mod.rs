//! Shared helpers for `sqf` e2e tests.

#![allow(dead_code)]

use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub fn sqf_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sqf"))
}

pub struct TestEnv {
    pub home: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            home: TempDir::new().expect("tempdir"),
        }
    }

    pub fn home_path(&self) -> &Path {
        self.home.path()
    }

    pub fn command(&self) -> Command {
        let mut cmd = Command::new(sqf_bin());
        cmd.env("SQFFY_HOME", self.home.path())
            .env("NO_COLOR", "1")
            .env("SQFFY_LANG", "en")
            .env_remove("COLUMNS");
        cmd
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(args)
            .output()
            .expect("failed to spawn sqf")
    }

    pub fn run_ok(&self, args: &[&str]) -> Output {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "sqf {:?} failed (status {:?})\nstdout:\n{}\nstderr:\n{}",
            args,
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    pub fn stdout(out: &Output) -> String {
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn stderr(out: &Output) -> String {
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    pub fn combined(out: &Output) -> String {
        format!("{}{}", Self::stdout(out), Self::stderr(out))
    }

    /// One-shot against a SQLite file (created if missing by opening).
    pub fn sqlite_script(&self, db: &Path, script: &str) -> Output {
        self.run_ok(&[
            "--no-pager",
            "--no-save",
            "--lang",
            "en",
            db.to_str().expect("utf8 path"),
            "-c",
            script,
        ])
    }

    pub fn write_connection(
        &self,
        name: &str,
        driver: &str,
        target: &str,
        url: Option<&str>,
    ) {
        let path = self.home.path().join("connections.toml");
        // TOML strings need escaped backslashes (Windows paths).
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let url_line = match url {
            Some(u) => format!("url = \"{}\"\n", esc(u)),
            None => String::new(),
        };
        let body = format!(
            "[[connections]]\nid = \"{name}-e2e\"\nname = \"{name}\"\ndriver = \"{driver}\"\ntarget = \"{}\"\n{url_line}last_used = 1\n",
            esc(target),
        );
        std::fs::write(&path, body).expect("write connections.toml");
    }
}

pub fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected to find `{needle}` in:\n{haystack}"
    );
}

pub fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "did not expect `{needle}` in:\n{haystack}"
    );
}

/// Free localhost TCP port.
pub fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("addr").port()
}

pub fn wait_tcp(host: &str, port: u16, timeout: Duration) {
    let start = Instant::now();
    loop {
        if TcpStream::connect((host, port)).is_ok() {
            return;
        }
        if start.elapsed() > timeout {
            panic!("timeout waiting for {host}:{port}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn http_post_json(host: &str, port: u16, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect((host, port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read");
    let (head, rest) = buf.split_once("\r\n\r\n").unwrap_or((&buf[..], ""));
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, rest.to_string())
}

pub fn http_get(host: &str, port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect((host, port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read");
    let (head, rest) = buf.split_once("\r\n\r\n").unwrap_or((&buf[..], ""));
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, rest.to_string())
}

pub fn mcp_rpc(host: &str, port: u16, method: &str, params: Value, id: u64) -> Value {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    let (status, resp) = http_post_json(host, port, "/mcp", &body.to_string());
    assert_eq!(status, 200, "mcp http status {status}: {resp}");
    serde_json::from_str(&resp).unwrap_or_else(|e| panic!("bad json ({e}): {resp}"))
}

pub struct McpChild {
    pub child: Child,
    pub port: u16,
}

impl McpChild {
    pub fn spawn(env: &TestEnv) -> Self {
        let port = free_port();
        let log = env.home_path().join(format!("mcp-{port}.log"));
        let log_file = std::fs::File::create(&log).expect("mcp log");
        let mut child = env
            .command()
            .args(["mcp", "--port", &port.to_string()])
            .stdout(Stdio::from(log_file.try_clone().expect("clone log")))
            .stderr(Stdio::from(log_file))
            .spawn()
            .expect("spawn mcp");
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().ok().flatten() {
                let tail = std::fs::read_to_string(&log).unwrap_or_default();
                panic!("mcp exited early ({status}):\n{tail}");
            }
            if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let req = format!(
                    "GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
                );
                if stream.write_all(req.as_bytes()).is_ok() {
                    let mut buf = String::new();
                    if stream.read_to_string(&mut buf).is_ok() && buf.contains("ok") {
                        break;
                    }
                }
            }
            if start.elapsed() > Duration::from_secs(15) {
                let _ = child.kill();
                let tail = std::fs::read_to_string(&log).unwrap_or_default();
                panic!("mcp /health not ready on port {port}:\n{tail}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Self { child, port }
    }
}

impl Drop for McpChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Skip unless env var is set (server e2e).
pub fn require_env(key: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("skip: set {key} to run this test");
            panic!("ignored without {key}");
        }
    }
}

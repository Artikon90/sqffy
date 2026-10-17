# How to add a database driver

**English** · [Русский](./DRIVERS.ru.md) · [中文](./DRIVERS.zh.md)

The REPL, meta commands, renderer, and autocomplete stay backend-agnostic. A new database is one module, a Cargo feature, a registry line, and a URL scheme.

See also [ARCHITECTURE.md](../ARCHITECTURE.md).

## Contract

1. **`Database`** (`src/db/traits.rs`) — the open session:
   - `execute_sql`, `list_tables`, `describe_table`, `table_ddl`, `inspect_table`
   - `driver_id`, `display_label`, `capabilities`
   - `inspect_table` has a default that returns columns only. Override it when the engine can report constraints and indexes.
2. **`Driver`** (`src/db/driver.rs`) — the factory:
   - `id`, `aliases`, `description`
   - `open(&ConnectionSpec) -> Box<dyn Database>`
   - `provision()` — how to create one locally (file vs server, Docker, winget, download page)
   - `probe_local()` — whether a server is already listening
3. **Register** the driver in `default_registry()` (`src/db/registry.rs`).
4. **Parse** the URL scheme in `src/db/connect.rs` (`parse_url`). `--driver <id>` already overrides the scheme.
5. **Cargo feature** and optional dependency in `Cargo.toml`. Gate the module in `src/db/mod.rs`.

`ConnectionSpec` is parsed before any driver runs:

| Input | Result |
| --- | --- |
| `./file.db` | driver `sqlite`, target = path |
| `:memory:` | sqlite, in memory |
| `sqlite:./file.db`, `file:./file.db` | sqlite |
| `postgres://…`, `postgresql://…` | postgres |
| `mysql://…` | mysql |
| `mariadb://…`, `maria://…` | mariadb |
| `clickhouse://…`, `ch://…` | clickhouse |
| `oracle://…`, `oracledb://…`, `ora://…` | oracle |
| `--driver mysql host/db` | mysql + that target |

`--driver` aliases (`pg`, `ch`, `ora`, `maria`, `file`, …) are resolved by the registry, not by `parse_url`.

## Checklist

```text
[ ] src/db/<name>.rs — Driver + Database
[ ] Cargo.toml — feature + optional crate
[ ] src/db/mod.rs — #[cfg(feature = "…")] mod
[ ] registry.rs — register behind the same cfg
[ ] connect.rs — scheme in parse_url, if the engine has a URL
[ ] Capabilities { schemas, table_ddl, quote_char }
[ ] inspect_table — constraints and indexes, when the engine has them
[ ] provision() / probe_local() — even if both are "not supported"
[ ] Errors via Error::backend("name", …)
[ ] locales — only if you add user-facing strings; keep en.toml, ru.toml, zh.toml in parity
[ ] cargo test, plus a manual connect
[ ] README.md, README.ru.md, README.zh.md — one connection example
```

Do **not** import the driver crate from `app`, `meta`, `render`, `repl`, or `complete`. Those crates see `dyn Database` only.

## Status

| Driver | id | Aliases | Crate / wire | Feature | Schemas | CREATE text |
| --- | --- | --- | --- | --- | --- | --- |
| SQLite | `sqlite` | `file` | `rusqlite`, bundled | `sqlite` | no | yes |
| PostgreSQL | `postgres` | `postgresql`, `pg` | sync `postgres` | `postgres` | yes | yes |
| MySQL | `mysql` | — | sync `mysql` | `mysql` | yes | yes |
| MariaDB | `mariadb` | `maria` | MySQL wire (`mysql` module) + own provision | `mariadb` | yes | yes |
| ClickHouse | `clickhouse` | `ch`, `click` | HTTP via `ureq` | `clickhouse` | yes | yes |
| Oracle | `oracle` | `oracledb`, `ora` | `oracle` (ODPI-C, Instant Client) | `oracle` | yes | no |

All six are default features. MariaDB does not duplicate the session type: `MariadbDriver::open` calls `open_mysql_url` and only the provision recipe differs. `quote_char` is `"` for SQLite, PostgreSQL, and Oracle, and `` ` `` for MySQL, MariaDB, and ClickHouse.

## Pattern

```rust
pub struct FooDriver;

impl Driver for FooDriver {
    fn id(&self) -> &'static str { "foo" }
    fn aliases(&self) -> &'static [&'static str] { &["foo-db"] }
    fn description(&self) -> &'static str { "FooDB" }

    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        Ok(Box::new(FooDb::connect(spec)?))
    }

    fn provision(&self) -> ProvisionInfo {
        ProvisionInfo {
            create_kind: CreateKind::Server,
            default_port: Some(5432),
            docker: None,
            download_url: Some("https://example.com/foodb"),
            winget_id: None,
            help_key: "provision.help.foo",
        }
    }
}
```

`CreateKind::File` is the SQLite path (create a file, no server). `CreateKind::Server` goes through the picker: probe, then Docker, download, or a pasted URL.

## Capabilities

| Flag | Effect |
| --- | --- |
| `schemas` | The engine has namespaces. Meta and completion can show them. |
| `table_ddl` | When false, `.schema t` still shows columns and keys, and omits the `CREATE` script. Oracle does this. |
| `quote_char` | Identifier quote used by `.generate` when it builds `INSERT` SQL. |

## Sync

Drivers are synchronous (`Database: Send` only). Prefer a blocking client. If a client is async, block inside the driver so `App` stays sync. Do not put a runtime on the REPL path.

## Smoke test

```bash
cargo build --no-default-features --features sqlite,foo
sqf --list-drivers
sqf foo://USER:PASSWORD@127.0.0.1:1/app
```

An end-to-end test belongs in `tests/e2e_<name>.rs` with `required-features = ["e2e"]`, gated on `SQFFY_E2E_<NAME>`. Wire it into the `e2e-all` alias the way the existing server tests are wired.

# How to add a database driver

Glint keeps REPL / meta / render / autocomplete **backend-agnostic**.
A new database is one module + a registry line.

## Contract

1. **`Database`** (`src/db/traits.rs`) — open session:
   - `execute_sql`, `list_tables`, `describe_table`, `table_ddl`
   - `driver_id`, `display_label`, `capabilities`
2. **`Driver`** (`src/db/driver.rs`) — factory:
   - `id` / `aliases` / `description`
   - `open(&ConnectionSpec) -> Box<dyn Database>`
3. **Register** in `default_registry()` (`src/db/registry.rs`).
4. **Cargo feature** (optional dependency) in `Cargo.toml`.

`ConnectionSpec` is already parsed from:

| Input | Result |
| --- | --- |
| `./file.db` | driver=`sqlite`, target=path |
| `sqlite:./file.db` | sqlite |
| `postgres://…` | postgres |
| `--driver mysql host/db` | mysql + target |

## Checklist for a new backend

```text
[ ] src/db/<name>.rs — Driver + Database impl
[ ] Cargo.toml — feature + optional crate
[ ] registry.rs — #[cfg(feature = "…")] register
[ ] db/mod.rs — #[cfg] mod + re-exports if needed
[ ] Set Capabilities { schemas, table_ddl, quote_char }
[ ] Map native errors via Error::backend("name", …)
[ ] cargo test / manual connect smoke
[ ] README: connection example
```

## Status

| Driver | Crate | Feature |
| --- | --- | --- |
| SQLite | `rusqlite` (bundled) | `sqlite` (default) |
| PostgreSQL | `postgres` (sync) | `postgres` (default) |
| MySQL | `mysql` (sync) | `mysql` (default) |
| MariaDB | `mysql` (sync wire) | `mariadb` (default) |
| ClickHouse | HTTP via `ureq` | `clickhouse` (default) |
| Oracle | `oracle` (ODPI-C) | `oracle` (default; needs Instant Client) |

## Pattern for a new backend

```rust
pub struct FooDriver;

impl Driver for FooDriver {
    fn id(&self) -> &'static str { "foo" }
    fn aliases(&self) -> &'static [&'static str] { &["foo-db"] }
    fn description(&self) -> &'static str { "FooDB" }
    fn open(&self, spec: &ConnectionSpec) -> Result<Box<dyn Database>> {
        Ok(Box::new(FooDb::connect(spec)?))
    }
}
```

Do **not** import driver crates from `app`, `meta`, `render`, or `repl`.

## Capabilities

| Flag | Effect |
| --- | --- |
| `schemas` | Future: schema-aware `.tables` / complete |
| `table_ddl` | If false, `.schema t` tells user to use `.describe` |
| `quote_char` | Reserved for SQL helpers |

## Sync vs async

v1 drivers are **sync** (`Send` only). Prefer blocking clients (`rusqlite`, `postgres` crate).
If you need tokio, wrap with `block_on` inside the driver so `App` stays sync — or introduce an async App later without changing `render`/`meta`.

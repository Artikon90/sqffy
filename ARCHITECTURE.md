# SQffy architecture (`sqf`)

**English** · [Русский](./ARCHITECTURE.ru.md) · [中文](./ARCHITECTURE.zh.md)

SQffy is a Rust command-line SQL client. It is a REPL: a line in, a result out. It is not a TUI. The binary is `sqf`.

The shell stays fast because the hot path is synchronous, SQLite is bundled, and schema completion reads a cache instead of the database on every Tab. Output is rendered — typed tables, a framed SQL error, highlighted DDL — rather than dumped as raw text. A read-only MCP server can sit beside that session for an editor agent.

## Scope

| In this tree | Not in this tree |
| --- | --- |
| REPL: line → SQL or meta → stdout | Panels, focus, vim modes |
| Six engines behind one `Database` trait | Full psql `\` command parity |
| Syntax highlighting with an in-tree tokenizer | A separate GUI |
| Clause-aware autocomplete from a schema cache | Writes through the MCP server |
| Row limit and a small built-in pager | A full-screen pager (`q` / Esc) |
| Pluggable drivers (`Driver` + registry) | An async runtime on the query path |
| Saved connections and local provision | |
| `.generate` for test and demo rows | |
| UI catalogs: English, Russian, Chinese | |

## Flow

```text
clap (cli)
    │
    ├─ sqf mcp ─────────────▶ ConnectionPool ──▶ MCP guard ──▶ tools
    │
    └─ target or picker ──▶ DriverRegistry.open ──▶ Box<dyn Database>
                                      │
                                      ▼
                              App::run_repl (rustyline)
                                      │
                    ┌─────────────────┼──────────────────┐
                    ▼                 ▼                  ▼
                  meta           execute_sql          complete
                    │                 │                  │
                    └──────── render ◀┘            SchemaCache
                              │                    + clause scan
                              ▼
                         stdout / pager

sql::highlight colors the input line and DDL
generate builds rows and INSERTs through Database
provision (Docker / winget) runs from the picker, before open
```

The interactive shell holds one `Box<dyn Database>`. The registry picks the concrete backend from the URL or `--driver`. Render never opens a connection. Meta never dumps the whole database as one column.

When MCP is started (`sqf mcp`, or `--mcp-port` beside the prompt), a `ConnectionPool` can hold several saved connections. The shell and the pool are separate doors: the shell runs writes, the pool's tools do not.

## Modules

```text
src/
  main.rs            process entry
  lib.rs             crate root
  cli.rs             target, --driver, --lang, --limit, -c, --mcp-port, mcp subcommand
  config.rs          session settings (row limit, pager)
  i18n.rs            t/tr, embedded locales/en.toml|ru.toml|zh.toml
  connections.rs     ConnectionStore → connections.toml
  picker.rs          connect / add / create, including provision
  app.rs             picker or CLI target → REPL, optional MCP beside it
  error.rs           Backend { driver, message } — no rusqlite in the crate root
  pool.rs            named connections for the MCP server
  db/                Driver, Database, registry, one module per engine
  meta/              dot-commands
  render/            tables, schema view, DDL color, errors, pager
  complete/          schema cache, clause scan, candidates
  repl/              rustyline, highlighter, completer
  sql/               tokenizer, keyword lists, highlighter
  generate/          preview and batched INSERT
  provision/         Docker pull/run, winget install
  mcp/               stdio and HTTP, guard, tools
  ui/                color, frames, picker widgets, progress
locales/
  en.toml ru.toml zh.toml
docs/
  DRIVERS.md         how to add an engine
  I18N.md            how to add a language or a string
```

### Startup and connections

1. `sqf` with no target opens the picker: a saved database, add an existing one, or create one.
2. A choice calls `DriverRegistry::open`, then stores `last_used` in `connections.toml`.
3. `sqf ./file.db` connects directly and records the connection, unless `--no-save` or `-c` is set.
4. Creating a server database probes localhost. If nothing is listening, the picker offers Docker, a download (winget on Windows, otherwise the official page), or a URL the user already has.

Config path comes from `directories` (`dev.SQffy.sqffy`): `~/.config/sqffy` on Linux, `~/Library/Application Support/dev.SQffy.sqffy` on macOS, `%APPDATA%\SQffy\sqffy\config` on Windows. `SQFFY_HOME` replaces that root for tests.

### i18n

Every user-facing string goes through `i18n::t` / `tr`. Resolution: `--lang`, then `SQFFY_LANG`, then `LC_ALL`, `LC_MESSAGES`, `LANG`, then `en`. Details: [`docs/I18N.md`](./docs/I18N.md).

### `db` — engines

Three roles:

| Type | Role |
| --- | --- |
| `ConnectionSpec` | What to open: driver id, target, optional original URL |
| `Driver` | Factory: `open(spec) → Box<dyn Database>`, plus `provision` and `probe_local` |
| `Database` | The session: SQL and introspection |

`Capabilities` tells meta whether the engine has schemas and whether `.schema` can show `CREATE` text. Errors are `Error::backend("sqlite", …)`, so the crate root does not depend on a driver crate.

| Engine | Feature | Session | Schemas | `CREATE` text | Quote |
| --- | --- | --- | --- | --- | --- |
| SQLite | `sqlite` | bundled `rusqlite` | no | yes | `"` |
| PostgreSQL | `postgres` | sync `postgres` | yes | yes | `"` |
| MySQL | `mysql` | sync `mysql` | yes | yes | `` ` `` |
| MariaDB | `mariadb` | MySQL wire, own provision recipe | yes | yes | `` ` `` |
| ClickHouse | `clickhouse` | HTTP via `ureq` | yes | yes | `` ` `` |
| Oracle | `oracle` | `oracle` crate, Instant Client | yes | no | `"` |

All six features are on by default. A new engine is a module plus a registry line: [`docs/DRIVERS.md`](./docs/DRIVERS.md).

URL schemes parsed in `db/connect.rs`: `sqlite:` / `file:`, `postgres:` / `postgresql:`, `mysql:`, `mariadb:` / `maria:`, `clickhouse:` / `ch:`, `oracle:` / `oracledb:` / `ora:`. A bare path is SQLite. `:memory:` is SQLite in memory.

### `meta`

| Command | Behavior |
| --- | --- |
| `.tables` (`.t`) | Tables and views |
| `.schema` (`.s`) | Name index, not a dump of the database |
| `.schema t` | One table: columns, constraints, indexes, DDL when the engine returns it |
| `.describe t` (`.d`) | Column list: type, null, default, pk, comment |
| `.generate` (`.gen`) | Preview, then batched inserts. 100 rows or more require `--yes` |
| `.help` (`.h`, `?`) | Command list |
| `.quit` (`.q`, `.exit`) | Leave |

### `render`

- `QueryResult` → `comfy-table`, truncated cells, `NULL` dimmed, the declared type under each column name.
- DDL and the input line → `sql::highlight` (`owo-colors`), not `syntect`.
- SQL errors → a closed frame, with a caret when the backend reports a position.
- `Rendered { text, tall }` → `page_if_needed` when the pager is on.

### `complete`

1. `SchemaCache::refresh` at connect and after DDL.
2. `complete::context` scans the unfinished statement (clause, tables in scope). It is a tolerant scanner, not a full SQL parser.
3. `Completer::candidates` ranks keywords, tables, columns, types, and meta commands for that clause.

### `repl`

- `rustyline`, not a TUI event loop.
- The helper highlights the buffer and completes on Tab.
- Ctrl+C interrupts the line and does not exit.
- Leave with `.quit` or EOF (Ctrl+D).

### `mcp`

`mcp/guard.rs` allows a statement only when it starts with `SELECT`, `WITH`, `SHOW`, `EXPLAIN`, `DESCRIBE`, `DESC`, or `PRAGMA`, after leading comments are stripped. Everything else is refused before `execute_sql`.

Tools: `list_connections`, `list_tables`, `describe_table`, `inspect_table`, `execute_readonly`, `list_drivers`.

Transports: newline JSON-RPC on stdio (`sqf mcp`), or HTTP on `127.0.0.1` (`sqf mcp --port`, or `--mcp-port` beside the REPL). `GET /health` on the HTTP port.

### `generate`

`.generate` builds a plan from `inspect_table`, fills foreign-key pools from the referenced tables, prints a preview, then inserts in batches. `--dry-run` stops after the preview. Without `--yes`, a count of 100 or more is refused. Values come from the `fake` crate.

### `provision`

Each `Driver` returns a `ProvisionInfo`: file or server, default port, optional Docker spec, optional winget id, optional download URL. The picker uses `probe_local` first. Docker pull/run shows a progress bar. Winget is Windows-only; other platforms get the download URL.

## Performance

1. **Synchronous drivers.** No async runtime on the query path. `Database: Send`.
2. **Default row limit 100** (`--limit`, `0` = unlimited) caps allocation and printing.
3. **Schema cache** so Tab does not re-introspect on every keystroke.
4. **Release profile:** LTO, `codegen-units = 1`, `strip`, `panic = abort`.
5. **Bundled SQLite** so the binary does not depend on a system `libsqlite`.

Still open, and not pretended otherwise:

- streaming into the pager without holding every row;
- a real pager (`q` / Esc / Ctrl+C) — today a tall result is printed and the pager waits for Enter;
- reusing cell buffers across renders.

## Pager

Short output goes straight to stdout. Tall output goes through `render/pager.rs`, which prints the text and waits for Enter. `--no-pager` skips that wait.

## Code rules

1. **CLI, not TUI.** Print, then return the terminal.
2. **Module boundaries.** `app`, `meta`, `render`, `repl`, and `complete` depend on `dyn Database`. They do not import driver crates.
3. **A new database is a new `Driver`.** The REPL does not grow a match arm per engine.
4. **Schema is structured.** Columns, constraints, and indexes matter more than a raw `.schema` dump.
5. **Features cut dependencies.** `cargo build --no-default-features --features sqlite` is a SQLite-only binary.
6. **User-facing text is a locale key.** English, Russian, and Chinese stay in parity.

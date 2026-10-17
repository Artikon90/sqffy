<div align="center">

# SQffy

### A simple CLI for SQL databases, in the terminal you already have.

Windows · Linux · macOS

<p>
  <a href="#databases"><img alt="SQLite" src="https://img.shields.io/badge/SQLite-003B57?style=flat-square&logo=sqlite&logoColor=white"></a>
  &nbsp;
  <a href="#databases"><img alt="PostgreSQL" src="https://img.shields.io/badge/PostgreSQL-4169E1?style=flat-square&logo=postgresql&logoColor=white"></a>
  &nbsp;
  <a href="#databases"><img alt="MySQL" src="https://img.shields.io/badge/MySQL-4479A1?style=flat-square&logo=mysql&logoColor=white"></a>
  &nbsp;
  <a href="#databases"><img alt="MariaDB" src="https://img.shields.io/badge/MariaDB-C32136?style=flat-square&logo=mariadb&logoColor=white"></a>
  &nbsp;
  <a href="#databases"><img alt="ClickHouse" src="https://img.shields.io/badge/ClickHouse-FFCC01?style=flat-square&logo=clickhouse&logoColor=black"></a>
  &nbsp;
  <a href="#databases"><img alt="Oracle" src="https://img.shields.io/badge/Oracle-F80000?style=flat-square&logo=oracle&logoColor=white"></a>
</p>

**English** · [Русский](./README.ru.md) · [中文](./README.zh.md)

<br>

[Fast](#fast-at-the-keyboard) ·
[Tables](#tables-that-are-easy-to-read) ·
[Agents](#a-safe-mcp-server-for-ai-agents) ·
[Start](#an-easy-start) ·
[Install](#install) ·
[Reference](#reference)

</div>

SQffy is a simple command-line utility for working with SQL databases from the terminal. Connect, write a query, read the answer.

SQLite, PostgreSQL, MySQL, MariaDB, ClickHouse, and Oracle are built in. Any other database can be added as an adapter: same shell, same commands. The checklist is in [docs/DRIVERS.md](./docs/DRIVERS.md).

---

## Fast at the keyboard

SQffy is written in Rust, and the shell stays quick. Syntax highlighting paints the query while you type. Autocomplete brings tables and columns onto the line, so you spend the time on the data, not on remembering names.

<p align="center">
  <img src="./docs/media/autocomplete.png" width="361" alt="Autocomplete at the prod.db prompt, offering documents, files, and users while a SELECT is typed">
</p>

After `FROM`, the list is tables. After the table name, it is columns.

---

## Tables that are easy to read

Schema and query results are rendered, not dumped as raw text. The type sits under the column name — the type the engine reported. `.schema` puts columns, keys, indexes, and the DDL on one screen.

<p align="center">
  <img src="./docs/media/results.png" width="426" alt="SELECT result with INTEGER and TEXT printed under the column names">
</p>

<p align="center">
  <img src="./docs/media/schema.png" width="662" alt="Schema view of users: columns, primary key, indexes, and a highlighted CREATE TABLE">
</p>

Need rows for a test or a demo? `.generate` fills a table: data generation for testing and visualization. A preview prints first. `--yes` inserts. A hundred rows or more without `--yes` is refused.

---

## Any terminal

Windows, Linux, and macOS. A normal terminal is the whole interface — the one you already have open.

---

## A safe MCP server for AI agents

Modern work often includes an agent in the editor. SQffy speaks MCP, and that server is the safe door.

The agent can inspect the schema and run a read: `SELECT`, `WITH`, `SHOW`, `EXPLAIN`, `PRAGMA`. A dangerous statement does not reach the database. `INSERT`, `UPDATE`, `DELETE`, `DROP`, and the rest of the writes are refused on this server only. At the prompt, the same statements run as you write them. HTTP listens on `127.0.0.1`.

```bash
sqf mcp                      # stdio, for an editor or agent
sqf mcp --port 8787          # http://127.0.0.1:8787/mcp   ·   GET /health
sqf --mcp-port 8787 ./app.db # the same server, beside the prompt
```

---

## An easy start

A database comes up in a couple of steps, even when the engine is not installed yet. Run `sqf` and choose **create**.

If nothing is listening locally, SQffy offers to pull a Docker image and run it, or to install the package for you (winget on Windows). When neither path fits, it opens the official download page. You do not have to arrive with Postgres already on the machine.

The picker has three moves:

1. Connect to a saved database
2. Add one you already have
3. Create a new one — a file, or a server

```bash
sqf                              # picker
sqf ./app.db                     # straight to a SQLite file
sqf postgres://USER:PASSWORD@localhost:5432/app
```

```text
.tables
.schema users
SELECT id, name FROM users WHERE id >= 5;
.generate users 1000 --dry-run
```

Connections are remembered in `connections.toml`. `--no-save` leaves the file alone.

| OS | Path |
| :--- | :--- |
| Linux | `~/.config/sqffy/connections.toml` |
| macOS | `~/Library/Application Support/dev.SQffy.sqffy/connections.toml` |
| Windows | `%APPDATA%\SQffy\sqffy\config\connections.toml` |

Tests can point the catalog elsewhere with `SQFFY_HOME`.

---

## Install

Rust 1.85 or newer. From a checkout of this repository:

```bash
cargo install --path .

# or run it without installing
cargo run --release --bin sqf
```

SQLite ships inside the binary. Oracle needs [Instant Client](https://www.oracle.com/database/technologies/instant-client.html) on the machine.

```bash
sqf --lang ru                   # en · ru · zh
SQFFY_LANG=zh sqf
```

Language resolution: `--lang`, then `SQFFY_LANG`, then the system locale, then English. Strings live in [docs/I18N.md](./docs/I18N.md).

<details>
<summary><b>Feature flags</b></summary>

| Feature | Default | |
| :--- | :---: | :--- |
| `sqlite` | yes | Bundled SQLite |
| `postgres` | yes | PostgreSQL |
| `mysql` | yes | MySQL |
| `mariadb` | yes | MariaDB, including local provisioning |
| `clickhouse` | yes | ClickHouse over HTTP |
| `oracle` | yes | Oracle, via Instant Client |
| `e2e` | no | Only for `cargo e2e` |

```bash
cargo build --release --no-default-features --features sqlite
```

</details>

---

## Reference

### Meta commands

| Command | |
| :--- | :--- |
| `.help` &nbsp; `.h` | Command list |
| `.tables` &nbsp; `.t` | Tables and views |
| `.schema` `[table]` | Index of names, or one table in full |
| `.describe` `<table>` | Columns |
| `.generate` `<table> <n> [--yes] [--dry-run] [col=ref.col …]` | Data generation for testing and visualization |
| `.quit` &nbsp; `.q` | Leave |

### Flags

| Flag | |
| :--- | :--- |
| `-c`, `--command` | Run SQL or a meta command and exit |
| `-d`, `--driver` | Force a driver id |
| `--limit N` | Rows to print. `0` prints them all. Default is 100. |
| `--no-pager` | Disable the pager |
| `--no-save` | Do not write this connection into the catalog |
| `--mcp-port PORT` | Safe MCP HTTP beside the session |
| `--lang en\|ru\|zh` | UI language |
| `--list-drivers` | Print built-in drivers and exit |

### Databases

| Engine | Open | Note |
| :--- | :--- | :--- |
| SQLite | `sqf ./app.db` · `sqf sqlite:./app.db` · `sqf :memory:` | Bundled. A new file is just a path. |
| PostgreSQL | `postgres://USER:PASSWORD@localhost:5432/app` | Docker or installer from the picker |
| MySQL | `mysql://USER:PASSWORD@localhost:3306/app` | Docker or installer from the picker |
| MariaDB | `mariadb://USER:PASSWORD@localhost:3306/app` | Docker or installer from the picker |
| ClickHouse | `clickhouse://USER:PASSWORD@localhost:8123/default` | HTTP. Docker or installer from the picker |
| Oracle | `oracle://USER:PASSWORD@localhost:1521/XEPDB1` | Needs Instant Client |

### MCP tools

| Tool | |
| :--- | :--- |
| `list_connections` | Connections open in this session |
| `list_tables` | Tables and views |
| `describe_table` | Columns |
| `inspect_table` | Columns, constraints, indexes |
| `execute_readonly` | A read query. Writes are refused. |
| `list_drivers` | Drivers compiled into this build |

---

## Docs

| | |
| :--- | :--- |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | How the crate is put together |
| [docs/DRIVERS.md](./docs/DRIVERS.md) | Add a database adapter |
| [docs/I18N.md](./docs/I18N.md) | Add a language |

<details>
<summary><b>Develop</b></summary>

```bash
cargo test       # unit tests
cargo e2e        # CLI end-to-end: SQLite, MCP, meta commands
cargo e2e-all    # plus server adapters — set SQFFY_E2E_* first
```

| Variable | Example |
| :--- | :--- |
| `SQFFY_E2E_POSTGRES` | `postgres://sqffy:sqffy@127.0.0.1:5432/sqffy` |
| `SQFFY_E2E_MYSQL` | `mysql://sqffy:sqffy@127.0.0.1:3306/sqffy` |
| `SQFFY_E2E_MARIADB` | `mariadb://sqffy:sqffy@127.0.0.1:3307/sqffy` |
| `SQFFY_E2E_CLICKHOUSE` | `clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy` |
| `SQFFY_E2E_ORACLE` | `oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1` |

Those URLs belong to the Docker test stack.

```bash
docker compose -f e2e/docker-compose.yml up --build --abort-on-container-exit
```

CI runs **Test**, **Build**, and **E2E** as separate jobs. Workflow: [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

```text
sqffy/
├── src/        CLI, drivers, REPL, MCP, generate, provision
├── locales/    en.toml · ru.toml · zh.toml
├── docs/       drivers, i18n, screenshots
├── tests/      e2e_*  (feature = e2e)
├── e2e/        Docker Compose
└── .github/    CI
```

</details>

---

<div align="center">

<a href="https://www.rust-lang.org/"><img alt="Rust 1.85+" src="https://img.shields.io/badge/Rust-1.85+-DEA584?style=flat-square&logo=rust&logoColor=white"></a>
&nbsp;
<a href="./LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-22C55E?style=flat-square"></a>

<br><br>

**[↑ Top](#sqffy)**

</div>

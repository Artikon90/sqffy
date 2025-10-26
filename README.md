<div align="center">

# ✨ SQffy

### Fast SQL CLI with saved connections — `sqf`

[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-2088FF?style=for-the-badge&logo=githubactions&logoColor=white)](.github/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-22C55E?style=for-the-badge)](./LICENSE)
[![SQLite](https://img.shields.io/badge/SQLite-bundled-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](#-databases)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-ready-4169E1?style=for-the-badge&logo=postgresql&logoColor=white)](#-databases)
[![MySQL](https://img.shields.io/badge/MySQL-ready-4479A1?style=for-the-badge&logo=mysql&logoColor=white)](#-databases)

**English** · [Русский](./README.ru.md)

<br/>

```text
   ____   ___   __  __
  / __/  / _ \ / /_/ /
 _\ \   / ___// __  /
/___/  /_/    \/_/_/    ·  sqf
```

A polished terminal SQL client for humans: pretty tables, rich schemas,
smart completion, synthetic data, and a read-only MCP server — all in one binary.

<br/>

[Features](#-features) ·
[Install](#-install) ·
[Quick start](#-quick-start) ·
[Usage](#-usage) ·
[MCP](#-mcp) ·
[Tests](#-tests--ci) ·
[Docs](#-docs)

</div>

---

## ✨ Features

| | Capability | What you get |
| :--- | :--- | :--- |
| 🗄️ | **Multi-database** | SQLite · PostgreSQL · MySQL / MariaDB |
| 🧭 | **Connection picker** | Saved catalog, add existing, create new |
| 🐳 | **Local provision** | Docker pull/run or winget download with a progress bar |
| 🎨 | **Readable output** | Typed columns, framed SQL errors, compact brand header |
| ⌨️ | **Smart autocomplete** | Clause-aware suggestions (tables, columns, meta) |
| 🧪 | **`.generate`** | Synthetic rows with preview + batched inserts + FK pools |
| 🔌 | **MCP** | Read-only tools over stdio or HTTP (`/mcp`) |
| 🌍 | **i18n** | English & Russian UI (`--lang` / `SQFFY_LANG`) |
| ⚡ | **One-shot mode** | `-c` scripts for CI, scripting, and e2e |

---

## 📦 Install

```bash
# from source
git clone <your-repo-url> sqffy
cd sqffy
cargo install --path .

# or just run
cargo run --release --bin sqf
```

<details>
<summary><b>🦀 Feature flags</b></summary>

| Feature | Default | Notes |
| :--- | :---: | :--- |
| `sqlite` | ✅ | Bundled `rusqlite` |
| `postgres` | ✅ | Sync `postgres` crate |
| `mysql` | ✅ | Sync `mysql` crate |
| `e2e` | ❌ | Only for `cargo e2e` |

```bash
# SQLite-only build
cargo build --release --no-default-features --features sqlite
```

</details>

---

## 🚀 Quick start

```bash
sqf                          # interactive picker
sqf --lang ru                # Russian UI
sqf ./path/to/db.sqlite      # open SQLite file
sqf postgres://u:p@localhost:5432/app
sqf mysql://u:p@localhost:3306/app
sqf --list-drivers
sqf --no-pager --no-save :memory: -c "SELECT 1 AS ok;"
```

<div align="center">

| Without a target | SQffy opens a picker |
| :---: | :--- |
| **1** | Connect to a saved database |
| **2** | Add an existing database |
| **3** | Create a new database (file or provision a server) |

Connections live in the OS config dir as `connections.toml`  
*(override for tests: `SQFFY_HOME`)*

</div>

---

## 📖 Usage

### Meta commands

| Command | Description |
| :--- | :--- |
| `.help` / `.h` | Show help |
| `.tables` / `.t` | List tables & views |
| `.schema` `[table]` | Schema index or full table inspect |
| `.describe` `<table>` | Column list |
| `.generate` `<table> <n> [--yes] [--dry-run] [col=ref.col …]` | Fill with fake data |
| `.quit` / `.q` | Exit |

### Useful flags

| Flag | Meaning |
| :--- | :--- |
| `-c` / `--command` | Run SQL / meta and exit |
| `-d` / `--driver` | Force driver id |
| `--limit N` | Cap printed rows (`0` = unlimited) |
| `--no-pager` | Disable pager |
| `--no-save` | Don’t touch the connection catalog |
| `--mcp-port PORT` | MCP HTTP beside the REPL |
| `--lang en\|ru` | UI language |

---

## 🗄️ Databases

```bash
# SQLite
sqf ./app.db
sqf sqlite:./app.db
sqf :memory:

# PostgreSQL
sqf postgres://sqffy:sqffy@127.0.0.1:5432/sqffy

# MySQL
sqf mysql://sqffy:sqffy@127.0.0.1:3306/sqffy
```

When creating a **server** DB and nothing is listening locally, SQffy can:

1. 🐳 **Start it in Docker** (pull + run, animated progress)
2. ⬇️ **Download / install** (winget on Windows, or open the official page)
3. 🔗 **Use a URL you already have**

See [docs/DRIVERS.md](./docs/DRIVERS.md) to add another backend.

---

## 🔌 MCP

Read-only tools for agents and IDE bridges.

```bash
# stdio (default)
sqf mcp

# HTTP JSON-RPC
sqf mcp --port 8787
# → http://127.0.0.1:8787/mcp
# → GET /health
```

| Tool | Purpose |
| :--- | :--- |
| `list_connections` | Open connections in the session |
| `list_tables` | Tables / views |
| `describe_table` | Columns |
| `inspect_table` | Columns + constraints + indexes |
| `execute_readonly` | `SELECT` / `WITH` / `SHOW` / `EXPLAIN` / `PRAGMA` … |
| `list_drivers` | Built-in drivers |

`DELETE` / `INSERT` / `DROP` and friends are rejected by the guard.

---

## 🌍 Localization

```bash
sqf --lang ru
SQFFY_LANG=ru sqf
```

Resolution order: `--lang` → `SQFFY_LANG` → system locale → `en`.

Full guide: [docs/I18N.md](./docs/I18N.md).

---

## 🧪 Tests & CI

```bash
cargo test          # unit tests only
cargo e2e           # CLI e2e (SQLite / MCP / meta)
cargo e2e-all       # + Postgres & MySQL (needs env)
```

| Variable | Example |
| :--- | :--- |
| `SQFFY_E2E_POSTGRES` | `postgres://sqffy:sqffy@127.0.0.1:5432/sqffy` |
| `SQFFY_E2E_MYSQL` | `mysql://sqffy:sqffy@127.0.0.1:3306/sqffy` |

Docker stack (Linux):

```bash
docker compose -f e2e/docker-compose.yml up --build --abort-on-container-exit
```

CI jobs (independent): **Test** · **Build** · **E2E** — see [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

---

## 📚 Docs

| Document | Topic |
| :--- | :--- |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | How the crate is layered |
| [docs/DRIVERS.md](./docs/DRIVERS.md) | Adding a database driver |
| [docs/I18N.md](./docs/I18N.md) | Localization catalog |

---

## 🗺️ Project layout

```text
sqffy/
├── src/           # CLI, drivers, REPL, MCP, generate, provision
├── locales/       # en.toml · ru.toml
├── tests/         # e2e_* (feature = e2e)
├── e2e/           # Docker Compose + Linux runner
└── .github/       # CI: test · build · e2e
```

---

<div align="center">

### Made for the terminal · written in Rust · MIT licensed

**[⬆ back to top](#-sqffy)**

</div>

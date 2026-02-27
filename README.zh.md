<div align="center">

# ✨ SQffy

### 带已保存连接的快速 SQL CLI — `sqf`

[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-2088FF?style=for-the-badge&logo=githubactions&logoColor=white)](.github/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-22C55E?style=for-the-badge)](./LICENSE)
[![SQLite](https://img.shields.io/badge/SQLite-bundled-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](#-数据库)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-ready-4169E1?style=for-the-badge&logo=postgresql&logoColor=white)](#-数据库)
[![MySQL](https://img.shields.io/badge/MySQL-ready-4479A1?style=for-the-badge&logo=mysql&logoColor=white)](#-数据库)
[![MariaDB](https://img.shields.io/badge/MariaDB-ready-C32136?style=for-the-badge&logo=mariadb&logoColor=white)](#-数据库)
[![ClickHouse](https://img.shields.io/badge/ClickHouse-ready-FFCC01?style=for-the-badge&logo=clickhouse&logoColor=black)](#-数据库)
[![Oracle](https://img.shields.io/badge/Oracle-ready-F80000?style=for-the-badge&logo=oracle&logoColor=white)](#-数据库)

[English](./README.md) · [Русский](./README.ru.md) · **中文**

<br/>

```text
   ____   ___   __  __
  / __/  / _ \ / /_/ /
 _\ \   / ___// __  /
/___/  /_/    \/_/_/    ·  sqf
```

面向人的终端 SQL 客户端：美观表格、丰富的 schema、智能补全、
合成数据与只读 MCP 服务 — 全部打进一个二进制。

<br/>

[功能](#-功能) ·
[安装](#-安装) ·
[快速开始](#-快速开始) ·
[用法](#-用法) ·
[MCP](#-mcp) ·
[测试](#-测试与-ci) ·
[文档](#-文档)

</div>

---

## ✨ 功能

| | 能力 | 说明 |
| :--- | :--- | :--- |
| 🗄️ | **多数据库** | SQLite · PostgreSQL · MySQL · MariaDB · ClickHouse · Oracle |
| 🧭 | **连接选择器** | 已保存目录、添加现有库、创建新库 |
| 🐳 | **本地准备** | Docker pull/run 或 winget 下载（带进度条） |
| 🎨 | **可读输出** | 列类型、框式 SQL 错误、紧凑品牌头 |
| ⌨️ | **智能补全** | 按子句提示（表、列、元命令） |
| 🧪 | **`.generate`** | 合成行：预览 + 批量插入 + 外键池 |
| 🔌 | **MCP** | 只读工具，stdio 或 HTTP（`/mcp`） |
| 🌍 | **i18n** | 英语 · 俄语 · 中文（`--lang` / `SQFFY_LANG`） |
| ⚡ | **一次性模式** | `-c` 脚本，适合 CI 与 e2e |

---

## 📦 安装

```bash
git clone <your-repo-url> sqffy
cd sqffy
cargo install --path .

cargo run --release --bin sqf
```

<details>
<summary><b>🦀 Feature 开关</b></summary>

| Feature | 默认 | 说明 |
| :--- | :---: | :--- |
| `sqlite` | ✅ | 内置 `rusqlite` |
| `postgres` | ✅ | 同步 `postgres` |
| `mysql` | ✅ | 同步 `mysql` |
| `mariadb` | ✅ | MySQL 协议 + MariaDB 配方 |
| `clickhouse` | ✅ | HTTP（`ureq`） |
| `oracle` | ✅ | `oracle` 库（需 Instant Client） |
| `e2e` | ❌ | 仅用于 `cargo e2e` |

</details>

---

## 🚀 快速开始

```bash
sqf                          # 交互选择器
sqf --lang zh                # 中文界面
sqf ./path/to/db.sqlite
sqf postgres://u:p@localhost:5432/app
sqf mysql://u:p@localhost:3306/app
sqf mariadb://u:p@localhost:3306/app
sqf clickhouse://u:p@localhost:8123/default
sqf oracle://u:p@localhost:1521/XEPDB1
sqf --list-drivers
sqf --no-pager --no-save :memory: -c "SELECT 1 AS ok;"
```

无目标参数时会打开选择器：连接已保存库 / 添加现有库 / 创建新库。  
连接保存在系统配置目录的 `connections.toml`（测试可用 `SQFFY_HOME`）。

---

## 📖 用法

| 命令 | 说明 |
| :--- | :--- |
| `.help` | 帮助 |
| `.tables` | 表与视图 |
| `.schema` `[table]` | schema 索引或完整检查 |
| `.describe` `<table>` | 列信息 |
| `.generate` `<table> <n> …` | 生成假数据 |
| `.quit` | 退出 |

常用参数：`-c`、`-d`、`--limit`、`--no-pager`、`--no-save`、`--mcp-port`、`--lang en|ru|zh`。

---

## 🗄️ 数据库

```bash
sqf ./app.db
sqf postgres://sqffy:sqffy@127.0.0.1:5432/sqffy
sqf mysql://sqffy:sqffy@127.0.0.1:3306/sqffy
sqf mariadb://sqffy:sqffy@127.0.0.1:3306/sqffy
sqf clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy
sqf oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1
```

创建**服务器型**数据库且本机无服务时，可选择 Docker、下载安装，或手动输入 URL。  
扩展驱动见 [docs/DRIVERS.md](./docs/DRIVERS.md)。

---

## 🔌 MCP

```bash
sqf mcp
sqf mcp --port 8787   # http://127.0.0.1:8787/mcp
```

工具：`list_connections`、`list_tables`、`describe_table`、`inspect_table`、`execute_readonly`、`list_drivers`。  
`DELETE` / `INSERT` / `DROP` 等会被拒绝。

---

## 🌍 本地化

```bash
sqf --lang zh
SQFFY_LANG=zh sqf
```

优先级：`--lang` → `SQFFY_LANG` → 系统区域 → `en`。详见 [docs/I18N.md](./docs/I18N.md)。

---

## 🧪 测试与 CI

```bash
cargo test
cargo e2e
cargo e2e-all
```

| 变量 | 示例 |
| :--- | :--- |
| `SQFFY_E2E_POSTGRES` | `postgres://sqffy:sqffy@127.0.0.1:5432/sqffy` |
| `SQFFY_E2E_MYSQL` | `mysql://sqffy:sqffy@127.0.0.1:3306/sqffy` |
| `SQFFY_E2E_MARIADB` | `mariadb://sqffy:sqffy@127.0.0.1:3307/sqffy` |
| `SQFFY_E2E_CLICKHOUSE` | `clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy` |
| `SQFFY_E2E_ORACLE` | `oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1` |

```bash
docker compose -f e2e/docker-compose.yml up --build --abort-on-container-exit
```

CI 独立任务：**Test** · **Build** · **E2E**。

---

## 📚 文档

| 文档 | 主题 |
| :--- | :--- |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 分层结构 |
| [docs/DRIVERS.md](./docs/DRIVERS.md) | 新增驱动 |
| [docs/I18N.md](./docs/I18N.md) | 本地化 |

---

<div align="center">

### 为终端而生 · 用 Rust 编写 · MIT 许可

**[⬆ 回到顶部](#-sqffy)**

</div>

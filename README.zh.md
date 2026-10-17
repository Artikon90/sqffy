<div align="center">

# SQffy

### 一个简单的 SQL CLI。就在你已经打开的终端里。

Windows · Linux · macOS

<p>
  <a href="#数据库"><img alt="SQLite" src="https://img.shields.io/badge/SQLite-003B57?style=flat-square&logo=sqlite&logoColor=white"></a>
  &nbsp;
  <a href="#数据库"><img alt="PostgreSQL" src="https://img.shields.io/badge/PostgreSQL-4169E1?style=flat-square&logo=postgresql&logoColor=white"></a>
  &nbsp;
  <a href="#数据库"><img alt="MySQL" src="https://img.shields.io/badge/MySQL-4479A1?style=flat-square&logo=mysql&logoColor=white"></a>
  &nbsp;
  <a href="#数据库"><img alt="MariaDB" src="https://img.shields.io/badge/MariaDB-C32136?style=flat-square&logo=mariadb&logoColor=white"></a>
  &nbsp;
  <a href="#数据库"><img alt="ClickHouse" src="https://img.shields.io/badge/ClickHouse-FFCC01?style=flat-square&logo=clickhouse&logoColor=black"></a>
  &nbsp;
  <a href="#数据库"><img alt="Oracle" src="https://img.shields.io/badge/Oracle-F80000?style=flat-square&logo=oracle&logoColor=white"></a>
</p>

[English](./README.md) · [Русский](./README.ru.md) · **中文**

<br>

[速度](#写得快) ·
[表](#看得清的表) ·
[代理](#给-ai-代理的安全-mcp) ·
[开始](#轻松开始) ·
[安装](#安装) ·
[参考](#参考)

</div>

SQffy 是一个简单的命令行工具，用来在终端里操作 SQL 数据库。连上，写下查询，读到结果。

SQLite、PostgreSQL、MySQL、MariaDB、ClickHouse 和 Oracle 已经内置。其他任何数据库都可以写成适配器接进来：同一个 shell，同一套命令。步骤见 [docs/DRIVERS.md](./docs/DRIVERS.md)。

---

## 写得快

SQffy 用 Rust 写成，shell 保持轻快。语法高亮在你输入时就把查询画出来。自动补全把表和列带到当前行，时间花在数据上，而不是花在回忆名字上。

<p align="center">
  <img src="./docs/media/autocomplete.png" width="361" alt="prod.db 提示符下的自动补全：输入 SELECT 时列出 documents、files 和 users">
</p>

`FROM` 后面是表，表名后面是列。

---

## 看得清的表

库的 schema 和查询结果是渲染出来的，不是一堆原始文本。类型写在列名下面——引擎报告的那个类型。`.schema` 把列、键、索引和 DDL 放在同一屏。

<p align="center">
  <img src="./docs/media/results.png" width="426" alt="SELECT 结果：列名下方印着 INTEGER 和 TEXT">
</p>

<p align="center">
  <img src="./docs/media/schema.png" width="662" alt="users 的 schema 视图：列、主键、索引，以及高亮的 CREATE TABLE">
</p>

需要测试或演示用的行？`.generate` 会填满一张表：生成用于测试和可视化的数据。先打印预览。`--yes` 执行插入。一百行及以上如果没有 `--yes`，会被拒绝。

---

## 任何终端

Windows、Linux 和 macOS。界面就是普通终端，你已经打开的那个。不需要再装一个带窗口的程序。

---

## 给 AI 代理的安全 MCP

现在的开发常常在编辑器里放着一个代理。SQffy 提供 MCP，这扇门是安全的。

代理可以查看 schema，并执行读取：`SELECT`、`WITH`、`SHOW`、`EXPLAIN`、`PRAGMA`。危险语句到不了数据库。`INSERT`、`UPDATE`、`DELETE`、`DROP` 以及其余写入只在这台服务器上被拒绝。在终端提示符里，同样的语句仍按你写的执行。HTTP 只监听 `127.0.0.1`。

```bash
sqf mcp                      # stdio，给编辑器或代理
sqf mcp --port 8787          # http://127.0.0.1:8787/mcp   ·   GET /health
sqf --mcp-port 8787 ./app.db # 同一个服务器，开在提示符旁边
```

---

## 轻松开始

建库只要几步，即使这台机器上还没有对应的数据库。运行 `sqf`，选择**创建**。

如果本机没有进程在听，SQffy 可以拉取 Docker 镜像并运行，或在 Windows 上用 winget 帮你安装。两条路都不合适时，它会打开官方下载页。你不必先自己装好 Postgres。

选择器有三步：

1. 连接已保存的数据库
2. 添加一个已有的
3. 新建一个——文件，或一台服务器

```bash
sqf                              # 选择器
sqf ./app.db                     # 直接打开 SQLite 文件
sqf postgres://USER:PASSWORD@localhost:5432/app
```

```text
.tables
.schema users
SELECT id, name FROM users WHERE id >= 5;
.generate users 1000 --dry-run
```

连接记在 `connections.toml`。`--no-save` 不会碰这个文件。

| 系统 | 路径 |
| :--- | :--- |
| Linux | `~/.config/sqffy/connections.toml` |
| macOS | `~/Library/Application Support/dev.SQffy.sqffy/connections.toml` |
| Windows | `%APPDATA%\SQffy\sqffy\config\connections.toml` |

测试可以用 `SQFFY_HOME` 把目录指到别处。

---

## 安装

需要 Rust 1.85 或更高版本。在本仓库的检出目录中：

```bash
cargo install --path .

# 或不安装，直接运行
cargo run --release --bin sqf
```

SQLite 已经打进二进制。Oracle 需要本机安装 [Instant Client](https://www.oracle.com/database/technologies/instant-client.html)。

```bash
sqf --lang zh                   # en · ru · zh
SQFFY_LANG=zh sqf
```

语言解析顺序：`--lang`，然后 `SQFFY_LANG`，然后系统区域，最后是英语。界面字符串见 [docs/I18N.md](./docs/I18N.md)。

<details>
<summary><b>Feature 开关</b></summary>

| Feature | 默认 | |
| :--- | :---: | :--- |
| `sqlite` | 是 | 内置 SQLite |
| `postgres` | 是 | PostgreSQL |
| `mysql` | 是 | MySQL |
| `mariadb` | 是 | MariaDB，含本地拉起 |
| `clickhouse` | 是 | 通过 HTTP 连接 ClickHouse |
| `oracle` | 是 | Oracle，经由 Instant Client |
| `e2e` | 否 | 仅用于 `cargo e2e` |

```bash
cargo build --release --no-default-features --features sqlite
```

</details>

---

## 参考

### 元命令

| 命令 | |
| :--- | :--- |
| `.help` &nbsp; `.h` | 命令列表 |
| `.tables` &nbsp; `.t` | 表与视图 |
| `.schema` `[table]` | 名称索引，或一张表的完整信息 |
| `.describe` `<table>` | 列 |
| `.generate` `<table> <n> [--yes] [--dry-run] [col=ref.col …]` | 生成用于测试和可视化的数据 |
| `.quit` &nbsp; `.q` | 退出 |

### 参数

| 参数 | |
| :--- | :--- |
| `-c`, `--command` | 执行 SQL 或元命令后退出 |
| `-d`, `--driver` | 强制指定驱动 |
| `--limit N` | 打印的行数。`0` 为全部。默认 100。 |
| `--no-pager` | 关闭分页器 |
| `--no-save` | 不把这次连接写入目录 |
| `--mcp-port PORT` | 在会话旁启动安全的 MCP HTTP |
| `--lang en\|ru\|zh` | 界面语言 |
| `--list-drivers` | 打印内置驱动并退出 |

### 数据库

| 引擎 | 打开方式 | 说明 |
| :--- | :--- | :--- |
| SQLite | `sqf ./app.db` · `sqf sqlite:./app.db` · `sqf :memory:` | 内置。新文件就是一个路径。 |
| PostgreSQL | `postgres://USER:PASSWORD@localhost:5432/app` | 可从选择器走 Docker 或安装程序 |
| MySQL | `mysql://USER:PASSWORD@localhost:3306/app` | 可从选择器走 Docker 或安装程序 |
| MariaDB | `mariadb://USER:PASSWORD@localhost:3306/app` | 可从选择器走 Docker 或安装程序 |
| ClickHouse | `clickhouse://USER:PASSWORD@localhost:8123/default` | HTTP。可从选择器走 Docker 或安装程序 |
| Oracle | `oracle://USER:PASSWORD@localhost:1521/XEPDB1` | 需要 Instant Client |

### MCP 工具

| 工具 | |
| :--- | :--- |
| `list_connections` | 本次会话中打开的连接 |
| `list_tables` | 表与视图 |
| `describe_table` | 列 |
| `inspect_table` | 列、约束、索引 |
| `execute_readonly` | 只读查询。写入会被拒绝。 |
| `list_drivers` | 编进这个构建的驱动 |

---

## 文档

| | |
| :--- | :--- |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 这个 crate 如何分层 |
| [docs/DRIVERS.md](./docs/DRIVERS.md) | 添加数据库适配器 |
| [docs/I18N.md](./docs/I18N.md) | 添加一种语言 |

<details>
<summary><b>开发</b></summary>

```bash
cargo test       # 单元测试
cargo e2e        # CLI 端到端：SQLite、MCP、元命令
cargo e2e-all    # 再加上服务器适配器——先设置 SQFFY_E2E_*
```

| 变量 | 示例 |
| :--- | :--- |
| `SQFFY_E2E_POSTGRES` | `postgres://sqffy:sqffy@127.0.0.1:5432/sqffy` |
| `SQFFY_E2E_MYSQL` | `mysql://sqffy:sqffy@127.0.0.1:3306/sqffy` |
| `SQFFY_E2E_MARIADB` | `mariadb://sqffy:sqffy@127.0.0.1:3307/sqffy` |
| `SQFFY_E2E_CLICKHOUSE` | `clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy` |
| `SQFFY_E2E_ORACLE` | `oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1` |

这些 URL 属于 Docker 测试栈。

```bash
docker compose -f e2e/docker-compose.yml up --build --abort-on-container-exit
```

CI 把 **Test**、**Build** 和 **E2E** 分成独立任务。工作流：[`.github/workflows/ci.yml`](.github/workflows/ci.yml)。

```text
sqffy/
├── src/        CLI、驱动、REPL、MCP、generate、provision
├── locales/    en.toml · ru.toml · zh.toml
├── docs/       驱动、i18n、截图
├── tests/      e2e_*  (feature = e2e)
├── e2e/        Docker Compose
└── .github/    CI
```

</details>

---

<div align="center">

<a href="https://www.rust-lang.org/"><img alt="Rust 1.85+" src="https://img.shields.io/badge/Rust-1.85+-DEA584?style=flat-square&logo=rust&logoColor=white"></a>
&nbsp;
<a href="./LICENSE"><img alt="MIT 许可证" src="https://img.shields.io/badge/license-MIT-22C55E?style=flat-square"></a>

<br><br>

**[↑ 回到顶部](#sqffy)**

</div>

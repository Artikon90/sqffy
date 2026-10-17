# SQffy 架构（`sqf`）

[English](./ARCHITECTURE.md) · [Русский](./ARCHITECTURE.ru.md) · **中文**

SQffy 是一个 Rust 写的命令行 SQL 客户端。它是 REPL：一行进去，一个结果出来。它不是 TUI。二进制名称是 `sqf`。

Shell 保持轻快，因为热路径是同步的，SQLite 打进了二进制，补全读的是 schema 缓存，而不是每次按 Tab 都去问数据库。输出是渲染出来的——带类型的表、框住的 SQL 错误、高亮的 DDL——不是一堆原始文本。只读的 MCP 服务器可以开在这个会话旁边，给编辑器里的代理用。

## 范围

| 这棵树里有 | 这棵树里没有 |
| --- | --- |
| REPL：一行 → SQL 或元命令 → stdout | 面板、焦点、vim 模式 |
| 六个引擎，共用一个 `Database` trait | 与 psql `\` 命令完全对齐 |
| 用树内分词器做语法高亮 | 单独的 GUI |
| 按子句、从 schema 缓存做自动补全 | 经 MCP 服务器写入 |
| 行数上限和一个小的内置分页器 | 全屏分页器（`q` / Esc） |
| 可插拔驱动（`Driver` + registry） | 查询路径上的异步运行时 |
| 已保存的连接和本地拉起 | |
| 用于测试和演示行的 `.generate` | |
| 界面目录：英语、俄语、中文 | |

## 流程

```text
clap (cli)
    │
    ├─ sqf mcp ─────────────▶ ConnectionPool ──▶ MCP guard ──▶ tools
    │
    └─ 目标或选择器 ──────▶ DriverRegistry.open ──▶ Box<dyn Database>
                                      │
                                      ▼
                              App::run_repl (rustyline)
                                      │
                    ┌─────────────────┼──────────────────┐
                    ▼                 ▼                  ▼
                  meta           execute_sql          complete
                    │                 │                  │
                    └──────── render ◀┘            SchemaCache
                              │                    + 子句扫描
                              ▼
                         stdout / pager

sql::highlight 给输入行和 DDL 上色
generate 造出行，并通过 Database 执行 INSERT
provision（Docker / winget）在 open 之前从选择器跑
```

交互式 shell 只持有一个 `Box<dyn Database>`。Registry 根据 URL 或 `--driver` 选择具体后端。渲染从不自己打开连接。元命令不会把整个库倒成一列。

MCP 启动时（`sqf mcp`，或在提示符旁加 `--mcp-port`），`ConnectionPool` 可以同时拿着若干已保存的连接。Shell 和池是两扇门：shell 执行写入，池里的工具不执行。

## 模块

```text
src/
  main.rs            进程入口
  lib.rs             crate 根
  cli.rs             目标、--driver、--lang、--limit、-c、--mcp-port、mcp 子命令
  config.rs          会话设置（行数上限、分页器）
  i18n.rs            t/tr，编译期嵌入 locales/en.toml|ru.toml|zh.toml
  connections.rs     ConnectionStore → connections.toml
  picker.rs          连接 / 添加 / 创建，含 provision
  app.rs             选择器或 CLI 目标 → REPL，可选地在旁边开 MCP
  error.rs           Backend { driver, message } — crate 根不依赖 rusqlite
  pool.rs            给 MCP 服务器用的具名连接
  db/                Driver、Database、registry，每个引擎一个模块
  meta/              点命令
  render/            表、schema 视图、DDL 颜色、错误、分页器
  complete/          schema 缓存、子句扫描、候选
  repl/              rustyline、高亮、completer
  sql/               分词器、关键字表、高亮
  generate/          预览和分批 INSERT
  provision/         Docker pull/run、winget 安装
  mcp/               stdio 与 HTTP、guard、工具
  ui/                颜色、边框、选择器控件、进度
locales/
  en.toml ru.toml zh.toml
docs/
  DRIVERS.md         如何加一个引擎
  I18N.md            如何加一种语言或一条字符串
```

### 启动与连接

1. 不带目标的 `sqf` 打开选择器：已保存的库、添加已有的，或新建。
2. 选定后调用 `DriverRegistry::open`，再把 `last_used` 写入 `connections.toml`。
3. `sqf ./file.db` 直接连接并记下这次连接，除非带了 `--no-save` 或 `-c`。
4. 创建服务器型数据库时先探测 localhost。没有进程在听，选择器就提供 Docker、下载（Windows 上用 winget，否则是官方页面），或用户已经有的 URL。

配置路径来自 `directories`（`dev.SQffy.sqffy`）：Linux 是 `~/.config/sqffy`，macOS 是 `~/Library/Application Support/dev.SQffy.sqffy`，Windows 是 `%APPDATA%\SQffy\sqffy\config`。测试用 `SQFFY_HOME` 换掉这个根。

### i18n

所有面向用户的字符串都走 `i18n::t` / `tr`。解析顺序：`--lang`，然后 `SQFFY_LANG`，然后 `LC_ALL`、`LC_MESSAGES`、`LANG`，最后是 `en`。详见 [`docs/I18N.zh.md`](./docs/I18N.zh.md)。

### `db` — 引擎

三种角色：

| 类型 | 角色 |
| --- | --- |
| `ConnectionSpec` | 打开什么：驱动 id、目标、可选的原始 URL |
| `Driver` | 工厂：`open(spec) → Box<dyn Database>`，外加 `provision` 和 `probe_local` |
| `Database` | 会话：SQL 和内省 |

`Capabilities` 告诉元命令：引擎有没有 schema，以及 `.schema` 能不能给出 `CREATE` 文本。错误是 `Error::backend("sqlite", …)`，所以 crate 根不依赖某个驱动 crate。

| 引擎 | Feature | 会话 | Schema | `CREATE` 文本 | 引号 |
| --- | --- | --- | --- | --- | --- |
| SQLite | `sqlite` | 内置 `rusqlite` | 否 | 有 | `"` |
| PostgreSQL | `postgres` | 同步 `postgres` | 有 | 有 | `"` |
| MySQL | `mysql` | 同步 `mysql` | 有 | 有 | `` ` `` |
| MariaDB | `mariadb` | MySQL 协议，单独的拉起配方 | 有 | 有 | `` ` `` |
| ClickHouse | `clickhouse` | 经 `ureq` 的 HTTP | 有 | 有 | `` ` `` |
| Oracle | `oracle` | `oracle` crate，Instant Client | 有 | 无 | `"` |

六个 feature 默认都开着。新引擎是一个模块加 registry 里的一行：[`docs/DRIVERS.zh.md`](./docs/DRIVERS.zh.md)。

URL scheme 在 `db/connect.rs` 里解析：`sqlite:` / `file:`、`postgres:` / `postgresql:`、`mysql:`、`mariadb:` / `maria:`、`clickhouse:` / `ch:`、`oracle:` / `oracledb:` / `ora:`。裸路径是 SQLite。`:memory:` 是内存中的 SQLite。

### `meta`

| 命令 | 行为 |
| --- | --- |
| `.tables`（`.t`） | 表和视图 |
| `.schema`（`.s`） | 名称索引，不是整库转储 |
| `.schema t` | 一张表：列、约束、索引，以及引擎能返回时的 DDL |
| `.describe t`（`.d`） | 列清单：类型、null、默认值、主键、注释 |
| `.generate`（`.gen`） | 先预览，再分批插入。一百行及以上需要 `--yes` |
| `.help`（`.h`、`?`） | 命令列表 |
| `.quit`（`.q`、`.exit`） | 离开 |

### `render`

- `QueryResult` → `comfy-table`，单元格截断，`NULL` 变暗，列名下面是声明的类型。
- DDL 和输入行 → `sql::highlight`（`owo-colors`），不是 `syntect`。
- SQL 错误 → 一个闭合的框；后端报告了位置时带一个插入符。
- `Rendered { text, tall }` → 分页器开着时走 `page_if_needed`。

### `complete`

1. 连接时和 DDL 之后调用 `SchemaCache::refresh`。
2. `complete::context` 扫描还没写完的语句（子句、作用域里的表）。这是一个宽容的扫描器，不是完整的 SQL 解析器。
3. `Completer::candidates` 按这个子句给关键字、表、列、类型和元命令排序。

### `repl`

- `rustyline`，不是 TUI 事件循环。
- Helper 给缓冲区上色，并在 Tab 时补全。
- Ctrl+C 打断当前行，不退出程序。
- 用 `.quit` 或 EOF（Ctrl+D）离开。

### `mcp`

`mcp/guard.rs` 只放行这样的语句：去掉前导注释后，以 `SELECT`、`WITH`、`SHOW`、`EXPLAIN`、`DESCRIBE`、`DESC` 或 `PRAGMA` 开头。其余在 `execute_sql` 之前被拒绝。

工具：`list_connections`、`list_tables`、`describe_table`、`inspect_table`、`execute_readonly`、`list_drivers`。

传输：stdio 上按行分隔的 JSON-RPC（`sqf mcp`），或 `127.0.0.1` 上的 HTTP（`sqf mcp --port`，或 REPL 旁的 `--mcp-port`）。HTTP 端口提供 `GET /health`。

### `generate`

`.generate` 用 `inspect_table` 做计划，从被引用的表填外键池，打印预览，再分批插入。`--dry-run` 停在预览之后。没有 `--yes` 时，100 及以上的数量会被拒绝。值来自 `fake` crate。

### `provision`

每个 `Driver` 返回 `ProvisionInfo`：文件还是服务器、默认端口、可选的 Docker 规格、可选的 winget id、可选的下载 URL。选择器先调用 `probe_local`。Docker pull/run 会画进度。Winget 只在 Windows 上；其他平台得到下载 URL。

## 性能

1. **同步驱动。** 查询路径上没有异步运行时。`Database: Send`。
2. **默认行数上限 100**（`--limit`，`0` = 不限制）卡住分配和打印。
3. **Schema 缓存**，Tab 不会在每次按键时重新内省。
4. **Release 配置：** LTO、`codegen-units = 1`、`strip`、`panic = abort`。
5. **内置 SQLite**，二进制不依赖系统里的 `libsqlite`。

还没做，也不假装已经做了：

- 分页时流式输出，而不是握着每一行；
- 真正的分页器（`q` / Esc / Ctrl+C）——现在长结果会整段打印，分页器等待 Enter；
- 多次渲染之间复用单元格缓冲区。

## 分页器

短输出直接去 stdout。长输出经过 `render/pager.rs`：打印文本，然后等 Enter。`--no-pager` 跳过这次等待。

## 代码规则

1. **CLI，不是 TUI。** 打印，然后把终端还回去。
2. **模块边界。** `app`、`meta`、`render`、`repl` 和 `complete` 依赖 `dyn Database`。它们不导入驱动 crate。
3. **新数据库就是一个新 `Driver`。** REPL 不会为每个引擎长出一个 `match` 臂。
4. **Schema 是结构化的。** 列、约束和索引比一份原始 `.schema` 转储更重要。
5. **Feature 切掉依赖。** `cargo build --no-default-features --features sqlite` 是只含 SQLite 的二进制。
6. **面向用户的文本是一条 locale 键。** 英语、俄语和中文保持对齐。

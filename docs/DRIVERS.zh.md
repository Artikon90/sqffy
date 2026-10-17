# 如何添加数据库驱动

[English](./DRIVERS.md) · [Русский](./DRIVERS.ru.md) · **中文**

REPL、元命令、渲染和自动补全都与具体后端无关。一个新数据库是一个模块、一个 Cargo feature、registry 里的一行，以及一个 URL scheme。

另见 [ARCHITECTURE.zh.md](../ARCHITECTURE.zh.md)。

## 契约

1. **`Database`**（`src/db/traits.rs`）— 已打开的会话：
   - `execute_sql`、`list_tables`、`describe_table`、`table_ddl`、`inspect_table`
   - `driver_id`、`display_label`、`capabilities`
   - `inspect_table` 的默认实现只返回列。引擎能报告约束和索引时，覆盖它。
2. **`Driver`**（`src/db/driver.rs`）— 工厂：
   - `id`、`aliases`、`description`
   - `open(&ConnectionSpec) -> Box<dyn Database>`
   - `provision()` — 如何在本地创建（文件还是服务器、Docker、winget、下载页）
   - `probe_local()` — 是否已经有服务器在听
3. 在 `default_registry()`（`src/db/registry.rs`）里**注册**驱动。
4. 在 `src/db/connect.rs` 的 `parse_url` 里**解析** URL scheme。`--driver <id>` 本来就会覆盖 scheme。
5. 在 `Cargo.toml` 里加 **Cargo feature** 和可选依赖。在 `src/db/mod.rs` 里用 cfg 关上模块。

`ConnectionSpec` 在任何驱动运行之前就解析好了：

| 输入 | 结果 |
| --- | --- |
| `./file.db` | 驱动 `sqlite`，target = 路径 |
| `:memory:` | 内存中的 sqlite |
| `sqlite:./file.db`、`file:./file.db` | sqlite |
| `postgres://…`、`postgresql://…` | postgres |
| `mysql://…` | mysql |
| `mariadb://…`、`maria://…` | mariadb |
| `clickhouse://…`、`ch://…` | clickhouse |
| `oracle://…`、`oracledb://…`、`ora://…` | oracle |
| `--driver mysql host/db` | mysql，加上这个目标 |

`--driver` 的别名（`pg`、`ch`、`ora`、`maria`、`file` 等）由 registry 解析，不是 `parse_url`。

## 清单

```text
[ ] src/db/<name>.rs — Driver + Database
[ ] Cargo.toml — feature + 可选 crate
[ ] src/db/mod.rs — #[cfg(feature = "…")] mod
[ ] registry.rs — 在同一个 cfg 后面注册
[ ] connect.rs — 如果引擎有 URL，就在 parse_url 里加 scheme
[ ] Capabilities { schemas, table_ddl, quote_char }
[ ] inspect_table — 引擎有约束和索引时实现它们
[ ] provision() / probe_local() — 即使两者都是「不支持」
[ ] 错误走 Error::backend("name", …)
[ ] locales — 只有新增了面向用户的字符串才改；en.toml、ru.toml、zh.toml 保持对齐
[ ] cargo test，再加一次手工连接
[ ] README.md、README.ru.md、README.zh.md — 一条连接示例
```

**不要**从 `app`、`meta`、`render`、`repl` 或 `complete` 导入驱动 crate。它们只看得到 `dyn Database`。

## 现状

| 驱动 | id | 别名 | Crate / 协议 | Feature | Schema | CREATE 文本 |
| --- | --- | --- | --- | --- | --- | --- |
| SQLite | `sqlite` | `file` | `rusqlite`，内置 | `sqlite` | 否 | 有 |
| PostgreSQL | `postgres` | `postgresql`、`pg` | 同步 `postgres` | `postgres` | 有 | 有 |
| MySQL | `mysql` | — | 同步 `mysql` | `mysql` | 有 | 有 |
| MariaDB | `mariadb` | `maria` | MySQL 协议（`mysql` 模块）+ 自己的 provision | `mariadb` | 有 | 有 |
| ClickHouse | `clickhouse` | `ch`、`click` | 经 `ureq` 的 HTTP | `clickhouse` | 有 | 有 |
| Oracle | `oracle` | `oracledb`、`ora` | `oracle`（ODPI-C，Instant Client） | `oracle` | 有 | 无 |

六个都是默认 feature。MariaDB 不另写一套会话类型：`MariadbDriver::open` 调用 `open_mysql_url`，只有拉起配方不同。`quote_char` 在 SQLite、PostgreSQL 和 Oracle 上是 `"`，在 MySQL、MariaDB 和 ClickHouse 上是 `` ` ``。

## 样子

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

`CreateKind::File` 是 SQLite 的路：创建一个文件，没有服务器。`CreateKind::Server` 走选择器：先探测，然后是 Docker、下载，或粘贴的 URL。

## Capabilities

| 标志 | 效果 |
| --- | --- |
| `schemas` | 引擎有命名空间。元命令和补全可以显示它们。 |
| `table_ddl` | 为 false 时，`.schema t` 仍然显示列和键，并省略 `CREATE` 脚本。Oracle 就是这样。 |
| `quote_char` | `.generate` 拼 `INSERT` 时用的标识符引号。 |

## 同步

驱动是同步的（只有 `Database: Send`）。优先用阻塞客户端。如果客户端是异步的，就在驱动内部阻塞，让 `App` 保持同步。不要把运行时放上 REPL 路径。

## 冒烟

```bash
cargo build --no-default-features --features sqlite,foo
sqf --list-drivers
sqf foo://USER:PASSWORD@127.0.0.1:1/app
```

端到端测试放在 `tests/e2e_<name>.rs`，`required-features = ["e2e"]`，用环境变量 `SQFFY_E2E_<NAME>` 开关。把它接进 `e2e-all` 别名的方式，跟现有的服务器测试一样。

# Как добавить драйвер базы

[English](./DRIVERS.md) · **Русский** · [中文](./DRIVERS.zh.md)

REPL, meta-команды, рендер и автодополнение не знают конкретный бэкенд. Новая база — это один модуль, Cargo feature, строка в registry и схема URL.

См. также [ARCHITECTURE.ru.md](../ARCHITECTURE.ru.md).

## Контракт

1. **`Database`** (`src/db/traits.rs`) — открытая сессия:
   - `execute_sql`, `list_tables`, `describe_table`, `table_ddl`, `inspect_table`
   - `driver_id`, `display_label`, `capabilities`
   - у `inspect_table` есть реализация по умолчанию: только колонки. Переопределите её, если движок умеет ограничения и индексы.
2. **`Driver`** (`src/db/driver.rs`) — фабрика:
   - `id`, `aliases`, `description`
   - `open(&ConnectionSpec) -> Box<dyn Database>`
   - `provision()` — как создать базу локально (файл или сервер, Docker, winget, страница загрузки)
   - `probe_local()` — слушает ли сервер уже сейчас
3. **Зарегистрировать** драйвер в `default_registry()` (`src/db/registry.rs`).
4. **Разобрать** схему URL в `src/db/connect.rs` (`parse_url`). `--driver <id>` и так перекрывает схему.
5. **Cargo feature** и optional-зависимость в `Cargo.toml`. Модуль закрыть `cfg` в `src/db/mod.rs`.

`ConnectionSpec` разбирается до запуска драйвера:

| Ввод | Результат |
| --- | --- |
| `./file.db` | драйвер `sqlite`, target = путь |
| `:memory:` | sqlite в памяти |
| `sqlite:./file.db`, `file:./file.db` | sqlite |
| `postgres://…`, `postgresql://…` | postgres |
| `mysql://…` | mysql |
| `mariadb://…`, `maria://…` | mariadb |
| `clickhouse://…`, `ch://…` | clickhouse |
| `oracle://…`, `oracledb://…`, `ora://…` | oracle |
| `--driver mysql host/db` | mysql и эта цель |

Алиасы `--driver` (`pg`, `ch`, `ora`, `maria`, `file`, …) решает registry, не `parse_url`.

## Чеклист

```text
[ ] src/db/<name>.rs — Driver + Database
[ ] Cargo.toml — feature + optional-крейт
[ ] src/db/mod.rs — #[cfg(feature = "…")] mod
[ ] registry.rs — регистрация под тем же cfg
[ ] connect.rs — схема в parse_url, если у движка есть URL
[ ] Capabilities { schemas, table_ddl, quote_char }
[ ] inspect_table — ограничения и индексы, если движок их отдаёт
[ ] provision() / probe_local() — даже если оба «не поддерживается»
[ ] Ошибки через Error::backend("name", …)
[ ] locales — только если добавились пользовательские строки; en.toml, ru.toml и zh.toml в паритете
[ ] cargo test и ручное подключение
[ ] README.md, README.ru.md, README.zh.md — один пример подключения
```

**Не** импортируйте крейт драйвера из `app`, `meta`, `render`, `repl` или `complete`. Они видят только `dyn Database`.

## Состояние

| Драйвер | id | Алиасы | Крейт / провод | Feature | Схемы | Текст CREATE |
| --- | --- | --- | --- | --- | --- | --- |
| SQLite | `sqlite` | `file` | `rusqlite`, вшит | `sqlite` | нет | да |
| PostgreSQL | `postgres` | `postgresql`, `pg` | синхронный `postgres` | `postgres` | да | да |
| MySQL | `mysql` | — | синхронный `mysql` | `mysql` | да | да |
| MariaDB | `mariadb` | `maria` | провод MySQL (модуль `mysql`) + свой provision | `mariadb` | да | да |
| ClickHouse | `clickhouse` | `ch`, `click` | HTTP через `ureq` | `clickhouse` | да | да |
| Oracle | `oracle` | `oracledb`, `ora` | `oracle` (ODPI-C, Instant Client) | `oracle` | да | нет |

Все шесть — feature по умолчанию. MariaDB не заводит второй тип сессии: `MariadbDriver::open` вызывает `open_mysql_url`, отличается только рецепт запуска. `quote_char` — `"` у SQLite, PostgreSQL и Oracle и `` ` `` у MySQL, MariaDB и ClickHouse.

## Образец

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

`CreateKind::File` — путь SQLite: создать файл, без сервера. `CreateKind::Server` идёт через пикер: проверка, затем Docker, скачивание или вставленный URL.

## Capabilities

| Флаг | Эффект |
| --- | --- |
| `schemas` | У движка есть пространства имён. Meta и дополнение могут их показать. |
| `table_ddl` | Если false, `.schema t` всё равно показывает колонки и ключи и опускает скрипт `CREATE`. Так делает Oracle. |
| `quote_char` | Кавычка идентификатора, которой `.generate` собирает `INSERT`. |

## Синхронность

Драйверы синхронные (только `Database: Send`). Берите блокирующий клиент. Если клиент асинхронный, блокируйте внутри драйвера, чтобы `App` оставался синхронным. Не ставьте рантайм на путь REPL.

## Дымовая проверка

```bash
cargo build --no-default-features --features sqlite,foo
sqf --list-drivers
sqf foo://USER:PASSWORD@127.0.0.1:1/app
```

Сквозной тест кладётся в `tests/e2e_<name>.rs` с `required-features = ["e2e"]` и переменной `SQFFY_E2E_<NAME>`. Подключите его к алиасу `e2e-all` так же, как подключены нынешние серверные тесты.

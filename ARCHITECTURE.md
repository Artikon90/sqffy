# Архитектура SQffy (`sqf`)

**SQffy** — CLI SQL-клиент на Rust (REPL, **не** TUI). Цель: быстрее и легче litecli/pgcli, с читаемым выводом схемы, сохранёнными подключениями и i18n.

## Цели и границы

| Делаем | Не делаем (v1) |
| --- | --- |
| REPL: ввод → SQL / meta → stdout | Панели, focus, vim-modes (rainfrog) |
| SQLite через `rusqlite` (bundled) | Полный паритет psql `\`-команд |
| Подсветка DDL и keywords | IDE с вкладками (Harlequin) |
| Autocomplete по schema cache | Мульти-БД в одном процессе |
| Лимит строк + свой pager | Зависимость от `less` |
| Pluggable drivers (`Driver` + registry) | GUI |
| URL / `--driver` подключение | Полная реализация PG/MySQL в v1 |

## Высокоуровневый поток

```text
┌────────────┐     ┌──────────┐     ┌────────────┐
│ clap (cli) │────▶│   App    │────▶│   Repl     │  rustyline
└─────┬──────┘     └────┬─────┘     └─────┬──────┘
      │                 │                 │ line
      ▼                 ▼                 ▼
 ConnectionSpec   ┌─────────┐     ┌────────────┐
      │           │  meta   │◀────│  dispatch  │
      ▼           └────┬────┘     └─────┬──────┘
 DriverRegistry        │                │ SQL
      │                ▼                ▼
      └────────▶ Box<dyn Database> ◀────┘
                        │
                        ▼
                   render (+ pager)

complete::SchemaCache ──▶ Completer ──▶ rustyline Helper
```

`App` держит только `Box<dyn Database>`. Конкретный backend выбирает registry по DSN/`--driver`.

Интерактивный цикл — `App::run_repl`. Рендер **не** ходит в БД. Meta **не** дампит всю схему одной колонкой.

## Модули

```text
src/
  main.rs          точка входа
  lib.rs           публичный API крейта
  cli.rs           target URL/path, --driver, --lang, --list-drivers, --limit, -c
  config.rs        runtime-настройки сессии
  i18n.rs          t/tr + embedded locales/en.toml|ru.toml
  connections.rs   ConnectionStore → connections.toml
  picker.rs        интерактивный выбор / создать / добавить БД
  app.rs           picker или CLI target → REPL → dispatch
  error.rs         Backend { driver, message } — без привязки к rusqlite
  db/
    connect.rs     ConnectionSpec (URL / path / override)
    driver.rs      trait Driver
    registry.rs    DriverRegistry + default_registry()
    traits.rs      Database, Capabilities, QueryResult, …
    sqlite.rs      #[cfg(feature = "sqlite")]
    postgres.rs    #[cfg(feature = "postgres")] stub
    mysql.rs       #[cfg(feature = "mysql")] stub
  meta/  render/  complete/  repl/   — только dyn Database
docs/
  DRIVERS.md       чеклист «как добавить БД»
  I18N.md          как добавлять языки и строки
locales/
  en.toml ru.toml  UI-каталоги (compile-time embed)
```

### Старт и подключения

1. `sqf` без target → `picker` (список + «добавить» + «создать»).
2. Выбор → `DriverRegistry::open` → сохранение `last_used` в `connections.toml`.
3. `sqf ./file.db` → прямое подключение; запись в каталог (кроме `--no-save` / `-c`).

### i18n

Все пользовательские строки через `i18n::t` / `tr`. Язык: `--lang` → `SQFFY_LANG` → система → `en`. Подробности: [`docs/I18N.md`](./docs/I18N.md).

### `db` — расширяемые бэкенды

Слой из трёх ролей:

| Тип | Роль |
| --- | --- |
| `ConnectionSpec` | что открыть (driver + target + optional URL) |
| `Driver` | фабрика: `open(spec) → Box<dyn Database>` |
| `Database` | сессия: SQL + introspection |

- `Capabilities` сообщает meta, есть ли DDL / schemas.
- Ошибки: `Error::backend("sqlite", …)` — crate root не зависит от `rusqlite`.
- Cargo features: `sqlite` (default), `postgres`, `mysql`.
- Как подключить новую БД: [`docs/DRIVERS.md`](./docs/DRIVERS.md).

### `meta` — точечные команды

Ответ на боль litecli:

| Команда | Поведение |
| --- | --- |
| `.tables` | список table/view |
| `.schema` | **индекс** имён, не dump всей БД |
| `.schema t` | один DDL, цветной блок |
| `.describe t` | таблица колонок (type/null/default/pk) |
| `.quit` | выход |

### `render` — только форматирование

- `QueryResult` → `comfy-table`, truncate ячеек, NULL серым.
- DDL → tokenizer + `owo-colors` (позже `syntect`).
- `Rendered { text, tall }` → `page_if_needed` при `config.pager`.

### `complete` — autocomplete

1. `SchemaCache::refresh` при старте и после DDL.
2. `Completer::candidates` — keywords + tables + columns.
3. v2: контекст через `sqlparser` (после `FROM` только таблицы и т.д.).

### `repl` — ввод

- `rustyline`, не event-loop TUI.
- Ctrl+C → interrupt строки, **не** выход (как в нормальных shell).
- Выход: `.quit` / EOF (Ctrl+D).

## Производительность и ресурсы

1. **Sync SQLite** — без async runtime на hot path.
2. **Лимит строк по умолчанию (100)** — меньше аллокаций и печати.
3. **Schema cache** — autocomplete без PRAGMA на каждый Tab.
4. **Release profile** — LTO, `codegen-units=1`, `strip`, `panic=abort`.
5. **Bundled libsqlite** — предсказуемый бинарник, без системного SQLite.

Будущие оптимизации (не в скелете):

- настоящий streaming в pager без `Vec` всех строк;
- arena/buffer reuse для строк ячеек;
- отложенная загрузка колонок в cache (только имена таблиц до первого Tab).

## Pager (контракт UX)

Проблема litecli: пользователь в `less`, Esc/Ctrl+C «не работают», выход — неочевидный `q`.

Контракт Glint:

1. Короткая выводка — сразу в stdout.
2. Длинная — встроенный pager; **q / Esc / Ctrl+C** выходят одинаково (v1 пока «Enter to continue», API уже отдельный модуль).
3. `--no-pager` и будущий `.nopager` отключают.

## Дорожная карта

### Milestone A — скелет (сейчас)

- [x] Модульная структура + компилируемый каркас
- [x] SQLite open / SQL execute / limit
- [x] `.tables` `.schema` `.describe` `.help` `.quit`
- [x] Базовый autocomplete + rustyline
- [x] Документ архитектуры

### Milestone B — «лучше litecli на схеме»

- [ ] Полноэкранный pager (q/Esc/Ctrl+C)
- [ ] Подсветка ввода в prompt (`syntect` / highlighter)
- [ ] `.schema` с FK из `PRAGMA foreign_key_list`
- [ ] История в файл (`~/.local/share/glint/history`)

### Milestone C — интеллект SQL

- [ ] `sqlparser` smart complete
- [ ] Multi-statement / продолжение строк (`...`)
- [ ] Тайминг + rows affected в едином footer

### Milestone D — расширение драйверов

- [x] `Driver` + `DriverRegistry` + `ConnectionSpec`
- [x] Feature-gated stubs postgres/mysql + `docs/DRIVERS.md`
- [ ] Реальный Postgres backend
- [ ] Реальный MySQL backend
- [ ] JSON output для агентов
- [ ] Конфиг `glint.toml`

## Зависимости (текущие и планируемые)

| Сейчас | Зачем |
| --- | --- |
| `clap` | CLI |
| `rustyline` | REPL |
| `rusqlite` (bundled) | SQLite |
| `comfy-table` | таблицы |
| `owo-colors` | цвет |
| `anyhow` / `thiserror` | ошибки |

| Позже | Зачем |
| --- | --- |
| `sqlparser` | smart complete |
| `syntect` | качественный highlight |
| `crossterm` | полноценный pager |
| `tokio` + `tokio-postgres` | только если появится PG async backend |

## Принципы кода

1. **CLI, не TUI** — печатаем и возвращаем управление shell.
2. **Границы модулей жёсткие** — `app`/`meta`/`render`/`repl` не импортируют `rusqlite` / PG crates.
3. **Новая БД = новый Driver**, не правки REPL.
4. **Схема семантически** — структура колонок важнее «как в sqlite3 .schema».
5. **Features режут зависимости** — бинарник без mysql/pg, пока они не нужны.
6. **Сначала качественный SQLite**, параллельно — стабильный plugin API.

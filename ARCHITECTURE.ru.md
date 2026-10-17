# Архитектура SQffy (`sqf`)

[English](./ARCHITECTURE.md) · **Русский** · [中文](./ARCHITECTURE.zh.md)

SQffy — консольный SQL-клиент на Rust. Это REPL: строка на входе, результат на выходе. Это не TUI. Бинарник — `sqf`.

Шелл остаётся быстрым, потому что горячий путь синхронный, SQLite вшит в бинарник, а автодополнение читает кэш схемы, а не базу на каждый Tab. Вывод рисуется — таблицы с типами, ошибка SQL в рамке, подсвеченный DDL, — а не вываливается сырым текстом. Рядом с сессией может работать read-only MCP-сервер для агента в редакторе.

## Границы

| Есть в этом дереве | Нет в этом дереве |
| --- | --- |
| REPL: строка → SQL или meta → stdout | Панели, фокус, vim-режимы |
| Шесть движков за одним трейтом `Database` | Полный паритет с `\`-командами psql |
| Подсветка своим токенизатором | Отдельный GUI |
| Автодополнение по клаузе из кэша схемы | Запись через MCP-сервер |
| Лимит строк и небольшой встроенный пейджер | Полноэкранный пейджер (`q` / Esc) |
| Подключаемые драйверы (`Driver` + registry) | Async-рантайм на пути запроса |
| Сохранённые подключения и локальный запуск | |
| `.generate` для тестовых и демо-строк | |
| Каталоги UI: английский, русский, китайский | |

## Поток

```text
clap (cli)
    │
    ├─ sqf mcp ─────────────▶ ConnectionPool ──▶ MCP guard ──▶ tools
    │
    └─ target или picker ─▶ DriverRegistry.open ──▶ Box<dyn Database>
                                      │
                                      ▼
                              App::run_repl (rustyline)
                                      │
                    ┌─────────────────┼──────────────────┐
                    ▼                 ▼                  ▼
                  meta           execute_sql          complete
                    │                 │                  │
                    └──────── render ◀┘            SchemaCache
                              │                    + разбор клаузы
                              ▼
                         stdout / pager

sql::highlight красит строку ввода и DDL
generate собирает строки и делает INSERT через Database
provision (Docker / winget) запускается из пикера, до open
```

Интерактивный шелл держит один `Box<dyn Database>`. Registry выбирает конкретный бэкенд по URL или `--driver`. Рендер соединение не открывает. Meta не вываливает всю базу одной колонкой.

Когда поднят MCP (`sqf mcp` или `--mcp-port` рядом с приглашением), `ConnectionPool` может держать несколько сохранённых подключений. Шелл и пул — разные двери: шелл выполняет запись, инструменты пула — нет.

## Модули

```text
src/
  main.rs            точка входа
  lib.rs             корень крейта
  cli.rs             target, --driver, --lang, --limit, -c, --mcp-port, подкоманда mcp
  config.rs          настройки сессии (лимит строк, пейджер)
  i18n.rs            t/tr, вшитые locales/en.toml|ru.toml|zh.toml
  connections.rs     ConnectionStore → connections.toml
  picker.rs          открыть / добавить / создать, включая provision
  app.rs             пикер или цель CLI → REPL, по желанию MCP рядом
  error.rs           Backend { driver, message } — без rusqlite в корне крейта
  pool.rs            именованные подключения для MCP-сервера
  db/                Driver, Database, registry, модуль на движок
  meta/              dot-команды
  render/            таблицы, схема, цвет DDL, ошибки, пейджер
  complete/          кэш схемы, разбор клаузы, кандидаты
  repl/              rustyline, подсветка, completer
  sql/               токенизатор, списки ключевых слов, подсветка
  generate/          превью и пакетный INSERT
  provision/         Docker pull/run, установка через winget
  mcp/               stdio и HTTP, guard, инструменты
  ui/                цвет, рамки, виджеты пикера, прогресс
locales/
  en.toml ru.toml zh.toml
docs/
  DRIVERS.md         как добавить движок
  I18N.md            как добавить язык или строку
```

### Старт и подключения

1. `sqf` без цели открывает пикер: сохранённая база, добавить существующую или создать новую.
2. Выбор вызывает `DriverRegistry::open`, затем пишет `last_used` в `connections.toml`.
3. `sqf ./file.db` подключается напрямую и записывает подключение, если не заданы `--no-save` или `-c`.
4. Создание серверной базы сначала проверяет localhost. Если никто не слушает, пикер предлагает Docker, скачивание (winget на Windows, иначе официальная страница) или URL, который уже есть.

Путь к конфигу берётся из `directories` (`dev.SQffy.sqffy`): `~/.config/sqffy` на Linux, `~/Library/Application Support/dev.SQffy.sqffy` на macOS, `%APPDATA%\SQffy\sqffy\config` на Windows. `SQFFY_HOME` подменяет этот корень в тестах.

### i18n

Все пользовательские строки идут через `i18n::t` / `tr`. Порядок: `--lang`, затем `SQFFY_LANG`, затем `LC_ALL`, `LC_MESSAGES`, `LANG`, затем `en`. Подробности: [`docs/I18N.ru.md`](./docs/I18N.ru.md).

### `db` — движки

Три роли:

| Тип | Роль |
| --- | --- |
| `ConnectionSpec` | Что открыть: id драйвера, цель, необязательный исходный URL |
| `Driver` | Фабрика: `open(spec) → Box<dyn Database>`, плюс `provision` и `probe_local` |
| `Database` | Сессия: SQL и интроспекция |

`Capabilities` сообщает meta, есть ли у движка схемы и может ли `.schema` показать текст `CREATE`. Ошибки — `Error::backend("sqlite", …)`, поэтому корень крейта не зависит от крейта драйвера.

| Движок | Feature | Сессия | Схемы | Текст `CREATE` | Кавычка |
| --- | --- | --- | --- | --- | --- |
| SQLite | `sqlite` | вшитый `rusqlite` | нет | да | `"` |
| PostgreSQL | `postgres` | синхронный `postgres` | да | да | `"` |
| MySQL | `mysql` | синхронный `mysql` | да | да | `` ` `` |
| MariaDB | `mariadb` | провод MySQL, свой рецепт запуска | да | да | `` ` `` |
| ClickHouse | `clickhouse` | HTTP через `ureq` | да | да | `` ` `` |
| Oracle | `oracle` | крейт `oracle`, Instant Client | да | нет | `"` |

Все шесть feature включены по умолчанию. Новый движок — это модуль и строка в registry: [`docs/DRIVERS.ru.md`](./docs/DRIVERS.ru.md).

Схемы URL разбираются в `db/connect.rs`: `sqlite:` / `file:`, `postgres:` / `postgresql:`, `mysql:`, `mariadb:` / `maria:`, `clickhouse:` / `ch:`, `oracle:` / `oracledb:` / `ora:`. Голый путь — это SQLite. `:memory:` — SQLite в памяти.

### `meta`

| Команда | Поведение |
| --- | --- |
| `.tables` (`.t`) | Таблицы и представления |
| `.schema` (`.s`) | Индекс имён, не дамп базы |
| `.schema t` | Одна таблица: колонки, ограничения, индексы, DDL, если движок его отдаёт |
| `.describe t` (`.d`) | Список колонок: тип, null, default, pk, комментарий |
| `.generate` (`.gen`) | Превью, затем вставка пачками. Сотня строк и больше требует `--yes` |
| `.help` (`.h`, `?`) | Список команд |
| `.quit` (`.q`, `.exit`) | Выход |

### `render`

- `QueryResult` → `comfy-table`, обрезка ячеек, тусклый `NULL`, объявленный тип под именем колонки.
- DDL и строка ввода → `sql::highlight` (`owo-colors`), не `syntect`.
- Ошибки SQL → закрытая рамка, каретка, если бэкенд сообщил позицию.
- `Rendered { text, tall }` → `page_if_needed`, когда пейджер включён.

### `complete`

1. `SchemaCache::refresh` при подключении и после DDL.
2. `complete::context` разбирает незаконченный запрос (клауза, таблицы в области видимости). Это терпимый сканер, не полный парсер SQL.
3. `Completer::candidates` ранжирует ключевые слова, таблицы, колонки, типы и meta-команды для этой клаузы.

### `repl`

- `rustyline`, не цикл событий TUI.
- Helper подсвечивает буфер и дополняет по Tab.
- Ctrl+C прерывает строку и не выходит из программы.
- Выход: `.quit` или EOF (Ctrl+D).

### `mcp`

`mcp/guard.rs` пропускает оператор только если после отброшенных ведущих комментариев он начинается с `SELECT`, `WITH`, `SHOW`, `EXPLAIN`, `DESCRIBE`, `DESC` или `PRAGMA`. Всё остальное отклоняется до `execute_sql`.

Инструменты: `list_connections`, `list_tables`, `describe_table`, `inspect_table`, `execute_readonly`, `list_drivers`.

Транспорты: JSON-RPC по строкам на stdio (`sqf mcp`) или HTTP на `127.0.0.1` (`sqf mcp --port` либо `--mcp-port` рядом с REPL). На HTTP-порту есть `GET /health`.

### `generate`

`.generate` строит план из `inspect_table`, наполняет пулы внешних ключей из связанных таблиц, печатает превью и вставляет пачками. `--dry-run` останавливается после превью. Без `--yes` количество от 100 отклоняется. Значения берёт крейт `fake`.

### `provision`

Каждый `Driver` возвращает `ProvisionInfo`: файл или сервер, порт по умолчанию, необязательный рецепт Docker, необязательный id winget, необязательный URL загрузки. Пикер сначала вызывает `probe_local`. Docker pull/run показывает прогресс. Winget только на Windows; на остальных платформах остаётся URL загрузки.

## Производительность

1. **Синхронные драйверы.** На пути запроса нет async-рантайма. `Database: Send`.
2. **Лимит строк по умолчанию 100** (`--limit`, `0` = без лимита) ограничивает выделение памяти и печать.
3. **Кэш схемы**, чтобы Tab не делал интроспекцию на каждое нажатие.
4. **Release-профиль:** LTO, `codegen-units = 1`, `strip`, `panic = abort`.
5. **Вшитый SQLite**, поэтому бинарник не зависит от системного `libsqlite`.

Ещё не сделано, и это не притворяется готовым:

- потоковая отдача в пейджер без хранения всех строк;
- настоящий пейджер (`q` / Esc / Ctrl+C) — сейчас длинный результат печатается, и пейджер ждёт Enter;
- повторное использование буферов ячеек между отрисовками.

## Пейджер

Короткий вывод идёт сразу в stdout. Длинный проходит через `render/pager.rs`: текст печатается, затем ожидание Enter. `--no-pager` это ожидание отключает.

## Правила кода

1. **CLI, не TUI.** Напечатали — вернули терминал.
2. **Границы модулей.** `app`, `meta`, `render`, `repl` и `complete` зависят от `dyn Database`. Крейты драйверов они не импортируют.
3. **Новая база — новый `Driver`.** В REPL не появляется ветка `match` на каждый движок.
4. **Схема структурная.** Колонки, ограничения и индексы важнее сырого дампа `.schema`.
5. **Feature отрезают зависимости.** `cargo build --no-default-features --features sqlite` — бинарник только с SQLite.
6. **Пользовательский текст — ключ локали.** Английский, русский и китайский держатся в паритете.

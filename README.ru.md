<div align="center">

# ✨ SQffy

### Быстрый SQL CLI с сохранёнными подключениями — `sqf`

[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-2088FF?style=for-the-badge&logo=githubactions&logoColor=white)](.github/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-22C55E?style=for-the-badge)](./LICENSE)
[![SQLite](https://img.shields.io/badge/SQLite-bundled-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](#-базы-данных)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-ready-4169E1?style=for-the-badge&logo=postgresql&logoColor=white)](#-базы-данных)
[![MySQL](https://img.shields.io/badge/MySQL-ready-4479A1?style=for-the-badge&logo=mysql&logoColor=white)](#-базы-данных)
[![MariaDB](https://img.shields.io/badge/MariaDB-ready-C32136?style=for-the-badge&logo=mariadb&logoColor=white)](#-базы-данных)
[![ClickHouse](https://img.shields.io/badge/ClickHouse-ready-FFCC01?style=for-the-badge&logo=clickhouse&logoColor=black)](#-базы-данных)
[![Oracle](https://img.shields.io/badge/Oracle-ready-F80000?style=for-the-badge&logo=oracle&logoColor=white)](#-базы-данных)

[English](./README.md) · **Русский** · [中文](./README.zh.md)

<br/>

```text
   ____   ___   __  __
  / __/  / _ \ / /_/ /
 _\ \   / ___// __  /
/___/  /_/    \/_/_/    ·  sqf
```

Терминальный SQL-клиент «для людей»: красивые таблицы, богатые схемы,
умный автокомплит, генерация данных и read-only MCP — всё в одном бинарнике.

<br/>

[Возможности](#-возможности) ·
[Установка](#-установка) ·
[Быстрый старт](#-быстрый-старт) ·
[Использование](#-использование) ·
[MCP](#-mcp) ·
[Тесты](#-тесты-и-ci) ·
[Документация](#-документация)

</div>

---

## ✨ Возможности

| | Возможность | Что даёт |
| :--- | :--- | :--- |
| 🗄️ | **Несколько БД** | SQLite · PostgreSQL · MySQL · MariaDB · ClickHouse · Oracle |
| 🧭 | **Пикер подключений** | Каталог, добавить существующую, создать новую |
| 🐳 | **Локальный provision** | Docker pull/run или скачивание (winget) с прогресс-баром |
| 🎨 | **Читаемый вывод** | Типы под колонками, ошибки в рамке, компактный бренд |
| ⌨️ | **Умный автокомплит** | Подсказки по клаузам (таблицы, колонки, meta) |
| 🧪 | **`.generate`** | Синтетические строки: превью + батчи + FK |
| 🔌 | **MCP** | Read-only инструменты по stdio или HTTP (`/mcp`) |
| 🌍 | **i18n** | EN · RU · ZH (`--lang` / `SQFFY_LANG`) |
| ⚡ | **One-shot** | Скрипты `-c` для CI, автоматизации и e2e |

---

## 📦 Установка

```bash
# из исходников
git clone <your-repo-url> sqffy
cd sqffy
cargo install --path .

# или сразу запустить
cargo run --release --bin sqf
```

<details>
<summary><b>🦀 Feature-флаги</b></summary>

| Feature | По умолчанию | Заметки |
| :--- | :---: | :--- |
| `sqlite` | ✅ | Bundled `rusqlite` |
| `postgres` | ✅ | Sync-крейт `postgres` |
| `mysql` | ✅ | Sync-крейт `mysql` |
| `mariadb` | ✅ | MySQL wire + MariaDB provision |
| `clickhouse` | ✅ | HTTP через `ureq` |
| `oracle` | ✅ | крейт `oracle` (Instant Client) |
| `e2e` | ❌ | Только для `cargo e2e` |

```bash
# сборка только с SQLite
cargo build --release --no-default-features --features sqlite
```

</details>

---

## 🚀 Быстрый старт

```bash
sqf                          # интерактивный пикер
sqf --lang ru                # русский UI
sqf ./path/to/db.sqlite      # открыть файл SQLite
sqf postgres://u:p@localhost:5432/app
sqf mysql://u:p@localhost:3306/app
sqf mariadb://u:p@localhost:3306/app
sqf clickhouse://u:p@localhost:8123/default
sqf oracle://u:p@localhost:1521/XEPDB1
sqf --lang zh
sqf --list-drivers
sqf --no-pager --no-save :memory: -c "SELECT 1 AS ok;"
```

<div align="center">

| Без аргумента | SQffy показывает пикер |
| :---: | :--- |
| **1** | Подключиться к сохранённой БД |
| **2** | Добавить существующую БД |
| **3** | Создать новую (файл или поднять сервер) |

Подключения хранятся в конфиге ОС: `connections.toml`  
*(для тестов: `SQFFY_HOME`)*

</div>

---

## 📖 Использование

### Meta-команды

| Команда | Описание |
| :--- | :--- |
| `.help` / `.h` | Справка |
| `.tables` / `.t` | Список таблиц и представлений |
| `.schema` `[table]` | Индекс схем или полный inspect |
| `.describe` `<table>` | Список колонок |
| `.generate` `<table> <n> [--yes] [--dry-run] [col=ref.col …]` | Заполнить фейковыми данными |
| `.quit` / `.q` | Выход |

### Полезные флаги

| Флаг | Смысл |
| :--- | :--- |
| `-c` / `--command` | Выполнить SQL / meta и выйти |
| `-d` / `--driver` | Принудительно выбрать драйвер |
| `--limit N` | Лимит строк (`0` = без лимита) |
| `--no-pager` | Отключить пейджер |
| `--no-save` | Не трогать каталог подключений |
| `--mcp-port PORT` | MCP HTTP параллельно с REPL |
| `--lang en\|ru` | Язык интерфейса |

---

## 🗄️ Базы данных

```bash
# SQLite
sqf ./app.db
sqf sqlite:./app.db
sqf :memory:

# PostgreSQL
sqf postgres://sqffy:sqffy@127.0.0.1:5432/sqffy

# MySQL
sqf mysql://sqffy:sqffy@127.0.0.1:3306/sqffy

# MariaDB
sqf mariadb://sqffy:sqffy@127.0.0.1:3306/sqffy

# ClickHouse (HTTP)
sqf clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy

# Oracle (нужен Instant Client)
sqf oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1
```

При создании **серверной** БД, если локально ничего не слушает, SQffy предложит:

1. 🐳 **Поднять в Docker** (pull + run, анимированный прогресс)
2. ⬇️ **Скачать / установить** (winget на Windows или официальная страница)
3. 🔗 **Ввести уже готовый URL**

Как добавить ещё один бэкенд: [docs/DRIVERS.md](./docs/DRIVERS.md).

---

## 🔌 MCP

Read-only инструменты для агентов и IDE.

```bash
# stdio (по умолчанию)
sqf mcp

# HTTP JSON-RPC
sqf mcp --port 8787
# → http://127.0.0.1:8787/mcp
# → GET /health
```

| Инструмент | Назначение |
| :--- | :--- |
| `list_connections` | Открытые подключения сессии |
| `list_tables` | Таблицы / представления |
| `describe_table` | Колонки |
| `inspect_table` | Колонки + constraints + indexes |
| `execute_readonly` | `SELECT` / `WITH` / `SHOW` / `EXPLAIN` / `PRAGMA` … |
| `list_drivers` | Встроенные драйверы |

`DELETE` / `INSERT` / `DROP` и подобное отсекает guard.

---

## 🌍 Локализация

```bash
sqf --lang ru
SQFFY_LANG=ru sqf
```

Порядок: `--lang` → `SQFFY_LANG` → системная локаль → `en`.

Подробнее: [docs/I18N.md](./docs/I18N.md).

---

## 🧪 Тесты и CI

```bash
cargo test          # только юнит-тесты
cargo e2e           # e2e CLI (SQLite / MCP / meta)
cargo e2e-all       # + серверные адаптеры (нужен SQFFY_E2E_*)
```

| Переменная | Пример |
| :--- | :--- |
| `SQFFY_E2E_POSTGRES` | `postgres://sqffy:sqffy@127.0.0.1:5432/sqffy` |
| `SQFFY_E2E_MYSQL` | `mysql://sqffy:sqffy@127.0.0.1:3306/sqffy` |
| `SQFFY_E2E_MARIADB` | `mariadb://sqffy:sqffy@127.0.0.1:3307/sqffy` |
| `SQFFY_E2E_CLICKHOUSE` | `clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy` |
| `SQFFY_E2E_ORACLE` | `oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1` |

Docker-стек (Linux):

```bash
docker compose -f e2e/docker-compose.yml up --build --abort-on-container-exit
```

Независимые джобы CI: **Test** · **Build** · **E2E** — [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

---

## 📚 Документация

| Документ | Тема |
| :--- | :--- |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Слои крейта |
| [docs/DRIVERS.md](./docs/DRIVERS.md) | Новый драйвер БД |
| [docs/I18N.md](./docs/I18N.md) | Каталог локализации |

---

## 🗺️ Структура проекта

```text
sqffy/
├── src/           # CLI, драйверы, REPL, MCP, generate, provision
├── locales/       # en.toml · ru.toml
├── tests/         # e2e_* (feature = e2e)
├── e2e/           # Docker Compose + Linux runner
└── .github/       # CI: test · build · e2e
```

---

<div align="center">

### Для терминала · на Rust · лицензия MIT

**[⬆ наверх](#-sqffy)**

</div>

<div align="center">

# SQffy

### Простая CLI для SQL-баз. В терминале, который у вас уже открыт.

Windows · Linux · macOS

<p>
  <a href="#базы"><img alt="SQLite" src="https://img.shields.io/badge/SQLite-003B57?style=flat-square&logo=sqlite&logoColor=white"></a>
  &nbsp;
  <a href="#базы"><img alt="PostgreSQL" src="https://img.shields.io/badge/PostgreSQL-4169E1?style=flat-square&logo=postgresql&logoColor=white"></a>
  &nbsp;
  <a href="#базы"><img alt="MySQL" src="https://img.shields.io/badge/MySQL-4479A1?style=flat-square&logo=mysql&logoColor=white"></a>
  &nbsp;
  <a href="#базы"><img alt="MariaDB" src="https://img.shields.io/badge/MariaDB-C32136?style=flat-square&logo=mariadb&logoColor=white"></a>
  &nbsp;
  <a href="#базы"><img alt="ClickHouse" src="https://img.shields.io/badge/ClickHouse-FFCC01?style=flat-square&logo=clickhouse&logoColor=black"></a>
  &nbsp;
  <a href="#базы"><img alt="Oracle" src="https://img.shields.io/badge/Oracle-F80000?style=flat-square&logo=oracle&logoColor=white"></a>
</p>

[English](./README.md) · **Русский** · [中文](./README.zh.md)

<br>

[Скорость](#быстрая-работа) ·
[Таблицы](#удобные-таблицы) ·
[Агенты](#безопасный-mcp-для-ai-агентов) ·
[Старт](#лёгкий-старт) ·
[Установка](#установка) ·
[Справочник](#справочник)

</div>

SQffy — простая CLI-утилита для работы с SQL-базами через терминал. Подключились, написали запрос, прочитали ответ.

SQLite, PostgreSQL, MySQL, MariaDB, ClickHouse и Oracle уже внутри. Для любой другой базы можно дописать адаптер: тот же шелл, те же команды. Чеклист — в [docs/DRIVERS.md](./docs/DRIVERS.md).

---

## Быстрая работа

Утилита написана на Rust, и шелл не тянется. Подсветка синтаксиса показывает запрос ещё до Enter. Автодополнение подставляет таблицы и колонки в строку — меньше вспоминать, быстрее работать с базой.

<p align="center">
  <img src="./docs/media/autocomplete.png" width="361" alt="Автодополнение в приглашении prod.db: documents, files и users прямо во время набора SELECT">
</p>

После `FROM` в списке таблицы. После имени таблицы — колонки.

---

## Удобные таблицы

Схему базы и результат запроса SQffy рисует, а не вываливает сырым текстом. Под именем колонки стоит её тип — тот, который сообщил движок. `.schema` собирает колонки, ключи, индексы и DDL на один экран.

<p align="center">
  <img src="./docs/media/results.png" width="426" alt="Результат SELECT: под именами колонок напечатаны INTEGER и TEXT">
</p>

<p align="center">
  <img src="./docs/media/schema.png" width="662" alt="Схема таблицы users: колонки, первичный ключ, индексы и подсвеченный CREATE TABLE">
</p>

Нужны строки для теста или демо? `.generate` наполнит таблицу: генерация данных для тестирования и визуализации. Сначала печатается превью. `--yes` вставляет. Сотня строк и больше без `--yes` отклоняется.

---

## Любой терминал

Windows, Linux и macOS. Интерфейс — обычный терминал, тот, который уже открыт. Отдельная оконная программа не нужна.

---

## Безопасный MCP для AI-агентов

Для современной разработки, где в редакторе сидит агент, у SQffy есть MCP-сервер. Это безопасная дверь.

Агент смотрит схему и выполняет чтение: `SELECT`, `WITH`, `SHOW`, `EXPLAIN`, `PRAGMA`. Опасный запрос до базы не доходит. `INSERT`, `UPDATE`, `DELETE`, `DROP` и остальная запись отклоняются только на этом сервере. В самом терминале те же команды выполняются, как вы их написали. HTTP слушает только `127.0.0.1`.

```bash
sqf mcp                      # stdio, для редактора или агента
sqf mcp --port 8787          # http://127.0.0.1:8787/mcp   ·   GET /health
sqf --mcp-port 8787 ./app.db # тот же сервер рядом с приглашением
```

---

## Лёгкий старт

База создаётся в пару шагов — даже если нужного движка на компьютере ещё нет. Запустили `sqf` и выбрали **создать**.

Если локально никто не слушает, SQffy предложит скачать образ и поднять его в Docker или поставить пакет за вас (winget на Windows). Если ни тот, ни другой путь не подходит, откроет официальную страницу. Приходить с уже установленным Postgres не обязательно.

В пикере три хода:

1. Подключиться к сохранённой базе
2. Добавить существующую
3. Создать новую — файл или сервер

```bash
sqf                              # пикер
sqf ./app.db                     # сразу к файлу SQLite
sqf postgres://USER:PASSWORD@localhost:5432/app
```

```text
.tables
.schema users
SELECT id, name FROM users WHERE id >= 5;
.generate users 1000 --dry-run
```

Подключения запоминаются в `connections.toml`. `--no-save` файл не трогает.

| ОС | Путь |
| :--- | :--- |
| Linux | `~/.config/sqffy/connections.toml` |
| macOS | `~/Library/Application Support/dev.SQffy.sqffy/connections.toml` |
| Windows | `%APPDATA%\SQffy\sqffy\config\connections.toml` |

Тесты уводят каталог в другое место через `SQFFY_HOME`.

---

## Установка

Нужен Rust 1.85 или новее. Из каталога этого репозитория:

```bash
cargo install --path .

# или запустить без установки
cargo run --release --bin sqf
```

SQLite уже внутри бинарника. Для Oracle на машине нужен [Instant Client](https://www.oracle.com/database/technologies/instant-client.html).

```bash
sqf --lang ru                   # en · ru · zh
SQFFY_LANG=zh sqf
```

Порядок языка: `--lang`, затем `SQFFY_LANG`, затем локаль системы, затем английский. Строки интерфейса — в [docs/I18N.md](./docs/I18N.md).

<details>
<summary><b>Feature-флаги</b></summary>

| Feature | По умолчанию | |
| :--- | :---: | :--- |
| `sqlite` | да | SQLite в составе бинарника |
| `postgres` | да | PostgreSQL |
| `mysql` | да | MySQL |
| `mariadb` | да | MariaDB, включая локальный запуск |
| `clickhouse` | да | ClickHouse по HTTP |
| `oracle` | да | Oracle через Instant Client |
| `e2e` | нет | Только для `cargo e2e` |

```bash
cargo build --release --no-default-features --features sqlite
```

</details>

---

## Справочник

### Meta-команды

| Команда | |
| :--- | :--- |
| `.help` &nbsp; `.h` | Список команд |
| `.tables` &nbsp; `.t` | Таблицы и представления |
| `.schema` `[table]` | Индекс имён или одна таблица целиком |
| `.describe` `<table>` | Колонки |
| `.generate` `<table> <n> [--yes] [--dry-run] [col=ref.col …]` | Генерация данных для тестирования и визуализации |
| `.quit` &nbsp; `.q` | Выход |

### Флаги

| Флаг | |
| :--- | :--- |
| `-c`, `--command` | Выполнить SQL или meta-команду и выйти |
| `-d`, `--driver` | Задать драйвер явно |
| `--limit N` | Сколько строк печатать. `0` — все. По умолчанию 100. |
| `--no-pager` | Выключить пейджер |
| `--no-save` | Не записывать это подключение в каталог |
| `--mcp-port PORT` | Безопасный MCP по HTTP рядом с сессией |
| `--lang en\|ru\|zh` | Язык интерфейса |
| `--list-drivers` | Список драйверов и выход |

### Базы

| Движок | Как открыть | Примечание |
| :--- | :--- | :--- |
| SQLite | `sqf ./app.db` · `sqf sqlite:./app.db` · `sqf :memory:` | Внутри бинарника. Новый файл — это просто путь. |
| PostgreSQL | `postgres://USER:PASSWORD@localhost:5432/app` | Docker или установщик из пикера |
| MySQL | `mysql://USER:PASSWORD@localhost:3306/app` | Docker или установщик из пикера |
| MariaDB | `mariadb://USER:PASSWORD@localhost:3306/app` | Docker или установщик из пикера |
| ClickHouse | `clickhouse://USER:PASSWORD@localhost:8123/default` | HTTP. Docker или установщик из пикера |
| Oracle | `oracle://USER:PASSWORD@localhost:1521/XEPDB1` | Нужен Instant Client |

### Инструменты MCP

| Инструмент | |
| :--- | :--- |
| `list_connections` | Подключения, открытые в этой сессии |
| `list_tables` | Таблицы и представления |
| `describe_table` | Колонки |
| `inspect_table` | Колонки, ограничения, индексы |
| `execute_readonly` | Запрос на чтение. Запись отклоняется. |
| `list_drivers` | Драйверы в этой сборке |

---

## Документация

| | |
| :--- | :--- |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Как устроен крейт |
| [docs/DRIVERS.md](./docs/DRIVERS.md) | Свой адаптер базы |
| [docs/I18N.md](./docs/I18N.md) | Новый язык |

<details>
<summary><b>Разработка</b></summary>

```bash
cargo test       # юнит-тесты
cargo e2e        # e2e CLI: SQLite, MCP, meta-команды
cargo e2e-all    # плюс серверные адаптеры — сначала задайте SQFFY_E2E_*
```

| Переменная | Пример |
| :--- | :--- |
| `SQFFY_E2E_POSTGRES` | `postgres://sqffy:sqffy@127.0.0.1:5432/sqffy` |
| `SQFFY_E2E_MYSQL` | `mysql://sqffy:sqffy@127.0.0.1:3306/sqffy` |
| `SQFFY_E2E_MARIADB` | `mariadb://sqffy:sqffy@127.0.0.1:3307/sqffy` |
| `SQFFY_E2E_CLICKHOUSE` | `clickhouse://sqffy:sqffy@127.0.0.1:8123/sqffy` |
| `SQFFY_E2E_ORACLE` | `oracle://sqffy:sqffy@127.0.0.1:1521/XEPDB1` |

Эти URL принадлежат Docker-стенду для тестов.

```bash
docker compose -f e2e/docker-compose.yml up --build --abort-on-container-exit
```

CI гоняет **Test**, **Build** и **E2E** отдельными джобами. Воркфлоу: [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

```text
sqffy/
├── src/        CLI, драйверы, REPL, MCP, generate, provision
├── locales/    en.toml · ru.toml · zh.toml
├── docs/       драйверы, i18n, скриншоты
├── tests/      e2e_*  (feature = e2e)
├── e2e/        Docker Compose
└── .github/    CI
```

</details>

---

<div align="center">

<a href="https://www.rust-lang.org/"><img alt="Rust 1.85+" src="https://img.shields.io/badge/Rust-1.85+-DEA584?style=flat-square&logo=rust&logoColor=white"></a>
&nbsp;
<a href="./LICENSE"><img alt="Лицензия MIT" src="https://img.shields.io/badge/license-MIT-22C55E?style=flat-square"></a>

<br><br>

**[↑ Наверх](#sqffy)**

</div>

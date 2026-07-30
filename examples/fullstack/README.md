# Пример: Fullstack-приложение

Монорепозиторий с Rust (axum) API и Vite (React/TS) фронтендом, плюс Postgres и
Redis в Docker. Показывает все основные возможности Dev Manager.

## Структура

```
fullstack/
├── dm.yaml              # конфиг Dev Manager
├── .env                 # единый .env (с секциями)
├── docker-compose.yml   # postgres + redis
├── migrations/          # SQL-миграции БД
│   ├── up.sql
│   └── seed.sql
└── services/
    ├── api/             # Rust (axum), порт 8080
    │   ├── Cargo.toml
    │   └── src/main.rs
    └── web/             # Vite (React/TS), порт 5173
        ├── package.json
        └── src/
```

## Быстрый старт

```sh
# 1. Инициализируйте сервисы (создайте структуру или используйте dm new):
dm new service api --lang=rust
dm new service web --lang=vite

# 2. Bootstrap: установить зависимости + .env + поднять инфру:
dm setup

# 3. Запустить всё с hot-reload:
dm start
```

Логирование пойдёт в одну консоль:
```
[dm] запуск проекта 'fullstack-app' — 2 сервис(ов)
[dm]   • api [rust] → cargo run
[dm]   • web [vite] → npm run dev
[api]  SYS запуск: cargo run
[api]  OUT     Compiling api v0.1.0
[web]  SYS запуск: npm run dev
[web]  OUT   VITE v5.x  ready in 300 ms
...
```

## Типовые операции

```sh
# Остановить всё (Ctrl+C в dm start, или в другом терминале):
dm docker down

# Пересоздать БД с нуля (drop → migrate → seed):
dm alias reset

# Зайти в psql:
dm alias dbq

# Перезапустить только API (watcher сделает это при изменении .rs):
dm restart api

# Коммит во все репозитории:
dm commit "feat: новый эндпоинт"

# Поднять только бэкенд (профиль):
dm start --profile=api-only

# Запуск тестов API при каждом изменении (заявлено в tests.on_change):
dm test api
```

## Как это устроено в dm.yaml

- **`defaults:`** — общий `watch: true` для всех сервисов (не дублируем в каждом).
- **`depends_on: [api]`** у web — web не стартует, пока api не пройдёт health-check.
- **`health:`** — TCP/HTTP проверки готовности.
- **`before_start:`** — миграции БД запускаются до старта процесса.
- **`profiles:`** — `dm start --profile=api-only` поднимет только api.
- **`aliases:`** — шорткаты `dm alias up` / `dm alias reset` / `dm alias dbq`.
- **`notify:`** — уведомление в Slack при крэше бэкенда.

## .env с секциями

Переменные группируются по сервисам; `dm env sync` раскидывает их:

```ini
LOG_LEVEL=info          # глобальная → во все сервисы

[api]                   # → только в services/api/.env
DATABASE_URL=postgres://app:app@localhost:5432/app
PORT=8080

[web]                   # → только в services/web/.env
VITE_API_URL=http://localhost:8080
```

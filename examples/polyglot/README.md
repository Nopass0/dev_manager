# Пример: Polyglot-проект (наследование + окружения)

Проект на 3 языках (Rust + Go + Python) с отладочной утилитой. Демонстрирует
расширенные возможности конфигурации: наследование (`extends`), окружения
(`dm.<env>.yaml`), `only_on`, профили.

## Структура

```
polyglot/
├── base.yaml          ← общая база (extends)
├── dm.yaml            ← основной (extends base.yaml)
├── dm.dev.yaml        ← оверлей для dev
├── dm.staging.yaml    ← оверлей для staging
└── services/
    ├── api/           ← Rust (axum)
    ├── worker/        ← Go
    ├── ml/            ← Python (FastAPI)
    └── debug-tool/    ← отладка (только в dev)
```

## Наследование (extends)

`dm.yaml` наследует `base.yaml` через `extends: base.yaml`. Это deep-merge:

```yaml
# base.yaml задаёт общее
defaults:
  watch: true
  language: rust
linter: { dr: true, kiss: true, ... }
```

```yaml
# dm.yaml extends базу и добавляет своё
extends: base.yaml
services:
  api:
    path: ./services/api
    language: rust          # переопределяет дефолт
```

Эффект: общие настройки в одном месте (DRY), specifics — в наследнике.

## Окружения (--env / DM_ENV)

Dev Manager подгружает `dm.<env>.yaml` поверх `dm.yaml`:

```sh
dm --env dev start        # → применится dm.dev.yaml
dm --env staging start    # → применится dm.staging.yaml
export DM_ENV=dev         # или через переменную
dm start
```

Порядок слияния: `base.yaml → dm.yaml → dm.<env>.yaml → defaults`.

### Что меняется по окружениям

```yaml
# dm.dev.yaml: api без оптимизаций, RUST_LOG=debug
services:
  api:
    run: "cargo run"
    env: { RUST_LOG: debug }

# dm.staging.yaml: release-сборка, RUST_LOG=info
services:
  api:
    run: "cargo run --release"
    env: { RUST_LOG: info }
```

## only_on: сервисы только для конкретных окружений

```yaml
debug-tool:
  path: ./services/debug-tool
  only_on: [dev]           # только в dev
```

В `staging` этот сервис автоматически **отфильтруется** при запуске:

```sh
dm --env dev start         # запустит api, worker, ml, debug-tool (4 сервиса)
dm --env staging start     # запустит api, worker, ml (3 — debug-tool отфильтрован)
```

## Запуск по окружениям

```sh
# Установить зависимости всех сервисов:
dm setup

# Запустить в dev (с debug-tool):
dm --env dev start

# Или через алиас (из dm.yaml):
dm alias dev
dm alias staging

# Проверить, какой набор сервисов виден в каждом окружении:
dm --env dev list services
dm --env staging list services
```

## Интерполяция переменных

В строковых полях работает `{{var}}` (из контекста) и `${VAR}` (из окружения):

```yaml
services:
  api:
    path: ./services/{{project_name}}-api   # → ./services/polyglot-platform-api
    run: "cargo run -- --db $DATABASE_URL"  # → подставится $DATABASE_URL из env
```

## Ресурсы процесса

```yaml
ml:
  resources:
    cpu_percent: 50      # лимит CPU (best-effort)
    memory_mb: 2048      # лимит RAM
```

Best-effort: где платформа позволяет (Job Objects на Windows, cgroups на Linux),
Dev Manager пытается ограничить ресурсоёмкий ML-сервис.

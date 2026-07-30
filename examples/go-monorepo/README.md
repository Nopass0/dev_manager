# Пример: Go-монорепозиторий

Несколько Go-сервисов с общим пакетом `shared/` в одном монорепозитории. Главное
преимущество с Dev Manager — `dm start --affected` перезапускает только те
сервисы, которые реально зависят от изменённого файла.

## Структура

```
go-monorepo/
├── dm.yaml
├── go.work              ← Go workspace (go 1.22+, мульти-модульный monorepo)
├── shared/              ← общая библиотека
│   ├── go.mod           ← module github.com/your-org/shared
│   └── auth.go
└── services/
    ├── api/             ← HTTP API, порт 8080
    │   ├── go.mod       ← module github.com/your-org/api
    │   └── main.go      ← import "github.com/your-org/shared"
    └── worker/          ← фоновый обработчик
        ├── go.mod
        └── main.go
```

## go.work (workspace)

Go 1.22+ поддерживает `go.work` для разработки нескольких модулей вместе:

```go
// go.work
go 1.22

use (
    ./shared
    ./services/api
    ./services/worker
)
```

Это позволяет сервисам импортировать `shared/` локально без publish в прокси.

## Запуск

```sh
dm setup          # go mod download для всех модулей
dm start          # запуск api + worker с hot-reload
```

## Умный перезапуск (--affected)

```sh
# Изменили shared/auth.go → dm перезапустит И api, И worker (оба зависят)
echo "// правка" >> shared/auth.go

# Изменили services/api/main.go → dm перезапустит ТОЛЬКО api
echo "// правка" >> services/api/main.go
```

Dev Manager строит граф импортов через tree-sitter и перезапускает только
затронутые сервисы — экономит время на больших монорепо.

```sh
dm start --affected       # запустить только затронутые git diff'ом
dm gen diagram            # посмотреть граф зависимостей как Mermaid
```

## Типовые операции

```sh
dm test                   # go test ./... во всех сервисах
dm build --release        # оптимизированные бинарники
dm lint                   # DRY/KISS/unused (tree-sitter для Go)
dm commit "feat: новая ручка"
dm alias t                # шорткат → dm test
dm alias b                # шорткат → dm build --release
```

## Профили

```sh
dm start --profile=api-only   # только HTTP API (без worker)
dm start --tag=http           # по тегу
dm start --only=worker        # конкретный сервис
```

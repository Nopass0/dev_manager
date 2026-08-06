# Команды `dm`

> Раздел: [Документация](./README.md)

## `dm init`
Создаёт `dm.yaml` в текущем каталоге из встроенного шаблона. Если файл уже
существует — пропускает.

## `dm start [--no-watch] [--no-restart]`
Запускает все сервисы в порядке `order`/`delay_ms`. Логи мультиплексируются в
одну консоль с цветными префиксами. **Ctrl+C** корректно останавливает всё
дерево процессов.
- `--no-watch` — отключить file-watcher.
- `--no-restart` — не поднимать упавшие процессы.

## `dm stop`
Останавливает сервисы. В текущей версии сервисы живут в рамках процесса
`dm start` — для остановки используйте Ctrl+C в нём. (Daemon-режим с PID-файлом
заложен на следующую итерацию.)

## `dm restart <svc>`
Перезапускает конкретный сервис (в текущей версии — подсказка; полный watcher
приходит в следующей итерации).

## `dm status`
Таблица сервисов и их статусов (`pending/starting/running/stopped/crashed/exited`).

## `dm logs [svc]`
Логи сервисов. В реальном времени стримятся из активного `dm start`.

## `dm commit [target] [message]`
Git-автоматизация:
- `dm commit "msg"` — коммитит во **все** репозитории одним сообщением.
- `dm commit <svc> "msg"` — только в репозиторий сервиса `<svc>`.
- `dm commit auto` — сообщение формируется из списка изменённых символов
  (функций/классов/структур) через tree-sitter.

Эквивалент `git add -A && git commit -m "msg"` для каждого репо. См.
[multi-repo.md](./multi-repo.md).

## `dm push`
Пушит все репозитории в их `origin`. Каждый — в свой remote.

## `dm test [svc]`
Запускает тесты. Использует `tests.cmd` из конфига; если не задано — дефолт для
языка (`cargo test`, `npm test`, `go test ./...`, `bun test`, `pytest`).

## `dm lint [svc]`
Анализ кода: DRY, KISS, поиск дубликатов и неиспользуемого кода. Включённые
проверки берутся из секции `linter:` в `dm.yaml`. См. [code-analysis.md](./code-analysis.md).

## `dm deploy <name>`
Запускает деплой по имени цели из секции `deploy:`. См. [deploy.md](./deploy.md).

## `dm cache clear`
Удаляет кэши сборок сервисов: `target`, `node_modules/.cache`, `.next/cache`,
`dist`, `build`, `__pycache__`, `.pytest_cache`.

## `dm env sync`
Распределяет единый `.env` по сервисам согласно секциям `[service]`. См.
[env-sync.md](./env-sync.md).

## `dm install`
Устанавливает текущий бинарник в системный PATH (`%LOCALAPPDATA%\Programs\dm`
на Windows, `~/.local/bin` на Unix). Идемпотентно.

## `dm version`
Выводит версию и информацию о сборке.

## Глобальные опции
- `--help` / `-h` — помощь по команде.
- `RUST_LOG=debug` — детальное логирование внутренних компонент `dm`.

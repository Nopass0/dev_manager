# Журнал изменений / Changelog

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/),
версионирование — [SemVer](https://semver.org/lang/ru/).

## [0.7.0] — 2026-07-30

### 🏗 Шаблоны проектов
- **`dm init --template=<name>`** — создание готового проекта одной командой с
  рабочим health-эндпоинтом, структурой и автозаписью в dm.yaml.
- **7 встроенных шаблонов**: bun-elysia, bun-express, go-api, rust-axum,
  python-fastapi, next-shadcn, react-vite.
- **`dm init --list-templates`** — список доступных шаблонов.
- **`dm new service <name> --template=`** — добавление сервиса с автозаписью
  правильного `path: ./<name>` в dm.yaml.

### 🧮 Лимиты ресурсов
- **`resources.memory_mb`** + **`resources.on_exceed: notify|kill`** — мониторинг
  RSS процесса (кросс-платформенно: /proc на Linux, wmic на Windows, ps на macOS).
- При превышении — toast-уведомление; при `kill` — перезапуск сервиса.
- Модуль `dm-runtime/monitor` с `check_memory`/`rss_mb`.

### 🔔 Уведомления (Windows UX)
- **Toast-уведомления** (BurntToast → BalloonNotify fallback) вместо модального
  `msg`. Появляются сбоку и **исчезают сами** — не требуют закрытия кнопкой.

### 🐛 Качество
- Unused-линтер больше не флагает точки входа (`main`) и тесты (`Test*`, `test_*`,
  `setUp`/`tearDown` и фикстуры).
- `dm` без аргументов показывает help вместо ошибки.
- `.gitattributes` (LF) убирает CRLF-warnings в git на Windows.
- Oneliner-скрипты указывают на актуальный репозиторий `Nopass0/dev_manager`.

## [0.6.0] — 2026-07-29

### ✨ Команды для ускорения разработки
- **`dm setup`** — bootstrap всего проекта за один запуск: зависимости всех
  сервисов (cargo/npm/go/pip/dotnet) + `.env` sync + compose up.
- **`dm update`** — `git pull --ff-only` во всех репозиториях проекта.
- **`dm todo`** — реестр TODO/FIXME/HACK/XXX по коду таблицей (масштаб долга).
- **`dm alias <name>`** — пользовательские шорткаты из секции `aliases:` в dm.yaml.

### 🛠 Качество (эргономика конфига)
- **`#[serde(default)]` всем конфиг-структурам** — теперь можно указывать только
  нужные поля (например, только `dr: true` в `linter:`), отсутствующие берутся
  из дефолтов. Конфиги стали намного лаконичнее.
- **Единый кросс-платформенный shell-модуль** (`dm-cli/src/shell.rs`): убрано
  дублирование блоков `#[cfg]` в 5 командных файлах (DRY). Функции `run`/`capture`/
  `run_ok`/`run_argv` с платформо-нейтральным API.

### 📁 Примеры проектов (`examples/`)
Три рабочих проекта с валидируемыми `dm.yaml`:
- **fullstack** — Rust API + Vite + Postgres/Redis: профили, depends_on, health,
  before_start, aliases, notify;
- **multi-repo** — отдельные git-репозитории: cross-repo commit/push/update;
- **polyglot** — Rust + Go + Python: наследование (`extends`), окружения
  (`dm.<env>.yaml`), `only_on` (проверено: dev=4 сервиса, staging=3).

### 📖 Документация
- README переписан: recipes (типовые сценарии), таблица troubleshooting,
- матрица кросс-платформенности, ссылки на примеры.
- Каждый пример снабжён собственным README.

## [0.5.0] — 2026-07-29

### 🐛 Качество (критическое исправление supervisor'а)
- **Корректный жизненный цикл процесса**: `ManagedProcess::wait_exit()` теперь
  блокирует до естественного выхода процесса и возвращает код. Раньше supervisor
  **не замечал** завершение процесса (поллинг статуса, который не менялся).
- **Реальный auto-recovery**: после `max_consecutive_crashes` (5) падений подряд
  сервис останавливается (прерывание цикла рестартов), шлётся уведомление.
  Счётчик сбрасывается при uptime > 10с.
- **Полноценный watcher→supervisor**: `dm start` запускает `FileWatcher`, при
  изменении файлов вызывается `supervisor.notify_file_changed()` → флаг
  `restart_requested` → цикл через `tokio::select!` корректно убивает и
  поднимает процесс. Hot-reload **работает по-настоящему**.
- **Корректный shutdown**: не `.abort()` задачи (что бросало процессы-сироты),
  а ожидание завершения циклов, которые сами убивают свои процессы по флагу.
- `restart()`/`stop_service()` работают через флаги состояния (а не прямой kill).

### ✨ Новые возможности
- **`dm clean [--target=all|cache|branches|docker] [-y]`** — умная очистка: кэши
  сборок, слитые orphan-ветки (защита main/master), `docker system prune`.
- **`dm history`** — лента недавних коммитов по всем репозиториям проекта.
- **`dm list services|profiles|tags|deploy|databases`** — обзор сущностей
  проекта таблицами.
- **before_start хуки**: команды выполняются до запуска процесса сервиса
  (мigrations, codegen).
- **notify-интеграция в supervisor**: webhook/desktop при старте/крэше сервиса.

### ♻️ Рефакторинг
- `ProcessExit { code, killed_by_signal }` — модель исхода процесса.
- `NotifyConfig` хранится в `Supervisor`, передаётся в циклы сервисов.
- 80 unit-тестов; все core-сценарии supervisor'а проверены E2E.

## [0.4.0] — 2026-07-28

### ✨ Конфигурация (гибкость)
- **`extends:`** — наследование конфигов с deep-merge (скаляры/карты/векторы).
- **Окружения**: `dm.<env>.yaml` overlay + глобальный флаг `--env` и переменная
  `DM_ENV`. Порядок: base → dm.yaml → dm.<env>.yaml → defaults.
- **`{{var}}` / `${VAR}` интерполяция** при загрузке (контекст + переменные окружения).
- **`defaults:`** — глобальные значения по умолчанию для всех сервисов.
- **`only_on: [env…]`** — сервис активен только в заданных окружениях.
- Новые поля сервиса: `working_dir`, `shell`, `before_start`, `after_start`,
  `resources` (cpu/memory), расширенная `restart_policy`.

### ✨ Комплексные команды
- **`dm db migrate|seed|reset|shell`** — работа с БД (config-driven, postgres/
  sqlite/redis/mongo/mysql).
- **`dm docker up|down|logs|ps`** — управление compose-инфраструктурой (v2/v1
  автодетект, fallback compose-файлов).
- **`dm build [svc] [--release]`** — унифицированная сборка (cargo/go/npm/dotnet).
- **`dm gen diagram`** — Mermaid-диаграмма архитектуры из `depends_on` и реальных
  cross-service импортов.
- **`dm new` расширено**: `route`/`component`/`test`/`migration` шаблоны под язык.

### ✨ Уведомления
- Модуль `dm-runtime/notify`: webhook (JSON-POST, без зависимостей) + desktop
  (`notify-send`/`osascript`/`msg`). Конфиг `notify:` в dm.yaml.

### ♻️ Рефакторинг
- Loader переписан: `load_resolved_config` с extends/env-overlay/интерполяцией.
- 80 unit-тестов (+4): extends, env-overlay, only_on, interpolation, deep-merge.

## [0.3.0] — 2026-07-28

### ✨ Новые возможности

- **`dm grep <pattern>`** — поиск по коду проекта (`-i` ignore case, `-w` whole word,
  `-t ext1,ext2` фильтр расширений), вывод в стиле ripgrep.
- **`dm replace <old> <new>`** — find & replace с `--dry-run`.
- **`dm refs <symbol>`** — найти все использования символа (word-boundary).
- **`dm secrets`** — детектор утёкших секретов (AWS, Google API, JWT, PEM,
  credential assignments, connection strings) с маскированием вывода.
- **`dm format`** — единый прогон форматтеров (cargo fmt / prettier / gofmt / black).
- **`dm hooks install|uninstall|run`** — git-хуки pre-commit (format+lint) и
  pre-push (test).
- **`dm watch [svc] -- <cmd>`** — универсальный watcher-runner: повторяет команду
  при изменении файлов.
- **`dm config list|get|edit|validate`** — управление dm.yaml из CLI.
- **`dm new service <name> --lang=`** — скаффолд нового сервиса + автодобавление
  в dm.yaml (rust/go/ts/js/python).
- **`dm dashboard`** — live-дашборд сервисов с refresh каждые 3 с.
- **`dm ping <svc>` / `dm url <svc>`** — быстрые проверки доступности и URL.
- **`dm git stash|branch|rebase`** — cross-repo git-операции сразу по всем репо.

### ♻️ Рефакторинг

- Новые модули: `dm-analysis/{search,secrets,refs}`, `dm-vcs/multi`.
- 76 unit-тестов (+11): search, replace, secrets, refs.

## [0.2.0] — 2026-07-27

### ✨ Новые возможности

- **Профили запуска** `dm start --profile=min` — именованные наборы сервисов.
- **Теги сервисов** `dm start --tag=backend` — группировка и фильтрация.
- **`--only/--skip`** — явный выбор сервисов для запуска.
- **`--affected`** — запуск только сервисов, затронутых `git diff`, через новый
  модуль графа зависимостей (tree-sitter: Rust/JS/TS/Go imports).
- **`--dry-run`** — показать план запуска без выполнения.
- **`--wait`** — дождаться health-check всех сервисов (TCP/HTTP/none).
- **`depends_on` + `health:`** в конфиге — зависимости и проверки готовности.
- **`dm doctor`** — диагностика окружения с fix-подсказками.
- **`dm ports [--free=N]`** — управление занятыми портами.
- **`dm kill <target>`** — завершение по PID/порту/имени процесса.
- **`dm open <svc|docs|url>`** — открытие в браузере.
- **`dm exec <svc> -- <cmd>`** — команда в контексте сервиса (с .env).
- **`dm shell <svc>`** — интерактивная shell-сессия в каталоге сервиса.
- **`dm top`** — сводная таблица сервисов и их сетевого состояния.
- **`dm deps audit|outdated`** — единый аудит зависимостей (cargo/npm/go/python).
- **`dm release <patch|minor|major>`** — SemVer-bump + авто-CHANGELOG из
  Conventional Commits (с группировкой и BREAKING CHANGES).
- **`dm completions <shell>`** — генерация автодополнения для bash/zsh/fish/powershell.
- **Auto-recovery** (каркас): счётчик последовательных падений для прерывания
  цикла рестартов.

### ♻️ Рефакторинг

- Новые crate-модули: `dm-analysis/graph`, `dm-vcs/conventional|release|changelog`.
- Общий селектор сервисов `dm-cli/select` (DRY для start/test/lint).

## [0.1.0] — 2026-07-27

Первый рабочий milestone: ядро Dev Manager полностью реализовано и покрыто
тестами (52 unit-теста). Дальнейшие подсистемы заложены как расширяемые трейты.

### Добавлено

#### Ядро (`dm-core`)
- Разбор и валидация конфигурации `dm.yaml` (serde, версионирование, семантическая
  проверка).
- Поиск `dm.yaml` вверх по дереву каталогов (как у `git`).
- Единый `.env` с поддержкой секций `[service]` и глобальных переменных;
  команда `dm env sync` распределяет переменные по сервисам.
- Единый тип ошибок `DmError` и кросс-платформенные хелперы путей.

#### Оркестрация процессов (`dm-runtime`)
- Кросс-платформенный запуск сервисов с гарантированным убийством всего дерева
  подпроцессов (`kill_tree` — Job Objects на Windows, process groups на Unix).
- Supervisor с очередью запуска (`order`) и задержками (`delay_ms`).
- Построчный захват stdout/stderr и мультиплексирование в один поток с цветными
  префиксами `[service]`.
- File-watcher с debounce на базе `notify` (готов к подключению hot-reload).
- Автоопределение команды запуска по файлам-маркерам (`Cargo.toml`,
  `package.json`, `go.mod`…).

#### Git-автоматизация (`dm-vcs`)
- `dm commit "msg"` — коммит во все репозитории одним сообщением.
- `dm commit <svc> "msg"` — коммит в конкретный репозиторий (multi-repo).
- `dm commit auto` — сообщение формируется из списка изменённых символов
  (функций/классов/структур) через tree-sitter.
- `dm push` — пуш каждого репозитория в свой origin.

#### Анализ кода (`dm-analysis`)
- Единый трейт `LanguageParser` + реализации для Rust, JavaScript, TypeScript, Go.
- Извлечение функций/методов/классов/структур с сигнатурами и doc-комментариями
  (JSDoc, `///`, `/** */`).
- Линтеры: поиск дубликатов определений, DRY, KISS (число параметров / длина),
  неиспользуемый код.
- Сравнение символов до/после — основа `commit auto`.

#### Деплой (`dm-deploy`)
- Каркас: трейт `Deployer`, триггеры (`manual`/`after_commit`/`after_push`),
  заглушка-движок. Подключение `russh` — следующая итерация.

#### Установка (`dm-installer`)
- Установка бинарника в PATH: `%LOCALAPPDATA%\Programs\dm` (Windows) или
  `~/.local/bin` (Linux/macOS).
- Идемпотентное добавление в PATH (реестр на Windows, `.bashrc`/`.zshrc` на Unix).
- Oneliner-скрипты `scripts/install.sh` и `scripts/install.ps1`.

#### CLI (`dm`)
- Подкоманды: `init`, `start`, `stop`, `restart`, `status`, `logs`, `commit`,
  `push`, `test`, `lint`, `deploy`, `cache clear`, `env sync`, `install`, `version`.
- Цветной вывод через `anstream`/`anstyle`, таблицы через `comfy-table`.

### Запланировано (следующие итерации)
- Полноценная связка watcher→supervisor (перезапуск изменённого сервиса).
- Отслеживание exit code процесса и политики рестарта.
- `russh`-реализация деплоя.
- Расширенные линтеры (auto-fix неиспользуемого кода).
- Дополнительные языковые грамматики (C/C++/C#/Lua/Python).
- Daemon-режим с PID-файлом для `dm stop`/`dm logs` из другого терминала.

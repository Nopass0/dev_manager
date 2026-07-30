# Конфигурация `dm.yaml`

> Раздел: [Документация](./README.md)

Файл `dm.yaml` лежит в корне проекта и описывает все сервисы, деплой и линтер.
Dev Manager ищет его от текущего каталога вверх по дереву (как `git`), поэтому
`dm` можно запускать из любого подкаталога.

## Полная схема

```yaml
version: 1                       # только 1 (обязательно)
project_name: my-app             # человекочитаемое имя (в логах/статусе)
env_file: .env                   # путь к единому .env (по умолчанию .env)

services:                        # карта сервисов (см. ниже)
  <имя>:
    path: ./services/<имя>       # каталог сервиса (обязательно)
    language: rust               # язык/стек (обязательно)
    repo: ./services/<имя>       # отдельный git-репозиторий (опц.)
    run: cargo run               # явная команда запуска (опц., иначе авто)
    watch: true                  # отслеживать файлы (по умолчанию true)
    restart_on_change: true      # перезапускать при изменениях (по умолчанию true)
    delay_ms: 0                  # задержка перед запуском, мс (по умолчанию 0)
    order: 100                   # приоритет в очереди запуска (меньше = раньше)
    env:                         # доп. переменные окружения (опц.)
      KEY: value
    tests:
      cmd: cargo test            # команда тестов (пусто → тесты выключены)
      on_change: true            # прогонять тесты при изменениях
    logs:
      enabled: true              # показывать логи сервиса в общем потоке
      level: info                # минимальный уровень

deploy:                          # цели деплоя (опц.)
  - name: prod
    host: prod.example.com
    user: deploy
    port: 22
    key: ~/.ssh/id_ed25519
    remote_dir: /srv/my-app
    on: after_push               # manual | after_commit | after_push
    steps:                       # команды на удалённом хосте
      - git pull
      - cargo build --release
      - systemctl restart my-app

linter:                          # анализатор кода
  dr: true                       # проверка DRY
  kiss: true                     # проверка KISS
  unused_code: true              # поиск неиспользуемого кода
  duplicates: true               # поиск дубликатов определений
  auto_fix: false                # авто-удаление неиспользуемого кода
```

## Поля сервиса подробно

### `language`
Поддерживаемые значения: `rust`, `go`, `c`, `cpp`, `csharp`, `javascript`,
`typescript`, `bun`, `nodejs`, `lua`, `python`, `vite`, `nextjs`, `remix`,
`other`. От значения зависит автоопределение команды запуска и выбор грамматики
tree-sitter.

### `run` (опционально)
Если не задано — `dm` пытается угадать команду по файлам-маркерам:
- `package.json` → `npm run dev` (или `bun run dev` при `bun.lockb`);
- `Cargo.toml` → `cargo run`;
- `go.mod` → `go run .`;
- `*.csproj` → `dotnet run`;
- иначе — дефолт по языку (`go run .`, `python main.py`…).

### `order` и `delay_ms`
- `order` — целое число; сервисы запускаются по возрастанию. При равенстве
  сохраняется порядок объявления в YAML.
- `delay_ms` — пауза перед запуском **следующего** сервиса в очереди.

### `repo` (multi-repo)
Если сервис живёт в отдельном git-репозитории, укажите путь. Тогда:
- `dm commit "msg"` закоммитит во **все** репозитории одним сообщением;
- `dm commit <svc> "msg"` — только в этот;
- `dm push` зальёт каждый в свой `origin`.

См. [multi-repo.md](./multi-repo.md).

## Пример

Полный пример со всеми полями — в [`dm.example.yaml`](../../dm.example.yaml).

## Валидация

При загрузке `dm` проверяет:
- `version == 1` (иначе ошибка);
- наличие хотя бы одного сервиса;
- непустые `path` и валидные имена сервисов.

Существование каталогов сервисов проверяется отдельно (при `dm start`), чтобы
конфиг можно было загрузить и в `dm init`, когда каталогов ещё нет.

# Начало работы

> Раздел: [Документация](./README.md)

В этом руководстве вы установите `dm` и запустите первый проект за 5 минут.

## 1. Установка

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.ps1 | iex
```

**Linux / macOS:**
```sh
curl -fsSL https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.sh | sh
```

После установки **перезапустите терминал**, затем проверьте:
```sh
dm version
```

Если `dm` не найден — см. [installation.md](./installation.md#troubleshooting).

## 2. Создание конфига

В корне вашего монорепозитория:
```sh
dm init
```
Будет создан `dm.yaml` из шаблона. Отредактируйте его — минимум укажите сервисы:

```yaml
version: 1
project_name: demo
services:
  api:
    path: ./services/api
    language: rust
  web:
    path: ./services/web
    language: vite
    order: 20
    delay_ms: 500
```

## 3. Запуск

```sh
dm start
```

Все сервисы запустятся в порядке `order` (с учётом `delay_ms`). Логи пойдут в
одну консоль с цветными префиксами:

```
[dm] запуск проекта 'demo' — 2 сервис(ов)
[api]  SYS запуск: cargo run
[api]  OUT     Compiling demo v0.1.0
[web]  SYS запуск: npm run dev
...
```

**Ctrl+C** корректно погасит все процессы (включая подпроцессы).

## 4. Единый `.env` (опционально)

Создайте в корне `.env` с секциями:
```ini
LOG_LEVEL=info

[api]
DATABASE_URL=postgres://localhost/demo
PORT=3001

[web]
API_URL=http://localhost:3001
```

Распределите по сервисам:
```sh
dm env sync
#  ✓ api: записано 3 переменных в ./services/api/.env
#  ✓ web: записано 2 переменных в ./services/web/.env
```

Подробно — в [env-sync.md](./env-sync.md).

## 5. Git в одной команде

```sh
dm commit "feat: новый эндпоинт"   # коммит во все репозитории
dm push                            # пуш каждого в свой origin
```

Для multi-repo и авто-сообщений см. [multi-repo.md](./multi-repo.md).

## Что дальше

- [Конфигурация](./configuration.md) — все опции `dm.yaml`.
- [Команды](./commands.md) — полный список.
- [Анализ кода](./code-analysis.md) — DRY/KISS/unused.

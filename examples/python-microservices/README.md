# Пример: Python-микросервисы (FastAPI + Celery)

Python-стек: FastAPI как HTTP API, Celery worker для фоновых задач, Redis как
брокер сообщений. Демонстрирует автоопределение команд для Python и интеграцию
с compose-инфраструктурой.

## Структура

```
python-microservices/
├── dm.yaml
├── .env
├── docker-compose.yml      ← redis
└── services/
    ├── api/                ← FastAPI (uvicorn), порт 8000
    │   ├── requirements.txt
    │   └── main.py
    └── worker/             ← Celery worker
        ├── requirements.txt
        └── tasks.py
```

## Запуск

```sh
# 1. Bootstrap: pip install + .env + redis в Docker:
dm setup

# 2. Запуск api + worker + (redis уже из setup):
dm start
```

`dm setup` определит `requirements.txt` и выполнит `pip install -r requirements.txt`
для каждого сервиса, раскидает `.env` и поднимет Redis через compose.

## Автоопределение команд

Python-сервисы требуют явную команду `run:` (т.к. способов запуска много):

```yaml
api:
  language: python
  run: "uvicorn main:app --reload --port 8000"   # FastAPI
worker:
  language: python
  run: "celery -A tasks worker --loglevel=info"  # Celery
```

`--reload` у uvicorn даёт свою горячую перезагрузку; `dm` дополнительно
перезапустит процесс при существенных изменениях (watcher).

## Тесты

```yaml
api:
  tests:
    cmd: pytest
    on_change: true        # прогонять при каждом изменении .py
```

```sh
dm test api               # pytest в services/api
dm test                   # pytest во всех сервисах
```

## Линтер

```sh
dm lint                   # DRY/KISS/unused (tree-sitter для Python)
dm format                 # black . (если установлен)
```

## Redis

```sh
dm docker up              # поднять redis
dm alias rq               # шорткат → redis-cli
dm docker logs            # хвост логов redis
dm docker down            # остановить
```

## .env с секциями

```ini
LOG_LEVEL=info

[api]                     # → services/api/.env
REDIS_URL=redis://localhost:6379/0
DATABASE_URL=sqlite:///app.db

[worker]                  # → services/worker/.env
REDIS_URL=redis://localhost:6379/0
CELERY_BROKER=redis://localhost:6379/0
```

# Пример: Multi-repo (отдельные репозитории)

Когда микросервисы живут в **разных git-репозиториях**, Dev Manager объединяет
их одним `dm.yaml` для локальной разработки. Git-операции работают по всем репо
сразу — без ручного `cd` в каждый.

## Структура

```
multi-repo/
├── dm.yaml
└── repos/
    ├── auth/       ← git-репо  (git remote: git@github.com:org/auth.git)
    ├── billing/    ← git-репо  (git remote: git@github.com:org/billing.git)
    └── gateway/    ← git-репо  (git remote: git@github.com:org/gateway.git)
```

Каждый подкаталог — самостоятельный клонированный репозиторий. Ключевое поле:
`repo:` указывает, что это отдельный репозиторий (не корневой монорепо).

## Клонирование репозиториев

```sh
mkdir -p repos && cd repos
git clone git@github.com:your-org/auth.git
git clone git@github.com:your-org/billing.git
git clone git@github.com:your-org/gateway.git
cd ..
```

## Запуск

```sh
dm start
# [auth] запускается первым (order: 10)
# [billing] ждёт, пока auth не пройдёт health-check (depends_on)
# [gateway] ждёт auth и billing
```

## Git-операции по всем репозиториям

```sh
# Коммит сразу во ВСЕ репозитории одним сообщением:
dm commit "fix: общая правка аутентификации"
# → git add -A && git commit -m "..." в auth/, billing/, gateway/

# Коммит в конкретный репозиторий:
dm commit auth "fix: правка только в auth"

# Push: каждый репозиторий зальётся в СВОЙ origin:
dm push
# → git push в auth/, billing/, gateway/  (каждый в свой GitHub-проект)

# Вытянуть изменения во всех репо:
dm update
# → git pull --ff-only в каждом

# Cross-repo git-операции:
dm git stash              # спрятать изменения во всех репо
dm git branch feature/x   # создать ветку во всех репо
dm git rebase main        # ребейзить на main
```

## Авто-сообщение коммита (commit auto)

```sh
dm commit auto
```

`dm` проанализирует `git diff` каждого репо через tree-sitter и сформирует
читаемое сообщение с изменёнными символами:
```
auto: изменены 2 символ(ов)

- изменена функция verify (auth/src/jwt.rs)
- добавлена структура Invoice (billing/models.go)
```

## Профили и теги

```sh
dm start --tag=core        # только auth + billing
dm start --tag=edge        # только gateway
dm start --only=auth       # только auth
dm start --affected        # только затронутые git diff'ом
```

## Почему `repo:` важно

Без `repo:` Dev Manager считает сервис частью корневого монорепозитория и
коммитит в него. С `repo:` он понимает, что у сервиса свой `.git` — и git-операции
выполняются в правильном каталоге. Это позволяет коммитить в каждый репозиторий
одно сообщение, а пушить — каждый в свой `origin`.

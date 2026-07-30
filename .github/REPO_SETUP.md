# Настройка репозитория для продвижения (для владельца)

После первого push откройте настройки репозитория и примените следующее.

## Описание (About)

**Short description** (на странице репозитория → About):
```
Единый менеджер разработки микросервисов: оркестрация, git-автоматизация, анализ кода и деплой. Rust + tree-sitter. Windows/Linux.
```

**Website:** (если есть лендинг) или оставьте пустым.

## Topics (теги для поиска на GitHub)

Добавьте в About → Topics (до 20, важны для discoverability):

```
rust  cli  developer-tools  devtools  monorepo  microservices  developer-experience  dx
orchestrator  process-manager  task-runner  hot-reload  file-watcher  git  git-workflow
tree-sitter  static-analysis  code-analysis  dev-environment  cli-app
```

Дополнительно (по стекам):
```
cargo  npm  typescript  go  polyglot  multi-repo  changelog  conventional-commits
```

## Releases

1. Создайте tag `v0.6.0` и Release с описанием из `CHANGELOG.md`.
2. Приложите бинарные архивы (CI уже собирает их как artifacts):
   - `dm-x86_64-pc-windows-msvc.zip`
   - `dm-x86_64-unknown-linux-musl.tar.gz`
3. Oneliner'ы в `scripts/install.{sh,ps1}` ссылаются на `releases/latest/download/...`.

## GitHub Pages (опционально)

`cargo doc --workspace` → опубликуйте `target/doc/` через GitHub Pages для
онлайн-документации API.

## SEO-заметки

- README начинается с баннера и бейджей (есть);
- английская версия README.en.md расширяет аудиторию;
- topics выше покрывают основные поисковые запросы разработчиков;
- 5 рабочих примеров в `examples/` повышают ценность и делятся ссылками.

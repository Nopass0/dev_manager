# Руководство для контрибьюторов

Спасибо за интерес к Dev Manager! Этот документ описывает, как добавлять свои
изменения в проект — от исправления опечатки до новой команды.

[English](#contributing-english) · [Кодекс поведения](#кодекс-поведения)

---

## Быстрый старт для разработчиков

### Требования
- **Rust nightly 1.93** (закреплён в `rust-toolchain.toml` — `rustup` скачает сам);
- **C-компилятор**: MSVC Build Tools (Windows, собирайте из *x64 Native Tools
  Command Prompt*) или gcc/clang (Linux/macOS) — нужен для tree-sitter-грамматик;
- **git** в PATH.

### Клонирование и сборка
```sh
git clone https://github.com/Nopass0/dev_manager.git
cd dev_manager
cargo build                 # debug-сборка: target/debug/dm[.exe]
cargo test --workspace      # все unit-тесты (83+)
cargo doc --workspace --open # HTML-документация всех crate'ов
```

Проверьте, что debug-бинарник работает:
```sh
./target/debug/dm version
```

---

## Структура проекта

Workspace из 7 crate'ов с чёткими границями (каждый добавляйте в правильный):

```
crates/
├── dm-core/       конфиг dm.yaml, единый .env, модель проекта, ошибки
│   └── src/config/schema.rs    ← НОВЫЕ ПОЛЯ КОНФИГА сюда
├── dm-runtime/    оркестрация процессов, watcher, notify, логи
├── dm-cli/        бинарь dm: команды, цветной вывод, shell-абстракция
│   └── src/commands/<cmd>.rs   ← НОВАЯ КОМАНДА сюда
├── dm-vcs/        git (через CLI), commit/push, semver, changelog
├── dm-analysis/   tree-sitter: символы, search, graph, lints
├── dm-deploy/     SSH-деплой (russh)
└── dm-installer/  установка в PATH, oneliner-скрипты
```

## Как добавить новую команду

Самый частый сценарий — добавление команды `dm <something>`.

### 1. Определите аргументы
В `crates/dm-cli/src/commands/mod.rs` добавьте структуру:
```rust
#[derive(Debug, Clone, clap::Args)]
pub struct MyCmdArgs {
    /// Что сделать.
    pub target: String,
    /// Опциональный флаг.
    #[arg(long)]
    pub verbose: bool,
}
```

### 2. Зарегистрируйте команду
В `crates/dm-cli/src/lib.rs`:
- добавьте вариант в `enum Command`:
  ```rust
  /// Описание для --help.
  MyCmd(commands::MyCmdArgs),
  ```
- добавьте диспатч в `fn run`:
  ```rust
  Command::MyCmd(args) => commands::my_cmd::run(args).await,
  ```

### 3. Создайте модуль команды
В `crates/dm-cli/src/commands/mod.rs` — `pub mod my_cmd;`, затем создайте файл
`crates/dm-cli/src/commands/my_cmd.rs`:
```rust
//! `dm my-cmd <target>` — что делает команда.

use crate::commands::{load_project_config, MyCmdArgs};
use crate::output::print_system;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(args: MyCmdArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    print_system(&format!("my-cmd: {}", args.target));
    // ... бизнес-логика ...
    Ok(())
}
```

### 4. Используйте общий shell-модуль
Для запуска внешних команд используйте `crate::shell` (кросс-платформенно):
```rust
use crate::shell;
let code = shell::run("npm test", &dir)?;
```
**Не дублируйте** блоки `#[cfg(windows)]` — `shell.rs` уже скрывает различия.

### 5. Добавьте тесты и документацию
- `///` rustdoc на каждой публичной функции/структуре (обязательно);
- unit-тесты в `#[cfg(test)] mod tests`;
- если команда значимая — упомяните в `README.md` (таблица команд) и
  `docs/ru/commands.md`.

## Как добавить новое поле конфига

1. В `crates/dm-core/src/config/schema.rs` добавьте поле в нужную структуру с
   `#[serde(default)]` (чтобы не ломать существующие конфиги):
   ```rust
   #[serde(default)]
   pub my_field: Option<String>,
   ```
2. Если поле нужно в runtime — пробросьте через `project_from_config` в
   `crates/dm-runtime/src/supervisor.rs`.
3. Обновите `dm.example.yaml` и документацию в `docs/ru/configuration.md`.

## Как добавить поддержку нового языка (tree-sitter)

1. Добавьте grammar-crate в `crates/dm-analysis/Cargo.toml`:
   ```toml
   tree-sitter-lua = "0.23"
   ```
2. Создайте `crates/dm-analysis/src/languages/lua.rs` по образцу `rust.rs`
   (реализуйте трейт `LanguageParser`).
3. Зарегистрируйте в `crates/dm-analysis/src/parser.rs` (`parser_for_extension`).
4. Добавьте расширения в `dm-core/src/project.rs` (`source_extensions`).
5. Напишите тест парсинга (по образцу `parses_rust_function_and_struct`).

## Стиль кода

- **DRY, KISS** — без дублирования, простые решения;
- все публичные API — с `///` rustdoc-комментариями;
- единая система ошибок через `dm_core::DmError` (не строки);
- cross-platform: платформенные детали — в `#[cfg]` или в общий `shell.rs`;
- `cargo fmt` + `cargo clippy` перед коммитом.

## Workflow отправки изменений

1. **Fork** репозитория и создайте ветку:
   ```sh
   git checkout -b feat/my-feature
   ```
2. Коммитьте малыми логическими порциями, используя
   [Conventional Commits](https://www.conventionalcommits.org/):
   ```
   feat(cli): добавить команду dm my-cmd
   fix(runtime): корректный exit-code tracking
   docs(readme): обновить таблицу команд
   ```
3. Убедитесь, что всё зелёное:
   ```sh
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
4. Откройте **Pull Request** в `main`. В описании укажите:
   - что делает изменение и зачем;
   - как тестировали;
   - breaking changes (если есть).

## Чек-лист перед PR

- [ ] `cargo test --workspace` проходит;
- [ ] `cargo clippy --workspace -- -D warnings` без замечаний;
- [ ] код отформатирован (`cargo fmt`);
- [ ] публичные API задокументированы (`///`);
- [ ] обновлены README/docs/examples, если поведение изменилось;
- [ ] добавлены тесты для новой функциональности.

## Отчёт о багах и идеи

Откройте [Issue](https://github.com/Nopass0/dev_manager/issues) с шаблоном:
- **Ожидаемое поведение** / **Фактическое**;
- шаги воспроизведения;
- ОС, версия Rust, версия `dm` (`dm version`);
- вывод `dm doctor` (если применимо).

---

## Кодекс поведения

Будьте уважительны и конструктивны. Мы следуем принципам
[Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
Оскорбления, домогательства и токсичное поведение недопустимы.

---

## Contributing (English)

Quick start: clone, `cargo build`, `cargo test --workspace`. Add commands in
`crates/dm-cli/src/commands/`, register in `lib.rs` enum + dispatch, use
`crate::shell` for cross-platform shell calls. Add config fields in
`dm-core/src/config/schema.rs` with `#[serde(default)]`. New languages:
implement `LanguageParser` trait + register in `parser.rs`.

Workflow: fork → branch (`feat/...`) → conventional commits →
`cargo fmt && cargo clippy -D warnings && cargo test` → PR to `main`.
See Russian section above for details.

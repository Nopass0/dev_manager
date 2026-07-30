//! `dm alias <name> [args...]` — пользовательские шорткаты из секции `aliases:`.
//!
//! Секция `aliases:` в dm.yaml задаёт именованные команды (shell-строки),
//! которые выполняются в корне проекта. Ускоряет типовые операции:
//!
//! ```yaml
//! aliases:
//!   dbq: "dm db shell --conn=api"
//!   re: "dm restart api"
//!   bs: "dm build api --release"
//! ```
//! Запуск: `dm alias dbq` → выполнит `dm db shell --conn=api` в корне проекта.

use crate::commands::load_project_config;
use crate::output::{print_system, success_style, warn_style, println_styled};
use crate::shell;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(name: &str, args: &[String]) -> DmResult<()> {
    let (config, root) = load_project_config()?;

    let command = config
        .aliases
        .get(name)
        .cloned()
        .ok_or_else(|| {
            let available: Vec<&str> = config.aliases.keys().map(|s| s.as_str()).collect();
            dm_core::DmError::invalid_config(format!(
                "алиас '{name}' не найден. Доступно: [{}]",
                available.join(", ")
            ))
        })?;

    // Если переданы доп. аргументы — дописываем их в конец команды (как args).
    let full = if args.is_empty() {
        command
    } else {
        format!("{command} {}", args.join(" "))
    };
    print_system(&format!("alias {name} → {full}"));
    match shell::run(&full, &root) {
        Ok(0) => println_styled(&format!("✓ {name} выполнен"), success_style()),
        Ok(code) => println_styled(&format!("! {name}: код {code}"), warn_style()),
        Err(e) => println_styled(&format!("✗ {name}: {e}"), crate::output::error_style()),
    }
    Ok(())
}

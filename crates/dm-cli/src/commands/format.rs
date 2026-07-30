//! `dm format` — единый прогон форматтеров по всем сервисам.
//!
//! По языку сервиса вызывает соответствующий системный форматтер:
//! - Rust → `cargo fmt`
//! - JS/TS/Vite/Next/Remix → `npx prettier --write .`
//! - Go → `gofmt -w .`
//! - Python → `black .`
//! - C/C++ → `clang-format -i` (если есть .clang-format)

use crate::commands::load_project_config;
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use std::process::Command;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    for (name, svc) in &config.services {
        let dir = dm_core::paths::resolve(&root, std::path::Path::new(&svc.path));
        let Some(cmd) = formatter_for(svc.language) else {
            continue;
        };
        print_system(&format!("форматирование '{name}': {cmd}"));
        let status = run_shell(cmd, &dir);
        match status {
            Ok(0) => println_styled(&format!("  ✓ {name} отформатирован"), success_style()),
            Ok(code) => println_styled(
                &format!("  ! {name}: форматтер вернул код {code}"),
                warn_style(),
            ),
            Err(e) => println_styled(&format!("  ✗ {name}: {e}"), crate::output::error_style()),
        }
    }
    Ok(())
}

/// Возвращает команду-форматтер по языку.
fn formatter_for(lang: ServiceLanguage) -> Option<&'static str> {
    match lang {
        ServiceLanguage::Rust => Some("cargo fmt"),
        ServiceLanguage::Go => Some("gofmt -w ."),
        ServiceLanguage::Python => Some("black ."),
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix
        | ServiceLanguage::Nodejs => Some("npx --no-install prettier --write ."),
        _ => None,
    }
}

/// Запускает shell-команду в каталоге `cwd` (синхронно; форматтеры быстрые).
fn run_shell(cmd: &str, cwd: &std::path::Path) -> Result<i32, String> {
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd]);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    };
    command.current_dir(cwd);
    command.stdin(std::process::Stdio::null());
    let status = command.status().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

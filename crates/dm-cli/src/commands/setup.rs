//! `dm setup` — bootstrap всего проекта за один запуск.
//!
//! Ускоряет онбординг и переключение между проектами: устанавливает зависимости
//! каждого сервиса соответствующим пакетным менеджером, синхронизирует единый
//! `.env`, поднимает compose-инфраструктуру. Одна команда вместо десятка.
//!
//! По языку вызывает:
//! - Rust → `cargo fetch`;
//! - Node-семейство → `npm ci` (или `bun install` при bun.lockb);
//! - Go → `go mod download`;
//! - Python → `pip install -r requirements.txt`;
//! - C# → `dotnet restore`.

use crate::commands::load_project_config;
use crate::output::{print_system, success_style, warn_style, println_styled};
use crate::shell;
use dm_core::project::ServiceLanguage;
use dm_core::DmResult;
use std::path::Path;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    print_system(&format!(
        "setup проекта '{}' ({} сервисов)",
        config.project_name,
        config.services.len()
    ));

    // 1. Единый .env → раскидать по сервисам.
    print_system("• синхронизация .env");
    if let Err(e) = sync_env(&config, &root) {
        println_styled(&format!("  ! .env: {e}"), warn_style());
    }

    // 2. Зависимости каждого сервиса.
    for (name, svc) in &config.services {
        let dir = shell::resolve_dir(&root, &svc.path);
        let Some(cmd) = install_cmd(svc.language, &dir) else {
            continue;
        };
        print_system(&format!("• {name}: {cmd}"));
        match shell::run(&cmd, &dir) {
            Ok(0) => println_styled(&format!("  ✓ {name} готов"), success_style()),
            Ok(code) => println_styled(
                &format!("  ✗ {name}: код {code}"),
                crate::output::error_style(),
            ),
            Err(e) => println_styled(&format!("  ✗ {name}: {e}"), crate::output::error_style()),
        }
    }

    // 3. compose-инфра (если есть).
    let compose = root.join(&config.docker.compose_file);
    if compose.exists() {
        print_system("• docker compose up -d");
        let _ = shell::run("docker compose up -d", &root);
    }

    println_styled("✓ setup завершён — можно запускать `dm start`", success_style());
    Ok(())
}

/// Возвращает команду установки зависимостей по языку (или None).
fn install_cmd(lang: ServiceLanguage, dir: &Path) -> Option<String> {
    match lang {
        ServiceLanguage::Rust => Some("cargo fetch".into()),
        ServiceLanguage::Go => Some("go mod download".into()),
        ServiceLanguage::Csharp => Some("dotnet restore".into()),
        ServiceLanguage::Python => {
            if dir.join("requirements.txt").exists() {
                Some("pip install -r requirements.txt".into())
            } else if dir.join("pyproject.toml").exists() {
                Some("pip install -e .".into())
            } else {
                None
            }
        }
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Nodejs
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix => {
            if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
                Some("bun install".into())
            } else {
                Some("npm ci".into())
            }
        }
        ServiceLanguage::Bun => Some("bun install".into()),
        _ => None,
    }
}

/// Синхронизирует единый .env (делегирует в env::sync без переподписки на команды).
fn sync_env(config: &dm_core::Config, root: &Path) -> dm_core::DmResult<()> {
    use dm_core::env::{parse_unified_env, write_service_env};
    let env_path = dm_core::paths::resolve(root, Path::new(&config.env_file));
    let content = std::fs::read_to_string(&env_path).unwrap_or_default();
    if content.trim().is_empty() {
        return Ok(());
    }
    let unified = parse_unified_env(&content)?;
    for (name, svc) in &config.services {
        let vars = unified.vars_for(name);
        if vars.is_empty() {
            continue;
        }
        let target = dm_core::paths::resolve(root, Path::new(&svc.path)).join(".env");
        write_service_env(&target, &vars)?;
    }
    Ok(())
}

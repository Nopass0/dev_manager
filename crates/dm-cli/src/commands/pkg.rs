//! `dm pkg add|remove|update|list|exec` — унифицированный пакетный менеджер.
//!
//! Автоопределяет пакетный менеджер для каждого сервиса:
//! - Bun (bun.lockb) → `bun add/remove/update`
//! - Node (package.json) → `npm install/uninstall/update`
//! - Rust (Cargo.toml) → `cargo add/remove/update`
//! - Go (go.mod) → `go get`
//! - Python (requirements.txt/pyproject.toml) → `pip install`
//!
//! Примеры:
//!   dm pkg add zod                  → добавить во все сервисы
//!   dm pkg add zod --service api    → только в api
//!   dm pkg remove lodash
//!   dm pkg update
//!   dm pkg list
//!   dm pkg exec "bun add left-pad"  → raw команда

use crate::commands::{PkgArgs, load_project_config};
use crate::output::{
    dim_style, error_style, print_system, println_styled, success_style, warn_style,
};
use crate::shell;
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use std::path::Path;

/// Точка входа команды.
pub async fn run(args: PkgArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let targets = select_services(&config, args.service.as_deref())?;

    match args.action.as_str() {
        "add" => {
            let pkg = args
                .package
                .as_deref()
                .ok_or_else(|| dm_core::DmError::invalid_config("dm pkg add <package-name>"))?;
            for name in &targets {
                pkg_op(&config, &root, name, "add", Some(pkg)).await?;
            }
        }
        "remove" | "rm" => {
            let pkg = args
                .package
                .as_deref()
                .ok_or_else(|| dm_core::DmError::invalid_config("dm pkg remove <package-name>"))?;
            for name in &targets {
                pkg_op(&config, &root, name, "remove", Some(pkg)).await?;
            }
        }
        "update" => {
            for name in &targets {
                pkg_op(&config, &root, name, "update", None).await?;
            }
        }
        "list" | "ls" => {
            for name in &targets {
                pkg_op(&config, &root, name, "list", None).await?;
            }
        }
        "exec" => {
            let cmd = args
                .package
                .as_deref()
                .ok_or_else(|| dm_core::DmError::invalid_config("dm pkg exec <command>"))?;
            for name in &targets {
                let dir = service_dir(&config, &root, name);
                print_system(&format!("exec in {name}: {cmd}"));
                match shell::run(cmd, &dir) {
                    Ok(0) => println_styled("  done", success_style()),
                    Ok(c) => println_styled(&format!("  exit {c}"), warn_style()),
                    Err(e) => println_styled(&format!("  error: {e}"), error_style()),
                }
            }
        }
        other => {
            return Err(dm_core::DmError::invalid_config(format!(
                "unknown pkg action: {other}. Use: add | remove | update | list | exec"
            )));
        }
    }
    Ok(())
}

/// Выбирает сервисы для операции.
fn select_services(config: &dm_core::Config, specific: Option<&str>) -> DmResult<Vec<String>> {
    match specific {
        Some(name) => {
            if config.services.contains_key(name) {
                Ok(vec![name.to_string()])
            } else {
                Err(dm_core::DmError::ServiceNotFound(name.to_string()))
            }
        }
        None => Ok(config.services.keys().cloned().collect()),
    }
}

/// Каталог сервиса.
fn service_dir(config: &dm_core::Config, root: &Path, name: &str) -> std::path::PathBuf {
    let svc = &config.services[name];
    shell::resolve_dir(root, &svc.path)
}

/// Определяет пакетный менеджер и команду для операции.
fn pkg_manager_cmd(
    lang: ServiceLanguage,
    dir: &Path,
    action: &str,
    pkg: Option<&str>,
) -> Option<String> {
    let has = |f: &str| dir.join(f).exists();

    // Bun优先итет если есть bun.lockb
    let use_bun = has("bun.lockb") || has("bun.lock") || lang == ServiceLanguage::Bun;

    Some(match action {
        "add" => {
            let pkg = pkg?;
            match lang {
                ServiceLanguage::Rust => format!("cargo add {pkg}"),
                ServiceLanguage::Go => format!("go get {pkg}"),
                ServiceLanguage::Python => format!("pip install {pkg}"),
                ServiceLanguage::Csharp => format!("dotnet add package {pkg}"),
                _ if use_bun => format!("bun add {pkg}"),
                _ => format!("npm install {pkg}"),
            }
        }
        "remove" => {
            let pkg = pkg?;
            match lang {
                ServiceLanguage::Rust => format!("cargo remove {pkg}"),
                ServiceLanguage::Go => format!("go get {pkg}@none"),
                ServiceLanguage::Python => format!("pip uninstall {pkg} -y"),
                ServiceLanguage::Csharp => format!("dotnet remove package {pkg}"),
                _ if use_bun => format!("bun remove {pkg}"),
                _ => format!("npm uninstall {pkg}"),
            }
        }
        "update" => match lang {
            ServiceLanguage::Rust => "cargo update".to_string(),
            ServiceLanguage::Go => "go get -u ./...".to_string(),
            ServiceLanguage::Python => "pip install --upgrade -r requirements.txt".to_string(),
            _ if use_bun => "bun update".to_string(),
            _ => "npm update".to_string(),
        },
        "list" => match lang {
            ServiceLanguage::Rust => "cargo tree --depth 1".to_string(),
            ServiceLanguage::Go => "go list -m all".to_string(),
            ServiceLanguage::Python => "pip list".to_string(),
            _ if use_bun => "bun pm ls".to_string(),
            _ => "npm list --depth=0".to_string(),
        },
        _ => return None,
    })
}

/// Выполняет операцию с пакетом для сервиса.
async fn pkg_op(
    config: &dm_core::Config,
    root: &Path,
    service_name: &str,
    action: &str,
    pkg: Option<&str>,
) -> DmResult<()> {
    let svc = &config.services[service_name];
    let dir = service_dir(config, root, service_name);

    let Some(cmd) = pkg_manager_cmd(svc.language, &dir, action, pkg) else {
        println_styled(
            &format!(
                "  {service_name}: no package manager for {:?}",
                svc.language
            ),
            dim_style(),
        );
        return Ok(());
    };

    let pkg_name = pkg.unwrap_or("");
    print_system(&format!("{action} {service_name}: {cmd}"));

    match shell::run(&cmd, &dir) {
        Ok(0) => println_styled(
            &format!("  OK: {service_name} {action} {pkg_name}"),
            success_style(),
        ),
        Ok(code) => println_styled(&format!("  FAIL: {service_name} exit {code}"), warn_style()),
        Err(e) => println_styled(&format!("  ERR: {service_name}: {e}"), error_style()),
    }
    Ok(())
}

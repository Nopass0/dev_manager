//! `dm docker up|down|logs|ps` — управление compose-инфраструктурой.
//!
//! Обнаруживает compose-файл (из `docker.compose_file` или `docker-compose.yml`
//! / `compose.yaml` в корне) и вызывает `docker compose` (v2) или `docker-compose` (v1).

use crate::commands::{DockerAction, DockerArgs, load_project_config};
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use std::path::Path;
use std::process::Command;

/// Точка входа команды.
pub async fn run(args: DockerArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let compose_file = resolve_compose_file(&root, &config.docker.compose_file);
    if !compose_file.exists() {
        return Err(dm_core::DmError::invalid_config(format!(
            "compose-файл не найден: {}. Настройте docker.compose_file или создайте docker-compose.yml.",
            compose_file.display()
        )));
    }
    let (program, base_args) = pick_compose_binary(&config.docker.project_name, &compose_file);

    match args.action {
        DockerAction::Up => {
            print_system("docker compose up -d");
            run_compose(&program, &base_args, &["up", "-d"], &root)?;
            println_styled("✓ инфраструктура поднята", success_style());
        }
        DockerAction::Down => {
            print_system("docker compose down");
            run_compose(&program, &base_args, &["down"], &root)?;
            println_styled("✓ инфраструктура остановлена", success_style());
        }
        DockerAction::Logs => {
            print_system("docker compose logs -f (Ctrl+C — выход)");
            run_compose(&program, &base_args, &["logs", "-f", "--tail=100"], &root)?;
        }
        DockerAction::Ps => {
            run_compose(&program, &base_args, &["ps"], &root)?;
        }
    }
    Ok(())
}

/// Определяет путь к compose-файлу с fallback на популярные имена.
fn resolve_compose_file(root: &Path, configured: &str) -> std::path::PathBuf {
    let primary = root.join(configured);
    if primary.exists() {
        return primary;
    }
    for alt in [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yaml",
        "compose.yml",
    ] {
        let p = root.join(alt);
        if p.exists() {
            return p;
        }
    }
    primary
}

/// Выбирает бинарник compose (v2 `docker compose` предпочтительнее) и базовые args.
fn pick_compose_binary(
    project_name: &Option<String>,
    compose_file: &Path,
) -> (String, Vec<String>) {
    let mut base = vec![
        "-f".to_string(),
        compose_file.to_string_lossy().into_owned(),
    ];
    if let Some(pn) = project_name {
        base.push("-p".into());
        base.push(pn.clone());
    }
    // Проверяем наличие `docker compose` (v2); иначе fallback на `docker-compose`.
    let has_v2 = Command::new("docker")
        .args(["compose", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if has_v2 {
        ("docker".into(), {
            let mut v = vec!["compose".to_string()];
            v.extend(base);
            v
        })
    } else {
        ("docker-compose".into(), base)
    }
}

/// Запускает compose-подкоманду.
fn run_compose(program: &str, base: &[String], extra: &[&str], cwd: &Path) -> DmResult<()> {
    let mut cmd = Command::new(program);
    cmd.args(base);
    cmd.args(extra);
    cmd.current_dir(cwd);
    let status = cmd
        .status()
        .map_err(|e| dm_core::DmError::Process(format!("docker: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(dm_core::DmError::ExternalCommand {
            command: format!("{program} {} {}", base.join(" "), extra.join(" ")),
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        })
    }
}

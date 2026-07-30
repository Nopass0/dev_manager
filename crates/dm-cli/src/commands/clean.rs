//! `dm clean [--target=all|cache|branches|docker] [-y]` — умная очистка проекта.
//!
//! Объединяет несколько операций уборки:
//! - `cache` — кэши сборок (target, node_modules/.cache, __pycache__…);
//! - `branches` — локальные ветки, уже слитые в текущую (orphan-ветки);
//! - `docker` — dangling-образы и остановленные контейнеры проекта;
//! - `all` — всё перечисленное.

use crate::commands::{CleanArgs, load_project_config};
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use std::path::Path;
use std::process::Command;

/// Каталоги кэшей по умолчанию.
const CACHE_DIRS: &[&str] = &[
    "target",
    "node_modules/.cache",
    ".next/cache",
    "dist",
    "build",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
];

/// Точка входа команды.
pub async fn run(args: CleanArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let do_cache = matches!(args.target.as_str(), "all" | "cache");
    let do_branches = matches!(args.target.as_str(), "all" | "branches");
    let do_docker = matches!(args.target.as_str(), "all" | "docker");

    if do_cache {
        clean_cache(&config, &root);
    }
    if do_branches {
        clean_branches(&root, args.yes);
    }
    if do_docker {
        clean_docker(args.yes);
    }
    println_styled("✓ очистка завершена", success_style());
    Ok(())
}

/// Удаляет кэши сборок сервисов.
fn clean_cache(config: &dm_core::Config, root: &Path) {
    print_system("очистка кэшей сборок…");
    let mut cleared = 0usize;
    for (_name, svc) in &config.services {
        let dir = dm_core::paths::resolve(root, Path::new(&svc.path));
        for cache in CACHE_DIRS {
            let target = dir.join(cache);
            if target.exists() && std::fs::remove_dir_all(&target).is_ok() {
                cleared += 1;
            }
        }
    }
    println_styled(
        &format!("  удалено каталогов кэша: {cleared}"),
        success_style(),
    );
}

/// Удаляет локальные ветки, слитые в текущую (orphan-ветки).
fn clean_branches(root: &Path, yes: bool) {
    print_system("очистка слитых локальных веток…");
    // git branch --merged | grep -v '\*\|main\|master' | xargs git branch -d
    let merged = Command::new("git")
        .args(["-C", root.to_str().unwrap_or("."), "branch", "--merged"])
        .output();
    let Ok(out) = merged else { return };
    let text = String::from_utf8_lossy(&out.stdout);
    let protected = ["main", "master", "develop", "dev"];
    let to_delete: Vec<&str> = text
        .lines()
        .map(|l| l.trim_start_matches('*').trim())
        .filter(|l| !l.is_empty() && !protected.contains(l))
        .collect();
    if to_delete.is_empty() {
        println_styled(
            "  слитых веток для удаления нет",
            crate::output::dim_style(),
        );
        return;
    }
    println_styled(
        &format!("  найдено слитых веток: {}", to_delete.len()),
        crate::output::warn_style(),
    );
    if !yes {
        // Без -y показываем, но не удаляем (безопасность).
        for b in &to_delete {
            println!("    {b}");
        }
        println_styled(
            "  используйте --yes (-y) для удаления",
            crate::output::dim_style(),
        );
        return;
    }
    let mut deleted = 0usize;
    for b in &to_delete {
        let ok = Command::new("git")
            .args(["-C", root.to_str().unwrap_or("."), "branch", "-d", b])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            deleted += 1;
        }
    }
    println_styled(&format!("  удалено веток: {deleted}"), success_style());
}

/// Чистит dangling-образы и остановленные контейнеры (docker prune).
fn clean_docker(yes: bool) {
    print_system("очистка Docker (prune)…");
    // docker system prune -f удаляет dangling-образы, остановленные контейнеры,
    // неиспользуемые сети. Требует подтверждения, поэтому -f только при --yes.
    if !yes {
        println_styled(
            "  используйте --yes (-y) для запуска docker system prune",
            crate::output::dim_style(),
        );
        return;
    }
    let ok = Command::new("docker")
        .args(["system", "prune", "-f"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println_styled("  docker prune выполнен", success_style());
    } else {
        println_styled(
            "  docker недоступен или prune не выполнен",
            crate::output::warn_style(),
        );
    }
}

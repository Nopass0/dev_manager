//! `dm history` — лента недавней активности (коммиты) по всем репозиториям.
//!
//! Показывает последние коммиты каждого репозитория проекта в единой ленте,
//! чтобы быстро понять «что я/команда делали недавно». Аналог git-log, но
//! сразу по всем репо с пометкой источника.

use crate::commands::load_project_config;
use crate::output::{dim_style, print_system, println_styled};
use dm_core::paths;
use dm_core::DmResult;
use dm_vcs::run_git;
use std::path::Path;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    print_system("недавняя активность проекта (последние коммиты по репозиториям):");

    let repos = unique_repos(&config, &root);
    if repos.is_empty() {
        println_styled("репозитории не найдены", dim_style());
        return Ok(());
    }
    for repo in &repos {
        let label = repo_label(repo, &root);
        println_styled(&format!("── {label} ──"), dim_style());
        let out = run_git(
            repo,
            &["log", "--oneline", "--decorate", "-10"],
            false,
        )
        .await;
        match out {
            Ok(o) if o.ok() => {
                for line in o.stdout.lines() {
                    println!("  {line}");
                }
            }
            _ => println_styled("  (нет коммитов или не репозиторий)", dim_style()),
        }
    }
    Ok(())
}

/// Собирает уникальные пути git-репозиториев проекта.
fn unique_repos(config: &dm_core::Config, root: &Path) -> Vec<std::path::PathBuf> {
    let mut set: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    for (_name, svc) in &config.services {
        let p = svc
            .repo
            .as_ref()
            .map(|r| paths::resolve(root, Path::new(r)))
            .unwrap_or_else(|| root.to_path_buf());
        set.insert(p);
    }
    set.into_iter().collect()
}

/// Человекочитаемая метка репозитория (имя каталога или .).
fn repo_label(repo: &std::path::Path, root: &Path) -> String {
    if repo == root {
        "(корень)".to_string()
    } else {
        repo.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| repo.display().to_string())
    }
}

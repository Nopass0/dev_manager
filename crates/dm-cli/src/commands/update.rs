//! `dm update` — git pull во всех репозиториях проекта одной командой.
//!
//! Объединяет обновление зависимостей после pull. Ускоряет синхронизацию с
//! командой: одна команда вместо `cd` в каждый репо и `git pull`.

use crate::commands::load_project_config;
use crate::output::{print_system, println_styled, success_style, warn_style};
use crate::shell;
use dm_core::DmResult;
use dm_core::paths;
use std::collections::HashSet;
use std::path::Path;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let repos = unique_repos(&config, &root);
    if repos.is_empty() {
        return Err(dm_core::DmError::invalid_config("репозитории не найдены"));
    }
    print_system(&format!("git pull в {} репозиториях", repos.len()));
    let mut ok = 0usize;
    for repo in &repos {
        let label = repo_label(repo, &root);
        print_system(&format!("• {label}"));
        match shell::run("git pull --ff-only", repo) {
            Ok(0) => {
                ok += 1;
                println_styled(&format!("  ✓ {label} обновлён"), success_style());
            }
            Ok(code) => {
                println_styled(&format!("  ! {label}: git вернул код {code}"), warn_style())
            }
            Err(e) => println_styled(&format!("  ✗ {label}: {e}"), crate::output::error_style()),
        }
    }
    println_styled(
        &format!("готово: {}/{} обновлено", ok, repos.len()),
        if ok == repos.len() {
            success_style()
        } else {
            warn_style()
        },
    );
    Ok(())
}

/// Уникальные каталоги git-репозиториев проекта (для multi-repo).
fn unique_repos(config: &dm_core::Config, root: &Path) -> Vec<std::path::PathBuf> {
    let mut set: HashSet<std::path::PathBuf> = HashSet::new();
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

/// Метка репозитория для вывода.
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

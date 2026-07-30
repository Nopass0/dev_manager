//! `dm push` — пуш всех репозиториев в их origin.

use crate::commands::{PREFIX_SYS, load_project_config};
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_core::paths;
use dm_vcs::push::push_all;
use std::collections::HashSet;
use std::path::PathBuf;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let repo_paths = collect_repo_paths(&config, &root)?;
    if repo_paths.is_empty() {
        return Err(dm_core::DmError::invalid_config(
            "не найдено ни одного репозитория для пуша",
        ));
    }
    print_system(&format!(
        "{PREFIX_SYS} пуш {} репозиториев…",
        repo_paths.len()
    ));
    let outcomes = push_all(&repo_paths).await;
    for o in &outcomes {
        let marker = if o.pushed { "↑" } else { "!" };
        println!("{} {} — {}", marker, o.repo.display(), o.note);
    }
    let ok = outcomes.iter().filter(|o| o.pushed).count();
    println_styled(
        &format!("готово: запушено {}/{}", ok, outcomes.len()),
        success_style(),
    );
    Ok(())
}

/// Собирает уникальные пути git-репозиториев всех сервисов.
fn collect_repo_paths(config: &dm_core::Config, root: &std::path::Path) -> DmResult<Vec<PathBuf>> {
    let mut set: HashSet<PathBuf> = HashSet::new();
    for (_name, svc) in &config.services {
        let p = svc
            .repo
            .as_ref()
            .map(|r| paths::resolve(root, std::path::Path::new(r)))
            .unwrap_or_else(|| root.to_path_buf());
        set.insert(p);
    }
    Ok(set.into_iter().collect())
}

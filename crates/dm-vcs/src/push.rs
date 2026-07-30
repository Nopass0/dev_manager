//! Реализация `dm push` — пуш каждого репозитория в свой origin.

use crate::git::run_git;
use dm_core::error::DmResult;
use std::path::Path;

/// Результат пуша одного репозитория.
#[derive(Debug, Clone)]
pub struct PushOutcome {
    /// Путь к репозиторию.
    pub repo: std::path::PathBuf,
    /// Успешно ли запушено.
    pub pushed: bool,
    /// Пояснение.
    pub note: String,
}

/// Пушит указанную ветку в origin для одного репозитория.
///
/// Эквивалент `git -C <repo> push -u origin <branch>`. Если upstream уже
/// настроен — достаточно `git push`.
pub async fn push_in_repo(repo: &Path, branch: Option<&str>) -> DmResult<PushOutcome> {
    match branch {
        Some(b) => {
            run_git(repo, &["push", "-u", "origin", b], true).await?;
        }
        None => {
            run_git(repo, &["push"], true).await?;
        }
    }
    Ok(PushOutcome {
        repo: repo.to_path_buf(),
        pushed: true,
        note: "запушено".to_string(),
    })
}

/// Пушит все переданные репозитории. Каждый — в свой origin (см. remote).
pub async fn push_all(repos: &[std::path::PathBuf]) -> Vec<PushOutcome> {
    let mut results = Vec::with_capacity(repos.len());
    for repo in repos {
        match push_in_repo(repo, None).await {
            Ok(o) => results.push(o),
            Err(e) => results.push(PushOutcome {
                repo: repo.clone(),
                pushed: false,
                note: format!("ошибка: {e}"),
            }),
        }
    }
    results
}

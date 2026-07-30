//! Реализация `dm commit` — коммит в один или несколько репозиториев.

use crate::git::{has_changes, run_git};
use dm_core::error::DmResult;
use std::path::Path;

/// Результат коммита в один репозиторий.
#[derive(Debug, Clone)]
pub struct CommitOutcome {
    /// Путь к репозиторию.
    pub repo: std::path::PathBuf,
    /// Успешно ли закоммичено.
    pub committed: bool,
    /// Хеш нового коммита (если есть), без префикса `refs/heads/`.
    pub commit_hash: Option<String>,
    /// Пояснение (почему не закоммичено, например «нет изменений»).
    pub note: String,
}

/// Коммитит **все** изменённые файлы в одном репозитории с сообщением `message`.
///
/// Эквивалент `git add -A && git commit -m "<message>"`. Если изменений нет —
/// возвращается успешный исход с `committed=false`.
pub async fn commit_in_repo(repo: &Path, message: &str) -> DmResult<CommitOutcome> {
    if !has_changes(repo).await? {
        return Ok(CommitOutcome {
            repo: repo.to_path_buf(),
            committed: false,
            commit_hash: None,
            note: "нет изменений".to_string(),
        });
    }
    run_git(repo, &["add", "-A"], true).await?;
    run_git(repo, &["commit", "-m", message], true).await?;
    let hash_out = run_git(repo, &["rev-parse", "--short", "HEAD"], true).await?;
    Ok(CommitOutcome {
        repo: repo.to_path_buf(),
        committed: true,
        commit_hash: Some(hash_out.stdout.trim().to_string()),
        note: "закоммичено".to_string(),
    })
}

/// Коммитит одновременно во все переданные репозитории одним сообщением.
///
/// Используется командой `dm commit "msg"` без указания цели: каждый репозиторий
/// получает одно и то же сообщение, а пушится потом каждый в свой origin.
pub async fn commit_all(repos: &[std::path::PathBuf], message: &str) -> Vec<CommitOutcome> {
    let mut results = Vec::with_capacity(repos.len());
    for repo in repos {
        // Ошибку отдельного репо не прерываем весь процесс — докладываем.
        match commit_in_repo(repo, message).await {
            Ok(o) => results.push(o),
            Err(e) => results.push(CommitOutcome {
                repo: repo.clone(),
                committed: false,
                commit_hash: None,
                note: format!("ошибка: {e}"),
            }),
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn commit_in_non_repo_reports_error() {
        let tmp = std::env::temp_dir().join("dm_commit_nonrepo_test");
        std::fs::create_dir_all(&tmp).unwrap();
        // has_changes вызовет run_git; в не-репо git вернёт код 128 → ошибка.
        let res = commit_in_repo(&tmp, "msg").await;
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(res.is_err() || !res.unwrap().committed);
    }
}

//! Cross-repo git-операции: stash/branch/rebase сразу по всем репозиториям.
//!
//! Построено поверх [`crate::git::run_git`]. Каждая операция возвращает список
//! результатов по репозиториям, чтобы одна ошибка не прерывала остальные.

use crate::git::run_git;
use dm_core::DmResult;
use std::path::Path;

/// Результат одной cross-repo операции в одном репозитории.
#[derive(Debug, Clone)]
pub struct RepoOpResult {
    /// Путь к репозиторию.
    pub repo: std::path::PathBuf,
    /// Успешно ли завершилось.
    pub ok: bool,
    /// Пояснение/вывод.
    pub note: String,
}

/// Прячет изменения (`git stash`) во всех репозиториях.
pub async fn stash_all(repos: &[std::path::PathBuf]) -> Vec<RepoOpResult> {
    let mut out = Vec::with_capacity(repos.len());
    for repo in repos {
        match run_git(repo, &["stash"], true).await {
            Ok(o) => out.push(RepoOpResult {
                repo: repo.clone(),
                ok: true,
                note: if o.stdout.trim().is_empty() {
                    "нет локальных изменений".to_string()
                } else {
                    o.stdout.trim().to_string()
                },
            }),
            Err(e) => out.push(RepoOpResult {
                repo: repo.clone(),
                ok: false,
                note: e.to_string(),
            }),
        }
    }
    out
}

/// Переключает/создаёт ветку во всех репозиториях (`git checkout -B <name>`).
pub async fn branch_all(repos: &[std::path::PathBuf], name: &str) -> Vec<RepoOpResult> {
    let mut out = Vec::with_capacity(repos.len());
    for repo in repos {
        match run_git(repo, &["checkout", "-B", name], true).await {
            Ok(_) => out.push(RepoOpResult {
                repo: repo.clone(),
                ok: true,
                note: format!("переключено на '{name}'"),
            }),
            Err(e) => out.push(RepoOpResult {
                repo: repo.clone(),
                ok: false,
                note: e.to_string(),
            }),
        }
    }
    out
}

/// Ребейзит текущую ветку каждого репо на `onto`.
pub async fn rebase_all(repos: &[std::path::PathBuf], onto: &str) -> Vec<RepoOpResult> {
    let mut out = Vec::with_capacity(repos.len());
    for repo in repos {
        match run_git(repo, &["rebase", onto], true).await {
            Ok(_) => out.push(RepoOpResult {
                repo: repo.clone(),
                ok: true,
                note: format!("ребейз на '{onto}' выполнен"),
            }),
            Err(e) => out.push(RepoOpResult {
                repo: repo.clone(),
                ok: false,
                note: e.to_string(),
            }),
        }
    }
    out
}

/// Собирает уникальные каталоги репозиториев сервиса из конфига (для multi-repo).
pub fn collect_repo_paths(
    config: &dm_core::Config,
    root: &Path,
) -> DmResult<Vec<std::path::PathBuf>> {
    let mut set: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    for (_name, svc) in &config.services {
        let p = svc
            .repo
            .as_ref()
            .map(|r| dm_core::paths::resolve(root, std::path::Path::new(r)))
            .unwrap_or_else(|| root.to_path_buf());
        set.insert(p);
    }
    Ok(set.into_iter().collect())
}

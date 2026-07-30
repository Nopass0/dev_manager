//! `dm commit [target] [msg]` и `dm commit auto` — git-автоматизация.
//!
//! Поведение:
//! - `dm commit "msg"` — коммитит во все репозитории одним сообщением.
//! - `dm commit <svc> "msg"` — коммитит только в репозиторий сервиса `<svc>`.
//! - `dm commit auto` — формирует сообщение из списка изменённых символов
//!   (функций/классов/структур) через `dm-analysis`.

use crate::commands::{load_project_config, CommitArgs};
use crate::output::{error_style, print_system, success_style, println_styled};
use dm_core::paths;
use dm_vcs::commit::{commit_all, commit_in_repo};
use dm_vcs::diff::changed_file_paths;
use dm_core::DmResult;
use std::collections::HashSet;
use std::path::PathBuf;

/// Точка входа команды.
pub async fn run(args: CommitArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;

    // Собираем уникальные каталоги git-репозиториев, затронутых сервисами.
    let repo_paths = collect_repo_paths(&config, &root)?;

    match args.target.as_deref() {
        // --- dm commit auto ---
        Some("auto") => run_auto(&root, &repo_paths).await,

        // --- dm commit <svc> "msg" ---
        Some(target) if config.services.contains_key(target) => {
            let svc = &config.services[target];
            let repo = svc
                .repo
                .as_ref()
                .map(|r| paths::resolve(&root, std::path::Path::new(r)))
                .unwrap_or_else(|| root.clone());
            let msg = args
                .message
                .ok_or_else(|| dm_core::DmError::invalid_config(
                    "укажите сообщение коммита: `dm commit <svc> \"сообщение\"`",
                ))?;
            let outcome = commit_in_repo(&repo, &msg).await?;
            report_one(&outcome);
            Ok(())
        }

        // --- dm commit "msg" (без цели → во все репо) ---
        Some(other) if repo_paths.len() == 1 || args.message.is_some() => {
            // Если пользователь явно передал сообщение, но target — это и есть
            // само сообщение (случай `dm commit "msg"`).
            let msg = args
                .message
                .unwrap_or_else(|| other.to_string());
            let outcomes = commit_all(&repo_paths, &msg).await;
            for o in &outcomes {
                report_one(o);
            }
            Ok(())
        }

        // --- дефолт: нет сообщения ---
        _ => {
            println_styled(
                "Использование: dm commit \"msg\" | dm commit <svc> \"msg\" | dm commit auto",
                error_style(),
            );
            Err(dm_core::DmError::invalid_config("некорректный вызов commit"))
        }
    }
}

/// Реализация `dm commit auto`: формирует сообщение из изменённых символов.
async fn run_auto(root: &PathBuf, repo_paths: &[PathBuf]) -> DmResult<()> {
    print_system("анализ изменённых файлов для авто-сообщения…");
    let mut changed_symbols: Vec<dm_analysis::ChangedCodeSymbol> = Vec::new();
    let mut seen_files: HashSet<PathBuf> = HashSet::new();

    for repo in repo_paths {
        for rel in changed_file_paths(repo).await? {
            // Абсолютный путь к файлу.
            let abs = repo.join(&rel);
            if seen_files.contains(&abs) {
                continue;
            }
            seen_files.insert(abs.clone());
            // Разбираем «до» и «после» по HEAD.
            let before = dm_vcs::git::run_git(repo, &["show", &format!("HEAD:{}", rel.display())], false)
                .await
                .map(|o| o.stdout)
                .unwrap_or_default();
            let after = std::fs::read_to_string(&abs).unwrap_or_default();

            if let (Some(before_syms), Some(after_syms)) = (
                dm_analysis::parse_file_str(&before, &rel),
                dm_analysis::parse_file_str(&after, &rel),
            ) {
                let _ = root;
                for ch in dm_analysis::changed_symbols(&before_syms, &after_syms) {
                    changed_symbols.push(ch);
                }
            }
        }
    }

    let message = build_auto_message_from_analysis(&changed_symbols);
    print_system(&format!("авто-сообщение:\n{}", message));
    let outcomes = commit_all(repo_paths, &message).await;
    for o in &outcomes {
        report_one(o);
    }
    Ok(())
}

/// Формирует текст сообщения коммита из списка изменений.
fn build_auto_message_from_analysis(changes: &[dm_analysis::ChangedCodeSymbol]) -> String {
    if changes.is_empty() {
        return "auto: изменения в коде".to_string();
    }
    let mut out = format!("auto: изменены {} символ(ов)\n\n", changes.len());
    for ch in changes {
        out.push_str(&format!("- {}\n", ch.describe()));
    }
    out
}

/// Собирает уникальные пути git-репозиториев всех сервисов (для multi-repo).
fn collect_repo_paths(
    config: &dm_core::Config,
    root: &PathBuf,
) -> DmResult<Vec<PathBuf>> {
    let mut set: HashSet<PathBuf> = HashSet::new();
    for (_name, svc) in &config.services {
        let p = svc
            .repo
            .as_ref()
            .map(|r| paths::resolve(root, std::path::Path::new(r)))
            .unwrap_or_else(|| root.clone());
        set.insert(p);
    }
    Ok(set.into_iter().collect())
}

/// Печатает результат коммита одного репозитория.
fn report_one(out: &dm_vcs::commit::CommitOutcome) {
    let marker = if out.committed { "✓" } else { "·" };
    let hash = out
        .commit_hash
        .as_deref()
        .unwrap_or("      ");
    println!(
        "{} {} {} — {}",
        marker,
        hash,
        out.repo.display(),
        out.note
    );
    let _ = success_style; // сохранить импорт для будущих цветных вариантов
}

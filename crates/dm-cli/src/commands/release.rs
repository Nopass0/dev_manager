//! `dm release <patch|minor|major>` — SemVer-bump + авто-changelog.
//!
//! Анализирует conventional-commits с прошлого тега, предлагает/применяет bump,
//! генерирует секцию CHANGELOG. `--changelog-only` печатает текст без тегирования.

use crate::commands::{ReleaseArgs, load_project_config};
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_vcs::{ConventionalCommit, Version, render_release_section, run_git};
use std::path::Path;

/// Точка входа команды.
pub async fn run(args: ReleaseArgs) -> DmResult<()> {
    let bump = dm_vcs::Bump::parse(&args.kind).ok_or_else(|| {
        dm_core::DmError::invalid_config(
            "укажите тип bump: patch | minor | major. Например: `dm release patch`.",
        )
    })?;
    let (config, root) = load_project_config()?;
    print_system(&format!(
        "подготовка релиза ({}) для проекта '{}'",
        args.kind, config.project_name
    ));

    let today = today_iso();
    let last_tag = last_version_tag(&root).await;
    let last_tag_str = last_tag.clone().unwrap_or_else(|| "(нет тегов)".into());
    let current = last_tag
        .as_deref()
        .and_then(Version::parse)
        .unwrap_or(Version {
            major: 0,
            minor: 1,
            patch: 0,
        });
    let next = current.bumped(bump);

    // Собираем conventional-commits с прошлого тега (или всю историю, если тегов нет).
    let commits = collect_commits_since(&root, last_tag.as_deref().unwrap_or("")).await;
    print_system(&format!(
        "найдено conventional-коммитов: {} (с тега {})",
        commits.len(),
        last_tag_str
    ));

    let section = render_release_section(next, &today, &commits);

    if args.changelog_only {
        println_styled(
            &format!("Предлагаемая версия: {next} (было {current})"),
            success_style(),
        );
        println!("{section}");
        return Ok(());
    }

    // Реальный bump: дописываем секцию в CHANGELOG.md и предлагаем git-tag.
    let changelog_path = root.join("CHANGELOG.md");
    let header = format!("# Журнал изменений\n\n## [{next}] — {today}\n\n");
    prepend_to_changelog(&changelog_path, &section, &header)?;
    println_styled(
        &format!("  ✓ CHANGELOG.md обновлён (версия {next})"),
        success_style(),
    );

    print_system(&format!(
        "Готово. Для завершения релиза:\n  git add CHANGELOG.md && git commit -m 'chore(release): {next}' && git tag v{next}"
    ));
    Ok(())
}

/// Возвращает последний тег вида v1.2.3 или 1.2.3, иначе версию по умолчанию.
async fn last_version_tag(root: &Path) -> Option<String> {
    let out = run_git(root, &["describe", "--tags", "--abbrev=0"], false)
        .await
        .ok()?;
    if out.ok() {
        Some(out.stdout.trim().to_string())
    } else {
        None
    }
}

/// Собирает conventional-commits с последнего тега (или с начала истории).
async fn collect_commits_since(root: &Path, since_tag: &str) -> Vec<ConventionalCommit> {
    let range = if since_tag.is_empty() {
        "HEAD".to_string()
    } else {
        format!("{since_tag}..HEAD")
    };
    let out = match run_git(root, &["log", "--pretty=%s", &range], false).await {
        Ok(o) if o.ok() => o.stdout,
        _ => return Vec::new(),
    };
    out.lines().filter_map(ConventionalCommit::parse).collect()
}

/// Дописывает секцию релиза в начало CHANGELOG.md (после заголовка).
fn prepend_to_changelog(
    path: &std::path::PathBuf,
    section: &str,
    fresh_header: &str,
) -> DmResult<()> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let new_content = if content.trim().is_empty() {
        format!("{fresh_header}{section}\n{content}")
    } else {
        // Вставляем после первой строки-заголовка (# ...).
        if let Some(idx) = content.find('\n') {
            let (head, rest) = content.split_at(idx + 1);
            format!("{head}{section}\n{rest}")
        } else {
            format!("{content}\n\n{section}\n")
        }
    };
    std::fs::write(path, new_content)?;
    Ok(())
}

/// Текущая дата в ISO-формате (YYYY-MM-DD).
fn today_iso() -> String {
    // Без внешней chrono: используем системную date-утилиту как опору для
    // MVP; при отсутствии — заглушка. Надёжная реализация — в roadmap.
    #[cfg(unix)]
    {
        if let Ok(out) = std::process::Command::new("date").arg("+%Y-%m-%d").output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                return s.trim().to_string();
            }
        }
    }
    "2026-01-01".to_string()
}

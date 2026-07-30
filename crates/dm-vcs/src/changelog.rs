//! Генерация `CHANGELOG.md` из conventional-commits.
//!
//! Формат — Keep a Changelog. Группирует коммиты по типам, выделяет breaking
//! changes отдельным блоком наверху.

use crate::conventional::ConventionalCommit;
use crate::release::Version;
use std::fmt::Write;

/// Формирует текст секции релиза для вставки в changelog.
///
/// `version` — новая версия; `date` — строка даты (например `2026-07-27`);
/// `commits` — уже разобранные conventional-commits этого релиза.
pub fn render_release_section(version: Version, date: &str, commits: &[ConventionalCommit]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## [{version}] — {date}\n");

    // Breaking changes — отдельным блоком.
    let breaking: Vec<&ConventionalCommit> = commits.iter().filter(|c| c.breaking).collect();
    if !breaking.is_empty() {
        let _ = writeln!(out, "### ⚠️ BREAKING CHANGES\n");
        for c in breaking {
            let _ = writeln!(out, "- {} ({})", c.description, scope_or_kind(c));
        }
        out.push('\n');
    }

    // Остальные группы по типам, сохраняя канонический порядок.
    let order = ["feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "style"];
    for kind in order {
        let group: Vec<&ConventionalCommit> = commits.iter().filter(|c| c.kind == kind && !c.breaking).collect();
        if group.is_empty() {
            continue;
        }
        let _ = writeln!(out, "### {}\n", group[0].group_label());
        for c in group {
            let _ = writeln!(out, "- {} ({})", c.description, scope_or_kind(c));
        }
        out.push('\n');
    }
    out
}

/// Возвращает строку вида `api` или `feat` для подписи.
fn scope_or_kind(c: &ConventionalCommit) -> String {
    c.scope.clone().unwrap_or_else(|| c.kind.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_grouped_release() {
        let commits = vec![
            ConventionalCommit::parse("feat(api): эндпоинт /health").unwrap(),
            ConventionalCommit::parse("fix(web): падение при пустом списке").unwrap(),
            ConventionalCommit::parse("feat!: новая схема БД").unwrap(),
        ];
        let section = render_release_section(
            Version { major: 1, minor: 1, patch: 0 },
            "2026-07-27",
            &commits,
        );
        assert!(section.contains("BREAKING CHANGES"));
        assert!(section.contains("Новые возможности"));
        assert!(section.contains("Исправления"));
        assert!(section.contains("эндпоинт /health"));
        assert!(section.contains("новая схема БД"));
    }
}

//! Conventional Commits: валидация сообщений и парсинг.
//!
//! Поддерживает формат `type(scope): description` из Conventional Commits 1.0.
//! Используется `dm commit` (валидация) и changelog-генератором.

use std::collections::BTreeMap;

/// Канонические типы Conventional Commits и их порядок в changelog.
pub const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "style",
];

/// Разобранное conventional-commit сообщение.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommit {
    /// Тип: `feat`, `fix` и т.д.
    pub kind: String,
    /// Опциональная область: `feat(api): ...` → `api`.
    pub scope: Option<String>,
    /// Признак breaking change (`!` после типа/scope).
    pub breaking: bool,
    /// Текст описания.
    pub description: String,
}

impl ConventionalCommit {
    /// Пытается разобрать заголовок коммита. Возвращает `None`, если не соответствует.
    pub fn parse(subject: &str) -> Option<Self> {
        let s = subject.trim();
        // Требуем наличия двоеточия после типа.
        let colon = s.find(':')?;
        let head = &s[..colon];
        let description = s[colon + 1..].trim().to_string();
        if description.is_empty() {
            return None;
        }
        // head = "type" | "type(scope)" | "type!" | "type(scope)!"
        let breaking = head.ends_with('!');
        let head = head.trim_end_matches('!');

        let (kind, scope) = match head.find('(') {
            Some(open) if head.ends_with(')') => {
                let k = head[..open].to_string();
                let sc = head[open + 1..head.len() - 1].to_string();
                (k, Some(sc))
            }
            _ => (head.to_string(), None),
        };
        if !COMMIT_TYPES.contains(&kind.as_str()) {
            return None;
        }
        Some(Self {
            kind,
            scope,
            breaking,
            description,
        })
    }

    /// Человекочитаемая метка для changelog (группы).
    pub fn group_label(&self) -> &'static str {
        match self.kind.as_str() {
            "feat" => "✨ Новые возможности",
            "fix" => "🐛 Исправления",
            "perf" => "⚡️ Производительность",
            "refactor" => "♻️ Рефакторинг",
            "docs" => "📚 Документация",
            "test" => "✅ Тесты",
            "build" => "📦 Сборка",
            "ci" => "👷 CI",
            "chore" => "🔧 Прочее",
            "style" => "🎨 Стиль",
            _ => "Прочее",
        }
    }
}

/// Группирует коммиты по типам в порядке, удобном для changelog.
pub fn group_by_type(
    commits: &[ConventionalCommit],
) -> BTreeMap<&'static str, Vec<&ConventionalCommit>> {
    // BTreeMap сортирует по ключу; используем индекс типа как порядок.
    let order: std::collections::HashMap<&str, usize> = COMMIT_TYPES
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, i))
        .collect();
    let mut groups: std::collections::HashMap<&'static str, Vec<&ConventionalCommit>> =
        std::collections::HashMap::new();
    for c in commits {
        groups.entry(c.group_label()).or_default().push(c);
    }
    let mut sorted: Vec<(&'static str, Vec<&ConventionalCommit>)> = groups.into_iter().collect();
    sorted.sort_by_key(|(label, _)| {
        // Порядок по первому типу в группе.
        COMMIT_TYPES
            .iter()
            .position(|t| label.contains(label_for(t)))
            .unwrap_or(99)
    });
    let _ = order;
    sorted.into_iter().collect()
}

/// Возвращает метку группы по коду типа (для сортировки).
fn label_for(kind: &str) -> &'static str {
    match kind {
        "feat" => "Новые возможности",
        "fix" => "Исправления",
        "perf" => "Производительность",
        "refactor" => "Рефакторинг",
        "docs" => "Документация",
        "test" => "Тесты",
        "build" => "Сборка",
        "ci" => "CI",
        "chore" => "Прочее",
        "style" => "Стиль",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_feat_with_scope() {
        let c = ConventionalCommit::parse("feat(api): добавить эндпоинт /health").unwrap();
        assert_eq!(c.kind, "feat");
        assert_eq!(c.scope.as_deref(), Some("api"));
        assert!(!c.breaking);
        assert_eq!(c.description, "добавить эндпоинт /health");
    }

    #[test]
    fn detects_breaking() {
        let c = ConventionalCommit::parse("fix!: критический баг").unwrap();
        assert!(c.breaking);
    }

    #[test]
    fn rejects_non_conventional() {
        assert!(ConventionalCommit::parse("произвольное сообщение").is_none());
        assert!(ConventionalCommit::parse("feat:").is_none()); // пустое описание
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(ConventionalCommit::parse("wat: неизвестный тип").is_none());
    }
}

//! Поиск неиспользуемого кода.
//!
//! Стратегия: символ считается использованным, если его имя встречается в
//! каком-либо файле проекта вне собственного определения. Это простая, но
//! эффективная эвристика, не требующая построения графа вызовов.

use crate::lints::{LintCategory, LintFinding};
use crate::symbols::Symbol;
use std::collections::HashSet;

/// Находит символы, имена которых не встречаются больше нигде в `corpus_files`.
///
/// `corpus` — это объединённый текст всех файлов проекта (для простоты поиска).
/// В реальном использовании его готовит вызывающий код, прочитав файлы один раз.
pub fn find_unused(symbols: &[Symbol], corpus: &[String]) -> Vec<LintFinding> {
    let mut out = Vec::new();
    let corpus_joined: String = corpus.join("\n");
    // Множество «собственных» вхождений учитывать не нужно — если имя встречается
    // где-то ещё, символ уже использован.
    for s in symbols {
        // Пропускаем слишком короткие/общие имена, чтобы не плодить ложные срабатывания.
        if s.name.len() < 2 {
            continue;
        }
        // Точки входа и тесты никогда не «вызываются» явно — не флагаем их.
        // `main` — точка входа (Rust/C/Go), `Test*`/`test_*` — тесты (Go/Rust/Python),
        // `setUp`/`tearDown` — фикстуры pytest/JUnit.
        if is_entry_point_or_test(&s.name) {
            continue;
        }
        // corpus — это текст ВСЕХ файлов проекта, включая определяющий символ.
        // Символ считается использованным, если его имя встречается хотя бы
        // дважды (определение + хотя бы одно использование). Один раз имя
        // встретится всегда (определение) — поэтому порог = 2.
        let occurrences = corpus_joined.matches(&s.name).count();
        if occurrences < 2 {
            out.push(LintFinding {
                category: LintCategory::Unused,
                message: format!(
                    "символ '{}' определяет, но нигде не используется в проекте",
                    s.name
                ),
                file: s.file.clone(),
                symbol: Some(s.name.clone()),
                line: Some(s.start_line),
            });
        }
    }
    out
}

/// Возвращает true для точек входа и тестовых функций, которые никогда явно
/// не вызываются (поэтому эвристика «unused» даёт на них false positive).
///
/// Покрывает:
/// - `main` — точка входа (Rust/C/Go);
/// - `Test*` — Go/JS тесты;
/// - `test_*` / `tests` — Rust/Python тесты;
/// - `setUp`/`tearDown`/`beforeEach`/`afterEach` — фикстуры.
fn is_entry_point_or_test(name: &str) -> bool {
    if name == "main" || name == "Main" {
        return true;
    }
    // Test* — Go/JS convention (TestHealthStatus, testSomething в lowerCamel).
    if name.starts_with("Test") || name.starts_with("test") {
        return true;
    }
    // Фикстуры.
    matches!(
        name,
        "setUp" | "tearDown" | "setup" | "teardown"
            | "beforeEach" | "afterEach" | "beforeAll" | "afterAll"
            | "before_each" | "after_each" | "before_all" | "after_all"
    )
}

/// Вспомогательная функция: собирает множество всех имён символов.
/// Удобно, чтобы быстро подготовить «используемые» имена для других проверок.
pub fn collect_symbol_names(symbols: &[Symbol]) -> HashSet<&str> {
    symbols.iter().map(|s| s.name.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::SymbolKind;
    use std::path::PathBuf;

    #[test]
    fn flags_unused_symbol() {
        let syms = vec![Symbol::new(
            "ghost",
            SymbolKind::Function,
            PathBuf::from("a.rs"),
            1,
        )];
        // corpus пустой → ghost не используется.
        let findings = find_unused(&syms, &[]);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn ignores_used_symbol() {
        let syms = vec![Symbol::new(
            "used",
            SymbolKind::Function,
            PathBuf::from("a.rs"),
            1,
        )];
        // corpus включает текст проекта с определением И использованием (≥2 вхождений).
        let corpus = vec![
            "fn used() {}".to_string(), // определение
            "let x = used();".to_string(), // использование
        ];
        assert!(find_unused(&syms, &corpus).is_empty());
    }

    #[test]
    fn does_not_flag_main_and_tests() {
        let syms = vec![
            Symbol::new("main", SymbolKind::Function, PathBuf::from("a.rs"), 1),
            Symbol::new("TestHealth", SymbolKind::Function, PathBuf::from("a_test.go"), 1),
            Symbol::new("setUp", SymbolKind::Function, PathBuf::from("a.py"), 1),
        ];
        // Все три — точки входа/тесты, corpus пустой, но флагаться не должны.
        assert!(find_unused(&syms, &[]).is_empty());
    }

    #[test]
    fn entry_point_detection() {
        assert!(is_entry_point_or_test("main"));
        assert!(is_entry_point_or_test("TestHealthStatus"));
        assert!(is_entry_point_or_test("test_parse"));
        assert!(is_entry_point_or_test("setUp"));
        assert!(!is_entry_point_or_test("parse"));
        assert!(!is_entry_point_or_test("UserService"));
    }
}

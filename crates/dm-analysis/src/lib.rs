//! # dm-analysis
//!
//! Анализатор исходного кода на базе tree-sitter. Извлекает символы (функции,
//! классы, структуры) с их сигнатурами и doc-комментариями, проверяет принципы
//! DRY/KISS, ищет дублирующиеся определения и неиспользуемый код.
//!
//! ## Архитектура
//! - Единый трейт [`parser::LanguageParser`] — точка расширения для новых языков.
//! - Реестр [`parser::parser_for_extension`] выбирает парсер по расширению файла.
//! - Все грамматики — это Rust-биндинги к C-грамматикам tree-sitter; для их
//!   сборки нужен C-компилятор (см. `docs/*/installation.md`).
//!
//! ## MVP-языки
//! Rust, JavaScript, TypeScript, Go. Остальные языки добавляются подключением
//! соответствующего crate'а и реализации [`parser::LanguageParser`].

pub mod diff;
pub mod docs;
pub mod graph;
pub mod languages;
pub mod lints;
pub mod parser;
pub mod refs;
pub mod search;
pub mod secrets;
pub mod symbols;

pub use diff::{changed_symbols, ChangedCodeSymbol};
pub use graph::{DependencyGraph, FileNode, PathNode};
pub use parser::{parse_file, parse_file_str, LanguageParser, ParsedFile, parser_for_extension};
pub use symbols::{Symbol, SymbolKind};

/// Лексическая нормализация пути (схлопывание `..` и `.` без обращения к ФС).
///
/// Переэкспорт внутренней утилиты для использования в модулях графа.
pub fn normalize_lexical_pub(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out: Vec<Component> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(comp),
            },
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out.iter().collect()
}

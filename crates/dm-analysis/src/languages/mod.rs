//! Языковые привязки tree-sitter.
//!
//! Каждый модуль реализует [`crate::parser::LanguageParser`] для своего языка.
//! Общая утилита — [`node_text`] для извлечения текста узла и [`line_for_byte`]
//! для перевода байтовых смещений в номера строк (нужно всем языкам, DRY).

pub mod go;
pub mod javascript;
pub mod rust;
pub mod typescript;

use tree_sitter::Node;

/// Возвращает текст, соответствующий узлу, из исходника.
pub fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    if start <= end && end <= source.len() {
        &source[start..end]
    } else {
        ""
    }
}

/// Преобразует байтовое смещение в 1-based номер строки.
pub fn line_for_byte(byte: usize, source: &str) -> usize {
    let upto = source.len().min(byte);
    source[..upto].matches('\n').count() + 1
}

/// Возвращает 1-based стартовую строку узла (обёртка над методом tree-sitter).
pub fn start_row(node: &Node) -> usize {
    node.start_position().row + 1
}

/// Возвращает 1-based конечную строку узла.
pub fn end_row(node: &Node) -> usize {
    node.end_position().row + 1
}

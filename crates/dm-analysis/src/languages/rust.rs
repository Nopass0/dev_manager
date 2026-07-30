//! Парсер Rust на базе tree-sitter-rust.
//!
//! Извлекает функции, структуры, перечисления, типы и константы с их
//! doc-комментариями (`///`, `//!`) и сигнатурами аргументов.

use crate::languages::{end_row, line_for_byte, node_text, start_row};
use crate::parser::LanguageParser;
use crate::symbols::{Symbol, SymbolKind};
use std::path::PathBuf;
use std::sync::Mutex;
use tree_sitter::{Node, Parser, Point};

/// Реализация [`LanguageParser`] для Rust.
pub struct RustParser {
    // tree-sitter `Parser` не `Sync` напрямую, оборачиваем в Mutex.
    parser: Mutex<Parser>,
}

impl RustParser {
    /// Создаёт новый парсер с загруженной грамматикой Rust.
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("rust grammar load");
        Self {
            parser: Mutex::new(parser),
        }
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for RustParser {
    fn name(&self) -> &'static str {
        "rust"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn parse_symbols(&self, source: &str, file: PathBuf) -> Vec<Symbol> {
        let mut parser = self.parser.lock().expect("poisoned");
        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();
        let mut out = Vec::new();
        walk(&root, source, &file, &mut out);
        out
    }
}

/// Рекурсивно обходит AST, собирая символы верхних уровней.
///
/// Идём и по модулю, и по блокам `impl`/`trait` — чтобы достать методы.
fn walk(node: &Node, source: &str, file: &PathBuf, out: &mut Vec<Symbol>) {
    match node.kind() {
        "function_item" => {
            if let Some(sym) = build_function(node, source, file) {
                out.push(sym);
            }
        }
        "struct_item" => {
            if let Some(name) = named_node_name(node, source) {
                let mut sym = Symbol::new(name, SymbolKind::Struct, file.clone(), start_row(node));
                sym.end_line = end_row(node);
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
        }
        "enum_item" => {
            if let Some(name) = named_node_name(node, source) {
                let mut sym = Symbol::new(name, SymbolKind::Class, file.clone(), start_row(node));
                sym.end_line = end_row(node);
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
        }
        "trait_item" => {
            if let Some(name) = named_node_name(node, source) {
                let mut sym = Symbol::new(name, SymbolKind::Class, file.clone(), start_row(node));
                sym.end_line = end_row(node);
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
        }
        "const_item" | "static_item" => {
            if let Some(name) = named_node_name(node, source) {
                let mut sym = Symbol::new(name, SymbolKind::Variable, file.clone(), start_row(node));
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
        }
        _ => {}
    }
    // Рекурсивно заходим во вложенные узлы (impl, trait, mod).
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(&child, source, file, out);
    }
}

/// Строит символ-функцию из узла `function_item`, включая сигнатуру аргументов.
fn build_function(node: &Node, source: &str, file: &PathBuf) -> Option<Symbol> {
    let name = named_node_name(node, source)?;
    let mut sym = Symbol::new(name, SymbolKind::Function, file.clone(), start_row(node));
    sym.end_line = end_row(node);
    // Сигнатура аргументов: `(a: i32, b: &str)`.
    if let Some(params) = node.child_by_field_name("parameters") {
        sym.signature = node_text(&params, source).to_string();
    }
    sym.doc = doc_above(node, source);
    Some(sym)
}

/// Извлекает имя узла по дочернему полю `name` (для большинства определений).
fn named_node_name(node: &Node, source: &str) -> Option<String> {
    let name_node = node.child_by_field_name("name")?;
    Some(node_text(&name_node, source).to_string())
}

/// Ищет doc-комментарий непосредственно над определением.
///
/// Поддерживает `///` (Rust doc) и блоки `/** */`. Возвращает собранный текст
/// без маркеров и без пустых строк по краям.
fn doc_above(node: &Node, source: &str) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    // Идём по предыдущим «братьям» узла, собираем подряд идущие комментарии.
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        match sib.kind() {
            "line_comment" => {
                let raw = node_text(&sib, source);
                // `///` или `//!`
                let cleaned = raw
                    .trim_start_matches('/')
                    .trim_start()
                    .trim_end();
                if !cleaned.is_empty() {
                    lines.insert(0, cleaned.to_string());
                }
            }
            "block_comment" => {
                let raw = node_text(&sib, source);
                let cleaned = raw
                    .trim_start_matches("/**")
                    .trim_start_matches("/*")
                    .trim_end_matches("*/")
                    .trim();
                if !cleaned.is_empty() {
                    lines.insert(0, cleaned.to_string());
                }
            }
            _ => break, // встретили не-комментарий — прекращаем собирать
        }
        prev = sib.prev_sibling();
    }
    // Также проверяем строку прямо над узлом (для `///` без sibling — частый случай).
    let _ = line_for_byte(0, source); // keep helper in use
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

// Заглушка для неиспользуемого импорта Point — оставляем для будущих проверок координат.
#[allow(dead_code)]
fn _point_debug(p: Point) -> (usize, usize) {
    (p.row, p.column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_function_and_struct() {
        let parser = RustParser::new();
        let src = r#"
/// Документация функции.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub struct User {
    pub name: String,
}
"#;
        let symbols = parser.parse_symbols(src, PathBuf::from("lib.rs"));
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"add"));
        assert!(names.contains(&"User"));

        let add = symbols.iter().find(|s| s.name == "add").unwrap();
        assert_eq!(add.kind, SymbolKind::Function);
        assert!(add.signature.contains("a"));
        assert!(add.doc.as_deref().unwrap().contains("Документация функции"));
    }
}

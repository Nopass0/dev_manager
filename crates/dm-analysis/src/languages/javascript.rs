//! Парсер JavaScript на базе tree-sitter-javascript.
//!
//! Извлекает function-declarations, arrow functions (на верхнем уровне), классы
//! и их методы, а также экспортируемые переменные/константы. Поддерживает JSDoc.

use crate::languages::{end_row, node_text, start_row};
use crate::parser::LanguageParser;
use crate::symbols::{Symbol, SymbolKind};
use std::path::PathBuf;
use std::sync::Mutex;
use tree_sitter::{Node, Parser};

/// Реализация [`LanguageParser`] для JavaScript (и JSX).
pub struct JavaScriptParser {
    parser: Mutex<Parser>,
}

impl JavaScriptParser {
    /// Создаёт парсер с грамматикой JavaScript.
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .expect("javascript grammar load");
        Self {
            parser: Mutex::new(parser),
        }
    }
}

impl Default for JavaScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for JavaScriptParser {
    fn name(&self) -> &'static str {
        "javascript"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "mjs", "cjs"]
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

fn walk(node: &Node, source: &str, file: &PathBuf, out: &mut Vec<Symbol>) {
    match node.kind() {
        "function_declaration" => {
            if let Some(sym) = build_function(node, source, file) {
                out.push(sym);
            }
        }
        "class_declaration" => {
            if let Some(name) = named_child_text(node, "name", source) {
                let mut sym = Symbol::new(name, SymbolKind::Class, file.clone(), start_row(node));
                sym.end_line = end_row(node);
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
            // Методы класса.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(child.kind(), "method_definition" | "getter" | "setter") {
                    if let Some(sym) = build_method(&child, source, file) {
                        out.push(sym);
                    }
                }
            }
        }
        "method_definition" => {
            if let Some(sym) = build_method(node, source, file) {
                out.push(sym);
            }
        }
        "variable_declaration" | "lexical_declaration" => {
            // Имя и значение первой декларации (если это arrow function — фиксируем).
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    if let Some(name) = named_child_text(&child, "name", source) {
                        let is_fn = child
                            .child_by_field_name("value")
                            .map(|v| matches!(v.kind(), "arrow_function" | "function_expression"))
                            .unwrap_or(false);
                        let kind = if is_fn {
                            SymbolKind::Function
                        } else {
                            SymbolKind::Variable
                        };
                        let mut sym = Symbol::new(name, kind, file.clone(), start_row(node));
                        sym.doc = doc_above(node, source);
                        out.push(sym);
                    }
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(&child, source, file, out);
    }
}

fn build_function(node: &Node, source: &str, file: &PathBuf) -> Option<Symbol> {
    let name = named_child_text(node, "name", source)?;
    let mut sym = Symbol::new(name, SymbolKind::Function, file.clone(), start_row(node));
    sym.end_line = end_row(node);
    if let Some(params) = node.child_by_field_name("parameters") {
        sym.signature = node_text(&params, source).to_string();
    }
    sym.doc = doc_above(node, source);
    Some(sym)
}

fn build_method(node: &Node, source: &str, file: &PathBuf) -> Option<Symbol> {
    let name = named_child_text(node, "name", source)?;
    let mut sym = Symbol::new(name, SymbolKind::Function, file.clone(), start_row(node));
    if let Some(params) = node.child_by_field_name("parameters") {
        sym.signature = node_text(&params, source).to_string();
    }
    sym.doc = doc_above(node, source);
    Some(sym)
}

/// Достаёт текст named-поля по имени поля (общий helper для всех языков с field-api).
fn named_child_text(node: &Node, field: &str, source: &str) -> Option<String> {
    let n = node.child_by_field_name(field)?;
    Some(node_text(&n, source).to_string())
}

/// JSDoc/`//`-комментарий над определением.
///
/// Учитывает, что в JS/TS определение часто обёрнуто в `export_statement`:
/// комментарий оказывается перед родителем, а не перед самой функцией.
fn doc_above(node: &Node, source: &str) -> Option<String> {
    // Сначала ищем комментарий прямо над узлом; если ничего нет и узел обёрнут
    // в export_statement — пробуем родителя.
    if let Some(d) = doc_above_sibling(node, source) {
        return Some(d);
    }
    if let Some(parent) = node.parent() {
        if parent.kind() == "export_statement" {
            if let Some(d) = doc_above_sibling(&parent, source) {
                return Some(d);
            }
        }
    }
    None
}

/// Идёт по предыдущим sibling-узлам, собирая подряд идущие комментарии.
fn doc_above_sibling(node: &Node, source: &str) -> Option<String> {
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "comment" {
            let raw = node_text(&sib, source);
            let cleaned = clean_js_comment(raw);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        } else {
            break;
        }
        prev = sib.prev_sibling();
    }
    None
}

/// Приводит JSDoc/`//` к чистому тексту.
fn clean_js_comment(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed.strip_prefix("/**").and_then(|s| s.strip_suffix("*/")) {
        // JSDoc: убираем ведущие `*`.
        inner
            .lines()
            .map(|l| l.trim().trim_start_matches('*').trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    } else if let Some(line) = trimmed.strip_prefix("//") {
        line.trim().to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_js_function_and_class() {
        let parser = JavaScriptParser::new();
        let src = r#"
/** Складывает два числа. */
export function add(a, b) {
  return a + b;
}

class Counter {
  /** Текущее значение. */
  current() { return this.n; }
}

export const ARROW = (x) => x + 1;
"#;
        let syms = parser.parse_symbols(src, PathBuf::from("a.js"));
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"add"));
        assert!(names.contains(&"Counter"));
        assert!(names.contains(&"current"));
        assert!(names.contains(&"ARROW"));

        let add = syms.iter().find(|s| s.name == "add").unwrap();
        assert!(add.doc.as_deref().unwrap().contains("Складывает"));
        let arrow = syms.iter().find(|s| s.name == "ARROW").unwrap();
        assert_eq!(arrow.kind, SymbolKind::Function); // arrow function
    }
}

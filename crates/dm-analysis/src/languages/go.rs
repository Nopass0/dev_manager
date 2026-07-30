//! Парсер Go на базе tree-sitter-go.
//!
//! Извлекает функции, методы (function_declaration с receiver), типы (struct,
//! interface) и верхнеуровневые переменные/константы. Doc-комментарии Go (`//`)
//! собираются как предшествующие узлу строки.

use crate::languages::{end_row, node_text, start_row};
use crate::parser::LanguageParser;
use crate::symbols::{Symbol, SymbolKind};
use std::path::PathBuf;
use std::sync::Mutex;
use tree_sitter::{Node, Parser};

/// Реализация [`LanguageParser`] для Go.
pub struct GoParser {
    parser: Mutex<Parser>,
}

impl GoParser {
    /// Создаёт парсер с грамматикой Go.
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .expect("go grammar load");
        Self {
            parser: Mutex::new(parser),
        }
    }
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for GoParser {
    fn name(&self) -> &'static str {
        "go"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["go"]
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
            if let Some(name) = named_child_text(node, "name", source) {
                let mut sym = Symbol::new(name, SymbolKind::Function, file.clone(), start_row(node));
                sym.end_line = end_row(node);
                if let Some(params) = node.child_by_field_name("parameters") {
                    sym.signature = node_text(&params, source).to_string();
                }
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
        }
        "method_declaration" => {
            if let Some(name) = named_child_text(node, "name", source) {
                let mut sym = Symbol::new(name, SymbolKind::Function, file.clone(), start_row(node));
                sym.end_line = end_row(node);
                if let Some(params) = node.child_by_field_name("parameters") {
                    sym.signature = node_text(&params, source).to_string();
                }
                sym.doc = doc_above(node, source);
                out.push(sym);
            }
        }
        "type_declaration" => {
            // Может содержать несколько type_spec.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_spec" {
                    if let Some(name) = named_child_text(&child, "name", source) {
                        // struct/interface — определяем по дочернему типу.
                        let kind = child
                            .child_by_field_name("type")
                            .map(|t| match t.kind() {
                                "struct_type" => SymbolKind::Struct,
                                "interface_type" => SymbolKind::Class,
                                _ => SymbolKind::Other,
                            })
                            .unwrap_or(SymbolKind::Other);
                        let mut sym = Symbol::new(name, kind, file.clone(), start_row(&child));
                        sym.doc = doc_above(node, source);
                        out.push(sym);
                    }
                }
            }
        }
        "var_declaration" | "const_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "var_spec" || child.kind() == "const_spec" {
                    // Имя берём из первого `identifier` ребёнка.
                    let mut name_cur = child.walk();
                    for c in child.children(&mut name_cur) {
                        if c.kind() == "identifier" {
                            let text = node_text(&c, source).to_string();
                            let mut sym = Symbol::new(text, SymbolKind::Variable, file.clone(), start_row(node));
                            sym.doc = doc_above(node, source);
                            out.push(sym);
                            break;
                        }
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

fn named_child_text(node: &Node, field: &str, source: &str) -> Option<String> {
    let n = node.child_by_field_name(field)?;
    Some(node_text(&n, source).to_string())
}

fn doc_above(node: &Node, source: &str) -> Option<String> {
    // Go использует `//` комментарии над определениями.
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "comment" {
            let raw = node_text(&sib, source);
            let cleaned = raw.trim_start_matches('/').trim().to_string();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_go_func_struct_interface() {
        let parser = GoParser::new();
        let src = r#"
package main

// User представляет пользователя.
type User struct {
	Name string
}

// Authenticator описывает сервис аутентификации.
type Authenticator interface {
	Login(u string) bool
}

// Login проверяет учётные данные.
func (s *Service) Login(user string) bool {
	return true
}

func main() {}
"#;
        let syms = parser.parse_symbols(src, PathBuf::from("main.go"));
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"User"));
        assert!(names.contains(&"Authenticator"));
        assert!(names.contains(&"Login"));
        assert!(names.contains(&"main"));

        let user = syms.iter().find(|s| s.name == "User").unwrap();
        assert_eq!(user.kind, SymbolKind::Struct);
        assert!(user.doc.as_deref().unwrap().contains("пользователя"));

        let auth = syms.iter().find(|s| s.name == "Authenticator").unwrap();
        assert_eq!(auth.kind, SymbolKind::Class);
    }
}

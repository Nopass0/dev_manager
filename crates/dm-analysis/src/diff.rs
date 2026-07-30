//! Сравнение символов до/после изменения — основа `dm commit auto`.
//!
//! На входе: разобранные символы «до» и «после». На выходе — список изменившихся
//! символов с типом изменения, из которого формируется читаемое сообщение коммита.

use crate::symbols::{Symbol, SymbolKind};
use std::collections::HashMap;
use std::path::PathBuf;

/// Тип изменения символа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Символ добавлен.
    Added,
    /// Символ удалён.
    Removed,
    /// Символ модифицирован (тело/сигнатура/док).
    Modified,
}

/// Одно изменение символа, готовое для вывода в сообщении коммита.
#[derive(Debug, Clone)]
pub struct ChangedCodeSymbol {
    /// Имя символа.
    pub name: String,
    /// Категория символа.
    pub kind: SymbolKind,
    /// Тип изменения.
    pub change: ChangeKind,
    /// Путь к файлу.
    pub file: PathBuf,
}

/// Вычисляет изменившиеся символы между двумя версиями файла.
///
/// Сопоставление — по имени символа в рамках одного файла. Это намеренно
/// простая стратегия: она устойчива к переименованиям внутри функции и
/// достаточно точна для типового diff'а при коммите.
pub fn changed_symbols(before: &[Symbol], after: &[Symbol]) -> Vec<ChangedCodeSymbol> {
    let mut before_map: HashMap<(String, String), &Symbol> = HashMap::new();
    for s in before {
        let key = (s.file.to_string_lossy().into_owned(), s.name.clone());
        before_map.insert(key, s);
    }

    let mut after_keys: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    let mut out = Vec::new();

    // Добавленные / модифицированные.
    for s in after {
        let key = (s.file.to_string_lossy().into_owned(), s.name.clone());
        after_keys.insert(key.clone());
        match before_map.get(&key) {
            None => out.push(ChangedCodeSymbol {
                name: s.name.clone(),
                kind: s.kind,
                change: ChangeKind::Added,
                file: s.file.clone(),
            }),
            Some(prev) => {
                if symbol_differs(prev, s) {
                    out.push(ChangedCodeSymbol {
                        name: s.name.clone(),
                        kind: s.kind,
                        change: ChangeKind::Modified,
                        file: s.file.clone(),
                    });
                }
            }
        }
    }
    // Удалённые.
    for s in before {
        let key = (s.file.to_string_lossy().into_owned(), s.name.clone());
        if !after_keys.contains(&key) {
            out.push(ChangedCodeSymbol {
                name: s.name.clone(),
                kind: s.kind,
                change: ChangeKind::Removed,
                file: s.file.clone(),
            });
        }
    }
    out
}

/// Сравнивает два символа на существенное изменение.
fn symbol_differs(a: &Symbol, b: &Symbol) -> bool {
    a.signature != b.signature || a.doc != b.doc || a.end_line != b.end_line
}

impl ChangedCodeSymbol {
    /// Человекочитаемое описание изменения одной строкой.
    pub fn describe(&self) -> String {
        let action = match self.change {
            ChangeKind::Added => "добавлена",
            ChangeKind::Removed => "удалена",
            ChangeKind::Modified => "изменена",
        };
        let kind_label = self.kind.label();
        format!(
            "{} {} '{}' ({})",
            action,
            kind_label,
            self.name,
            self.file.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn f(name: &str, sig: &str, line: usize) -> Symbol {
        let mut s = Symbol::new(name, SymbolKind::Function, PathBuf::from("a.rs"), line);
        s.signature = sig.into();
        s.end_line = line + 5;
        s
    }

    #[test]
    fn detects_modified() {
        let before = vec![f("foo", "(a: i32)", 1)];
        let after = vec![f("foo", "(a: i32, b: i32)", 1)];
        let changes = changed_symbols(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change, ChangeKind::Modified);
        assert!(changes[0].describe().contains("изменена"));
    }

    #[test]
    fn detects_added_and_removed() {
        let before = vec![f("a", "()", 1)];
        let after = vec![f("b", "()", 1)];
        let changes = changed_symbols(&before, &after);
        assert_eq!(changes.len(), 2);
        assert!(
            changes
                .iter()
                .any(|c| c.change == ChangeKind::Removed && c.name == "a")
        );
        assert!(
            changes
                .iter()
                .any(|c| c.change == ChangeKind::Added && c.name == "b")
        );
    }

    #[test]
    fn no_changes_when_identical() {
        let before = vec![f("a", "()", 1)];
        let after = vec![f("a", "()", 1)];
        assert!(changed_symbols(&before, &after).is_empty());
    }
}

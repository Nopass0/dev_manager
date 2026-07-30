//! Поиск дублирующихся определений.
//!
//! Один и тот же класс/функция/структура, определённая в разных файлах под одним
//! именем — частый источник путаницы в монорепозиториях. Этот линтер подсвечивает
//! такие случаи.

use crate::lints::{LintCategory, LintFinding};
use crate::symbols::Symbol;
use std::collections::HashMap;

/// Находит символы с одинаковыми `(name, kind)` в разных файлах.
///
/// Группировка идёт по `(имя, категория)`; если в группе больше одного файла —
/// для каждого создаётся отдельное замечание.
pub fn find_duplicates(symbols: &[Symbol]) -> Vec<LintFinding> {
    // Группируем по (имя, категория) → список путей.
    let mut groups: HashMap<(String, u8), Vec<&Symbol>> = HashMap::new();
    for s in symbols {
        // u8 как компактное представление kind; stable hash через u8.
        let key = (s.name.clone(), s.kind as u8);
        groups.entry(key).or_default().push(s);
    }

    let mut out = Vec::new();
    for ((name, _kind), group) in groups {
        // Уникальные файлы в группе.
        let unique_files: Vec<_> = group
            .iter()
            .map(|s| s.file.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if unique_files.len() > 1 {
            for s in &group {
                out.push(LintFinding {
                    category: LintCategory::Duplicate,
                    message: format!(
                        "символ '{}' определён в {} разных файлах — возможен конфликт имен",
                        name,
                        unique_files.len()
                    ),
                    file: s.file.clone(),
                    symbol: Some(name.clone()),
                    line: Some(s.start_line),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::SymbolKind;
    use std::path::PathBuf;

    #[test]
    fn flags_same_name_in_two_files() {
        let syms = vec![
            Symbol::new("User", SymbolKind::Struct, PathBuf::from("a.rs"), 1),
            Symbol::new("User", SymbolKind::Struct, PathBuf::from("b.rs"), 5),
            Symbol::new("Unique", SymbolKind::Function, PathBuf::from("a.rs"), 9),
        ];
        let findings = find_duplicates(&syms);
        assert_eq!(findings.len(), 2); // по одному на каждое определение User
        assert!(findings.iter().all(|f| f.symbol.as_deref() == Some("User")));
    }

    #[test]
    fn ignores_single_definition() {
        let syms = vec![Symbol::new(
            "Solo",
            SymbolKind::Function,
            PathBuf::from("a.rs"),
            1,
        )];
        assert!(find_duplicates(&syms).is_empty());
    }
}

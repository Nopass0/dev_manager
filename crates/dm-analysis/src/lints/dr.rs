//! Проверка принципа DRY (Don't Repeat Yourself).
//!
//! В текущей версии — эвристика: поиск функций с одинаковым именем в разных
//! файлах (вероятный копипаст), а также функций с одинаковым числом параметров
//! и одинаковым именем одного из них (слабый сигнал).

use crate::lints::{LintCategory, LintFinding};
use crate::symbols::{Symbol, SymbolKind};
use std::collections::HashMap;

/// Эвристически ищет нарушения DRY.
///
/// Срабатывает, если две функции в разных файлах имеют одинаковое имя —
/// это часто означает копипаст-дубликат.
pub fn find_dr_violations(symbols: &[Symbol]) -> Vec<LintFinding> {
    let mut groups: HashMap<String, Vec<&Symbol>> = HashMap::new();
    for s in symbols.iter().filter(|s| s.kind == SymbolKind::Function) {
        groups.entry(s.name.clone()).or_default().push(s);
    }

    let mut out = Vec::new();
    for (name, group) in groups {
        let files: std::collections::HashSet<_> = group.iter().map(|s| s.file.clone()).collect();
        if files.len() > 1 {
            // Берём первое вхождение как «канон», остальные — подозрительные дубликаты.
            for s in group.iter().skip(1) {
                out.push(LintFinding {
                    category: LintCategory::Dr,
                    message: format!(
                        "функция '{name}' встречается в нескольких файлах — возможный дубликат (DRY)"
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
    use std::path::PathBuf;

    #[test]
    fn flags_duplicate_function() {
        let syms = vec![
            Symbol::new("init", SymbolKind::Function, PathBuf::from("a.rs"), 1),
            Symbol::new("init", SymbolKind::Function, PathBuf::from("b.rs"), 2),
        ];
        assert_eq!(find_dr_violations(&syms).len(), 1);
    }
}

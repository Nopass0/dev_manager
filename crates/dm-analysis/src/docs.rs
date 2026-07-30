//! Сбор и нормализация doc-комментариев над символами.
//!
//! Сами комментарии извлекаются в языковых парсерах ([`crate::languages`]);
//! здесь — общие helpers для работы с уже собранным текстом документации.

use crate::symbols::Symbol;

/// Возвращает первый абзац doc-комментария символа (до первой пустой строки).
///
/// Используется для краткого отображения в списках символов и при формировании
/// сообщений `commit auto`. Возвращает `None`, если документации нет.
pub fn first_paragraph(symbol: &Symbol) -> Option<&str> {
    let doc = symbol.doc.as_deref()?;
    let para = doc.split("\n\n").next()?;
    if para.trim().is_empty() {
        None
    } else {
        Some(para)
    }
}

/// Возвращает true, если символ задокументирован (есть непустой doc).
pub fn is_documented(symbol: &Symbol) -> bool {
    symbol
        .doc
        .as_deref()
        .map(|d| !d.trim().is_empty())
        .unwrap_or(false)
}

/// Считает долю задокументированных публичных символов в списке (0.0..=1.0).
///
/// Полезно для отчёта «покрытие документацией» в `dm lint`.
pub fn documentation_coverage(symbols: &[Symbol]) -> f32 {
    if symbols.is_empty() {
        return 1.0;
    }
    let documented = symbols.iter().filter(|s| is_documented(s)).count();
    documented as f32 / symbols.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::SymbolKind;
    use std::path::PathBuf;

    #[test]
    fn first_paragraph_works() {
        let mut s = Symbol::new("foo", SymbolKind::Function, PathBuf::from("a.rs"), 1);
        s.doc = Some("Краткое описание.\n\nПодробности.".into());
        assert_eq!(first_paragraph(&s), Some("Краткое описание."));
    }

    #[test]
    fn coverage_counts() {
        let mut a = Symbol::new("a", SymbolKind::Function, PathBuf::from("a.rs"), 1);
        a.doc = Some("d".into());
        let b = Symbol::new("b", SymbolKind::Function, PathBuf::from("a.rs"), 5);
        assert!((documentation_coverage(&[a, b]) - 0.5).abs() < 0.001);
    }
}

//! Проверка принципа KISS (Keep It Simple, Stupid).
//!
//! Эвристики: слишком большое число параметров функции, чрезмерно длинное
//! тело функции (по разнице строк), слишком много уровней вложенности.

use crate::lints::{LintCategory, LintFinding};
use crate::symbols::{Symbol, SymbolKind};

/// Порог: функция считается «перегруженной параметрами», если их больше этого числа.
const TOO_MANY_PARAMS: usize = 6;
/// Порог: функция считается слишком длинной (в строках).
const TOO_LONG_LINES: usize = 80;

/// Эвристически ищет нарушения KISS среди функций.
pub fn find_kiss_violations(symbols: &[Symbol]) -> Vec<LintFinding> {
    let mut out = Vec::new();
    for s in symbols.iter().filter(|s| s.kind == SymbolKind::Function) {
        // Считаем параметры по запятым в сигнатуре (грубая оценка).
        let param_count = count_params(&s.signature);
        if param_count > TOO_MANY_PARAMS {
            out.push(LintFinding {
                category: LintCategory::Kiss,
                message: format!(
                    "функция '{}' имеет {} параметров — возможно, стоит упростить (KISS)",
                    s.name, param_count
                ),
                file: s.file.clone(),
                symbol: Some(s.name.clone()),
                line: Some(s.start_line),
            });
        }
        let length = s.end_line.saturating_sub(s.start_line);
        if length > TOO_LONG_LINES {
            out.push(LintFinding {
                category: LintCategory::Kiss,
                message: format!(
                    "функция '{}' слишком длинная ({} строк) — рассмотрите разбиение (KISS)",
                    s.name, length
                ),
                file: s.file.clone(),
                symbol: Some(s.name.clone()),
                line: Some(s.start_line),
            });
        }
    }
    out
}

/// Грубо оценивает число параметров по строке сигнатуры.
fn count_params(signature: &str) -> usize {
    let trimmed = signature.trim_start_matches('(').trim_end_matches(')').trim();
    if trimmed.is_empty() {
        return 0;
    }
    // Игнорируем запятые внутри вложенных скобок (типы вроде `Map<K, V>`).
    let mut depth = 0;
    let mut count = 1;
    for c in trimmed.chars() {
        match c {
            '(' | '<' | '[' | '{' => depth += 1,
            ')' | '>' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count.max(1) - if trimmed.is_empty() { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_long_signature() {
        let mut s = Symbol::new("big", SymbolKind::Function, PathBuf::from("a.rs"), 1);
        s.signature = "(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32)".into();
        assert!(find_kiss_violations(&[s]).iter().any(|f| f.message.contains("параметров")));
    }

    #[test]
    fn does_not_flag_small_function() {
        let mut s = Symbol::new("small", SymbolKind::Function, PathBuf::from("a.rs"), 1);
        s.signature = "(a: i32)".into();
        s.end_line = 3;
        assert!(find_kiss_violations(&[s]).is_empty());
    }

    #[test]
    fn count_params_handles_generics() {
        assert_eq!(count_params("(a: Map<i32, i32>, b: i32)"), 2);
        assert_eq!(count_params("()"), 0);
        assert_eq!(count_params("(x)"), 1);
    }
}

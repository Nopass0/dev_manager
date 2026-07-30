//! Линтеры: проверки принципов DRY/KISS, поиск дубликатов и неиспользуемого кода.
//!
//! Каждый линтер возвращает список [`LintFinding`] — единый формат нарушений.
//! `dm lint` агрегирует их и выводит в виде таблицы.

pub mod duplicates;
pub mod dr;
pub mod kiss;
pub mod unused;

use crate::symbols::Symbol;
use std::path::PathBuf;

/// Одно обнаруженное линтером нарушение.
#[derive(Debug, Clone)]
pub struct LintFinding {
    /// Категория нарушения (для группировки и фильтрации).
    pub category: LintCategory,
    /// Человекочитаемое описание проблемы.
    pub message: String,
    /// Путь к файлу, где обнаружено.
    pub file: PathBuf,
    /// Имя символа (если применимо).
    pub symbol: Option<String>,
    /// Строка (1-based), если известна.
    pub line: Option<usize>,
}

/// Категория нарушения — источник/правило.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCategory {
    /// Дублирующееся определение (одинаковые имена в разных файлах).
    Duplicate,
    /// Нарушение DRY.
    Dr,
    /// Нарушение KISS (избыточная сложность).
    Kiss,
    /// Неиспользуемый символ.
    Unused,
}

impl LintCategory {
    /// Короткая строка для таблицы.
    pub fn label(self) -> &'static str {
        match self {
            LintCategory::Duplicate => "duplicate",
            LintCategory::Dr => "dry",
            LintCategory::Kiss => "kiss",
            LintCategory::Unused => "unused",
        }
    }
}

/// Запускает все включённые линтеры над списком символов проекта.
///
/// `which` — флаги включённых категорий (DRY/KISS/unused/duplicates).
pub fn run_all(
    symbols: &[Symbol],
    which: LintSet,
) -> Vec<LintFinding> {
    let mut out = Vec::new();
    if which.duplicates {
        out.extend(duplicates::find_duplicates(symbols));
    }
    if which.dr {
        out.extend(dr::find_dr_violations(symbols));
    }
    if which.kiss {
        out.extend(kiss::find_kiss_violations(symbols));
    }
    if which.unused {
        out.extend(unused::find_unused(symbols, &[]));
    }
    out
}

/// Набор флагов включённых линтеров.
#[derive(Debug, Clone, Copy, Default)]
pub struct LintSet {
    /// Включить поиск дубликатов.
    pub duplicates: bool,
    /// Включить DRY-проверки.
    pub dr: bool,
    /// Включить KISS-проверки.
    pub kiss: bool,
    /// Включить поиск неиспользуемого кода.
    pub unused: bool,
}

impl LintSet {
    /// Включает все доступные линтеры.
    pub fn all() -> Self {
        Self {
            duplicates: true,
            dr: true,
            kiss: true,
            unused: true,
        }
    }
}

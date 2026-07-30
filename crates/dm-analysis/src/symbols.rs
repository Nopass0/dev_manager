//! Доменная модель символов кода, не зависящая от tree-sitter.

use std::path::PathBuf;

/// Категория извлечённого символа.
///
/// Унифицирована для всех языков: хотя в каждом языке свои синтаксические формы
/// (function/method/closure, class/struct/record, …), мы приводим их к одному
/// набору категорий, чтобы линтеры и diff работали единообразно.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Функция, метод, процедура, замыкание верхнего уровня.
    Function,
    /// Класс, type, interface, trait, protocol.
    Class,
    /// Структура, record, data class.
    Struct,
    /// Переменная/константа/field на верхнем уровне.
    Variable,
    /// Прочий поименованный элемент (макрос, module, enum variant…).
    Other,
}

impl SymbolKind {
    /// Человекочитаемое русское название.
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Function => "функция",
            SymbolKind::Class => "класс",
            SymbolKind::Struct => "структура",
            SymbolKind::Variable => "переменная",
            SymbolKind::Other => "символ",
        }
    }
}

/// Извлечённый символ кода: функция, класс, структура и т.д.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Имя символа (например, `parse_symbols` или `UserService`).
    pub name: String,
    /// Категория символа.
    pub kind: SymbolKind,
    /// Сигнатура аргументов как текст (например, `(a: i32, b: &str)`).
    /// Для классов/структур может содержать поля или быть пустым.
    pub signature: String,
    /// Doc-комментарий над символом (JSDoc, `///`, `/** */`…), если есть.
    pub doc: Option<String>,
    /// Путь к файлу, где определён символ.
    pub file: PathBuf,
    /// Номер строки начала определения (1-based).
    pub start_line: usize,
    /// Номер строки конца определения (1-based).
    pub end_line: usize,
}

impl Symbol {
    /// Создаёт символ с обязательными полями; опциональные — по умолчанию.
    #[inline]
    pub fn new(name: impl Into<String>, kind: SymbolKind, file: PathBuf, start_line: usize) -> Self {
        Self {
            name: name.into(),
            kind,
            signature: String::new(),
            doc: None,
            file,
            start_line,
            end_line: start_line,
        }
    }
}

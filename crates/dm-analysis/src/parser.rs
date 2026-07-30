//! Точка расширения анализатора: трейт [`LanguageParser`] и реестр парсеров.
//!
//! Добавление нового языка = реализация этого трейта + регистрация в
//! [`parser_for_extension`]. Внутри реализация обычно делегирует в modules
//! `languages::*`, где живёт tree-sitter-специфичная логика.

use crate::languages;
use crate::symbols::Symbol;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Тракт, реализуемый каждым поддерживаемым языком.
///
/// Методы принимают `&self` и хранят язык как данные (например, преднастроенный
/// `tree_sitter::Parser`), чтобы парсеры можно было переиспользовать без
/// пересоздания (производительность при сканировании проекта).
pub trait LanguageParser: Send + Sync {
    /// Имя языка для логов (`"rust"`, `"typescript"`…).
    fn name(&self) -> &'static str;

    /// Расширения файлов (без точки), которые обрабатывает этот парсер.
    fn extensions(&self) -> &'static [&'static str];

    /// Извлекает символы из исходного текста `source`.
    ///
    /// `file` используется только для записи пути в [`Symbol::file`]; чтение
    /// файла из ФС тут не происходит — вызывающий передаёт уже прочитанный текст.
    fn parse_symbols(&self, source: &str, file: PathBuf) -> Vec<Symbol>;
}

/// Результат разбора одного файла: путь + найденные символы.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// Путь к файлу.
    pub path: PathBuf,
    /// Найденные символы.
    pub symbols: Vec<Symbol>,
}

/// Возвращает парсер для файла по его расширению, если язык поддерживается.
///
/// Возвращает `Arc<dyn LanguageParser>` — один общий экземпляр на язык
/// (кешируется, чтобы не пересоздавать tree-sitter `Parser` на каждый файл).
pub fn parser_for_extension(ext: &str) -> Option<Arc<dyn LanguageParser>> {
    let ext = ext.trim_start_matches('.');
    // Реестр явно перечисляет языки. При добавлении нового языка — добавляем
    // строку здесь и модуль в `languages/`.
    let parsers: Vec<Arc<dyn LanguageParser>> = vec![
        Arc::new(languages::rust::RustParser::new()),
        Arc::new(languages::javascript::JavaScriptParser::new()),
        Arc::new(languages::typescript::TypeScriptParser::new()),
        Arc::new(languages::go::GoParser::new()),
    ];
    parsers.into_iter().find(|p| p.extensions().iter().any(|e| *e == ext))
}

/// Разбирает один файл с диска и возвращает [`ParsedFile`].
///
/// Если расширение не поддерживается — возвращает `None`. Ошибки чтения
/// пробрасываются наверх.
pub fn parse_file(path: &Path) -> std::io::Result<Option<ParsedFile>> {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e,
        None => return Ok(None),
    };
    let Some(parser) = parser_for_extension(ext) else {
        return Ok(None);
    };
    let source = std::fs::read_to_string(path)?;
    let symbols = parser.parse_symbols(&source, path.to_path_buf());
    Ok(Some(ParsedFile {
        path: path.to_path_buf(),
        symbols,
    }))
}

/// Разбирает строку исходника (уже прочитанный текст) по пути `file`
/// (используется только для расширения и записи в [`Symbol::file`]).
///
/// Удобен для `commit auto`, где нужно сравнивать версии из git с версиями с диска.
/// Возвращает `None`, если расширение не поддерживается.
pub fn parse_file_str(source: &str, file: &Path) -> Option<Vec<Symbol>> {
    let ext = file.extension().and_then(|e| e.to_str())?;
    let parser = parser_for_extension(ext)?;
    Some(parser.parse_symbols(source, file.to_path_buf()))
}

//! Поддержка `dm commit auto` — формирование человекочитаемого сообщения коммита
//! на основе того, **какие именно символы** (функции/классы/структуры) изменились.
//!
//! Поток данных:
//! 1. Берём `git diff` изменённых файлов (через [`changed_file_paths`]).
//! 2. Для каждого файла `dm-analysis` вычисляет список затронутых символов.
//! 3. Собираем структурированный текст коммита функцией [`build_auto_message`].

use crate::git::run_git;
use dm_core::error::DmResult;
use std::path::{Path, PathBuf};

/// Возвращает список изменённых файлов (относительно корня репозитория),
/// полученный из `git status --porcelain`.
pub async fn changed_file_paths(repo: &Path) -> DmResult<Vec<PathBuf>> {
    let out = run_git(repo, &["status", "--porcelain"], true).await?;
    let mut files = Vec::new();
    for line in out.stdout.lines() {
        // Формат: "XY path", где XY — два символа статуса.
        if line.len() < 4 {
            continue;
        }
        // Берём путь после статуса; игнорируем переименования (R) с `->`.
        let path_part = &line[3..];
        let path_part = path_part.split(" -> ").last().unwrap_or(path_part);
        let path_part = path_part.trim_matches('"');
        if !path_part.is_empty() {
            files.push(PathBuf::from(path_part));
        }
    }
    Ok(files)
}

/// Элемент «что изменилось» — описание одного затронутенного символа.
///
/// Заполняется анализатором (`dm-analysis`) для каждого файла из diff'а.
#[derive(Debug, Clone)]
pub struct ChangedSymbol {
    /// Имя символа (функции/класса/структуры).
    pub name: String,
    /// Тип символа для человекочитаемого вывода.
    pub kind: SymbolKind,
    /// Путь к файлу (относительно корня репо).
    pub file: PathBuf,
}

/// Категория символа кода.
#[derive(Debug, Clone, Copy)]
pub enum SymbolKind {
    /// Функция или метод.
    Function,
    /// Класс / type.
    Class,
    /// Структура (struct/record).
    Struct,
    /// Переменная/константа верхнего уровня.
    Variable,
    /// Прочее (макрос, trait, interface).
    Other,
}

impl SymbolKind {
    /// Русское название для текста коммита.
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

/// Формирует финальный текст сообщения коммита из списка изменённых символов.
///
/// Формат — краткий и читаемый:
/// ```text
/// auto: изменены 3 символа
///
/// - функция foo (api/src/handlers.rs)
/// - структура User (api/src/models.rs)
/// - класс AuthService (web/src/auth.ts)
/// ```
pub fn build_auto_message(symbols: &[ChangedSymbol]) -> String {
    if symbols.is_empty() {
        return "auto: изменения в коде".to_string();
    }
    let mut out = format!("auto: изменены {} символ(ов)\n\n", symbols.len());
    for s in symbols {
        out.push_str(&format!(
            "- {} {} ({})\n",
            s.kind.label(),
            s.name,
            s.file.display()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_message_from_symbols() {
        let symbols = vec![
            ChangedSymbol {
                name: "foo".into(),
                kind: SymbolKind::Function,
                file: PathBuf::from("api/src/lib.rs"),
            },
            ChangedSymbol {
                name: "User".into(),
                kind: SymbolKind::Struct,
                file: PathBuf::from("api/models.rs"),
            },
        ];
        let msg = build_auto_message(&symbols);
        assert!(msg.contains("функция foo"));
        assert!(msg.contains("структура User"));
        assert!(msg.contains("api/src/lib.rs"));
    }

    #[test]
    fn empty_symbols_gives_generic_message() {
        assert_eq!(build_auto_message(&[]), "auto: изменения в коде");
    }
}

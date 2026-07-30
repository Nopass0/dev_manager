//! Поиск использований (references) символа по проекту.
//!
//! Находит символ по имени в исходниках, затем ищет все textual references на
//! это имя во всех файлах. Используется командой `dm refs <symbol>`.

use crate::search::{Match, SearchOptions, search};
use std::path::Path;

/// Находит все использования имени `symbol` в файлах под `root`.
///
/// Возвращает совпадения как [`Match`] с word-boundary фильтром, чтобы не ловить
/// подстроки внутри других идентификаторов (например, `User` внутри `UserRepo`).
pub fn find_references(root: &Path, symbol: &str) -> Vec<Match> {
    let opts = SearchOptions {
        whole_word: true,
        ..Default::default()
    };
    search(root, symbol, &opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_whole_word_only() {
        let dir = std::env::temp_dir().join("dm_refs_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.rs"),
            "struct User {}\nlet u = User {};\nstruct UserRepo {}\n",
        )
        .unwrap();
        let refs = find_references(&dir, "User");
        // Должно найти только "User" (определение + использование), но не "UserRepo".
        assert_eq!(refs.len(), 2);
        assert!(!refs.iter().any(|m| m.text.contains("UserRepo")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

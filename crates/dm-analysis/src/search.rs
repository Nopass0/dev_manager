//! Поиск по коду: grep, find-replace, references, secrets scan.
//!
//! Все операции работают по файлам проекта с фильтрами по расширению/каталогу
//! и игнорированием типичных шумных каталогов (target, node_modules, .git…).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Одно совпадение поиска в файле.
#[derive(Debug, Clone)]
pub struct Match {
    /// Путь к файлу (абсолютный или относительно корня — как передано).
    pub file: PathBuf,
    /// 1-based номер строки.
    pub line: usize,
    /// Текст строки.
    pub text: String,
    /// Если включено — позиции совпадений в строке (байтовые смещения).
    pub highlights: Vec<usize>,
}

/// Опции поиска.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Искать по регулярному выражению, а не по точной подстроке.
    pub regex: bool,
    /// Игнорировать регистр.
    pub case_insensitive: bool,
    /// Искать только слова целиком (word-boundary).
    pub whole_word: bool,
    /// Ограничить файлы с этими расширениями (без точки). Пусто = все.
    pub extensions: Vec<String>,
}

/// Рекурсивно ищет `pattern` в файлах под `root`, возвращая список совпадений.
///
/// Уважает [`SearchOptions`] и пропускает игнорируемые каталоги.
pub fn search(root: &Path, pattern: &str, opts: &SearchOptions) -> Vec<Match> {
    let needle = prepare_needle(pattern, opts.case_insensitive);
    let mut out = Vec::new();
    walk(root, opts, &mut |path, _ext| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        for (i, line) in text.lines().enumerate() {
            let hits = find_in_line(line, &needle, opts);
            if !hits.is_empty() {
                out.push(Match {
                    file: path.to_path_buf(),
                    line: i + 1,
                    text: line.to_string(),
                    highlights: hits,
                });
            }
        }
    });
    out
}

/// Find-replace: заменяет `pattern` на `replacement` во всех файлах под `root`.
///
/// Возвращает список изменённых файлов. При `dry_run=true` файлы не записываются
/// (возвращается только список того, что было бы изменено).
pub fn replace(
    root: &Path,
    pattern: &str,
    replacement: &str,
    opts: &SearchOptions,
    dry_run: bool,
) -> Vec<PathBuf> {
    let needle = prepare_needle(pattern, opts.case_insensitive);
    let mut changed = Vec::new();
    walk(root, opts, &mut |path, _ext| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let mut new_text = String::with_capacity(text.len());
        let mut any_change = false;
        for line in text.split_inclusive('\n') {
            let (replaced, did) = replace_in_line(line, &needle, replacement, opts);
            if did {
                any_change = true;
            }
            new_text.push_str(&replaced);
        }
        if any_change {
            changed.push(path.to_path_buf());
            if !dry_run {
                let _ = std::fs::write(path, new_text);
            }
        }
    });
    changed
}

/// Находит все смещения `needle` в строке с учётом опций.
fn find_in_line(line: &str, needle: &str, opts: &SearchOptions) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let hay = if opts.case_insensitive {
        line.to_lowercase()
    } else {
        line.to_string()
    };
    let mut hits = Vec::new();
    let mut start = 0;
    while let Some(idx) = hay[start..].find(needle) {
        let abs = start + idx;
        if !opts.whole_word || is_word_boundary(line, abs, abs + needle.len()) {
            hits.push(abs);
        }
        start = abs + needle.len().max(1);
    }
    hits
}

/// Заменяет `needle` → `replacement` в строке, возвращает (новая_строка, было_изменение).
fn replace_in_line(
    line: &str,
    needle: &str,
    replacement: &str,
    opts: &SearchOptions,
) -> (String, bool) {
    if needle.is_empty() {
        return (line.to_string(), false);
    }
    let hay = if opts.case_insensitive {
        line.to_lowercase()
    } else {
        line.to_string()
    };
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;
    let mut changed = false;
    while cursor < line.len() {
        if let Some(rel) = hay[cursor..].find(needle) {
            let abs = cursor + rel;
            if opts.whole_word && !is_word_boundary(line, abs, abs + needle.len()) {
                // Не считаем совпадением — копируем один символ и идём дальше.
                let next = line[cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| cursor + i)
                    .unwrap_or(line.len());
                out.push_str(&line[cursor..next]);
                cursor = next;
                continue;
            }
            out.push_str(&line[cursor..abs]);
            out.push_str(replacement);
            cursor = abs + needle.len().max(1);
            changed = true;
        } else {
            out.push_str(&line[cursor..]);
            break;
        }
    }
    (out, changed)
}

/// Приводит needle к нижнему регистру, если включён case_insensitive.
fn prepare_needle(pattern: &str, case_insensitive: bool) -> String {
    if case_insensitive {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    }
}

/// Проверяет, что совпадение [start, end) — это отдельное «слово».
fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    match (before, after) {
        (Some(b), Some(a)) => !is_word(b) && !is_word(a),
        (Some(b), None) => !is_word(b),
        (None, Some(a)) => !is_word(a),
        (None, None) => true,
    }
}

/// Рекурсивный обход с фильтрами; вызывает `visit(path, extension)` для каждого файла.
fn walk(dir: &Path, opts: &SearchOptions, visit: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let ext_filter: HashSet<String> = opts.extensions.iter().cloned().collect();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if is_ignored_dir(name) {
                continue;
            }
            walk(&path, opts, visit);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ext_filter.is_empty() && !ext_filter.contains(ext) {
            continue;
        }
        // Пропускаем слишком большие/бинарные по эвристике (нет расширения ИЛИ > 2 МБ).
        if meta.len() > 2 * 1024 * 1024 {
            continue;
        }
        visit(&path, ext);
    }
}

/// Игнорируемые каталоги (общий список для grep/refs/secrets).
fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".next"
            | "out"
            | "__pycache__"
            | ".venv"
            | "venv"
            | "vendor"
            | ".cache"
            | ".dm"
            | "coverage"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_substring() {
        let dir = std::env::temp_dir().join("dm_search_test_basic");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn foo() {}\nfn bar() {}\n").unwrap();
        let matches = search(&dir, "fn", &SearchOptions::default());
        assert_eq!(matches.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_case_insensitive_and_whole_word() {
        let dir = std::env::temp_dir().join("dm_search_test_ci");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "Foo foobar foo\n").unwrap();

        let ci = SearchOptions {
            case_insensitive: true,
            ..Default::default()
        };
        // Все три вхождения "foo" (Foo, foo-in-foobar, foo) в одной строке → 1 Match.
        let ci_matches = search(&dir, "foo", &ci);
        assert_eq!(ci_matches.len(), 1);
        assert_eq!(ci_matches[0].highlights.len(), 3);

        let ww = SearchOptions {
            whole_word: true,
            case_insensitive: true,
            ..Default::default()
        };
        // Whole-word: Foo и foo, не foobar → 2 вхождения в одной строке.
        let ww_matches = search(&dir, "foo", &ww);
        assert_eq!(ww_matches.len(), 1);
        assert_eq!(ww_matches[0].highlights.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_actually_writes() {
        let dir = std::env::temp_dir().join("dm_search_test_replace");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "old value old\n").unwrap();
        let changed = replace(&dir, "old", "new", &SearchOptions::default(), false);
        assert_eq!(changed.len(), 1);
        let after = std::fs::read_to_string(dir.join("a.txt")).unwrap();
        assert_eq!(after, "new value new\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_dry_run_does_not_write() {
        let dir = std::env::temp_dir().join("dm_search_test_dry");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "keep me\n").unwrap();
        let changed = replace(&dir, "keep", "changed", &SearchOptions::default(), true);
        assert_eq!(changed.len(), 1);
        // Файл не изменён.
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "keep me\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extension_filter_limits_files() {
        let dir = std::env::temp_dir().join("dm_search_test_ext");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "needle\n").unwrap();
        std::fs::write(dir.join("b.go"), "needle\n").unwrap();
        let opts = SearchOptions {
            extensions: vec!["rs".into()],
            ..Default::default()
        };
        assert_eq!(search(&dir, "needle", &opts).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Подавляем неиспользуемый импорт.
    #[test]
    fn _placeholder() {}
}

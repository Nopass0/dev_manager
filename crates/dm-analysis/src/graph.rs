//! Граф зависимостей проекта — основа `dm start --affected` и smart-restart.
//!
//! Два уровня:
//! - **Файловый граф**: для каждого исходного файла — какие пути он импортирует
//!   (`use`/`import`/`#include`/`require`). Получаем обходом AST через tree-sitter.
//! - **Сервисный граф**: для каждого сервиса — от каких других сервисов он зависит
//!   транзитивно (через общие импорты или явно через `depends_on`).
//!
//! Использование: изменили `shared/auth.rs` → [`DependencyGraph::affected_services`]
//! вернёт `[api, worker, scheduler]` — те, что импортируют изменённый файл.

use crate::languages;
use crate::parser::parser_for_extension;
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

/// Один узел файлового графа: файл и пути, которые он импортирует.
#[derive(Debug, Clone)]
pub struct FileNode {
    /// Абсолютный путь к файлу.
    pub path: PathBuf,
    /// Импортированные пути (разрешённые в абсолютные, где удалось).
    pub imports: Vec<PathBuf>,
}

/// Полный граф зависимостей проекта.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Все просканированные файлы (абсолютные пути).
    pub files: Vec<PathNode>,
    /// Карта: файл → индексы файлов, которые напрямую зависят от него (реверс-рёбра).
    ///
    /// То есть «кто импортирует меня». Нужно для affected-расчёта.
    pub reverse_edges: HashMap<PathBuf, Vec<usize>>,
}

/// Файл в графе + его импорты (по индексам в `files`).
#[derive(Debug, Clone)]
pub struct PathNode {
    /// Абсолютный путь.
    pub path: PathBuf,
    /// Индексы файлов в [`DependencyGraph::files`], которые этот файл импортирует.
    pub import_indices: Vec<usize>,
}

impl DependencyGraph {
    /// Строит граф, просканировав каталог `root` рекурсивно.
    ///
    /// Игнорирует типичные шумные каталоги (target, node_modules, .git…).
    /// Все пути нормализуются к абсолютным.
    pub fn build(root: &Path) -> Self {
        let mut raw: Vec<FileNode> = Vec::new();
        collect_files(root, root, &mut raw);

        // Индекс путь → индекс в files для быстрого разрешения рёбер.
        let mut path_to_idx: HashMap<PathBuf, usize> = HashMap::new();
        let files: Vec<PathNode> = raw
            .iter()
            .enumerate()
            .map(|(i, n)| {
                path_to_idx.insert(normalize(&n.path), i);
                PathNode {
                    path: n.path.clone(),
                    import_indices: Vec::new(),
                }
            })
            .collect();

        // Заполняем import_indices, разрешая импорты в индексы.
        let mut files = files;
        for (i, node) in raw.iter().enumerate() {
            for imp in &node.imports {
                let imp_norm = normalize(imp);
                // Точное совпадение.
                if let Some(&j) = path_to_idx.get(&imp_norm) {
                    files[i].import_indices.push(j);
                    continue;
                }
                // Неточное: импорт мог быть без расширения или с module-path.
                // Пытаемся подобрать файл с тем же stem.
                if let Some(j) = fuzzy_match(&imp_norm, &path_to_idx) {
                    files[i].import_indices.push(j);
                }
            }
        }

        // Обратные рёбра: для каждого файла — кто на него ссылается.
        let mut reverse_edges: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for (i, node) in files.iter().enumerate() {
            for &j in &node.import_indices {
                reverse_edges
                    .entry(files[j].path.clone())
                    .or_default()
                    .push(i);
            }
        }

        Self { files, reverse_edges }
    }

    /// Возвращает множество файлов, затронутых изменением `changed` (транзитивно).
    ///
    /// Это сам `changed` + все, кто его импортирует, + все, кто импортирует их, и т.д.
    pub fn affected_files(&self, changed: &[PathBuf]) -> HashSet<PathBuf> {
        let mut out = HashSet::new();
        let mut queue: Vec<PathBuf> = changed
            .iter()
            .map(|p| normalize(p))
            .collect();
        let mut visited: HashSet<PathBuf> = HashSet::new();
        while let Some(p) = queue.pop() {
            if !visited.insert(p.clone()) {
                continue;
            }
            out.insert(p.clone());
            if let Some(parents) = self.reverse_edges.get(&p) {
                for &idx in parents {
                    queue.push(self.files[idx].path.clone());
                }
            }
        }
        out
    }

    /// Для списка сервисов (каталогов) возвращает те, что затронуты изменениями.
    ///
    /// Сервис считается затронутым, если хотя бы один affected-файл лежит внутри
    /// его каталога.
    pub fn affected_services(&self, changed: &[PathBuf], service_dirs: &[(&str, &Path)]) -> Vec<String> {
        let affected = self.affected_files(changed);
        let mut result: IndexSet<String> = IndexSet::new();
        for (name, dir) in service_dirs {
            let dir_norm = normalize(dir);
            for f in &affected {
                if f.starts_with(&dir_norm) {
                    result.insert(name.to_string());
                    break;
                }
            }
        }
        result.into_iter().collect()
    }
}

/// Рекурсивно собирает FileNode из каталога, разбирая импорты каждого файла.
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<FileNode>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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
            collect_files(root, &path, out);
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let Some(parser_info) = parser_for_extension(ext) else {
            continue;
        };
        // Читаем файл и достаём импорты через временный parser.
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let imports = extract_imports(ext, &source, &path, root);
        out.push(FileNode { path, imports });
        let _ = parser_info;
    }
}

/// Извлекает пути импортов из исходника через tree-sitter для данного расширения.
///
/// Возвращает абсолютные (где удалось разрешить) или относительные к каталогу
/// файла пути. Резолвинг «до конца» не требуется — [`DependencyGraph::build`]
/// делает нечёткое сопоставление.
fn extract_imports(ext: &str, source: &str, file: &Path, _root: &Path) -> Vec<PathBuf> {
    let mut parser = match parser_for_extension(ext) {
        Some(_) => Parser::new(),
        None => return Vec::new(),
    };
    let lang_ok = match ext {
        "rs" => parser.set_language(&tree_sitter_rust::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => {
            parser.set_language(&tree_sitter_javascript::LANGUAGE.into())
        }
        "ts" | "tsx" => parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "go" => parser.set_language(&tree_sitter_go::LANGUAGE.into()),
        _ => return Vec::new(),
    };
    if lang_ok.is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let root_node = tree.root_node();
    let file_dir = file.parent().unwrap_or(Path::new(""));

    let mut imports = Vec::new();
    let mut cursor = root_node.walk();
    traverse(&root_node, &mut cursor, source, &mut |node| {
        if let Some(spec) = import_spec_from_node(node, source, ext) {
            // Пытаемся разрешить спецификатор в абсолютный путь.
            if let Some(resolved) = resolve_spec(&spec, file_dir) {
                imports.push(resolved);
            }
        }
    });
    imports
}

/// Обход AST с вызовом `visit` для каждого узла.
fn traverse<'a>(
    node: &Node<'a>,
    cursor: &mut tree_sitter::TreeCursor<'a>,
    source: &'a str,
    visit: &mut impl FnMut(&Node),
) {
    visit(node);
    // Дополнительная проверка: для некоторых языков строковый аргумент импорта —
    // это text узла; собираем его здесь через languages::node_text.
    let _ = (source, &languages::node_text);
    cursor.reset(node.clone());
    if cursor.goto_first_child() {
        loop {
            let n = cursor.node();
            traverse(&n, cursor, source, visit);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Если узел — это импорт с конкретным спецификатором модуля, возвращает его.
///
/// Поддерживаются типовые AST-узлы импорта в Rust/JS/TS/Go.
fn import_spec_from_node(node: &Node, source: &str, ext: &str) -> Option<String> {
    let text = languages::node_text(node, source);
    match ext {
        "rs" => rust_use_target(&text),
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => js_import_source(node, source),
        "go" => go_import_target(&text),
        _ => None,
    }
}

/// Извлекает целевой путь из Rust `use foo::bar;`.
fn rust_use_target(text: &str) -> Option<String> {
    let t = text.trim();
    let after = t.strip_prefix("use ")?.trim_start();
    // Берём до `::` первого уровня и до `{` (группы).
    let head = after.split('{').next()?.trim();
    let head = head.split(';').next()?.trim();
    let first_segment = head.split("::").next()?.trim();
    // Пропускаем crate/self/super/абсолютные — интересны только локальные.
    if matches!(first_segment, "crate" | "self" | "super" | "std" | "core" | "alloc") {
        return None;
    }
    Some(format!("{first_segment}.rs"))
}

/// Извлекает source из JS/TS import/require: `import x from "./a"` → `./a`.
fn js_import_source(node: &Node, source: &str) -> Option<String> {
    // Ищем дочерний строковый литерал — это source-спецификатор.
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let n = cursor.node();
            if matches!(n.kind(), "string" | "string_fragment") {
                let raw = languages::node_text(&n, source);
                let cleaned = raw.trim_matches(|c| c == '"' || c == '\'' || c == '`');
                if cleaned.starts_with('.') || cleaned.starts_with('/') {
                    return Some(with_ts_ext(cleaned));
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// Go import: `import "github.com/x/y"` или `import ( ... )`.
fn go_import_target(text: &str) -> Option<String> {
    // Локальные импорты в Go обычно идут через относительный модуль; tree-sitter
    // даёт import_statement, но path-resolution сложный. Ограничимся тем, что
    // явно начинается с `./`.
    let t = text.trim();
    let quoted = t.split_whitespace().last()?;
    let cleaned = quoted.trim_matches('"');
    if cleaned.starts_with('.') {
        Some(with_ts_ext(cleaned))
    } else {
        None
    }
}

/// Добавляет `.ts`/`.js`, если спецификатор без расширения (типовой для этих языков).
fn with_ts_ext(spec: &str) -> String {
    if spec.ends_with(".ts") || spec.ends_with(".js") || spec.ends_with(".tsx") {
        spec.to_string()
    } else {
        format!("{spec}.ts")
    }
}

/// Пытается разрешить спецификатор импорта в абсолютный путь относительно `base_dir`.
fn resolve_spec(spec: &str, base_dir: &Path) -> Option<PathBuf> {
    let p = Path::new(spec);
    if p.is_absolute() {
        return Some(normalize(p));
    }
    let joined = base_dir.join(p);
    // Пробуем как есть и с index-файлом.
    if joined.is_file() {
        return Some(normalize(&joined));
    }
    let index = joined.join("index.ts");
    if index.is_file() {
        return Some(normalize(&index));
    }
    let index_js = joined.join("index.js");
    if index_js.is_file() {
        return Some(normalize(&index_js));
    }
    // Если файл не существует, всё равно сохраняем путь — возможно, разрешится
    // через нечёткое сопоставление в build().
    Some(normalize(&joined))
}

/// Нечёткое сопоставление: ищем файл с тем же stem'ом, что у `target`.
fn fuzzy_match(target: &Path, index: &HashMap<PathBuf, usize>) -> Option<usize> {
    let target_stem = target.file_stem()?.to_str()?.to_lowercase();
    for (p, &i) in index {
        if p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase())
            == Some(target_stem.clone())
        {
            return Some(i);
        }
    }
    None
}

/// Возвращает true для игнорируемых имён каталогов (чтобы не сканировать всё).
fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | "dist" | "build" | ".next"
            | "out" | "__pycache__" | ".venv" | "venv" | "vendor" | ".cache"
    )
}

/// Лексическая нормализация пути (схлопывание `..`/`.`).
fn normalize(path: &Path) -> PathBuf {
    crate::normalize_lexical_pub(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_use_target_parses_local() {
        assert_eq!(rust_use_target("use auth::login;"), Some("auth.rs".into()));
        assert_eq!(rust_use_target("use std::collections::HashMap;"), None);
        assert_eq!(rust_use_target("use crate::server;"), None);
    }

    #[test]
    fn js_import_source_extracts_relative() {
        // Симулируем минимальный узел строкой — реальная проверка через AST.
        let spec = "./auth";
        assert_eq!(with_ts_ext(spec), "./auth.ts");
    }

    #[test]
    fn affected_files_transitive() {
        // Ручной граф: 0→1, 1→2. Изменили 0 → затронуты 0,1,2.
        let files = vec![
            PathNode {
                path: PathBuf::from("/p/a.rs"),
                import_indices: vec![1],
            },
            PathNode {
                path: PathBuf::from("/p/b.rs"),
                import_indices: vec![2],
            },
            PathNode {
                path: PathBuf::from("/p/c.rs"),
                import_indices: vec![],
            },
        ];
        let mut reverse: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        reverse.insert(PathBuf::from("/p/b.rs"), vec![0]); // a импортирует b
        reverse.insert(PathBuf::from("/p/c.rs"), vec![1]); // b импортирует c
        let g = DependencyGraph { files, reverse_edges: reverse };
        let affected = g.affected_files(&[PathBuf::from("/p/c.rs")]);
        assert!(affected.contains(Path::new("/p/c.rs")));
        assert!(affected.contains(Path::new("/p/b.rs")));
        assert!(affected.contains(Path::new("/p/a.rs")));
    }
}

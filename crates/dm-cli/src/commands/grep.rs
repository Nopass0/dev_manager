//! `dm grep <pattern>` — поиск по коду проекта с фильтрами.
//!
//! Поддерживает `-i` (ignore case), `-w` (whole word), `-t ext1,ext2` (фильтр
//! по расширениям). Вывод в стиле ripgrep: путь к файлу, затем строки с номерами.

use crate::commands::load_project_config;
use crate::commands::GrepArgs;
use crate::output::{dim_style, print_system};
use dm_analysis::search::{search as run_search, SearchOptions};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(args: GrepArgs) -> DmResult<()> {
    let (_config, root) = load_project_config()?;
    let opts = SearchOptions {
        case_insensitive: args.ignore_case,
        whole_word: args.word,
        extensions: args.r#type.clone(),
        ..Default::default()
    };
    let matches = run_search(&root, &args.pattern, &opts);
    if matches.is_empty() {
        print_system("совпадений не найдено.");
        return Ok(());
    }

    // Группируем по файлу, печатаем «как ripgrep»: имя файла, затем N: строка.
    let mut current: Option<&std::path::Path> = None;
    let header = format!(
        "{}",
        paint_dim(&root.to_string_lossy())
    );
    let _ = header;

    for m in &matches {
        if current.map(|p| p != m.file.as_path()).unwrap_or(true) {
            println!("{}", m.file.display());
            current = Some(m.file.as_path());
        }
        println!("{}:{}", m.line, m.text.trim_end());
    }
    let _ = dim_style;
    Ok(())
}

/// Вспомогательная функция для подавления неиспользуемого импорта.
fn paint_dim(_s: &str) -> String {
    String::new()
}

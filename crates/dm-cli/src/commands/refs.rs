//! `dm refs <symbol>` — найти все использования символа в проекте.

use crate::commands::load_project_config;
use crate::output::print_system;
use dm_analysis::refs::find_references;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(symbol: &str) -> DmResult<()> {
    let (_config, root) = load_project_config()?;
    let refs = find_references(&root, symbol);
    if refs.is_empty() {
        print_system(&format!("использования '{symbol}' не найдены."));
        return Ok(());
    }
    let mut current: Option<&std::path::Path> = None;
    println!("найдено ссылок: {}", refs.len());
    for m in &refs {
        if current.map(|p| p != m.file.as_path()).unwrap_or(true) {
            println!("{}", m.file.display());
            current = Some(m.file.as_path());
        }
        println!("{}:{}", m.line, m.text.trim_end());
    }
    Ok(())
}

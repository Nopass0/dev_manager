//! `dm secrets` — поиск потенциально утёкших секретов в проекте.

use crate::commands::load_project_config;
use crate::output::{error_style, print_system, success_style, warn_style, println_styled};
use comfy_table::{ContentArrangement, Table};
use dm_analysis::secrets::scan;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (_config, root) = load_project_config()?;
    print_system("сканирование на потенциальные секреты…");
    let findings = scan(&root);
    if findings.is_empty() {
        println_styled("подозрительных строк не найдено ✨", success_style());
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Категория", "Файл", "Строка", "Содержимое"]);
    for f in &findings {
        table.add_row(vec![
            f.kind.label().to_string(),
            f.file.display().to_string(),
            f.line.to_string(),
            mask_secret(&f.text),
        ]);
    }
    println!("{table}");
    println_styled(
        &format!("всего предупреждений: {} — проверьте вручную", findings.len()),
        warn_style(),
    );
    let _ = error_style;
    Ok(())
}

/// Маскирует середину строки-кандидата, чтобы не выводить секрет целиком.
fn mask_secret(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= 20 {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(12).collect();
    let tail: String = trimmed.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}

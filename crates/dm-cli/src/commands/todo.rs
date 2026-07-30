//! `dm todo` — реестр TODO/FIXME/HACK/XXX по коду проекта.
//!
//! Собирает все маркеры из исходников и показывает таблицей: файл, строка,
//! тип маркера и текст. Ускоряет работу: все «долги» кода в одном месте,
//! видно масштаб технического долга.

use crate::commands::load_project_config;
use crate::output::{dim_style, print_system, println_styled};
use comfy_table::{ContentArrangement, Table};
use dm_analysis::search::{search, SearchOptions};
use dm_core::DmResult;

/// Маркеры, которые ищем.
const MARKERS: &[&str] = &["TODO", "FIXME", "HACK", "XXX", "BUG"];

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (_config, root) = load_project_config()?;
    print_system("поиск TODO/FIXME/HACK маркеров…");

    let opts = SearchOptions {
        whole_word: true,
        ..Default::default()
    };
    let mut rows: Vec<(String, std::path::PathBuf, usize, String)> = Vec::new();
    for marker in MARKERS {
        for m in search(&root, marker, &opts) {
            let kind = marker.to_string();
            let text = extract_marker_text(&m.text, marker);
            rows.push((kind, m.file.clone(), m.line, text));
        }
    }

    if rows.is_empty() {
        println_styled("TODO/FIXME маркеры не найдены ✨", crate::output::success_style());
        return Ok(());
    }

    rows.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Тип", "Файл", "Строка", "Текст"]);
    for (kind, file, line, text) in &rows {
        table.add_row(vec![
            kind.clone(),
            short(file, &root),
            line.to_string(),
            text.clone(),
        ]);
    }
    println!("{table}");

    // Сводка по типам.
    let mut by_kind: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (k, _, _, _) in &rows {
        *by_kind.entry(k.as_str()).or_default() += 1;
    }
    let summary: Vec<String> = by_kind.iter().map(|(k, n)| format!("{k}: {n}")).collect();
    println_styled(&format!("всего {}: {}", rows.len(), summary.join(", ")), dim_style());
    Ok(())
}

/// Извлекает текст после маркера из строки.
fn extract_marker_text(line: &str, marker: &str) -> String {
    let upper = line.to_uppercase();
    let idx = match upper.find(marker) {
        Some(i) => i + marker.len(),
        None => return line.trim().to_string(),
    };
    line[idx..].trim_start_matches([':', ' ', '\t']).trim().to_string()
}

/// Сокращает путь файла относительно корня проекта.
fn short(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

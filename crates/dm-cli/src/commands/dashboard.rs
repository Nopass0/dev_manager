//! `dm dashboard` — live-дашборд всех сервисов с периодическим refresh.
//!
//! Перерисовывает таблицу статусов каждые N секунд, пока не прервут (Ctrl+C).

use crate::commands::load_project_config;
use crate::output::print_system;
use comfy_table::{ContentArrangement, Table};
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use std::time::Duration;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, _root) = load_project_config()?;
    print_system(&format!(
        "дашборд проекта '{}' (Ctrl+C — выход, refresh каждые 3с)",
        config.project_name
    ));
    loop {
        // Очистка экрана (ANSI) — работает на большинстве терминалов.
        print!("\x1b[2J\x1b[H");
        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Сервис", "Язык", "Порт", "Статус"]);
        for (name, svc) in &config.services {
            let port = default_port(svc.language);
            let busy = tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok();
            let status = if busy {
                "🟢 слушает"
            } else {
                "⚫️ свободен"
            };
            table.add_row(vec![
                name.to_string(),
                svc.language.label().to_string(),
                port.to_string(),
                status.to_string(),
            ]);
        }
        println!("{table}");
        println!("\n(обновление каждые 3 с, Ctrl+C — выход)");
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

/// Стандартный dev-порт по языку.
fn default_port(lang: ServiceLanguage) -> u16 {
    use ServiceLanguage::*;
    match lang {
        Vite => 5173,
        Nextjs | Remix => 3000,
        Nodejs | JavaScript | TypeScript => 3000,
        Rust => 8080,
        _ => 3000,
    }
}

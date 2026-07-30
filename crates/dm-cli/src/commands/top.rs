//! `dm top` — сводная таблица сервисов и их сетевого состояния (htop-подобная).
//!
//! В текущей версии (без запущенного supervisor в этом же процессе) показывает
//! для каждого сервиса: язык, стандартный dev-порт и слушает ли он сейчас.
//! Полноценные метрики CPU/RAM требуют интеграции с supervisor'ом (см. roadmap).

use crate::commands::load_project_config;
use crate::output::{dim_style, print_system, println_styled};
use comfy_table::{ContentArrangement, Table};
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, _root) = load_project_config()?;
    print_system(&format!(
        "мониторинг сервисов проекта '{}'",
        config.project_name
    ));

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Сервис", "Язык", "Порт", "Статус"]);

    for (name, svc) in &config.services {
        let port = default_port_for_language(svc.language);
        let busy = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok();
        let status = if busy {
            "🟢 слушает"
        } else {
            "⚫️ не запущен"
        };
        table.add_row(vec![
            name.to_string(),
            svc.language.label().to_string(),
            port.to_string(),
            status.to_string(),
        ]);
    }
    println!("{table}");
    println_styled(
        "Метрики CPU/RAM в реальном времени — в roadmap; запустите `dm start` для live-режима.",
        dim_style(),
    );
    Ok(())
}

/// Стандартный dev-порт по языку/фреймворку.
fn default_port_for_language(lang: ServiceLanguage) -> u16 {
    use ServiceLanguage::*;
    match lang {
        Vite => 5173,
        Nextjs | Remix => 3000,
        Nodejs | JavaScript | TypeScript => 3000,
        Rust => 8080,
        _ => 3000,
    }
}

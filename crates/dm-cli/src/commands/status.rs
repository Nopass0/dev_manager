//! `dm status` — сводка по проекту: конфиг, сервисы, git-состояние.

use crate::commands::load_project_config;
use crate::output::{build_status_table, print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_runtime::supervisor::project_from_config;
use dm_runtime::logs::ServiceStatus;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let project = project_from_config(&config, &root)?;

    print_system(&format!("проект: {}", project.name));
    print_system(&format!("корень:  {}", root.display()));
    println_styled(&format!("сервисов: {}", project.services.len()), success_style());

    // Таблица: имя, язык, команда, «статус» (для офлайн-команды — pending).
    let rows: Vec<(String, ServiceStatus)> = project
        .services
        .iter()
        .map(|s| (s.name.clone(), ServiceStatus::Pending))
        .collect();
    let table = build_status_table(&rows);
    println!("{table}");

    println_styled(
        "Подсказка: статусы сервисов обновляются в реальном времени во время `dm start`.",
        crate::output::dim_style(),
    );
    Ok(())
}

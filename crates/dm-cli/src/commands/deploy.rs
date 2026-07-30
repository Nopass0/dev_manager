//! `dm deploy <name>` — запуск деплоя по имени цели.

use crate::commands::load_project_config;
use crate::output::{print_system, success_style, error_style, println_styled};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(name: &str) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    print_system(&format!("деплой цели '{name}'…"));
    match dm_deploy::run_deploy(&config, name, &root).await {
        Ok(report) => {
            if report.success {
                println_styled(&format!("✓ деплой '{}' выполнен", report.target_name), success_style());
            } else {
                println_styled(&format!("✗ деплой '{}' завершён с ошибками", report.target_name), error_style());
            }
            for (cmd, log) in &report.step_logs {
                println!("  • {cmd} → {log}");
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

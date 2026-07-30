//! `dm env sync` — распределение единого `.env` по сервисам.

use crate::commands::{EnvAction, load_project_config};
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use dm_core::env::{parse_unified_env, write_service_env};
use dm_core::paths;
use std::path::Path;

/// Точка входа команды.
pub async fn run(action: EnvAction) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    match action {
        EnvAction::Sync => {
            let env_path = paths::resolve(&root, Path::new(&config.env_file));
            print_system(&format!("чтение единого .env: {}", env_path.display()));
            let content = match std::fs::read_to_string(&env_path) {
                Ok(c) => c,
                Err(_) => {
                    println_styled(
                        &format!(
                            "файл {} не найден — нечего синхронизировать",
                            env_path.display()
                        ),
                        warn_style(),
                    );
                    return Ok(());
                }
            };
            let unified = parse_unified_env(&content)?;
            let mut written = 0usize;
            for (name, svc) in &config.services {
                let vars = unified.vars_for(name);
                if vars.is_empty() {
                    continue;
                }
                let svc_dir = paths::resolve(&root, Path::new(&svc.path));
                let target = svc_dir.join(".env");
                write_service_env(&target, &vars)?;
                println_styled(
                    &format!(
                        "  ✓ {name}: записано {} переменных в {}",
                        vars.len(),
                        target.display()
                    ),
                    success_style(),
                );
                written += 1;
            }
            print_system(&format!(
                "синхронизация завершена, обновлено сервисов: {written}"
            ));
            Ok(())
        }
    }
}

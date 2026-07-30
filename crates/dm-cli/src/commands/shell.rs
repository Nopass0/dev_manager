//! `dm shell <svc>` — открыть интерактивную shell-сессию в каталоге сервиса.
//!
//! Запускает shell пользователя (cmd на Windows, $SHELL на Unix) с cwd в
//! каталоге сервиса и подставляет переменные из `.env` сервиса.

use crate::commands::load_project_config;
use crate::output::print_system;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(name: &str) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let svc = config
        .services
        .get(name)
        .ok_or_else(|| dm_core::DmError::ServiceNotFound(name.to_string()))?;
    let cwd = dm_core::paths::resolve(&root, std::path::Path::new(&svc.path));
    print_system(&format!("shell в '{name}/' ({})", cwd.display()));

    #[cfg(windows)]
    let mut cmd = std::process::Command::new("cmd");
    #[cfg(unix)]
    let mut cmd = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        std::process::Command::new(shell)
    };

    cmd.current_dir(&cwd);

    // Подставляем переменные из .env сервиса.
    if let Ok(env_text) = std::fs::read_to_string(cwd.join(".env")) {
        for line in env_text.lines() {
            if let Some((k, v)) = line.split_once('=')
                && !k.starts_with('#')
            {
                cmd.env(k.trim(), v.trim());
            }
        }
    }

    let status = cmd
        .status()
        .map_err(|e| dm_core::DmError::Process(format!("не удалось запустить shell: {e}")))?;
    if !status.success() {
        return Err(dm_core::DmError::Process(format!(
            "shell завершился с кодом {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

//! `dm exec <svc> -- <cmd>` — выполнить команду в контексте сервиса.
//!
//! Запускает `<cmd>` с `cwd = <svc>.path`, наследуя окружение и подставляя
//! переменные из `.env` сервиса (если он синхронизирован). Удобно запускать
//! psql/redis-cli/migrations без ручного `cd` и `export`.

use crate::commands::{ExecArgs, load_project_config};
use crate::output::print_system;
use dm_core::DmResult;
use std::process::Command;

/// Точка входа команды.
pub async fn run(args: ExecArgs) -> DmResult<()> {
    if args.command.is_empty() {
        return Err(dm_core::DmError::invalid_config(
            "укажите команду: dm exec <svc> -- <cmd> <args...>",
        ));
    }
    let (config, root) = load_project_config()?;
    let svc = config
        .services
        .get(&args.service)
        .ok_or_else(|| dm_core::DmError::ServiceNotFound(args.service.clone()))?;

    let cwd = dm_core::paths::resolve(&root, std::path::Path::new(&svc.path));
    print_system(&format!(
        "exec в '{}/': {}",
        args.service,
        args.command.join(" ")
    ));

    let mut cmd = build_command(&args.command);
    cmd.current_dir(&cwd);
    // Загружаем .env сервиса, если он существует (результат dm env sync).
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
        .map_err(|e| dm_core::DmError::Process(format!("не удалось запустить команду: {e}")))?;
    if !status.success() {
        return Err(dm_core::DmError::ExternalCommand {
            command: args.command.join(" "),
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }
    Ok(())
}

/// Строит Command из argv: первый элемент — программа, остальные — args.
fn build_command(argv: &[String]) -> Command {
    let mut cmd = Command::new(&argv[0]);
    for a in &argv[1..] {
        cmd.arg(a);
    }
    cmd
}

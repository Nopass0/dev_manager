//! `dm kill <target>` — завершить процесс по имени/PID/порту/имени процесса.
//!
//! Target может быть:
//! - числом (PID);
//! - числом с префиксом `:` (порт, например `:3001`);
//! - именем процесса (`node`, `vite`) для группового завершения.

use crate::commands::ports::{kill_pid, pid_of_port};
use crate::commands::KillArgs;
use crate::output::{dim_style, print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use std::process::Command;

/// Точка входа команды.
pub async fn run(args: KillArgs) -> DmResult<()> {
    let target = args.target.trim();
    if target.is_empty() {
        return Err(dm_core::DmError::invalid_config(
            "укажите цель: PID, порт (:3001) или имя процесса.",
        ));
    }

    // 1. Порт: :3001
    if let Some(port_str) = target.strip_prefix(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return kill_by_port(port).await;
        }
    }

    // 2. PID (только цифры)
    if let Ok(pid) = target.parse::<u32>() {
        if kill_pid(pid) {
            println_styled(&format!("  ✓ процесс {pid} завершён"), success_style());
        } else {
            println_styled(&format!("  ! не удалось завершить процесс {pid}"), warn_style());
        }
        return Ok(());
    }

    // 3. Имя образа процесса (node, vite, deno…).
    kill_by_name(target)
}

/// Завершает процесс, занимающий порт.
async fn kill_by_port(port: u16) -> DmResult<()> {
    print_system(&format!("поиск процесса на порту {port}…"));
    match pid_of_port(port) {
        Some(p) if kill_pid(p) => {
            println_styled(&format!("  ✓ процесс {p} (порт {port}) завершён"), success_style());
        }
        Some(p) => {
            println_styled(&format!("  ! не удалось завершить процесс {p}"), warn_style());
        }
        None => {
            println_styled(&format!("  • порт {port} не занят"), dim_style());
        }
    }
    Ok(())
}

/// Завершает все процессы с заданным именем образа.
fn kill_by_name(name: &str) -> DmResult<()> {
    print_system(&format!("завершение процессов с именем '{name}'…"));
    let ok = kill_by_name_platform(name);
    if ok {
        println_styled(&format!("  ✓ процессы '{name}' завершены"), success_style());
    } else {
        println_styled(
            &format!("  ! процессов '{name}' не найдено или нет прав"),
            warn_style(),
        );
    }
    Ok(())
}

/// Платформенная реализация завершения по имени образа.
fn kill_by_name_platform(name: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("taskkill")
            .args(["/F", "/T", "/IM", name])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(unix)]
    {
        Command::new("pkill")
            .args(["-9", "-f", name])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

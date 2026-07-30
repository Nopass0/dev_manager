//! `dm ports [--free=N]` — управление занятыми портами.
//!
//! Без аргументов: список процессов, слушающих типичные dev-порты. С `--free=N`
//! завершает процесс, занимающий порт N (через платформенную утилиту).

use crate::commands::PortsArgs;
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use std::process::Command;

/// Точка входа команды.
pub async fn run(args: PortsArgs) -> DmResult<()> {
    if let Some(port) = args.free {
        return free_port(port).await;
    }
    print_system("активные слушатели на типичных dev-портах:");
    let ports = [3000, 3001, 5173, 8080, 5432, 6379, 9000];
    for port in ports {
        let busy = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok();
        if busy {
            let owner = owner_of_port(port);
            println_styled(
                &format!("  • {port} занят {owner}"),
                warn_style(),
            );
        }
    }
    Ok(())
}

/// Завершает процесс, занимающий порт `port`.
async fn free_port(port: u16) -> DmResult<()> {
    print_system(&format!("освобождение порта {port}…"));
    let pid = pid_of_port(port);
    match pid {
        Some(pid) => {
            let killed = kill_pid(pid);
            if killed {
                println_styled(&format!("  ✓ процесс {pid} завершён, порт {port} свободен"), success_style());
            } else {
                println_styled(&format!("  ! не удалось завершить процесс {pid}"), warn_style());
            }
        }
        None => {
            println_styled(&format!("  • порт {port} не занят"), crate::output::dim_style());
        }
    }
    Ok(())
}

/// Возвращает описание владельца порта (строка для вывода).
fn owner_of_port(port: u16) -> String {
    match pid_of_port(port) {
        Some(pid) => format!("(pid {pid})"),
        None => "(владелец неизвестен)".to_string(),
    }
}

/// Определяет PID процесса, слушающего порт (через платформенную утилиту).
///
/// Публичная, чтобы переиспользовать из `dm kill`.
pub fn pid_of_port(port: u16) -> Option<u32> {
    #[cfg(windows)]
    {
        // netstat -ano | findstr :PORT → последняя колонка PID.
        let out = Command::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains(&format!(":{port} ")) && line.contains("LISTENING") {
                return line.split_whitespace().last().and_then(|s| s.parse().ok());
            }
        }
        None
    }
    #[cfg(unix)]
    {
        // lsof -ti :PORT возвращает PID напрямую.
        Command::new("lsof")
            .args(["-ti", &format!(":{port}")])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Завершает процесс с заданным PID. Возвращает true при успехе.
pub fn kill_pid(pid: u32) -> bool {
    #[cfg(windows)]
    {
        Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

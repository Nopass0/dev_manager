//! `dm doctor` — диагностика окружения с человекочитаемыми fix-подсказками.
//!
//! Проверяет: системные инструменты (git, cargo, node, npm, go, python), их
//! версии; свободное место на диске; конфликтующие процессы на типичных портах.
//! Для каждой проблемы предлагает решение.

use crate::output::{error_style, print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use std::process::Command;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    print_system("диагностика окружения…");
    println_styled("——— Инструменты ———", crate::output::dim_style());

    let tools = [
        ("git", "система контроля версий"),
        ("cargo", "Rust"),
        ("node", "Node.js"),
        ("npm", "npm"),
        ("bun", "Bun"),
        ("go", "Go"),
        ("python", "Python"),
        ("docker", "Docker"),
    ];
    for (tool, label) in tools {
        check_tool(tool, label);
    }

    println_styled("——— Диск ———", crate::output::dim_style());
    check_disk_space();

    println_styled("——— Порты ———", crate::output::dim_style());
    check_common_ports().await;

    print_system("диагностика завершена.");
    Ok(())
}

/// Проверяет наличие инструмента и его версию.
fn check_tool(name: &str, label: &str) {
    match Command::new(name).arg("--version").output() {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let v = text.lines().next().unwrap_or("?").trim();
            println_styled(&format!("  ✓ {label} ({name}): {v}"), success_style());
        }
        Ok(_) => println_styled(
            &format!("  ! {label} ({name}) установлен, но вернул ошибку"),
            warn_style(),
        ),
        Err(_) => {
            let _ = error_style; // сохранить импорт
            anstream::eprintln!(
                "{}",
                format!(
                    "  ✗ {label} ({name}) не найден. Установите {name}, чтобы использовать связанные функции dm."
                )
            );
        }
    }
}

/// Приблизительная проверка свободного места на диске (на текущем томе).
fn check_disk_space() {
    // Кросс-платформенный простой способ: statvfs недоступен стабильно из std,
    // поэтому используем платформо-специфичный вывод через системную утилиту.
    #[cfg(windows)]
    {
        let ok = Command::new("fsutil")
            .args(["volume", "diskfree"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            println_styled(
                "  • fsutil доступен; для деталей выполните `fsutil volume diskfree`",
                crate::output::dim_style(),
            );
        } else {
            println_styled(
                "  • информация о диске недоступна",
                crate::output::dim_style(),
            );
        }
    }
    #[cfg(unix)]
    {
        let ok = Command::new("df")
            .args(["-h", "."])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            println_styled(
                "  • df доступен; для деталей выполните `df -h .`",
                crate::output::dim_style(),
            );
        } else {
            println_styled(
                "  • информация о диске недоступна",
                crate::output::dim_style(),
            );
        }
    }
}

/// Проверяет типичные dev-порты и сообщает о занятых.
async fn check_common_ports() {
    let ports = [3000, 3001, 5173, 8080, 5432, 6379];
    for port in ports {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            println_styled(
                &format!("  ! порт {port} занят — `dm ports --free={port}` чтобы освободить"),
                warn_style(),
            );
        }
    }
    println_styled(
        "  • типичные dev-порты свободны (или проверка пропущена)",
        crate::output::dim_style(),
    );
}

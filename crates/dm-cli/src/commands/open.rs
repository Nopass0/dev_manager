//! `dm open <target>` — открыть цель в браузере/IDE.
//!
//! Target:
//! - имя сервиса: открыть его URL (берётся из `open:` или `health.url`, либо
//!   стандартный `http://localhost:3000`);
//! - `docs`: открыть документацию `docs/ru/README.md`;
//! - URL: открыть как есть.

use crate::commands::{load_project_config, OpenArgs};
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(args: OpenArgs) -> DmResult<()> {
    let target = args.target.trim();
    let url = match target {
        "docs" => {
            // Открываем корневой README (если docs есть — оглавление).
            "https://github.com/your-org/dev_manager/tree/main/docs/ru".to_string()
        }
        svc if std::path::Path::new(svc).exists() || is_url(svc) => svc.to_string(),
        svc => {
            // Имя сервиса: пытаемся вытащить URL из health-check или использовать
            // стандартный dev-порт по языку.
            let (config, root) = load_project_config()?;
            let _ = root;
            let url = config
                .services
                .get(svc)
                .and_then(|s| s.health.as_ref())
                .and_then(|h| h.url.clone())
                .unwrap_or_else(|| {
                    let port = config
                        .services
                        .get(svc)
                        .map(|s| default_port_for_language(s.language))
                        .unwrap_or(3000);
                    format!("http://localhost:{port}")
                });
            url
        }
    };
    print_system(&format!("открытие {url}…"));
    if open_in_browser(&url) {
        println_styled(&format!("  ✓ открыто в браузере по умолчанию"), success_style());
    } else {
        println_styled("  ! не удалось открыть браузер; откройте URL вручную.", warn_style());
        println_styled(&format!("    {url}"), crate::output::dim_style());
    }
    Ok(())
}

/// Открывает URL системным способом (xdg-open / open / start).
fn open_in_browser(url: &str) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Проверяет, является ли строка URL.
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Стандартный dev-порт по языку фреймворка.
fn default_port_for_language(lang: dm_core::project::ServiceLanguage) -> u16 {
    use dm_core::project::ServiceLanguage::*;
    match lang {
        Vite => 5173,
        Nextjs | Remix => 3000,
        Nodejs | JavaScript | TypeScript => 3000,
        Rust => 8080,
        _ => 3000,
    }
}

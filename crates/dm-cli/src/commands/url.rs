//! `dm url <svc>` — вывести URL сервиса (из health.url или стандартный порт).

use crate::commands::load_project_config;
use crate::output::println_styled;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(name: &str) -> DmResult<()> {
    let (config, _root) = load_project_config()?;
    let svc = config
        .services
        .get(name)
        .ok_or_else(|| dm_core::DmError::ServiceNotFound(name.to_string()))?;
    let url = svc
        .health
        .as_ref()
        .and_then(|h| h.url.clone())
        .unwrap_or_else(|| {
            let port = default_port(svc.language);
            format!("http://localhost:{port}")
        });
    println_styled(&url, crate::output::success_style());
    Ok(())
}

fn default_port(lang: dm_core::project::ServiceLanguage) -> u16 {
    use dm_core::project::ServiceLanguage::*;
    match lang {
        Vite => 5173,
        Nextjs | Remix => 3000,
        Nodejs | JavaScript | TypeScript => 3000,
        Rust => 8080,
        _ => 3000,
    }
}

//! `dm ping <svc>` — проверить доступность сервиса по его health-config.

use crate::commands::load_project_config;
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_core::config::HealthCheckKind;

/// Точка входа команды.
pub async fn run(name: &str) -> DmResult<()> {
    let (config, _root) = load_project_config()?;
    let svc = config
        .services
        .get(name)
        .ok_or_else(|| dm_core::DmError::ServiceNotFound(name.to_string()))?;

    let healthy = match &svc.health {
        Some(hc) => match hc.kind {
            HealthCheckKind::None => true,
            HealthCheckKind::Tcp => match hc.port {
                Some(p) => tcp_ok(p).await,
                None => true,
            },
            HealthCheckKind::Http => match &hc.url {
                Some(u) => http_ok(u).await,
                None => true,
            },
        },
        None => {
            // Без health-config пробуем стандартный порт по языку.
            let port = default_port(svc.language);
            tcp_ok(port).await
        }
    };
    if healthy {
        println_styled(&format!("✓ {name} доступен"), success_style());
    } else {
        print_system(&format!("✗ {name} не отвечает"));
    }
    Ok(())
}

async fn tcp_ok(port: u16) -> bool {
    tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_ok()
}

async fn http_ok(url: &str) -> bool {
    reqwest_status(url).await
}

/// Минимальная HTTP-проверка без внешних зависимостей (raw TCP).
async fn reqwest_status(url: &str) -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let parsed = match parse(url) {
        Some(u) => u,
        None => return false,
    };
    let mut stream = match tokio::net::TcpStream::connect((parsed.host.as_str(), parsed.port)).await
    {
        Ok(s) => s,
        Err(_) => return false,
    };
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        parsed.path, parsed.host
    );
    if stream.write_all(req.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return false;
    }
    String::from_utf8_lossy(&buf[..n])
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .map(|c| c.starts_with('2'))
        .unwrap_or(false)
}

fn parse(url: &str) -> Option<Parsed> {
    let (scheme, rest) = url.split_once("://")?;
    let default = if scheme == "https" { 443 } else { 80 };
    let (hp, path) = match rest.split_once('/') {
        Some((a, b)) => (a.to_string(), format!("/{b}")),
        None => (rest.to_string(), "/".to_string()),
    };
    let (host, port) = match hp.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(default)),
        None => (hp, default),
    };
    Some(Parsed { host, port, path })
}

struct Parsed {
    host: String,
    port: u16,
    path: String,
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

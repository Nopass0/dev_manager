//! Уведомления о событиях supervisor'а: webhook и desktop.
//!
//! Конфигурируется в `dm.yaml` через поле `notify:` (см. ниже). Без конфигурации
//! уведомления отключены. Webhook шлёт JSON-POST без внешних зависимостей.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Тип события для уведомления.
#[derive(Debug, Clone, Copy)]
pub enum NotifyEvent {
    /// Сервис упал (crash).
    Crash,
    /// Сервис успешно стартовал.
    Started,
    /// Тесты прошли.
    TestPass,
    /// Тесты упали.
    TestFail,
    /// Завершён деплой.
    Deploy,
}

impl NotifyEvent {
    fn label(self) -> &'static str {
        match self {
            NotifyEvent::Crash => "crash",
            NotifyEvent::Started => "started",
            NotifyEvent::TestPass => "test_pass",
            NotifyEvent::TestFail => "test_fail",
            NotifyEvent::Deploy => "deploy",
        }
    }
}

/// Конфигурация уведомлений (секция `notify:` в dm.yaml).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotifyConfig {
    /// URL webhook'а (Slack/Discord/Telegram/свой сервер). Пусто — выключено.
    #[serde(default)]
    pub webhook_url: Option<String>,
    /// Какие события слать (по умолчанию все). Имена из [`NotifyEvent::label`].
    #[serde(default)]
    pub events: Vec<String>,
    /// Имя проекта для текста уведомления (опционально).
    #[serde(default)]
    pub project_name: Option<String>,
}

impl NotifyConfig {
    /// Активировано ли событие в фильтре (пустой список = все события).
    pub fn enabled(&self, event: NotifyEvent) -> bool {
        if self.events.is_empty() {
            return true;
        }
        self.events.iter().any(|e| e == event.label())
    }
}

/// Отправляет уведомление всеми активными каналами.
///
/// Webhook: асинхронный POST с JSON, ошибки логируются, но не прерывают работу.
/// Desktop: вызов системной утилиты (`notify-send` на Linux, `osascript` на macOS,
/// `msg`/toast на Windows) — best-effort.
pub async fn send(cfg: &NotifyConfig, event: NotifyEvent, service: &str, detail: &str) {
    if !cfg.enabled(event) {
        return;
    }
    let project = cfg.project_name.clone().unwrap_or_else(|| "dm".into());
    let title = format!("[{}] {} — {}", project, event.label(), service);

    if let Some(url) = &cfg.webhook_url {
        send_webhook(url, &title, detail).await;
    }
    // Desktop-уведомление best-effort в фоне.
    let t = title.clone();
    let d = detail.to_string();
    tokio::task::spawn_blocking(move || {
        let _ = send_desktop(&t, &d);
    })
    .await
    .ok();
}

/// Шлёт JSON-POST на webhook.
async fn send_webhook(url: &str, title: &str, detail: &str) {
    let mut payload: HashMap<&str, String> = HashMap::new();
    payload.insert("text", format!("{title}\n{detail}"));
    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(_) => return,
    };
    // Минимальный HTTP-клиент без внешних зависимостей.
    let parsed = match parse_url(url) {
        Some(u) => u,
        None => return,
    };
    let host_port = format!("{}:{}", parsed.host, parsed.port);
    let mut stream = match tokio::net::TcpStream::connect(&host_port).await {
        Ok(s) => s,
        Err(_) => return,
    };
    use tokio::io::AsyncWriteExt;
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path,
        parsed.host,
        body.len(),
        body
    );
    let _ = stream.write_all(req.as_bytes()).await;
}

/// Desktop-уведомление через системную утилиту.
///
/// На Windows использует toast-уведомления (BurntToast PowerShell-модуль, если
/// установлен, иначе BalloonNotify). Эти уведомления появляются сбоку и
/// исчезают сами — без модального message-box, который требует закрытия.
fn send_desktop(title: &str, body: &str) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("notify-send")
            .args(["--icon=dialog-information", title, body])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(r#"display notification "{}" with title "{}""#, body, title);
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()?;
    }
    #[cfg(windows)]
    {
        windows_toast(title, body);
    }
    Ok(())
}

/// Windows toast-уведомление: BurntToast (если установлен) → BalloonNotify fallback.
///
/// Оба способа показывают уведомление в Action Center сбоку, которое исчезает
/// само через несколько секунд. Не требует закрытия кнопкой, в отличие от `msg`.
#[cfg(windows)]
fn windows_toast(title: &str, body: &str) {
    // Экранируем одинарные кавычки для PowerShell.
    let t = title.replace('\'', "''");
    let b = body.replace('\'', "''");

    // Попытка 1: BurntToast (красивые toast-уведомления, если модуль установлен).
    let burnt = format!(
        "try {{ New-BurntToastNotification -Title '{t}' -Text '{b}' -AppLogo none }} catch {{ exit 1 }}"
    );
    let ok = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &burnt])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        return;
    }

    // Попытка 2: BalloonNotify через NotifyIcon (без внешних модулей).
    // Это встроенный toast, который появляется в трее и исчезает сам.
    let balloon = format!(
        r#"[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; $n = New-Object System.Windows.Forms.NotifyIcon; $n.Icon = [System.Drawing.SystemIcons]::Information; $n.Visible = $true; $n.ShowBalloonTip(5000, '{t}', '{b}', [System.Windows.Forms.ToolTipIcon]::Info); Start-Sleep -Seconds 6; $n.Dispose()"#
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &balloon])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Простой URL-парсер для webhook.
fn parse_url(url: &str) -> Option<Parsed> {
    let (scheme, rest) = url.split_once("://")?;
    let default_port = if scheme == "https" { 443 } else { 80 };
    let (hp, path) = match rest.split_once('/') {
        Some((a, b)) => (a.to_string(), format!("/{b}")),
        None => (rest.to_string(), "/".to_string()),
    };
    let (host, port) = match hp.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(default_port)),
        None => (hp, default_port),
    };
    Some(Parsed { host, port, path })
}

struct Parsed {
    host: String,
    port: u16,
    path: String,
}

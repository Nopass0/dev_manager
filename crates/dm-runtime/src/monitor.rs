//! Мониторинг ресурсов процесса (RSS памяти) для enforcement лимитов.
//!
//! Кросс-платформенное получение RSS:
//! - Linux: чтение `/proc/<pid>/status` (поле VmRSS);
//! - Windows: вызов `wmic process where ProcessId=<pid> get WorkingSetSize`;
//! - macOS: `ps -o rss= -p <pid>`.
//!
//! Используется supervisor'ом для enforcement `resources.memory_mb`: при
//! превышении — уведомление или kill (согласно `resources.on_exceed`).


/// Возвращает текущее RSS (resident set size) процесса в мегабайтах.
///
/// Best-effort: при ошибке возвращает `None` (мониторинг пропускает итерацию).
pub fn rss_mb(pid: u32) -> Option<u64> {
    let bytes = rss_bytes(pid)?;
    Some(bytes / 1024 / 1024)
}

/// Возвращает RSS процесса в байтах (платформенно).
fn rss_bytes(pid: u32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        rss_linux(pid)
    }
    #[cfg(windows)]
    {
        rss_windows(pid)
    }
    #[cfg(target_os = "macos")]
    {
        rss_macos(pid)
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

/// Linux: /proc/<pid>/status → VmRSS (в kB).
#[cfg(target_os = "linux")]
fn rss_linux(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            // "VmRSS:\t      1234 kB"
            let kb: u64 = rest
                .trim()
                .split_whitespace()
                .next()?
                .parse()
                .ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

/// Windows: wmic process get WorkingSetSize (в байтах).
#[cfg(windows)]
fn rss_windows(pid: u32) -> Option<u64> {
    let out = std::process::Command::new("wmic")
        .args([
            "process",
            "where",
            &format!("ProcessId={pid}"),
            "get",
            "WorkingSetSize",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // Вывод: "WorkingSetSize\n12345678\n"
    text.lines()
        .nth(1)?
        .trim()
        .parse::<u64>()
        .ok()
}

/// macOS: ps -o rss= -p <pid> (RSS в kB).
#[cfg(target_os = "macos")]
fn rss_macos(pid: u32) -> Option<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let kb: u64 = text.trim().parse().ok()?;
    Some(kb * 1024)
}

/// Результат проверки лимита памяти процесса.
#[derive(Debug, Clone)]
pub enum MemoryCheck {
    /// В пределах нормы (RSS МБ при лимите МБ).
    Ok { rss_mb: u64, limit_mb: u64 },
    /// Превышен лимит.
    Exceeded { rss_mb: u64, limit_mb: u64 },
    /// Лимит не задан (memory_mb == 0) или RSS недоступен.
    NotApplicable,
}

/// Проверяет, укладывается ли процесс `pid` в лимит памяти.
///
/// `limit_mb == 0` означает «без лимита».
pub fn check_memory(pid: u32, limit_mb: u64) -> MemoryCheck {
    if limit_mb == 0 {
        return MemoryCheck::NotApplicable;
    }
    match rss_mb(pid) {
        Some(rss) if rss > limit_mb => MemoryCheck::Exceeded {
            rss_mb: rss,
            limit_mb,
        },
        Some(rss) => MemoryCheck::Ok {
            rss_mb: rss,
            limit_mb,
        },
        None => MemoryCheck::NotApplicable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_memory_respects_zero_limit() {
        // Лимит 0 = без лимита.
        let r = check_memory(std::process::id(), 0);
        assert!(matches!(r, MemoryCheck::NotApplicable));
    }

    #[test]
    fn check_memory_flags_exceed() {
        // Текущий процесс точно занимает > 1 МБ → Exceeded при лимите 1.
        let r = check_memory(std::process::id(), 1);
        match r {
            MemoryCheck::Exceeded { .. } => {}
            MemoryCheck::NotApplicable => {
                // На некоторых платформах RSS недоступен — пропускаем.
            }
            other => panic!("неожиданный результат: {other:?}"),
        }
    }
}

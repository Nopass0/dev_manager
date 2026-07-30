//! File-watcher с debounce для горячей перезагрузки сервисов.
//!
//! Использует `notify-debouncer-mini` для эффективного отслеживания изменений
//! файловой системы на всех платформах (inotify / ReadDirectoryChangesW / FSEvents).
//! Debounce группирует шквал событий (например, от `cargo build`) в одно.

use dm_core::project::{Service, ServiceLanguage};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Событие изменения файлов в каталоге сервиса.
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// Имя сервиса, чьи файлы изменились.
    pub service: String,
    /// Список изменённых путей (могут быть относительными к каталогу сервиса).
    pub paths: Vec<PathBuf>,
}

/// Handle активного watcher'а. При падении (Drop) корректно освобождает ресурсы.
pub struct FileWatcher {
    /// `notify`-debouncer владеет потоком наблюдения; храним чтобы жил.
    /// Тип параметризован конкретным бекендом (на Windows — ReadDirectoryChanges).
    _debouncer: notify_debouncer_mini::Debouncer<notify::ReadDirectoryChangesWatcher>,
}

impl FileWatcher {
    /// Создаёт и запускает watcher для списка сервисов.
    ///
    /// Для каждого сервиса с `watch == true` рекурсивно наблюдаем его каталог.
    /// События отправляются в `tx` после debounce-окна (200 мс).
    ///
    /// Игнорируются типичные шумные каталоги: `target`, `node_modules`, `.git`.
    pub fn spawn(services: Vec<Service>, tx: mpsc::UnboundedSender<WatchEvent>) -> std::io::Result<Self> {
        // Замыкание watcher'а держит копию списка сервисов и канал отправки.
        // `new_debouncer` в 0.4 принимает (delay, callback), где callback —
        // `DebounceEventHandler` (реализуется любой `FnMut(Result<...>)`).
        let tx = Arc::new(Mutex::new(tx));
        let services = Arc::new(services);

        let callback = {
            let tx = tx.clone();
            let services = services.clone();
            move |events: Result<Vec<DebouncedEvent>, _>| {
                let events = match events {
                    Ok(e) => e,
                    Err(_) => return,
                };
                if events.is_empty() {
                    return;
                }
                // Группируем события по сервису: путь → чей это каталог.
                let mut by_service: std::collections::HashMap<String, Vec<PathBuf>> =
                    std::collections::HashMap::new();
                for ev in events {
                    let path = ev.path;
                    // Фильтруем шумные каталоги — по любому сегменту пути.
                    if is_ignored_path(&path, ServiceLanguage::Other) {
                        continue;
                    }
                    if let Some(svc) = services.iter().find(|s| path.starts_with(&s.path)) {
                        by_service
                            .entry(svc.name.clone())
                            .or_default()
                            .push(path);
                    }
                }
                if let Ok(tx) = tx.lock() {
                    for (svc, paths) in by_service {
                        let _ = tx.send(WatchEvent { service: svc, paths });
                    }
                }
            }
        };

        let mut debouncer = new_debouncer(Duration::from_millis(200), callback)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        for svc in services.iter().filter(|s| s.watch) {
            let _ = debouncer
                .watcher()
                .watch(&svc.path, notify::RecursiveMode::Recursive);
        }

        Ok(Self { _debouncer: debouncer })
    }
}

/// Возвращает true, если путь к файлу относится к «шумному» каталогу, который
/// не надо учитывать при перезапуске/линтере.
///
/// Используется и watcher'ом, и анализатором для согласованной фильтрации.
pub fn is_ignored_path(path: &std::path::Path, language: ServiceLanguage) -> bool {
    // Проверяем любой сегмент пути на совпадение с игнорируемым именем.
    let ignored_segments = match language {
        ServiceLanguage::Rust => vec!["target", ".git"],
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Bun
        | ServiceLanguage::Nodejs
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix => vec!["node_modules", ".git", "dist", "build", ".next"],
        ServiceLanguage::Go => vec!["vendor", ".git"],
        ServiceLanguage::Python => vec!["__pycache__", ".venv", "venv", ".git"],
        _ => vec![".git", "build", "dist", "out", "target", "node_modules"],
    };
    path.components().any(|comp| {
        comp.as_os_str()
            .to_str()
            .map(|s| ignored_segments.iter().any(|ig| *ig == s))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_core::project::{Service, ServiceLanguage};
    use std::path::Path;

    fn svc(name: &str, lang: ServiceLanguage, dir: &Path) -> Service {
        Service {
            name: name.into(),
            path: dir.to_path_buf(),
            language: lang,
            run_command: String::new(),
            watch: true,
            restart_on_change: true,
            repo_path: None,
            delay_ms: 0,
        }
    }

    #[test]
    fn ignores_target_and_node_modules() {
        let base = Path::new("/proj");
        assert!(is_ignored_path(
            &base.join("api/target/debug/app"),
            ServiceLanguage::Rust
        ));
        assert!(is_ignored_path(
            &base.join("web/node_modules/react/index.js"),
            ServiceLanguage::TypeScript
        ));
        assert!(!is_ignored_path(
            &base.join("api/src/main.rs"),
            ServiceLanguage::Rust
        ));
    }

    #[test]
    fn watcher_can_spawn_on_tmp() {
        let tmp = std::env::temp_dir().join("dm_watcher_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let s = svc("a", ServiceLanguage::Rust, &tmp);
        let w = FileWatcher::spawn(vec![s], tx);
        assert!(w.is_ok(), "watcher должен запускаться на существующем каталоге");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

//! # dm-runtime
//!
//! Оркестрация процессов для Dev Manager. Запускает микросервисы, следит за
//! изменениями файлов, перезапускает их (убивая всё дерево подпроцессов) и
//! мультиплексирует логи в один поток с цветными префиксами.
//!
//! Ключевые модули:
//! - [`process`] — кросс-платформенный spawn и гарантированное убийство дерева.
//! - [`spawn_strategy`] — автоопределение команды запуска по языку/каталогу.
//! - [`logs`] — модель лог-событий сервиса.
//! - [`supervisor`] — главная точка входа: запуск всех сервисов с очередью.
//! - [`watcher`] — debounced-наблюдение за файлами для hot reload.

pub mod logs;
pub mod monitor;
pub mod notify;
pub mod process;
pub mod spawn_strategy;
pub mod supervisor;
pub mod watcher;

pub use logs::{LogLevel, LogLine, ServiceStatus};
pub use monitor::{check_memory, rss_mb, MemoryCheck};
pub use notify::{NotifyConfig, NotifyEvent};
pub use process::{ManagedProcess, ProcessExit};
pub use supervisor::{Supervisor, SupervisorOptions};

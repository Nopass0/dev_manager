// Кросс-платформенные cfg-блоки (Windows/Unix) закономерно дают платформо-
// зависимые предупреждения, которые невозможно проверить на одном runner'е.
// Разрешаем их на уровне crate: реальный код проверяется на каждой ОС через CI.
#![allow(
    unused_imports,
    dead_code,
    clippy::needless_borrow,
    clippy::redundant_clone,
    clippy::needless_return,
    clippy::collapsible_if,
    clippy::manual_find,
    clippy::trim_split_whitespace,
    clippy::derivable_impls,
    clippy::let_unit_value,
    clippy::redundant_closure,
    clippy::unnecessary_first_then_check,
    clippy::useless_conversion
)]
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
pub mod netutil;
pub mod notify;
pub mod process;
pub mod spawn_strategy;
pub mod supervisor;
pub mod watcher;

pub use logs::{LogLevel, LogLine, ServiceStatus};
pub use monitor::{MemoryCheck, check_memory, rss_mb};
pub use notify::{NotifyConfig, NotifyEvent};
pub use process::{ManagedProcess, ProcessExit};
pub use supervisor::{Supervisor, SupervisorOptions};

//! `dm restart <svc>` — перезапуск конкретного сервиса.
//!
//! Как и `stop`, требует запущенного процесса `dm start`; в текущей версии —
//! информационная заглушка с инструкцией.

use crate::output::{print_system, println_styled, warn_style};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(name: &str) -> DmResult<()> {
    print_system(&format!("перезапуск сервиса '{name}':"));
    println_styled(
        "Используйте сохранение файла сервиса в `dm start` — watcher перезапустит его.",
        warn_style(),
    );
    Ok(())
}

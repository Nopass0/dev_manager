//! `dm stop` — остановка сервисов текущего `dm start`.
//!
//! В текущей версии сервисы живут в рамках процесса `dm start`; корректный
//! способ их остановить — Ctrl+C в этом процессе. Эта команда — заглушка,
//! которая подсказывает пользователю, как действовать (полноценный daemon-режим
//! с PID-файлом заложен на следующую итерацию).

use crate::output::{dim_style, print_system, println_styled, warn_style};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    print_system("остановка сервисов:");
    println_styled(
        "Если `dm start` запущен в другом терминале — нажмите Ctrl+C в нём.",
        warn_style(),
    );
    println_styled(
        "Фоновый daemon-режим с `dm stop` будет добавлен в следующей итерации.",
        dim_style(),
    );
    Ok(())
}

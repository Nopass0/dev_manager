//! `dm logs [svc]` — вывод логов сервисов.
//!
//! Логи стримятся в реальном времени из активного `dm start`. В офлайн-режиме
//! (без запущенного `dm start`) команда показывает последние сохранённые логи,
//! если они есть (планируется в следующей итерации).

use crate::output::{dim_style, print_system, println_styled};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(name: Option<&str>) -> DmResult<()> {
    match name {
        Some(n) => print_system(&format!("логи сервиса '{n}':")),
        None => print_system("логи всех сервисов:"),
    }
    println_styled(
        "Логи стримятся в процессе `dm start`. Сохранение и чтение истории — в следующей итерации.",
        dim_style(),
    );
    Ok(())
}

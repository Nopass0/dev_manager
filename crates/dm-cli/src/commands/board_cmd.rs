//! `dm board [--port=11001]` — запуск локальной kanban-доски задач.
//!
//! Поднимает HTTP-сервер на порту 11001 (или указанном) с красивым kanban UI.
//! Данные хранятся в `.dm/board.json` (с SHA-хешем для защиты целостности).

use crate::board;
use crate::output::print_system;
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(port: Option<u16>) -> DmResult<()> {
    let port = port.unwrap_or(board::DEFAULT_PORT);
    // Путь к файлу доски: .dm/board.json в корне проекта (где dm.yaml).
    let root = std::env::current_dir()?;
    let board_path = root.join(".dm").join("board.json");
    print_system(&format!(
        "kanban-доска: http://localhost:{port} | данные: {}",
        board_path.display()
    ));
    // Сервер блокирующий — запускаем в текущем потоке.
    board::serve(board_path, port)
        .map_err(|e| dm_core::DmError::Process(format!("kanban-сервер: {e}")))?;
    Ok(())
}

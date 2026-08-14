//! `dm lua <script.lua>` — выполнить Lua-скрипт с dm API.
//!
//! Скрипты получают модули: `dm_os` (exec/spawn/sleep/getenv), `fs` (read/write/
//! copy/mkdir/remove), `http` (get/post/put/delete), `log` (info/warn/error),
//! `dm` (run — вызов dm-команд).
//!
//! Пример smoke-теста сервиса:
//! ```lua
//! local resp = http.get("http://localhost:8080/health")
//! assert(resp.status == 200, "health failed")
//! log.info("OK: " .. resp.body)
//! ```

use crate::output::{error_style, print_system, println_styled, success_style};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(script: &str) -> DmResult<()> {
    let path = std::path::Path::new(script);
    if !path.exists() {
        return Err(dm_core::DmError::invalid_config(format!(
            "скрипт не найден: {script}. Создайте .lua файл или используйте `dm lua scripts/test.lua`."
        )));
    }
    print_system(&format!("выполнение Lua: {script}"));
    match dm_lua::run_script(path) {
        Ok(()) => {
            println_styled("✓ скрипт выполнен успешно", success_style());
            Ok(())
        }
        Err(e) => {
            println_styled(&format!("✗ ошибка скрипта: {e}"), error_style());
            Err(dm_core::DmError::Process(format!("lua script failed: {e}")))
        }
    }
}

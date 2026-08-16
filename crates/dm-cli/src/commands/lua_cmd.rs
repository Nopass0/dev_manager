//! `dm lua <script.lua>` / `dm lua -e '<code>'` — выполнение Lua с dm API.
//!
//! Два режима:
//! 1. Файл: `dm lua scripts/test.lua`
//! 2. Inline: `dm lua -e 'log.info("hello")'`

use crate::output::{error_style, print_system, println_styled, success_style};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(script: Option<&str>, eval: Option<&str>) -> DmResult<()> {
    if let Some(code) = eval {
        print_system(&format!("lua (inline): {code}"));
        return match dm_lua::run_inline(code) {
            Ok(()) => {
                println_styled("OK", success_style());
                Ok(())
            }
            Err(e) => {
                println_styled(&format!("FAIL: {e}"), error_style());
                Err(dm_core::DmError::Process(format!("lua inline failed: {e}")))
            }
        };
    }

    let script = script.ok_or_else(|| {
        dm_core::DmError::invalid_config(
            "specify script or inline code: `dm lua <script.lua>` or `dm lua -e '<code>'`",
        )
    })?;

    let path = std::path::Path::new(script);
    if !path.exists()
        && !script.contains('/')
        && !script.contains('\\')
        && !script.ends_with(".lua")
    {
        print_system(&format!("lua (inline): {script}"));
        return match dm_lua::run_inline(script) {
            Ok(()) => {
                println_styled("OK", success_style());
                Ok(())
            }
            Err(e) => {
                println_styled(&format!("FAIL: {e}"), error_style());
                Err(dm_core::DmError::Process(format!("lua failed: {e}")))
            }
        };
    }

    if !path.exists() {
        return Err(dm_core::DmError::invalid_config(format!(
            "script not found: {script}"
        )));
    }
    print_system(&format!("lua: {script}"));
    match dm_lua::run_script(path) {
        Ok(()) => {
            println_styled("OK", success_style());
            Ok(())
        }
        Err(e) => {
            println_styled(&format!("FAIL: {e}"), error_style());
            Err(dm_core::DmError::Process(format!("lua script failed: {e}")))
        }
    }
}

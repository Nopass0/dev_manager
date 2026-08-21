//! `dm lua <script.lua>` / `dm lua -e '<code>'` — выполнение Lua с dm API.
//!
//! Режимы:
//! 1. Файл: `dm lua scripts/test.lua`
//! 2. Inline: `dm lua -e 'code'`
//! 3. Dry-run: `dm lua --dry-run file.lua` (синтаксис без выполнения)

use crate::output::{error_style, print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;

pub async fn run(script: Option<&str>, eval: Option<&str>, dry_run: bool) -> DmResult<()> {
    let code = if let Some(e) = eval {
        e.to_string()
    } else if let Some(s) = script {
        let path = std::path::Path::new(s);
        if path.exists() {
            std::fs::read_to_string(path)
                .map_err(|e| dm_core::DmError::invalid_config(format!("read {s}: {e}")))?
        } else if s.contains('/') || s.contains('\\') || s.ends_with(".lua") {
            return Err(dm_core::DmError::invalid_config(format!(
                "script not found: {s}"
            )));
        } else {
            if has_stripped_quotes(s) {
                println_styled("", crate::output::dim_style());
                println_styled("PowerShell stripped your quotes!", warn_style());
                println_styled("", crate::output::dim_style());
                println_styled(
                    "  Your input:  dm lua 'print(x, \"text\")'",
                    crate::output::dim_style(),
                );
                println_styled(
                    "  Received:    print(x, text)          ← quotes missing!",
                    crate::output::dim_style(),
                );
                println_styled("", crate::output::dim_style());
                println_styled(
                    "  FIX - use single quotes inside double quotes:",
                    warn_style(),
                );
                println_styled("  dm lua -e \"print('hello')\"", success_style());
                println_styled("", crate::output::dim_style());
                println_styled(
                    "  Or for scripts with double quotes, use a .lua file instead.",
                    crate::output::dim_style(),
                );
                println_styled("", crate::output::dim_style());
            }
            s.to_string()
        }
    } else {
        return Err(dm_core::DmError::invalid_config(
            "specify script or code: dm lua <file.lua> or dm lua -e '<code>'",
        ));
    };

    if dry_run {
        let preview = &code[..code.len().min(60)];
        print_system(&format!("dry-run: {}...", preview));
        return match dm_lua::check_syntax(&code) {
            Ok(()) => {
                println_styled("syntax OK", success_style());
                Ok(())
            }
            Err(e) => {
                println_styled(&format!("syntax error: {e}"), error_style());
                Err(dm_core::DmError::Process(format!("syntax error: {e}")))
            }
        };
    }

    let is_inline = eval.is_some() || !std::path::Path::new(script.unwrap_or("")).exists();
    if is_inline {
        let preview = &code[..code.len().min(80)];
        print_system(&format!("lua: {}", preview));
    } else if let Some(s) = script {
        print_system(&format!("lua: {s}"));
    }

    match dm_lua::run_inline(&code) {
        Ok(()) => {
            if !is_inline {
                println_styled("done", success_style());
            }
            Ok(())
        }
        Err(e) => {
            println_styled(&format!("FAIL: {e}"), error_style());
            Err(dm_core::DmError::Process(format!("lua failed: {e}")))
        }
    }
}

fn has_stripped_quotes(code: &str) -> bool {
    code.contains('(') && !code.contains('"') && !code.contains('\'') && code.contains(')')
}

//! `dm completions <shell>` — генерация shell-completion для bash/zsh/fish/powershell.
//!
//! Использует `clap_complete`. Выводит скрипт в stdout; пользователь сохраняет
//! его в нужное место (или использует `eval "$(dm completions bash)"`).

use crate::output::{println_styled, success_style};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use dm_core::DmResult;

/// Точка входа команды.
pub fn run(shell: &str) -> DmResult<()> {
    let shell = shell.trim().to_lowercase();
    let shell: Shell = match shell.as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        other => {
            return Err(dm_core::DmError::invalid_config(format!(
                "неподдерживаемая оболочка: '{other}'. Доступно: bash, zsh, fish, powershell, elvish."
            )));
        }
    };
    let mut cmd = crate::Cli::command();
    generate(shell, &mut cmd, "dm", &mut std::io::stdout());
    let _ = success_style; // сохранить импорт для будущего цветного вывода
    let _ = println_styled;
    Ok(())
}

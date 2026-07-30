//! `dm init` — создаёт `dm.yaml` в текущем каталоге.

use crate::commands::PREFIX_SYS;
use crate::output::{info_style, println_styled, success_style};
use dm_core::config::CONFIG_FILENAME;
use dm_core::DmResult;

/// Шаблон минимального `dm.yaml`, который создаст `dm init`.
const TEMPLATE: &str = include_str!("../../../../dm.example.yaml");

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    let cwd = std::env::current_dir()?;
    let target = cwd.join(CONFIG_FILENAME);
    if target.exists() {
        println_styled(
            &format!("{} уже существует — пропускаем.", CONFIG_FILENAME),
            crate::output::warn_style(),
        );
        return Ok(());
    }
    std::fs::write(&target, TEMPLATE)?;
    println_styled(
        &format!("{PREFIX_SYS} создан {CONFIG_FILENAME} в {}", cwd.display()),
        success_style(),
    );
    println_styled(
        "Отредактируйте его под свой проект, затем запустите `dm start`.",
        info_style(),
    );
    Ok(())
}

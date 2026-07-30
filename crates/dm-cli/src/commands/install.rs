//! `dm install` — установка текущего бинарника в PATH.

use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_installer::install;

/// Точка входа команды.
pub async fn run() -> DmResult<()> {
    // Определяем путь к текущему исполняемому файлу.
    let exe = std::env::current_exe().map_err(|e| {
        dm_core::DmError::Process(format!("не удалось определить путь к бинарнику: {e}"))
    })?;
    print_system(&format!("установка из {}", exe.display()));
    let result = install(&exe)?;
    println_styled(
        &format!("✓ бинарник: {}", result.bin_path.display()),
        success_style(),
    );
    if result.path_updated {
        println_styled("✓ PATH обновлён", success_style());
        println_styled(
            "Перезапустите терминал, чтобы команда `dm` стала доступна.",
            crate::output::info_style(),
        );
    } else {
        println_styled("• каталог уже был в PATH", crate::output::dim_style());
    }
    Ok(())
}

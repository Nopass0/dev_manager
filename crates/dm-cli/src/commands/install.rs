//! `dm install [--all-users]` — установка текущего бинарника в PATH.
//!
//! `--all-users` устанавливает для всех пользователей (Windows: Program Files +
//! Machine-scope PATH; нужен запуск от администратора). По умолчанию — только
//! текущий пользователь.

use crate::commands::InstallArgs;
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_installer::install_for;

/// Точка входа команды.
pub async fn run(args: InstallArgs) -> DmResult<()> {
    let exe = std::env::current_exe().map_err(|e| {
        dm_core::DmError::Process(format!("не удалось определить путь к бинарнику: {e}"))
    })?;
    let scope = if args.all_users {
        "всех пользователей"
    } else {
        "текущего пользователя"
    };
    print_system(&format!("установка из {} (scope: {scope})", exe.display()));
    let result = install_for(&exe, args.all_users)?;
    println_styled(
        &format!("✓ бинарник: {}", result.bin_path.display()),
        success_style(),
    );
    if result.path_updated {
        println_styled(&format!("✓ PATH обновлён ({scope})"), success_style());
        println_styled(
            "Перезапустите терминал, чтобы команда `dm` стала доступна.",
            crate::output::info_style(),
        );
    } else {
        println_styled("• каталог уже был в PATH", crate::output::dim_style());
    }
    Ok(())
}

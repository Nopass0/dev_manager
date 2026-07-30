//! `dm hooks install|uninstall|run` — управление git-хуками проекта.
//!
//! Устанавливает shell-хуки в `.git/hooks/`, которые вызывают `dm` для:
//! - `pre-commit` → `dm format && dm lint` (быстрые проверки);
//! - `pre-push` → `dm test` (полные тесты).

use crate::commands::{load_project_config, HooksArgs};
use crate::output::{print_system, success_style, warn_style, println_styled};
use dm_core::DmResult;
use std::path::PathBuf;

/// Точка входа команды.
pub async fn run(args: HooksArgs) -> DmResult<()> {
    let (_config, root) = load_project_config()?;
    let git_dir = find_git_dir(&root);
    let Some(git_dir) = git_dir else {
        return Err(dm_core::DmError::invalid_config(
            "каталог .git не найден — это не git-репозиторий.",
        ));
    };
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    match args.action.as_str() {
        "install" => install_hook(&hooks_dir, &args.hook),
        "uninstall" => uninstall_hook(&hooks_dir, &args.hook),
        "run" => run_hook_inline(&args.hook).await,
        other => Err(dm_core::DmError::invalid_config(format!(
            "неизвестное действие '{other}'. Доступно: install | uninstall | run."
        ))),
    }
}

/// Находит `.git` поднимаясь от `start` вверх.
fn find_git_dir(start: &std::path::Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

/// Тело pre-commit хука: format + lint.
const PRE_COMMIT_BODY: &str = r#"#!/usr/bin/env sh
# Установлено Dev Manager (`dm hooks install`).
set -e
echo "[dm] pre-commit: format + lint"
dm format
dm lint
"#;

/// Тело pre-push хука: test.
const PRE_PUSH_BODY: &str = r#"#!/usr/bin/env sh
# Установлено Dev Manager (`dm hooks install`).
set -e
echo "[dm] pre-push: test"
dm test
"#;

/// Устанавливает хук `name`.
fn install_hook(hooks_dir: &std::path::Path, name: &str) -> DmResult<()> {
    let (filename, body) = match name {
        "pre-commit" => ("pre-commit", PRE_COMMIT_BODY),
        "pre-push" => ("pre-push", PRE_PUSH_BODY),
        other => {
            return Err(dm_core::DmError::invalid_config(format!(
                "неподдерживаемый хук '{other}'. Доступно: pre-commit | pre-push."
            )));
        }
    };
    let path = hooks_dir.join(filename);
    std::fs::write(&path, body)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).ok();
    }
    println_styled(&format!("  ✓ хук {filename} установлен в {}", path.display()), success_style());
    Ok(())
}

/// Удаляет хук.
fn uninstall_hook(hooks_dir: &std::path::Path, name: &str) -> DmResult<()> {
    let filename = match name {
        "pre-commit" => "pre-commit",
        "pre-push" => "pre-push",
        other => {
            return Err(dm_core::DmError::invalid_config(format!(
                "неподдерживаемый хук '{other}'"
            )));
        }
    };
    let path = hooks_dir.join(filename);
    if path.exists() {
        std::fs::remove_file(&path)?;
        println_styled(&format!("  ✓ хук {filename} удалён"), success_style());
    } else {
        println_styled(&format!("  • хук {filename} не найден"), warn_style());
    }
    Ok(())
}

/// Запускает тело хука прямо сейчас (без коммита) — удобно для проверки.
async fn run_hook_inline(name: &str) -> DmResult<()> {
    match name {
        "pre-commit" => {
            print_system("pre-commit: format + lint");
            crate::commands::format::run().await?;
            crate::commands::lint::run(crate::commands::TargetArgs { name: None }).await
        }
        "pre-push" => {
            print_system("pre-push: test");
            crate::commands::test::run(crate::commands::TargetArgs { name: None }).await
        }
        other => Err(dm_core::DmError::invalid_config(format!(
            "неподдерживаемый хук '{other}'"
        ))),
    }
}

//! `dm test [svc]` — запуск тестов сервисов.
//!
//! Использует `tests.cmd` из конфига сервиса. Если команда не задана — пробует
//! дефолт для языка (cargo test / npm test / go test).

use crate::commands::{TargetArgs, load_project_config};
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use tokio::process::Command;

/// Точка входа команды.
pub async fn run(args: TargetArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;

    let targets: Vec<_> = match &args.name {
        Some(n) => {
            let svc = config
                .services
                .get(n)
                .ok_or_else(|| dm_core::DmError::ServiceNotFound(n.clone()))?;
            vec![(n.clone(), svc.clone())]
        }
        None => config
            .services
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    };

    for (name, svc) in targets {
        let cmd = match svc.tests.cmd.as_deref() {
            Some(c) => c.to_string(),
            None => match default_test_cmd(svc.language) {
                Some(c) => c.to_string(),
                None => {
                    println_styled(
                        &format!("пропускаю '{name}': команда тестов не настроена"),
                        warn_style(),
                    );
                    continue;
                }
            },
        };
        print_system(&format!("тесты '{name}': {cmd}"));
        let status = run_shell(&cmd, &root.join(&svc.path)).await;
        match status {
            Ok(0) => println_styled(&format!("  ✓ {name} — тесты прошли"), success_style()),
            Ok(code) => println_styled(
                &format!("  ✗ {name} — тесты упали (код {code})"),
                warn_style(),
            ),
            Err(e) => println_styled(
                &format!("  ✗ {name} — ошибка запуска: {e}"),
                crate::output::error_style(),
            ),
        }
    }
    Ok(())
}

/// Запускает команду в shell текущей платформы в каталоге `cwd`.
async fn run_shell(cmd: &str, cwd: &std::path::Path) -> Result<i32, std::io::Error> {
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd]);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    };
    command.current_dir(cwd);
    command.stdin(std::process::Stdio::null());
    let status = command.status().await?;
    Ok(status.code().unwrap_or(-1))
}

/// Дефолтная команда тестов по языку.
fn default_test_cmd(lang: ServiceLanguage) -> Option<&'static str> {
    match lang {
        ServiceLanguage::Rust => Some("cargo test"),
        ServiceLanguage::Go => Some("go test ./..."),
        ServiceLanguage::Python => Some("pytest"),
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Nodejs
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix => Some("npm test"),
        ServiceLanguage::Bun => Some("bun test"),
        _ => None,
    }
}

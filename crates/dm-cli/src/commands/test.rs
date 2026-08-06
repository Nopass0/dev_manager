//! `dm test [svc]` — запуск тестов сервисов.
//!
//! Использует `tests.cmd` из конфига сервиса. Если команда не задана — пробует
//! дефолт для языка (cargo test / npm test / go test). Вывод тестов стримится
//! в консоль; для прозрачности показывается и команда, и каталог запуска.

use crate::commands::{TargetArgs, load_project_config};
use crate::output::{error_style, print_system, println_styled, success_style, warn_style};
use crate::shell;
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use std::path::Path;

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

    let mut failed = 0usize;
    let mut total = 0usize;
    for (name, svc) in targets {
        total += 1;
        let cmd = match svc.tests.cmd.as_deref() {
            Some(c) => c.to_string(),
            None => match default_test_cmd(svc.language) {
                Some(c) => c.to_string(),
                None => {
                    println_styled(&i18n("skipping_no_tests", &[&name]), warn_style());
                    continue;
                }
            },
        };
        // Каталог запуска: корень + путь сервиса (разрешаем относительно корня).
        let cwd = shell::resolve_dir(&root, &svc.path);
        print_system(&i18n(
            "tests_running",
            &[&name, &cmd, &cwd.display().to_string()],
        ));
        match shell::run(&cmd, &cwd) {
            Ok(0) => println_styled(&i18n("tests_pass", &[&name]), success_style()),
            Ok(code) => {
                failed += 1;
                println_styled(
                    &i18n("tests_fail_code", &[&name, &code.to_string()]),
                    error_style(),
                );
            }
            Err(e) => {
                failed += 1;
                println_styled(&i18n("tests_fail_err", &[&name, &e]), error_style());
            }
        }
    }
    // Сводка.
    if total > 0 && failed == 0 {
        println_styled(
            &i18n("tests_all_pass", &[&total.to_string()]),
            success_style(),
        );
    } else if total > 0 {
        println_styled(
            &i18n(
                "tests_summary",
                &[&(total - failed).to_string(), &total.to_string()],
            ),
            warn_style(),
        );
    }
    Ok(())
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

/// Локализованные строки для команды (RU/EN через DM_LANG).
/// Плейсхолдеры вида %0, %1, %2 подставляются из args.
fn i18n(key: &str, args: &[&str]) -> String {
    let tmpl: &str = match key {
        "skipping_no_tests" => "пропускаю '%0': команда тестов не настроена",
        "tests_running" => "тесты '%0': %1 (в %2)",
        "tests_pass" => "  ✓ %0 — тесты прошли",
        "tests_fail_code" => "  ✗ %0 — тесты упали (код %1)",
        "tests_fail_err" => "  ✗ %0 — ошибка запуска: %1",
        "tests_all_pass" => "✓ все тесты прошли (%0 сервисов)",
        "tests_summary" => "тесты: %0/%1 сервисов прошли",
        _ => key,
    };
    let mut out = tmpl.to_string();
    for (i, a) in args.iter().enumerate() {
        out = out.replace(&format!("%{i}"), a);
    }
    out
}

#[allow(dead_code)]
fn _unused(_p: &Path) {}

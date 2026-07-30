//! `dm build [svc] [--release]` — унифицированная сборка всех сервисов.
//!
//! По языку вызывает системный билдер: cargo build / npm run build / go build /
//! tsc / dotnet build. `--release` включает оптимизацию где применимо.

use crate::commands::{BuildArgs, load_project_config};
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use std::path::Path;
use std::process::Command;

/// Точка входа команды.
pub async fn run(args: BuildArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let targets: Vec<_> = match &args.service {
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
        let dir = dm_core::paths::resolve(&root, Path::new(&svc.path));
        let Some(cmd) = build_cmd(svc.language, args.release) else {
            println_styled(
                &format!("  ! {name}: сборка для языка не настроена"),
                warn_style(),
            );
            continue;
        };
        print_system(&format!("сборка '{name}': {cmd}"));
        match run_shell(&cmd, &dir) {
            Ok(0) => println_styled(&format!("  ✓ {name} собран"), success_style()),
            Ok(code) => println_styled(
                &format!("  ✗ {name}: билдер вернул код {code}"),
                crate::output::error_style(),
            ),
            Err(e) => println_styled(&format!("  ✗ {name}: {e}"), crate::output::error_style()),
        }
    }
    Ok(())
}

/// Команда сборки по языку.
fn build_cmd(lang: ServiceLanguage, release: bool) -> Option<String> {
    Some(match lang {
        ServiceLanguage::Rust => {
            if release {
                "cargo build --release".into()
            } else {
                "cargo build".into()
            }
        }
        ServiceLanguage::Go => {
            if release {
                "go build -ldflags='-s -w' ./...".into()
            } else {
                "go build ./...".into()
            }
        }
        ServiceLanguage::C => "cc *.c -o app".into(),
        ServiceLanguage::Cpp => "c++ *.cpp -o app".into(),
        ServiceLanguage::Csharp => "dotnet build".into(),
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Nodejs
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix => "npm run build".into(),
        ServiceLanguage::Bun => "bun run build".into(),
        ServiceLanguage::Python => "python -m compileall .".into(),
        _ => return None,
    })
}

/// Запуск shell-команды синхронно.
fn run_shell(cmd: &str, cwd: &Path) -> Result<i32, String> {
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
    let status = command.status().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

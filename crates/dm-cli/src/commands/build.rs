//! `dm build [svc] [--release]` — унифицированная сборка и сборочный пайплайн.
//!
//! Два режима:
//! 1. Простой (`dm build [svc]`): сборка одного/всех сервисов командой по языку.
//! 2. Пайплайн (`dm build` при наличии секции `build.stages`): упорядоченные
//!    этапы собирают артефакты (DLL, exe, бинарники) из разных языков в единую
//!    папку `build.output_dir`. Позволяет собрать C++ DLL и Rust приложение в
//!    один чистый `dist/`.

use crate::commands::{BuildArgs, load_project_config};
use crate::output::{error_style, print_system, println_styled, success_style, warn_style};
use crate::shell;
use dm_core::DmResult;
use dm_core::project::ServiceLanguage;
use std::path::Path;

pub async fn run(args: BuildArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    if !config.build.stages.is_empty() && args.service.is_none() {
        return run_pipeline(&config, &root).await;
    }
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
        let dir = shell::resolve_dir(&root, &svc.path);
        let Some(cmd) = build_cmd(svc.language, args.release) else {
            println_styled(
                &format!("  ! {name}: сборка для языка не настроена"),
                warn_style(),
            );
            continue;
        };
        print_system(&format!("сборка '{name}': {cmd}"));
        match shell::run(&cmd, &dir) {
            Ok(0) => println_styled(&format!("  ✓ {name} собран"), success_style()),
            Ok(code) => println_styled(
                &format!("  ✗ {name}: билдер вернул код {code}"),
                error_style(),
            ),
            Err(e) => println_styled(&format!("  ✗ {name}: {e}"), error_style()),
        }
    }
    Ok(())
}

async fn run_pipeline(config: &dm_core::Config, root: &Path) -> DmResult<()> {
    let output = shell::resolve_dir(root, &config.build.output_dir);
    print_system(&format!(
        "сборочный пайплайн ({} этапов → {})",
        config.build.stages.len(),
        output.display()
    ));
    if config.build.clean {
        if output.exists() {
            print_system(&format!("  очистка {}", output.display()));
            let _ = std::fs::remove_dir_all(&output);
        }
        std::fs::create_dir_all(&output)?;
    }
    for (i, stage) in config.build.stages.iter().enumerate() {
        let label = if stage.name.is_empty() {
            format!("этап {}", i + 1)
        } else {
            stage.name.clone()
        };
        print_system(&format!(
            "  [{}/{}] {}",
            i + 1,
            config.build.stages.len(),
            label
        ));
        let src_dir = if let Some(svc) = config.services.get(&stage.source) {
            shell::resolve_dir(root, &svc.path)
        } else {
            shell::resolve_dir(root, &stage.source)
        };
        if !stage.command.is_empty() {
            print_system(&format!("    ▸ {}", stage.command));
            match shell::run(&stage.command, &src_dir) {
                Ok(0) => {}
                Ok(code) => {
                    println_styled(&format!("    ✗ команда вернула код {code}"), error_style());
                    return Err(dm_core::DmError::Process(
                        "сборочный пайплайн завершился с ошибкой".to_string(),
                    ));
                }
                Err(e) => {
                    println_styled(&format!("    ✗ ошибка: {e}"), error_style());
                    return Err(dm_core::DmError::Process(format!("сборочный этап: {e}")));
                }
            }
        }
        let dest = if stage.dest_subdir.is_empty() {
            output.clone()
        } else {
            output.join(&stage.dest_subdir)
        };
        std::fs::create_dir_all(&dest).ok();
        for pattern in &stage.artifacts {
            let copied = copy_artifacts(&src_dir, pattern, &dest);
            if copied > 0 {
                println_styled(
                    &format!(
                        "    ✓ скопировано артефактов: {copied} → {}",
                        dest.display()
                    ),
                    success_style(),
                );
            }
        }
    }
    println_styled(
        &format!("✓ сборка завершена, артефакты в {}", output.display()),
        success_style(),
    );
    Ok(())
}

fn copy_artifacts(base: &Path, pattern: &str, dest: &Path) -> usize {
    let (dir_part, glob_part) = match pattern.rsplit_once('/') {
        Some((d, g)) => (base.join(d), g),
        None => (base.to_path_buf(), pattern),
    };
    let Ok(entries) = std::fs::read_dir(&dir_part) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if glob_matches(glob_part, name_str) {
            let src = entry.path();
            let dst = dest.join(&name);
            if std::fs::copy(&src, &dst).is_ok() {
                count += 1;
            }
        }
    }
    count
}

fn glob_matches(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_inner(&p, &n)
}

fn glob_inner(p: &[char], n: &[char]) -> bool {
    match (p.first(), n.first()) {
        (None, None) => true,
        (Some('*'), _) => {
            if glob_inner(&p[1..], n) {
                return true;
            }
            if !n.is_empty() {
                return glob_inner(p, &n[1..]);
            }
            false
        }
        (Some('?'), Some(_)) => glob_inner(&p[1..], &n[1..]),
        (Some(&pc), Some(&nc)) if pc == nc => glob_inner(&p[1..], &n[1..]),
        _ => false,
    }
}

fn build_cmd(lang: ServiceLanguage, release: bool) -> Option<String> {
    Some(match lang {
        ServiceLanguage::Rust => if release {
            "cargo build --release"
        } else {
            "cargo build"
        }
        .to_string(),
        ServiceLanguage::Go => if release {
            "go build -ldflags='-s -w' ./..."
        } else {
            "go build ./..."
        }
        .to_string(),
        ServiceLanguage::C => "cc *.c -o app".to_string(),
        ServiceLanguage::Cpp => "c++ *.cpp -o app".to_string(),
        ServiceLanguage::Csharp => "dotnet build".to_string(),
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Nodejs
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix => "npm run build".to_string(),
        ServiceLanguage::Bun => "bun run build".to_string(),
        ServiceLanguage::Python => "python -m compileall .".to_string(),
        _ => return None,
    })
}

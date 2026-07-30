//! `dm watch [svc] -- <cmd>` — запускать команду при изменении файлов.
//!
//! Универсальный watcher-runner: при любом изменении файлов в каталоге сервиса
//! (или корне проекта) повторно выполняет `<cmd>`. Удобно для тестов/линтеров/
//! сборки без полного `dm start`.

use crate::commands::{WatchArgs, load_project_config};
use crate::output::{print_log_line, print_system};
use dm_core::DmResult;
use dm_core::project::Service;
use dm_runtime::logs::{LogLevel, LogLine};
use dm_runtime::watcher::FileWatcher;
use tokio::process::Command;
use tokio::sync::mpsc;

/// Точка входа команды.
pub async fn run(args: WatchArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let watch_dir = match &args.service {
        Some(name) => {
            let svc = config
                .services
                .get(name)
                .ok_or_else(|| dm_core::DmError::ServiceNotFound(name.clone()))?;
            dm_core::paths::resolve(&root, std::path::Path::new(&svc.path))
        }
        None => root.clone(),
    };

    if args.command.is_empty() {
        return Err(dm_core::DmError::invalid_config(
            "укажите команду: dm watch [svc] -- <cmd>",
        ));
    }

    // Создаём фиктивный Service, чтобы переиспользовать FileWatcher.
    let svc = Service {
        name: "watch".into(),
        path: watch_dir.clone(),
        language: dm_core::project::ServiceLanguage::Other,
        run_command: String::new(),
        watch: true,
        restart_on_change: true,
        repo_path: None,
        delay_ms: 0,
    };

    let (tx, mut rx) = mpsc::unbounded_channel();
    let _watcher = FileWatcher::spawn(vec![svc.clone()], tx)
        .map_err(|e| dm_core::DmError::Process(format!("watcher: {e}")))?;

    let cmd_str = args.command.join(" ");
    print_system(&format!("watch {}: {cmd_str}", watch_dir.display()));

    // Сразу один прогон.
    run_once(&args.command, &watch_dir).await;

    // Цикл реакций на изменения.
    while let Some(ev) = rx.recv().await {
        let files: Vec<String> = ev.paths.iter().map(|p| p.display().to_string()).collect();
        print_log_line(&LogLine::new(
            "watch".into(),
            LogLevel::System,
            format!("изменения: {}", files.join(", ")),
        ));
        run_once(&args.command, &watch_dir).await;
    }
    Ok(())
}

/// Выполняет команду один раз в `cwd`.
async fn run_once(argv: &[String], cwd: &std::path::Path) {
    let mut cmd = Command::new(&argv[0]);
    for a in &argv[1..] {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    match cmd.status().await {
        Ok(status) => {
            print_log_line(&LogLine::new(
                "watch".into(),
                if status.success() {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                },
                format!("завершено с кодом {}", status.code().unwrap_or(-1)),
            ));
        }
        Err(e) => {
            print_log_line(&LogLine::new(
                "watch".into(),
                LogLevel::Error,
                format!("ошибка запуска: {e}"),
            ));
        }
    }
}

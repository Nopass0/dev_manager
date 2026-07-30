//! `dm start` — запуск всех сервисов с горячей перезагрузкой.
//!
//! Поддержка флагов фильтрации: `--only/--skip/--tag/--profile/--affected`.
//! Affected вычисляется через граф зависимостей `dm-analysis` по `git diff`.
//! `--dry-run` показывает план запуска без выполнения; `--wait` дожидается
//! health-check всех запускаемых сервисов.

use crate::commands::{load_project_config, PREFIX_SYS, StartArgs};
use crate::output::{print_log_line, print_system, println_styled, warn_style};
use crate::select::Selection;
use dm_core::DmResult;
use dm_runtime::supervisor::{project_from_config, Supervisor, SupervisorOptions};
use tokio::signal;
use tokio::sync::mpsc;

/// Точка входа команды.
pub async fn run(args: StartArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let all_in_order = config.services_in_start_order();

    // Affected: рассчитываем через граф зависимостей, если включён --affected.
    let affected = if args.affected {
        Some(compute_affected(&config, &root).await?)
    } else {
        None
    };

    let selection = Selection {
        only: args.only.clone(),
        skip: args.skip.clone(),
        tag: args.tag.clone(),
        profile: args.profile.clone(),
        affected,
    };
    let selected = selection.apply(&config, &all_in_order);

    if selected.is_empty() {
        println_styled("Нет сервисов для запуска по заданным фильтрам.", warn_style());
        return Ok(());
    }

    // Build a filtered project: только выбранные сервисы, в исходном порядке.
    let mut full_project = project_from_config(&config, &root)?;
    full_project.services.retain(|s| selected.contains(&s.name));

    print_system(&format!(
        "запуск проекта '{}' — {} сервис(ов)",
        full_project.name,
        full_project.services.len()
    ));
    for svc in &full_project.services {
        print_system(&format!(
            "  • {} [{}] → {}",
            svc.name,
            svc.language_label(),
            svc.run_command
        ));
    }

    if args.dry_run {
        print_system("это пробный запуск (--dry-run): процессы не запускаются.");
        return Ok(());
    }

    // Канал логов: все сервисы шлют сюда строки, фоновая задача печатает.
    let (log_tx, mut log_rx) = mpsc::unbounded_channel();
    let options = SupervisorOptions {
        no_watch: args.no_watch,
        no_restart: args.no_restart,
    };

    // Парсим notify-конфиг из секции notify: (serde_yaml::Mapping → NotifyConfig).
    let notify_cfg = parse_notify_config(&config.notify, &config.project_name);
    let supervisor = std::sync::Arc::new(Supervisor::with_notify(
        full_project.clone(),
        options,
        log_tx.clone(),
        notify_cfg,
    ));

    // before_start хуки для каждого сервиса (выполняются до запуска процесса).
    run_before_start_hooks(&config, &selected, &root).await;

    // Устанавливаем лимиты ресурсов для сервисов (для monitor'а).
    for name in &selected {
        if let Some(svc) = config.services.get(name) {
            if let Some(res) = &svc.resources {
                if res.memory_mb > 0 {
                    supervisor
                        .set_resource_limits(name, res.memory_mb, res.on_exceed)
                        .await;
                }
            }
        }
    }

    supervisor.start_all().await?;

    // Запускаем мониторинг ресурсов (RSS памяти) раз в 5 секунд.
    supervisor.start_resource_monitor(5);

    // Фоновая печать логов до сигнала остановки.
    let printer = tokio::spawn(async move {
        while let Some(line) = log_rx.recv().await {
            print_log_line(&line);
        }
    });

    // File-watcher: на изменение файла → supervisor.notify_file_changed().
    // Сервис дожидается в tokio::select! и корректно перезапускается.
    if !args.no_watch {
        let watchable: Vec<dm_core::project::Service> = full_project
            .services
            .iter()
            .filter(|s| s.watch)
            .cloned()
            .collect();
        let (wtx, mut wrx) = mpsc::unbounded_channel();
        let sv = supervisor.clone();
        match dm_runtime::watcher::FileWatcher::spawn(watchable, wtx) {
            Ok(_) => {
                tokio::spawn(async move {
                    while let Some(ev) = wrx.recv().await {
                        sv.notify_file_changed(&ev.service, &ev.paths).await;
                    }
                });
            }
            Err(e) => {
                let _ = log_tx.send(dm_runtime::logs::LogLine::new(
                    PREFIX_SYS.into(),
                    dm_runtime::logs::LogLevel::Error,
                    format!("watcher не запущен: {e}"),
                ));
            }
        }
    }

    // Опционально: дождаться health-check всех запускаемых сервисов.
    if args.wait {
        wait_for_health(&config, &selected, &root).await;
    }

    // Ждём Ctrl+C — затем корректно гасим всё дерево процессов.
    signal::ctrl_c().await.ok();
    print_system(&format!("{PREFIX_SYS} получен Ctrl+C, останавливаю сервисы…"));
    supervisor.shutdown().await;
    drop(log_tx);
    let _ = printer.await;
    print_system("все сервисы остановлены.");
    Ok(())
}

/// Разбирает секцию `notify:` (serde_yaml::Mapping) в `NotifyConfig`.
fn parse_notify_config(mapping: &serde_yaml::Mapping, project_name: &str) -> dm_runtime::NotifyConfig {
    let mut cfg = dm_runtime::NotifyConfig {
        project_name: Some(project_name.to_string()),
        ..Default::default()
    };
    if let Some(serde_yaml::Value::String(url)) = mapping.get("webhook_url") {
        cfg.webhook_url = Some(url.clone());
    }
    if let Some(serde_yaml::Value::Sequence(events)) = mapping.get("events") {
        cfg.events = events
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    cfg
}

/// Выполняет `before_start` команды сервисов до их запуска.
async fn run_before_start_hooks(
    config: &dm_core::Config,
    selected: &[String],
    root: &std::path::Path,
) {
    for name in selected {
        let Some(svc) = config.services.get(name) else { continue };
        if svc.before_start.is_empty() {
            continue;
        }
        let dir = dm_core::paths::resolve(root, std::path::Path::new(&svc.path));
        for cmd in &svc.before_start {
            print_system(&format!("  ▸ {name}: before_start → {cmd}"));
            let _ = run_shell_inline(cmd, &dir);
        }
    }
}

/// Запуск shell-команды синхронно (для хуков).
fn run_shell_inline(cmd: &str, cwd: &std::path::Path) -> Result<i32, String> {
    #[cfg(windows)]
    let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", cmd]);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", cmd]);
        c
    };
    command.current_dir(cwd);
    command.stdin(std::process::Stdio::null());
    let status = command.status().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

/// Рассчитывает список имён сервисов, затронутых изменениями в git.
///
/// Берёт `git diff` изменённых файлов, строит граф импортов проекта и через
/// [`dm_analysis::DependencyGraph::affected_services`] определяет, какие сервисы
/// нужно перезапустить.
async fn compute_affected(
    config: &dm_core::Config,
    root: &std::path::Path,
) -> DmResult<Vec<String>> {
    print_system("расчёт затронутых сервисов (--affected)…");
    // Корневой репозиторий = root; берём изменённые файлы из git status.
    let changed = match dm_vcs::diff::changed_file_paths(root).await {
        Ok(files) => files,
        Err(_) => {
            println_styled(
                "не удалось получить git diff — запускаю все сервисы.",
                warn_style(),
            );
            return Ok(config.services_in_start_order());
        }
    };
    // Абсолютные пути изменённых файлов.
    let changed_abs: Vec<std::path::PathBuf> =
        changed.iter().map(|f| root.join(f)).collect();

    let graph = dm_analysis::DependencyGraph::build(root);

    // Резолвим каталоги сервисов в абсолютные пути и заимствуем их.
    let resolved: Vec<(String, std::path::PathBuf)> = config
        .services
        .iter()
        .map(|(name, svc)| {
            (
                name.clone(),
                dm_core::paths::resolve(root, std::path::Path::new(&svc.path)),
            )
        })
        .collect();
    let dirs: Vec<(&str, &std::path::Path)> = resolved
        .iter()
        .map(|(n, p)| (n.as_str(), p.as_path()))
        .collect();
    let affected = graph.affected_services(&changed_abs, &dirs);

    if affected.is_empty() {
        println_styled("изменения не затрагивают ни одного сервиса.", warn_style());
    } else {
        print_system(&format!("затронуто: {}", affected.join(", ")));
    }
    Ok(affected)
}

/// Дожидается прохождения health-check для каждого из выбранных сервисов.
///
/// Для каждого сервиса с `health:` конфигом проверяет URL/TCP-порт. Сервисы без
/// health-конфига считаются «здоровыми» сразу после запуска.
async fn wait_for_health(config: &dm_core::Config, selected: &[String], root: &std::path::Path) {
    use dm_core::config::HealthCheckKind;
    print_system("ожидание health-check (--wait)…");
    for name in selected {
        let Some(svc) = config.services.get(name) else { continue };
        let Some(hc) = &svc.health else { continue };
        let warmup = std::time::Duration::from_secs(hc.warmup_secs);
        let interval = std::time::Duration::from_secs(hc.interval_secs);
        tokio::time::sleep(warmup).await;
        let mut ok = false;
        for _ in 0..hc.retries {
            let healthy = match hc.kind {
                HealthCheckKind::None => true,
                HealthCheckKind::Tcp => match hc.port {
                    Some(port) => tcp_check(port).await,
                    None => true,
                },
                HealthCheckKind::Http => match &hc.url {
                    Some(url) => http_check(url).await,
                    None => true,
                },
            };
            if healthy {
                ok = true;
                break;
            }
            tokio::time::sleep(interval).await;
        }
        if ok {
            print_system(&format!("  ✓ {name} healthy"));
        } else {
            print_system(&format!("  ✗ {name} не прошёл health-check"));
        }
    }
    let _ = root;
}

/// Проверяет, открыт ли TCP-порт (на localhost).
async fn tcp_check(port: u16) -> bool {
    tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_ok()
}

/// Делает HTTP GET и считает здоровым любой 2xx-ответ.
async fn http_check(url: &str) -> bool {
    // Минимальный HTTP-клиент без внешних зависимостей.
    let parsed = match parse_url(url) {
        Some(u) => u,
        None => return false,
    };
    let mut stream = match tokio::net::TcpStream::connect((parsed.host.as_str(), parsed.port)).await {
        Ok(s) => s,
        Err(_) => return false,
    };
    use tokio::io::AsyncWriteExt;
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        parsed.path, parsed.host
    );
    if stream.write_all(req.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    use tokio::io::AsyncReadExt;
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return false;
    }
    let head = String::from_utf8_lossy(&buf[..n]);
    // HTTP/1.1 200 OK → статус 2xx.
    head.lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .map(|code| code.starts_with('2'))
        .unwrap_or(false)
}

/// Минимальный URL-парсер для health-check (без внешних зависимостей).
fn parse_url(url: &str) -> Option<ParsedUrl> {
    let (scheme, rest) = url.split_once("://")?;
    let default_port = match scheme {
        "https" => 443,
        _ => 80,
    };
    let (host_port, path) = match rest.split_once('/') {
        Some((hp, p)) => (hp.to_string(), format!("/{p}")),
        None => (rest.to_string(), "/".to_string()),
    };
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(default_port)),
        None => (host_port, default_port),
    };
    Some(ParsedUrl { host, port, path })
}

struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
}

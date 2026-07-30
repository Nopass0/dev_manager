//! `dm db migrate|seed|reset|shell` — работа с БД (config-driven).
//!
//! Берёт подключение из секции `database.connections:` и выполняет команды:
//! - `migrate` → `migrate_cmd` (или угаданный по `kind`);
//! - `seed` → `seed_cmd`;
//! - `reset` → drop + migrate + seed (если поддерживается);
//! - `shell` → интерактивный клиент (`psql`, `redis-cli`, `sqlite3`…).

use crate::commands::{DbAction, DbArgs, load_project_config};
use crate::output::{print_system, println_styled, success_style, warn_style};
use dm_core::DmResult;
use std::process::Command;

/// Точка входа команды.
pub async fn run(args: DbArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let conn = pick_connection(&config, args.conn.as_deref())?;

    match args.action {
        DbAction::Migrate => run_migrate(&conn, &root).await,
        DbAction::Seed => run_seed(&conn, &root).await,
        DbAction::Reset => run_reset(&conn, &root).await,
        DbAction::Shell => run_shell(&conn, &root).await,
    }
}

/// Выбирает подключение по имени или первое доступное.
fn pick_connection(
    config: &dm_core::Config,
    name: Option<&str>,
) -> DmResult<dm_core::config::DatabaseConnection> {
    if config.database.connections.is_empty() {
        return Err(dm_core::DmError::invalid_config(
            "секция `database.connections` пуста — БД не настроена.",
        ));
    }
    match name {
        Some(n) => config.database.connections.get(n).cloned().ok_or_else(|| {
            dm_core::DmError::invalid_config(format!(
                "подключение '{n}' не найдено в database.connections"
            ))
        }),
        None => {
            // Приоритет: default → api → первое.
            config
                .database
                .connections
                .get("default")
                .or_else(|| config.database.connections.get("api"))
                .or_else(|| config.database.connections.values().next())
                .cloned()
                .ok_or_else(|| dm_core::DmError::invalid_config("нет ни одного подключения к БД"))
        }
    }
}

/// Применяет миграции.
async fn run_migrate(
    conn: &dm_core::config::DatabaseConnection,
    root: &std::path::Path,
) -> DmResult<()> {
    let cmd = conn
        .migrate_cmd
        .clone()
        .unwrap_or_else(|| default_migrate_cmd(&conn.kind, conn));
    print_system(&format!("миграции: {cmd}"));
    run_shell_cmd(&cmd, root)
}

/// Накатывает seed-данные.
async fn run_seed(
    conn: &dm_core::config::DatabaseConnection,
    root: &std::path::Path,
) -> DmResult<()> {
    let cmd = match &conn.seed_cmd {
        Some(c) => c.clone(),
        None => {
            println_styled(
                "команда seed не настроена (database.<conn>.seed_cmd)",
                warn_style(),
            );
            return Ok(());
        }
    };
    print_system(&format!("seed: {cmd}"));
    run_shell_cmd(&cmd, root)
}

/// Reset: drop + migrate + seed.
async fn run_reset(
    conn: &dm_core::config::DatabaseConnection,
    root: &std::path::Path,
) -> DmResult<()> {
    print_system("reset: пересоздание схемы (drop → migrate → seed)");
    // Простой подход для postgres: dropdb + createdb, потом migrate.
    let reset_cmd = match conn.kind.as_str() {
        "postgres" | "postgresql" => Some(format!(
            "dropdb --if-exists '{}' && createdb '{}'",
            dbname_from_url(&conn.url),
            dbname_from_url(&conn.url)
        )),
        "sqlite" => Some(format!("rm -f '{}'", conn.url)),
        _ => None,
    };
    if let Some(rc) = reset_cmd {
        let _ = run_shell_cmd(&rc, root);
    }
    run_migrate(conn, root).await?;
    run_seed(conn, root).await?;
    println_styled("✓ reset завершён", success_style());
    Ok(())
}

/// Открывает интерактивный клиент БД.
async fn run_shell(
    conn: &dm_core::config::DatabaseConnection,
    _root: &std::path::Path,
) -> DmResult<()> {
    let (program, args) = shell_client(conn)?;
    print_system(&format!("db shell: {program} {}", args.join(" ")));
    let status = Command::new(&program)
        .args(&args)
        .status()
        .map_err(|e| dm_core::DmError::Process(format!("{program}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(dm_core::DmError::Process(format!(
            "{program} завершился с кодом {}",
            status.code().unwrap_or(-1)
        )))
    }
}

/// Возвращает (программа, аргументы) интерактивного клиента по типу БД.
fn shell_client(conn: &dm_core::config::DatabaseConnection) -> DmResult<(String, Vec<String>)> {
    Ok(match conn.kind.as_str() {
        "postgres" | "postgresql" => ("psql".into(), vec![conn.url.clone()]),
        "mysql" => ("mysql".into(), vec![conn.url.clone()]),
        "sqlite" => ("sqlite3".into(), vec![conn.url.clone()]),
        "redis" => ("redis-cli".into(), vec![conn.url.clone()]),
        "mongo" | "mongodb" => ("mongosh".into(), vec![conn.url.clone()]),
        other => {
            return Err(dm_core::DmError::invalid_config(format!(
                "неизвестный тип БД '{other}' для shell"
            )));
        }
    })
}

/// Дефолтная команда миграций по типу БД.
fn default_migrate_cmd(kind: &str, conn: &dm_core::config::DatabaseConnection) -> String {
    match kind {
        "postgres" | "postgresql" => {
            let dir = conn
                .migrations_dir
                .clone()
                .unwrap_or_else(|| "migrations".into());
            format!(
                "psql '{}' -f {dir}/up.sql || for f in {dir}/*.up.sql; do psql '{}' -f \"$f\"; done",
                conn.url, conn.url
            )
        }
        "sqlite" => {
            let dir = conn
                .migrations_dir
                .clone()
                .unwrap_or_else(|| "migrations".into());
            format!(
                "for f in {dir}/*.sql; do sqlite3 '{}' < \"$f\"; done",
                conn.url
            )
        }
        _ => format!("echo 'настройте migrate_cmd для типа {kind}'"),
    }
}

/// Извлекает имя БД из postgres URL.
fn dbname_from_url(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

/// Запускает shell-команду в `cwd` синхронно.
fn run_shell_cmd(cmd: &str, cwd: &std::path::Path) -> DmResult<()> {
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
    let status = command
        .status()
        .map_err(|e| dm_core::DmError::Process(format!("shell: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(dm_core::DmError::ExternalCommand {
            command: cmd.into(),
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        })
    }
}

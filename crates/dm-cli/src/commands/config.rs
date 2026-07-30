//! `dm config list|get|edit|validate` — управление dm.yaml из CLI.

use crate::commands::{ConfigAction, ConfigArgs};
use crate::output::{print_system, println_styled, success_style};
use dm_core::DmResult;
use dm_core::config::{discover_config, load_config};
use std::env;

// Переэкспорт для удобства внутри файла.

/// Точка входа команды.
pub async fn run(args: ConfigArgs) -> DmResult<()> {
    match args.action {
        ConfigAction::List => {
            let path = discover_config(&env::current_dir()?)?;
            let content = std::fs::read_to_string(&path)?;
            println!("{content}");
            Ok(())
        }
        ConfigAction::Get { key } => {
            let path = discover_config(&env::current_dir()?)?;
            let raw = std::fs::read_to_string(&path)?;
            if let Some(value) = get_yaml_value(&raw, &key) {
                println_styled(&value, success_style());
            } else {
                print_system(&format!("ключ '{key}' не найден."));
            }
            Ok(())
        }
        ConfigAction::Edit => {
            let path = discover_config(&env::current_dir()?)?;
            let editor = env::var("EDITOR").unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            });
            let status = std::process::Command::new(&editor)
                .arg(&path)
                .status()
                .map_err(|e| dm_core::DmError::Process(format!("редактор {editor}: {e}")))?;
            if !status.success() {
                return Err(dm_core::DmError::Process(format!(
                    "редактор завершился с кодом {}",
                    status.code().unwrap_or(-1)
                )));
            }
            Ok(())
        }
        ConfigAction::Validate => {
            let path = discover_config(&env::current_dir()?)?;
            let mut cfg = load_config(&path)?;
            cfg.validate()?;
            println_styled("✓ конфигурация корректна", success_style());
            println_styled(
                &format!("  сервисов: {}", cfg.services.len()),
                crate::output::dim_style(),
            );
            println_styled(
                &format!("  профилей: {}", cfg.profiles.len()),
                crate::output::dim_style(),
            );
            Ok(())
        }
    }
}

/// Извлекает значение по dotted-key пути из YAML-текста (минимальный парсер).
///
/// Поддерживает верхнеуровневые скаляры и простые пути вида `services.api.path`.
/// Глубокая навигация по картам делается построчно — без полной YAML-библиотеки
/// здесь (config crate её и так использует).
fn get_yaml_value(raw: &str, key: &str) -> Option<String> {
    // Простой подход: ищем строку вида `<last_segment>:` на нужном уровне отступа.
    // Для MVP поддерживаем только верхний уровень и `services.<name>.<field>`.
    let segments: Vec<&str> = key.split('.').collect();
    if segments.len() == 1 {
        return find_top_level_scalar(raw, segments[0]);
    }
    if segments.len() == 3 && segments[0] == "services" {
        return find_service_field(raw, segments[1], segments[2]);
    }
    None
}

/// Ищет скаляр верхнего уровня: `key: value`.
fn find_top_level_scalar(raw: &str, key: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Ищет поле сервиса: `services:\n  <svc>:\n    <field>: value`.
fn find_service_field(raw: &str, svc: &str, field: &str) -> Option<String> {
    let mut in_services = false;
    let mut in_svc = false;
    for line in raw.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if trimmed == "services:" {
            in_services = true;
            continue;
        }
        if !in_services {
            continue;
        }
        if indent == 0 {
            break; // вышли из services
        }
        if indent == 2 && trimmed.starts_with(&format!("{svc}:")) {
            in_svc = true;
            continue;
        }
        if indent == 2 {
            in_svc = false;
        }
        if in_svc
            && indent == 4
            && let Some(rest) = trimmed.strip_prefix(&format!("{field}:"))
        {
            return Some(rest.trim().to_string());
        }
    }
    None
}

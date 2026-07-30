//! `dm list <what>` — перечислить сущности проекта.
//!
//! Поддерживаемые `what`: `services` (по умолчанию), `profiles`, `tags`,
//! `deploy`, `databases`. Выводит компактные таблицы для быстрого обзора.

use crate::commands::load_project_config;
use crate::output::{dim_style, print_system, println_styled};
use comfy_table::{ContentArrangement, Table};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(what: &str) -> DmResult<()> {
    let (config, _root) = load_project_config()?;
    match what {
        "services" | "svc" => list_services(&config),
        "profiles" => list_profiles(&config),
        "tags" => list_tags(&config),
        "deploy" | "deploys" => list_deploys(&config),
        "databases" | "db" => list_databases(&config),
        other => Err(dm_core::DmError::invalid_config(format!(
            "неизвестный объект '{other}'. Доступно: services | profiles | tags | deploy | databases."
        ))),
    }
}

/// Список сервисов с ключевыми атрибутами.
fn list_services(config: &dm_core::Config) -> DmResult<()> {
    let mut t = Table::new();
    t.load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Сервис", "Язык", "Путь", "Теги", "Зависит от"]);
    for (name, svc) in &config.services {
        t.add_row(vec![
            name.clone(),
            svc.language.label().to_string(),
            svc.path.clone(),
            svc.tags.join(","),
            svc.depends_on.join(","),
        ]);
    }
    println!("{t}");
    println_styled(&format!("всего сервисов: {}", config.services.len()), dim_style());
    Ok(())
}

/// Список профилей запуска.
fn list_profiles(config: &dm_core::Config) -> DmResult<()> {
    if config.profiles.is_empty() {
        println_styled("профили не настроены", dim_style());
        return Ok(());
    }
    let mut t = Table::new();
    t.load_preset(comfy_table::presets::UTF8_FULL)
        .set_header(vec!["Профиль", "Сервисы", "Описание"]);
    for (name, p) in &config.profiles {
        t.add_row(vec![
            name.clone(),
            p.services.join(", "),
            p.description.clone(),
        ]);
    }
    println!("{t}");
    Ok(())
}

/// Список уникальных тегов и сервисов под ними.
fn list_tags(config: &dm_core::Config) -> DmResult<()> {
    let mut by_tag: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, svc) in &config.services {
        for tag in &svc.tags {
            by_tag.entry(tag.clone()).or_default().push(name.clone());
        }
    }
    if by_tag.is_empty() {
        println_styled("теги не заданы", dim_style());
        return Ok(());
    }
    let mut t = Table::new();
    t.load_preset(comfy_table::presets::UTF8_FULL)
        .set_header(vec!["Тег", "Сервисы"]);
    for (tag, svcs) in by_tag {
        t.add_row(vec![tag, svcs.join(", ")]);
    }
    println!("{t}");
    Ok(())
}

/// Список целей деплоя.
fn list_deploys(config: &dm_core::Config) -> DmResult<()> {
    if config.deploy.is_empty() {
        println_styled("цели деплоя не настроены", dim_style());
        return Ok(());
    }
    let mut t = Table::new();
    t.load_preset(comfy_table::presets::UTF8_FULL)
        .set_header(vec!["Цель", "Хост", "Триггер", "Шагов"]);
    for d in &config.deploy {
        t.add_row(vec![
            d.name.clone(),
            format!("{}@{}:{}", d.user, d.host, d.port),
            format!("{:?}", d.on),
            d.steps.len().to_string(),
        ]);
    }
    println!("{t}");
    Ok(())
}

/// Список подключений к БД.
fn list_databases(config: &dm_core::Config) -> DmResult<()> {
    if config.database.connections.is_empty() {
        println_styled("подключения к БД не настроены", dim_style());
        return Ok(());
    }
    let mut t = Table::new();
    t.load_preset(comfy_table::presets::UTF8_FULL)
        .set_header(vec!["Имя", "Тип", "URL"]);
    for (name, conn) in &config.database.connections {
        t.add_row(vec![name.clone(), conn.kind.clone(), conn.url.clone()]);
    }
    println!("{t}");
    let _ = print_system;
    Ok(())
}

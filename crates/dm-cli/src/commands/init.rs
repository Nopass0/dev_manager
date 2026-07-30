//! `dm init [--template=<name>] [--list-templates] [--name=<name>]`.
//!
//! Без аргументов: создаёт `dm.yaml` из шаблона.
//! С `--template`: создаёт проект/сервис по встроенному шаблону (bun-elysia,
//! go-api, rust-axum, next-shadcn и др.) с рабочим эндпоинтом и автозаписью
//! в dm.yaml.

use crate::commands::{InitArgs, PREFIX_SYS};
use crate::output::{info_style, println_styled, success_style};
use crate::templates;
use dm_core::DmResult;
use dm_core::config::CONFIG_FILENAME;

/// Шаблон конфига (dm.yaml), который создаст `dm init` без --template.
const TEMPLATE: &str = include_str!("../../../../dm.example.yaml");

/// Точка входа команды.
pub async fn run(args: InitArgs) -> DmResult<()> {
    // --list-templates: показать список и выйти.
    if args.list_templates {
        println_styled("Доступные шаблоны проектов:", success_style());
        for t in templates::all_templates() {
            println_styled(&format!("  {:<18} {}", t.name, t.description), info_style());
        }
        return Ok(());
    }

    // --template: создать проект по шаблону.
    if let Some(template_name) = &args.template {
        return create_from_template(template_name, args.name.as_deref()).await;
    }

    // Иначе: только dm.yaml (как раньше).
    let cwd = std::env::current_dir()?;
    let target = cwd.join(CONFIG_FILENAME);
    if target.exists() {
        println_styled(
            &format!("{} уже существует — пропускаем.", CONFIG_FILENAME),
            crate::output::warn_style(),
        );
        return Ok(());
    }
    std::fs::write(&target, TEMPLATE)?;
    println_styled(
        &format!("{PREFIX_SYS} создан {CONFIG_FILENAME} в {}", cwd.display()),
        success_style(),
    );
    println_styled(
        "Отредактируйте его под свой проект, затем запустите `dm start`.",
        info_style(),
    );
    println_styled(
        "Совет: создайте проект из шаблона — `dm init --template=bun-elysia`.",
        crate::output::dim_style(),
    );
    Ok(())
}

/// Создаёт проект/сервис по шаблону в текущем каталоге.
async fn create_from_template(template_name: &str, name: Option<&str>) -> DmResult<()> {
    let template = templates::find(template_name).ok_or_else(|| {
        let available: Vec<&str> = templates::all_templates()
            .iter()
            .map(|t| t.name)
            .collect();
        dm_core::DmError::invalid_config(format!(
            "шаблон '{template_name}' не найден. Доступно: [{}]. Список: `dm init --list-templates`.",
            available.join(", ")
        ))
    })?;

    let cwd = std::env::current_dir()?;
    let project_name = name
        .map(|s| s.to_string())
        .or_else(|| {
            cwd.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "app".to_string());

    let created = templates::apply(&template, &cwd, &project_name)?;
    println_styled(
        &format!(
            "{PREFIX_SYS} создан проект '{project_name}' [{}] из шаблона '{}'",
            template.language, template.name
        ),
        success_style(),
    );
    for f in &created {
        println_styled(&format!("  ✓ {}", f.display()), crate::output::dim_style());
    }

    // Создаём/обновляем dm.yaml с записью сервиса.
    let svc_entry = build_service_yaml(&project_name, &template);
    upsert_dm_yaml(&cwd, &svc_entry)?;

    println_styled("Готово! Запуск:", success_style());
    println_styled("  dm setup   # установить зависимости", info_style());
    println_styled("  dm start   # запустить с hot-reload", info_style());
    Ok(())
}

/// Формирует YAML-фрагмент сервиса для записи в dm.yaml.
///
/// `path` — путь к каталогу сервиса относительно корня. Для `dm init` это `.`
/// (проект в текущем каталоге); для `dm new service <name>` — `./<name>`.
pub fn build_service_yaml_with_path(
    name: &str,
    path: &str,
    template: &templates::Template,
) -> String {
    let mut s = format!(
        "  {name}:\n    path: {path}\n    language: {}\n    run: \"{}\"\n",
        template.language, template.run_command
    );
    if let Some(tests) = template.test_command {
        s.push_str(&format!(
            "    tests:\n      cmd: {}\n      on_change: true\n",
            tests
        ));
    }
    s
}

/// Совместимость: `dm init` создаёт проект в текущем каталоге (path = `.`).
pub fn build_service_yaml(name: &str, template: &templates::Template) -> String {
    build_service_yaml_with_path(name, ".", template)
}

/// Создаёт или обновляет dm.yaml, добавляя сервис `svc_entry`.
pub fn upsert_dm_yaml(root: &std::path::Path, svc_entry: &str) -> DmResult<()> {
    let path = root.join(CONFIG_FILENAME);
    if !path.exists() {
        // Создаём минимальный dm.yaml с этим сервисом.
        let content = format!("version: 1\nproject_name: app\nservices:\n{svc_entry}");
        std::fs::write(&path, content)?;
        println_styled(&format!("  ✓ создан {CONFIG_FILENAME}"), success_style());
        return Ok(());
    }
    // Существующий dm.yaml: добавляем сервис в секцию services.
    let content = std::fs::read_to_string(&path)?;
    if content.contains("services:") {
        // Вставляем после строки "services:".
        let new = if let Some(idx) = content.find("services:") {
            let mut result = String::new();
            let after = &content[idx..];
            // найдём конец строки services:
            let line_end = after
                .find('\n')
                .map(|p| idx + p + 1)
                .unwrap_or(content.len());
            result.push_str(&content[..line_end]);
            result.push_str(svc_entry);
            result.push_str(&content[line_end..]);
            result
        } else {
            content.clone()
        };
        std::fs::write(&path, new)?;
    } else {
        let new = format!("{content}\nservices:\n{svc_entry}");
        std::fs::write(&path, new)?;
    }
    println_styled(
        &format!("  ✓ сервис добавлен в {CONFIG_FILENAME}"),
        success_style(),
    );
    Ok(())
}

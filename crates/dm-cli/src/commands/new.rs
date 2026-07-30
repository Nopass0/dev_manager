//! `dm new service <name> --lang=rust` — скаффолд нового сервиса.
//!
//! Создаёт минимальный сервис-скелет под выбранный язык в `./<name>/` и
//! добавляет запись в `dm.yaml`.

use crate::output::{print_system, success_style, println_styled};
use dm_core::config::discover_config;
use dm_core::project::ServiceLanguage;
use dm_core::DmResult;
use std::path::Path;

/// Точка входа команды.
pub async fn run(args: crate::commands::NewArgs) -> DmResult<()> {
    match args.kind.as_str() {
        "service" => run_service(args).await,
        "route" => run_file(&args, FileKind::Route).await,
        "component" => run_file(&args, FileKind::Component).await,
        "test" => run_file(&args, FileKind::Test).await,
        "migration" => run_migration(&args).await,
        other => Err(dm_core::DmError::invalid_config(format!(
            "неизвестный вид '{other}'. Доступно: service | route | component | test | migration."
        ))),
    }
}

/// Вид создаваемого файла (для шаблонных артефактов внутри сервиса).
enum FileKind {
    Route,
    Component,
    Test,
}

/// `dm new service <name> [--template=...] [--lang=...]` — скаффолд нового сервиса.
///
/// Приоритет: `--template` (полный шаблон с эндпоинтом) > `--lang` (минимальный скелет).
/// Сервис автоматически добавляется в dm.yaml (создаётся, если отсутствует).
async fn run_service(args: crate::commands::NewArgs) -> DmResult<()> {
    let cwd = std::env::current_dir()?;
    let target = cwd.join(&args.name);
    if target.exists() {
        return Err(dm_core::DmError::invalid_config(format!(
            "каталог '{}' уже существует", target.display()
        )));
    }
    std::fs::create_dir_all(&target)?;

    // Маршрут 1: --template (полный рабочий шаблон).
    if let Some(template_name) = &args.template {
        let template = crate::templates::find(template_name).ok_or_else(|| {
            let available: Vec<&str> = crate::templates::all_templates()
                .iter()
                .map(|t| t.name)
                .collect();
            dm_core::DmError::invalid_config(format!(
                "шаблон '{template_name}' не найден. Доступно: [{}].",
                available.join(", ")
            ))
        })?;
        let created = crate::templates::apply(&template, &target, &args.name)?;
        println_styled(
            &format!("  ✓ сервис '{}' [{}] из шаблона '{}'", args.name, template.language, template.name),
            success_style(),
        );
        for f in &created {
            println_styled(&format!("    {}", f.strip_prefix(&cwd).unwrap_or(f).display()), crate::output::dim_style());
        }
        // Добавляем в dm.yaml с правильным путём ./<name>.
        let svc_entry = crate::commands::init::build_service_yaml_with_path(
            &args.name,
            &format!("./{}", args.name),
            &template,
        );
        crate::commands::init::upsert_dm_yaml(&cwd, &svc_entry)?;
        return Ok(());
    }

    // Маршрут 2: --lang (минимальный скелет).
    let lang_str = args.lang.as_deref().unwrap_or("rust");
    let lang = parse_language(lang_str)
        .ok_or_else(|| dm_core::DmError::invalid_config(format!("неизвестный язык '{lang_str}'")))?;
    scaffold(&target, &args.name, lang)?;
    println_styled(&format!("  ✓ создан сервис '{}' [{}]", args.name, lang.label()), success_style());

    // Добавляем запись в dm.yaml.
    if let Ok(config_path) = discover_config(&cwd) {
        append_service_to_config(&config_path, &args.name, lang_str)?;
        println_styled(&format!("  ✓ запись добавлена в {}", config_path.display()), success_style());
    } else {
        print_system("dm.yaml не найден — создайте его через `dm init`.");
    }
    Ok(())
}

/// Парсит язык из строки CLI (без зависимостей от serde в этом crate).
fn parse_language(s: &str) -> Option<ServiceLanguage> {
    use ServiceLanguage::*;
    Some(match s.trim().to_lowercase().as_str() {
        "rust" => Rust,
        "go" => Go,
        "c" => C,
        "cpp" | "c++" => Cpp,
        "csharp" | "cs" => Csharp,
        "javascript" | "js" => JavaScript,
        "typescript" | "ts" => TypeScript,
        "bun" => Bun,
        "nodejs" | "node" => Nodejs,
        "lua" => Lua,
        "python" | "py" => Python,
        "vite" => Vite,
        "nextjs" => Nextjs,
        "remix" => Remix,
        "other" => Other,
        _ => return None,
    })
}

/// Создаёт файлы скелета сервиса под язык.
fn scaffold(dir: &Path, name: &str, lang: ServiceLanguage) -> DmResult<()> {
    match lang {
        ServiceLanguage::Rust => {
            std::fs::write(
                dir.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"),
            )?;
            std::fs::create_dir_all(dir.join("src"))?;
            std::fs::write(dir.join("src/main.rs"), "fn main() {\n    println!(\"hello from {name}\");\n}\n")?;
        }
        ServiceLanguage::Go => {
            std::fs::write(dir.join("go.mod"), format!("module {name}\n\ngo 1.22\n"))?;
            std::fs::write(dir.join("main.go"), "package main\n\nimport \"fmt\"\n\nfunc main() {\n    fmt.Println(\"hello\")\n}\n")?;
        }
        ServiceLanguage::TypeScript | ServiceLanguage::Vite | ServiceLanguage::Nextjs | ServiceLanguage::Remix => {
            std::fs::write(
                dir.join("package.json"),
                format!("{{\n  \"name\": \"{name}\",\n  \"version\": \"0.1.0\",\n  \"scripts\": {{ \"dev\": \"vite\" }}\n}}\n"),
            )?;
            std::fs::create_dir_all(dir.join("src"))?;
            std::fs::write(dir.join("src/index.ts"), "console.log('hello');\n")?;
        }
        ServiceLanguage::JavaScript | ServiceLanguage::Nodejs => {
            std::fs::write(dir.join("package.json"), format!("{{\n  \"name\": \"{name}\",\n  \"version\": \"0.1.0\"\n}}\n"))?;
            std::fs::write(dir.join("index.js"), "console.log('hello');\n")?;
        }
        ServiceLanguage::Python => {
            std::fs::write(dir.join("main.py"), "def main():\n    print('hello')\n\nif __name__ == '__main__':\n    main()\n")?;
        }
        _ => {
            std::fs::write(dir.join("README.md"), format!("# {name}\n"))?;
        }
    }
    Ok(())
}

/// Дописывает сервис в dm.yaml (простая текстовая вставка секции).
fn append_service_to_config(path: &Path, name: &str, lang: &str) -> DmResult<()> {
    let content = std::fs::read_to_string(path)?;
    let entry = format!(
        "  {name}:\n    path: ./{name}\n    language: {lang}\n"
    );
    // Если секция services уже есть — вставляем перед следующей верхнеуровневой
    // секцией; иначе создаём.
    let new = if let Some(idx) = content.find("services:") {
        // найдём конец блока services (следующая строка с отступом 0).
        let after = &content[idx..];
        let mut insert_at = idx + after.len();
        for (i, line) in after.lines().enumerate() {
            if i > 0 && !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#') {
                insert_at = idx + after.find(line).unwrap_or(after.len());
                break;
            }
        }
        let mut result = String::new();
        result.push_str(&content[..insert_at]);
        result.push_str(&entry);
        result.push_str(&content[insert_at..]);
        result
    } else {
        format!("{content}\nservices:\n{entry}")
    };
    std::fs::write(path, new)?;
    Ok(())
}

/// `dm new route|component|test <name> [--lang=]` — файл по шаблону в текущем каталоге.
///
/// Размещает файл в стандартной папке по языку: `src/routes/`, `src/components/`,
/// `tests/` (Rust), `src/` (TS/JS) и т.д.
async fn run_file(args: &crate::commands::NewArgs, kind: FileKind) -> DmResult<()> {
    let lang_str = args.lang.as_deref().unwrap_or("rust");
    let lang = parse_language(lang_str)
        .ok_or_else(|| dm_core::DmError::invalid_config(format!("неизвестный язык '{lang_str}'")))?;
    let cwd = std::env::current_dir()?;

    let (rel_dir, ext, content) = template_for(kind, &args.name, lang);
    let dir = cwd.join(&rel_dir);
    std::fs::create_dir_all(&dir)?;
    let filename = format!("{}.{}", args.name, ext);
    let path = dir.join(&filename);
    if path.exists() {
        return Err(dm_core::DmError::invalid_config(format!(
            "файл '{}' уже существует", path.display()
        )));
    }
    std::fs::write(&path, content)?;
    println_styled(&format!("  ✓ создан {}", path.display()), success_style());
    Ok(())
}

/// `dm new migration <name>` — SQL-миграция с timestamp-префиксом.
async fn run_migration(args: &crate::commands::NewArgs) -> DmResult<()> {
    let cwd = std::env::current_dir()?;
    let dir = cwd.join("migrations");
    std::fs::create_dir_all(&dir)?;
    // timestamp в формате YYYYMMDDHHMMSS (через системную утилиту или заглушку).
    let ts = migration_timestamp();
    let base = dir.join(format!("{ts}_{}", args.name));
    std::fs::write(format!("{}.up.sql", base.display()), "-- up\n")?;
    std::fs::write(format!("{}.down.sql", base.display()), "-- down\n")?;
    println_styled(
        &format!("  ✓ миграция {ts}_{} (up.sql + down.sql)", args.name),
        success_style(),
    );
    Ok(())
}

/// Возвращает (подкаталог, расширение, содержимое) для шаблонного файла.
fn template_for(
    kind: FileKind,
    name: &str,
    lang: ServiceLanguage,
) -> (&'static str, &'static str, String) {
    match (kind, lang) {
        (FileKind::Route, ServiceLanguage::Rust) => (
            "src/routes",
            "rs",
            format!(
                "use axum::routing::get;\n\npub async fn {name}() {{\n    // TODO\n}}\n"
            ),
        ),
        (FileKind::Route, ServiceLanguage::TypeScript)
        | (FileKind::Route, ServiceLanguage::JavaScript)
        | (FileKind::Route, ServiceLanguage::Vite)
        | (FileKind::Route, ServiceLanguage::Nextjs)
        | (FileKind::Route, ServiceLanguage::Remix) => (
            "src/routes",
            "ts",
            format!("export async function {name}() {{\n  // TODO\n}}\n"),
        ),
        (FileKind::Route, ServiceLanguage::Go) => (
            "internal/routes",
            "go",
            format!("package routes\n\nfunc {name}() {{\n    // TODO\n}}\n"),
        ),
        (FileKind::Component, ServiceLanguage::TypeScript)
        | (FileKind::Component, ServiceLanguage::Vite)
        | (FileKind::Component, ServiceLanguage::Nextjs)
        | (FileKind::Component, ServiceLanguage::Remix) => (
            "src/components",
            "tsx",
            format!(
                "export function {name}() {{\n  return (\n    <div>{name}</div>\n  );\n}}\n"
            ),
        ),
        (FileKind::Test, ServiceLanguage::Rust) => (
            "tests",
            "rs",
            format!("#[cfg(test)]\nmod tests {{\n    #[test]\n    fn {name}() {{\n        assert!(true);\n    }}\n}}\n"),
        ),
        (FileKind::Test, ServiceLanguage::TypeScript)
        | (FileKind::Test, ServiceLanguage::JavaScript)
        | (FileKind::Test, ServiceLanguage::Bun) => (
            "tests",
            "test.ts",
            format!("import {{ test, expect }} from 'bun:test';\n\ntest('{name}', () => {{\n  expect(1).toBe(1);\n}});\n"),
        ),
        (FileKind::Test, ServiceLanguage::Go) => (
            "",
            "go",
            format!("package main\n\nimport \"testing\"\n\nfunc Test{name}(t *testing.T) {{}}\n"),
        ),
        // Fallback: простой текстовый файл.
        _ => ("", "txt", format!("# {name}\n")),
    }
}

/// Timestamp для имени миграции (без внешних зависимостей от chrono).
fn migration_timestamp() -> String {
    // Используем системную `date` где доступно; иначе — millis-заглушка.
    #[cfg(unix)]
    {
        if let Ok(out) = std::process::Command::new("date").arg("+%Y%m%d%H%M%S").output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                return s.trim().to_string();
            }
        }
    }
    // Заглушка: секунды от старта процесса (однозначно в рамках сессии).
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

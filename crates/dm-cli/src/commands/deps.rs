//! `dm deps audit|outdated` — единый аудит зависимостей всех сервисов.
//!
//! Каркас с трейтом [`DepTool`]: каждый язык реализует свой способ аудита.
//! MVP-провайдеры вызывают системные `cargo audit`, `npm audit`, `govulncheck`,
//! `pip-audit`. Трейт готов к расширению (SBOM, Renovate-стиль PR и т.д.).

use crate::commands::{load_project_config, DepsAction, DepsArgs};
use crate::output::{print_system, println_styled, success_style, warn_style};
use comfy_table::{ContentArrangement, Table};
use dm_core::project::ServiceLanguage;
use dm_core::DmResult;
use std::path::Path;
use std::process::Command;

/// Точка расширения: аудит-инструмент для конкретного языка/стека.
pub trait DepTool {
    /// Запускает аудит/проверку устаревания в каталоге `dir`.
    fn run(&self, dir: &Path, outdated: bool) -> Result<DepResult, String>;
}

/// Результат проверки зависимостей одного сервиса.
#[derive(Debug, Clone)]
pub struct DepResult {
    /// Имя сервиса.
    pub service: String,
    /// Успешно ли завершился аудит.
    pub ok: bool,
    /// Сводный текст вывода (первые строки).
    pub summary: String,
}

/// Подбирает аудит-инструмент по языку сервиса.
pub fn tool_for(lang: ServiceLanguage) -> Option<Box<dyn DepTool>> {
    match lang {
        ServiceLanguage::Rust => Some(Box::new(CargoAudit)),
        ServiceLanguage::JavaScript
        | ServiceLanguage::TypeScript
        | ServiceLanguage::Nodejs
        | ServiceLanguage::Vite
        | ServiceLanguage::Nextjs
        | ServiceLanguage::Remix
        | ServiceLanguage::Bun => Some(Box::new(NpmAudit)),
        ServiceLanguage::Go => Some(Box::new(GoVuln)),
        ServiceLanguage::Python => Some(Box::new(PipAudit)),
        _ => None,
    }
}

/// Точка входа команды.
pub async fn run(args: DepsArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let outdated = matches!(args.action, DepsAction::Outdated);
    let label = if outdated { "устаревших зависимостей" } else { "уязвимостей зависимостей" };
    print_system(&format!("аудит {label} по {} сервисам", config.services.len()));

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Сервис", "Язык", "Статус", "Сводка"]);

    let mut total_ok = 0usize;
    for (name, svc) in &config.services {
        let dir = dm_core::paths::resolve(&root, Path::new(&svc.path));
        let summary = match tool_for(svc.language) {
            Some(tool) => match tool.run(&dir, outdated) {
                Ok(r) if r.ok => {
                    total_ok += 1;
                    (true, r.summary)
                }
                Ok(r) => (false, r.summary),
                Err(e) => (false, format!("инструмент недоступен: {e}")),
            },
            None => (false, "аудит для языка не настроен".to_string()),
        };
        let status = if summary.0 { "✓ чисто" } else { "⚠ есть замечания" };
        table.add_row(vec![
            name.to_string(),
            svc.language.label().to_string(),
            status.to_string(),
            summary.1,
        ]);
    }
    println!("{table}");
    println_styled(
        &format!("чистых сервисов: {total_ok}/{}", config.services.len()),
        if total_ok == config.services.len() {
            success_style()
        } else {
            warn_style()
        },
    );
    Ok(())
}

// --- Провайдеры для конкретных языков ---

struct CargoAudit;
impl DepTool for CargoAudit {
    fn run(&self, dir: &Path, outdated: bool) -> Result<DepResult, String> {
        if outdated {
            return run_cmd(dir, &["cargo", "update", "--dry-run"]);
        }
        run_cmd(dir, &["cargo", "audit"])
    }
}

struct NpmAudit;
impl DepTool for NpmAudit {
    fn run(&self, dir: &Path, outdated: bool) -> Result<DepResult, String> {
        if outdated {
            return run_cmd(dir, &["npm", "outdated"]);
        }
        run_cmd(dir, &["npm", "audit", "--omit=dev"])
    }
}

struct GoVuln;
impl DepTool for GoVuln {
    fn run(&self, dir: &Path, _outdated: bool) -> Result<DepResult, String> {
        // govulncheck ./...
        run_cmd(dir, &["govulncheck", "./..."])
    }
}

struct PipAudit;
impl DepTool for PipAudit {
    fn run(&self, dir: &Path, _outdated: bool) -> Result<DepResult, String> {
        run_cmd(dir, &["pip-audit"])
    }
}

/// Запускает команду `[program, args...]` в `dir`, возвращая краткий результат.
fn run_cmd(dir: &Path, argv: &[&str]) -> Result<DepResult, String> {
    let out = Command::new(argv[0])
        .args(&argv[1..])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("{e}"))?;
    // Берём последние 2 строки как сводку (часто там счётчик уязвимостей).
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let tail: String = combined
        .lines()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ");
    Ok(DepResult {
        service: String::new(),
        ok: out.status.success(),
        summary: tail,
    })
}

// Подавить неиспользуемые импорты при компиляции без них.
#[allow(dead_code)]
fn _silence(_: &dyn DepTool) {}

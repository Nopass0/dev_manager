//! `dm lint [svc]` — анализ кода (DRY/KISS/unused/duplicates).
//!
//! Читает все исходники сервисов, разбирает символы и прогоняет включённые
//! линтеры. Результат — таблица замечаний.

use crate::commands::{load_project_config, TargetArgs};
use crate::output::{print_system, success_style, println_styled};
use comfy_table::{ContentArrangement, Table};
use dm_analysis::lints::{run_all, LintCategory, LintFinding, LintSet};
use dm_analysis::{parse_file, Symbol};
use dm_core::DmResult;
use std::path::{Path, PathBuf};

/// Точка входа команды.
pub async fn run(args: TargetArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let lintset = LintSet {
        duplicates: config.linter.duplicates,
        dr: config.linter.dr,
        kiss: config.linter.kiss,
        unused: config.linter.unused_code,
    };

    let mut all_symbols: Vec<Symbol> = Vec::new();
    let mut corpus: Vec<String> = Vec::new();

    let targets: Vec<_> = match &args.name {
        Some(n) => {
            let svc = config
                .services
                .get(n)
                .ok_or_else(|| dm_core::DmError::ServiceNotFound(n.clone()))?;
            vec![(n.clone(), svc.clone())]
        }
        None => config.services.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    };

    for (name, svc) in &targets {
        let dir = dm_core::paths::resolve(&root, Path::new(&svc.path));
        print_system(&format!("сканирую '{name}': {}", dir.display()));
        collect_symbols(&dir, &mut all_symbols, &mut corpus);
    }

    let findings = run_all(&all_symbols, lintset);
    if findings.is_empty() {
        println_styled("замечаний не найдено ✨", success_style());
        return Ok(());
    }
    println_styled(&format!("найдено {} замечаний:", findings.len()), success_style());
    let table = build_findings_table(&findings);
    println!("{table}");
    Ok(())
}

/// Рекурсивно собирает символы и корпус текста из каталога.
fn collect_symbols(dir: &Path, symbols: &mut Vec<Symbol>, corpus: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            // Пропускаем типичные шумные каталоги.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, ".git" | "target" | "node_modules" | "dist" | "build" | ".next" | "out") {
                continue;
            }
            collect_symbols(&path, symbols, corpus);
            continue;
        }
        if let Ok(Some(parsed)) = parse_file(&path) {
            if let Ok(src) = std::fs::read_to_string(&path) {
                corpus.push(src);
            }
            symbols.extend(parsed.symbols);
        }
    }
}

/// Строит таблицу замечаний линтера.
fn build_findings_table(findings: &[LintFinding]) -> Table {
    let mut t = Table::new();
    t.load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Категория", "Файл", "Строка", "Символ", "Сообщение"]);
    for f in findings {
        t.add_row(vec![
            f.category.label().to_string(),
            short_path(&f.file),
            f.line.map(|l| l.to_string()).unwrap_or_default(),
            f.symbol.clone().unwrap_or_default(),
            f.message.clone(),
        ]);
    }
    t
}

/// Сокращает длинный путь до последних двух компонентов для компактности.
fn short_path(p: &PathBuf) -> String {
    let comps: Vec<_> = p.components().collect();
    let n = comps.len();
    if n <= 2 {
        p.display().to_string()
    } else {
        format!("…/{}", comps[n - 2..].iter().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join(std::path::MAIN_SEPARATOR_STR))
    }
}

// Сохраняем ссылку на категорию для будущей фильтрации.
#[allow(dead_code)]
fn _category_used(c: LintCategory) -> &'static str {
    c.label()
}

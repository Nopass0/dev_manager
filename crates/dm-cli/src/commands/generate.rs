//! `dm gen diagram|docs` — генерация артефактов проекта.
//!
//! `dm gen diagram` — Mermaid-диаграмма архитектуры: сервисы и их зависимости
//! (из `depends_on` + реальных cross-service импортов через граф).

use crate::commands::{GenArgs, load_project_config};
use crate::output::print_system;
use dm_core::DmResult;
use std::path::Path;

/// Точка входа команды.
pub async fn run(args: GenArgs) -> DmResult<()> {
    match args.kind.as_str() {
        "diagram" | "arch" => run_diagram().await,
        other => Err(dm_core::DmError::invalid_config(format!(
            "неизвестный артефакт '{other}'. Доступно: diagram | docs."
        ))),
    }
}

/// Генерирует Mermaid-диаграмму зависимостей сервисов.
async fn run_diagram() -> DmResult<()> {
    let (config, root) = load_project_config()?;
    print_system("генерация Mermaid-диаграммы архитектуры…");
    println!("```mermaid");
    println!("graph LR");

    // 1. Узлы сервисов с подписями языка.
    for (name, svc) in &config.services {
        println!(
            "  {name}[\"{name}<br/><small>{}</small>\"]",
            svc.language.label()
        );
    }

    // 2. Рёбра из явных depends_on.
    for (name, svc) in &config.services {
        for dep in &svc.depends_on {
            if config.services.contains_key(dep) {
                println!("  {dep} --> {name}");
            }
        }
    }

    // 3. Рёбра из реальных cross-service импортов (через граф tree-sitter).
    let graph = dm_analysis::DependencyGraph::build(&root);
    let dirs: Vec<(String, std::path::PathBuf)> = config
        .services
        .iter()
        .map(|(n, s)| {
            (
                n.clone(),
                dm_core::paths::resolve(&root, Path::new(&s.path)),
            )
        })
        .collect();
    for (i, (a_name, a_dir)) in dirs.iter().enumerate() {
        for (j, (b_name, b_dir)) in dirs.iter().enumerate() {
            if i == j {
                continue;
            }
            // Если сервис A импортирует файлы из каталога B — рисуем ребро.
            if imports_into(&graph, a_dir, b_dir) {
                println!("  {b_name} -.-> {a_name}");
            }
        }
    }
    println!("```");
    Ok(())
}

/// Возвращает true, если в графе есть ребро «a зависит от чего-то внутри b».
fn imports_into(
    graph: &dm_analysis::DependencyGraph,
    a_dir: &std::path::Path,
    b_dir: &std::path::Path,
) -> bool {
    for node in &graph.files {
        if !node.path.starts_with(a_dir) {
            continue;
        }
        for &imported_idx in &node.import_indices {
            if let Some(imported) = graph.files.get(imported_idx)
                && imported.path.starts_with(b_dir)
            {
                return true;
            }
        }
    }
    false
}

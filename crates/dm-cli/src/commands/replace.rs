//! `dm replace <pattern> <replacement>` — find & replace по всему проекту.

use crate::commands::ReplaceArgs;
use crate::commands::load_project_config;
use crate::output::{print_system, println_styled, success_style};
use dm_analysis::search::{SearchOptions, replace as run_replace};
use dm_core::DmResult;

/// Точка входа команды.
pub async fn run(args: ReplaceArgs) -> DmResult<()> {
    let (_config, root) = load_project_config()?;
    let opts = SearchOptions {
        case_insensitive: args.ignore_case,
        whole_word: args.word,
        ..Default::default()
    };
    let changed = run_replace(&root, &args.pattern, &args.replacement, &opts, args.dry_run);
    if changed.is_empty() {
        print_system("совпадений для замены не найдено.");
        return Ok(());
    }
    let verb = if args.dry_run {
        "было бы изменено"
    } else {
        "изменено"
    };
    println_styled(
        &format!("файлов {verb}: {}", changed.len()),
        success_style(),
    );
    for f in &changed {
        println!("  {}", f.display());
    }
    Ok(())
}

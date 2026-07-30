//! `dm git stash|branch|rebase` — cross-repo git-операции.

use crate::commands::{load_project_config, GitAction, GitArgs};
use crate::output::{print_system, success_style, warn_style, println_styled};
use dm_core::DmResult;
use dm_vcs::{branch_all, collect_repo_paths, rebase_all, stash_all};

/// Точка входа команды.
pub async fn run(args: GitArgs) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    let repos = collect_repo_paths(&config, &root)?;
    if repos.is_empty() {
        return Err(dm_core::DmError::invalid_config("репозитории не найдены"));
    }

    let results = match args.action {
        GitAction::Stash => {
            print_system(&format!("git stash в {} репо", repos.len()));
            stash_all(&repos).await
        }
        GitAction::Branch { name } => {
            print_system(&format!("git checkout -B {name} в {} репо", repos.len()));
            branch_all(&repos, &name).await
        }
        GitAction::Rebase { onto } => {
            print_system(&format!("git rebase {onto} в {} репо", repos.len()));
            rebase_all(&repos, &onto).await
        }
    };

    let ok = results.iter().filter(|r| r.ok).count();
    for r in &results {
        let marker = if r.ok { "✓" } else { "✗" };
        println!("{} {} — {}", marker, r.repo.display(), r.note);
    }
    if ok == results.len() {
        println_styled(&format!("готово: {}/{}", ok, results.len()), success_style());
    } else {
        println_styled(&format!("с ошибками: {}/{}", ok, results.len()), warn_style());
    }
    Ok(())
}

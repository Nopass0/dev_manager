//! `dm cache clear` — очистка кэшей сборок сервисов.

use crate::commands::{load_project_config, CacheAction};
use crate::output::{print_system, success_style, println_styled};
use dm_core::DmResult;
use std::path::Path;

/// Каталоги кэшей по платформе/языку, которые чистит Dev Manager.
const CACHE_DIRS: &[&str] = &[
    "target",
    "node_modules/.cache",
    ".next/cache",
    "dist",
    "build",
    "__pycache__",
    ".pytest_cache",
];

/// Точка входа команды.
pub async fn run(action: CacheAction) -> DmResult<()> {
    let (config, root) = load_project_config()?;
    match action {
        CacheAction::Clear => {
            let mut cleared = 0usize;
            for (_name, svc) in &config.services {
                let dir = dm_core::paths::resolve(&root, Path::new(&svc.path));
                for cache in CACHE_DIRS {
                    let target = dir.join(cache);
                    if target.exists() {
                        print_system(&format!("удаляю {}", target.display()));
                        if std::fs::remove_dir_all(&target).is_ok() {
                            cleared += 1;
                        }
                    }
                }
            }
            println_styled(
                &format!("очищено каталогов кэша: {cleared}"),
                success_style(),
            );
            Ok(())
        }
    }
}

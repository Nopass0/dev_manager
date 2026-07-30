//! Автоопределение команды запуска сервиса.
//!
//! Если в `dm.yaml` поле `run` не задано, Dev Manager пытается угадать разумную
//! команду по языку/стеку и/или по наличию известных файлов-маркеров в каталоге
//! (например, `package.json` → `npm run dev`).

use dm_core::project::Service;
use std::path::Path;

/// Определяет финальную команду запуска для сервиса.
///
/// Порядок разрешения:
/// 1. Явная команда из конфига (`run`), если задана.
/// 2. Эвристика по файлам-маркерам в каталоге (`package.json`, `Cargo.toml`…).
/// 3. Дефолтная команда по языку ([`ServiceLanguage::default_run_command`]).
///
/// Возвращает строку команды. Если ничего не удалось определить — возвращает
/// пустую строку (вызывающий код решит, как реагировать).
pub fn resolve_run_command(svc: &Service, explicit: Option<&str>) -> String {
    if let Some(cmd) = explicit
        && !cmd.trim().is_empty()
    {
        return cmd.to_string();
    }
    if let Some(detected) = detect_by_markers(&svc.path) {
        return detected;
    }
    svc.language.default_run_command().unwrap_or("").to_string()
}

/// Пытается подобрать команду запуска по известным файлам-маркерам.
///
/// Возвращает `None`, если в каталоге нет ни одного из распознанных файлов.
fn detect_by_markers(dir: &Path) -> Option<String> {
    let has = |name: &str| dir.join(name).exists();

    // Node-семейство
    if has("package.json") {
        // bun.lockb приоритетнее, если установлен bun.
        if has("bun.lockb") || has("bun.lock") {
            return Some("bun run dev".into());
        }
        return Some("npm run dev".into());
    }
    // Rust
    if has("Cargo.toml") {
        return Some("cargo run".into());
    }
    // Go
    if has("go.mod") {
        return Some("go run .".into());
    }
    // .NET
    if has("*.csproj") || has("project.json") {
        return Some("dotnet run".into());
    }
    // Python
    if has("pyproject.toml") || has("requirements.txt") || has("main.py") {
        return Some("python main.py".into());
    }
    // Lua
    if has("main.lua") {
        return Some("lua main.lua".into());
    }
    None
}

/// Разовый helper для внешнего использования: возвращает true, если в каталоге
/// узнаваемая Node-экосистема (нужно для env-распределения и тестов).
pub fn looks_like_node_project(dir: &Path) -> bool {
    dir.join("package.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_core::project::ServiceLanguage;

    fn svc(name: &str, lang: ServiceLanguage, dir: &Path) -> Service {
        Service {
            name: name.into(),
            path: dir.to_path_buf(),
            language: lang,
            run_command: String::new(),
            watch: true,
            restart_on_change: true,
            repo_path: None,
            delay_ms: 0,
        }
    }

    #[test]
    fn explicit_command_wins() {
        let tmp = std::env::temp_dir().join("dm_spawn_explicit");
        std::fs::create_dir_all(&tmp).unwrap();
        let s = svc("a", ServiceLanguage::Rust, &tmp);
        let cmd = resolve_run_command(&s, Some("./my-runner"));
        assert_eq!(cmd, "./my-runner");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detects_cargo_toml() {
        let tmp = std::env::temp_dir().join("dm_spawn_cargo");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "").unwrap();
        let s = svc("a", ServiceLanguage::Rust, &tmp);
        let cmd = resolve_run_command(&s, None);
        assert_eq!(cmd, "cargo run");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn falls_back_to_language_default() {
        // Пустой каталог, без маркеров — берём дефолт языка.
        let tmp = std::env::temp_dir().join("dm_spawn_empty");
        std::fs::create_dir_all(&tmp).unwrap();
        let s = svc("a", ServiceLanguage::Go, &tmp);
        let cmd = resolve_run_command(&s, None);
        assert_eq!(cmd, "go run .");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

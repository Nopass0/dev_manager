//! Поиск, чтение, наследование и валидация `dm.yaml`.
//!
//! Возможности загрузчика:
//! - **Поиск вверх**: от текущего каталога к корню, как у `git`.
//! - **Наследование (`extends`)**: базовый конфиг загружается первым, текущий
//!   deep-merge'ится поверх него. Скаляры — «последний выигрывает», карты
//!   (services/profiles) — объединяются по ключу, векторы — конкатенируются.
//! - **Env-оверлеи**: если задано окружение (`--env`/`DM_ENV`), ищется
//!   `dm.<env>.yaml` рядом с основным и мержится поверх.
//! - **Интерполяция**: `{{var}}` берутся из секции `defaults`/сервисов, `${VAR}` —
//!   из переменных окружения процесса.
//!
//! Порядок применения (от базового к финальному):
//! `base (extends) → dm.yaml → dm.<env>.yaml → defaults-merge`.

use crate::config::schema::Config;
use crate::error::{DmError, DmResult};
use crate::paths;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Имя файла конфигурации, которое мы ищем.
pub const CONFIG_FILENAME: &str = "dm.yaml";

/// Альтернативное имя (короткая форма), поддерживается наравне с основным.
pub const CONFIG_FILENAME_ALT: &str = "dm.yml";

/// Имя env-оверлея: `dm.<env>.yaml`.
pub fn env_overlay_filename(env: &str) -> String {
    format!("dm.{env}.yaml")
}

/// Имя переменной окружения для выбора профиля окружения.
pub const DM_ENV_VAR: &str = "DM_ENV";

/// Ищет файл конфигурации начиная с `start_dir` и поднимаясь к корню ФС.
///
/// Возвращает путь к найденному `dm.yaml`/`dm.yml`. Если ничего не найдено —
/// [`DmError::ConfigNotFound`].
///
/// # Пример
/// ```no_run
/// # use dm_core::config::discover_config;
/// let path = discover_config(std::env::current_dir().unwrap()).unwrap();
/// println!("Конфиг найден: {}", path.display());
/// ```
pub fn discover_config(start_dir: impl AsRef<Path>) -> DmResult<PathBuf> {
    let mut dir: Option<&Path> = Some(start_dir.as_ref());
    while let Some(current) = dir {
        for name in [CONFIG_FILENAME, CONFIG_FILENAME_ALT] {
            let candidate = current.join(name);
            if candidate.is_file() {
                return Ok(paths::simplify(&candidate));
            }
        }
        dir = current.parent();
    }
    Err(DmError::ConfigNotFound)
}

/// Главная точка входа: грузит конфиг с наследованием, env-оверлеем и интерполяцией.
///
/// Разрешает:
/// 1. `extends` рекурсивно (глубина ограничена защитой от циклов);
/// 2. env-оверлей `dm.<env>.yaml`, если `env` задан (явно или через `DM_ENV`);
/// 3. интерполяцию `{{var}}`/`${VAR}` в строковых полях;
/// 4. слияние `defaults:` в каждый сервис;
/// 5. фильтрацию сервисов по `only_on`.
pub fn load_resolved_config(path: &Path, env: Option<&str>) -> DmResult<Config> {
    // 1. Базовая цепочка extends → текущий файл.
    let mut cfg = load_with_extends(path)?;

    // 2. Env-оверлей: dm.<env>.yaml в том же каталоге, если есть.
    let env_name = env
        .map(|s| s.to_string())
        .or_else(|| std::env::var(DM_ENV_VAR).ok())
        .unwrap_or_default();
    if !env_name.is_empty() {
        cfg.env = env_name.clone();
        let overlay_path = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(env_overlay_filename(&env_name));
        if overlay_path.is_file() {
            let overlay = load_with_extends(&overlay_path)?;
            cfg = deep_merge(cfg, overlay);
        }
    }

    // 3. defaults → каждый сервис (deep-merge).
    if let Some(defaults) = cfg.defaults.clone() {
        apply_defaults(&mut cfg, &defaults);
    }

    // 4. Интерполяция переменных.
    interpolate(&mut cfg)?;

    // 5. Фильтр only_on: убираем сервисы, не активные в текущем окружении.
    if !env_name.is_empty() {
        cfg.services
            .retain(|_, svc| svc.only_on.is_empty() || svc.only_on.iter().any(|e| e == &env_name));
    }

    cfg.validate()?;
    Ok(cfg)
}

/// Грузит конфиг с разрешением цепочки `extends` (рекурсивно, с защитой от циклов).
fn load_with_extends(path: &Path) -> DmResult<Config> {
    let mut visited: Vec<PathBuf> = Vec::new();
    load_with_extends_inner(path, &mut visited)
}

/// Рекурсивный помощник [`load_with_extends`]; `visited` — защита от циклов.
fn load_with_extends_inner(path: &Path, visited: &mut Vec<PathBuf>) -> DmResult<Config> {
    let canonical = paths::simplify(path);
    if visited.contains(&canonical) {
        return Err(DmError::invalid_config(format!(
            "обнаружен цикл в `extends`: {}",
            canonical.display()
        )));
    }
    visited.push(canonical.clone());

    let raw = std::fs::read_to_string(path).map_err(|source| DmError::ConfigIo {
        path: path.to_path_buf(),
        source,
    })?;
    let cfg: Config = serde_yaml::from_str(&raw)?;

    // Если есть extends — сначала грузим базу, потом мержим текущий сверху.
    if let Some(extends_rel) = cfg.extends.as_ref() {
        let base_path = path.parent().unwrap_or(Path::new(".")).join(extends_rel);
        let base = load_with_extends_inner(&base_path, visited)?;
        // cfg — текущий (override), base — унаследованный.
        // deep_merge(base, override): override выигрывает.
        let mut merged = deep_merge(base, cfg);
        // После merge поле extends уже не нужно (оно разрешено).
        merged.extends = None;
        Ok(merged)
    } else {
        Ok(cfg)
    }
}

/// Глубокое слияние двух конфигов: `override` перекрывает `base`.
///
/// - Скаляры: из `override` (если не default/пусто), иначе из `base`.
/// - Карты (services/profiles): ключи объединяются, значения — рекурсивно.
/// - Векторы: конкатенируются (deps, tags, hooks), дубли убираются.
pub fn deep_merge(mut base: Config, override_cfg: Config) -> Config {
    // Скалярные поля: override выигрывает, если задано (непусто).
    if !override_cfg.project_name.is_empty() {
        base.project_name = override_cfg.project_name;
    }
    if override_cfg.env_file != ".env" {
        base.env_file = override_cfg.env_file;
    }
    if override_cfg.version != 1 && override_cfg.version != 0 {
        base.version = override_cfg.version;
    }

    // services: merge по ключу.
    for (name, svc) in override_cfg.services {
        base.services
            .entry(name)
            .and_modify(|existing| {
                *existing = merge_service(existing.clone(), svc.clone());
            })
            .or_insert(svc);
    }

    // profiles: merge по ключу.
    for (name, p) in override_cfg.profiles {
        base.profiles.entry(name).or_insert(p);
    }

    // deploy: объединяем, убирая дубли по name.
    for d in override_cfg.deploy {
        if !base.deploy.iter().any(|x| x.name == d.name) {
            base.deploy.push(d);
        }
    }

    // defaults: override выигрывает целиком, если задан.
    if override_cfg.defaults.is_some() {
        base.defaults = override_cfg.defaults;
    }

    // runtime: override выигрывает, если задан (ненулевой max_parallel или непустые exts).
    if override_cfg.runtime.max_parallel != 0
        || !override_cfg.runtime.watch_ignore_extensions.is_empty()
    {
        base.runtime = override_cfg.runtime;
    }

    // linter: берем значения override там, где они true (булевы — простое «или»).
    base.linter.dr |= override_cfg.linter.dr;
    base.linter.kiss |= override_cfg.linter.kiss;
    base.linter.unused_code |= override_cfg.linter.unused_code;
    base.linter.duplicates |= override_cfg.linter.duplicates;
    // auto_fix — override выигрывает, если явно true.
    if override_cfg.linter.auto_fix {
        base.linter.auto_fix = true;
    }

    base
}

/// Слияние двух ServiceConfig: override перекрывает base для скаляров,
/// векторы конкатенируются (с дедупликацией).
fn merge_service(
    mut base: crate::config::ServiceConfig,
    ov: crate::config::ServiceConfig,
) -> crate::config::ServiceConfig {
    use crate::project::ServiceLanguage;
    // Скаляры/опциональные: override если задано.
    if !ov.path.is_empty() {
        base.path = ov.path;
    }
    if ov.language != ServiceLanguage::default() {
        base.language = ov.language;
    }
    if ov.repo.is_some() {
        base.repo = ov.repo;
    }
    if ov.run.is_some() {
        base.run = ov.run;
    }
    // Булевы: override если не дефолт.
    if ov.watch != base.watch {
        base.watch = ov.watch;
    }
    if ov.restart_on_change != base.restart_on_change {
        base.restart_on_change = ov.restart_on_change;
    }
    if ov.delay_ms != 0 {
        base.delay_ms = ov.delay_ms;
    }
    if ov.order != 100 {
        base.order = ov.order;
    }
    // Векторы: конкатенация с дедупом.
    for d in ov.depends_on {
        if !base.depends_on.contains(&d) {
            base.depends_on.push(d);
        }
    }
    for t in ov.tags {
        if !base.tags.contains(&t) {
            base.tags.push(t);
        }
    }
    for c in ov.before_start {
        if !base.before_start.contains(&c) {
            base.before_start.push(c);
        }
    }
    for c in ov.after_start {
        if !base.after_start.contains(&c) {
            base.after_start.push(c);
        }
    }
    for e in ov.only_on {
        if !base.only_on.contains(&e) {
            base.only_on.push(e);
        }
    }
    // Опциональные сложные: override выигрывает.
    if ov.health.is_some() {
        base.health = ov.health;
    }
    if ov.resources.is_some() {
        base.resources = ov.resources;
    }
    if ov.shell.is_some() {
        base.shell = ov.shell;
    }
    if ov.working_dir.is_some() {
        base.working_dir = ov.working_dir;
    }
    // env-карта: merge.
    for (k, v) in ov.env {
        base.env.insert(k, v);
    }
    // tests/logs/restart_policy: override целиком, если задано.
    if ov.tests.cmd.is_some() || ov.tests.on_change {
        base.tests = ov.tests;
    }
    base
}

/// Применяет `defaults` ко всем сервисам (поля, не заданные явно, берутся из defaults).
fn apply_defaults(cfg: &mut Config, defaults: &crate::config::ServiceConfig) {
    for (_name, svc) in cfg.services.iter_mut() {
        let merged = merge_service(defaults.clone(), svc.clone());
        *svc = merged;
    }
}

/// Интерполирует `{{var}}` (из секции defaults.env / сервисов) и `${VAR}` (из env процесса)
/// во всех строковых полях конфига.
fn interpolate(cfg: &mut Config) -> DmResult<()> {
    // Контекст {{var}}: переменные из defaults.env (если есть) + проектные.
    let mut ctx: HashMap<String, String> = HashMap::new();
    ctx.insert("project_name".into(), cfg.project_name.clone());
    ctx.insert("env".into(), cfg.env.clone());
    if let Some(d) = &cfg.defaults {
        for (k, v) in &d.env {
            ctx.insert(k.clone(), v.clone());
        }
    }

    // Интерполируем поля сервисов.
    for (_name, svc) in cfg.services.iter_mut() {
        svc.path = interp_string(&svc.path, &ctx);
        if let Some(r) = svc.run.as_mut() {
            *r = interp_string(r, &ctx);
        }
        if let Some(r) = svc.repo.as_mut() {
            *r = interp_string(r, &ctx);
        }
        if let Some(w) = svc.working_dir.as_mut() {
            *w = interp_string(w, &ctx);
        }
        for c in svc.before_start.iter_mut() {
            *c = interp_string(c, &ctx);
        }
        for c in svc.after_start.iter_mut() {
            *c = interp_string(c, &ctx);
        }
        for (_k, v) in svc.env.iter_mut() {
            *v = interp_string(v, &ctx);
        }
    }
    Ok(())
}

/// Заменяет `{{key}}` значениями из `ctx`, а `${VAR}` — переменными окружения.
fn interp_string(s: &str, ctx: &HashMap<String, String>) -> String {
    let mut out = s.to_string();
    // {{var}}
    while let Some(start) = out.find("{{") {
        let Some(end) = out[start..].find("}}").map(|e| start + e) else {
            break;
        };
        let key = out[start + 2..end].trim().to_string();
        let value = ctx.get(&key).cloned().unwrap_or_default();
        out.replace_range(start..end + 2, &value);
    }
    // ${VAR}
    while let Some(start) = out.find("${") {
        let Some(end) = out[start..].find('}').map(|e| start + e) else {
            break;
        };
        let key = out[start + 2..end].trim().to_string();
        let value = std::env::var(&key).unwrap_or_default();
        out.replace_range(start..end + 1, &value);
    }
    out
}

/// Читает и разбирает `dm.yaml` по заданному пути, затем валидирует.
///
/// Упрощённый API без extends/env — для обратной совместимости.
/// Для полной загрузки используйте [`load_resolved_config`].
pub fn load_config(path: &Path) -> DmResult<Config> {
    load_resolved_config(path, None)
}

impl Config {
    /// Семантическая валидация после успешного разбора.
    ///
    /// Проверяет: версию схемы, непустоту сервисов, корректность имён, что
    /// `path` есть у каждого сервиса. Каталоги здесь НЕ проверяются — это
    /// делается отдельно в runtime, чтобы конфиг можно было загрузить до того,
    /// как сервисные каталоги созданы (например, в `dm init`).
    pub fn validate(&mut self) -> DmResult<()> {
        if self.version != 1 {
            return Err(DmError::ConfigUnsupportedVersion(self.version));
        }
        if self.services.is_empty() {
            return Err(DmError::invalid_config(
                "в конфиге не описано ни одного сервиса в секции `services`.",
            ));
        }
        for (name, svc) in &self.services {
            if svc.path.trim().is_empty() {
                return Err(DmError::invalid_config(format!(
                    "у сервиса '{name}' не задано поле `path`."
                )));
            }
            if name.trim().is_empty() {
                return Err(DmError::invalid_config(
                    "имя сервиса не может быть пустой строкой.",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn discovers_config_in_parent() {
        let tmp = env::temp_dir().join("dm_test_discover2");
        let sub = tmp.join("deep/sub");
        std::fs::create_dir_all(&sub).unwrap();
        let cfg_path = tmp.join(CONFIG_FILENAME);
        std::fs::write(
            &cfg_path,
            "version: 1\nservices:\n  a:\n    path: ./a\n    language: rust\n",
        )
        .unwrap();
        let found = discover_config(&sub).unwrap();
        assert_eq!(found, paths::simplify(&cfg_path));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn returns_not_found_when_absent() {
        let tmp = env::temp_dir().join("dm_test_empty_discover2");
        std::fs::create_dir_all(&tmp).unwrap();
        let err = discover_config(&tmp).unwrap_err();
        assert!(matches!(err, DmError::ConfigNotFound));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn deep_merge_overrides_scalars_and_merges_maps() {
        let base_yaml = r#"
version: 1
project_name: base
services:
  api:
    path: ./api
    language: rust
    tags: [backend]
"#;
        let ov_yaml = r#"
project_name: override
services:
  web:
    path: ./web
    language: vite
"#;
        let base: Config = serde_yaml::from_str(base_yaml).unwrap();
        let ov: Config = serde_yaml::from_str(ov_yaml).unwrap();
        let merged = deep_merge(base, ov);
        assert_eq!(merged.project_name, "override"); // override выигрывает
        assert_eq!(merged.services.len(), 2); // api + web
        assert!(merged.services.contains_key("api"));
        assert!(merged.services.contains_key("web"));
    }

    #[test]
    fn extends_resolves_base_then_override() {
        let dir = env::temp_dir().join("dm_extends_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("base.yaml"),
            "version: 1\nproject_name: base\nservices:\n  api:\n    path: ./api\n    language: rust\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("dm.yaml"),
            "extends: base.yaml\nservices:\n  web:\n    path: ./web\n    language: vite\n",
        )
        .unwrap();
        let cfg = load_resolved_config(&dir.join("dm.yaml"), None).unwrap();
        assert_eq!(cfg.project_name, "base");
        assert!(cfg.services.contains_key("api")); // из base
        assert!(cfg.services.contains_key("web")); // из override
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_overlay_applied() {
        let dir = env::temp_dir().join("dm_env_overlay_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("dm.yaml"),
            "version: 1\nservices:\n  api:\n    path: ./api\n    language: rust\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("dm.staging.yaml"),
            "services:\n  api:\n    order: 5\n",
        )
        .unwrap();
        let cfg = load_resolved_config(&dir.join("dm.yaml"), Some("staging")).unwrap();
        assert_eq!(cfg.env, "staging");
        assert_eq!(cfg.services["api"].order, 5); // overlay применился
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_on_filters_inactive_services() {
        let dir = env::temp_dir().join("dm_only_on_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("dm.yaml"),
            "version: 1\nservices:\n  api:\n    path: ./api\n    language: rust\n  debug:\n    path: ./debug\n    language: rust\n    only_on: [dev]\n",
        )
        .unwrap();
        let cfg = load_resolved_config(&dir.join("dm.yaml"), Some("prod")).unwrap();
        assert!(cfg.services.contains_key("api"));
        assert!(!cfg.services.contains_key("debug")); // отфильтрован в prod
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn interpolation_replaces_vars() {
        let mut ctx = HashMap::new();
        ctx.insert("name".into(), "world".into());
        assert_eq!(interp_string("hello {{name}}", &ctx), "hello world");
        assert_eq!(interp_string("{{name}}-{{name}}", &ctx), "world-world");
        // ${VAR} из env процесса.
        unsafe {
            std::env::set_var("DM_TEST_INTERP", "xyz");
        }
        assert_eq!(interp_string("v=${DM_TEST_INTERP}", &ctx), "v=xyz");
    }
}

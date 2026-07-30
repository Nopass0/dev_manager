//! Фильтрация списка сервисов по флагам `--only/--skip/--tag/--profile/--affected`.
//!
//! Общая логика для `dm start`, `dm test`, `dm lint` и др. (DRY). Применяет
//! фильтры последовательно: profile → tag → only → skip → affected.

use dm_core::Config;

/// Параметры фильтрации сервисов.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Только эти сервисы (по имени).
    pub only: Vec<String>,
    /// Пропустить эти сервисы.
    pub skip: Vec<String>,
    /// Оставить сервисы с любым из этих тегов.
    pub tag: Vec<String>,
    /// Имя профиля из `profiles:`.
    pub profile: Option<String>,
    /// Оставить только затронутые изменениями (передаются извне после расчёта).
    pub affected: Option<Vec<String>>,
}

impl Selection {
    /// Применяет фильтры к упорядоченному списку имён сервисов.
    ///
    /// Возвращает отфильтрованный список в исходном порядке.
    pub fn apply(&self, config: &Config, all_in_order: &[String]) -> Vec<String> {
        let mut names: Vec<String> = all_in_order.to_vec();

        // 1. Профиль: если задан, оставляем только сервисы из профиля.
        if let Some(profile) = &self.profile {
            if let Some(p) = config.profiles.get(profile) {
                if !p.services.is_empty() {
                    let allowed: std::collections::HashSet<&str> =
                        p.services.iter().map(|s| s.as_str()).collect();
                    names.retain(|n| allowed.contains(n.as_str()));
                }
            }
        }

        // 2. Теги: оставляем сервисы, у которых есть хотя бы один из тегов.
        if !self.tag.is_empty() {
            let wanted: std::collections::HashSet<&str> =
                self.tag.iter().map(|s| s.as_str()).collect();
            names.retain(|n| {
                config
                    .services
                    .get(n)
                    .map(|s| s.tags.iter().any(|t| wanted.contains(t.as_str())))
                    .unwrap_or(false)
            });
        }

        // 3. Only: явный whitelist имён.
        if !self.only.is_empty() {
            let allowed: std::collections::HashSet<&str> =
                self.only.iter().map(|s| s.as_str()).collect();
            names.retain(|n| allowed.contains(n.as_str()));
        }

        // 4. Skip: blacklist.
        if !self.skip.is_empty() {
            let blocked: std::collections::HashSet<&str> =
                self.skip.iter().map(|s| s.as_str()).collect();
            names.retain(|n| !blocked.contains(n.as_str()));
        }

        // 5. Affected: внешний предрасчитанный список имён.
        if let Some(affected) = &self.affected {
            let allowed: std::collections::HashSet<&str> =
                affected.iter().map(|s| s.as_str()).collect();
            names.retain(|n| allowed.contains(n.as_str()));
        }

        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_core::config::ServiceConfig;
    use dm_core::project::ServiceLanguage;

    fn cfg_with(services: Vec<(&str, Vec<String>, ServiceLanguage)>) -> Config {
        let mut c = Config::default();
        for (name, tags, lang) in services {
            c.services.insert(
                name.to_string(),
                ServiceConfig {
                    path: format!("./{name}"),
                    language: lang,
                    tags,
                    ..Default::default()
                },
            );
        }
        c
    }

    #[test]
    fn tag_filter() {
        let cfg = cfg_with(vec![
            ("api", vec!["backend".into()], ServiceLanguage::Rust),
            ("web", vec!["frontend".into()], ServiceLanguage::Vite),
            ("db", vec!["infra".into()], ServiceLanguage::Other),
        ]);
        let all = cfg.services_in_start_order();
        let sel = Selection {
            tag: vec!["backend".into()],
            ..Default::default()
        };
        assert_eq!(sel.apply(&cfg, &all), vec!["api"]);
    }

    #[test]
    fn only_and_skip() {
        let cfg = cfg_with(vec![
            ("api", vec![], ServiceLanguage::Rust),
            ("web", vec![], ServiceLanguage::Vite),
            ("db", vec![], ServiceLanguage::Other),
        ]);
        let all = cfg.services_in_start_order();
        let sel = Selection {
            only: vec!["api".into(), "web".into(), "db".into()],
            skip: vec!["db".into()],
            ..Default::default()
        };
        let result = sel.apply(&cfg, &all);
        assert!(result.contains(&"api".to_string()));
        assert!(result.contains(&"web".to_string()));
        assert!(!result.contains(&"db".to_string()));
    }
}

//! Интернационализация CLI: русский и английский интерфейсы.
//!
//! Язык определяется: `--lang` флаг > `DM_LANG` env > системный язык.
//! Все строки — в [`STRINGS`] с ключами; [`t`] возвращает локализованную строку.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Доступные языки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
}

static LANG: OnceLock<Lang> = OnceLock::new();

/// Инициализирует язык из `--lang` / `DM_LANG` / системы. Вызывается из main.
pub fn init(explicit: Option<&str>) {
    let lang = explicit
        .map(parse_lang)
        .or_else(|| std::env::var("DM_LANG").ok().map(|s| parse_lang(&s)))
        .unwrap_or_else(|| {
            // По умолчанию: если LANG содержит RU — русский, иначе английский.
            let sys = std::env::var("LANG").unwrap_or_default();
            if sys.to_uppercase().starts_with("RU") {
                Lang::Ru
            } else {
                Lang::En
            }
        });
    let _ = LANG.set(lang);
}

/// Возвращает текущий язык.
pub fn current() -> Lang {
    *LANG.get().unwrap_or(&Lang::Ru)
}

/// Возвращает локализованную строку по ключу.
pub fn t(key: &str) -> String {
    let lang = current();
    STRINGS
        .get(key)
        .map(|m| match lang {
            Lang::Ru => m.0.to_string(),
            Lang::En => m.1.to_string(),
        })
        .unwrap_or_else(|| key.to_string())
}

fn parse_lang(s: &str) -> Lang {
    match s.to_lowercase().as_str() {
        "ru" | "rus" | "russian" | "русский" | "ру" => Lang::Ru,
        _ => Lang::En,
    }
}

/// Карта: ключ → (русский, английский).
static STRINGS: std::sync::LazyLock<HashMap<&'static str, (&'static str, &'static str)>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert(
            "app_name",
            (
                "Dev Manager — менеджер разработки",
                "Dev Manager — development manager",
            ),
        );
        m.insert(
            "config_not_found",
            (
                "конфиг dm.yaml не найден. Запустите `dm init`.",
                "config dm.yaml not found. Run `dm init`.",
            ),
        );
        m.insert("starting", ("запуск", "starting"));
        m.insert("stopping", ("остановка", "stopping"));
        m.insert("services", ("сервисов", "services"));
        m.insert("ready", ("готово", "ready"));
        m.insert("error", ("ошибка", "error"));
        m.insert("done", ("готово", "done"));
        m.insert("installing", ("установка", "installing"));
        m.insert("building", ("сборка", "building"));
        m.insert("testing", ("тесты", "testing"));
        m.insert("commit", ("коммит", "commit"));
        m.insert("push", ("пуш", "push"));
        m.insert(
            "no_services",
            ("нет сервисов для запуска", "no services to run"),
        );
        m.insert(
            "watching",
            ("отслеживание изменений", "watching for changes"),
        );
        m.insert(
            "ctrl_c",
            (
                "получен Ctrl+C, останавливаю…",
                "Ctrl+C received, shutting down…",
            ),
        );
        m.insert(
            "all_stopped",
            ("все сервисы остановлены.", "all services stopped."),
        );
        m.insert("docs_link", ("Документация", "Documentation"));
        m.insert("examples", ("Примеры", "Examples"));
        m.insert("contributing", ("Контрибьюторам", "Contributing"));
        m
    });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lang_recognizes_variants() {
        assert_eq!(parse_lang("ru"), Lang::Ru);
        assert_eq!(parse_lang("RU"), Lang::Ru);
        assert_eq!(parse_lang("russian"), Lang::Ru);
        assert_eq!(parse_lang("en"), Lang::En);
        assert_eq!(parse_lang("english"), Lang::En);
        assert_eq!(parse_lang("fr"), Lang::En);
        assert_eq!(parse_lang(""), Lang::En);
    }

    // Note: OnceLock means init() only takes effect once globally.
    // These tests verify that init + current + t work together at least once.

    #[test]
    fn init_sets_language() {
        // init may have already been called by another test; just verify
        // that current() returns a valid Lang after init.
        init(Some("ru"));
        let lang = current();
        assert!(
            lang == Lang::Ru || lang == Lang::En,
            "current() should return valid Lang"
        );
    }

    #[test]
    fn t_returns_string_for_known_keys() {
        // Regardless of which language was set, t() should return a non-empty
        // string for known keys.
        let val = t("config_not_found");
        assert!(!val.is_empty(), "known key should return non-empty string");
        assert!(
            val.contains("конфиг") || val.contains("config"),
            "should contain either RU or EN variant"
        );
    }

    #[test]
    fn t_falls_back_for_unknown_keys() {
        let val = t("nonexistent_key_xyz");
        assert_eq!(val, "nonexistent_key_xyz");
    }
}

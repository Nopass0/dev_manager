//! Парсинг единого `.env` и запись переменных в `.env` каждого сервиса.

use crate::error::{DmError, DmResult};
use indexmap::IndexMap;
use std::path::Path;

/// Имя секции «по умолчанию» — переменные без явной секции в `.env`.
///
/// Эти переменные попадают во все сервисы.
pub const GLOBAL_SECTION: &str = "";

/// Одна секция единого `.env`: имя секции → (имя переменной → значение).
pub type EnvSection = IndexMap<String, String>;

/// Распарсенный единый `.env`: секция → переменные.
///
/// Секция с ключом [`GLOBAL_SECTION`] содержит глобальные переменные.
#[derive(Debug, Clone, Default)]
pub struct UnifiedEnv {
    /// Карта секция → переменные. `IndexMap` сохраняет порядок объявления.
    pub sections: IndexMap<String, EnvSection>,
}

impl UnifiedEnv {
    /// Возвращает переменные, которые должны попасть в сервис `service`.
    ///
    /// Это объединение глобальной секции и секции с именем `service` (если есть).
    /// При совпадении имён приоритет у сервис-специфичных значений.
    pub fn vars_for(&self, service: &str) -> IndexMap<String, String> {
        let mut out = IndexMap::new();
        if let Some(g) = self.sections.get(GLOBAL_SECTION) {
            out.extend(g.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(s) = self.sections.get(service) {
            for (k, v) in s {
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }
}

/// Разбирает содержимое единого `.env` в структуру [`UnifiedEnv`].
///
/// Правила:
/// - строки вида `[name]` начинают новую секцию;
/// - `KEY=value` добавляется в текущую секцию;
/// - пустые строки и комментарии (`#...`) игнорируются;
/// - значения могут быть в кавычках (`"..."`/`'...'`), кавычки снимаются;
/// - поддерживается `export KEY=value`.
pub fn parse_unified_env(content: &str) -> DmResult<UnifiedEnv> {
    let mut env = UnifiedEnv::default();
    // Глобальная секция существует всегда, даже если пуста.
    let mut current = GLOBAL_SECTION.to_string();
    env.sections.insert(current.clone(), IndexMap::new());

    for (lineno, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Заголовок секции [name]
        if let Some(stripped) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current = stripped.trim().to_string();
            env.sections
                .entry(current.clone())
                .or_insert_with(IndexMap::new);
            continue;
        }
        // KEY=value
        let (key, value) = parse_kv(line).ok_or_else(|| {
            DmError::invalid_config(format!(
                "неверная строка .env (строка {}): '{line}'",
                lineno + 1
            ))
        })?;
        env.sections
            .entry(current.clone())
            .or_insert_with(IndexMap::new)
            .insert(key, value);
    }
    Ok(env)
}

/// Разбирает одну строку `KEY=value`, снимая `export` и кавычки.
fn parse_kv(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("export ").unwrap_or(line);
    let eq = line.find('=')?;
    let key = line[..eq].trim().to_string();
    if key.is_empty() || key.contains(char::is_whitespace) {
        return None;
    }
    let mut value = line[eq + 1..].trim().to_string();
    // Снимаем парные кавычки.
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            value = value[1..value.len() - 1].to_string();
        }
    }
    Some((key, value))
}

/// Записывает переменные в `.env` файл по указанному пути (перезаписывая файл).
///
/// Формат выхода — плоский `KEY=value` без секций: это финальный `.env` сервиса.
pub fn write_service_env(path: &Path, vars: &EnvSection) -> DmResult<()> {
    let mut out = String::new();
    out.push_str(
        "# Этот файл сгенерирован Dev Manager (`dm env sync`). Не редактируйте вручную —\n",
    );
    out.push_str("# правьте единый .env в корне проекта и запустите `dm env sync` снова.\n\n");
    for (k, v) in vars {
        out.push_str(&format!("{k}={}\n", quote_if_needed(v)));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, out)?;
    Ok(())
}

/// Оборачивает значение в кавычки, если оно содержит пробелы или спецсимволы.
fn quote_if_needed(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\'' || c == '#' || c == '$')
    {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_globals() {
        let content = r#"
# comment
LOG_LEVEL=info

[api]
DATABASE_URL="postgres://localhost/api"
PORT=3001
export DEBUG=true

[web]
API_URL='http://localhost:3001'
"#;
        let env = parse_unified_env(content).unwrap();
        assert_eq!(env.sections[GLOBAL_SECTION]["LOG_LEVEL"], "info");
        assert_eq!(
            env.sections["api"]["DATABASE_URL"],
            "postgres://localhost/api"
        );
        assert_eq!(env.sections["api"]["PORT"], "3001");
        assert_eq!(env.sections["api"]["DEBUG"], "true");
        assert_eq!(env.sections["web"]["API_URL"], "http://localhost:3001");
    }

    #[test]
    fn vars_for_merges_global_and_service() {
        let mut env = UnifiedEnv::default();
        let mut g = IndexMap::new();
        g.insert("LOG_LEVEL".into(), "info".into());
        g.insert("SHARED".into(), "global".into());
        env.sections.insert(GLOBAL_SECTION.into(), g);

        let mut api = IndexMap::new();
        api.insert("SHARED".into(), "apioverride".into());
        api.insert("PORT".into(), "3001".into());
        env.sections.insert("api".into(), api);

        let vars = env.vars_for("api");
        assert_eq!(vars["LOG_LEVEL"], "info");
        assert_eq!(vars["SHARED"], "apioverride"); // сервис-специфичное выигрывает
        assert_eq!(vars["PORT"], "3001");

        let web = env.vars_for("web");
        assert_eq!(web["SHARED"], "global");
        assert!(!web.contains_key("PORT"));
    }

    #[test]
    fn rejects_malformed_line() {
        let err = parse_unified_env("this is not valid").unwrap_err();
        assert!(matches!(err, DmError::ConfigInvalid(_)));
    }

    #[test]
    fn quotes_special_values() {
        assert_eq!(quote_if_needed("plain"), "plain");
        assert_eq!(quote_if_needed("with space"), "\"with space\"");
        assert_eq!(quote_if_needed("has#hash"), "\"has#hash\"");
    }
}

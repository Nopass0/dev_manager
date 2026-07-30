//! Доменная модель проекта и микросервисов.
//!
//! Содержит типы, описывающие язык/стек сервиса и удобные операции над ним.
//! Конкретная конфигурация из YAML живёт в [`crate::config`], а здесь —
//! стабильные доменные сущности, не зависящие от формата сериализации.

use serde::{Deserialize, Serialize};

/// Язык или стек программирования, известные Dev Manager.
///
/// От значения напрямую зависит:
/// - автоопределение команды запуска ([`ServiceLanguage::default_run_command`]);
/// - выбор грамматики tree-sitter в `dm-analysis`;
/// - эвристики линтера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceLanguage {
    /// Язык Rust (cargo).
    #[default]
    Rust,
    /// Go (go run / go test).
    Go,
    /// C (gcc/clang).
    C,
    /// C++ (g++/clang++).
    Cpp,
    /// C# / .NET (dotnet).
    Csharp,
    /// Чистый JavaScript (node).
    JavaScript,
    /// TypeScript (через ts-node/vite/next).
    TypeScript,
    /// Bun runtime.
    Bun,
    /// Node.js (как платформа-менеджер пакетов).
    Nodejs,
    /// Lua.
    Lua,
    /// Python.
    Python,
    /// Vite-фронтенд.
    Vite,
    /// Next.js (React framework).
    Nextjs,
    /// Remix (React framework).
    Remix,
    /// Прочий/неизвестный стек — команда запуска задаётся явно в `run`.
    Other,
}

impl ServiceLanguage {
    /// Возвращает разумную команду запуска по умолчанию для языка.
    ///
    /// Возвращается `None`, если для языка нет общепринятой однострочной команды
    /// (например, для [`ServiceLanguage::Other`] — запуск должен быть задан явно).
    pub fn default_run_command(self) -> Option<&'static str> {
        match self {
            ServiceLanguage::Rust => Some("cargo run"),
            ServiceLanguage::Go => Some("go run ."),
            ServiceLanguage::C => Some("cc main.c -o main && ./main"),
            ServiceLanguage::Cpp => Some("c++ main.cpp -o main && ./main"),
            ServiceLanguage::Csharp => Some("dotnet run"),
            ServiceLanguage::JavaScript => Some("node ."),
            ServiceLanguage::TypeScript => Some("ts-node src/index.ts"),
            ServiceLanguage::Bun => Some("bun run dev"),
            ServiceLanguage::Nodejs => Some("npm run dev"),
            ServiceLanguage::Lua => Some("lua main.lua"),
            ServiceLanguage::Python => Some("python -m ."),
            ServiceLanguage::Vite => Some("npm run dev"),
            ServiceLanguage::Nextjs => Some("npm run dev"),
            ServiceLanguage::Remix => Some("npm run dev"),
            ServiceLanguage::Other => None,
        }
    }

    /// Человекочитаемая метка языка (для логов и статуса).
    pub fn label(self) -> &'static str {
        match self {
            ServiceLanguage::Rust => "rust",
            ServiceLanguage::Go => "go",
            ServiceLanguage::C => "c",
            ServiceLanguage::Cpp => "cpp",
            ServiceLanguage::Csharp => "csharp",
            ServiceLanguage::JavaScript => "javascript",
            ServiceLanguage::TypeScript => "typescript",
            ServiceLanguage::Bun => "bun",
            ServiceLanguage::Nodejs => "nodejs",
            ServiceLanguage::Lua => "lua",
            ServiceLanguage::Python => "python",
            ServiceLanguage::Vite => "vite",
            ServiceLanguage::Nextjs => "nextjs",
            ServiceLanguage::Remix => "remix",
            ServiceLanguage::Other => "other",
        }
    }

    /// Список расширений файлов исходников, характерных для языка.
    ///
    /// Используется watcher'ом и анализатором, чтобы фильтровать релевантные
    /// изменения и игнорировать (например) `node_modules` и `target`.
    pub fn source_extensions(self) -> &'static [&'static str] {
        match self {
            ServiceLanguage::Rust => &["rs"],
            ServiceLanguage::Go => &["go"],
            ServiceLanguage::C => &["c", "h"],
            ServiceLanguage::Cpp => &["cpp", "cc", "cxx", "hpp", "hh"],
            ServiceLanguage::Csharp => &["cs"],
            ServiceLanguage::JavaScript => &["js", "jsx", "mjs", "cjs"],
            ServiceLanguage::TypeScript => &["ts", "tsx"],
            ServiceLanguage::Bun => &["ts", "js", "tsx", "jsx"],
            ServiceLanguage::Nodejs => &["js", "ts"],
            ServiceLanguage::Lua => &["lua"],
            ServiceLanguage::Python => &["py"],
            // Фронтенд-фреймворки в основном пишут на TS/JS.
            ServiceLanguage::Vite | ServiceLanguage::Nextjs | ServiceLanguage::Remix => {
                &["ts", "tsx", "js", "jsx"]
            }
            ServiceLanguage::Other => &[],
        }
    }
}

/// Развёрнутое представление сервиса в runtime.
///
/// В отличие от [`crate::config::ServiceConfig`] (который — это «как написано в
/// YAML»), `Service` хранит уже разрешённые абсолютные пути и финальную команду
/// запуска. Создаётся из конфига один раз при старте `dm start`.
#[derive(Debug, Clone)]
pub struct Service {
    /// Имя сервиса (ключ в `services:`).
    pub name: String,
    /// Абсолютный путь к каталогу сервиса.
    pub path: std::path::PathBuf,
    /// Язык/стек.
    pub language: ServiceLanguage,
    /// Команда запуска (уже определённая, явная или авто).
    pub run_command: String,
    /// Включён ли watcher.
    pub watch: bool,
    /// Перезапускать ли при изменениях.
    pub restart_on_change: bool,
    /// Каталог git-репозитория (если отличается от корневого).
    pub repo_path: Option<std::path::PathBuf>,
    /// Задержка (мс) перед запуском сервиса в очереди.
    pub delay_ms: u64,
}

/// Доменный объект «проект» — набор разрешённых сервисов плюс метаданные.
#[derive(Debug, Clone)]
pub struct Project {
    /// Человекочитаемое имя.
    pub name: String,
    /// Абсолютный путь к корню (где лежит `dm.yaml`).
    pub root: std::path::PathBuf,
    /// Сервисы в порядке запуска.
    pub services: Vec<Service>,
}

impl Project {
    /// Находит сервис по имени. Возвращает `None`, если такого нет.
    pub fn find_service(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|s| s.name == name)
    }
}

impl Service {
    /// Человекочитаемая метка языка для отображения в логах/статусе.
    pub fn language_label(&self) -> &'static str {
        self.language.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_run_commands() {
        assert_eq!(
            ServiceLanguage::Rust.default_run_command(),
            Some("cargo run")
        );
        assert_eq!(ServiceLanguage::Other.default_run_command(), None);
    }

    #[test]
    fn parses_from_yaml_lowercase() {
        let yaml = "rust";
        let lang: ServiceLanguage = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(lang, ServiceLanguage::Rust);

        let yaml = "typescript";
        let lang: ServiceLanguage = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(lang, ServiceLanguage::TypeScript);
    }
}

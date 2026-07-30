//! # dm-core
//!
//! Ядро [`Dev Manager`](..). Содержит фундаментальные типы и операции, на которые
//! опираются все остальные crate'ы workspace:
//!
//! - [`config`] — разбор и валидация файла `dm.yaml`.
//! - [`env`] — единый `.env` в корне проекта и его распределение по сервисам.
//! - [`project`] — доменная модель проекта и микросервисов.
//! - [`paths`] — кросс-платформенные помощники для работы с путями.
//! - [`error`] — единый тип ошибок [`DmError`] и псевдоним [`DmResult`].
//!
//! Crate намеренно не имеет тяжёлых зависимостей (токio, tree-sitter и т.д.),
//! чтобы быть дешёвым фундаментом для всего остального.

pub mod config;
pub mod env;
pub mod error;
pub mod paths;
pub mod project;

/// Переэкспорт часто используемых типов для удобства потребителей.
pub use config::{
    CONFIG_FILENAME, CONFIG_FILENAME_ALT, Config, DeployTarget, HealthCheckConfig, HealthCheckKind,
    LinterConfig, ProfileConfig, RestartBehavior, RestartPolicy, RuntimeConfig, ServiceConfig,
    TestsConfig,
};
pub use env::{EnvSection, UnifiedEnv, parse_unified_env, write_service_env};
pub use error::{DmError, DmResult};
pub use project::{Project, Service, ServiceLanguage};

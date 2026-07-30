//! Разбор и валидация файла `dm.yaml`.
//!
//! Модуль состоит из двух частей:
//! - [`schema`] — serde-структуры, точно отражающие схему конфигурации.
//! - [`loader`] — поиск файла вверх по дереву, чтение и валидация.

pub mod loader;
pub mod schema;

pub use loader::{
    CONFIG_FILENAME, CONFIG_FILENAME_ALT, DM_ENV_VAR, deep_merge, discover_config,
    env_overlay_filename, load_config, load_resolved_config,
};
pub use schema::{
    Config, DatabaseConfig, DatabaseConnection, DeployTarget, DeployTrigger, DockerConfig,
    HealthCheckConfig, HealthCheckKind, LinterConfig, LogsConfig, ProfileConfig, ResourceAction,
    ResourceLimits, RestartBehavior, RestartPolicy, RuntimeConfig, ServiceConfig, TestsConfig,
};

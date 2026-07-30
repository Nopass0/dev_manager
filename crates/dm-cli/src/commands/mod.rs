//! Подкоманды `dm`. Каждая подкоманда — отдельный модуль с функцией `run`.
//!
//! Аргументы clap-структуры собраны в этом модуле и переиспользуются командами.

use dm_core::config::{discover_config, load_resolved_config};
use dm_core::{Config, DmResult};
use std::path::PathBuf;

pub mod alias;
pub mod build;
pub mod cache;
pub mod clean;
pub mod commit;
pub mod completions;
pub mod config;
pub mod dashboard;
pub mod db;
pub mod deploy;
pub mod deps;
pub mod docker;
pub mod doctor;
pub mod env;
pub mod exec;
pub mod format;
pub mod generate;
pub mod git;
pub mod grep;
pub mod history;
pub mod hooks;
pub mod init;
pub mod install;
pub mod kill;
pub mod lint;
pub mod list;
pub mod logs;
pub mod new;
pub mod open;
pub mod ping;
pub mod ports;
pub mod push;
pub mod refs;
pub mod release;
pub mod replace;
pub mod restart;
pub mod secrets;
pub mod setup;
pub mod shell;
pub mod start;
pub mod status;
pub mod stop;
pub mod test;
pub mod todo;
pub mod top;
pub mod update;
pub mod url;
pub mod watch;

/// Префикс системных сообщений самого `dm` (а не сервиса).
pub const PREFIX_SYS: &str = "[dm]";

/// Аргументы `dm start`.
#[derive(Debug, Clone, clap::Args)]
pub struct StartArgs {
    /// Отключить file-watching (без hot reload).
    #[arg(long)]
    pub no_watch: bool,
    /// Не перезапускать упавшие процессы.
    #[arg(long)]
    pub no_restart: bool,
    /// Запустить только указанные сервисы (через запятую): `--only=api,web`.
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,
    /// Пропустить указанные сервисы: `--skip=db`.
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<String>,
    /// Запустить сервисы с тегом: `--tag=backend`.
    #[arg(long, value_delimiter = ',')]
    pub tag: Vec<String>,
    /// Имя профиля из секции `profiles:`: `--profile=min`.
    #[arg(long)]
    pub profile: Option<String>,
    /// Запустить только сервисы, затронутые изменениями в git (через граф импортов).
    #[arg(long)]
    pub affected: bool,
    /// Не стартовать, а показать, какие сервисы были бы запущены.
    #[arg(long)]
    pub dry_run: bool,
    /// Дождаться health-check всех запускаемых сервисов перед выходом.
    #[arg(long)]
    pub wait: bool,
}

/// Аргументы `dm commit`.
#[derive(Debug, Clone, clap::Args)]
pub struct CommitArgs {
    /// Цель коммита: имя сервиса или `auto` для авто-сообщения.
    /// Если опущен и репозиториев несколько — коммитим во все.
    pub target: Option<String>,
    /// Текст сообщения коммита. Для `auto` — игнорируется.
    pub message: Option<String>,
}

/// Аргументы команды, работающей по сервисам (test/lint).
#[derive(Debug, Clone, clap::Args)]
pub struct TargetArgs {
    /// Имя сервиса (опущен → все сервисы).
    pub name: Option<String>,
}

/// Подкоманды `dm cache`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum CacheAction {
    /// Очистить кэш сборок.
    Clear,
}

/// Подкоманды `dm env`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum EnvAction {
    /// Распределить единый .env по сервисам.
    Sync,
}

/// Аргументы `dm ports`.
#[derive(Debug, Clone, clap::Args)]
pub struct PortsArgs {
    /// Освободить указанный порт (завершить занявший его процесс).
    #[arg(long)]
    pub free: Option<u16>,
}

/// Аргументы `dm kill`.
#[derive(Debug, Clone, clap::Args)]
pub struct KillArgs {
    /// Цель: PID, номер порта или имя процесса/сервиса.
    pub target: String,
}

/// Аргументы `dm open`.
#[derive(Debug, Clone, clap::Args)]
pub struct OpenArgs {
    /// Что открыть: имя сервиса, `docs`, `admin`, либо URL.
    pub target: String,
}

/// Аргументы `dm exec`.
#[derive(Debug, Clone, clap::Args)]
pub struct ExecArgs {
    /// Имя сервиса, в контексте которого выполнить команду.
    pub service: String,
    /// Команда (всё после `--`). Пример: `dm exec api -- psql -c '\\d'`.
    #[arg(last = true)]
    pub command: Vec<String>,
}

/// Аргументы `dm deps`.
#[derive(Debug, Clone, clap::Args)]
pub struct DepsArgs {
    /// Подкоманда deps.
    #[command(subcommand)]
    pub action: DepsAction,
}

/// Подкоманды `dm deps`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum DepsAction {
    /// Аудит безопасности зависимостей.
    Audit,
    /// Поиск устаревших зависимостей.
    Outdated,
}

/// Аргументы `dm release`.
#[derive(Debug, Clone, clap::Args)]
pub struct ReleaseArgs {
    /// Тип SemVer-bump: patch | minor | major.
    pub kind: String,
    /// Сгенерировать changelog вместо тега (dry-run-режим).
    #[arg(long)]
    pub changelog_only: bool,
}

/// Аргументы `dm grep`.
#[derive(Debug, Clone, clap::Args)]
pub struct GrepArgs {
    /// Что искать.
    pub pattern: String,
    /// Игнорировать регистр.
    #[arg(long, short = 'i')]
    pub ignore_case: bool,
    /// Только слова целиком.
    #[arg(long, short = 'w')]
    pub word: bool,
    /// Фильтр по расширениям (через запятую): `--type=rs,go`.
    #[arg(long, short = 't', value_delimiter = ',')]
    pub r#type: Vec<String>,
}

/// Аргументы `dm replace`.
#[derive(Debug, Clone, clap::Args)]
pub struct ReplaceArgs {
    /// Что искать.
    pub pattern: String,
    /// На что заменить.
    pub replacement: String,
    /// Игнорировать регистр.
    #[arg(long, short = 'i')]
    pub ignore_case: bool,
    /// Только слова целиком.
    #[arg(long, short = 'w')]
    pub word: bool,
    /// Не записывать изменения, только показать затронутые файлы.
    #[arg(long)]
    pub dry_run: bool,
}

/// Аргументы `dm hooks`.
#[derive(Debug, Clone, clap::Args)]
pub struct HooksArgs {
    /// Действие: install | uninstall | run.
    pub action: String,
    /// Какой хук (pre-commit | pre-push | все). По умолчанию pre-commit.
    #[arg(long, default_value = "pre-commit")]
    pub hook: String,
}

/// Аргументы `dm watch`.
#[derive(Debug, Clone, clap::Args)]
pub struct WatchArgs {
    /// Имя сервиса (каталог для watcher'а). Опущен — корень проекта.
    pub service: Option<String>,
    /// Команда для запуска при изменении (всё после `--`).
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

/// Аргументы `dm config`.
#[derive(Debug, Clone, clap::Args)]
pub struct ConfigArgs {
    /// Подкоманда.
    #[command(subcommand)]
    pub action: ConfigAction,
}

/// Подкоманды `dm config`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum ConfigAction {
    /// Показать весь dm.yaml.
    List,
    /// Получить значение по dotted-key пути: `services.api.path`.
    Get { key: String },
    /// Открыть dm.yaml в редакторе ($EDITOR).
    Edit,
    /// Проверить корректность конфига.
    Validate,
}

/// Аргументы `dm init`.
#[derive(Debug, Clone, clap::Args)]
pub struct InitArgs {
    /// Создать проект из шаблона (без = только dm.yaml).
    /// Доступно: bun-elysia, bun-express, go-api, rust-axum, next-shadcn,
    /// react-vite, python-fastapi. Список: `dm init --list-templates`.
    #[arg(long)]
    pub template: Option<String>,
    /// Показать список доступных шаблонов и выйти.
    #[arg(long)]
    pub list_templates: bool,
    /// Имя проекта/сервиса для шаблона (по умолчанию — имя каталога).
    #[arg(long)]
    pub name: Option<String>,
}

/// Аргументы `dm new`.
#[derive(Debug, Clone, clap::Args)]
pub struct NewArgs {
    /// Что создать: service.
    pub kind: String,
    /// Имя.
    pub name: String,
    /// Язык: rust|go|typescript|vite|... (совместимо со старым синтаксисом).
    #[arg(long)]
    pub lang: Option<String>,
    /// Шаблон сервиса: bun-elysia, go-api, rust-axum и т.д. (приоритет над --lang).
    #[arg(long)]
    pub template: Option<String>,
}

/// Аргументы `dm git` (cross-repo операции).
#[derive(Debug, Clone, clap::Args)]
pub struct GitArgs {
    /// Подкоманда.
    #[command(subcommand)]
    pub action: GitAction,
}

/// Подкоманды `dm git`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum GitAction {
    /// Спрятать изменения во всех репозиториях.
    Stash,
    /// Переключить/создать ветку во всех репозиториях.
    Branch { name: String },
    /// Ребейзить текущие ветки на указанную.
    Rebase { onto: String },
}

/// Аргументы `dm db`.
#[derive(Debug, Clone, clap::Args)]
pub struct DbArgs {
    /// Подкоманда.
    #[command(subcommand)]
    pub action: DbAction,
    /// Имя подключения из database.connections (по умолчанию первое/default).
    #[arg(long)]
    pub conn: Option<String>,
}

/// Подкоманды `dm db`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum DbAction {
    /// Применить миграции.
    Migrate,
    /// Накатить тестовые данные (seed).
    Seed,
    /// Пересоздать схему: откат + миграции + seed.
    Reset,
    /// Открыть интерактивный клиент БД (psql/redis-cli/...).
    Shell,
}

/// Аргументы `dm docker`.
#[derive(Debug, Clone, clap::Args)]
pub struct DockerArgs {
    /// Подкоманда.
    #[command(subcommand)]
    pub action: DockerAction,
}

/// Подкоманды `dm docker`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum DockerAction {
    /// Поднять compose-инфраструктуру.
    Up,
    /// Остановить compose-инфраструктуру.
    Down,
    /// Хвост логов контейнеров.
    Logs,
    /// Список контейнеров проекта.
    Ps,
}

/// Аргументы `dm build`.
#[derive(Debug, Clone, clap::Args)]
pub struct BuildArgs {
    /// Сервис для сборки (опущен — все).
    pub service: Option<String>,
    /// Release-сборка (оптимизация).
    #[arg(long)]
    pub release: bool,
}

/// Аргументы `dm gen`.
#[derive(Debug, Clone, clap::Args)]
pub struct GenArgs {
    /// Что генерировать: diagram | docs.
    pub kind: String,
}

/// Аргументы `dm clean`.
#[derive(Debug, Clone, clap::Args)]
pub struct CleanArgs {
    /// Что чистить: all | cache | branches | docker. По умолчанию all.
    #[arg(long, default_value = "all")]
    pub target: String,
    /// Не спрашивать подтверждение.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

/// Находит `dm.yaml` от текущего каталога, разбирает и возвращает конфиг и корень.
///
/// Используется большинством команд как первый шаг. Корень нужен для разрешения
/// относительных путей сервисов.
pub fn load_project_config() -> DmResult<(Config, PathBuf)> {
    let cwd = std::env::current_dir().map_err(|e| {
        dm_core::DmError::Process(format!("не удалось определить текущий каталог: {e}"))
    })?;
    let config_path = discover_config(&cwd)?;
    let root = config_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let config = load_resolved_config(&config_path, None)?;
    Ok((config, root))
}

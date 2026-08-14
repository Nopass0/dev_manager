//! Serde-схема файла `dm.yaml`.
//!
//! Каждое поле имеет `#[serde(default)]`, где это разумно, чтобы конфиг был
//! минимальным и расширялся по мере необходимости. Это обеспечивает обратную
//! совместимость: добавление новых полей не ломает старые конфиги.

use crate::project::ServiceLanguage;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Корневая структура конфигурации Dev Manager.
///
/// Один `dm.yaml` описывает весь монорепозиторий (или мультрепозиторий) целиком.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Версия схемы. Поддерживается только `1`. Защищает от молчаливого
    /// игнорирования полей при будущих несовместимых изменениях.
    #[serde(default = "default_version")]
    pub version: u32,

    /// Человекочитаемое имя проекта — используется в логах и статусе.
    #[serde(default)]
    pub project_name: String,

    /// Путь к базовому конфигу для наследования (`extends`).
    ///
    /// Загружается **до** текущего; поля текущего конфига перекрывают базовый
    /// (deep-merge по сервисам и картам, скаляры — «последний выигрывает»).
    /// Путь относительно каталога текущего файла.
    #[serde(default)]
    pub extends: Option<String>,

    /// Имя окружения, для которого загружен конфиг (например `dev`, `staging`).
    ///
    /// Заполняется загрузчиком при разрешении `dm.<env>.yaml`; в самом файле не
    /// задаётся. Используется командами для логирования и выбора веток логики.
    #[serde(default, skip_serializing)]
    pub env: String,

    /// Путь к единому `.env` относительно корня конфига. По умолчанию `.env`.
    #[serde(default = "default_env_file")]
    pub env_file: String,

    /// Глобальные значения по умолчанию для всех сервисов (deep-merged в каждый).
    ///
    /// Поддерживает те же поля, что и [`ServiceConfig`]; отсутствующие поля
    /// берутся отсюда. Удобно задать общий `language`, `watch`, `restart_policy`.
    #[serde(default)]
    pub defaults: Option<Box<ServiceConfig>>,

    /// Карта сервисов: имя → настройки. `IndexMap` сохраняет порядок объявления
    /// (важно для предсказуемого отображения в статусе).
    #[serde(default)]
    pub services: IndexMap<String, ServiceConfig>,

    /// Цели деплоя по SSH. Пусто — деплой не настроен.
    #[serde(default)]
    pub deploy: Vec<DeployTarget>,

    /// Настройки анализатора кода (DRY/KISS/unused-проверки).
    #[serde(default)]
    pub linter: LinterConfig,

    /// Профили запуска — именованные наборы сервисов.
    ///
    /// Позволяют одной командой поднять разный набор: `dm start --profile=min`.
    #[serde(default)]
    pub profiles: IndexMap<String, ProfileConfig>,

    /// Глобальные настройки запуска (параллелизм, поведение restart).
    #[serde(default)]
    pub runtime: RuntimeConfig,

    /// Настройки базы данных (миграции, seed, shell).
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Настройки Docker/Compose инфраструктуры.
    #[serde(default)]
    pub docker: DockerConfig,

    /// Настройки сборочного пайплайна (multi-stage build в единую папку).
    #[serde(default)]
    pub build: BuildConfig,

    /// Настройки уведомлений (webhook + desktop).
    ///
    /// NB: сам тип живёт в `dm-runtime`, здесь — лишь raw-map для серилизации,
    /// чтобы dm-core не зависел от runtime. Загрузчик прокидывает значения в
    /// `dm_runtime::notify::NotifyConfig`.
    #[serde(default)]
    pub notify: serde_yaml::Mapping,

    /// Пользовательские алиасы-шорткаты: имя → shell-команда.
    ///
    /// Выполняются через `dm alias <name>` в корне проекта.
    /// ```yaml
    /// aliases:
    ///   dbq: "dm db shell"
    ///   re: "dm restart api"
    /// ```
    #[serde(default)]
    pub aliases: IndexMap<String, String>,
}

/// Конфигурация работы с БД (миграции/seed/shell/snapshot).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct DatabaseConfig {
    /// Дочерняя карта по имени БД. Ключ — имя (например `api`, `analytics`).
    #[serde(default)]
    pub connections: IndexMap<String, DatabaseConnection>,
}

/// Одно подключение к БД и связанные команды.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConnection {
    /// Тип БД: `postgres`, `mysql`, `sqlite`, `mongo`, `redis`.
    #[serde(default)]
    pub kind: String,
    /// URL подключения (поддерживает интерполяцию `${VAR}`).
    #[serde(default)]
    pub url: String,
    /// Каталог с миграциями (относительно корня проекта).
    #[serde(default)]
    pub migrations_dir: Option<String>,
    /// Команда apply миграций (если пусто — угадываем по kind).
    #[serde(default)]
    pub migrate_cmd: Option<String>,
    /// Файл/команда сидинга тестовых данных.
    #[serde(default)]
    pub seed_cmd: Option<String>,
}

/// Конфигурация Docker/Compose.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DockerConfig {
    /// Путь к compose-файлу относительно корня проекта (по умолчанию `docker-compose.yml`).
    #[serde(default = "default_compose_file")]
    pub compose_file: String,
    /// Имя compose-проекта (переопределяет `-p`).
    #[serde(default)]
    pub project_name: Option<String>,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            compose_file: default_compose_file(),
            project_name: None,
        }
    }
}

fn default_compose_file() -> String {
    "docker-compose.yml".to_string()
}

/// Сборочный пайплайн: упорядоченные этапы сборки артефактов в единую папку.
///
/// Позволяет собирать multi-language проект (например, DLL на C++ + приложение
/// на Rust) в один чистый каталог `output_dir` с правильным порядком.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct BuildConfig {
    /// Куда складывать готовые артефакты (относительно корня проекта).
    /// По умолчанию `dist/`. Каталог очищается перед сборкой.
    #[serde(default = "default_build_output")]
    pub output_dir: String,
    /// Упорядоченные этапы сборки. Каждый этап — команда + артефакты для копирования.
    #[serde(default)]
    pub stages: Vec<BuildStage>,
    /// Очищать ли output_dir перед сборкой (по умолчанию true).
    #[serde(default = "default_true_bool")]
    pub clean: bool,
}

fn default_build_output() -> String {
    "dist".to_string()
}
fn default_true_bool() -> bool {
    true
}

/// Один этап сборочного пайплайна.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct BuildStage {
    /// Имя этапа (для логов).
    #[serde(default)]
    pub name: String,
    /// Сервис/каталог, где выполнять команду (имя сервиса из `services:` или путь).
    #[serde(default)]
    pub source: String,
    /// Команда сборки (например, `cargo build --release` или `nasm -f win64`).
    #[serde(default)]
    pub command: String,
    /// Glob-шаблоны артефактов для копирования в output_dir
    /// (например, `target/release/*.dll`, `target/release/myapp.exe`).
    /// Относительно каталога source.
    #[serde(default)]
    pub artifacts: Vec<String>,
    /// Подкаталог внутри output_dir для артефактов этого этапа
    /// (пусто = корень output_dir).
    #[serde(default)]
    pub dest_subdir: String,
    /// Этапы, которые должны успешно завершиться до этого (по имени).
    /// Обеспечивает порядок: lib → dll → app.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Lua-скрипт, выполняемый ПОСЛЕ успешной сборки этапа (путь к .lua).
    #[serde(default)]
    pub on_success: Option<String>,
    /// Lua-скрипт, выполняемый при ПРОВАЛЕ этапа (путь к .lua).
    #[serde(default)]
    pub on_failure: Option<String>,
}

/// Хуки жизненного цикла сервиса (Lua-скрипты или команды).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct HooksConfig {
    /// Lua-скрипт или shell-команда ПЕРЕД запуском сервиса.
    /// Для Lua укажите путь к .lua файлу; для shell — команду.
    #[serde(default)]
    pub before_start: Vec<String>,
    /// Lua-скрипт ПОСЛЕ успешного health-check.
    #[serde(default)]
    pub after_start: Vec<String>,
    /// Lua-скрипт ПОСЛЕ сборки.
    #[serde(default)]
    pub after_build: Vec<String>,
    /// Lua-скрипт ПОСЛЕ тестов (запускается независимо от результата).
    #[serde(default)]
    pub after_test: Vec<String>,
    /// Проверять/устанавливать зависимости перед каждым запуском.
    /// Выполняет `install_cmd` если файл-маркер dependencies_file отсутствует.
    #[serde(default)]
    pub check_deps: bool,
    /// Команда установки зависимостей (например, `npm ci`, `cargo fetch`).
    #[serde(default)]
    pub install_cmd: Option<String>,
    /// Файл-маркер наличия зависимостей (например, `node_modules`, `Cargo.lock`).
    #[serde(default)]
    pub deps_marker: Option<String>,
}

fn default_version() -> u32 {
    1
}
fn default_env_file() -> String {
    ".env".to_string()
}

impl Config {
    /// Возвращает список имён сервисов, отсортированный по `order` (возрастание).
    /// При равенстве `order` сохраняется порядок объявления в YAML.
    ///
    /// Это канонический порядок запуска сервисов в `dm start`.
    pub fn services_in_start_order(&self) -> Vec<String> {
        let mut names: Vec<(String, ServiceConfig)> = self
            .services
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        names.sort_by_key(|(_, cfg)| cfg.order);
        names.into_iter().map(|(k, _)| k).collect()
    }
}

/// Настройки отдельного микросервиса.
///
/// `path` сделан опциональным (с дефолтом пустой строкой), чтобы конфиги-оверлеи
/// (`extends`/`dm.<env>.yaml`) могли.partialно перекрывать отдельные поля без
/// дублирования обязательных. Пустой `path` ловится в [`Config::validate`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    /// Каталог сервиса относительно корня конфига.
    #[serde(default)]
    pub path: String,

    /// Основной язык/стек — влияет на автоопределение команды запуска.
    /// Основной язык/стек — влияет на автоопределение команды запуска.
    ///
    /// Опциональное с дефолтом `Rust`, чтобы конфиги-оверлеи могли перекрывать
    /// отдельные поля без указания языка (он наследуется из базы).
    #[serde(default)]
    pub language: ServiceLanguage,

    /// Путь к git-репозиторию (для multi-repo commit/push). Если опущен —
    /// используется корневой репозиторий проекта.
    #[serde(default)]
    pub repo: Option<String>,

    /// Явная команда запуска. Если не задана — выводится из `language`/`path`.
    #[serde(default)]
    pub run: Option<String>,

    /// Включить file-watcher для этого сервиса.
    #[serde(default = "default_true")]
    pub watch: bool,

    /// Перезапускать сервис при изменении файлов (hot reload).
    #[serde(default = "default_true")]
    pub restart_on_change: bool,

    /// Задержка (мс) перед запуском этого сервиса в очереди запуска.
    #[serde(default)]
    pub delay_ms: u64,

    /// Приоритет в очереди запуска: меньше = раньше. По умолчанию `100`.
    #[serde(default = "default_order")]
    pub order: i32,

    /// Дополнительные переменные окружения. Поддерживает шаблоны `{{svc.VAR}}`.
    #[serde(default)]
    pub env: IndexMap<String, String>,

    /// Настройки тестов для этого сервиса.
    #[serde(default)]
    pub tests: TestsConfig,

    /// Настройки логирования сервиса.
    #[serde(default)]
    pub logs: LogsConfig,

    /// Сервисы, от которых зависит этот: не стартует, пока они не станут healthy.
    ///
    /// Имена других сервисов из `services:`. Образуют DAG; циклы запрещены.
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Health-check, по которому считается, что сервис «поднялся».
    ///
    /// Используется для health-gated запуска зависимых сервисов и для `dm start --wait`.
    #[serde(default)]
    pub health: Option<HealthCheckConfig>,

    /// Произвольные теги для группировки: `dm start --tag=backend`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Политика перезапуска при падении.
    #[serde(default)]
    pub restart_policy: RestartPolicy,

    /// Хуки жизненного цикла: Lua-скрипты на события + проверка зависимостей.
    #[serde(default)]
    pub hooks: HooksConfig,

    /// Команды, выполняемые ПЕРЕД запуском основного процесса
    /// (например, миграции, генерация кода).
    #[serde(default)]
    pub before_start: Vec<String>,

    /// Команды, выполняемые ПОСЛЕ успешного health-check.
    #[serde(default)]
    pub after_start: Vec<String>,

    /// Лимиты ресурсов процесса (CPU/RAM). Best-effort на платформе.
    #[serde(default)]
    pub resources: Option<ResourceLimits>,

    /// Явное указание shell для запуска `run` (по умолчанию — системный).
    /// На Unix: `/bin/sh -c`, на Windows: `cmd /C`.
    #[serde(default)]
    pub shell: Option<String>,

    /// Рабочий каталог внутри `path` (если нужно отличаться от корня сервиса).
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Сервис активен только в этих окружениях (`env:`). Пусто = везде.
    ///
    /// Пример: `only_on: [dev, ci]` — в `prod` сервис не запустится.
    #[serde(default)]
    pub only_on: Vec<String>,
}

/// Лимиты ресурсов процесса (best-effort, платформенно-зависимо).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ResourceLimits {
    /// Максимум CPU в процентах (0 = без лимита). Windows Job Objects / cgroups.
    #[serde(default)]
    pub cpu_percent: u32,
    /// Максимум ОЗУ в мегабайтах (0 = без лимита). Мониторится watcher'ом.
    #[serde(default)]
    pub memory_mb: u64,
    /// Что делать при превышении memory_mb: `notify` (по умолчанию) или `kill`.
    #[serde(default)]
    pub on_exceed: ResourceAction,
}

/// Действие при превышении лимита ресурсов.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAction {
    /// Только уведомить (webhook/desktop), процесс не трогать.
    #[default]
    Notify,
    /// Убить процесс (supervisor перезапустит, если включён restart).
    Kill,
}

/// Конфигурация health-check для сервиса.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HealthCheckConfig {
    /// Тип проверки.
    #[serde(default)]
    pub kind: HealthCheckKind,
    /// URL для HTTP-проверки (например `http://localhost:3001/health`).
    #[serde(default)]
    pub url: Option<String>,
    /// TCP-порт для проверки готовности (если HTTP не нужен).
    #[serde(default)]
    pub port: Option<u16>,
    /// Сколько секунд ждать перед первой проверкой (warmup).
    #[serde(default = "default_health_warmup")]
    pub warmup_secs: u64,
    /// Сколько секунд между попытками.
    #[serde(default = "default_health_interval")]
    pub interval_secs: u64,
    /// Максимум попыток до признания сервиса упавшим.
    #[serde(default = "default_health_retries")]
    pub retries: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            kind: HealthCheckKind::Tcp,
            url: None,
            port: None,
            warmup_secs: default_health_warmup(),
            interval_secs: default_health_interval(),
            retries: default_health_retries(),
        }
    }
}

fn default_health_warmup() -> u64 {
    1
}
fn default_health_interval() -> u64 {
    2
}
fn default_health_retries() -> u32 {
    10
}

/// Тип health-проверки.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckKind {
    /// Проверка TCP-порта (соединение устанавливается = здоров).
    #[default]
    Tcp,
    /// HTTP GET по `url`, статус 2xx = здоров.
    Http,
    /// Никакой проверки; считается здоровым сразу после запуска процесса.
    None,
}

/// Политика перезапуска при падении процесса.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RestartPolicy {
    /// Что делать при падении: `always` (по умолчанию), `on-failure`, `never`.
    #[serde(default)]
    pub on_crash: RestartBehavior,
    /// Сколько секунд ждать перед повторным запуском.
    #[serde(default = "default_restart_delay")]
    pub delay_secs: u64,
    /// После стольких падений подряд — остановить сервис и сообщить причину
    /// (auto-recovery: бесконечный цикл рестартов прерывается).
    #[serde(default = "default_restart_max")]
    pub max_consecutive_crashes: u32,
}

impl RestartPolicy {
    /// Политика «никогда не перезапускать».
    pub fn never() -> Self {
        Self {
            on_crash: RestartBehavior::Never,
            delay_secs: 0,
            max_consecutive_crashes: 0,
        }
    }
}

fn default_restart_delay() -> u64 {
    2
}
fn default_restart_max() -> u32 {
    5
}

/// Поведение при крэше.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartBehavior {
    /// Перезапускать всегда.
    #[default]
    Always,
    /// Только при ненулевом exit code.
    OnFailure,
    /// Никогда не перезапускать.
    Never,
}

/// Именованный профиль запуска (набор сервисов).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ProfileConfig {
    /// Сервисы, входящие в профиль (по имени). Пусто = все.
    #[serde(default)]
    pub services: Vec<String>,
    /// Удобные алиасы-теги внутри профиля (необязательно).
    #[serde(default)]
    pub description: String,
}

/// Глобальные настройки запуска.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RuntimeConfig {
    /// Сколько сервисов запускать параллельно (0 = без лимита).
    #[serde(default = "default_max_parallel")]
    pub max_parallel: u32,
    /// Игнорировать эти расширения в watcher'е (smart-restart).
    #[serde(default = "default_watch_ignore_exts")]
    pub watch_ignore_extensions: Vec<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_parallel: default_max_parallel(),
            watch_ignore_extensions: default_watch_ignore_exts(),
        }
    }
}

fn default_max_parallel() -> u32 {
    0
}
fn default_watch_ignore_exts() -> Vec<String> {
    [
        "lock", "log", "tmp", "cache", "pid", "svg", "png", "jpg", "jpeg", "gif", "pdf", "md",
        "txt", "json.gz", "bin",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_true() -> bool {
    true
}
fn default_order() -> i32 {
    100
}

/// Конфигурация запуска тестов для сервиса.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct TestsConfig {
    /// Команда тестирования, например `cargo test` или `bun test`. Если пусто —
    /// тесты для сервиса отключены.
    #[serde(default)]
    pub cmd: Option<String>,

    /// Запускать тесты автоматически при каждом изменении файлов.
    #[serde(default)]
    pub on_change: bool,
}

/// Конфигурация логирования сервиса в общем потоке.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LogsConfig {
    /// Показывать ли логи этого сервиса в общем выводе.
    pub enabled: bool,
    /// Минимальный уровень: `error`, `warn`, `info`, `debug`, `trace`.
    pub level: String,
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
        }
    }
}

/// Цель деплоя по SSH.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DeployTarget {
    /// Человекочитаемое имя цели (используется в `dm deploy <name>`).
    pub name: String,
    /// Хост SSH.
    pub host: String,
    /// Пользователь SSH.
    #[serde(default = "default_deploy_user")]
    pub user: String,
    /// Порт SSH. По умолчанию 22.
    #[serde(default = "default_deploy_port")]
    pub port: u16,
    /// Путь к приватному ключу. `~` раскрывается в домашний каталог.
    #[serde(default)]
    pub key: Option<String>,
    /// Удалённый каталог, куда деплоимся.
    #[serde(default)]
    pub remote_dir: Option<String>,
    /// Когда запускать деплой автоматически.
    #[serde(default)]
    pub on: DeployTrigger,
    /// Последовательность shell-команд на удалённом хосте.
    #[serde(default)]
    pub steps: Vec<String>,
}

fn default_deploy_user() -> String {
    "deploy".to_string()
}
fn default_deploy_port() -> u16 {
    22
}

/// Триггер автоматического деплоя.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployTrigger {
    /// Деплой только по явной команде `dm deploy <name>`.
    #[default]
    Manual,
    /// После каждого успешного `dm push`.
    AfterPush,
    /// После каждого успешного `dm commit`.
    AfterCommit,
}

/// Настройки встроенного линтера/анализатора кода.
///
/// Все поля имеют `#[serde(default)]`, поэтому в dm.yaml можно указать только
/// нужные проверки — отсутствующие возьмутся из [`Default`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LinterConfig {
    /// Проверка принципа Don't Repeat Yourself.
    pub dr: bool,
    /// Проверка принципа Keep It Simple, Stupid.
    pub kiss: bool,
    /// Поиск неиспользуемого кода.
    pub unused_code: bool,
    /// Поиск дублирующихся определений (класс/функция с одним именем в разных файлах).
    pub duplicates: bool,
    /// Автоматически удалять найденный неиспользуемый код (без подтверждения).
    pub auto_fix: bool,
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            dr: true,
            kiss: true,
            unused_code: true,
            duplicates: true,
            auto_fix: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let yaml = r#"
project_name: demo
services:
  api:
    path: ./api
    language: rust
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.project_name, "demo");
        assert_eq!(cfg.services.len(), 1);
        assert_eq!(cfg.services["api"].language, ServiceLanguage::Rust);
        assert!(cfg.services["api"].watch); // default true
        assert_eq!(cfg.services["api"].order, 100); // default 100
    }

    #[test]
    fn start_order_sorts_by_order() {
        let yaml = r#"
services:
  first:
    path: ./a
    language: go
    order: 1
  second:
    path: ./b
    language: go
    order: 5
  third:
    path: ./c
    language: go
    order: 2
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        let order = cfg.services_in_start_order();
        assert_eq!(order, vec!["first", "third", "second"]);
    }

    #[test]
    fn linter_defaults_are_sensible() {
        let l = LinterConfig::default();
        assert!(l.dr && l.kiss && l.unused_code && l.duplicates);
        assert!(!l.auto_fix);
    }
}

//! # dm-cli
//!
//! Командный интерфейс Dev Manager. Парсит аргументы (`clap`), диспетчеризует
//! команды по модулям [`commands`] и печатает результат в консоль.
//!
//! Сам по себе этот crate не содержит `main` — точка входа живёт в `dm` crate
//! и просто вызывает [`Cli::parse`] и [`run`] отсюда.

pub mod commands;
pub mod output;
pub mod select;
pub mod shell;
pub mod templates;

use clap::{Parser, Subcommand};
use dm_core::DmResult;

/// Корневая структура аргументов CLI.
#[derive(Debug, Parser)]
#[command(
    name = "dm",
    version,
    about = "Dev Manager — единый менеджер разработки микросервисов",
    long_about = "Оркестрация процессов, git-автоматизация, анализ кода и деплой.\nСм. https://github.com/Nopass0/dev_manager",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Окружение конфига (`dev`, `staging`, `prod`...). Подгружает `dm.<env>.yaml`.
    /// Также читается из переменной `DM_ENV`.
    #[arg(long, global = true)]
    pub env: Option<String>,

    /// Подкоманда.
    #[command(subcommand)]
    pub command: Command,
}

/// Все поддерживаемые подкоманды `dm`.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Создать dm.yaml и/или проект из шаблона.
    Init(commands::InitArgs),
    /// Запустить все сервисы (с watcher/hot-reload).
    Start(commands::StartArgs),
    /// Остановить все сервисы (если запущены через `dm start`).
    Stop,
    /// Перезапустить конкретный сервис.
    Restart {
        /// Имя сервиса из `dm.yaml`.
        name: String,
    },
    /// Показать статус сервисов и git-состояние проекта.
    Status,
    /// Вывести логи сервисов (из текущего запуска `dm start`).
    Logs {
        /// Имя сервиса (если опущен — все).
        name: Option<String>,
    },
    /// Закоммитить изменения (во все репозитории или в указанный).
    Commit(commands::CommitArgs),
    /// Запушить все репозитории в их origin.
    Push,
    /// Запустить тесты сервисов.
    Test(commands::TargetArgs),
    /// Проверить код линтерами (DRY/KISS/unused/duplicates).
    Lint(commands::TargetArgs),
    /// Выполнить деплой по имени цели из `deploy:`.
    Deploy {
        /// Имя цели деплоя.
        name: String,
    },
    /// Очистить кэш сборок сервисов (target, node_modules/.cache…).
    Cache {
        #[command(subcommand)]
        action: commands::CacheAction,
    },
    /// Работа с единым `.env`.
    Env {
        #[command(subcommand)]
        action: commands::EnvAction,
    },
    /// Установить этот бинарник в PATH.
    Install,
    /// Вывести версию и информацию о сборке.
    Version,
    /// Диагностика окружения: версии инструментов, порты, конфликты.
    Doctor,
    /// Показать, кто занял порты; освободить порт.
    Ports(commands::PortsArgs),
    /// Завершить процесс по имени/порту/PID.
    Kill(commands::KillArgs),
    /// Открыть сервис/документацию в браузере или IDE.
    Open(commands::OpenArgs),
    /// Выполнить команду в контексте сервиса (с его env, cwd).
    Exec(commands::ExecArgs),
    /// Открыть интерактивную shell-сессию в каталоге сервиса.
    Shell { name: String },
    /// Интерактивный режим: метрики процессов по сервисам (htop-подобный).
    Top,
    /// Единый аудит зависимостей всех сервисов (cargo/npm/pip/go).
    Deps(commands::DepsArgs),
    /// Управление релизами: bump версии и auto-changelog.
    Release(commands::ReleaseArgs),
    /// Сгенерировать shell-completion или JSON-schema.
    Completions { shell: String },
    /// Поиск по коду (grep с фильтрами).
    Grep(commands::GrepArgs),
    /// Find & replace по всему проекту (с --dry-run).
    Replace(commands::ReplaceArgs),
    /// Найти все использования символа.
    Refs { symbol: String },
    /// Поиск потенциально утёкших секретов.
    Secrets,
    /// Отформатировать код всех сервисов (rustfmt/prettier/gofmt/black...).
    Format,
    /// Управление git-хуками.
    Hooks(commands::HooksArgs),
    /// Запускать команду при изменении файлов (универсальный watcher-runner).
    Watch(commands::WatchArgs),
    /// Управление dm.yaml из CLI.
    Config(commands::ConfigArgs),
    /// Скаффолд нового сервиса.
    New(commands::NewArgs),
    /// Live-дашборд всех сервисов (периодический refresh).
    Dashboard,
    /// Проверить доступность сервиса (ping).
    Ping { name: String },
    /// Показать URL сервиса.
    Url { name: String },
    /// Cross-repo git-операции (stash/branch sync/rebase).
    Git(commands::GitArgs),
    /// Работа с БД: миграции, seed, reset, shell.
    Db(commands::DbArgs),
    /// Управление Docker/Compose инфраструктурой.
    Docker(commands::DockerArgs),
    /// Унифицированная сборка всех сервисов.
    Build(commands::BuildArgs),
    /// Генерация артефактов (диаграммы, документация).
    Gen(commands::GenArgs),
    /// Умная очистка проекта (кэши, orphan-ветки, stale-контейнеры).
    Clean(commands::CleanArgs),
    /// Лента активности: что я делал (коммиты/тесты/деплои).
    History,
    /// Перечислить сервисы/профили/теги/dep-цели проекта.
    List { what: String },
    /// Bootstrap проекта: установить зависимости всех сервисов за раз.
    Setup,
    /// git pull во всех репозиториях проекта.
    Update,
    /// Реестр TODO/FIXME/HACK по коду с авторами (git blame).
    Todo,
    /// Пользовательские алиасы-шорткаты из ~/.dm/aliases или dm.yaml.
    Alias { name: String, args: Vec<String> },
}

/// Главная точка входа: выполняет выбранную команду.
///
/// Возвращает `DmResult`, который `main` превратит в код выхода.
pub async fn run(cli: Cli) -> DmResult<()> {
    // Сохраняем выбранное окружение в поток-локальную переменную для команд.
    if let Some(env) = &cli.env {
        // Также экспонируем через std::env, чтобы load_project_config подхватил.
        // Безопасно: мы в начале процесса.
        unsafe { std::env::set_var(dm_core::config::DM_ENV_VAR, env); }
    }
    match cli.command {
        Command::Init(args) => commands::init::run(args).await,
        Command::Start(args) => commands::start::run(args).await,
        Command::Stop => commands::stop::run().await,
        Command::Restart { name } => commands::restart::run(&name).await,
        Command::Status => commands::status::run().await,
        Command::Logs { name } => commands::logs::run(name.as_deref()).await,
        Command::Commit(args) => commands::commit::run(args).await,
        Command::Push => commands::push::run().await,
        Command::Test(args) => commands::test::run(args).await,
        Command::Lint(args) => commands::lint::run(args).await,
        Command::Deploy { name } => commands::deploy::run(&name).await,
        Command::Cache { action } => commands::cache::run(action).await,
        Command::Env { action } => commands::env::run(action).await,
        Command::Install => commands::install::run().await,
        Command::Version => {
            println!("dm {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Doctor => commands::doctor::run().await,
        Command::Ports(args) => commands::ports::run(args).await,
        Command::Kill(args) => commands::kill::run(args).await,
        Command::Open(args) => commands::open::run(args).await,
        Command::Exec(args) => commands::exec::run(args).await,
        Command::Shell { name } => commands::shell::run(&name).await,
        Command::Top => commands::top::run().await,
        Command::Deps(args) => commands::deps::run(args).await,
        Command::Release(args) => commands::release::run(args).await,
        Command::Completions { shell } => commands::completions::run(&shell),
        Command::Grep(args) => commands::grep::run(args).await,
        Command::Replace(args) => commands::replace::run(args).await,
        Command::Refs { symbol } => commands::refs::run(&symbol).await,
        Command::Secrets => commands::secrets::run().await,
        Command::Format => commands::format::run().await,
        Command::Hooks(args) => commands::hooks::run(args).await,
        Command::Watch(args) => commands::watch::run(args).await,
        Command::Config(args) => commands::config::run(args).await,
        Command::New(args) => commands::new::run(args).await,
        Command::Dashboard => commands::dashboard::run().await,
        Command::Ping { name } => commands::ping::run(&name).await,
        Command::Url { name } => commands::url::run(&name).await,
        Command::Git(args) => commands::git::run(args).await,
        Command::Db(args) => commands::db::run(args).await,
        Command::Docker(args) => commands::docker::run(args).await,
        Command::Build(args) => commands::build::run(args).await,
        Command::Gen(args) => commands::generate::run(args).await,
        Command::Clean(args) => commands::clean::run(args).await,
        Command::History => commands::history::run().await,
        Command::List { what } => commands::list::run(&what).await,
        Command::Setup => commands::setup::run().await,
        Command::Update => commands::update::run().await,
        Command::Todo => commands::todo::run().await,
        Command::Alias { name, args } => commands::alias::run(&name, &args).await,
    }
}

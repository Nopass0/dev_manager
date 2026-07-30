//! Точка входа `dm` — тонкая обёртка над `dm_cli`.
//!
//! Весь разбор аргументов и логика живут в `dm-cli`. Здесь только инициализация
//! логирования, парсинг и корректный код выхода.

use clap::Parser;
use dm_cli::{run, Cli};

#[tokio::main]
async fn main() {
    // Минимальный логгер; уровень задаётся через RUST_LOG.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .try_init();

    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("ошибка: {e}");
        std::process::exit(1);
    }
}

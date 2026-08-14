//! Точка входа `dm` — тонкая обёртка над `dm_cli`.
//!
//! Весь разбор аргументов и логика живут в `dm-cli`. Здесь только инициализация
//! логирования, парсинг и корректный код выхода.
//!
//! **Особенность**: если бинарник вызван как `dmx` (копия dm для шортката
//! алиасов), аргументы автоматически преобразуются: `dmx <name> args...`
//! → `dm x <name> args...`.

use clap::Parser;
use dm_cli::{Cli, run};

#[tokio::main]
async fn main() {
    // Минимальный логгер; уровень задаётся через RUST_LOG.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .try_init();

    // Если вызван как dmx — преобразуем аргументы в форму `dm x <name> ...`.
    let invoked_as_dmx = std::env::args_os()
        .next()
        .map(|arg0| {
            let name = std::path::Path::new(&arg0)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            name == "dmx"
        })
        .unwrap_or(false);

    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if invoked_as_dmx && !args.is_empty() {
        // dmx <name> [args...] → dm x <name> [args...]
        args.insert(0, "x".to_string());
    }

    let cli = Cli::parse_from(std::iter::once("dm".to_string()).chain(args));
    if let Err(e) = run(cli).await {
        eprintln!("ошибка: {e}");
        std::process::exit(1);
    }
}

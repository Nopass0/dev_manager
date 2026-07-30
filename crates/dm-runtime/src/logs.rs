//! Модель лог-событий и статуса сервиса.
//!
//! Эти типы не зависят от способа вывода (консоль, файл, TUI) — конкретный
//! рендеринг живёт в `dm-cli`.

use dm_core::project::ServiceLanguage;

/// Уровень лог-сообщения, полученного от процесса сервиса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Стандартный вывод процесса (stdout).
    Info,
    /// Поток ошибок процесса (stderr).
    Error,
    /// Системное сообщение от самого Dev Manager (не от сервиса).
    System,
}

impl LogLevel {
    /// Короткая текстовая метка для рендеринга в логах.
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Info => "OUT",
            LogLevel::Error => "ERR",
            LogLevel::System => "SYS",
        }
    }
}

/// Одна строка лога сервиса.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Имя сервиса, из которого пришла строка.
    pub service: String,
    /// Уровень сообщения.
    pub level: LogLevel,
    /// Текст строки (без завершающего перевода строки).
    pub text: String,
}

impl LogLine {
    /// Создаёт новую лог-строку.
    #[inline]
    pub fn new(service: String, level: LogLevel, text: String) -> Self {
        Self {
            service,
            level,
            text,
        }
    }
}

/// Текущее состояние сервиса в supervisor'е.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Запланирован к запуску (ожидает своей очереди или задержки).
    Pending,
    /// В процессе запуска.
    Starting,
    /// Успешно работает.
    Running,
    /// Остановлен пользователем (`dm stop`/`dm restart`).
    Stopped,
    /// Завершился с ошибкой (ненулевой код).
    Crashed,
    /// Завершился штатно.
    Exited,
}

impl ServiceStatus {
    /// Человекочитаемая метка для таблицы статуса.
    pub fn label(self) -> &'static str {
        match self {
            ServiceStatus::Pending => "pending",
            ServiceStatus::Starting => "starting",
            ServiceStatus::Running => "running",
            ServiceStatus::Stopped => "stopped",
            ServiceStatus::Crashed => "crashed",
            ServiceStatus::Exited => "exited",
        }
    }

    /// Возвращает true, если сервис ещё «жив» (процесс активен).
    pub fn is_alive(self) -> bool {
        matches!(self, ServiceStatus::Starting | ServiceStatus::Running)
    }
}

/// Стабильный цвет, привязанный к языку сервиса — чтобы в мультиплексированных
/// логах один и тот же сервис всегда окрашивался одинаково.
///
/// Возвращает ANSI-escape-последовательность цвета. Используется для префикса
/// `[service]` в общем потоке.
pub fn service_color(language: ServiceLanguage) -> &'static str {
    match language {
        ServiceLanguage::Rust => "\x1b[31m",      // красный
        ServiceLanguage::Go => "\x1b[36m",        // cyan
        ServiceLanguage::C | ServiceLanguage::Cpp => "\x1b[35m", // magenta
        ServiceLanguage::Csharp => "\x1b[95m",    // bright magenta
        ServiceLanguage::JavaScript => "\x1b[33m", // жёлтый
        ServiceLanguage::TypeScript => "\x1b[93m", // bright yellow
        ServiceLanguage::Bun => "\x1b[92m",       // bright green
        ServiceLanguage::Nodejs => "\x1b[32m",    // green
        ServiceLanguage::Lua => "\x1b[34m",       // blue
        ServiceLanguage::Python => "\x1b[94m",    // bright blue
        ServiceLanguage::Vite => "\x1b[95m",      // bright magenta
        ServiceLanguage::Nextjs => "\x1b[97m",    // bright white
        ServiceLanguage::Remix => "\x1b[96m",     // bright cyan
        ServiceLanguage::Other => "\x1b[37m",     // white
    }
}

/// Сброс цвета в терминале.
pub const COLOR_RESET: &str = "\x1b[0m";

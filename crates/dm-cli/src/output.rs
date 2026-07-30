//! Утилиты форматированного вывода: цвета, префиксы сервисов, таблицы.
//!
//! Все печатающие функции консоли собраны здесь, чтобы команды оставались
//! короткими и сосредоточенными на бизнес-логике.
//!
//! Подход к цветам: используем `anstyle::Style` для формирования ANSI-кодов и
//! `anstream::println`/`print` для авто-адаптации (отключение цвета при
//! перенаправлении в pipe, на платформах без поддержки и т.д.).

use anstyle::{AnsiColor, Color, Style};
use comfy_table::{ContentArrangement, Table};
use dm_runtime::logs::{LogLine, LogLevel, ServiceStatus};

/// Применяет стиль к тексту, возвращая готовую ANSI-строку с reset на конце.
fn paint(text: &str, style: Style) -> String {
    format!("{style}{text}{style:#}")
}

/// Стиль текста: сообщение об успехе.
pub fn success_style() -> Style {
    Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Green)))
        .bold()
}
/// Стиль текста: предупреждение.
pub fn warn_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
}
/// Стиль текста: ошибка.
pub fn error_style() -> Style {
    Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold()
}
/// Стиль текста: информационное/нейтральное сообщение.
pub fn info_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
}
/// Стиль: тусклая подпись.
pub fn dim_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
}

/// Печатает строку с заданным стилем (с переносом строки).
pub fn println_styled(text: &str, style: Style) {
    anstream::println!("{}", paint(text, style));
}

/// Печатает лог-строку с цветным префиксом сервиса и меткой уровня.
pub fn print_log_line(line: &LogLine) {
    let prefix_style = match line.level {
        LogLevel::Error => error_style(),
        LogLevel::System => info_style(),
        LogLevel::Info => Style::new(),
    };
    let prefix = format!("[{}]", line.service);
    let level_tag = match line.level {
        LogLevel::Error => "ERR",
        LogLevel::System => "SYS",
        LogLevel::Info => "OUT",
    };
    anstream::println!(
        "{} {} {}",
        paint(&prefix, prefix_style),
        paint(level_tag, dim_style()),
        line.text,
    );
}

/// Цветной префикс для системных сообщений самого `dm`.
pub fn print_system(text: &str) {
    anstream::println!("{} {}", paint(crate::commands::PREFIX_SYS, info_style().bold()), text);
}

/// Строит таблицу статусов сервисов для команды `dm status`.
pub fn build_status_table(rows: &[(String, ServiceStatus)]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Сервис", "Статус"]);

    for (name, status) in rows {
        table.add_row(vec![name.to_string(), status_label(*status)]);
    }
    table
}

/// Человекочитаемая метка статуса (с эмодзи и русским словом).
fn status_label(status: ServiceStatus) -> String {
    let (emoji, ru) = match status {
        ServiceStatus::Pending => ("⏳", "ожидание"),
        ServiceStatus::Starting => ("🚀", "запуск"),
        ServiceStatus::Running => ("✅", "работает"),
        ServiceStatus::Stopped => ("⏹️", "остановлен"),
        ServiceStatus::Crashed => ("💥", "упал"),
        ServiceStatus::Exited => ("🏁", "завершён"),
    };
    format!("{emoji} {ru}")
}

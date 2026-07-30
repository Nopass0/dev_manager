//! Единая система ошибок всего проекта.
//!
//! Все публичные функции Dev Manager возвращают [`DmResult`]. Пользовательские
//! ошибки используют [`DmError`] с понятными сообщениями, а не строковые константы.

use std::path::PathBuf;
use thiserror::Error;

/// Канонический тип ошибок Dev Manager.
///
/// Каждая варианта отражает конкретный класс сбоя, чтобы вызывающая сторона
/// могла принять решение (продолжить, прервать, спросить пользователя).
#[derive(Debug, Error)]
pub enum DmError {
    /// Файл `dm.yaml` не найден в текущем каталоге и его родителях.
    #[error("конфиг dm.yaml не найден. Запустите `dm init`, чтобы создать его.")]
    ConfigNotFound,

    /// Ошибка чтения или записи файла конфигурации.
    #[error("не удалось прочитать конфигурацию {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Ошибка разбора YAML — неверный синтаксис или структура.
    #[error("ошибка разбора dm.yaml: {0}")]
    ConfigParse(#[from] serde_yaml::Error),

    /// Неподдерживаемая версия схемы конфига.
    #[error("неподдерживаемая версия конфига: {0}. Поддерживается только версия 1.")]
    ConfigUnsupportedVersion(u32),

    /// Конфигурация прошла парсинг, но семантически некорректна.
    #[error("неверная конфигурация: {0}")]
    ConfigInvalid(String),

    /// Запрошенный сервис не описан в `dm.yaml`.
    #[error("сервис '{0}' не найден в dm.yaml")]
    ServiceNotFound(String),

    /// Сбой запуска или управления дочерним процессом.
    #[error("ошибка процесса: {0}")]
    Process(String),

    /// Внешний инструмент (git, cargo, npm…) завершился с ошибкой.
    #[error("команда '{command}' завершилась с кодом {code}: {stderr}")]
    ExternalCommand {
        command: String,
        code: i32,
        stderr: String,
    },

    /// Требуемый внешний инструмент отсутствует в PATH.
    #[error("внешний инструмент '{0}' не найден в PATH. Установите его и повторите.")]
    ToolMissing(String),

    /// Операционная система / платформа не поддерживает запрошенное действие.
    #[error("операция не поддерживается на этой платформе: {0}")]
    UnsupportedPlatform(String),

    /// Прочая ошибка ввода-вывода, не укладывающаяся в остальные варианты.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Универсальная обёртка для ошибок, не требующих специальной обработки.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Псевдоним результата для всех публичных API Dev Manager.
pub type DmResult<T> = Result<T, DmError>;

impl DmError {
    /// Создаёт ошибку [`DmError::ConfigInvalid`] из форматной строки.
    ///
    /// Удобен для валидации конфигурации без явного `String::from`.
    #[inline]
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::ConfigInvalid(msg.into())
    }
}

//! Кросс-платформенные помощники для работы с путями файловой системы.
//!
//! Dev Manager активно использует относительные пути из `dm.yaml` и должен
//! корректно работать как на Windows (UNC-пути, разделители `\`), так и на
//! Linux/macOS. Этот модуль собирает общие операции в одном месте (DRY).

use crate::error::{DmError, DmResult};
use std::path::{Path, PathBuf};

/// Канонизирует путь к короткой, человекочитаемой форме.
///
/// Использует [`dunce::simplified`] чтобы избежать UNC-префиксов вида
/// `\\?\C:\...` на Windows, сохраняя путь валидным для записи в логи и показа
/// пользователю.
///
/// # Ошибки
/// Возвращает [`DmError::Io`], если не удалось получить метаданные пути.
pub fn simplify(path: &Path) -> PathBuf {
    dunce::simplified(path).to_path_buf()
}

/// Превращает путь в строку с прямыми слешами `/` независимо от платформы.
///
/// Полезно для логов и сообщений, чтобы вывод выглядел одинаково в Windows и Linux.
pub fn to_display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Разрешает `relative` относительно `base`, возвращая абсолютный путь.
///
/// Если `relative` уже абсолютный — возвращает его как есть (после упрощения).
/// Нормализует `..` и `.`_lexical без обращения к файловой системе.
pub fn resolve(base: &Path, relative: &Path) -> PathBuf {
    if relative.is_absolute() {
        return simplify(relative);
    }
    let joined = base.join(relative);
    normalize_lexical(&joined)
}

/// Лексическая нормализация `..` и `.` без запроса к ФС.
///
/// В отличие от [`std::fs::canonicalize`], не требует существования пути и не
/// добавляет UNC-префиксы — это безопасно для путей из конфига.
pub fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out: Vec<std::path::Component> = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                // Схлопываем `..` только если поверх обычного компонента,
                // но не трогаем корень или префикс диска.
                match out.last() {
                    Some(std::path::Component::Normal(_)) => {
                        out.pop();
                    }
                    _ => out.push(comp),
                }
            }
            std::path::Component::CurDir => {} // `.` игнорируем
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Проверяет, что каталог существует, иначе возвращает понятную ошибку.
///
/// Используется при валидации `path` у сервиса в `dm.yaml`.
pub fn ensure_dir_exists(path: &Path, label: &str) -> DmResult<()> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(DmError::invalid_config(format!(
            "каталог {label} не найден: {}",
            to_display(path)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative() {
        let base = Path::new("/home/user/project");
        let rel = Path::new("./services/api");
        assert_eq!(
            resolve(base, rel),
            PathBuf::from("/home/user/project/services/api")
        );
    }

    #[test]
    fn normalizes_parent_dirs() {
        let p = Path::new("/a/b/../c/./d");
        assert_eq!(normalize_lexical(p), PathBuf::from("/a/c/d"));
    }

    #[test]
    #[cfg(windows)]
    fn resolves_windows_drive() {
        let base = Path::new("C:\\proj");
        let rel = Path::new("services\\api");
        assert_eq!(resolve(base, rel), PathBuf::from("C:\\proj\\services\\api"));
    }
}

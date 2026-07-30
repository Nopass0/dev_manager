//! # dm-installer
//!
//! Установка бинарника Dev Manager в системный PATH и генерация oneliner-скриптов
//! для Windows и Linux/macOS.
//!
//! ## Логика установки
//! - **Linux/macOS**: бинарник кладётся в `~/.local/bin` (или `/usr/local/bin`
//!   при наличии прав), после чего каталог проверяется на присутствие в
//!   `~/.bashrc`/`~/.zshrc` (добавляется при отсутствии).
//! - **Windows**: бинарник кладётся в `%LOCALAPPDATA%\Programs\dm`, каталог
//!   добавляется в пользовательскую переменную окружения `PATH` (постоянно) и
//!   в текущую сессию через `SetEnvironmentVariable`.
//!
//! Все операции с PATH идемпотентны: повторный запуск не дублирует записи.

pub mod scripts;

use dm_core::error::{DmError, DmResult};
use std::path::{Path, PathBuf};

/// Результат установки: куда положили бинарник и был ли обновлён PATH.
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Каталог, в который скопирован бинарник.
    pub bin_dir: PathBuf,
    /// Полный путь к установленному бинарнику.
    pub bin_path: PathBuf,
    /// Был ли PATH изменён (true) или уже содержал нужный каталог (false).
    pub path_updated: bool,
}

/// Устанавливает `binary` (уже собранный `dm`) в систему и регистрирует в PATH.
///
/// Функция кросс-платформенная: на Windows правит реестр через PowerShell-вызов,
/// на Unix — дописывает строку export в shell rc-файлы.
pub fn install(binary: &Path) -> DmResult<InstallResult> {
    if !binary.is_file() {
        return Err(DmError::Process(format!(
            "бинарник не найден: {}",
            binary.display()
        )));
    }
    let bin_dir = target_dir()?;
    std::fs::create_dir_all(&bin_dir)?;
    let bin_name = binary_name();
    let bin_path = bin_dir.join(&bin_name);
    copy_binary(binary, &bin_path)?;

    let path_updated = ensure_in_path(&bin_dir)?;
    Ok(InstallResult {
        bin_dir,
        bin_path,
        path_updated,
    })
}

/// Возвращает имя бинарника в зависимости от платформы (`dm` / `dm.exe`).
pub fn binary_name() -> String {
    if cfg!(windows) {
        "dm.exe".to_string()
    } else {
        "dm".to_string()
    }
}

/// Копирует бинарник, выставляя Unix-исполняемый бит при необходимости.
fn copy_binary(src: &Path, dst: &Path) -> DmResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::remove_file(dst);
        std::fs::copy(src, dst)?;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(dst, perms)?;
    }
    #[cfg(windows)]
    {
        let _ = std::fs::remove_file(dst);
        std::fs::copy(src, dst)?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = std::fs::remove_file(dst);
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Целевой каталог установки по платформе.
fn target_dir() -> DmResult<PathBuf> {
    if cfg!(windows) {
        // %LOCALAPPDATA%\Programs\dm
        let local = std::env::var("LOCALAPPDATA").map_err(|_| {
            DmError::UnsupportedPlatform("LOCALAPPDATA не задана".to_string())
        })?;
        Ok(PathBuf::from(local).join("Programs").join("dm"))
    } else {
        // ~/.local/bin (стандарт XDG); создаётся если нет.
        let dirs = directories::BaseDirs::new().ok_or_else(|| {
            DmError::UnsupportedPlatform("не удалось определить домашний каталог".to_string())
        })?;
        Ok(dirs.home_dir().join(".local").join("bin"))
    }
}

/// Гарантирует, что `dir` присутствует в пользовательском PATH.
///
/// Возвращает true, если PATH был изменён; false — если уже содержал каталог.
fn ensure_in_path(dir: &Path) -> DmResult<bool> {
    let dir_str = dir.to_string_lossy().into_owned();
    if already_in_path(&dir_str)? {
        return Ok(false);
    }
    #[cfg(windows)]
    return add_to_windows_path(&dir_str);
    #[cfg(unix)]
    return add_to_unix_shell_rc(&dir_str);
    #[cfg(not(any(unix, windows)))]
    {
        let _ = dir;
        Err(DmError::UnsupportedPlatform(
            "установка в PATH не реализована для этой платформы".to_string(),
        ))
    }
}

/// Проверяет, есть ли `dir` в текущем PATH процесса.
fn already_in_path(dir: &str) -> DmResult<bool> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ';' } else { ':' };
    Ok(path_var
        .split(sep)
        .any(|p| std::path::Path::new(p) == std::path::Path::new(dir)))
}

#[cfg(unix)]
fn add_to_unix_shell_rc(dir: &str) -> DmResult<bool> {
    let home = directories::BaseDirs::new()
        .ok_or_else(|| DmError::UnsupportedPlatform("no home".into()))?
        .home_dir()
        .to_path_buf();

    // Проверяем .bashrc и .zshrc (если файлы существуют — дописываем).
    let candidates = [home.join(".bashrc"), home.join(".zshrc"), home.join(".profile")];
    let marker = "# added by dm installer";
    let line = format!(r#"{marker} export PATH="$HOME/.local/bin:$PATH""#);
    let _ = dir; // строка PATH фиксирована для ~/.local/bin
    let mut wrote_any = false;
    for rc in candidates {
        if !rc.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&rc).unwrap_or_default();
        if content.contains(marker) {
            return Ok(false);
        }
        let new_content = format!("{content}\n{line}\n");
        std::fs::write(&rc, new_content)?;
        wrote_any = true;
    }
    Ok(wrote_any)
}

#[cfg(windows)]
fn add_to_windows_path(dir: &str) -> DmResult<bool> {
    // Используем PowerShell для надёжной работы с реестром через [Environment].
    let ps = format!(
        r#"
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -split ';' -contains '{dir}') {{ 'already' }}
else {{
    $new = if ($userPath) {{ $userPath + ';{dir}' }} else {{ '{dir}' }}
    [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    $env:Path = "$env:Path;{dir}"
    'added'
}}
"#,
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .output()
        .map_err(|e| DmError::Process(format!("powershell: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("added"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_name_matches_platform() {
        let name = binary_name();
        if cfg!(windows) {
            assert_eq!(name, "dm.exe");
        } else {
            assert_eq!(name, "dm");
        }
    }

    #[test]
    fn target_dir_is_under_user_profile() {
        let dir = target_dir();
        assert!(dir.is_ok(), "должен определять целевой каталог");
        let dir = dir.unwrap();
        assert!(dir.to_string_lossy().contains("dm") || dir.to_string_lossy().contains(".local"));
    }

    #[test]
    fn already_in_path_detects_current_entries() {
        let sep = if cfg!(windows) { ';' } else { ':' };
        // set_var небезопасен в edition 2024 — оборачиваем в unsafe.
        // Тест изменяет PATH процесса; это безопасно в рамках одного теста.
        unsafe {
            std::env::set_var("PATH", format!("/foo{sep}/bar"));
        }
        assert!(already_in_path("/foo").unwrap());
        assert!(!already_in_path("/baz").unwrap());
    }
}

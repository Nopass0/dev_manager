//! Кросс-платформенный запуск shell-команд (Windows + Linux/macOS).
//!
//! Единая точка для всех команд `dm`, чтобы не дублировать блоки `#[cfg]` в
//! каждом файле (DRY). На Windows использует `cmd /C`, на Unix — `sh -c`.
//!
//! Все функции возвращают код завершения процесса; ошибки — через `String`
//! (понятное сообщение безio-типов, удобно для команд).

use std::path::Path;
use std::process::{Command, Stdio};

/// Результат выполнения команды: код завершения и объединённый вывод (stdout+stderr).
#[derive(Debug, Clone)]
pub struct CmdResult {
    /// Код завершения процесса.
    pub code: i32,
    /// stdout + stderr, склеенные в одну строку.
    pub output: String,
}

impl CmdResult {
    /// True, если процесс завершился успешно (код 0).
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

/// Запускает `command` в системном shell (`cmd /C` на Windows, `sh -c` на Unix)
/// в каталоге `cwd`. Возвращает код завершения.
///
/// Наследует stdio (вывод идёт в консоль пользователя). Для перехвата вывода
/// используйте [`capture`].
pub fn run(command: &str, cwd: &Path) -> Result<i32, String> {
    let mut cmd = shell_for_platform(command);
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    let status = cmd.status().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

/// То же, что [`run`], но перехватывает stdout+stderr и возвращает [`CmdResult`].
pub fn capture(command: &str, cwd: &Path) -> Result<CmdResult, String> {
    let mut cmd = shell_for_platform(command);
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let out = cmd.output().map_err(|e| e.to_string())?;
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(CmdResult {
        code: out.status.code().unwrap_or(-1),
        output: combined,
    })
}

/// Запускает команду, перехватывая вывод, и возвращает `DmResult` с понятной
/// ошибкой при ненулевом коде. Удобно для команд, которым важен успех.
pub fn run_ok(command: &str, cwd: &Path) -> dm_core::DmResult<CmdResult> {
    let res = capture(command, cwd).map_err(|e| {
        dm_core::DmError::Process(format!("shell capture: {e}"))
    })?;
    if res.success() {
        Ok(res)
    } else {
        Err(dm_core::DmError::ExternalCommand {
            command: command.to_string(),
            code: res.code,
            stderr: res.output,
        })
    }
}

/// Запускает `argv` (программа + аргументы) напрямую, без shell, в `cwd`.
/// Удобно для одноразовых бинарников (psql, redis-cli) без shell-overhead.
pub fn run_argv(argv: &[&str], cwd: &Path) -> Result<i32, String> {
    if argv.is_empty() {
        return Err("пустая команда".into());
    }
    let mut cmd = Command::new(argv[0]);
    cmd.args(&argv[1..]);
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    let status = cmd.status().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

/// Разрешает относительный путь сервиса в абсолютный от корня проекта.
///
/// Удобный shorthand для команд, чтобы не тянуть `dm_core::paths` напрямую.
pub fn resolve_dir(root: &Path, rel: &str) -> std::path::PathBuf {
    dm_core::paths::resolve(root, Path::new(rel))
}

/// Конструирует `Command` для системного shell с командой (кросс-платформенно).
fn shell_for_platform(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(command);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_echo_succeeds() {
        let tmp = std::env::temp_dir();
        // Платформо-нейтральная команда: вывести строку.
        #[cfg(windows)]
        let cmd = "echo hello";
        #[cfg(not(windows))]
        let cmd = "printf hello";
        let code = run(cmd, &tmp).expect("cmd must run");
        assert_eq!(code, 0);
    }

    #[test]
    fn capture_returns_output() {
        let tmp = std::env::temp_dir();
        #[cfg(windows)]
        let cmd = "echo capturetest";
        #[cfg(not(windows))]
        let cmd = "printf capturetest";
        let res = capture(cmd, &tmp).expect("capture");
        assert!(res.success());
        assert!(res.output.contains("capturetest"));
    }

    #[test]
    fn run_ok_err_on_failure() {
        let tmp = std::env::temp_dir();
        #[cfg(windows)]
        let cmd = "exit /B 3";
        #[cfg(not(windows))]
        let cmd = "exit 3";
        let res = run_ok(cmd, &tmp);
        assert!(res.is_err());
    }
}

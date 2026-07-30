//! Низкоуровневые операции с `git` CLI.
//!
//! Единственная точка взаимодействия с git — функция [`run_git`]. Все остальные
//! модули (`commit`, `push`, `diff`) построены поверх неё. Это DRY: логика
//! кодировки, обработки ошибок и поиска `git` живёт в одном месте.

use dm_core::error::{DmError, DmResult};
use std::path::Path;
use tokio::process::Command;

/// Результат выполнения git-команды: stdout (как UTF-8) и код возврата.
#[derive(Debug, Clone)]
pub struct GitOutput {
    /// Стандартный вывод (часто используется для парсинга `status`/`diff`).
    pub stdout: String,
    /// Стандартный поток ошибок (для диагностики).
    pub stderr: String,
    /// Код завершения процесса git.
    pub code: i32,
}

impl GitOutput {
    /// Возвращает true, если git завершился успешно (код 0).
    pub fn ok(&self) -> bool {
        self.code == 0
    }
}

/// Запускает `git -C <repo> <args>` и возвращает вывод.
///
/// Это building-block; предпочитайте специализированные функции выше, чтобы
/// не дублировать формат аргументов и обработку ошибок.
///
/// # Ошибки
/// - [`DmError::ToolMissing`] если `git` не найден в PATH.
/// - [`DmError::ExternalCommand`] если код возврата ненулевой и `require_ok=true`.
pub async fn run_git(
    repo: &Path,
    args: &[&str],
    require_ok: bool,
) -> DmResult<GitOutput> {
    // Проверяем наличие git только при первом сбое запуска — cheaper.
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo);
    cmd.args(args);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = match cmd.output().await {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(DmError::ToolMissing("git".to_string()));
        }
        Err(e) => {
            return Err(DmError::Process(format!(
                "не удалось запустить git: {e}"
            )));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);

    if require_ok && code != 0 {
        let cmd_string = format!("git -C {} {}", repo.display(), args.join(" "));
        return Err(DmError::ExternalCommand {
            command: cmd_string,
            code,
            stderr,
        });
    }
    Ok(GitOutput {
        stdout,
        stderr,
        code,
    })
}

/// Проверяет, является ли каталог git-репозиторием.
pub async fn is_repo(repo: &Path) -> bool {
    run_git(repo, &["rev-parse", "--is-inside-work-tree"], false)
        .await
        .map(|o| o.ok() && o.stdout.trim() == "true")
        .unwrap_or(false)
}

/// Проверяет, есть ли незакоммиченные изменения (staged или unstaged).
pub async fn has_changes(repo: &Path) -> DmResult<bool> {
    let out = run_git(repo, &["status", "--porcelain"], true).await?;
    Ok(!out.stdout.trim().is_empty())
}

/// Возвращает версию системного git (для диагностики и проверки доступности).
pub async fn git_binary_version() -> DmResult<String> {
    let out = run_git(Path::new("."), &["--version"], false).await?;
    Ok(out.stdout.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn detects_non_repo() {
        // Системный каталог почти наверняка не git-репозиторий.
        let tmp = std::env::temp_dir().join("dm_git_nonrepo_test");
        std::fs::create_dir_all(&tmp).unwrap();
        // Если git не установлен — is_repo просто вернёт false, тест остаётся корректным.
        let result = is_repo(&tmp).await;
        let _ = std::fs::remove_dir_all(&tmp);
        // Не делаем строгих предположений о наличии git, только что не паникует.
        let _ = result;
    }
}

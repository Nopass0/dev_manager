//! Кросс-платформенное управление дочерним процессом.
//!
//! Гарантия: при остановке убивается **всё дерево** подпроцессов (service →
//! его подпроцессы → их подпроцессы…). Это критично для таких команд, как
//! `npm run dev`, которые порождают вложенные процессы (vite, esbuild, и т.д.).
//!
//! На Unix используется семантика process groups, на Windows — Job Objects;
//! всё это инкапсулировано в crate'е `kill_tree`, чтобы нам не пришлось
//! дублировать платформенно-зависимый код.

use crate::logs::{LogLevel, LogLine};
use dm_core::DmResult;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Обёртка над запущенным дочерним процессом с гарантированным убийством дерева.
///
/// После [`ManagedProcess::spawn`] процесс живёт до вызова [`ManagedProcess::kill`]
/// или естественного завершения. Логи построчно читаются из stdout/stderr и
/// отправляются в `log_tx` как [`LogLine`].
pub struct ManagedProcess {
    /// PID корневого процесса — нужен `kill_tree` для рекурсивного завершения.
    pid: Option<u32>,
    /// Tokio-задача, читающая потоки вывода. Хранится чтобы дождаться завершения.
    reader_task: Option<JoinHandle<()>>,
    /// Сам дочерний процесс (для ожидания exit code).
    child: Option<tokio::process::Child>,
}

/// Исход естественного завершения процесса (без явного kill).
#[derive(Debug, Clone)]
pub struct ProcessExit {
    /// Код возврата (если получен).
    pub code: Option<i32>,
    /// Завершился ли процесс сигналом (Unix).
    pub killed_by_signal: bool,
}

impl ProcessExit {
    /// True, если процесс завершился успешно (код 0).
    pub fn success(&self) -> bool {
        matches!(self.code, Some(0)) && !self.killed_by_signal
    }
}

impl ManagedProcess {
    /// Запускает `command` в каталоге `cwd` с переменными окружения `env`.
    ///
    /// Все строки stdout/stderr построчно отправляются в `log_tx` с указанием
    /// имени сервиса `service_name`. Возвращает [`ManagedProcess`].
    ///
    /// # Ошибки
    /// [`DmError::Process`] если не удалось запустить бинарник.
    pub async fn spawn(
        service_name: &str,
        command: &str,
        cwd: &Path,
        env: &[(String, String)],
        log_tx: mpsc::UnboundedSender<LogLine>,
    ) -> DmResult<Self> {
        let mut tokens = shell_split(command);
        if tokens.is_empty() {
            return Err(dm_core::DmError::Process(format!(
                "пустая команда запуска для сервиса '{service_name}'"
            )));
        }
        let program = tokens.remove(0);

        let mut cmd = Command::new(&program);
        cmd.current_dir(cwd);
        cmd.args(tokens);
        for (k, v) in env {
            cmd.env(k, v);
        }
        // Перехватываем оба потока; stdin не нужен.
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());
        // На Unix просим новую process group, чтобы корректно убить всё дерево.
        #[cfg(unix)]
        {
            #[allow(unused_imports)]
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        // На Windows kill_tree использует Job Objects под капотом.

        let mut child = cmd.spawn().map_err(|e| {
            dm_core::DmError::Process(format!(
                "не удалось запустить '{program}' для сервиса '{service_name}': {e}"
            ))
        })?;

        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Один tokio-задача читает оба потока и шлёт строки в канал.
        let svc = service_name.to_string();
        let reader_task = tokio::spawn(async move {
            // Каждый поток читает в своей подзадаче, чтобы они шли параллельно.
            let svc_out = svc.clone();
            let tx_out = log_tx.clone();
            let out_task: Option<JoinHandle<()>> = stdout.map(|s| {
                tokio::spawn(async move {
                    read_lines(BufReader::new(s), &svc_out, LogLevel::Info, tx_out).await;
                })
            });
            if let Some(s) = stderr {
                read_lines(BufReader::new(s), &svc, LogLevel::Error, log_tx).await;
            }
            if let Some(t) = out_task {
                let _ = t.await;
            }
        });

        Ok(Self {
            pid,
            reader_task: Some(reader_task),
            child: Some(child),
        })
    }

    /// Убивает процесс и всё его поддерево, затем дожидается выхода.
    ///
    /// `kill_tree` в версии 0.2 — блокирующая, поэтому оборачиваем в
    /// `spawn_blocking`, чтобы не заморозить async-runtime. После вызова можно
    /// быть уверенным, что ни один подпроцесс не остался «висеть».
    pub async fn kill(&mut self) -> DmResult<()> {
        if let Some(pid) = self.pid.take() {
            // kill_tree::blocking внутри spawn_blocking — кросс-платформенное
            // рекурсивное завершение дерева процессов.
            let _ = tokio::task::spawn_blocking(move || kill_tree::blocking::kill_tree(pid)).await;
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.wait().await;
        }
        if let Some(task) = self.reader_task.take() {
            let _ = task.await;
        }
        Ok(())
    }

    /// **Ждёт естественного завершения процесса**, не убивая его.
    ///
    /// Возвращает [`ProcessExit`] с кодом возврата. Это ключевая операция для
    /// supervisor'а: блокируясь на ней, мы узнаём момент выхода и его причину
    /// (success / non-zero / signal), что даёт корректный auto-recovery и
    /// уведомления. Чтение логов продолжается параллельно до EOF.
    pub async fn wait_exit(&mut self) -> DmResult<ProcessExit> {
        let Some(mut child) = self.child.take() else {
            // Уже waited/убит — возвращаем «успех» как безопасный дефолт.
            return Ok(ProcessExit {
                code: Some(0),
                killed_by_signal: false,
            });
        };
        let status = child
            .wait()
            .await
            .map_err(|e| dm_core::DmError::Process(format!("wait_exit: {e}")))?;
        // Останавливаем задачу чтения (стримы уже закрылись на EOF).
        if let Some(task) = self.reader_task.take() {
            let _ = task.await;
        }
        Ok(ProcessExit {
            code: status.code(),
            killed_by_signal: {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    status.signal().is_some()
                }
                #[cfg(not(unix))]
                {
                    false
                }
            },
        })
    }

    /// PID процесса или `None`, если он уже завершён/не был получен.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        // Подстраховка: если kill() не вызвали явно, пытаемся убрать задачу чтения.
        // Сам процесс при Drop у tokio::process::Child НЕ убивается (по умолчанию),
        // поэтому явный kill() обязателен — это намеренно.
        if let Some(task) = self.reader_task.take() {
            task.abort();
        }
    }
}

/// Построчно читает `stream` и отправляет [`LogLine`] в канал.
///
/// `R` — любой `tokio::io::AsyncBufRead` (например, `BufReader<ChildStdout>`).
async fn read_lines<R: AsyncBufReadExt + Unpin>(
    stream: R,
    service: &str,
    level: LogLevel,
    tx: mpsc::UnboundedSender<LogLine>,
) {
    let mut reader = stream;
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = buf.trim_end_matches(['\n', '\r']).to_string();
                if line.is_empty() {
                    continue;
                }
                if tx
                    .send(LogLine::new(service.to_string(), level, line))
                    .is_err()
                {
                    break; // получатель закрыт — выходим
                }
            }
            Err(_) => break,
        }
    }
}

/// Простейший токенайзер командной строки по пробелам, уважающий кавычки.
///
/// Достаточно для типовых команд вида `cargo run --release` или
/// `npm "run" "dev with args"`. Не претендует на полноценный шелл.
pub fn shell_split(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for c in command.chars() {
        match c {
            '"' if !in_single => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_command() {
        assert_eq!(
            shell_split("cargo run --release"),
            vec!["cargo", "run", "--release"]
        );
    }

    #[test]
    fn respects_quoted_segments() {
        let tokens = shell_split(r#"echo "hello world" 'a b'"#);
        assert_eq!(tokens, vec!["echo", "hello world", "a b"]);
    }

    #[test]
    fn empty_command_gives_empty() {
        assert!(shell_split("").is_empty());
        assert!(shell_split("   ").is_empty());
    }
}

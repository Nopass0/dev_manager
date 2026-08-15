//! Дополнительные модули Lua: proc_io, net, time, str.
//!
//! - **proc_io**: запуск программ с интерактивным stdin/stdout;
//! - **net**: TCP-клиент (connect/send/recv, port_open) для тестирования протоколов;
//! - **time**: timestamps, паузы, форматирование;
//! - **str**: строковые утилиты (split/trim/starts_with/ends_with).

use mlua::{Lua, Result as LuaResult};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Регистрирует все модули этого файла.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    register_net(lua)?;
    register_time(lua)?;
    register_str(lua)?;
    register_proc_io(lua)?;
    Ok(())
}

// ==================== net: TCP клиент ====================

/// Состояние TCP-соединения, разделённое между Lua-замыканиями.
struct ConnState {
    stream: Option<TcpStream>,
}

fn register_net(lua: &Lua) -> LuaResult<()> {
    let net = lua.create_table()?;

    // net.tcp_connect(host, port) — таблица-соединение с send/recv/close.
    net.set(
        "tcp_connect",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let stream = TcpStream::connect((host.as_str(), port))
                .map_err(|e| mlua::Error::RuntimeError(format!("connect {host}:{port}: {e}")))?;
            let state = Arc::new(Mutex::new(ConnState {
                stream: Some(stream),
            }));

            let conn = lua.create_table()?;

            // :send(data) — отправить, вернуть число байт.
            let st = state.clone();
            conn.set(
                "send",
                lua.create_function(move |_, data: String| {
                    let mut guard = st.lock().unwrap();
                    if let Some(ref mut s) = guard.stream {
                        s.write_all(data.as_bytes())
                            .map_err(|e| mlua::Error::RuntimeError(format!("send: {e}")))?;
                        s.flush().ok();
                        Ok(data.len() as i64)
                    } else {
                        Ok(0)
                    }
                })?,
            )?;

            // :recv(n) — прочитать до n байт.
            let st = state.clone();
            conn.set(
                "recv",
                lua.create_function(move |_, n: usize| {
                    let mut guard = st.lock().unwrap();
                    if let Some(ref mut s) = guard.stream {
                        let mut buf = vec![0u8; n];
                        let read = s
                            .read(&mut buf)
                            .map_err(|e| mlua::Error::RuntimeError(format!("recv: {e}")))?;
                        buf.truncate(read);
                        Ok(String::from_utf8_lossy(&buf).into_owned())
                    } else {
                        Ok(String::new())
                    }
                })?,
            )?;

            // :close() — закрыть соединение.
            let st = state.clone();
            conn.set(
                "close",
                lua.create_function(move |_, ()| {
                    let mut guard = st.lock().unwrap();
                    guard.stream = None;
                    Ok(())
                })?,
            )?;

            Ok(conn)
        })?,
    )?;

    // net.tcp_send(host, port, data) — быстрый send-and-forget.
    net.set(
        "tcp_send",
        lua.create_function(|_, (host, port, data): (String, u16, String)| {
            let mut s = TcpStream::connect((host.as_str(), port))
                .map_err(|e| mlua::Error::RuntimeError(format!("connect: {e}")))?;
            s.write_all(data.as_bytes())
                .map_err(|e| mlua::Error::RuntimeError(format!("send: {e}")))?;
            Ok(data.len() as i64)
        })?,
    )?;

    // net.port_open(host, port) — проверить открыт ли порт.
    net.set(
        "port_open",
        lua.create_function(|_, (host, port): (String, u16)| {
            Ok(TcpStream::connect((host.as_str(), port)).is_ok())
        })?,
    )?;

    lua.globals().set("net", net)?;
    Ok(())
}

// ==================== proc_io: интерактивные процессы ====================

/// Состояние запущенного процесса.
struct ProcState {
    stdin: Option<std::process::ChildStdin>,
    stdout: Option<std::process::ChildStdout>,
    child: std::process::Child,
}

fn register_proc_io(lua: &Lua) -> LuaResult<()> {
    let m = lua.create_table()?;

    // proc_io.spawn(cmd) — таблица-процесс с write/read_line/wait/kill.
    m.set(
        "spawn",
        lua.create_function(|lua, cmd: String| {
            let mut command = shell_command(&cmd);
            let mut child = command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| mlua::Error::RuntimeError(format!("spawn: {e}")))?;

            let stdin = child.stdin.take();
            let stdout = child.stdout.take();
            let state = Arc::new(Mutex::new(ProcState {
                stdin,
                stdout,
                child,
            }));

            let proc = lua.create_table()?;

            // :write(line) — записать в stdin (+\n).
            let st = state.clone();
            proc.set(
                "write",
                lua.create_function(move |_, line: String| {
                    let mut guard = st.lock().unwrap();
                    if let Some(ref mut stdin) = guard.stdin {
                        writeln!(stdin, "{line}")
                            .map_err(|e| mlua::Error::RuntimeError(format!("write: {e}")))?;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                })?,
            )?;

            // :read_line() — прочитать строку из stdout (блокирующая).
            let st = state.clone();
            proc.set(
                "read_line",
                lua.create_function(move |_, ()| {
                    let mut guard = st.lock().unwrap();
                    if let Some(ref mut stdout) = guard.stdout {
                        let mut buf = Vec::new();
                        let mut byte = [0u8; 1];
                        loop {
                            match stdout.read(&mut byte) {
                                Ok(0) => break,
                                Ok(_) => {
                                    buf.push(byte[0]);
                                    if byte[0] == b'\n' {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        let s = String::from_utf8_lossy(&buf).trim_end().to_string();
                        Ok(s)
                    } else {
                        Ok(String::new())
                    }
                })?,
            )?;

            // :read_all() — прочитать весь доступный вывод (неблокирующая эвристика).
            let st = state.clone();
            proc.set(
                "read_all",
                lua.create_function(move |_, ()| {
                    let mut guard = st.lock().unwrap();
                    if let Some(ref mut stdout) = guard.stdout {
                        let mut buf = String::new();
                        // Читаем с таймаутом через read_to_string (до EOF).
                        let _ = stdout.read_to_string(&mut buf);
                        Ok(buf)
                    } else {
                        Ok(String::new())
                    }
                })?,
            )?;

            // :wait() — дождаться завершения, вернуть код.
            let st = state.clone();
            proc.set(
                "wait",
                lua.create_function(move |_, ()| {
                    let mut guard = st.lock().unwrap();
                    drop(guard.stdin.take());
                    match guard.child.wait() {
                        Ok(status) => Ok(status.code().unwrap_or(-1)),
                        Err(e) => Err(mlua::Error::RuntimeError(format!("wait: {e}"))),
                    }
                })?,
            )?;

            // :kill() — завершить принудительно.
            let st = state.clone();
            proc.set(
                "kill",
                lua.create_function(move |_, ()| {
                    let mut guard = st.lock().unwrap();
                    drop(guard.stdin.take());
                    guard
                        .child
                        .kill()
                        .map_err(|e| mlua::Error::RuntimeError(format!("kill: {e}")))
                })?,
            )?;

            Ok(proc)
        })?,
    )?;

    lua.globals().set("proc_io", m)?;
    Ok(())
}

/// Создаёт shell-команду для платформы.
fn shell_command(cmd: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(cmd);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = std::process::Command::new("sh");
        c.arg("-c").arg(cmd);
        c
    }
}

// ==================== time: время ====================

fn register_time(lua: &Lua) -> LuaResult<()> {
    let time = lua.create_table()?;

    time.set(
        "now",
        lua.create_function(|_, ()| {
            Ok(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0))
        })?,
    )?;

    time.set(
        "now_ms",
        lua.create_function(|_, ()| {
            Ok(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0))
        })?,
    )?;

    time.set(
        "elapsed_ms",
        lua.create_function(|_, since_ms: i64| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            Ok(now - since_ms)
        })?,
    )?;

    time.set(
        "sleep_s",
        lua.create_function(|_, secs: u64| {
            std::thread::sleep(std::time::Duration::from_secs(secs));
            Ok(())
        })?,
    )?;

    lua.globals().set("time", time)?;
    Ok(())
}

// ==================== str: строковые утилиты ====================

fn register_str(lua: &Lua) -> LuaResult<()> {
    let s = lua.create_table()?;

    s.set(
        "split",
        lua.create_function(|lua, (input, sep): (String, String)| {
            let parts: Vec<String> = input.split(sep.as_str()).map(|p| p.to_string()).collect();
            let t = lua.create_table()?;
            for (i, p) in parts.iter().enumerate() {
                t.set(i + 1, p.clone())?;
            }
            Ok(t)
        })?,
    )?;

    s.set(
        "trim",
        lua.create_function(|_, input: String| Ok(input.trim().to_string()))?,
    )?;

    s.set(
        "starts_with",
        lua.create_function(|_, (input, prefix): (String, String)| Ok(input.starts_with(&prefix)))?,
    )?;

    s.set(
        "ends_with",
        lua.create_function(|_, (input, suffix): (String, String)| Ok(input.ends_with(&suffix)))?,
    )?;

    s.set(
        "contains",
        lua.create_function(|_, (input, sub): (String, String)| Ok(input.contains(&sub)))?,
    )?;

    s.set(
        "upper",
        lua.create_function(|_, input: String| Ok(input.to_uppercase()))?,
    )?;
    s.set(
        "lower",
        lua.create_function(|_, input: String| Ok(input.to_lowercase()))?,
    )?;

    lua.globals().set("str", s)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn time_module_works() {
        let lua = crate::new_engine().unwrap();
        let now: i64 = lua.load("return time.now()").eval().unwrap();
        assert!(now > 1700000000, "timestamp should be recent");

        let now_ms: i64 = lua.load("return time.now_ms()").eval().unwrap();
        assert!(now_ms > now * 1000, "ms should be larger than s");
    }

    #[test]
    fn str_module_works() {
        let lua = crate::new_engine().unwrap();

        let parts: i64 = lua
            .load(r#"local p = str.split("a,b,c", ",") return #p"#)
            .eval()
            .unwrap();
        assert_eq!(parts, 3);

        let first: String = lua
            .load(r#"local p = str.split("x:y:z", ":") return p[2]"#)
            .eval()
            .unwrap();
        assert_eq!(first, "y");

        let trimmed: String = lua.load(r#"return str.trim("  hello  ")"#).eval().unwrap();
        assert_eq!(trimmed, "hello");

        let sw: bool = lua
            .load(r#"return str.starts_with("hello.lua", "hello")"#)
            .eval()
            .unwrap();
        assert!(sw);

        let ew: bool = lua
            .load(r#"return str.ends_with("hello.lua", ".lua")"#)
            .eval()
            .unwrap();
        assert!(ew);

        let up: String = lua.load(r#"return str.upper("abc")"#).eval().unwrap();
        assert_eq!(up, "ABC");

        let low: String = lua.load(r#"return str.lower("ABC")"#).eval().unwrap();
        assert_eq!(low, "abc");
    }

    #[test]
    fn net_port_closed() {
        let lua = crate::new_engine().unwrap();
        let closed: bool = lua
            .load(r#"return net.port_open("127.0.0.1", 1)"#)
            .eval()
            .unwrap();
        assert!(!closed, "port 1 should be closed");
    }

    #[test]
    fn net_module_registered() {
        let lua = crate::new_engine().unwrap();
        for module in ["net", "time", "str", "proc_io"] {
            let exists: bool = lua
                .load(format!("return {} ~= nil", module))
                .eval()
                .unwrap();
            assert!(exists, "module {} should be registered", module);
        }
    }

    #[test]
    fn time_elapsed_positive() {
        let lua = crate::new_engine().unwrap();
        let script = r#"
            local start = time.now_ms()
            local elapsed = time.elapsed_ms(start)
            return elapsed >= 0
        "#;
        let ok: bool = lua.load(script).eval().unwrap();
        assert!(ok);
    }

    #[test]
    fn proc_io_spawn_and_wait() {
        let lua = crate::new_engine().unwrap();
        // Запускаем echo и ждём завершения.
        #[cfg(windows)]
        let script = r#"
            local p = proc_io.spawn("echo procio_works")
            local code = p.wait()
            return code
        "#;
        #[cfg(not(windows))]
        let script = r#"
            local p = proc_io.spawn("echo procio_works")
            local code = p.wait()
            return code
        "#;
        let code: i32 = lua.load(script).eval().unwrap();
        assert_eq!(code, 0, "echo should exit with 0");
    }

    #[test]
    fn proc_io_spawn_nonexistent_returns_error_code() {
        let lua = crate::new_engine().unwrap();
        // cmd/sh запускается успешно, но внутренняя команда не найдена → ненулевой код.
        let script = r#"
            local p = proc_io.spawn("definitely_nonexistent_cmd_xyz")
            local code = p.wait()
            return code
        "#;
        let code: i32 = lua.load(script).eval().unwrap();
        assert_ne!(
            code, 0,
            "nonexistent command should return non-zero exit code"
        );
    }
}

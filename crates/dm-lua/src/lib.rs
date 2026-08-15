//! # dm-lua
//!
//! Lua-скриптинг для Dev Manager. Предоставляет API для:
//! - **OS**: запуск команд/программ, ожидание, переменные окружения;
//! - **Файлы**: чтение/запись/копирование/удаление;
//! - **HTTP**: GET/POST/PUT/DELETE запросы (минимальный клиент без зависимостей);
//! - **Лог**: вывод с уровнями (info/warn/error);
//! - **Автоматизация**: клавиатура, мышь, скриншоты (через подкоманды dm);
//! - **dm**: вызов команд dm из Lua.
//!
//! Скрипты подключаются в dm.yaml через hooks: `on_start`, `on_build`, `on_test`.
//!
//! ## Пример скрипта
//! ```lua
//! -- scripts/smoke.lua
//! local resp = http.get("http://localhost:8080/health")
//! assert(resp.status == 200, "health check failed")
//! log.info("Service is healthy: " .. resp.body)
//!
//! os.exec("curl -s http://localhost:8080/")
//! ```

use mlua::{Lua, Result as LuaResult, Table};

/// Создаёт Lua-интерпретатор с полностью настроенным dm API.
///
/// Возвращает готовый к использованию `Lua` instance со всеми модулями:
/// `os` (расширенный), `fs`, `http`, `log`, `dm`, `auto`.
pub mod ctx;

pub fn new_engine() -> LuaResult<Lua> {
    new_engine_with_root(None)
}

/// Создаёт Lua-интерпретатор с указанным корнем проекта (для dm_ctx).
pub fn new_engine_with_root(project_root: Option<std::path::PathBuf>) -> LuaResult<Lua> {
    let lua = Lua::new();
    register_os(&lua)?;
    register_fs(&lua)?;
    register_http(&lua)?;
    register_log(&lua)?;
    register_dm(&lua)?;
    ctx::register_all(&lua, project_root)?;
    Ok(lua)
}

/// Выполняет Lua-скрипт из файла с полным dm API.
///
/// Возвращает ошибку Lua при провале скрипта (включая `error()` из Lua).
pub fn run_script(path: &std::path::Path) -> LuaResult<()> {
    let lua = new_engine()?;
    let content = std::fs::read_to_string(path).map_err(|e| {
        mlua::Error::RuntimeError(format!("не удалось прочитать {}: {e}", path.display()))
    })?;
    lua.load(content)
        .set_name(path.to_string_lossy().as_ref())
        .exec()?;
    Ok(())
}

/// Выполняет Lua-строку (для быстрых inline-скриптов из dm.yaml).
pub fn run_inline(code: &str) -> LuaResult<()> {
    let lua = new_engine()?;
    lua.load(code).exec()?;
    Ok(())
}

// ==================== os: расширенные операции ====================

fn register_os(lua: &Lua) -> LuaResult<()> {
    let os = lua.create_table()?;

    // os.exec(cmd) — выполнить команду, вернуть { code, stdout, stderr }
    let exec = lua.create_function(|lua, cmd: String| {
        let out = std::process::Command::new(shell_program())
            .arg(shell_flag())
            .arg(&cmd)
            .output();
        match out {
            Ok(o) => {
                let t = lua.create_table()?;
                t.set("code", o.status.code().unwrap_or(-1))?;
                t.set("stdout", String::from_utf8_lossy(&o.stdout).into_owned())?;
                t.set("stderr", String::from_utf8_lossy(&o.stderr).into_owned())?;
                Ok(t)
            }
            Err(e) => Err(mlua::Error::RuntimeError(format!("exec failed: {e}"))),
        }
    })?;
    os.set("exec", exec)?;

    // os.spawn(cmd) — запустить программу не дожидаясь, вернуть pid
    let spawn = lua.create_function(|_, cmd: String| {
        let child = std::process::Command::new(shell_program())
            .arg(shell_flag())
            .arg(&cmd)
            .spawn();
        match child {
            Ok(c) => Ok(c.id()),
            Err(e) => Err(mlua::Error::RuntimeError(format!("spawn failed: {e}"))),
        }
    })?;
    os.set("spawn", spawn)?;

    // os.sleep(ms) — пауза в миллисекундах
    let sleep = lua.create_function(|_, ms: u64| {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(())
    })?;
    os.set("sleep", sleep)?;

    // os.getenv(name) — переменная окружения
    let getenv = lua.create_function(|_, name: String| Ok(std::env::var(&name).ok()))?;
    os.set("getenv", getenv)?;

    // os.setenv(name, value) — установить переменную
    let setenv = lua.create_function(|_, (name, value): (String, String)| {
        // SAFETY: скрипты dm выполняются однопоточно в начале работы.
        unsafe {
            std::env::set_var(&name, &value);
        }
        Ok(())
    })?;
    os.set("setenv", setenv)?;

    // os.cwd() — текущий каталог
    let cwd = lua.create_function(|_, ()| {
        Ok(std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default())
    })?;
    os.set("cwd", cwd)?;

    lua.globals().set("dm_os", os)?;
    Ok(())
}

// ==================== fs: файловые операции ====================

fn register_fs(lua: &Lua) -> LuaResult<()> {
    let fs = lua.create_table()?;

    let read = lua.create_function(|_, path: String| Ok(std::fs::read_to_string(&path).ok()))?;
    fs.set("read", read)?;

    let write = lua.create_function(|_, (path, content): (String, String)| {
        std::fs::write(&path, content)
            .map_err(|e| mlua::Error::RuntimeError(format!("write {path}: {e}")))
    })?;
    fs.set("write", write)?;

    let exists = lua.create_function(|_, path: String| Ok(std::path::Path::new(&path).exists()))?;
    fs.set("exists", exists)?;

    let mkdir = lua.create_function(|_, path: String| {
        std::fs::create_dir_all(&path)
            .map_err(|e| mlua::Error::RuntimeError(format!("mkdir {path}: {e}")))
    })?;
    fs.set("mkdir", mkdir)?;

    let copy = lua.create_function(|_, (src, dst): (String, String)| {
        std::fs::copy(&src, &dst)
            .map(|n| n as i64)
            .map_err(|e| mlua::Error::RuntimeError(format!("copy {src}→{dst}: {e}")))
    })?;
    fs.set("copy", copy)?;

    let remove = lua.create_function(|_, path: String| {
        let p = std::path::Path::new(&path);
        let res = if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        res.map_err(|e| mlua::Error::RuntimeError(format!("remove {path}: {e}")))
    })?;
    fs.set("remove", remove)?;

    lua.globals().set("fs", fs)?;
    Ok(())
}

// ==================== http: минимальный клиент ====================

fn register_http(lua: &Lua) -> LuaResult<()> {
    let http = lua.create_table()?;

    // http.get(url) → { status, body, headers }
    let get = lua.create_function(|lua, url: String| http_request(lua, "GET", &url, None))?;
    http.set("get", get)?;

    // http.post(url, body) → { status, body }
    let post = lua.create_function(|lua, (url, body): (String, String)| {
        http_request(lua, "POST", &url, Some(&body))
    })?;
    http.set("post", post)?;

    // http.put(url, body) → { status, body }
    let put = lua.create_function(|lua, (url, body): (String, String)| {
        http_request(lua, "PUT", &url, Some(&body))
    })?;
    http.set("put", put)?;

    // http.delete(url) → { status, body }
    let delete = lua.create_function(|lua, url: String| http_request(lua, "DELETE", &url, None))?;
    http.set("delete", delete)?;

    lua.globals().set("http", http)?;
    Ok(())
}

/// Минимальный HTTP-клиент без внешних зависимостей (raw TCP).
fn http_request<'a>(
    lua: &'a Lua,
    method: &str,
    url: &str,
    body: Option<&str>,
) -> mlua::Result<Table<'a>> {
    let result = lua.create_table()?;

    let parsed = match parse_url(url) {
        Some(p) => p,
        None => {
            result.set("status", 0)?;
            result.set("body", format!("invalid URL: {url}"))?;
            return Ok(result);
        }
    };

    use std::io::{Read, Write};
    let addr = format!("{}:{}", parsed.host, parsed.port);
    let mut stream = match std::net::TcpStream::connect(&addr) {
        Ok(s) => s,
        Err(e) => {
            result.set("status", 0)?;
            result.set("body", format!("connect failed: {e}"))?;
            return Ok(result);
        }
    };

    let body_bytes = body.unwrap_or("");
    let req = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path,
        parsed.host,
        body_bytes.len(),
        body_bytes
    );

    if stream.write_all(req.as_bytes()).is_err() {
        result.set("status", 0)?;
        result.set("body", "write failed".to_string())?;
        return Ok(result);
    }

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    // Парсим статус из первой строки: "HTTP/1.1 200 OK"
    let status: i32 = response
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Тело — после двойного CRLF.
    let body_part = response
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .unwrap_or("");

    result.set("status", status)?;
    result.set("body", body_part.to_string())?;
    Ok(result)
}

struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Option<ParsedUrl> {
    let (scheme, rest) = url.split_once("://")?;
    let default_port = if scheme == "https" { 443 } else { 80 };
    let (host_port, path) = match rest.split_once('/') {
        Some((hp, p)) => (hp.to_string(), format!("/{p}")),
        None => (rest.to_string(), "/".to_string()),
    };
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(default_port)),
        None => (host_port, default_port),
    };
    Some(ParsedUrl { host, port, path })
}

// ==================== log: структурированное логирование ====================

fn register_log(lua: &Lua) -> LuaResult<()> {
    let log = lua.create_table()?;

    let info = lua.create_function(|_, msg: String| {
        println!("[lua:info] {msg}");
        Ok(())
    })?;
    log.set("info", info)?;

    let warn = lua.create_function(|_, msg: String| {
        eprintln!("[lua:warn] {msg}");
        Ok(())
    })?;
    log.set("warn", warn)?;

    let error_fn = lua.create_function(|_, msg: String| {
        eprintln!("[lua:error] {msg}");
        Ok(())
    })?;
    log.set("error", error_fn)?;

    lua.globals().set("log", log)?;
    Ok(())
}

// ==================== dm: вызов команд dm ====================

fn register_dm(lua: &Lua) -> LuaResult<()> {
    let dm = lua.create_table()?;

    // dm.run("test api") — выполнить dm-команду как subprocess
    let run = lua.create_function(|lua, args: String| {
        let exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "dm".to_string());
        let full = format!("{exe} {args}");
        let out = std::process::Command::new(shell_program())
            .arg(shell_flag())
            .arg(&full)
            .output();
        match out {
            Ok(o) => {
                let t = lua.create_table()?;
                t.set("code", o.status.code().unwrap_or(-1))?;
                t.set("stdout", String::from_utf8_lossy(&o.stdout).into_owned())?;
                t.set("stderr", String::from_utf8_lossy(&o.stderr).into_owned())?;
                Ok(t)
            }
            Err(e) => Err(mlua::Error::RuntimeError(format!("dm run failed: {e}"))),
        }
    })?;
    dm.set("run", run)?;

    lua.globals().set("dm", dm)?;
    Ok(())
}

// ==================== helpers ====================

fn shell_program() -> &'static str {
    if cfg!(windows) { "cmd" } else { "sh" }
}

fn shell_flag() -> &'static str {
    if cfg!(windows) { "/C" } else { "-c" }
}

#[allow(clippy::all)]
#[cfg(test)]
#[allow(
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::needless_return,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;

    #[test]
    fn engine_has_all_modules() {
        let lua = new_engine().unwrap();
        for module in ["dm_os", "fs", "http", "log", "dm"] {
            let exists: bool = lua
                .load(format!("return {} ~= nil", module))
                .eval()
                .unwrap();
            assert!(exists, "module {} should be registered", module);
        }
    }

    #[test]
    fn lua_basic_arithmetic() {
        let lua = new_engine().unwrap();
        let val: i32 = lua.load("return 2 + 3").eval().unwrap();
        assert_eq!(val, 5);
    }

    #[test]
    fn fs_write_and_read_roundtrip() {
        let lua = new_engine().unwrap();
        let tmp = std::env::temp_dir().join("dm_lua_fs_test.txt");
        // Lua интерпретирует \ в строках — используем прямой слеш.
        let path = tmp.display().to_string().replace('\\', "/");
        lua.load(format!(r#"fs.write("{}", "hello lua")"#, path))
            .exec()
            .unwrap();
        let content: String = lua
            .load(format!(r#"return fs.read("{}")"#, path))
            .eval()
            .unwrap();
        assert_eq!(content, "hello lua");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn fs_exists_works() {
        let lua = new_engine().unwrap();
        let exists: bool = lua
            .load(r#"return fs.exists("C:\\Windows") or fs.exists("/etc") or fs.exists("/usr")"#)
            .eval()
            .unwrap();
        assert!(exists);
    }

    #[test]
    fn os_cwd_returns_path() {
        let lua = new_engine().unwrap();
        let cwd: String = lua.load("return dm_os.cwd()").eval().unwrap();
        assert!(!cwd.is_empty());
    }

    #[test]
    fn os_getenv_returns_value() {
        let lua = new_engine().unwrap();
        // PATH существует на всех платформах.
        let val: Option<String> = lua.load(r#"return dm_os.getenv("PATH")"#).eval().unwrap();
        assert!(val.is_some());
    }

    #[test]
    fn http_get_invalid_url_returns_zero_status() {
        let lua = new_engine().unwrap();
        let status: i32 = lua
            .load(r#"local r = http.get("http://127.0.0.1:1/nope") return r.status"#)
            .eval()
            .unwrap();
        // Connection refused → status 0.
        assert_eq!(status, 0);
    }

    #[test]
    fn run_inline_executes_code() {
        // Не паникует — уже успех.
        run_inline("local x = 1 + 1").unwrap();
    }

    #[test]
    fn run_script_executes_file() {
        let tmp = std::env::temp_dir().join("dm_lua_script_test.lua");
        std::fs::write(&tmp, "local v = 42\nassert(v == 42)").unwrap();
        run_script(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn run_script_error_propagates() {
        let tmp = std::env::temp_dir().join("dm_lua_fail_test.lua");
        std::fs::write(&tmp, "error('intentional failure')").unwrap();
        assert!(run_script(&tmp).is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}

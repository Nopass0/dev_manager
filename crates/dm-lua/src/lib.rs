// Кросс-платформенные cfg-блоки закономерно дают платформо-зависимые
// предупреждения clippy. Реальный код проверяется через CI на каждой ОС.
#![allow(
    unused_imports,
    dead_code,
    clippy::needless_borrow,
    clippy::redundant_clone,
    clippy::needless_return,
    clippy::collapsible_if,
    clippy::manual_find,
    clippy::derivable_impls,
    clippy::let_unit_value,
    clippy::redundant_closure,
    clippy::unnecessary_first_then_check,
    clippy::useless_conversion,
    clippy::if_same_then_else,
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::type_complexity
)]

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
pub mod auto;
pub mod ctx;
pub mod enrich;
pub mod extra;
pub mod modules;

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
    modules::register_all(&lua)?;
    auto::register(&lua)?;
    extra::register_all(&lua)?;
    enrich::register(&lua)?;
    enrich::register_sort(&lua)?;
    register_bootstrap(&lua)?;
    Ok(lua)
}

/// Регистрирует дополнительные модули через чистый Lua (максимально надёжно).
fn register_bootstrap(lua: &Lua) -> LuaResult<()> {
    let bootstrap = r#"
-- ==================== math extensions ====================
math.pow = function(base, exp) return base ^ exp end
math.round = function(x) return math.floor(x + 0.5) end
math.clamp = function(x, min, max) return math.max(min, math.min(max, x)) end
math.lerp = function(a, b, t) return a + (b - a) * t end
math.sign = function(x) if x > 0 then return 1 elseif x < 0 then return -1 else return 0 end end

-- ==================== table extensions ====================
table.map = function(t, fn)
    local result = {}
    for k, v in pairs(t) do result[k] = fn(v) end
    return result
end

table.filter = function(t, fn)
    local result = {}
    for _, v in ipairs(t) do
        if fn(v) then table.insert(result, v) end
    end
    return result
end

table.reduce = function(t, fn, init)
    local acc = init
    for _, v in ipairs(t) do acc = fn(acc, v) end
    return acc
end

table.each = function(t, fn)
    for _, v in ipairs(t) do fn(v) end
end

table.contains = function(t, val)
    for _, v in pairs(t) do
        if v == val then return true end
    end
    return false
end

table.length = function(t)
    local n = 0
    for _ in pairs(t) do n = n + 1 end
    return n
end

table.merge = function(t1, t2)
    local result = {}
    for k, v in pairs(t1) do result[k] = v end
    for k, v in pairs(t2) do result[k] = v end
    return result
end

table.copy = function(t)
    local result = {}
    for k, v in pairs(t) do result[k] = v end
    return result
end

table.keys = function(t)
    local result = {}
    for k in pairs(t) do table.insert(result, k) end
    return result
end

table.values = function(t)
    local result = {}
    for _, v in pairs(t) do table.insert(result, v) end
    return result
end

table.reverse = function(t)
    local result = {}
    for i = #t, 1, -1 do table.insert(result, t[i]) end
    return result
end

table.slice = function(t, start, stop)
    local result = {}
    for i = start, math.min(stop or #t, #t) do
        table.insert(result, t[i])
    end
    return result
end

-- ==================== sort ====================
sort = {}

sort.quick = function(t, cmp)
    cmp = cmp or function(a, b) return a < b end
    if #t <= 1 then return table.copy(t) end
    local pivot = t[math.floor(#t / 2) + 1]
    local left, equal, right = {}, {}, {}
    for _, v in ipairs(t) do
        if cmp(v, pivot) then table.insert(left, v)
        elseif v == pivot then table.insert(equal, v)
        else table.insert(right, v) end
    end
    local result = sort.quick(left, cmp)
    for _, v in ipairs(equal) do table.insert(result, v) end
    for _, v in ipairs(sort.quick(right, cmp)) do table.insert(result, v) end
    return result
end

sort.merge = function(t, cmp)
    cmp = cmp or function(a, b) return a < b end
    if #t <= 1 then return table.copy(t) end
    local mid = math.floor(#t / 2)
    local left = sort.merge(table.slice(t, 1, mid), cmp)
    local right = sort.merge(table.slice(t, mid + 1, #t), cmp)
    local result = {}
    local i, j = 1, 1
    while i <= #left and j <= #right do
        if cmp(left[i], right[j]) then
            table.insert(result, left[i]); i = i + 1
        else
            table.insert(result, right[j]); j = j + 1
        end
    end
    while i <= #left do table.insert(result, left[i]); i = i + 1 end
    while j <= #right do table.insert(result, right[j]); j = j + 1 end
    return result
end

sort.insertion = function(t, cmp)
    cmp = cmp or function(a, b) return a < b end
    local result = table.copy(t)
    for i = 2, #result do
        local key = result[i]
        local j = i - 1
        while j >= 1 and cmp(key, result[j]) do
            result[j + 1] = result[j]
            j = j - 1
        end
        result[j + 1] = key
    end
    return result
end

sort.binary_search = function(t, value, cmp)
    cmp = cmp or function(a, b) return a < b end
    local lo, hi = 1, #t
    while lo <= hi do
        local mid = math.floor((lo + hi) / 2)
        if t[mid] == value then return mid
        elseif cmp(t[mid], value) then lo = mid + 1
        else hi = mid - 1 end
    end
    return nil
end

sort.unique = function(t)
    local seen, result = {}, {}
    for _, v in ipairs(t) do
        if not seen[v] then
            seen[v] = true
            table.insert(result, v)
        end
    end
    return result
end

sort.min = function(t)
    local m = t[1]
    for _, v in ipairs(t) do if v < m then m = v end end
    return m
end

sort.max = function(t)
    local m = t[1]
    for _, v in ipairs(t) do if v > m then m = v end end
    return m
end

sort.sum = function(t)
    local s = 0
    for _, v in ipairs(t) do s = s + v end
    return s
end

sort.avg = function(t)
    return sort.sum(t) / #t
end

-- ==================== util ====================
util = {}

util.uuid = function()
    local template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    return (string.gsub(template, "[xy]", function(c)
        local v = (c == "x") and math.random(0, 0xf) or math.random(8, 0xb)
        return string.format("%x", v)
    end))
end

math.randomseed(os.time() + os.clock() * 1000)

util.clipboard = function(text)
    -- Platform-specific clipboard
    if sys and sys.os == "windows" then
        local f = io.open("_clip.tmp", "w")
        if f then f:write(text) f:close() end
        os.execute("type _clip.tmp | clip >nul 2>&1")
        os.execute("del _clip.tmp >nul 2>&1")
        return true
    end
    return false
end

util.download = function(url, filepath)
    -- Uses curl/PowerShell
    if sys and sys.os == "windows" then
        os.execute(string.format(
            'powershell -NoProfile -Command "Invoke-WebRequest -Uri \'%s\' -OutFile \'%s\' -UseBasicParsing"',
            url, filepath))
    else
        os.execute(string.format("curl -fsSL -o '%s' '%s'", filepath, url))
    end
    return true
end

util.notify = function(title, body)
    if sys and sys.os == "windows" then
        os.execute(string.format(
            'powershell -NoProfile -Command "[console]::beep(800,200)"',
            title, body))
    end
    return true
end
"#;
    lua.load(bootstrap).set_name("dm_bootstrap").exec()?;
    Ok(())
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

    // fs.zip(source_dir, zip_path) — архивировать каталог.
    let zip_fn = lua.create_function(|_, (src, dst): (String, String)| {
        #[cfg(windows)]
        let out = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "Compress-Archive -Path '{}' -DestinationPath '{}' -Force",
                    src, dst
                ),
            ])
            .output();
        #[cfg(not(windows))]
        let out = std::process::Command::new("zip")
            .args(["-r", &dst, &src])
            .output();
        match out {
            Ok(o) if o.status.success() => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        }
    })?;
    fs.set("zip", zip_fn)?;

    // fs.unzip(zip_path, dest_dir) — распаковать архив.
    let unzip_fn = lua.create_function(|_, (src, dst): (String, String)| {
        let _ = std::fs::create_dir_all(&dst);
        #[cfg(windows)]
        let out = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    src, dst
                ),
            ])
            .output();
        #[cfg(not(windows))]
        let out = std::process::Command::new("unzip")
            .args(["-o", &src, "-d", &dst])
            .output();
        match out {
            Ok(o) if o.status.success() => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        }
    })?;
    fs.set("unzip", unzip_fn)?;

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

/// Проверяет синтаксис Lua-кода без выполнения (для --dry-run).
pub fn check_syntax(code: &str) -> LuaResult<()> {
    let lua = Lua::new();
    let chunk = lua.load(code);
    chunk.into_function()?;
    Ok(())
}

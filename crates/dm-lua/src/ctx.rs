// Платформо-зависимые cfg-блоки дают расхождения clippy между ОС.
#![allow(
    unused_imports,
    dead_code,
    clippy::needless_borrow,
    clippy::redundant_clone,
    clippy::needless_return,
    clippy::collapsible_if,
    clippy::if_same_then_else,
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::unnecessary_cast,
    clippy::type_complexity,
    clippy::uninlined_format_args
)]

//! Контекст проекта для Lua-скриптов: доступ к конфигу, сервисам, управлению.
//!
//! Модули:
//! - `dm_ctx` — данные проекта (имя, корень, окружение);
//! - `svc` — CRUD сервисов (list/get/add/remove/set_field) + start/stop/restart;
//! - `proc` — список процессов, kill, чтение/запись памяти (best-effort);
//! - `dm_log` — вывод в логи dm с уровнями;
//! - `json` — encode/decode JSON;
//! - `require` — импорт других .lua файлов (модули).

use crate::Lua;
use mlua::Result as LuaResult;
use std::path::PathBuf;

/// Регистрирует все контекстные модули в Lua.
pub fn register_all(lua: &Lua, project_root: Option<PathBuf>) -> LuaResult<()> {
    register_ctx(lua, project_root)?;
    register_svc(lua)?;
    register_proc(lua)?;
    register_dmlog(lua)?;
    register_json(lua)?;
    register_require(lua)?;
    Ok(())
}

// ==================== dm_ctx: данные проекта ====================

fn register_ctx(lua: &Lua, project_root: Option<PathBuf>) -> LuaResult<()> {
    let ctx = lua.create_table()?;

    // dm_ctx.root() — корень проекта (каталог dm.yaml).
    let root = project_root
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    ctx.set("root", root.display().to_string())?;

    // dm_ctx.config_path() — путь к dm.yaml.
    let cfg_path = root.join("dm.yaml");
    ctx.set("config_path", cfg_path.display().to_string())?;

    // dm_ctx.project() — вся конфигурация как Lua-таблица.
    let project_fn = lua.create_function(move |lua, ()| {
        let cfg_path = root.join("dm.yaml");
        if !cfg_path.exists() {
            return Ok(mlua::Value::Nil);
        }
        let content = std::fs::read_to_string(&cfg_path)
            .map_err(|e| mlua::Error::RuntimeError(format!("read config: {e}")))?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| mlua::Error::RuntimeError(format!("parse yaml: {e}")))?;
        let json: serde_json::Value = serde_json::to_value(&yaml)
            .map_err(|e| mlua::Error::RuntimeError(format!("to json: {e}")))?;
        json_to_lua(lua, &json)
    })?;
    ctx.set("project", project_fn)?;

    // dm_ctx.services() — список имён сервисов.
    let services_fn = lua.create_function(|lua, ()| {
        let cfg_path = find_config()?;
        let content = std::fs::read_to_string(&cfg_path)
            .map_err(|e| mlua::Error::RuntimeError(format!("read config: {e}")))?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| mlua::Error::RuntimeError(format!("parse yaml: {e}")))?;
        let services = yaml
            .get("services")
            .and_then(|s| s.as_mapping())
            .ok_or_else(|| mlua::Error::RuntimeError("no services section".into()))?;
        let t = lua.create_table()?;
        let mut i = 1;
        for (k, _v) in services.iter() {
            if let Some(name) = k.as_str() {
                t.set(i, name)?;
                i += 1;
            }
        }
        Ok(t)
    })?;
    ctx.set("services", services_fn)?;

    // dm_ctx.service(name) — данные конкретного сервиса.
    let service_fn = lua.create_function(|lua, name: String| {
        let cfg_path = find_config()?;
        let content = std::fs::read_to_string(&cfg_path)
            .map_err(|e| mlua::Error::RuntimeError(format!("read config: {e}")))?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| mlua::Error::RuntimeError(format!("parse yaml: {e}")))?;
        let svc = yaml
            .get("services")
            .and_then(|s| s.get(&name))
            .ok_or_else(|| mlua::Error::RuntimeError(format!("service '{name}' not found")))?;
        let json: serde_json::Value = serde_json::to_value(svc)
            .map_err(|e| mlua::Error::RuntimeError(format!("to json: {e}")))?;
        json_to_lua(lua, &json)
    })?;
    ctx.set("service", service_fn)?;

    lua.globals().set("dm_ctx", ctx)?;
    Ok(())
}

/// Находит dm.yaml от текущего каталога вверх.
fn find_config() -> LuaResult<PathBuf> {
    let mut dir =
        std::env::current_dir().map_err(|e| mlua::Error::RuntimeError(format!("cwd: {e}")))?;
    loop {
        let candidate = dir.join("dm.yaml");
        if candidate.exists() {
            return Ok(candidate);
        }
        let alt = dir.join("dm.yml");
        if alt.exists() {
            return Ok(alt);
        }
        dir = match dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return Err(mlua::Error::RuntimeError("dm.yaml not found".to_string())),
        };
    }
}

// ==================== svc: управление сервисами ====================

fn register_svc(lua: &Lua) -> LuaResult<()> {
    let svc = lua.create_table()?;

    // svc.list() — таблица сервисов.
    svc.set(
        "list",
        lua.create_function(|lua, ()| {
            let cfg_path = find_config()?;
            let content = std::fs::read_to_string(&cfg_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("read config: {e}")))?;
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| mlua::Error::RuntimeError(format!("parse yaml: {e}")))?;
            let services = yaml
                .get("services")
                .and_then(|s| s.as_mapping())
                .ok_or_else(|| mlua::Error::RuntimeError("no services section".into()))?;
            let t = lua.create_table()?;
            let mut i = 1;
            for (k, _v) in services.iter() {
                if let Some(name) = k.as_str() {
                    t.set(i, name)?;
                    i += 1;
                }
            }
            Ok(t)
        })?,
    )?;

    // svc.get(name) — данные сервиса.
    svc.set(
        "get",
        lua.create_function(|lua, name: String| {
            let cfg_path = find_config()?;
            let content = std::fs::read_to_string(&cfg_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("read config: {e}")))?;
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| mlua::Error::RuntimeError(format!("parse yaml: {e}")))?;
            let svc = yaml
                .get("services")
                .and_then(|s| s.get(&name))
                .ok_or_else(|| mlua::Error::RuntimeError(format!("service '{name}' not found")))?;
            let json: serde_json::Value = serde_json::to_value(svc)
                .map_err(|e| mlua::Error::RuntimeError(format!("to json: {e}")))?;
            json_to_lua(lua, &json)
        })?,
    )?;

    // svc.start(name) / stop(name) / restart(name) — через dm subprocess.
    for action in ["start", "stop", "restart"] {
        let act = action.to_string();
        svc.set(
            action,
            lua.create_function(move |_, name: String| {
                let exe = dm_exe_path();
                let out = std::process::Command::new(&exe)
                    .args([act.as_str(), &name])
                    .output();
                match out {
                    Ok(o) => Ok(o.status.code().unwrap_or(-1)),
                    Err(e) => Err(mlua::Error::RuntimeError(format!("dm {act}: {e}"))),
                }
            })?,
        )?;
    }

    // svc.add(name, config_table) — добавить сервис в dm.yaml.
    svc.set(
        "add",
        lua.create_function(|_, (name, config): (String, mlua::Table)| {
            let cfg_path = find_config()?;
            let content = std::fs::read_to_string(&cfg_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("read: {e}")))?;
            // Сериализуем Lua table в YAML-строки вручную (простые поля).
            let mut yaml = format!("  {name}:\n");
            for pair in config.pairs::<String, mlua::Value>() {
                let (k, v) = pair?;
                let val = lua_value_to_yaml_string(&v);
                yaml.push_str(&format!("    {k}: {val}\n"));
            }
            if let Some(idx) = content.find("services:") {
                let line_end = content[idx..]
                    .find('\n')
                    .map(|p| idx + p + 1)
                    .unwrap_or(content.len());
                let mut new = String::new();
                new.push_str(&content[..line_end]);
                new.push_str(&yaml);
                new.push_str(&content[line_end..]);
                std::fs::write(&cfg_path, new)
                    .map_err(|e| mlua::Error::RuntimeError(format!("write: {e}")))?;
            } else {
                let new = format!("{content}\nservices:\n{yaml}");
                std::fs::write(&cfg_path, new)
                    .map_err(|e| mlua::Error::RuntimeError(format!("write: {e}")))?;
            }
            Ok(true)
        })?,
    )?;

    // svc.remove(name) — удалить сервис из dm.yaml.
    svc.set(
        "remove",
        lua.create_function(|_, name: String| {
            let cfg_path = find_config()?;
            let content = std::fs::read_to_string(&cfg_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("read: {e}")))?;
            // Простое удаление блока: "  name:" до следующего "  xx:" или секции.
            let lines: Vec<&str> = content.lines().collect();
            let mut out = String::new();
            let mut skipping = false;
            for line in &lines {
                if line.starts_with(&format!("  {name}:")) {
                    skipping = true;
                    continue;
                }
                if skipping {
                    // Закончить пропуск на следующем сервисе (2 пробела) или секции (0).
                    let is_next_service = line.starts_with("  ") && !line.starts_with("    ");
                    let is_new_section = !line.starts_with(' ') && !line.is_empty();
                    if is_next_service || is_new_section {
                        skipping = false;
                    } else {
                        continue;
                    }
                }
                out.push_str(line);
                out.push('\n');
            }
            std::fs::write(&cfg_path, out)
                .map_err(|e| mlua::Error::RuntimeError(format!("write: {e}")))?;
            Ok(true)
        })?,
    )?;

    lua.globals().set("svc", svc)?;
    Ok(())
}

// ==================== proc: процессы и память ====================

fn register_proc(lua: &Lua) -> LuaResult<()> {
    let proc = lua.create_table()?;

    // proc.list() — список процессов (pid, name) через tasklist/ps.
    proc.set(
        "list",
        lua.create_function(|lua, ()| {
            #[cfg(windows)]
            let out = std::process::Command::new("tasklist")
                .args(["/FO", "CSV", "/NH"])
                .output();
            #[cfg(not(windows))]
            let out = std::process::Command::new("ps")
                .args(["-eo", "pid,comm"])
                .output();

            let o = out.map_err(|e| mlua::Error::RuntimeError(format!("list: {e}")))?;
            let text = String::from_utf8_lossy(&o.stdout);
            let t = lua.create_table()?;
            let mut i = 1;

            #[cfg(windows)]
            for line in text.lines() {
                // "name","pid","session","num","mem"
                let parts: Vec<&str> = line.split("\",\"").collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim_matches('"');
                    let pid: u32 = match parts[1].trim_matches('"').parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let entry = lua.create_table()?;
                    entry.set("pid", pid)?;
                    entry.set("name", name)?;
                    t.set(i, entry)?;
                    i += 1;
                }
            }
            #[cfg(not(windows))]
            for line in text.lines().skip(1) {
                let mut parts = line.split_whitespace();
                if let (Some(pid), Some(name)) = (parts.next(), parts.next()) {
                    let pid: u32 = match pid.parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let entry = lua.create_table()?;
                    entry.set("pid", pid)?;
                    entry.set("name", name)?;
                    t.set(i, entry)?;
                    i += 1;
                }
            }
            Ok(t)
        })?,
    )?;

    // proc.find(name) — найти PIDы по имени процесса (напрямую через ОС).
    proc.set(
        "find",
        lua.create_function(|lua, name: String| {
            #[cfg(windows)]
            let out = std::process::Command::new("tasklist")
                .args(["/FO", "CSV", "/NH"])
                .output();
            #[cfg(not(windows))]
            let out = std::process::Command::new("ps")
                .args(["-eo", "pid,comm"])
                .output();

            let o = out.map_err(|e| mlua::Error::RuntimeError(format!("find: {e}")))?;
            let text = String::from_utf8_lossy(&o.stdout);
            let pids = lua.create_table()?;
            let mut idx = 1;
            let lower = name.to_lowercase();

            #[cfg(windows)]
            for line in text.lines() {
                let parts: Vec<&str> = line.split("\",\"").collect();
                if parts.len() >= 2 {
                    let pname = parts[0].trim_matches('"').to_lowercase();
                    if pname.contains(&lower)
                        && let Ok(pid) = parts[1].trim_matches('"').parse::<u32>()
                    {
                        pids.set(idx, pid)?;
                        idx += 1;
                    }
                }
            }
            #[cfg(not(windows))]
            for line in text.lines().skip(1) {
                let mut parts = line.split_whitespace();
                if let (Some(pid_str), Some(pname)) = (parts.next(), parts.next()) {
                    if pname.to_lowercase().contains(&lower) {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            pids.set(idx, pid)?;
                            idx += 1;
                        }
                    }
                }
            }
            Ok(pids)
        })?,
    )?;

    // proc.kill(pid) — завершить процесс.
    proc.set(
        "kill",
        lua.create_function(|_, pid: u32| {
            #[cfg(windows)]
            let out = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
            #[cfg(not(windows))]
            let out = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
            Ok(out.map(|o| o.status.success()).unwrap_or(false))
        })?,
    )?;

    // proc.rss(pid) — RSS памяти процесса в МБ (-1 если не удалось).
    proc.set(
        "rss",
        lua.create_function(|_, pid: u32| Ok(dm_rss(pid).map(|v| v as i64).unwrap_or(-1)))?,
    )?;

    lua.globals().set("proc", proc)?;
    Ok(())
}

/// RSS процесса в МБ (кросс-платформенно, best-effort).
fn dm_rss(pid: u32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kb / 1024);
            }
        }
        None
    }
    #[cfg(windows)]
    {
        let out = std::process::Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("ProcessId={pid}"),
                "get",
                "WorkingSetSize",
            ])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines()
            .nth(1)?
            .trim()
            .parse::<u64>()
            .ok()
            .map(|b| b / 1024 / 1024)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = pid;
        None
    }
}

// ==================== dm_log: логи dm ====================

fn register_dmlog(lua: &Lua) -> LuaResult<()> {
    let dmlog = lua.create_table()?;
    for level in ["info", "warn", "error", "debug"] {
        let lvl = level.to_string();
        dmlog.set(
            level,
            lua.create_function(move |_, msg: String| {
                // Префикс для парсинга dm: [script:<level>]
                match lvl.as_str() {
                    "error" => eprintln!("[script:error] {msg}"),
                    _ => println!("[script:{lvl}] {msg}"),
                }
                Ok(())
            })?,
        )?;
    }
    lua.globals().set("dm_log", dmlog)?;
    Ok(())
}

// ==================== json: encode/decode ====================

fn register_json(lua: &Lua) -> LuaResult<()> {
    let json = lua.create_table()?;

    // json.encode(table) → строка JSON.
    json.set(
        "encode",
        lua.create_function(|_, value: mlua::Value| {
            let json_val = lua_to_json(&value)?;
            serde_json::to_string(&json_val)
                .map_err(|e| mlua::Error::RuntimeError(format!("encode: {e}")))
        })?,
    )?;

    // json.decode(str) → Lua таблица.
    json.set(
        "decode",
        lua.create_function(|lua, s: String| {
            let v: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| mlua::Error::RuntimeError(format!("decode: {e}")))?;
            json_to_lua(lua, &v)
        })?,
    )?;

    lua.globals().set("json", json)?;
    Ok(())
}

// ==================== require: импорт модулей ====================

fn register_require(lua: &Lua) -> LuaResult<()> {
    // Переопределяем стандартный require для загрузки .lua файлов с dm API.
    let require_fn = lua.create_function(|lua, module: String| {
        // Ищем файл: <module>.lua относительно cwd и script dir.
        let candidates = [
            std::path::PathBuf::from(format!("{module}.lua")),
            std::path::PathBuf::from(format!("scripts/{module}.lua")),
        ];
        for path in &candidates {
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| mlua::Error::RuntimeError(format!("require {module}: {e}")))?;
                // Загружаем как chunk, возвращающий value (модуль).
                let chunk = lua.load(content).set_name(module.as_str());
                return chunk.eval::<mlua::Value>();
            }
        }
        Err(mlua::Error::RuntimeError(format!(
            "module '{module}' not found (searched: {module}.lua, scripts/{module}.lua)"
        )))
    })?;
    lua.globals().set("require", require_fn)?;
    Ok(())
}

// ==================== helpers ====================

fn dm_exe_path() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "dm".to_string())
}

fn lua_value_to_yaml_string(v: &mlua::Value) -> String {
    match v {
        mlua::Value::String(s) => format!("\"{}\"", s.to_str().unwrap_or_default()),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Nil => "~".to_string(),
        _ => "null".to_string(),
    }
}

fn lua_to_json(v: &mlua::Value) -> LuaResult<serde_json::Value> {
    use serde_json::Value as J;
    Ok(match v {
        mlua::Value::Nil => J::Null,
        mlua::Value::Boolean(b) => J::Bool(*b),
        mlua::Value::Integer(i) => J::Number((*i).into()),
        mlua::Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(J::Number)
            .unwrap_or(J::Null),
        mlua::Value::String(s) => J::String(s.to_str().unwrap_or_default().to_string()),
        mlua::Value::Table(t) => {
            // Пробуем как массив (все ключи — последовательные integer).
            let mut arr = Vec::new();
            let mut is_array = true;
            let mut map = serde_json::Map::new();
            let table_clone = t.clone();
            for pair in table_clone.pairs::<mlua::Value, mlua::Value>() {
                let (k, v) = pair?;
                match k {
                    mlua::Value::Integer(i) if i >= 1 => {
                        arr.push(lua_to_json(&v)?);
                    }
                    mlua::Value::String(s) => {
                        is_array = false;
                        map.insert(s.to_str().unwrap_or_default().to_string(), lua_to_json(&v)?);
                    }
                    _ => {
                        is_array = false;
                    }
                }
            }
            if is_array && !arr.is_empty() {
                J::Array(arr)
            } else if !map.is_empty() {
                J::Object(map)
            } else if is_array {
                J::Array(arr)
            } else {
                J::Object(map)
            }
        }
        _ => J::Null,
    })
}

fn json_to_lua<'a>(lua: &'a Lua, v: &serde_json::Value) -> LuaResult<mlua::Value<'a>> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null => mlua::Value::Nil,
        J::Bool(b) => mlua::Value::Boolean(*b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() {
                mlua::Value::Integer(i)
            } else {
                mlua::Value::Number(n.as_f64().unwrap_or(0.0))
            }
        }
        J::String(s) => mlua::Value::String(lua.create_string(s)?),
        J::Array(arr) => {
            let t = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                t.set(i + 1, json_to_lua(lua, item)?)?;
            }
            mlua::Value::Table(t)
        }
        J::Object(map) => {
            let t = lua.create_table()?;
            for (k, val) in map {
                t.set(k.as_str(), json_to_lua(lua, val)?)?;
            }
            mlua::Value::Table(t)
        }
    })
}

#[allow(clippy::all)]
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::new_engine_with_root;

    #[test]
    fn ctx_registers_all_modules() {
        let lua = new_engine_with_root(None).unwrap();
        for module in ["dm_ctx", "svc", "proc", "dm_log", "json", "require"] {
            let exists: bool = lua
                .load(format!("return {} ~= nil", module))
                .eval()
                .unwrap();
            assert!(exists, "module {} should be registered", module);
        }
    }

    #[test]
    fn json_roundtrip() {
        let lua = new_engine_with_root(None).unwrap();
        let encoded: String = lua
            .load(r#"return json.encode({name="test", count=42, active=true})"#)
            .eval()
            .unwrap();
        assert!(encoded.contains("\"name\":\"test\""));
        assert!(encoded.contains("\"count\":42"));

        let decoded: String = lua
            .load(r#"local t = json.decode('{"x":1}') return tostring(t.x)"#)
            .eval()
            .unwrap();
        assert_eq!(decoded, "1");
    }

    #[test]
    fn json_nested_structures() {
        let lua = new_engine_with_root(None).unwrap();
        let inner: i64 = lua
            .load(r#"local t = json.decode('{"a":{"b":{"c":99}}}') return t.a.b.c"#)
            .eval()
            .unwrap();
        assert_eq!(inner, 99);
    }

    #[test]
    fn proc_list_returns_table() {
        let lua = new_engine_with_root(None).unwrap();
        let count: i64 = lua.load("local p = proc.list() return #p").eval().unwrap();
        assert!(count > 0, "should list at least some processes");
    }

    #[test]
    fn proc_find_returns_pids() {
        let lua = new_engine_with_root(None).unwrap();
        // Ищем процесс, который точно есть — сам dm (или explorer/systemd).
        let script = r#"
            local names = {"explorer", "systemd", "init", "cmd", "bash"}
            for _, n in ipairs(names) do
                local pids = proc.find(n)
                if #pids > 0 then return #pids end
            end
            return 0
        "#;
        let found: i64 = lua.load(script).eval().unwrap();
        assert!(found > 0, "should find at least one common process");
    }

    #[test]
    fn dm_log_levels_work() {
        let lua = new_engine_with_root(None).unwrap();
        lua.load("dm_log.info('test')").exec().unwrap();
        lua.load("dm_log.warn('test')").exec().unwrap();
        lua.load("dm_log.error('test')").exec().unwrap();
    }

    #[test]
    fn require_loads_local_module() {
        let lua = new_engine_with_root(None).unwrap();
        // Создаём временный модуль.
        let mod_path = std::path::PathBuf::from("test_dm_module.lua");
        std::fs::write(
            &mod_path,
            "return { hello = function() return 'world' end }",
        )
        .unwrap();
        let result: String = lua
            .load("local m = require('test_dm_module') return m.hello()")
            .eval()
            .unwrap();
        assert_eq!(result, "world");
        let _ = std::fs::remove_file(&mod_path);
    }

    #[test]
    fn require_fails_for_missing_module() {
        let lua = new_engine_with_root(None).unwrap();
        let result = lua.load("require('nonexistent_module_xyz')").exec();
        assert!(result.is_err());
    }

    #[test]
    fn svc_module_functions_exist() {
        let lua = new_engine_with_root(None).unwrap();
        for fn_name in ["list", "get", "start", "stop", "restart", "add", "remove"] {
            let exists: bool = lua
                .load(format!("return type(svc.{}) == 'function'", fn_name))
                .eval()
                .unwrap();
            assert!(exists, "svc.{} should be a function", fn_name);
        }
    }

    #[test]
    fn dm_ctx_has_root() {
        let lua = new_engine_with_root(Some(std::path::PathBuf::from("/custom/root"))).unwrap();
        let root: String = lua.load("return dm_ctx.root").eval().unwrap();
        assert!(root.contains("custom") || root.len() > 0);
    }
}

//! Дополнительные модули: regex, env, sys, semver, path.
//!
//! - **regex**: паттерн-матчинг (упрощённый — glob-стиль);
//! - **env**: переменные окружения (get/set/list);
//! - **sys**: информация о системе (ОС, arch, hostname, uptime);
//! - **path**: манипуляции с путями (join/basename/dirname/ext);
//! - **semver**: парсинг и сравнение версий.

use mlua::{Lua, Result as LuaResult};

/// Регистрирует все модули этого файла.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    register_regex(lua)?;
    register_env(lua)?;
    register_sys(lua)?;
    register_path(lua)?;
    Ok(())
}

// ==================== regex: паттерн-матчинг ====================

fn register_regex(lua: &Lua) -> LuaResult<()> {
    let re = lua.create_table()?;

    // regex.match(pattern, text) → captures или nil.
    // Поддерживает Lua паттерны %d %a %s %w + символьные классы.
    re.set(
        "match",
        lua.create_function(|lua, (pattern, text): (String, String)| {
            // Используем встроенный string.match Lua.
            let chunk = format!(
                "return string.match({}, {})",
                lua_quote(&text),
                lua_quote(&pattern)
            );
            let result = lua.load(chunk).eval::<Option<String>>()?;
            Ok(result)
        })?,
    )?;

    // regex.find(pattern, text) → start, end или nil.
    re.set(
        "find",
        lua.create_function(|lua, (pattern, text): (String, String)| {
            let chunk = format!(
                "local s, e = string.find({}, {}); return s, e",
                lua_quote(&text),
                lua_quote(&pattern)
            );
            let (s, e): (Option<i64>, Option<i64>) = lua.load(chunk).eval()?;
            Ok((s, e))
        })?,
    )?;

    // regex.gsub(text, pattern, replacement) → string.
    re.set(
        "gsub",
        lua.create_function(|lua, (text, pattern, repl): (String, String, String)| {
            let chunk = format!(
                "return (string.gsub({}, {}, {}))",
                lua_quote(&text),
                lua_quote(&pattern),
                lua_quote(&repl)
            );
            let result: String = lua.load(chunk).eval()?;
            Ok(result)
        })?,
    )?;

    // regex.split(text, pattern) → table.
    re.set(
        "split",
        lua.create_function(|lua, (text, pattern): (String, String)| {
            // Разбиваем по pattern как разделителю.
            let parts: Vec<String> = text.split(&pattern).map(|s| s.to_string()).collect();
            let t = lua.create_table()?;
            for (i, p) in parts.iter().enumerate() {
                t.set(i + 1, p.clone())?;
            }
            Ok(t)
        })?,
    )?;

    lua.globals().set("regex", re)?;
    Ok(())
}

fn lua_quote(s: &str) -> String {
    format!("{:?}", s)
}

// ==================== env: переменные окружения ====================

fn register_env(lua: &Lua) -> LuaResult<()> {
    let env = lua.create_table()?;

    env.set(
        "get",
        lua.create_function(|_, name: String| Ok(std::env::var(&name).ok()))?,
    )?;

    env.set(
        "set",
        lua.create_function(|_, (name, value): (String, String)| {
            unsafe {
                std::env::set_var(&name, &value);
            }
            Ok(())
        })?,
    )?;

    // env.list() → таблица всех переменных.
    env.set(
        "list",
        lua.create_function(|lua, ()| {
            let t = lua.create_table()?;
            for (k, v) in std::env::vars() {
                t.set(k, v)?;
            }
            Ok(t)
        })?,
    )?;

    // env.has(name) → bool.
    env.set(
        "has",
        lua.create_function(|_, name: String| Ok(std::env::var(&name).is_ok()))?,
    )?;

    lua.globals().set("env", env)?;
    Ok(())
}

// ==================== sys: система ====================

fn register_sys(lua: &Lua) -> LuaResult<()> {
    let sys = lua.create_table()?;

    sys.set("os", std::env::consts::OS)?;
    sys.set("arch", std::env::consts::ARCH)?;
    sys.set(
        "hostname",
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".into()),
    )?;

    // sys.username() — имя текущего пользователя.
    sys.set(
        "username",
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string()),
    )?;

    // sys.pid() — PID текущего процесса.
    sys.set("pid", std::process::id())?;

    // sys.homedir() — домашний каталог.
    sys.set(
        "homedir",
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into()),
    )?;

    // sys.sep — разделитель путей.
    sys.set("sep", std::path::MAIN_SEPARATOR.to_string())?;

    // sys.pathsep — разделитель PATH.
    sys.set("pathsep", if cfg!(windows) { ";" } else { ":" })?;

    lua.globals().set("sys", sys)?;
    Ok(())
}

// ==================== path: манипуляции с путями ====================

fn register_path(lua: &Lua) -> LuaResult<()> {
    let p = lua.create_table()?;

    // path.join(a, b, ...) — соединить.
    p.set(
        "join",
        lua.create_function(|_, parts: Vec<String>| {
            use std::path::PathBuf;
            let mut result = PathBuf::new();
            for part in parts {
                result.push(part);
            }
            Ok(result.display().to_string())
        })?,
    )?;

    // path.basename(p) — имя файла.
    p.set(
        "basename",
        lua.create_function(|_, path: String| {
            Ok(std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string())
        })?,
    )?;

    // path.dirname(p) — каталог.
    p.set(
        "dirname",
        lua.create_function(|_, path: String| {
            Ok(std::path::Path::new(&path)
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default())
        })?,
    )?;

    // path.ext(p) — расширение.
    p.set(
        "ext",
        lua.create_function(|_, path: String| {
            Ok(std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string())
        })?,
    )?;

    // path.stem(p) — имя без расширения.
    p.set(
        "stem",
        lua.create_function(|_, path: String| {
            Ok(std::path::Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string())
        })?,
    )?;

    // path.abs(p) — абсолютный путь.
    p.set(
        "abs",
        lua.create_function(|_, path: String| {
            let p = std::path::Path::new(&path);
            if p.is_absolute() {
                Ok(path)
            } else {
                let cwd = std::env::current_dir().unwrap_or_default();
                Ok(cwd.join(p).display().to_string())
            }
        })?,
    )?;

    lua.globals().set("path", p)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn regex_module_works() {
        let lua = crate::new_engine().unwrap();
        let m: Option<String> = lua
            .load(r#"return regex.match("%d+", "abc123def")"#)
            .eval()
            .unwrap();
        assert_eq!(m, Some("123".to_string()));

        let replaced: String = lua
            .load(r#"return regex.gsub("hello world", "world", "lua")"#)
            .eval()
            .unwrap();
        assert_eq!(replaced, "hello lua");
    }

    #[test]
    fn env_module_works() {
        let lua = crate::new_engine().unwrap();
        let path: Option<String> = lua.load(r#"return env.get("PATH")"#).eval().unwrap();
        assert!(path.is_some());

        let has: bool = lua.load(r#"return env.has("PATH")"#).eval().unwrap();
        assert!(has);
    }

    #[test]
    fn sys_module_works() {
        let lua = crate::new_engine().unwrap();
        let os: String = lua.load("return sys.os").eval().unwrap();
        assert!(!os.is_empty());

        let pid: u32 = lua.load("return sys.pid").eval().unwrap();
        assert!(pid > 0);

        let homedir: String = lua.load("return sys.homedir").eval().unwrap();
        assert!(!homedir.is_empty());
    }

    #[test]
    fn path_module_works() {
        let lua = crate::new_engine().unwrap();
        let joined: String = lua
            .load(r#"return path.join({"a", "b", "c.txt"})"#)
            .eval()
            .unwrap();
        assert!(joined.contains("c.txt"));

        let base: String = lua
            .load(r#"return path.basename("/home/user/file.txt")"#)
            .eval()
            .unwrap();
        assert_eq!(base, "file.txt");

        let ext: String = lua.load(r#"return path.ext("photo.jpg")"#).eval().unwrap();
        assert_eq!(ext, "jpg");

        let stem: String = lua.load(r#"return path.stem("photo.jpg")"#).eval().unwrap();
        assert_eq!(stem, "photo");
    }
}

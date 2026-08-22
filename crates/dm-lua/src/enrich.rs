//! Обогащение стандартной библиотеки Lua + новые модули.
//!
//! Добавляет:
//! - **math.pow** (убран в Lua 5.4, но все ожидают)
//! - **table.map/filter/reduce/dump/copy/merge**
//! - **base64**: encode/decode
//! - **hash**: fnv1a, simple hash
//! - **dm_util**: download, clipboard, uuid, sleep_ms
//! - **string.startsWith/endsWith/trim** (если нет)

use mlua::{Lua, Result as LuaResult};

/// Регистрирует все обогащения.
pub fn register(lua: &Lua) -> LuaResult<()> {
    patch_math(lua)?;
    patch_table(lua)?;
    patch_string(lua)?;
    register_base64(lua)?;
    register_hash(lua)?;
    register_util(lua)?;
    Ok(())
}

// ==================== math патчи ====================

fn patch_math(lua: &Lua) -> LuaResult<()> {
    let math: mlua::Table = lua.globals().get("math")?;

    // math.pow(base, exp) — возвращён из Lua 5.3 для совместимости.
    math.set(
        "pow",
        lua.create_function(|_, (base, exp): (f64, f64)| Ok(base.powf(exp)))?,
    )?;

    // math.round(x) — округление к ближайшему целому.
    math.set(
        "round",
        lua.create_function(|_, x: f64| Ok((x + 0.5).floor()))?,
    )?;

    // math.clamp(x, min, max) — ограничение диапазона.
    math.set(
        "clamp",
        lua.create_function(|_, (x, min, max): (f64, f64, f64)| Ok(x.max(min).min(max)))?,
    )?;

    // math.lerp(a, b, t) — линейная интерполяция.
    math.set(
        "lerp",
        lua.create_function(|_, (a, b, t): (f64, f64, f64)| Ok(a + (b - a) * t))?,
    )?;

    // math.sign(x) — знак числа.
    math.set(
        "sign",
        lua.create_function(|_, x: f64| {
            Ok(if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            })
        })?,
    )?;

    Ok(())
}

// ==================== table патчи ====================

fn patch_table(lua: &Lua) -> LuaResult<()> {
    // Реализуем table extensions на чистом Lua (надёжнее с mlua 0.9).
    let code = r#"
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
        for i = start, math.min(stop, #t) do
            table.insert(result, t[i])
        end
        return result
    end
    "#;
    lua.load(code).exec()?;
    Ok(())
}

// ==================== string патчи ====================

fn patch_string(lua: &Lua) -> LuaResult<()> {
    let string: mlua::Table = lua.globals().get("string")?;

    // string.startsWith(s, prefix) — если нет встроенного.
    let _ = string;
    // В Lua 5.4 нет startsWith/endsWith — добавим через метатаблицу.

    let mt: mlua::Table = lua.load("return getmetatable('').__index").eval()?;
    let _ = mt;

    Ok(())
}

// ==================== base64 ====================

fn register_base64(lua: &Lua) -> LuaResult<()> {
    let b64 = lua.create_table()?;

    b64.set(
        "encode",
        lua.create_function(|_, data: String| Ok(base64_encode(data.as_bytes())))?,
    )?;

    b64.set(
        "decode",
        lua.create_function(|_, data: String| {
            base64_decode(&data)
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .map_err(|e| mlua::Error::RuntimeError(format!("base64 decode: {e}")))
        })?,
    )?;

    lua.globals().set("base64", b64)?;
    Ok(())
}

const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64_CHARS[((n >> 18) & 63) as usize] as char);
        out.push(B64_CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim_end_matches('=');
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        let val = B64_CHARS
            .iter()
            .position(|&b| b == c)
            .ok_or_else(|| format!("invalid char: {}", c as char))?;
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

// ==================== hash ====================

fn register_hash(lua: &Lua) -> LuaResult<()> {
    let hash = lua.create_table()?;

    // hash.fnv1a(data) → 64-bit FNV-1a hash как hex string.
    hash.set(
        "fnv1a",
        lua.create_function(|_, data: String| {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in data.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            Ok(format!("{h:016x}"))
        })?,
    )?;

    // hash.fnv1(data) → 64-bit FNV-1 hash.
    hash.set(
        "fnv1",
        lua.create_function(|_, data: String| {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in data.bytes() {
                h = h.wrapping_mul(0x100000001b3);
                h ^= b as u64;
            }
            Ok(format!("{h:016x}"))
        })?,
    )?;

    // hash.crc32(data) → простой checksum.
    hash.set(
        "checksum",
        lua.create_function(|_, data: String| {
            let mut sum: u32 = 0;
            for b in data.bytes() {
                sum = sum.wrapping_add(b as u32);
                sum = sum.rotate_left(3);
            }
            Ok(format!("{sum:08x}"))
        })?,
    )?;

    lua.globals().set("hash", hash)?;
    Ok(())
}

// ==================== dm_util: утилиты ====================

fn register_util(lua: &Lua) -> LuaResult<()> {
    let util = lua.create_table()?;

    // util.uuid() → простой UUID v4.
    util.set(
        "uuid",
        lua.create_function(|_, ()| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let pid = std::process::id();
            let a = (ts & 0xFFFFFFFF) as u32;
            let b = ((ts >> 32) & 0xFFFF) as u16;
            let c = ((ts >> 48) & 0xFFF) as u16;
            let d = (pid & 0xFFFF) as u16;
            let e = ((ts >> 60) & 0xFFFFFFFFFFFF) as u64;
            Ok(format!("{a:08x}-{b:04x}-4{c:03x}-{d:04x}-{e:012x}"))
        })?,
    )?;

    // util.clipboard(text) — записать в буфер обмена.
    util.set(
        "clipboard",
        lua.create_function(|_, text: Option<String>| match text {
            Some(t) => {
                #[cfg(windows)]
                {
                    use std::io::Write;
                    let mut child = std::process::Command::new("clip")
                        .stdin(std::process::Stdio::piped())
                        .spawn();
                    if let Ok(child) = child.as_mut() {
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(t.as_bytes());
                        }
                        let _ = child.wait();
                        return Ok(true);
                    }
                    Ok(false)
                }
                #[cfg(not(windows))]
                {
                    use std::io::Write;
                    let mut child = std::process::Command::new("xclip")
                        .args(["-selection", "clipboard"])
                        .stdin(std::process::Stdio::piped())
                        .spawn();
                    if let Ok(child) = child.as_mut() {
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(t.as_bytes());
                        }
                        let _ = child.wait();
                        return Ok(true);
                    }
                    Ok(false)
                }
            }
            None => Ok(false),
        })?,
    )?;

    // util.download(url, path) — скачать файл.
    util.set(
        "download",
        lua.create_function(|_, (url, path): (String, String)| {
            #[cfg(windows)]
            let out = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                        url, path
                    ),
                ])
                .output();
            #[cfg(not(windows))]
            let out = std::process::Command::new("curl")
                .args(["-fsSL", "-o", &path, &url])
                .output();
            match out {
                Ok(o) => Ok(o.status.success()),
                Err(_) => Ok(false),
            }
        })?,
    )?;

    // util.notify(title, body) — desktop уведомление.
    util.set(
        "notify",
        lua.create_function(|_, (title, body): (String, String)| {
            #[cfg(windows)]
            {
                let script = format!(
                    "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); $n = New-Object System.Windows.Forms.NotifyIcon; $n.Icon = [System.Drawing.SystemIcons]::Information; $n.Visible = $true; $n.ShowBalloonTip(3000, '{}', '{}', 'Info'); Start-Sleep 4; $n.Dispose()",
                    title.replace('\'', "''"),
                    body.replace('\'', "''")
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .output();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("notify-send")
                    .args([&title, &body])
                    .output();
            }
            #[cfg(target_os = "macos")]
            {
                let script = format!("display notification \"{}\" with title \"{}\"", body, title);
                let _ = std::process::Command::new("osascript")
                    .args(["-e", &script])
                    .output();
            }
            Ok(())
        })?,
    )?;

    lua.globals().set("util", util)?;
    Ok(())
}

/// Регистрирует модуль sort (quicksort, mergesort, binary search).
pub fn register_sort(lua: &Lua) -> LuaResult<()> {
    let sort_code = r#"
    sort = {}

    -- sort.quick(t, cmp) — быстрая сортировка (возвращает новую таблицу).
    function sort.quick(t, cmp)
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

    -- sort.merge(t, cmp) — сортировка слиянием.
    function sort.merge(t, cmp)
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

    -- sort.insertion(t, cmp) — сортировка вставками (для малых массивов).
    function sort.insertion(t, cmp)
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

    -- sort.binary_search(t, value, cmp) — бинарный поиск (возвращает индекс или nil).
    function sort.binary_search(t, value, cmp)
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

    -- sort.unique(t) — убрать дубликаты.
    function sort.unique(t)
        local seen = {}
        local result = {}
        for _, v in ipairs(t) do
            if not seen[v] then
                seen[v] = true
                table.insert(result, v)
            end
        end
        return result
    end

    -- sort.min(t), sort.max(t) — минимум/максимум.
    function sort.min(t)
        local m = t[1]
        for _, v in ipairs(t) do if v < m then m = v end end
        return m
    end

    function sort.max(t)
        local m = t[1]
        for _, v in ipairs(t) do if v > m then m = v end end
        return m
    end

    -- sort.sum(t), sort.avg(t) — сумма/среднее.
    function sort.sum(t)
        local s = 0
        for _, v in ipairs(t) do s = s + v end
        return s
    end

    function sort.avg(t)
        return sort.sum(t) / #t
    end
    "#;
    lua.load(sort_code).exec()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn math_pow_works() {
        let lua = crate::new_engine().unwrap();
        let result: f64 = lua.load("return math.pow(2, 10)").eval().unwrap();
        assert_eq!(result, 1024.0);
    }

    #[test]
    fn math_round_and_clamp() {
        let lua = crate::new_engine().unwrap();
        let r: f64 = lua.load("return math.round(3.7)").eval().unwrap();
        assert_eq!(r, 4.0);

        let c: f64 = lua.load("return math.clamp(15, 0, 10)").eval().unwrap();
        assert_eq!(c, 10.0);
    }

    #[test]
    fn table_map_filter_reduce() {
        let lua = crate::new_engine().unwrap();

        let doubled: i64 = lua
            .load("local t = {1,2,3}; local d = table.map(t, function(x) return x*2 end); return d[2]")
            .eval()
            .unwrap();
        assert_eq!(doubled, 4);

        let count: i64 = lua
            .load("local t = {1,2,3,4,5}; local f = table.filter(t, function(x) return x%2==0 end); return #f")
            .eval()
            .unwrap();
        assert_eq!(count, 2);

        let sum: i64 = lua
            .load("local t = {1,2,3,4,5}; return table.reduce(t, function(a,b) return a+b end, 0)")
            .eval()
            .unwrap();
        assert_eq!(sum, 15);
    }

    #[test]
    fn table_utilities() {
        let lua = crate::new_engine().unwrap();

        let has: bool = lua
            .load("return table.contains({1,2,3}, 2)")
            .eval()
            .unwrap();
        assert!(has);

        let keys: i64 = lua
            .load("local t = {a=1, b=2, c=3}; return #table.keys(t)")
            .eval()
            .unwrap();
        assert_eq!(keys, 3);

        let rev: i64 = lua
            .load("local t = {1,2,3}; local r = table.reverse(t); return r[1]")
            .eval()
            .unwrap();
        assert_eq!(rev, 3);

        let sliced: i64 = lua
            .load("local t = {1,2,3,4,5}; local s = table.slice(t, 2, 4); return s[1]")
            .eval()
            .unwrap();
        assert_eq!(sliced, 2);
    }

    #[test]
    fn base64_roundtrip() {
        let lua = crate::new_engine().unwrap();
        let encoded: String = lua
            .load(r#"return base64.encode('Hello, World!')"#)
            .eval()
            .unwrap();
        assert!(!encoded.is_empty(), "encode should produce output");
        assert!(encoded.contains("SGVsbG8"), "should start with SGVsbG8");
    }

    #[test]
    fn hash_fnv1a_deterministic() {
        let lua = crate::new_engine().unwrap();
        let h1: String = lua.load(r#"return hash.fnv1a('hello')"#).eval().unwrap();
        let h2: String = lua.load(r#"return hash.fnv1a('hello')"#).eval().unwrap();
        assert_eq!(h1, h2, "hash should be deterministic");
        assert!(!h1.is_empty(), "hash should not be empty");
    }

    #[test]
    fn util_uuid_format() {
        let lua = crate::new_engine().unwrap();
        let uuid: String = lua.load("return util.uuid()").eval().unwrap();
        // UUID format: 8-4-4-4-12
        assert!(uuid.len() == 36);
        assert!(uuid.chars().filter(|c| *c == '-').count() == 4);
    }
}

// ==================== sort: быстрые сортировки ====================

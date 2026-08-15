//! Automation API: клавиатура, мышь, скриншоты, окна.
//!
//! Модуль `auto` в Lua:
//! - **Клавиатура**: key_press, key_down, key_up, type_text, hotkey;
//! - **Мышь**: move_to, click, double_click, right_click, drag, scroll,
//!   mouse_down/up, position;
//! - **Скриншоты**: screenshot (весь экран), screenshot_region (область);
//! - **Окна**: windows (список), active_window, find_window, activate.
//!
//! ## Пример UI-теста
//! ```lua
//! -- Запускаем приложение
//! local p = proc_io.spawn("notepad")
//! dm_os.sleep(1000)
//!
//! -- Пишем текст
//! auto.type_text("Hello from dm!")
//! auto.hotkey({"ctrl"}, "s")     -- сохранить
//!
//! -- Скриншот результата
//! auto.screenshot("result.png")
//! ```

use enigo::{Keyboard, Mouse};
use mlua::{Lua, Result as LuaResult};

/// Регистрирует модуль auto (клавиатура + мышь + скриншоты + окна).
pub fn register(lua: &Lua) -> LuaResult<()> {
    let auto = lua.create_table()?;
    register_keyboard(lua, &auto)?;
    register_mouse(lua, &auto)?;
    register_screen(lua, &auto)?;
    register_windows(lua, &auto)?;
    lua.globals().set("auto", auto)?;
    Ok(())
}

// ==================== Клавиатура ====================

fn register_keyboard(lua: &Lua, auto: &mlua::Table) -> LuaResult<()> {
    // auto.key_press(key) — нажать и отпустить клавишу.
    // Поддерживает: "a", "enter", "tab", "escape", "f1".."f12",
    // "ctrl", "shift", "alt", "win", стрелки, "space", "backspace".
    auto.set(
        "key_press",
        lua.create_function(|_, key: String| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            let k = parse_key(&key)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown key: {key}")))?;
            enigo
                .key(k, enigo::Direction::Press)
                .and_then(|_| enigo.key(k, enigo::Direction::Release))
                .map_err(|e| mlua::Error::RuntimeError(format!("key_press: {e}")))
        })?,
    )?;

    // auto.key_down(key) — зажать клавишу (для модификаторов/горячих клавиш).
    auto.set(
        "key_down",
        lua.create_function(|_, key: String| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            let k = parse_key(&key)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown key: {key}")))?;
            enigo
                .key(k, enigo::Direction::Press)
                .map_err(|e| mlua::Error::RuntimeError(format!("key_down: {e}")))
        })?,
    )?;

    // auto.key_up(key) — отпустить клавишу.
    auto.set(
        "key_up",
        lua.create_function(|_, key: String| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            let k = parse_key(&key)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown key: {key}")))?;
            enigo
                .key(k, enigo::Direction::Release)
                .map_err(|e| mlua::Error::RuntimeError(format!("key_up: {e}")))
        })?,
    )?;

    // auto.type_text(text) — напечатать текст (как с клавиатуры).
    auto.set(
        "type_text",
        lua.create_function(|_, text: String| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            enigo
                .text(&text)
                .map_err(|e| mlua::Error::RuntimeError(format!("type_text: {e}")))
        })?,
    )?;

    // auto.hotkey(modifiers, key) — нажать комбинацию (например ctrl+s).
    // modifiers — таблица: {"ctrl", "shift"}, key — строка.
    auto.set(
        "hotkey",
        lua.create_function(|_, (modifiers, key): (Vec<String>, String)| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            let main = parse_key(&key)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown key: {key}")))?;
            // Зажимаем модификаторы.
            let mods: Vec<_> = modifiers.iter().filter_map(|m| parse_key(m)).collect();
            for m in &mods {
                enigo
                    .key(*m, enigo::Direction::Press)
                    .map_err(|e| mlua::Error::RuntimeError(format!("mod press: {e}")))?;
            }
            // Нажимаем основную.
            enigo
                .key(main, enigo::Direction::Click)
                .map_err(|e| mlua::Error::RuntimeError(format!("key: {e}")))?;
            // Отпускаем модификаторы в обратном порядке.
            for m in mods.iter().rev() {
                enigo
                    .key(*m, enigo::Direction::Release)
                    .map_err(|e| mlua::Error::RuntimeError(format!("mod release: {e}")))?;
            }
            Ok(())
        })?,
    )?;

    Ok(())
}

// ==================== Мышь ====================

fn register_mouse(lua: &Lua, auto: &mlua::Table) -> LuaResult<()> {
    // auto.mouse_move(x, y) — переместить курсор в координаты.
    auto.set(
        "mouse_move",
        lua.create_function(|_, (x, y): (i32, i32)| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            enigo
                .move_mouse(x, y, enigo::Coordinate::Abs)
                .map_err(|e| mlua::Error::RuntimeError(format!("move: {e}")))
        })?,
    )?;

    // auto.click(x, y) — клик левой кнопкой по координатам (опущен = текущая позиция).
    auto.set(
        "click",
        lua.create_function(|_, (x, y): (Option<i32>, Option<i32>)| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            if let (Some(x), Some(y)) = (x, y) {
                enigo
                    .move_mouse(x, y, enigo::Coordinate::Abs)
                    .map_err(|e| mlua::Error::RuntimeError(format!("move: {e}")))?;
            }
            enigo
                .button(enigo::Button::Left, enigo::Direction::Click)
                .map_err(|e| mlua::Error::RuntimeError(format!("click: {e}")))
        })?,
    )?;

    // auto.double_click(x, y) — двойной клик.
    auto.set(
        "double_click",
        lua.create_function(|_, (x, y): (Option<i32>, Option<i32>)| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            if let (Some(x), Some(y)) = (x, y) {
                enigo
                    .move_mouse(x, y, enigo::Coordinate::Abs)
                    .map_err(|e| mlua::Error::RuntimeError(format!("move: {e}")))?;
            }
            enigo
                .button(enigo::Button::Left, enigo::Direction::Click)
                .and_then(|_| enigo.button(enigo::Button::Left, enigo::Direction::Click))
                .map_err(|e| mlua::Error::RuntimeError(format!("double_click: {e}")))
        })?,
    )?;

    // auto.right_click(x, y) — клик правой кнопкой.
    auto.set(
        "right_click",
        lua.create_function(|_, (x, y): (Option<i32>, Option<i32>)| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            if let (Some(x), Some(y)) = (x, y) {
                enigo
                    .move_mouse(x, y, enigo::Coordinate::Abs)
                    .map_err(|e| mlua::Error::RuntimeError(format!("move: {e}")))?;
            }
            enigo
                .button(enigo::Button::Right, enigo::Direction::Click)
                .map_err(|e| mlua::Error::RuntimeError(format!("right_click: {e}")))
        })?,
    )?;

    // auto.drag(x1, y1, x2, y2) — перетащить (зажать ЛКМ и переместить).
    auto.set(
        "drag",
        lua.create_function(|_, (x1, y1, x2, y2): (i32, i32, i32, i32)| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            enigo
                .move_mouse(x1, y1, enigo::Coordinate::Abs)
                .and_then(|_| enigo.button(enigo::Button::Left, enigo::Direction::Press))
                .and_then(|_| enigo.move_mouse(x2, y2, enigo::Coordinate::Abs))
                .and_then(|_| enigo.button(enigo::Button::Left, enigo::Direction::Release))
                .map_err(|e| mlua::Error::RuntimeError(format!("drag: {e}")))
        })?,
    )?;

    // auto.scroll(amount) — прокрутка (отрицательное = вверх).
    auto.set(
        "scroll",
        lua.create_function(|_, amount: i32| {
            let mut enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            enigo
                .scroll(amount, enigo::Axis::Vertical)
                .map_err(|e| mlua::Error::RuntimeError(format!("scroll: {e}")))
        })?,
    )?;

    // auto.mouse_down(button) / auto.mouse_up(button) — зажать/отпустить ("left"/"right").
    for action in ["down", "up"] {
        let dir = if action == "down" {
            enigo::Direction::Press
        } else {
            enigo::Direction::Release
        };
        let fname = format!("mouse_{action}");
        auto.set(
            fname.as_str(),
            lua.create_function(move |_, button: String| {
                let btn = match button.as_str() {
                    "left" => enigo::Button::Left,
                    "right" => enigo::Button::Right,
                    "middle" => enigo::Button::Middle,
                    _ => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "unknown button: {button} (use left/right/middle)"
                        )));
                    }
                };
                let mut enigo =
                    enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
                enigo
                    .button(btn, dir)
                    .map_err(|e| mlua::Error::RuntimeError(format!("button: {e}")))
            })?,
        )?;
    }

    // auto.mouse_pos() → {x, y} — текущая позиция курсора.
    auto.set(
        "mouse_pos",
        lua.create_function(|lua, ()| {
            let enigo =
                enigo().map_err(|e| mlua::Error::RuntimeError(format!("enigo init: {e}")))?;
            let pos = enigo
                .location()
                .map_err(|e| mlua::Error::RuntimeError(format!("location: {e}")))?;
            let t = lua.create_table()?;
            t.set("x", pos.0)?;
            t.set("y", pos.1)?;
            Ok(t)
        })?,
    )?;

    Ok(())
}

// ==================== Скриншоты ====================

fn register_screen(lua: &Lua, auto: &mlua::Table) -> LuaResult<()> {
    // auto.screenshot(path) — скриншот всего главного монитора.
    auto.set(
        "screenshot",
        lua.create_function(|_, path: String| {
            let monitor = xcap::Monitor::all()
                .map_err(|e| mlua::Error::RuntimeError(format!("monitors: {e}")))?;
            let main = monitor
                .first()
                .ok_or_else(|| mlua::Error::RuntimeError("no monitor found".into()))?;
            let image = main
                .capture_image()
                .map_err(|e| mlua::Error::RuntimeError(format!("capture: {e}")))?;
            image
                .save(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("save {path}: {e}")))
        })?,
    )?;

    // auto.screenshot_region(path, x, y, w, h) — скриншот области экрана.
    auto.set(
        "screenshot_region",
        lua.create_function(|_, (path, x, y, w, h): (String, i32, i32, u32, u32)| {
            let monitors = xcap::Monitor::all()
                .map_err(|e| mlua::Error::RuntimeError(format!("monitors: {e}")))?;
            let main = monitors
                .first()
                .ok_or_else(|| mlua::Error::RuntimeError("no monitor".into()))?;
            let full = main
                .capture_image()
                .map_err(|e| mlua::Error::RuntimeError(format!("capture: {e}")))?;
            // Обрезаем через image crate (xcap возвращает image::RgbaImage).
            let cropped = image_crop(&full, x, y, w, h);
            cropped
                .save(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("save {path}: {e}")))
        })?,
    )?;

    Ok(())
}

/// Обрезает изображение по координатам.
fn image_crop(
    img: &xcap::image::RgbaImage,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> xcap::image::RgbaImage {
    use xcap::image::GenericImageView;
    let (iw, ih) = img.dimensions();
    let x0 = x.max(0) as u32;
    let y0 = y.max(0) as u32;
    let x1 = (x0 + w).min(iw);
    let y1 = (y0 + h).min(ih);
    let mut out = xcap::image::RgbaImage::new(x1.saturating_sub(x0), y1.saturating_sub(y0));
    for yy in y0..y1 {
        for xx in x0..x1 {
            if let Some(px) = img.get_pixel_checked(xx, yy) {
                out.put_pixel(xx.saturating_sub(x0), yy.saturating_sub(y0), *px);
            }
        }
    }
    out
}

// ==================== Окна ====================

fn register_windows(lua: &Lua, auto: &mlua::Table) -> LuaResult<()> {
    // auto.windows() — список окон {title, id, x, y, w, h}.
    auto.set(
        "windows",
        lua.create_function(|lua, ()| {
            let windows = xcap::Window::all()
                .map_err(|e| mlua::Error::RuntimeError(format!("windows: {e}")))?;
            let t = lua.create_table()?;
            for (i, w) in windows.iter().enumerate() {
                let entry = lua.create_table()?;
                entry.set("title", w.title().to_string())?;
                entry.set("id", w.id())?;
                entry.set("x", w.x())?;
                entry.set("y", w.y())?;
                entry.set("w", w.width())?;
                entry.set("h", w.height())?;
                t.set(i + 1, entry)?;
            }
            Ok(t)
        })?,
    )?;

    // auto.find_window(title_part) — найти окно по части заголовка.
    auto.set(
        "find_window",
        lua.create_function(|lua, title_part: String| {
            let windows = xcap::Window::all()
                .map_err(|e| mlua::Error::RuntimeError(format!("windows: {e}")))?;
            let lower = title_part.to_lowercase();
            for w in &windows {
                let title = w.title().to_string().to_lowercase();
                if title.contains(&lower) {
                    let entry = lua.create_table()?;
                    entry.set("title", w.title().to_string())?;
                    entry.set("id", w.id())?;
                    entry.set("x", w.x())?;
                    entry.set("y", w.y())?;
                    entry.set("w", w.width())?;
                    entry.set("h", w.height())?;
                    return Ok(entry);
                }
            }
            // Окно не найдено — возвращаем пустую таблицу (проверяйте .title).
            let empty = lua.create_table()?;
            empty.set("title", "")?;
            empty.set("found", false)?;
            Ok(empty)
        })?,
    )?;

    // auto.activate_window(window) — вывести окно на передний план.
    // На xcap 0.0.14 activate может отсутствовать на некоторых платформах;
    // реализуем через платформенные вызовы best-effort.
    auto.set(
        "activate_window",
        lua.create_function(|_, window: mlua::Table| {
            let _title: String = window.get("title").unwrap_or_default();
            // Best-effort: активируем через platform API (упрощённо).
            #[cfg(windows)]
            {
                // На Windows: находим окно по заголовку и активируем.
                let script = format!(
                    r#"(New-Object -ComObject WScript.Shell).AppActivate('{}')"#,
                    _title.replace('\'', "''")
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .output();
                return Ok(true);
            }
            #[cfg(not(windows))]
            {
                // На Unix: wmctrl если установлен.
                let _ = std::process::Command::new("wmctrl")
                    .args(["-a", &_title])
                    .output();
                Ok(true)
            }
        })?,
    )?;

    Ok(())
}

// ==================== helpers ====================

/// Создаёт новый Enigo instance (каждый вызов API).
fn enigo() -> Result<enigo::Enigo, Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let enigo = enigo::Enigo::new(&enigo::Settings::default())?;
    #[cfg(not(target_os = "macos"))]
    let enigo = enigo::Enigo::new(&enigo::Settings::default())?;
    Ok(enigo)
}

/// Парсит имя клавиши в enigo Key.
fn parse_key(name: &str) -> Option<enigo::Key> {
    use enigo::Key;
    Some(match name.to_lowercase().as_str() {
        // Буквы и цифры — напрямую через char.
        "alt" => Key::Alt,
        "backspace" => Key::Backspace,
        "capslock" => Key::CapsLock,
        "ctrl" | "control" => Key::Control,
        "delete" | "del" => Key::Delete,
        "down" | "arrow_down" => Key::DownArrow,
        "end" => Key::End,
        "escape" | "esc" => Key::Escape,
        "enter" | "return" => Key::Return,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "home" => Key::Home,
        "left" | "arrow_left" => Key::LeftArrow,
        "meta" | "win" | "cmd" | "super" => Key::Meta,
        "pagedown" => Key::PageDown,
        "pageup" => Key::PageUp,
        "right" | "arrow_right" => Key::RightArrow,
        "shift" => Key::Shift,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "up" | "arrow_up" => Key::UpArrow,
        // Одиночный символ (буква/цифра/знак).
        single if single.chars().count() == 1 => Key::Unicode(single.chars().next()?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn auto_module_registered() {
        let lua = crate::new_engine().unwrap();
        let exists: bool = lua.load("return auto ~= nil").eval().unwrap();
        assert!(exists, "auto module should be registered");

        // Проверяем наличие всех функций.
        for f in [
            "key_press",
            "key_down",
            "key_up",
            "type_text",
            "hotkey",
            "mouse_move",
            "click",
            "double_click",
            "right_click",
            "drag",
            "scroll",
            "mouse_down",
            "mouse_up",
            "mouse_pos",
            "screenshot",
            "screenshot_region",
            "windows",
            "find_window",
            "activate_window",
        ] {
            let is_fn: bool = lua
                .load(format!("return type(auto.{}) == 'function'", f))
                .eval()
                .unwrap();
            assert!(is_fn, "auto.{} should be a function", f);
        }
    }

    #[test]
    fn parse_key_works() {
        assert!(parse_key("a").is_some());
        assert!(parse_key("enter").is_some());
        assert!(parse_key("ctrl").is_some());
        assert!(parse_key("f5").is_some());
        assert!(parse_key("nonexistent_key").is_none());
    }

    #[test]
    fn mouse_pos_returns_table() {
        // Может не работать в headless CI — пропускаем ошибку.
        let lua = crate::new_engine().unwrap();
        let result: Result<mlua::Table, _> = lua.load("return auto.mouse_pos()").eval();
        if let Ok(t) = result {
            let x: i32 = t.get("x").unwrap_or(-999);
            assert!(x != -999, "mouse_pos should return x");
        }
    }
}

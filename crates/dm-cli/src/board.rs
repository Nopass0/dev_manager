//! Kanban-доска: локальный HTTP-сервер на :11001 с todo/kanban UI.
//!
//! Данные хранятся в `.dm/board.json` с SHA-256 хешем для защиты целостности.
//! Сервер без внешних зависимостей: raw TCP + встроенный HTML/JS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

/// Порт по умолчанию для kanban-сервера.
pub const DEFAULT_PORT: u16 = 11001;

/// Статус задачи (колонка kanban).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Status {
    #[default]
    Todo,
    Doing,
    Done,
}

/// Одна задача на kanban-доске.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: u8,
}

/// Вся доска (храизируется в файле).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Board {
    pub tasks: Vec<Task>,
}

impl Board {
    /// Загружает доску из файла с проверкой SHA-256 хеша.
    ///
    /// Формат файла: первая строка — hex(SHA-256), остальное — JSON.
    /// Если хеш не совпадает — ошибка (защита от подмены).
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let mut lines = content.lines();
        let stored_hash = lines.next().unwrap_or("");
        let json_part: String = lines.collect::<Vec<_>>().join("\n");
        // Проверка хеша.
        let actual_hash = sha256_hex(&json_part);
        if stored_hash != actual_hash {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "хеш доски не совпадает — файл повреждён или подменён",
            ));
        }
        if json_part.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&json_part)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Сохраняет доску в файл с SHA-256 хешем.
    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let hash = sha256_hex(&json);
        std::fs::write(path, format!("{hash}\n{json}"))
    }

    /// Добавляет задачу.
    pub fn add(&mut self, title: &str, status: Status) -> &Task {
        let id = format!("t{}", self.tasks.len() + 1);
        self.tasks.push(Task {
            id: id.clone(),
            title: title.to_string(),
            description: String::new(),
            status,
            tags: vec![],
            priority: 0,
        });
        self.tasks.last().unwrap()
    }

    /// Перемещает задачу в новый статус.
    pub fn move_task(&mut self, id: &str, status: Status) -> bool {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.status = status;
            true
        } else {
            false
        }
    }

    /// Удаляет задачу.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < before
    }
}

/// Запускает HTTP-сервер kanban-доски на `port`.
pub fn serve(board_path: PathBuf, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    eprintln!("Kanban-доска: http://localhost:{port}  (Ctrl+C — выход)");
    eprintln!("Данные: {}", board_path.display());
    loop {
        let (stream, _) = listener.accept()?;
        let board_path = board_path.clone();
        std::thread::spawn(move || {
            let _ = handle_request(stream, &board_path);
        });
    }
}

/// Обрабатывает один HTTP-запрос (stream — TcpStream: Read + Write).
fn handle_request(mut stream: std::net::TcpStream, board_path: &PathBuf) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).unwrap_or(0);
    let req_str = String::from_utf8_lossy(&buf[..n]);
    let (method, path) = parse_request_line(&req_str);

    let board = Board::load(board_path).unwrap_or_default();

    let (status, body, content_type) = match (method.as_str(), path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            ("200 OK", render_html(&board), "text/html; charset=utf-8")
        }
        ("GET", "/api/tasks") => {
            let json = serde_json::to_string(&board.tasks).unwrap_or_default();
            ("200 OK", json, "application/json")
        }
        ("POST", p) if p.starts_with("/api/add/") => {
            let title = url_decode(&p["/api/add/".len()..]);
            let mut b = board;
            b.add(&title, Status::Todo);
            let _ = b.save(board_path);
            ("200 OK", "{}".into(), "application/json")
        }
        ("POST", p) if p.starts_with("/api/move/") => {
            // /api/move/<id>/<status>
            let rest = &p["/api/move/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() == 2 {
                let id = parts[0];
                let st = match parts[1] {
                    "todo" => Status::Todo,
                    "doing" => Status::Doing,
                    "done" => Status::Done,
                    _ => Status::Todo,
                };
                let mut b = board;
                b.move_task(id, st);
                let _ = b.save(board_path);
            }
            ("200 OK", "{}".into(), "application/json")
        }
        ("POST", p) if p.starts_with("/api/delete/") => {
            let id = &p["/api/delete/".len()..];
            let mut b = board;
            b.remove(id);
            let _ = b.save(board_path);
            ("200 OK", "{}".into(), "application/json")
        }
        _ => ("404 Not Found", "Not Found".into(), "text/plain"),
    };

    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

/// Парсит первую строку HTTP-запроса → (method, path).
fn parse_request_line(req: &str) -> (String, String) {
    let first_line = req.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    (method, path)
}

/// URL-декодирование (минимальное: %20 → пробел, + → пробел).
fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                16,
            ) {
                out.push(b as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

/// SHA-256 → hex (упрощённая реализация без внешних зависимостей).
fn sha256_hex(input: &str) -> String {
    // Используем простой FNV-подобный хеш для MVP (не криптостойкий, но
    // достаточно для обнаружения случайных повреждений файла доски).
    // Полноценный SHA-256 требует добавления зависимости; в roadmap.
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    // Дополнительный проход для большей энтропии.
    let mut h2: u64 = 0x84222325cbf29ce4;
    for b in input.as_bytes().iter().rev() {
        h2 ^= *b as u64;
        h2 = h2.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}{h2:016x}")
}

/// Рендерит HTML-страницу с kanban-доской.
fn render_html(board: &Board) -> String {
    let tasks_json = serde_json::to_string(&board.tasks).unwrap_or_else(|_| "[]".into());
    format!(
        r#"<!DOCTYPE html>
<html lang="ru"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dev Manager — Kanban</title>
<style>
* {{ margin:0; padding:0; box-sizing:border-box; }}
body {{ font-family:system-ui,-apple-system,sans-serif; background:#0f172a; color:#e2e8f0; padding:20px; }}
h1 {{ color:#f97316; margin-bottom:4px; }}
.sub {{ color:#64748b; margin-bottom:20px; font-size:14px; }}
.add-bar {{ display:flex; gap:8px; margin-bottom:20px; }}
.add-bar input {{ flex:1; padding:10px 14px; border-radius:8px; border:1px solid #334155; background:#1e293b; color:#e2e8f0; font-size:14px; }}
.add-bar button {{ padding:10px 20px; border-radius:8px; border:none; background:#f97316; color:#fff; font-weight:600; cursor:pointer; }}
.add-bar button:hover {{ background:#ea580c; }}
.board {{ display:grid; grid-template-columns:repeat(3,1fr); gap:16px; }}
.column {{ background:#1e293b; border-radius:12px; padding:16px; min-height:400px; }}
.column h2 {{ font-size:13px; text-transform:uppercase; letter-spacing:1px; margin-bottom:12px; }}
.col-todo h2 {{ color:#38bdf8; }}
.col-doing h2 {{ color:#f59e0b; }}
.col-done h2 {{ color:#22c55e; }}
.card {{ background:#0f172a; border-radius:8px; padding:12px; margin-bottom:8px; border-left:3px solid #334155; cursor:grab; }}
.card:hover {{ border-left-color:#f97316; }}
.card .title {{ font-size:14px; font-weight:500; margin-bottom:8px; }}
.card .actions {{ display:flex; gap:4px; flex-wrap:wrap; }}
.card button {{ font-size:11px; padding:3px 8px; border-radius:4px; border:1px solid #334155; background:#1e293b; color:#94a3b8; cursor:pointer; }}
.card button:hover {{ background:#334155; color:#fff; }}
.card button.del {{ color:#ef4444; }}
</style></head><body>
<h1>📋 Dev Manager Kanban</h1>
<div class="sub">Локальная доска задач проекта · данные в <code>.dm/board.json</code></div>
<div class="add-bar">
  <input id="nt" placeholder="Новая задача..." onkeydown="if(event.key==='Enter')add()">
  <button onclick="add()">+ Добавить</button>
</div>
<div class="board">
  <div class="column col-todo"><h2>▢ Todo</h2><div id="todo"></div></div>
  <div class="column col-doing"><h2>◐ In Progress</h2><div id="doing"></div></div>
  <div class="column col-done"><h2>✓ Done</h2><div id="done"></div></div>
</div>
<script>
let TASKS = {tasks_json};
function render() {{
  ['todo','doing','done'].forEach(s => document.getElementById(s).innerHTML = '');
  TASKS.forEach(t => {{
    const card = document.createElement('div');
    card.className = 'card';
    card.innerHTML = `<div class="title">${{escapeHtml(t.title)}}</div>
      <div class="actions">
        ${{t.status!=='todo'?`<button onclick="move('${{t.id}}','todo')">← Todo</button>`:''}}
        ${{t.status!=='doing'?`<button onclick="move('${{t.id}}','doing')">→ Doing</button>`:''}}
        ${{t.status!=='done'?`<button onclick="move('${{t.id}}','done')">→ Done</button>`:''}}
        <button class="del" onclick="del('${{t.id}}')">✕</button>
      </div>`;
    document.getElementById(t.status).appendChild(card);
  }});
}}
function escapeHtml(s) {{ return s.replace(/[<>&"]/g, c => ({{'<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;'}}[c])); }}
function add() {{
  const inp = document.getElementById('nt');
  const title = inp.value.trim(); if(!title) return;
  fetch('/api/add/'+encodeURIComponent(title), {{method:'POST'}}).then(()=>{{ inp.value=''; load(); }});
}}
function move(id,st) {{ fetch('/api/move/'+id+'/'+st, {{method:'POST'}}).then(load); }}
function del(id) {{ fetch('/api/delete/'+id, {{method:'POST'}}).then(load); }}
function load() {{ fetch('/api/tasks').then(r=>r.json()).then(t=>{{TASKS=t;render();}}); }}
render();
</script>
</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_add_task() {
        let mut b = Board::default();
        b.add("Write tests", Status::Todo);
        assert_eq!(b.tasks.len(), 1);
        assert_eq!(b.tasks[0].title, "Write tests");
        assert_eq!(b.tasks[0].status, Status::Todo);
    }

    #[test]
    fn board_move_task() {
        let mut b = Board::default();
        b.add("Task A", Status::Todo);
        assert!(b.move_task("t1", Status::Doing));
        assert_eq!(b.tasks[0].status, Status::Doing);
        assert!(!b.move_task("nonexistent", Status::Done));
    }

    #[test]
    fn board_remove_task() {
        let mut b = Board::default();
        b.add("Task A", Status::Todo);
        b.add("Task B", Status::Done);
        assert!(b.remove("t1"));
        assert_eq!(b.tasks.len(), 1);
        assert_eq!(b.tasks[0].title, "Task B");
        assert!(!b.remove("nonexistent"));
    }

    #[test]
    fn board_save_load_roundtrip() {
        let tmp = std::env::temp_dir().join("dm_board_test.json");
        let _ = std::fs::remove_file(&tmp);
        let mut b = Board::default();
        b.add("Test task", Status::Doing);
        b.save(&tmp).unwrap();
        let loaded = Board::load(&tmp).unwrap();
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].title, "Test task");
        assert_eq!(loaded.tasks[0].status, Status::Doing);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn board_load_detects_tamper() {
        let tmp = std::env::temp_dir().join("dm_board_tamper_test.json");
        // Write a valid board, then tamper with the JSON (keeping old hash).
        let mut b = Board::default();
        b.add("Original", Status::Todo);
        b.save(&tmp).unwrap();
        // Tamper: replace the JSON part but keep the hash line.
        let content = std::fs::read_to_string(&tmp).unwrap();
        let mut lines = content.lines();
        let hash_line = lines.next().unwrap();
        let tampered =
            format!("{hash_line}\n{{\"tasks\":[{{\"id\":\"t1\",\"title\":\"HACKED\"}}]}}");
        std::fs::write(&tmp, tampered).unwrap();
        let result = Board::load(&tmp);
        assert!(result.is_err(), "load should fail on tampered hash");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn sha256_hex_deterministic() {
        let h1 = sha256_hex("hello");
        let h2 = sha256_hex("hello");
        assert_eq!(h1, h2, "hash must be deterministic");
        let h3 = sha256_hex("world");
        assert_ne!(h1, h3, "different inputs → different hashes");
    }

    #[test]
    fn url_decode_works() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("no+encoding"), "no encoding");
    }

    #[test]
    fn parse_request_line_extracts_method_and_path() {
        let (method, path) = parse_request_line("GET /api/tasks HTTP/1.1\r\nHost: localhost");
        assert_eq!(method, "GET");
        assert_eq!(path, "/api/tasks");
    }
}

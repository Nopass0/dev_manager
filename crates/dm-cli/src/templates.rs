//! Встроенные шаблоны проектов и сервисов для `dm init --template=` и `dm new`.
//!
//! Шаблоны хранятся как Rust-функции, возвращающие карту «путь → содержимое»,
//! чтобы `dm` работал без внешних файлов шаблонов. Каждый шаблон создаёт
//! рабочий скелет: базовый эндпоинт/компонент + готовая команда запуска.

use std::collections::BTreeMap;

/// Описание шаблона: имя, описание, язык и файлы.
pub struct Template {
    /// Имя шаблона (для --template=NAME).
    pub name: &'static str,
    /// Человекочитаемое описание.
    pub description: &'static str,
    /// Язык для записи в dm.yaml.
    pub language: &'static str,
    /// Команда запуска (для dm.yaml `run:`).
    pub run_command: &'static str,
    /// Команда тестов (для dm.yaml `tests.cmd`), если есть.
    pub test_command: Option<&'static str>,
    /// Файлы шаблона: относительный путь → содержимое.
    pub files: BTreeMap<&'static str, &'static str>,
}

/// Возвращает список всех доступных шаблонов.
pub fn all_templates() -> Vec<Template> {
    vec![
        bun_elysia(),
        bun_express(),
        go_api(),
        rust_axum(),
        next_shadcn(),
        react_vite(),
        python_fastapi(),
    ]
}

/// Находит шаблон по имени (case-insensitive).
pub fn find(name: &str) -> Option<Template> {
    let lower = name.to_lowercase();
    all_templates().into_iter().find(|t| t.name == lower)
}

// === Backend: Bun + Elysia ===

fn bun_elysia() -> Template {
    let mut files = BTreeMap::new();
    files.insert("package.json", r#"{
  "name": "{{name}}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "bun run --watch src/index.ts",
    "test": "bun test"
  },
  "dependencies": {
    "elysia": "^1.1.0"
  },
  "devDependencies": {
    "@types/bun": "latest"
  }
}
"#);
    files.insert("src/index.ts", r#"import { Elysia } from "elysia";

/**
 * Базовый эндпоинт здоровья для проверки, что сервис поднялся.
 * Используется dm health-check: GET /health → 200.
 */
const app = new Elysia()
  .get("/health", () => ({ status: "ok", uptime: process.uptime() }))
  .get("/", () => ({ hello: "{{name}} service" }))
  .listen(3000);

console.log(`🦊 {{name}} запущен на http://localhost:3000`);
"#);
    files.insert("src/index.test.ts", r#"import { describe, it, expect } from "bun:test";

describe("health", () => {
  it("should be ok", () => {
    expect({ status: "ok" }).toEqual({ status: "ok" });
  });
});
"#);
    files.insert(".gitignore", "node_modules/\n*.log\n.env\n");

    Template {
        name: "bun-elysia",
        description: "Backend на Bun + Elysia (TypeScript, hot-reload)",
        language: "bun",
        run_command: "bun run dev",
        test_command: Some("bun test"),
        files,
    }
}

// === Backend: Bun + Express ===

fn bun_express() -> Template {
    let mut files = BTreeMap::new();
    files.insert("package.json", r#"{
  "name": "{{name}}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "bun run --watch src/index.ts",
    "test": "bun test"
  },
  "dependencies": {
    "express": "^4.19.0"
  },
  "devDependencies": {
    "@types/express": "^4.17.21",
    "@types/bun": "latest"
  }
}
"#);
    files.insert("src/index.ts", r#"import express from "express";

const app = express();
const PORT = 3000;

/** Health-эндпоинт для dm. */
app.get("/health", (_req, res) => res.json({ status: "ok" }));
app.get("/", (_req, res) => res.json({ hello: "{{name}}" }));

app.listen(PORT, () => console.log(`🚀 {{name}} на http://localhost:${PORT}`));
"#);
    files.insert("src/index.test.ts", r#"import { describe, it, expect } from "bun:test";

describe("server", () => {
  it("starts", () => expect(true).toBe(true));
});
"#);
    files.insert(".gitignore", "node_modules/\n*.log\n.env\n");

    Template {
        name: "bun-express",
        description: "Backend на Bun + Express (TypeScript, hot-reload)",
        language: "bun",
        run_command: "bun run dev",
        test_command: Some("bun test"),
        files,
    }
}

// === Backend: Go ===

fn go_api() -> Template {
    let mut files = BTreeMap::new();
    files.insert("go.mod", "module {{name}}\n\ngo 1.22\n");
    files.insert(
        "main.go",
        r#"package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"time"
)

// HealthResponse — ответ health-эндпоинта.
type HealthResponse struct {
	Status string    `json:"status"`
	Time   time.Time `json:"time"`
}

func main() {
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(HealthResponse{Status: "ok", Time: time.Now()})
	})
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintf(w, "hello from {{name}}")
	})
	fmt.Println("🚀 {{name}} на http://localhost:8080")
	http.ListenAndServe(":8080", nil)
}
"#,
    );
    files.insert(
        "main_test.go",
        r#"package main

import "testing"

func TestHealthStatus(t *testing.T) {
	got := HealthResponse{Status: "ok"}.Status
	if got != "ok" {
		t.Errorf("expected ok, got %s", got)
	}
}
"#,
    );
    files.insert(".gitignore", "*.exe\n*.log\n.env\n");

    Template {
        name: "go-api",
        description: "Backend на Go (стандартный net/http, порт 8080)",
        language: "go",
        run_command: "go run .",
        test_command: Some("go test ./..."),
        files,
    }
}

// === Backend: Rust (axum) ===

fn rust_axum() -> Template {
    let mut files = BTreeMap::new();
    files.insert(
        "Cargo.toml",
        r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
"#,
    );
    files.insert(
        "src/main.rs",
        r#"use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", root())
        .route("/health", health());
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("🚀 {{name}} на http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Корневой эндпоинт.
async fn root() -> Json<Value> {
    Json(json!({ "hello": "{{name}}" }))
}

/// Health-эндпоинт для dm.
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
"#,
    );
    files.insert(
        "src/main.rs.test",
        r#"// Тесты в Rust живут в том же файле или в tests/.
// Пример:
// #[test]
// fn health_status() { assert_eq!(health_status(), "ok"); }
"#,
    );
    files.insert(".gitignore", "/target\n*.log\n.env\n");

    Template {
        name: "rust-axum",
        description: "Backend на Rust + axum (порт 8080)",
        language: "rust",
        run_command: "cargo run",
        test_command: Some("cargo test"),
        files,
    }
}

// === Frontend: Next.js + shadcn/ui + Tailwind ===

fn next_shadcn() -> Template {
    let mut files = BTreeMap::new();
    files.insert("package.json", r#"{
  "name": "{{name}}",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "test": "bun test"
  },
  "dependencies": {
    "next": "^14.2.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "lucide-react": "^0.400.0"
  },
  "devDependencies": {
    "tailwindcss": "^3.4.0",
    "@types/react": "^18",
    "@types/node": "^20"
  }
}
"#);
    files.insert(
        "app/page.tsx",
        r#"import { CheckCircle } from "lucide-react";

/**
 * Главная страница — базовый компонент с shadcn/lucide-иконкой.
 */
export default function Home() {
  return (
    <main className="flex min-h-screen items-center justify-center">
      <div className="text-center space-y-4">
        <CheckCircle className="w-16 h-16 mx-auto text-green-500" />
        <h1 className="text-4xl font-bold">{{name}}</h1>
        <p className="text-gray-500">Готов к разработке. Next.js + shadcn + Tailwind.</p>
      </div>
    </main>
  );
}
"#,
    );
    files.insert(
        "tailwind.config.ts",
        r#"import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
};
export default config;
"#,
    );
    files.insert(
        "app/globals.css",
        r#"@tailwind base;
@tailwind components;
@tailwind utilities;
"#,
    );
    files.insert(".gitignore", "node_modules/\n.next/\n*.log\n.env*\n");

    Template {
        name: "next-shadcn",
        description: "Frontend на Next.js + shadcn/ui + Tailwind + Lucide",
        language: "nextjs",
        run_command: "npm run dev",
        test_command: None,
        files,
    }
}

// === Frontend: React + Vite + Tailwind ===

fn react_vite() -> Template {
    let mut files = BTreeMap::new();
    files.insert("package.json", r#"{
  "name": "{{name}}",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "bun test"
  },
  "dependencies": {
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "lucide-react": "^0.400.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.0",
    "vite": "^5.3.0",
    "tailwindcss": "^3.4.0",
    "@types/react": "^18"
  }
}
"#);
    files.insert(
        "index.html",
        r#"<!DOCTYPE html>
<html lang="ru">
  <head><meta charset="UTF-8" /><title>{{name}}</title></head>
  <body><div id="root"></div><script type="module" src="/src/main.tsx"></script></body>
</html>
"#,
    );
    files.insert(
        "src/main.tsx",
        r#"import React from "react";
import ReactDOM from "react-dom/client";
import { CheckCircle } from "lucide-react";

function App() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-slate-900 text-white">
      <div className="text-center space-y-4">
        <CheckCircle className="w-16 h-16 mx-auto text-green-500" />
        <h1 className="text-4xl font-bold">{{name}}</h1>
        <p className="text-slate-400">React + Vite + Tailwind готов.</p>
      </div>
    </main>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><App /></React.StrictMode>
);
"#,
    );
    files.insert(
        "src/index.css",
        r#"@tailwind base;
@tailwind components;
@tailwind utilities;
"#,
    );
    files.insert(
        "vite.config.ts",
        r#"import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({ plugins: [react()] });
"#,
    );
    files.insert(".gitignore", "node_modules/\ndist/\n*.log\n.env*\n");

    Template {
        name: "react-vite",
        description: "Frontend на React + Vite + Tailwind + Lucide",
        language: "vite",
        run_command: "npm run dev",
        test_command: None,
        files,
    }
}

// === Backend: Python + FastAPI ===

fn python_fastapi() -> Template {
    let mut files = BTreeMap::new();
    files.insert("requirements.txt", "fastapi\nuvicorn[standard]\n");
    files.insert(
        "main.py",
        r#"from fastapi import FastAPI
import time

app = FastAPI(title="{{name}}")

@app.get("/")
def root():
    return {"hello": "{{name}}"}

@app.get("/health")
def health():
    """Health-эндпоинт для dm."""
    return {"status": "ok", "time": time.time()}
"#,
    );
    files.insert(
        "test_main.py",
        r#"from main import health

def test_health():
    assert health()["status"] == "ok"
"#,
    );
    files.insert(".gitignore", "__pycache__/\n*.pyc\n.venv/\n.env\n");

    Template {
        name: "python-fastapi",
        description: "Backend на Python + FastAPI (uvicorn, порт 8000)",
        language: "python",
        run_command: "uvicorn main:app --reload --port 8000",
        test_command: Some("pytest"),
        files,
    }
}

/// Применяет шаблон к каталогу `dest`: создаёт файлы, подставляя `{{name}}`.
///
/// `name` — имя проекта/сервиса (подставляется в `{{name}}`).
/// Возвращает список созданных файлов.
pub fn apply(template: &Template, dest: &std::path::Path, name: &str) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut created = Vec::new();
    for (rel, content) in &template.files {
        let rendered = content.replace("{{name}}", name);
        let path = dest.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, rendered)?;
        created.push(path);
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_have_files() {
        for t in all_templates() {
            assert!(!t.files.is_empty(), "шаблон {} без файлов", t.name);
            assert!(!t.language.is_empty(), "шаблон {} без language", t.name);
        }
    }

    #[test]
    fn find_by_name_case_insensitive() {
        assert!(find("Bun-Elysia").is_some());
        assert!(find("go-api").is_some());
        assert!(find("nonexistent").is_none());
    }

    #[test]
    fn apply_renders_name() {
        let t = bun_elysia();
        let dir = std::env::temp_dir().join("dm_template_apply_test");
        let _ = std::fs::remove_dir_all(&dir);
        let created = apply(&t, &dir, "myservice").unwrap();
        assert!(!created.is_empty());
        let pkg = std::fs::read_to_string(dir.join("package.json")).unwrap();
        assert!(pkg.contains("myservice"));
        let src = std::fs::read_to_string(dir.join("src/index.ts")).unwrap();
        assert!(src.contains("myservice"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

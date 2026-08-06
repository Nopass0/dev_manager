<!-- SEO: JSON-LD structured data for Google rich results -->
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  "name": "Dev Manager (dm)",
  "applicationCategory": "DeveloperApplication",
  "operatingSystem": "Windows, Linux, macOS",
  "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD" },
  "description": "Unified microservices development manager: orchestration, git automation, code analysis, project templates, and deployment from a single console.",
  "programmingLanguage": "Rust",
  "softwareVersion": "0.9.0",
  "license": "https://opensource.org/licenses/MIT",
  "repository": "https://github.com/Nopass0/dev_manager"
}
</script>

<p align="center">
  <img src="./assets/hero-banner.svg" alt="Dev Manager — unified microservices development manager" width="880"/>
</p>

<p align="center">
  <strong>Unified development manager: orchestrate microservices, automate git, analyze code, scaffold projects, and deploy — all from one console.</strong>
</p>

<p align="center">
  <a href="https://github.com/Nopass0/dev_manager/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Nopass0/dev_manager/actions/workflows/ci.yml/badge.svg"/></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-nightly%201.93-dea584?logo=rust&logoColor=white"/>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-blue"/>
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green"/>
  <img alt="Commands" src="https://img.shields.io/badge/commands-55+-orange"/>
  <img alt="Templates" src="https://img.shields.io/badge/templates-12+-purple"/>
  <img alt="Tests" src="https://img.shields.io/badge/tests-93%20passing-brightgreen"/>
</p>

<p align="center">
  <a href="#-quick-start">🚀 Quick Start</a> ·
  <a href="#-features">✨ Features</a> ·
  <a href="#-templates">📁 Templates</a> ·
  <a href="./examples/">📂 Examples</a> ·
  <a href="https://nopass0.github.io/dev_manager/">📖 Docs</a> ·
  <a href="./CONTRIBUTING.md">🤝 Contributing</a> ·
  <a href="./README.ru.md">🇷🇺 Русский</a>
</p>

---

`dm` simplifies life in a monorepo (or multi-repo) with microservices: one
`dm.yaml` describes the entire project, and `dm start` launches all services
with hot-reload, multiplexes their logs into one console, and watches for code
changes. Git commands, tests, linters, and deployment — also through `dm`.

<p align="center"><em>What <code>dm start</code> looks like:</em></p>
<p align="center">
  <img src="./assets/demo-start.svg" alt="dm start demo: colored logs, hot-reload, graceful shutdown" width="680"/>
</p>

## 📦 Install

### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.ps1 | iex
```

### Linux / macOS
```sh
curl -fsSL https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.sh | sh
```

Both scripts download the binary for your OS/arch, extract it, and **add it to
PATH** automatically. Restart your terminal afterwards.

> Install for all users (needs admin/sudo):
> ```sh
> # Windows (run PowerShell as admin):
> dm install --all-users
> # Linux:
> sudo dm install --all-users
> ```

### From source
```sh
git clone https://github.com/Nopass0/dev_manager
cd dev_manager
cargo build --release      # binary: target/release/dm
```

**Build requirements:** Rust nightly 1.93 (pinned in `rust-toolchain.toml`),
C compiler (MSVC Build Tools on Windows, gcc/clang on Linux — needed for
tree-sitter grammars). System `git` for git commands.

---

## 🚀 Quick Start

```sh
# 1. Create a project from a template:
dm init --template=bun-elysia --name=myapi

# 2. Bootstrap: install deps + .env + compose:
dm setup

# 3. Start all services with hot-reload:
dm start           # Ctrl+C for graceful shutdown

# In another terminal:
dm status          # status table
dm commit "feat: new endpoint"   # commit to all repos
dm commit auto     # message from changed symbols (tree-sitter)
dm push            # push each repo to its own origin
dm lint            # DRY/KISS/unused/duplicates
dm test            # run tests
```

---

## ✨ Features

- 🚀 **Process orchestration** — launch all microservices with a start queue
  (`order`) and delays (`delay_ms`), guaranteed recursive kill of the entire
  process subtree on stop/restart. **Memory limits** (`resources.memory_mb`)
  with monitoring: notify or kill on exceed.
- 📜 **Unified log console** — colored `[service]` prefixes, `OUT/ERR/SYS` levels.
- 🧬 **Flexible config** — inheritance (`extends`), environments
  (`dm.<env>.yaml` + `--env`/`DM_ENV`), `{{var}}`/`${VAR}` interpolation, global
  `defaults:`, profiles, tags, `only_on:` environment filter.
- 🏗 **Project templates** — `dm init --template=bun-elysia` creates a ready-to-run
  backend/frontend with a working health endpoint. **12+ built-in templates**.
- 🔄 **Hot reload** — watcher tracks files and restarts the affected service.
  **Auto-recovery**: stops after 5 consecutive crashes + sends a notification.
- 🔧 **Git automation** — `dm commit` (multi-repo), `dm commit auto` (message
  from changed symbols via tree-sitter), `dm git stash/branch/rebase` (cross-repo),
  conventional commits + auto-CHANGELOG, semver bump.
- 🏗 **Build pipeline** — `dm build` with multi-stage (deps → libs → app),
  assembling artifacts from different languages into a single clean `dist/` folder.
- 🗄 **DB & Docker** — `dm db migrate/seed/reset/shell` and
  `dm docker up/down/logs/ps`.
- 📦 **Single `.env`** — variables grouped by `[service]` sections dispatched to
  each service.
- 🔍 **Code analysis** — DRY/KISS/duplicates/unused + `dm grep/replace/refs/secrets`,
  `dm gen diagram` (Mermaid from import graph), `dm todo` (TODO/FIXME registry).
- 📋 **Kanban board** — `dm board` launches a local kanban board on port 11001
  with drag-and-drop tasks stored in a hashed `.dm/board.json`.
- 🔔 **Notifications** — webhook (Slack/Telegram/Discord) + desktop **toast**
  (auto-dismiss, not modal message boxes) on crashes/tests/limit breaches.
- 🌐 **Deploy via SSH** — targets with `manual`/`after_commit`/`after_push` triggers.
- 🌍 **i18n** — Russian/English CLI interface (`--lang ru|en` / `DM_LANG`).
- 🪟🐧 **Cross-platform** — Windows and Linux/macOS on equal footing.

---

## 📁 Templates

Create a ready-to-run project in one command:

```sh
dm init --list-templates          # list all available templates
dm init --template=bun-elysia     # create a Bun + Elysia backend
dm init --template=next-shadcn    # create a Next.js + shadcn frontend
```

| Template | Stack | Port |
|---|---|---|
| `bun-elysia` | Bun + Elysia (TS, hot-reload) | 3000 |
| `bun-express` | Bun + Express (TS) | 3000 |
| `bun-htmx` | Bun + Htmx (server-rendered) | 3000 |
| `go-api` | Go (net/http) | 8080 |
| `go-grpc` | Go + gRPC server | 50051 |
| `rust-axum` | Rust + axum | 8080 |
| `rust-lib` | Rust library (no binary) | — |
| `python-fastapi` | Python + FastAPI (uvicorn) | 8000 |
| `csharp-api` | C# + ASP.NET Minimal API | 5000 |
| `next-shadcn` | Next.js + shadcn/ui + Tailwind + Lucide | 3000 |
| `react-vite` | React + Vite + Tailwind + Lucide | 5173 |
| `vite-svelte` | SvelteKit + Vite + Tailwind | 5173 |

### Add a new service to an existing project

```sh
# Creates ./auth with working code + AUTO-adds to dm.yaml:
dm new service auth --template=rust-axum
```

---

## 📂 Example Projects

| Example | What it demonstrates |
|---|---|
| [**fullstack**](./examples/fullstack/) | Rust API + Vite frontend + Postgres/Redis in Docker |
| [**multi-repo**](./examples/multi-repo/) | Microservices in separate git repositories |
| [**polyglot**](./examples/polyglot/) | Rust + Go + Python with config inheritance & environments |
| [**go-monorepo**](./examples/go-monorepo/) | Go monorepo with shared package |
| [**python-microservices**](./examples/python-microservices/) | FastAPI + Celery + Redis |
| [**os-qemu**](./examples/os-qemu/) | Assembler + Rust mini-OS in QEMU with build pipeline |

---

## Minimal `dm.yaml`

```yaml
version: 1
project_name: my-app

services:
  api:
    path: ./services/api
    language: rust
    tags: [backend]
    depends_on: [db]
    health:
      kind: http
      url: http://localhost:8080/health
    resources:
      memory_mb: 512
      on_exceed: notify
    before_start:
      - dm db migrate
  web:
    path: ./services/web
    language: vite
    order: 20
    delay_ms: 500

profiles:
  min:
    services: [api]

linter:
  dr: true
  kiss: true
  unused_code: true
  duplicates: true

notify:
  webhook_url: ${SLACK_WEBHOOK_URL}
  events: [crash]
```

Full schema: [docs/configuration.md](https://nopass0.github.io/dev_manager/configuration/)

---

## Commands (55+)

| Command | Description |
|---|---|
| `dm init --template=` | Create dm.yaml and/or project from template |
| `dm new service <name> --template=` | Scaffold new service + auto-add to dm.yaml |
| `dm start` | Start services (`--only/--skip/--tag/--profile/--affected/--dry-run/--wait`) |
| `dm stop` / `dm restart <svc>` | Stop / restart |
| `dm status` / `dm logs [svc]` / `dm top` / `dm dashboard` | Status, logs, live tables |
| `dm board` | Local kanban board on port 11001 |
| `dm build [svc] [--release]` | Unified build or multi-stage pipeline |
| `dm db migrate\|seed\|reset\|shell` | Database (postgres/sqlite/redis/mongo/mysql) |
| `dm docker up\|down\|logs\|ps` | Docker/Compose infrastructure |
| `dm gen diagram` | Mermaid architecture diagram |
| `dm grep <pat>` / `dm replace <old> <new>` / `dm refs <sym>` / `dm secrets` | Search/replace/secrets |
| `dm format` / `dm lint [svc]` / `dm todo` | Format / analyze code / TODO registry |
| `dm watch [svc] -- <cmd>` / `dm hooks install` | Watcher-runner / git hooks |
| `dm commit [target] "msg"` / `dm commit auto` / `dm push` / `dm git stash\|branch\|rebase` | Git automation |
| `dm release <patch\|minor\|major>` | SemVer bump + auto-CHANGELOG |
| `dm test [svc]` / `dm deps audit\|outdated` | Tests / dependency audit |
| `dm doctor` / `dm config list\|get\|edit\|validate` | Diagnostics / config management |
| `dm ping <svc>` / `dm url <svc>` / `dm open` / `dm ports` / `dm kill` / `dm exec` / `dm shell` | Checks/processes/commands |
| `dm deploy <name>` / `dm env sync` / `dm cache clear` / `dm clean` | Deploy / .env / cleanup |
| `dm setup` / `dm update` / `dm history` / `dm list` / `dm alias` | Bootstrap / update / activity / overview |
| `dm completions <shell>` / `dm install --all-users` / `dm version` | Shell completion / PATH install / version |

---

## 📖 Documentation

**Full docs site:** [https://nopass0.github.io/dev_manager/](https://nopass0.github.io/dev_manager/)

| Page | Topic |
|---|---|
| [Getting Started](https://nopass0.github.io/dev_manager/) | Install, first run |
| [Configuration](https://nopass0.github.io/dev_manager/configuration/) | Full dm.yaml schema |
| [Commands](https://nopass0.github.io/dev_manager/commands/) | All 55+ commands |
| [Templates](https://nopass0.github.io/dev_manager/templates/) | Project templates guide |
| [Multi-repo](./examples/multi-repo/) | Cross-repo git operations |
| [Code Analysis](https://nopass0.github.io/dev_manager/code-analysis/) | DRY/KISS/unused/graph |
| [Build Pipeline](https://nopass0.github.io/dev_manager/build-pipeline/) | Multi-stage builds |
| [Kanban Board](https://nopass0.github.io/dev_manager/kanban/) | Local task board |
| [Recipes](https://nopass0.github.io/dev_manager/recipes/) | Typical workflows |
| [Troubleshooting](https://nopass0.github.io/dev_manager/troubleshooting/) | Common issues |
| [Contributing](./CONTRIBUTING.md) | How to add features |

**Examples** (`examples/`): fullstack, multi-repo, polyglot, go-monorepo,
python-microservices, os-qemu.

---

## Architecture

```
crates/
├── dm-core       config dm.yaml, single .env, project model, errors
├── dm-runtime    process orchestration, kill_tree, watcher, logs, notify, monitor, netutil
├── dm-cli        the dm binary: 55+ commands, colored output, templates, i18n, board
├── dm-vcs        git (via CLI), commit/push multi-repo, commit auto, semver, changelog
├── dm-analysis   tree-sitter: symbols, search, graph, lints, secrets
├── dm-deploy     SSH deploy (russh scaffold)
└── dm-installer  PATH install (Win+Linux), one-liner scripts
```

Principles: **DRY**, **KISS**, unified error system, trait-based extensibility,
rustdoc on all public APIs, **93 unit tests**, zero clippy warnings.

---

## Cross-Platform

| Operation | Windows | Linux/macOS |
|---|---|---|
| Shell commands | `cmd /C` | `sh -c` |
| Kill process tree | Job Objects (`kill_tree`) | process groups |
| PATH install (user) | `%LOCALAPPDATA%\Programs\dm` | `~/.local/bin` |
| PATH install (all users) | `Program Files` + Machine scope | `/usr/local/bin` |
| Desktop notifications | BurntToast / BalloonNotify | `notify-send` / `osascript` |
| Port detection | `netstat -ano` | `lsof -ti :PORT` |
| RSS monitoring | `wmic process` | `/proc/<pid>/status` |

---

## License

[MIT](./LICENSE)

---

<details>
<summary><b>⭐ If this project is useful — give it a star!</b></summary>

Stars help other developers find Dev Manager. Issues with ideas and Pull Requests
are also very welcome.
</details>

<!-- SMM: Open Graph / Twitter Card meta (GitHub renders og:image from repo) -->
<!--
og:title: Dev Manager — Unified Microservices Development Manager
og:description: Orchestrate, automate git, analyze code, scaffold projects, and deploy from one console. Rust + tree-sitter. Cross-platform.
og:type: software
og:url: https://github.com/Nopass0/dev_manager
twitter:card: summary_large_image
twitter:title: Dev Manager — DevOps from one console
twitter:description: 55+ commands for microservice orchestration, git automation, code analysis, project templates, and deployment.
-->

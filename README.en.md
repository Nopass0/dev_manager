# Dev Manager (`dm`)

> A unified development manager: microservice orchestration, git automation,
> code analysis and deployment — from a single console.

[По-русски](./README.md) · [Documentation](./docs/en/)

`dm` simplifies working with a microservice monorepo (or multi-repo): a single
`dm.yaml` describes the whole project, and `dm start` brings up every service
with hot reload, multiplexes their logs into one console and watches for code
changes. Git, tests, linters and deploy — also through `dm`.

---

## Features

- 🚀 **Process orchestration** — launch every microservice with a start order
  (`order`) and delays (`delay_ms`); guaranteed recursive kill of the whole
  process subtree on stop/restart.
- 📜 **Unified log console** — colored `[service]` prefixes, `OUT/ERR/SYS` levels.
- 🔄 **Hot reload** — the file watcher tracks changes and restarts the affected
  service (modules are ready; the full watcher→supervisor wiring lands next).
- 🔧 **Git automation** — `dm commit "msg"` commits to every repo,
  `dm commit <svc> "msg"` to a specific one, `dm commit auto` builds the message
  from the list of changed **functions/classes/structs** via tree-sitter.
- 📦 **Single `.env`** — variables grouped by `[service]` sections are dispatched
  to each service with one `dm env sync` command.
- 🔍 **Code analysis** — DRY / KISS / duplicate and unused-code detection for
  Rust, JavaScript, TypeScript, Go (extensible via the `LanguageParser` trait).
- 🌐 **SSH deployment** — targets with `manual`/`after_commit`/`after_push` triggers.
- 🪟🐧 **Cross-platform** — Windows and Linux on equal footing; one-liner install
  with automatic PATH registration.

---

## Installation

### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.ps1 | iex
```

### Linux / macOS
```sh
curl -fsSL https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.sh | sh
```

Both scripts download the right binary for your OS/arch, extract it and **add
its directory to PATH** (persistently). Restart your terminal afterwards.

> ⚠️ Replace `Nopass0/dev_manager` with the real repository path after publishing.

### From source (for development)
```sh
git clone https://github.com/Nopass0/dev_manager
cd dev_manager
cargo build --release      # binary: target/release/dm
cargo install --path crates/dm-cli   # install to ~/.cargo/bin
```

**Build requirements:** Rust nightly 1.93 (pinned in `rust-toolchain.toml`), a
C compiler (MSVC Build Tools on Windows, gcc/clang on Linux) — required by the
tree-sitter grammars. System `git` — for git commands.

---

## Quick start

```sh
# 1. Create a config in your project root:
dm init                      # → dm.yaml from a template

# 2. Edit dm.yaml for your services (see dm.example.yaml)

# 3. (opt.) set up a single .env and dispatch it:
dm env sync

# 4. Start every service with hot reload:
dm start                     # Ctrl+C for a clean shutdown

# In another terminal:
dm status                    # status table
dm commit "feat: new endpoint"   # commit to every repo
dm commit auto               # message from changed symbols
dm push                      # push each repo to its own origin
dm lint                      # DRY/KISS/unused/duplicates
dm test                      # run tests
```

---

## Minimal `dm.yaml`

```yaml
version: 1
project_name: my-app
env_file: .env

services:
  api:
    path: ./services/api
    language: rust
    watch: true
    restart_on_change: true
    order: 10              # starts first
  web:
    path: ./services/web
    language: vite
    order: 20
    delay_ms: 500          # wait half a second

linter:
  dr: true
  kiss: true
  unused_code: true
  duplicates: true
```

The full schema and every option — in [docs/en/configuration.md](./docs/en/configuration.md).

---

## Commands

| Command | Description |
|---|---|
| `dm init` | Create `dm.yaml` in the current directory |
| `dm start` | Start every service (watcher/hot-reload) |
| `dm stop` | Stop services |
| `dm restart <svc>` | Restart a service |
| `dm status` | Service status |
| `dm logs [svc]` | Service logs |
| `dm commit [target] "msg"` | Commit (every repo or a specific one); `auto` — auto message |
| `dm push` | Push every repository |
| `dm test [svc]` | Run service tests |
| `dm lint [svc]` | Code analysis (DRY/KISS/unused/duplicates) |
| `dm deploy <name>` | Deploy by target from `deploy:` |
| `dm cache clear` | Clear build caches (target, node_modules/.cache…) |
| `dm env sync` | Dispatch the single `.env` |
| `dm install` | Install this binary into PATH |
| `dm version` | Version and build info |

Details — in [docs/en/commands.md](./docs/en/commands.md).

---

## Architecture

A Cargo workspace of 7 crates with clear boundaries:

```
crates/
├── dm-core       dm.yaml config, single .env, project model, errors
├── dm-runtime    process orchestration, kill_tree, watcher, log streaming
├── dm-cli        the dm binary: clap commands, colored output
├── dm-vcs        git (via CLI), commit/push multi-repo, commit auto
├── dm-analysis   tree-sitter: symbols, doc comments, DRY/KISS/unused
├── dm-deploy     SSH deploy (russh, scaffold)
└── dm-installer  PATH install (Win+Linux), one-liner scripts
```

All code is documented with Rust doc-comments (`///` on every public
function/struct, `//!` at the top of each module). Generate HTML docs with:
```sh
cargo doc --workspace --open
```

Principles: **DRY**, **KISS**, a unified error system, feature flags for heavy
subsystems, 52 unit tests (`cargo test --workspace`).

---

## Documentation

- 📖 [Getting started](./docs/en/getting-started.md)
- ⚙️ [`dm.yaml` configuration](./docs/en/configuration.md)
- 🎛 [Commands](./docs/en/commands.md)
- 🌿 [Multi-repo commit/push](./docs/en/multi-repo.md)
- 🔬 [Code analysis](./docs/en/code-analysis.md)
- 🗂 [Single `.env`](./docs/en/env-sync.md)
- 🚀 [Deployment](./docs/en/deploy.md)
- 📥 [Installation](./docs/en/installation.md)

---

## License

[MIT](./LICENSE)

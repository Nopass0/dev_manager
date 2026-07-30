# Getting started

> Section: [Documentation](./README.md)

This guide gets you from install to a running project in 5 minutes.

## 1. Install

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.ps1 | iex
```

**Linux / macOS:**
```sh
curl -fsSL https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.sh | sh
```

After installing, **restart your terminal**, then verify:
```sh
dm version
```

If `dm` is not found — see [installation.md](./installation.md#troubleshooting).

## 2. Create a config

In the root of your monorepo:
```sh
dm init
```
This creates `dm.yaml` from a template. Edit it — at minimum list your services:

```yaml
version: 1
project_name: demo
services:
  api:
    path: ./services/api
    language: rust
  web:
    path: ./services/web
    language: vite
    order: 20
    delay_ms: 500
```

## 3. Start

```sh
dm start
```

Every service starts in `order` (respecting `delay_ms`). Logs stream into one
console with colored prefixes:

```
[dm] starting project 'demo' — 2 service(s)
[api]  SYS start: cargo run
[api]  OUT     Compiling demo v0.1.0
[web]  SYS start: npm run dev
...
```

**Ctrl+C** cleanly kills the whole process tree (including subprocesses).

## 4. Single `.env` (optional)

Create a root `.env` with sections:
```ini
LOG_LEVEL=info

[api]
DATABASE_URL=postgres://localhost/demo
PORT=3001

[web]
API_URL=http://localhost:3001
```

Dispatch to services:
```sh
dm env sync
#  ✓ api: 3 variables written to ./services/api/.env
#  ✓ web: 2 variables written to ./services/web/.env
```

Details — in [env-sync.md](./env-sync.md).

## 5. Git in one command

```sh
dm commit "feat: new endpoint"   # commit to every repository
dm push                          # push each to its own origin
```

For multi-repo and auto messages see [multi-repo.md](./multi-repo.md).

## Next steps

- [Configuration](./configuration.md) — every `dm.yaml` option.
- [Commands](./commands.md) — full list.
- [Code analysis](./code-analysis.md) — DRY/KISS/unused.

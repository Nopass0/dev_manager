# `dm.yaml` configuration

> Section: [Documentation](./README.md)

`dm.yaml` lives in the project root and describes all services, deployment and
linters. Dev Manager searches for it from the current directory upward (like
`git`), so `dm` can be run from any subdirectory.

## Full schema

```yaml
version: 1                       # only 1 (required)
project_name: my-app             # human-readable name (logs/status)
env_file: .env                   # path to the single .env (default .env)

services:                        # map of services (see below)
  <name>:
    path: ./services/<name>      # service directory (required)
    language: rust               # language/stack (required)
    repo: ./services/<name>      # separate git repository (optional)
    run: cargo run               # explicit run command (optional, else auto)
    watch: true                  # watch files (default true)
    restart_on_change: true      # restart on changes (default true)
    delay_ms: 0                  # delay before start, ms (default 0)
    order: 100                   # start-queue priority (lower = earlier)
    env:                         # extra env vars (optional)
      KEY: value
    tests:
      cmd: cargo test            # test command (empty → tests disabled)
      on_change: true            # run tests on changes
    logs:
      enabled: true              # show this service's logs in the stream
      level: info                # minimum level

deploy:                          # deploy targets (optional)
  - name: prod
    host: prod.example.com
    user: deploy
    port: 22
    key: ~/.ssh/id_ed25519
    remote_dir: /srv/my-app
    on: after_push               # manual | after_commit | after_push
    steps:                       # commands on the remote host
      - git pull
      - cargo build --release
      - systemctl restart my-app

linter:                          # code analyzer
  dr: true                       # DRY check
  kiss: true                     # KISS check
  unused_code: true              # find unused code
  duplicates: true               # find duplicate definitions
  auto_fix: false                # auto-remove unused code
```

## Service fields in detail

### `language`
Supported values: `rust`, `go`, `c`, `cpp`, `csharp`, `javascript`,
`typescript`, `bun`, `nodejs`, `lua`, `python`, `vite`, `nextjs`, `remix`,
`other`. The value drives run-command detection and tree-sitter grammar choice.

### `run` (optional)
If omitted, `dm` tries to guess the command from marker files:
- `package.json` → `npm run dev` (or `bun run dev` with `bun.lockb`);
- `Cargo.toml` → `cargo run`;
- `go.mod` → `go run .`;
- `*.csproj` → `dotnet run`;
- otherwise — a per-language default (`go run .`, `python main.py`…).

### `order` and `delay_ms`
- `order` — integer; services start in ascending order. Ties keep YAML
  declaration order.
- `delay_ms` — pause before the **next** service in the queue starts.

### `repo` (multi-repo)
If a service lives in its own git repository, set its path. Then:
- `dm commit "msg"` commits to **every** repository with one message;
- `dm commit <svc> "msg"` — only that one;
- `dm push` pushes each to its own `origin`.

See [multi-repo.md](./multi-repo.md).

## Example

A full example with all fields — [`dm.example.yaml`](../../dm.example.yaml).

## Validation

On load, `dm` checks:
- `version == 1` (else an error);
- at least one service present;
- non-empty `path` and valid service names.

Service directories' existence is checked separately (during `dm start`), so the
config can be loaded even during `dm init`, before directories exist.

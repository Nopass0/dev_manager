# `dm` commands

> Section: [Documentation](./README.md)

## `dm init`
Creates `dm.yaml` in the current directory from the built-in template. Skips if
the file already exists.

## `dm start [--no-watch] [--no-restart]`
Starts every service in `order`/`delay_ms`. Logs are multiplexed into one
console with colored prefixes. **Ctrl+C** cleanly stops the whole process tree.
- `--no-watch` — disable the file watcher.
- `--no-restart` — do not restart crashed processes.

## `dm stop`
Stops services. In the current version services live within the `dm start`
process — to stop, press Ctrl+C there. (A daemon mode with a PID file is on the
roadmap for the next iteration.)

## `dm restart <svc>`
Restarts a specific service (currently an informational hint; the full watcher
arrives in the next iteration).

## `dm status`
A table of services and their status (`pending/starting/running/stopped/crashed/exited`).

## `dm logs [svc]`
Service logs. Stream live from an active `dm start`.

## `dm commit [target] [message]`
Git automation:
- `dm commit "msg"` — commits to **every** repository with one message.
- `dm commit <svc> "msg"` — only to service `<svc>`'s repository.
- `dm commit auto` — the message is built from the list of changed symbols
  (functions/classes/structs) via tree-sitter.

Equivalent to `git add -A && git commit -m "msg"` per repository. See
[multi-repo.md](./multi-repo.md).

## `dm push`
Pushes every repository to its `origin`. Each to its own remote.

## `dm test [svc]`
Runs tests. Uses `tests.cmd` from the config; if unset — the per-language
default (`cargo test`, `npm test`, `go test ./...`, `bun test`, `pytest`).

## `dm lint [svc]`
Code analysis: DRY, KISS, duplicate and unused-code detection. Enabled checks
come from the `linter:` section of `dm.yaml`. See [code-analysis.md](./code-analysis.md).

## `dm deploy <name>`
Runs a deploy by target name from the `deploy:` section. See [deploy.md](./deploy.md).

## `dm cache clear`
Deletes service build caches: `target`, `node_modules/.cache`, `.next/cache`,
`dist`, `build`, `__pycache__`, `.pytest_cache`.

## `dm env sync`
Dispatches the single `.env` to services per `[service]` sections. See
[env-sync.md](./env-sync.md).

## `dm install`
Installs the current binary into the system PATH (`%LOCALAPPDATA%\Programs\dm`
on Windows, `~/.local/bin` on Unix). Idempotent.

## `dm version`
Prints version and build info.

## Global options
- `--help` / `-h` — command help.
- `RUST_LOG=debug` — verbose logging of `dm` internals.

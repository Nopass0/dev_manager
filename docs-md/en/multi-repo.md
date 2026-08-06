# Multi-repo: commit and push to several repositories

> Section: [Documentation](./README.md)

Dev Manager supports projects where microservices live in **separate** git
repositories. Declare this with the `repo` field:

```yaml
services:
  api:
    path: ./services/api
    language: rust
    repo: ./services/api     # ← separate repository
  web:
    path: ./services/web
    language: vite
    repo: ./services/web     # ← separate repository
```

## Commit to every repository

```sh
dm commit "feat: shared change"
```

`dm` runs `git add -A && git commit -m "feat: shared change"` in **each**
repository with the same message. Report:
```
✓ a1b2c3d ./services/api  — committed
✓ e4f5g6h ./services/web  — committed
```

## Commit to a specific repository

```sh
dm commit api "fix: API-only change"
```

`api` is the service name from the config. Only its repository is committed.

## Push

```sh
dm push
```

Each repository is pushed to **its own** `origin` (per `git remote`). This lets
you keep services in different GitHub/GitLab projects.

## `dm commit auto`

If you don't want to write the message by hand — use the auto mode:
```sh
dm commit auto
```

`dm` analyzes changed files via tree-sitter, finds the affected
functions/classes/structs and builds a readable message:
```
auto: 3 symbol(s) changed

- modified function parse (api/src/lib.rs)
- added struct User (api/src/models.rs)
- removed function old_handler (web/src/handlers.ts)
```

This message is committed to all repositories (or to the specified one).

## Mixed monorepo

If some services share one repo while others are separate, simply omit `repo`
for the shared ones: they commit to the root repository.

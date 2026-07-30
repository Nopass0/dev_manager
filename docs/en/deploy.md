# SSH deployment

> Section: [Documentation](./README.md)

Dev Manager can run a sequence of shell commands on a remote host over SSH. In
the current version this is a scaffold (the `Deployer` trait + a stub); the full
`russh` implementation lands in the next iteration.

## Target configuration

```yaml
deploy:
  - name: prod
    host: prod.example.com
    user: deploy
    port: 22
    key: ~/.ssh/id_ed25519       # private key (~ is expanded)
    remote_dir: /srv/my-app
    on: after_push               # when to run automatically
    steps:                       # remote commands, in order
      - git pull
      - cargo build --release
      - systemctl restart my-app
```

## Triggers (`on`)

| Value | When it fires |
|---|---|
| `manual` | Only on an explicit `dm deploy <name>` (default). |
| `after_commit` | After every successful `dm commit`. |
| `after_push` | After every successful `dm push`. |

## Running

```sh
dm deploy prod
```

`dm` connects to the host by key (or password), runs `steps` in order and
prints each command's result.

## Security

- Keys are stored locally under `~/.ssh/`; the path goes in the config.
- Passwords are **not** stored in the config — use an SSH agent or keys.
- `remote_dir` — the target directory on the server (for future rsync/scp ops).

## Current status (v0.1)

In this version `dm deploy` runs a **stub**: it walks the steps and prints
`[stub] step skipped`. This lets you validate the configuration and trigger
logic without a real connection. The `russh` integration (pure Rust, no C
dependency) is on the roadmap.

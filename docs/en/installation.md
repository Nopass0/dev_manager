# Installation

> Section: [Documentation](./README.md)

## One-liner (prebuilt binaries from GitHub Releases)

### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.ps1 | iex
```
The script downloads `dm-x86_64-pc-windows-msvc.zip`, extracts it to
`%LOCALAPPDATA%\Programs\dm` and adds the directory to the user `PATH`
(via `[Environment]::SetEnvironmentVariable(..., 'User')`).

### Linux / macOS
```sh
curl -fsSL https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.sh | sh
```
The script detects the architecture (`x86_64`/`aarch64`), downloads
`dm-<arch>-<os>.tar.gz`, extracts `dm` to `~/.local/bin` and, if needed, appends
`export PATH="$HOME/.local/bin:$PATH"` to `~/.bashrc` / `~/.zshrc` / `~/.profile`.

After installing, **restart your terminal** and verify:
```sh
dm version
```

> ⚠️ Replace `your-org/dev_manager` with the real repository path after publishing.

## From source

```sh
git clone https://github.com/your-org/dev_manager
cd dev_manager
cargo build --release          # binary: target/release/dm
cargo install --path crates/dm-cli   # install to ~/.cargo/bin (already on PATH)
```

Or install the built binary into the system PATH:
```sh
dm install                     # cross-platform self-registration into PATH
```

## Build requirements

| Component | Why | Windows | Linux |
|---|---|---|---|
| Rust nightly 1.93 | Compiler (pinned in `rust-toolchain.toml`) | ✓ | ✓ |
| C compiler | tree-sitter grammars (build scripts) | MSVC Build Tools | gcc/clang |
| `git` | `dm` git commands | ✓ | ✓ |

Dependency check:
```sh
rustc --version    # should report nightly
git --version
# Windows: where cl.exe  (or run from "x64 Native Tools Command Prompt")
# Linux: gcc --version
```

## Verification

```sh
cargo test --workspace          # 52 unit tests
cargo doc --workspace --open    # HTML docs for all crates
```

## Troubleshooting

### `dm: command not found` after install
- **Windows**: restart the terminal/PowerShell; check that
  `$env:LOCALAPPDATA\Programs\dm` is in `Path`
  (`[Environment]::GetEnvironmentVariable('Path','User')`).
- **Linux/macOS**: run `export PATH="$HOME/.local/bin:$PATH"` or open a new
  terminal tab; make sure the line was added to your rc file.

### tree-sitter build errors
A C compiler is required. On Windows install "Visual Studio Build Tools" with
the "Desktop development with C++" workload and build from an
**x64 Native Tools Command Prompt**.

### `git` not found
Install Git for Windows / your distro's `git` package. All of `dm`'s git
operations go through the system `git`.

# Multi-Language Build Example

Builds a project from **3 languages**: Rust library + C shared library + Go application,
all orchestrated and validated by **Lua scripts**.

## Project Structure

```
multilang-build/
├── dm.yaml                  ← build pipeline config
├── lib-rust/                ← Rust static library
│   ├── Cargo.toml
│   └── src/lib.rs           ← add, multiply, factorial (+ tests)
├── lib-c/                   ← C shared library
│   └── utils.c              ← to_upper, str_reverse, count_char
├── app-go/                  ← Go application (uses C lib via CGO)
│   ├── go.mod
│   └── main.go
├── scripts/                 ← Lua automation
│   ├── validate_lib.lua     ← validates library artifacts
│   ├── final_check.lua      ← verifies complete dist/ structure
│   ├── test_all.lua         ← 15+ tests across all languages
│   └── package.lua          ← creates versioned archive
└── dist/                    ← clean output (created by dm build)
    ├── libs/
    │   ├── libmath.a        ← Rust
    │   └── libutils.so      ← C
    └── bin/
        └── app              ← Go
```

## Build Pipeline

```yaml
build:
  output_dir: dist
  stages:
    - name: "1. Rust library"       # cargo build --release
      on_success: scripts/validate_lib.lua

    - name: "2. C shared library"   # cc -shared
      depends_on: ["1. Rust library"]
      on_success: scripts/validate_lib.lua

    - name: "3. Go application"     # go build
      depends_on: ["2. C shared library"]
      on_success: scripts/final_check.lua
```

## Usage

```sh
cd examples/multilang-build

# Build all (with Lua validation after each stage):
dm build
# → [1/3] Rust library → validate_lib.lua ✓
# → [2/3] C shared library → validate_lib.lua ✓
# → [3/3] Go application → final_check.lua ✓
# → dist/libs/ + dist/bin/ ready

# Run all tests (15+ tests):
dmx test
# → Configuration: dm.yaml valid, stages ordered, deps correct
# → Source: all files exist, Rust has tests
# → Rust unit tests: cargo test
# → Go tests: go test
# → Artifacts: dist structure verified
# → Lua API: JSON, regex, path, zip/unzip, process

# Package into versioned archive:
dmx package
# → multilang-app-v20260816120000-windows-x86_64.zip

# Run the app:
dm start
```

## What Lua Scripts Do

### validate_lib.lua (after each library build)
- Checks dist/libs/ exists and has files
- Verifies each file is not empty
- Reports file sizes

### final_check.lua (after complete build)
- Verifies expected directory structure (libs/ + bin/)
- Checks all expected files exist (libmath.a, libutils.so, app)
- Warns about unexpected files in dist root

### test_all.lua (dmx test)
- **Config tests**: dm.yaml valid, stages ordered, dependencies correct
- **Source tests**: all source files exist, Rust has #[cfg(test)]
- **Rust tests**: runs `cargo test` in lib-rust/
- **Go tests**: runs `go test` in app-go/
- **Artifact tests**: dist/ structure complete
- **Lua API tests**: JSON roundtrip, regex, path manipulation, zip/unzip, process listing

### package.lua (dmx package)
- Creates versioned archive with timestamp
- Verifies archive by extracting to temp dir
- Lists all files in the archive
- Provides distribution instructions

## Writing Your Own Build Script

```lua
-- my_build_check.lua
local dist = path.join({dm_ctx.root, "dist"})

if not fs.exists(dist) then
    error("dist/ not found - run dm build first")
end

-- Check your specific artifacts:
local required = {"myapp.exe", "config.json", "README.md"}
for _, f in ipairs(required) do
    if not fs.exists(path.join({dist, f})) then
        error("Missing: " .. f)
    end
end

log.info("All artifacts present!")
```

Register in dm.yaml:
```yaml
build:
  stages:
    - name: "my stage"
      on_success: my_build_check.lua
```

# Example: Lua-Driven Development

This project demonstrates using **Lua scripts for everything** — build validation,
smoke tests, integration tests, and health monitoring.

## Quick Start

```sh
cd examples/lua-driven

# Create the API service from template:
dm init --template=bun-elysia --name=api
cd api && mv ../api/* . 2>/dev/null || true
cd ..

# Install deps and start:
dm setup
dm start  # smoke tests run automatically after startup

# Run integration tests:
dmx test          # or: dm lua scripts/integration_test.lua

# Start health monitor (in another terminal):
dmx monitor       # or: dm lua scripts/monitor.lua
```

## How It Works

The `dm.yaml` configures Lua hooks:

```yaml
services:
  api:
    hooks:
      after_start: [scripts/smoke.lua]      # Runs after health check passes
      after_build: [scripts/check_artifacts.lua]  # Runs after build
      check_deps: true                       # Auto-installs dependencies
```

## Scripts

| Script | Trigger | What it does |
|---|---|---|
| `smoke.lua` | auto (after_start) | Waits for health, verifies endpoint, checks process & memory |
| `check_artifacts.lua` | auto (after_build) | Validates dist/ has files, warns on large artifacts |
| `integration_test.lua` | manual (`dmx test`) | 10 tests: HTTP, process, memory, config, JSON, fs, regex |
| `monitor.lua` | manual (`dmx monitor`) | Continuous health check, auto-restart on failures |

## What the Scripts Use

```lua
-- HTTP requests
http.get("http://localhost:3000/health")

-- Process management
proc.find("bun")           -- find PIDs
proc.rss(pid)              -- memory usage
proc.kill(pid)             -- kill process

-- Service management
svc.restart("api")          -- restart service
svc.start("api")

-- Project context
dm_ctx.services()           -- list services
dm_ctx.service("api")       -- get service config

-- Files
fs.write("/tmp/test", "data")
fs.read("/tmp/test")
fs.exists("dist")

-- OS
dm_os.exec("ls")            -- run command
dm_os.sleep(5000)           -- wait 5s

-- JSON
json.encode({name = "test"})
json.decode('{"x": 1}')

-- Regex
regex.match("%d+", "abc123")  -- "123"

-- System info
sys.os       -- "windows" / "linux"
sys.homedir  -- home directory
path.join({"a", "b"})  -- join paths

-- Logging
log.info("message")
dm_log.warn("warning")
```

## Writing Your Own Tests

Create a Lua file and use the `test()` helper pattern:

```lua
-- my_test.lua
local passed = 0

local function test(name, fn)
    local ok, err = pcall(fn)
    if ok then
        passed = passed + 1
        log.info("  ✓ " .. name)
    else
        error(name .. " failed: " .. tostring(err))
    end
end

test("something works", function()
    assert(1 + 1 == 2)
end)

log.info("All " .. passed .. " tests passed")
```

Run it:
```sh
dm lua my_test.lua
```

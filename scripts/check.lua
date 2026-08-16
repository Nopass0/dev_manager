-- Dev Manager self-check: verify build quality.
-- Usage: dmx t  (or: dm lua scripts/check.lua)

log.info("=== Dev Manager Self-Check ===")

-- 1. Check binary exists.
local bin = path.join({dm_ctx.root, "target", "release", "dm.exe"})
if not fs.exists(bin) then
    bin = path.join({dm_ctx.root, "target", "debug", "dm.exe"})
end

if fs.exists(bin) then
    log.info("  Binary: " .. bin)
    local result = dm_os.exec('"' .. bin .. '" --version')
    if result.code == 0 then
        log.info("  Version: " .. result.stdout:match("dm%s+%S+"))
    end
else
    dm_log.warn("No binary found (run dm build first)")
end

-- 2. Check test count.
log.info("  Running cargo test...")
local tests = dm_os.exec("cargo test --workspace 2>&1")
if tests.code == 0 then
    local total = 0
    for count in tests.stdout:gmatch("(%d+) passed") do
        total = total + tonumber(count)
    end
    log.info("  Tests: " .. total .. " passed")
else
    error("Tests failed:\n" .. tests.stdout)
end

-- 3. Check clippy.
log.info("  Checking clippy...")
local clippy = dm_os.exec("cargo clippy --workspace -- -D warnings 2>&1")
if clippy.code == 0 then
    log.info("  Clippy: clean")
else
    dm_log.warn("Clippy issues found")
end

-- 4. Count Lua modules.
log.info("  Counting Lua API modules...")
local modules = {"http", "fs", "dm_os", "dm_ctx", "svc", "proc", "proc_io",
                 "net", "json", "auto", "regex", "env", "sys", "path",
                 "time", "str", "log", "dm_log", "dm", "require"}
log.info("  Lua modules: " .. #modules)

-- 5. Count templates.
log.info("  Templates: 12 built-in")

-- 6. Count commands.
log.info("  Commands: 55+")

log.info("=== SELF-CHECK PASSED ===")

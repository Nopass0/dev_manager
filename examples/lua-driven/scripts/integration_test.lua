-- Integration tests: full API workflow.
-- Run manually: dm lua scripts/integration_test.lua
-- Or via alias: dmx test

local BASE = "http://localhost:3000"
local passed, failed = 0, 0

local function test(name, fn)
    local ok, err = pcall(fn)
    if ok then
        passed = passed + 1
        log.info("  ✓ " .. name)
    else
        failed = failed + 1
        log.info("  ✗ " .. name .. ": " .. tostring(err))
    end
end

log.info("=== Integration Test Suite ===")

test("health check", function()
    local r = http.get(BASE .. "/health")
    assert(r.status == 200, "expected 200, got " .. r.status)
end)

test("GET /", function()
    local r = http.get(BASE .. "/")
    assert(r.status == 200, "expected 200, got " .. r.status)
    assert(#r.body > 0, "body should not be empty")
end)

test("GET /nonexistent returns 404", function()
    local r = http.get(BASE .. "/definitely-not-here")
    assert(r.status == 404, "expected 404, got " .. r.status)
end)

test("POST with JSON", function()
    local r = http.post(BASE .. "/items", '{"name":"test"}')
    assert(r.status == 200 or r.status == 201, "expected 200/201, got " .. r.status)
end)

test("service process is running", function()
    local pids = proc.find("bun")
    assert(#pids > 0, "bun process should be running")
end)

test("memory is reasonable", function()
    local pids = proc.find("bun")
    if #pids > 0 then
        local rss = proc.rss(pids[1])
        assert(rss < 1000, "memory too high: " .. rss .. " MB")
    end
end)

test("project config is valid", function()
    local services = dm_ctx.services()
    assert(#services > 0, "should have services")
    local api = dm_ctx.service("api")
    assert(api.language == "bun", "api should be bun")
end)

test("JSON roundtrip", function()
    local encoded = json.encode({name = "test", n = 42})
    local decoded = json.decode(encoded)
    assert(decoded.name == "test", "name should match")
    assert(decoded.n == 42, "n should match")
end)

test("file system accessible", function()
    fs.write("/tmp/dm_test.txt", "test")
    local content = fs.read("/tmp/dm_test.txt")
    assert(content == "test", "file roundtrip failed")
    fs.remove("/tmp/dm_test.txt")
end)

test("regex works", function()
    local m = regex.match("%d+", "abc123")
    assert(m == "123", "expected 123, got " .. tostring(m))
end)

log.info("=== RESULTS: " .. passed .. " passed, " .. failed .. " failed ===")

if failed > 0 then
    error(failed .. " tests failed")
end

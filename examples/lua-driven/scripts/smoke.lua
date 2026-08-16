-- Smoke test: verify service is healthy after startup.
-- Called automatically by dm.yaml: hooks.after_start

local BASE = "http://localhost:3000"
log.info("Running smoke tests against " .. BASE)

-- Wait for service to be ready (up to 30 attempts).
local healthy = false
for i = 1, 30 do
    local resp = http.get(BASE .. "/health")
    if resp.status == 200 then
        healthy = true
        log.info("Healthy after " .. i .. " attempts")
        break
    end
    dm_os.sleep(1000)
end

if not healthy then
    error("Service did not become healthy within 30 seconds")
end

-- Verify main endpoint.
local main = http.get(BASE .. "/")
if main.status ~= 200 then
    error("Main endpoint returned " .. main.status)
end
log.info("Main endpoint OK")

-- Check process is running.
local pids = proc.find("bun")
if #pids == 0 then
    error("Bun process not found")
end
log.info("Process running, PID: " .. pids[1])

-- Check memory usage.
local rss = proc.rss(pids[1])
log.info("Memory usage: " .. rss .. " MB")
if rss > 500 then
    dm_log.warn("High memory usage detected: " .. rss .. " MB")
end

log.info("=== SMOKE TESTS PASSED ===")

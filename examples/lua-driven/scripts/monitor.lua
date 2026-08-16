-- Health monitor: watches service and restarts if needed.
-- Run via alias: dmx monitor
-- Stop with Ctrl+C

log.info("=== Health Monitor Started ===")
log.info("Press Ctrl+C to stop")

local BASE = "http://localhost:3000"
local CHECK_INTERVAL = 5000  -- 5 seconds
local MAX_FAILURES = 3
local failures = 0
local restarts = 0

while true do
    local resp = http.get(BASE .. "/health")

    if resp.status == 200 then
        if failures > 0 then
            log.info("Service recovered")
        end
        failures = 0
    else
        failures = failures + 1
        dm_log.warn("Health check failed (" .. failures .. "/" .. MAX_FAILURES .. ")")

        if failures >= MAX_FAILURES then
            log.info("Restarting service...")
            svc.restart("api")
            restarts = restarts + 1
            failures = 0
            dm_os.sleep(3000)  -- Wait for restart

            -- Verify it came back.
            local check = http.get(BASE .. "/health")
            if check.status == 200 then
                log.info("Service restarted successfully (total restarts: " .. restarts .. ")")
            else
                dm_log.error("Service did not recover after restart")
            end
        end
    end

    -- Also check memory.
    local pids = proc.find("bun")
    if #pids > 0 then
        local rss = proc.rss(pids[1])
        if rss > 500 then
            dm_log.warn("High memory: " .. rss .. " MB")
            if rss > 1000 then
                dm_log.error("Memory too high, restarting")
                proc.kill(pids[1])
                dm_os.sleep(2000)
                svc.start("api")
            end
        end
    end

    dm_os.sleep(CHECK_INTERVAL)
end

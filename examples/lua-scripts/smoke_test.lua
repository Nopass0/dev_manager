-- =============================================================================
--  Smoke-тест сервиса: проверка health-эндпоинта после запуска.
--
--  Использование в dm.yaml:
--    services:
--      api:
--        hooks:
--          after_start:
--            - scripts/smoke_test.lua
--
--  Или вручную: dm lua scripts/smoke_test.lua
-- =============================================================================

local BASE_URL = "http://localhost:8080"

log.info("Smoke test: checking " .. BASE_URL .. "/health")

-- Ждём до 30 попыток (по 1 сек), пока сервис поднимется.
local max_attempts = 30
local healthy = false

for i = 1, max_attempts do
    local resp = http.get(BASE_URL .. "/health")
    if resp.status == 200 then
        healthy = true
        log.info("Service is healthy (attempt " .. i .. ")")
        break
    end
    dm_os.sleep(1000) -- 1 секунда
end

if not healthy then
    error("Service did not become healthy after " .. max_attempts .. " attempts")
end

-- Проверяем основной эндпоинт.
local main = http.get(BASE_URL .. "/")
if main.status ~= 200 then
    error("Main endpoint returned status " .. main.status)
end
log.info("Main endpoint OK: " .. string.sub(main.body, 1, 100))
log.info("SMOKE TEST PASSED")

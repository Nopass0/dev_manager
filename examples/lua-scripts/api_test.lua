-- =============================================================================
--  API-тест: полный CRUD через REST эндпоинты.
--
--  Использование: dm lua scripts/api_test.lua
--  (после того как сервис запущен через dm start)
-- =============================================================================

local BASE = "http://localhost:8080"

local function assert_status(resp, expected, context)
    if resp.status ~= expected then
        error(string.format("%s: expected %d, got %d. Body: %s",
            context, expected, resp.status, resp.body))
    end
end

log.info("API Test Suite")

-- === GET /health ===
local health = http.get(BASE .. "/health")
assert_status(health, 200, "GET /health")
log.info("✓ GET /health → 200")

-- === POST /items ===
local create = http.post(BASE .. "/items", '{"name":"test item"}')
assert_status(create, 200 or 201, "POST /items")
log.info("✓ POST /items → " .. create.status)

-- === GET /items ===
local list = http.get(BASE .. "/items")
assert_status(list, 200, "GET /items")
log.info("✓ GET /items → 200, body: " .. string.sub(list.body, 1, 100))

-- === PUT /items/1 ===
local update = http.put(BASE .. "/items/1", '{"name":"updated"}')
assert_status(update, 200, "PUT /items/1")
log.info("✓ PUT /items/1 → 200")

-- === DELETE /items/1 ===
local delete_resp = http.delete(BASE .. "/items/1")
assert_status(delete_resp, 200, "DELETE /items/1")
log.info("✓ DELETE /items/1 → 200")

log.info("ALL API TESTS PASSED")

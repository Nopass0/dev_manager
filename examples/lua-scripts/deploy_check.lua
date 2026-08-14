-- =============================================================================
--  Проверка после сборки: валидация артефактов в dist/.
--
--  Использование в dm.yaml:
--    build:
--      stages:
--        - name: "build app"
--          on_success: scripts/deploy_check.lua
-- =============================================================================

local DIST = "dist"

log.info("Checking build artifacts in " .. DIST .. "/")

-- Проверяем что каталог существует.
if not fs.exists(DIST) then
    error("Build output directory not found: " .. DIST)
end

-- Проверяем что есть хотя бы один файл.
local result = dm_os.exec("dir /b " .. DIST .. " 2>nul || ls " .. DIST)
if result.code ~= 0 or #result.stdout == 0 then
    error("No artifacts found in " .. DIST)
end

log.info("Artifacts found:")
for line in string.gmatch(result.stdout, "[^\r\n]+") do
    log.info("  " .. line)
end

log.info("BUILD CHECK PASSED")

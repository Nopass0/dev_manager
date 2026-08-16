-- Build artifact validation.
-- Called after build completes: hooks.after_build

local DIST = "dist"

log.info("Validating build artifacts in " .. DIST .. "/")

-- Check directory exists.
if not fs.exists(DIST) then
    error("Build output directory not found: " .. DIST)
end

-- List files using OS command.
local result
if sys.os == "windows" then
    result = dm_os.exec("dir /b " .. DIST)
else
    result = dm_os.exec("ls " .. DIST)
end

if result.code ~= 0 then
    error("Failed to list " .. DIST)
end

-- Count files.
local files = {}
for line in result.stdout:gmatch("[^\r\n]+") do
    if #line > 0 then
        table.insert(files, line)
    end
end

if #files == 0 then
    error("No build artifacts found in " .. DIST)
end

log.info("Found " .. #files .. " artifacts:")
for _, f in ipairs(files) do
    log.info("  " .. f)
end

-- Check file sizes (warn if too large).
for _, f in ipairs(files) do
    local filepath = path.join({DIST, f})
    local size_cmd
    if sys.os == "windows" then
        size_cmd = 'for %F in ("' .. filepath .. '") do @echo %~zF'
    else
        size_cmd = 'stat -f%z "' .. filepath .. '"'
    end
    local r = dm_os.exec(size_cmd)
    if r.code == 0 and r.stdout and #r.stdout > 0 then
        local size = tonumber(r.stdout:match("%d+"))
        if size and size > 50 * 1024 * 1024 then
            dm_log.warn("Large artifact: " .. f .. " (" .. math.floor(size / 1024 / 1024) .. " MB)")
        end
    end
end

log.info("=== BUILD VALIDATION PASSED ===")

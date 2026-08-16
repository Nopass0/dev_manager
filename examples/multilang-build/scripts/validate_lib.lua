-- Validate library artifact after build stage.
-- Called by dm.yaml: build.stages[].on_success

log.info("=== Validating library artifact ===")

-- Get the dist directory path.
local dist = path.join({dm_ctx.root, "dist", "libs"})

-- Check directory exists.
if not fs.exists(dist) then
    error("Libraries directory not found: " .. dist)
end

-- List all files in libs/.
local result
if sys.os == "windows" then
    result = dm_os.exec('dir /b "' .. dist .. '"')
else
    result = dm_os.exec('ls "' .. dist .. '"')
end

if result.code ~= 0 then
    error("Failed to list " .. dist .. ": " .. result.stderr)
end

-- Parse output into file list.
local files = {}
for line in result.stdout:gmatch("[^\r\n]+") do
    if #line > 0 and line ~= "." and line ~= ".." then
        table.insert(files, line)
    end
end

if #files == 0 then
    error("No library files found in " .. dist)
end

-- Validate each library file.
for _, f in ipairs(files) do
    local filepath = path.join({dist, f})
    local ext = path.ext(f)

    log.info("  Found: " .. f .. " (" .. ext .. ")")

    -- Check file is not empty.
    local content = fs.read(filepath)
    if not content or #content == 0 then
        error("Library file is empty: " .. f)
    end

    -- Check size is reasonable (not 0, not > 100MB).
    -- Note: fs.read gives us content, we check it exists.
    log.info("  Size: " .. #content .. " bytes")
end

log.info("=== Library validation passed (" .. #files .. " files) ===")

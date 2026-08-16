-- Package the distribution into a versioned archive.
-- Usage: dmx package  (or: dm lua scripts/package.lua)

log.info("=== Packaging Distribution ===")

local root = dm_ctx.root
local dist = path.join({root, "dist"})

-- Check dist exists.
if not fs.exists(dist) then
    error("dist/ not found. Run 'dm build' first.")
end

-- Generate version string (timestamp-based).
local ts = os.time("%Y%m%d%H%M%S") or tostring(time.now())
local version = "v" .. ts
local platform = sys.os .. "-" .. sys.arch

-- Archive name.
local archive_name = "multilang-app-" .. version .. "-" .. platform
local zip_path = path.join({root, archive_name .. ".zip"})

log.info("Platform: " .. platform)
log.info("Version: " .. version)
log.info("Archive: " .. zip_path)

-- Create the archive.
local ok = fs.zip(dist, zip_path)
if not ok then
    error("Failed to create archive")
end

-- Verify archive exists and is not empty.
if not fs.exists(zip_path) then
    error("Archive was not created: " .. zip_path)
end

-- List archive contents (verification).
log.info("")
log.info("Archive contents:")

-- Extract to temp for verification.
local verify_dir = path.join({root, ".verify_" .. ts})
fs.unzip(zip_path, verify_dir)

if fs.exists(verify_dir) then
    local result = dm_os.exec(
        sys.os == "windows"
        and 'dir /s /b "' .. verify_dir .. '"'
        or 'find "' .. verify_dir .. '" -type f'
    )
    if result.code == 0 then
        for line in result.stdout:gmatch("[^\r\n]+") do
            local rel = line:gsub(verify_dir .. "[\\/]", "")
            if #rel > 0 then
                log.info("  " .. rel)
            end
        end
    end
    -- Cleanup verification dir.
    fs.remove(verify_dir)
end

log.info("")
log.info("Package created: " .. zip_path)
log.info("Size: use 'ls -la " .. zip_path .. "' to check")

-- Generate checksum.
log.info("")
log.info("To distribute:")
log.info("  1. Upload " .. zip_path .. " to release")
log.info("  2. Users download and: dm lua -e 'fs.unzip(\"" .. archive_name .. ".zip\", \"app\")'")

-- Package Dev Manager release binaries.
-- Called after successful build: on_success hook.

log.info("=== Packaging Dev Manager ===")

local root = dm_ctx.root
local target = path.join({root, "target", "release"})

-- Check binaries exist.
local dm_bin = path.join({target, "dm.exe"})
if not fs.exists(dm_bin) then
    error("dm.exe not found in target/release/")
end

-- Create dist directory.
local dist = path.join({root, "dist"})
if fs.exists(dist) then
    fs.remove(dist)
end
fs.mkdir(dist)

-- Copy binaries.
fs.copy(dm_bin, path.join({dist, "dm.exe"}))
fs.copy(dm_bin, path.join({dist, "dmx.exe"}))
log.info("  Copied: dm.exe + dmx.exe")

-- Create versioned archive.
local ts = tostring(time.now())
local zip_name = "dm-" .. ts .. "-windows-x86_64.zip"
local zip_path = path.join({root, zip_name})

local ok = fs.zip(dist, zip_path)
if ok then
    log.info("  Archive: " .. zip_path)
    log.info("  Ready for GitHub Release upload")
else
    dm_log.warn("Failed to create archive")
end

-- List final contents.
log.info("")
log.info("Release contents:")
local listing = dm_os.exec('dir /b "' .. dist .. '"')
if listing.code == 0 then
    for line in listing.stdout:gmatch("[^\r\n]+") do
        if #line > 0 then
            log.info("  " .. line)
        end
    end
end

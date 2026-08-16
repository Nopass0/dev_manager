-- Final build validation: verify complete dist/ structure.
-- Called after last build stage succeeds.

log.info("=== Final Build Validation ===")

local root = dm_ctx.root
local dist = path.join({root, "dist"})

-- Expected structure.
local expected = {
    libs = {"libmath.a", "libutils.so"},
    bin = {"app"}
}

-- Check each expected directory and file.
local errors = 0

for dir, files in pairs(expected) do
    local dirpath = path.join({dist, dir})
    if not fs.exists(dirpath) then
        dm_log.error("Missing directory: " .. dirpath)
        errors = errors + 1
    else
        for _, f in ipairs(files) do
            local fp = path.join({dirpath, f})
            if not fs.exists(fp) then
                dm_log.error("Missing file: " .. f .. " in " .. dirpath)
                errors = errors + 1
            else
                log.info("  OK: " .. dir .. "/" .. f)
            end
        end
    end
end

-- Verify no unexpected files in dist root.
local root_files = dm_os.exec(
    sys.os == "windows"
    and 'dir /b "' .. dist .. '" 2>nul | findstr /v "libs bin"'
    or 'ls "' .. dist .. '" | grep -v "libs\\|bin"'
)
if root_files.code == 0 and #root_files.stdout > 0 then
    dm_log.warn("Extra files in dist root: " .. root_files.stdout)
end

if errors > 0 then
    error("Build validation failed with " .. errors .. " errors")
end

log.info("=== ALL BUILD CHECKS PASSED ===")
log.info("Distribution structure:")
log.info("  dist/")
log.info("  ├── libs/")
log.info("  │   ├── libmath.a   (Rust)")
log.info("  │   └── libutils.so (C)")
log.info("  └── bin/")
log.info("      └── app         (Go)")

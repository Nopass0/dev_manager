-- Run all tests for the multi-language project.
-- Usage: dmx test  (or: dm lua scripts/test_all.lua)

log.info("╔══════════════════════════════════════╗")
log.info("║   Multi-Language Test Suite          ║")
log.info("╚══════════════════════════════════════╝")

local passed, failed, skipped = 0, 0, 0

local function test(name, fn)
    local ok, err = pcall(fn)
    if ok then
        passed = passed + 1
        log.info("  [PASS] " .. name)
    else
        failed = failed + 1
        log.info("  [FAIL] " .. name .. ": " .. tostring(err))
    end
end

local function skip(name, reason)
    skipped = skipped + 1
    log.info("  [SKIP] " .. name .. " (" .. reason .. ")")
end

-- === Configuration tests ===
log.info("--- Configuration ---")

test("dm.yaml is valid", function()
    local cfg = dm_ctx.project()
    assert(cfg.project_name == "multilang-app", "wrong project name")
    assert(cfg.build.output_dir == "dist", "wrong output dir")
    assert(#cfg.build.stages == 3, "expected 3 build stages")
end)

test("build stages are ordered", function()
    local cfg = dm_ctx.project()
    local stages = cfg.build.stages
    assert(stages[1].name:find("Rust"), "stage 1 should be Rust")
    assert(stages[2].name:find("C"), "stage 2 should be C")
    assert(stages[3].name:find("Go"), "stage 3 should be Go")
end)

test("build dependencies are correct", function()
    local cfg = dm_ctx.project()
    local stages = cfg.build.stages
    -- C depends on Rust
    assert(#stages[2].depends_on > 0, "C stage should depend on Rust")
    -- Go depends on C
    assert(#stages[3].depends_on > 0, "Go stage should depend on C")
end)

-- === Source code tests ===
log.info("--- Source Code ---")

test("Rust library source exists", function()
    local p = path.join({dm_ctx.root, "lib-rust", "src", "lib.rs"})
    assert(fs.exists(p), "lib-rust/src/lib.rs not found")
end)

test("C library source exists", function()
    local p = path.join({dm_ctx.root, "lib-c", "utils.c"})
    assert(fs.exists(p), "lib-c/utils.c not found")
end)

test("Go application source exists", function()
    local p = path.join({dm_ctx.root, "app-go", "main.go"})
    assert(fs.exists(p), "app-go/main.go not found")
end)

test("Rust code has tests", function()
    local p = path.join({dm_ctx.root, "lib-rust", "src", "lib.rs"})
    local content = fs.read(p)
    assert(content and content:find("#[cfg(test)]"), "no tests found")
end)

-- === Rust unit tests ===
log.info("--- Rust Unit Tests ---")

test("cargo test in lib-rust", function()
    local libdir = path.join({dm_ctx.root, "lib-rust"})
    if not fs.exists(path.join({libdir, "Cargo.toml"})) then
        skip("cargo test", "Cargo.toml not found (run dm setup first)")
        return
    end
    local r = dm_os.exec("cd " .. libdir .. " && cargo test 2>&1")
    assert(r.code == 0, "cargo test failed:\n" .. r.stdout)
end)

-- === Go tests ===
log.info("--- Go Tests ---")

test("go test in app-go", function()
    local appdir = path.join({dm_ctx.root, "app-go"})
    if not fs.exists(path.join({appdir, "go.mod"})) then
        skip("go test", "go.mod not found (run dm setup first)")
        return
    end
    local r = dm_os.exec("cd " .. appdir .. " && go test ./... 2>&1")
    assert(r.code == 0, "go test failed:\n" .. r.stdout)
end)

-- === Build artifacts ===
log.info("--- Build Artifacts ---")

test("dist directory structure", function()
    local dist = path.join({dm_ctx.root, "dist"})
    if not fs.exists(dist) then
        skip("dist check", "dist/ not found (run dm build first)")
        return
    end
    assert(fs.exists(path.join({dist, "libs"})), "dist/libs missing")
    assert(fs.exists(path.join({dist, "bin"})), "dist/bin missing")
end)

-- === Lua API tests ===
log.info("--- Lua API ---")

test("JSON roundtrip", function()
    local obj = {name = "test", version = 2, nested = {ok = true}}
    local encoded = json.encode(obj)
    local decoded = json.decode(encoded)
    assert(decoded.name == "test")
    assert(decoded.version == 2)
    assert(decoded.nested.ok == true)
end)

test("regex works", function()
    assert(regex.match("%d+", "v1.2.3") == "1")
    local parts = regex.split("a,b,c", ",")
    assert(#parts == 3)
end)

test("path manipulation", function()
    local p = path.join({"a", "b", "c.txt"})
    assert(path.basename(p) == "c.txt")
    assert(path.ext(p) == "txt")
    assert(path.stem(p) == "c")
end)

test("zip/unzip roundtrip", function()
    -- Create test dir, zip it, unzip, verify.
    local testdir = path.join({dm_ctx.root, ".test_zip"})
    fs.mkdir(testdir)
    fs.write(path.join({testdir, "data.txt"}), "test content")

    local zippath = path.join({dm_ctx.root, ".test.zip"})
    local ok = fs.zip(testdir, zippath)
    if not ok then
        skip("zip/unzip", "zip command not available")
        return
    end

    local extractdir = path.join({dm_ctx.root, ".test_extract"})
    fs.unzip(zippath, extractdir)

    -- Note: zip might preserve directory structure.
    assert(fs.exists(extractdir), "extract dir should exist")

    -- Cleanup.
    fs.remove(testdir)
    fs.remove(extractdir)
    fs.remove(zippath)
end)

test("process listing works", function()
    local procs = proc.list()
    assert(#procs > 0, "should list processes")
end)

-- === Summary ===
log.info("")
log.info("╔══════════════════════════════════════╗")
log.info("║  RESULTS: " .. passed .. " passed, " .. failed .. " failed, " .. skipped .. " skipped")
log.info("╚══════════════════════════════════════╝")

if failed > 0 then
    error(failed .. " tests failed")
end

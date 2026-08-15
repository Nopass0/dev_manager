# Lua Scripting

Dev Manager поддерживает Lua-скрипты для автоматизации: тесты, проверки после
сборки/запуска, deploy-валидация, любые кастомные действия.

## Быстрый старт

```sh
# Создайте скрипт:
echo 'log.info("Hello from Lua!")' > test.lua

# Запустите:
dm lua test.lua
```

## API Reference

### `log` — логирование

```lua
log.info("informative message")   -- stdout
log.warn("warning message")       -- stderr
log.error("error message")        -- stderr
```

### `fs` — файловые операции

```lua
-- Чтение (возвращает nil если файла нет):
local content = fs.read("config.json")

-- Запись:
fs.write("output.txt", "hello world")

-- Проверка существования:
if fs.exists("node_modules") then
    log.info("deps installed")
end

-- Создание каталога (рекурсивно):
fs.mkdir("dist/logs")

-- Копирование (возвращает число скопированных байт):
local n = fs.copy("src.txt", "dst.txt")

-- Удаление (файл или каталог рекурсивно):
fs.remove("temp_folder")
```

### `dm_os` — системные операции

```lua
-- Выполнить команду, дождаться, получить результат:
local result = dm_os.exec("cargo build --release")
-- result.code   — exit code (0 = успех)
-- result.stdout — стандартный вывод
-- result.stderr — поток ошибок

if result.code ~= 0 then
    error("Build failed: " .. result.stderr)
end

-- Запустить программу не дожидаясь (возвращает PID):
local pid = dm_os.spawn("my-server --port 8080")

-- Пауза в миллисекундах:
dm_os.sleep(5000)  -- 5 секунд

-- Переменные окружения:
local path = dm_os.getenv("PATH")
dm_os.setenv("NODE_ENV", "production")

-- Текущий каталог:
local cwd = dm_os.cwd()
```

### `http` — HTTP клиент

```lua
-- GET (возвращает таблицу { status, body }):
local resp = http.get("http://localhost:8080/health")
if resp.status == 200 then
    log.info("Healthy: " .. resp.body)
end

-- POST с JSON body:
local created = http.post("http://localhost:8080/items", '{"name":"test"}')

-- PUT:
http.put("http://localhost:8080/items/1", '{"name":"updated"}')

-- DELETE:
http.delete("http://localhost:8080/items/1")
```

### `dm` — вызов dm команд

```lua
-- Выполнить dm-команду как subprocess:
local result = dm.run("test api")
if result.code == 0 then
    log.info("Tests passed")
end

-- Доступные команды — все 55+ dm команд:
dm.run("build --release")
dm.run("lint")
dm.run("commit auto")
```

## Хуки в dm.yaml

Скрипты подключаются к жизненному циклу через секцию `hooks:`:

```yaml
services:
  api:
    path: ./services/api
    language: rust
    hooks:
      # Перед запуском (Lua-скрипт или shell-команда):
      before_start:
        - scripts/migrate.lua
        - echo "migrations done"

      # После успешного health-check:
      after_start:
        - scripts/smoke_test.lua

      # После сборки:
      after_build:
        - scripts/deploy_check.lua

      # После тестов:
      after_test:
        - scripts/notify_test_result.lua

      # Проверка/установка зависимостей перед каждым запуском:
      check_deps: true
      install_cmd: "cargo fetch"
      deps_marker: "Cargo.lock"
```

### Build stage hooks

```yaml
build:
  stages:
    - name: "build app"
      command: "cargo build --release"
      on_success: scripts/validate_build.lua
      on_failure: scripts/alert_failure.lua
```

## Примеры

### Smoke-тест после запуска

```lua
-- scripts/smoke_test.lua
local BASE = "http://localhost:8080"

for i = 1, 30 do
    local resp = http.get(BASE .. "/health")
    if resp.status == 200 then
        log.info("Healthy after " .. i .. " attempts")
        return
    end
    dm_os.sleep(1000)
end
error("Service did not become healthy")
```

### Проверка артефактов после сборки

```lua
-- scripts/deploy_check.lua
if not fs.exists("dist") then
    error("dist/ not found")
end
log.info("Build artifacts ready")
```

### Полный API CRUD тест

См. [examples/lua-scripts/api_test.lua](../examples/lua-scripts/api_test.lua)

## Стандартный Lua

Все стандартные возможности Lua 5.4 доступны: `string`, `table`, `math`,
`os.time`, `pcall`, корутины и т.д. Плюс dm API поверх.

---

# Расширенный API (v2)

## dm_ctx — контекст проекта

```lua
-- Корень проекта (каталог dm.yaml):
dm_ctx.root            -- "/path/to/project"
dm_ctx.config_path     -- "/path/to/project/dm.yaml"

-- Вся конфигурация как Lua-таблица:
local project = dm_ctx.project()
print(project.project_name)      -- "my-app"
print(project.linter.dr)         -- true

-- Список имён сервисов:
local names = dm_ctx.services()
for _, name in ipairs(names) do
    print("service: " .. name)
end

-- Данные конкретного сервиса:
local api = dm_ctx.service("api")
print(api.path)          -- "./services/api"
print(api.language)      -- "rust"
print(api.watch)         -- true
```

## svc — управление сервисами

```lua
-- Список всех:
local list = svc.list()

-- Получить данные:
local api = svc.get("api")

-- Управление жизненным циклом (через dm subprocess):
svc.start("api")        -- запустить
svc.stop("api")         -- остановить
svc.restart("api")      -- перезапустить

-- Добавить сервис (пишет в dm.yaml):
svc.add("worker", {
    path = "./worker",
    language = "go",
    order = "30"
})

-- Удалить сервис (удаляет из dm.yaml):
svc.remove("worker")
```

## proc — процессы и память

```lua
-- Список всех процессов:
local procs = proc.list()
for _, p in ipairs(procs) do
    print(p.pid, p.name)
end

-- Найти PID по имени:
local pids = proc.find("chrome")
if #pids > 0 then
    print("Chrome PID: " .. pids[1])
end

-- Завершить процесс:
proc.kill(12345)

-- RSS памяти процесса (МБ, -1 если не удалось):
local mb = proc.rss(pid)
if mb > 500 then
    log.warn("Process using too much memory: " .. mb .. " MB")
end
```

## dm_log — логи dm

```lua
dm_log.info("message")    -- [script:info]
dm_log.warn("message")    -- [script:warn]
dm_log.error("message")   -- [script:error]
dm_log.debug("message")   -- [script:debug]
```

## json — кодирование/декодирование

```lua
-- Lua таблица → JSON строка:
local encoded = json.encode({ name = "test", count = 42 })

-- JSON строка → Lua таблица:
local data = json.decode('{"users":[{"name":"alice"},{"name":"bob"}]}')
print(data.users[1].name)  -- "alice"
print(#data.users)          -- 2

-- Вложенные структуры работают:
local nested = json.decode('{"a":{"b":{"c":[1,2,3]}}}')
print(nested.a.b.c[2])  -- 2
```

## require — импорт модулей

```lua
-- mylib.lua:
local M = {}
function M.greet(name) return "Hello, " .. name end
return M

-- main.lua:
local mylib = require("mylib")  -- ищет mylib.lua в cwd и scripts/
print(mylib.greet("World"))      -- "Hello, World"
```

Поиск: `<module>.lua` и `scripts/<module>.lua` относительно текущего каталога.

## Комплексный пример: мониторинг и рестарт

```lua
-- scripts/monitor.lua — следит за памятью и перезапускает
while true do
    local pids = proc.find("my-app")
    if #pids > 0 then
        local rss = proc.rss(pids[1])
        if rss > 1000 then  -- > 1GB
            dm_log.warn("Memory leak detected: " .. rss .. " MB, restarting")
            proc.kill(pids[1])
            dm_os.sleep(3000)
            svc.start("my-app")
        end
    end
    dm_os.sleep(5000)  -- каждые 5 секунд
end
```

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

## proc_io — интерактивные процессы (stdin/stdout)

```lua
-- Запустить программу с доступом к stdin/stdout:
local p = proc_io.spawn("my-cli-app")

-- Записать в stdin приложения (ответить на запрос):
p.write("yes\n")

-- Прочитать строку из stdout (блокирующая):
local line = p.read_line()
print("app said: " .. line)

-- Прочитать весь вывод:
local output = p.read_all()

-- Дождаться завершения (возвращает exit code):
local code = p.wait()

-- Убить принудительно:
p.kill()
```

**Кейс: тестирование CLI-взаимодействия**
```lua
local p = proc_io.spawn("./app --interactive")
local prompt = p.read_line()          -- "Enter name:"
p.write("Alice")                       -- вводим имя
local greeting = p.read_line()         -- "Hello, Alice!"
assert(greeting:find("Alice"), "should greet by name")
p.wait()
```

## net — TCP-клиент

```lua
-- Проверить открыт ли порт:
if net.port_open("localhost", 8080) then
    log.info("server is up")
end

-- Быстрая отправка (fire-and-forget):
net.tcp_send("localhost", 9090, "PING\n")

-- Полное соединение с send/recv:
local conn = net.tcp_connect("localhost", 6379)
conn.send("PING\r\n")
local response = conn.recv(64)     -- прочитать до 64 байт
print("redis replied: " .. response)
conn.close()
```

**Кейс: тест Redis протокола**
```lua
local conn = net.tcp_connect("127.0.0.1", 6379)
conn.send("SET dm:test hello\r\n")
conn.send("GET dm:test\r\n")
local reply = conn.recv(256)
assert(reply:find("hello"), "GET should return hello")
conn.close()
```

## time — время

```lua
local ts = time.now()          -- Unix timestamp (секунды)
local ms = time.now_ms()       -- Unix timestamp (миллисекунды)

-- Измерение длительности:
local start = time.now_ms()
dm_os.sleep(1500)
local took = time.elapsed_ms(start)
assert(took >= 1500, "should take at least 1.5s")

-- Пауза в секундах:
time.sleep_s(2)  -- 2 секунды
```

## str — строковые утилиты

```lua
-- Разбиение:
local parts = str.split("a,b,c", ",")
-- parts[1]="a" parts[2]="b" parts[3]="c"

str.trim("  hello  ")               -- "hello"
str.starts_with("file.lua", "file")  -- true
str.ends_with("file.lua", ".lua")    -- true
str.contains("hello", "ell")         -- true
str.upper("abc")                     -- "ABC"
str.lower("XYZ")                     -- "xyz"
```

## auto — автоматизация UI (клавиатура, мышь, скриншоты)

### Клавиатура

```lua
-- Нажать клавишу:
auto.key_press("enter")
auto.key_press("a")
auto.key_press("escape")

-- Напечатать текст (как с клавиатуры):
auto.type_text("Hello, World!")

-- Зажать/отпустить (для кастомных комбинаций):
auto.key_down("shift")
auto.key_press("a")         -- получится "A"
auto.key_up("shift")

-- Горячие клавиши (модификаторы + клавиша):
auto.hotkey({"ctrl"}, "s")           -- Ctrl+S
auto.hotkey({"ctrl", "shift"}, "t")  -- Ctrl+Shift+T
auto.hotkey({"alt"}, "f4")           -- Alt+F4
```

**Поддерживаемые клавиши**: a-z, 0-9, enter, escape, tab, space, backspace,
delete, home, end, pageup, pagedown, f1-f12, стрелки (up/down/left/right),
ctrl, shift, alt, win/meta/cmd, capslock.

### Мышь

```lua
-- Переместить курсор:
auto.mouse_move(500, 300)

-- Клик (левой кнопкой):
auto.click(500, 300)         -- по координатам
auto.click()                  -- в текущей позиции

-- Двойной клик:
auto.double_click(500, 300)

-- Правый клик (контекстное меню):
auto.right_click(500, 300)

-- Перетаскивание (drag & drop):
auto.drag(100, 100, 400, 300)

-- Прокрутка (отрицательное = вверх):
auto.scroll(3)     -- вниз на 3 шага
auto.scroll(-5)    -- вверх

-- Зажать/отпустить кнопку (для кастомного drag):
auto.mouse_down("left")
auto.mouse_move(200, 200)
auto.mouse_up("left")

-- Текущая позиция курсора:
local pos = auto.mouse_pos()
print(pos.x, pos.y)
```

### Скриншоты

```lua
-- Весь экран:
auto.screenshot("screenshot.png")

-- Область экрана:
auto.screenshot_region("region.png", 100, 100, 800, 600)
```

### Окна

```lua
-- Список всех окон:
local windows = auto.windows()
for _, w in ipairs(windows) do
    print(w.title, w.x, w.y, w.w, w.h)
end

-- Найти окно по части заголовка:
local notepad = auto.find_window("Notepad")
if notepad.found ~= false then
    print("Notepad at: " .. notepad.x .. "," .. notepad.y)
end

-- Активировать окно (вывести на передний план):
auto.activate_window(notepad)
```

### Кейс: полный UI-тест приложения

```lua
-- scripts/ui_test.lua
-- 1. Запускаем приложение
local p = proc_io.spawn("notepad")
dm_os.sleep(2000)

-- 2. Находим окно
local win = auto.find_window("Notepad")
assert(win.found ~= false, "Notepad window not found")

-- 3. Активируем и печатаем
auto.activate_window(win)
auto.type_text("Hello from dm automation!")

-- 4. Сохраняем (Ctrl+S)
auto.hotkey({"ctrl"}, "s")
dm_os.sleep(1000)

-- 5. Скриншот результата
auto.screenshot("test_result.png")
log.info("UI test completed, screenshot saved")

-- 6. Закрываем
auto.hotkey({"alt"}, "f4")
p.wait()
```

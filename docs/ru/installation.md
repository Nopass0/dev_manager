# Установка

> Раздел: [Документация](./README.md)

## Oneliner (готовые бинарники из GitHub Releases)

### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.ps1 | iex
```
Скрипт скачивает `dm-x86_64-pc-windows-msvc.zip`, распаковывает в
`%LOCALAPPDATA%\Programs\dm` и добавляет каталог в пользовательский `PATH`
(через `[Environment]::SetEnvironmentVariable(..., 'User')`).

### Linux / macOS
```sh
curl -fsSL https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.sh | sh
```
Скрипт определяет архитектуру (`x86_64`/`aarch64`), скачивает
`dm-<arch>-<os>.tar.gz`, распаковывает `dm` в `~/.local/bin` и при необходимости
дописывает `export PATH="$HOME/.local/bin:$PATH"` в `~/.bashrc` / `~/.zshrc` /
`~/.profile`.

После установки **перезапустите терминал** и проверьте:
```sh
dm version
```

> ⚠️ Замените `your-org/dev_manager` на реальный путь репозитория после публикации.

## Из исходников

```sh
git clone https://github.com/your-org/dev_manager
cd dev_manager
cargo build --release          # бинарник: target/release/dm
cargo install --path crates/dm-cli   # установить в ~/.cargo/bin (уже в PATH)
```

Или установить собранный бинарник в системный PATH:
```sh
dm install                     # добавит себя в PATH кросс-платформенно
```

## Требования для сборки

| Компонент | Зачем | Windows | Linux |
|---|---|---|---|
| Rust nightly 1.93 | Компилятор (закреплён в `rust-toolchain.toml`) | ✓ | ✓ |
| C-компилятор | tree-sitter-грамматики (build-скрипты) | MSVC Build Tools | gcc/clang |
| `git` | Git-команды `dm` | ✓ | ✓ |

Проверка зависимостей:
```sh
rustc --version    # должно быть nightly
git --version
# Windows: где cl.exe  (или запустите из "x64 Native Tools Command Prompt")
# Linux: gcc --version
```

## Проверка

```sh
cargo test --workspace          # 52 unit-теста
cargo doc --workspace --open    # HTML-документация всех crate'ов
```

## Troubleshooting

### `dm: command not found` после установки
- **Windows**: перезапустите терминал/PowerShell; проверьте
  `$env:LOCALAPPDATA\Programs\dm` в `Path` (`[Environment]::GetEnvironmentVariable('Path','User')`).
- **Linux/macOS**: выполните `export PATH="$HOME/.local/bin:$PATH"` или
  откройте новую вкладку терминала; убедитесь, что строка добавлена в rc-файл.

### Ошибки сборки tree-sitter
Нужен C-компилятор. На Windows установите «Visual Studio Build Tools» с
компонентом «Desktop development with C++» и собирайте из
**x64 Native Tools Command Prompt**.

### `git` не найден
Установите Git for Windows / системный пакет `git`. Все git-операции `dm`
идут через системный `git`.

# Пример: Mini-OS (Assembler + Rust) в QEMU с hot-reload

Миниатюрная ОС: ассемблерный загрузчик (NASM) + no_std ядро на Rust, собираемые
через build-pipeline Dev Manager в единый образ и запускаемые в QEMU. При
изменении исходников `dm` автоматически пересобирает и перезапускает QEMU.

## Структура

```
os-qemu/
├── dm.yaml              ← конфиг с build-pipeline
├── boot/
│   └── boot.asm         ← 16-битный загрузчик (NASM)
├── kernel/
│   ├── Cargo.toml       ← no_std, panic = "abort"
│   └── src/main.rs      ← точка входа ядра
└── linker.ld            ← скрипт линкера
```

## Требования

- `nasm` (ассемблер);
- `qemu-system-i386` (эмулятор);
- Rust nightly с `rust-src` (для no_std);
- `ld` (линкер, входит в binutils).

## Build-pipeline

`dm.yaml` описывает **3 упорядоченных этапа** сборки:

```yaml
build:
  output_dir: dist
  stages:
    - name: "1. Ассемблер bootloader"
      source: ./boot
      command: "nasm -f elf32 boot.asm -o boot.o"
      artifacts: ["boot.o"]

    - name: "2. Rust kernel (no_std)"
      source: ./kernel
      command: "cargo build --release"
      artifacts: ["target/release/libkernel.a"]

    - name: "3. Линковка в единый образ"
      source: .
      command: "ld -m elf_i386 -T linker.ld -o dist/os.bin boot/boot.o kernel/target/release/libkernel.a"
      artifacts: ["os.bin"]
```

Каждый этап собирает артефакты в `dist/` (очищается перед сборкой).

## Запуск с hot-reload

```sh
# 1. Собрать образ:
dm build
# → [1/3] Ассемблер bootloader ✓
# → [2/3] Rust kernel (no_std) ✓
# → [3/3] Линковка в единый образ ✓
# → ✓ сборка завершена, артефакты в dist/

# 2. Запустить QEMU с hot-reload:
dm start
# watcher отслеживает boot/*.asm и kernel/src/*.rs
# при изменении → пересборка + перезапуск QEMU
```

Изменили `kernel/src/main.rs`? `dm` автоматически:
1. Пересоберёт Rust kernel;
2. Перелинкует `os.bin`;
3. Перезапустит QEMU.

## Файлы примера

**boot.asm** — минимальный 16-битный загрузчик:
```nasm
[BITS 16]
[ORG 0x7C00]
start:
    cli
    mov si, msg
    call print_string
    jmp $
print_string:
    lodsb
    or al, al
    jz .done
    mov ah, 0x0E
    int 0x10
    jmp print_string
.done:
    ret
msg db "Mini-OS loaded!", 0
times 510-($-$$) db 0
dw 0xAA55
```

**kernel/src/main.rs** — no_std точка входа:
```rust
#![no_std]
#![no_main]
#![feature(lang_items)]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Здесь — код ядра (ввод/вывод через порты, видеопамять 0xB8000 и т.д.)
    loop {}
}
```

**linker.ld** — скрипт линкера:
```
ENTRY(_start)
SECTIONS {
    . = 0x100000;
    .text : { *(.text*) }
    .data : { *(.data*) }
    .bss  : { *(.bss*) }
}
```

## Зачем этот пример

- Демонстрирует **build-pipeline**: разные ЯП (ASM + Rust) в единую папку;
- Показывает **hot-reload для нетипичных сценариев** (ОС в QEMU);
- Проверяет, что `dm` работает не только с web-стеком, но и с низкоуровневой разработкой.

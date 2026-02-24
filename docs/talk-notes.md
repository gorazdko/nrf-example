# Embedded Rust Talk Notes

Things worth pointing out for an audience new to embedded Rust.

## Project structure — what each file does

- **`Cargo.toml`** — like `package.json` or `CMakeLists.txt`. Declares
  dependencies and their *features* (feature flags that toggle
  chip-specific code, drivers, etc. at compile time)
- **`.cargo/config.toml`** — tells Cargo the compilation target
  (`thumbv7em-none-eabihf` = ARM Cortex-M4 with hardware float) and
  what to run after building (`probe-rs` flashes the chip over SWD)
- **`memory.x`** — linker script describing the chip's memory layout
  (where FLASH and RAM live). Without a bootloader or softdevice we get
  the full 1 MB flash + 256 KB RAM
- **`build.rs`** — runs at compile time, passes linker flags so the
  binary gets the right memory layout and defmt log sections
- **`rust-toolchain.toml`** — pins the Rust toolchain and target so
  anyone cloning the repo gets the same setup automatically

## The code — things to highlight

- **`#![no_std]`** — no standard library (no OS, no heap, no libc).
  Everything runs on bare metal
- **`#![no_main]`** — no regular `fn main()`. The entry point is
  provided by `cortex-m-rt` and Embassy wraps it with
  `#[embassy_executor::main]`
- **`async/await` on bare metal** — looks like normal Rust async code,
  but there's no OS scheduler. Embassy's executor is a single-threaded
  cooperative scheduler that runs in the main thread. Tasks yield at
  `.await` and get woken by hardware interrupts (timers, GPIO, DMA, etc.)
- **`Timer::after_millis(500).await`** — this doesn't spin-wait. The
  MCU goes to sleep (WFE) until the RTC interrupt fires. Zero CPU usage
  while waiting
- **Channels** — `Channel<CriticalSectionRawMutex, T, N>` is a
  fixed-capacity, no-alloc, async multi-producer multi-consumer channel.
  The `CriticalSectionRawMutex` means it disables interrupts briefly to
  protect the internal buffer — the simplest synchronization on
  single-core chips
- **Type-safe pins** — `p.P0_13` is a zero-cost singleton type. You
  can't accidentally use the same pin twice — the borrow checker
  prevents it at compile time. Passing it to `Output::new()` *consumes*
  the pin
- **Active-low LEDs** — on the DK, LEDs turn on when the pin is low.
  `Level::High` = LED off. This is just a hardware detail of the board

## Cargo.toml features worth explaining

- `"nrf52840"` — compiles the HAL for this specific chip (register
  addresses, peripheral configs)
- `"time-driver-rtc1"` — uses the RTC1 peripheral as Embassy's time
  source (32.768 kHz, very low power)
- `"gpiote"` — enables async GPIO through the GPIOTE peripheral
  (GPIO Tasks and Events — hardware event routing)
- `"task-arena-size-4096"` — Embassy allocates tasks from a static
  arena (no heap). This sets its size in bytes
- `"critical-section-single-core"` — tells the `critical-section`
  crate to just disable interrupts (no spinlocks needed on single-core)

## defmt + RTT logging

- **defmt** — a logging framework designed for microcontrollers. Unlike
  `println!`, it doesn't format strings on the target. It sends compact
  token IDs over the wire; formatting happens on the host. Tiny code
  size, fast
- **RTT (Real-Time Transfer)** — uses a shared RAM buffer that the
  debug probe reads. No UART needed, no extra wires — logs flow over
  the same SWD debug connection used for flashing

## Release profile

```toml
[profile.release]
debug = 2    # full debug info (doesn't increase flash size, stays in ELF)
lto = true   # link-time optimization across all crates — smaller binary
opt-level = "s"  # optimize for size (important on 1 MB flash)
```

## Live demo flow

1. `cargo run --release` — builds, flashes, streams logs
2. Point at the LEDs cycling on the board
3. Point at the RTT log output showing the three tasks communicating
4. `xdg-open docs/counter_state_machine.svg` — show what the compiler
   actually generated from the async code

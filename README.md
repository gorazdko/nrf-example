# nRF52840-DK Embassy LED Blink Example

A bare-metal Rust application for the **nRF52840-DK** using the [Embassy](https://embassy.dev/) async framework.
Three cooperative async tasks communicate over channels to produce a cycling LED pattern on the four on-board LEDs.

## Architecture

```
counter ──TICK_CH──► processor ──CMD_CH──► led_controller
```

| Task | Role |
|---|---|
| **counter** | Wakes every 500 ms, sends an incrementing tick to `TICK_CH` |
| **processor** | Maps each tick to a `LedCommand` — cycles `Toggle(0..3)` and sends `AllOff` every 8th tick |
| **led_controller** | Owns the four DK LEDs (P0.13–P0.16, active-low) and executes incoming commands |

Both channels are `Channel<CriticalSectionRawMutex, T, 4>` from `embassy-sync`.

## Prerequisites

- Rust stable toolchain with the `thumbv7em-none-eabihf` target
- [probe-rs](https://probe.rs/) for flashing and RTT log output

## Build & Flash

```bash
# Build
cargo build --release

# Flash and stream RTT logs
cargo run --release
```

## Visualizing Async State Machines

The Rust compiler transforms each `async fn` into a state machine. You can inspect these
with the tools below.

### Expand macros (see generated state machine structs)

```bash
# Install cargo-expand
cargo install cargo-expand

# Show the fully expanded source (requires nightly)
cargo +nightly expand --release
```

### Dump MIR with state-machine graphviz diagrams

```bash
# Generate MIR .dot files for all functions (requires nightly)
RUSTFLAGS="-Z dump-mir=all -Z dump-mir-graphviz" cargo +nightly build --release

# The .dot files land in ./mir_dump/
# Render a specific task's state machine to SVG:
dot -Tsvg mir_dump/nrf_example.counter.-------.renumber.0.mir.dot -o counter.svg
dot -Tsvg mir_dump/nrf_example.processor.-------.renumber.0.mir.dot -o processor.svg
dot -Tsvg mir_dump/nrf_example.led_controller.-------.renumber.0.mir.dot -o led_controller.svg
```

> **Tip:** The filenames in `mir_dump/` vary by pass. List them with
> `ls mir_dump/*counter*` and pick the pass you want to inspect
> (e.g. `built`, `renumber`, `optimized`).

### Dump LLVM-IR

```bash
# Emit LLVM-IR to target/<target>/release/deps/*.ll
cargo rustc --release -- --emit=llvm-ir

# Find the generated file
ls target/thumbv7em-none-eabihf/release/deps/*.ll
```

### View assembly

```bash
# Install cargo-asm (works on stable)
cargo install cargo-asm

# List available symbols
cargo asm --release --lib

# Disassemble a specific function
cargo asm --release --lib nrf_example::counter
```

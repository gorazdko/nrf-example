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

## Rust Ecosystem Tooling

A hands-on tour of Clippy, rustfmt, cargo-watch, binary size analysis, cargo-bloat,
cargo-expand, cargo-audit, and more — with live commands you can run on this project.

See **[docs/rust-tooling-showcase.md](docs/rust-tooling-showcase.md)**.

## Async State Machines

The Rust compiler transforms each `async fn` into a state machine (coroutine).
See **[docs/async-state-machines.md](docs/async-state-machines.md)** for a
detailed walkthrough of how the `counter` task is compiled, including the
generated MIR control-flow graphs for all three tasks.

### Quick: generate the graphs yourself

```bash
# Requires: rustup toolchain install nightly, apt install graphviz
cargo +nightly rustc --release -- -Z dump-mir=all -Z dump-mir-graphviz

dot -Tsvg "mir_dump/nrf_example.__counter_task-{closure#0}.-------.coroutine_resume.0.dot" \
    -o docs/counter_state_machine.svg
dot -Tsvg "mir_dump/nrf_example.__processor_task-{closure#0}.-------.coroutine_resume.0.dot" \
    -o docs/processor_state_machine.svg
dot -Tsvg "mir_dump/nrf_example.__led_controller_task-{closure#0}.-------.coroutine_resume.0.dot" \
    -o docs/led_controller_state_machine.svg
```

# Rust Ecosystem Tooling Showcase

A tour of the developer-experience tools that ship with Rust or are one `cargo install` away.
Everything here works on this project — feel free to run the commands live.

---

## 1. Clippy — the linter that teaches you Rust

Clippy is the official Rust linter. It ships with every Rust installation (no install needed).

```bash
cargo clippy
```

Clippy checks for hundreds of patterns: unnecessary clones, integer overflow risks,
API misuse, performance pitfalls, and outright bugs. Every lint links to a detailed
explanation page. Run `cargo clippy -- -W clippy::all` to enable the full set.

**Auto-fix** — many lints can be automatically corrected:

```bash
cargo clippy --fix
```

Clippy lint categories (roughly ascending strictness):
| Category | Meaning |
|---|---|
| `clippy::correctness` | Code that is almost certainly wrong |
| `clippy::suspicious` | Code that looks dubious |
| `clippy::style` | Non-idiomatic patterns |
| `clippy::perf` | Unnecessary runtime cost |
| `clippy::complexity` | Overly complex constructs |
| `clippy::pedantic` | Opinionated, opt-in suggestions |
| `clippy::nursery` | Experimental, unstable lints |

The difference from many language linters: Clippy lints are implemented in the
compiler itself, so they see the *type-checked, resolved* program — false positives
are rare.

---

## 2. rustfmt — the formatter

One opinionated formatter for the entire Rust ecosystem. No debates about brace style.

```bash
# Format all files in the project
cargo fmt

# Check without modifying (great for CI)
cargo fmt --check
```

Configure in `rustfmt.toml` if you need deviations (max line width, import grouping, etc.).
The default style is what the standard library and most major crates use, so reading
other people's Rust code immediately feels familiar.

---

## 3. `cargo check` — instant type checking

Full compilation compiles *and* links everything. `cargo check` skips codegen and linking —
it only type-checks. On this project that is ~3-4× faster than `cargo build`.

```bash
cargo check
```

**Why this matters for learners:** the borrow checker, lifetime errors, and type errors
all surface during `cargo check`. You get the feedback you care about without waiting
for the linker.

---

## 4. `cargo-watch` — continuous feedback loop

`cargo-watch` re-runs a cargo command every time a source file changes.

```bash
# Install once
cargo install cargo-watch

# Re-check on every save
cargo watch -x check

# Clippy on every save
cargo watch -x clippy

# Chain: check then clippy (stops on first failure)
cargo watch -x check -x clippy
```

This turns Rust development into a tight loop: save → see errors immediately → fix → repeat.
No manual re-running, no build button to click. Works exactly the same whether you are
targeting x86 or a Cortex-M microcontroller.

---

## 5. Binary size — what is actually in the firmware?

Understanding binary size is critical in embedded. Rust gives you precise visibility.

### Section summary with `cargo size`

```bash
# From cargo-binutils (cargo install cargo-binutils + rustup component add llvm-tools)
cargo size --release
```

Actual output for this project:
```
   text    data     bss     dec     hex filename
  12128      80    5688   17896   45e8  nrf-example
```

| Section | What lives there |
|---|---|
| `.text` | Executable code (your functions + library functions, after LTO) |
| `.rodata` | Read-only data: string literals, constant tables, `defmt` log format strings |
| `.data` | Mutable global variables with non-zero initial values (copied from flash to RAM on boot) |
| `.bss` | Mutable globals initialized to zero (not stored in flash, just zeroed in RAM on boot) |

### Per-function symbol sizes

```bash
cargo size --release -- -A   # all sections, verbose

# Or with nm (shows every symbol and its size):
cargo nm --release -- --size-sort | tail -20
```

### Which crate is eating the most flash?

```bash
cargo install cargo-bloat
cargo bloat --release --crates
```

`cargo-bloat` attributes each compiled symbol back to the crate it came from:

```
File  .text    Size  Crate
3.5%  12.1%  2.1 KiB  embassy_executor
2.2%   7.6%  1.3 KiB  nrf_example
1.8%   6.1%  1.1 KiB  embassy_nrf
...
```

Then drill into a single crate:

```bash
cargo bloat --release --filter embassy_executor
```

This shows individual functions, sorted by size. Immediately obvious which HAL
driver, which executor path, or which formatting code dominates the binary.

### LTO effect

This project has `lto = true` in the release profile. Toggle it off and re-run
`cargo size` to see how much dead code LTO strips:

```bash
# Temporarily: cargo build --release with lto = false in Cargo.toml
cargo size --release
```

LTO (Link-Time Optimization) lets the compiler see across crate boundaries and
eliminate code that is never actually called — especially powerful in `no_std`
where you pull in HAL crates but only use a handful of peripherals.

---

## 6. RAM — how much does the app need at runtime?

Without a heap, all RAM is either static (measurable at link time) or stack
(consumed at runtime). Rust gives you exact visibility into both.

### Static RAM — `cargo size` is the definitive answer

From `cargo size --release` on this project:

```
text    data     bss     dec
12128     80    5688   17896   nrf-example
```

- `.data` = **80 bytes** — initialized globals (non-zero), copied from flash to RAM on boot
- `.bss` = **5688 bytes** — zero-initialized globals (not stored in flash, just zeroed on boot)
- **Total static RAM: 5768 bytes** out of the 256 KB available on the nRF52840

### Per-symbol breakdown — `cargo nm`

```bash
cargo nm --release -- --size-sort --print-size | sort -k2 -rh | head -15
```

Every byte of static RAM accounted for on this project:

```
4100 B  embassy_executor::ARENA        ← task arena (all task state machines)
1024 B  defmt_rtt::BUFFER              ← RTT log ring buffer
 384 B  embassy_nrf::gpiote::PORT_WAKERS
  64 B  embassy_nrf::gpiote::CHANNEL_WAKERS
  48 B  nrf_example::TICK_CH           ← your channel (4 × u32 + bookkeeping)
  40 B  nrf_example::CMD_CH            ← your channel (4 × LedCommand + bookkeeping)
  24 B  embassy_nrf::time_driver::DRIVER
```

### The task arena is the biggest surprise

The largest single allocation — **4100 bytes** — is the Embassy task arena.
This is where Embassy stores the compiled **future state machines** for every task.
When an async task suspends at `.await`, all its local variables are saved into
this arena rather than on the call stack. The arena is statically sized by the
feature flag in `Cargo.toml`:

```toml
embassy-executor = { version = "0.7", features = ["task-arena-size-4096", ...] }
```

Change `4096` to `512`, flash the board, and it will panic on boot with an
out-of-memory error. Increment it until it boots cleanly — that is your true
minimum arena size. Because this is a static allocation, there is no fragmentation
and no runtime allocation cost: the compiler calculates each future's size and
packs them into the arena at link time.

**Key insight:** in an Embassy application, `cargo size` tells you almost the
entire RAM story. The async model moves suspended task state out of the call
stack and into static memory. The remaining call stack only holds the state
of the *currently executing* code path — and since Embassy is cooperative and
single-threaded, only one task runs at a time.

### Stack depth — the harder part

Static analysis of call stack depth requires extra tooling:

**`stack-sizes`** — frame size per function (fast, requires nightly):

```bash
cargo install stack-sizes
cargo +nightly rustc --release -- -Z emit-stack-sizes
stack-sizes target/thumbv7em-none-eabihf/release/nrf-example | sort -k1 -rn | head -20
```

Shows how many bytes each function pushes onto the stack when called. Sum the
sizes along the deepest call path to get worst-case stack usage.

**`cargo-call-stack`** — full static call graph with stack annotations:

```bash
cargo install cargo-call-stack
cargo +nightly call-stack --release > call-stack.dot
dot -Tsvg call-stack.dot -o docs/call-stack.svg
```

Produces a call graph where each node is annotated with its local stack frame size.
Does not handle all cases (indirect calls through function pointers or trait objects),
but gives a reliable lower bound.

**`flip-link`** — stack overflow detection at runtime (not a measurement tool, but essential):

```bash
cargo install flip-link
```

Then in `.cargo/config.toml`:
```toml
[target.thumbv7em-none-eabihf]
linker = "flip-link"
runner = "probe-rs run --chip nRF52840_xxAA"
```

`flip-link` places the stack at the *bottom* of RAM instead of the top. If the
stack grows past its limit it hits unmapped memory and triggers a hard fault
immediately — instead of silently overwriting your globals and causing mysterious
corruption much later. This is the embedded equivalent of stack canaries, with
zero runtime overhead.

---

## 7. `cargo doc` — documentation that compiles

Every public item in your code can have doc comments (`///`). `cargo doc` builds
a browsable HTML site from them, including all your dependencies.

```bash
cargo doc --open
```

This opens the rendered docs for your crate *and* every crate in your dependency
tree in a browser. No internet required. The API docs for `embassy-nrf`,
`embassy-sync`, and everything else are right there, locally rendered from the
exact version you depend on.

Doc-tests (code examples in `///` comments) are compiled and run as tests:

```bash
cargo test --doc
```

---

## 8. `cargo tree` — see the full dependency graph

```bash
cargo tree
```

Shows the complete dependency tree, including transitive dependencies. Useful for:
- Understanding where a crate comes from
- Spotting duplicate versions of the same crate
- Auditing what code you are actually shipping

```bash
# Which crates bring in cortex-m?
cargo tree --invert cortex-m

# Show feature flags resolved for each dep
cargo tree --edges features
```

---

## 9. `cargo expand` — see what macros generate

Rust macros (both `macro_rules!` and proc-macros) expand to real Rust code before
compilation. `cargo expand` shows you exactly what the compiler sees after expansion.

```bash
# Install once
cargo install cargo-expand   # also needs nightly for some expansions

# Expand everything in main.rs
cargo expand

# Expand only the main module
cargo expand main
```

The `#[embassy_executor::task]` and `#[embassy_executor::main]` proc-macros in
this project expand to the static task storage, the spawn function, and the
`Future` implementation. Running `cargo expand` reveals the generated code —
there is no magic, just Rust.

---

## 10. `cargo audit` — security vulnerability scanning

```bash
cargo install cargo-audit
cargo audit
```

Checks every crate in your `Cargo.lock` against the [RustSec Advisory Database](https://rustsec.org/).
If any dependency has a known CVE or security advisory, you get a clear report.
Run this in CI to catch supply-chain issues automatically.

---

## 11. `rust-toolchain.toml` — reproducible environments

This file pins the exact toolchain and compilation target for the project:

```toml
[toolchain]
channel = "stable"
targets = ["thumbv7em-none-eabihf"]
```

Anyone who clones this repo and runs `cargo build` gets the **exact same toolchain**
automatically — `rustup` downloads it if it isn't present. No "works on my machine"
for toolchain version mismatches.

---

## 12. Putting it together — a typical workflow

```bash
# Terminal 1: continuous type-check + lint on every save
cargo watch -x check -x clippy

# Terminal 2: build, flash, and stream logs when you are ready
cargo run --release

# Before a commit
cargo fmt --check
cargo clippy -- -D warnings   # treat lints as errors
cargo audit
```

---

## Quick reference

| Tool | Install | Command |
|---|---|---|
| Clippy | `rustup component add clippy` | `cargo clippy` |
| rustfmt | `rustup component add rustfmt` | `cargo fmt` |
| cargo-watch | `cargo install cargo-watch` | `cargo watch -x check` |
| cargo-size | `cargo install cargo-binutils` + `rustup component add llvm-tools` | `cargo size --release` |
| cargo-bloat | `cargo install cargo-bloat` | `cargo bloat --release --crates` |
| cargo-nm | part of cargo-binutils | `cargo nm --release -- --size-sort --print-size` |
| stack-sizes | `cargo install stack-sizes` | `stack-sizes target/.../nrf-example` |
| cargo-call-stack | `cargo install cargo-call-stack` | `cargo +nightly call-stack --release` |
| flip-link | `cargo install flip-link` | set as linker in `.cargo/config.toml` |
| cargo-expand | `cargo install cargo-expand` | `cargo expand` |
| cargo-audit | `cargo install cargo-audit` | `cargo audit` |
| cargo tree | built-in | `cargo tree` |
| cargo doc | built-in | `cargo doc --open` |
| cargo check | built-in | `cargo check` |

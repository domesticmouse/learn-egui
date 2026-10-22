---
name: craft-testing-infrastructure
description: >-
  Use this skill when running, writing, debugging, or auditing tests, benchmarks,
  corpora, fuzzers, or CI gates in PhotoCraft: fast nextest loops, GPU test
  serialization (gpu_lock), real-file corpus oracles and floor ratchets,
  panic_hunt adversarial command fuzzing, offscreen UI snapshots, MCP acceptance
  tests, performance budgets (xtask perf), and scorecard generation (xtask scorecard).
---

# PhotoCraft Testing, Corpus, Performance & Scorecard Infrastructure

PhotoCraft enforces quality across ~629k LoC and ~9,900 `#[test]` functions through a 7-tier verification architecture. Use this skill whenever you write tests, run verification gates, update corpus/parity floors, benchmark hot paths, or update `docs/scorecard.md`.

---

## 1. The 7 Verification Tiers

| Tier | Scope & Mechanism | Primary Files & Commands |
|---|---|---|
| **1. Compile-Time & Static Gates** | `L0`–`L7` acyclic layering (`cargo xtask layers`), `wasm32` check (`cargo xtask wasm`), `unsafe_code = "forbid"` (except `crates/tablet`), `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, ...)]` on all 30 crates (relaxed in tests via `clippy.toml`), and exhaustive struct destructuring in `crates/format/src/convert.rs`. | `xtask/src/layers.rs`, `clippy.toml`, `crates/format/src/convert.rs` |
| **2. Unit, Property & Format Fuzz Tests** | Multi-depth (`U8`, `U16`, `F32`) and multi-color-mode tests, 15 `proptest!` suites (COW tiles, regions, codecs, PSD), deterministic synthetic PSD/PSB generation (`psd::testgen`), truncation/corruption sweeps, and 20 `cargo-fuzz` targets (`psd`, `codecs`, `affinity`, `format`). | `crates/psd/src/testgen.rs`, `crates/*/fuzz/` |
| **3. CPU vs. GPU Compositor Parity** | `photocraft-compose` is the CPU reference oracle; `photocraft-gpu` asserts WGSL shader output within $\le 1/255$ of `compose`. Every test using a `wgpu` device or `egui_kittest` GPU harness **must** hold `let _gpu = photocraft_testkit::gpu_lock();` on its first line. | `crates/testkit/src/gpu.rs`, `.config/nextest.toml` |
| **4. Pinned Real-File Oracles (`corpus`)** | Gitignored real files in `corpus/` fetched at pinned commits and verified against `xtask/*.sha256` manifests (`photoshop`, `gimp`, `psd`, `psd-tools`, `heif`, `exr`, `affinity`, `atn`, `pngsuite`). Asserts monotonically increasing oracle pass ($\le 2/255$) and export round-trip ($\le 1/255$) floors, strict tagged-block padding, and mutation safety. | `xtask/src/corpus_pins.rs`, `crates/io/tests/corpus.rs`, `crates/engine/tests/photoshop_oracles.rs` |
| **5. Command Registry & `panic_hunt`** | `crates/engine/tests/panic_hunt.rs` fuzzes every registered `CommandSpec` against adversarial JSON params on its own thread with a 4 s deadline and a custom `std::panic::set_hook` counter that catches even panics swallowed by `Session::execute`'s `catch_unwind`. | `crates/engine/tests/panic_hunt.rs` |
| **6. Headless UI (`egui_kittest`) & MCP Acceptance** | Widget/canvas tests in `crates/ui-egui`, offscreen wgpu PNG snapshots (`examples/snapshot.rs`), menu parity ratchet (`parity::FLOOR`), and 20 end-to-end agent workflows driven over in-memory `rmcp` transport (`crates/automation/tests/agent_tasks.rs`). | `crates/ui-egui/src/parity.rs`, `crates/ui-egui/examples/snapshot.rs`, `crates/automation/tests/agent_tasks.rs` |
| **7. Performance Budgets & Scorecard** | Release benches report p50/p95/max ms, in-process peak RSS (`photocraft_testkit::perf::RssSampler` + Linux `VmHWM`), and GPU bytes against `perf/budgets.toml` and `perf/baseline.json`. `cargo xtask scorecard` compiles budgets, corpus floors, checklists (`scorecard/*.toml`), and unread-preference audits into `docs/scorecard.md`. | `crates/testkit/src/perf.rs`, `xtask/src/perf.rs`, `xtask/src/scorecard.rs`, `perf/budgets.toml` |

---

## 2. Fast Local Iteration vs. Pre-Finish Gate

### Fast Iteration Loops
Avoid compiling all workspace test binaries and examples while iterating:

```sh
# 1. Run unit + integration tests for a single crate (skips compiling examples and doctests)
cargo test -p <crate> --lib --tests

# 2. Run a single integration test binary
cargo test -p <crate> --test <test_file_stem>

# 3. Run parallel process-per-test execution via nextest (caps concurrent GPU tests at 4)
cargo nextest run -p <crate> --lib --tests

# 4. If working alongside parallel agents, isolate your build directory
CARGO_TARGET_DIR=target/agent-<name> cargo test -p <crate> --lib --tests
```

### Mandatory Pre-Finish Verification Gate
Before finishing a task, run the checks corresponding to the crates you touched:

```sh
# Always run on touched crates:
cargo test -p <crates>
cargo clippy -p <crates> --all-targets -- -D warnings
cargo xtask layers

# If you touched L0–L6:
cargo xtask wasm

# If you added or modified engine commands:
PHOTOCRAFT_NO_PRINT=1 LPDEST=photocraft-no-such-printer \
  cargo test -p photocraft-engine --test panic_hunt -- --ignored
cargo xtask parity

# If you touched psd, io, codecs, heif, compose, gpu, text, format, affinity, or raw:
CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  cargo xtask test-corpus --changed

# If you touched a hot path, fixed a scorecard item, raised a floor, or wired a preference:
cargo xtask perf --quick
cargo xtask scorecard
cargo xtask scorecard --check
```

---

## 3. Critical Rules When Writing Tests

1. **Always Hold `gpu_lock()` in GPU Tests:**
   Multiple concurrent `wgpu` devices in one process crash Windows WARP (`STATUS_ACCESS_VIOLATION`), Mesa `llvmpipe`/RADV, and multi-ICD Vulkan. Under `cargo test` (multi-threaded within one process), every test creating a `wgpu` device or `egui_kittest` GPU harness must acquire `photocraft_testkit::gpu_lock()` on its first line:
   ```rust
   #[test]
   fn my_gpu_test() {
       let _gpu = photocraft_testkit::gpu_lock();
       // build wgpu device or egui_kittest harness...
   }
   ```
   If you add a new GPU test module in `ui-egui`, ensure its name matches the `gpu` test-group filter in `.config/nextest.toml`.
2. **Never Spool to Physical Printers:**
   When testing or fuzzing print commands (`file.print`, `file.printOneCopy`), call `photocraft_engine::print_cmds::disable_printing()`, pass `"dryRun": true`, and set `PHOTOCRAFT_NO_PRINT=1 LPDEST=photocraft-no-such-printer`.
3. **Test Multi-Depth & Multi-Color-Mode Paths:**
   Pixel algorithms and commands must be tested across `8`, `16`, and `32f` bit depths and non-RGB models (`Gray`, `CMYK`, `Lab`) where supported.
4. **Every Crash Fix Requires a Synthetic Regression Test:**
   Never commit corpus files into the repository. Reproduce any corpus or fuzz crash with a small synthetic test in the crate (e.g., `crates/io/tests/corpus_regressions.rs`) that panicked before the fix.
5. **Ratchets Only Move Up:**
   Never lower `parity::FLOOR` in `crates/ui-egui/src/parity.rs` or any `pass_floor` / `roundtrip_floor` in `crates/io/tests/corpus.rs` and `crates/engine/tests/photoshop_oracles.rs`. When your change improves pass counts, raise the floor and regenerate `docs/scorecard.md` via `cargo xtask scorecard`.

---

## 4. Deep-Dive Reference

For detailed workflows on:
- Fetching, authoring, and debugging real-file corpora (`oracle_diff`, `photocraft-corpus` local mode, pin bumps)
- Adding performance scenarios (`photocraft_testkit::perf`, `perf/budgets.toml`, `perf/baseline.json`)
- Updating `scorecard/*.toml` and understanding CI matrix optimizations (`ci.yml`, `perf-nightly.yml`)

Read [corpus-perf-and-ci-reference.md](./references/corpus-perf-and-ci-reference.md).

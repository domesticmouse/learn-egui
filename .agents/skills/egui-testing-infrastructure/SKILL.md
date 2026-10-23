---
name: egui-testing-infrastructure
description: >-
  Use this skill when running, writing, debugging, or auditing tests, mutation
  coverage (cargo-mutants), benchmarks, corpora, fuzzers, or CI gates in an
  engine-first Rust and egui application: fast nextest loops, cargo-mutants
  configuration (.cargo/mutants.toml), GPU test serialization (gpu_lock),
  real-file corpus oracles and floor ratchets, panic_hunt adversarial command
  fuzzing, offscreen UI snapshots (egui_kittest), MCP acceptance tests,
  performance budgets (xtask perf), and scorecard generation (xtask scorecard).
---

# Testing, Corpus, Performance & Scorecard Infrastructure (`egui` & Rust)

An engine-first multi-crate `egui` application enforces quality through a 7-tier verification architecture. Use this skill whenever you write tests, run verification gates, update corpus/parity floors, benchmark hot paths, or update `docs/scorecard.md`.

Related skills:
- [egui-app-architecture](../egui-app-architecture/SKILL.md) — `L0`–`L7` crate layering, `Testkit` rules, and lint/profile configuration.
- [egui-app-runner](../egui-app-runner/SKILL.md) — Running the desktop app, CLI, offscreen snapshots, and live MCP/TCP control servers.
- [egui-command-engine](../egui-command-engine/SKILL.md) — Authoring `CommandSpec`s validated by `panic_hunt` and parity checks.
- [egui-cow-document-model](../egui-cow-document-model/SKILL.md) — Copy-on-write chunks, snapshot undo/redo history, and exhaustive format conversion.

---

## 1. The 7 Verification Tiers

| Tier | Scope & Mechanism | Primary Files & Commands |
|---|---|---|
| **1. Compile-Time & Static Gates** | `L0`–`L7` acyclic layering (`cargo xtask layers`), `wasm32` check (`cargo xtask wasm`), `unsafe_code = "forbid"`, `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, ...)]` across all workspace crates (relaxed in tests via `clippy.toml`), and exhaustive struct destructuring in `crates/format/src/convert.rs`. | `xtask/src/layers.rs`, `clippy.toml`, `crates/format/src/convert.rs` |
| **2. Unit, Property, Mutation & Format Fuzz Tests** | Domain unit tests across supported data representations, `cargo-mutants` mutation coverage (`0 missed mutants`), `proptest!` suites (COW chunks/tiles, spatial regions, codecs, file formats), deterministic synthetic document generators (`testgen`), truncation/corruption sweeps, and `cargo-fuzz` targets (`crates/*/fuzz/`). | `crates/*/src/`, `.cargo/mutants.toml`, `crates/*/fuzz/` |
| **3. CPU vs. GPU Rendering Parity** | CPU reference renderer vs. `wgpu` WGSL shader output parity within a bounded tolerance. Every test using a `wgpu` device or `egui_kittest` GPU harness **must** hold `let _gpu = testkit::gpu_lock();` on its first line. | `crates/testkit/src/gpu.rs`, `.config/nextest.toml` |
| **4. Pinned Real-File Oracles (`corpus`)** | Gitignored real-world files in `corpus/` fetched at pinned commits and verified against `xtask/*.sha256` manifests. Asserts monotonically increasing oracle pass and export round-trip floors, strict structural validation, and mutation safety. | `xtask/src/corpus_pins.rs`, `crates/io/tests/corpus.rs` |
| **5. Command Registry & `panic_hunt`** | `crates/engine/tests/panic_hunt.rs` fuzzes every registered `CommandSpec` against adversarial JSON params on its own thread with a per-command deadline (e.g., 4 s) and a custom `std::panic::set_hook` counter that catches even panics swallowed by `Session::execute`'s `catch_unwind`. | `crates/engine/tests/panic_hunt.rs` |
| **6. Headless UI (`egui_kittest`) & MCP Acceptance** | Widget/canvas tests in `crates/ui-egui`, offscreen `wgpu` PNG snapshots (`examples/snapshot.rs`), menu parity ratchet (`parity::FLOOR`), and end-to-end agent workflows driven over in-memory `rmcp` transport (`crates/automation/tests/agent_tasks.rs`). | `crates/ui-egui/src/parity.rs`, `crates/ui-egui/examples/snapshot.rs`, `crates/automation/tests/agent_tasks.rs` |
| **7. Performance Budgets & Scorecard** | Release benches report p50/p95/max ms, in-process peak RSS (`testkit::perf::RssSampler` + Linux `VmHWM`), and GPU bytes against `perf/budgets.toml` and `perf/baseline.json`. `cargo xtask scorecard` compiles budgets, corpus floors, checklists (`scorecard/*.toml`), and unread-preference audits into `docs/scorecard.md`. | `crates/testkit/src/perf.rs`, `xtask/src/perf.rs`, `xtask/src/scorecard.rs`, `perf/budgets.toml` |

---

## 2. Fast Local Iteration vs. Pre-Finish Gate

### Fast Iteration Loops
Avoid compiling all workspace test binaries and examples while iterating:

```sh
# 1. Run unit + integration tests for a single crate (skips compiling examples and doctests)
cargo test -p <crate> --lib --tests

# 2. Run a single integration test binary
cargo test -p <crate> --test <test_file_stem>

# 3. Run parallel process-per-test execution via nextest (caps concurrent GPU tests)
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
cargo test -p <app>-engine --test panic_hunt -- --ignored
cargo xtask parity

# If you touched format parsers, io, rendering, or persistence crates:
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
   Multiple concurrent `wgpu` devices in one process crash Windows WARP (`STATUS_ACCESS_VIOLATION`), Mesa `llvmpipe`/RADV, and multi-ICD Vulkan. Under `cargo test` (multi-threaded within one process), every test creating a `wgpu` device or `egui_kittest` GPU harness must acquire `testkit::gpu_lock()` on its first line:
   ```rust
   #[test]
   fn my_gpu_test() {
       let _gpu = testkit::gpu_lock();
       // build wgpu device or egui_kittest harness...
   }
   ```
   If you add a new GPU test module in `ui-egui`, ensure its name matches the `gpu` test-group filter in `.config/nextest.toml`.
2. **Isolate OS Side Effects in Tests & Fuzzers:**
   Commands that trigger external OS side effects (such as physical printing, system dialogs, or network/external process spawns) must be stubbed or disabled in dry-run/fuzz mode (e.g., calling a `disable_*()` hook or setting a `NO_*=1` environment guard) before running `panic_hunt`.
3. **Test Full Domain Representations & Edge Cases:**
   Domain algorithms and commands should be tested across all supported precision modes, coordinate spaces, and entity variants where applicable.
4. **Every Crash Fix Requires a Synthetic Regression Test:**
   Never commit large external corpus files into the repository. Reproduce any corpus or fuzz crash with a small, deterministic synthetic test in the crate (e.g., `crates/io/tests/corpus_regressions.rs`) that failed or panicked before the fix.
5. **Ratchets Only Move Up:**
   Never lower `parity::FLOOR` in `crates/ui-egui/src/parity.rs` or any `pass_floor` / `roundtrip_floor` in `crates/io/tests/corpus.rs`. When your change improves pass counts, raise the floor and regenerate `docs/scorecard.md` via `cargo xtask scorecard`.
6. **Mutation Testing (`cargo-mutants`) & Headless `egui_kittest` Idioms:**
   - **Prerequisite Installation:** Ensure `cargo-mutants` is installed (`cargo install --locked cargo-mutants`) and documented whenever onboarding or writing verification instructions.
   - **Sandboxed Offline Execution on macOS (`CARGO_HOME` & `TMPDIR` in Brain Scratch):** On macOS, `/tmp` is a symlink to `/private/tmp` and `~/.cargo` is outside the workspace, causing `std::fs::canonicalize` in `cargo` and `cargo-mutants` to fail with `Operation not permitted` inside Standard Sandbox Mode (`BypassSandbox: false`). Seed `~/.cargo/registry` once via APFS copy-on-write into `<appDataDir>/brain/<conversation-id>/scratch/cargo-home/registry`, and run with `CARGO_HOME` and `TMPDIR` pointed at `<scratch>` plus `--in-place --timeout 120`:
     ```sh
     CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp \
       cargo mutants --in-place --timeout 120 -d <crate_dir> -o <scratch>/mutants-<name> -C --offline
     ```
   - **Sequential `cargo mutants` Runs & `--timeout 120`:** On a warm `target/` cache, the unmutated baseline test finishes in `< 2s`, causing `cargo-mutants` to auto-set a `20s` test timeout. Running multiple `cargo test` or `cargo mutants` jobs concurrently causes linker/lock contention that triggers false `TIMEOUT` results. Always run `cargo mutants --in-place` sequentially (one crate at a time after all `cargo test` builds finish) with `--timeout 120`.
   - **APFS Copy-on-Write (`cp -cR`) Target Seeding with Matching `CARGO_HOME`:** Cargo hashes the `CARGO_HOME` path into `target/debug/.fingerprint/`. Always build the seed crate's `target/` directory using the exact `CARGO_HOME` path that downstream crates will use, wait for the seed build to finish completely before running `cp -cR <prev>/target <next>/target`, copy the resolved `Cargo.lock` whenever dependencies change, and **always run `touch <next>/src/main.rs`** before `cargo test` so Cargo recompiles the new crate's test binary.
   - **Headless `eframe::App` Verification via `egui_kittest::Harness::new_eframe`:** Add `egui_kittest = { version = "0.33", features = ["eframe"] }` to `[dev-dependencies]` and boot the full app headlessly via `Harness::new_eframe(|_cc| MyApp::default())`. Once `Harness::new_eframe` tests query rendered labels (`harness.get_by_label(...)`), simulate clicks (`.click()`), and press keys (`harness.key_press(egui::Key::...)`), **remove `"<impl eframe::App for MyApp>::update"` from `.cargo/mutants.toml` `exclude_re`** so `cargo-mutants` verifies `update()` and panel UI methods directly, leaving only pure pixel-coordinate `Painter` helpers (`render_canvas`, `paint_board`) excluded until offscreen GPU snapshot tests come online.
   - **`egui::Scene::show` Closure Borrow Split (`E0500` Prevention):** Passing `&mut self.view.pcb_scene_rect` directly to `egui::Scene::new().show(ui, &mut self.view.pcb_scene_rect, |ui| { ... })` borrows `self.view.pcb_scene_rect` for the entire `.show(...)` call, preventing the inner closure from calling methods on `self`. Because `egui::Rect` is `Copy`, copy `let mut scene_rect = self.view.pcb_scene_rect;` before `Scene::show`, pass `&mut scene_rect`, and assign `self.view.pcb_scene_rect = scene_rect;` afterward.
   - **Configure `.cargo/mutants.toml` with Exact `--list` Mutant Names:** `exclude_re` matches the generated mutant description from `cargo mutants --list`, *not* raw Rust syntax:
     - To exclude `fn main()`, match `"replace main"` (not `"fn main"`).
     - To exclude a trait method like `eframe::App::update` (before `egui_kittest` is introduced), match `"<impl eframe::App for MyApp>::update"` (not `"impl eframe::App for MyApp::update"`).
     - Set `copy_target = true` in `.cargo/mutants.toml` and gitignore `mutants.out/` and `mutants.out.old/`.
   - **Avoid `..Default::default()` Inside Untested Entrypoints (`fn main`):** `cargo-mutants` (v27.1+) generates `StructField` (`delete field <field> from struct <Struct> expression`) mutants on struct literals using `..Default::default()`, and `StructField` mutants bypass `exclude_re` filtering. In `fn main()`, initialize `eframe::NativeOptions` via `let mut options = eframe::NativeOptions::default(); options.viewport = ...;` rather than struct update syntax so `main()` does not produce an un-excludable missed mutant.
   - **Delegate `impl Default` / `fn new` Logic to Named Methods:** `cargo-mutants` skips `impl Default` (`fn default`) and `fn new` completely to avoid infinite recursion with `Default::default()`. Put non-trivial initialization or state-reset logic in a named method (e.g., `pub fn reset_layers(&mut self)`) called by `Default::default()` so `cargo-mutants` can mutate and verify it.
   - **Designing Arithmetic, Boolean & Match-Arm Tests for Zero Missed Mutants:**
     - *Arithmetic (`Pos2`, `Vec2`, `Rect`, `TSTransform`, `hypot`):* Never test at `0.0` or `1.0` coordinates/factors (where `0.0 - x == 0.0 / x` or `x * 1.0 == x / 1.0`). Use non-zero, asymmetric values (e.g., `pos2(100.0, 60.0)`, `vec2(16.0, 10.0)`, `zoom_factor = 2.0`, and `3-4-5` right triangles at non-zero offsets) so every `+`/`-`/`*`/`/`/`%` mutant produces a distinct float caught by `assert_eq!`.
     - *Compound Booleans (`&&` vs. `||` and `!`):* For `&&` predicates like `self.is_layer_visible(p.layer) && p.contains_world_point(world_mm)`, include negative tests for both `(true, false)` and `(false, true)`. For multi-branch `||` search filters (e.g., `matches_query` checking `ref_des || value || package || signal_name`), include one query that matches *exclusively* via each branch plus a non-matching query so every `|| -> &&` and `!` negation mutant is caught.
     - *Exhaustive `match` Arms:* Assert each enum variant's behavior in isolation so no `match` branch in `z_rank()`, `color()`, `signal_name()`, or `is_layer_visible()` can be deleted or collapsed.
   - **Rust 1.97+ `float_literal_f32_fallback` Warning with `egui::Stroke::new`:** Because `Stroke::new` takes generic `impl Into<f32>`, untyped float literals like `Stroke::new(2.0, color)` trigger `#[warn(float_literal_f32_fallback)]` on `rustc 1.97+`. Use explicit `_f32` suffixes (`Stroke::new(2.0_f32, color)`) or typed `f32` variables.

---

## 4. Deep-Dive Reference

For detailed workflows on:
- Fetching, authoring, and debugging real-file corpora (`oracle_diff`, local corpus testing, pin bumps)
- Adding performance scenarios (`testkit::perf`, `perf/budgets.toml`, `perf/baseline.json`)
- Updating `scorecard/*.toml` and understanding CI matrix optimizations (`ci.yml`, `perf-nightly.yml`)

Read [corpus-perf-and-ci-reference.md](./references/corpus-perf-and-ci-reference.md).

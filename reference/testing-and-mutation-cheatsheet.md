---
title: "Reference: Continuous Verification & Mutation Testing (`cargo-mutants`)"
subtitle: How we verify every PCB Reviewer feature with `cargo test` and measure real assertion coverage with `cargo mutants` across Acts I–VI.
page_type: reference
---

## 1. Why Mutation Testing Beats Line Coverage in `egui`

Traditional line coverage only tells you whether a line of Rust code _executed_ during a test—not whether your test would fail if that line were wrong. Turns out, in an immediate-mode GUI, calling `app.update(...)` executes almost every line of your panel layout even if your test contains zero assertions!

So, we use Martin Pool's [`cargo-mutants`](https://mutants.rs/) to test our tests: it copies your project to a scratch directory, systematically injects small faults (**mutants**—such as flipping `true` to `false`, replacing `+` with `-`, or emptying a method body), and runs `cargo test` against each mutant.

```mermaid
flowchart LR
    Source["Rust Source Code<br/>(e.g. isolate_top_copper)"] --> Mutate["cargo mutants injects fault<br/>(e.g. show_top_copper = false)"]
    Mutate --> Run["Runs cargo test<br/>in scratch build dir"]
    Run -->|"Test FAILS"| Caught["CAUGHT MUTANT<br/>(Your test assertion works!)"]
    Run -->|"Test PASSES"| Missed["MISSED MUTANT<br/>(Coverage gap: add assertion!)"]
```
_How `cargo mutants` distinguishes verified logic (Caught) from unasserted code paths (Missed)._

## 2. Essential Commands & Outcomes

```bash
# 1. Install cargo-mutants once (works with stable Rust, no nightly required)
cargo install --locked cargo-mutants

# 2. Run the unit & integration test suite
cargo test

# 3. Run mutation testing across the current crate and list any missed mutants
cargo mutants

# 4. Mutate only a single file or function during fast iteration
cargo mutants -f src/main.rs
```

| Outcome | Meaning | Action to Take |
| --- | --- | --- |
| **Caught** | `cargo test` failed when this mutation was injected. Your test suite actively guards this line of code. | Celebrate—this behaviour is locked down. |
| **Missed** | `cargo test` still passed even with the mutated code! Either no test exercises this path or an assertion is missing. | Add a unit/harness assertion that fails under the mutated value. |
| **Unviable** | The mutated code failed to type-check or compile (Rust's type system caught it before tests even ran). | Ignored automatically by `cargo mutants`. |
| **Timeout** | The mutation caused an infinite loop in a test. | Inspect loop termination bounds. |

## 3. Scoping Visual Entrypoints with `.cargo/mutants.toml`

In a desktop application, `fn main()` opens a blocking OS window via `eframe::run_native`, which should not run inside headless unit tests. Create `.cargo/mutants.toml` in your crate root to explicitly track which functions are excluded from mutation until a higher-tier harness is introduced:

```toml
# .cargo/mutants.toml
# Reuse the target directory across scratch builds so mutation runs finish quickly.
copy_target = true

# Explicitly exclude the OS window launcher and raw panel draw loop in Act I;
# we remove "update" from this list in Lesson 0006 when we add egui_kittest!
exclude_re = [
    "replace main",
    "<impl eframe::App for PcbReviewerApp>::update",
]
```

## 4. Act IV Multi-Crate Workspace & `proptest` Patterns (`0014`–`0017`)

Once domain state lives in `crates/pcb-doc` (`L1`, depending only on `emath`), you can run `cargo test` and `cargo mutants` on the domain crate in milliseconds without compiling or linking `eframe`:

```bash
# Run only L1 domain tests or mutations (zero egui/eframe linking overhead)
cargo test --manifest-path ./apps/0017-undo-redo-history-and-slider-coalescing/Cargo.toml -p pcb-doc
cargo mutants --manifest-path ./apps/0017-undo-redo-history-and-slider-coalescing/Cargo.toml -p pcb-doc --in-place

# Filter cargo-mutants to specific functions across the workspace
cargo mutants --manifest-path ./apps/0017-undo-redo-history-and-slider-coalescing/Cargo.toml --workspace --in-place -F "BoardHistory|record_edit|undo|redo"
```

Combine `proptest!` with `Arc::ptr_eq` to assert both value equivalence and copy-on-write pointer sharing across randomized drag sequences:

```rust
proptest! {
    #[test]
    fn prop_coalesced_drag_restores_exact_initial_state_in_single_undo(
        drag_steps in prop::collection::vec(10.0_f32..90.0_f32, 2..12),
    ) {
        let initial = BoardModel::default();
        let mut working = initial.clone();
        let mut history = BoardHistory::default();

        for x in &drag_steps {
            let before = working.clone();
            working.move_review_pin_x(PinId(1), *x);
            history.record_edit("Drag Pin #1 X", before, Some("drag_pin_1_x"));
        }
        history.commit_coalesce();

        prop_assert_eq!(history.undo_len(), 1);
        let undone_label = history.undo(&mut working);
        prop_assert_eq!(undone_label.as_deref(), Some("Drag Pin #1 X"));
        prop_assert_eq!(&working, &initial);
        prop_assert_eq!(working.shared_arc_buffers(&initial), 4);
    }
}
```

## 5. Act V `panic_hunt`, `gpu_lock()` Snapshot Digests & Scorecards (`0018`–`0021`)

In Act V, we harden the multi-crate workspace with three production gates:

1. **Tier 5 `panic_hunt` Adversarial Command Fuzzing (`0018`)**: Iterates over every registered `CommandSpec` in `COMMAND_SPECS` (asserting `COMMAND_SPECS.len() >= COMMAND_PARITY_FLOOR`) and feeds malformed/out-of-bounds JSON payloads into `BoardModel::execute_command`, asserting `Err(EngineError::BadParams)` and zero panics.
2. **Tier 2 + Tier 7 Spatial Culling Equivalence & Budgets (`0019`)**: Uses `proptest!` to prove `culled_pads_in_rect` and `culled_traces_in_rect` match reference filtering across arbitrary camera viewports while `check_cull_budget` enforces microsecond budgets.
3. **Tier 3/6 `gpu_lock()` & `FrameSnapshotDigest` (`0021`)**: Acquires `let _gpu = gpu_lock();` (`OnceLock<Mutex<()>>` with `PoisonError::into_inner`) on the first line of snapshot tests, computes a deterministic 64-bit FNV-1a digest over visible layer bitmasks and culled geometry counts, and verifies release readiness via `QualityScorecard::is_release_ready()`.

## 6. Progression of Testing Across the Six Acts

| Curriculum Stage | Testing Mechanism Introduced | What `cargo test` + `cargo mutants` Verifies |
| --- | --- | --- |
| **Lessons 0000–0001 & Act I (0002–0005)** | `#[cfg(test)]`, `#[test]`, `assert_eq!`, pure domain/maths methods | Default layer visibility flags, preset state transitions, `Rect` bounds, `TSTransform` pan/zoom round-trips, layer Z-ordering, and net hit-testing. |
| **Act II (0006–0009)** | `egui_kittest::Harness` (headless UI queries & simulated key presses) | Un-skips `eframe::App::update`! Verifies rendered schematic labels, dual-viewport cross-probing, BOM filtering, and `F` / `M` / `Esc` hotkeys. |
| **Act III (0010–0013)** | Workflow state-machine tests & `Harness::new_ui` widget mounts | Review pin CRUD in `HashMap`, `Open` → `Resolved` → `Verified` triage rules, DRC clearance bounds, and custom `egui::Widget` layer swatches. |
| **Act IV (0014–0017)** | Workspace layer gates, panic-free lints & `proptest!` | `#![deny(clippy::unwrap_used)]`, sub-second `cargo mutants -p pcb-doc`, corrupt JSON rejection, and `Arc::make_mut` undo/redo snapshot invariants. |
| **Act V (0018–0021)** | `panic_hunt` command fuzzer, perf budgets & `gpu_lock()` snapshots | Adversarial JSON fuzzing of every `CommandSpec`, spatial culling equivalence, background worker cancellation, and offscreen `wgpu` PNG visual parity. |
| **Act VI (0022–0025)** | Real-file KiCad corpus oracles & `#[tokio::test]` SQL harnesses | `nom` S-expression parser edge cases, pinned `.kicad_pcb` oracle pass/round-trip ratchets, Turso (`libsql`) transactions, and `docs/scorecard.md`. |

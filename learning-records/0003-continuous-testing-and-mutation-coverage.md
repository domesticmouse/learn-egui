# Continuous Cargo Verification & Mutation Testing Across All Six Acts

The learner requested that testing be integrated gradually across the entire curriculum story (`0000`–`0025`) rather than deferred to a single module near the end (`0021`), with **every lesson** containing explicit Cargo verification prompts (`cargo test`) paired with **mutation testing (`cargo-mutants`)** to monitor real assertion coverage (`Caught` vs. `Missed` mutants) as the Schematic & PCB Design Review System grows. Cross-links to [[MISSION.md]] and supersedes the deferred-testing framing in [[0002-five-act-curriculum-arc.md]].

## Implications
- Every lesson (`0000`–`0025`) must include a hands-on **Cargo Verification & Mutation Coverage** lab/checklist running `cargo test` and `cargo mutants`, tracking caught vs. missed mutants and showing how to eliminate surviving mutants with tighter assertions.
- Scale the verification techniques progressively across the six Acts toward the 7-tier verification architecture:
  1. **Lessons `0000`–`0001` & Act I (`0002`–`0005`)**: Pure Rust `#[cfg(test)]` unit tests on default state, state-transition methods, coordinate transforms (`TSTransform`), layer Z-ordering, and hit-testing, paired with `.cargo/mutants.toml` (skipping only the OS window entry point `fn main`).
  2. **Acts II–III (`0006`–`0013`)**: Headless UI and custom `egui::Widget` testing via `egui_kittest::Harness`, allowing us to remove UI mutation exclusions and catch mutated widget labels, hotkeys, and review-status workflows.
  3. **Act IV (`0014`–`0017`)**: Multi-crate static gates (`#![deny(clippy::unwrap_used)]`, layer checks), fast targeted `cargo mutants -p pcb-doc`, corrupt JSON error tests, and `proptest!` property-based suites on `Arc::make_mut` copy-on-write undo/redo history.
  4. **Act V (`0018`–`0021`)**: `panic_hunt` adversarial command-registry fuzzing, spatial-culling equivalence + frame-time performance budgets, background-thread cancellation tests, and `testkit::gpu_lock()` offscreen `wgpu` visual snapshot parity + Wasm gate.
  5. **Act VI (`0022`–`0025`)**: `nom` S-expression fuzz/edge-case tests, pinned real-file KiCad corpus oracle ratchets (`pass_floor` / `roundtrip_floor`), async `#[tokio::test]` Turso (`libsql`) SQL transaction/migration tests, and a self-auditing `docs/scorecard.md`.

# Learning Record 0008: Act V (`0018`–`0021`) — Command Engine, Performance & Polish Implemented

**Date**: 2026-10-25
**Status**: Completed & Verified (`cargo test --workspace` + `cargo mutants --workspace --in-place` + `.venv/bin/python build.py --check`)

## 1. Summary of Completed Work

Implemented **Act V: Command Engine, Performance & Polish (`0018`–`0021`)** of the Rust & `egui` Schematic and PCB Design Review System course, following the `teach`, `egui-command-engine`, `egui-app-runner`, and `egui-testing-infrastructure` skills:

1. **Lesson 0018 (`lessons/0018-the-command-pattern-and-command-palette.md` -> `.html`, `apps/0018-the-command-pattern-and-command-palette`)**:
   - Defined `COMMAND_PARITY_FLOOR = 4`, `EngineError`, `CommandSpec`, `COMMAND_SPECS`, `find_command_spec(id)`, `filter_command_specs(query)`, and `BoardModel::execute_command(&mut self, history: &mut BoardHistory, id: &str, params: &serde_json::Value) -> Result<serde_json::Value, EngineError>` in `crates/pcb-doc` (`L1`).
   - Added `Key::K` hotkey and floating `Command Palette` `egui::Window` in `pcb-reviewer` (`L6`/`L7`), plus `dispatch_ui_command`.
   - Built Tier 5 `panic_hunt` adversarial JSON fuzzing (`command_registry_dispatch_and_panic_hunt_fuzzer_preserve_invariants`), testing malformed, non-object, negative, and out-of-range JSON payloads across all registered `CommandSpec`s.
   - Added interactive browser widget `{{widget:command-palette-sim}}` (`initCommandPaletteSimulator`).
   - **Verification**: `17 / 17` workspace tests (`11` in `pcb-doc`, `6` in `pcb-reviewer`); `238 / 238` viable mutants caught (`0 missed`).

2. **Lesson 0019 (`lessons/0019-viewport-culling-and-cached-tessellation.md` -> `.html`, `apps/0019-viewport-culling-and-cached-tessellation`)**:
   - Implemented stroke-expanded `TraceSegment::bounds_mm()`, `ViewportCullStats` (`culled_count()`, `summary_badge()`), `check_cull_budget(elapsed_micros, max_budget_micros)`, `BoardModel::culled_pads_in_rect`, `BoardModel::culled_traces_in_rect`, and `BoardModel::cull_stats_for_rect` in `crates/pcb-doc`.
   - Wired `enable_viewport_culling` checkbox and live `Cull: X/Y pads, Z/W traces (N skipped)` badge into `pcb-reviewer`, filtering canvas `Painter` submission through `culled_pads_in_rect` and `culled_traces_in_rect`.
   - Added `proptest!` spatial culling equivalence proof (`prop_spatial_culling_matches_reference_and_preserves_counts`).
   - Added interactive browser widget `{{widget:viewport-cull-sim}}` (`initViewportCullSimulator`).
   - **Verification**: `19 / 19` workspace tests (`13` in `pcb-doc`, `6` in `pcb-reviewer`); `255 / 255` viable mutants caught (`0 missed`).

3. **Lesson 0020 (`lessons/0020-non-blocking-file-import-and-progress-ui.md` -> `.html`, `apps/0020-non-blocking-file-import-and-progress-ui`)**:
   - Implemented `progress_fraction(completed_steps, total_steps)`, `ImportJobMessage`, `ImportJobState`, `spawn_background_session_import`, and `BoardModel::apply_import_message` in `crates/pcb-doc`.
   - Wired `start_async_import`, `cancel_async_import`, non-blocking `poll_async_import` (`try_recv()` + `ctx.request_repaint()`), and `egui::ProgressBar` into `pcb-reviewer`.
   - Added interactive browser widget `{{widget:async-import-sim}}` (`initAsyncImportSimulator`).
   - **Verification**: `20 / 20` workspace tests (`14` in `pcb-doc`, `6` in `pcb-reviewer`); `269 / 269` viable mutants caught (`0 missed`).

4. **Lesson 0021 (`lessons/0021-headless-snapshot-testing-and-wasm-build.md` -> `.html`, `apps/0021-headless-snapshot-testing-and-wasm-build`)**:
   - Implemented process-wide GPU/snapshot mutex `gpu_lock()` (`OnceLock<Mutex<()>>` recovering via `PoisonError::into_inner`), `runtime_target_label()` (`#[cfg(target_arch = "wasm32")]`), `FrameSnapshotDigest` + `BoardModel::compute_snapshot_digest`, and `QualityScorecard` in `crates/pcb-doc`.
   - Wired `ViewState::visible_layer_mask()`, `PcbReviewerApp::capture_frame_digest()`, `Target: native-desktop`, and `Snapshot: 0x...` badges into `pcb-reviewer`.
   - Added interactive browser widget `{{widget:snapshot-wasm-sim}}` (`initSnapshotWasmSimulator`).
   - **Verification**: `21 / 21` workspace tests (`15` in `pcb-doc`, `6` in `pcb-reviewer`); `304 / 304` viable mutants caught (`0 missed`).

5. **Course Build & Quality Gates (`build.py --check`)**:
   - Hardened `build.py` to validate that every `:::exercise` block contains at least 1 parsed `{{blank:...}}` input and fixed `lessons/0014`–`0017` blank syntax.
   - Updated `reference/GLOSSARY.md`, `reference/testing-and-mutation-cheatsheet.md`, `index.md`, `README.md`, and `NOTES.md`.

## 2. Next Steps (Act VI: `0022`–`0025`)

- **Act VI (`0022`–`0025`)**: Real-World KiCad Parsing (`nom`) & Embedded SQL (`turso` / `libsql`).

# Learning Record 0007: Act IV (`0014`–`0017`) — Engine-First Architecture, `serde` Persistence, Copy-on-Write (`Arc`) Snapshots & `proptest` Undo/Redo

- **Date**: 2026-10-25
- **Scope**: Lessons `0014` through `0017` (`Act IV: Scaling Up — Engine-First Architecture & Persistence`)

## Summary of Deliverables Completed

1. **Lesson 0014 — Multi-Crate Workspace (`pcb-doc` `L1` vs. `pcb-reviewer` `L6`/`L7`)**
   - **Markdown Source & HTML Lesson**: [`lessons/0014-multi-crate-workspace-doc-vs-ui-egui.md`](../lessons/0014-multi-crate-workspace-doc-vs-ui-egui.md) → [`lessons/0014-multi-crate-workspace-doc-vs-ui-egui.html`](../lessons/0014-multi-crate-workspace-doc-vs-ui-egui.html)
   - **Companion Workspace**: [`apps/0014-multi-crate-workspace-doc-vs-ui-egui`](../apps/0014-multi-crate-workspace-doc-vs-ui-egui) (`crates/pcb-doc` + root `pcb-reviewer`)
   - **Core Concepts**: Splitting pure domain geometry, DRC math, and review pin invariants into `crates/pcb-doc` (`L1`, depending only on `emath = "0.33"`), bridging `PcbLayer::rgb(self) -> (u8, u8, u8)` into `egui::Color32` in `pcb-reviewer`, and enforcing `#![forbid(unsafe_code)]`, `#![deny(clippy::unwrap_used, ...)]`, and `validate_l1_manifest_no_ui_deps`.
   - **Verification**: `11 / 11` workspace tests passed (`5` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants --in-place` caught **`144 / 144` viable mutants** (`98` in `pcb-doc`, `46` in `pcb-reviewer`, `21` unviable, **`0` missed**).

2. **Lesson 0015 — Loading & Saving Board & Review Session Files (`serde`, `serde_json` & `thiserror`)**
   - **Markdown Source & HTML Lesson**: [`lessons/0015-loading-board-and-review-files-serde.md`](../lessons/0015-loading-board-and-review-files-serde.md) → [`lessons/0015-loading-board-and-review-files-serde.html`](../lessons/0015-loading-board-and-review-files-serde.html)
   - **Companion Workspace**: [`apps/0015-loading-board-and-review-files-serde`](../apps/0015-loading-board-and-review-files-serde)
   - **Core Concepts**: Enabling `emath`'s `"serde"` feature, wrapping `BoardModel` in `ReviewSessionEnvelope` (`CURRENT_SCHEMA_VERSION = 1`), exhaustive struct destructuring in `BoardModel::from_session_json`, typed `SessionIoError` via `thiserror::Error`, and non-destructive error banners in `egui::SidePanel::right("session_panel")`.
   - **Verification**: `12 / 12` workspace tests passed (`6` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants --in-place` caught **`155 / 155` viable mutants** (`21` unviable, **`0` missed**).

3. **Lesson 0016 — Copy-on-Write Document Snapshots with `Arc` & `Arc::make_mut`**
   - **Markdown Source & HTML Lesson**: [`lessons/0016-copy-on-write-arc-document-snapshots.md`](../lessons/0016-copy-on-write-arc-document-snapshots.md) → [`lessons/0016-copy-on-write-arc-document-snapshots.html`](../lessons/0016-copy-on-write-arc-document-snapshots.html)
   - **Companion Workspace**: [`apps/0016-copy-on-write-arc-document-snapshots`](../apps/0016-copy-on-write-arc-document-snapshots)
   - **Core Concepts**: Wrapping `BoardModel` collections in `Arc<Vec<...>>` and `Arc<HashMap<...>>` (`serde` `"rc"` feature), mutating touched buffers via `Arc::make_mut`, inspecting structural sharing via `Arc::ptr_eq` (`shared_arc_buffers`), `O(1)` baseline snapshot take/restore in `SidePanel::right`, and `proptest!` property verification.
   - **Verification**: `14 / 14` workspace tests passed (`8` in `pcb-doc` including `proptest`, `6` in `pcb-reviewer`); `cargo mutants --in-place` caught **`169 / 169` viable mutants** (`21` unviable, **`0` missed**).

4. **Lesson 0017 — Undo/Redo History (`VecDeque`) & Continuous Slider Drag Coalescing**
   - **Markdown Source & HTML Lesson**: [`lessons/0017-undo-redo-history-and-slider-coalescing.md`](../lessons/0017-undo-redo-history-and-slider-coalescing.md) → [`lessons/0017-undo-redo-history-and-slider-coalescing.html`](../lessons/0017-undo-redo-history-and-slider-coalescing.html)
   - **Companion Workspace**: [`apps/0017-undo-redo-history-and-slider-coalescing`](../apps/0017-undo-redo-history-and-slider-coalescing)
   - **Core Concepts**: Bounded `BoardHistory` (`VecDeque<HistoryEntry>`, `redo_stack`, `revision`), continuous 60-FPS slider drag coalescing via `coalesce_key` and `Response::drag_stopped()`, `O(1)` undo/redo swaps via `std::mem::replace`, `Key::Z` / `Key::Y` hotkeys, and randomized drag-sequence `proptest!`.
   - **Verification**: `16 / 16` workspace tests passed (`10` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants --in-place` caught **`217 / 217` viable mutants** (`22` unviable, **`0` missed**).

# Teaching Notes & Preferences

- **User Formatting Preferences**:
  - **Dark Mode**: Use dark mode across all lessons and reference documents (`<meta name="color-scheme" content="dark">`, `:root { color-scheme: dark; }`, dark Tufte palette in `./assets/style.css`, and Mermaid `theme: "dark"`, with `@media print` overrides for clean printing).
  - Always use **Prism.js** (`<pre><code class="language-rust">`, `language-toml`, `language-bash`, etc.) for syntax highlighting across all lessons and reference documents.
  - Always use **Mermaid.js** (`<pre class="mermaid">`) for architecture, lifecycle, and flow diagrams (avoid ASCII art diagrams).
- **Testing & Mutation Coverage Preference**:
  - Integrate testing gradually across the **entire narrative arc** (`0001`–`0030`+), not as an isolated module near the end.
  - **Every lesson** must include concrete **Cargo Verification Prompts** (`cargo test` + `cargo mutants`) so the learner continuously verifies new code and monitors real assertion coverage via mutation testing (`Caught` vs. `Missed` mutants).
  - Use `.cargo/mutants.toml` (or `--exclude-re`) to explicitly demarcate what is currently excluded (such as the OS window launcher `fn main`) and progressively shrink exclusions as higher verification tiers (`egui_kittest` in Act II, offscreen `wgpu` snapshots in Act V, in-memory `rmcp` duplex acceptance suites in Act VII) come online.
- **Target Application**: Schematic and PCB design review system in Rust + `egui`.
- **Key Ecosystem Crates & Tools Planned**:
  - `eframe` / `egui` (Immediate-mode GUI and 2D CAD canvas)
  - `cargo-mutants` (Mutation testing CLI to monitor test coverage quality across every lesson)
  - `egui_kittest` (Headless `egui` interaction harness and offscreen visual snapshot testing)
  - `proptest` (Property-based testing for coordinate math, spatial culling, and COW undo/redo)
  - `serde` / `serde_json` (Structured serialization)
  - `nom` (Parser combinators for KiCad `.kicad_pcb` and `.kicad_sch` S-expression files)
  - `turso` / `libsql` (Embedded SQL database & local-first sync for engineering review threads and DRC audit logs)
  - `rmcp` / `schemars` (Official Model Context Protocol Rust SDK + JSON Schema generation for Headless and Bridged MCP servers and the Mock MCP Client & Inspector Workbench)
- **Rust Background**: Learning Rust alongside `egui` — introduce one Rust concept just-in-time alongside one `egui` skill and one verification/mutation step in every lesson.
- **Installed Toolchain**: `cargo 1.97.1` / `rustc 1.97.1` on macOS.

---

## Curriculum Arc (`0001`–`0030`+) with Continuous Verification & Mutation Testing

### Completed Lessons (`0001`–`0030`: Foundation + Act I + Act II + Act III + Act IV + Act V + Act VI + Act VII)
- `0001-project-setup-and-eframe-boot.html` (App: `apps/0001-project-setup-and-eframe-boot`)
  - **`egui` Skill & PCB Feature**: Creating the Cargo crate (`Cargo.toml`, `eframe = "0.37"`, dev-profile optimization) and booting the native window with `eframe::run_native`.
  - **Just-in-Time Rust**: `impl Default` delegating to `reset_layers(&mut self)`, `ViewportBuilder` method chaining, `Box::new`, and `#[cfg(test)]` / `#[test]` / `assert!`.
  - **Cargo Verification & Mutation Coverage**: First `#[test]` verifying `PcbReviewerApp::default()` initial layer flags (`show_top_copper == true`, `show_ratsnest == false`); installing `cargo-mutants`, configuring `.cargo/mutants.toml` (`exclude_re = ["replace main", "<impl eframe::App for PcbReviewerApp>::ui"]`), and catching the `PcbReviewerApp::reset_layers` mutant (`1 / 1 caught, 0 missed`).
- `0002-the-immediate-mode-loop.html` (App: `apps/0002-the-immediate-mode-loop`)
  - **`egui` Skill & PCB Feature**: `eframe::App::ui`, `&mut self` state, `Panel::left` + `CentralPanel`, `ui.checkbox`, `ui.button().clicked()` for PCB layer toggles.
  - **Just-in-Time Rust**: Mutable borrows (`&mut`), closures (`|ui|`), and extracting pure `&mut self` state-transition methods (`isolate_top_copper`, `show_all_layers`).
  - **Cargo Verification & Mutation Coverage**: Unit-testing `isolate_top_copper()` and `show_all_layers()` with `cargo test`, then running `cargo mutants` across all 3 state methods (`reset_layers`, `isolate_top_copper`, `show_all_layers`) to verify `3 / 3 caught, 0 missed`.

### Act I: The 2D CAD Canvas & Viewport (`0003`–`0006`) — *Completed*
*Goal: Turn the empty `CentralPanel` into an interactive, multi-layer 2D board viewer while establishing pure geometry & domain unit tests verified by `cargo mutants`.*
- `0003-custom-2d-canvas-and-painter.html` (App: `apps/0003-custom-2d-canvas-and-painter`)
  - **`egui` Skill & PCB Feature**: Reserve canvas space with `ui.allocate_painter`, draw PCB pads, traces, and vias using `Pos2`, `Vec2`, `Rect`, `Color32`, and `Stroke`.
  - **Just-in-Time Rust**: Tuple structs, value semantics (`Copy` vs. move), and method chaining.
  - **Cargo Verification & Mutation Coverage**: 5 unit tests (`centered_board_rect` and `smd_pad_rect`); `cargo mutants --in-place` catches `23 / 23 viable mutants (2 unviable, 0 missed)`.
- `0004-world-to-screen-transforms-pan-and-zoom.html` (App: `apps/0004-world-to-screen-transforms-pan-and-zoom`)
  - **`egui` Skill & PCB Feature**: Map physical board coordinates (mm) to screen points (`egui::emath::TSTransform`), drag to pan (`Sense::drag`), and zoom toward the cursor (`ui.input`).
  - **Just-in-Time Rust**: Pure functions, `impl` methods on a custom `BoardViewport` struct, and closures (`|i|`).
  - **Cargo Verification & Mutation Coverage**: 6 unit tests verifying `world_to_screen` ↔ `screen_to_world` round-trips, `pan_by`, and cursor-anchored `zoom_around` + clamping; `cargo mutants --in-place` catches `14 / 14 viable mutants (6 unviable, 0 missed)`.
- `0005-modeling-layers-traces-and-footprints.html` (App: `apps/0005-modeling-layers-traces-and-footprints`)
  - **`egui` Skill & PCB Feature**: Render a structured board model (`BottomCopper`, `TopCopper`, `Silkscreen`, `Ratsnest`) in deterministic Z-order based on layer visibility toggles.
  - **Just-in-Time Rust**: `Vec<T>`, `enum PcbLayer`, `match` exhaustiveness, and `for` loop borrowing (`&self.traces`).
  - **Cargo Verification & Mutation Coverage**: 9 unit tests verifying `PcbLayer::z_rank()`, `color()`, `is_layer_visible()`, and `visible_traces_in_z_order()`; `cargo mutants --in-place` catches `20 / 20 viable mutants (7 unviable, 0 missed)`.
- `0006-hit-testing-and-net-cross-probing.html` (App: `apps/0006-hit-testing-and-net-cross-probing`)
  - **`egui` Skill & PCB Feature**: Hover and click pads on the canvas (`Sense::click_and_drag()`), highlight the connected net across layers, and inspect its properties in the `Panel::left` Net Inspector.
  - **Just-in-Time Rust**: `Option<NetId>`, `if let Some(id) = ...`, `.or(...)`, and iterator filtering.
  - **Cargo Verification & Mutation Coverage**: 11 unit tests verifying `hit_test_net_at_mm` (hit, miss in empty space, hidden-layer rejection, and overlapping Z-order priority) and `active_net`; `cargo mutants --in-place` catches `25 / 25 viable mutants (10 unviable, 0 missed)`.

### Act II: Schematic Views, Layout & Inspection (`0007`–`0010`) — *Completed*
*Goal: Add schematic symbols, dual-pane cross-probing, and searchable BOM tables while introducing headless UI widget & interaction testing with `egui_kittest`.*
- `0007-schematic-symbols-and-text-rendering.html` (App: `apps/0007-schematic-symbols-and-text-rendering`)
  - **`egui` Skill & PCB Feature**: Draw IC packages, pin numbers, net labels, and reference designators (`U1`, `R12`, `J1`) using `Painter::text`, `FontId`, and `Align2`.
  - **Just-in-Time Rust**: `String` vs. `&str` string slices and `format!()`.
  - **Cargo Verification & Mutation Coverage**: Introduced [`egui_kittest::Harness`](https://docs.rs/egui_kittest/latest/egui_kittest/) in `[dev-dependencies]`, removed `<impl eframe::App for PcbReviewerApp>::ui` from `.cargo/mutants.toml` exclusions, and verified 14 unit + headless UI tests (`35 / 35 viable mutants caught, 10 unviable, 0 missed`).
- `0008-split-viewports-and-egui-scene.html` (App: `apps/0008-split-viewports-and-egui-scene`)
  - **`egui` Skill & PCB Feature**: Split Schematic + PCB viewports (`Panel::top`, `egui::Scene`), clicking a schematic symbol button to center the PCB camera rect (`pcb_scene_rect`) on that footprint.
  - **Just-in-Time Rust**: Shared immutable state (`&BoardModel`) vs. scoped mutable view state (`&mut ViewState`).
  - **Cargo Verification & Mutation Coverage**: 16 unit + `egui_kittest` tests verifying cross-probe selection and camera centering (`40 / 40 viable mutants caught, 11 unviable, 0 missed`).
- `0009-searchable-netlist-and-bom-inspector.html` (App: `apps/0009-searchable-netlist-and-bom-inspector`)
  - **`egui` Skill & PCB Feature**: Filterable component/net list using `TextEdit::singleline`, `ScrollArea::vertical().show_rows(...)`, and `egui::Grid`.
  - **Just-in-Time Rust**: Iterator adapters (`.filter()`), case-insensitive string matching, and slices (`&[T]`).
  - **Cargo Verification & Mutation Coverage**: 17 unit + `egui_kittest` tests verifying case-insensitive BOM query filtering across all 4 fields (`47 / 47 viable mutants caught, 12 unviable, 0 missed`).
- `0010-keyboard-shortcuts-and-point-to-point-ruler.html` (App: `apps/0010-keyboard-shortcuts-and-point-to-point-ruler`)
  - **`egui` Skill & PCB Feature**: Bind hotkeys (`F` to fit board, `M` for measurement tool, `Esc` to clear) via `ctx.input_mut` + `consume_key`, and draw a live $\Delta x / \Delta y$ / Euclidean distance CAD ruler overlay.
  - **Just-in-Time Rust**: State machines with data-carrying enums (`enum ActiveTool { Select, Measure { start_mm: Option<Pos2>, end_mm: Option<Pos2> } }`).
  - **Cargo Verification & Mutation Coverage**: 19 unit + `egui_kittest` tests simulating key presses (`Key::M`, `Key::Escape`, `Key::F`) and verifying Euclidean $\Delta x / \Delta y$ / hypotenuse math (`59 / 59 viable mutants caught, 14 unviable, 0 missed`).

### Act III: Design Review Workflows (`0011`–`0014`) — *Completed*
*Goal: Turn the viewer into an engineering review tool with annotations, DRC markers, and custom widgets, testing multi-step UI workflows and custom `egui::Widget` traits.*
- `0011-review-pins-and-sticky-note-callouts.html` (App: `apps/0011-review-pins-and-sticky-note-callouts`)
  - **`egui` Skill & PCB Feature**: Drop review comments anchored to `(x, y)` board coordinates and render interactive floating overlays (`egui::Window`, `Id::unique`).
  - **Just-in-Time Rust**: `HashMap<PinId, ReviewNote>`, unique IDs, and the `Entry` API (`Entry::Vacant` vs. `Entry::Occupied`).
  - **Cargo Verification & Mutation Coverage**: 21 unit + `egui_kittest` tests verifying pin insertion, duplicate-ID rejection in `HashMap`, deterministic sorting, and floating callout window lifecycle (`72 / 72 viable mutants caught, 16 unviable, 0 missed`).
- `0012-review-status-workflow-and-popups.html` (App: `apps/0012-review-status-workflow-and-popups`)
  - **`egui` Skill & PCB Feature**: Triage review items (`Open` → `Resolved` → `Verified`), filter by severity (`PinSeverity::ALL`), and build interactive triage controls (`ui.selectable_value`).
  - **Just-in-Time Rust**: Custom methods on enums, `#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]`, and methods returning static slices (`&'static [Self]`).
  - **Cargo Verification & Mutation Coverage**: 22 unit + `egui_kittest` tests verifying all $3 \times 3$ review status transitions and multi-criteria filtering (`severity × status`), catching all boolean combinator mutations (`88 / 88 viable mutants caught, 17 unviable, 0 missed`).
- `0013-drc-violation-overlay-and-minimap.html` (App: `apps/0013-drc-violation-overlay-and-minimap`)
  - **`egui` Skill & PCB Feature**: Render Design Rule Check error markers (copper pad clearance halos) and clamp a bird's-eye minimap camera rectangle.
  - **Just-in-Time Rust**: Bounding-box reduction (`Iterator::reduce(Rect::union)`) and coordinate clamping (`f32::clamp`).
  - **Cargo Verification & Mutation Coverage**: 23 unit + `egui_kittest` tests verifying minimum copper-to-copper clearance detection (including exact `7.6 mm` boundary assertions catching `<` vs. `<=`) and minimap coordinate clamping (`125 / 125 viable mutants caught, 20 unviable, 0 missed`).
- `0014-custom-layer-swatch-widget-and-design-tokens.html` (App: `apps/0014-custom-layer-swatch-widget-and-design-tokens`)
  - **`egui` Skill & PCB Feature**: Extract a reusable PCB layer row widget (`LayerSwatch<'a>`: color swatch + visibility checkbox + opacity slider) implementing `egui::Widget`.
  - **Just-in-Time Rust**: Implementing a foreign trait (`impl<'a> egui::Widget for LayerSwatch<'a>`) and struct lifetimes (`'a`).
  - **Cargo Verification & Mutation Coverage**: 24 unit + `egui_kittest` tests mounting `LayerSwatch<'a>` in isolation via `egui_kittest::Harness::new_ui_state` and inside the full `PcbReviewerApp` (`134 / 134 viable mutants caught, 21 unviable, 0 missed`).

### Act IV: Scaling Up — Engine-First Architecture & Persistence (`0015`–`0018`) — *Completed*
*Goal: Refactor the growing prototype into a clean multi-crate architecture (`egui-app-architecture` & `egui-cow-document-model`) with static lint gates, `proptest` property suites, and per-crate `cargo mutants -p <crate>`.*
- `0015-multi-crate-workspace-doc-vs-ui-egui.html` (App: `apps/0015-multi-crate-workspace-doc-vs-ui-egui`)
  - **`egui` Skill & PCB Feature**: Split pure PCB/review domain data (`L1` `crates/pcb-doc`, depending only on `emath`) from `egui` rendering (`L6`/`L7` `pcb-reviewer`) so the core model compiles headlessly without UI dependencies.
  - **Just-in-Time Rust**: Cargo `[workspace]`, module visibility (`pub`, `pub(crate)`), and strict dependency layering (`validate_l1_manifest_no_ui_deps`).
  - **Cargo Verification & Mutation Coverage**: Added Tier 1 static verification gates (`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, `#![forbid(unsafe_code)]`, `validate_l1_manifest_no_ui_deps`) and verified 11 workspace tests (`98 / 98 caught` in `pcb-doc`, `46 / 46 caught` in `pcb-reviewer`; `144 / 144 viable mutants caught, 21 unviable, 0 missed`).
- `0016-loading-board-and-review-files-serde.html` (App: `apps/0016-loading-board-and-review-files-serde`)
  - **`egui` Skill & PCB Feature**: Serialize and deserialize board layouts and review sessions to versioned JSON (`ReviewSessionEnvelope`), surfacing dismissible `SessionIoError` banners in `Panel::right` without panicking.
  - **Just-in-Time Rust**: `#[derive(Serialize, Deserialize)]` (`emath` `"serde"` feature), `Result<T, E>`, exhaustive struct destructuring, and `thiserror`.
  - **Cargo Verification & Mutation Coverage**: 12 workspace tests verifying lossless `to_session_json -> from_session_json` round-trips, `CURRENT_SCHEMA_VERSION` validation, empty `board_id` rejection, and corrupt JSON recovery (`155 / 155 viable mutants caught, 21 unviable, 0 missed`).
- `0017-copy-on-write-arc-document-snapshots.html` (App: `apps/0017-copy-on-write-arc-document-snapshots`)
  - **`egui` Skill & PCB Feature**: Store `BoardModel` collections in `Arc<Vec<...>>` and `Arc<HashMap<...>>` (`serde` `"rc"` feature) so taking or restoring a baseline snapshot in `Panel::right` costs only 4 pointer copies (`O(1)`), with live `Shared Arc Buffers: X / 4` telemetry.
  - **Just-in-Time Rust**: Smart pointers (`Arc<T>`), `Arc::make_mut` copy-on-write mutation, `Arc::ptr_eq`, and `proptest!`.
  - **Cargo Verification & Mutation Coverage**: 14 workspace tests (including `prop_cow_pin_mutation_preserves_baseline_snapshot_and_shares_untouched_arcs`) verifying `4 -> 3 -> 2 -> 1 -> 0` pointer sharing (`169 / 169 viable mutants caught, 21 unviable, 0 missed`).
- `0018-undo-redo-history-and-slider-coalescing.html` (App: `apps/0018-undo-redo-history-and-slider-coalescing`)
  - **`egui` Skill & PCB Feature**: Wire `Z` / `Y` hotkeys, `Undo (Z)` / `Redo (Y)` buttons in `Panel::right`, and coalesce continuous 60-FPS `egui::Slider` pin X drags into a single undo step via `coalesce_key` and `Response::drag_stopped()`.
  - **Just-in-Time Rust**: Bounded `VecDeque<HistoryEntry>` ring buffer, `std::mem::replace` zero-clone state swaps, and randomized drag-sequence `proptest!`.
  - **Cargo Verification & Mutation Coverage**: 16 workspace tests (including `prop_coalesced_drag_restores_exact_initial_state_in_single_undo`) verifying coalescing, `commit_coalesce()`, redo clearing, and ring-buffer eviction (`217 / 217 viable mutants caught, 22 unviable, 0 missed`).

### Act V: Command Engine, Performance & Polish (`0019`–`0022`) — *Completed*
*Goal: Unify menus/shortcuts/CLI into a Command Engine (`egui-command-engine` & `egui-app-runner`), add `panic_hunt` command fuzzing, enforce performance budgets, and run offscreen GPU visual snapshot tests.*
- `0019-the-command-pattern-and-command-palette.html` (App: `apps/0019-the-command-pattern-and-command-palette`)
  - **`egui` Skill & PCB Feature**: Route menu items, toolbar buttons, and a searchable `Key::K` Command Palette modal (`egui::Window`) through a single `CommandSpec` registry (`session.inspect`, `review.addPin`, `review.setStatus`, `review.clearPins`).
  - **Just-in-Time Rust**: Static slices (`&'static [CommandSpec]`), typed `EngineError` via `thiserror`, `serde_json::Value` parameter inspection, and `COMMAND_PARITY_FLOOR`.
  - **Cargo Verification & Mutation Coverage**: 17 workspace tests including the Tier 5 `panic_hunt` adversarial JSON fuzzer (`command_registry_dispatch_and_panic_hunt_fuzzer_preserve_invariants`) feeding malformed/non-object/negative/huge JSON payloads into every registered `CommandSpec` (`238 / 238 viable mutants caught, 24 unviable, 0 missed`).
- `0020-viewport-culling-and-cached-tessellation.html` (App: `apps/0020-viewport-culling-and-cached-tessellation`)
  - **`egui` Skill & PCB Feature**: Spatial viewport culling (`Rect::intersects`) skipping offscreen pads and stroke-expanded traces before submitting `Painter` commands, plus live `ViewportCullStats` telemetry in `Panel::right`.
  - **Just-in-Time Rust**: Stroke-expanded axis-aligned bounding boxes (`TraceSegment::bounds_mm`), `ViewportCullStats::culled_count()`, `check_cull_budget(elapsed_micros, max_budget_micros)`, and `proptest!`.
  - **Cargo Verification & Mutation Coverage**: 19 workspace tests (including `prop_spatial_culling_matches_reference_and_preserves_counts`) proving spatial culling equivalence and budget gate boundaries (`255 / 255 viable mutants caught, 29 unviable, 0 missed`).
- `0021-non-blocking-file-import-and-progress-ui.html` (App: `apps/0021-non-blocking-file-import-and-progress-ui`)
  - **`egui` Skill & PCB Feature**: Parse JSON board sessions on a background worker thread while polling non-blockingly (`Receiver::try_recv`), rendering `egui::ProgressBar`, and waking the UI loop via `ctx.request_repaint()`.
  - **Just-in-Time Rust**: `std::thread::spawn`, `std::sync::mpsc::{channel, Receiver}`, `ImportJobMessage`, `ImportJobState`, and cancellation by dropping the `Receiver`.
  - **Cargo Verification & Mutation Coverage**: 20 workspace tests deterministically testing multi-stage progress fractions, channel disconnection when the UI drops the receiver, and corrupt JSON worker recovery (`269 / 269 viable mutants caught, 33 unviable, 0 missed`).
- `0022-headless-snapshot-testing-and-wasm-build.html` (App: `apps/0022-headless-snapshot-testing-and-wasm-build`)
  - **`egui` Skill & PCB Feature**: Serialize GPU/snapshot tests via `gpu_lock()`, compute 64-bit FNV-1a `FrameSnapshotDigest` hashes over the visible layer bitmask (`visible_layer_mask`), culled geometry, and DRC violations, display `Target: native-desktop` / `wasm32-unknown-unknown`, and evaluate release readiness with `QualityScorecard`.
  - **Just-in-Time Rust**: `std::sync::OnceLock<Mutex<()>>`, panic-free lock recovery via `PoisonError::into_inner`, 64-bit FNV-1a hashing, and `#[cfg(target_arch = "wasm32")]`.
  - **Cargo Verification & Mutation Coverage**: 21 workspace tests (`15` in `pcb-doc`, `6` in `pcb-reviewer`) verifying `gpu_lock()`, `FrameSnapshotDigest`, `visible_layer_mask()`, and `QualityScorecard::is_release_ready()` (`304 / 304 viable mutants caught, 39 unviable, 0 missed`).

### Act VI: Real-World KiCad Parsing (`nom`) & Embedded SQL (`turso` / `libsql`) (`0023`–`0026`) — *Completed*
*Goal: Parse real-world KiCad `.kicad_pcb` and `.kicad_sch` files with `nom` in an `L0` standalone parser crate, verify against pinned real-file corpus oracles, and persist/sync multi-engineer review threads in an embedded Turso (`libsql`) SQL database.*
- `0023-sexpr-tokenizer-and-ast-with-nom.html` (App: `apps/0023-sexpr-tokenizer-and-ast-with-nom`)
  - **`egui` Skill & PCB Feature**: Interactive S-expression AST inspector panel (`egui::CollapsingHeader` tree view) visualizing raw KiCad `(kicad_pcb ...)` nodes parsed by a standalone `L0` `crates/kicad-sexpr` crate (`SAMPLE_KICAD_PCB_SNIPPET`).
  - **Just-in-Time Rust**: Parser combinators (`nom::IResult`, `delimited`, `multispace0`, `alt`, `many0`), zero-copy string slices, recursive `SexprNode` ASTs, and `validate_l0_standalone_manifest`.
  - **Cargo Verification & Mutation Coverage**: 24 workspace tests (`3` in `kicad-sexpr` including `prop_sexpr_parser_roundtrips_arbitrary_footprints`, `15` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `38 / 38` viable `L0` + UI inspector mutants (`342 / 342 viable workspace mutants caught, 0 missed`).
- `0024-parsing-kicad-pcb-and-sch-into-domain-model.html` (App: `apps/0024-parsing-kicad-pcb-and-sch-into-domain-model`)
  - **`egui` Skill & PCB Feature**: Lower `(kicad_pcb ...)` ASTs into `BoardModel` (`from_kicad_pcb_sexpr`) and export back (`to_kicad_pcb_sexpr`) with `"Import KiCad Board"` and `"Export KiCad S-Expr"` controls and a live `Corpus [PASS]: 3/3 pass, 3/3 round-trip` badge in `Panel::right`.
  - **Just-in-Time Rust**: Typed `KicadLowerError` diagnostics (`#[from] SexprParseError`, `UnexpectedRootTag`, `MissingField`, `InvalidNumber`, `UnknownLayer`), `PcbLayer::from_kicad_name`, and Tier 4 `CORPUS_PASS_FLOOR` / `CORPUS_ROUNDTRIP_FLOOR` ratchets.
  - **Cargo Verification & Mutation Coverage**: 25 workspace tests (`3` in `kicad-sexpr`, `16` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `43 / 43` viable lowering & corpus oracle mutants (`385 / 385 viable workspace mutants caught, 0 missed`).
- `0025-embedded-sql-review-store-with-turso.html` (App: `apps/0025-embedded-sql-review-store-with-turso`)
  - **`egui` Skill & PCB Feature**: Validate SQLite/Turso DDL & DML syntax (`REVIEW_STORE_SCHEMA_SQL`, `CREATE TABLE IF NOT EXISTS`, `INSERT INTO`, `SELECT`) via `turso_parser::parser::Parser`, enforce `board_id` foreign-key constraints in `TursoReviewStore`, and sync review pins from `egui` (`"Sync Pins to SQL"`).
  - **Just-in-Time Rust**: `turso_parser::ast::{Cmd, Stmt}`, `async`/`await` storage methods paired with synchronous bridges, and `#[tokio::test]` async integration tests.
  - **Cargo Verification & Mutation Coverage**: 26 workspace tests (`3` in `kicad-sexpr`, `17` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `40 / 40` viable SQL parser & store mutants (`425 / 425 viable workspace mutants caught, 0 missed`).
- `0026-local-first-review-sync-and-audit-trail.html` (App: `apps/0026-local-first-review-sync-and-audit-trail`)
  - **`egui` Skill & PCB Feature**: Execute atomic `BEGIN` / `COMMIT` / `ROLLBACK` sign-off batches (`"Commit Sign-Off Batch"`), verify 64-bit hash-chained `AuditLogEntry` records (`verify_audit_chain` & `Replica: X pending, Y audit [VALID]`), and merge remote replica rows (`"Sync Replica"`).
  - **Just-in-Time Rust**: Staged transactional rollback snapshots (`execute_atomic_signoff_batch`), hash-chained audit logs (`compute_audit_entry_hash`), and monotonic revision replica merging (`remote.revision > local.revision`).
  - **Cargo Verification & Mutation Coverage**: 27 workspace tests (`3` in `kicad-sexpr`, `18` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `51 / 51` viable transaction, audit-chain, and replica-sync mutants (`476 / 476 viable workspace mutants caught, 0 missed`).

### Act VII: Agentic Hardware Review & Model Context Protocol (`rmcp`) (`0027`–`0030`) — *Completed*
*Goal: Turn the PCB Reviewer into an AI-controllable engineering co-pilot via the official `rmcp` SDK (`egui-app-runner` & `egui-command-engine`), adding LLM-native spatial/topological queries, Headless and Bridged live-GUI MCP servers, and a companion **Mock MCP Client & Inspector Workbench** app to test and drive multi-step agent playbooks.*
- `0027-llm-native-spatial-and-electrical-queries.html` (App: `apps/0027-llm-native-spatial-and-electrical-queries`)
  - **`egui` Skill & PCB Feature**: Equip the engine with high-leverage LLM-native queries: `query_spatial_neighborhood` (`SpatialQueryParams` -> `SpatialNeighborhoodReport`), `query_net_topology` (`NetTopologyReport`), and parameterized `run_custom_drc` (`CustomDrcParams` -> `CustomDrcReport`), exposed in `Panel::right` (`"Query U1 Radius"`, `"Query VDD Net"`, `"Run Custom DRC"`).
  - **Just-in-Time Rust**: `schemars::JsonSchema` derive macros, `schemars::schema_for!()` JSON Schema 2020-12 introspection, and `PcbLayer::is_copper()` filtering.
  - **Cargo Verification & Mutation Coverage**: 28 workspace tests (`3` in `kicad-sexpr`, `19` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `21 / 21` viable spatial, topology, and custom DRC mutants (`497 / 497 viable workspace mutants caught, 0 missed`).
- `0028-headless-mcp-server-rmcp-and-mock-client.html` (App: `apps/0028-headless-mcp-server-rmcp-and-mock-client`)
  - **`egui` Skill & PCB Feature**: Implement `PcbMcpServer` using the official `rmcp` v1.4 SDK (`#[tool_router]`, `#[tool]`, `#[tool_handler]`), exposing 4 typed MCP tools (`pcb_query_spatial`, `pcb_query_net`, `pcb_run_custom_drc`, `pcb_add_review_pin`), read-only `kicad://board/summary`, `kicad://net/{name}`, and `kicad://bom` resources (`read_kicad_resource`), and review prompts (`build_review_prompt`), paired with `MockMcpClient` over `tokio::io::duplex(4096)` and `egui` inspection triggers (`"MCP Board Summary"`, `"MCP Power Prompt"`).
  - **Just-in-Time Rust**: `rmcp::ServerHandler`, `rmcp::ClientHandler`, `Parameters<T>`, `Json<T>`, `Arc<Mutex<BoardModel>>`, and in-memory `tokio::io::duplex` streams.
  - **Cargo Verification & Mutation Coverage**: 29 workspace tests (`3` in `kicad-sexpr`, `20` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `10 / 10` viable MCP server, resource, prompt, and duplex round-trip mutants (`507 / 507 viable workspace mutants caught, 0 missed`).
- `0029-bridged-mcp-control-channel-and-live-gui-snapshots.html` (App: `apps/0029-bridged-mcp-control-channel-and-live-gui-snapshots`)
  - **`egui` Skill & PCB Feature**: Authenticate loopback control requests with constant-time `verify_bearer_token`, pan/zoom the live 2D CAD camera via `LoopbackControlBridge::dispatch` (`"Bridge Focus U1"`), and capture cropped PNG region snapshots (`capture_cropped_region_png`, `"Capture U1 PNG"`) prefixed with `PNG_MAGIC_HEADER`.
  - **Just-in-Time Rust**: Constant-time byte slice comparison (`^` and `|=`), `PNG_MAGIC_HEADER` (`[u8; 8]`), and pixel budget guards (`width_px.saturating_mul(height_px) > 16_384`).
  - **Cargo Verification & Mutation Coverage**: 30 workspace tests (`3` in `kicad-sexpr`, `21` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `26 / 26` viable bearer-auth, loopback bridge, and cropped PNG snapshot mutants (`533 / 533 viable workspace mutants caught, 0 missed`).
- `0030-egui-mcp-inspector-workbench-and-agent-playbooks.html` (App: `apps/0030-egui-mcp-inspector-workbench-and-agent-playbooks`)
  - **`egui` Skill & PCB Feature**: Embed `McpInspectorWorkbench` inside `PcbReviewerApp` with transport toggling (`McpTransportMode::HeadlessDuplex` $\leftrightarrow$ `BridgedLoopback`, `"Switch MCP Transport"`), 4-step `run_power_integrity_playbook` (`"Run Agent Playbook"`), and `RpcFrameRecord` telemetry, sealing the final Act VII `QualityScorecard`.
  - **Just-in-Time Rust**: State-machine playbook runners (`PlaybookReport`), `RpcFrameRecord` audit logs, and dual-transport execution.
  - **Cargo Verification & Mutation Coverage**: 31 workspace tests (`3` in `kicad-sexpr`, `22` in `pcb-doc`, `6` in `pcb-reviewer`); `cargo mutants` catches `6 / 6` viable workbench and playbook mutants (`539 / 539 viable workspace mutants caught, 0 missed`).

### Future Acts Under Exploration (`Act VIII+`, `0031`+)
- *The course remains open-ended beyond `0030`—future acts can explore Gerber/Excellon manufacturing output & visual copper XOR revision diffing (`Rev A` vs. `Rev B`), custom WGSL GPU shaders (`egui_wgpu::CallbackTrait`) for sub-pixel copper pour & thermal relief rendering, interactive length-tuning/impedance calculators, or multi-board panelization.*

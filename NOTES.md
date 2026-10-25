# Teaching Notes & Preferences

- **User Formatting Preferences**:
  - **Dark Mode**: Use dark mode across all lessons and reference documents (`<meta name="color-scheme" content="dark">`, `:root { color-scheme: dark; }`, dark Tufte palette in `./assets/style.css`, and Mermaid `theme: "dark"`, with `@media print` overrides for clean printing).
  - Always use **Prism.js** (`<pre><code class="language-rust">`, `language-toml`, `language-bash`, etc.) for syntax highlighting across all lessons and reference documents.
  - Always use **Mermaid.js** (`<pre class="mermaid">`) for architecture, lifecycle, and flow diagrams (avoid ASCII art diagrams).
- **Testing & Mutation Coverage Preference**:
  - Integrate testing gradually across the **entire narrative arc** (`0000`–`0029`+), not as an isolated module near the end.
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

## Curriculum Arc (`0000`–`0029`+) with Continuous Verification & Mutation Testing

### Completed Lessons (`0000`–`0021`: Foundation + Act I + Act II + Act III + Act IV + Act V)
- `0000-project-setup-and-eframe-boot.html` (App: `apps/0000-project-setup-and-eframe-boot`)
  - **`egui` Skill & PCB Feature**: Creating the Cargo crate (`Cargo.toml`, `eframe = "0.33"`, dev-profile optimization) and booting the native window with `eframe::run_native`.
  - **Just-in-Time Rust**: `impl Default` delegating to `reset_layers(&mut self)`, `ViewportBuilder` method chaining, `Box::new`, and `#[cfg(test)]` / `#[test]` / `assert!`.
  - **Cargo Verification & Mutation Coverage**: First `#[test]` verifying `PcbReviewerApp::default()` initial layer flags (`show_top_copper == true`, `show_ratsnest == false`); installing `cargo-mutants`, configuring `.cargo/mutants.toml` (`exclude_re = ["replace main", "<impl eframe::App for PcbReviewerApp>::update"]`), and catching the `PcbReviewerApp::reset_layers` mutant (`1 / 1 caught, 0 missed`).
- `0001-the-immediate-mode-loop.html` (App: `apps/0001-the-immediate-mode-loop`)
  - **`egui` Skill & PCB Feature**: `eframe::App::update`, `&mut self` state, `SidePanel` + `CentralPanel`, `ui.checkbox`, `ui.button().clicked()` for PCB layer toggles.
  - **Just-in-Time Rust**: Mutable borrows (`&mut`), closures (`|ui|`), and extracting pure `&mut self` state-transition methods (`isolate_top_copper`, `show_all_layers`).
  - **Cargo Verification & Mutation Coverage**: Unit-testing `isolate_top_copper()` and `show_all_layers()` with `cargo test`, then running `cargo mutants` across all 3 state methods (`reset_layers`, `isolate_top_copper`, `show_all_layers`) to verify `3 / 3 caught, 0 missed`.

### Act I: The 2D CAD Canvas & Viewport (`0002`–`0005`) — *Completed*
*Goal: Turn the empty `CentralPanel` into an interactive, multi-layer 2D board viewer while establishing pure geometry & domain unit tests verified by `cargo mutants`.*
- `0002-custom-2d-canvas-and-painter.html` (App: `apps/0002-custom-2d-canvas-and-painter`)
  - **`egui` Skill & PCB Feature**: Reserve canvas space with `ui.allocate_painter`, draw PCB pads, traces, and vias using `Pos2`, `Vec2`, `Rect`, `Color32`, and `Stroke`.
  - **Just-in-Time Rust**: Tuple structs, value semantics (`Copy` vs. move), and method chaining.
  - **Cargo Verification & Mutation Coverage**: 5 unit tests (`centered_board_rect` and `smd_pad_rect`); `cargo mutants --in-place` catches `23 / 23 viable mutants (2 unviable, 0 missed)`.
- `0003-world-to-screen-transforms-pan-and-zoom.html` (App: `apps/0003-world-to-screen-transforms-pan-and-zoom`)
  - **`egui` Skill & PCB Feature**: Map physical board coordinates (mm) to screen points (`egui::emath::TSTransform`), drag to pan (`Sense::drag`), and zoom toward the cursor (`ui.input`).
  - **Just-in-Time Rust**: Pure functions, `impl` methods on a custom `BoardViewport` struct, and closures (`|i|`).
  - **Cargo Verification & Mutation Coverage**: 6 unit tests verifying `world_to_screen` ↔ `screen_to_world` round-trips, `pan_by`, and cursor-anchored `zoom_around` + clamping; `cargo mutants --in-place` catches `14 / 14 viable mutants (6 unviable, 0 missed)`.
- `0004-modeling-layers-traces-and-footprints.html` (App: `apps/0004-modeling-layers-traces-and-footprints`)
  - **`egui` Skill & PCB Feature**: Render a structured board model (`BottomCopper`, `TopCopper`, `Silkscreen`, `Ratsnest`) in deterministic Z-order based on layer visibility toggles.
  - **Just-in-Time Rust**: `Vec<T>`, `enum PcbLayer`, `match` exhaustiveness, and `for` loop borrowing (`&self.traces`).
  - **Cargo Verification & Mutation Coverage**: 9 unit tests verifying `PcbLayer::z_rank()`, `color()`, `is_layer_visible()`, and `visible_traces_in_z_order()`; `cargo mutants --in-place` catches `20 / 20 viable mutants (7 unviable, 0 missed)`.
- `0005-hit-testing-and-net-cross-probing.html` (App: `apps/0005-hit-testing-and-net-cross-probing`)
  - **`egui` Skill & PCB Feature**: Hover and click pads on the canvas (`Sense::click_and_drag()`), highlight the connected net across layers, and inspect its properties in the `SidePanel` Net Inspector.
  - **Just-in-Time Rust**: `Option<NetId>`, `if let Some(id) = ...`, `.or(...)`, and iterator filtering.
  - **Cargo Verification & Mutation Coverage**: 11 unit tests verifying `hit_test_net_at_mm` (hit, miss in empty space, hidden-layer rejection, and overlapping Z-order priority) and `active_net`; `cargo mutants --in-place` catches `25 / 25 viable mutants (10 unviable, 0 missed)`.

### Act II: Schematic Views, Layout & Inspection (`0006`–`0009`) — *Completed*
*Goal: Add schematic symbols, dual-pane cross-probing, and searchable BOM tables while introducing headless UI widget & interaction testing with `egui_kittest`.*
- `0006-schematic-symbols-and-text-rendering.html` (App: `apps/0006-schematic-symbols-and-text-rendering`)
  - **`egui` Skill & PCB Feature**: Draw IC packages, pin numbers, net labels, and reference designators (`U1`, `R12`, `J1`) using `Painter::text`, `FontId`, and `Align2`.
  - **Just-in-Time Rust**: `String` vs. `&str` string slices and `format!()`.
  - **Cargo Verification & Mutation Coverage**: Introduced [`egui_kittest::Harness`](https://docs.rs/egui_kittest/latest/egui_kittest/) in `[dev-dependencies]`, removed `<impl eframe::App for PcbReviewerApp>::update` from `.cargo/mutants.toml` exclusions, and verified 14 unit + headless UI tests (`35 / 35 viable mutants caught, 10 unviable, 0 missed`).
- `0007-split-viewports-and-egui-scene.html` (App: `apps/0007-split-viewports-and-egui-scene`)
  - **`egui` Skill & PCB Feature**: Split Schematic + PCB viewports (`TopBottomPanel`, `egui::Scene`), clicking a schematic symbol button to center the PCB camera rect (`pcb_scene_rect`) on that footprint.
  - **Just-in-Time Rust**: Shared immutable state (`&BoardModel`) vs. scoped mutable view state (`&mut ViewState`).
  - **Cargo Verification & Mutation Coverage**: 16 unit + `egui_kittest` tests verifying cross-probe selection and camera centering (`40 / 40 viable mutants caught, 11 unviable, 0 missed`).
- `0008-searchable-netlist-and-bom-inspector.html` (App: `apps/0008-searchable-netlist-and-bom-inspector`)
  - **`egui` Skill & PCB Feature**: Filterable component/net list using `TextEdit::singleline`, `ScrollArea::vertical().show_rows(...)`, and `egui::Grid`.
  - **Just-in-Time Rust**: Iterator adapters (`.filter()`), case-insensitive string matching, and slices (`&[T]`).
  - **Cargo Verification & Mutation Coverage**: 17 unit + `egui_kittest` tests verifying case-insensitive BOM query filtering across all 4 fields (`47 / 47 viable mutants caught, 12 unviable, 0 missed`).
- `0009-keyboard-shortcuts-and-point-to-point-ruler.html` (App: `apps/0009-keyboard-shortcuts-and-point-to-point-ruler`)
  - **`egui` Skill & PCB Feature**: Bind hotkeys (`F` to fit board, `M` for measurement tool, `Esc` to clear) via `ctx.input_mut` + `consume_key`, and draw a live $\Delta x / \Delta y$ / Euclidean distance CAD ruler overlay.
  - **Just-in-Time Rust**: State machines with data-carrying enums (`enum ActiveTool { Select, Measure { start_mm: Option<Pos2>, end_mm: Option<Pos2> } }`).
  - **Cargo Verification & Mutation Coverage**: 19 unit + `egui_kittest` tests simulating key presses (`Key::M`, `Key::Escape`, `Key::F`) and verifying Euclidean $\Delta x / \Delta y$ / hypotenuse math (`59 / 59 viable mutants caught, 14 unviable, 0 missed`).

### Act III: Design Review Workflows (`0010`–`0013`) — *Completed*
*Goal: Turn the viewer into an engineering review tool with annotations, DRC markers, and custom widgets, testing multi-step UI workflows and custom `egui::Widget` traits.*
- `0010-review-pins-and-sticky-note-callouts.html` (App: `apps/0010-review-pins-and-sticky-note-callouts`)
  - **`egui` Skill & PCB Feature**: Drop review comments anchored to `(x, y)` board coordinates and render interactive floating overlays (`egui::Window`, `Id::new`).
  - **Just-in-Time Rust**: `HashMap<PinId, ReviewNote>`, unique IDs, and the `Entry` API (`Entry::Vacant` vs. `Entry::Occupied`).
  - **Cargo Verification & Mutation Coverage**: 21 unit + `egui_kittest` tests verifying pin insertion, duplicate-ID rejection in `HashMap`, deterministic sorting, and floating callout window lifecycle (`72 / 72 viable mutants caught, 16 unviable, 0 missed`).
- `0011-review-status-workflow-and-popups.html` (App: `apps/0011-review-status-workflow-and-popups`)
  - **`egui` Skill & PCB Feature**: Triage review items (`Open` → `Resolved` → `Verified`), filter by severity (`PinSeverity::ALL`), and build interactive triage controls (`ui.selectable_value`).
  - **Just-in-Time Rust**: Custom methods on enums, `#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]`, and methods returning static slices (`&'static [Self]`).
  - **Cargo Verification & Mutation Coverage**: 22 unit + `egui_kittest` tests verifying all $3 \times 3$ review status transitions and multi-criteria filtering (`severity × status`), catching all boolean combinator mutations (`88 / 88 viable mutants caught, 17 unviable, 0 missed`).
- `0012-drc-violation-overlay-and-minimap.html` (App: `apps/0012-drc-violation-overlay-and-minimap`)
  - **`egui` Skill & PCB Feature**: Render Design Rule Check error markers (copper pad clearance halos) and clamp a bird's-eye minimap camera rectangle.
  - **Just-in-Time Rust**: Bounding-box reduction (`Iterator::reduce(Rect::union)`) and coordinate clamping (`f32::clamp`).
  - **Cargo Verification & Mutation Coverage**: 23 unit + `egui_kittest` tests verifying minimum copper-to-copper clearance detection (including exact `7.6 mm` boundary assertions catching `<` vs. `<=`) and minimap coordinate clamping (`125 / 125 viable mutants caught, 20 unviable, 0 missed`).
- `0013-custom-layer-swatch-widget-and-design-tokens.html` (App: `apps/0013-custom-layer-swatch-widget-and-design-tokens`)
  - **`egui` Skill & PCB Feature**: Extract a reusable PCB layer row widget (`LayerSwatch<'a>`: color swatch + visibility checkbox + opacity slider) implementing `egui::Widget`.
  - **Just-in-Time Rust**: Implementing a foreign trait (`impl<'a> egui::Widget for LayerSwatch<'a>`) and struct lifetimes (`'a`).
  - **Cargo Verification & Mutation Coverage**: 24 unit + `egui_kittest` tests mounting `LayerSwatch<'a>` in isolation via `egui_kittest::Harness::new_ui_state` and inside the full `PcbReviewerApp` (`134 / 134 viable mutants caught, 21 unviable, 0 missed`).

### Act IV: Scaling Up — Engine-First Architecture & Persistence (`0014`–`0017`) — *Completed*
*Goal: Refactor the growing prototype into a clean multi-crate architecture (`egui-app-architecture` & `egui-cow-document-model`) with static lint gates, `proptest` property suites, and per-crate `cargo mutants -p <crate>`.*
- `0014-multi-crate-workspace-doc-vs-ui-egui.html` (App: `apps/0014-multi-crate-workspace-doc-vs-ui-egui`)
  - **`egui` Skill & PCB Feature**: Split pure PCB/review domain data (`L1` `crates/pcb-doc`, depending only on `emath`) from `egui` rendering (`L6`/`L7` `pcb-reviewer`) so the core model compiles headlessly without UI dependencies.
  - **Just-in-Time Rust**: Cargo `[workspace]`, module visibility (`pub`, `pub(crate)`), and strict dependency layering (`validate_l1_manifest_no_ui_deps`).
  - **Cargo Verification & Mutation Coverage**: Added Tier 1 static verification gates (`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, `#![forbid(unsafe_code)]`, `validate_l1_manifest_no_ui_deps`) and verified 11 workspace tests (`98 / 98 caught` in `pcb-doc`, `46 / 46 caught` in `pcb-reviewer`; `144 / 144 viable mutants caught, 21 unviable, 0 missed`).
- `0015-loading-board-and-review-files-serde.html` (App: `apps/0015-loading-board-and-review-files-serde`)
  - **`egui` Skill & PCB Feature**: Serialize and deserialize board layouts and review sessions to versioned JSON (`ReviewSessionEnvelope`), surfacing dismissible `SessionIoError` banners in `SidePanel::right` without panicking.
  - **Just-in-Time Rust**: `#[derive(Serialize, Deserialize)]` (`emath` `"serde"` feature), `Result<T, E>`, exhaustive struct destructuring, and `thiserror`.
  - **Cargo Verification & Mutation Coverage**: 12 workspace tests verifying lossless `to_session_json -> from_session_json` round-trips, `CURRENT_SCHEMA_VERSION` validation, empty `board_id` rejection, and corrupt JSON recovery (`155 / 155 viable mutants caught, 21 unviable, 0 missed`).
- `0016-copy-on-write-arc-document-snapshots.html` (App: `apps/0016-copy-on-write-arc-document-snapshots`)
  - **`egui` Skill & PCB Feature**: Store `BoardModel` collections in `Arc<Vec<...>>` and `Arc<HashMap<...>>` (`serde` `"rc"` feature) so taking or restoring a baseline snapshot in `SidePanel::right` costs only 4 pointer copies (`O(1)`), with live `Shared Arc Buffers: X / 4` telemetry.
  - **Just-in-Time Rust**: Smart pointers (`Arc<T>`), `Arc::make_mut` copy-on-write mutation, `Arc::ptr_eq`, and `proptest!`.
  - **Cargo Verification & Mutation Coverage**: 14 workspace tests (including `prop_cow_pin_mutation_preserves_baseline_snapshot_and_shares_untouched_arcs`) verifying `4 -> 3 -> 2 -> 1 -> 0` pointer sharing (`169 / 169 viable mutants caught, 21 unviable, 0 missed`).
- `0017-undo-redo-history-and-slider-coalescing.html` (App: `apps/0017-undo-redo-history-and-slider-coalescing`)
  - **`egui` Skill & PCB Feature**: Wire `Z` / `Y` hotkeys, `Undo (Z)` / `Redo (Y)` buttons in `SidePanel::right`, and coalesce continuous 60-FPS `egui::Slider` pin X drags into a single undo step via `coalesce_key` and `Response::drag_stopped()`.
  - **Just-in-Time Rust**: Bounded `VecDeque<HistoryEntry>` ring buffer, `std::mem::replace` zero-clone state swaps, and randomized drag-sequence `proptest!`.
  - **Cargo Verification & Mutation Coverage**: 16 workspace tests (including `prop_coalesced_drag_restores_exact_initial_state_in_single_undo`) verifying coalescing, `commit_coalesce()`, redo clearing, and ring-buffer eviction (`217 / 217 viable mutants caught, 22 unviable, 0 missed`).

### Act V: Command Engine, Performance & Polish (`0018`–`0021`) — *Completed*
*Goal: Unify menus/shortcuts/CLI into a Command Engine (`egui-command-engine` & `egui-app-runner`), add `panic_hunt` command fuzzing, enforce performance budgets, and run offscreen GPU visual snapshot tests.*
- `0018-the-command-pattern-and-command-palette.html` (App: `apps/0018-the-command-pattern-and-command-palette`)
  - **`egui` Skill & PCB Feature**: Route menu items, toolbar buttons, and a searchable `Key::K` Command Palette modal (`egui::Window`) through a single `CommandSpec` registry (`session.inspect`, `review.addPin`, `review.setStatus`, `review.clearPins`).
  - **Just-in-Time Rust**: Static slices (`&'static [CommandSpec]`), typed `EngineError` via `thiserror`, `serde_json::Value` parameter inspection, and `COMMAND_PARITY_FLOOR`.
  - **Cargo Verification & Mutation Coverage**: 17 workspace tests including the Tier 5 `panic_hunt` adversarial JSON fuzzer (`command_registry_dispatch_and_panic_hunt_fuzzer_preserve_invariants`) feeding malformed/non-object/negative/huge JSON payloads into every registered `CommandSpec` (`238 / 238 viable mutants caught, 24 unviable, 0 missed`).
- `0019-viewport-culling-and-cached-tessellation.html` (App: `apps/0019-viewport-culling-and-cached-tessellation`)
  - **`egui` Skill & PCB Feature**: Spatial viewport culling (`Rect::intersects`) skipping offscreen pads and stroke-expanded traces before submitting `Painter` commands, plus live `ViewportCullStats` telemetry in `SidePanel::right`.
  - **Just-in-Time Rust**: Stroke-expanded axis-aligned bounding boxes (`TraceSegment::bounds_mm`), `ViewportCullStats::culled_count()`, `check_cull_budget(elapsed_micros, max_budget_micros)`, and `proptest!`.
  - **Cargo Verification & Mutation Coverage**: 19 workspace tests (including `prop_spatial_culling_matches_reference_and_preserves_counts`) proving spatial culling equivalence and budget gate boundaries (`255 / 255 viable mutants caught, 29 unviable, 0 missed`).
- `0020-non-blocking-file-import-and-progress-ui.html` (App: `apps/0020-non-blocking-file-import-and-progress-ui`)
  - **`egui` Skill & PCB Feature**: Parse JSON board sessions on a background worker thread while polling non-blockingly (`Receiver::try_recv`), rendering `egui::ProgressBar`, and waking the UI loop via `ctx.request_repaint()`.
  - **Just-in-Time Rust**: `std::thread::spawn`, `std::sync::mpsc::{channel, Receiver}`, `ImportJobMessage`, `ImportJobState`, and cancellation by dropping the `Receiver`.
  - **Cargo Verification & Mutation Coverage**: 20 workspace tests deterministically testing multi-stage progress fractions, channel disconnection when the UI drops the receiver, and corrupt JSON worker recovery (`269 / 269 viable mutants caught, 33 unviable, 0 missed`).
- `0021-headless-snapshot-testing-and-wasm-build.html` (App: `apps/0021-headless-snapshot-testing-and-wasm-build`)
  - **`egui` Skill & PCB Feature**: Serialize GPU/snapshot tests via `gpu_lock()`, compute 64-bit FNV-1a `FrameSnapshotDigest` hashes over the visible layer bitmask (`visible_layer_mask`), culled geometry, and DRC violations, display `Target: native-desktop` / `wasm32-unknown-unknown`, and evaluate release readiness with `QualityScorecard`.
  - **Just-in-Time Rust**: `std::sync::OnceLock<Mutex<()>>`, panic-free lock recovery via `PoisonError::into_inner`, 64-bit FNV-1a hashing, and `#[cfg(target_arch = "wasm32")]`.
  - **Cargo Verification & Mutation Coverage**: 21 workspace tests (`15` in `pcb-doc`, `6` in `pcb-reviewer`) verifying `gpu_lock()`, `FrameSnapshotDigest`, `visible_layer_mask()`, and `QualityScorecard::is_release_ready()` (`304 / 304 viable mutants caught, 39 unviable, 0 missed`).

### Act VI: Real-World KiCad Parsing (`nom`) & Embedded SQL (`turso` / `libsql`) (`0022`–`0025`)
*Goal: Parse real-world KiCad `.kicad_pcb` and `.kicad_sch` files with `nom` in an `L0` standalone parser crate, verify against pinned real-file corpus oracles, and persist/sync multi-engineer review threads in an embedded Turso (`libsql`) SQL database.*
- `0022-sexpr-tokenizer-and-ast-with-nom.html`
  - **`egui` Skill & PCB Feature**: Build an interactive S-expression AST inspector panel (`CollapsingHeader` tree view) visualizing raw KiCad `(kicad_pcb ...)` nodes parsed by a standalone `L0` `kicad-sexpr` crate.
  - **Just-in-Time Rust**: Parser combinators (`nom::IResult`, `delimited`, `multispace0`, `alt`, `many0`), zero-copy string slices (`&'a str`), and recursive enums (`Box<Sexpr<'a>>`).
  - **Cargo Verification & Mutation Coverage**: Fuzz and unit-test `kicad-sexpr` against deeply nested, unclosed, and non-UTF8/escaped inputs; run `cargo mutants -p kicad-sexpr` to guarantee 100% of grammar combinator branches are asserted.
- `0023-parsing-kicad-pcb-and-sch-into-domain-model.html`
  - **`egui` Skill & PCB Feature**: Lower KiCad S-expression trees into typed `Footprint`, `Pad`, `Segment`, and schematic `Symbol`/`Wire` structs (`L4` `io` crate) to render real open-source KiCad boards on the 2D canvas.
  - **Just-in-Time Rust**: `TryFrom` / `FromStr` conversions, typed parse diagnostics with source spans, and fallible allocation (`Vec::try_reserve`).
  - **Cargo Verification & Mutation Coverage**: Build a Tier 4 pinned KiCad corpus oracle harness (`pass_floor` and `roundtrip_floor` ratchets + synthetic regression tests) and run `cargo mutants -p pcb-io` on coordinate, layer, and net-lowering logic.
- `0024-embedded-sql-review-store-with-turso.html`
  - **`egui` Skill & PCB Feature**: Persist engineering review sessions, anchored comment threads, and DRC sign-off statuses in a local embedded Turso (`libsql`) SQL database with schema migrations and instant SQL search/filtering in the `egui` inspector.
  - **Just-in-Time Rust**: Async Rust fundamentals (`async`/`await`, `tokio` runtime bridge for `egui`), parameterized SQL queries (`params!`), and row mapping.
  - **Cargo Verification & Mutation Coverage**: Write async integration tests (`#[tokio::test]`) against ephemeral `:memory:` / temp `libsql::Builder::new_local` databases verifying idempotent migrations, foreign-key constraints, and SQL query filters under `cargo mutants`.
- `0025-local-first-review-sync-and-audit-trail.html`
  - **`egui` Skill & PCB Feature**: Add a live database sync indicator, offline-first review merge view, and an interactive SQL audit-log timeline comparing engineering sign-offs across board revisions.
  - **Just-in-Time Rust**: Turso embedded replica sync (`Database::sync`), SQL transactions (`Transaction`), and thread-safe async state channels.
  - **Cargo Verification & Mutation Coverage**: Test atomic transaction rollback on failure, concurrent engineer review merges, tamper-evident audit-log ordering, and compile our workspace `docs/scorecard.md` summarizing test, corpus, perf, and `cargo mutants` coverage across the workspace.

### Act VII: Agentic Hardware Review & Model Context Protocol (`rmcp`) (`0026`–`0029`)
*Goal: Turn the PCB Reviewer into an AI-controllable engineering co-pilot via the official `rmcp` SDK (`egui-app-runner` & `egui-command-engine`), adding LLM-native spatial/topological queries, Headless and Bridged live-GUI MCP servers, and a companion **Mock MCP Client & Inspector Workbench** app to test and drive multi-step agent playbooks.*
- `0026-llm-native-spatial-and-electrical-queries.html`
  - **`egui` Skill & PCB Feature**: Equip the engine (`L2`/`L5`) with high-leverage queries that LLMs struggle to compute from raw S-expressions: spatial neighborhood radius lookups (`pcb.query_spatial_neighborhood` returning nearby pads/components and edge-to-edge copper gaps around a target pin like `U1.VDD`), net-topology summaries (`pcb.query_net_topology`), and parameterized custom DRC/decoupling checks (`pcb.run_custom_drc`), exposed in an interactive **Query Preview** inspector tab.
  - **Just-in-Time Rust**: `schemars::JsonSchema` derive macros, structured JSON Schema 2020-12 introspection, and trait-based query dispatch.
  - **Cargo Verification & Mutation Coverage**: Unit + `proptest!` suites on radius/topology queries and strict JSON Schema parameter validation; `100%` caught mutants in `pcb-doc` / `pcb-engine` via `cargo mutants`.
- `0027-headless-mcp-server-rmcp-and-mock-client.html`
  - **`egui` Skill & PCB Feature**: Implement `PcbMcpServer` in `crates/pcb-automation` (`L6`) using `rmcp` (`#[tool_router]`, `#[tool]`, `#[tool_handler]`), exposing `CommandSpec` execution, read-only `kicad://board/summary`, `kicad://net/{name}`, and `kicad://bom` MCP **Resources**, and engineering review **Prompts** (`review_power_integrity`, `triage_open_drc`), paired with a programmatic **Mock MCP Client** driving a headless in-process `Session`.
  - **Just-in-Time Rust**: Async trait implementations (`rmcp::ServerHandler`), `Arc<Mutex<Session>>`, `Parameters<T>`, and bidirectional in-memory `tokio::io::duplex` streams.
  - **Cargo Verification & Mutation Coverage**: Deterministic client$\leftrightarrow$server round-trip integration tests over `tokio::io::duplex` verifying `list_tools`, `read_resource`, `get_prompt`, `call_tool`, and structured MCP error codes under `cargo mutants -p pcb-automation`.
- `0028-bridged-mcp-control-channel-and-live-gui-snapshots.html`
  - **`egui` Skill & PCB Feature**: Add an authenticated loopback TCP control channel (`127.0.0.1` + bearer token file) to the `egui` desktop app so `PcbMcpServer` in `--bridge` mode can pan/zoom the live 2D CAD camera (`ui.focus_region`), toggle layers, drop review pins, and capture cropped board region PNG snapshots returned as MCP `Content::image` payloads for multimodal inspection.
  - **Just-in-Time Rust**: Non-blocking TCP command polling on the `egui` immediate-mode frame loop (`try_recv` + `ctx.request_repaint()`), constant-time bearer token verification, and capability-scoped filesystem paths.
  - **Cargo Verification & Mutation Coverage**: End-to-end bridged tests verifying unauthorized token rejection, live `egui` camera rect updates, and cropped PNG snapshot payload generation with `0 missed mutants`.
- `0029-egui-mcp-inspector-workbench-and-agent-playbooks.html`
  - **`egui` Skill & PCB Feature**: Build the companion **Mock MCP Client & Inspector Workbench** (`egui` app / dual-pane console) that connects to either the **Headless** or **Bridged Live-GUI** MCP server, featuring a live `tools/list` schema explorer, raw JSON-RPC frame inspector (`Request` $\leftrightarrow$ `Response` with latency telemetry), inline `Content::image` snapshot viewer, and one-click **Multi-Step Agent Playbooks** (e.g., *"Audit `U1` decoupling caps $\to$ Run custom DRC $\to$ Drop `Critical` pins $\to$ Focus GUI viewport"*).
  - **Just-in-Time Rust**: Async-to-GUI command/event bridges (`tokio::sync::mpsc`), state-machine playbook runners, and dual-process/in-memory transport switching.
  - **Cargo Verification & Mutation Coverage**: Tier 6 `agent_tasks` acceptance suite executing full multi-step playbooks across both Headless and Bridged backends, verified with `egui_kittest` and `cargo mutants`.

### Future Acts Under Exploration (`Act VIII+`, `0030`+)
- *The course remains open-ended beyond `0029`—future acts can explore Gerber/Excellon manufacturing output & visual copper XOR revision diffing (`Rev A` vs. `Rev B`), custom WGSL GPU shaders (`egui_wgpu::CallbackTrait`) for sub-pixel copper pour & thermal relief rendering, interactive length-tuning/impedance calculators, or multi-board panelization.*

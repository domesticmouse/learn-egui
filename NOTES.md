# Teaching Notes & Preferences

- **User Formatting Preferences**:
  - **Dark Mode**: Use dark mode across all lessons and reference documents (`<meta name="color-scheme" content="dark">`, `:root { color-scheme: dark; }`, dark Tufte palette in `./assets/style.css`, and Mermaid `theme: "dark"`, with `@media print` overrides for clean printing).
  - Always use **Prism.js** (`<pre><code class="language-rust">`, `language-toml`, `language-bash`, etc.) for syntax highlighting across all lessons and reference documents.
  - Always use **Mermaid.js** (`<pre class="mermaid">`) for architecture, lifecycle, and flow diagrams (avoid ASCII art diagrams).
- **Testing & Mutation Coverage Preference**:
  - Integrate testing gradually across the **entire narrative arc** (`0000`–`0025`), not as an isolated module near the end.
  - **Every lesson** must include concrete **Cargo Verification Prompts** (`cargo test` + `cargo mutants`) so the learner continuously verifies new code and monitors real assertion coverage via mutation testing (`Caught` vs. `Missed` mutants).
  - Use `.cargo/mutants.toml` (or `--exclude-re`) to explicitly demarcate what is currently excluded (such as the OS window launcher `fn main`) and progressively shrink exclusions as higher verification tiers (`egui_kittest` in Act II, offscreen `wgpu` snapshots in Act V) come online.
- **Target Application**: Schematic and PCB design review system in Rust + `egui`.
- **Key Ecosystem Crates & Tools Planned**:
  - `eframe` / `egui` (Immediate-mode GUI and 2D CAD canvas)
  - `cargo-mutants` (Mutation testing CLI to monitor test coverage quality across every lesson)
  - `egui_kittest` (Headless `egui` interaction harness and offscreen visual snapshot testing)
  - `proptest` (Property-based testing for coordinate math, spatial culling, and COW undo/redo)
  - `serde` / `serde_json` (Structured serialization)
  - `nom` (Parser combinators for KiCad `.kicad_pcb` and `.kicad_sch` S-expression files)
  - `turso` / `libsql` (Embedded SQL database & local-first sync for engineering review threads and DRC audit logs)
- **Rust Background**: Learning Rust alongside `egui` — introduce one Rust concept just-in-time alongside one `egui` skill and one verification/mutation step in every lesson.
- **Installed Toolchain**: `cargo 1.97.1` / `rustc 1.97.1` on macOS.

---

## Curriculum Arc (`0000`–`0025`) with Continuous Verification & Mutation Testing

### Completed Lessons (`0000`–`0001`)
- `0000-project-setup-and-eframe-boot.html`
  - **`egui` Skill & PCB Feature**: Creating the Cargo crate (`Cargo.toml`, `eframe = "0.33"`, dev-profile optimization) and booting the native window with `eframe::run_native`.
  - **Just-in-Time Rust**: `impl Default`, struct update syntax (`..Default::default()`), `Box::new`, and `#[cfg(test)]` / `#[test]` / `assert!` / `assert_eq!`.
  - **Cargo Verification & Mutation Coverage**: First `#[test]` verifying `PcbReviewerApp::default()` initial layer flags (`show_top_copper == true`, `show_ratsnest == false`); installing `cargo-mutants`, configuring `.cargo/mutants.toml` to skip only the OS entrypoint `main`, and catching all 4 boolean-field mutants in `Default::default()`.
- `0001-the-immediate-mode-loop.html`
  - **`egui` Skill & PCB Feature**: `eframe::App::update`, `&mut self` state, `SidePanel` + `CentralPanel`, `ui.checkbox`, `ui.button().clicked()` for PCB layer toggles.
  - **Just-in-Time Rust**: Mutable borrows (`&mut`), closures (`|ui|`), and extracting pure `&mut self` state-transition methods (`isolate_top_copper`, `show_all_layers`).
  - **Cargo Verification & Mutation Coverage**: Unit-testing `isolate_top_copper()` and `show_all_layers()` with `cargo test`, then running `cargo mutants` to see how a weak test leaves a **Missed Mutant** when `show_top_copper = true` is mutated, and tightening assertions to reach `0 missed mutants`.

### Act I: The 2D CAD Canvas & Viewport (`0002`–`0005`)
*Goal: Turn the empty `CentralPanel` into an interactive, multi-layer 2D board viewer while establishing pure geometry & domain unit tests verified by `cargo mutants`.*
- `0002-custom-2d-canvas-and-painter.html`
  - **`egui` Skill & PCB Feature**: Reserve canvas space with `ui.allocate_painter`, draw PCB pads, traces, and vias using `Pos2`, `Vec2`, `Rect`, `Color32`, and `Stroke`.
  - **Just-in-Time Rust**: Tuple structs, value semantics (`Copy` vs. move), and method chaining.
  - **Cargo Verification & Mutation Coverage**: Test pad/board `Rect` geometry helpers (center-to-bounds calculation and aspect containment) with `cargo test`; run `cargo mutants` to catch arithmetic `+`/`-` and `*`/`/` sign/factor mutations.
- `0003-world-to-screen-transforms-pan-and-zoom.html`
  - **`egui` Skill & PCB Feature**: Map physical board coordinates (mm) to screen points (`egui::emath::TSTransform`), drag to pan (`Sense::drag`), and zoom toward the cursor (`ui.input`).
  - **Just-in-Time Rust**: Pure functions, `impl` methods on a custom `Viewport` struct, and closures (`|i|`).
  - **Cargo Verification & Mutation Coverage**: Write round-trip coordinate tests (`world_to_screen` ↔ `screen_to_world`) and cursor-anchored zoom invariant tests; use `cargo mutants` to verify zoom-clamping boundary conditions (`<` vs. `<=`).
- `0004-modeling-layers-traces-and-footprints.html`
  - **`egui` Skill & PCB Feature**: Render a structured board model (Top/Bottom Copper, Silkscreen, Solder Mask) in deterministic Z-order based on layer visibility toggles.
  - **Just-in-Time Rust**: `Vec<T>`, `enum PcbLayer`, `match` exhaustiveness, and `for` loop borrowing (`&self.traces`).
  - **Cargo Verification & Mutation Coverage**: Test `PcbLayer` Z-order sorting and visibility filtering; run `cargo mutants` to ensure every `match` arm on `PcbLayer` is asserted by a test (zero surviving match-arm deletions).
- `0005-hit-testing-and-net-cross-probing.html`
  - **`egui` Skill & PCB Feature**: Hover and click traces/pads on the canvas, highlight the entire connected net, and inspect its properties in the `SidePanel`.
  - **Just-in-Time Rust**: `Option<NetId>`, `if let Some(id) = ...`, and iterator filtering (`.iter().find(...)`).
  - **Cargo Verification & Mutation Coverage**: Test point-to-pad and point-to-segment distance hit-testing (`Some(NetId)` vs. `None` at exact clearance thresholds); verify with `cargo mutants` that boundary-distance mutants are caught.

### Act II: Schematic Views, Layout & Inspection (`0006`–`0009`)
*Goal: Add schematic symbols, dual-pane cross-probing, and searchable BOM tables while introducing headless UI widget & interaction testing with `egui_kittest`.*
- `0006-schematic-symbols-and-text-rendering.html`
  - **`egui` Skill & PCB Feature**: Draw IC packages, pin numbers, net labels, and reference designators (`U1`, `R12`) using `Painter::text`, `FontId`, and `Align2`.
  - **Just-in-Time Rust**: `String` vs. `&str` string slices and `format!()`.
  - **Cargo Verification & Mutation Coverage**: Introduce [`egui_kittest::Harness`](https://docs.rs/egui_kittest/latest/egui_kittest/) in `[dev-dependencies]`! Boot our `eframe::App` headlessly inside `cargo test`, un-skip UI panel rendering in `.cargo/mutants.toml`, and verify rendered reference-designator labels so `cargo mutants` catches deleted UI label calls.
- `0007-split-viewports-and-egui-scene.html`
  - **`egui` Skill & PCB Feature**: Side-by-side or tabbed Schematic + PCB viewports (`TopBottomPanel`, `egui::Scene`), clicking a schematic pin to pan/zoom the PCB view to that footprint.
  - **Just-in-Time Rust**: Shared immutable state (`&Board`) vs. scoped mutable view state (`&mut ViewState`).
  - **Cargo Verification & Mutation Coverage**: Drive a cross-probe selection in `egui_kittest` and assert that selecting `U1` in the schematic updates the PCB camera rect; verify with `cargo mutants` that cross-probe state synchronization cannot be stubbed out.
- `0008-searchable-netlist-and-bom-inspector.html`
  - **`egui` Skill & PCB Feature**: Filterable component/net list using `TextEdit::singleline`, `ScrollArea::vertical().show_rows(...)` (virtualized scrolling for 10,000+ pins), and `Grid`.
  - **Just-in-Time Rust**: Iterator adapters (`.filter()`, `.enumerate()`), string matching, and slices (`&[T]`).
  - **Cargo Verification & Mutation Coverage**: Test case-insensitive BOM query filtering (`"u1"`, `"GND"`, empty query) via unit + `egui_kittest` harness tests; run `cargo mutants` to catch filter-predicate negations (`!` flips).
- `0009-keyboard-shortcuts-and-point-to-point-ruler.html`
  - **`egui` Skill & PCB Feature**: Bind hotkeys (`F` to fit board, `M` for measurement tool, `Esc` to clear) via `ctx.input_mut` + `consume_key`, and draw a live Δx/Δy/distance CAD ruler overlay.
  - **Just-in-Time Rust**: State machines with data-carrying enums (`enum Tool { Select, Measure { start: Option<Pos2> } }`).
  - **Cargo Verification & Mutation Coverage**: Simulate key presses (`harness.key_press(egui::Key::M)`, `Key::Escape`, `Key::F`) in `egui_kittest` and verify Euclidean Δx/Δy/hypotenuse math; confirm `cargo mutants` catches every state-transition and hotkey branch mutant.

### Act III: Design Review Workflows (`0010`–`0013`)
*Goal: Turn the viewer into an engineering review tool with annotations, DRC markers, and custom widgets, testing multi-step UI workflows and custom `egui::Widget` traits.*
- `0010-review-pins-and-sticky-note-callouts.html`
  - **`egui` Skill & PCB Feature**: Drop review comments anchored to `(x, y)` board coordinates and render interactive floating overlays (`egui::Area`, `egui::Window`, `Id::new`).
  - **Just-in-Time Rust**: `HashMap<PinId, ReviewNote>`, unique IDs, and the Entry API.
  - **Cargo Verification & Mutation Coverage**: Test pin insertion, coordinate lookup, and duplicate-ID rejection in `HashMap`; run `cargo mutants` on pin-store methods to ensure no silent overwrites or dropped notes survive.
- `0011-review-status-workflow-and-popups.html`
  - **`egui` Skill & PCB Feature**: Triage review items (`Open` → `Resolved` / `Verified`), filter by author/severity, and build context menus (`Response::context_menu`, `ComboBox`).
  - **Just-in-Time Rust**: Custom traits, `#[derive(PartialEq, Eq, Clone, Copy)]`, and methods returning static slices.
  - **Cargo Verification & Mutation Coverage**: Test valid vs. invalid review status transitions and multi-criteria filtering (severity × status); run `cargo mutants` to verify boolean `&&` vs. `||` filter combinator mutations are caught.
- `0012-drc-violation-overlay-and-minimap.html`
  - **`egui` Skill & PCB Feature**: Render Design Rule Check error markers (clearance/unrouted net halos) and a corner bird's-eye minimap showing the active camera rectangle.
  - **Just-in-Time Rust**: Bounding-box math, `Iterator::fold`, and coordinate clamping (`f32::clamp`).
  - **Cargo Verification & Mutation Coverage**: Test minimum copper-to-copper clearance detection and minimap coordinate clamping at board edges; use `cargo mutants` to catch off-by-epsilon clearance threshold mutations (`<` vs. `<=`).
- `0013-custom-layer-swatch-widget-and-design-tokens.html`
  - **`egui` Skill & PCB Feature**: Extract a reusable PCB layer row widget (color swatch + visibility eye + opacity slider) implementing `egui::Widget`, styled with dark-mode CAD tokens.
  - **Just-in-Time Rust**: Implementing a foreign trait (`impl egui::Widget for ...`) and lifetimes (`'a`) in UI builders.
  - **Cargo Verification & Mutation Coverage**: Mount the standalone `LayerSwatch<'a>` widget directly inside `egui_kittest::Harness::new_ui`, simulate toggling visibility and dragging opacity, and run `cargo mutants` on the widget module.

### Act IV: Scaling Up — Engine-First Architecture & Persistence (`0014`–`0017`)
*Goal: Refactor the growing prototype into a clean multi-crate architecture (`egui-app-architecture` & `egui-cow-document-model`) with static lint gates, `proptest` property suites, and per-crate `cargo mutants -p <crate>`.*
- `0014-multi-crate-workspace-doc-vs-ui-egui.html`
  - **`egui` Skill & PCB Feature**: Split pure PCB/review domain data (`L1` `doc`) from `egui` rendering (`L6` `ui-egui`) so the core model compiles headlessly without UI dependencies.
  - **Just-in-Time Rust**: Cargo workspaces, module visibility (`pub`, `pub(crate)`), and strict dependency layering.
  - **Cargo Verification & Mutation Coverage**: Add Tier 1 static verification gates (`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, `unsafe_code = "forbid"`, layer check) and run fast targeted `cargo test -p pcb-doc` + `cargo mutants -p pcb-doc` (which runs in seconds because `pcb-doc` never links `egui`/`wgpu`!).
- `0015-loading-board-and-review-files-serde.html`
  - **`egui` Skill & PCB Feature**: Serialize and deserialize board layouts and review sessions to JSON, surfacing clean error banners in `egui` when a file is malformed (no panics!).
  - **Just-in-Time Rust**: `#[derive(Serialize, Deserialize)]`, `Result<T, E>`, the `?` operator, and `thiserror`.
  - **Cargo Verification & Mutation Coverage**: Test lossless `save -> load` round-trips, schema version checks, and corrupt/truncated JSON inputs returning typed `Err`s; run `cargo mutants` to ensure error-mapping and validation branches cannot be bypassed.
- `0016-copy-on-write-arc-document-snapshots.html`
  - **`egui` Skill & PCB Feature**: Store board chunks and review annotations in `Arc<T>` so taking a full document snapshot before an edit costs only pointer copies.
  - **Just-in-Time Rust**: Smart pointers (`Arc<T>`), `Arc::make_mut` copy-on-write mutation, and structural sharing.
  - **Cargo Verification & Mutation Coverage**: Introduce [`proptest`](https://docs.rs/proptest/latest/proptest/) (Tier 2 verification)! Prove that mutating a cloned `Document` via `Arc::make_mut` never alters an older snapshot and preserves `Arc::ptr_eq` on untouched layers; verify with `cargo mutants`.
- `0017-undo-redo-history-and-slider-coalescing.html`
  - **`egui` Skill & PCB Feature**: Wire `Cmd+Z` / `Cmd+Shift+Z` and a visual History panel; coalesce continuous `egui` drags/sliders into a single undo step on release.
  - **Just-in-Time Rust**: `VecDeque<HistoryState>`, snapshot diffing/revision counters, and drag-lifecycle detection.
  - **Cargo Verification & Mutation Coverage**: Write a stateful `proptest!` sequence generator (`ApplyEdit`, `DragStep`, `CommitDrag`, `Undo`, `Redo`) verifying history invariants, and run `cargo mutants` on the history engine to achieve `0 missed mutants`.

### Act V: Command Engine, Performance & Polish (`0018`–`0021`)
*Goal: Unify menus/shortcuts/CLI into a Command Engine (`egui-command-engine` & `egui-app-runner`), add `panic_hunt` command fuzzing, enforce performance budgets, and run offscreen GPU visual snapshot tests.*
- `0018-the-command-pattern-and-command-palette.html`
  - **`egui` Skill & PCB Feature**: Route menu items, toolbar buttons, and a fuzzy-searchable `Cmd+K` modal through a single `CommandSpec` registry (`layer.toggle`, `review.addPin`).
  - **Just-in-Time Rust**: Function pointers (`fn(&mut Session, &Value) -> Result<Value>`), static registries, and dynamic dispatch.
  - **Cargo Verification & Mutation Coverage**: Build the Tier 5 `panic_hunt` adversarial command fuzzer (feeding arbitrary/malformed JSON params into every registered `CommandSpec` and asserting zero panics or hangs) + menu parity ratchet (`parity::FLOOR`), verified with `cargo mutants -p pcb-engine`.
- `0019-viewport-culling-and-cached-tessellation.html`
  - **`egui` Skill & PCB Feature**: Keep 60 FPS on dense boards (50,000+ pads/segments) by spatial viewport culling (`Rect::intersects`) and batching geometry into `Vec<egui::Shape>`.
  - **Just-in-Time Rust**: Spatial grid indexing, pre-allocation (`Vec::with_capacity`), and profiling hot loops.
  - **Cargo Verification & Mutation Coverage**: Prove spatial-index equivalence (`culled_query(rect) == brute_force_filter(rect)`) with `proptest!`, add a Tier 7 frame-time performance budget check (`p50`/`p95` ms), and run `cargo mutants` on the spatial grid.
- `0020-non-blocking-file-import-and-progress-ui.html`
  - **`egui` Skill & PCB Feature**: Parse large board files on a background worker while keeping the `egui` window responsive with a live progress bar and `ctx.request_repaint()`.
  - **Just-in-Time Rust**: Concurrency with `std::thread::spawn`, message passing (`std::sync::mpsc::channel`), and `Send + 'static`.
  - **Cargo Verification & Mutation Coverage**: Deterministically test background worker progress reporting, clean cancellation on drop, and channel disconnection handling in `cargo test` + `cargo mutants`.
- `0021-headless-snapshot-testing-and-wasm-build.html`
  - **`egui` Skill & PCB Feature**: Render the full PCB canvas + inspector offscreen with `wgpu` to capture pixel-accurate PNG visual regression baselines and compile the reviewer for WebAssembly (`wasm32-unknown-unknown`).
  - **Just-in-Time Rust**: Conditional compilation (`#[cfg(target_arch = "wasm32")]`), feature flags, and process-wide GPU serialization (`testkit::gpu_lock()`).
  - **Cargo Verification & Mutation Coverage**: Implement Tier 3 & Tier 6 offscreen `wgpu` snapshot diffing guarded by `let _gpu = testkit::gpu_lock();` (so even visual shader/color mutations get caught!) and verify `cargo check --target wasm32-unknown-unknown`.

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
  - **Cargo Verification & Mutation Coverage**: Test atomic transaction rollback on failure, concurrent engineer review merges, tamper-evident audit-log ordering, and compile our final self-auditing `docs/scorecard.md` summarizing test, corpus, perf, and `cargo mutants` coverage across the entire workspace.

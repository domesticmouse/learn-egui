# Teaching Notes & Preferences

- **User Formatting Preferences**:
  - **Dark Mode**: Use dark mode across all lessons and reference documents (`<meta name="color-scheme" content="dark">`, `:root { color-scheme: dark; }`, dark Tufte palette in `./assets/style.css`, and Mermaid `theme: "dark"`, with `@media print` overrides for clean printing).
  - Always use **Prism.js** (`<pre><code class="language-rust">`, `language-toml`, `language-bash`, etc.) for syntax highlighting across all lessons and reference documents.
  - Always use **Mermaid.js** (`<pre class="mermaid">`) for architecture, lifecycle, and flow diagrams (avoid ASCII art diagrams).
- **Target Application**: Schematic and PCB design review system in Rust + `egui`.
- **Rust Background**: Learning Rust alongside `egui` — introduce one Rust concept just-in-time alongside one `egui` skill in every lesson.
- **Installed Toolchain**: `cargo 1.97.1` / `rustc 1.97.1` on macOS.

---

## Curriculum Arc (`0000`–`0021`)

### Completed Lessons
- `0000-project-setup-and-eframe-boot.html`: Creating the Cargo crate (`Cargo.toml`, `eframe = "0.33"`, dev-profile optimization), `#[derive(Default)]`, and booting the native window with `eframe::run_native`.
- `0001-the-immediate-mode-loop.html`: `eframe::App::update`, `&mut self` state, `SidePanel` + `CentralPanel`, `ui.checkbox`, `ui.button().clicked()` for PCB layer toggles.

### Act I: The 2D CAD Canvas & Viewport (`0002`–`0005`)
*Goal: Turn the empty `CentralPanel` into an interactive, multi-layer 2D board viewer.*
- `0002-custom-2d-canvas-and-painter.html`
  - **`egui` Skill & PCB Feature**: Reserve canvas space with `ui.allocate_painter`, draw PCB pads, traces, and vias using `Pos2`, `Vec2`, `Rect`, `Color32`, and `Stroke`.
  - **Just-in-Time Rust**: Tuple structs, value semantics (`Copy` vs. move), and method chaining.
- `0003-world-to-screen-transforms-pan-and-zoom.html`
  - **`egui` Skill & PCB Feature**: Map physical board coordinates (mm) to screen points (`egui::emath::TSTransform`), drag to pan (`Sense::drag`), and zoom toward the cursor (`ui.input`).
  - **Just-in-Time Rust**: Pure functions, `impl` methods on a custom `Viewport` struct, and closures (`|i|`).
- `0004-modeling-layers-traces-and-footprints.html`
  - **`egui` Skill & PCB Feature**: Render a structured board model (Top/Bottom Copper, Silkscreen, Solder Mask) in deterministic Z-order based on layer visibility toggles.
  - **Just-in-Time Rust**: `Vec<T>`, `enum PcbLayer`, `match` exhaustiveness, and `for` loop borrowing (`&self.traces`).
- `0005-hit-testing-and-net-cross-probing.html`
  - **`egui` Skill & PCB Feature**: Hover and click traces/pads on the canvas, highlight the entire connected net, and inspect its properties in the `SidePanel`.
  - **Just-in-Time Rust**: `Option<NetId>`, `if let Some(id) = ...`, and iterator filtering (`.iter().find(...)`).

### Act II: Schematic Views, Layout & Inspection (`0006`–`0009`)
*Goal: Add schematic symbols, dual-pane schematic/PCB cross-probing, and searchable component tables.*
- `0006-schematic-symbols-and-text-rendering.html`
  - **`egui` Skill & PCB Feature**: Draw IC packages, pin numbers, net labels, and reference designators (`U1`, `R12`) using `Painter::text`, `FontId`, and `Align2`.
  - **Just-in-Time Rust**: `String` vs. `&str` string slices and `format!()`.
- `0007-split-viewports-and-egui-scene.html`
  - **`egui` Skill & PCB Feature**: Side-by-side or tabbed Schematic + PCB viewports (`TopBottomPanel`, `egui::Scene`), clicking a schematic pin to pan/zoom the PCB view to that footprint.
  - **Just-in-Time Rust**: Shared immutable state (`&Board`) vs. scoped mutable view state (`&mut ViewState`).
- `0008-searchable-netlist-and-bom-inspector.html`
  - **`egui` Skill & PCB Feature**: Filterable component/net list using `TextEdit::singleline`, `ScrollArea::vertical().show_rows(...)` (virtualized scrolling for 10,000+ pins), and `Grid`.
  - **Just-in-Time Rust**: Iterator adapters (`.filter()`, `.enumerate()`), string matching, and slices (`&[T]`).
- `0009-keyboard-shortcuts-and-point-to-point-ruler.html`
  - **`egui` Skill & PCB Feature**: Bind hotkeys (`F` to fit board, `M` for measurement tool, `Esc` to clear) via `ctx.input_mut` + `consume_key`, and draw a live Δx/Δy/distance CAD ruler overlay.
  - **Just-in-Time Rust**: State machines with data-carrying enums (`enum Tool { Select, Measure { start: Option<Pos2> } }`).

### Act III: Design Review Workflows (`0010`–`0013`)
*Goal: Turn the viewer into an engineering review tool with annotations, DRC markers, and custom widgets.*
- `0010-review-pins-and-sticky-note-callouts.html`
  - **`egui` Skill & PCB Feature**: Drop review comments anchored to `(x, y)` board coordinates and render interactive floating overlays (`egui::Area`, `egui::Window`, `Id::new`).
  - **Just-in-Time Rust**: `HashMap<PinId, ReviewNote>`, unique IDs, and the Entry API.
- `0011-review-status-workflow-and-popups.html`
  - **`egui` Skill & PCB Feature**: Triage review items (`Open` → `Resolved` / `Verified`), filter by author/severity, and build context menus (`Response::context_menu`, `ComboBox`).
  - **Just-in-Time Rust**: Custom traits, `#[derive(PartialEq, Eq, Clone, Copy)]`, and methods returning static slices.
- `0012-drc-violation-overlay-and-minimap.html`
  - **`egui` Skill & PCB Feature**: Render Design Rule Check error markers (clearance/unrouted net halos) and a corner bird's-eye minimap showing the active camera rectangle.
  - **Just-in-Time Rust**: Bounding-box math, `Iterator::fold`, and coordinate clamping (`f32::clamp`).
- `0013-custom-layer-swatch-widget-and-design-tokens.html`
  - **`egui` Skill & PCB Feature**: Extract a reusable PCB layer row widget (color swatch + visibility eye + opacity slider) implementing `egui::Widget`, styled with dark-mode CAD tokens.
  - **Just-in-Time Rust**: Implementing a foreign trait (`impl egui::Widget for ...`) and lifetimes (`'a`) in UI builders.

### Act IV: Scaling Up — Engine-First Architecture & Persistence (`0014`–`0017`)
*Goal: Refactor the growing prototype into a clean multi-crate architecture (`egui-app-architecture` & `egui-cow-document-model`) with zero-copy undo/redo and file save/load.*
- `0014-multi-crate-workspace-doc-vs-ui-egui.html`
  - **`egui` Skill & PCB Feature**: Split pure PCB/review domain data (`L1` `doc`) from `egui` rendering (`L6` `ui-egui`) so the core model compiles headlessly without UI dependencies.
  - **Just-in-Time Rust**: Cargo workspaces, module visibility (`pub`, `pub(crate)`), and strict dependency layering.
- `0015-loading-board-and-review-files-serde.html`
  - **`egui` Skill & PCB Feature**: Serialize and deserialize board layouts and review sessions to JSON, surfacing clean error banners in `egui` when a file is malformed (no panics!).
  - **Just-in-Time Rust**: `#[derive(Serialize, Deserialize)]`, `Result<T, E>`, the `?` operator, and `thiserror`.
- `0016-copy-on-write-arc-document-snapshots.html`
  - **`egui` Skill & PCB Feature**: Store board chunks and review annotations in `Arc<T>` so taking a full document snapshot before an edit costs only pointer copies.
  - **Just-in-Time Rust**: Smart pointers (`Arc<T>`), `Arc::make_mut` copy-on-write mutation, and structural sharing.
- `0017-undo-redo-history-and-slider-coalescing.html`
  - **`egui` Skill & PCB Feature**: Wire `Cmd+Z` / `Cmd+Shift+Z` and a visual History panel; coalesce continuous `egui` drags/sliders into a single undo step on release.
  - **Just-in-Time Rust**: `VecDeque<HistoryState>`, snapshot diffing/revision counters, and drag-lifecycle detection.

### Act V: Command Engine, Performance & Polish (`0018`–`0021`)
*Goal: Unify menus/shortcuts/CLI into a Command Engine (`egui-command-engine` & `egui-app-runner`), optimize rendering for huge boards, and test headlessly.*
- `0018-the-command-pattern-and-command-palette.html`
  - **`egui` Skill & PCB Feature**: Route menu items, toolbar buttons, and a fuzzy-searchable `Cmd+K` modal through a single `CommandSpec` registry (`layer.toggle`, `review.addPin`).
  - **Just-in-Time Rust**: Function pointers (`fn(&mut Session, &Value) -> Result<Value>`), static registries, and dynamic dispatch.
- `0019-viewport-culling-and-cached-tessellation.html`
  - **`egui` Skill & PCB Feature**: Keep 60 FPS on dense boards (50,000+ pads/segments) by spatial viewport culling (`Rect::intersects`) and batching geometry into `Vec<egui::Shape>`.
  - **Just-in-Time Rust**: Spatial grid indexing, pre-allocation (`Vec::with_capacity`), and profiling hot loops.
- `0020-non-blocking-file-import-and-progress-ui.html`
  - **`egui` Skill & PCB Feature**: Parse large board files on a background worker while keeping the `egui` window responsive with a live progress bar and `ctx.request_repaint()`.
  - **Just-in-Time Rust**: Concurrency with `std::thread::spawn`, message passing (`std::sync::mpsc::channel`), and `Send + 'static`.
- `0021-headless-snapshot-testing-and-wasm-build.html`
  - **`egui` Skill & PCB Feature**: Render the PCB review UI offscreen in `cargo test` to verify visual regressions and compile the reviewer to run in the browser via WebAssembly.
  - **Just-in-Time Rust**: Integration tests (`#[test]`), conditional compilation (`#[cfg(target_arch = "wasm32")]`), and feature flags.

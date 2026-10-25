---
title: Glossary of Rust & `egui` Terms
subtitle: Canonical definitions used across all lessons and reference sheets. Designed for quick screen lookup and clean printing.
page_type: glossary
---

## Canonical Glossary

**Immediate-Mode GUI (IMGUI)**:
A UI paradigm where widgets are not persistent objects in memory; instead, your application state lives in a plain Rust `struct` and every frame you call functions like `ui.checkbox(...)` or `ui.button(...)` that simultaneously query input and emit draw commands ([docs.rs/egui](https://docs.rs/egui/latest/egui/#understanding-immediate-mode)). Analogy: redrawing the active copper and silkscreen layers directly from the board data model on every display refresh.
_Avoid_: "stateless UI" (your Rust struct holds persistent state across frames; only the widget tree is transient).

**Retained-Mode GUI**:
A traditional UI paradigm (DOM, Qt, Flutter widget trees) where the framework stores a persistent graph of widget objects with their own internal state that must be synchronised with application state via callbacks or listeners.
_Avoid_: "stateful UI" (both immediate-mode and retained-mode UIs manage state; they differ in whether widgets retain a separate copy of it).

**Manifest (`Cargo.toml`)**:
The configuration file for a Rust package defining its metadata, external dependencies (like `eframe`), and compiler optimisation profiles ([Cargo Book](https://doc.rust-lang.org/cargo/reference/manifest.html)).
_Avoid_: "package.json" or "build script" (`Cargo.toml` is declarative metadata; `build.rs` is a build script).

**Struct (`struct`)**:
A composite Rust data type that groups named fields together into a single unit of ownership ([Rust Book §5.1](https://doc.rust-lang.org/book/ch05-01-defining-structs.html)), such as `struct PcbReviewerApp`.
_Avoid_: "class" or "object" (Rust structs have no inheritance hierarchy and separate data layout from `impl` behaviour blocks).

**Default Trait (`Default`)**:
A standard library trait granting a type a canonical parameterless constructor (`T::default()`) for initial values ([std::default::Default](https://doc.rust-lang.org/std/default/trait.Default.html)).
_Avoid_: "default constructor" (Rust has no implicit constructors; `Default::default()` is an explicit trait method).

**Heap Box (`Box<T>`)**:
A smart pointer that allocates a value of type `T` on the heap and owns it through a fixed-size pointer on the stack ([Rust Book §15.1](https://doc.rust-lang.org/book/ch15-01-box.html)), used to hand `Box<dyn eframe::App>` to `eframe::run_native`.
_Avoid_: "raw pointer" (`Box<T>` uniquely owns its heap allocation and drops it deterministically when it goes out of scope).

**Shared Borrow (`&T`)**:
An immutable, read-only reference to a value of type `T`; multiple `&T` references can coexist simultaneously, but the target cannot be mutated while borrowed ([Rust Book §4.2](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)).
_Avoid_: "pointer" or "copy" (a shared borrow is a compile-time lifetime-checked non-owning reference).

**Mutable Borrow (`&mut T`)**:
An exclusive, read-write reference to a value of type `T` that allows a called function (such as `ui.checkbox(&mut self.show_top_copper, "Top Cu")`) to read and modify a value in place without taking ownership ([Rust Book §4.2](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)).
_Avoid_: "mutable pointer" or "pass by reference" (emphasise exclusivity: no other `&T` or `&mut T` to the same data may overlap).

**Closure (`|ui| { ... }`)**:
An anonymous inline function that can capture variables (like `self`) from its surrounding scope, with parameters written between vertical pipes `|...|` ([Rust Book §13.1](https://doc.rust-lang.org/book/ch13-01-closures.html)).
_Avoid_: "callback" (in `egui`, panel closures execute synchronously right now during the current frame, not asynchronously later).

**Copy Semantics (`Copy`)**:
A marker trait for small stack-only values (`Pos2`, `Vec2`, `Rect`, `Color32`, `Stroke`, `PcbLayer`) that duplicate trivially by copying bits on assignment or function call without moving ownership ([Rust Book §4.1](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html#stack-only-data-copy)).
_Avoid_: "deep clone" (`Copy` is an implicit bitwise stack copy; heap-allocating duplicates use explicit `.clone()`).

**Enum & Exhaustive `match`**:
An algebraic data type enumerating a closed set of variants (`PcbLayer`, `Option<NetId>`, `ActiveTool`) where `match` forces every variant to be handled at compile time ([Rust Book §6](https://doc.rust-lang.org/book/ch06-00-enums.html)).
_Avoid_: "integer constants" (Rust enums are strongly typed sum types and can carry per-variant data).

**Optional Value (`Option<T>`)**:
Rust's null-free enum representing a value that is either present (`Some(T)`) or absent (`None`), used for hover and selection state (`Option<NetId>`).
_Avoid_: "null" or "nullable pointer" (Rust references are never null; absence is always explicit in the type signature).

**String Slice vs. Owned String (`&'static str` vs. `String`)**:
`&'static str` is a zero-allocation borrowed view into UTF-8 text baked into the compiled binary, whereas `String` is a heap-allocated, growable UTF-8 buffer constructed dynamically with `format!(...)` or mutated by `TextEdit::singleline` ([Rust Book §8.2](https://doc.rust-lang.org/book/ch08-02-strings.html)).
_Avoid_: calling both "strings" without distinguishing borrowed slices (`&str`) from owned buffers (`String`).

**Disjoint Struct Field Borrowing**:
Rust's borrow checker tracks borrows per struct field when fields are accessed directly, allowing a shared borrow (`&self.board`) and a mutable borrow (`&mut self.view`) to coexist in the same scope ([Rustonomicon](https://doc.rust-lang.org/nomicon/borrow-splitting.html)).
_Avoid_: "borrowing `self` twice" (borrowing distinct fields `&self.board` and `&mut self.view` works precisely because whole-struct `&mut self` is split across disjoint fields).

**Data-Carrying Enum (Tagged Union)**:
A Rust `enum` whose variants store variant-specific payload fields directly inside the variant (such as `ActiveTool::Measuring { start_mm: Pos2, end_mm: Pos2 }`), making invalid state combinations unrepresentable ([Rust Book §6.1](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)).
_Avoid_: "boolean flag + optional fields" (which permit contradictory states like `is_measuring = true` with `start_mm = None`).

**`eframe::App`**:
The root application trait in `eframe` whose core method, `fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`, is invoked on every repaint ([docs.rs/eframe](https://docs.rs/eframe/latest/eframe/trait.App.html)).
_Avoid_: "window controller" (`eframe::App` describes per-frame UI state transitions and rendering).

**`egui::Context` (`ctx`)**:
The top-level handle to `egui` for the entire window: manages input state, fonts, style, widget ID memory, and root panels (`SidePanel`, `TopBottomPanel`, `CentralPanel`) ([docs.rs/egui](https://docs.rs/egui/latest/egui/struct.Context.html)).
_Avoid_: confusing `ctx: &egui::Context` (whole-window coordinator) with `ui: &mut egui::Ui` (a bounded region inside a panel).

**`egui::Ui` (`ui`)**:
A bounded rectangular region inside a panel or container with an automatic layout cursor on which you invoke widget methods (`ui.heading`, `ui.checkbox`, `ui.button`, `ui.allocate_painter`) ([docs.rs/egui](https://docs.rs/egui/latest/egui/struct.Ui.html)).
_Avoid_: storing `Ui` across frames (`Ui` is a short-lived per-frame layout builder).

**`egui::Response`**:
The immediate return value of every widget call (e.g., `ui.button("Reset").clicked()` or `.hovered()`) reporting how the user interacted with that widget during the current frame ([docs.rs/egui](https://docs.rs/egui/latest/egui/struct.Response.html)).
_Avoid_: "event listener" (`Response` is a plain value inspected synchronously inside `update`).

**`egui::Painter`**:
The 2D vector drawing API returned by `ui.allocate_painter(size, sense)` that emits clipped shapes (`rect_filled`, `line_segment`, `circle_filled`, `text`) in back-to-front painter's algorithm order ([docs.rs/egui](https://docs.rs/egui/latest/egui/struct.Painter.html)).
_Avoid_: "GPU shader" (`Painter` tessellates 2D vector primitives into vertex/index meshes for `eframe` to render).

**`egui::emath::TSTransform`**:
A 2D translation plus uniform scaling transform (`scaling * pt + translation`) with `.inverse()` for mapping between physical board millimetres (world space) and window points (screen space) ([docs.rs/emath](https://docs.rs/emath/latest/emath/struct.TSTransform.html)).
_Avoid_: manually mixing unscaled millimetres and screen pixels without a typed coordinate transform.

**Mutation Testing (`cargo-mutants`)**:
A verification technique that injects small deliberate faults (*mutants*, such as flipping `true` to `false`, replacing `+` with `-`, or emptying a method body) and verifies that `cargo test` fails for every injected fault ([mutants.rs](https://mutants.rs/)).
_Avoid_: equating "100% line coverage" with verified behaviour (line coverage only proves code ran, whereas caught mutants prove assertions fail when logic breaks).

**Headless UI Harness (`egui_kittest`)**:
The official testing library for `egui` that drives an app or widget closure without opening an OS window, supporting semantic AccessKit widget queries (`get_by_label`), simulated clicks/key presses, and offscreen visual snapshots ([docs.rs/egui_kittest](https://docs.rs/egui_kittest/latest/egui_kittest/)).
_Avoid_: skipping `eframe::App::update` forever once headless UI testing is available in Act II+.

**Hash Map & `Entry` API (`HashMap<K, V>`)**:
A hash-table collection mapping unique keys `K: Eq + Hash` to values `V`, whose `.entry(key)` method returns `Entry::Vacant` or `Entry::Occupied` for in-place inspection or insertion without double lookups or silent overwrites ([std::collections::HashMap](https://doc.rust-lang.org/std/collections/struct.HashMap.html)).
_Avoid_: relying on `HashMap` iteration order in immediate-mode UIs (sort keys first so widget order is deterministic).

**Floating Window & Widget Identity (`egui::Window`, `egui::Id`)**:
`egui::Window` renders a floating, draggable overlay layer above panels, keyed by a deterministic `egui::Id::new(...)` hash so `egui` remembers window state without collisions ([docs.rs/egui](https://docs.rs/egui/latest/egui/containers/struct.Window.html)).
_Avoid_: creating multiple windows or grids with identical titles and no disambiguating `.id(...)`.

**Struct Lifetime (`'a`) & `egui::Widget`**:
A generic lifetime annotation `<'a>` on a builder struct (such as `LayerSwatch<'a>`) proving that the struct cannot outlive the `&'a mut` state fields it borrows while `impl<'a> egui::Widget for LayerSwatch<'a>` consumes `self` inside `ui.add(...)` ([Rust Book §10.3](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)).
_Avoid_: thinking lifetimes change runtime performance (`'a` is purely a compile-time borrow check erased before code generation).

**Engine-First Crate Layering (`L0`–`L7`)**:
A multi-crate Cargo `[workspace]` architecture that isolates pure domain state (`crates/pcb-doc` at `L1`, depending only on `L0` `emath`) from UI rendering (`pcb-reviewer` at `L6`/`L7`), enforced by Tier 1 manifest gates (`validate_l1_manifest_no_ui_deps`) so domain tests and `cargo-mutants` run in milliseconds without linking `eframe` ([Cargo Book §3.3](https://doc.rust-lang.org/cargo/reference/workspaces.html)).
_Avoid_: importing `egui::Color32` or `eframe` inside `L1` domain crates (expose UI-agnostic RGB tuples or geometry primitives in `L1` and bridge them in `L6`).

**Versioned Envelope Serialization (`serde`, `thiserror`)**:
Wrapping a serialized domain model in an explicit envelope struct (`ReviewSessionEnvelope { schema_version, board_id, board }`) and returning typed `Result<T, SessionIoError>` values derived via `thiserror::Error` so malformed JSON or incompatible schema versions surface as non-destructive UI banners rather than panics ([serde.rs](https://serde.rs/)).
_Avoid_: calling `.unwrap()` on `serde_json::from_str` or using `..` when destructuring persisted envelopes.

**Copy-on-Write (`Arc::make_mut`, `Arc::ptr_eq`)**:
Wrapping document collections in `std::sync::Arc<...>` so cloning a `BoardModel` snapshot copies only four 8-byte pointers (`O(1)`), while mutating a collection via `Arc::make_mut` clones the inner buffer only when `strong_count > 1` and leaves untouched buffers pointer-shared (`Arc::ptr_eq == true`) ([std::sync::Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.make_mut)).
_Avoid_: deep-cloning `Vec<TraceSegment>` or `HashMap<PinId, ReviewNote>` on every edit or snapshot.

**Undo/Redo Ring Buffer & Drag Coalescing (`VecDeque`, `coalesce_key`)**:
A bounded history stack (`VecDeque<HistoryEntry>`) that evicts oldest snapshots via `pop_front()`, swaps snapshots in `O(1)` via `std::mem::replace`, and merges consecutive 60-FPS `egui::Slider` drag frames sharing `Some(coalesce_key)` into a single undo entry until `Response::drag_stopped()` calls `commit_coalesce()` ([std::collections::VecDeque](https://doc.rust-lang.org/std/collections/struct.VecDeque.html)).
_Avoid_: pushing a new undo entry on every `slider_resp.changed()` frame without a `coalesce_key`.

**Command Registry & `panic_hunt` Fuzzer (`CommandSpec`)**:
A static slice (`COMMAND_SPECS: &[CommandSpec]`) pairing canonical command IDs (`"review.addPin"`, `"session.inspect"`) with an `EnabledFn` precondition guard (`fn(&BoardModel, &BoardHistory) -> Result<(), String>`) and a `RunFn` execution pointer, verified by a Tier 5 `panic_hunt` fuzzer that feeds adversarial JSON payloads into `execute_command`.
_Avoid_: scattering untyped state mutations across ad-hoc button callbacks without parameter bounds checks.

**Viewport Frustum Culling (`Rect::intersects`)**:
Testing each pad's or stroke-expanded trace's axis-aligned bounding box (`bounds_mm().intersects(viewport_mm)`) before submitting geometry to `egui::Painter`, skipping off-screen primitives in `O(N)` bounds checks rather than tessellating invisible meshes ([docs.rs/emath](https://docs.rs/emath/latest/emath/struct.Rect.html#method.intersects)).
_Avoid_: computing trace bounding boxes from centreline endpoints alone without expanding by `width_mm * 0.5`.

**Non-Blocking Background Channel (`std::sync::mpsc::try_recv`)**:
Offloading file parsing onto a worker thread (`std::thread::spawn`) that streams `ImportJobMessage` progress updates over an `mpsc::channel`, while `eframe::App::update` drains ready messages via `rx.try_recv()` and calls `ctx.request_repaint()` while `ImportJobState::Running` is active ([std::sync::mpsc](https://doc.rust-lang.org/std/sync/mpsc/index.html)).
_Avoid_: calling blocking `rx.recv()` inside `eframe::App::update` (which freezes the UI thread).

**Process-Wide GPU Lock & Frame Snapshot Digest (`gpu_lock`, `FrameSnapshotDigest`)**:
A `OnceLock<Mutex<()>>` guard (`let _gpu = gpu_lock();` using `PoisonError::into_inner`) acquired on the first line of GPU/snapshot tests to serialize concurrent driver access, paired with a deterministic 64-bit FNV-1a `FrameSnapshotDigest` over visible layer masks, culled geometry counts, and DRC markers.
_Avoid_: running multiple concurrent `wgpu` or snapshot harnesses across threads without acquiring `gpu_lock()`.

**Zero-Copy S-Expression Parser Combinators (`nom::IResult`, `SexprNode`)**:
Functional parser combinators (`delimited`, `many0`, `alt`, `take_while1`) from `nom` that thread an unconsumed `&str` slice through `IResult<&str, SexprNode>` in a standalone `L0` crate (`crates/kicad-sexpr`) enforced by `validate_l0_standalone_manifest` ([docs.rs/nom](https://docs.rs/nom/latest/nom/)).
_Avoid_: writing ad-hoc regexes or byte-index loops that panic on unclosed parentheses or unterminated string literals.

**Pinned KiCad Corpus Oracle & Ratchet Floors (`CORPUS_PASS_FLOOR`, `CORPUS_ROUNDTRIP_FLOOR`)**:
A Tier 4 verification harness (`run_pinned_kicad_corpus`) that lowers real-structure `.kicad_pcb` S-expression fixtures into `BoardModel`, re-exports them (`to_kicad_pcb_sexpr`), and asserts non-decreasing pass and round-trip floor constants (`reloaded == board`).
_Avoid_: testing a CAD parser on a single synthetic toy string without lossless round-trip oracles.

**Embedded SQL AST Validation & Foreign-Key Store (`turso_parser`, `TursoReviewStore`)**:
Validating SQLite/Turso DDL and DML statements (`Stmt::CreateTable`, `Stmt::Insert`, `Stmt::Select`, `Stmt::Begin`, `Stmt::Commit`, `Stmt::Rollback`) with `turso_parser::parser::Parser` before executing idempotent schema migrations and foreign-key-checked `SqlReviewRecord` queries in `TursoReviewStore`.
_Avoid_: concatenating unvalidated raw SQL strings or inserting review threads for unregistered `board_id` keys.

**Atomic SQL Transactions, Audit Hash Chain & Local-First Replica Sync (`AuditLogEntry`, `verify_audit_chain`)**:
Executing multi-pin status sign-offs on a staged transaction snapshot that commits or rolls back atomically (`execute_atomic_signoff_batch`), chaining each `AuditLogEntry` via `compute_audit_entry_hash(seq, pin_id, from_status, to_status, prev_hash)`, and merging remote replica rows when `remote.revision > local.revision` (`sync_with_remote_replica`).
_Avoid_: partially applying a failed sign-off batch or overwriting newer local edits with stale remote revisions.

---

## Quick Lookup: Key `eframe` & `egui` APIs by Lesson

| Type / Method | Role in the Frame Loop | Introduced In |
| --- | --- | --- |
| `eframe::run_native` | Opens a native desktop OS window, initialises the renderer, constructs your app struct, and starts the frame loop. | [Lesson 0000](../lessons/0000-project-setup-and-eframe-boot.html) |
| `SidePanel` / `CentralPanel` | Splits the window into a left/right inspector and a remaining central canvas (`SidePanel` must come before `CentralPanel`). | [Lesson 0001](../lessons/0001-the-immediate-mode-loop.html) |
| `ui.allocate_painter(size, sense)` | Reserves a 2D canvas `Rect`, tracks pointer interaction (`Response`), and returns a clipped `Painter`. | [Lesson 0002](../lessons/0002-custom-2d-canvas-and-painter.html) |
| `TSTransform` & `Sense::drag()` | Maps world-space millimetres (`mm`) to screen points (`px`), pans via `drag_delta()`, and zooms around the cursor. | [Lesson 0003](../lessons/0003-world-to-screen-transforms-pan-and-zoom.html) |
| `PcbLayer` & Z-Ordering | Encodes physical stackup layers (`BottomCopper`, `TopCopper`, `Silkscreen`, `Ratsnest`) and sorts primitives back-to-front. | [Lesson 0004](../lessons/0004-modeling-layers-traces-and-footprints.html) |
| `Sense::click_and_drag()` & `Option<NetId>` | Inverse-transforms cursor positions to hit-test pads in top-down order and locks net cross-probing on click. | [Lesson 0005](../lessons/0005-hit-testing-and-net-cross-probing.html) |
| `Painter::text`, `Align2`, `egui_kittest` | Draws anchored component reference designators (`U1`, `J1`) and verifies widgets headlessly in `cargo test`. | [Lesson 0006](../lessons/0006-schematic-symbols-and-text-rendering.html) |
| `TopBottomPanel` & `egui::Scene` | Splits the window into a schematic bar and an infinite pan/zoom PCB scene with camera cross-probing. | [Lesson 0007](../lessons/0007-split-viewports-and-egui-scene.html) |
| `TextEdit::singleline` & `ScrollArea::show_rows` | Filters the Bill of Materials live and virtualises scrolling so only visible rows are laid out. | [Lesson 0008](../lessons/0008-searchable-netlist-and-bom-inspector.html) |
| `InputState::consume_key` & `ActiveTool` | Consumes single-key CAD hotkeys (`F`, `M`, `Esc`) and drives a point-to-point distance ruler state machine. | [Lesson 0009](../lessons/0009-keyboard-shortcuts-and-point-to-point-ruler.html) |
| `HashMap::entry`, `egui::Window` & `Id::new` | Stores anchored review pins without silent overwrites and renders floating callout overlays. | [Lesson 0010](../lessons/0010-review-pins-and-sticky-note-callouts.html) |
| `ui.selectable_value` & `ReviewStatus` | Enforces `Open` → `Resolved` → `Verified` sign-off transitions and filters pins by severity. | [Lesson 0011](../lessons/0011-review-status-workflow-and-popups.html) |
| `Iterator::reduce(Rect::union)` & `f32::clamp` | Detects copper pad-to-pad DRC clearance violations and clamps minimap camera bounds. | [Lesson 0012](../lessons/0012-drc-violation-overlay-and-minimap.html) |
| `impl<'a> egui::Widget` & `Harness::new_ui_state` | Packages a colour swatch, visibility toggle, and opacity slider into a custom widget tested in isolation. | [Lesson 0013](../lessons/0013-custom-layer-swatch-widget-and-design-tokens.html) |
| `[workspace]` & `validate_l1_manifest_no_ui_deps` | Splits pure `L1` domain state (`crates/pcb-doc`) from the `L6`/`L7` `eframe` UI crate and enforces zero upward UI dependencies. | [Lesson 0014](../lessons/0014-multi-crate-workspace-doc-vs-ui-egui.html) |
| `ReviewSessionEnvelope` & `SessionIoError` | Serializes and validates versioned JSON review sessions with `serde_json` and `thiserror`, surfacing errors in `SidePanel::right`. | [Lesson 0015](../lessons/0015-loading-board-and-review-files-serde.html) |
| `Arc::make_mut` & `Arc::ptr_eq` | Turns `BoardModel::clone()` into `O(1)` pointer sharing and clones only touched collections on mutation, verified with `proptest`. | [Lesson 0016](../lessons/0016-copy-on-write-arc-document-snapshots.html) |
| `BoardHistory`, `coalesce_key` & `drag_stopped()` | Maintains a bounded `VecDeque` undo/redo stack and coalesces continuous 60-FPS slider drags into a single undo step. | [Lesson 0017](../lessons/0017-undo-redo-history-and-slider-coalescing.html) |
| `CommandSpec`, `Key::K` Palette & `panic_hunt` | Dispatches commands through a static `COMMAND_SPECS` registry with `EnabledFn` guards and fuzzes JSON inputs. | [Lesson 0018](../lessons/0018-the-command-pattern-and-command-palette.html) |
| `Rect::intersects` & `ViewportCullStats` | Culls off-screen pads and stroke-expanded traces against `pcb_scene_rect` and enforces frame budgets. | [Lesson 0019](../lessons/0019-viewport-culling-and-cached-tessellation.html) |
| `mpsc::Receiver::try_recv` & `egui::ProgressBar` | Streams background file import progress without blocking the UI thread and wakes `egui` via `ctx.request_repaint()`. | [Lesson 0020](../lessons/0020-non-blocking-file-import-and-progress-ui.html) |
| `gpu_lock()`, `FrameSnapshotDigest` & `QualityScorecard` | Serializes snapshot tests with `OnceLock<Mutex<()>>`, computes 64-bit FNV-1a visual digests, and gates `wasm32` release readiness. | [Lesson 0021](../lessons/0021-headless-snapshot-testing-and-wasm-build.html) |
| `parse_sexpr_str`, `SexprNode` & `CollapsingHeader` | Parses KiCad S-expressions with `nom` in standalone `L0` `crates/kicad-sexpr` and renders a collapsible AST inspector. | [Lesson 0022](../lessons/0022-sexpr-tokenizer-and-ast-with-nom.html) |
| `from_kicad_pcb_sexpr` & `run_pinned_kicad_corpus` | Lowers `(kicad_pcb ...)` ASTs into `BoardModel` and enforces Tier 4 corpus pass & round-trip ratchet floors. | [Lesson 0023](../lessons/0023-parsing-kicad-pcb-and-sch-into-domain-model.html) |
| `validate_turso_sql` & `TursoReviewStore` | Validates SQLite/Turso DDL/DML ASTs via `turso_parser`, enforces `board_id` foreign keys, and syncs pins to SQL. | [Lesson 0024](../lessons/0024-embedded-sql-review-store-with-turso.html) |
| `execute_atomic_signoff_batch` & `verify_audit_chain` | Executes all-or-nothing SQL sign-off transactions, verifies 64-bit hash-chained `AuditLogEntry` logs, and merges replica rows. | [Lesson 0025](../lessons/0025-local-first-review-sync-and-audit-trail.html) |

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
A traditional UI paradigm (DOM, Qt, Flutter widget trees) where the framework stores a persistent graph of widget objects with their own internal state that must be synchronized with application state via callbacks or listeners.
_Avoid_: "stateful UI" (both immediate-mode and retained-mode UIs manage state; they differ in whether widgets retain a separate copy of it).

**Manifest (`Cargo.toml`)**:
The configuration file for a Rust package defining its metadata, external dependencies (like `eframe`), and compiler optimization profiles ([Cargo Book](https://doc.rust-lang.org/cargo/reference/manifest.html)).
_Avoid_: "package.json" or "build script" (`Cargo.toml` is declarative metadata; `build.rs` is a build script).

**Struct (`struct`)**:
A composite Rust data type that groups named fields together into a single unit of ownership ([Rust Book §5.1](https://doc.rust-lang.org/book/ch05-01-defining-structs.html)), such as `struct PcbReviewerApp`.
_Avoid_: "class" or "object" (Rust structs have no inheritance hierarchy and separate data layout from `impl` behavior blocks).

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
_Avoid_: "mutable pointer" or "pass by reference" (emphasize exclusivity: no other `&T` or `&mut T` to the same data may overlap).

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
A 2D translation plus uniform scaling transform (`scaling * pt + translation`) with `.inverse()` for mapping between physical board millimeters (world space) and window points (screen space) ([docs.rs/emath](https://docs.rs/emath/latest/emath/struct.TSTransform.html)).
_Avoid_: manually mixing unscaled millimeters and screen pixels without a typed coordinate transform.

**Mutation Testing (`cargo-mutants`)**:
A verification technique that injects small deliberate faults (*mutants*, such as flipping `true` to `false`, replacing `+` with `-`, or emptying a method body) and verifies that `cargo test` fails for every injected fault ([mutants.rs](https://mutants.rs/)).
_Avoid_: equating "100% line coverage" with verified behavior (line coverage only proves code ran, whereas caught mutants prove assertions fail when logic breaks).

**Headless UI Harness (`egui_kittest`)**:
The official testing library for `egui` that drives an app or widget closure without opening an OS window, supporting semantic AccessKit widget queries (`get_by_label`), simulated clicks/key presses, and offscreen visual snapshots ([docs.rs/egui_kittest](https://docs.rs/egui_kittest/latest/egui_kittest/)).
_Avoid_: skipping `eframe::App::update` forever once headless UI testing is available in Act II+.

---

## Quick Lookup: Key `eframe` & `egui` APIs by Lesson

| Type / Method | Role in the Frame Loop | Introduced In |
| --- | --- | --- |
| `eframe::run_native` | Opens a native desktop OS window, initializes the renderer, constructs your app struct, and starts the frame loop. | [Lesson 0000](../lessons/0000-project-setup-and-eframe-boot.html) |
| `SidePanel` / `CentralPanel` | Splits the window into a left/right inspector and a remaining central canvas (`SidePanel` must come before `CentralPanel`). | [Lesson 0001](../lessons/0001-the-immediate-mode-loop.html) |
| `ui.allocate_painter(size, sense)` | Reserves a 2D canvas `Rect`, tracks pointer interaction (`Response`), and returns a clipped `Painter`. | [Lesson 0002](../lessons/0002-custom-2d-canvas-and-painter.html) |
| `TSTransform` & `Sense::drag()` | Maps world-space millimeters (`mm`) to screen points (`px`), pans via `drag_delta()`, and zooms around the cursor. | [Lesson 0003](../lessons/0003-world-to-screen-transforms-pan-and-zoom.html) |
| `PcbLayer` & Z-Ordering | Encodes physical stackup layers (`BottomCopper`, `TopCopper`, `Silkscreen`, `Ratsnest`) and sorts primitives back-to-front. | [Lesson 0004](../lessons/0004-modeling-layers-traces-and-footprints.html) |
| `Sense::click_and_drag()` & `Option<NetId>` | Inverse-transforms cursor positions to hit-test pads in top-down order and locks net cross-probing on click. | [Lesson 0005](../lessons/0005-hit-testing-and-net-cross-probing.html) |
| `Painter::text`, `Align2`, `egui_kittest` | Draws anchored component reference designators (`U1`, `J1`) and verifies widgets headlessly in `cargo test`. | [Lesson 0006](../lessons/0006-schematic-symbols-and-text-rendering.html) |
| `TopBottomPanel` & `egui::Scene` | Splits the window into a schematic bar and an infinite pan/zoom PCB scene with camera cross-probing. | [Lesson 0007](../lessons/0007-split-viewports-and-egui-scene.html) |
| `TextEdit::singleline` & `ScrollArea::show_rows` | Filters the Bill of Materials live and virtualizes scrolling so only visible rows are laid out. | [Lesson 0008](../lessons/0008-searchable-netlist-and-bom-inspector.html) |
| `InputState::consume_key` & `ActiveTool` | Consumes single-key CAD hotkeys (`F`, `M`, `Esc`) and drives a point-to-point distance ruler state machine. | [Lesson 0009](../lessons/0009-keyboard-shortcuts-and-point-to-point-ruler.html) |

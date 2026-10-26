---
nav_id: "0015"
page_title: "Lesson 0015: Multi-Crate Workspace — Separating pcb-doc (L1) from pcb-reviewer (L6) — Learn Rust & egui"
act_badge: "Act IV • Engine-First Architecture & Persistence"
title: "0015. Multi-Crate Workspace: Separating `pcb-doc` (`L1`) from `pcb-reviewer` (`L6`)"
subtitle: "Splitting domain geometry, DRC math, and review pin invariants into a pure, headless `pcb-doc` library crate (`L1` depending only on `emath`) so `cargo test -p pcb-doc` and `cargo mutants -p pcb-doc` execute in milliseconds without linking `egui` or `eframe`."
card:
  num: "Lesson 0015"
  mutants: "144 / 144 Caught"
  title: "Multi-Crate Workspace (`pcb-doc` L1 vs. `pcb-reviewer` L6)"
  summary: "Refactor the single-file app into a Cargo `[workspace]` separating pure `L1` domain state (`crates/pcb-doc`, depending only on `emath`) from the `L6`/`L7` `eframe` UI crate, enforced by a Tier 1 manifest dependency gate."
  egui_skill: "Engine-First Layering (`L1` Domain vs. `L6` UI)"
  rust_concept: "Cargo `[workspace]`, `pub`/`pub(crate)` Visibility & Lint Gates"
  app_dir: "apps/0015-multi-crate-workspace-doc-vs-ui-egui"
---

## Why Engine-First `egui` Apps Split Domain State into `L1` Crates

In Lessons `0001` through `0014`, our `BoardModel` (`traces`, `pads`, `components`, `review_pins`, and DRC clearance math) lived in the same `src/main.rs` binary crate as `ViewState` and `PcbReviewerApp`. That kept early lessons compact, but every single edit to a pure domain function—like `pad_edge_gap_mm` or `ReviewStatus::can_transition_to`—forced `rustc` and `cargo-mutants` to re-link the entire `eframe` + `egui` + windowing stack.{{sidenote: **Strict L0–L7 Layering:** Production `egui` CAD and graphics workspaces organize crates into a strict directed acyclic graph (DAG): `L0` foundational math (`emath`), `L1` pure document model (`pcb-doc`), `L2`–`L5` file formats and command engines, `L6` reusable `egui` widgets/panels, and `L7` native/Wasm shell binaries.}}

By refactoring `./apps/0015-multi-crate-workspace-doc-vs-ui-egui` into a Cargo `[workspace]` with two crates:

1. **`crates/pcb-doc` (`L1` Domain Crate)**: Depends *only* on `emath = "0.37"` for `Pos2`, `Vec2`, and `Rect`. It contains zero `egui`, `eframe`, `winit`, or `Color32` types. For layer colors, `PcbLayer::rgb(self) -> (u8, u8, u8)` returns a UI-agnostic RGB tuple that `pcb-reviewer` bridges into `egui::Color32`.
2. **`pcb-reviewer` (`L6`/`L7` UI Crate)**: Depends on `pcb-doc = { path = "crates/pcb-doc" }`, `eframe = "0.37"`, and `egui_kittest = "0.37"`. It owns `ViewState`, `BoardViewport`, `LayerSwatch<'a>`, and the `eframe::App` shell.

```mermaid
flowchart TD
    L7["pcb-reviewer (L6 UI / L7 Shell)<br/>eframe + egui + egui_kittest<br/>ViewState, LayerSwatch, Painter"] --> L1["crates/pcb-doc (L1 Domain)<br/>BoardModel, SmdPad, ReviewNote, DrcViolation"]
    L1 --> L0["emath 0.37 (L0 Foundational Math)<br/>Pos2, Vec2, Rect, TSTransform"]
    style L7 fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style L1 fill:#141a21,stroke:#3b8eea,color:#e6edf3
    style L0 fill:#1c1c24,stroke:#f0e68c,color:#e6edf3
```

## Enforcing Layer Boundaries and Panic-Free Domain Code

A multi-crate workspace only stays clean if upward dependency leaks are caught automatically.{{sidenote: **Tier 1 Static & Manifest Gates:** Checking `include_str!("../Cargo.toml")` inside a unit test—or running an `xtask` workspace auditor—prevents an accidental `cargo add egui` inside `crates/pcb-doc` from silently coupling domain logic to UI rendering.}} In `crates/pcb-doc/src/lib.rs`, we enforce two compile-time and Tier 1 invariants:

- **Panic-Free / Unsafe-Free Lints**: `#![forbid(unsafe_code)]` and `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the top of `lib.rs` and `main.rs` guarantee that domain queries never crash the host process. Test modules opt in explicitly with `#[cfg(test)] #[allow(clippy::unwrap_used, ...)]`.
- **Tier 1 Manifest Gate (`validate_l1_manifest_no_ui_deps`)**: Scans `crates/pcb-doc/Cargo.toml` against `FORBIDDEN_UI_DEPS_BELOW_L6` (`&["egui", "eframe", "winit", "egui_kittest", "rfd"]`) and fails immediately if any UI crate is present.

```rust caption="Tier 1 manifest gate and UI-agnostic RGB domain tokens in crates/pcb-doc/src/lib.rs"
pub const FORBIDDEN_UI_DEPS_BELOW_L6: &[&str] =
    &["egui", "eframe", "winit", "egui_kittest", "rfd"];

pub fn validate_l1_manifest_no_ui_deps(cargo_toml: &str) -> Result<(), &'static str> {
    for &forbidden in FORBIDDEN_UI_DEPS_BELOW_L6 {
        let key_eq = format!("{forbidden} =");
        let key_dot = format!("dependencies.{forbidden}");
        if cargo_toml.contains(&key_eq) || cargo_toml.contains(&key_dot) {
            return Err(forbidden);
        }
    }
    Ok(())
}

impl PcbLayer {
    /// UI-agnostic RGB triple `(r, g, b)` for each layer in `L1` domain state.
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            PcbLayer::BottomCopper => (59, 142, 234),
            PcbLayer::TopCopper => (224, 82, 67),
            PcbLayer::Silkscreen => (240, 230, 140),
            PcbLayer::Ratsnest => (111, 247, 255),
        }
    }
}
```

{{widget:workspace-layers-sim}}

:::exercise id="ex0015" title="Interactive Lab 2: Bridge `pcb-doc` (`L1`) RGB Tokens into `pcb-reviewer` (`L6`)" badge="Skill Practice • Workspace Boundaries"
Complete the `validate_l1_manifest_no_ui_deps` check in `pcb-doc` and the `layer_color` bridge function in `pcb-reviewer` so `PcbLayer::rgb()` is converted into an `egui::Color32` only at the `L6` UI layer.

```rust
pub fn validate_l1_manifest_no_ui_deps(cargo_toml: &str) -> Result<(), &'static str> {
    for &forbidden in FORBIDDEN_UI_DEPS_BELOW_L6 {
        let key_eq = format!("{forbidden} =");
        if cargo_toml.contains(&key_eq) {
            return {{blank:Err(forbidden)|Return Err(forbidden) when a forbidden UI dependency is detected|Err(...)|145px}};
        }
    }
    Ok(())
}

pub fn layer_color(layer: PcbLayer) -> Color32 {
    let (r, g, b) = layer.{{blank:rgb()|Call PcbLayer::rgb() from the L1 pcb-doc crate|rgb()|95px}};
    Color32::{{blank:from_rgb(r, g, b);;from_rgb(r,g,b)|Construct an opaque Color32 from the (r, g, b) tuple|from_rgb(...)|165px}}
}
```

:::prompts
hint: Return `Err(forbidden)` when `cargo_toml` contains a forbidden UI crate, then unpack `layer.rgb()` into `Color32::from_rgb(r, g, b)`.
success: Confirmed! `pcb-doc` stays 100% free of `egui` types while `pcb-reviewer` bridges `(r, g, b)` into `Color32` at the UI boundary.
:::
:::

:::verify id="verify-0015" title="Lab 3: Multi-Crate Workspace & Mutation Gate (`0015`)" badge="cargo test --workspace + cargo mutants" tier="Tier 1 + Tier 2 + Tier 3" caught="144 / 144 (100%)" missed="0 Missed"
Run the full workspace test suite and notice how you can also target `pcb-doc` alone (`-p pcb-doc`) for sub-second domain test and mutation feedback:

```bash caption="Run workspace tests and fast L1 domain mutation checks"
cargo test --manifest-path ./apps/0015-multi-crate-workspace-doc-vs-ui-egui/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0015-multi-crate-workspace-doc-vs-ui-egui/Cargo.toml -p pcb-doc --in-place
```

- [ ] `cargo test --manifest-path ./apps/0015-multi-crate-workspace-doc-vs-ui-egui/Cargo.toml --workspace` passes all **11 tests** (**5** in `pcb-doc` and **6** in `pcb-reviewer`).
- [ ] `cargo mutants -p pcb-doc` catches all **98 viable domain mutants** (with 9 unviable and **0 missed**) without compiling or linking `eframe`.
- [ ] `cargo mutants -p pcb-reviewer` catches all **46 viable UI mutants** (with 12 unviable and **0 missed**), for a workspace total of **144 / 144 viable mutants caught**.
:::

:::quiz id="quiz-0015"
Q: Why does `crates/pcb-doc` (`L1`) depend on `emath` instead of `egui` or `eframe`?
- [ ] Because `emath` automatically renders GPU triangle meshes without windowing
- [x] Because `emath` provides pure geometry types without UI dependencies
- [ ] Because `eframe` forbids workspaces containing more than one binary
- [ ] Because `egui` cannot serialize floating-point vectors on macOS targets
why: `emath` (`L0`) contains lightweight, UI-agnostic 2D math types (`Pos2`, `Vec2`, `Rect`, `TSTransform`) so `pcb-doc` (`L1`) compiles and tests headlessly in milliseconds without pulling in `egui` or windowing crates.
hint: Check the dependency graph between `L0` (`emath`), `L1` (`pcb-doc`), and `L6`/`L7` (`pcb-reviewer`).

Q: What does the Tier 1 test `l1_manifest_gate_rejects_ui_dependencies_and_accepts_pcb_doc_manifest` verify?
- [ ] That every component footprint lies inside the millimetre bounding box
- [ ] That `egui_kittest` renders every custom layer swatch at full opacity
- [x] That `crates/pcb-doc/Cargo.toml` never declares forbidden `L6+` UI crate dependencies anywhere
- [ ] That `cargo-mutants` skips all private helper functions during CI runs
why: Inspecting `include_str!("../Cargo.toml")` with `validate_l1_manifest_no_ui_deps` guarantees that no developer accidentally adds `egui`, `eframe`, `winit`, or `rfd` to the `L1` domain crate.
hint: Look at `FORBIDDEN_UI_DEPS_BELOW_L6` in `crates/pcb-doc/src/lib.rs`.

Q: Why do we place `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` on `pcb-doc`?
- [ ] To allow `unsafe` raw pointer arithmetic inside hot canvas loops
- [x] To guarantee domain queries return typed values instead of panicking
- [ ] To force `rustc` to inline every public method across crates
- [ ] To disable all debug assertions when running `egui_kittest` harness tests
why: Forbidding `.unwrap()`, `.expect()`, and `panic!()` in library code forces domain operations to use `Option` and `Result`, preventing unexpected crashes during interactive CAD sessions.
hint: Review the crate-level lint attributes at the top of `crates/pcb-doc/src/lib.rs`.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [The Cargo Book — §3.3 "Workspaces"](https://doc.rust-lang.org/cargo/reference/workspaces.html) and [`emath` Official Crate Docs (`Pos2`, `Vec2`, `Rect`, `TSTransform`)](https://docs.rs/emath/latest/emath/).
:::

:::teacher-prompt
Next up: [Lesson 0016: Loading & Saving Board & Review Session Files (`serde`, `serde_json` & `thiserror`)](./0016-loading-board-and-review-files-serde.html), where we add versioned JSON persistence and panic-free `SessionIoError` banners to `pcb-doc` and `pcb-reviewer`!
:::

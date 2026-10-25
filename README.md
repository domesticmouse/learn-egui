# Learn Rust & `egui` — Schematic and PCB Design Review System

Interactive engineering course for learning **Rust** and **[`egui`](https://docs.rs/egui/latest/egui/)** by building a 2D CAD Schematic and PCB Design Review System, verified continuously with unit tests (`cargo test`) and mutation testing ([`cargo-mutants`](https://mutants.rs/)).

## Live Course on GitHub Pages

- **[Course Index](https://domesticmouse.github.io/learn-egui/)**
- **Lessons:**
  - [Lesson 0000: Bootstrapping Your Rust & `eframe` Project](https://domesticmouse.github.io/learn-egui/lessons/0000-project-setup-and-eframe-boot.html) (`apps/0000-project-setup-and-eframe-boot`)
  - [Lesson 0001: The Immediate-Mode Loop & Mutable State (`&mut`)](https://domesticmouse.github.io/learn-egui/lessons/0001-the-immediate-mode-loop.html) (`apps/0001-the-immediate-mode-loop`)
  - [Lesson 0002: Custom 2D Canvas & Drawing Primitives with `Painter`](https://domesticmouse.github.io/learn-egui/lessons/0002-custom-2d-canvas-and-painter.html) (`apps/0002-custom-2d-canvas-and-painter`)
  - [Lesson 0003: World-to-Screen Transforms, Drag Panning & Cursor-Centred Zoom](https://domesticmouse.github.io/learn-egui/lessons/0003-world-to-screen-transforms-pan-and-zoom.html) (`apps/0003-world-to-screen-transforms-pan-and-zoom`)
  - [Lesson 0004: Modelling PCB Layers, Traces & Deterministic Z-Ordering](https://domesticmouse.github.io/learn-egui/lessons/0004-modeling-layers-traces-and-footprints.html) (`apps/0004-modeling-layers-traces-and-footprints`)
  - [Lesson 0005: Spatial Hit-Testing, Selection State & Net Cross-Probing](https://domesticmouse.github.io/learn-egui/lessons/0005-hit-testing-and-net-cross-probing.html) (`apps/0005-hit-testing-and-net-cross-probing`)
  - [Lesson 0006: Schematic Symbols, Text Rendering & Headless UI Verification (`egui_kittest`)](https://domesticmouse.github.io/learn-egui/lessons/0006-schematic-symbols-and-text-rendering.html) (`apps/0006-schematic-symbols-and-text-rendering`)
  - [Lesson 0007: Split Viewports (`egui::Scene`) & Schematic-to-PCB Cross-Probing](https://domesticmouse.github.io/learn-egui/lessons/0007-split-viewports-and-egui-scene.html) (`apps/0007-split-viewports-and-egui-scene`)
  - [Lesson 0008: Searchable Netlist & Virtualised BOM Inspector](https://domesticmouse.github.io/learn-egui/lessons/0008-searchable-netlist-and-bom-inspector.html) (`apps/0008-searchable-netlist-and-bom-inspector`)
  - [Lesson 0009: Keyboard Shortcuts (`F`, `M`, `Esc`) & Point-to-Point CAD Ruler](https://domesticmouse.github.io/learn-egui/lessons/0009-keyboard-shortcuts-and-point-to-point-ruler.html) (`apps/0009-keyboard-shortcuts-and-point-to-point-ruler`)
  - [Lesson 0010: Review Pins, `HashMap` Entry API & Floating Callout Windows](https://domesticmouse.github.io/learn-egui/lessons/0010-review-pins-and-sticky-note-callouts.html) (`apps/0010-review-pins-and-sticky-note-callouts`)
  - [Lesson 0011: Review Status Triage, Severity Filtering & Sign-Off Rules](https://domesticmouse.github.io/learn-egui/lessons/0011-review-status-workflow-and-popups.html) (`apps/0011-review-status-workflow-and-popups`)
  - [Lesson 0012: DRC Clearance Violation Halos, Bounding-Box Reduction & Minimap Clamping](https://domesticmouse.github.io/learn-egui/lessons/0012-drc-violation-overlay-and-minimap.html) (`apps/0012-drc-violation-overlay-and-minimap`)
  - [Lesson 0013: Custom `egui::Widget` Layer Swatches, Struct Lifetimes (`'a`) & `Harness::new_ui_state`](https://domesticmouse.github.io/learn-egui/lessons/0013-custom-layer-swatch-widget-and-design-tokens.html) (`apps/0013-custom-layer-swatch-widget-and-design-tokens`)
  - [Lesson 0014: Multi-Crate Workspace — Separating `pcb-doc` (`L1`) from `pcb-reviewer` (`L6`)](https://domesticmouse.github.io/learn-egui/lessons/0014-multi-crate-workspace-doc-vs-ui-egui.html) (`apps/0014-multi-crate-workspace-doc-vs-ui-egui`)
  - [Lesson 0015: Loading & Saving Board & Review Session Files (`serde`, `serde_json` & `thiserror`)](https://domesticmouse.github.io/learn-egui/lessons/0015-loading-board-and-review-files-serde.html) (`apps/0015-loading-board-and-review-files-serde`)
  - [Lesson 0016: Copy-on-Write Document Snapshots with `Arc` & `Arc::make_mut`](https://domesticmouse.github.io/learn-egui/lessons/0016-copy-on-write-arc-document-snapshots.html) (`apps/0016-copy-on-write-arc-document-snapshots`)
  - [Lesson 0017: Undo/Redo History (`VecDeque`) & Continuous Slider Drag Coalescing](https://domesticmouse.github.io/learn-egui/lessons/0017-undo-redo-history-and-slider-coalescing.html) (`apps/0017-undo-redo-history-and-slider-coalescing`)
  - [Lesson 0018: The Command Pattern, `CommandSpec` Registry & `panic_hunt` Fuzzer](https://domesticmouse.github.io/learn-egui/lessons/0018-the-command-pattern-and-command-palette.html) (`apps/0018-the-command-pattern-and-command-palette`)
  - [Lesson 0019: Spatial Viewport Culling (`Rect::intersects`), Batched Geometry & Frame Budgets](https://domesticmouse.github.io/learn-egui/lessons/0019-viewport-culling-and-cached-tessellation.html) (`apps/0019-viewport-culling-and-cached-tessellation`)
  - [Lesson 0020: Non-Blocking Background File Import (`std::sync::mpsc`) & Live `ProgressBar` UI](https://domesticmouse.github.io/learn-egui/lessons/0020-non-blocking-file-import-and-progress-ui.html) (`apps/0020-non-blocking-file-import-and-progress-ui`)
  - [Lesson 0021: Offscreen Snapshot Parity (`gpu_lock`), Quality Scorecards & WebAssembly (`wasm32`)](https://domesticmouse.github.io/learn-egui/lessons/0021-headless-snapshot-testing-and-wasm-build.html) (`apps/0021-headless-snapshot-testing-and-wasm-build`)
- **Reference & Cheat Sheets:**
  - [Glossary of Rust & `egui` Terms](https://domesticmouse.github.io/learn-egui/reference/glossary.html)
  - [Immediate-Mode GUI Cheat Sheet](https://domesticmouse.github.io/learn-egui/reference/immediate-mode-cheatsheet.html)
  - [Testing & Mutation Coverage Cheat Sheet](https://domesticmouse.github.io/learn-egui/reference/testing-and-mutation-cheatsheet.html)

## Running the Companion Rust Applications

Each lesson has a runnable companion Cargo crate in `./apps/<NNNN-lesson-slug>/`:

```bash
# Run Lesson 0005's interactive PCB Reviewer desktop app
cargo run --manifest-path ./apps/0005-hit-testing-and-net-cross-probing/Cargo.toml

# Run unit tests and mutation testing
cargo test --manifest-path ./apps/0005-hit-testing-and-net-cross-probing/Cargo.toml
cargo mutants -d ./apps/0005-hit-testing-and-net-cross-probing
```

## Building the Course Pages from Markdown (`build.py`)

All course pages (`index.md`, `lessons/*.md`, and `reference/*.md`) are authored in Markdown and compiled to standalone HTML via [`build.py`](./build.py):

```bash
# Compile all Markdown pages in-place and run link, frontmatter, and equal-word-count quiz checks
.venv/bin/python build.py --check

# Or build a standalone bundle into _site/ (used by GitHub Pages CI)
.venv/bin/python build.py --out-dir _site --check
```

## License

Licensed under the [Apache License, Version 2.0](./LICENSE.txt).

## Disclaimer

This is not an official Google product.

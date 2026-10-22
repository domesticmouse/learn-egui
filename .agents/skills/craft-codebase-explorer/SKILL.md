---
name: craft-codebase-explorer
description: >-
  Use this skill when a developer or agent asks to explore, navigate, onboard to,
  or understand the PhotoCraft codebase: mapping the 25 workspace crates (L0–L7),
  locating where a menu item/command/tool/panel/file format/shader lives,
  tracing end-to-end execution flows, or inspecting the live/headless engine,
  parity checklists, and scorecard metrics.
---

# Exploring & Understanding the PhotoCraft Codebase

PhotoCraft is a native, pure-Rust, Photoshop-comparable image editor (~540k LoC across 25 library crates, 4 apps, and `xtask`). Every user-visible action flows through a single command engine (`crates/engine`), and strict `L0`–`L7` acyclic layering keeps the core independent of the `egui`/`wgpu` UI shell.

---

## 1. 5-Minute Orientation: Which Document Answers What

Start with the authoritative root and `docs/` guides depending on the question:

| Question / Topic | Primary Source |
|---|---|
| High-level rules, golden rules ("Never crash"), workflow | `AGENTS.md` |
| Crate dependency graph, engine/UI seam, design history | `docs/architecture.md` and `xtask/src/layers.rs` |
| Building, fast test loops (`nextest`), logs, env vars, corpora | `docs/development.md` |
| How to add a command, clean-room & safety rules | `docs/contributing.md` |
| Driving & screenshotting a live or headless instance via JSON | `docs/control-protocol.md` |
| Design tokens, themes, widgets, matching Photoshop's UI | `docs/ui-design.md` |
| Current stage, overall Photoshop 2026 parity & ranked gaps | `ROADMAP.md`, `docs/target-app-parity.md`, `docs/gaps.md` |
| Measured performance budgets, corpus floors, dead settings | `docs/scorecard.md` (from `scorecard/*.toml` & `perf/budgets.toml`) |
| Every Photoshop menu item and whether it is wired | `docs/parity-checklist.md` (generated from `crates/ui-egui/src/menu_catalog.rs`) |

---

## 2. Where to Find Things (`L0`–`L7` Quick Lookup)

For a complete file-by-file breakdown of all 25 crates and 4 apps, read [crate-and-module-atlas.md](./references/crate-and-module-atlas.md).

- **Geometry, ICC Color, Pixel Math, COW Tiles (`L0`):** `crates/geom`, `crates/cms`, `crates/color`, `crates/raster`
- **Standalone File Parsers & OS Helpers (`L0 Standalone`, zero workspace deps):**
  - `crates/psd` (PSD/PSB parser/writer), `crates/codecs` (PNG, JPEG, TIFF, WebP, EXR, GIF, TGA, BMP, ICO, DDS, DPX, HDR, PNM, SVG, PDF), `crates/raw` (DNG/CR2/NEF/ARW camera raw), `crates/heif` (HEIC/AVIF), `crates/affinity` (`.afphoto`/`.afdesign`), `crates/adobe-assets` (`.abr`, `.aco`, `.ase`, `.act`, `.grd`, `.pat`, `.csh`, `.atn`, `.asl`, `.tpl`, `.kys`)
  - `crates/tablet` (macOS AppKit / X11 / Windows pen pressure & tilt — the only crate with isolated `unsafe`), `crates/ipp` (IPP printing client)
- **Document Data Model (`L1`):** `crates/doc/src/lib.rs` (`Document`, `Layer`, `LayerContent`, `Mask`, `Adjustment`, `LayerEffects`, `SmartObject`)
- **History & Domain Engines (`L2`):**
  - `crates/ops` (`History`, `LayerTarget` undo/redo)
  - `crates/algo` (filters, adjustments, selections, healing/PatchMatch, warp/liquify, resampling)
  - `crates/paint` (brush dab engine, dynamics, stroke smoothing)
  - `crates/text` (font DB, `parley`/`skrifa`/`harfrust` shaping, CJK/craft-fonts, glyph rasterization)
  - `crates/vector` (Bezier paths, boolean ops, shape rasterization)
- **Compositors & Native Persistence (`L3`):**
  - `crates/compose` (CPU reference compositor — the oracle)
  - `crates/gpu` (`wgpu` compositor, WGSL shaders, tile residency cache, $\le 1/255$ parity with `compose`)
  - `crates/format` (`.pcraft` content-addressed `blake3` + `zstd` bundle; fails compilation if a `doc` field is omitted)
- **Format Mapping & Plugins (`L4`):**
  - `crates/io` (`Document` $\leftrightarrow$ PSD, layered TIFF, ORA, KRA, XCF, PDN, Affinity, RAW, flat formats)
  - `crates/plugins` (sandboxed `wasmi` WebAssembly filter plug-ins)
- **Session & Command Registry (`L5`):**
  - `crates/engine/src/lib.rs` (`Session`, `DocState`, `Session::execute`, `Session::edit`)
  - `crates/engine/src/commands.rs` (`CommandSpec` registry aggregating `*_cmds.rs`)
- **UI Shell & Automation (`L6`):**
  - `crates/ui-egui` (`state.rs` `UiState`, `menu_catalog.rs`, `menus.rs`, `canvas.rs`, `panels.rs`, `dialogs.rs`, `filter_dialog.rs`, `control.rs`)
  - `crates/automation` (`PhotocraftMcp` MCP server, `Headless` session wrapper, `rpc.rs` JSON-lines server)
- **Applications & Build Tooling (`L7`):**
  - `apps/photocraft` (desktop app), `apps/photocraft-cli` (headless CLI), `apps/photocraft-web` & `apps/photocraft-web-boot` (browser Wasm), `xtask` (workspace automation)

---

## 3. Tracing Common Code Flows

See [feature-tracing-and-inspection.md](./references/feature-tracing-and-inspection.md) for concrete step-by-step walkthroughs and CLI/control commands to trace:

1. **Menu Item $\to$ Engine Command $\to$ Algorithm $\to$ Undo Step:**
   Start at `crates/ui-egui/src/menu_catalog.rs` (find the canonical command ID) $\to$ search `crates/engine/src/*_cmds.rs` for that `id` $\to$ follow its `run` closure into `Session::edit` and the `L2` algorithm (`crates/algo`, `paint`, `vector`, `text`).
2. **Interactive Tool Gesture $\to$ Canvas Overlay $\to$ Engine Execution:**
   Start at `crates/ui-egui/src/state.rs` (`Tool` enum & tool state structs) $\to$ `crates/ui-egui/src/canvas.rs` & tool modules (`brush_*.rs`, `crop_*.rs`, `transform_*.rs`) $\to$ `Session::execute` (often with `"coalesce"` key for live drag updates).
3. **File Open / Save Pipeline:**
   Start at `crates/io/src/lib.rs` (`psd_import.rs`, `psd_export.rs`, `flat.rs`, `raw.rs`, `xcf.rs`, `ora.rs`, `kra.rs`, `affinity.rs`) $\to$ `crates/psd` or `crates/codecs` $\to$ `crates/doc`.
4. **Canvas Redraw & Color Pipeline:**
   Start at `crates/ui-egui/src/canvas.rs` (`DocState::revision` / `last_damage`) $\to$ `crates/compose/src/lib.rs` (CPU) or `crates/gpu/src/lib.rs` (GPU) $\to$ `crates/engine/src/display_color.rs` (`photocraft-cms` matrix/shaper curves or 33³ 3D LUT).

---

## 4. Live & Headless Exploration Commands

Run these commands to inspect the codebase structure, command registry, and runtime state without reading every file manually:

```sh
# 1. Print the crate dependency graph and verify L0-L7 layering rules
cargo xtask layers

# 2. Count lines of code and #[test] functions per crate
cargo xtask stats

# 3. Search registered engine commands, shortcuts, and parameter schemas
cargo run -p photocraft-cli -- commands --filter blur
cargo run -p photocraft-cli -- commands --json

# 4. Inspect a document's layer tree, color mode, and bit depth as JSON
cargo run -p photocraft-cli -- info path/to/file.psd

# 5. Render an offscreen UI snapshot (no window or display server required)
cargo run -p photocraft-ui-egui --example snapshot
```

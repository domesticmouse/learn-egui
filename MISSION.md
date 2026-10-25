# Mission: Rust & egui for a Schematic and PCB Design Review System

## Why
Build a responsive Schematic and PCB design review application in Rust using `egui`, enabling engineers (and AI engineering co-pilots via MCP) to inspect real KiCad schematics and board layouts, toggle layers and nets, annotate DRC/review pins backed by an embedded SQL database, and scale into a multi-crate, engine-first application verified continuously with unit/headless tests and `cargo-mutants` mutation coverage.

## Success looks like
- Write idiomatic Rust state models (`struct`, `enum`, `&mut` borrows, closures, traits, smart pointers, `async`/`await`) that cleanly drive an `eframe::App` immediate-mode update loop.
- Verify every feature as it is built using `cargo test`, headless `egui_kittest` harnesses, and `cargo-mutants` mutation testing—tracking caught vs. missed mutants across every lesson so test coverage measures real assertion strength rather than mere line execution.
- Build an interactive 2D CAD canvas (`ui.allocate_painter`, `TSTransform`, `egui::Scene`) with smooth pan/zoom, layer ordering, hit-testing, and schematic-to-PCB cross-probing.
- Implement design review workflows: virtualized BOM/netlist tables, point-to-point measurement rulers, anchored review callout pins, DRC error overlays, and custom layer-swatch widgets.
- Refactor the prototype into an engine-first multi-crate workspace (`doc` vs. `ui-egui`) with `serde` persistence, copy-on-write (`Arc`) undo/redo history, a unified `CommandSpec` palette, viewport culling, background file loading, and the 7-tier verification architecture (`proptest`, `panic_hunt`, `gpu_lock` visual snapshots, and performance budgets).
- Parse real KiCad `.kicad_pcb` and `.kicad_sch` S-expression files using `nom` parser combinators (verified against pinned real-file corpus oracles), and persist/sync multi-engineer review threads and DRC sign-off audit trails in an embedded Turso (`libsql`) SQL database.
- Expose the PCB Reviewer as a Model Context Protocol (MCP) server using `rmcp` (supporting both Headless stdio/in-memory mode and Bridged live-GUI TCP control mode) with LLM-native spatial/topological queries, custom DRC checks, cropped visual region snapshots, and a companion **Mock MCP Client & Inspector Workbench** app to test and drive multi-step agent playbooks.

## Constraints
- Learning Rust alongside `egui` — each lesson introduces one focused `egui` capability paired just-in-time with one Rust concept and a concrete `cargo test` + `cargo mutants` verification step, keeping cognitive load low.

## Out of scope
- Automated PCB trace routing (autorouting) or SPICE analog/digital circuit simulation.
- 3D mechanical MCAD enclosure rendering.

# Mission: Rust & egui for a Schematic and PCB Design Review System

## Why
Build a responsive Schematic and PCB design review application in Rust using `egui`, enabling engineers to inspect real KiCad schematics and board layouts, toggle layers and nets, annotate DRC/review pins backed by an embedded SQL database, and scale into a multi-crate, engine-first application with undo/redo and headless testing.

## Success looks like
- Write idiomatic Rust state models (`struct`, `enum`, `&mut` borrows, closures, traits, smart pointers, `async`/`await`) that cleanly drive an `eframe::App` immediate-mode update loop.
- Build an interactive 2D CAD canvas (`ui.allocate_painter`, `TSTransform`, `egui::Scene`) with smooth pan/zoom, layer ordering, hit-testing, and schematic-to-PCB cross-probing.
- Implement design review workflows: virtualized BOM/netlist tables, point-to-point measurement rulers, anchored review callout pins, DRC error overlays, and custom layer-swatch widgets.
- Refactor the prototype into an engine-first multi-crate workspace (`doc` vs. `ui-egui`) with `serde` persistence, copy-on-write (`Arc`) undo/redo history, a unified `CommandSpec` palette, viewport culling, background file loading, and headless/Wasm targets.
- Parse real KiCad `.kicad_pcb` and `.kicad_sch` S-expression files using `nom` parser combinators, and persist/sync multi-engineer review threads and DRC sign-off audit trails in an embedded Turso (`libsql`) SQL database.

## Constraints
- Learning Rust alongside `egui` — each lesson introduces one focused `egui` capability paired just-in-time with one Rust concept, keeping cognitive load low.

## Out of scope
- Automated PCB trace routing (autorouting) or SPICE analog/digital circuit simulation.
- 3D mechanical MCAD enclosure rendering.

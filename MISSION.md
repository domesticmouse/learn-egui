# Mission: Rust & egui for a Schematic and PCB Design Review System

## Why
Build a responsive Schematic and PCB design review application in Rust using `egui`, enabling engineers to inspect schematics and board layouts, toggle layers and nets, annotate DRC/review pins, and scale into a multi-crate, engine-first application with undo/redo and headless testing.

## Success looks like
- Write idiomatic Rust state models (`struct`, `enum`, `&mut` borrows, closures, traits, smart pointers) that cleanly drive an `eframe::App` immediate-mode update loop.
- Build an interactive 2D CAD canvas (`ui.allocate_painter`, `TSTransform`, `egui::Scene`) with smooth pan/zoom, layer ordering, hit-testing, and schematic-to-PCB cross-probing.
- Implement design review workflows: virtualized BOM/netlist tables, point-to-point measurement rulers, anchored review callout pins, DRC error overlays, and custom layer-swatch widgets.
- Refactor the prototype into an engine-first multi-crate workspace (`doc` vs. `ui-egui`) with `serde` persistence, copy-on-write (`Arc`) undo/redo history, a unified `CommandSpec` palette, viewport culling, background file loading, and headless/Wasm targets.

## Constraints
- Learning Rust alongside `egui` — each lesson introduces exactly one `egui` capability paired just-in-time with one Rust concept, keeping cognitive load low.

## Out of scope
- Automated PCB trace routing (autorouting) or SPICE analog/digital circuit simulation.
- 3D mechanical MCAD enclosure rendering.

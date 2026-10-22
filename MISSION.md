# Mission: Rust & egui for a Schematic and PCB Design Review System

## Why
Build a responsive Schematic and PCB design review application in Rust using `egui`, enabling engineers to inspect schematics and board layouts, toggle layers and nets, and review designs interactively.

## Success looks like
- Write idiomatic Rust state models (`struct`, `enum`, `&mut` borrows, closures) that cleanly drive an `eframe::App` immediate-mode update loop.
- Build an interactive 2D canvas (`egui::Scene` and `ui.allocate_painter`) with smooth pan and zoom for viewing schematic symbols, nets, and PCB footprints/traces.
- Wire up side panels and toolbars that toggle PCB layer visibility (e.g., Top Copper, Bottom Copper, Silkscreen) and highlight nets on the canvas in real time.

## Constraints
- Learning Rust alongside `egui` — Rust concepts (ownership, `&mut` mutable references, closures, pattern matching) must be taught just-in-time as `egui` requires them, keeping cognitive load low.

## Out of scope
- Automated PCB trace routing (autorouting) or SPICE analog/digital circuit simulation.
- 3D mechanical MCAD enclosure rendering.

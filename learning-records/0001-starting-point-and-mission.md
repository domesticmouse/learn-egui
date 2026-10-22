# Starting Point and Mission: Learning Rust & egui Together for a PCB/Schematic Reviewer

The learner is building towards a **Schematic and PCB design review system** (first milestone: an interactive 2D canvas with pan, zoom, and layer/net visibility toggles) while **learning Rust alongside `egui`**.

## Implications
- Never assume prior fluency with Rust's borrow checker (`&mut`), closure syntax (`|ui| { ... }`), or `impl` blocks; introduce each Rust construct immediately before the `egui` widget or pattern that uses it.
- Ground every example and exercise in the domain of schematic and PCB design review (layers, nets, components, pads, coordinate viewports) rather than generic todo lists or counters.

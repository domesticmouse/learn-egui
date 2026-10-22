# Teaching Notes & Preferences

- **Target Application**: Schematic and PCB design review system in Rust + `egui`.
- **First Milestone**: Interactive 2D canvas with pan, zoom, and layer/net visibility toggles for schematics and PCB layouts.
- **Rust Background**: Learning Rust alongside `egui` — introduce Rust syntax (`struct`, `&mut`, closures, `enum`, `Option`) just-in-time as each `egui` concept requires it.
- **Planned Progression**:
  - `0001-the-immediate-mode-loop.html`: `eframe::App::update`, `&mut self` state, `SidePanel` + `CentralPanel`, `ui.checkbox`, `ui.button().clicked()` for PCB layer toggles.
  - `0002-custom-2d-canvas-and-painter.html`: `ui.allocate_painter`, `Rect`, `Pos2`, `Vec2`, `Stroke`, drawing PCB pads/traces/vias in screen space.
  - `0003-pan-zoom-and-coordinate-transforms.html`: World-space (mm/mils) vs. Screen-space (points) transforms, `egui::Scene` vs. custom pan/zoom math, `Sense::click_and_drag()`.
  - `0004-hit-testing-and-net-highlighting.html`: Selecting nets/components on the canvas, cross-probing with the inspector panel using Rust `enum`s and `Option<NetId>`.

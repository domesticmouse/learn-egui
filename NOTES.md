# Teaching Notes & Preferences

- **User Formatting Preferences**:
  - **Dark Mode**: Use dark mode across all lessons and reference documents (`<meta name="color-scheme" content="dark">`, `:root { color-scheme: dark; }`, dark Tufte palette in `./assets/style.css`, and Mermaid `theme: "dark"`, with `@media print` overrides for clean printing).
  - Always use **Prism.js** (`<pre><code class="language-rust">`, `language-toml`, `language-bash`, etc.) for syntax highlighting across all lessons and reference documents.
  - Always use **Mermaid.js** (`<pre class="mermaid">`) for architecture, lifecycle, and flow diagrams (avoid ASCII art diagrams).
- **Target Application**: Schematic and PCB design review system in Rust + `egui`.
- **First Milestone**: Interactive 2D canvas with pan, zoom, and layer/net visibility toggles for schematics and PCB layouts.
- **Rust Background**: Learning Rust alongside `egui` — introduce Rust syntax (`Cargo.toml`, `#[derive(Default)]`, `Result`, `Box`, `struct`, `&mut`, closures, `enum`, `Option`) just-in-time as each `egui` concept requires it.
- **Installed Toolchain**: `cargo 1.97.1` / `rustc 1.97.1` on macOS.
- **Planned Progression**:
  - `0000-project-setup-and-eframe-boot.html`: Creating the Cargo crate (`Cargo.toml`, `eframe = "0.33"`, dev-profile optimization), `#[derive(Default)]`, and booting the native window with `eframe::run_native` so Lesson 0001's `update()` loop has a complete, runnable home.
  - `0001-the-immediate-mode-loop.html`: `eframe::App::update`, `&mut self` state, `SidePanel` + `CentralPanel`, `ui.checkbox`, `ui.button().clicked()` for PCB layer toggles.
  - `0002-custom-2d-canvas-and-painter.html`: `ui.allocate_painter`, `Rect`, `Pos2`, `Vec2`, `Stroke`, drawing PCB pads/traces/vias in screen space.
  - `0003-pan-zoom-and-coordinate-transforms.html`: World-space (mm/mils) vs. Screen-space (points) transforms, `egui::Scene` vs. custom pan/zoom math, `Sense::click_and_drag()`.
  - `0004-hit-testing-and-net-highlighting.html`: Selecting nets/components on the canvas, cross-probing with the inspector panel using Rust `enum`s and `Option<NetId>`.

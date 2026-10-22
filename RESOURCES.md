# Rust & egui Resources

## Knowledge

- [Official API Docs: `egui` on docs.rs](https://docs.rs/egui/latest/egui/)
  Primary reference by Emil Ernerfeldt covering the immediate-mode architecture, `Context`, `Ui`, `Painter`, `Scene`, and built-in widgets. Use for: authoritative explanations of widget signatures, coordinate spaces, and frame lifecycle.
- [Official API Docs: `eframe` on docs.rs](https://docs.rs/eframe/latest/eframe/)
  Official application framework wrapper that runs `egui` natively (via `winit`/`wgpu`/`glow`) and on the web (WebAssembly). Use for: setting up `eframe::App`, window configuration, and persistence.
- [GitHub Repository: `emilk/egui` Examples](https://github.com/emilk/egui/tree/master/examples)
  Canonical, runnable examples maintained alongside `egui` itself. Use for: minimal working patterns for custom painting, custom widgets, and multi-panel layouts.
- [Interactive Web Demo: `egui.rs/#demo`](https://www.egui.rs/#demo)
  Live WebAssembly build of `egui_demo_lib` with direct links to the exact Rust source line for every widget and canvas demo (including the `Painting` and `Scene` demos). Use for: inspecting how `egui` behaviors feel before coding them.
- [Book: _The Rust Programming Language_ ("The Book")](https://doc.rust-lang.org/book/)
  The official foundational text on Rust ownership, borrowing (`&` and `&mut`), structs, enums, and closures (`|ui|`). Use for: grounding the Rust mechanics that make `egui`'s API ergonomic.
- [Local Skill: `egui-app-architecture`](file:///Users/brett/tmp/learn-egui/.agents/skills/egui-app-architecture/SKILL.md)
  Workspace guide on structuring larger engine-first `egui` applications with strict `L0`–`L7` crate layering. Use for: scaling the prototype into a production multi-crate CAD/review system once core `egui` skills are solid.

## Wisdom (Communities)

- [`emilk/egui` GitHub Discussions](https://github.com/emilk/egui/discussions)
  Official Q&A and architecture forum monitored by maintainers and experienced practitioners. Use for: design review of custom canvas widgets, performance tuning, and idiomatic patterns.
- [Official `egui` Discord Server](https://discord.gg/JFcEma9bJq)
  Active real-time community with `#help` and `#showcase` channels where developers share CAD, game editor, and scientific visualization tools built in `egui`. Use for: quick feedback and seeing how others structure 2D/3D editors.
- [The Rust Programming Language Users Forum (`URLO`)](https://users.rust-lang.org/)
  High-signal, well-moderated forum for Rust learners and experts alike. Use for: borrow-checker questions when structuring application state across panels and canvases.

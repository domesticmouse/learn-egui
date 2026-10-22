# Learn Rust & `egui` — Schematic and PCB Design Review System

Interactive engineering course for learning **Rust** and **[`egui`](https://docs.rs/egui/latest/egui/)** by building a 2D CAD Schematic and PCB Design Review System, verified continuously with unit tests (`cargo test`) and mutation testing ([`cargo-mutants`](https://mutants.rs/)).

## Live Course on GitHub Pages

- **[Course Index](https://domesticmouse.github.io/learn-egui/)**
- **Lessons:**
  - [Lesson 0000: Bootstrapping Your Rust & `eframe` Project](https://domesticmouse.github.io/learn-egui/lessons/0000-project-setup-and-eframe-boot.html) (`apps/0000-project-setup-and-eframe-boot`)
  - [Lesson 0001: The Immediate-Mode Loop & Mutable State (`&mut`)](https://domesticmouse.github.io/learn-egui/lessons/0001-the-immediate-mode-loop.html) (`apps/0001-the-immediate-mode-loop`)
  - [Lesson 0002: Custom 2D Canvas & Drawing Primitives with `Painter`](https://domesticmouse.github.io/learn-egui/lessons/0002-custom-2d-canvas-and-painter.html) (`apps/0002-custom-2d-canvas-and-painter`)
  - [Lesson 0003: World-to-Screen Transforms, Drag Panning & Cursor-Centered Zoom](https://domesticmouse.github.io/learn-egui/lessons/0003-world-to-screen-transforms-pan-and-zoom.html) (`apps/0003-world-to-screen-transforms-pan-and-zoom`)
  - [Lesson 0004: Modeling PCB Layers, Traces & Deterministic Z-Ordering](https://domesticmouse.github.io/learn-egui/lessons/0004-modeling-layers-traces-and-footprints.html) (`apps/0004-modeling-layers-traces-and-footprints`)
  - [Lesson 0005: Spatial Hit-Testing, Selection State & Net Cross-Probing](https://domesticmouse.github.io/learn-egui/lessons/0005-hit-testing-and-net-cross-probing.html) (`apps/0005-hit-testing-and-net-cross-probing`)
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

## License

Licensed under the [Apache License, Version 2.0](./LICENSE.txt).

## Disclaimer

This is not an official Google product.

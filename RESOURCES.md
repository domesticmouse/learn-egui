# Rust & egui Resources

## Knowledge

- [Official API Docs: `egui` on docs.rs](https://docs.rs/egui/latest/egui/)
  Primary reference by Emil Ernerfeldt covering the immediate-mode architecture, `Context`, `Ui`, `Painter`, `Scene`, and built-in widgets. Use for: authoritative explanations of widget signatures, coordinate spaces, and frame lifecycle.
- [Official API Docs: `eframe` on docs.rs](https://docs.rs/eframe/latest/eframe/)
  Official application framework wrapper that runs `egui` natively (via `winit`/`wgpu`/`glow`) and on the web (WebAssembly). Use for: setting up `eframe::App`, window configuration, and persistence.
- [Official User Guide: `cargo-mutants` (`mutants.rs`)](https://mutants.rs/) and [`sourcefrog/cargo-mutants` on GitHub](https://github.com/sourcefrog/cargo-mutants)
  Official documentation by Martin Pool for Rust mutation testing. Explains how `cargo mutants` injects deliberate logic faults into functions to verify whether `cargo test` catches them (`Caught` vs. `Missed` vs. `Unviable` mutants), and how to configure `.cargo/mutants.toml`. Use for: monitoring true assertion coverage in every lesson from `0000` through `0025`.
- [Official API Docs: `egui_kittest` on docs.rs](https://docs.rs/egui_kittest/latest/egui_kittest/)
  Official headless UI test runner and visual snapshot harness for `egui` (inspired by Testing Library). Supports querying widgets by label/role, simulating clicks and key presses, and rendering offscreen `wgpu` images. Use for: testing `eframe::App` panels, custom `egui::Widget`s, and visual snapshots without opening an OS window.
- [Official Book & API Docs: `proptest` on docs.rs](https://docs.rs/proptest/latest/proptest/)
  Property-based testing framework for Rust that generates random structured inputs and automatically shrinks failing counterexamples to minimal reproductions. Use for: verifying coordinate transforms, spatial culling, `Arc` copy-on-write undo/redo histories, and parser round-trips.
- [GitHub Repository: `emilk/egui` Examples](https://github.com/emilk/egui/tree/master/examples)
  Canonical, runnable examples maintained alongside `egui` itself. Use for: minimal working patterns for custom painting, custom widgets, and multi-panel layouts.
- [Interactive Web Demo: `egui.rs/#demo`](https://www.egui.rs/#demo)
  Live WebAssembly build of `egui_demo_lib` with direct links to the exact Rust source line for every widget and canvas demo (including the `Painting` and `Scene` demos). Use for: inspecting how `egui` behaviors feel before coding them.
- [Book: _The Rust Programming Language_ ("The Book")](https://doc.rust-lang.org/book/)
  The official foundational text on Rust ownership, borrowing (`&` and `&mut`), structs, enums, closures (`|ui|`), and automated tests ([Chapter 11: Writing Automated Tests](https://doc.rust-lang.org/book/ch11-00-testing.html)). Use for: grounding the Rust mechanics that make `egui`'s API and test suite ergonomic.
- [Official API Docs: `nom` on docs.rs](https://docs.rs/nom/latest/nom/)
  Byte- and slice-oriented, zero-copy parser combinator library in Rust. Use for: building a standalone `L0` S-expression parser for KiCad `.kicad_pcb` and `.kicad_sch` files in Act VI.
- [KiCad Developer Documentation: S-Expression File Formats](https://dev-docs.kicad.org/en/file-formats/)
  Official specification for KiCad's symbolic-expression (`.kicad_pcb`, `.kicad_sch`, `.kicad_sym`, `.kicad_mod`) grammar, coordinate systems, layers, pads, segments, and nets. Use for: mapping parsed S-expressions into typed Rust domain structs.
- [Official API Docs: `libsql` / Turso Rust SDK on docs.rs](https://docs.rs/libsql/latest/libsql/) and [Turso Rust Reference](https://docs.turso.tech/sdk/rust/reference)
  Official Rust SDK for Turso's embedded SQLite-compatible database engine (`libsql` / `turso`) supporting local database files, parameterized SQL queries, and embedded cloud-synced replicas. Use for: storing review pins, comment threads, and DRC sign-off audit trails in Act VI.
- [Local Skill: `egui-app-architecture`](./.agents/skills/egui-app-architecture/SKILL.md)
  Workspace guide on structuring larger engine-first `egui` applications with strict `L0`–`L7` crate layering. Use for: scaling the prototype into a production multi-crate CAD/review system once core `egui` skills are solid.
- [Local Skill: `egui-command-engine`](./.agents/skills/egui-command-engine/SKILL.md)
  Workspace guide on the three-tier command taxonomy (`CommandSpec`, shell UI commands, and automation transports), schema-driven dialogs, and `panic_hunt` fuzzing. Use for: Act V (`0018`) command architecture.
- [Local Skill: `egui-cow-document-model`](./.agents/skills/egui-cow-document-model/SKILL.md)
  Workspace guide on immutable copy-on-write (`Arc`) document trees, O(1) snapshot undo/redo history, and deterministic serialization. Use for: Act IV (`0017`) undo/redo engine.
- [Local Skill: `egui-testing-infrastructure`](./.agents/skills/egui-testing-infrastructure/SKILL.md)
  Workspace guide on the 7-tier verification architecture (`nextest`, `gpu_lock`, real-file corpus oracles, `panic_hunt` command fuzzing, headless `egui_kittest` snapshots, performance budgets, and scorecards). Use for: scaling our per-lesson `cargo test` + `cargo mutants` workflow into production CI gates.

## Wisdom (Communities)

- [`emilk/egui` GitHub Discussions](https://github.com/emilk/egui/discussions)
  Official Q&A and architecture forum monitored by maintainers and experienced practitioners. Use for: design review of custom canvas widgets, performance tuning, and idiomatic patterns.
- [Official `egui` Discord Server](https://discord.gg/JFcEma9bJq)
  Active real-time community with `#help` and `#showcase` channels where developers share CAD, game editor, and scientific visualization tools built in `egui`. Use for: quick feedback and seeing how others structure 2D/3D editors.
- [The Rust Programming Language Users Forum (`URLO`)](https://users.rust-lang.org/)
  High-signal, well-moderated forum for Rust learners and experts alike. Use for: borrow-checker questions when structuring application state across panels and canvases.

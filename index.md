---
title: Learn Rust & `egui` by Building a Schematic & PCB Design Reviewer
subtitle: A hands-on, mutation-tested engineering course from immediate-mode fundamentals to an engine-first CAD architecture
page_type: index
---

Learning a new GUI framework from toy counter apps is easy. Building a real tool with a 2D CAD canvas, spatial hit-testing, and undo history is where things get interesting.

So, in this course, we are going to learn Rust and Emil Ernerfeldt's [`egui`](https://github.com/emilk/egui) by building a **Schematic & PCB Design Reviewer** from scratch.{{sidenote:Every lesson in this repository pairs an interactive HTML tutorial (with live in-browser simulators, code-completion labs, and retrieval quizzes) with a real, runnable Cargo project under `./apps/<NNNN-slug>/`.}} Each lesson introduces **one focused `egui` capability** alongside **one just-in-time Rust concept**—and because we like keeping our sanity as the codebase grows, we verify our assertions continuously with `cargo test` and Martin Pool's [`cargo-mutants`](https://mutants.rs/).

```mermaid
flowchart LR
  L0["0000–0001\nFoundation & IMGUI Loop"] --> A1["Act I (0002–0005)\n2D CAD Canvas & Hit-Testing"]
  A1 --> A2["Act II (0006–0009)\nSchematics & egui_kittest"]
  A2 --> A3["Act III (0010–0013)\nDRC Overlays & Review Pins"]
  A3 --> A4["Act IV (0014–0017)\nMulti-Crate CoW Engine"]
  A4 --> A5["Act V (0018–0021)\nCommand Engine, Culling & Snapshots"]
  A5 --> A6["Act VI (0022–0025)\nKiCad nom Parser & Turso SQL"]
  A6 --> A7["Act VII (0026–0029)\nMCP Server (rmcp) & Agent Workbench"]
```
_Multi-Act Curriculum Progression: Every lesson builds toward an engine-first, AI-controllable Schematic & PCB Design Review System verified with unit tests and mutation coverage._

## Published Interactive Lessons (`0000`–`0021`)

Start with [Lesson 0000](./lessons/0000-project-setup-and-eframe-boot.html) to set up your Cargo project and mutation-testing harness, or jump straight into any completed module below:

{{widget:lesson-cards}}

## Reference Sheets & Printable Cheat Sheets

Turns out, having a one-page cheat sheet next to your editor saves a lot of tab-switching. These reference documents compress the core Rust, immediate-mode GUI, and mutation-testing mechanics into quick-lookup tables designed for both screen review and clean paper printing:

| Reference Sheet | What It Covers |
| --- | --- |
| [**Glossary of Rust & `egui` Terms**](./reference/glossary.html) | Canonical definitions of Immediate-Mode vs. Retained-Mode GUI, `&mut` borrows, closures, `TSTransform`, `Sense`, `Arc::make_mut`, and PCB domain analogies. |
| [**Immediate-Mode GUI Cheat Sheet**](./reference/immediate-mode-cheatsheet.html) | The 5-step single-frame mental model, panel layout ordering rules (`SidePanel` before `CentralPanel`), and custom 2D CAD painter idioms. |
| [**Testing & Mutation Coverage Cheat Sheet**](./reference/testing-and-mutation-cheatsheet.html) | Why line coverage lies, how `cargo-mutants` tests your tests, fast APFS copy-on-write workflows, and the evolutionary verification ladder (`proptest`, workspace gates, and `egui_kittest`). |

## Running the Companion Rust Apps Locally

Every lesson is backed by a standalone, runnable Cargo package or workspace under `./apps/<NNNN-lesson-slug>/` that compiles with zero warnings and passes both `cargo test` and `cargo mutants`:

```bash
# Launch Lesson 0021's interactive multi-crate PCB Reviewer desktop window
cargo run --manifest-path ./apps/0021-headless-snapshot-testing-and-wasm-build/Cargo.toml

# Run workspace unit + proptest + panic_hunt suites and fast L1 domain mutation checks
cargo test --manifest-path ./apps/0021-headless-snapshot-testing-and-wasm-build/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0021-headless-snapshot-testing-and-wasm-build/Cargo.toml -p pcb-doc --in-place
```

:::teacher-prompt
**Ready to dive in?** Start with [**Lesson 0000: Bootstrapping Your Rust & `eframe` Project**](./lessons/0000-project-setup-and-eframe-boot.html) or jump straight to [**Lesson 0021: Offscreen Snapshot Parity (`gpu_lock`), Quality Scorecards & WebAssembly (`wasm32`)**](./lessons/0021-headless-snapshot-testing-and-wasm-build.html).
:::

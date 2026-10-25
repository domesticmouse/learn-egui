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

## Published Interactive Lessons (`0000`–`0029`)

Start with [Lesson 0000](./lessons/0000-project-setup-and-eframe-boot.html) to set up your Cargo project and mutation-testing harness, or jump directly to any section using the curriculum table of contents below:

<nav class="act-toc" aria-label="Course curriculum table of contents">
  <div class="act-toc-title">Curriculum Table of Contents</div>
  <ol>
    <li><a href="#foundation">Foundation: Project Setup &amp; Immediate-Mode Core</a> <span class="act-toc-badge">Lessons 0000–0001</span></li>
    <li><a href="#act-1">Act I: The 2D CAD Canvas &amp; Viewport</a> <span class="act-toc-badge">Lessons 0002–0005</span></li>
    <li><a href="#act-2">Act II: Schematic Views, Layout &amp; egui_kittest</a> <span class="act-toc-badge">Lessons 0006–0009</span></li>
    <li><a href="#act-3">Act III: Design Review Workflows</a> <span class="act-toc-badge">Lessons 0010–0013</span></li>
    <li><a href="#act-4">Act IV: Engine-First Architecture &amp; Persistence</a> <span class="act-toc-badge">Lessons 0014–0017</span></li>
    <li><a href="#act-5">Act V: Command Engine, Performance &amp; Polish</a> <span class="act-toc-badge">Lessons 0018–0021</span></li>
    <li><a href="#act-6">Act VI: Real-World KiCad Parsing (nom) &amp; Embedded SQL (Turso)</a> <span class="act-toc-badge">Lessons 0022–0025</span></li>
    <li><a href="#act-7">Act VII: Agentic Hardware Review &amp; Model Context Protocol (rmcp)</a> <span class="act-toc-badge">Lessons 0026–0029</span></li>
  </ol>
</nav>

### Foundation: Project Setup & Immediate-Mode Core (`0000`–`0001`) {#foundation}

Every hardware review tool starts with a reliable application shell and a clear understanding of how immediate-mode GUIs manage state. In this foundational module, we configure a high-performance Cargo project with dev-profile optimizations, boot a native desktop window with `eframe::run_native`, and explore Emil Ernerfeldt’s single-frame immediate-mode loop (`eframe::App::update`) by building an interactive PCB layer visibility inspector backed by mutation testing.

{{widget:lesson-cards act="foundation"}}

### Act I: The 2D CAD Canvas & Viewport (`0002`–`0005`) {#act-1}

With a native window running, Act I transforms an empty `CentralPanel` into a responsive 2D CAD canvas. We reserve custom drawing space with `ui.allocate_painter`, master world-to-screen camera math for drag-panning and cursor-centered zoom, build deterministic multi-layer PCB models (substrate, copper traces, SMD pads, and vias), and implement spatial hit-testing for cross-probing interconnected nets under the mouse cursor.

{{widget:lesson-cards act="1"}}

### Act II: Schematic Views, Layout & `egui_kittest` (`0006`–`0009`) {#act-2}

Act II bridges PCB layouts with schematic diagrams and inspector tooling. We render vector schematic symbols and reference designators, coordinate side-by-side viewports with shared `egui::Scene` navigation, build a virtualized Bill of Materials (BOM) table that scales to thousands of components, implement keyboard shortcuts and an interactive point-to-point CAD ruler, and verify headless UI widgets using `egui_kittest`.

{{widget:lesson-cards act="2"}}

### Act III: Design Review Workflows (`0010`–`0013`) {#act-3}

Hardware design reviews require contextual annotations and clearance rule checks. Act III builds production review workflows: pinning interactive callout comments directly onto board coordinates, orchestrating a triage state machine with right-click context menus and severity filters, rendering Design Rule Check (DRC) violation halos with an interactive minimap, and packaging custom reusable `egui::Widget` layer swatches.

{{widget:lesson-cards act="3"}}

### Act IV: Engine-First Architecture & Persistence (`0014`–`0017`) {#act-4}

As applications grow, UI code must separate from pure domain logic. Act IV refactors our single-file app into a strict multi-crate workspace (`pcb-doc` L1 vs. `pcb-reviewer` L6), introduces robust JSON serialization and file format upgrades with `serde` and `thiserror`, adopts copy-on-write `Arc` document snapshots for $O(1)$ zero-cost branching, and implements a multi-level undo/redo history stack with slider drag coalescing.

{{widget:lesson-cards act="4"}}

### Act V: Command Engine, Performance & Polish (`0018`–`0021`) {#act-5}

Act V brings commercial CAD performance and reliability to the review system. We route all state mutations through a static `CommandSpec` registry with a searchable `Key::K` command palette and adversarial fuzz testing (`panic_hunt`), implement viewport bounding-box culling with cached tessellation for 60 FPS budgets, offload heavy file parsing to worker threads with non-blocking progress UI, and establish headless snapshot testing (`gpu_lock`) with WebAssembly (`wasm32`) deployment targets.

{{widget:lesson-cards act="5"}}

### Act VI: Real-World KiCad Parsing (`nom`) & Embedded SQL (`Turso`) (`0022`–`0025`) {#act-6}

Act VI connects our CAD engine to real-world hardware files and relational review databases. We extract a standalone `L0` S-expression parser crate (`crates/kicad-sexpr`) using `nom` combinators, lower `.kicad_pcb` S-expression trees into `BoardModel` verified by a Tier 4 pinned corpus oracle (`CORPUS_PASS_FLOOR` & `CORPUS_ROUNDTRIP_FLOOR`), validate SQLite/Turso DDL and DML syntax with `turso_parser` inside an async-tested `TursoReviewStore`, and execute atomic `BEGIN` / `COMMIT` / `ROLLBACK` sign-off batches backed by a tamper-evident hash-chained audit trail (`verify_audit_chain`) and local-first replica sync.

{{widget:lesson-cards act="6"}}

### Act VII: Agentic Hardware Review & Model Context Protocol (`rmcp`) (`0026`–`0029`) {#act-7}

Act VII turns our PCB Design Reviewer into an AI-native hardware verification platform. We design token-efficient, `schemars`-typed spatial neighborhood, net topology, and custom DRC query engines (`query_spatial_neighborhood`, `query_net_topology`, `run_custom_drc`), build a headless Model Context Protocol (`rmcp`) server exposing `kicad://` resources, `#[tool]` endpoints, and structured prompts tested over in-memory `tokio::io::duplex` streams, bridge the MCP server to the live `egui` desktop viewport over a constant-time bearer-authenticated control channel (`verify_bearer_token`) with cropped PNG region snapshots (`capture_cropped_region_png`), and embed an interactive **MCP Inspector Workbench** inside `egui` to audit JSON-RPC 2.0 frames and execute multi-step **Agent Review Playbooks**.

{{widget:lesson-cards act="7"}}

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
# Launch Lesson 0029's interactive 3-crate PCB Reviewer & MCP Inspector Workbench
cargo run --manifest-path ./apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks/Cargo.toml

# Run workspace unit + proptest + tokio::test suites and fast L0/L1 mutation checks
cargo test --manifest-path ./apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks/Cargo.toml -p kicad-sexpr -p pcb-doc --in-place
```

:::teacher-prompt
**Ready to dive in?** Start with [**Lesson 0000: Bootstrapping Your Rust & `eframe` Project**](./lessons/0000-project-setup-and-eframe-boot.html) or jump straight to [**Lesson 0029: Embedded `egui` MCP Inspector Workbench, Agent Playbooks & Act VII Scorecard**](./lessons/0029-egui-mcp-inspector-workbench-and-agent-playbooks.html).
:::

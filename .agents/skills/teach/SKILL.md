---
name: teach
description: >-
  Interactive, stateful teaching workflow that builds self-contained HTML lessons
  (compiled from Markdown via build.py), reference sheets, learning records, and
  runnable Rust/egui companion crates in ./apps/ verified with cargo test and
  cargo-mutants. Use when the user asks to learn a topic, build or extend the
  Rust and egui Schematic/PCB Design Reviewer course, create lessons or reference
  sheets, or invokes /teach.
---

# Interactive Teaching Workflow (`learn-egui`)

The user has asked you to teach them something or build out the course materials. This is a stateful request — they intend to learn the topic over multiple sessions.

## Teaching Workspace (Repository Root)

Treat the repository root (`.`) as the teaching workspace. All course paths below resolve from the repository root, while `references/*-FORMAT.md` links resolve from this skill's directory (`.agents/skills/teach/`):

- `MISSION.md`: A document capturing the _reason_ the user is interested in the topic. This grounds all teaching. Use the format in [MISSION-FORMAT.md](./references/MISSION-FORMAT.md).
- `reference/*.md`: A directory of Markdown reference materials (compiled to `reference/*.html` by `build.py`, with `GLOSSARY.md` compiling to `glossary.html`). These are the compressed learnings from the lessons — Immediate-Mode GUI mental models, Rust ownership/borrowing & `egui` widget signatures, `cargo-mutants` & `egui_kittest` verification cheat sheets, and glossaries. They are the raw units of learning, designed for quick reference and clean printing.
- `RESOURCES.md`: A curated list of trusted resources and repository skills used to ground your teaching in verified knowledge. Use the format in [RESOURCES-FORMAT.md](./references/RESOURCES-FORMAT.md).
- `learning-records/*.md`: A directory of learning records capturing what the user has learned. These are loosely equivalent to architectural decision records (ADRs) — they capture non-obvious lessons, prior knowledge, and key insights that drive future sessions and calculate the zone of proximal development. Titled `0001-<dash-case-name>.md`, incrementing sequentially. Use the format in [LEARNING-RECORD-FORMAT.md](./references/LEARNING-RECORD-FORMAT.md).
- `lessons/*.md`: A directory of Markdown lesson sources (compiled to `lessons/*.html` by `build.py`). A **lesson** is a single, self-contained tutorial unit that teaches one tightly-scoped `egui` capability paired just-in-time with one Rust concept and one `cargo test` + `cargo mutants` verification gate tied to the mission. This is the primary unit of teaching in this workspace.
- `index.md`: The Markdown course landing page (compiled to `index.html` by `build.py`) for GitHub Pages hosting, auto-rendering lesson cards via `{{widget:lesson-cards}}` and reference sheet links via `{{widget:reference-cards}}`.
- `build.py`: The Markdown-to-HTML static site builder (`markdown-it-py` + `pyyaml`). Only `.md` source files are tracked in Git (`index.html`, `lessons/*.html`, and `reference/*.html` are gitignored and generated locally via `.venv/bin/python build.py --check` or in CI via `.github/workflows/pages.yml`).
- `apps/<NNNN-lesson-slug>/`: A series of runnable Rust / Cargo crate directories where each directory represents the exact state of the Schematic & PCB Design Reviewer application after completing the corresponding lesson (`lessons/<NNNN-lesson-slug>.md`).
- `assets/*`: Reusable **components** shared across lessons (`style.css`, `course.js`, quiz/exercise widgets, interactive 2D CAD & immediate-mode simulators). See [Assets](#assets).
- `README.md`: Repository overview linking to the live GitHub Pages site, published lessons, reference sheets, `./LICENSE.txt`, and the disclaimer `This is not an official Google product.`
- `NOTES.md`: A scratchpad for recording user preferences, the 6-act curriculum roadmap (`0000`–`0025`), and working notes.

## Philosophy

To learn at a deep level, the user needs three things:

- **Knowledge**, captured from high-quality, high-trust resources (including the repository's `.agents/skills/egui-app-architecture/`, `.agents/skills/egui-cow-document-model/`, `.agents/skills/egui-command-engine/`, `.agents/skills/egui-app-runner/`, and `.agents/skills/egui-testing-infrastructure/` skills, plus official `egui`, `eframe`, _The Rust Programming Language_, `cargo-mutants`, `egui_kittest`, `proptest`, `nom`, and `libsql` documentation).
- **Skills**, acquired through highly-relevant interactive lessons devised by you, based on the knowledge.
- **Wisdom**, which comes from interacting with other learners and practitioners.

Before `RESOURCES.md` is well-populated, your focus should be to catalog high-quality resources (both external docs and the verified `.agents/skills/*` reference guides in this repo) to help the user acquire knowledge. Never trust unverified parametric knowledge when a compiler check (`cargo test`), mutation run (`cargo mutants`), or official crate documentation can verify it.

### Fluency vs Storage Strength

Always distinguish between two types of learning:

- **Fluency strength**: in-the-moment retrieval of knowledge
- **Storage strength**: long-term retention of knowledge

Fluency can give an illusory sense of mastery, whereas storage strength is the real goal. Design lessons that build long-term retention through desirable difficulty:

- Using retrieval practice (recall from memory via interactive fill-in-the-blank Rust/`egui` code exercises and quizzes)
- Spacing (distributing practice and revisiting earlier concepts across lessons)
- Interleaving (mixing related skills, such as Rust ownership/borrowing, 2D coordinate transforms, widget layout, and mutation-proof test assertions)

## Lessons & Markdown-to-HTML Pipeline (`build.py`)

A lesson is the primary artifact you produce: the unit in which knowledge and skills reach the user. Each lesson is authored as a clean Markdown file saved to `lessons/` and titled `0001-<dash-case-name>.md` (starting at `0000-<dash-case-name>.md` for project bootstrap), then compiled to HTML (`lessons/<NNNN-slug>.html`) by running `.venv/bin/python build.py --check`.

When teaching a programming language, framework, or tool where Lesson `0001` needs a configured project manifest and entry point, start with a `0000-<dash-case-name>.md` bootstrap lesson covering Cargo package creation (`Cargo.toml` with `eframe = "0.33"` and `[profile.dev.package."*"] opt-level = 2`), `eframe::run_native`, `#[cfg(test)]`, and `.cargo/mutants.toml` setup so subsequent lessons have a concrete, runnable foundation.

### Markdown Directives & Frontmatter (`build.py`)

All course pages (`index.md`, `lessons/*.md`, `reference/*.md`) use YAML frontmatter (`title`, `page_title`, `nav_id`, `badge`, `subtitle`, plus `card` metadata for lessons and reference sheets, or `nav_label` for reference sheets) and clean Markdown directives:

- **Tufte Sidenotes**: `{{sidenote:Inline Markdown note text...}}` (renders a numbered superscript `<span class="sidenote-number"></span><span class="sidenote">...</span>` in the right margin).
- **Callouts & Teacher Prompts**: `:::callout` (for recommended primary sources and notes) and `:::teacher-prompt` (for next-step prompts and reminders to ask questions) ... `:::`
- **Code Blocks & Mermaid Diagrams**: ```` ```rust ````, ```` ```toml ````, ```` ```bash caption="Optional caption" ````, and ```` ```mermaid caption="Figure N.1: ..." ```` (wrapped in `<figure class="mermaid-figure">`).
- **Tables**: Standard GFM pipe tables (`| ... |`), optionally preceded by `Table: Caption text`, automatically styled with `<table class="ref-table">`.
- **Interactive Fill-in-the-Blank Exercises**: `:::exercise id="ex-NNNN" title="Interactive Lab 2: ..." badge="Skill Practice • Tight Feedback Loop"` containing explanatory prose, a fenced code block with `{{blank:answer|error_hint|placeholder|width}}` tokens (use `||` inside `answer` for multi-answer alternatives like `size.x / 2.0||size.x * 0.5`), and a `:::prompts` YAML block with `hint` and `success` messages.
- **Cargo Verification & Mutation Coverage Gates**: `:::verify id="verify-NNNN" title="Interactive Lab 3: ..." badge="cargo test + cargo mutants" tier="Tier 2: ..." caught="N Caught" missed="0 Missed"` containing prose, test code blocks, and `- [ ]` step checklist items.
- **Multiple-Choice Quizzes**: `:::quiz id="quiz-NNNN" title="Retrieval Practice: ..." badge="3 Questions • Equal-Length Options"` with a YAML list of questions (`question`, `wrong_hint`, and 4 `options` with `letter`, `text`, `correct`, `explain`). `build.py --check` enforces that all 4 options within each question have identical word counts!
- **Interactive Widgets**: `{{widget:lesson-cards}}`, `{{widget:reference-cards}}`, `{{widget:pcb-layer-sim}}`, `{{widget:painter-canvas-sim}}`, `{{widget:viewport-transform-sim}}`, `{{widget:layer-zorder-sim}}`, `{{widget:hittest-sim}}`, `{{widget:sch-text-sim}}`, `{{widget:dual-scene-sim}}`, `{{widget:bom-scroll-sim}}`, `{{widget:hotkey-ruler-sim}}`.

### Runnable Per-Lesson Rust / `egui` Crates (`./apps/<NNNN-lesson-slug>/`)

Back every lesson with a real, runnable Cargo crate under `./apps/<NNNN-lesson-slug>/` (matching the lesson filename stem):

1. **Tooling Stack (`cargo` + `cargo-mutants` + `egui_kittest` + `uv` for `build.py`):**
   - Use `.venv/bin/python build.py --check` (created via `uv venv --clear --python 3.14 .venv` and `uv pip install markdown-it-py pyyaml`) to compile and validate all Markdown lessons and reference sheets.
   - Start at `./apps/0000-project-setup-and-eframe-boot/` and progressively grow the Schematic & PCB Design Reviewer across lessons (`0000`–`0025`).
2. **Sandboxed Offline Cargo Setup (`CARGO_HOME` & `TMPDIR` in Brain Scratch):**
   - On macOS, `/tmp` is a symlink to `/private/tmp` and `~/.cargo` lies outside the workspace, so `std::fs::canonicalize` inside `cargo` and `cargo-mutants` fails with `Operation not permitted` in Standard Sandbox Mode (`BypassSandbox: false`).
   - Seed the Cargo registry once into `<appDataDir>/brain/<conversation-id>/scratch/cargo-home/registry` (`cp -cR ~/.cargo/registry <scratch>/cargo-home/registry`) and pass `CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp` on every `cargo test` and `cargo mutants` command so all builds and mutation runs execute 100% inside Standard Sandbox Mode (`BypassSandbox: false`).
3. **Copy-Then-Apply Progression (with APFS CoW, Matching `CARGO_HOME` & `touch`):**
   - Wait until the seed lesson's `cargo test` finishes completely (so no mid-link `.o`/`.rmeta` temp files are in flight) and ensure it was built with the exact same `CARGO_HOME` path (Cargo hashes `CARGO_HOME` into `target/debug/.fingerprint/`).
   - For each subsequent lesson $N$, copy the resolved `Cargo.lock` (re-copying whenever `[dependencies]` or `[dev-dependencies]` like `egui_kittest` change) and clone `target/` using APFS copy-on-write (`cp -cR ./apps/<N-1>/target ./apps/<N>/target`), write the updated `src/main.rs` and `.cargo/mutants.toml`, and **always run `touch ./apps/<N>/src/main.rs`** before running `cargo test` so Cargo's timestamp check compiles the new lesson's code rather than executing the previous lesson's cached test binary.
4. **Execute, Mutate & Reconcile Lessons with Reality:**
   - Run the verification commands taught in the lesson against `./apps/<NNNN-slug>/`:
     ```bash
     CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp cargo test --manifest-path ./apps/<NNNN-slug>/Cargo.toml --offline
     CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp cargo mutants --in-place --timeout 120 -d ./apps/<NNNN-slug> -o <scratch>/mutants-<NNNN> -C --offline
     ```
   - Run `cargo mutants --in-place` **sequentially** (one crate at a time after all `cargo test` builds finish) with `--timeout 120` so CPU/linker contention cannot trigger false 20-second timeouts.
   - Ensure zero compiler warnings (e.g., use `2.0_f32` suffixes on generic `Stroke::new(2.0_f32, ...)` calls for Rust 1.97+, and copy `Copy` temporaries like `let mut scene_rect = self.view.pcb_scene_rect;` around `egui::Scene::show` closures to avoid `E0500` borrow conflicts) and **0 missed mutants** (using non-zero asymmetric test coordinates for geometry math and testing each `||` filter branch in isolation).
   - If anything does not line up between the runnable crate and the lesson Markdown/HTML, reference sheets (`reference/*.md`), or interactive simulators (`assets/course.js`), update them immediately and re-run `.venv/bin/python build.py --check` so the course and runnable code stay 100% in sync.

### Visual Design, Prism.js & Mermaid.js

A lesson should be **beautiful**, with clean, readable typography and layout (think Tufte), since the user will return to these later to review:

- **Dark mode by default**: Use dark mode across all lessons and `reference/*.html` documents (`<meta name="color-scheme" content="dark">` in `<head>` and `:root { color-scheme: dark; }` in `assets/style.css`), paired with a `@media print` light-mode override so lessons and reference documents print cleanly on paper. Pair `scrollbar-color` on `:root` with `scrollbar-width: thin` (required on macOS so custom scrollbars render) and wrap legacy `::-webkit-scrollbar` fallbacks in `@supports not (scrollbar-color: auto)`. Use high-contrast `accent-color` tokens for interactive form controls.
- **Modern web typography, transitions, and accessibility**: Enable cross-document view transitions (`@view-transition { navigation: auto; }` inside `@media (prefers-reduced-motion: no-preference)`), style focus rings with `:focus-visible` and `outline-offset: 2px` via `:where(...)` (never bare `:focus`), apply `text-wrap: balance` to headings/table headers (`h1, h2, h3, th`) and `text-wrap: pretty` to body copy (`p, li, blockquote`), use semantic HTML landmarks (`<header>`, `<nav>`, `<main>`, `<article>`, `<footer>`), and ensure scrollable `<pre>` code blocks receive `tabindex="0"` for keyboard scrolling (handled automatically in `assets/course.js`).
- **Prism.js for syntax highlighting**: Format all code blocks with Prism.js (`<pre><code class="language-rust">`, `language-toml`, `language-bash`, etc.), and load the Prism `keep-markup` plugin when embedding interactive `<input>` blanks inside code blocks. Never hand-roll syntax `<span>` tags.
- **Mermaid.js for diagrams**: Use Mermaid.js (`<pre class="mermaid">` configured with `theme: "dark"`) for architecture, frame lifecycle, state-machine, and data-flow diagrams instead of ASCII-art diagrams.

### Course Index, Navigation & GitHub Pages Hosting

Keep the course navigable both locally and on GitHub Pages:

1. **Synchronize Navigation & Index via `build.py --check` on Every New Lesson:** Whenever you add a lesson (`lessons/<NNNN-slug>.md`) or reference sheet (`reference/*.md`):
   - Populate its YAML frontmatter (`title`, `page_title`, `nav_id`, `badge`, `subtitle`, `card`) and add the live link to `README.md`.
   - Run `.venv/bin/python build.py --check` so `build.py` automatically regenerates `<nav class="course-nav">` across every page, updates the `.lesson-card` grid in `index.html`, compiles all `.md` files to `.html`, and validates equal-word-count quiz rules and internal `.md` links (rewritten to `.html`).
   - Preserve the license link (`[Apache License, Version 2.0](./LICENSE.txt)`) and the `This is not an official Google product.` disclaimer in `README.md`.
2. **GitHub Pages Deployment (`.github/workflows/pages.yml` & `.nojekyll`):**
   - `.github/workflows/pages.yml` runs `python build.py --out-dir _site --check` to compile the Markdown sources directly into `_site/` (`index.html`, `.nojekyll`, `assets/`, `lessons/*.html`, `reference/*.html`) so companion Rust crates (`apps/`) and internal markdown files (`MISSION.md`, `RESOURCES.md`, `NOTES.md`, `learning-records/`) are excluded from the published web artifact.
   - Pass `env: NODE_OPTIONS: --no-deprecation` on the `actions/deploy-pages@v4` step to suppress the benign Node.js `[DEP0040] DeprecationWarning: The punycode module is deprecated` warning.

Each lesson should be short, focused, and completable quickly within working-memory limits, giving the user a single tangible win directly tied to the mission and within their zone of proximal development.

If possible, open the lesson file for the user or provide a direct clickable link. Every lesson should:
- Link via Markdown/HTML anchors to adjacent lessons and `reference/*.md` (compiled to `reference/*.html`) documents.
- Recommend a primary source (such as the relevant `egui`/`eframe` docs page, Rust Book chapter, `mutants.rs` section, or workspace skill reference).
- Include a reminder to ask follow-up questions to the agent.

## Assets

Lessons are built from reusable **components** stored in `assets/`: stylesheets, quiz widgets, interactive 2D CAD and immediate-mode simulators, code completion exercises, and anything else a second lesson could reuse.

Reuse is the default, not the exception. Before authoring a lesson, read `assets/` and build from the components already there. When a lesson needs something new and reusable, write it as a component in `assets/` and wire it into `build.py`; never inline code a future lesson would duplicate.

A shared dark-mode stylesheet (`assets/style.css`) and a shared script (`assets/course.js`) that centrally loads and configures **Prism.js** (including `keep-markup`, `rust`, `toml`, `bash`) and **Mermaid.js** (`theme: "dark"`) alongside reusable quiz, fill-in-the-blank code exercise, and simulator widgets are the foundational components of the workspace: every lesson and reference sheet links them so the tutorial looks like one cohesive site.

## The Mission

Every lesson should be tied into the mission — the reason the user is learning the topic, captured in `MISSION.md`.

If the user is unclear about aspects of the mission, or `MISSION.md` is not yet populated, ask clarifying questions **one at a time** to confirm their concrete goal, starting point, and what success looks like.

Missions may evolve as the user develops more skills and knowledge. Update `MISSION.md` and add a learning record when the mission shifts, confirming with the user first.

## Zone Of Proximal Development

Each lesson should challenge the user "just enough". Determine the next lesson by:

- Reading `learning-records/`
- Checking what has already been built in `lessons/` and `apps/`
- Selecting the next step toward the mission in `MISSION.md` and `NOTES.md`

## Knowledge & Skills

- **Knowledge**: Design each lesson around a concrete skill. Present only the knowledge required to acquire that skill, backed by citations to `RESOURCES.md` and `reference/*.html`. Keep cognitive load low during knowledge acquisition.
- **Skills**: Build durability through effortful retrieval and immediate feedback loops:
  - Interactive fill-in-the-blank Rust/`egui` code blocks (using `:::exercise` and Prism `keep-markup`)
  - Multiple-choice retrieval quizzes (`:::quiz`) with immediate feedback. **Quiz rule**: Every option within a quiz question must have the exact same word count (and similar character length), with the correct answer position varied across questions so formatting or length never leaks the answer.
  - Hands-on terminal verification and mutation testing steps (`:::verify`) using `cargo test` and `cargo mutants`.

## Acquiring Wisdom

Wisdom comes from testing skills in real-world engineering communities (e.g., `emilk/egui` GitHub Discussions, the official `egui` Discord server, and The Rust Programming Language Users Forum `URLO`). Include high-signal communities in `RESOURCES.md` while respecting any user preferences regarding community participation.

## Reference Documents

While creating lessons, maintain `reference/*.md` cheat sheets (compiled to `reference/*.html` by `build.py`) — the compressed essence of the lessons designed for quick reference and printing (e.g., Immediate-Mode GUI Cheat Sheet, Testing & Mutation Coverage Cheat Sheet, and Glossary following [GLOSSARY-FORMAT.md](./references/GLOSSARY-FORMAT.md)). Once a glossary is created, adhere to its terminology across every lesson.

## `NOTES.md`

Record any teaching preferences or workflow notes expressed by the user in `NOTES.md` and consult it before designing each lesson.

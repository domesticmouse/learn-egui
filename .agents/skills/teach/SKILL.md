---
name: teach
description: Teach the user a new skill or concept, within this workspace.
disable-model-invocation: true
argument-hint: "What would you like to learn about?"
---

The user has asked you to teach them something. This is a stateful request - they intend to learn the topic over multiple sessions.

## Teaching Workspace

Treat the current directory as a teaching workspace. Workspace paths (`./lessons/` and the rest) resolve from the directory `/teach` was run in; only the `*-FORMAT.md` links resolve from this skill's folder. The state of their learning is captured in this directory in several files:

- `MISSION.md`: A document capturing the _reason_ the user is interested in the topic. This should be used to ground all teaching. Use the format in [MISSION-FORMAT.md](./MISSION-FORMAT.md).
- `./reference/*.html`: A directory of reference materials. These are the compressed learnings from the lessons - cheat sheets, reference algorithms, syntax, yoga poses, glossaries. They are the raw units of learning. They should be beautiful documents which print out well, and are designed for quick reference.
- `RESOURCES.md`: A list of resources which can be explored to ground your teaching in contextual knowledge, or to acquire knowledge and wisdom. Use the format in [RESOURCES-FORMAT.md](./RESOURCES-FORMAT.md).
- `./learning-records/*.md`: A directory of learning records, which capture what the user has learned. These are loosely equivalent to architectural decision records in software development - they capture non-obvious lessons and key insights that may need to be revised later, or drive future sessions. These should be used to calculate the zone of proximal development. They are titled `0001-<dash-case-name>.md`, where the number increments each time. Use the format in [LEARNING-RECORD-FORMAT.md](./LEARNING-RECORD-FORMAT.md).
- `./lessons/*.html`: A directory of lessons. A **lesson** is a single, self-contained HTML output that teaches one tightly-scoped thing tied to the mission. This is the primary unit of teaching in this workspace.
- `./index.html`: The root course landing page for GitHub Pages hosting, linking to all published lessons (via `.lesson-grid` / `.lesson-card` cards with mutation scoreboards and runnable app paths) and `./reference/*.html` sheets.
- `./apps/<NNNN-lesson-slug>/`: For software/coding courses, a series of runnable application directories where each directory represents the exact state of the project after completing the corresponding lesson (`./lessons/<NNNN-lesson-slug>.html`).
- `./assets/*`: Reusable **components** shared across lessons. See [Assets](#assets).
- `README.md`: Repository overview linking to the live GitHub Pages site, published lessons, reference sheets, `./LICENSE.txt`, and the disclaimer `This is not an official Google product.`
- `NOTES.md`: A scratchpad for you to jot down user preferences, or working notes.

## Philosophy

To learn at a deep level, the user needs three things:

- **Knowledge**, captured from high-quality, high-trust resources
- **Skills**, acquired through highly-relevant interactive lessons devised by you, based on the knowledge
- **Wisdom**, which comes from interacting with other learners and practitioners

Before the `RESOURCES.md` is well-populated, your focus should be to find high-quality resources which will help the user acquire knowledge. Never trust your parametric knowledge.

Some topics may require more skills than knowledge. Learning more about theoretical physics might be more knowledge-based. For yoga, more skills-based.

### Fluency vs Storage Strength

You should be careful to split between two types of learning:

- **Fluency strength**: in-the-moment retrieval of knowledge
- **Storage strength**: long-term retention of knowledge

Fluency can give the user an illusory sense of mastery, but storage strength is the real goal. Try to design lessons which build long-term retention by desirable difficulty:

- Using retrieval practice (recall from memory)
- Spacing (distributing practice over time)
- Interleaving (mixing up different but related topics in practice - for skills practice only)

## Lessons

A lesson is the main thing you produce: the unit in which knowledge and skills reach the user. Each lesson is one self-contained HTML file, saved to `./lessons/` and titled `0001-<dash-case-name>.html` where the number increments each time.

When teaching a programming language, framework, or tool where the code in `0001` needs a real project, manifest, and entry-point file to inhabit (especially when the learner is new to the ecosystem), start with a `0000-<dash-case-name>.html` bootstrap lesson covering initial project setup and scaffolding so Lesson `0001`'s code has a concrete, runnable home.

### Runnable Per-Lesson App Directories (`./apps/<NNNN-lesson-slug>/`)

For coding courses, back every lesson with a real, runnable application directory under `./apps/<NNNN-lesson-slug>/` (matching the lesson filename stem):

1. **Start at Lesson `0000`:** Follow the exact instructions in `./lessons/0000-<slug>.html` to create `./apps/0000-<slug>/`.
2. **Sandboxed Offline Cargo Setup (`CARGO_HOME` & `TMPDIR` in Brain Scratch):** On macOS, `/tmp` is a symlink to `/private/tmp` and `~/.cargo` lies outside the workspace, so `std::fs::canonicalize` inside `cargo` and `cargo-mutants` fails with `Operation not permitted` in Standard Sandbox Mode (`BypassSandbox: false`). Seed the Cargo registry once into `<appDataDir>/brain/<conversation-id>/scratch/cargo-home/registry` (`cp -cR ~/.cargo/registry <scratch>/cargo-home/registry`) and pass `CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp` on every `cargo test` and `cargo mutants` command so all builds and mutation runs execute 100% inside Standard Sandbox Mode (`BypassSandbox: false`).
3. **Copy-Then-Apply Progression (with APFS CoW, Matching `CARGO_HOME` & `touch`):** Wait until the seed lesson's `cargo test` finishes completely (so no mid-link `.o`/`.rmeta` temp files are in flight) and ensure it was built with the exact same `CARGO_HOME` path (Cargo hashes `CARGO_HOME` into `target/debug/.fingerprint/`). For each subsequent lesson $N$, copy the resolved `Cargo.lock` (re-copying whenever `[dependencies]` or `[dev-dependencies]` like `egui_kittest` change) and clone `target/` using APFS copy-on-write (`cp -cR ./apps/<N-1>/target ./apps/<N>/target`), write the updated `src/main.rs` and `.cargo/mutants.toml`, and **always run `touch ./apps/<N>/src/main.rs`** before running `cargo test` so Cargo's timestamp check compiles the new lesson's code rather than executing the previous lesson's cached test binary.
4. **Execute & Reconcile Lessons with Reality:** Run the exact verification commands taught in the lesson (`CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp cargo test --manifest-path ./apps/<NNNN-slug>/Cargo.toml --offline` and `CARGO_HOME=<scratch>/cargo-home TMPDIR=<scratch>/tmp cargo mutants --in-place --timeout 120 -d ./apps/<NNNN-slug> -o <scratch>/mutants-<NNNN> -C --offline`) against `./apps/<NNNN-slug>/`. Run `cargo mutants --in-place` **sequentially** (one crate at a time after all `cargo test` builds finish) with `--timeout 120` so CPU/linker contention cannot trigger false 20-second timeouts. Ensure zero compiler warnings (e.g., use `2.0_f32` suffixes on generic `Stroke::new(2.0_f32, ...)` calls for Rust 1.97+, and copy `let mut scene_rect = self.view.pcb_scene_rect;` around `egui::Scene::show` closures to avoid `E0500` borrow conflicts) and **0 missed mutants**. If anything does not line up—such as missing prerequisite tool installation instructions, config filter syntax mismatches, omitted glue code, or differences between interactive simulators (`./assets/course.js`) and code listings—fix the lesson HTML, reference sheets (`./reference/*.html`), and shared assets immediately so the lesson and runnable app are 100% in sync.

A lesson should be **beautiful**, with clean, readable typography and layout, since the user will return to these later to review. Think Tufte:

- **Dark mode by default**: Use dark mode across all lessons and `./reference/*.html` documents (`<meta name="color-scheme" content="dark">` in `<head>` and `:root { color-scheme: dark; }` in the shared stylesheet), paired with a `@media print` light-mode override so lessons and reference documents still print cleanly on paper. Pair `scrollbar-color` on `:root` with `scrollbar-width: thin` (required on macOS so custom scrollbars render) and wrap legacy `::-webkit-scrollbar` fallbacks in `@supports not (scrollbar-color: auto)`. Use high-contrast `accent-color` tokens for interactive form controls with `@supports not (accent-color: ...)` fallbacks for range inputs.
- **Modern web typography, transitions, and accessibility**: Enable cross-document view transitions (`@view-transition { navigation: auto; }` inside `@media (prefers-reduced-motion: no-preference)`), style focus rings with `:focus-visible` and `outline-offset: 2px` via `:where(...)` (never bare `:focus`), apply `text-wrap: balance` to headings/table headers (`h1, h2, h3, th`) and `text-wrap: pretty` to body copy (`p, li, blockquote`), use semantic HTML landmarks (`<header>`, `<nav>`, `<main>`, `<article>`, `<footer>`), and ensure scrollable `<pre>` code blocks receive `tabindex="0"` for keyboard scrolling (handled automatically in `./assets/course.js`).
- **Prism.js for syntax highlighting**: Format all code blocks with Prism.js (`<pre><code class="language-...">`), and load the Prism `keep-markup` plugin when embedding interactive `<input>` blanks inside code blocks. Never hand-roll syntax `<span>` tags.
- **Mermaid.js for diagrams**: Use Mermaid.js (`<pre class="mermaid">` configured with `theme: "dark"`) for architecture, lifecycle, and flow diagrams instead of ASCII-art diagrams.

### Course Index, Navigation & GitHub Pages Hosting

Keep the course navigable both locally and on GitHub Pages:

1. **Synchronize Navigation & Index on Every New Lesson:** Whenever you add a lesson (`./lessons/<NNNN-slug>.html`) or reference sheet (`./reference/*.html`):
   - Include `<a href="../index.html">Index</a>` as the first link in `<nav class="course-nav">` across all lessons and reference pages, and add the new lesson link (`NNNN`) to `.course-nav` in `./index.html`, `./lessons/*.html`, and `./reference/*.html`.
   - Add a `.lesson-card` entry to `./index.html` (showing the lesson number, `cargo mutants` caught score badge, `egui` skill, Rust concept, and `./apps/<NNNN-slug>` path) and add the live link to `README.md`.
   - Preserve the license link (`[Apache License, Version 2.0](./LICENSE.txt)`) and the `This is not an official Google product.` disclaimer in `README.md`.
2. **GitHub Pages Deployment (`.github/workflows/pages.yml` & `.nojekyll`):**
   - Stage only the static course web files (`index.html`, `.nojekyll`, `assets/`, `lessons/`, `reference/`) into `_site/` in `.github/workflows/pages.yml` (`actions/configure-pages@v5` with `enablement: true`, `actions/upload-pages-artifact@v3`, and `actions/deploy-pages@v4`) so companion Rust crates (`apps/`) and internal markdown files are excluded from the web artifact.
   - Pass `env: NODE_OPTIONS: --no-deprecation` on the `actions/deploy-pages@v4` step to suppress the benign Node.js `[DEP0040] DeprecationWarning: The punycode module is deprecated` log warning emitted by the action's bundled `@actions/artifact` dependency tree on Node 20+.
   - When enabling GitHub Pages on a new repository, configure workflow-based Pages via `gh api repos/<owner>/<repo>/pages -X POST -f build_type=workflow`. If the initial `push` workflow run executes before the Pages API endpoint is enabled, trigger `gh workflow run pages.yml` after enabling Pages.

The lesson should be short, and completable very quickly. Learners' working memory is very small, and we need to stay within it. But each lesson should give the user a single tangible win that they can build on. It should be directly tied to the mission, and should be in the user's zone of proximal development.

If possible, open the lesson file for the user by running a CLI command.

Each lesson should link via HTML anchors to other lessons and reference documents.

Each lesson should recommend a primary source for the user to read or watch. This should be the most high-quality, high-trust resource you found on the topic.

Each lesson should contain a reminder to ask followup questions to the agent. The agent is their teacher, and can assist with anything that's unclear.

## Assets

Lessons are built from reusable **components**, stored in `./assets/`: stylesheets, quiz widgets, simulators, diagram helpers, and anything else a second lesson could reuse.

Reuse is the default, not the exception. Before authoring a lesson, read `./assets/` and build from the components already there. When a lesson needs something new and reusable, write it as a component in `./assets/` and link to it; never inline code a future lesson would duplicate.

A shared dark-mode stylesheet (`./assets/style.css`) and a shared script (`./assets/course.js`) that centrally loads and configures **Prism.js** (including `keep-markup`) and **Mermaid.js** (`theme: "dark"`) alongside reusable quiz/exercise widgets are the first components every workspace earns: every lesson and reference sheet links them, so the workspace looks like one consistent course rather than a pile of one-offs. As the workspace grows, so should the component library.

## The Mission

Every lesson should be tied into the mission - the reason that the user is interested in learning about the topic.

If the user is unclear about the mission, or the `MISSION.md` is not populated, your first job should be to question the user on why they want to learn this. Ask clarifying questions **one at a time** to uncover their concrete real-world goal, starting point, and what success looks like.

Failing to understand the mission will mean knowledge acquisition is not grounded in real-world goals. Lessons will feel too abstract. You will have no way of judging what the user should do next.

Missions may change as the user develops more skills and knowledge. This is normal - make sure to update the `MISSION.md` and add a learning record to capture the change. Confirm with the user before changing the mission.

## Zone Of Proximal Development

Each lesson, the user should always feel as if they are being challenged 'just enough'.

The user may specify an exact thing they want to learn. If they don't, figure out their zone of proximal development by:

- Reading their `learning-records`
- Figuring out the right thing to teach them based on their mission
- Teach the most relevant thing that fits in their zone of proximal development

## Knowledge

Lessons should be designed around a skill the user is going to learn. The knowledge in the lesson should be only what's required to acquire that skill. You teach the knowledge first, then get the user to practice the skills via an interactive feedback loop.

Knowledge should first be gathered from trusted resources. Use `RESOURCES.md` to keep track of them. Lessons should be littered with citations - links to external resources to back up any claim made. This increases the trustworthiness of the lesson.

For acquiring knowledge, difficulty is the enemy. It eats working memory you need for understanding.

## Skills

If knowledge is all about acquisition, skills are about durability and flexibility. Make the knowledge stick.

For skill acquisition, difficulty is the tool. Effortful retrieval is what builds storage strength. Skills should be taught through interactive lessons. There are several tools at your disposal:

- Interactive lessons, using quizzes and light in-browser tasks
- Lessons which guide the user through a list of real-world steps to take (for instance, yoga poses)

Each of these should be based on a **feedback loop**, where the user receives feedback on their performance. This feedback loop should be as tight as possible, giving feedback immediately - and ideally automatically.

For quizzes, each answer should be exactly the same number of words (and characters, if possible). Vary which position holds the correct answer across questions. Don't give the user any clues about the answer through formatting or order.

## Acquiring Wisdom

Wisdom comes from true real-world interaction - testing your skills outside the learning environment.

When the user asks a question that appears to require wisdom, your default posture should be to attempt to answer - but to ultimately delegate to a **community**.

A community is a place (online or offline) where the user can test their skills in the real world. This might be a forum, a subreddit, a real-world class (budget permitting) or a local interest group.

You should attempt to find high-reputation communities the user can join. If the user expresses a preference that they don't want to join a community, respect it.

## Reference Documents

While creating lessons, you should also create reference documents. Lessons can reference these documents - they are useful for tracking raw units of knowledge useful across lessons.

Lessons will rarely be revisited later - reference documents will be. They should be the compressed essence of the lesson, in a format designed for quick reference.

Some learning topics lend themselves to reference:

- Syntax and code snippets for programming
- Algorithms and flowcharts for processes
- Yoga poses and sequences for yoga
- Exercises and routines for fitness
- Glossaries for any topic with its own nomenclature

Glossaries, in particular, are an essential reference. Once one is created, it should be adhered to in every lesson.

## `NOTES.md`

The user will sometimes express preferences of how they want to be taught, or things you should keep in mind. This is the place to record those preferences, so you can refer back to them when designing lessons or working with the user.

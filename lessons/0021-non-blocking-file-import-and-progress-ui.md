---
nav_id: "0021"
page_title: "Lesson 0021: Non-Blocking Background File Import (mpsc::channel) & ProgressBar UI — Learn Rust & egui"
act_badge: "Act V • Command Engine, Performance & Polish"
title: "0021. Non-Blocking Background File Import (`std::sync::mpsc`) & Live `ProgressBar` UI"
subtitle: "Offloading board and session parsing onto a background worker thread (`std::thread::spawn`), streaming staged progress over an `mpsc::channel`, polling `try_recv()` without blocking the 60-FPS `egui` loop, and driving an `egui::ProgressBar` with `ctx.request_repaint()`."
card:
  num: "Lesson 0021"
  mutants: "269 / 269 Caught"
  title: "Non-Blocking File Import (`mpsc::channel`) & `ProgressBar`"
  summary: "Parse session files on a background worker (`std::thread::spawn` + `mpsc::channel`), poll `try_recv()` on each frame, render a live `egui::ProgressBar`, request continuous repaints while running, and support instant cancellation."
  egui_skill: "`egui::ProgressBar`, Non-Blocking `try_recv()` Polling & `ctx.request_repaint()`"
  rust_concept: "`std::thread::spawn`, `std::sync::mpsc::channel` & `Send + 'static` Channel Ownership"
  app_dir: "apps/0021-non-blocking-file-import-and-progress-ui"
---

## Why Synchronous File Parsing Freezes Immediate-Mode GUIs

In Lesson `0016`, `import_session_from_json` ran synchronously on the main UI thread inside `ui(&mut self, ui, frame)`. For a small 2 KB review envelope, that takes microseconds—but when an engineer opens a 50 MB industrial `.kicad_pcb` board file with 100,000 trace segments, synchronous parsing blocks `ui()` for hundreds of milliseconds, freezing panning, zooming, and OS window events.{{sidenote: **Never Block `eframe::App::ui`:** Because `egui` runs on a single UI thread, any operation taking more than ~4 ms should run on a background thread (`std::thread::spawn`) and communicate back via non-blocking message passing (`std::sync::mpsc::Receiver::try_recv`).}}

We keep the UI locked at 60 FPS with a three-part pattern:

1. **Spawn & Stream (`spawn_background_session_import`)**: Creates a `std::sync::mpsc::channel::<ImportJobMessage>()`, spawns a worker thread owning the raw JSON `String` (`Send + 'static`), emits staged `ImportJobMessage::Progress` updates (`"Validating envelope"`, `"Loading copper layers"`), and finishes with `Completed { board_id, board }` or `Failed(SessionIoError)`.
2. **Non-Blocking Poll (`poll_async_import`)**: At the start of each frame, drains available messages with `while let Ok(msg) = rx.try_recv()`—never blocking `ui()`!
3. **Continuous Repaint & Cancellation (`ctx.request_repaint()`)**: In reactive desktop mode, `egui` sleeps when the mouse is idle. Calling `ctx.request_repaint()` while `matches!(self.import_state, ImportJobState::Running { .. })` ensures `ui()` keeps polling the channel until the worker finishes, and dropping `self.import_rx = None` cancels an in-flight job cleanly.

```mermaid
sequenceDiagram
    participant UI as PcbReviewerApp (UI Thread)
    participant Chan as mpsc::channel
    participant Worker as std::thread::spawn (Worker)

    UI->>Chan: spawn_background_session_import(raw_json)
    Worker->>Chan: send(Progress { 1/2, "Validating envelope" })
    UI->>Chan: poll_async_import() -> rx.try_recv()
    Note over UI: Updates ProgressBar (50%)<br/>Calls ctx.request_repaint()
    Worker->>Chan: send(Completed { board_id, board })
    UI->>Chan: poll_async_import() -> rx.try_recv()
    Note over UI: Swaps self.board, sets ImportJobState::Succeeded
```

## 2. Anatomy of Non-Blocking File Import & Progress UI: Block by Block

Let's walk through the new code in `crates/pcb-doc/src/lib.rs` and `src/main.rs` across four logical blocks—covering the channel message protocol, the background worker thread, the non-blocking `try_recv()` frame poll with `egui::ProgressBar`, and the test suite—before viewing the complete assembled listing in context.

### Block 1: Progress Ratio Math, `ImportJobMessage` & `ImportJobState` (`crates/pcb-doc/src/lib.rs`)

```rust
pub fn progress_fraction(completed_steps: usize, total_steps: usize) -> f32 {
    if total_steps == 0 {
        return 0.0;
    }
    (completed_steps as f32 / total_steps as f32).clamp(0.0, 1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportJobMessage {
    Progress {
        completed_steps: usize,
        total_steps: usize,
        phase: String,
    },
    Completed {
        board_id: String,
        board: BoardModel,
    },
    Failed(SessionIoError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportJobState {
    Idle,
    Running { fraction: f32, phase: String },
    Succeeded { board_id: String },
    Failed(String),
}

impl ImportJobState {
    pub fn is_running(&self) -> bool {
        matches!(self, ImportJobState::Running { .. })
    }

    pub fn status_badge(&self) -> String {
        match self {
            ImportJobState::Idle => "Import: Idle".to_owned(),
            ImportJobState::Running { fraction, phase } => {
                format!("Importing: {phase} ({:.0}%)", fraction * 100.0)
            }
            ImportJobState::Succeeded { board_id } => format!("Import OK: {board_id}"),
            ImportJobState::Failed(err) => format!("Import Error: {err}"),
        }
    }
}
```

- **`progress_fraction(completed_steps, total_steps)`**: Guards against division-by-zero (`if total_steps == 0 { return 0.0; }`) so `f32::NAN` never reaches `egui::ProgressBar`, then clamps the ratio to `0.0..=1.0`.
- **`ImportJobMessage`**: The typed message protocol sent across `std::sync::mpsc::channel` from the worker thread to the UI thread (`Progress`, `Completed { board_id, board }`, or `Failed(SessionIoError)`). Because `BoardModel`'s collections are wrapped in `Arc<...>`, `ImportJobMessage` is `Send + 'static` and moves across thread boundaries in `O(1)` pointer copies.
- **`ImportJobState::is_running` & `status_badge`**: Tracks the UI-side state machine (`Idle`, `Running { fraction, phase }`, `Succeeded { board_id }`, `Failed(String)`) and formats a status badge for the session panel.

### Block 2: Background Worker Thread (`spawn_background_session_import`) & State Transitions (`crates/pcb-doc/src/lib.rs`)

```rust
pub fn spawn_background_session_import(
    raw_json: String,
) -> std::sync::mpsc::Receiver<ImportJobMessage> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if tx
            .send(ImportJobMessage::Progress {
                completed_steps: 1,
                total_steps: 2,
                phase: "Validating envelope".to_owned(),
            })
            .is_err()
        {
            return;
        }
        match BoardModel::from_session_json(&raw_json) {
            Ok((board_id, board)) => {
                let _ = tx.send(ImportJobMessage::Progress {
                    completed_steps: 2,
                    total_steps: 2,
                    phase: "Loading copper layers".to_owned(),
                });
                let _ = tx.send(ImportJobMessage::Completed { board_id, board });
            }
            Err(err) => {
                let _ = tx.send(ImportJobMessage::Failed(err));
            }
        }
    });
    rx
}

impl BoardModel {
    pub fn apply_import_message(
        &mut self,
        active_board_id: &mut String,
        state: &mut ImportJobState,
        msg: ImportJobMessage,
    ) {
        match msg {
            ImportJobMessage::Progress {
                completed_steps,
                total_steps,
                phase,
            } => {
                let fraction = progress_fraction(completed_steps, total_steps);
                *state = ImportJobState::Running { fraction, phase };
            }
            ImportJobMessage::Completed { board_id, board } => {
                *active_board_id = board_id.clone();
                *self = board;
                *state = ImportJobState::Succeeded { board_id };
            }
            ImportJobMessage::Failed(err) => {
                *state = ImportJobState::Failed(err.to_string());
            }
        }
    }
}
```

- **`std::sync::mpsc::channel()` & `std::thread::spawn(move || { ... })`**: Creates a multi-producer, single-consumer channel pair `(tx, rx)`. The `move` closure transfers ownership of `tx` and `raw_json: String` onto a background OS thread while returning `rx` (`Receiver<ImportJobMessage>`) immediately to the UI caller.
- **Early Cancellation Detection (`if tx.send(...).is_err() { return; }`)**: If the user clicks `"Cancel Import"` on the UI thread (dropping `rx`), `tx.send(...)` returns `Err(SendError)` and the worker exits before doing expensive parsing work.
- **`BoardModel::apply_import_message`**: A pure domain state-transition method that applies each `ImportJobMessage` to `self`, `active_board_id`, and `state`.

### Block 3: Non-Blocking `try_recv()` Polling, `ctx.request_repaint()` & `egui::ProgressBar` (`src/main.rs`)

```rust
// Added fields on `pub struct PcbReviewerApp`:
pub import_rx: Option<std::sync::mpsc::Receiver<ImportJobMessage>>,
pub import_state: ImportJobState,

// Added methods on `impl PcbReviewerApp`:
pub fn start_async_import(&mut self, raw_json: String) {
    self.import_state = ImportJobState::Running {
        fraction: 0.0,
        phase: "Spawning worker".to_owned(),
    };
    self.import_rx = Some(spawn_background_session_import(raw_json));
}

pub fn cancel_async_import(&mut self) {
    self.import_rx = None;
    self.import_state = ImportJobState::Idle;
}

pub fn poll_async_import(&mut self, ctx: &egui::Context) {
    if let Some(rx) = &self.import_rx {
        while let Ok(msg) = rx.try_recv() {
            self.board.apply_import_message(
                &mut self.view.active_board_id,
                &mut self.import_state,
                msg,
            );
        }
    }
    if self.import_state.is_running() {
        ctx.request_repaint();
    }
}

// Inside `PcbReviewerApp::render_session_panel`:
ui.label(self.import_state.status_badge());
if let ImportJobState::Running { fraction, phase } = &self.import_state {
    ui.add(egui::ProgressBar::new(*fraction).text(phase));
}
if ui.button("Cancel Import").clicked() {
    self.cancel_async_import();
}
```

- **`while let Ok(msg) = rx.try_recv()`**: Called at the top of `eframe::App::ui` via `self.poll_async_import(ui.ctx())`. Unlike `rx.recv()` (which blocks the calling thread and freezes the window), `rx.try_recv()` drains all currently queued messages and returns `Err(TryRecvError::Empty)` in nanoseconds when no more messages are ready.
- **`if self.import_state.is_running() { ctx.request_repaint(); }`**: Wakes `eframe`'s reactive event loop on the next frame so `poll_async_import` keeps polling the channel even when the mouse cursor is completely still.
- **`ui.add(egui::ProgressBar::new(*fraction).text(phase))`**: Renders an animated `egui::ProgressBar` with the current worker phase text while `self.import_state` is `Running`.

### Block 4: Worker Channel Streaming & Headless Progress Tests (`crates/pcb-doc/src/lib.rs` & `src/main.rs`)

```rust
// In `crates/pcb-doc/src/lib.rs` (`#[cfg(test)] mod tests`):
#[test]
fn background_session_import_streams_progress_and_handles_errors_and_cancellation() {
    assert_eq!(progress_fraction(0, 0), 0.0);
    assert_eq!(progress_fraction(1, 4), 0.25);
    assert_eq!(progress_fraction(5, 2), 1.0);

    assert!(!ImportJobState::Idle.is_running());
    assert_eq!(ImportJobState::Idle.status_badge(), "Import: Idle");
    let running = ImportJobState::Running {
        fraction: 0.5,
        phase: "Validating envelope".to_owned(),
    };
    assert!(running.is_running());
    assert_eq!(
        running.status_badge(),
        "Importing: Validating envelope (50%)"
    );

    let mut source_board = BoardModel::default();
    source_board.clear_review_pins();
    let valid_json = source_board.to_session_json("async-rev-d").unwrap();

    let rx = spawn_background_session_import(valid_json);
    let msgs: Vec<ImportJobMessage> = rx.iter().collect();
    assert_eq!(msgs.len(), 3);
    assert_eq!(
        msgs[0],
        ImportJobMessage::Progress {
            completed_steps: 1,
            total_steps: 2,
            phase: "Validating envelope".to_owned(),
        }
    );
    assert_eq!(
        msgs[1],
        ImportJobMessage::Progress {
            completed_steps: 2,
            total_steps: 2,
            phase: "Loading copper layers".to_owned(),
        }
    );

    let mut target_board = BoardModel::default();
    let mut active_id = "old-id".to_owned();
    let mut state = ImportJobState::Idle;
    for msg in msgs {
        target_board.apply_import_message(&mut active_id, &mut state, msg);
    }
    assert_eq!(active_id, "async-rev-d");
    assert!(target_board.review_pins.is_empty());
    assert_eq!(
        state,
        ImportJobState::Succeeded {
            board_id: "async-rev-d".to_owned()
        }
    );
    assert_eq!(state.status_badge(), "Import OK: async-rev-d");

    let bad_rx = spawn_background_session_import("{ broken".to_owned());
    let bad_msgs: Vec<ImportJobMessage> = bad_rx.iter().collect();
    assert_eq!(bad_msgs.len(), 2);
    for msg in bad_msgs {
        target_board.apply_import_message(&mut active_id, &mut state, msg);
    }
    assert!(matches!(state, ImportJobState::Failed(_)));
    assert!(state.status_badge().starts_with("Import Error:"));
}
```

- **Deterministic Channel Collection (`rx.iter().collect()`)**: In unit tests, `rx.iter().collect()` collects every message emitted by `spawn_background_session_import` until the worker thread drops `tx`, letting us assert the exact sequence of `Progress` and `Completed` / `Failed` messages without sleep timers.
- **Controlled `mpsc` Injection in `egui_kittest` (`src/main.rs`)**: In `standalone_layer_swatch_and_headless_ui_workflows_pass`, we create a controlled `std::sync::mpsc::channel()`, send `ImportJobMessage::Progress { completed_steps: 1, total_steps: 2, phase: "Validating envelope".to_owned() }`, assign `harness.state_mut().import_rx = Some(rx)`, call `harness.step()`, and verify `"Importing: Validating envelope (50%)"` before clicking `"Cancel Import"`.

### Complete Lesson 0021 Code Listing (In Context)

Here are the complete background import worker, state machine, UI polling, and test additions in `crates/pcb-doc/src/lib.rs` and `src/main.rs`:

```rust caption="Complete ImportJobMessage, ImportJobState, and worker in crates/pcb-doc/src/lib.rs"
pub fn progress_fraction(completed_steps: usize, total_steps: usize) -> f32 {
    if total_steps == 0 {
        return 0.0;
    }
    (completed_steps as f32 / total_steps as f32).clamp(0.0, 1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportJobMessage {
    Progress {
        completed_steps: usize,
        total_steps: usize,
        phase: String,
    },
    Completed {
        board_id: String,
        board: BoardModel,
    },
    Failed(SessionIoError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportJobState {
    Idle,
    Running { fraction: f32, phase: String },
    Succeeded { board_id: String },
    Failed(String),
}

impl ImportJobState {
    pub fn is_running(&self) -> bool {
        matches!(self, ImportJobState::Running { .. })
    }

    pub fn status_badge(&self) -> String {
        match self {
            ImportJobState::Idle => "Import: Idle".to_owned(),
            ImportJobState::Running { fraction, phase } => {
                format!("Importing: {phase} ({:.0}%)", fraction * 100.0)
            }
            ImportJobState::Succeeded { board_id } => format!("Import OK: {board_id}"),
            ImportJobState::Failed(err) => format!("Import Error: {err}"),
        }
    }
}

pub fn spawn_background_session_import(
    raw_json: String,
) -> std::sync::mpsc::Receiver<ImportJobMessage> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if tx
            .send(ImportJobMessage::Progress {
                completed_steps: 1,
                total_steps: 2,
                phase: "Validating envelope".to_owned(),
            })
            .is_err()
        {
            return;
        }
        match BoardModel::from_session_json(&raw_json) {
            Ok((board_id, board)) => {
                let _ = tx.send(ImportJobMessage::Progress {
                    completed_steps: 2,
                    total_steps: 2,
                    phase: "Loading copper layers".to_owned(),
                });
                let _ = tx.send(ImportJobMessage::Completed { board_id, board });
            }
            Err(err) => {
                let _ = tx.send(ImportJobMessage::Failed(err));
            }
        }
    });
    rx
}

impl BoardModel {
    pub fn apply_import_message(
        &mut self,
        active_board_id: &mut String,
        state: &mut ImportJobState,
        msg: ImportJobMessage,
    ) {
        match msg {
            ImportJobMessage::Progress {
                completed_steps,
                total_steps,
                phase,
            } => {
                let fraction = progress_fraction(completed_steps, total_steps);
                *state = ImportJobState::Running { fraction, phase };
            }
            ImportJobMessage::Completed { board_id, board } => {
                *active_board_id = board_id.clone();
                *self = board;
                *state = ImportJobState::Succeeded { board_id };
            }
            ImportJobMessage::Failed(err) => {
                *state = ImportJobState::Failed(err.to_string());
            }
        }
    }
}
```

```rust caption="Complete non-blocking mpsc polling, ProgressBar UI, and headless test in src/main.rs"
impl PcbReviewerApp {
    pub fn start_async_import(&mut self, raw_json: String) {
        self.import_state = ImportJobState::Running {
            fraction: 0.0,
            phase: "Spawning worker".to_owned(),
        };
        self.import_rx = Some(spawn_background_session_import(raw_json));
    }

    pub fn cancel_async_import(&mut self) {
        self.import_rx = None;
        self.import_state = ImportJobState::Idle;
    }

    pub fn poll_async_import(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.import_rx {
            while let Ok(msg) = rx.try_recv() {
                self.board.apply_import_message(
                    &mut self.view.active_board_id,
                    &mut self.import_state,
                    msg,
                );
            }
        }
        if self.import_state.is_running() {
            ctx.request_repaint();
        }
    }
}

// Inside `standalone_layer_swatch_and_headless_ui_workflows_pass` (`src/main.rs`):
assert!(harness.query_by_label("Import: Idle").is_some());
let (tx, rx) = std::sync::mpsc::channel();
tx.send(ImportJobMessage::Progress {
    completed_steps: 1,
    total_steps: 2,
    phase: "Validating envelope".to_owned(),
})
.unwrap();
harness.state_mut().import_rx = Some(rx);
harness.step();
assert!(
    harness
        .query_by_label("Importing: Validating envelope (50%)")
        .is_some()
);

harness.get_by_label("Cancel Import").click();
harness.run();
assert_eq!(harness.state().import_state, ImportJobState::Idle);
assert!(harness.state().import_rx.is_none());
```

{{widget:async-import-sim}}

:::exercise id="ex0021" title="Interactive Lab 2: Poll `mpsc::Receiver::try_recv()` & Drive `egui::ProgressBar`" badge="Skill Practice • Background mpsc & ProgressBar"
Complete `ImportJobState::progress_fraction`, `PcbReviewerApp::cancel_async_import`, and the `egui::ProgressBar` rendering inside `render_session_panel`.

```rust
pub fn progress_fraction(completed_steps: usize, total_steps: usize) -> f32 {
    if total_steps == 0 {
        return 0.0;
    }
    (completed_steps as f32 / total_steps as f32).{{blank:clamp(0.0, 1.0)|Clamp the progress ratio to 0.0..=1.0|clamp(...)|155px}}
}

pub fn cancel_async_import(&mut self) {
    self.import_rx = {{blank:None|Drop the mpsc::Receiver by setting import_rx to None|None|85px}};
    self.import_state = ImportJobState::Idle;
}

// Inside render_session_panel:
if let ImportJobState::Running { fraction, phase } = &self.import_state {
    ui.add(egui::{{blank:ProgressBar::new(*fraction)|Construct an egui::ProgressBar with *fraction|ProgressBar::new(...)|235px}}.text(phase));
}
```

:::prompts
hint: Clamp the step ratio with `.clamp(0.0, 1.0)`, drop the channel receiver by setting `self.import_rx = None`, and construct `egui::ProgressBar::new(*fraction)`.
success: Well done! Background imports stream progress over `mpsc::channel`, update `egui::ProgressBar` without blocking the UI thread, and cancel cleanly when `import_rx` is set to `None`.
:::
:::

:::verify id="verify-0021" title="Lab 3: Background Worker `mpsc` Channel & Progress UI Mutation Gate (`0021`)" badge="cargo test --workspace + cargo mutants" tier="Tier 2 + Tier 6 (Headless UI)" caught="269 / 269 (100%)" missed="0 Missed"
Run the workspace test suite and mutate the background import worker, progress fraction math, non-blocking `try_recv()` polling, and `Panel::right` controls:

```bash caption="Run workspace tests and cargo-mutants on 0020 async import functions"
cargo test --manifest-path ./apps/0021-non-blocking-file-import-and-progress-ui/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0021-non-blocking-file-import-and-progress-ui/Cargo.toml --workspace --in-place -F "progress_fraction|ImportJobState|apply_import_message|spawn_background_session_import|start_async_import|cancel_async_import|poll_async_import"
```

- [ ] `cargo test --manifest-path ./apps/0021-non-blocking-file-import-and-progress-ui/Cargo.toml --workspace` passes all **20 tests** (**14** in `pcb-doc` and **6** in `pcb-reviewer`).
- [ ] `background_session_import_streams_progress_and_handles_errors_and_cancellation` verifies staged `Progress` messages, `Completed` state transitions, `Failed(SessionIoError)` reporting, and receiver drop cancellation.
- [ ] `cargo mutants` catches all **14 viable new background import & UI mutants** (`4 unviable, 0 missed`), bringing the workspace total to **269 / 269 viable mutants caught (0 missed)**.
:::

:::quiz id="quiz-0021"
Q: Why does `poll_async_import` call `Receiver::try_recv()` inside a `while let Ok(msg)` loop instead of `recv()`?
- [ ] Because `recv()` drops all pending messages from the channel buffer
- [x] Because `recv()` blocks the UI thread and freezes immediate-mode frames
- [ ] Because `try_recv()` automatically spawns a fresh operating system worker thread
- [ ] Because `recv()` cannot deserialize versioned JSON envelopes across thread boundaries
why: `Receiver::recv()` blocks the calling thread until a message arrives, which would freeze the `egui` window; `try_recv()` returns `Err(TryRecvError::Empty)` immediately when no message is waiting.
hint: Think about what happens to a 60-FPS UI loop if `ui()` blocks waiting on a thread.

Q: Why do we call `ctx.request_repaint()` whenever `self.import_state` is `ImportJobState::Running`?
- [ ] To clear the undo history stack before loading copper layers
- [ ] To lock the GPU mutex across all active test threads
- [x] To wake `egui` on the next frame so progress updates
- [ ] To terminate the background worker thread immediately after one message
why: When the user is not moving the mouse or pressing keys, `eframe` pauses repaints to save CPU/battery; `ctx.request_repaint()` schedules another frame so `poll_async_import` continues draining worker progress.
hint: Check `poll_async_import` in `apps/0021-non-blocking-file-import-and-progress-ui/src/main.rs`.

Q: How does `cancel_async_import` cleanly stop receiving messages from an in-flight background import worker?
- [x] By setting `import_rx` to `None` and resetting `import_state` to `Idle`
- [ ] By calling `std::process::exit` inside the right-hand session inspector panel closure
- [ ] By mutating the static command registry slice at runtime directly
- [ ] By panicking inside the worker thread when progress reaches fifty
why: Dropping the `mpsc::Receiver` (`self.import_rx = None`) closes the channel, causing subsequent `tx.send(...)` calls in the worker thread to return `Err` and exit cleanly.
hint: Look at `cancel_async_import` and `if tx.send(...).is_err() { return; }`.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [`std::sync::mpsc::Receiver::try_recv` Standard Library Docs](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.try_recv) and [`egui::ProgressBar` Official API Docs](https://docs.rs/egui/latest/egui/widgets/struct.ProgressBar.html).
:::

:::teacher-prompt
Next up: [Lesson 0022: Offscreen Snapshot Parity (`gpu_lock`), Quality Scorecards & WebAssembly (`wasm32`) Readiness](./0022-headless-snapshot-testing-and-wasm-build.html), where we complete Act V with deterministic 64-bit FNV-1a visual snapshot digests, process-wide `gpu_lock()` serialization, `wasm32` target gates, and a self-auditing release scorecard!
:::

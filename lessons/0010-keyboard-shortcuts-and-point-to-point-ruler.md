---
title: "Lesson 0010: Keyboard Shortcuts, Data-Carrying Enums & Point-to-Point CAD Ruler"
subtitle: Binding CAD hotkeys (`F`, `M`, `Esc`) via `ctx.input_mut` + `consume_key`, modelling tool state machines with data-carrying Rust enums, and simulating key presses in `egui_kittest`
lesson_num: "0010"
act_badge: "Act II Finale: Schematic Views, Layout & egui_kittest"
card:
  num: "Lesson 0010"
  mutants: "59 / 59 Mutants Caught"
  title: "Keyboard Shortcuts (`F`, `M`, `Esc`) & Point-to-Point CAD Ruler"
  summary: "Consume single-key CAD shortcuts with `ctx.input_mut` and `consume_key`, model tool states with data-carrying enums, and render a live Δx / Δy distance ruler."
  egui_skill: "`InputState::consume_key`, `Key`, CAD ruler overlay"
  rust_concept: "Data-carrying enums (`enum ActiveTool`) & pattern matching"
  app_dir: "apps/0010-keyboard-shortcuts-and-point-to-point-ruler"
---

Watch anyone who spends all day in KiCad or Altium: their left hand never leaves the keyboard. Reaching for a toolbar button every time you want to measure a pad pitch or reset the camera is slow; tapping `M`, `F`, or `Esc` is instant.

In [Lesson 0009](./0009-searchable-netlist-and-bom-inspector.html), we built a searchable, virtualised BOM table. Now, we wrap up **Act II** by adding two essential CAD ergonomics features: single-key shortcuts (`F` to fit/reset the board camera, `M` to activate the point-to-point measurement ruler, and `Esc` to cancel/clear) and a live **Euclidean distance ruler overlay** reporting $\Delta x$, $\Delta y$, and $\text{distance} = \sqrt{\Delta x^2 + \Delta y^2}$ in physical millimetres.

```mermaid
stateDiagram-v2
    [*] --> Select: Default
    Select --> Measure: Key::M (activate_measure_tool)
    Measure --> Measure: set_measure_points(start_mm, end_mm)
    Measure --> Select: Key::Escape (cancel_tool_and_selection)
    Select --> Select: Key::F (fit_board_in_viewport)
```
_Figure 10.1: Tool state machine modelled with a data-carrying Rust `enum ActiveTool` so ruler coordinates only exist while in `Measure` mode._

## 1. Just-in-Time Rust: State Machines with Data-Carrying Enums

Unlike C or Java enums that are glorified integer constants, each variant of a Rust `enum` can carry its own typed payload ([Rust Book §6.1](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)).

So, by modelling our CAD tool as `ActiveTool`, it becomes impossible for stale ruler coordinates to hang around when the user switches back to `Select` mode:{{sidenote:Making illegal states unrepresentable is one of Rust's best party tricks: instead of juggling separate `is_measuring: bool` and `ruler_start: Option<Pos2>` fields that inevitably get out of sync, the `ActiveTool::Measure` variant owns its endpoints directly.}}

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveTool {
    Select,
    Measure {
        start_mm: Option<Pos2>,
        end_mm: Option<Pos2>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerMeasurement {
    pub dx_mm: f32,
    pub dy_mm: f32,
    pub distance_mm: f32,
}

impl RulerMeasurement {
    pub fn between(start_mm: Pos2, end_mm: Pos2) -> Self {
        let dx_mm = end_mm.x - start_mm.x;
        let dy_mm = end_mm.y - start_mm.y;
        let distance_mm = dx_mm.hypot(dy_mm);
        Self {
            dx_mm,
            dy_mm,
            distance_mm,
        }
    }
}
```

## 2. Consuming Keyboard Shortcuts with `ctx.input_mut` & `consume_key`

Now, what happens if the user is typing `"m"` into the BOM search box from Lesson 0009? You definitely don't want that keystroke switching tools mid-search!

So, we first check `!ctx.wants_keyboard_input()` to make sure no text box has keyboard focus, and then match our CAD hotkeys using [`InputState::consume_key`](https://docs.rs/egui/latest/egui/struct.InputState.html#method.consume_key). Calling `consume_key` both returns `true` when the key was pressed and removes the key event from the frame's queue so no other widget reacts to it twice:

```rust
pub fn handle_hotkeys(&mut self, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() {
        return;
    }
    let pressed_f = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F));
    let pressed_m = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M));
    let pressed_esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

    if pressed_f {
        self.view.fit_board_in_viewport();
    }
    if pressed_m {
        self.view.activate_measure_tool();
    }
    if pressed_esc {
        self.view.cancel_tool_and_selection();
    }
}
```

{{widget:hotkey-ruler-sim}}

:::exercise title="Interactive Lab 2: Complete the Euclidean Ruler & Hotkey Consumer" badge="hypot() • consume_key • harness.key_press"
Fill in the three blanks below to (1) compute the Euclidean hypotenuse $\sqrt{\Delta x^2 + \Delta y^2}$ in `RulerMeasurement::between`, (2) consume `Key::M` from `InputState`, and (3) simulate pressing `Key::M` in our headless `egui_kittest` test:

```rust
impl RulerMeasurement {
    pub fn between(start_mm: Pos2, end_mm: Pos2) -> Self {
        let dx_mm = end_mm.x - start_mm.x;
        let dy_mm = end_mm.y - start_mm.y;
        let distance_mm = dx_mm.{{blank:hypot(dy_mm)|Call `hypot(dy_mm)` on `dx_mm` to compute the Euclidean distance.|/* 1: hypot */|125px}};
        Self { dx_mm, dy_mm, distance_mm }
    }
}

// Inside handle_hotkeys():
let pressed_m = ctx.input_mut(|i| i.{{blank:consume_key|Use the `InputState` method that matches and consumes a key press (`consume_key`).|/* 2: consume */|135px}}(egui::Modifiers::NONE, egui::Key::M));
if pressed_m {
    self.view.activate_measure_tool();
}

// Inside #[test] fn headless_hotkeys_switch_tools_and_fit_viewport():
harness.{{blank:key_press|Call `key_press` on `harness` to queue a key-down and key-up event.|/* 3: press */|120px}}(egui::Key::M);
harness.run();
```
:::prompts
hint: **Hints:** (1) `hypot(dy_mm)`. (2) `consume_key`. (3) `key_press`.
success: **Bravo!** Your CAD ruler computes $\sqrt{\Delta x^2 + \Delta y^2}$ with `dx_mm.hypot(dy_mm)`, consumes hotkeys via `i.consume_key(...)`, and tests them headlessly with `harness.key_press(...)`.
:::
:::

:::verify title="Interactive Lab 3: Simulating Hotkeys with `harness.key_press` Under `cargo mutants`" badge="cargo test + cargo mutants" tier="Tier 2 + 6: Hotkeys & CAD Ruler" caught="59 Caught (14 Unviable)" missed="0 Missed"
In `apps/0010-keyboard-shortcuts-and-point-to-point-ruler`, our test suite uses `harness.key_press(egui::Key::M)`, `harness.key_press(egui::Key::Escape)`, and `harness.key_press(egui::Key::F)` to drive `handle_hotkeys` headlessly, while asserting non-zero asymmetric 3-4-5 right-triangle coordinates (`start = (20.0, 22.0)`, `end = (68.0, 58.0)` → `dx = 48.0`, `dy = 36.0`, `distance = 60.0`) so every arithmetic and hotkey mutant is caught!

```rust
#[test]
fn headless_hotkeys_switch_tools_and_fit_viewport() {
    let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());

    // 1. Press 'M' -> switches to ActiveTool::Measure and updates status label
    harness.key_press(egui::Key::M);
    harness.run();
    assert_eq!(
        harness.state().view.active_tool,
        ActiveTool::Measure { start_mm: None, end_mm: None }
    );
    assert!(harness.query_by_label("Active Tool: Measure (M)").is_some());

    // 2. Press 'Esc' -> returns to ActiveTool::Select and clears selection
    harness.state_mut().view.selected_net = Some(NetId(1));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().view.active_tool, ActiveTool::Select);
    assert_eq!(harness.state().view.selected_net, None);

    // 3. Pan away, then press 'F' -> restores default viewport transform
    harness.state_mut().view.viewport.pan_by(vec2(45.0, -30.0));
    harness.key_press(egui::Key::F);
    harness.run();
    assert_eq!(harness.state().view.viewport, BoardViewport::default());
}
```

- [ ] **1. Run `cargo test` in `apps/0010-keyboard-shortcuts-and-point-to-point-ruler`:** Confirm all 19 unit + headless `egui_kittest` tests pass.
- [ ] **2. Run `cargo mutants --in-place`:** Verify that all 59 viable mutants across Act I & Act II (`0001`–`0010`) are **Caught** with **0 Missed**.
:::

:::quiz title="Retrieval Practice: Data-Carrying Enums, `consume_key` & `key_press`"
Q: Why is `enum ActiveTool { Select, Measure { ... } }` safer than keeping separate boolean and coordinate fields?
- [ ] Booleans allocate heap memory during frames
- [ ] Enums disable borrow checking inside frames
- [ ] Structs forbid storing Pos2 fields directly
- [x] Ruler points exist exclusively when enabled
why: Storing `start_mm` and `end_mm` inside `ActiveTool::Measure` makes illegal states unrepresentable: ruler endpoints only exist when the Measure tool is active.
hint: Think about what happens to `start_mm` and `end_mm` when `active_tool` switches back to `ActiveTool::Select`.

Q: What is the advantage of calling `ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::M))`?
- [ ] Blocking pointer clicks during next frames
- [x] Preventing other widgets from handling key
- [ ] Resetting transform matrices on every step
- [ ] Spawning background threads for hotkey run
why: `consume_key` checks if the key shortcut was pressed and removes that key event from `InputState` so no other widget processes the same key press.
hint: Consider why `consume_key` requires `input_mut` instead of read-only `input`.

Q: How does `harness.key_press(egui::Key::M)` test our hotkey handler inside `egui_kittest`?
- [ ] Opening native windows to capture hardware
- [ ] Bypassing update loops to overwrite fields
- [x] Queueing synthetic key events before steps
- [ ] Compiling shader binaries during key press
why: `harness.key_press` queues synthetic `Event::Key` press and release events that are fed into `egui::Context` on the next `harness.run()` step.
hint: Think about how `egui_kittest::Harness` simulates keyboard input without an OS window.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [`egui::InputState::consume_key` Official API Docs](https://docs.rs/egui/latest/egui/struct.InputState.html#method.consume_key) and [The Rust Programming Language — §6.1 "Defining an Enum"](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html).
:::

:::teacher-prompt
**Act II Complete!** You have expanded the PCB Design Reviewer with schematic component symbols, `Painter::text` reference designators, split-viewport cross-probing with `egui::Scene`, virtualised BOM search tables, CAD keyboard shortcuts, and a point-to-point measurement ruler—all verified headlessly with `egui_kittest::Harness` and 100% caught mutants. Ask any questions about Act II in chat, or let me know when you are ready for **Act III (`0011`–`0014`): Design Review Workflows, Sticky-Note Callout Pins, DRC Overlays & Custom `egui::Widget` Layer Swatches**!
:::

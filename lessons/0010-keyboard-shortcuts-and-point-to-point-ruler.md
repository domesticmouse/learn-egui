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

So, by modelling our CAD tool as `ActiveTool`, it becomes impossible for stale ruler coordinates to hang around when the user switches back to `Select` mode.{{sidenote:Making illegal states unrepresentable is one of Rust's best party tricks: instead of juggling separate `is_measuring: bool` and `ruler_start: Option<Pos2>` fields that inevitably get out of sync, the `ActiveTool::Measure` variant owns its endpoints directly.}}

## 2. Anatomy of Lesson 0010's Additions: Block by Block

Let's examine each block of new and updated code in `apps/0010-keyboard-shortcuts-and-point-to-point-ruler/src/main.rs` compared to Lesson 0009, and then view the complete implementation in context.

### Block 1: `ActiveTool` Data-Carrying Enum & `RulerMeasurement` Geometry

```rust
/// Data-carrying enum modeling the active CAD interaction tool.
/// Ruler endpoints (`start_mm`, `end_mm`) only exist when `Measure` is active.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveTool {
    Select,
    Measure {
        start_mm: Option<Pos2>,
        end_mm: Option<Pos2>,
    },
}

/// Point-to-point Euclidean measurement result in physical millimeters.
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

    pub fn summary_label(&self) -> String {
        format!(
            "Ruler: dx={:.1} mm, dy={:.1} mm, dist={:.1} mm",
            self.dx_mm, self.dy_mm, self.distance_mm
        )
    }
}
```

- **`pub enum ActiveTool`**: Defines two variants: a unit variant `Select` and a struct-like variant `Measure { start_mm: Option<Pos2>, end_mm: Option<Pos2> }`. Because `Option<Pos2>` is `Copy`, the entire `ActiveTool` enum derives `Clone, Copy, Debug, PartialEq`.
- **`dx_mm.hypot(dy_mm)`**: Standard library `f32::hypot` computes $\sqrt{\Delta x^2 + \Delta y^2}$ without intermediate overflow or underflow.
- **`pub fn summary_label(&self) -> String`**: Formats the measurement to 1 decimal place (`{:.1}`) for both the top status bar and the canvas ruler callout.

### Block 2: Tool State Transitions on `ViewState`

```rust
pub struct ViewState {
    // ... [layer booleans, viewport, pcb_scene_rect, selection, bom_search_query] ...
    pub active_tool: ActiveTool,
}

impl Default for ViewState {
    fn default() -> Self {
        let mut view = Self {
            // ...
            active_tool: ActiveTool::Select,
        };
        view.reset_layers();
        view
    }
}

impl ViewState {
    // ...
    pub fn fit_board_in_viewport(&mut self) {
        self.viewport = BoardViewport::default();
        self.pcb_scene_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
    }

    pub fn activate_measure_tool(&mut self) {
        self.active_tool = ActiveTool::Measure {
            start_mm: None,
            end_mm: None,
        };
    }

    pub fn set_measure_points(&mut self, start_mm: Pos2, end_mm: Pos2) {
        self.active_tool = ActiveTool::Measure {
            start_mm: Some(start_mm),
            end_mm: Some(end_mm),
        };
    }

    pub fn current_measurement(&self) -> Option<RulerMeasurement> {
        match self.active_tool {
            ActiveTool::Measure {
                start_mm: Some(s),
                end_mm: Some(e),
            } => Some(RulerMeasurement::between(s, e)),
            _ => None,
        }
    }

    pub fn cancel_tool_and_selection(&mut self) {
        self.active_tool = ActiveTool::Select;
        self.selected_net = None;
        self.selected_component = None;
    }
}
```

- **`fit_board_in_viewport(&mut self)`**: Renames our earlier `reset_viewport` helper to match CAD terminology (`Fit Board`, bound to `Key::F`), restoring both `BoardViewport::default()` and the full `100×70 mm` `pcb_scene_rect`.
- **`activate_measure_tool` & `set_measure_points`**: Transition `self.active_tool` into `ActiveTool::Measure` with either empty (`None`) or populated (`Some(...)`) endpoints.
- **Nested Pattern Matching in `current_measurement(&self)`**: The pattern `ActiveTool::Measure { start_mm: Some(s), end_mm: Some(e) }` destructures both the enum variant _and_ both inner `Option<Pos2>` fields in a single match arm, returning `Some(RulerMeasurement::between(s, e))` only when both endpoints have been placed.
- **`cancel_tool_and_selection(&mut self)`**: Bound to `Key::Escape` (and the `"Clear Selection"` button), resetting `active_tool` back to `ActiveTool::Select`—which automatically drops any active ruler endpoints—while clearing `selected_net` and `selected_component`.

### Block 3: Consuming Hotkeys (`consume_key`) & Drawing the Ruler Overlay

We match our CAD hotkeys using [`InputState::consume_key`](https://docs.rs/egui/latest/egui/struct.InputState.html#method.consume_key). Calling `consume_key` both returns `true` when the key was pressed and removes the key event from the frame's queue so no other widget reacts to it twice:

```rust
impl PcbReviewerApp {
    // ...
    pub fn handle_hotkeys(&mut self, ctx: &egui::Context) {
        let pressed_f = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F));
        let pressed_m = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M));
        let pressed_esc =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

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

    pub fn render_schematic_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // ... [Schematic symbol buttons & Focused Footprint label] ...
            ui.separator();
            match self.view.active_tool {
                ActiveTool::Select => {
                    ui.label("Active Tool: Select");
                }
                ActiveTool::Measure { .. } => {
                    ui.label("Active Tool: Measure (M)");
                }
            }
            if let Some(measurement) = self.view.current_measurement() {
                ui.label(measurement.summary_label());
            }
        });
    }
}
```

And inside `render_canvas` and `paint_board`, we branch on `self.view.active_tool` to record ruler clicks and draw the yellow measurement vector on top of the board:

```rust
// Inside render_canvas() when `response.clicked()`:
match &mut self.view.active_tool {
    ActiveTool::Select => {
        self.view.selected_net = self.view.hovered_net;
    }
    ActiveTool::Measure { start_mm, end_mm } => {
        if start_mm.is_none() {
            *start_mm = Some(world_mm);
        } else {
            *end_mm = Some(world_mm);
        }
    }
}

// Inside paint_board() after drawing silkscreen:
if let ActiveTool::Measure {
    start_mm: Some(s_mm),
    end_mm: Some(e_mm),
} = self.view.active_tool
{
    let p0 = self.view.viewport.world_to_screen(s_mm);
    let p1 = self.view.viewport.world_to_screen(e_mm);
    let ruler_color = Color32::from_rgb(255, 230, 100);
    painter.line_segment([p0, p1], Stroke::new(2.0_f32, ruler_color));
    if let Some(m) = self.view.current_measurement() {
        let mid = pos2((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5 - 10.0);
        painter.text(
            mid,
            Align2::CENTER_BOTTOM,
            m.summary_label(),
            FontId::monospace(11.0),
            ruler_color,
        );
    }
}
```

- **`ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, ...))`**: Borrows `egui::InputState` mutably at the start of `eframe::App::ui` (`self.handle_hotkeys(ui.ctx())`), consuming unmodified `Key::F`, `Key::M`, and `Key::Escape` presses.
- **`ActiveTool::Measure { .. }`**: In `render_schematic_bar`, the `..` wildcard pattern ignores the inner `start_mm` and `end_mm` fields when we only care whether the user is in `Measure` mode.
- **Mutable Struct-Variant Binding (`match &mut self.view.active_tool`)**: Matching on `&mut self.view.active_tool` binds `start_mm` and `end_mm` as `&mut Option<Pos2>` references, allowing `*start_mm = Some(world_mm)` and `*end_mm = Some(world_mm)` to update the enum payload in place on canvas clicks.
- **`if let ActiveTool::Measure { start_mm: Some(s_mm), end_mm: Some(e_mm) } = self.view.active_tool`**: Projects both millimetre endpoints into screen coordinates (`p0`, `p1`), draws a `2.0_f32` yellow ruler segment, and anchors `m.summary_label()` above the midpoint using `Align2::CENTER_BOTTOM`.

### Block 4: Unit & Headless Hotkey Tests (`harness.key_press`)

```rust
#[test]
fn ruler_measurement_computes_deltas_hypotenuse_and_summary_label() {
    let mut view = ViewState::default();
    assert_eq!(view.current_measurement(), None);

    view.activate_measure_tool();
    assert_eq!(view.current_measurement(), None);

    // 3-4-5 right triangle scaled by 12: dx=48, dy=36 -> dist=60
    view.set_measure_points(pos2(20.0, 22.0), pos2(68.0, 58.0));
    let m = view.current_measurement().expect("measurement should exist");
    assert_eq!(m.dx_mm, 48.0);
    assert_eq!(m.dy_mm, 36.0);
    assert_eq!(m.distance_mm, 60.0);
    assert_eq!(
        m.summary_label(),
        "Ruler: dx=48.0 mm, dy=36.0 mm, dist=60.0 mm"
    );
}

#[test]
fn headless_hotkeys_switch_tools_and_fit_viewport() {
    let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());
    assert!(harness.query_by_label("Active Tool: Select").is_some());

    harness.key_press(egui::Key::M);
    harness.run();
    assert_eq!(
        harness.state().view.active_tool,
        ActiveTool::Measure {
            start_mm: None,
            end_mm: None,
        }
    );
    assert!(harness.query_by_label("Active Tool: Measure (M)").is_some());

    harness
        .state_mut()
        .view
        .set_measure_points(pos2(20.0, 22.0), pos2(68.0, 58.0));
    harness.state_mut().view.selected_net = Some(NetId(1));
    harness.run();
    assert!(
        harness
            .query_by_label("Ruler: dx=48.0 mm, dy=36.0 mm, dist=60.0 mm")
            .is_some()
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().view.active_tool, ActiveTool::Select);
    assert_eq!(harness.state().view.selected_net, None);

    harness.state_mut().view.viewport.pan_by(vec2(45.0, -30.0));
    harness.key_press(egui::Key::F);
    harness.run();
    assert_eq!(harness.state().view.viewport, BoardViewport::default());
}
```

- **Non-Zero Asymmetric 3-4-5 Right Triangle (`(20.0, 22.0)` to `(68.0, 58.0)`)**: Choosing non-zero start coordinates (`20.0, 22.0`) and asymmetric deltas (`dx = 48.0`, `dy = 36.0`, `distance = 60.0`) ensures `cargo-mutants` catches any mutation that swaps `-` for `+` in `end_mm.x - start_mm.x` or `end_mm.y - start_mm.y`.
- **`harness.key_press(egui::Key::M)` / `Key::Escape` / `Key::F`**: Injects synthetic key-down and key-up events into the headless `egui::Context` before `harness.run()`, testing every branch of `handle_hotkeys` and the rendered status bar labels without opening an OS window.

### Complete Lesson 0010 Listing (In Context)

Here is how `ActiveTool`, `RulerMeasurement`, `ViewState`, `handle_hotkeys`, and the ruler overlay assemble in `apps/0010-keyboard-shortcuts-and-point-to-point-ruler/src/main.rs`:

```rust
// ... [Imports, MIN_ZOOM, MAX_ZOOM, main(), NetId, PcbLayer, TraceSegment, SmdPad, ComponentSymbol] ...

/// Data-carrying enum modeling the active CAD interaction tool.
/// Ruler endpoints (`start_mm`, `end_mm`) only exist when `Measure` is active.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveTool {
    Select,
    Measure {
        start_mm: Option<Pos2>,
        end_mm: Option<Pos2>,
    },
}

/// Point-to-point Euclidean measurement result in physical millimeters.
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

    pub fn summary_label(&self) -> String {
        format!(
            "Ruler: dx={:.1} mm, dy={:.1} mm, dist={:.1} mm",
            self.dx_mm, self.dy_mm, self.distance_mm
        )
    }
}

// ... [BoardViewport, sample_traces(), sample_pads(), sample_components(), BoardModel] ...

pub struct ViewState {
    pub show_top_copper: bool,
    pub show_bottom_copper: bool,
    pub show_silkscreen: bool,
    pub show_ratsnest: bool,
    pub viewport: BoardViewport,
    pub pcb_scene_rect: Rect,
    pub selected_component: Option<String>,
    pub hovered_net: Option<NetId>,
    pub selected_net: Option<NetId>,
    pub bom_search_query: String,
    pub active_tool: ActiveTool,
}

impl Default for ViewState {
    fn default() -> Self {
        let mut view = Self {
            show_top_copper: false,
            show_bottom_copper: false,
            show_silkscreen: false,
            show_ratsnest: false,
            viewport: BoardViewport::default(),
            pcb_scene_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0)),
            selected_component: None,
            hovered_net: None,
            selected_net: None,
            bom_search_query: String::new(),
            active_tool: ActiveTool::Select,
        };
        view.reset_layers();
        view
    }
}

impl ViewState {
    // ... [reset_layers, isolate_top_copper, show_all_layers, clear_bom_query,
    //      is_layer_visible, active_net, focus_component] ...

    pub fn fit_board_in_viewport(&mut self) {
        self.viewport = BoardViewport::default();
        self.pcb_scene_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
    }

    pub fn activate_measure_tool(&mut self) {
        self.active_tool = ActiveTool::Measure {
            start_mm: None,
            end_mm: None,
        };
    }

    pub fn set_measure_points(&mut self, start_mm: Pos2, end_mm: Pos2) {
        self.active_tool = ActiveTool::Measure {
            start_mm: Some(start_mm),
            end_mm: Some(end_mm),
        };
    }

    pub fn current_measurement(&self) -> Option<RulerMeasurement> {
        match self.active_tool {
            ActiveTool::Measure {
                start_mm: Some(s),
                end_mm: Some(e),
            } => Some(RulerMeasurement::between(s, e)),
            _ => None,
        }
    }

    pub fn cancel_tool_and_selection(&mut self) {
        self.active_tool = ActiveTool::Select;
        self.selected_net = None;
        self.selected_component = None;
    }
}

#[derive(Default)]
pub struct PcbReviewerApp {
    pub board: BoardModel,
    pub view: ViewState,
}

impl PcbReviewerApp {
    // ... [visible_traces_in_z_order, hit_test_net_at_mm] ...

    pub fn handle_hotkeys(&mut self, ctx: &egui::Context) {
        let pressed_f = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F));
        let pressed_m = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M));
        let pressed_esc =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

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

    pub fn render_schematic_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Schematic Symbols (Cross-Probe):");
            for comp in &self.board.components {
                let label = format!("Probe {} ({})", comp.ref_des, comp.value);
                if ui.button(label).clicked() {
                    self.view.focus_component(&self.board, &comp.ref_des);
                }
            }
            if let Some(focused) = &self.view.selected_component {
                ui.label(format!("Focused Footprint: {focused}"));
            }
            ui.separator();
            match self.view.active_tool {
                ActiveTool::Select => {
                    ui.label("Active Tool: Select");
                }
                ActiveTool::Measure { .. } => {
                    ui.label("Active Tool: Measure (M)");
                }
            }
            if let Some(measurement) = self.view.current_measurement() {
                ui.label(measurement.summary_label());
            }
        });
    }

    // ... [render_side_panel calling self.view.cancel_tool_and_selection() on "Clear Selection"] ...

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let mut scene_rect = self.view.pcb_scene_rect;
        egui::Scene::new()
            .zoom_range(0.5..=8.0)
            .show(ui, &mut scene_rect, |ui| {
                let (response, painter) =
                    ui.allocate_painter(vec2(720.0, 520.0), Sense::click_and_drag());

                if response.dragged() {
                    self.view.viewport.pan_by(response.drag_delta());
                }

                if let Some(hover_pos) = response.hover_pos() {
                    let world_mm = self.view.viewport.screen_to_world(hover_pos);
                    self.view.hovered_net = self.hit_test_net_at_mm(world_mm);
                    if response.clicked() {
                        match &mut self.view.active_tool {
                            ActiveTool::Select => {
                                self.view.selected_net = self.view.hovered_net;
                            }
                            ActiveTool::Measure { start_mm, end_mm } => {
                                if start_mm.is_none() {
                                    *start_mm = Some(world_mm);
                                } else {
                                    *end_mm = Some(world_mm);
                                }
                            }
                        }
                    }
                } else {
                    self.view.hovered_net = None;
                }

                self.paint_board(&painter, response.rect);
            });
        self.view.pcb_scene_rect = scene_rect;
    }

    fn paint_board(&self, painter: &egui::Painter, canvas_rect: Rect) {
        // ... [board outline, traces, pads, and silkscreen painting unchanged] ...

        if let ActiveTool::Measure {
            start_mm: Some(s_mm),
            end_mm: Some(e_mm),
        } = self.view.active_tool
        {
            let p0 = self.view.viewport.world_to_screen(s_mm);
            let p1 = self.view.viewport.world_to_screen(e_mm);
            let ruler_color = Color32::from_rgb(255, 230, 100);
            painter.line_segment([p0, p1], Stroke::new(2.0_f32, ruler_color));
            if let Some(m) = self.view.current_measurement() {
                let mid = pos2((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5 - 10.0);
                painter.text(
                    mid,
                    Align2::CENTER_BOTTOM,
                    m.summary_label(),
                    FontId::monospace(11.0),
                    ruler_color,
                );
            }
        }
    }
}

impl eframe::App for PcbReviewerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_hotkeys(ui.ctx());

        egui::Panel::top("schematic_cross_probe_bar").show(ui, |ui| {
            self.render_schematic_bar(ui);
        });

        egui::Panel::left("layer_panel").show(ui, |ui| {
            self.render_side_panel(ui);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.render_canvas(ui);
        });
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

---
title: "Lesson 0013: DRC Clearance Violation Halos, Bounding-Box Reduction & Minimap Clamping"
subtitle: Detecting copper-to-copper pad clearance violations, reducing pad bounds with `Iterator::reduce`, and clamping minimap camera rectangles with `f32::clamp`
lesson_num: "0013"
act_badge: "Act III: Design Review Workflows"
card:
  num: "Lesson 0013"
  mutants: "125 / 125 Mutants Caught"
  title: "DRC Violation Halos & Minimap Clamping"
  summary: "Compute Euclidean edge-to-edge copper pad clearance, highlight Design Rule Check (DRC) violations on the canvas, and clamp bird's-eye minimap camera bounds."
  egui_skill: "`painter.circle_stroke` DRC halos & camera jump"
  rust_concept: "`Iterator::reduce(Rect::union)` & `f32::clamp`"
  app_dir: "apps/0013-drc-violation-overlay-and-minimap"
---

Even experienced PCB designers occasionally nudge a high-voltage pad or trace a fraction of a millimetre too close to an adjacent net. A **Design Rule Check (DRC)** overlay flags every pair of same-layer, different-net copper primitives whose edge-to-edge gap falls below `min_clearance_mm`—and lets you click any violation in the inspector to jump the camera straight to the offending copper.

In [Lesson 0012](./0012-review-status-workflow-and-popups.html), we added review status triage and severity filtering. Now, let's build an automated pad-to-pad DRC clearance detector (`pad_edge_gap_mm` and `detect_pad_clearance_violations`), compute the board's pad bounding box with `Iterator::reduce(Rect::union)`, and clamp the active camera rectangle (`clamped_minimap_camera_rect`) so a bird's-eye minimap indicator never spills outside the physical board edges.

```mermaid
flowchart LR
    Pads["board.pads: Vec&lt;SmdPad&gt;"] --> Pair["Same PcbLayer &amp;&amp; a.net != b.net"]
    Pair --> Gap["pad_edge_gap_mm(a, b)<br/>dx.max(0.0).hypot(dy.max(0.0))"]
    Gap -->|"gap &lt; min_clearance_mm"| Viol["DrcViolation { center_mm, measured_gap_mm, ... }"]
    Viol --> Halo["Canvas: painter.circle_stroke(center_px, 14.0, red)<br/>Panel::left: Click violation -> center camera"]
```
_Figure 13.1: Same-layer pads on distinct nets are checked against `min_clearance_mm`, emitting `DrcViolation` markers rendered as red halos on the canvas and clickable jump buttons in `Panel::left`._

## 1. Just-in-Time Rust: Bounding-Box Reduction (`Iterator::reduce`) & `f32::clamp`

To compute the edge-to-edge Euclidean distance between two axis-aligned rectangular pads `a` and `b`, we subtract their half-widths `(a.size_mm.x + b.size_mm.x) * 0.5` from the centre-to-centre distance `(a.center_mm.x - b.center_mm.x).abs()` along each axis, clamp negative values (which represent axis overlap) to `0.0` via `.max(0.0)`, and take the hypotenuse with `.hypot(...)`.{{sidenote:Notice how `pads_bounding_box_mm` uses `.map(SmdPad::bounds_mm).reduce(Rect::union)`: `Iterator::reduce` folds the iterator using the first pad's `Rect` as the initial accumulator and returns `None` if the board has zero pads.}}

## 2. Detecting DRC Clearance Violations & Clamping the Minimap Camera

When iterating over pad pairs in `detect_pad_clearance_violations`, we maintain a `seen: Vec<&SmdPad>` slice of previously visited pads. That way, each unordered pair `(a, b)` is tested once without index arithmetic (`idx + 1`), and whenever `a.layer == b.layer && a.net != b.net` with `gap < min_clearance_mm`, we record a `DrcViolation` at the midpoint between the two pads.

Meanwhile, when the engineer pans the `egui::Scene` camera near or beyond the board edge `(0, 0)..(100, 70) mm`, `ViewState::clamped_minimap_camera_rect` uses [`f32::clamp`](https://doc.rust-lang.org/std/primitive.f32.html#method.clamp) to keep the minimap's camera box inside `board_bounds_mm`.

## 3. Code Walkthrough: Block by Block

Let's walk through the new `DrcViolation` struct, `pad_edge_gap_mm` geometry math, `BoardModel` DRC/bounding-box methods, `ViewState::clamped_minimap_camera_rect`, UI halo rendering, and tests in `apps/0013-drc-violation-overlay-and-minimap/src/main.rs` block by block before viewing the complete listing in context.

### Block 1: `DrcViolation` & `pad_edge_gap_mm` Euclidean Clearance Math

```rust
/// Design Rule Check (DRC) copper-to-copper clearance violation between two nets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrcViolation {
    pub layer: PcbLayer,
    pub net_a: NetId,
    pub net_b: NetId,
    pub center_mm: Pos2,
    pub measured_gap_mm: f32,
    pub required_gap_mm: f32,
}

impl DrcViolation {
    pub fn summary_label(&self) -> String {
        format!(
            "DRC: {} vs {} ({:.1} mm < {:.1} mm)",
            self.net_a.signal_name(),
            self.net_b.signal_name(),
            self.measured_gap_mm,
            self.required_gap_mm
        )
    }
}

/// Computes the Euclidean edge-to-edge gap (in mm) between two axis-aligned SMD pads.
pub fn pad_edge_gap_mm(a: &SmdPad, b: &SmdPad) -> f32 {
    let dx = (a.center_mm.x - b.center_mm.x).abs() - (a.size_mm.x + b.size_mm.x) * 0.5;
    let dy = (a.center_mm.y - b.center_mm.y).abs() - (a.size_mm.y + b.size_mm.y) * 0.5;
    dx.max(0.0).hypot(dy.max(0.0))
}
```

- **`DrcViolation`**: Value type (`Copy`) storing the offending `PcbLayer`, both `NetId`s, the midpoint `center_mm` between the two pads, `measured_gap_mm`, and `required_gap_mm`.
- **`(a.center_mm.x - b.center_mm.x).abs() - (a.size_mm.x + b.size_mm.x) * 0.5`**: Computes the 1D edge-to-edge separation along X by subtracting the sum of both pads' half-widths from their centre-to-centre distance.
- **`dx.max(0.0).hypot(dy.max(0.0))`**: Clamps negative axis separations (when two pads overlap horizontally or vertically) to `0.0` before computing the 2D Euclidean hypotenuse $\sqrt{\max(dx, 0)^2 + \max(dy, 0)^2}$.

### Block 2: `pads_bounding_box_mm` (`Iterator::reduce`) & `detect_pad_clearance_violations`

```rust
impl BoardModel {
    /// Computes the union bounding box across all pads on the board using `Iterator::reduce`.
    pub fn pads_bounding_box_mm(&self) -> Option<Rect> {
        self.pads
            .iter()
            .map(SmdPad::bounds_mm)
            .reduce(Rect::union)
    }

    /// Detects same-layer, different-net pad pairs whose edge gap is strictly below `min_clearance_mm`.
    pub fn detect_pad_clearance_violations(&self, min_clearance_mm: f32) -> Vec<DrcViolation> {
        let mut violations = Vec::new();
        let mut seen: Vec<&SmdPad> = Vec::new();
        for b in &self.pads {
            for a in &seen {
                if a.layer == b.layer && a.net != b.net {
                    let gap = pad_edge_gap_mm(a, b);
                    if gap < min_clearance_mm {
                        violations.push(DrcViolation {
                            layer: a.layer,
                            net_a: a.net,
                            net_b: b.net,
                            center_mm: pos2(
                                (a.center_mm.x + b.center_mm.x) * 0.5,
                                (a.center_mm.y + b.center_mm.y) * 0.5,
                            ),
                            measured_gap_mm: gap,
                            required_gap_mm: min_clearance_mm,
                        });
                    }
                }
            }
            seen.push(b);
        }
        violations
    }
}
```

- **`.map(SmdPad::bounds_mm).reduce(Rect::union)`**: Passed a function pointer `SmdPad::bounds_mm` (`fn(&SmdPad) -> Rect`) and `Rect::union` (`fn(Rect, Rect) -> Rect`), `Iterator::reduce` folds all pad rectangles into `Some(union_rect)` (or `None` when `self.pads` is empty).
- **`let mut seen: Vec<&SmdPad>`**: Holds shared references `&SmdPad` to pads visited so far, comparing each new pad `b` against every prior pad `a` without index arithmetic (`idx + 1`) that could spawn equivalent index-offset mutants.
- **`if a.layer == b.layer && a.net != b.net` and `if gap < min_clearance_mm`**: Only copper pads on the **same layer** belonging to **different nets** can short-circuit each other, and a violation is raised when `gap` is strictly less than `min_clearance_mm`.

### Block 3: `clamped_minimap_camera_rect` (`f32::clamp`), Inspector Jump & Red DRC Halos

```rust
impl ViewState {
    /// Clamps `pcb_scene_rect` so the minimap viewport indicator stays within `board_bounds_mm`.
    pub fn clamped_minimap_camera_rect(&self, board_bounds_mm: Rect) -> Rect {
        let min_x = self
            .pcb_scene_rect
            .min
            .x
            .clamp(board_bounds_mm.min.x, board_bounds_mm.max.x);
        let min_y = self
            .pcb_scene_rect
            .min
            .y
            .clamp(board_bounds_mm.min.y, board_bounds_mm.max.y);
        let max_x = self
            .pcb_scene_rect
            .max
            .x
            .clamp(min_x, board_bounds_mm.max.x);
        let max_y = self
            .pcb_scene_rect
            .max
            .y
            .clamp(min_y, board_bounds_mm.max.y);
        Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y))
    }
}

// Inside PcbReviewerApp::render_side_panel():
        ui.separator();
        ui.heading("DRC & Minimap");
        let violations = self
            .board
            .detect_pad_clearance_violations(self.view.min_clearance_mm);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.view.show_drc_overlay, "Show DRC Violations");
            ui.label(format!("DRC Violations: {}", violations.len()));
        });
        for v in &violations {
            if ui.button(v.summary_label()).clicked() {
                self.view.pcb_scene_rect = Rect::from_center_size(v.center_mm, vec2(40.0, 30.0));
            }
        }
        let board_bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
        let clamped_cam = self.view.clamped_minimap_camera_rect(board_bounds);
        ui.label(format!(
            "Minimap Camera: ({:.0}, {:.0})..({:.0}, {:.0}) mm",
            clamped_cam.min.x, clamped_cam.min.y, clamped_cam.max.x, clamped_cam.max.y
        ));

// Inside PcbReviewerApp::paint_board():
        if self.view.show_drc_overlay {
            for v in self
                .board
                .detect_pad_clearance_violations(self.view.min_clearance_mm)
            {
                let center_px = self.view.viewport.world_to_screen(v.center_mm);
                painter.circle_stroke(
                    center_px,
                    14.0_f32,
                    Stroke::new(2.0_f32, Color32::from_rgb(255, 85, 85)),
                );
            }
        }
```

- **Chained `f32::clamp` in `clamped_minimap_camera_rect`**: First clamps `min_x` and `min_y` into `board_bounds_mm`, then clamps `max_x` to `min_x..=board_bounds_mm.max.x` (and `max_y` to `min_y..=board_bounds_mm.max.y`) so the returned `Rect` is always valid (`min <= max`) and strictly inside the board outline.
- **Clicking a DRC Violation Button**: Clicking `ui.button(v.summary_label())` in `Panel::left` recentres `self.view.pcb_scene_rect` directly over `v.center_mm` (`40×30 mm` window), immediately updating the `"Minimap Camera: ..."` readout.
- **`painter.circle_stroke(center_px, 14.0_f32, ...)`**: When `self.view.show_drc_overlay` is enabled, draws a `14.0 px` radius red warning halo around each violation midpoint on the 2D canvas.

### Block 4: DRC Geometry, Boundary & Minimap Clamping Tests (`#[cfg(test)] mod tests`)

```rust
    #[test]
    fn drc_clearance_bounding_box_and_minimap_clamping_are_exact() {
        let pad_a = SmdPad {
            center_mm: pos2(10.0, 12.0),
            size_mm: vec2(4.0, 2.0),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        };
        let pad_b = SmdPad {
            center_mm: pos2(18.0, 27.0),
            size_mm: vec2(6.0, 4.0),
            layer: PcbLayer::TopCopper,
            net: NetId(2),
        };
        // dx = |10 - 18| - 5 = 3.0, dy = |12 - 27| - 3 = 12.0 -> hypot(3, 12)
        assert!((pad_edge_gap_mm(&pad_a, &pad_b) - 3.0_f32.hypot(12.0_f32)).abs() < 1e-4);

        let mut board = BoardModel::default();
        assert_eq!(
            board.pads_bounding_box_mm(),
            Some(Rect::from_min_max(pos2(18.0, 20.8), pos2(22.0, 49.2)))
        );

        // At exact boundary 7.6 mm, `gap < min_clearance_mm` must be false
        assert!(board.detect_pad_clearance_violations(7.6).is_empty());

        board.pads.push(SmdPad {
            center_mm: pos2(20.0, 20.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        });
        let violations = board.detect_pad_clearance_violations(8.0);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].center_mm, pos2(20.0, 27.0));
        assert!((violations[0].measured_gap_mm - 7.6).abs() < 1e-4);
        assert_eq!(
            violations[0].summary_label(),
            "DRC: VCC_3V3 vs SPI_CLK (7.6 mm < 8.0 mm)"
        );

        // Minimap camera clamping when panned outside board bounds (0,0)..(100,70)
        let mut view = ViewState::default();
        view.pcb_scene_rect = Rect::from_min_max(pos2(-25.0, -10.0), pos2(130.0, 95.0));
        let board_bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
        assert_eq!(
            view.clamped_minimap_camera_rect(board_bounds),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0))
        );
    }
```

- **Diagonal & Axis-Overlapping Pad Checks**: Testing `pad_a` vs. `pad_b` with non-zero, asymmetric `dx = 3.0` and `dy = 12.0` catches arithmetic mutations in `pad_edge_gap_mm`, while the default board pads test `dx < 0.0` clamped to `0.0` via `.max(0.0)`.
- **Strict Inequality Boundary (`7.6 mm` vs. `8.0 mm`)**: Asserting `detect_pad_clearance_violations(7.6).is_empty()` alongside `detect_pad_clearance_violations(8.0).len() == 1` kills the `<` $\to$ `<=` boundary mutant.

### Complete Lesson 0013 DRC Violation Overlay & Minimap Clamping Listing

Here is the complete assembled DRC geometry engine, `BoardModel` methods, `ViewState` minimap clamping, `PcbReviewerApp` inspector/canvas halo rendering, and tests in context from `apps/0013-drc-violation-overlay-and-minimap/src/main.rs`:

```rust
/// Design Rule Check (DRC) copper-to-copper clearance violation between two nets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrcViolation {
    pub layer: PcbLayer,
    pub net_a: NetId,
    pub net_b: NetId,
    pub center_mm: Pos2,
    pub measured_gap_mm: f32,
    pub required_gap_mm: f32,
}

impl DrcViolation {
    pub fn summary_label(&self) -> String {
        format!(
            "DRC: {} vs {} ({:.1} mm < {:.1} mm)",
            self.net_a.signal_name(),
            self.net_b.signal_name(),
            self.measured_gap_mm,
            self.required_gap_mm
        )
    }
}

/// Computes the Euclidean edge-to-edge gap (in mm) between two axis-aligned SMD pads.
pub fn pad_edge_gap_mm(a: &SmdPad, b: &SmdPad) -> f32 {
    let dx = (a.center_mm.x - b.center_mm.x).abs() - (a.size_mm.x + b.size_mm.x) * 0.5;
    let dy = (a.center_mm.y - b.center_mm.y).abs() - (a.size_mm.y + b.size_mm.y) * 0.5;
    dx.max(0.0).hypot(dy.max(0.0))
}

impl BoardModel {
    // ... find_component, filter_components, review pin methods ...

    /// Computes the union bounding box across all pads on the board using `Iterator::reduce`.
    pub fn pads_bounding_box_mm(&self) -> Option<Rect> {
        self.pads
            .iter()
            .map(SmdPad::bounds_mm)
            .reduce(Rect::union)
    }

    /// Detects same-layer, different-net pad pairs whose edge gap is strictly below `min_clearance_mm`.
    pub fn detect_pad_clearance_violations(&self, min_clearance_mm: f32) -> Vec<DrcViolation> {
        let mut violations = Vec::new();
        let mut seen: Vec<&SmdPad> = Vec::new();
        for b in &self.pads {
            for a in &seen {
                if a.layer == b.layer && a.net != b.net {
                    let gap = pad_edge_gap_mm(a, b);
                    if gap < min_clearance_mm {
                        violations.push(DrcViolation {
                            layer: a.layer,
                            net_a: a.net,
                            net_b: b.net,
                            center_mm: pos2(
                                (a.center_mm.x + b.center_mm.x) * 0.5,
                                (a.center_mm.y + b.center_mm.y) * 0.5,
                            ),
                            measured_gap_mm: gap,
                            required_gap_mm: min_clearance_mm,
                        });
                    }
                }
            }
            seen.push(b);
        }
        violations
    }
}

pub struct ViewState {
    // ... layer booleans, viewport, selection & triage fields ...
    pub min_clearance_mm: f32,
    pub show_drc_overlay: bool,
}

impl ViewState {
    // ... layer & selection methods ...

    /// Clamps `pcb_scene_rect` so the minimap viewport indicator stays within `board_bounds_mm`.
    pub fn clamped_minimap_camera_rect(&self, board_bounds_mm: Rect) -> Rect {
        let min_x = self
            .pcb_scene_rect
            .min
            .x
            .clamp(board_bounds_mm.min.x, board_bounds_mm.max.x);
        let min_y = self
            .pcb_scene_rect
            .min
            .y
            .clamp(board_bounds_mm.min.y, board_bounds_mm.max.y);
        let max_x = self
            .pcb_scene_rect
            .max
            .x
            .clamp(min_x, board_bounds_mm.max.x);
        let max_y = self
            .pcb_scene_rect
            .max
            .y
            .clamp(min_y, board_bounds_mm.max.y);
        Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y))
    }
}

impl PcbReviewerApp {
    pub fn render_side_panel(&mut self, ui: &mut egui::Ui) {
        // ... PCB Layers, BOM Inspector, Review Pins & Triage ...

        ui.separator();
        ui.heading("DRC & Minimap");
        let violations = self
            .board
            .detect_pad_clearance_violations(self.view.min_clearance_mm);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.view.show_drc_overlay, "Show DRC Violations");
            ui.label(format!("DRC Violations: {}", violations.len()));
        });
        for v in &violations {
            if ui.button(v.summary_label()).clicked() {
                self.view.pcb_scene_rect = Rect::from_center_size(v.center_mm, vec2(40.0, 30.0));
            }
        }
        let board_bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
        let clamped_cam = self.view.clamped_minimap_camera_rect(board_bounds);
        ui.label(format!(
            "Minimap Camera: ({:.0}, {:.0})..({:.0}, {:.0}) mm",
            clamped_cam.min.x, clamped_cam.min.y, clamped_cam.max.x, clamped_cam.max.y
        ));

        // ... Net Inspector ...
    }

    fn paint_board(&self, painter: &egui::Painter, canvas_rect: Rect) {
        // ... board outline, traces, pads, silkscreen ...

        if self.view.show_drc_overlay {
            for v in self
                .board
                .detect_pad_clearance_violations(self.view.min_clearance_mm)
            {
                let center_px = self.view.viewport.world_to_screen(v.center_mm);
                painter.circle_stroke(
                    center_px,
                    14.0_f32,
                    Stroke::new(2.0_f32, Color32::from_rgb(255, 85, 85)),
                );
            }
        }

        // ... review pins & ruler overlay ...
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    // ... Lesson 0001-0012 unit tests ...

    #[test]
    fn drc_clearance_bounding_box_and_minimap_clamping_are_exact() {
        let pad_a = SmdPad {
            center_mm: pos2(10.0, 12.0),
            size_mm: vec2(4.0, 2.0),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        };
        let pad_b = SmdPad {
            center_mm: pos2(18.0, 27.0),
            size_mm: vec2(6.0, 4.0),
            layer: PcbLayer::TopCopper,
            net: NetId(2),
        };
        assert!((pad_edge_gap_mm(&pad_a, &pad_b) - 3.0_f32.hypot(12.0_f32)).abs() < 1e-4);

        let mut board = BoardModel::default();
        assert_eq!(
            board.pads_bounding_box_mm(),
            Some(Rect::from_min_max(pos2(18.0, 20.8), pos2(22.0, 49.2)))
        );

        assert!(board.detect_pad_clearance_violations(7.6).is_empty());

        board.pads.push(SmdPad {
            center_mm: pos2(20.0, 20.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        });
        let violations = board.detect_pad_clearance_violations(8.0);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].center_mm, pos2(20.0, 27.0));
        assert!((violations[0].measured_gap_mm - 7.6).abs() < 1e-4);
        assert_eq!(
            violations[0].summary_label(),
            "DRC: VCC_3V3 vs SPI_CLK (7.6 mm < 8.0 mm)"
        );

        let mut view = ViewState::default();
        view.pcb_scene_rect = Rect::from_min_max(pos2(-25.0, -10.0), pos2(130.0, 95.0));
        let board_bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
        assert_eq!(
            view.clamped_minimap_camera_rect(board_bounds),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0))
        );

        view.pcb_scene_rect = Rect::from_min_max(pos2(40.0, 30.0), pos2(20.0, 15.0));
        assert_eq!(
            view.clamped_minimap_camera_rect(board_bounds),
            Rect::from_min_max(pos2(40.0, 30.0), pos2(40.0, 30.0))
        );
    }

    #[test]
    fn headless_drc_violation_button_and_triage_workflow() {
        let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());
        assert!(harness.query_by_label("DRC Violations: 1").is_some());
        assert!(
            harness
                .query_by_label("Minimap Camera: (0, 0)..(100, 70) mm")
                .is_some()
        );

        // Click the DRC violation button -> centers camera on violation midpoint (20, 27)
        harness
            .get_by_label("DRC: VCC_3V3 vs SPI_CLK (7.6 mm < 8.0 mm)")
            .click();
        harness.run();
        assert_eq!(
            harness.state().view.pcb_scene_rect,
            Rect::from_center_size(pos2(20.0, 27.0), vec2(40.0, 30.0))
        );
        assert!(
            harness
                .query_by_label("Minimap Camera: (0, 12)..(40, 42) mm")
                .is_some()
        );
    }
}
```

{{widget:drc-minimap-sim}}

:::exercise id="ex-0013" title="Interactive Lab 2: Complete the Pad Edge Gap, Bounding-Box Reduce & Minimap Clamp" badge="hypot • Iterator::reduce • f32::clamp"
Fill in the three blanks below to (1) clamp negative axis overlaps to `0.0` before calling `.hypot(...)`, (2) fold pad rectangles with `Iterator::reduce(Rect::union)`, and (3) clamp `min_x` within `board_bounds_mm`:

```rust
pub fn pad_edge_gap_mm(a: &SmdPad, b: &SmdPad) -> f32 {
    let dx = (a.center_mm.x - b.center_mm.x).abs() - (a.size_mm.x + b.size_mm.x) * 0.5;
    let dy = (a.center_mm.y - b.center_mm.y).abs() - (a.size_mm.y + b.size_mm.y) * 0.5;
    dx.{{blank:max(0.0)|Clamp negative `dx` overlap to `0.0` using `max(0.0)`.|/* 1: max(0.0) */|105px}}.hypot(dy.max(0.0))
}

impl BoardModel {
    pub fn pads_bounding_box_mm(&self) -> Option<Rect> {
        self.pads
            .iter()
            .map(SmdPad::bounds_mm)
            .{{blank:reduce(Rect::union)|Fold the iterator of `Rect`s into `Option<Rect>` using `reduce(Rect::union)`.|/* 2: reduce */|165px}}
    }
}

// Inside ViewState::clamped_minimap_camera_rect():
let min_x = self
    .pcb_scene_rect
    .min
    .x
    .{{blank:clamp|Use `clamp` to bound `min.x` between `board_bounds_mm.min.x` and `board_bounds_mm.max.x`.|/* 3: clamp */|95px}}(board_bounds_mm.min.x, board_bounds_mm.max.x);
```
:::prompts
hint: **Hints:** (1) `max(0.0)`. (2) `reduce(Rect::union)`. (3) `clamp`.
success: **Great work!** Your DRC geometry engine computes exact edge-to-edge pad gaps, unions pad bounding boxes, and clamps minimap camera coordinates.
:::
:::

:::verify id="verify-0013" title="Interactive Lab 3: Catching `<` vs. `<=` DRC Boundary Mutants" badge="cargo test + cargo mutants" tier="Tier 2 + 6: DRC Geometry & Minimap" caught="125 Caught (20 Unviable)" missed="0 Missed"
If a test only checks `detect_pad_clearance_violations(8.0)` when the actual gap is `7.6 mm`, `cargo mutants` can mutate `if gap < min_clearance_mm` into `if gap <= min_clearance_mm` and the test will still pass! Asserting `board.detect_pad_clearance_violations(7.6).is_empty()` at the exact boundary kills the `<` $\to$ `<=` mutant immediately:

```rust
#[test]
fn drc_clearance_bounding_box_and_minimap_clamping_are_exact() {
    let mut board = BoardModel::default();
    assert_eq!(
        board.pads_bounding_box_mm(),
        Some(Rect::from_min_max(pos2(18.0, 20.8), pos2(22.0, 49.2)))
    );

    // Exact boundary check (gap == 7.6 mm): catches `<` mutated to `<=`!
    assert!(board.detect_pad_clearance_violations(7.6).is_empty());

    let violations = board.detect_pad_clearance_violations(8.0);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].center_mm, pos2(20.0, 27.0));
}
```

- [ ] **1. Run `cargo test` in `apps/0013-drc-violation-overlay-and-minimap`:** Confirm all 23 unit + headless `egui_kittest` tests pass.
- [ ] **2. Run `cargo mutants --in-place`:** Verify that all viable mutants across `pad_edge_gap_mm`, `detect_pad_clearance_violations`, `pads_bounding_box_mm`, and `clamped_minimap_camera_rect` are **Caught** with **0 Missed**.
:::

:::quiz id="quiz-0013" title="Retrieval Practice: DRC Clearance Math, `Iterator::reduce` & `f32::clamp`"
Q: Why does `pad_edge_gap_mm` clamp `dx` and `dy` with `.max(0.0)` before calling `.hypot(...)`?
- [x] Negative axis overlap means zero gap
- [ ] Hypotenuse panics on positive float inputs
- [ ] Copper layers ignore vertical pad spacing
- [ ] Screen transforms invert negative world coordinates
why: When two pads overlap along the X axis (`dx < 0`), their horizontal separation is `0.0 mm` and the edge gap is purely vertical (`dy`).
hint: Think about two pads aligned vertically at `x = 20.0 mm`: what is `|20 - 20| - 4.0`?

Q: How does `pads_bounding_box_mm` compute the union `Rect` across all pads on the board?
- [ ] Sorting pad coordinates by layer rank
- [x] Reducing pad rectangles with Rect union
- [ ] Multiplying viewport scale by board width
- [ ] Filtering hidden silkscreen labels each frame
why: `.map(SmdPad::bounds_mm).reduce(Rect::union)` folds every pad's `Rect` into a single bounding rectangle (or `None` if `pads` is empty).
hint: Look at the iterator method chained after `.map(SmdPad::bounds_mm)`.

Q: Why should a DRC test assert `detect_pad_clearance_violations(7.6).is_empty()` at the exact `7.6 mm` measured pad gap?
- [ ] Skipping floating point math during tests
- [ ] Forcing headless harnesses to open windows
- [x] Catching strict inequality boundary operator mutants
- [ ] Clearing review pins from hash maps
why: Testing `min_clearance_mm == 7.6` ensures that mutating `gap < min_clearance_mm` into `gap <= min_clearance_mm` causes the test to fail.
hint: What happens at `gap == 7.6` if `<` is mutated into `<=`?
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [`std::iter::Iterator::reduce` Official Docs](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.reduce) and [`egui::Rect::union` API Docs](https://docs.rs/egui/latest/egui/struct.Rect.html#method.union).
:::

:::teacher-prompt
**Next Up ([Lesson 0014: Custom `egui::Widget` Layer Swatches, Struct Lifetimes (`'a`) & `Harness::new_ui_state`](./0014-custom-layer-swatch-widget-and-design-tokens.html)):** To wrap up **Act III**, we will extract our first reusable custom `egui::Widget`—a PCB `LayerSwatch<'a>` combining a colour swatch, visibility checkbox, and opacity slider—and test it in isolation with `egui_kittest::Harness::new_ui_state`!
:::

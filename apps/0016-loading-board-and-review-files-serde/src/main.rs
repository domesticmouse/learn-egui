#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, StrokeKind};
pub use pcb_doc::{
    BoardModel, CURRENT_SCHEMA_VERSION, ComponentSymbol, DrcViolation,
    FORBIDDEN_UI_DEPS_BELOW_L6, NetId, PcbLayer, PinId, PinSeverity, Pos2, Rect, ReviewNote,
    ReviewSessionEnvelope, ReviewStatus, RulerMeasurement, SessionIoError, SmdPad, TSTransform,
    TraceSegment, Vec2, pad_edge_gap_mm, pos2, validate_l1_manifest_no_ui_deps, vec2,
};

pub const MIN_ZOOM: f32 = 1.5;
pub const MAX_ZOOM: f32 = 60.0;

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    options.viewport = egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]);

    eframe::run_native(
        "Schematic & PCB Design Reviewer — Lesson 0016 (Serde JSON Session I/O)",
        options,
        Box::new(|_cc| Ok(Box::new(PcbReviewerApp::default()))),
    )
}

pub fn layer_color(layer: PcbLayer) -> Color32 {
    let (r, g, b) = layer.rgb();
    Color32::from_rgb(r, g, b)
}

pub fn layer_color_with_opacity(layer: PcbLayer, opacity: f32) -> Color32 {
    let (r, g, b) = layer.rgb();
    let alpha = (opacity.clamp(0.1, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

pub fn opacity_badge(opacity: f32) -> String {
    format!("{:.0}%", opacity.clamp(0.1, 1.0) * 100.0)
}

pub struct LayerSwatch<'a> {
    pub visible: &'a mut bool,
    pub opacity: &'a mut f32,
    pub label: &'a str,
    pub swatch_color: Color32,
}

impl<'a> LayerSwatch<'a> {
    pub fn new(
        visible: &'a mut bool,
        opacity: &'a mut f32,
        label: &'a str,
        swatch_color: Color32,
    ) -> Self {
        Self {
            visible,
            opacity,
            label,
            swatch_color,
        }
    }
}

impl<'a> egui::Widget for LayerSwatch<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let badge = opacity_badge(*self.opacity);
        ui.horizontal(|ui| {
            let (swatch_rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter()
                .rect_filled(swatch_rect, 2.0_f32, self.swatch_color);
            let check_resp = ui.checkbox(self.visible, self.label);
            let slider_resp = ui.add(
                egui::Slider::new(self.opacity, 0.1..=1.0)
                    .show_value(false)
                    .text(badge),
            );
            check_resp.union(slider_resp)
        })
        .inner
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveTool {
    Select,
    Measure {
        start_mm: Option<Pos2>,
        end_mm: Option<Pos2>,
    },
    PlacePin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardViewport {
    pub transform: TSTransform,
}

impl Default for BoardViewport {
    fn default() -> Self {
        Self {
            transform: TSTransform::new(vec2(80.0, 60.0), 6.0),
        }
    }
}

impl BoardViewport {
    pub fn world_to_screen(&self, world_mm: Pos2) -> Pos2 {
        self.transform * world_mm
    }

    pub fn screen_to_world(&self, screen_pt: Pos2) -> Pos2 {
        self.transform.inverse() * screen_pt
    }

    pub fn pan_by(&mut self, screen_delta: Vec2) {
        self.transform.translation += screen_delta;
    }

    pub fn zoom_around(&mut self, screen_anchor: Pos2, zoom_factor: f32) {
        let world_anchor = self.screen_to_world(screen_anchor);
        let new_scale = (self.transform.scaling * zoom_factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.transform.scaling = new_scale;
        self.transform.translation =
            screen_anchor.to_vec2() - world_anchor.to_vec2() * new_scale;
    }
}

pub struct ViewState {
    pub show_top_copper: bool,
    pub show_bottom_copper: bool,
    pub show_silkscreen: bool,
    pub show_ratsnest: bool,
    pub top_copper_opacity: f32,
    pub bottom_copper_opacity: f32,
    pub silkscreen_opacity: f32,
    pub ratsnest_opacity: f32,
    pub viewport: BoardViewport,
    pub pcb_scene_rect: Rect,
    pub selected_component: Option<String>,
    pub hovered_net: Option<NetId>,
    pub selected_net: Option<NetId>,
    pub bom_search_query: String,
    pub active_tool: ActiveTool,
    pub selected_pin: Option<PinId>,
    pub min_pin_severity: PinSeverity,
    pub filter_only_open_pins: bool,
    pub min_clearance_mm: f32,
    pub show_drc_overlay: bool,
    pub active_board_id: String,
    pub last_io_error: Option<String>,
}

impl Default for ViewState {
    fn default() -> Self {
        let mut view = Self {
            show_top_copper: false,
            show_bottom_copper: false,
            show_silkscreen: false,
            show_ratsnest: false,
            top_copper_opacity: 0.0,
            bottom_copper_opacity: 0.0,
            silkscreen_opacity: 0.0,
            ratsnest_opacity: 0.0,
            viewport: BoardViewport::default(),
            pcb_scene_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0)),
            selected_component: None,
            hovered_net: None,
            selected_net: None,
            bom_search_query: String::new(),
            active_tool: ActiveTool::Select,
            selected_pin: None,
            min_pin_severity: PinSeverity::Info,
            filter_only_open_pins: false,
            min_clearance_mm: 8.0,
            show_drc_overlay: true,
            active_board_id: "pwr-stage-rev-b".to_owned(),
            last_io_error: None,
        };
        view.reset_layers();
        view
    }
}

impl ViewState {
    pub fn reset_layers(&mut self) {
        self.show_top_copper = true;
        self.show_bottom_copper = true;
        self.show_silkscreen = true;
        self.show_ratsnest = false;
        self.top_copper_opacity = 1.0;
        self.bottom_copper_opacity = 1.0;
        self.silkscreen_opacity = 1.0;
        self.ratsnest_opacity = 1.0;
    }

    pub fn isolate_top_copper(&mut self) {
        self.show_top_copper = true;
        self.show_bottom_copper = false;
        self.show_silkscreen = false;
        self.show_ratsnest = false;
    }

    pub fn show_all_layers(&mut self) {
        self.show_top_copper = true;
        self.show_bottom_copper = true;
        self.show_silkscreen = true;
        self.show_ratsnest = true;
    }

    pub fn layer_opacity(&self, layer: PcbLayer) -> f32 {
        match layer {
            PcbLayer::BottomCopper => self.bottom_copper_opacity,
            PcbLayer::TopCopper => self.top_copper_opacity,
            PcbLayer::Silkscreen => self.silkscreen_opacity,
            PcbLayer::Ratsnest => self.ratsnest_opacity,
        }
    }

    pub fn effective_layer_color(&self, layer: PcbLayer) -> Color32 {
        layer_color_with_opacity(layer, self.layer_opacity(layer))
    }

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

    pub fn activate_pin_tool(&mut self) {
        self.active_tool = ActiveTool::PlacePin;
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
        self.selected_pin = None;
    }

    pub fn clear_bom_query(&mut self) {
        self.bom_search_query.clear();
    }

    pub fn dismiss_io_error(&mut self) {
        self.last_io_error = None;
    }

    pub fn is_layer_visible(&self, layer: PcbLayer) -> bool {
        match layer {
            PcbLayer::BottomCopper => self.show_bottom_copper,
            PcbLayer::TopCopper => self.show_top_copper,
            PcbLayer::Silkscreen => self.show_silkscreen,
            PcbLayer::Ratsnest => self.show_ratsnest,
        }
    }

    pub fn active_net(&self) -> Option<NetId> {
        self.selected_net.or(self.hovered_net)
    }

    pub fn focus_component(&mut self, board: &BoardModel, ref_des: &str) -> bool {
        if let Some(comp) = board.find_component(ref_des) {
            self.selected_component = Some(comp.ref_des.clone());
            self.selected_net = Some(comp.primary_net);
            self.pcb_scene_rect = Rect::from_center_size(comp.center_mm, vec2(40.0, 30.0));
            true
        } else {
            false
        }
    }

    pub fn select_review_pin(&mut self, board: &BoardModel, id: PinId) -> bool {
        if let Some(note) = board.review_pins.get(&id) {
            self.selected_pin = Some(id);
            self.pcb_scene_rect = Rect::from_center_size(note.position_mm, vec2(40.0, 30.0));
            true
        } else {
            false
        }
    }

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

#[derive(Default)]
pub struct PcbReviewerApp {
    pub board: BoardModel,
    pub view: ViewState,
    pub saved_session_json: String,
}

impl PcbReviewerApp {
    /// Serializes `self.board` into `self.saved_session_json`, recording any error in `view.last_io_error`.
    pub fn export_session_snapshot(&mut self) -> bool {
        match self.board.to_session_json(&self.view.active_board_id) {
            Ok(json) => {
                self.saved_session_json = json;
                self.view.last_io_error = None;
                true
            }
            Err(err) => {
                self.view.last_io_error = Some(err.to_string());
                false
            }
        }
    }

    /// Loads `raw` JSON into `self.board`, or surfaces a clean error banner without panicking.
    pub fn import_session_from_json(&mut self, raw: &str) -> bool {
        match BoardModel::from_session_json(raw) {
            Ok((board_id, board)) => {
                self.view.active_board_id = board_id;
                self.board = board;
                self.view.last_io_error = None;
                true
            }
            Err(err) => {
                self.view.last_io_error = Some(err.to_string());
                false
            }
        }
    }

    pub fn visible_traces_in_z_order(&self) -> Vec<TraceSegment> {
        let mut visible: Vec<TraceSegment> = self
            .board
            .traces
            .iter()
            .copied()
            .filter(|t| self.view.is_layer_visible(t.layer))
            .collect();
        visible.sort_by_key(|t| t.layer.z_rank());
        visible
    }

    pub fn hit_test_net_at_mm(&self, world_mm: Pos2) -> Option<NetId> {
        let mut matching_pads: Vec<SmdPad> = self
            .board
            .pads
            .iter()
            .copied()
            .filter(|p| self.view.is_layer_visible(p.layer) && p.contains_world_point(world_mm))
            .collect();
        matching_pads.sort_by_key(|p| p.layer.z_rank());
        matching_pads.last().map(|p| p.net)
    }

    pub fn handle_hotkeys(&mut self, ctx: &egui::Context) {
        let pressed_f = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F));
        let pressed_m = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M));
        let pressed_p = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P));
        let pressed_esc =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

        if pressed_f {
            self.view.fit_board_in_viewport();
        }
        if pressed_m {
            self.view.activate_measure_tool();
        }
        if pressed_p {
            self.view.activate_pin_tool();
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
                ActiveTool::PlacePin => {
                    ui.label("Active Tool: Place Pin (P)");
                }
            }
            if let Some(measurement) = self.view.current_measurement() {
                ui.label(measurement.summary_label());
            }
        });
    }

    pub fn render_session_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Session Persistence (JSON)");
        ui.label(format!("Board ID: {}", self.view.active_board_id));
        if ui.button("Export JSON").clicked() {
            self.export_session_snapshot();
        }
        if ui.button("Reload JSON").clicked() {
            let raw = self.saved_session_json.clone();
            self.import_session_from_json(&raw);
        }
        if ui.button("Test Corrupt JSON").clicked() {
            self.import_session_from_json("{corrupt");
        }
        if let Some(err) = self.view.last_io_error.clone() {
            ui.label(format!("I/O Error: {err}"));
            if ui.button("Dismiss Error").clicked() {
                self.view.dismiss_io_error();
            }
        }
    }

    pub fn render_side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("PCB Layers");
        ui.add(LayerSwatch::new(
            &mut self.view.show_top_copper,
            &mut self.view.top_copper_opacity,
            "Top Copper (F.Cu)",
            layer_color(PcbLayer::TopCopper),
        ));
        ui.add(LayerSwatch::new(
            &mut self.view.show_bottom_copper,
            &mut self.view.bottom_copper_opacity,
            "Bottom Copper (B.Cu)",
            layer_color(PcbLayer::BottomCopper),
        ));
        ui.add(LayerSwatch::new(
            &mut self.view.show_silkscreen,
            &mut self.view.silkscreen_opacity,
            "Silkscreen (F.SilkS)",
            layer_color(PcbLayer::Silkscreen),
        ));
        ui.add(LayerSwatch::new(
            &mut self.view.show_ratsnest,
            &mut self.view.ratsnest_opacity,
            "Ratsnest (Unrouted)",
            layer_color(PcbLayer::Ratsnest),
        ));

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Isolate Top Copper").clicked() {
                self.view.isolate_top_copper();
            }
            if ui.button("Show All Layers").clicked() {
                self.view.show_all_layers();
            }
        });

        ui.separator();
        ui.heading("BOM & Netlist Inspector");
        ui.horizontal(|ui| {
            ui.label("Filter BOM:");
            ui.text_edit_singleline(&mut self.view.bom_search_query);
            if ui.button("Clear Filter").clicked() {
                self.view.clear_bom_query();
            }
        });

        if self.view.show_silkscreen {
            let matching = self.board.filter_components(&self.view.bom_search_query);
            ui.label(format!("Matching Components: {}", matching.len()));
            egui::ScrollArea::vertical()
                .max_height(48.0)
                .show_rows(ui, 20.0, matching.len(), |ui, row_range| {
                    egui::Grid::new("bom_inspector_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            for comp in &matching[row_range] {
                                ui.label(comp.primary_net.signal_name());
                                if ui.button(comp.inspector_label()).clicked() {
                                    self.view.focus_component(&self.board, &comp.ref_des);
                                }
                                ui.end_row();
                            }
                        });
                });
        } else {
            ui.label("Enable Silkscreen to inspect component RefDes.");
        }

        ui.separator();
        ui.heading("Review Pins & Triage");
        ui.checkbox(&mut self.view.filter_only_open_pins, "Open Pins Only");
        ui.horizontal(|ui| {
            ui.label("Min Severity:");
            for &sev in PinSeverity::ALL {
                ui.selectable_value(&mut self.view.min_pin_severity, sev, sev.label());
            }
        });

        let pins = self
            .board
            .filter_review_pins(self.view.min_pin_severity, self.view.filter_only_open_pins);
        ui.label(format!("Review Pins: {}", pins.len()));
        for pin in &pins {
            if ui.button(pin.summary_line()).clicked() {
                self.view.select_review_pin(&self.board, pin.id);
            }
        }

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

        ui.separator();
        ui.heading("Net Inspector");
        if let Some(net) = self.view.active_net() {
            ui.label(format!("Active Net: {} ({})", net.code(), net.signal_name()));
            if ui.button("Clear Selection").clicked() {
                self.view.cancel_tool_and_selection();
            }
        } else {
            ui.label("Hover or click a pad to cross-probe its net.");
        }
    }

    pub fn render_active_pin_callout(&mut self, ctx: &egui::Context) {
        let Some(pin_id) = self.view.selected_pin else {
            return;
        };
        let Some(pin) = self.board.review_pins.get(&pin_id).cloned() else {
            return;
        };

        egui::Window::new("Review Callout")
            .id(egui::Id::new(("review_pin_callout", pin.id.0)))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(pin.summary_line());
                ui.label(pin.coordinates_label());
                ui.horizontal(|ui| {
                    if ui.button("Mark Resolved").clicked() {
                        self.board
                            .transition_pin_status(pin.id, ReviewStatus::Resolved);
                    }
                    if ui.button("Mark Verified").clicked() {
                        self.board
                            .transition_pin_status(pin.id, ReviewStatus::Verified);
                    }
                    if ui.button("Reopen Pin").clicked() {
                        self.board.transition_pin_status(pin.id, ReviewStatus::Open);
                    }
                });
                if ui.button("Close Callout").clicked() {
                    self.view.selected_pin = None;
                }
            });
    }

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
                            ActiveTool::PlacePin => {
                                let new_id = self.board.add_review_pin_at(
                                    world_mm,
                                    "reviewer",
                                    "Inspect clearance",
                                    PinSeverity::Warning,
                                );
                                self.view.selected_pin = Some(new_id);
                                self.view.active_tool = ActiveTool::Select;
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
        painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(12, 16, 14));

        let board_min = self.view.viewport.world_to_screen(pos2(0.0, 0.0));
        let board_max = self.view.viewport.world_to_screen(pos2(100.0, 70.0));
        let board_rect = Rect::from_min_max(board_min, board_max);
        let scale = self.view.viewport.transform.scaling;
        let active = self.view.active_net();

        painter.rect(
            board_rect,
            1.5 * scale,
            Color32::from_rgb(16, 40, 30),
            Stroke::new((0.4 * scale).max(1.0), Color32::from_rgb(47, 111, 82)),
            StrokeKind::Inside,
        );

        for seg in &self.visible_traces_in_z_order() {
            let p0 = self.view.viewport.world_to_screen(seg.start);
            let p1 = self.view.viewport.world_to_screen(seg.end);
            let is_highlighted = active.is_some() && seg.net == active;
            let color = if is_highlighted {
                Color32::from_rgb(255, 230, 100)
            } else {
                self.view.effective_layer_color(seg.layer)
            };
            let stroke_px = (seg.width_mm * scale).max(1.5) * if is_highlighted { 1.4 } else { 1.0 };
            painter.line_segment([p0, p1], Stroke::new(stroke_px, color));
        }

        for pad in &self.board.pads {
            if !self.view.is_layer_visible(pad.layer) {
                continue;
            }
            let bounds = pad.bounds_mm();
            let screen_rect = Rect::from_min_max(
                self.view.viewport.world_to_screen(bounds.min),
                self.view.viewport.world_to_screen(bounds.max),
            );
            let is_highlighted = active == Some(pad.net);
            let fill = if is_highlighted {
                Color32::from_rgb(255, 230, 100)
            } else {
                self.view.effective_layer_color(pad.layer)
            };
            painter.rect_filled(screen_rect, 0.3 * scale, fill);
        }

        if self.view.show_silkscreen {
            let silk_color = self.view.effective_layer_color(PcbLayer::Silkscreen);
            for comp in &self.board.components {
                let body_mm = Rect::from_center_size(comp.center_mm, comp.body_size_mm);
                let screen_body = Rect::from_min_max(
                    self.view.viewport.world_to_screen(body_mm.min),
                    self.view.viewport.world_to_screen(body_mm.max),
                );
                painter.rect(
                    screen_body,
                    0.4 * scale,
                    Color32::from_rgba_unmultiplied(240, 230, 140, 14),
                    Stroke::new((0.25 * scale).max(1.0), silk_color),
                    StrokeKind::Inside,
                );
                let center_px = self.view.viewport.world_to_screen(comp.center_mm);
                painter.text(
                    center_px,
                    Align2::CENTER_CENTER,
                    &comp.ref_des,
                    FontId::monospace((2.2 * scale).clamp(9.0, 22.0)),
                    silk_color,
                );
            }
        }

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

        for pin in self
            .board
            .filter_review_pins(self.view.min_pin_severity, self.view.filter_only_open_pins)
        {
            let screen_pt = self.view.viewport.world_to_screen(pin.position_mm);
            let is_selected = self.view.selected_pin == Some(pin.id);
            let fill = if is_selected {
                Color32::from_rgb(255, 230, 100)
            } else {
                Color32::from_rgb(255, 139, 107)
            };
            painter.circle_filled(screen_pt, 6.0_f32, fill);
            painter.text(
                screen_pt + vec2(8.0, -6.0),
                Align2::LEFT_BOTTOM,
                format!("#{}", pin.id.0),
                FontId::monospace(11.0),
                fill,
            );
        }

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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_hotkeys(ctx);

        egui::TopBottomPanel::top("schematic_cross_probe_bar").show(ctx, |ui| {
            self.render_schematic_bar(ui);
        });

        egui::SidePanel::left("layer_panel").show(ctx, |ui| {
            self.render_side_panel(ui);
        });

        egui::SidePanel::right("session_panel").show(ctx, |ui| {
            self.render_session_panel(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });

        self.render_active_pin_callout(ctx);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    #[test]
    fn default_enables_physical_layers_and_resets_opacities_to_full() {
        let mut app = PcbReviewerApp::default();
        assert!(app.view.show_top_copper);
        assert!(app.view.show_bottom_copper);
        assert!(app.view.show_silkscreen);
        assert!(!app.view.show_ratsnest);
        assert_eq!(app.view.top_copper_opacity, 1.0);
        assert_eq!(app.view.bottom_copper_opacity, 1.0);
        assert_eq!(app.view.silkscreen_opacity, 1.0);
        assert_eq!(app.view.ratsnest_opacity, 1.0);

        app.view.top_copper_opacity = 0.25;
        app.view.bottom_copper_opacity = 0.25;
        app.view.silkscreen_opacity = 0.25;
        app.view.ratsnest_opacity = 0.25;
        app.view.reset_layers();
        assert_eq!(app.view.top_copper_opacity, 1.0);
        assert_eq!(app.view.bottom_copper_opacity, 1.0);
        assert_eq!(app.view.silkscreen_opacity, 1.0);
        assert_eq!(app.view.ratsnest_opacity, 1.0);
    }

    #[test]
    fn isolate_top_copper_and_show_all_layers_toggle_visibility_flags() {
        let mut app = PcbReviewerApp::default();
        app.view.show_top_copper = false;
        app.view.show_ratsnest = true;

        app.view.isolate_top_copper();
        assert!(app.view.show_top_copper);
        assert!(!app.view.show_bottom_copper);
        assert!(!app.view.show_silkscreen);
        assert!(!app.view.show_ratsnest);

        app.view.show_top_copper = false;
        app.view.show_all_layers();
        assert!(app.view.show_top_copper);
        assert!(app.view.show_bottom_copper);
        assert!(app.view.show_silkscreen);
        assert!(app.view.show_ratsnest);
    }

    #[test]
    fn viewport_transforms_pan_and_zoom_are_exact() {
        let mut app = PcbReviewerApp::default();
        let world_pad = pos2(25.0, 40.0);
        let screen_pt = app.view.viewport.world_to_screen(world_pad);
        assert_eq!(screen_pt, pos2(230.0, 300.0));

        let round_trip = app.view.viewport.screen_to_world(screen_pt);
        assert!((round_trip.x - world_pad.x).abs() < 1e-4);
        assert!((round_trip.y - world_pad.y).abs() < 1e-4);

        let before = app.view.viewport.world_to_screen(pos2(10.0, 10.0));
        app.view.viewport.pan_by(vec2(15.0, -25.0));
        app.view.pcb_scene_rect = Rect::from_min_max(pos2(5.0, 5.0), pos2(25.0, 25.0));
        assert_eq!(
            app.view.viewport.world_to_screen(pos2(10.0, 10.0)),
            before + vec2(15.0, -25.0)
        );

        app.view.fit_board_in_viewport();
        assert_eq!(app.view.viewport, BoardViewport::default());
        assert_eq!(
            app.view.pcb_scene_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0))
        );

        let mut vp = BoardViewport::default();
        let cursor_screen = pos2(200.0, 180.0);
        let world_before = vp.screen_to_world(cursor_screen);
        vp.zoom_around(cursor_screen, 2.0);
        assert_eq!(vp.transform.scaling, 12.0);
        let world_after = vp.screen_to_world(cursor_screen);
        assert!((world_after.x - world_before.x).abs() < 1e-4);
        assert!((world_after.y - world_before.y).abs() < 1e-4);

        vp.zoom_around(cursor_screen, 100.0);
        assert_eq!(vp.transform.scaling, MAX_ZOOM);
        vp.zoom_around(cursor_screen, 0.001);
        assert_eq!(vp.transform.scaling, MIN_ZOOM);
    }

    #[test]
    fn layer_colors_opacities_visibility_and_hit_testing_bridge_doc_and_ui() {
        assert_eq!(
            layer_color(PcbLayer::BottomCopper),
            Color32::from_rgb(59, 142, 234)
        );
        assert_eq!(
            layer_color(PcbLayer::TopCopper),
            Color32::from_rgb(224, 82, 67)
        );
        assert_eq!(
            layer_color(PcbLayer::Silkscreen),
            Color32::from_rgb(240, 230, 140)
        );
        assert_eq!(
            layer_color(PcbLayer::Ratsnest),
            Color32::from_rgb(111, 247, 255)
        );

        assert_eq!(
            layer_color_with_opacity(PcbLayer::TopCopper, 0.0),
            Color32::from_rgba_unmultiplied(224, 82, 67, 26)
        );
        assert_eq!(
            layer_color_with_opacity(PcbLayer::TopCopper, 0.5),
            Color32::from_rgba_unmultiplied(224, 82, 67, 128)
        );
        assert_eq!(
            layer_color_with_opacity(PcbLayer::TopCopper, 2.0),
            Color32::from_rgba_unmultiplied(224, 82, 67, 255)
        );

        assert_eq!(opacity_badge(0.0), "10%");
        assert_eq!(opacity_badge(0.75), "75%");
        assert_eq!(opacity_badge(1.5), "100%");

        let mut app = PcbReviewerApp::default();
        app.view.bottom_copper_opacity = 0.4;
        app.view.top_copper_opacity = 0.5;
        app.view.silkscreen_opacity = 0.6;
        app.view.ratsnest_opacity = 0.8;
        assert_eq!(app.view.layer_opacity(PcbLayer::BottomCopper), 0.4);
        assert_eq!(app.view.layer_opacity(PcbLayer::TopCopper), 0.5);
        assert_eq!(app.view.layer_opacity(PcbLayer::Silkscreen), 0.6);
        assert_eq!(app.view.layer_opacity(PcbLayer::Ratsnest), 0.8);
        assert_eq!(
            app.view.effective_layer_color(PcbLayer::TopCopper),
            Color32::from_rgba_unmultiplied(224, 82, 67, 128)
        );

        app.view.show_top_copper = false;
        app.view.show_bottom_copper = false;
        app.view.show_silkscreen = false;
        app.view.show_ratsnest = false;
        assert!(!app.view.is_layer_visible(PcbLayer::BottomCopper));
        assert!(!app.view.is_layer_visible(PcbLayer::TopCopper));
        assert!(!app.view.is_layer_visible(PcbLayer::Silkscreen));
        assert!(!app.view.is_layer_visible(PcbLayer::Ratsnest));

        app.view.show_bottom_copper = true;
        assert!(app.view.is_layer_visible(PcbLayer::BottomCopper));
        assert!(!app.view.is_layer_visible(PcbLayer::TopCopper));

        app.view.show_bottom_copper = false;
        app.view.show_top_copper = true;
        assert!(app.view.is_layer_visible(PcbLayer::TopCopper));
        assert!(!app.view.is_layer_visible(PcbLayer::Silkscreen));

        app.view.show_top_copper = false;
        app.view.show_silkscreen = true;
        assert!(app.view.is_layer_visible(PcbLayer::Silkscreen));
        assert!(!app.view.is_layer_visible(PcbLayer::Ratsnest));

        app.view.show_silkscreen = false;
        app.view.show_ratsnest = true;
        assert!(app.view.is_layer_visible(PcbLayer::Ratsnest));
        assert!(!app.view.is_layer_visible(PcbLayer::BottomCopper));

        app.view.reset_layers();
        let default_layers: Vec<PcbLayer> = app
            .visible_traces_in_z_order()
            .iter()
            .map(|t| t.layer)
            .collect();
        assert_eq!(
            default_layers,
            vec![
                PcbLayer::BottomCopper,
                PcbLayer::BottomCopper,
                PcbLayer::TopCopper,
                PcbLayer::TopCopper,
                PcbLayer::Silkscreen,
            ]
        );

        assert_eq!(app.hit_test_net_at_mm(pos2(20.5, 22.2)), Some(NetId(1)));
        assert_eq!(app.hit_test_net_at_mm(pos2(80.0, 60.0)), None);
        app.view.show_top_copper = false;
        assert_eq!(app.hit_test_net_at_mm(pos2(20.5, 22.2)), None);
        app.view.show_top_copper = true;
        app.board.pads.push(SmdPad {
            center_mm: pos2(20.0, 48.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(99),
        });
        assert_eq!(app.hit_test_net_at_mm(pos2(20.0, 48.0)), Some(NetId(99)));
    }

    #[test]
    fn view_state_selection_focus_measurement_and_session_io_work() {
        let board = BoardModel::default();
        let mut view = ViewState::default();

        assert_eq!(view.active_net(), None);
        view.hovered_net = Some(NetId(2));
        assert_eq!(view.active_net(), Some(NetId(2)));
        view.selected_net = Some(NetId(1));
        assert_eq!(view.active_net(), Some(NetId(1)));

        assert!(view.focus_component(&board, "R12"));
        assert_eq!(view.selected_component.as_deref(), Some("R12"));
        assert_eq!(view.selected_net, Some(NetId(2)));
        assert!(!view.focus_component(&board, "U999"));

        assert!(view.select_review_pin(&board, PinId(2)));
        assert_eq!(view.selected_pin, Some(PinId(2)));
        assert!(!view.select_review_pin(&board, PinId(999)));

        assert_eq!(view.current_measurement(), None);
        view.activate_measure_tool();
        assert_eq!(view.current_measurement(), None);
        view.set_measure_points(pos2(20.0, 22.0), pos2(68.0, 58.0));
        assert_eq!(view.current_measurement().unwrap().distance_mm, 60.0);

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

        let mut app = PcbReviewerApp::default();
        app.view.active_board_id = "   ".to_owned();
        assert!(!app.export_session_snapshot());
        assert!(app.view.last_io_error.is_some());

        app.view.active_board_id = "sensor-rev-c".to_owned();
        assert!(app.export_session_snapshot());
        assert!(app.view.last_io_error.is_none());

        app.board.review_pins.clear();
        assert!(app.board.review_pins.is_empty());
        let snapshot = app.saved_session_json.clone();
        assert!(app.import_session_from_json(&snapshot));
        assert_eq!(app.board.review_pins.len(), 2);

        assert!(!app.import_session_from_json("{ bad json"));
        assert!(app.view.last_io_error.is_some());
        app.view.dismiss_io_error();
        assert!(app.view.last_io_error.is_none());
    }

    #[test]
    fn standalone_layer_swatch_and_headless_ui_workflows_pass() {
        struct SwatchFixture {
            visible: bool,
            opacity: f32,
        }

        let mut swatch_harness = Harness::new_ui_state(
            |ui, state: &mut SwatchFixture| {
                ui.add(LayerSwatch::new(
                    &mut state.visible,
                    &mut state.opacity,
                    "Top Copper (F.Cu)",
                    layer_color(PcbLayer::TopCopper),
                ));
            },
            SwatchFixture {
                visible: true,
                opacity: 0.75,
            },
        );
        assert!(swatch_harness.query_by_label("75%").is_some());
        swatch_harness.get_by_label("Top Copper (F.Cu)").click();
        swatch_harness.run();
        assert!(!swatch_harness.state().visible);

        let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());
        assert!(harness.query_by_label("U1: LDO_3V3 (SOT-23)").is_some());

        harness.get_by_label("Export JSON").click();
        harness.run();
        assert!(!harness.state().saved_session_json.is_empty());

        harness.state_mut().board.review_pins.clear();
        harness.run();
        assert!(harness.query_by_label("Review Pins: 0").is_some());

        harness.get_by_label("Reload JSON").click();
        harness.run();
        assert!(harness.query_by_label("Review Pins: 2").is_some());

        harness.get_by_label("Test Corrupt JSON").click();
        harness.run();
        assert!(harness.state().view.last_io_error.is_some());

        harness.get_by_label("Dismiss Error").click();
        harness.run();
        assert!(harness.state().view.last_io_error.is_none());

        harness.get_by_label("Isolate Top Copper").click();
        harness.run();
        assert!(!harness.state().view.show_bottom_copper);
        assert!(harness.query_by_label("U1: LDO_3V3 (SOT-23)").is_none());

        harness.get_by_label("Show All Layers").click();
        harness.run();
        assert!(harness.state().view.show_ratsnest);

        harness.get_by_label("Probe R12 (10k)").click();
        harness.run();
        assert_eq!(
            harness.state().view.selected_component.as_deref(),
            Some("R12")
        );

        harness.state_mut().view.bom_search_query = "j1".to_owned();
        harness.run();
        harness.get_by_label("J1: CONN_2P (Header)").click();
        harness.run();
        assert_eq!(
            harness.state().view.selected_component.as_deref(),
            Some("J1")
        );

        harness.get_by_label("Clear Filter").click();
        harness.run();
        assert_eq!(harness.state().view.bom_search_query, "");

        harness.get_by_label("Clear Selection").click();
        harness.run();
        assert_eq!(harness.state().view.selected_net, None);

        harness
            .get_by_label("DRC: VCC_3V3 vs SPI_CLK (7.6 mm < 8.0 mm)")
            .click();
        harness.run();
        assert!(
            harness
                .query_by_label("Minimap Camera: (0, 12)..(40, 42) mm")
                .is_some()
        );

        harness.get_by_label("Critical").click();
        harness.run();
        assert_eq!(harness.state().view.min_pin_severity, PinSeverity::Critical);
        harness.get_by_label("Info").click();
        harness
            .get_by_label("Pin #1 [Critical | Open]: Check U1 thermal relief (@alice)")
            .click();
        harness.run();
        harness.get_by_label("Mark Resolved").click();
        harness.run();
        harness.get_by_label("Mark Verified").click();
        harness.run();
        harness.get_by_label("Reopen Pin").click();
        harness.run();
        harness.get_by_label("Close Callout").click();
        harness.run();
        assert_eq!(harness.state().view.selected_pin, None);

        harness.key_press(egui::Key::P);
        harness.run();
        assert!(harness.query_by_label("Active Tool: Place Pin (P)").is_some());

        harness.key_press(egui::Key::M);
        harness.run();
        assert!(harness.query_by_label("Active Tool: Measure (M)").is_some());

        harness
            .state_mut()
            .view
            .set_measure_points(pos2(20.0, 22.0), pos2(68.0, 58.0));
        harness.run();
        assert!(
            harness
                .query_by_label("Ruler: dx=48.0 mm, dy=36.0 mm, dist=60.0 mm")
                .is_some()
        );

        harness.key_press(egui::Key::Escape);
        harness.run();
        assert_eq!(harness.state().view.active_tool, ActiveTool::Select);

        harness.state_mut().view.viewport.pan_by(vec2(45.0, -30.0));
        harness.key_press(egui::Key::F);
        harness.run();
        assert_eq!(harness.state().view.viewport, BoardViewport::default());
    }
}

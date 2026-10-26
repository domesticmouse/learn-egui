use std::collections::{HashMap, hash_map::Entry};

use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, emath::TSTransform,
    pos2, vec2,
};

pub const MIN_ZOOM: f32 = 1.5;
pub const MAX_ZOOM: f32 = 60.0;

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    options.viewport = egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]);

    eframe::run_native(
        "Schematic & PCB Design Reviewer — Lesson 0012",
        options,
        Box::new(|_cc| Ok(Box::new(PcbReviewerApp::default()))),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetId(pub u32);

impl NetId {
    pub fn code(self) -> String {
        format!("NET_{}", self.0)
    }

    pub fn signal_name(self) -> &'static str {
        match self.0 {
            1 => "VCC_3V3",
            2 => "SPI_CLK",
            _ => "UNASSIGNED",
        }
    }
}

/// Unique identifier for an engineering review pin anchored to the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PinId(pub u32);

/// Severity level assigned to an engineering review pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PinSeverity {
    Info,
    Warning,
    Critical,
}

impl PinSeverity {
    pub const ALL: &'static [PinSeverity] = &[
        PinSeverity::Info,
        PinSeverity::Warning,
        PinSeverity::Critical,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PinSeverity::Info => "Info",
            PinSeverity::Warning => "Warning",
            PinSeverity::Critical => "Critical",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            PinSeverity::Info => 0,
            PinSeverity::Warning => 1,
            PinSeverity::Critical => 2,
        }
    }
}

/// Engineering review lifecycle state machine:
/// `Open` -> `Resolved` -> `Verified` (with ability to reopen to `Open`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewStatus {
    Open,
    Resolved,
    Verified,
}

impl ReviewStatus {
    pub const ALL: &'static [ReviewStatus] = &[
        ReviewStatus::Open,
        ReviewStatus::Resolved,
        ReviewStatus::Verified,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ReviewStatus::Open => "Open",
            ReviewStatus::Resolved => "Resolved",
            ReviewStatus::Verified => "Verified",
        }
    }

    /// Enforces sign-off discipline: an `Open` pin cannot jump straight to `Verified`
    /// without first being `Resolved`.
    pub fn can_transition_to(self, next: ReviewStatus) -> bool {
        matches!(
            (self, next),
            (ReviewStatus::Open, ReviewStatus::Resolved)
                | (ReviewStatus::Resolved, ReviewStatus::Verified)
                | (ReviewStatus::Resolved, ReviewStatus::Open)
                | (ReviewStatus::Verified, ReviewStatus::Open)
        )
    }
}

/// Engineering review note anchored to a physical `(x, y)` coordinate in mm.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewNote {
    pub id: PinId,
    pub position_mm: Pos2,
    pub author: String,
    pub comment: String,
    pub severity: PinSeverity,
    pub status: ReviewStatus,
}

impl ReviewNote {
    pub fn summary_line(&self) -> String {
        format!(
            "Pin #{} [{} | {}]: {} (@{})",
            self.id.0,
            self.severity.label(),
            self.status.label(),
            self.comment,
            self.author
        )
    }

    pub fn coordinates_label(&self) -> String {
        format!(
            "Anchor: ({:.1}, {:.1}) mm",
            self.position_mm.x, self.position_mm.y
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcbLayer {
    BottomCopper,
    TopCopper,
    Silkscreen,
    Ratsnest,
}

impl PcbLayer {
    pub fn z_rank(self) -> u8 {
        match self {
            PcbLayer::BottomCopper => 0,
            PcbLayer::TopCopper => 1,
            PcbLayer::Silkscreen => 2,
            PcbLayer::Ratsnest => 3,
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            PcbLayer::BottomCopper => Color32::from_rgb(59, 142, 234),
            PcbLayer::TopCopper => Color32::from_rgb(224, 82, 67),
            PcbLayer::Silkscreen => Color32::from_rgb(240, 230, 140),
            PcbLayer::Ratsnest => Color32::from_rgb(111, 247, 255),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceSegment {
    pub start: Pos2,
    pub end: Pos2,
    pub width_mm: f32,
    pub layer: PcbLayer,
    pub net: Option<NetId>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmdPad {
    pub center_mm: Pos2,
    pub size_mm: Vec2,
    pub layer: PcbLayer,
    pub net: NetId,
}

impl SmdPad {
    pub fn bounds_mm(&self) -> Rect {
        Rect::from_center_size(self.center_mm, self.size_mm)
    }

    pub fn contains_world_point(&self, world_mm: Pos2) -> bool {
        self.bounds_mm().contains(world_mm)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentSymbol {
    pub ref_des: String,
    pub value: String,
    pub package_name: String,
    pub center_mm: Pos2,
    pub body_size_mm: Vec2,
    pub primary_net: NetId,
}

impl ComponentSymbol {
    pub fn inspector_label(&self) -> String {
        format!("{}: {} ({})", self.ref_des, self.value, self.package_name)
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return true;
        }
        self.ref_des.to_ascii_lowercase().contains(&needle)
            || self.value.to_ascii_lowercase().contains(&needle)
            || self.package_name.to_ascii_lowercase().contains(&needle)
            || self
                .primary_net
                .signal_name()
                .to_ascii_lowercase()
                .contains(&needle)
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

fn sample_traces() -> Vec<TraceSegment> {
    vec![
        TraceSegment {
            start: pos2(20.0, 32.0),
            end: pos2(55.0, 48.0),
            width_mm: 0.25,
            layer: PcbLayer::Ratsnest,
            net: Some(NetId(2)),
        },
        TraceSegment {
            start: pos2(14.0, 16.0),
            end: pos2(28.0, 16.0),
            width_mm: 0.3,
            layer: PcbLayer::Silkscreen,
            net: None,
        },
        TraceSegment {
            start: pos2(20.0, 22.0),
            end: pos2(52.0, 22.0),
            width_mm: 0.8,
            layer: PcbLayer::TopCopper,
            net: Some(NetId(1)),
        },
        TraceSegment {
            start: pos2(52.0, 22.0),
            end: pos2(68.0, 35.0),
            width_mm: 0.8,
            layer: PcbLayer::TopCopper,
            net: Some(NetId(1)),
        },
        TraceSegment {
            start: pos2(20.0, 48.0),
            end: pos2(55.0, 48.0),
            width_mm: 0.8,
            layer: PcbLayer::BottomCopper,
            net: Some(NetId(2)),
        },
        TraceSegment {
            start: pos2(55.0, 48.0),
            end: pos2(68.0, 35.0),
            width_mm: 0.8,
            layer: PcbLayer::BottomCopper,
            net: Some(NetId(1)),
        },
    ]
}

fn sample_pads() -> Vec<SmdPad> {
    vec![
        SmdPad {
            center_mm: pos2(20.0, 22.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        },
        SmdPad {
            center_mm: pos2(20.0, 32.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(2),
        },
        SmdPad {
            center_mm: pos2(20.0, 48.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::BottomCopper,
            net: NetId(2),
        },
    ]
}

fn sample_components() -> Vec<ComponentSymbol> {
    vec![
        ComponentSymbol {
            ref_des: "U1".to_owned(),
            value: "LDO_3V3".to_owned(),
            package_name: "SOT-23".to_owned(),
            center_mm: pos2(20.0, 27.0),
            body_size_mm: vec2(12.0, 16.0),
            primary_net: NetId(1),
        },
        ComponentSymbol {
            ref_des: "R12".to_owned(),
            value: "10k".to_owned(),
            package_name: "0603".to_owned(),
            center_mm: pos2(52.0, 35.0),
            body_size_mm: vec2(10.0, 6.0),
            primary_net: NetId(2),
        },
        ComponentSymbol {
            ref_des: "J1".to_owned(),
            value: "CONN_2P".to_owned(),
            package_name: "Header".to_owned(),
            center_mm: pos2(68.0, 35.0),
            body_size_mm: vec2(10.0, 14.0),
            primary_net: NetId(1),
        },
    ]
}

fn sample_review_pins() -> HashMap<PinId, ReviewNote> {
    let mut pins = HashMap::new();
    pins.insert(
        PinId(1),
        ReviewNote {
            id: PinId(1),
            position_mm: pos2(20.0, 27.0),
            author: "alice".to_owned(),
            comment: "Check U1 thermal relief".to_owned(),
            severity: PinSeverity::Critical,
            status: ReviewStatus::Open,
        },
    );
    pins.insert(
        PinId(2),
        ReviewNote {
            id: PinId(2),
            position_mm: pos2(68.0, 35.0),
            author: "bob".to_owned(),
            comment: "Verify J1 polarity mark".to_owned(),
            severity: PinSeverity::Warning,
            status: ReviewStatus::Resolved,
        },
    );
    pins
}

pub struct BoardModel {
    pub traces: Vec<TraceSegment>,
    pub pads: Vec<SmdPad>,
    pub components: Vec<ComponentSymbol>,
    pub review_pins: HashMap<PinId, ReviewNote>,
    pub next_pin_id: u32,
}

impl Default for BoardModel {
    fn default() -> Self {
        Self {
            traces: sample_traces(),
            pads: sample_pads(),
            components: sample_components(),
            review_pins: sample_review_pins(),
            next_pin_id: 3,
        }
    }
}

impl BoardModel {
    pub fn find_component(&self, ref_des: &str) -> Option<&ComponentSymbol> {
        self.components.iter().find(|c| c.ref_des == ref_des)
    }

    pub fn filter_components(&self, query: &str) -> Vec<ComponentSymbol> {
        self.components
            .iter()
            .filter(|c| c.matches_query(query))
            .cloned()
            .collect()
    }

    pub fn insert_review_pin(
        &mut self,
        id: PinId,
        position_mm: Pos2,
        author: &str,
        comment: &str,
        severity: PinSeverity,
    ) -> bool {
        match self.review_pins.entry(id) {
            Entry::Vacant(slot) => {
                slot.insert(ReviewNote {
                    id,
                    position_mm,
                    author: author.to_owned(),
                    comment: comment.to_owned(),
                    severity,
                    status: ReviewStatus::Open,
                });
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn add_review_pin_at(
        &mut self,
        position_mm: Pos2,
        author: &str,
        comment: &str,
        severity: PinSeverity,
    ) -> PinId {
        let id = PinId(self.next_pin_id);
        self.next_pin_id += 1;
        self.insert_review_pin(id, position_mm, author, comment, severity);
        id
    }

    pub fn sorted_review_pins(&self) -> Vec<ReviewNote> {
        let mut list: Vec<ReviewNote> = self.review_pins.values().cloned().collect();
        list.sort_by_key(|p| p.id);
        list
    }

    pub fn transition_pin_status(&mut self, id: PinId, next: ReviewStatus) -> bool {
        let Some(pin) = self.review_pins.get_mut(&id) else {
            return false;
        };
        if pin.status.can_transition_to(next) {
            pin.status = next;
            true
        } else {
            false
        }
    }

    pub fn filter_review_pins(
        &self,
        min_severity: PinSeverity,
        only_open: bool,
    ) -> Vec<ReviewNote> {
        self.sorted_review_pins()
            .into_iter()
            .filter(|pin| {
                pin.severity.rank() >= min_severity.rank()
                    && (!only_open || pin.status == ReviewStatus::Open)
            })
            .collect()
    }
}

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
    pub selected_pin: Option<PinId>,
    pub min_pin_severity: PinSeverity,
    pub filter_only_open_pins: bool,
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
            selected_pin: None,
            min_pin_severity: PinSeverity::Info,
            filter_only_open_pins: false,
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
}

#[derive(Default)]
pub struct PcbReviewerApp {
    pub board: BoardModel,
    pub view: ViewState,
}

impl PcbReviewerApp {
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

    pub fn render_side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("PCB Layers");
        ui.checkbox(&mut self.view.show_top_copper, "Top Copper (F.Cu)");
        ui.checkbox(&mut self.view.show_bottom_copper, "Bottom Copper (B.Cu)");
        ui.checkbox(&mut self.view.show_silkscreen, "Silkscreen (F.SilkS)");
        ui.checkbox(&mut self.view.show_ratsnest, "Ratsnest (Unrouted)");

        ui.separator();
        if ui.button("Isolate Top Copper").clicked() {
            self.view.isolate_top_copper();
        }
        if ui.button("Show All Layers").clicked() {
            self.view.show_all_layers();
        }

        ui.separator();
        ui.heading("BOM & Netlist Inspector");
        ui.horizontal(|ui| {
            let bom_label = ui.label("Filter BOM:");
            ui.text_edit_singleline(&mut self.view.bom_search_query)
                .labelled_by(bom_label.id);
            if ui.button("Clear Filter").clicked() {
                self.view.clear_bom_query();
            }
        });

        if self.view.show_silkscreen {
            let matching = self.board.filter_components(&self.view.bom_search_query);
            ui.label(format!("Matching Components: {}", matching.len()));
            egui::ScrollArea::vertical()
                .max_height(130.0)
                .show_rows(ui, 22.0, matching.len(), |ui, row_range| {
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
            .id(egui::Id::unique(("review_pin_callout", pin.id.0)))
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
                seg.layer.color()
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
                pad.layer.color()
            };
            painter.rect_filled(screen_rect, 0.3 * scale, fill);
        }

        if self.view.show_silkscreen {
            let silk_color = PcbLayer::Silkscreen.color();
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

        self.render_active_pin_callout(ui.ctx());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    #[test]
    fn default_enables_physical_layers_and_hides_ratsnest() {
        let app = PcbReviewerApp::default();
        assert!(app.view.show_top_copper);
        assert!(app.view.show_bottom_copper);
        assert!(app.view.show_silkscreen);
        assert!(!app.view.show_ratsnest);
    }

    #[test]
    fn isolate_top_copper_forces_only_top_copper_visible() {
        let mut app = PcbReviewerApp::default();
        app.view.show_top_copper = false;
        app.view.show_ratsnest = true;

        app.view.isolate_top_copper();

        assert!(app.view.show_top_copper);
        assert!(!app.view.show_bottom_copper);
        assert!(!app.view.show_silkscreen);
        assert!(!app.view.show_ratsnest);
    }

    #[test]
    fn show_all_layers_enables_every_layer() {
        let mut app = PcbReviewerApp::default();
        app.view.show_top_copper = false;
        app.view.show_bottom_copper = false;
        app.view.show_silkscreen = false;
        app.view.show_ratsnest = false;

        app.view.show_all_layers();

        assert!(app.view.show_top_copper);
        assert!(app.view.show_bottom_copper);
        assert!(app.view.show_silkscreen);
        assert!(app.view.show_ratsnest);
    }

    #[test]
    fn world_and_screen_transforms_are_exact_inverses() {
        let vp = BoardViewport::default();
        let world_pad = pos2(25.0, 40.0);

        let screen_pt = vp.world_to_screen(world_pad);
        assert_eq!(screen_pt, pos2(230.0, 300.0));

        let round_trip = vp.screen_to_world(screen_pt);
        assert!((round_trip.x - world_pad.x).abs() < 1e-4);
        assert!((round_trip.y - world_pad.y).abs() < 1e-4);
    }

    #[test]
    fn pan_by_translates_screen_coordinates_and_fit_restores_default() {
        let mut app = PcbReviewerApp::default();
        let before = app.view.viewport.world_to_screen(pos2(10.0, 10.0));

        app.view.viewport.pan_by(vec2(15.0, -25.0));
        app.view.pcb_scene_rect = Rect::from_min_max(pos2(5.0, 5.0), pos2(25.0, 25.0));
        let after = app.view.viewport.world_to_screen(pos2(10.0, 10.0));
        assert_eq!(after, before + vec2(15.0, -25.0));

        app.view.fit_board_in_viewport();
        assert_eq!(app.view.viewport, BoardViewport::default());
        assert_eq!(
            app.view.pcb_scene_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0))
        );
    }

    #[test]
    fn zoom_around_keeps_cursor_world_point_stationary_and_clamps_scale() {
        let mut vp = BoardViewport::default();
        let cursor_screen = pos2(200.0, 180.0);
        let world_under_cursor_before = vp.screen_to_world(cursor_screen);

        vp.zoom_around(cursor_screen, 2.0);
        assert_eq!(vp.transform.scaling, 12.0);

        let world_under_cursor_after = vp.screen_to_world(cursor_screen);
        assert!((world_under_cursor_after.x - world_under_cursor_before.x).abs() < 1e-4);
        assert!((world_under_cursor_after.y - world_under_cursor_before.y).abs() < 1e-4);

        vp.zoom_around(cursor_screen, 100.0);
        assert_eq!(vp.transform.scaling, MAX_ZOOM);

        vp.zoom_around(cursor_screen, 0.001);
        assert_eq!(vp.transform.scaling, MIN_ZOOM);
    }

    #[test]
    fn pcb_layer_ranks_and_colors_are_distinct_and_ordered_back_to_front() {
        assert_eq!(PcbLayer::BottomCopper.z_rank(), 0);
        assert_eq!(PcbLayer::TopCopper.z_rank(), 1);
        assert_eq!(PcbLayer::Silkscreen.z_rank(), 2);
        assert_eq!(PcbLayer::Ratsnest.z_rank(), 3);

        assert_eq!(PcbLayer::BottomCopper.color(), Color32::from_rgb(59, 142, 234));
        assert_eq!(PcbLayer::TopCopper.color(), Color32::from_rgb(224, 82, 67));
        assert_eq!(PcbLayer::Silkscreen.color(), Color32::from_rgb(240, 230, 140));
        assert_eq!(PcbLayer::Ratsnest.color(), Color32::from_rgb(111, 247, 255));
    }

    #[test]
    fn is_layer_visible_reflects_each_individual_layer_flag() {
        let mut app = PcbReviewerApp::default();
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
    }

    #[test]
    fn visible_traces_in_z_order_filters_hidden_layers_and_sorts_back_to_front() {
        let mut app = PcbReviewerApp::default();
        let default_visible = app.visible_traces_in_z_order();
        let default_layers: Vec<PcbLayer> = default_visible.iter().map(|t| t.layer).collect();
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

        app.view.show_all_layers();
        let all_visible = app.visible_traces_in_z_order();
        assert_eq!(all_visible.last().map(|t| t.layer), Some(PcbLayer::Ratsnest));
    }

    #[test]
    fn hit_test_net_at_mm_respects_pad_bounds_layer_visibility_and_z_order() {
        let mut app = PcbReviewerApp::default();

        assert_eq!(app.hit_test_net_at_mm(pos2(20.5, 22.2)), Some(NetId(1)));
        assert_eq!(app.hit_test_net_at_mm(pos2(80.0, 60.0)), None);

        app.view.show_top_copper = false;
        assert_eq!(app.hit_test_net_at_mm(pos2(20.5, 22.2)), None);

        app.view.show_top_copper = true;
        app.view.show_bottom_copper = true;
        app.board.pads.push(SmdPad {
            center_mm: pos2(20.0, 48.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(99),
        });
        assert_eq!(app.hit_test_net_at_mm(pos2(20.0, 48.0)), Some(NetId(99)));
    }

    #[test]
    fn active_net_prefers_selected_net_over_hovered_net() {
        let mut app = PcbReviewerApp::default();
        assert_eq!(app.view.active_net(), None);

        app.view.hovered_net = Some(NetId(2));
        assert_eq!(app.view.active_net(), Some(NetId(2)));

        app.view.selected_net = Some(NetId(1));
        assert_eq!(app.view.active_net(), Some(NetId(1)));
    }

    #[test]
    fn net_id_code_and_signal_name_distinguish_owned_string_from_str_slice() {
        assert_eq!(NetId(1).code(), "NET_1");
        assert_eq!(NetId(1).signal_name(), "VCC_3V3");

        assert_eq!(NetId(2).code(), "NET_2");
        assert_eq!(NetId(2).signal_name(), "SPI_CLK");

        assert_eq!(NetId(99).code(), "NET_99");
        assert_eq!(NetId(99).signal_name(), "UNASSIGNED");
    }

    #[test]
    fn component_symbol_inspector_label_formats_ref_des_value_and_package() {
        let comp = ComponentSymbol {
            ref_des: "U1".to_owned(),
            value: "LDO_3V3".to_owned(),
            package_name: "SOT-23".to_owned(),
            center_mm: pos2(20.0, 27.0),
            body_size_mm: vec2(12.0, 16.0),
            primary_net: NetId(1),
        };
        assert_eq!(comp.inspector_label(), "U1: LDO_3V3 (SOT-23)");
    }

    #[test]
    fn focus_component_centers_scene_rect_and_returns_false_for_unknown_ref_des() {
        let board = BoardModel::default();
        let mut view = ViewState::default();

        assert!(view.focus_component(&board, "R12"));
        assert_eq!(view.selected_component.as_deref(), Some("R12"));
        assert_eq!(view.selected_net, Some(NetId(2)));
        assert_eq!(
            view.pcb_scene_rect,
            Rect::from_center_size(pos2(52.0, 35.0), vec2(40.0, 30.0))
        );

        assert!(!view.focus_component(&board, "U999"));
    }

    #[test]
    fn matches_query_checks_ref_des_value_package_and_net_case_insensitively() {
        let board = BoardModel::default();

        assert_eq!(board.filter_components("   ").len(), 3);

        let by_ref = board.filter_components("r12");
        assert_eq!(by_ref.len(), 1);
        assert_eq!(by_ref[0].ref_des, "R12");

        let by_val = board.filter_components("ldo_3v3");
        assert_eq!(by_val.len(), 1);
        assert_eq!(by_val[0].ref_des, "U1");

        let by_pkg = board.filter_components("header");
        assert_eq!(by_pkg.len(), 1);
        assert_eq!(by_pkg[0].ref_des, "J1");

        let by_net = board.filter_components("spi_clk");
        assert_eq!(by_net.len(), 1);
        assert_eq!(by_net[0].ref_des, "R12");

        assert!(board.filter_components("zz_missing").is_empty());
    }

    #[test]
    fn ruler_measurement_computes_deltas_hypotenuse_and_summary_label() {
        let mut view = ViewState::default();
        assert_eq!(view.current_measurement(), None);

        view.activate_measure_tool();
        assert_eq!(view.current_measurement(), None);

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
    fn review_pin_entry_api_prevents_overwrites_and_sorts_deterministically() {
        let mut board = BoardModel::default();
        assert_eq!(board.review_pins.len(), 2);

        assert!(!board.insert_review_pin(
            PinId(1),
            pos2(99.0, 99.0),
            "intruder",
            "Overwritten comment",
            PinSeverity::Info,
        ));
        assert_eq!(board.review_pins[&PinId(1)].author, "alice");

        let p3 = board.add_review_pin_at(
            pos2(52.0, 35.0),
            "carol",
            "Check R12 value",
            PinSeverity::Info,
        );
        let p4 = board.add_review_pin_at(
            pos2(40.0, 18.0),
            "dave",
            "Add test point",
            PinSeverity::Warning,
        );
        assert_eq!(p3, PinId(3));
        assert_eq!(p4, PinId(4));
        assert_eq!(board.next_pin_id, 5);

        let sorted_ids: Vec<PinId> = board.sorted_review_pins().iter().map(|n| n.id).collect();
        assert_eq!(sorted_ids, vec![PinId(1), PinId(2), PinId(3), PinId(4)]);
    }

    #[test]
    fn select_review_pin_focuses_camera_and_rejects_unknown_id() {
        let board = BoardModel::default();
        let mut view = ViewState::default();

        assert!(view.select_review_pin(&board, PinId(2)));
        assert_eq!(view.selected_pin, Some(PinId(2)));
        assert_eq!(
            view.pcb_scene_rect,
            Rect::from_center_size(pos2(68.0, 35.0), vec2(40.0, 30.0))
        );

        assert!(!view.select_review_pin(&board, PinId(999)));
    }

    #[test]
    fn severity_ranks_status_transitions_and_filtering_enforce_signoff_rules() {
        assert_eq!(PinSeverity::Info.label(), "Info");
        assert_eq!(PinSeverity::Warning.label(), "Warning");
        assert_eq!(PinSeverity::Critical.label(), "Critical");
        assert_eq!(PinSeverity::Info.rank(), 0);
        assert_eq!(PinSeverity::Warning.rank(), 1);
        assert_eq!(PinSeverity::Critical.rank(), 2);

        assert_eq!(
            ReviewStatus::ALL,
            &[
                ReviewStatus::Open,
                ReviewStatus::Resolved,
                ReviewStatus::Verified
            ]
        );
        assert_eq!(ReviewStatus::Open.label(), "Open");
        assert_eq!(ReviewStatus::Resolved.label(), "Resolved");
        assert_eq!(ReviewStatus::Verified.label(), "Verified");

        // Exhaustively check all 3x3 transition pairs
        assert!(!ReviewStatus::Open.can_transition_to(ReviewStatus::Open));
        assert!(ReviewStatus::Open.can_transition_to(ReviewStatus::Resolved));
        assert!(!ReviewStatus::Open.can_transition_to(ReviewStatus::Verified));

        assert!(ReviewStatus::Resolved.can_transition_to(ReviewStatus::Open));
        assert!(!ReviewStatus::Resolved.can_transition_to(ReviewStatus::Resolved));
        assert!(ReviewStatus::Resolved.can_transition_to(ReviewStatus::Verified));

        assert!(ReviewStatus::Verified.can_transition_to(ReviewStatus::Open));
        assert!(!ReviewStatus::Verified.can_transition_to(ReviewStatus::Resolved));
        assert!(!ReviewStatus::Verified.can_transition_to(ReviewStatus::Verified));

        let mut board = BoardModel::default();
        board.add_review_pin_at(
            pos2(52.0, 35.0),
            "carol",
            "Silkscreen note",
            PinSeverity::Info,
        );

        // Pin #1 is Open — cannot jump straight to Verified!
        assert!(!board.transition_pin_status(PinId(1), ReviewStatus::Verified));
        assert_eq!(board.review_pins[&PinId(1)].status, ReviewStatus::Open);

        // Pin #1 Open -> Resolved -> Verified -> Open
        assert!(board.transition_pin_status(PinId(1), ReviewStatus::Resolved));
        assert_eq!(board.review_pins[&PinId(1)].status, ReviewStatus::Resolved);
        assert!(board.transition_pin_status(PinId(1), ReviewStatus::Verified));
        assert_eq!(board.review_pins[&PinId(1)].status, ReviewStatus::Verified);
        assert!(board.transition_pin_status(PinId(1), ReviewStatus::Open));
        assert_eq!(board.review_pins[&PinId(1)].status, ReviewStatus::Open);

        // Unknown PinId returns false
        assert!(!board.transition_pin_status(PinId(999), ReviewStatus::Resolved));

        // Filter combinations: Info (all 3), Warning (Pin 1 & 2), Critical (Pin 1)
        assert_eq!(board.filter_review_pins(PinSeverity::Info, false).len(), 3);
        assert_eq!(
            board.filter_review_pins(PinSeverity::Warning, false).len(),
            2
        );
        assert_eq!(
            board.filter_review_pins(PinSeverity::Critical, false).len(),
            1
        );

        // Only open pins (Pin 1 is Open, Pin 2 is Resolved, Pin 3 is Open)
        let open_warnings = board.filter_review_pins(PinSeverity::Warning, true);
        assert_eq!(open_warnings.len(), 1);
        assert_eq!(open_warnings[0].id, PinId(1));
    }

    #[test]
    fn headless_ui_renders_component_labels_and_toggles_layers() {
        let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());

        assert!(harness.query_by_label("U1: LDO_3V3 (SOT-23)").is_some());
        assert!(harness.query_by_label("R12: 10k (0603)").is_some());

        harness.get_by_label("Isolate Top Copper").click();
        harness.run();
        assert!(!harness.state().view.show_bottom_copper);
        assert!(!harness.state().view.show_silkscreen);
        assert!(harness.query_by_label("U1: LDO_3V3 (SOT-23)").is_none());

        harness.get_by_label("Show All Layers").click();
        harness.run();
        assert!(harness.state().view.show_ratsnest);
        assert!(harness.query_by_label("U1: LDO_3V3 (SOT-23)").is_some());
    }

    #[test]
    fn clicking_schematic_probe_and_bom_filter_buttons_synchronizes_view() {
        let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());

        harness.get_by_label("Probe R12 (10k)").click();
        harness.run();
        assert_eq!(
            harness.state().view.selected_component.as_deref(),
            Some("R12")
        );

        harness.state_mut().view.bom_search_query = "j1".to_owned();
        harness.run();
        assert!(harness.query_by_label("Matching Components: 1").is_some());

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
    }

    #[test]
    fn headless_triage_workflow_and_pin_callout_transitions() {
        let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());
        assert!(harness.query_by_label("Review Pins: 2").is_some());

        // Filter to Critical only -> only Pin #1 remains
        harness.get_by_label("Critical").click();
        harness.run();
        assert_eq!(harness.state().view.min_pin_severity, PinSeverity::Critical);
        assert!(harness.query_by_label("Review Pins: 1").is_some());

        // Reset to Info, enable Open Pins Only -> only Pin #1 (Open) remains
        harness.get_by_label("Info").click();
        harness.get_by_label("Open Pins Only").click();
        harness.run();
        assert!(harness.state().view.filter_only_open_pins);
        assert!(harness.query_by_label("Review Pins: 1").is_some());

        // Uncheck Open Pins Only and open Pin #1 callout
        harness.get_by_label("Open Pins Only").click();
        harness
            .get_by_label("Pin #1 [Critical | Open]: Check U1 thermal relief (@alice)")
            .click();
        harness.run();
        assert_eq!(harness.state().view.selected_pin, Some(PinId(1)));
        assert!(harness.query_by_label("Anchor: (20.0, 27.0) mm").is_some());

        // Transition Open -> Resolved -> Verified -> Reopen inside the callout window
        harness.get_by_label("Mark Resolved").click();
        harness.run();
        assert_eq!(
            harness.state().board.review_pins[&PinId(1)].status,
            ReviewStatus::Resolved
        );

        harness.get_by_label("Mark Verified").click();
        harness.run();
        assert_eq!(
            harness.state().board.review_pins[&PinId(1)].status,
            ReviewStatus::Verified
        );

        harness.get_by_label("Reopen Pin").click();
        harness.run();
        assert_eq!(
            harness.state().board.review_pins[&PinId(1)].status,
            ReviewStatus::Open
        );

        harness.get_by_label("Close Callout").click();
        harness.run();
        assert_eq!(harness.state().view.selected_pin, None);

        // Hotkeys P, M, Esc, F
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

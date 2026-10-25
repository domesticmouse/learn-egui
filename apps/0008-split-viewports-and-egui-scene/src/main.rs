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
        "Schematic & PCB Design Reviewer — Lesson 0008",
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

/// Immutable board domain data (`&self.board`) separated from `ViewState` (`&mut self.view`)
/// so UI panels can iterate over components while mutating camera and selection state.
pub struct BoardModel {
    pub traces: Vec<TraceSegment>,
    pub pads: Vec<SmdPad>,
    pub components: Vec<ComponentSymbol>,
}

impl Default for BoardModel {
    fn default() -> Self {
        Self {
            traces: sample_traces(),
            pads: sample_pads(),
            components: sample_components(),
        }
    }
}

impl BoardModel {
    pub fn find_component(&self, ref_des: &str) -> Option<&ComponentSymbol> {
        self.components.iter().find(|c| c.ref_des == ref_des)
    }
}

/// Mutable camera, layer visibility, and cross-probe selection state (`&mut self.view`).
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

    pub fn reset_viewport(&mut self) {
        self.viewport = BoardViewport::default();
        self.pcb_scene_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
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

    /// Cross-probes a component from the Schematic Symbol Bar into the PCB camera.
    /// Demonstrates split borrowing: `board` is borrowed immutably (`&BoardModel`)
    /// while `self` (`&mut ViewState`) is mutated in place.
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
        ui.heading("Components (RefDes)");
        if self.view.show_silkscreen {
            for comp in &self.board.components {
                ui.label(comp.inspector_label());
            }
        } else {
            ui.label("Enable Silkscreen to inspect component RefDes.");
        }

        ui.separator();
        ui.heading("Net Inspector");
        if let Some(net) = self.view.active_net() {
            ui.label(format!("Active Net: {} ({})", net.code(), net.signal_name()));
            if ui.button("Clear Selection").clicked() {
                self.view.selected_net = None;
                self.view.selected_component = None;
            }
        } else {
            ui.label("Hover or click a pad to cross-probe its net.");
        }
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
                        self.view.selected_net = self.view.hovered_net;
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
    }
}

impl eframe::App for PcbReviewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("schematic_cross_probe_bar").show(ctx, |ui| {
            self.render_schematic_bar(ui);
        });

        egui::SidePanel::left("layer_panel").show(ctx, |ui| {
            self.render_side_panel(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
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
    fn pan_by_translates_screen_coordinates_and_reset_restores_default() {
        let mut app = PcbReviewerApp::default();
        let before = app.view.viewport.world_to_screen(pos2(10.0, 10.0));

        app.view.viewport.pan_by(vec2(15.0, -25.0));
        app.view.pcb_scene_rect = Rect::from_min_max(pos2(5.0, 5.0), pos2(25.0, 25.0));
        let after = app.view.viewport.world_to_screen(pos2(10.0, 10.0));
        assert_eq!(after, before + vec2(15.0, -25.0));

        app.view.reset_viewport();
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
    fn clicking_schematic_probe_button_updates_pcb_camera_rect_and_net() {
        let mut harness = Harness::new_eframe(|_cc| PcbReviewerApp::default());

        harness.get_by_label("Probe R12 (10k)").click();
        harness.run();

        assert_eq!(
            harness.state().view.selected_component.as_deref(),
            Some("R12")
        );
        assert_eq!(harness.state().view.selected_net, Some(NetId(2)));
        assert_eq!(
            harness.state().view.pcb_scene_rect.center(),
            pos2(52.0, 35.0)
        );
        assert!(harness.query_by_label("Focused Footprint: R12").is_some());

        harness.get_by_label("Clear Selection").click();
        harness.run();
        assert_eq!(harness.state().view.selected_net, None);
        assert_eq!(harness.state().view.selected_component, None);
    }
}

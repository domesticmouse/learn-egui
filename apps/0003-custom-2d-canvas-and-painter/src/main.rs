use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    options.viewport = egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]);

    eframe::run_native(
        "Schematic & PCB Design Reviewer — Lesson 0003",
        options,
        Box::new(|_cc| Ok(Box::new(PcbReviewerApp::default()))),
    )
}

/// Computes the inset FR4 board outline `Rect` inside the allocated `canvas_rect`
/// by shrinking all four edges inward by `margin` screen points.
pub fn centered_board_rect(canvas_rect: Rect, margin: f32) -> Rect {
    Rect::from_min_max(
        pos2(canvas_rect.min.x + margin, canvas_rect.min.y + margin),
        pos2(canvas_rect.max.x - margin, canvas_rect.max.y - margin),
    )
}

/// Computes the bounding `Rect` for a rectangular SMD pad centered at `center`
/// with width `size.x` and height `size.y`.
pub fn smd_pad_rect(center: Pos2, size: Vec2) -> Rect {
    let half_w = size.x / 2.0;
    let half_h = size.y / 2.0;
    Rect::from_min_max(
        pos2(center.x - half_w, center.y - half_h),
        pos2(center.x + half_w, center.y + half_h),
    )
}

pub struct PcbReviewerApp {
    pub show_top_copper: bool,
    pub show_bottom_copper: bool,
    pub show_silkscreen: bool,
    pub show_ratsnest: bool,
}

impl Default for PcbReviewerApp {
    fn default() -> Self {
        let mut app = Self {
            show_top_copper: false,
            show_bottom_copper: false,
            show_silkscreen: false,
            show_ratsnest: false,
        };
        app.reset_layers();
        app
    }
}

impl PcbReviewerApp {
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

    fn paint_static_board(&self, painter: &egui::Painter, canvas_rect: Rect) {
        // 1. Dark CAD background
        painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(12, 16, 14));

        // 2. Inset FR4 PCB substrate outline
        let board_rect = centered_board_rect(canvas_rect, 28.0);
        painter.rect(
            board_rect,
            6.0,
            Color32::from_rgb(16, 40, 30),
            Stroke::new(2.0_f32, Color32::from_rgb(47, 111, 82)),
            StrokeKind::Inside,
        );

        let origin = board_rect.min;

        // 3. Bottom Copper (B.Cu - Blue)
        if self.show_bottom_copper {
            let b_cu_stroke = Stroke::new(5.0_f32, Color32::from_rgb(59, 142, 234));
            painter.line_segment(
                [origin + vec2(80.0, 180.0), origin + vec2(220.0, 180.0)],
                b_cu_stroke,
            );
            painter.line_segment(
                [origin + vec2(220.0, 180.0), origin + vec2(280.0, 120.0)],
                b_cu_stroke,
            );
        }

        // 4. Top Copper (F.Cu - Terracotta Red) + SMD Pads + Plated Via
        if self.show_top_copper {
            let f_cu_color = Color32::from_rgb(224, 82, 67);
            let f_cu_stroke = Stroke::new(5.0_f32, f_cu_color);

            let pad_a = origin + vec2(80.0, 90.0);
            let pad_b = origin + vec2(80.0, 130.0);
            let via_pos = origin + vec2(280.0, 120.0);

            painter.line_segment([pad_a, origin + vec2(200.0, 90.0)], f_cu_stroke);
            painter.line_segment([origin + vec2(200.0, 90.0), via_pos], f_cu_stroke);
            painter.line_segment([pad_b, origin + vec2(170.0, 130.0)], f_cu_stroke);

            let pad_size = vec2(20.0, 12.0);
            painter.rect_filled(smd_pad_rect(pad_a, pad_size), 2.0, f_cu_color);
            painter.rect_filled(smd_pad_rect(pad_b, pad_size), 2.0, f_cu_color);

            // Plated through-hole via ring + drill hole
            painter.circle(
                via_pos,
                8.0,
                Color32::from_rgb(16, 40, 30),
                Stroke::new(3.5_f32, Color32::from_rgb(230, 193, 90)),
            );
        }

        // 5. Front Silkscreen (F.SilkS - Cream component courtyard)
        if self.show_silkscreen {
            let u1_rect = Rect::from_min_size(origin + vec2(62.0, 68.0), vec2(54.0, 132.0));
            painter.rect_stroke(
                u1_rect,
                2.0,
                Stroke::new(1.5_f32, Color32::from_rgb(240, 230, 140)),
                StrokeKind::Outside,
            );
        }

        // 6. Unrouted Ratsnest airwire
        if self.show_ratsnest {
            painter.line_segment(
                [origin + vec2(80.0, 130.0), origin + vec2(220.0, 180.0)],
                Stroke::new(1.5_f32, Color32::from_rgb(111, 247, 255)),
            );
        }
    }
}

impl eframe::App for PcbReviewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("layer_panel").show(ctx, |ui| {
            ui.heading("PCB Layers");
            ui.checkbox(&mut self.show_top_copper, "Top Copper (F.Cu)");
            ui.checkbox(&mut self.show_bottom_copper, "Bottom Copper (B.Cu)");
            ui.checkbox(&mut self.show_silkscreen, "Silkscreen (F.SilkS)");
            ui.checkbox(&mut self.show_ratsnest, "Ratsnest (Unrouted)");

            ui.separator();
            if ui.button("Isolate Top Copper").clicked() {
                self.isolate_top_copper();
            }
            if ui.button("Show All Layers").clicked() {
                self.show_all_layers();
            }
            if ui.button("Reset Defaults").clicked() {
                self.reset_layers();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
            self.paint_static_board(&painter, response.rect);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_enables_physical_layers_and_hides_ratsnest() {
        let app = PcbReviewerApp::default();
        assert!(app.show_top_copper);
        assert!(app.show_bottom_copper);
        assert!(app.show_silkscreen);
        assert!(!app.show_ratsnest);
    }

    #[test]
    fn isolate_top_copper_forces_only_top_copper_visible() {
        let mut app = PcbReviewerApp {
            show_top_copper: false,
            show_bottom_copper: true,
            show_silkscreen: true,
            show_ratsnest: true,
        };

        app.isolate_top_copper();

        assert!(app.show_top_copper);
        assert!(!app.show_bottom_copper);
        assert!(!app.show_silkscreen);
        assert!(!app.show_ratsnest);
    }

    #[test]
    fn show_all_layers_enables_every_layer() {
        let mut app = PcbReviewerApp {
            show_top_copper: false,
            show_bottom_copper: false,
            show_silkscreen: false,
            show_ratsnest: false,
        };

        app.show_all_layers();

        assert!(app.show_top_copper);
        assert!(app.show_bottom_copper);
        assert!(app.show_silkscreen);
        assert!(app.show_ratsnest);
    }

    #[test]
    fn centered_board_rect_insets_all_four_edges_by_margin() {
        let canvas = Rect::from_min_max(pos2(10.0, 20.0), pos2(310.0, 220.0));
        let board = centered_board_rect(canvas, 25.0);

        assert_eq!(board.min, pos2(35.0, 45.0));
        assert_eq!(board.max, pos2(285.0, 195.0));
    }

    #[test]
    fn smd_pad_rect_computes_symmetric_bounds_around_center() {
        let pad = smd_pad_rect(pos2(100.0, 60.0), vec2(16.0, 10.0));

        assert_eq!(pad.min, pos2(92.0, 55.0));
        assert_eq!(pad.max, pos2(108.0, 65.0));
    }
}

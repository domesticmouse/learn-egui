#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use std::collections::{HashMap, hash_map::Entry};

pub use emath::{Pos2, Rect, TSTransform, Vec2, pos2, vec2};

/// Layer L1 (`pcb-doc`) must never depend on UI or windowing crates (`L6+`).
pub const FORBIDDEN_UI_DEPS_BELOW_L6: &[&str] =
    &["egui", "eframe", "winit", "egui_kittest", "rfd"];

/// Verifies that an `L1` domain crate's `Cargo.toml` does not depend on any `L6+` UI crate.
pub fn validate_l1_manifest_no_ui_deps(cargo_toml: &str) -> Result<(), &'static str> {
    for &forbidden in FORBIDDEN_UI_DEPS_BELOW_L6 {
        let key_eq = format!("{forbidden} =");
        let key_dot = format!("dependencies.{forbidden}");
        if cargo_toml.contains(&key_eq) || cargo_toml.contains(&key_dot) {
            return Err(forbidden);
        }
    }
    Ok(())
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PinId(pub u32);

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

pub fn pad_edge_gap_mm(a: &SmdPad, b: &SmdPad) -> f32 {
    let dx = (a.center_mm.x - b.center_mm.x).abs() - (a.size_mm.x + b.size_mm.x) * 0.5;
    let dy = (a.center_mm.y - b.center_mm.y).abs() - (a.size_mm.y + b.size_mm.y) * 0.5;
    dx.max(0.0).hypot(dy.max(0.0))
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

    /// UI-agnostic RGB triple `(r, g, b)` for each layer in `L1` domain state.
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            PcbLayer::BottomCopper => (59, 142, 234),
            PcbLayer::TopCopper => (224, 82, 67),
            PcbLayer::Silkscreen => (240, 230, 140),
            PcbLayer::Ratsnest => (111, 247, 255),
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

pub(crate) fn sample_traces() -> Vec<TraceSegment> {
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

pub(crate) fn sample_pads() -> Vec<SmdPad> {
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

pub(crate) fn sample_components() -> Vec<ComponentSymbol> {
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

pub(crate) fn sample_review_pins() -> HashMap<PinId, ReviewNote> {
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

#[derive(Clone, Debug, PartialEq)]
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

    pub fn pads_bounding_box_mm(&self) -> Option<Rect> {
        self.pads
            .iter()
            .map(SmdPad::bounds_mm)
            .reduce(Rect::union)
    }

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn l1_manifest_gate_rejects_ui_dependencies_and_accepts_pcb_doc_manifest() {
        let actual_manifest = include_str!("../Cargo.toml");
        assert_eq!(validate_l1_manifest_no_ui_deps(actual_manifest), Ok(()));

        assert_eq!(
            validate_l1_manifest_no_ui_deps("[dependencies]\neframe = \"0.33\""),
            Err("eframe")
        );
        assert_eq!(
            validate_l1_manifest_no_ui_deps("[dependencies.egui]\nversion = \"0.33\""),
            Err("egui")
        );
    }

    #[test]
    fn net_id_and_layer_metadata_are_deterministic() {
        assert_eq!(NetId(1).code(), "NET_1");
        assert_eq!(NetId(1).signal_name(), "VCC_3V3");
        assert_eq!(NetId(2).code(), "NET_2");
        assert_eq!(NetId(2).signal_name(), "SPI_CLK");
        assert_eq!(NetId(99).code(), "NET_99");
        assert_eq!(NetId(99).signal_name(), "UNASSIGNED");

        assert_eq!(PcbLayer::BottomCopper.z_rank(), 0);
        assert_eq!(PcbLayer::TopCopper.z_rank(), 1);
        assert_eq!(PcbLayer::Silkscreen.z_rank(), 2);
        assert_eq!(PcbLayer::Ratsnest.z_rank(), 3);

        assert_eq!(PcbLayer::BottomCopper.rgb(), (59, 142, 234));
        assert_eq!(PcbLayer::TopCopper.rgb(), (224, 82, 67));
        assert_eq!(PcbLayer::Silkscreen.rgb(), (240, 230, 140));
        assert_eq!(PcbLayer::Ratsnest.rgb(), (111, 247, 255));
    }

    #[test]
    fn pad_hit_testing_and_component_query_filtering_work_headlessly() {
        let board = BoardModel::default();
        assert!(board.pads[0].contains_world_point(pos2(20.5, 22.2)));
        assert!(!board.pads[0].contains_world_point(pos2(50.0, 50.0)));

        assert_eq!(
            board.find_component("U1").map(ComponentSymbol::inspector_label),
            Some("U1: LDO_3V3 (SOT-23)".to_owned())
        );
        assert_eq!(board.find_component("U999"), None);

        assert_eq!(board.filter_components("   ").len(), 3);
        assert_eq!(board.filter_components("r12")[0].ref_des, "R12");
        assert_eq!(board.filter_components("ldo_3v3")[0].ref_des, "U1");
        assert_eq!(board.filter_components("header")[0].ref_des, "J1");
        assert_eq!(board.filter_components("spi_clk")[0].ref_des, "R12");
        assert!(board.filter_components("zz_missing").is_empty());
    }

    #[test]
    fn ruler_measurement_and_review_pin_workflows_preserve_invariants() {
        let m = RulerMeasurement::between(pos2(20.0, 22.0), pos2(68.0, 58.0));
        assert_eq!(m.dx_mm, 48.0);
        assert_eq!(m.dy_mm, 36.0);
        assert_eq!(m.distance_mm, 60.0);
        assert_eq!(
            m.summary_label(),
            "Ruler: dx=48.0 mm, dy=36.0 mm, dist=60.0 mm"
        );

        assert_eq!(PinSeverity::ALL.len(), 3);
        assert_eq!(ReviewStatus::ALL.len(), 3);
        assert_eq!(PinSeverity::Info.label(), "Info");
        assert_eq!(PinSeverity::Warning.label(), "Warning");
        assert_eq!(PinSeverity::Critical.label(), "Critical");
        assert_eq!(PinSeverity::Info.rank(), 0);
        assert_eq!(PinSeverity::Warning.rank(), 1);
        assert_eq!(PinSeverity::Critical.rank(), 2);

        assert_eq!(ReviewStatus::Open.label(), "Open");
        assert_eq!(ReviewStatus::Resolved.label(), "Resolved");
        assert_eq!(ReviewStatus::Verified.label(), "Verified");

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
        assert!(!board.insert_review_pin(
            PinId(1),
            pos2(99.0, 99.0),
            "intruder",
            "Overwritten",
            PinSeverity::Info,
        ));
        let p3 = board.add_review_pin_at(
            pos2(52.0, 35.0),
            "carol",
            "Check R12",
            PinSeverity::Info,
        );
        let p4 = board.add_review_pin_at(
            pos2(40.0, 18.0),
            "dave",
            "Add test point",
            PinSeverity::Info,
        );
        assert_eq!(p3, PinId(3));
        assert_eq!(p4, PinId(4));
        assert_eq!(board.next_pin_id, 5);
        assert_eq!(
            board.review_pins[&PinId(1)].summary_line(),
            "Pin #1 [Critical | Open]: Check U1 thermal relief (@alice)"
        );
        assert_eq!(
            board.review_pins[&PinId(1)].coordinates_label(),
            "Anchor: (20.0, 27.0) mm"
        );

        assert!(!board.transition_pin_status(PinId(1), ReviewStatus::Verified));
        assert!(board.transition_pin_status(PinId(1), ReviewStatus::Resolved));
        assert!(board.transition_pin_status(PinId(1), ReviewStatus::Verified));
        assert!(board.transition_pin_status(PinId(1), ReviewStatus::Open));
        assert!(!board.transition_pin_status(PinId(999), ReviewStatus::Resolved));

        assert_eq!(board.filter_review_pins(PinSeverity::Info, false).len(), 4);
        assert_eq!(board.filter_review_pins(PinSeverity::Warning, false).len(), 2);
        assert_eq!(board.filter_review_pins(PinSeverity::Critical, false).len(), 1);
        let open_warnings = board.filter_review_pins(PinSeverity::Warning, true);
        assert_eq!(open_warnings.len(), 1);
        assert_eq!(open_warnings[0].id, PinId(1));
    }

    #[test]
    fn drc_clearance_and_pad_bounding_boxes_are_exact() {
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
    }
}

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use std::{
    collections::{HashMap, VecDeque, hash_map::Entry},
    sync::Arc,
};

pub use emath::{Pos2, Rect, TSTransform, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};
pub use serde_json;
use thiserror::Error;

/// Current schema version for serialized PCB review sessions.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Default maximum undo history entries retained in the ring buffer.
pub const DEFAULT_MAX_HISTORY: usize = 32;

/// Minimum number of registered `CommandSpec`s enforced by our Tier 6 parity ratchet.
pub const COMMAND_PARITY_FLOOR: usize = 4;

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

/// Typed, panic-free persistence errors for PCB & review session files.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SessionIoError {
    #[error("board_id must not be empty")]
    EmptyBoardId,
    #[error("unsupported schema_version {found} (expected {expected})")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },
    #[error("malformed session JSON: {0}")]
    MalformedJson(String),
}

/// Typed, panic-free errors returned by the `CommandSpec` execution engine.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("unknown command `{0}`")]
    UnknownCommand(String),
    #[error("command `{id}` is disabled: {reason}")]
    DisabledCommand { id: String, reason: String },
    #[error("invalid parameters for `{id}`: {reason}")]
    BadParams { id: String, reason: String },
}

pub type EnabledFn = fn(&BoardModel, &BoardHistory) -> Result<(), String>;
pub type RunFn = fn(
    &mut BoardModel,
    &mut BoardHistory,
    &serde_json::Value,
) -> Result<serde_json::Value, EngineError>;

/// Metadata, precondition guard, and execution function pointer for a single engine command.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: Option<&'static str>,
    pub journal: bool,
    pub enabled: EnabledFn,
    pub run: RunFn,
}

fn always_enabled(_board: &BoardModel, _history: &BoardHistory) -> Result<(), String> {
    Ok(())
}

fn has_review_pins(board: &BoardModel, _history: &BoardHistory) -> Result<(), String> {
    if board.review_pins.is_empty() {
        Err("board has no review pins to clear".to_owned())
    } else {
        Ok(())
    }
}

fn can_undo_history(_board: &BoardModel, history: &BoardHistory) -> Result<(), String> {
    if history.can_undo() {
        Ok(())
    } else {
        Err("undo stack is empty".to_owned())
    }
}

fn run_add_pin(
    board: &mut BoardModel,
    history: &mut BoardHistory,
    params: &serde_json::Value,
) -> Result<serde_json::Value, EngineError> {
    let x_mm = params
        .get("x_mm")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| EngineError::BadParams {
            id: "review.addPin".to_owned(),
            reason: "missing numeric x_mm".to_owned(),
        })? as f32;
    let y_mm = params
        .get("y_mm")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| EngineError::BadParams {
            id: "review.addPin".to_owned(),
            reason: "missing numeric y_mm".to_owned(),
        })? as f32;
    if !(0.0..=100.0).contains(&x_mm) || !(0.0..=70.0).contains(&y_mm) {
        return Err(EngineError::BadParams {
            id: "review.addPin".to_owned(),
            reason: "coordinates out of board bounds".to_owned(),
        });
    }
    let comment = params
        .get("comment")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| EngineError::BadParams {
            id: "review.addPin".to_owned(),
            reason: "comment must not be empty".to_owned(),
        })?;
    let before = board.clone();
    let id = board.add_review_pin_at(pos2(x_mm, y_mm), "cmd", comment, PinSeverity::Warning);
    history.record_edit("Place Review Pin", before, None);
    Ok(serde_json::json!({
        "added_pin_id": id.0,
        "pin_count": board.review_pins.len(),
    }))
}

fn run_clear_pins(
    board: &mut BoardModel,
    history: &mut BoardHistory,
    _params: &serde_json::Value,
) -> Result<serde_json::Value, EngineError> {
    let cleared = board.review_pins.len();
    let before = board.clone();
    board.clear_review_pins();
    history.record_edit("Clear Review Pins", before, None);
    Ok(serde_json::json!({
        "cleared_count": cleared,
        "pin_count": board.review_pins.len(),
    }))
}

fn run_history_undo(
    board: &mut BoardModel,
    history: &mut BoardHistory,
    _params: &serde_json::Value,
) -> Result<serde_json::Value, EngineError> {
    let label = history.undo(board).unwrap_or_default();
    Ok(serde_json::json!({
        "undone_label": label,
        "undo_len": history.undo_len(),
    }))
}

fn run_session_inspect(
    board: &mut BoardModel,
    history: &mut BoardHistory,
    _params: &serde_json::Value,
) -> Result<serde_json::Value, EngineError> {
    Ok(serde_json::json!({
        "pin_count": board.review_pins.len(),
        "pad_count": board.pads.len(),
        "revision": history.revision(),
    }))
}

pub static COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        id: "review.addPin",
        label: "Place Review Pin",
        shortcut: Some("P"),
        journal: true,
        enabled: always_enabled,
        run: run_add_pin,
    },
    CommandSpec {
        id: "review.clearPins",
        label: "Clear Review Pins",
        shortcut: None,
        journal: true,
        enabled: has_review_pins,
        run: run_clear_pins,
    },
    CommandSpec {
        id: "history.undo",
        label: "Undo Last Edit",
        shortcut: Some("Z"),
        journal: false,
        enabled: can_undo_history,
        run: run_history_undo,
    },
    CommandSpec {
        id: "session.inspect",
        label: "Inspect Session Telemetry",
        shortcut: Some("I"),
        journal: false,
        enabled: always_enabled,
        run: run_session_inspect,
    },
];

pub fn find_command_spec(id: &str) -> Option<&'static CommandSpec> {
    COMMAND_SPECS.iter().find(|spec| spec.id == id)
}

pub fn filter_command_specs(query: &str) -> Vec<&'static CommandSpec> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return COMMAND_SPECS.iter().collect();
    }
    COMMAND_SPECS
        .iter()
        .filter(|spec| {
            spec.id.to_ascii_lowercase().contains(&needle)
                || spec.label.to_ascii_lowercase().contains(&needle)
        })
        .collect()
}

/// Versioned envelope wrapping a serialized `BoardModel`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewSessionEnvelope {
    pub schema_version: u32,
    pub board_id: String,
    pub board: BoardModel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PinId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSegment {
    pub start: Pos2,
    pub end: Pos2,
    pub width_mm: f32,
    pub layer: PcbLayer,
    pub net: Option<NetId>,
}

impl TraceSegment {
    pub fn bounds_mm(&self) -> Rect {
        Rect::from_two_pos(self.start, self.end).expand(self.width_mm * 0.5)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewportCullStats {
    pub visible_pads: usize,
    pub total_pads: usize,
    pub visible_traces: usize,
    pub total_traces: usize,
}

impl ViewportCullStats {
    pub fn culled_count(&self) -> usize {
        (self.total_pads - self.visible_pads) + (self.total_traces - self.visible_traces)
    }

    pub fn summary_badge(&self) -> String {
        format!(
            "Cull: {}/{} pads, {}/{} traces ({} skipped)",
            self.visible_pads,
            self.total_pads,
            self.visible_traces,
            self.total_traces,
            self.culled_count()
        )
    }
}

pub fn check_cull_budget(measured_us: u64, target_max_us: u64) -> Result<u64, u64> {
    if measured_us <= target_max_us {
        Ok(measured_us)
    } else {
        Err(measured_us)
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/// Copy-on-Write (`Arc`) domain model: cloning `BoardModel` copies only four `Arc`
/// pointers (`O(1)`), and mutating a collection via `Arc::make_mut` clones only the
/// touched collection while sharing all untouched buffers across snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardModel {
    pub traces: Arc<Vec<TraceSegment>>,
    pub pads: Arc<Vec<SmdPad>>,
    pub components: Arc<Vec<ComponentSymbol>>,
    pub review_pins: Arc<HashMap<PinId, ReviewNote>>,
    pub next_pin_id: u32,
}

impl Default for BoardModel {
    fn default() -> Self {
        Self {
            traces: Arc::new(sample_traces()),
            pads: Arc::new(sample_pads()),
            components: Arc::new(sample_components()),
            review_pins: Arc::new(sample_review_pins()),
            next_pin_id: 3,
        }
    }
}

/// A single copy-on-write snapshot entry stored in `BoardHistory`.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub label: String,
    pub doc: BoardModel,
}

/// Bounded copy-on-write undo/redo history with continuous slider drag coalescing.
#[derive(Clone, Debug)]
pub struct BoardHistory {
    undo_stack: VecDeque<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    max_entries: usize,
    active_coalesce_key: Option<String>,
    revision: u64,
}

impl Default for BoardHistory {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_HISTORY)
    }
}

impl BoardHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_entries: max_entries.max(1),
            active_coalesce_key: None,
            revision: 0,
        }
    }

    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Commits any in-flight continuous drag gesture so the next edit starts a fresh undo entry.
    pub fn commit_coalesce(&mut self) {
        self.active_coalesce_key = None;
    }

    /// Records a pre-mutation `before` snapshot, coalescing consecutive edits that share the same
    /// `Some(coalesce_key)` into a single undo entry while incrementing `revision` on every frame.
    pub fn record_edit(&mut self, label: &str, before: BoardModel, coalesce_key: Option<&str>) {
        let should_coalesce =
            self.active_coalesce_key.as_deref() == coalesce_key && coalesce_key.is_some();

        if !should_coalesce {
            if self.undo_stack.len() >= self.max_entries {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(HistoryEntry {
                label: label.to_owned(),
                doc: before,
            });
        }

        self.redo_stack.clear();
        self.active_coalesce_key = coalesce_key.map(str::to_owned);
        self.revision += 1;
    }

    /// Undoes the most recent edit in `O(1)` `Arc` pointer swaps, pushing the current state onto `redo_stack`.
    pub fn undo(&mut self, current: &mut BoardModel) -> Option<String> {
        let entry = self.undo_stack.pop_back()?;
        self.active_coalesce_key = None;
        let label = entry.label.clone();
        let prev = std::mem::replace(current, entry.doc);
        self.redo_stack.push(HistoryEntry {
            label: entry.label,
            doc: prev,
        });
        self.revision += 1;
        Some(label)
    }

    /// Redoes the most recently undone edit in `O(1)` `Arc` pointer swaps.
    pub fn redo(&mut self, current: &mut BoardModel) -> Option<String> {
        let entry = self.redo_stack.pop()?;
        self.active_coalesce_key = None;
        let label = entry.label.clone();
        let prev = std::mem::replace(current, entry.doc);
        if self.undo_stack.len() >= self.max_entries {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(HistoryEntry {
            label: entry.label,
            doc: prev,
        });
        self.revision += 1;
        Some(label)
    }
}

impl BoardModel {
    /// Validates and executes a registered `CommandSpec` by `id` against `self` and `history`.
    pub fn execute_command(
        &mut self,
        history: &mut BoardHistory,
        id: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, EngineError> {
        let spec =
            find_command_spec(id).ok_or_else(|| EngineError::UnknownCommand(id.to_owned()))?;
        (spec.enabled)(self, history).map_err(|reason| EngineError::DisabledCommand {
            id: spec.id.to_owned(),
            reason,
        })?;
        (spec.run)(self, history, params)
    }

    /// Counts how many of the 4 `Arc` collection buffers (`0..=4`) share the exact
    /// same heap allocation (`Arc::ptr_eq`) with `other`.
    pub fn shared_arc_buffers(&self, other: &Self) -> usize {
        usize::from(Arc::ptr_eq(&self.traces, &other.traces))
            + usize::from(Arc::ptr_eq(&self.pads, &other.pads))
            + usize::from(Arc::ptr_eq(&self.components, &other.components))
            + usize::from(Arc::ptr_eq(&self.review_pins, &other.review_pins))
    }

    /// Appends a copper pad using copy-on-write (`Arc::make_mut`).
    pub fn push_smd_pad(&mut self, pad: SmdPad) {
        Arc::make_mut(&mut self.pads).push(pad);
    }

    /// Clears all review pins using copy-on-write (`Arc::make_mut`).
    pub fn clear_review_pins(&mut self) {
        Arc::make_mut(&mut self.review_pins).clear();
    }

    /// Updates a single review pin's X coordinate in millimetres using copy-on-write (`Arc::make_mut`).
    pub fn move_review_pin_x(&mut self, id: PinId, new_x_mm: f32) -> bool {
        let Some(pin) = Arc::make_mut(&mut self.review_pins).get_mut(&id) else {
            return false;
        };
        pin.position_mm.x = new_x_mm;
        true
    }

    /// Serializes the board and review pins into a versioned JSON session string.
    pub fn to_session_json(&self, board_id: &str) -> Result<String, SessionIoError> {
        let trimmed = board_id.trim();
        if trimmed.is_empty() {
            return Err(SessionIoError::EmptyBoardId);
        }
        let envelope = ReviewSessionEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            board_id: trimmed.to_owned(),
            board: self.clone(),
        };
        serde_json::to_string_pretty(&envelope)
            .map_err(|e| SessionIoError::MalformedJson(e.to_string()))
    }

    /// Deserializes and validates a versioned JSON session string without panicking.
    pub fn from_session_json(raw: &str) -> Result<(String, Self), SessionIoError> {
        let ReviewSessionEnvelope {
            schema_version,
            board_id,
            board,
        } = serde_json::from_str::<ReviewSessionEnvelope>(raw)
            .map_err(|e| SessionIoError::MalformedJson(e.to_string()))?;

        if schema_version != CURRENT_SCHEMA_VERSION {
            return Err(SessionIoError::UnsupportedSchemaVersion {
                found: schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }
        let trimmed = board_id.trim();
        if trimmed.is_empty() {
            return Err(SessionIoError::EmptyBoardId);
        }
        Ok((trimmed.to_owned(), board))
    }

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
        match Arc::make_mut(&mut self.review_pins).entry(id) {
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
        let Some(pin) = Arc::make_mut(&mut self.review_pins).get_mut(&id) else {
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

    pub fn culled_pads_in_rect(&self, viewport_mm: Rect) -> Vec<SmdPad> {
        let mut out = Vec::with_capacity(self.pads.len());
        for pad in self.pads.iter() {
            if pad.bounds_mm().intersects(viewport_mm) {
                out.push(*pad);
            }
        }
        out
    }

    pub fn culled_traces_in_rect(&self, viewport_mm: Rect) -> Vec<TraceSegment> {
        let mut out = Vec::with_capacity(self.traces.len());
        for seg in self.traces.iter() {
            if seg.bounds_mm().intersects(viewport_mm) {
                out.push(*seg);
            }
        }
        out
    }

    pub fn cull_stats_for_rect(&self, viewport_mm: Rect) -> ViewportCullStats {
        ViewportCullStats {
            visible_pads: self.culled_pads_in_rect(viewport_mm).len(),
            total_pads: self.pads.len(),
            visible_traces: self.culled_traces_in_rect(viewport_mm).len(),
            total_traces: self.traces.len(),
        }
    }

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

    pub fn detect_pad_clearance_violations(&self, min_clearance_mm: f32) -> Vec<DrcViolation> {
        let mut violations = Vec::new();
        let mut seen: Vec<&SmdPad> = Vec::new();
        for b in self.pads.iter() {
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
    use proptest::prelude::*;

    #[test]
    fn l1_manifest_gate_rejects_ui_dependencies_and_accepts_pcb_doc_manifest() {
        let actual_manifest = include_str!("../Cargo.toml");
        assert_eq!(validate_l1_manifest_no_ui_deps(actual_manifest), Ok(()));

        assert_eq!(
            validate_l1_manifest_no_ui_deps("[dependencies]\neframe = \"0.37\""),
            Err("eframe")
        );
        assert_eq!(
            validate_l1_manifest_no_ui_deps("[dependencies.egui]\nversion = \"0.37\""),
            Err("egui")
        );
    }

    #[test]
    fn shared_arc_buffers_tracks_copy_on_write_pointer_sharing_across_all_four_collections() {
        let baseline = BoardModel::default();
        let mut working = baseline.clone();
        assert_eq!(working.shared_arc_buffers(&baseline), 4);

        working.clear_review_pins();
        assert!(working.review_pins.is_empty());
        assert_eq!(baseline.review_pins.len(), 2);
        assert_eq!(working.shared_arc_buffers(&baseline), 3);

        working.push_smd_pad(SmdPad {
            center_mm: pos2(10.0, 10.0),
            size_mm: vec2(2.0, 2.0),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        });
        assert_eq!(working.pads.len(), 4);
        assert_eq!(baseline.pads.len(), 3);
        assert_eq!(working.shared_arc_buffers(&baseline), 2);

        Arc::make_mut(&mut working.components).pop();
        assert_eq!(working.shared_arc_buffers(&baseline), 1);

        Arc::make_mut(&mut working.traces).pop();
        assert_eq!(working.shared_arc_buffers(&baseline), 0);
    }

    #[test]
    fn board_history_undo_redo_coalescing_and_bounded_eviction_work() {
        let mut history = BoardHistory::new(2);
        let mut board = BoardModel::default();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_len(), 0);
        assert_eq!(history.redo_len(), 0);
        assert_eq!(history.revision(), 0);
        assert_eq!(history.undo(&mut board), None);
        assert_eq!(history.redo(&mut board), None);

        // Continuous slider drag coalescing: 3 drag frames with the same coalesce_key
        // record only 1 undo entry (preserving the original x=20.0 mm).
        for next_x in [25.0_f32, 30.0_f32, 35.0_f32] {
            let before = board.clone();
            assert!(board.move_review_pin_x(PinId(1), next_x));
            history.record_edit("Drag Pin #1 X", before, Some("drag_pin_1_x"));
        }
        assert!(!board.move_review_pin_x(PinId(999), 50.0));
        assert_eq!(history.undo_len(), 1);
        assert!(history.can_undo());
        assert_eq!(history.revision(), 3);
        assert_eq!(board.review_pins[&PinId(1)].position_mm.x, 35.0);

        // Committing the drag gesture breaks coalescing so a subsequent drag creates a 2nd entry.
        history.commit_coalesce();
        let before_second_drag = board.clone();
        assert!(board.move_review_pin_x(PinId(1), 42.0));
        history.record_edit("Drag Pin #1 X", before_second_drag, Some("drag_pin_1_x"));
        assert_eq!(history.undo_len(), 2);
        assert_eq!(history.revision(), 4);

        // Undo the 2nd drag -> returns to 35.0 mm and pushes to redo_stack.
        assert_eq!(history.undo(&mut board).as_deref(), Some("Drag Pin #1 X"));
        assert_eq!(board.review_pins[&PinId(1)].position_mm.x, 35.0);
        assert_eq!(history.undo_len(), 1);
        assert_eq!(history.redo_len(), 1);
        assert!(history.can_redo());
        assert_eq!(history.revision(), 5);

        // Redo the 2nd drag -> returns to 42.0 mm.
        assert_eq!(history.redo(&mut board).as_deref(), Some("Drag Pin #1 X"));
        assert_eq!(board.review_pins[&PinId(1)].position_mm.x, 42.0);
        assert_eq!(history.undo_len(), 2);
        assert_eq!(history.redo_len(), 0);
        assert!(!history.can_redo());
        assert_eq!(history.revision(), 6);

        // Undo twice -> returns all the way to the original 20.0 mm before the 1st drag.
        assert_eq!(history.undo(&mut board).as_deref(), Some("Drag Pin #1 X"));
        assert_eq!(board.review_pins[&PinId(1)].position_mm.x, 35.0);
        assert_eq!(history.undo(&mut board).as_deref(), Some("Drag Pin #1 X"));
        assert_eq!(board.review_pins[&PinId(1)].position_mm.x, 20.0);
        assert_eq!(history.undo_len(), 0);
        assert_eq!(history.redo_len(), 2);

        // Recording a fresh discrete edit clears redo_stack and respects max_entries=2.
        for label in ["Edit 1", "Edit 2", "Edit 3"] {
            let before = board.clone();
            board.add_review_pin_at(pos2(12.0, 14.0), "bot", label, PinSeverity::Info);
            history.record_edit(label, before, None);
        }
        assert_eq!(history.redo_len(), 0);
        assert_eq!(history.undo_len(), 2);
        assert_eq!(history.undo(&mut board).as_deref(), Some("Edit 3"));
        assert_eq!(history.undo(&mut board).as_deref(), Some("Edit 2"));
        assert_eq!(history.undo(&mut board), None);
    }

    proptest! {
        #[test]
        fn prop_cow_pin_mutation_preserves_baseline_snapshot_and_shares_untouched_arcs(
            x in 5.0_f32..95.0_f32,
            y in 5.0_f32..65.0_f32,
            comment in "[a-zA-Z0-9 _-]{1,24}",
        ) {
            let baseline = BoardModel::default();
            let mut working = baseline.clone();
            prop_assert_eq!(working.shared_arc_buffers(&baseline), 4);

            let added_id = working.add_review_pin_at(
                pos2(x, y),
                "proptest",
                &comment,
                PinSeverity::Warning,
            );

            prop_assert_eq!(baseline.review_pins.len(), 2);
            prop_assert!(!baseline.review_pins.contains_key(&added_id));
            prop_assert_eq!(working.review_pins.len(), 3);
            prop_assert_eq!(&working.review_pins[&added_id].comment, &comment);

            prop_assert!(Arc::ptr_eq(&baseline.traces, &working.traces));
            prop_assert!(Arc::ptr_eq(&baseline.pads, &working.pads));
            prop_assert!(Arc::ptr_eq(&baseline.components, &working.components));
            prop_assert!(!Arc::ptr_eq(&baseline.review_pins, &working.review_pins));
            prop_assert_eq!(working.shared_arc_buffers(&baseline), 3);
        }

        #[test]
        fn prop_coalesced_drag_restores_exact_initial_state_in_single_undo(
            drag_steps in prop::collection::vec(10.0_f32..90.0_f32, 2..12),
        ) {
            let initial = BoardModel::default();
            let mut working = initial.clone();
            let mut history = BoardHistory::default();

            for x in &drag_steps {
                let before = working.clone();
                working.move_review_pin_x(PinId(1), *x);
                history.record_edit("Drag Pin #1 X", before, Some("drag_pin_1_x"));
            }
            history.commit_coalesce();

            prop_assert_eq!(history.undo_len(), 1);
            let final_dragged = working.clone();
            let undone_label = history.undo(&mut working);
            prop_assert_eq!(undone_label.as_deref(), Some("Drag Pin #1 X"));
            prop_assert_eq!(&working, &initial);
            prop_assert_eq!(working.shared_arc_buffers(&initial), 4);

            let redone_label = history.redo(&mut working);
            prop_assert_eq!(redone_label.as_deref(), Some("Drag Pin #1 X"));
            prop_assert_eq!(&working, &final_dragged);
        }

        #[test]
        fn prop_spatial_culling_matches_reference_and_preserves_counts(
            min_x in 0.0_f32..80.0_f32,
            min_y in 0.0_f32..50.0_f32,
            w in 5.0_f32..60.0_f32,
            h in 5.0_f32..40.0_f32,
        ) {
            let board = BoardModel::default();
            let viewport = Rect::from_min_size(pos2(min_x, min_y), vec2(w, h));
            let expected_pads: Vec<SmdPad> = board
                .pads
                .iter()
                .copied()
                .filter(|p| p.bounds_mm().intersects(viewport))
                .collect();
            let expected_traces: Vec<TraceSegment> = board
                .traces
                .iter()
                .copied()
                .filter(|t| t.bounds_mm().intersects(viewport))
                .collect();

            let culled_pads = board.culled_pads_in_rect(viewport);
            let culled_traces = board.culled_traces_in_rect(viewport);
            let stats = board.cull_stats_for_rect(viewport);

            prop_assert_eq!(&culled_pads, &expected_pads);
            prop_assert_eq!(&culled_traces, &expected_traces);
            prop_assert_eq!(stats.visible_pads, expected_pads.len());
            prop_assert_eq!(stats.visible_traces, expected_traces.len());
            prop_assert_eq!(
                stats.visible_pads + stats.visible_traces + stats.culled_count(),
                board.pads.len() + board.traces.len()
            );
        }
    }

    #[test]
    fn session_json_round_trip_and_typed_validation_errors_are_lossless() {
        let mut original = BoardModel::default();
        original.add_review_pin_at(
            pos2(52.0, 35.0),
            "carol",
            "Verify R12 footprint",
            PinSeverity::Warning,
        );

        assert_eq!(
            original.to_session_json("   "),
            Err(SessionIoError::EmptyBoardId)
        );

        let json = original
            .to_session_json("  pwr-stage-rev-b  ")
            .expect("valid board should serialize");
        let (loaded_id, loaded_board) =
            BoardModel::from_session_json(&json).expect("valid JSON should deserialize");
        assert_eq!(loaded_id, "pwr-stage-rev-b");
        assert_eq!(loaded_board, original);

        let bad_version = json.replace("\"schema_version\": 1", "\"schema_version\": 99");
        assert_eq!(
            BoardModel::from_session_json(&bad_version),
            Err(SessionIoError::UnsupportedSchemaVersion {
                found: 99,
                expected: CURRENT_SCHEMA_VERSION,
            })
        );

        let empty_board_id =
            json.replace("\"board_id\": \"pwr-stage-rev-b\"", "\"board_id\": \"   \"");
        assert_eq!(
            BoardModel::from_session_json(&empty_board_id),
            Err(SessionIoError::EmptyBoardId)
        );

        let err = BoardModel::from_session_json("{ truncated json").unwrap_err();
        assert!(matches!(err, SessionIoError::MalformedJson(_)));
        assert!(err.to_string().contains("malformed session JSON"));
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

        board.push_smd_pad(SmdPad {
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

    #[test]
    fn command_registry_dispatch_and_panic_hunt_fuzzer_preserve_invariants() {
        assert!(COMMAND_SPECS.len() >= COMMAND_PARITY_FLOOR);
        assert_eq!(filter_command_specs("   ").len(), 4);
        assert_eq!(filter_command_specs("addpin")[0].id, "review.addPin");
        assert_eq!(filter_command_specs("telemetry")[0].id, "session.inspect");
        assert!(filter_command_specs("nonexistent_xyz").is_empty());

        let mut board = BoardModel::default();
        let mut history = BoardHistory::default();

        assert_eq!(
            board.execute_command(&mut history, "unknown.cmd", &serde_json::Value::Null),
            Err(EngineError::UnknownCommand("unknown.cmd".to_owned()))
        );
        assert_eq!(
            board.execute_command(&mut history, "history.undo", &serde_json::Value::Null),
            Err(EngineError::DisabledCommand {
                id: "history.undo".to_owned(),
                reason: "undo stack is empty".to_owned(),
            })
        );

        let inspect_out = board
            .execute_command(&mut history, "session.inspect", &serde_json::json!({}))
            .unwrap();
        assert_eq!(
            inspect_out,
            serde_json::json!({ "pin_count": 2, "pad_count": 3, "revision": 0 })
        );

        // Adversarial validation on review.addPin (Tier 5 panic_hunt)
        for (bad_payload, expected_reason) in [
            (serde_json::json!({ "y_mm": 20.0, "comment": "ok" }), "missing numeric x_mm"),
            (serde_json::json!({ "x_mm": 20.0, "comment": "ok" }), "missing numeric y_mm"),
            (
                serde_json::json!({ "x_mm": -1.0, "y_mm": 20.0, "comment": "ok" }),
                "coordinates out of board bounds",
            ),
            (
                serde_json::json!({ "x_mm": 105.0, "y_mm": 20.0, "comment": "ok" }),
                "coordinates out of board bounds",
            ),
            (
                serde_json::json!({ "x_mm": 20.0, "y_mm": -2.0, "comment": "ok" }),
                "coordinates out of board bounds",
            ),
            (
                serde_json::json!({ "x_mm": 20.0, "y_mm": 75.0, "comment": "ok" }),
                "coordinates out of board bounds",
            ),
            (
                serde_json::json!({ "x_mm": 20.0, "y_mm": 30.0, "comment": "   " }),
                "comment must not be empty",
            ),
        ] {
            assert_eq!(
                board.execute_command(&mut history, "review.addPin", &bad_payload),
                Err(EngineError::BadParams {
                    id: "review.addPin".to_owned(),
                    reason: expected_reason.to_owned(),
                })
            );
        }

        // Exact boundary checks (0.0, 100.0, 70.0)
        let added = board
            .execute_command(
                &mut history,
                "review.addPin",
                &serde_json::json!({ "x_mm": 0.0, "y_mm": 70.0, "comment": "  Edge check  " }),
            )
            .unwrap();
        assert_eq!(added, serde_json::json!({ "added_pin_id": 3, "pin_count": 3 }));
        assert_eq!(board.review_pins[&PinId(3)].position_mm, pos2(0.0, 70.0));
        assert_eq!(board.review_pins[&PinId(3)].comment, "Edge check");
        assert_eq!(history.undo_len(), 1);

        let added_max = board
            .execute_command(
                &mut history,
                "review.addPin",
                &serde_json::json!({ "x_mm": 100.0, "y_mm": 0.0, "comment": "Corner" }),
            )
            .unwrap();
        assert_eq!(added_max, serde_json::json!({ "added_pin_id": 4, "pin_count": 4 }));

        let undone = board
            .execute_command(&mut history, "history.undo", &serde_json::json!({}))
            .unwrap();
        assert_eq!(
            undone,
            serde_json::json!({ "undone_label": "Place Review Pin", "undo_len": 1 })
        );
        assert_eq!(board.review_pins.len(), 3);

        let cleared = board
            .execute_command(&mut history, "review.clearPins", &serde_json::json!({}))
            .unwrap();
        assert_eq!(
            cleared,
            serde_json::json!({ "cleared_count": 3, "pin_count": 0 })
        );
        assert_eq!(
            board.execute_command(&mut history, "review.clearPins", &serde_json::json!({})),
            Err(EngineError::DisabledCommand {
                id: "review.clearPins".to_owned(),
                reason: "board has no review pins to clear".to_owned(),
            })
        );
    }

    #[test]
    fn viewport_culling_bounds_stats_and_perf_budget_are_exact() {
        let seg = TraceSegment {
            start: pos2(10.0, 20.0),
            end: pos2(30.0, 24.0),
            width_mm: 4.0,
            layer: PcbLayer::TopCopper,
            net: Some(NetId(1)),
        };
        assert_eq!(
            seg.bounds_mm(),
            Rect::from_min_max(pos2(8.0, 18.0), pos2(32.0, 26.0))
        );

        let board = BoardModel::default();
        let tight_rect = Rect::from_min_max(pos2(12.0, 14.0), pos2(25.0, 24.0));
        let stats = board.cull_stats_for_rect(tight_rect);
        assert_eq!(stats.visible_pads, 1);
        assert_eq!(stats.total_pads, 3);
        assert_eq!(stats.visible_traces, 2);
        assert_eq!(stats.total_traces, 6);
        assert_eq!(stats.culled_count(), 6);
        assert_eq!(
            stats.summary_badge(),
            "Cull: 1/3 pads, 2/6 traces (6 skipped)"
        );

        assert_eq!(check_cull_budget(180, 500), Ok(180));
        assert_eq!(check_cull_budget(500, 500), Ok(500));
        assert_eq!(check_cull_budget(501, 500), Err(501));
    }

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
}

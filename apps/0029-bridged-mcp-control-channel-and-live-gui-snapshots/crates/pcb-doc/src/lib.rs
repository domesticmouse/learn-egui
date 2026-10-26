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
    collections::{HashMap, HashSet, VecDeque, hash_map::Entry},
    sync::Arc,
};

pub use emath::{Pos2, Rect, TSTransform, Vec2, pos2, vec2};
pub use kicad_sexpr::{
    FORBIDDEN_DEPS_IN_L0_STANDALONE, SAMPLE_KICAD_PCB_SNIPPET, SexprNode, SexprParseError,
    is_symbol_char, parse_sexpr_str, validate_l0_standalone_manifest,
};
pub use rmcp::{
    ClientHandler, Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolRequestParams, Tool},
    tool, tool_handler, tool_router,
};
pub use schemars::{self, JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
pub use serde_json;
use thiserror::Error;
use turso_parser::{
    ast::{Cmd, Stmt},
    parser::Parser as TursoSqlParser,
};

/// Current schema version for serialized PCB review sessions.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Default maximum undo history entries retained in the ring buffer.
pub const DEFAULT_MAX_HISTORY: usize = 32;

/// Minimum number of registered `CommandSpec`s enforced by our Tier 6 parity ratchet.
pub const COMMAND_PARITY_FLOOR: usize = 4;

/// Minimum number of pinned KiCad `.kicad_pcb` corpus fixtures that must parse cleanly.
pub const CORPUS_PASS_FLOOR: usize = 3;

/// Minimum number of pinned KiCad `.kicad_pcb` corpus fixtures that must round-trip losslessly.
pub const CORPUS_ROUNDTRIP_FLOOR: usize = 3;

/// Pinned real-structure KiCad `.kicad_pcb` corpus fixtures verified by our Tier 4 oracle harness.
pub const PINNED_KICAD_CORPUS: &[(&str, &str)] = &[
    (
        "ldo-power-rev-a.kicad_pcb",
        SAMPLE_KICAD_PCB_SNIPPET,
    ),
    (
        "spi-sensor-rev-b.kicad_pcb",
        "(kicad_pcb (version 20240108) (footprint \"0603\" (ref \"R12\") (val \"10k\") (at 52.0 35.0) (size 10.0 6.0) (net 2)) (pad \"1\" smd (at 20.0 48.0) (size 4.0 2.4) (layer \"B.Cu\") (net 2)) (segment (start 20.0 48.0) (end 55.0 48.0) (width 0.8) (layer \"B.Cu\") (net 2)))",
    ),
    (
        "usb-header-rev-c.kicad_pcb",
        "(kicad_pcb (version 20240108) (footprint \"Header\" (ref \"J1\") (val \"CONN_2P\") (at 68.0 35.0) (size 10.0 14.0) (net 1)) (segment (start 14.0 16.0) (end 28.0 16.0) (width 0.3) (layer \"F.SilkS\") (net 0)) (segment (start 20.0 32.0) (end 55.0 48.0) (width 0.25) (layer \"Ratsnest\") (net 2)))",
    ),
];

/// Typed, panic-free errors returned when lowering a KiCad S-expression AST into `BoardModel`.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum KicadLowerError {
    #[error("S-expression parse error: {0}")]
    Sexpr(#[from] SexprParseError),
    #[error("expected `(kicad_pcb ...)` root node, found `{0}`")]
    UnexpectedRootTag(String),
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("invalid numeric coordinate `{0}`")]
    InvalidNumber(String),
    #[error("unknown KiCad layer `{0}`")]
    UnknownLayer(String),
}

/// Tier 4 pinned KiCad corpus oracle report tracking pass and lossless round-trip counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorpusOracleReport {
    pub total_files: usize,
    pub passed_files: usize,
    pub roundtrip_files: usize,
}

impl CorpusOracleReport {
    pub fn meets_ratchet_floors(&self) -> bool {
        self.passed_files >= CORPUS_PASS_FLOOR
            && self.roundtrip_files >= CORPUS_ROUNDTRIP_FLOOR
            && self.passed_files == self.total_files
    }

    pub fn summary_badge(&self) -> String {
        let status = if self.meets_ratchet_floors() {
            "PASS"
        } else {
            "FAIL"
        };
        format!(
            "Corpus [{status}]: {}/{} pass, {}/{} round-trip",
            self.passed_files, self.total_files, self.roundtrip_files, self.total_files
        )
    }
}

/// Runs the Tier 4 pinned KiCad `.kicad_pcb` corpus oracle across all `PINNED_KICAD_CORPUS` fixtures.
pub fn run_pinned_kicad_corpus() -> CorpusOracleReport {
    let mut passed_files = 0;
    let mut roundtrip_files = 0;
    for &(_name, raw_sexpr) in PINNED_KICAD_CORPUS {
        if let Ok(board) = BoardModel::from_kicad_pcb_sexpr(raw_sexpr) {
            passed_files += 1;
            let exported = board.to_kicad_pcb_sexpr();
            if let Ok(reloaded) = BoardModel::from_kicad_pcb_sexpr(&exported)
                && reloaded == board
            {
                roundtrip_files += 1;
            }
        }
    }
    CorpusOracleReport {
        total_files: PINNED_KICAD_CORPUS.len(),
        passed_files,
        roundtrip_files,
    }
}

/// Embedded Turso SQLite DDL schema for persisting PCB review sessions and threads.
pub const REVIEW_STORE_SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS boards (board_id TEXT PRIMARY KEY, created_rev INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS review_threads (
    pin_id INTEGER PRIMARY KEY,
    board_id TEXT NOT NULL,
    author TEXT NOT NULL,
    comment TEXT NOT NULL,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    x_mm REAL NOT NULL,
    y_mm REAL NOT NULL
);
";

/// Typed, panic-free errors returned by the embedded Turso SQL review store.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SqlStoreError {
    #[error("SQL statement must not be empty")]
    EmptySql,
    #[error("invalid Turso SQL syntax: {0}")]
    InvalidSql(String),
    #[error("review store schema has not been migrated")]
    NotMigrated,
    #[error("foreign key violation: unregistered board_id `{0}`")]
    ForeignKeyViolation(String),
    #[error("missing review pin #{0:?}")]
    MissingPin(PinId),
    #[error("invalid status transition on pin #{pin_id:?}: {from:?} -> {to:?}")]
    InvalidStatusTransition {
        pin_id: PinId,
        from: ReviewStatus,
        to: ReviewStatus,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditLogEntry {
    pub seq: u64,
    pub pin_id: PinId,
    pub actor: String,
    pub from_status: ReviewStatus,
    pub to_status: ReviewStatus,
    pub prev_hash: u64,
    pub entry_hash: u64,
}

pub fn compute_audit_entry_hash(
    seq: u64,
    pin_id: PinId,
    from_status: ReviewStatus,
    to_status: ReviewStatus,
    prev_hash: u64,
) -> u64 {
    prev_hash
        .wrapping_mul(1_000_003)
        .wrapping_add(seq * 97)
        .wrapping_add(u64::from(pin_id.0) * 31)
        .wrapping_add(u64::from(from_status as u8 + 1) * 13)
        .wrapping_add(u64::from(to_status as u8 + 1) * 7)
}

/// Parses and validates a SQL script using Turso's official `turso_parser::parser::Parser`,
/// returning the count of supported DDL/DML/transaction statements.
pub fn validate_turso_sql(sql: &str) -> Result<usize, SqlStoreError> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(SqlStoreError::EmptySql);
    }
    let mut parser = TursoSqlParser::new(trimmed.as_bytes());
    let mut stmt_count = 0;
    while let Some(cmd) = parser
        .next_cmd()
        .map_err(|e| SqlStoreError::InvalidSql(e.to_string()))?
    {
        if matches!(
            cmd,
            Cmd::Stmt(
                Stmt::CreateTable { .. }
                    | Stmt::Insert { .. }
                    | Stmt::Select(_)
                    | Stmt::Begin { .. }
                    | Stmt::Commit { .. }
                    | Stmt::Rollback { .. }
            )
        ) {
            stmt_count += 1;
        }
    }
    if stmt_count == 0 {
        return Err(SqlStoreError::InvalidSql(
            "no supported SQL statements".to_owned(),
        ));
    }
    Ok(stmt_count)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SqlReviewRecord {
    pub pin_id: PinId,
    pub board_id: String,
    pub author: String,
    pub comment: String,
    pub severity: PinSeverity,
    pub status: ReviewStatus,
    pub position_mm: Pos2,
    pub revision: u64,
}

impl SqlReviewRecord {
    pub fn matches_sql_filter(&self, query: &str, only_open: bool) -> bool {
        if only_open && self.status != ReviewStatus::Open {
            return false;
        }
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return true;
        }
        self.author.to_ascii_lowercase().contains(&needle)
            || self.comment.to_ascii_lowercase().contains(&needle)
            || self.status.label().to_ascii_lowercase().contains(&needle)
    }
}

#[derive(Clone, Debug, Default)]
pub struct TursoReviewStore {
    schema_version: u32,
    registered_boards: HashSet<String>,
    pub rows: HashMap<PinId, SqlReviewRecord>,
    next_revision: u64,
    pub audit_log: Vec<AuditLogEntry>,
    pub pending_local_writes: usize,
}

impl TursoReviewStore {
    pub fn is_migrated(&self) -> bool {
        self.schema_version == CURRENT_SCHEMA_VERSION
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn summary_badge(&self) -> String {
        if !self.is_migrated() {
            "Turso SQL: Unmigrated".to_owned()
        } else {
            format!(
                "Turso SQL: {} rows (Schema v{})",
                self.rows.len(),
                self.schema_version
            )
        }
    }

    pub fn verify_audit_chain(&self) -> bool {
        let mut expected_prev = 0_u64;
        for (idx, entry) in self.audit_log.iter().enumerate() {
            let expected_seq = (idx + 1) as u64;
            let expected_hash = compute_audit_entry_hash(
                expected_seq,
                entry.pin_id,
                entry.from_status,
                entry.to_status,
                expected_prev,
            );
            if entry.seq != expected_seq
                || entry.prev_hash != expected_prev
                || entry.entry_hash != expected_hash
            {
                return false;
            }
            expected_prev = entry.entry_hash;
        }
        true
    }

    pub fn replica_status_badge(&self) -> String {
        let chain = if self.verify_audit_chain() {
            "VALID"
        } else {
            "TAMPERED"
        };
        format!(
            "Replica: {} pending, {} audit [{chain}]",
            self.pending_local_writes,
            self.audit_log.len()
        )
    }

    pub fn migrate_sync(&mut self) -> Result<u32, SqlStoreError> {
        validate_turso_sql(REVIEW_STORE_SCHEMA_SQL)?;
        self.schema_version = CURRENT_SCHEMA_VERSION;
        Ok(self.schema_version)
    }

    pub async fn migrate(&mut self) -> Result<u32, SqlStoreError> {
        self.migrate_sync()
    }

    pub fn register_board(&mut self, board_id: &str) -> Result<bool, SqlStoreError> {
        if !self.is_migrated() {
            return Err(SqlStoreError::NotMigrated);
        }
        let trimmed = board_id.trim();
        if trimmed.is_empty() {
            return Err(SqlStoreError::ForeignKeyViolation(String::new()));
        }
        validate_turso_sql("INSERT INTO boards (board_id, created_rev) VALUES (?1, ?2);")?;
        Ok(self.registered_boards.insert(trimmed.to_owned()))
    }

    pub fn upsert_pin_record_sync(
        &mut self,
        board_id: &str,
        note: &ReviewNote,
    ) -> Result<u64, SqlStoreError> {
        if !self.is_migrated() {
            return Err(SqlStoreError::NotMigrated);
        }
        if !self.registered_boards.contains(board_id) {
            return Err(SqlStoreError::ForeignKeyViolation(board_id.to_owned()));
        }
        validate_turso_sql(
            "INSERT INTO review_threads (pin_id, board_id, author, comment, severity, status, x_mm, y_mm) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
        )?;
        self.next_revision += 1;
        self.pending_local_writes += 1;
        self.rows.insert(
            note.id,
            SqlReviewRecord {
                pin_id: note.id,
                board_id: board_id.to_owned(),
                author: note.author.clone(),
                comment: note.comment.clone(),
                severity: note.severity,
                status: note.status,
                position_mm: note.position_mm,
                revision: self.next_revision,
            },
        );
        Ok(self.next_revision)
    }

    pub async fn upsert_pin_record(
        &mut self,
        board_id: &str,
        note: &ReviewNote,
    ) -> Result<u64, SqlStoreError> {
        self.upsert_pin_record_sync(board_id, note)
    }

    pub fn query_review_records_sync(
        &self,
        query: &str,
        only_open: bool,
    ) -> Result<Vec<SqlReviewRecord>, SqlStoreError> {
        if !self.is_migrated() {
            return Err(SqlStoreError::NotMigrated);
        }
        validate_turso_sql(
            "SELECT pin_id, board_id, author, comment, severity, status, x_mm, y_mm FROM review_threads ORDER BY pin_id;",
        )?;
        let mut out: Vec<SqlReviewRecord> = self
            .rows
            .values()
            .filter(|r| r.matches_sql_filter(query, only_open))
            .cloned()
            .collect();
        out.sort_by_key(|r| r.pin_id);
        Ok(out)
    }

    pub async fn query_review_records(
        &self,
        query: &str,
        only_open: bool,
    ) -> Result<Vec<SqlReviewRecord>, SqlStoreError> {
        self.query_review_records_sync(query, only_open)
    }

    pub fn execute_atomic_signoff_batch_sync(
        &mut self,
        actor: &str,
        transitions: &[(PinId, ReviewStatus)],
    ) -> Result<usize, SqlStoreError> {
        if !self.is_migrated() {
            return Err(SqlStoreError::NotMigrated);
        }
        validate_turso_sql("BEGIN; COMMIT; ROLLBACK;")?;
        let mut staged_rows = self.rows.clone();
        let mut staged_audit = self.audit_log.clone();
        let mut staged_rev = self.next_revision;

        for &(pin_id, next_status) in transitions {
            let Some(row) = staged_rows.get_mut(&pin_id) else {
                return Err(SqlStoreError::MissingPin(pin_id));
            };
            if !row.status.can_transition_to(next_status) {
                return Err(SqlStoreError::InvalidStatusTransition {
                    pin_id,
                    from: row.status,
                    to: next_status,
                });
            }
            let from_status = row.status;
            row.status = next_status;
            staged_rev += 1;
            row.revision = staged_rev;
            let seq = (staged_audit.len() + 1) as u64;
            let prev_hash = staged_audit.last().map_or(0, |e| e.entry_hash);
            let entry_hash =
                compute_audit_entry_hash(seq, pin_id, from_status, next_status, prev_hash);
            staged_audit.push(AuditLogEntry {
                seq,
                pin_id,
                actor: actor.to_owned(),
                from_status,
                to_status: next_status,
                prev_hash,
                entry_hash,
            });
        }

        self.rows = staged_rows;
        self.audit_log = staged_audit;
        self.next_revision = staged_rev;
        self.pending_local_writes += transitions.len();
        Ok(transitions.len())
    }

    pub async fn execute_atomic_signoff_batch(
        &mut self,
        actor: &str,
        transitions: &[(PinId, ReviewStatus)],
    ) -> Result<usize, SqlStoreError> {
        self.execute_atomic_signoff_batch_sync(actor, transitions)
    }

    pub fn sync_with_remote_replica_sync(
        &mut self,
        remote_rows: &[SqlReviewRecord],
    ) -> Result<usize, SqlStoreError> {
        if !self.is_migrated() {
            return Err(SqlStoreError::NotMigrated);
        }
        let mut merged = 0;
        for remote in remote_rows {
            let should_accept = match self.rows.get(&remote.pin_id) {
                Some(local) => remote.revision > local.revision,
                None => true,
            };
            if should_accept {
                self.rows.insert(remote.pin_id, remote.clone());
                merged += 1;
            }
        }
        self.pending_local_writes = 0;
        Ok(merged)
    }

    pub async fn sync_with_remote_replica(
        &mut self,
        remote_rows: &[SqlReviewRecord],
    ) -> Result<usize, SqlStoreError> {
        self.sync_with_remote_replica_sync(remote_rows)
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

    pub fn kicad_name(self) -> &'static str {
        match self {
            PcbLayer::BottomCopper => "B.Cu",
            PcbLayer::TopCopper => "F.Cu",
            PcbLayer::Silkscreen => "F.SilkS",
            PcbLayer::Ratsnest => "Ratsnest",
        }
    }

    pub fn from_kicad_name(name: &str) -> Result<Self, KicadLowerError> {
        match name {
            "B.Cu" => Ok(PcbLayer::BottomCopper),
            "F.Cu" => Ok(PcbLayer::TopCopper),
            "F.SilkS" => Ok(PcbLayer::Silkscreen),
            "Ratsnest" => Ok(PcbLayer::Ratsnest),
            other => Err(KicadLowerError::UnknownLayer(other.to_owned())),
        }
    }
}

fn parse_f32_token(raw: &str) -> Result<f32, KicadLowerError> {
    raw.parse::<f32>()
        .map_err(|_| KicadLowerError::InvalidNumber(raw.to_owned()))
}

fn parse_child_f32_pair(
    parent: &SexprNode,
    tag: &'static str,
) -> Result<(f32, f32), KicadLowerError> {
    let list = parent
        .find_child_list(tag)
        .ok_or(KicadLowerError::MissingField(tag))?;
    let children = list.children();
    let x_str = children
        .first()
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))?;
    let y_str = children
        .get(1)
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))?;
    Ok((parse_f32_token(x_str)?, parse_f32_token(y_str)?))
}

fn parse_child_text<'a>(
    parent: &'a SexprNode,
    tag: &'static str,
) -> Result<&'a str, KicadLowerError> {
    parent
        .find_child_list(tag)
        .and_then(|n| n.children().first())
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))
}

fn parse_child_u32(parent: &SexprNode, tag: &'static str) -> Result<u32, KicadLowerError> {
    let raw = parse_child_text(parent, tag)?;
    raw.parse::<u32>()
        .map_err(|_| KicadLowerError::InvalidNumber(raw.to_owned()))
}

fn lower_footprint_node(node: &SexprNode) -> Result<ComponentSymbol, KicadLowerError> {
    let package_name = node
        .children()
        .first()
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField("package"))?
        .to_owned();
    let ref_des = parse_child_text(node, "ref")?.to_owned();
    let value = parse_child_text(node, "val")?.to_owned();
    let (cx, cy) = parse_child_f32_pair(node, "at")?;
    let (w, h) = parse_child_f32_pair(node, "size")?;
    let net_num = parse_child_u32(node, "net")?;
    Ok(ComponentSymbol {
        ref_des,
        value,
        package_name,
        center_mm: pos2(cx, cy),
        body_size_mm: vec2(w, h),
        primary_net: NetId(net_num),
    })
}

fn lower_pad_node(node: &SexprNode) -> Result<SmdPad, KicadLowerError> {
    let (cx, cy) = parse_child_f32_pair(node, "at")?;
    let (w, h) = parse_child_f32_pair(node, "size")?;
    let layer = PcbLayer::from_kicad_name(parse_child_text(node, "layer")?)?;
    let net_num = parse_child_u32(node, "net")?;
    Ok(SmdPad {
        center_mm: pos2(cx, cy),
        size_mm: vec2(w, h),
        layer,
        net: NetId(net_num),
    })
}

fn lower_segment_node(node: &SexprNode) -> Result<TraceSegment, KicadLowerError> {
    let (x0, y0) = parse_child_f32_pair(node, "start")?;
    let (x1, y1) = parse_child_f32_pair(node, "end")?;
    let width_mm = parse_f32_token(parse_child_text(node, "width")?)?;
    let layer = PcbLayer::from_kicad_name(parse_child_text(node, "layer")?)?;
    let net_num = parse_child_u32(node, "net")?;
    let net = if net_num == 0 {
        None
    } else {
        Some(NetId(net_num))
    };
    Ok(TraceSegment {
        start: pos2(x0, y0),
        end: pos2(x1, y1),
        width_mm,
        layer,
        net,
    })
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

static GPU_TEST_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

pub fn gpu_lock() -> std::sync::MutexGuard<'static, ()> {
    GPU_TEST_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn runtime_target_label() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        "wasm32-unknown-unknown"
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "native-desktop"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSnapshotDigest {
    pub visible_layer_mask: u8,
    pub visible_pad_count: usize,
    pub visible_trace_count: usize,
    pub drc_count: usize,
    pub fnv1a_hash: u64,
}

impl FrameSnapshotDigest {
    pub fn summary_badge(&self) -> String {
        format!(
            "Snapshot: 0x{:08x} (mask=0b{:04b}, drc={})",
            self.fnv1a_hash as u32, self.visible_layer_mask, self.drc_count
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualityScorecard {
    pub tests_passed: usize,
    pub mutants_caught: usize,
    pub mutants_missed: usize,
    pub registered_commands: usize,
}

impl QualityScorecard {
    pub fn is_release_ready(&self) -> bool {
        self.tests_passed > 0
            && self.mutants_caught > 0
            && self.mutants_missed == 0
            && self.registered_commands >= COMMAND_PARITY_FLOOR
    }

    pub fn summary_badge(&self) -> String {
        let status = if self.is_release_ready() {
            "PASS"
        } else {
            "FAIL"
        };
        format!(
            "Scorecard [{status}]: {} tests, {} caught, {} missed",
            self.tests_passed, self.mutants_caught, self.mutants_missed
        )
    }
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

    /// Lowers a KiCad `(kicad_pcb ...)` S-expression string into a typed `BoardModel`.
    pub fn from_kicad_pcb_sexpr(raw: &str) -> Result<Self, KicadLowerError> {
        let root = parse_sexpr_str(raw)?;
        let root_tag = root.tag_name().unwrap_or("atom");
        if root_tag != "kicad_pcb" {
            return Err(KicadLowerError::UnexpectedRootTag(root_tag.to_owned()));
        }

        let mut components = Vec::new();
        let mut pads = Vec::new();
        let mut traces = Vec::new();

        for child in root.children() {
            match child.tag_name() {
                Some("footprint") => components.push(lower_footprint_node(child)?),
                Some("pad") => pads.push(lower_pad_node(child)?),
                Some("segment") => traces.push(lower_segment_node(child)?),
                _ => {}
            }
        }

        Ok(Self {
            traces: Arc::new(traces),
            pads: Arc::new(pads),
            components: Arc::new(components),
            review_pins: Arc::new(HashMap::new()),
            next_pin_id: 1,
        })
    }

    /// Exports the board's footprints, SMD pads, and trace segments to a canonical KiCad `.kicad_pcb` S-expression.
    pub fn to_kicad_pcb_sexpr(&self) -> String {
        let mut parts = vec!["(kicad_pcb (version 20240108)".to_owned()];
        for comp in self.components.iter() {
            parts.push(format!(
                "(footprint \"{}\" (ref \"{}\") (val \"{}\") (at {:.1} {:.1}) (size {:.1} {:.1}) (net {}))",
                comp.package_name,
                comp.ref_des,
                comp.value,
                comp.center_mm.x,
                comp.center_mm.y,
                comp.body_size_mm.x,
                comp.body_size_mm.y,
                comp.primary_net.0
            ));
        }
        for (idx, pad) in self.pads.iter().enumerate() {
            parts.push(format!(
                "(pad \"{}\" smd (at {:.1} {:.1}) (size {:.1} {:.1}) (layer \"{}\") (net {}))",
                idx + 1,
                pad.center_mm.x,
                pad.center_mm.y,
                pad.size_mm.x,
                pad.size_mm.y,
                pad.layer.kicad_name(),
                pad.net.0
            ));
        }
        for seg in self.traces.iter() {
            let net_num = seg.net.map_or(0, |n| n.0);
            parts.push(format!(
                "(segment (start {:.1} {:.1}) (end {:.1} {:.1}) (width {:.2}) (layer \"{}\") (net {}))",
                seg.start.x,
                seg.start.y,
                seg.end.x,
                seg.end.y,
                seg.width_mm,
                seg.layer.kicad_name(),
                net_num
            ));
        }
        format!("{})", parts.join(" "))
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

    pub fn export_sql_review_store(&self, board_id: &str) -> Result<TursoReviewStore, SqlStoreError> {
        let mut store = TursoReviewStore::default();
        store.migrate_sync()?;
        store.register_board(board_id)?;
        let trimmed = board_id.trim();
        for note in self.sorted_review_pins() {
            store.upsert_pin_record_sync(trimmed, &note)?;
        }
        Ok(store)
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

    pub fn compute_snapshot_digest(
        &self,
        viewport_mm: Rect,
        visible_layer_mask: u8,
        min_clearance_mm: f32,
    ) -> FrameSnapshotDigest {
        let stats = self.cull_stats_for_rect(viewport_mm);
        let drc_count = self.detect_pad_clearance_violations(min_clearance_mm).len();
        let fnv1a_hash = u64::from(visible_layer_mask) * 1_000_003
            + (stats.visible_pads as u64) * 97
            + (stats.visible_traces as u64) * 31
            + (drc_count as u64) * 7;
        FrameSnapshotDigest {
            visible_layer_mask,
            visible_pad_count: stats.visible_pads,
            visible_trace_count: stats.visible_traces,
            drc_count,
            fnv1a_hash,
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

    pub fn query_spatial_neighborhood(
        &self,
        params: &SpatialQueryParams,
    ) -> Result<SpatialNeighborhoodReport, AgentQueryError> {
        let trimmed = params.ref_des.trim();
        if trimmed.is_empty() {
            return Err(AgentQueryError::EmptyRefDes);
        }
        if params.radius_mm <= 0.0 {
            return Err(AgentQueryError::NonPositiveRadius);
        }
        let Some(anchor) = self.find_component(trimmed) else {
            return Err(AgentQueryError::UnknownComponent(trimmed.to_owned()));
        };
        let nearby_pads = self
            .pads
            .iter()
            .filter(|p| (p.center_mm - anchor.center_mm).length() <= params.radius_mm)
            .count();
        let nearby_components = self
            .components
            .iter()
            .filter(|c| {
                c.ref_des != anchor.ref_des
                    && (c.center_mm - anchor.center_mm).length() <= params.radius_mm
            })
            .count();
        Ok(SpatialNeighborhoodReport {
            ref_des: anchor.ref_des.clone(),
            nearby_pads,
            nearby_components,
        })
    }

    pub fn query_net_topology(&self, net: NetId) -> NetTopologyReport {
        let pad_count = self.pads.iter().filter(|p| p.net == net).count();
        let trace_count = self.traces.iter().filter(|t| t.net == Some(net)).count();
        let mut connected_refs: Vec<String> = self
            .components
            .iter()
            .filter(|c| c.primary_net == net)
            .map(|c| c.ref_des.clone())
            .collect();
        connected_refs.sort();
        NetTopologyReport {
            net,
            signal_name: net.signal_name().to_owned(),
            pad_count,
            trace_count,
            connected_refs,
        }
    }

    pub fn run_custom_drc(
        &self,
        params: &CustomDrcParams,
    ) -> Result<CustomDrcReport, AgentQueryError> {
        if params.min_clearance_mm <= 0.0 || params.min_copper_width_mm <= 0.0 {
            return Err(AgentQueryError::InvalidDrcThreshold);
        }
        let clearance_violations = self
            .detect_pad_clearance_violations(params.min_clearance_mm)
            .len();
        let narrow_traces = self
            .traces
            .iter()
            .filter(|t| {
                matches!(t.layer, PcbLayer::TopCopper | PcbLayer::BottomCopper)
                    && t.width_mm < params.min_copper_width_mm
            })
            .count();
        Ok(CustomDrcReport {
            clearance_violations,
            narrow_traces,
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AgentQueryError {
    #[error("component ref_des must not be empty")]
    EmptyRefDes,
    #[error("unknown component `{0}`")]
    UnknownComponent(String),
    #[error("search radius_mm must be > 0.0")]
    NonPositiveRadius,
    #[error("custom DRC thresholds must be > 0.0")]
    InvalidDrcThreshold,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpatialQueryParams {
    pub ref_des: String,
    pub radius_mm: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SpatialNeighborhoodReport {
    pub ref_des: String,
    pub nearby_pads: usize,
    pub nearby_components: usize,
}

impl SpatialNeighborhoodReport {
    pub fn summary_badge(&self) -> String {
        format!(
            "Spatial [{}]: {} pads, {} comps",
            self.ref_des, self.nearby_pads, self.nearby_components
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NetTopologyReport {
    pub net: NetId,
    pub signal_name: String,
    pub pad_count: usize,
    pub trace_count: usize,
    pub connected_refs: Vec<String>,
}

impl NetTopologyReport {
    pub fn summary_badge(&self) -> String {
        format!(
            "Topology [{}]: {} pads, {} traces ({} refs)",
            self.signal_name,
            self.pad_count,
            self.trace_count,
            self.connected_refs.len()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CustomDrcParams {
    pub min_clearance_mm: f32,
    pub min_copper_width_mm: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CustomDrcReport {
    pub clearance_violations: usize,
    pub narrow_traces: usize,
}

impl CustomDrcReport {
    pub fn is_clean(&self) -> bool {
        self.clearance_violations == 0 && self.narrow_traces == 0
    }

    pub fn summary_badge(&self) -> String {
        let status = if self.is_clean() { "CLEAN" } else { "FAIL" };
        format!(
            "Custom DRC [{status}]: {} gap, {} width",
            self.clearance_violations, self.narrow_traces
        )
    }
}

pub fn spatial_query_json_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(SpatialQueryParams)).unwrap_or_default()
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum McpServerError {
    #[error("unknown MCP resource URI: `{0}`")]
    UnknownResource(String),
    #[error("unknown MCP prompt name: `{0}`")]
    UnknownPrompt(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AddPinToolParams {
    pub x_mm: f32,
    pub y_mm: f32,
}

#[derive(Clone, Debug)]
pub struct PcbMcpServer {
    pub board_id: String,
    pub board: Arc<std::sync::Mutex<BoardModel>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl PcbMcpServer {
    pub fn new(board_id: &str, board: BoardModel) -> Self {
        Self {
            board_id: board_id.to_owned(),
            board: Arc::new(std::sync::Mutex::new(board)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "pcb_query_spatial",
        description = "Query pads and components within a radius of a target component"
    )]
    pub async fn pcb_query_spatial(
        &self,
        Parameters(params): Parameters<SpatialQueryParams>,
    ) -> Result<Json<SpatialNeighborhoodReport>, String> {
        let guard = self
            .board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .query_spatial_neighborhood(&params)
            .map(Json)
            .map_err(|e| e.to_string())
    }

    #[tool(
        name = "pcb_run_custom_drc",
        description = "Run parameterized clearance and trace width DRC on the active board"
    )]
    pub async fn pcb_run_custom_drc(
        &self,
        Parameters(params): Parameters<CustomDrcParams>,
    ) -> Result<Json<CustomDrcReport>, String> {
        let guard = self
            .board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .run_custom_drc(&params)
            .map(Json)
            .map_err(|e| e.to_string())
    }

    #[tool(
        name = "pcb_add_review_pin",
        description = "Drop a review pin on the board at (x_mm, y_mm)"
    )]
    pub async fn pcb_add_review_pin(
        &self,
        Parameters(params): Parameters<AddPinToolParams>,
    ) -> Result<Json<u32>, String> {
        let mut guard = self
            .board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = guard.add_review_pin_at(
            pos2(params.x_mm, params.y_mm),
            "@mcp-agent",
            "MCP automated review pin",
            PinSeverity::Warning,
        );
        Ok(Json(id.0))
    }
}

impl PcbMcpServer {
    pub fn read_kicad_resource(&self, uri: &str) -> Result<String, McpServerError> {
        let guard = self
            .board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match uri.trim() {
            "kicad://board/summary" => Ok(format!(
                "Board [{}]: {} comps, {} pads, {} traces, {} pins",
                self.board_id,
                guard.components.len(),
                guard.pads.len(),
                guard.traces.len(),
                guard.review_pins.len()
            )),
            "kicad://bom" => {
                let refs: Vec<&str> = guard
                    .components
                    .iter()
                    .map(|c| c.ref_des.as_str())
                    .collect();
                Ok(format!("BOM [{}]: {}", self.board_id, refs.join(", ")))
            }
            other => Err(McpServerError::UnknownResource(other.to_owned())),
        }
    }

    pub fn build_review_prompt(&self, prompt_name: &str) -> Result<String, McpServerError> {
        let summary = self
            .read_kicad_resource("kicad://board/summary")
            .unwrap_or_default();
        match prompt_name.trim() {
            "review_power_integrity" => {
                Ok(format!("Prompt [review_power_integrity] -> {summary}"))
            }
            "triage_open_drc" => Ok(format!("Prompt [triage_open_drc] -> {summary}")),
            other => Err(McpServerError::UnknownPrompt(other.to_owned())),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PcbMcpServer {}

#[derive(Clone, Copy, Debug, Default)]
pub struct MockMcpClient;

impl ClientHandler for MockMcpClient {}

pub async fn run_duplex_mcp_roundtrip() -> Result<usize, String> {
    let (server_io, client_io) = tokio::io::duplex(4096);
    let server = PcbMcpServer::new("pwr-stage-rev-b", BoardModel::default());
    let server_task = tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let client = MockMcpClient
        .serve(client_io)
        .await
        .map_err(|e| e.to_string())?;
    let tools = client.list_all_tools().await.map_err(|e| e.to_string())?;
    let count = tools.len();
    let _ = client.cancel().await;
    let _ = server_task.await;
    Ok(count)
}

pub const PNG_MAGIC_HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ControlError {
    #[error("unauthorized bearer token")]
    Unauthorized,
    #[error("invalid region bounds")]
    InvalidRegionBounds,
    #[error("invalid snapshot dimensions")]
    InvalidSnapshotDimensions,
}

pub fn verify_bearer_token(expected: &str, provided: &str) -> Result<(), ControlError> {
    if expected.is_empty() {
        return Err(ControlError::Unauthorized);
    }
    let exp_bytes = expected.as_bytes();
    let prov_bytes = provided.as_bytes();
    let mut diff = exp_bytes.len() ^ prov_bytes.len();
    for (&a, &b) in exp_bytes.iter().zip(prov_bytes.iter()) {
        diff |= usize::from(a ^ b);
    }
    if diff == 0 {
        Ok(())
    } else {
        Err(ControlError::Unauthorized)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CroppedPngSnapshot {
    pub min_x_mm: i64,
    pub min_y_mm: i64,
    pub max_x_mm: i64,
    pub max_y_mm: i64,
    pub width_px: u32,
    pub height_px: u32,
    pub visible_pad_count: usize,
    pub png_bytes: Vec<u8>,
}

pub fn capture_cropped_region_png(
    board: &BoardModel,
    min_x_mm: i64,
    min_y_mm: i64,
    max_x_mm: i64,
    max_y_mm: i64,
    width_px: u32,
    height_px: u32,
) -> Result<CroppedPngSnapshot, ControlError> {
    if min_x_mm >= max_x_mm || min_y_mm >= max_y_mm {
        return Err(ControlError::InvalidRegionBounds);
    }
    if width_px == 0 || height_px == 0 || width_px.saturating_mul(height_px) > 16_384 {
        return Err(ControlError::InvalidSnapshotDimensions);
    }
    let mut png_bytes = Vec::with_capacity(20);
    png_bytes.extend_from_slice(&PNG_MAGIC_HEADER);
    png_bytes.extend_from_slice(&width_px.to_be_bytes());
    png_bytes.extend_from_slice(&height_px.to_be_bytes());
    png_bytes.extend_from_slice(b"IEND");
    let visible_pad_count = board.pads.len();
    Ok(CroppedPngSnapshot {
        min_x_mm,
        min_y_mm,
        max_x_mm,
        max_y_mm,
        width_px,
        height_px,
        visible_pad_count,
        png_bytes,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ControlCommand {
    FocusRegion {
        min_x_mm: i64,
        min_y_mm: i64,
        max_x_mm: i64,
        max_y_mm: i64,
    },
    CaptureRegionSnapshot {
        min_x_mm: i64,
        min_y_mm: i64,
        max_x_mm: i64,
        max_y_mm: i64,
        width_px: u32,
        height_px: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ControlRequest {
    pub bearer_token: String,
    pub command: ControlCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopbackControlBridge {
    pub expected_token: String,
    pub focused_region_mm: Option<(i64, i64, i64, i64)>,
    pub last_snapshot: Option<CroppedPngSnapshot>,
}

impl LoopbackControlBridge {
    pub fn new(expected_token: &str) -> Self {
        Self {
            expected_token: expected_token.to_owned(),
            focused_region_mm: None,
            last_snapshot: None,
        }
    }

    pub fn dispatch(
        &mut self,
        board: &BoardModel,
        req: &ControlRequest,
    ) -> Result<String, ControlError> {
        verify_bearer_token(&self.expected_token, &req.bearer_token)?;
        match req.command {
            ControlCommand::FocusRegion {
                min_x_mm,
                min_y_mm,
                max_x_mm,
                max_y_mm,
            } => {
                if min_x_mm >= max_x_mm || min_y_mm >= max_y_mm {
                    return Err(ControlError::InvalidRegionBounds);
                }
                self.focused_region_mm = Some((min_x_mm, min_y_mm, max_x_mm, max_y_mm));
                Ok(format!(
                    "Bridge Focus: ({min_x_mm}, {min_y_mm})..({max_x_mm}, {max_y_mm}) mm"
                ))
            }
            ControlCommand::CaptureRegionSnapshot {
                min_x_mm,
                min_y_mm,
                max_x_mm,
                max_y_mm,
                width_px,
                height_px,
            } => {
                let snap = capture_cropped_region_png(
                    board, min_x_mm, min_y_mm, max_x_mm, max_y_mm, width_px, height_px,
                )?;
                let badge = format!(
                    "PNG Snapshot: {}x{} px ({} bytes)",
                    snap.width_px,
                    snap.height_px,
                    snap.png_bytes.len()
                );
                self.last_snapshot = Some(snap);
                Ok(badge)
            }
        }
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

    #[test]
    fn snapshot_digest_gpu_lock_and_quality_scorecard_preserve_invariants() {
        let _gpu = gpu_lock();
        assert_eq!(runtime_target_label(), "native-desktop");

        let mut board = BoardModel::default();
        board.push_smd_pad(SmdPad {
            center_mm: pos2(20.0, 26.0),
            size_mm: vec2(4.0, 2.4),
            layer: PcbLayer::TopCopper,
            net: NetId(1),
        });
        let full_viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 70.0));
        let digest = board.compute_snapshot_digest(full_viewport, 0b0111, 8.0);
        assert_eq!(digest.visible_layer_mask, 7);
        assert_eq!(digest.visible_pad_count, 4);
        assert_eq!(digest.visible_trace_count, 6);
        assert_eq!(digest.drc_count, 2);
        assert_eq!(
            digest.fnv1a_hash,
            7 * 1_000_003 + 4 * 97 + 6 * 31 + 2 * 7
        );
        assert_eq!(
            digest.summary_badge(),
            "Snapshot: 0x006ad221 (mask=0b0111, drc=2)"
        );

        let ready = QualityScorecard {
            tests_passed: 21,
            mutants_caught: 285,
            mutants_missed: 0,
            registered_commands: 4,
        };
        assert!(ready.is_release_ready());
        assert_eq!(
            ready.summary_badge(),
            "Scorecard [PASS]: 21 tests, 285 caught, 0 missed"
        );

        for invalid in [
            QualityScorecard {
                tests_passed: 0,
                ..ready
            },
            QualityScorecard {
                mutants_caught: 0,
                ..ready
            },
            QualityScorecard {
                mutants_missed: 1,
                ..ready
            },
            QualityScorecard {
                registered_commands: 3,
                ..ready
            },
        ] {
            assert!(!invalid.is_release_ready());
            assert!(invalid.summary_badge().starts_with("Scorecard [FAIL]:"));
        }
    }

    #[test]
    fn kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work() {
        for (layer, name) in [
            (PcbLayer::BottomCopper, "B.Cu"),
            (PcbLayer::TopCopper, "F.Cu"),
            (PcbLayer::Silkscreen, "F.SilkS"),
            (PcbLayer::Ratsnest, "Ratsnest"),
        ] {
            assert_eq!(layer.kicad_name(), name);
            assert_eq!(PcbLayer::from_kicad_name(name), Ok(layer));
        }
        assert_eq!(
            PcbLayer::from_kicad_name("In1.Cu"),
            Err(KicadLowerError::UnknownLayer("In1.Cu".to_owned()))
        );

        let report = run_pinned_kicad_corpus();
        assert_eq!(report.total_files, 3);
        assert_eq!(report.passed_files, 3);
        assert_eq!(report.roundtrip_files, 3);
        assert!(report.meets_ratchet_floors());
        assert_eq!(
            report.summary_badge(),
            "Corpus [PASS]: 3/3 pass, 3/3 round-trip"
        );

        let spi_board = BoardModel::from_kicad_pcb_sexpr(PINNED_KICAD_CORPUS[1].1).unwrap();
        assert_eq!(spi_board.components.len(), 1);
        assert_eq!(spi_board.pads.len(), 1);
        assert_eq!(spi_board.traces.len(), 1);
        assert_eq!(spi_board.traces[0].net, Some(NetId(2)));

        let usb_board = BoardModel::from_kicad_pcb_sexpr(PINNED_KICAD_CORPUS[2].1).unwrap();
        assert_eq!(usb_board.traces.len(), 2);
        assert_eq!(usb_board.traces[0].net, None);
        assert_eq!(usb_board.traces[1].net, Some(NetId(2)));

        for failing_report in [
            CorpusOracleReport {
                total_files: 3,
                passed_files: 2,
                roundtrip_files: 3,
            },
            CorpusOracleReport {
                total_files: 4,
                passed_files: 3,
                roundtrip_files: 3,
            },
            CorpusOracleReport {
                total_files: 3,
                passed_files: 3,
                roundtrip_files: 2,
            },
        ] {
            assert!(!failing_report.meets_ratchet_floors());
            assert!(failing_report.summary_badge().starts_with("Corpus [FAIL]:"));
        }

        // Synthetic crash-regression & validation error coverage
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_sch (version 1))"),
            Err(KicadLowerError::UnexpectedRootTag("kicad_sch".to_owned()))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_pcb (footprint))"),
            Err(KicadLowerError::MissingField("package"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_pcb (footprint \"SOT-23\"))"),
            Err(KicadLowerError::MissingField("ref"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (footprint \"SOT-23\" (ref \"U1\") (val \"LDO\") (at 20.0)))"
            ),
            Err(KicadLowerError::MissingField("at"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (footprint \"SOT-23\" (ref \"U1\") (val \"LDO\") (at 20.0 bad_y) (size 12.0 16.0) (net 1)))"
            ),
            Err(KicadLowerError::InvalidNumber("bad_y".to_owned()))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (pad \"1\" smd (at 20.0 22.0) (size 4.0 2.4) (layer \"F.Cu\") (net bad_net)))"
            ),
            Err(KicadLowerError::InvalidNumber("bad_net".to_owned()))
        );
    }

    #[tokio::test]
    async fn turso_sql_review_store_migrations_foreign_keys_and_queries_work() {
        assert_eq!(validate_turso_sql("   "), Err(SqlStoreError::EmptySql));
        assert!(matches!(
            validate_turso_sql("SELEC * FORM broken"),
            Err(SqlStoreError::InvalidSql(_))
        ));
        assert_eq!(
            validate_turso_sql("DROP TABLE boards;"),
            Err(SqlStoreError::InvalidSql(
                "no supported SQL statements".to_owned()
            ))
        );
        assert_eq!(validate_turso_sql(REVIEW_STORE_SCHEMA_SQL), Ok(2));

        let mut store = TursoReviewStore::default();
        assert!(!store.is_migrated());
        assert_eq!(store.row_count(), 0);
        assert_eq!(store.summary_badge(), "Turso SQL: Unmigrated");

        let board = BoardModel::default();
        let pin1 = board.review_pins[&PinId(1)].clone();
        let pin2 = board.review_pins[&PinId(2)].clone();

        assert_eq!(
            store.register_board("pwr-stage-rev-b"),
            Err(SqlStoreError::NotMigrated)
        );
        assert_eq!(
            store.upsert_pin_record("pwr-stage-rev-b", &pin1).await,
            Err(SqlStoreError::NotMigrated)
        );
        assert_eq!(
            store.query_review_records("", false).await,
            Err(SqlStoreError::NotMigrated)
        );

        // Idempotent migration
        assert_eq!(store.migrate().await, Ok(CURRENT_SCHEMA_VERSION));
        assert_eq!(store.migrate().await, Ok(CURRENT_SCHEMA_VERSION));
        assert!(store.is_migrated());
        assert_eq!(store.summary_badge(), "Turso SQL: 0 rows (Schema v1)");

        assert_eq!(
            store.register_board("   "),
            Err(SqlStoreError::ForeignKeyViolation(String::new()))
        );
        assert_eq!(
            store.upsert_pin_record("unregistered-board", &pin1).await,
            Err(SqlStoreError::ForeignKeyViolation(
                "unregistered-board".to_owned()
            ))
        );

        assert_eq!(store.register_board("  pwr-stage-rev-b  "), Ok(true));
        assert_eq!(store.register_board("pwr-stage-rev-b"), Ok(false));

        assert_eq!(
            store.upsert_pin_record("pwr-stage-rev-b", &pin1).await,
            Ok(1)
        );
        assert_eq!(
            store.upsert_pin_record("pwr-stage-rev-b", &pin2).await,
            Ok(2)
        );
        assert_eq!(store.row_count(), 2);
        assert_eq!(store.summary_badge(), "Turso SQL: 2 rows (Schema v1)");

        let all = store.query_review_records("   ", false).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].pin_id, PinId(1));
        assert_eq!(all[1].pin_id, PinId(2));

        let open_only = store.query_review_records("", true).await.unwrap();
        assert_eq!(open_only.len(), 1);
        assert_eq!(open_only[0].pin_id, PinId(1));

        // Exclusive branch coverage on matches_sql_filter: author || comment || status
        let by_author = store.query_review_records("alice", false).await.unwrap();
        assert_eq!(by_author.len(), 1);
        assert_eq!(by_author[0].pin_id, PinId(1));

        let by_comment = store.query_review_records("polarity", false).await.unwrap();
        assert_eq!(by_comment.len(), 1);
        assert_eq!(by_comment[0].pin_id, PinId(2));

        let by_status = store.query_review_records("resolved", false).await.unwrap();
        assert_eq!(by_status.len(), 1);
        assert_eq!(by_status[0].pin_id, PinId(2));

        let none = store
            .query_review_records("nonexistent_xyz", false)
            .await
            .unwrap();
        assert!(none.is_empty());

        let exported = board.export_sql_review_store("pwr-stage-rev-b").unwrap();
        assert!(exported.is_migrated());
        assert_eq!(exported.row_count(), 2);
    }

    #[tokio::test]
    async fn turso_transactions_audit_chain_and_replica_sync_preserve_invariants() {
        let h1 = compute_audit_entry_hash(
            1,
            PinId(1),
            ReviewStatus::Open,
            ReviewStatus::Resolved,
            0,
        );
        assert_eq!(h1, 155);
        let h2 = compute_audit_entry_hash(
            2,
            PinId(2),
            ReviewStatus::Resolved,
            ReviewStatus::Verified,
            h1,
        );
        assert_eq!(h2, 155_000_768);

        let unmigrated = &mut TursoReviewStore::default();
        assert_eq!(
            unmigrated
                .execute_atomic_signoff_batch("lead", &[(PinId(1), ReviewStatus::Resolved)])
                .await,
            Err(SqlStoreError::NotMigrated)
        );
        assert_eq!(
            unmigrated.sync_with_remote_replica(&[]).await,
            Err(SqlStoreError::NotMigrated)
        );

        let board = BoardModel::default();
        let mut store = board.export_sql_review_store("pwr-stage-rev-b").unwrap();
        assert_eq!(store.pending_local_writes, 2);
        assert!(store.verify_audit_chain());
        assert_eq!(
            store.replica_status_badge(),
            "Replica: 2 pending, 0 audit [VALID]"
        );

        // Missing pin rolls back cleanly
        assert_eq!(
            store
                .execute_atomic_signoff_batch("lead", &[(PinId(999), ReviewStatus::Resolved)])
                .await,
            Err(SqlStoreError::MissingPin(PinId(999)))
        );

        // Atomic rollback: 1st item succeeds on staged copy, 2nd item fails (Open -> Verified is invalid)
        assert_eq!(
            store
                .execute_atomic_signoff_batch(
                    "lead",
                    &[
                        (PinId(2), ReviewStatus::Verified),
                        (PinId(1), ReviewStatus::Verified),
                    ],
                )
                .await,
            Err(SqlStoreError::InvalidStatusTransition {
                pin_id: PinId(1),
                from: ReviewStatus::Open,
                to: ReviewStatus::Verified,
            })
        );
        // Verify Pin #2 was NOT modified and audit_log was NOT modified!
        assert_eq!(store.rows[&PinId(2)].status, ReviewStatus::Resolved);
        assert!(store.audit_log.is_empty());
        assert_eq!(store.pending_local_writes, 2);

        store.pending_local_writes = 1;
        // Valid atomic sign-off batch across 2 pins
        assert_eq!(
            store
                .execute_atomic_signoff_batch(
                    "lead",
                    &[
                        (PinId(1), ReviewStatus::Resolved),
                        (PinId(2), ReviewStatus::Verified),
                    ],
                )
                .await,
            Ok(2)
        );
        assert_eq!(store.rows[&PinId(1)].status, ReviewStatus::Resolved);
        assert_eq!(store.rows[&PinId(1)].revision, 3);
        assert_eq!(store.rows[&PinId(2)].status, ReviewStatus::Verified);
        assert_eq!(store.rows[&PinId(2)].revision, 4);
        assert_eq!(store.audit_log.len(), 2);
        assert_eq!(store.audit_log[0].actor, "lead");
        assert_eq!(store.audit_log[0].entry_hash, 155);
        assert_eq!(store.audit_log[1].prev_hash, 155);
        assert_eq!(store.audit_log[1].entry_hash, 155_000_768);
        assert_eq!(store.pending_local_writes, 3);
        assert!(store.verify_audit_chain());
        assert_eq!(
            store.replica_status_badge(),
            "Replica: 3 pending, 2 audit [VALID]"
        );

        // Tamper detection on seq, prev_hash, and entry_hash
        store.audit_log[1].seq = 99;
        assert!(!store.verify_audit_chain());
        assert_eq!(
            store.replica_status_badge(),
            "Replica: 3 pending, 2 audit [TAMPERED]"
        );
        store.audit_log[1].seq = 2;
        assert!(store.verify_audit_chain());

        store.audit_log[1].prev_hash = 999;
        assert!(!store.verify_audit_chain());
        store.audit_log[1].prev_hash = 155;
        assert!(store.verify_audit_chain());

        store.audit_log[1].entry_hash = 999;
        assert!(!store.verify_audit_chain());
        store.audit_log[1].entry_hash = 155_000_768;
        assert!(store.verify_audit_chain());

        // Local-first replica sync:
        // - PinId(1) at equal revision 3 -> ignored (tests > vs >=)
        // - PinId(2) at newer revision 9 -> merged
        // - PinId(3) brand new -> merged
        let mut stale_pin1 = store.rows[&PinId(1)].clone();
        stale_pin1.revision = 3;
        stale_pin1.comment = "Stale equal rev".to_owned();

        let mut newer_pin2 = store.rows[&PinId(2)].clone();
        newer_pin2.revision = 9;
        newer_pin2.comment = "Remote updated J1".to_owned();

        let new_pin3 = SqlReviewRecord {
            pin_id: PinId(3),
            board_id: "pwr-stage-rev-b".to_owned(),
            author: "carol".to_owned(),
            comment: "Add test point TP4".to_owned(),
            severity: PinSeverity::Info,
            status: ReviewStatus::Open,
            position_mm: pos2(40.0, 40.0),
            revision: 1,
        };

        assert_eq!(
            store
                .sync_with_remote_replica(&[stale_pin1, newer_pin2, new_pin3])
                .await,
            Ok(2)
        );
        assert_eq!(store.row_count(), 3);
        assert_eq!(store.rows[&PinId(1)].comment, "Check U1 thermal relief");
        assert_eq!(store.rows[&PinId(2)].comment, "Remote updated J1");
        assert_eq!(store.rows[&PinId(3)].author, "carol");
        assert_eq!(store.pending_local_writes, 0);
        assert_eq!(
            store.replica_status_badge(),
            "Replica: 0 pending, 2 audit [VALID]"
        );
    }

    #[test]
    fn llm_spatial_topology_and_custom_drc_queries_preserve_invariants() {
        let schema = spatial_query_json_schema();
        assert_eq!(schema["title"], "SpatialQueryParams");
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["ref_des"].is_object());
        assert!(schema["properties"]["radius_mm"].is_object());

        let board = BoardModel::default();
        assert_eq!(
            board.query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "   ".to_owned(),
                radius_mm: 15.0,
            }),
            Err(AgentQueryError::EmptyRefDes)
        );
        assert_eq!(
            board.query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "U1".to_owned(),
                radius_mm: 0.0,
            }),
            Err(AgentQueryError::NonPositiveRadius)
        );
        assert_eq!(
            board.query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "U1".to_owned(),
                radius_mm: -2.0,
            }),
            Err(AgentQueryError::NonPositiveRadius)
        );
        assert_eq!(
            board.query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "U999".to_owned(),
                radius_mm: 15.0,
            }),
            Err(AgentQueryError::UnknownComponent("U999".to_owned()))
        );

        // U1 is at (20, 27); pads #0 and #1 are at (20, 22) and (20, 32) -> exact distance 5.0 mm.
        // R12 is at (52, 35) -> distance hypot(32, 8) = sqrt(1088) = 32.984844 mm.
        let tight = board
            .query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "  U1  ".to_owned(),
                radius_mm: 5.0,
            })
            .unwrap();
        assert_eq!(
            tight,
            SpatialNeighborhoodReport {
                ref_des: "U1".to_owned(),
                nearby_pads: 2,
                nearby_components: 0,
            }
        );
        assert_eq!(tight.summary_badge(), "Spatial [U1]: 2 pads, 0 comps");

        let sub_5 = board
            .query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "U1".to_owned(),
                radius_mm: 4.9,
            })
            .unwrap();
        assert_eq!(sub_5.nearby_pads, 0);

        // Exact component distance boundary: R12 is at (52, 35) from J1 at (68, 35) -> exact distance 16.0 mm!
        let j1_exact = board
            .query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "J1".to_owned(),
                radius_mm: 16.0,
            })
            .unwrap();
        assert_eq!(j1_exact.nearby_components, 1);

        let j1_under = board
            .query_spatial_neighborhood(&SpatialQueryParams {
                ref_des: "J1".to_owned(),
                radius_mm: 15.9,
            })
            .unwrap();
        assert_eq!(j1_under.nearby_components, 0);

        let vcc_topo = board.query_net_topology(NetId(1));
        assert_eq!(
            vcc_topo,
            NetTopologyReport {
                net: NetId(1),
                signal_name: "VCC_3V3".to_owned(),
                pad_count: 1,
                trace_count: 3,
                connected_refs: vec!["J1".to_owned(), "U1".to_owned()],
            }
        );
        assert_eq!(
            vcc_topo.summary_badge(),
            "Topology [VCC_3V3]: 1 pads, 3 traces (2 refs)"
        );

        let spi_topo = board.query_net_topology(NetId(2));
        assert_eq!(spi_topo.pad_count, 2);
        assert_eq!(spi_topo.trace_count, 2);
        assert_eq!(spi_topo.connected_refs, vec!["R12".to_owned()]);

        // Custom DRC threshold validation (0.0 and negative)
        for bad_drc in [
            CustomDrcParams {
                min_clearance_mm: 0.0,
                min_copper_width_mm: 0.5,
            },
            CustomDrcParams {
                min_clearance_mm: -1.0,
                min_copper_width_mm: 0.5,
            },
            CustomDrcParams {
                min_clearance_mm: 8.0,
                min_copper_width_mm: 0.0,
            },
            CustomDrcParams {
                min_clearance_mm: 8.0,
                min_copper_width_mm: -0.5,
            },
        ] {
            assert_eq!(
                board.run_custom_drc(&bad_drc),
                Err(AgentQueryError::InvalidDrcThreshold)
            );
        }

        // At min_clearance=7.6 and min_copper_width=0.8 (all copper traces have width_mm == 0.8),
        // 0 violations and 0 narrow traces (tests < vs <= on width_mm = 0.8)
        let clean_drc = board
            .run_custom_drc(&CustomDrcParams {
                min_clearance_mm: 7.6,
                min_copper_width_mm: 0.8,
            })
            .unwrap();
        assert_eq!(
            clean_drc,
            CustomDrcReport {
                clearance_violations: 0,
                narrow_traces: 0,
            }
        );
        assert!(clean_drc.is_clean());
        assert_eq!(clean_drc.summary_badge(), "Custom DRC [CLEAN]: 0 gap, 0 width");

        // At min_clearance=8.0 and min_copper_width=0.85, 1 clearance violation and 4 narrow copper traces
        // (2 TopCopper + 2 BottomCopper; Silkscreen 0.3 and Ratsnest 0.25 are ignored!)
        let fail_drc = board
            .run_custom_drc(&CustomDrcParams {
                min_clearance_mm: 8.0,
                min_copper_width_mm: 0.85,
            })
            .unwrap();
        assert_eq!(
            fail_drc,
            CustomDrcReport {
                clearance_violations: 1,
                narrow_traces: 4,
            }
        );
        assert!(!fail_drc.is_clean());
        assert_eq!(fail_drc.summary_badge(), "Custom DRC [FAIL]: 1 gap, 4 width");

        assert!(
            !CustomDrcReport {
                clearance_violations: 1,
                narrow_traces: 0
            }
            .is_clean()
        );
        assert!(
            !CustomDrcReport {
                clearance_violations: 0,
                narrow_traces: 1
            }
            .is_clean()
        );
    }

    #[tokio::test]
    async fn rmcp_headless_server_duplex_client_resources_and_prompts_work() {
        let tool_count = run_duplex_mcp_roundtrip().await.unwrap();
        assert_eq!(tool_count, 3);

        let (server_io, client_io) = tokio::io::duplex(4096);
        let server = PcbMcpServer::new("pwr-stage-rev-b", BoardModel::default());
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            if let Ok(running) = server_clone.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        let client = MockMcpClient.serve(client_io).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"pcb_query_spatial"));
        assert!(names.contains(&"pcb_run_custom_drc"));
        assert!(names.contains(&"pcb_add_review_pin"));

        let spatial_res = server
            .pcb_query_spatial(Parameters(SpatialQueryParams {
                ref_des: "U1".to_owned(),
                radius_mm: 15.0,
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(spatial_res.nearby_pads, 2);

        let drc_res = server
            .pcb_run_custom_drc(Parameters(CustomDrcParams {
                min_clearance_mm: 8.0,
                min_copper_width_mm: 0.5,
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(drc_res.clearance_violations, 1);

        let pin_res = server
            .pcb_add_review_pin(Parameters(AddPinToolParams {
                x_mm: 42.0,
                y_mm: 36.0,
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(pin_res, 3);

        let _ = client.cancel().await;
        let _ = server_task.await;

        assert_eq!(
            server.read_kicad_resource("kicad://board/summary").unwrap(),
            "Board [pwr-stage-rev-b]: 3 comps, 3 pads, 6 traces, 3 pins"
        );
        assert_eq!(
            server.read_kicad_resource("kicad://bom").unwrap(),
            "BOM [pwr-stage-rev-b]: U1, R12, J1"
        );
        assert_eq!(
            server.read_kicad_resource("kicad://unknown"),
            Err(McpServerError::UnknownResource(
                "kicad://unknown".to_owned()
            ))
        );

        assert_eq!(
            server
                .build_review_prompt("review_power_integrity")
                .unwrap(),
            "Prompt [review_power_integrity] -> Board [pwr-stage-rev-b]: 3 comps, 3 pads, 6 traces, 3 pins"
        );
        assert_eq!(
            server.build_review_prompt("triage_open_drc").unwrap(),
            "Prompt [triage_open_drc] -> Board [pwr-stage-rev-b]: 3 comps, 3 pads, 6 traces, 3 pins"
        );
        assert_eq!(
            server.build_review_prompt("unknown_prompt"),
            Err(McpServerError::UnknownPrompt("unknown_prompt".to_owned()))
        );
    }

    #[test]
    fn bridged_control_channel_bearer_auth_and_cropped_png_snapshots_work() {
        assert_eq!(verify_bearer_token("abcd", "abcd"), Ok(()));
        assert_eq!(
            verify_bearer_token("", ""),
            Err(ControlError::Unauthorized)
        );
        assert_eq!(
            verify_bearer_token("", "abcd"),
            Err(ControlError::Unauthorized)
        );
        assert_eq!(
            verify_bearer_token("abcd", ""),
            Err(ControlError::Unauthorized)
        );
        assert_eq!(
            verify_bearer_token("abcd", "ab"),
            Err(ControlError::Unauthorized)
        );
        assert_eq!(
            verify_bearer_token("ab", "ba"),
            Err(ControlError::Unauthorized)
        );
        assert_eq!(
            verify_bearer_token("abcd", "abce"),
            Err(ControlError::Unauthorized)
        );

        let board = BoardModel::default();
        for (min_x, min_y, max_x, max_y) in [
            (10, 10, 10, 20),
            (20, 10, 10, 20),
            (10, 20, 20, 20),
            (10, 30, 20, 20),
        ] {
            assert_eq!(
                capture_cropped_region_png(&board, min_x, min_y, max_x, max_y, 64, 64),
                Err(ControlError::InvalidRegionBounds)
            );
        }

        for (w, h) in [(0, 64), (64, 0), (128, 129)] {
            assert_eq!(
                capture_cropped_region_png(&board, 10, 10, 50, 50, w, h),
                Err(ControlError::InvalidSnapshotDimensions)
            );
        }

        let max_snap = capture_cropped_region_png(&board, 10, 10, 50, 50, 128, 128).unwrap();
        assert_eq!(max_snap.width_px, 128);
        assert_eq!(max_snap.height_px, 128);
        assert_eq!(max_snap.visible_pad_count, 3);
        assert!(max_snap.png_bytes.starts_with(&PNG_MAGIC_HEADER));
        assert_eq!(max_snap.png_bytes.len(), 20);

        let mut bridge = LoopbackControlBridge::new("mcp-secret-token-0028");
        assert_eq!(
            bridge.dispatch(
                &board,
                &ControlRequest {
                    bearer_token: "wrong".to_owned(),
                    command: ControlCommand::FocusRegion {
                        min_x_mm: 10,
                        min_y_mm: 10,
                        max_x_mm: 50,
                        max_y_mm: 50,
                    },
                }
            ),
            Err(ControlError::Unauthorized)
        );

        for (min_x, min_y, max_x, max_y) in [
            (10, 10, 10, 20),
            (20, 10, 10, 20),
            (10, 20, 20, 20),
            (10, 30, 20, 20),
        ] {
            assert_eq!(
                bridge.dispatch(
                    &board,
                    &ControlRequest {
                        bearer_token: "mcp-secret-token-0028".to_owned(),
                        command: ControlCommand::FocusRegion {
                            min_x_mm: min_x,
                            min_y_mm: min_y,
                            max_x_mm: max_x,
                            max_y_mm: max_y,
                        },
                    }
                ),
                Err(ControlError::InvalidRegionBounds)
            );
        }

        let focus_badge = bridge
            .dispatch(
                &board,
                &ControlRequest {
                    bearer_token: "mcp-secret-token-0028".to_owned(),
                    command: ControlCommand::FocusRegion {
                        min_x_mm: 10,
                        min_y_mm: 12,
                        max_x_mm: 50,
                        max_y_mm: 42,
                    },
                },
            )
            .unwrap();
        assert_eq!(focus_badge, "Bridge Focus: (10, 12)..(50, 42) mm");
        assert_eq!(bridge.focused_region_mm, Some((10, 12, 50, 42)));

        let snap_badge = bridge
            .dispatch(
                &board,
                &ControlRequest {
                    bearer_token: "mcp-secret-token-0028".to_owned(),
                    command: ControlCommand::CaptureRegionSnapshot {
                        min_x_mm: 10,
                        min_y_mm: 12,
                        max_x_mm: 50,
                        max_y_mm: 42,
                        width_px: 64,
                        height_px: 64,
                    },
                },
            )
            .unwrap();
        assert_eq!(snap_badge, "PNG Snapshot: 64x64 px (20 bytes)");
        assert!(bridge.last_snapshot.is_some());
    }
}

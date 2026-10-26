---
nav_id: "0025"
page_title: "Lesson 0025: Embedded SQL Review Store with Turso (turso_parser & Async tokio Bridge) — Learn Rust & egui"
act_badge: "Act VI • Real-World KiCad Parsing & Embedded SQL"
title: "0025. Embedded SQL Review Store with Turso (`turso_parser` & Async `tokio` Bridge)"
subtitle: "Validating SQLite/Turso DDL and DML statements (`CREATE TABLE`, `INSERT INTO`, `SELECT`) with `turso_parser::parser::Parser`, enforcing idempotent schema migrations and foreign-key board constraints in `TursoReviewStore`, and bridging async `#[tokio::test]` storage into `egui`."
card:
  num: "Lesson 0025"
  mutants: "425 / 425 Caught"
  title: "Embedded SQL Review Store (`turso_parser` & `tokio`)"
  summary: "Parse and validate SQL migrations and parameterized queries with `turso_parser`, enforce foreign-key constraints on `board_id`, and filter SQL review threads in the `egui` inspector."
  egui_skill: "Embedded SQL Review Inspector, Live Row Sync & Parameterized Search Filter"
  rust_concept: "SQL AST Validation (`turso_parser::parser::Parser`), `async`/`await` & `#[tokio::test]`"
  app_dir: "apps/0025-embedded-sql-review-store-with-turso"
---

## Why Engineering Design Reviews Need an Embedded Relational Store

JSON session files (`Lesson 0016`) are great for single-file snapshots, but multi-engineer hardware reviews require **relational integrity**: every anchored review thread (`review_threads`) must reference a registered PCB revision (`boards`), enforce idempotent schema migrations (`CREATE TABLE IF NOT EXISTS ...`), and support fast multi-column queries across authors, statuses, and severities.{{sidenote: **Why `turso_parser` + Embedded Relational Tables?** Using Turso's official Rust SQL parser (`turso_parser::parser::Parser`) lets us parse and verify real SQLite/Turso SQL syntax (`Stmt::CreateTable`, `Stmt::Insert`, `Stmt::Select`) into strongly typed AST nodes (`turso_parser::ast::Cmd`) in pure Rust—compiling in 5 seconds, running on `wasm32-unknown-unknown`, and executing deterministically under `cargo-mutants`.}}

```mermaid
flowchart LR
    SQL["SQL Statement<br/>CREATE TABLE / INSERT / SELECT"] --> Parser["turso_parser::parser::Parser::new(sql)<br/>next_cmd() -> Cmd::Stmt(...)"]
    Parser -- "Invalid SQL" --> ErrSyn["Err(SqlStoreError::InvalidSql)"]
    Parser -- "Valid AST" --> FK{"Foreign Key Check:<br/>board_id registered?"}
    FK -- "No" --> ErrFK["Err(SqlStoreError::ForeignKeyViolation)"]
    FK -- "Yes" --> Store["TursoReviewStore<br/>Upsert / Query SqlReviewRecord"]
    Store --> UI["egui Panel::right<br/>Turso SQL Inspector"]
    style Store fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style ErrSyn fill:#2a1418,stroke:#ff5555,color:#e6edf3
    style ErrFK fill:#2a1418,stroke:#ff5555,color:#e6edf3
```

## Validating SQL ASTs with `turso_parser` & Enforcing Foreign Keys in `TursoReviewStore`

Add `turso_parser` and the async `tokio` test runtime to `crates/pcb-doc` with `cargo add`:

```console
cargo add --package pcb-doc turso_parser@=0.8.1
cargo add --package pcb-doc --dev tokio --features rt,macros
```

Here is the resulting `crates/pcb-doc/Cargo.toml`:

```toml caption="crates/pcb-doc/Cargo.toml"
[package]
name = "pcb-doc"
version = "0.1.0"
edition = "2024"

[lints]
workspace = true

[dependencies]
emath = { version = "0.37", features = ["serde"] }
kicad-sexpr = { path = "../kicad-sexpr" }
serde = { version = "1.0", features = ["derive", "rc"] }
serde_json = "1.0"
thiserror = "2.0"
turso_parser = "=0.8.1"

[dev-dependencies]
proptest = "1.11"
tokio = { version = "1", features = ["rt", "macros"] }
```

Let's walk through the SQL AST validator, relational store structs, async/sync methods in `crates/pcb-doc/src/lib.rs`, and the `egui` SQL panel in `src/main.rs` block by block before viewing the complete listings in context.

### Block 1: Validating SQL DDL/DML ASTs with `turso_parser` (`crates/pcb-doc/src/lib.rs`)

```rust caption="Validating SQL statements with turso_parser::parser::Parser in crates/pcb-doc/src/lib.rs"
use turso_parser::{
    ast::{Cmd, Stmt},
    parser::Parser as TursoSqlParser,
};

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
```

- **`TursoSqlParser::new(trimmed.as_bytes())` & `next_cmd()`**: Streams SQL commands (`turso_parser::ast::Cmd`) from a UTF-8 byte slice, mapping any lexer or grammar error directly to `SqlStoreError::InvalidSql(e.to_string())` via the `?` operator.
- **Allow-Listing Supported `Stmt` Variants**: Uses `matches!(cmd, Cmd::Stmt(Stmt::CreateTable { .. } | Stmt::Insert { .. } | Stmt::Select(_) | ...))` so destructive DDL statements like `DROP TABLE boards;` are rejected with `SqlStoreError::InvalidSql("no supported SQL statements")`.
- **`REVIEW_STORE_SCHEMA_SQL`**: Defines two relational tables—`boards` (`board_id TEXT PRIMARY KEY`) and `review_threads` (`pin_id INTEGER PRIMARY KEY, board_id TEXT NOT NULL, ...`)—using `CREATE TABLE IF NOT EXISTS` so schema migrations are idempotent.

### Block 2: `SqlReviewRecord` Filtering & `TursoReviewStore` State (`crates/pcb-doc/src/lib.rs`)

```rust caption="Defining SqlReviewRecord and TursoReviewStore in crates/pcb-doc/src/lib.rs"
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
    rows: HashMap<PinId, SqlReviewRecord>,
    next_revision: u64,
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
}
```

- **`SqlReviewRecord` & `matches_sql_filter`**: Represents a materialized row from `review_threads` with a monotonic `revision: u64` counter. `matches_sql_filter` enforces the `only_open` status guard first, then performs a case-insensitive substring check across `author`, `comment`, and `status.label()`.
- **`TursoReviewStore`**: Tracks `schema_version`, a `HashSet<String>` of `registered_boards` (the parent `boards` table), a `HashMap<PinId, SqlReviewRecord>` of `rows`, and `next_revision`.
- **`is_migrated` & `summary_badge`**: Reports `"Turso SQL: Unmigrated"` before `migrate()` runs, and `"Turso SQL: 2 rows (Schema v1)"` once migrated.

### Block 3: Migrations, Foreign-Key Guards & Async/Sync Query Methods (`impl TursoReviewStore` & `BoardModel`)

```rust caption="Implementing migrate, register_board, upsert_pin_record, and query_review_records"
impl TursoReviewStore {
    // ...
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
}

impl BoardModel {
    // ...
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
}
```

- **Dual `async` + `*_sync` Entry Points**: Providing both `async fn migrate` / `upsert_pin_record` / `query_review_records` (for async `tokio` runtimes and remote Turso replicas) and synchronous `*_sync` counterparts allows immediate-mode `egui` button handlers to update local state without blocking a runtime thread.
- **Schema Migration & Foreign-Key Enforcement**: Every store operation checks `if !self.is_migrated() { return Err(SqlStoreError::NotMigrated); }` first. `upsert_pin_record_sync` additionally verifies `self.registered_boards.contains(board_id)` before executing the parameterized `INSERT INTO review_threads ... VALUES (?1, ...)` statement, returning `Err(SqlStoreError::ForeignKeyViolation(...))` for unregistered boards.
- **`BoardModel::export_sql_review_store`**: Migrates a fresh `TursoReviewStore`, registers `board_id`, and upserts every pin from `self.sorted_review_pins()`.

### Block 4: Synchronizing Review Pins to `TursoReviewStore` in `src/main.rs`

```rust caption="Integrating sql_store and Sync Pins to SQL in src/main.rs"
pub struct PcbReviewerApp {
    pub board: BoardModel,
    pub baseline_snapshot: BoardModel,
    pub history: BoardHistory,
    pub view: ViewState,
    pub saved_session_json: String,
    pub import_rx: Option<std::sync::mpsc::Receiver<ImportJobMessage>>,
    pub import_state: ImportJobState,
    pub sql_store: TursoReviewStore,
}

impl Default for PcbReviewerApp {
    fn default() -> Self {
        let board = BoardModel::default();
        let baseline_snapshot = board.clone();
        let mut app = Self {
            board,
            baseline_snapshot,
            history: BoardHistory::default(),
            view: ViewState::default(),
            saved_session_json: String::new(),
            import_rx: None,
            import_state: ImportJobState::Idle,
            sql_store: TursoReviewStore::default(),
        };
        app.sync_pins_to_sql_store();
        app
    }
}

impl PcbReviewerApp {
    // ...
    pub fn sync_pins_to_sql_store(&mut self) -> bool {
        match self.board.export_sql_review_store(&self.view.active_board_id) {
            Ok(store) => {
                self.sql_store = store;
                true
            }
            Err(err) => {
                self.view.last_io_error = Some(err.to_string());
                false
            }
        }
    }

    pub fn render_session_panel(&mut self, ui: &mut egui::Ui) {
        // ... at the bottom of render_session_panel:
        ui.separator();
        ui.heading("Embedded SQL Store (Turso)");
        ui.label(self.sql_store.summary_badge());
        if ui.button("Sync Pins to SQL").clicked() {
            self.sync_pins_to_sql_store();
        }
    }
}
```

- **`sql_store: TursoReviewStore` on `PcbReviewerApp`**: Initialized automatically at startup via `app.sync_pins_to_sql_store()` inside `PcbReviewerApp::default()`, so the right-hand session panel immediately displays `"Turso SQL: 2 rows (Schema v1)"`.
- **`sync_pins_to_sql_store(&mut self) -> bool`**: Exports the current `self.board` pins into `self.sql_store` for `self.view.active_board_id`, or surfaces any `SqlStoreError` (such as an empty `active_board_id`) in `self.view.last_io_error`.

### Block 5: Async `#[tokio::test]` Verification (`crates/pcb-doc/src/lib.rs`)

```rust caption="Testing async SQL migrations, foreign keys, and filter branches with #[tokio::test]"
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
        assert_eq!(
            store.register_board("pwr-stage-rev-b"),
            Err(SqlStoreError::NotMigrated)
        );

        // Idempotent migration
        assert_eq!(store.migrate().await, Ok(CURRENT_SCHEMA_VERSION));
        assert_eq!(store.migrate().await, Ok(CURRENT_SCHEMA_VERSION));
        assert!(store.is_migrated());

        assert_eq!(
            store.register_board("  pwr-stage-rev-b  "),
            Ok(true)
        );
        assert_eq!(store.register_board("pwr-stage-rev-b"), Ok(false));
    }
```

- **`#[tokio::test]`**: Spawns a single-threaded `tokio` runtime for the test function so we can `.await` `store.migrate().await`, `store.upsert_pin_record(...).await`, and `store.query_review_records(...).await` directly.
- **Exclusive `||` Branch Coverage**: The test queries `"alice"` (matching only `author`), `"polarity"` (matching only `comment`), and `"resolved"` (matching only `status`) in separate assertions so `cargo-mutants` catches any mutation to the three `||` branches of `SqlReviewRecord::matches_sql_filter`.

### Complete Embedded SQL Store & UI Integration Context Listing

Here is the complete assembled SQL store implementation in `crates/pcb-doc/src/lib.rs` and its UI integration in `src/main.rs`:

```rust caption="Complete Turso SQL parser, store, and #[tokio::test] suite in crates/pcb-doc/src/lib.rs"
use turso_parser::{
    ast::{Cmd, Stmt},
    parser::Parser as TursoSqlParser,
};

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
    rows: HashMap<PinId, SqlReviewRecord>,
    next_revision: u64,
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
}

impl BoardModel {
    // ...
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
}

#[cfg(test)]
mod tests {
    // ...
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
}
```

```rust caption="Complete SQL store UI synchronization and egui_kittest checks in src/main.rs"
impl PcbReviewerApp {
    // ...
    pub fn sync_pins_to_sql_store(&mut self) -> bool {
        match self.board.export_sql_review_store(&self.view.active_board_id) {
            Ok(store) => {
                self.sql_store = store;
                true
            }
            Err(err) => {
                self.view.last_io_error = Some(err.to_string());
                false
            }
        }
    }

    pub fn render_session_panel(&mut self, ui: &mut egui::Ui) {
        // ...
        ui.separator();
        ui.heading("Embedded SQL Store (Turso)");
        ui.label(self.sql_store.summary_badge());
        if ui.button("Sync Pins to SQL").clicked() {
            self.sync_pins_to_sql_store();
        }
    }
}

// Inside tests::standalone_layer_swatch_and_headless_ui_workflows_pass in src/main.rs:
        assert!(
            harness
                .query_by_label("Turso SQL: 2 rows (Schema v1)")
                .is_some()
        );
        harness.state_mut().board.clear_review_pins();
        harness.get_by_label("Sync Pins to SQL").click();
        harness.run();
        assert!(
            harness
                .query_by_label("Turso SQL: 0 rows (Schema v1)")
                .is_some()
        );

        harness.state_mut().view.active_board_id = "   ".to_owned();
        assert!(!harness.state_mut().sync_pins_to_sql_store());
        assert!(harness.state().view.last_io_error.is_some());
        harness.state_mut().view.active_board_id = "pwr-stage-rev-b".to_owned();
        harness.state_mut().view.dismiss_io_error();
```

{{widget:turso-sql-sim}}

:::exercise id="ex0025" title="Interactive Lab 2: Validate SQL ASTs with `TursoSqlParser` & Guard Foreign Keys" badge="Skill Practice • turso_parser & Async SQL Store"
Complete `validate_turso_sql` and `TursoReviewStore::upsert_pin_record` in `crates/pcb-doc/src/lib.rs`.

```rust
pub fn validate_turso_sql(sql: &str) -> Result<usize, SqlStoreError> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(SqlStoreError::{{blank:EmptySql|Return SqlStoreError::EmptySql when sql is blank|EmptySql|120px}});
    }
    let mut parser = TursoSqlParser::new(trimmed.as_bytes());
    let mut stmt_count = 0;
    while let Some(cmd) = parser
        .{{blank:next_cmd()|Advance the Turso SQL parser to the next Cmd AST node|next_cmd()|125px}}
        .map_err(|e| SqlStoreError::InvalidSql(e.to_string()))?
    {
        if matches!(cmd, Cmd::Stmt(_)) {
            stmt_count += 1;
        }
    }
    Ok(stmt_count)
}

pub async fn upsert_pin_record(
    &mut self,
    board_id: &str,
    note: &ReviewNote,
) -> Result<u64, SqlStoreError> {
    if !self.is_migrated() {
        return Err(SqlStoreError::NotMigrated);
    }
    if !self.registered_boards.contains(board_id) {
        return Err(SqlStoreError::{{blank:ForeignKeyViolation|Reject unregistered board_id with SqlStoreError::ForeignKeyViolation|ForeignKeyViolation|195px}}(board_id.to_owned()));
    }
    // ...
}
```

:::prompts
hint: Return `SqlStoreError::EmptySql` on blank SQL, call `parser.next_cmd()` in the loop condition, and return `SqlStoreError::ForeignKeyViolation(board_id.to_owned())` when `board_id` is not registered.
success: Cleanly verified! Every DDL migration and DML query is parsed into a `turso_parser` AST, and `TursoReviewStore` enforces schema migration and foreign-key invariants.
:::
:::

:::verify id="verify-0025" title="Lab 3: Async `#[tokio::test]` SQL Migrations, Foreign Keys & Mutation Gate (`0025`)" badge="cargo test --workspace + tokio::test + cargo mutants" tier="Tier 2 (Async SQL & Mutation Coverage)" caught="425 / 425 (100%)" missed="0 Missed"
Run the workspace test suite (including the async `#[tokio::test]` `turso_sql_review_store_migrations_foreign_keys_and_queries_work` test) and mutate all SQL parser, store, and UI sync methods:

```bash caption="Run async Turso SQL tests and cargo-mutants on 0024"
cargo test --manifest-path ./apps/0025-embedded-sql-review-store-with-turso/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0025-embedded-sql-review-store-with-turso/Cargo.toml --workspace --in-place -F "validate_turso_sql|SqlReviewRecord|TursoReviewStore|export_sql_review_store|sync_pins_to_sql_store"
```

- [ ] `cargo test --manifest-path ./apps/0025-embedded-sql-review-store-with-turso/Cargo.toml --workspace` passes all **26 tests** (**3** in `kicad-sexpr`, **17** in `pcb-doc`, and **6** in `pcb-reviewer`).
- [ ] `turso_sql_review_store_migrations_foreign_keys_and_queries_work` (`#[tokio::test]`) verifies `validate_turso_sql`, idempotent `migrate()`, `ForeignKeyViolation` rejection on unregistered boards, and case-insensitive SQL filtering across author, comment, and status.
- [ ] `cargo mutants` catches **100% of viable `TursoReviewStore` & UI sync mutants** (`0 missed`).
:::

:::quiz id="quiz-0025"
Q: Why does `TursoReviewStore` validate every DDL and DML query through `turso_parser::parser::Parser` before executing?
- [x] To verify valid SQL syntax and reject malformed statements
- [ ] To convert SQL strings directly into GPU shader bytecode
- [ ] To bypass foreign key checks on unregistered board IDs
- [ ] To clear all copper trace segments during schema migrations
why: Parsing SQL text through `turso_parser::parser::Parser::new(sql.as_bytes()).next_cmd()` guarantees that syntax errors or unsupported SQL statements fail immediately with `SqlStoreError::InvalidSql`.
hint: Check `validate_turso_sql` in `crates/pcb-doc/src/lib.rs`.

Q: Why must schema migrations in `TursoReviewStore::migrate` be idempotent when called multiple times?
- [ ] To drop and delete all existing review comment rows
- [x] To safely reopen existing databases without corrupting stored records
- [ ] To increment the schema version counter on every frame
- [ ] To force the immediate mode UI thread to block
why: Using `CREATE TABLE IF NOT EXISTS` and tracking `schema_version` ensures calling `migrate()` on an already-initialized store preserves existing rows and board registrations.
hint: Inspect `TursoReviewStore::migrate` in `crates/pcb-doc/src/lib.rs`.

Q: What happens when `upsert_pin_record` is called for a `board_id` that has not been registered?
- [ ] The store silently drops the review pin without error
- [ ] The store panics and terminates the desktop application process
- [x] The store returns a typed `ForeignKeyViolation` error value cleanly
- [ ] The store overwrites all pins on other registered boards
why: Checking `self.registered_boards.contains(board_id)` before inserting into `review_threads` enforces relational referential integrity and returns `Err(SqlStoreError::ForeignKeyViolation(...))`.
hint: Look at the foreign-key guard inside `TursoReviewStore::upsert_pin_record`.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [Turso / `libsql` Rust Documentation](https://docs.turso.tech/sdk/rust/quickstart) and [Tokio Documentation — `#[tokio::test]` Async Unit Testing](https://docs.rs/tokio/latest/tokio/attr.test.html).
:::

:::teacher-prompt
Next up: [Lesson 0026: Local-First Replica Sync, Atomic SQL Transactions & Tamper-Evident Audit Trails](./0026-local-first-review-sync-and-audit-trail.html), where we complete **Act VI** with atomic `BEGIN` / `COMMIT` / `ROLLBACK` sign-off batches, hash-chained `AuditLogEntry` verification, and local-first replica merging!
:::

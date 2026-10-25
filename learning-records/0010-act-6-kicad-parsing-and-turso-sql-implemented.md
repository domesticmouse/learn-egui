# Learning Record 0010 — Act VI (`0023`–`0026`) Implemented: Real-World KiCad Parsing (`nom`), Corpus Oracles & Embedded Turso SQL

- **Date:** 2026-10-25
- **Scope:** Implemented **Act VI (`0023`–`0026`)** of the Rust & `egui` Schematic and PCB Design Review System course, expanding each companion workspace to **3 crates** (`crates/kicad-sexpr` at `L0` Standalone, `crates/pcb-doc` at `L1`/`L4`, and `pcb-reviewer` at `L6`/`L7`), adding four interactive browser simulators (`{{widget:sexpr-ast-sim}}`, `{{widget:kicad-corpus-sim}}`, `{{widget:turso-sql-sim}}`, `{{widget:replica-sync-sim}}`), and verifying **100% mutation coverage (`0 missed`)** across all four workspaces.

---

## 1. Companion Workspaces & Mutation Coverage Summary

| Lesson | Companion Workspace | Workspace Tests | New Viable Mutants Caught | Cumulative Workspace Mutants Caught |
| --- | --- | --- | --- | --- |
| [`0023`](../lessons/0023-sexpr-tokenizer-and-ast-with-nom.html) | `apps/0023-sexpr-tokenizer-and-ast-with-nom` | **24** (`3` in `kicad-sexpr`, `15` in `pcb-doc`, `6` in `pcb-reviewer`) | **38 / 38** (`7` unviable, `0` missed) | **342 / 342 (100%, 0 missed)** |
| [`0024`](../lessons/0024-parsing-kicad-pcb-and-sch-into-domain-model.html) | `apps/0024-parsing-kicad-pcb-and-sch-into-domain-model` | **25** (`3` in `kicad-sexpr`, `16` in `pcb-doc`, `6` in `pcb-reviewer`) | **43 / 43** (`6` unviable, `0` missed) | **385 / 385 (100%, 0 missed)** |
| [`0025`](../lessons/0025-embedded-sql-review-store-with-turso.html) | `apps/0025-embedded-sql-review-store-with-turso` | **26** (`3` in `kicad-sexpr`, `17` in `pcb-doc`, `6` in `pcb-reviewer`) | **40 / 40** (`2` unviable, `0` missed) | **425 / 425 (100%, 0 missed)** |
| [`0026`](../lessons/0026-local-first-review-sync-and-audit-trail.html) | `apps/0026-local-first-review-sync-and-audit-trail` | **27** (`3` in `kicad-sexpr`, `18` in `pcb-doc`, `6` in `pcb-reviewer`) | **51 / 51** (`0` missed) | **476 / 476 (100%, 0 missed)** |

---

## 2. Key Architectural & Pedagogical Milestones in Act VI

1. **Lesson `0023` — Standalone `L0` S-Expression Parser (`crates/kicad-sexpr`) with `nom`**:
   - Enforced `validate_l0_standalone_manifest` so `crates/kicad-sexpr` never depends on `emath`, `pcb-doc`, or `egui`.
   - Built `parse_sexpr_str` using `nom` combinators (`delimited`, `many0`, `alt`, `take_while1`) to parse symbols, quoted strings, and nested `(kicad_pcb ...)` lists into `SexprNode`, verified by `proptest` (`prop_sexpr_parser_roundtrips_arbitrary_footprints`) and rendered inside an `egui::CollapsingHeader` AST inspector.
2. **Lesson `0024` — KiCad `.kicad_pcb` Lowering & Tier 4 Pinned Corpus Oracles**:
   - Implemented `BoardModel::from_kicad_pcb_sexpr` and `BoardModel::to_kicad_pcb_sexpr` with typed `KicadLowerError` diagnostics (`Sexpr`, `UnexpectedRootTag`, `MissingField`, `InvalidNumber`, `UnknownLayer`).
   - Added `PINNED_KICAD_CORPUS`, `CORPUS_PASS_FLOOR = 3`, `CORPUS_ROUNDTRIP_FLOOR = 3`, and `run_pinned_kicad_corpus()` verifying lossless parse-and-re-export equivalence (`reloaded == board`).
3. **Lesson `0025` — Embedded SQL Review Store (`turso_parser` & `#[tokio::test]`)**:
   - Integrated Turso's official Rust SQL parser (`turso_parser::parser::Parser`) via `validate_turso_sql` to parse and verify SQLite/Turso DDL (`CREATE TABLE IF NOT EXISTS`) and DML (`INSERT INTO`, `SELECT`) ASTs (`Cmd::Stmt`).
   - Implemented `TursoReviewStore` with idempotent migrations (`migrate`), foreign-key checks on `board_id` (`SqlStoreError::ForeignKeyViolation`), parameterized filtering (`query_review_records`), and `egui` synchronization (`"Sync Pins to SQL"`).
4. **Lesson `0026` — Atomic SQL Transactions, Tamper-Evident Audit Hash Chains & Replica Sync**:
   - Added `execute_atomic_signoff_batch` with `BEGIN` / `COMMIT` / `ROLLBACK` SQL validation and staged copy-on-write rollback when any status transition in a batch is invalid (`SqlStoreError::InvalidStatusTransition`).
   - Added `AuditLogEntry`, `compute_audit_entry_hash`, and `verify_audit_chain()` to detect tampering with `seq`, `prev_hash`, or `entry_hash`.
   - Added `sync_with_remote_replica` merging remote engineer records when `remote.revision > local.revision`.

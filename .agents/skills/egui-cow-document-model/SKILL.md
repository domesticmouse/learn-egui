---
name: egui-cow-document-model
description: >-
  Use this skill when exploring, understanding, designing, or modifying an
  immutable/copy-on-write (Arc) document model, sparse chunked/tiled storage,
  O(1) snapshot undo/redo history, or compile-time complete content-addressed
  file persistence in an engine-first Rust and egui application.
---

# Copy-on-Write Document Model, Snapshot History & Persistence

This skill covers how application state is stored, mutated, undone, and persisted across `L0` (containers/chunks), `L1` (`doc`), `L2` (`ops`), and `L3` (`format`) so that an `egui` UI can render and mutate large documents with zero-copy undo/redo.

## Core Rules

1. **Pure Serializable Domain Model (`L1`):**
   - `Document` and its child entities in `crates/doc` are pure data structures (`Serialize`, `Deserialize`, `Clone`) with no UI or rendering dependencies.
2. **Sparse Copy-on-Write (`Arc`) Storage:**
   - Large buffers or collections are partitioned into fixed-size chunks/tiles stored in `BTreeMap<ChunkId, Arc<Chunk>>`.
   - Mutating a chunk uses `Arc::make_mut`, cloning only the touched chunk while sharing all untouched chunks across snapshots.
   - Allocating buffers from untrusted/external dimensions uses fallible reservation (`Vec::try_reserve_exact` $\to$ `AllocationError`) so huge/hostile inputs never abort the process.
3. **Whole-Document Snapshot History (`O(chunks)` pointer copy, zero data copy):**
   - Because heavy data payloads are `Arc`-shared, cloning a `Document` costs only pointer copies.
   - `History` (`crates/ops`) stores `VecDeque<HistoryState>` where each `HistoryState` holds `Arc<Document>` plus `SelectionTarget` (the active/selected entity IDs when the state was created).
   - Memory is bounded by `max_states` and `max_bytes` (counting unique `Arc<Chunk>` addresses via `Arc::as_ptr`).
4. **Compile-Time Complete Native Format:**
   - `crates/format` serializes `Document` into a bundle with `manifest.json` + content-addressed `blobs/<blake3>.zst`.
   - Struct conversions in `crates/format/src/convert.rs` deliberately destructure `doc` structs **without `..`** so adding a field to `doc` fails compilation until `format` (with `#[serde(default)]`) explicitly handles it.

---

## Workflow: Adding or Modifying Document State

1. **Add the field in `crates/doc`** with `Serialize, Deserialize` (and `#[serde(default)]` where needed).
2. **Update `crates/format/src/manifest.rs` and `convert.rs`** (which will fail to compile until you wire the new field) with `#[serde(default)]` so existing saved files continue to load.
3. **Mutate via `Session::edit` in `crates/engine`**, which automatically snapshots `before: Arc<Document>`, applies the mutation closure, computes `step_damage(&before, &after)`, records the undo state (or coalesces if the interactive `egui` `"coalesce"` key matches), and increments `DocState::revision` so the `egui` viewport knows to repaint.
4. See [cow-tiles-and-history.md](./references/cow-tiles-and-history.md) for concrete code patterns.

## Validation Steps

See [egui-testing-infrastructure](../egui-testing-infrastructure/SKILL.md) for property-based (`proptest!`) COW chunk testing, round-trip corpus oracles, and memory/RSS benchmarking:

```sh
cargo test -p <app>-doc -p <app>-ops -p <app>-format --lib --tests
cargo clippy -p <app>-doc -p <app>-ops -p <app>-format --all-targets -- -D warnings
```

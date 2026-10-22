---
name: craft-cow-document-model
description: >-
  Use this skill when exploring, understanding, designing, or modifying the
  document model, tiled pixel surfaces, undo/redo history, or native file
  persistence (.pcraft) in a PhotoCraft-style application (runtime PixelFormat,
  sparse Arc-shared 256x256 copy-on-write tiles, O(1) snapshot history, and
  compile-time complete content-addressed serialization).
---

# Copy-on-Write Tiled Document Model, History & Native Format

This skill covers how data is stored, mutated, undone, and persisted across `L0` (`raster`, `color`), `L1` (`doc`), `L2` (`ops`), and `L3` (`format`).

## Core Rules

1. **No Format or Color Assumptions ("Avoid GIMP's Hole"):**
   - Bit depth (`SampleType::U8 | U16 | F32`) and color mode (`ColorMode::Rgb | Gray | Cmyk | Lab | Indexed | Bitmap | Multichannel`) are runtime fields on `PixelFormat` and `Document`, never hard-coded `u8` RGB buffers.
   - Non-destructive layer types (`Raster`, `Group`, `Adjustment`, `Fill`, `Text`, `Shape`, `Smart`) are first-class variants of `LayerContent` from day one.
2. **Sparse Copy-on-Write (`Arc`) 256×256 Tiles:**
   - `Surface` stores `BTreeMap<TileCoord, Arc<Tile>>`. Unallocated tiles implicitly read as `default_pixel` (transparent for layers, white for "reveal all" masks).
   - Mutating a tile uses `Arc::make_mut`, cloning only the touched 256×256 tile.
   - Allocating tiles from input dimensions uses fallible reservation (`Vec::try_reserve_exact` $\to$ `AllocationError`) so huge/hostile dimensions never abort the process.
3. **Whole-Document Snapshot History (`O(tiles)` pointer copy, zero pixel copy):**
   - Because `Surface` tiles are `Arc`-shared, cloning a `Document` costs only pointer copies.
   - `History` (`crates/ops`) stores `VecDeque<HistoryState>` where each `HistoryState` holds `Arc<Document>` plus `LayerTarget` (the active and selected layers when the state was created).
   - Memory is bounded by `max_states` and `max_bytes` (counting unique `Arc<Tile>` addresses via `Arc::as_ptr`).
4. **Compile-Time Complete Native Format (`.pcraft`):**
   - `crates/format` serializes `Document` into a ZIP/directory bundle with `manifest.json` + content-addressed `tiles/<blake3>.zst` (including sample depth in tile hash identity and reusing a per-document save writer with a bounded 64 MiB compressed-payload cache across repeated/small-edit saves).
   - Struct conversions in `crates/format/src/convert.rs` deliberately destructure `photocraft-doc` structs **without `..`** so adding a field to `doc` fails compilation until `format` (with `#[serde(default)]`) and `io` (if PSD-mapped) handle it.

---

## Workflow: Adding or Modifying Document State

1. **Add the field in `crates/doc`** with `Serialize, Deserialize` (and `#[serde(default)]` where needed).
2. **Update `crates/format/src/manifest.rs` and `convert.rs`** (which will fail to compile until you wire the new field) with `#[serde(default)]` so existing `.pcraft` files continue to load.
3. **Map to/from PSD in `crates/io`** if the property has an Adobe PSD/PSB equivalent, preserving unknown PSD tagged blocks verbatim.
4. **Mutate via `Session::edit` in `crates/engine`**, which automatically snapshots `before: Arc<Document>`, applies the mutation closure, computes `step_damage(&before, &after)`, records the undo state (or coalesces if `coalesce` key matches), and increments `DocState::revision`.
5. See [cow-tiles-and-history.md](./references/cow-tiles-and-history.md) for concrete code patterns.

## Validation Steps

```sh
cargo test -p photocraft-raster -p photocraft-doc -p photocraft-ops -p photocraft-format --lib --tests
cargo clippy -p photocraft-raster -p photocraft-doc -p photocraft-ops -p photocraft-format --all-targets -- -D warnings
```

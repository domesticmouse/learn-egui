# Copy-on-Write Tiles, History & Persistence Patterns

## 1. Fallible, Sparse COW Tile Storage (`crates/raster/src/lib.rs`)

```rust
pub struct Tile {
    data: Box<[u8]>, // TILE_SIZE (256) * TILE_SIZE (256) * format.bytes_per_pixel()
}

impl Tile {
    pub fn try_filled(format: &PixelFormat, pixel: &[u8]) -> Result<Self, AllocationError> {
        let n = (TILE_SIZE * TILE_SIZE) as usize;
        let bytes = n.checked_mul(format.bytes_per_pixel()).ok_or(AllocationError::SizeOverflow)?;
        let mut data = Vec::new();
        data.try_reserve_exact(bytes).map_err(|_| AllocationError::NotEnoughMemory { bytes })?;
        data.resize(bytes, 0);
        if pixel.iter().any(|&b| b != 0) {
            for px in data.chunks_exact_mut(pixel.len()) {
                px.copy_from_slice(pixel);
            }
        }
        Ok(Tile { data: data.into_boxed_slice() })
    }
}

#[derive(Clone, Debug)]
pub struct Surface {
    format: PixelFormat,
    default_pixel: Box<[u8]>,
    tiles: BTreeMap<TileCoord, Arc<Tile>>,
}
```

- **Reading a pixel** in an unallocated tile returns `default_pixel`.
- **Writing a pixel** inserts a default tile if missing and calls `Arc::make_mut` on the target `Arc<Tile>`. If the tile is shared with `History`, only that 256×256 tile is cloned.

## 2. Snapshot History & Layer Target Restoration (`crates/ops/src/lib.rs`)

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayerTarget {
    pub active: Option<LayerId>,
    pub selected: Vec<LayerId>,
}

#[derive(Clone, Debug)]
pub struct HistoryState {
    pub label: String,
    pub doc: Arc<Document>,
    pub layers: LayerTarget,
}
```

- **Why `LayerTarget` is stored per state:** Selecting layers is not an undoable history step by itself. However, when a user undoes or redoes an edit, the active and selected layers must return to what was targeted when that state was created. Just before recording a new step, `Session::edit` calls `st.history.set_current_layers(prior)`.

## 3. Transactional Document Mutation (`crates/engine/src/lib.rs`)

All undoable mutations flow through `Session::edit`:

```rust
pub fn edit<R>(
    &mut self,
    label: &str,
    f: impl FnOnce(&mut Document, &mut Option<LayerId>) -> Result<R>,
) -> Result<R> {
    let st = self.active_mut().ok_or(EngineError::NoDocument)?;
    let before = st.doc.clone();          // O(1) Arc clone
    let prior = st.layer_target();
    let mut doc = (*before).clone();      // O(layers + tile Arcs), zero pixel copies
    let mut active = st.active_layer;

    let r = f(&mut doc, &mut active)?;    // Mutates only touched tiles via Arc::make_mut

    st.doc = Arc::new(doc);
    st.active_layer = active;
    let damage = layer_multi_cmds::step_damage(&before, &st.doc);
    let key = self.coalesce_request.clone();
    let layers = st.layer_target();

    if key.is_none() || st.coalesce != key || !st.history.can_undo() {
        st.history.set_current_layers(prior);
        st.history.record(label, before, layers);
        st.trim_history();
    } else {
        st.history.set_current_layers(layers);
    }
    st.coalesce = key;
    st.revision += 1;
    st.last_damage = damage;
    Ok(r)
}
```

## 4. Content-Addressed Native Bundle (`crates/format`)

- Each tile's raw bytes (together with its sample depth) are hashed with `blake3`.
- Tile data is compressed with Zstandard (`ruzstd` for pure-Rust decoding on native + wasm) and stored at `tiles/<blake3_hex>.zst`.
- `manifest.json` references tiles by their `<blake3_hex>` digest. Identical tiles (e.g., solid fill regions or duplicated layers) are stored once.
- **Save Writer Reuse:** Each open document can retain a `format` writer across saves so unchanged tile and blob identities reuse weakly cached hashes and compressed payloads (bounded to 64 MiB in the UI), cutting repeated and small-edit save times by 20–34%. Fallible in-place edits move uniquely owned tile pixels when only weak cache references remain, while shared tiles keep copy-on-write behavior.
- Native saves write to a temporary file/directory and atomically replace the destination while preserving OS metadata (POSIX permissions, macOS ACLs via `exacl`, Windows DACLs and Alternate Data Streams via `winsafe::ReplaceFileW`).

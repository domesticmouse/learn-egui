# Copy-on-Write Storage, History & Persistence Patterns

## 1. Fallible, Sparse COW Chunk/Tile Storage (`L0`)

```rust
pub struct Chunk {
    data: Box<[u8]>,
}

impl Chunk {
    pub fn try_allocate(bytes: usize) -> Result<Self, AllocationError> {
        let mut data = Vec::new();
        data.try_reserve_exact(bytes)
            .map_err(|_| AllocationError::NotEnoughMemory { bytes })?;
        data.resize(bytes, 0);
        Ok(Chunk { data: data.into_boxed_slice() })
    }
}

#[derive(Clone, Debug)]
pub struct SparseStore<Id: Ord> {
    chunks: BTreeMap<Id, Arc<Chunk>>,
}
```

- **Reading** an unallocated chunk returns a default view without allocating.
- **Writing** inserts a default chunk if missing and calls `Arc::make_mut` on the target `Arc<Chunk>`. If the chunk is shared with `History`, only that touched chunk is cloned.

## 2. Snapshot History & Selection Target Restoration (`crates/ops/src/lib.rs`)

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionTarget {
    pub active: Option<EntityId>,
    pub selected: Vec<EntityId>,
}

#[derive(Clone, Debug)]
pub struct HistoryState {
    pub label: String,
    pub doc: Arc<Document>,
    pub target: SelectionTarget,
}
```

- **Why `SelectionTarget` is stored per state:** Changing selection in the UI is often not an undoable history step by itself. However, when a user undoes or redoes an edit, the active and selected items should return to what was targeted when that state was created. Just before recording a new step, `Session::edit` calls `st.history.set_current_target(prior)`.

## 3. Transactional Document Mutation (`crates/engine/src/lib.rs`)

All undoable mutations flow through `Session::edit`:

```rust
pub fn edit<R>(
    &mut self,
    label: &str,
    f: impl FnOnce(&mut Document, &mut Option<EntityId>) -> Result<R>,
) -> Result<R> {
    let st = self.active_mut().ok_or(EngineError::NoDocument)?;
    let before = st.doc.clone();          // O(1) Arc clone
    let prior = st.selection_target();
    let mut doc = (*before).clone();      // O(entities + chunk Arcs), zero bulk data copies
    let mut active = st.active_entity;

    let r = f(&mut doc, &mut active)?;    // Mutates only touched chunks via Arc::make_mut

    st.doc = Arc::new(doc);
    st.active_entity = active;
    let damage = compute_step_damage(&before, &st.doc);
    let key = self.coalesce_request.clone();
    let target = st.selection_target();

    if key.is_none() || st.coalesce != key || !st.history.can_undo() {
        st.history.set_current_target(prior);
        st.history.record(label, before, target);
        st.trim_history();
    } else {
        st.history.set_current_target(target);
    }
    st.coalesce = key;
    st.revision += 1;
    st.last_damage = damage;
    Ok(r)
}
```

## 4. Content-Addressed Native Bundle (`crates/format`)

- Each chunk/blob's raw bytes are hashed with `blake3`.
- Data is compressed with Zstandard (`ruzstd` for pure-Rust decoding on native + wasm) and stored at `blobs/<blake3_hex>.zst`.
- `manifest.json` references blobs by their `<blake3_hex>` digest. Identical chunks across entities are stored once.
- **Exhaustive Destructuring:** In `crates/format/src/convert.rs`, destructure `Document` and entity structs without `..` so adding any new field in `crates/doc` triggers a compile error until persistence is wired.
- **Atomic Saves:** Write to a temporary file/directory and atomically replace the destination so interrupted saves never corrupt existing files.

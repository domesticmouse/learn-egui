---
nav_id: "0017"
page_title: "Lesson 0017: Copy-on-Write Document Snapshots with Arc and Arc::make_mut — Learn Rust & egui"
act_badge: "Act IV • Engine-First Architecture & Persistence"
title: "0017. Copy-on-Write Document Snapshots with `Arc` & `Arc::make_mut`"
subtitle: "Wrapping `BoardModel` collections in `Arc<Vec<T>>` and `Arc<HashMap<K, V>>` (`serde` `rc` feature) so taking a snapshot is `O(1)`, mutating a review pin clones only the touched `HashMap`, and `proptest` verifies structural pointer sharing (`Arc::ptr_eq`)."
card:
  num: "Lesson 0017"
  mutants: "169 / 169 Caught"
  title: "Copy-on-Write Document Snapshots (`Arc` & `Arc::make_mut`)"
  summary: "Upgrade `BoardModel` collections to `Arc<Vec<...>>` and `Arc<HashMap<...>>`, mutate touched buffers with `Arc::make_mut`, and verify `O(1)` snapshot structural sharing (`Arc::ptr_eq`) with `proptest`."
  egui_skill: "Snapshot Telemetry & Instant `O(1)` Rollback UI"
  rust_concept: "`std::sync::Arc`, `Arc::make_mut`, `Arc::ptr_eq` & `proptest`"
  app_dir: "apps/0017-copy-on-write-arc-document-snapshots"
---

## Why `O(N)` Deep Clones Break 60 FPS Immediate-Mode CAD Apps

Suppose a production PCB layout holds 50,000 `TraceSegment`s, 20,000 `SmdPad`s, and 2,000 `ComponentSymbol`s, while an engineer adds or edits a single `ReviewNote` pin. If `BoardModel` stores plain `Vec<T>` and `HashMap<K, V>` fields, calling `let snapshot = self.board.clone()` before an edit allocates and copies *every* trace, pad, and component on the heap (`O(N)` memory and CPU time).{{sidenote: **Structural Sharing (`Arc` Copy-on-Write):** By wrapping each collection in `Arc<...>` (`Arc<Vec<TraceSegment>>`, `Arc<Vec<SmdPad>>`, `Arc<Vec<ComponentSymbol>>`, `Arc<HashMap<PinId, ReviewNote>>`), `BoardModel::clone()` copies only four 8-byte pointers and increments four atomic reference counts (`O(1)`).}}

When we need to mutate a collection—such as inserting or updating a review pin—we call `Arc::make_mut(&mut self.review_pins)`:

- **If `strong_count == 1`** (no snapshot shares this buffer): `Arc::make_mut` returns a `&mut HashMap<PinId, ReviewNote>` in-place with **zero allocation**.
- **If `strong_count > 1`** (a snapshot or undo entry shares this buffer): `Arc::make_mut` clones *only* `review_pins` into a new `Arc` allocation and mutates the new copy, while `traces`, `pads`, and `components` remain 100% pointer-shared (`Arc::ptr_eq == true`) with the snapshot!

```mermaid
flowchart TD
    subgraph Baseline["baseline_snapshot: BoardModel"]
        B1["traces: Arc"]
        B2["pads: Arc"]
        B3["components: Arc"]
        B4["review_pins: Arc"]
    end
    subgraph Working["working: BoardModel (after Arc::make_mut(&mut working.review_pins))"]
        W1["traces: Arc"]
        W2["pads: Arc"]
        W3["components: Arc"]
        W4["review_pins: Arc (Cloned)"]
    end
    HeapT["Heap: Vec&lt;TraceSegment&gt; (Shared)"]
    HeapP["Heap: Vec&lt;SmdPad&gt; (Shared)"]
    HeapC["Heap: Vec&lt;ComponentSymbol&gt; (Shared)"]
    HeapROld["Heap: HashMap (2 Pins — Baseline)"]
    HeapRNew["Heap: HashMap (3 Pins — Working)"]

    B1 & W1 --> HeapT
    B2 & W2 --> HeapP
    B3 & W3 --> HeapC
    B4 --> HeapROld
    W4 --> HeapRNew
    style HeapT fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style HeapP fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style HeapC fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style HeapROld fill:#141a21,stroke:#3b8eea,color:#e6edf3
    style HeapRNew fill:#2a1b14,stroke:#ff8b6b,color:#e6edf3
```

## Serializing `Arc<T>` with `serde`'s `"rc"` Feature & Property Testing with `proptest`

To keep `BoardModel`'s `#[derive(Serialize, Deserialize)]` working seamlessly across `Arc<...>` fields, we enable `serde = { version = "1.0", features = ["derive", "rc"] }` in `crates/pcb-doc/Cargo.toml`.{{sidenote: **Tier 2 Property-Based Testing (`proptest`):** Unit tests check fixed examples; `proptest!` generates hundreds of randomized `(x, y, comment)` inputs and shrinks any failure to its minimal counterexample.}} We also add `shared_arc_buffers(&self, other: &Self) -> usize` using `Arc::ptr_eq` so both our `proptest!` suite and the `Panel::right` UI (`"Shared Arc Buffers: 3 / 4"`) can inspect structural sharing directly.

```rust caption="Copy-on-Write BoardModel with Arc::make_mut and Arc::ptr_eq in crates/pcb-doc/src/lib.rs"
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardModel {
    pub traces: Arc<Vec<TraceSegment>>,
    pub pads: Arc<Vec<SmdPad>>,
    pub components: Arc<Vec<ComponentSymbol>>,
    pub review_pins: Arc<HashMap<PinId, ReviewNote>>,
    pub next_pin_id: u32,
}

impl BoardModel {
    pub fn shared_arc_buffers(&self, other: &Self) -> usize {
        usize::from(Arc::ptr_eq(&self.traces, &other.traces))
            + usize::from(Arc::ptr_eq(&self.pads, &other.pads))
            + usize::from(Arc::ptr_eq(&self.components, &other.components))
            + usize::from(Arc::ptr_eq(&self.review_pins, &other.review_pins))
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
}
```

{{widget:cow-arc-sim}}

:::exercise id="ex0017" title="Interactive Lab 2: Mutate `Arc` Buffers with `Arc::make_mut` & Measure Sharing with `Arc::ptr_eq`" badge="Skill Practice • Copy-on-Write Arc"
Complete `shared_arc_buffers`, `clear_review_pins`, and `transition_pin_status` on `BoardModel` so mutations clone only the touched `Arc` buffer when shared.

```rust
pub fn shared_arc_buffers(&self, other: &Self) -> usize {
    usize::from(Arc::{{blank:ptr_eq(&self.traces, &other.traces);;ptr_eq(&self.traces,&other.traces)|Compare heap pointers of self.traces and other.traces with Arc::ptr_eq|ptr_eq(...)|295px}})
        + usize::from(Arc::ptr_eq(&self.pads, &other.pads))
        + usize::from(Arc::ptr_eq(&self.components, &other.components))
        + usize::from(Arc::ptr_eq(&self.review_pins, &other.review_pins))
}

pub fn clear_review_pins(&mut self) {
    Arc::{{blank:make_mut(&mut self.review_pins)|Obtain a mutable reference to review_pins via copy-on-write|make_mut(...)|255px}}.clear();
}

pub fn transition_pin_status(&mut self, id: PinId, next: ReviewStatus) -> bool {
    let Some(pin) = Arc::{{blank:make_mut(&mut self.review_pins)|Use Arc::make_mut to mutate only the review_pins map|make_mut(...)|255px}}.get_mut(&id) else {
        return false;
    };
    if pin.status.can_transition_to(next) {
        pin.status = next;
        true
    } else {
        false
    }
}
```

:::prompts
hint: Use `Arc::ptr_eq(&self.traces, &other.traces)` to check pointer identity and `Arc::make_mut(&mut self.review_pins)` for copy-on-write mutation.
success: Spot on! `Arc::make_mut` clones `review_pins` only when `strong_count > 1`, keeping `traces`, `pads`, and `components` pointer-shared (`3 / 4`).
:::
:::

:::verify id="verify-0017" title="Lab 3: Copy-on-Write `proptest` & Mutation Gate (`0017`)" badge="cargo test --workspace + proptest + cargo mutants" tier="Tier 2 (proptest) + Tier 3" caught="169 / 169 (100%)" missed="0 Missed"
Run the workspace tests (including `prop_cow_pin_mutation_preserves_baseline_snapshot_and_shares_untouched_arcs`) and the mutation suite for the new CoW methods:

```bash caption="Run proptest and cargo-mutants on 0016 Copy-on-Write functions"
cargo test --manifest-path ./apps/0017-copy-on-write-arc-document-snapshots/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0017-copy-on-write-arc-document-snapshots/Cargo.toml --workspace --in-place -F "shared_arc_buffers|push_smd_pad|clear_review_pins|take_baseline_snapshot|restore_baseline_snapshot|shared_buffers_with_baseline|render_session_panel"
```

- [ ] `cargo test --manifest-path ./apps/0017-copy-on-write-arc-document-snapshots/Cargo.toml --workspace` passes all **14 tests** (**8** in `pcb-doc` including `proptest`, and **6** in `pcb-reviewer`).
- [ ] `shared_arc_buffers_tracks_copy_on_write_pointer_sharing_across_all_four_collections` verifies the exact `4 → 3 → 2 → 1 → 0` pointer-sharing progression as each `Arc` collection is mutated.
- [ ] `cargo mutants` catches all **15 new CoW & snapshot UI mutants** (`15 / 15 caught`), bringing the workspace total to **169 / 169 viable mutants caught (0 missed)**.
:::

:::quiz id="quiz-0017"
Q: What does `Arc::make_mut(&mut self.review_pins)` do when `self.review_pins` has a strong reference count of `2`?
- [ ] It panics immediately because shared `Arc` pointers are permanently immutable
- [ ] It blocks the thread until the baseline snapshot is dropped
- [x] It clones the inner `HashMap` into a fresh `Arc` allocation
- [ ] It deep-clones all four data collections inside `BoardModel` simultaneously now
why: When `strong_count > 1`, `Arc::make_mut` clones the inner value (`HashMap<PinId, ReviewNote>`) into a new unique `Arc` (`strong_count == 1`), detaching it from the shared allocation while leaving all other `Arc` fields untouched.
hint: Trace what happens to `working.review_pins` vs. `working.traces` in the interactive simulator above.

Q: What does `Arc::ptr_eq(&baseline.traces, &working.traces)` check?
- [ ] Whether every `TraceSegment` has the same copper width in millimetres
- [x] Whether both `Arc` handles point to the same heap allocation
- [ ] Whether the two vectors have equal capacities after a re-allocation
- [ ] Whether `serde_json` serialized both trace vectors into identical JSON strings
why: `Arc::ptr_eq` compares the raw data pointers in `O(1)` time without iterating over the elements, proving that no deep clone occurred.
hint: Check how `BoardModel::shared_arc_buffers` is implemented in `crates/pcb-doc/src/lib.rs`.

Q: Which Cargo feature on `serde` enables `#[derive(Serialize, Deserialize)]` for structs containing `Arc<T>`?
- [ ] The `alloc` feature flag inside `eframe`
- [x] The `rc` feature flag inside `serde`
- [ ] The `unstable` feature flag inside `emath`
- [ ] The `sync` feature flag inside `thiserror`
why: Enabling `features = ["derive", "rc"]` on `serde` provides `Serialize` and `Deserialize` implementations for `std::rc::Rc<T>` and `std::sync::Arc<T>`.
hint: Inspect `apps/0017-copy-on-write-arc-document-snapshots/crates/pcb-doc/Cargo.toml`.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [`std::sync::Arc::make_mut` Official Rust Standard Library Docs](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.make_mut) and [Proptest Book — "Getting Started"](https://proptest-rs.github.io/proptest/proptest/getting-started.html).
:::

:::teacher-prompt
Next up: [Lesson 0018: Undo/Redo History (`VecDeque`) & Continuous Slider Drag Coalescing](./0018-undo-redo-history-and-slider-coalescing.html), where we build `BoardHistory` on top of our `O(1)` `Arc` snapshots and coalesce 60-FPS slider drags into a single undo step!
:::

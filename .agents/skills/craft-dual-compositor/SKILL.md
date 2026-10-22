---
name: craft-dual-compositor
description: >-
  Use this skill when exploring, understanding, or working on layer
  compositing, blend modes, adjustment layers, layer effects, GPU rendering
  (wgpu), ICC display color management (GPU matrix/TRC shaper curves and 3D
  LUTs), or CPU/GPU and corpus parity tests in a PhotoCraft-style application.
---

# Dual CPU/GPU Compositing, ICC Color & Oracle Parity

This skill guides work across `L0` (`cms`, `color`), `L3` (`compose`, `gpu`), and corpus/parity verification.

## Core Design Principles

1. **CPU Compositor First (The Oracle):**
   - `photocraft-compose` (`crates/compose/src/lib.rs`) is the authoritative reference implementation of Photoshop layer semantics: 27+ blend modes, opacity vs. fill opacity, clipping groups, pass-through vs. isolated groups, raster/vector/filter masks, `BlendIf` ranges, knockout, layer effects (`lfx2`), and adjustment layers.
   - It composites `Rect` regions in independent 256×256 tiles in parallel using `rayon` on native targets (and single-threaded on `wasm32`).
   - It caps single-call buffer allocations (`MAX_RENDER_PIXELS = 1 << 30`) so hostile canvas sizes (up to 300,000×300,000 px) never abort the process.
2. **GPU Compositor Matched Within $\le 1/255$:**
   - `photocraft-gpu` (`crates/gpu/src/lib.rs`) executes the same compositing semantics on `wgpu` using `rgba16float`/`rgba32float` accumulators and a tile residency cache keyed by `(TileId, generation)`.
   - Every blend mode, adjustment, and layer effect in `photocraft-gpu` must pass automated parity tests asserting output within $\le 1/255$ of `photocraft-compose`.
3. **End-to-End ICC Color Management (`photocraft-cms`):**
   - Never assume sRGB. Document working spaces (RGB, Gray, CMYK, Lab) are carried by ICC profiles (`photocraft-cms`).
   - Display color management (`crates/engine/src/display_color.rs`) transforms from the document profile to the active monitor's ICC profile (with relative colorimetric intent, Black Point Compensation, Proof Colors, and Gamut Warning).
   - On the GPU canvas, RGB matrix/TRC transforms use **accurate float matrix/shaper stages** (`Transform::gpu_matrix_shaper()` in `crates/cms/src/gpu_matrix_shaper.rs`: 4097-entry input curves, 3×3 matrix + offsets, and 4096-entry output curves indexed by $x^{1/4}$ to preserve near-black shadow detail), falling back to a **33³ 3D LUT** for LUT-based profiles, soft-proofing, or Gamut Warning; on the CPU canvas, it runs through `photocraft-cms`'s cached 8-bit transform (`transform::cached`).
4. **Damage-Driven Partial Redraws:**
   - Never recomposite or scan a full surface per frame. Use `DocState::last_damage` (`Option<Rect>`) and `DocState::revision` / `view_revision` to re-render only the tiles touched since the previous frame.
5. **External Format & Merged Composite Oracle Testing:**
   - Real Photoshop-authored PSD files (`corpus/photoshop/`, `corpus/psd/`, `corpus/psd-tools/`) and GIMP 3.2.6 XCF files (`corpus/gimp/`) store reference merged composites. Corpus tests render the imported `Document` layer tree with `photocraft-compose` and diff against the reference composite.

---

## Workflow: Adding or Modifying a Blend Mode, Adjustment, or Effect

1. **Implement the CPU reference first** in `crates/color` or `crates/compose` (`adjust.rs`, `effects.rs`, `psblend.rs`, `native.rs`, `native_lab.rs`), testing at `U8`, `U16`, and `F32` depths.
2. **Implement the matching WGSL shader/pipeline** in `crates/gpu`.
3. **Add a CPU vs. GPU parity test** in `crates/gpu` verifying that GPU output matches `photocraft_compose::render` within `1/255`.
4. **Benchmark on a 24–36 MP image in release** (`cargo xtask perf --quick`) to ensure per-tile parallelism and empty-tile skipping keep frame times within budget.
5. See [compositor-and-parity.md](./references/compositor-and-parity.md) for implementation details.

## Validation Steps

```sh
cargo test -p photocraft-compose -p photocraft-gpu --lib --tests
cargo xtask test-corpus --changed
cargo xtask perf --quick
cargo clippy -p photocraft-compose -p photocraft-gpu --all-targets -- -D warnings
```

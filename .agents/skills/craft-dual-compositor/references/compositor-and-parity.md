# Compositor, Color Pipeline & Oracle Parity Reference

## 1. Tile-Parallel CPU Compositing (`crates/compose/src/lib.rs`)

```rust
pub const RENDER_TILE: i32 = 256;
pub const MAX_RENDER_PIXELS: u64 = 1 << 30;

/// Straight-alpha RGBA float buffer covering a rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct Buffer {
    pub rect: Rect,
    pub px: Vec<[f32; 4]>,
}

pub fn render(doc: &Document, rect: Rect) -> Buffer {
    render_tiled(doc, rect, RENDER_TILE)
}
```

Key invariants in `render_tiled_with`:
- **Empty & bounds check:** Returns `Buffer::transparent(Rect::EMPTY)` if `rect.is_empty()` or `width * height > MAX_RENDER_PIXELS`.
- **Mode-native compositing paths:**
  - CMYK documents with ink-native adjustments composite directly in CMYK via `native::render_cmyk`.
  - Lab documents composite in CIELAB via `native_lab::render_lab` or `psblend::LAB_MIX`.
  - 32-bit float (`SampleType::F32`) documents disable $[0, 1]$ clamping in `Add`/`Divide`/HSL blend modes (`psblend::HDR`).
- **Precomputed effect maps:** Layer styles (`prepare_effects`) are prepared once over the region before `rayon` workers composite individual 256×256 tiles in bands.

## 2. Display Color Management: GPU Matrix/Shaper & 33³ 3D LUT (`crates/engine/src/display_color.rs`)

```text
Document Pixels (RGB / CMYK / Lab / Gray, 8/16/32f)
  │
  ▼  Compositor (photocraft-compose or photocraft-gpu)
Linear or Working-Space Composite
  │
  ├─► Fast Path (doc profile == monitor profile, no proofing/gamut warning)
  │     └── Identity (zero LUT / zero transform overhead)
  │
  └─► Color-Managed Path
        ├── GPU Canvas (RGB Matrix/TRC): Float 1D input curves (4097) + 3×3 matrix/offsets
        │     + 1D output curves (4096, indexed by x^(1/4) for near-black accuracy)
        ├── GPU Canvas (LUT/Proof/Gamut Fallback): Baked 33×33×33 3D LUT texture sampled
        │     via trilinear interpolation in WGSL
        └── CPU Canvas: Cached 8-bit transform (`photocraft_cms::transform::cached`)
```

## 3. Two-Tier Oracle Testing

1. **Tier 1: External Format Oracles (`photocraft-corpus`)**
   - Pinned in `xtask/src/corpus_pins.rs` and fetched via `cargo xtask corpus --all` (`photoshop`, `gimp`, `psd`, `psd-tools`, `heif`, `exr`, `affinity`, `atn`, `pngsuite`).
   - Tests decode real `.psd` or `.xcf` files, map them to `photocraft_doc::Document` via `photocraft-io`, render via `photocraft_compose::render`, and compare against Photoshop's embedded merged composite or GIMP 3.2.6's reference PNGs.
2. **Tier 2: CPU vs. GPU Parity (`crates/gpu`)**
   - Every GPU pipeline is tested against `photocraft_compose::render` on synthetic multi-layer fixtures covering all blend modes, masks, clipping groups, adjustments, and layer effects.
   - Maximum per-channel difference must be $\le 1/255$ on 8-bit output.

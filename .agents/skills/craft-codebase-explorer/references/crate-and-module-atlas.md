# Crate & Module Atlas (`L0`–`L7`)

This reference maps every crate and application in the PhotoCraft workspace to its key modules and types.

---

## `L0` Foundation Crates (Intra-layer chain: `geom` $\to$ `cms` $\to$ `color` $\to$ `raster`)

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-geom` | `crates/geom/src/lib.rs` | `Point`, `Rect`, `Affine`, `TileCoord`, Bezier curves (`kurbo`), perspective math. |
| `photocraft-cms` | `crates/cms/src/lib.rs` | Pure-Rust ICC v2/v4 color management: `Profile`, `Transform`, `RenderingIntent`, Black Point Compensation, soft-proofing, `transform::cached`, `gpu_matrix_shaper.rs` (4097/4096-entry float curves + 3×3 matrix), and 33³ 3D LUT generation. |
| `photocraft-color` | `crates/color/src/lib.rs` | `PixelFormat`, `SampleType` (`U8`, `U16`, `F32`), `ColorMode` (`Rgb`, `Gray`, `Cmyk`, `Lab`, `Indexed`, `Bitmap`, `Multichannel`), scalar reference blend mode math (`BlendMode`). |
| `photocraft-raster` | `crates/raster/src/lib.rs` | `Tile` (256×256 pixels, fallibly allocated via `try_reserve_exact`), `Surface` (sparse `BTreeMap<TileCoord, Arc<Tile>>` with copy-on-write `Arc::make_mut` and `default_pixel`), `Mask`. |

---

## `L0` Standalone Crates (Zero workspace dependencies; publishable)

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-psd` | `crates/psd/src/lib.rs` | Standalone Adobe PSD & PSB (large document) parser and encoder (`PsdFile`, header, color mode data, image resources, layer/mask records, tagged blocks, PackBits/ZIP compression). |
| `photocraft-codecs` | `crates/codecs/src/lib.rs` | Flat image decoders & encoders: PNG, JPEG, TIFF, WebP, OpenEXR, GIF, TGA, BMP, ICO, DDS (BC1/BC2/BC3 + mipmaps), DPX, Radiance HDR, PNM, SVG rasterization, PDF import/export. |
| `photocraft-heif` | `crates/heif/src/lib.rs` | Standalone HEIF/HEIC/AVIF decoder (used by `photocraft-codecs` via the sole `STANDALONE_EXCEPTIONS` rule behind the `heif` feature). |
| `photocraft-raw` | `crates/raw/src/lib.rs` | Camera RAW decoder and demosaicing pipeline (DNG, Canon CR2, Nikon NEF, Sony ARW, TIFF-EP). |
| `photocraft-affinity` | `crates/affinity/src/lib.rs` | Parser for Serif Affinity `.afphoto`, `.afdesign`, `.afpub` documents. |
| `photocraft-adobe-assets` | `crates/adobe-assets/src/lib.rs` | Parsers/writers for Adobe preset formats: brushes (`.abr`), swatches (`.aco`, `.ase`, `.act`), gradients (`.grd`), patterns (`.pat`), shapes (`.csh`), actions (`.atn`), styles (`.asl`), tool presets (`.tpl`), keyboard shortcuts (`.kys`). |
| `photocraft-tablet` | `crates/tablet/src/lib.rs` | Pen tablet pressure, tilt, and barrel button input (`macos.rs` AppKit `NSEvent` monitor and `windows.rs` `GetPointerPenInfo` are the **only** places in the entire workspace permitted to use `unsafe`, with `SAFETY:` comments; `x11.rs` is safe). |
| `photocraft-ipp` | `crates/ipp/src/lib.rs` | Pure-Rust IPP (Internet Printing Protocol) client for CUPS/network printing on Unix. |

---

## `L1` Document Model

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-doc` | `crates/doc/src/lib.rs` | Pure serializable document state: `Document`, `Layer`, `LayerId`, `LayerContent` (`Raster`, `Group`, `Adjustment`, `Fill`, `Text`, `Shape`, `Smart`), `Adjustment` variants, `LayerEffects` (`lfx2`), `SmartFilter`, `Channel`, `Guide`, `Slice`, `LayerComp`, metadata. |

---

## `L2` History & Domain Algorithms

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-ops` | `crates/ops/src/lib.rs` | `History`, `HistoryState`, `LayerTarget`. Whole-document `Arc<Document>` snapshot undo/redo bounded by state count and unique `Arc<Tile>` byte usage. |
| `photocraft-algo` | `crates/algo/src/lib.rs` | Pure CPU imaging algorithms parallelized per tile with `rayon`: blurs, sharpening, noise, distortions, Liquify, Puppet Warp, Content-Aware Fill/Move (PatchMatch), healing/inpainting, morphology, histograms, selections (marching ants, Select Subject, Color Range). |
| `photocraft-paint` | `crates/paint/src/lib.rs` | Brush dab engine, spacing, pressure/tilt/velocity dynamics, dual brush, wet edges, mixer brush, clone/healing stamp sampling, stroke smoothing. |
| `photocraft-text` | `crates/text/src/lib.rs` | Font database (`FontDb`), OpenType shaping and paragraph layout (`parley`/`skrifa`/`harfrust`), vertical/CJK text, optional embedded `CRAFT_FONTS` (`build.rs`) and web-served fonts (`served.rs`), glyph coverage rasterization. |
| `photocraft-vector` | `crates/vector/src/lib.rs` | Vector paths, Bezier editing, boolean path operations, stroke/fill coverage rasterization. |

---

## `L3` Compositors & Native Format

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-compose` | `crates/compose/src/lib.rs` | Authoritative CPU reference compositor ("the oracle"): `render(doc, rect) -> Buffer`. Implements all 27+ Photoshop blend modes (`psblend.rs`), native CMYK (`native.rs`) & Lab (`native_lab.rs`) compositing, adjustments (`adjust.rs`), layer effects (`effects.rs`), clipping groups, knockout, and `BlendIf`. |
| `photocraft-gpu` | `crates/gpu/src/lib.rs` | `wgpu` hardware compositor (`compose` $\to$ `gpu` intra-layer dependency allowed for LUTs & parity tests). Uses `rgba16float`/`rgba32float` targets and a tile residency cache; verified within $\le 1/255$ of `photocraft-compose`. |
| `photocraft-format` | `crates/format/src/lib.rs` | `.pcraft` native file bundle (`manifest.rs`, `convert.rs`). Content-addressed `tiles/<blake3>.zst` with per-document save-writer cache reuse and atomic OS-metadata-preserving saves. Exhaustively destructures `photocraft-doc` structs so unpersisted fields fail to compile. |

---

## `L4` File I/O Mapping & Plugins

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-io` | `crates/io/src/lib.rs` | Bridges `photocraft-doc::Document` $\leftrightarrow$ external formats: `psd_import.rs`, `psd_export.rs`, `tiff_layers.rs`, `ora.rs` (OpenRaster), `kra.rs` (Krita), `xcf.rs` (GIMP), `pdn.rs` (Paint.NET), `affinity.rs`, `raw.rs`, `flat.rs`, `pdf.rs`, `svg.rs`, and Adobe preset mappers (`abr_map.rs`, `effects_map.rs`, `text_styles_map.rs`, `vector_map.rs`). |
| `photocraft-plugins` | `crates/plugins/src/lib.rs` | Sandboxed WebAssembly filter plug-in host (`wasmi`) with fuel and memory limits. |

---

## `L5` Engine Façade

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-engine` | `crates/engine/src/lib.rs` | `Session` (multi-document manager, presets, actions, preferences, clipboard), `DocState` (`Arc<Document>`, `History`, `revision`, `last_damage`), `Session::execute(id, params)` (single command entry point), `Session::edit` (transactional COW snapshot + undo + damage), `commands.rs` (~700 `CommandSpec`s across `*_cmds.rs`), `param_check.rs` (strict param validation), `display_color.rs` (monitor ICC transform). |

---

## `L6` UI Shell, Automation & Testkit

| Crate | Path | Key Types & Responsibilities |
|---|---|---|
| `photocraft-ui-egui` | `crates/ui-egui/src/lib.rs` | Thin `egui`/`eframe` shell: `state.rs` (`UiState`, serializable), `menu_catalog.rs` (628 Photoshop menu items $\to$ command IDs), `menus.rs`, `canvas.rs` (viewport, overlays, GPU/CPU presentation), `panels.rs` & `dock.rs`, `dialogs.rs` & `filter_dialog.rs` (schema-driven filter previews), `theme.rs` (`Tokens`), `i18n.rs` (`tl!` macro), `control.rs` (TCP JSON control server). |
| `photocraft-automation` | `crates/automation/src/lib.rs` | `PhotocraftMcp` (`rmcp` MCP server), `Headless` session host, `rpc.rs` (`photocraft-cli serve` JSON-lines server), `BridgeClient` (forwards MCP calls to a live GUI `--control` port), `AuthorizedWorkspace` (`cap-std` sandboxed file roots). |
| `photocraft-testkit` | `crates/testkit/src/lib.rs` | Shared test builders, image diffing helpers, and synthetic fixture generators (`dev-dependency` only). |

---

## `L7` Applications & Workspace Tooling (`Exempt`)

| Binary / Tool | Path | Purpose |
|---|---|---|
| `photocraft` | `apps/photocraft/src/main.rs` | Native desktop app (`eframe`/`wgpu`), single-instance lock (`single_instance.rs`), GPU crash recovery marker (`gpu-starting.json`), logging (`logging.rs`), TCP `--control` listener. |
| `photocraft-cli` | `apps/photocraft-cli/src/lib.rs` | Headless CLI (`convert`, `info`, `run`, `batch`, `droplet`, `commands`, `mcp`, `serve`). |
| `photocraft-web` | `apps/photocraft-web/src/lib.rs` | Browser WebAssembly app (`trunk` + `wasm-bindgen`). |
| `photocraft-web-boot` | `apps/photocraft-web-boot/src/lib.rs` | Browser bootstrapper that reassembles compressed `.wasm` parts under CDN per-file size limits. |
| `xtask` | `xtask/src/main.rs` | Workspace build/test/verification tasks (`layers`, `wasm`, `ci`, `corpus`, `test-corpus`, `stats`, `parity`, `i18n-coverage`, `perf`, `scorecard`, `version`, `ico`). |

# Six-Act Curriculum Arc (`0002`–`0025`) Including `nom` & `turso` (`libsql`)

The learner approved a 24-lesson progression (`0002`–`0025`) across six 4-lesson acts that takes the Schematic & PCB Design Review System from a single-file immediate-mode window (`0000`–`0001`) through:
1. **Act I (`0002`–`0005`)**: 2D CAD canvas rendering, pan/zoom coordinate transforms, layer Z-ordering, and net hit-testing.
2. **Act II (`0006`–`0009`)**: Schematic symbols/text, dual-viewport `egui::Scene` cross-probing, virtualized BOM tables, and a hotkey CAD measurement ruler.
3. **Act III (`0010`–`0013`)**: Anchored review callout pins, triage workflows/context menus, DRC violation overlays + minimap, and custom `egui::Widget` layer swatches.
4. **Act IV (`0014`–`0017`)**: Multi-crate engine-first architecture (`doc` vs. `ui-egui`), `serde` JSON persistence, `Arc` copy-on-write snapshots, and undo/redo history with slider coalescing.
5. **Act V (`0018`–`0021`)**: Unified `CommandSpec` palette, spatial viewport culling + shape batching, background thread file loading, and headless snapshot testing + Wasm builds.
6. **Act VI (`0022`–`0025`)**: Real-world KiCad `.kicad_pcb` / `.kicad_sch` S-expression parsing using `nom` (as an `L0` standalone parser crate + `L4` `io` lowering) and embedded SQL review persistence, querying, audit logging, and replica sync using Turso (`libsql`).

## Implications
- Keep lessons `0002`–`0013` in a single, friction-free crate while building core `egui` + Rust fluency (`Copy`, `impl`, `Vec`, `enum`, `Option`, `&str`/`String`, borrowing splits, iterators, `HashMap`, custom `egui::Widget` traits, and lifetimes).
- Introduce multi-crate layering (`L0`–`L7`), `serde`, `Arc::make_mut`, and `CommandSpec` in Acts IV–V (`0014`–`0021`), preparing the exact architectural boundaries where `nom` (`L0` standalone `kicad-sexpr` crate + `L4` `io` crate) and `turso`/`libsql` (`L3`/`L5` async persistence & replica sync) slot in cleanly in Act VI (`0022`–`0025`).

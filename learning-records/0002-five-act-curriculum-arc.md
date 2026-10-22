# Five-Act Curriculum Arc (`0002`–`0021`)

The learner approved a 20-lesson progression (`0002`–`0021`) across five 4-lesson acts that takes the Schematic & PCB Design Review System from a single-file immediate-mode window (`0000`–`0001`) through 2D CAD canvas rendering (Act I), schematic cross-probing and virtualized BOM inspection (Act II), review pins/DRC workflows and custom widgets (Act III), multi-crate engine-first architecture with `Arc` copy-on-write undo/redo (Act IV), and a unified command engine, spatial culling, background jobs, and headless/Wasm builds (Act V).

## Implications
- Keep lessons `0002`–`0013` in a single, friction-free crate while building core `egui` + Rust fluency (`Copy`, `impl`, `Vec`, `enum`, `Option`, `&str`/`String`, borrowing splits, iterators, `HashMap`, custom `egui::Widget` traits, and lifetimes).
- Introduce multi-crate layering (`doc` vs. `ui-egui`), `serde`, `Arc::make_mut`, and `CommandSpec` in lessons `0014`–`0021` once the learner has concrete PCB and review state that genuinely benefits from undo/redo, persistence, and headless testing.

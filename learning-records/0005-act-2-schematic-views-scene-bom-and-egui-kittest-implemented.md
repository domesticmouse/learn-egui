# Act II (`0007`–`0010`) Implemented: Schematic Views, `egui::Scene`, Virtualized BOM & `egui_kittest`

Implemented both the runnable Rust/egui applications (`apps/0006-*` through `apps/0009-*`) and interactive HTML lessons (`lessons/0006-*` through `lessons/0009-*`) for Act II, advancing [[MISSION.md]] and following [[0003-continuous-testing-and-mutation-coverage.md]] and [[0004-act-1-2d-cad-canvas-and-viewport-implemented.md]].

## Verification Summary
- **`0006-schematic-symbols-and-text-rendering`**: 14 unit + `egui_kittest` tests pass; removed `<impl eframe::App for PcbReviewerApp>::update` from `.cargo/mutants.toml` exclusions; `cargo mutants --in-place` catches `35 / 35 viable mutants (10 unviable, 0 missed)`.
- **`0007-split-viewports-and-egui-scene`**: 16 unit + `egui_kittest` tests pass; `cargo mutants --in-place` catches `40 / 40 viable mutants (11 unviable, 0 missed)`.
- **`0008-searchable-netlist-and-bom-inspector`**: 17 unit + `egui_kittest` tests pass; `cargo mutants --in-place` catches `47 / 47 viable mutants (12 unviable, 0 missed)`.
- **`0009-keyboard-shortcuts-and-point-to-point-ruler`**: 19 unit + `egui_kittest` tests pass; `cargo mutants --in-place` catches `59 / 59 viable mutants (14 unviable, 0 missed)`.

## Implications
- Ready to advance to Act III (`0011`–`0014`: Design Review Workflows), covering anchored review pins (`HashMap<PinId, ReviewNote>`), triage popups (`Response::context_menu`, `ComboBox`), DRC violation overlays & minimap clamping, and custom `egui::Widget` layer swatches.

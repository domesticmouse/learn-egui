# Act I (`0003`–`0006`) Implemented: 2D CAD Canvas, `TSTransform`, Z-Order & Net Cross-Probing

Implemented both the runnable Rust/egui applications (`apps/0002-*` through `apps/0005-*`) and interactive HTML lessons (`lessons/0002-*` through `lessons/0005-*`) for Act I, advancing [[MISSION.md]] and following [[0003-continuous-testing-and-mutation-coverage.md]].

## Verification Summary
- **`0002-custom-2d-canvas-and-painter`**: 5 unit tests pass; `cargo mutants --in-place` catches `23 / 23 viable mutants (2 unviable, 0 missed)`.
- **`0003-world-to-screen-transforms-pan-and-zoom`**: 6 unit tests pass; `cargo mutants --in-place` catches `14 / 14 viable mutants (6 unviable, 0 missed)`.
- **`0004-modeling-layers-traces-and-footprints`**: 9 unit tests pass; `cargo mutants --in-place` catches `20 / 20 viable mutants (7 unviable, 0 missed)`.
- **`0005-hit-testing-and-net-cross-probing`**: 11 unit tests pass; `cargo mutants --in-place` catches `25 / 25 viable mutants (10 unviable, 0 missed)`.

## Implications
- Ready to advance to Act II (`0007`–`0010`), where Lesson `0007` introduces `egui_kittest::Harness` to test rendered UI widgets headlessly and remove `<impl eframe::App for PcbReviewerApp>::update` from `.cargo/mutants.toml` exclusions.

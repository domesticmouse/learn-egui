# Corpus Oracles, Performance Budgets, Scorecard & CI Reference

This reference provides step-by-step instructions for working with PhotoCraft's real-file corpora, performance benchmark suite, generated scorecard, and CI pipeline.

---

## 1. Real-File Test Corpora (`cargo xtask corpus` & `test-corpus`)

Real-world files are gitignored under `corpus/`, pinned in `xtask/src/corpus_pins.rs`, and verified against SHA-256 manifests (`xtask/*.sha256`).

### Corpus Inventory
| Flag | Directory | Contents & Oracle | Manifest |
|---|---|---|---|
| `--photoshop` | `corpus/photoshop/` | 256 Photoshop-authored PSDs (`smart-filters`, `effects`, `text`, `adjustments/<mode><bits>`) from `storytold/photocraft-corpus`. | `xtask/photoshop-corpus.sha256` |
| `--gimp` | `corpus/gimp/` | 14 GIMP 3.2.6 XCF files + 14 reference PNG renders from `storytold/photocraft-corpus`. | `xtask/gimp-corpus.sha256` |
| `--psd` | `corpus/psd/` | 170 small `psd-tools` and `ag-psd` files. | `xtask/psd-corpus.sha256` |
| `--psd-tools` | `corpus/psd-tools/` | Complete 309-file `psd-tools` suite. | `xtask/psd-tools-corpus.sha256` |
| `--heif` | `corpus/heif/` | 9 HEIC/HEIF files with Apple `.ref.png` decodes (requires `--features heif`). | `xtask/heif-corpus.sha256` |
| `--exr` | `corpus/exr/` | 5 OpenEXR v3.5.2 deep scanline images + sidecar ID manifests. | `xtask/exr-corpus.sha256` |
| `--affinity` | `corpus/affinity/` | 39 Affinity documents (`.afphoto`/`.afdesign`) with PNG references. | `xtask/affinity-corpus.sha256` |
| `--atn` | `corpus/atn/` | 16 Photoshop Actions (`.atn`) sets (byte-exact round-trip in `psd`, import/save in `engine`). | `xtask/atn-corpus.sha256` |
| `--pngsuite` | `corpus/pngsuite/` | Schaik PngSuite archive. | Fixed archive SHA-256 |
| `--pixls` | `corpus/pixls/` | Opt-in camera RAW samples (`cargo xtask test-corpus --pixls` runs `photocraft-raw`). | `xtask/src/pinned.rs` |

### Running & Debugging Corpus Tests
```sh
# Check status of all corpora
cargo xtask corpus

# Fetch all standard corpora (everything except --pixls)
cargo xtask corpus --all

# Run corpus tests faster by disabling thin LTO (matches CI's 2.7x build speedup)
CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  cargo xtask test-corpus -p photocraft-io -- --nocapture

# Inspect a single PSD's pixel diff vs. Photoshop's merged image (writes ours | Photoshop | heatmap)
cargo run --release -p photocraft-io --example oracle_diff -- \
  corpus/psd/<file>.psd 0 png /tmp/diff.png
```
Other `oracle_diff` subcommands: `col`, `row`, `worst [n]`, `grid x0 y0 x1 y1 [ch]`, `layerpx x y`, and `DUMP_FX=1` (prints raw effect descriptors).

### Two PSD Oracle Levels
1. **Import Composite Oracle (`crates/io/tests/corpus.rs`):**
   Flattens the imported `Document` (using Photoshop's cached pixels for Smart Objects and Type layers) and compares against the PSD's merged image (`PASS_TOL = 2.0 / 255.0`), or embedded JPEG thumbnail (resource 1036) when Maximize Compatibility was off. Also verifies `export -> re-import -> flatten` matches within `ROUNDTRIP_TOL = 1.0 / 255.0` and passes `common::strict_block_errors`.
2. **Re-Rendered Smart Object & Text Oracle (`crates/engine/tests/photoshop_oracles.rs`):**
   Discards Photoshop's cached pixels on every `LayerContent::Smart` and `LayerContent::Text` layer, re-renders them through `photocraft-engine`'s smart-filter stack and text engine, and compares against Photoshop's merged composite.

### Testing Local `photocraft-corpus` Changes & Bumping Pins
1. Clone `storytold/photocraft-corpus` as `../photocraft-corpus` (or set `PHOTOCRAFT_CORPUS_REPO`).
2. Test uncommitted/local corpus files with `cargo xtask test-corpus --local`.
3. Once pushed to `photocraft-corpus`, update `PHOTOCRAFT_CORPUS_COMMIT` (or `PHOTOCRAFT_CORPUS_GIMP_COMMIT`) in `xtask/src/corpus_pins.rs`.
4. Run `cargo xtask corpus --photoshop --update-manifest` (or `--gimp --update-manifest`), run `cargo xtask test-corpus`, raise any improved floors in `crates/io/tests/corpus.rs` or `crates/engine/tests/photoshop_oracles.rs`, and run `cargo xtask scorecard`.

---

## 2. Performance Benchmarks (`cargo xtask perf`)

`xtask perf` runs release benchmark examples with `--json`, merges their outputs into `target/perf/results.json` and `target/perf/summary.md`, and checks them against `perf/budgets.toml` and `perf/baseline.json`.

### How a Benchmark Example Reports Metrics (`photocraft_testkit::perf`)
Every benchmark registered in `perf/budgets.toml` uses `photocraft_testkit::perf`:
- `RssSampler::start(Duration::from_millis(2))` polls resident memory on a background thread; call `sampler.reset()` before each scenario and `sampler.peak()` after.
- `perf::row(name, &samples_ms, peak_rss, gpu_bytes)` computes nearest-rank `p50_ms`, `p95_ms`, and `max_ms`.
- `perf::report(bench_name, context_json, rows, Some(&sampler))` wraps the rows with `process_peak_rss_bytes` (`VmHWM` on Linux).
- `perf::write_report(path, &report)` writes the JSON file expected by `xtask perf`.

### Adding or Updating a Performance Scenario
1. Add the measurement row to the benchmark example (e.g., `crates/ui-egui/examples/perf_scenarios.rs`, `interactive_bench.rs`, `fx_bench.rs`, `bevel_bench.rs`, `type_bench.rs`, `large_image_bench.rs`). Keep the row's `name` stable.
2. Add a `[[scenario]]` entry in `perf/budgets.toml` linking `bench` and `row` to its `id` (`P1`..`Pn`), `metric` (`p50` | `p95` | `max`), `budget_ms`, and `enforce = true|false`.
   - `enforce = false`: reported as `over budget (target not met yet)` in the scorecard; only p50 regressions > 15% (and > 1.0 ms) against the same machine class fail the run.
   - Flip `enforce = true` in the PR that meets the budget.
3. Run `cargo xtask perf --quick` locally to verify the harness, then regenerate `cargo xtask scorecard`.
4. Only update `perf/baseline.json` via `cargo xtask perf --update-baseline` on an idle machine (check the 1/5/15-min load average recorded in the baseline header—noisy background load invalidates latency numbers).

---

## 3. The Self-Auditing Scorecard (`cargo xtask scorecard`)

`docs/scorecard.md` is generated deterministically from committed files (`xtask/src/scorecard.rs`). Never edit `docs/scorecard.md` by hand; CI enforces `cargo xtask scorecard --check`.

### What `xtask scorecard` Reads
1. **`perf/budgets.toml` + `perf/baseline.json`:** The 60 performance scenarios table.
2. **`crates/io/tests/corpus.rs`:** Parses `const *: Source = Source { .. }` blocks and their doc comments to extract corpus file counts, `pass_floor`, and `roundtrip_floor`.
3. **`scorecard/*.toml`:** Per-area TOML checklists (`file_compat.toml`, `tools.toml`, `ui.toml`, `type.toml`, `automation.toml`, `reliability.toml`, `distribution.toml`, `bugs.toml`).
   - Note: `target` and `note` fields must not contain `|` (breaks Markdown tables), and `id`s must be globally unique.
4. **Dead Preferences Audit:** Automatically scans `crates/engine/src/prefs.rs` for `Preferences` struct fields that are never read outside `prefs.rs` and `prefs_ui.rs` (target: 0 dead settings).
5. **Source Tree Audits:** Counts `#![deny(clippy::unwrap_used, ...)]` coverage across all workspace crates (`30 of 30`), checks for `panic_hunt.rs`, and reads live menu parity from `docs/parity-checklist.md`.

---

## 4. CI Pipeline Architecture (`.github/workflows/ci.yml`)

Understanding how CI runs tests helps diagnose failures and avoid slowing down builds:
- **Dynamic Platform Selection (`platforms` job):**
  - `ubuntu-latest` runs on every PR.
  - `windows-latest` splits into two parallel legs (`unit`: `--lib --bins`; `integration`: `--test '*'`) and runs on `main`, `full-ci`-labeled PRs, or when platform-sensitive paths (`apps/`, `crates/{ui-egui,text,gpu,tablet,heif}/`, `Cargo.lock`, `build.rs`) change.
  - `web` (`packaging/web/package.sh` + 24 MiB per-file gate) runs when web or crate sources change.
  - `macos` is reserved for `release.yml`.
- **Test Variants (`test-variants` job):**
  - `without heif`: verifies `photocraft-heif`, `photocraft-codecs`, and `photocraft-io` compile and return clear errors when the optional `heif` feature is disabled.
  - `with craft-fonts`: checks out `storytold/craft-fonts` at the pinned commit and sets `CRAFT_FONTS_DIR` + `CRAFT_FONTS_REQUIRED=1`, running the Japanese/CJK font tests in `photocraft-text`, `photocraft-io`, and `photocraft-ui-egui` that skip when `CRAFT_FONTS` is empty.
- **Cache Discipline:**
  - Only `refs/heads/main` saves `rust-cache` and `sccache` entries; PRs restore `main`'s cache in read-only mode so PR caches do not evict `main` within GitHub's 10 GB cache limit.
  - Whenever running follow-up `nextest` steps (like `panic_hunt`), pass the **exact same** `--workspace --locked --features heif --lib --bins --tests` flags and filter with `-E` so Cargo does not invalidate feature metadata and recompile the workspace.

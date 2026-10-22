# Corpus Oracles, Performance Budgets, Scorecard & CI Reference

This reference provides step-by-step instructions for working with an engine-first `egui` application's real-file corpora, performance benchmark suite, generated scorecard, and CI pipeline.

---

## 1. Real-File Test Corpora (`cargo xtask corpus` & `test-corpus`)

Real-world test files are gitignored under `corpus/`, pinned in `xtask/src/corpus_pins.rs`, and verified against SHA-256 manifests (`xtask/*.sha256`).

### Corpus Design Pattern
| Component | Location | Purpose |
|---|---|---|
| **Pin Definitions** | `xtask/src/corpus_pins.rs` | Pins external repository commits or archive URLs for each format suite (e.g., `--<suite>`). |
| **SHA-256 Manifests** | `xtask/<suite>-corpus.sha256` | Verifies every downloaded file's cryptographic hash before running tests. |
| **Corpus Test Runner** | `crates/io/tests/corpus.rs` | Defines `Source` structs with `pass_floor` and `roundtrip_floor` ratchets. |
| **Diagnostic Diff Tool** | `crates/io/examples/oracle_diff.rs` | Inspects differences between the application's output/render and the reference oracle for a single file. |

### Running & Debugging Corpus Tests
```sh
# Check status of all corpora
cargo xtask corpus

# Fetch all standard corpora
cargo xtask corpus --all

# Run corpus tests faster by disabling thin LTO (matches CI's build speedup)
CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  cargo xtask test-corpus -p <app>-io -- --nocapture

# Inspect a single file's diff against its reference oracle
cargo run --release -p <app>-io --example oracle_diff -- \
  corpus/<suite>/<file>.ext
```

### Multi-Level Oracle Verification
1. **Import & Reference Oracle (`crates/io/tests/corpus.rs`):**
   Imports the external file into a `doc::Document`, evaluates or renders it, and compares against the reference oracle within a defined tolerance. Also verifies `export -> re-import` round-trip fidelity and structural invariants.
2. **Full Engine Re-Evaluation Oracle (`crates/engine/tests/*_oracles.rs`):**
   Discards any pre-cached previews or baked outputs stored in the source file, re-evaluates all procedural/computed elements through `crates/engine`, and compares against the reference composite/output.

### Testing Local Corpus Changes & Bumping Pins
1. Clone the external corpus repository as `../<app>-corpus` (or set the corresponding `*_CORPUS_REPO` environment variable).
2. Test uncommitted/local corpus files with `cargo xtask test-corpus --local`.
3. Once pushed to the corpus repository, update the pinned commit constant in `xtask/src/corpus_pins.rs`.
4. Run `cargo xtask corpus --<suite> --update-manifest`, run `cargo xtask test-corpus`, raise any improved floors in `crates/io/tests/corpus.rs`, and regenerate `cargo xtask scorecard`.

---

## 2. Performance Benchmarks (`cargo xtask perf`)

`xtask perf` runs release benchmark examples with `--json`, merges their outputs into `target/perf/results.json` and `target/perf/summary.md`, and checks them against `perf/budgets.toml` and `perf/baseline.json`.

### How a Benchmark Example Reports Metrics (`testkit::perf`)
Every benchmark registered in `perf/budgets.toml` uses `crates/testkit/src/perf.rs`:
- `RssSampler::start(Duration::from_millis(2))` polls resident memory on a background thread; call `sampler.reset()` before each scenario and `sampler.peak()` after.
- `perf::row(name, &samples_ms, peak_rss, gpu_bytes)` computes nearest-rank `p50_ms`, `p95_ms`, and `max_ms`.
- `perf::report(bench_name, context_json, rows, Some(&sampler))` wraps the rows with `process_peak_rss_bytes` (`VmHWM` on Linux).
- `perf::write_report(path, &report)` writes the JSON file expected by `xtask perf`.

### Adding or Updating a Performance Scenario
1. Add the measurement row to the benchmark example (e.g., `crates/ui-egui/examples/perf_scenarios.rs` or `interactive_bench.rs`). Keep the row's `name` stable.
2. Add a `[[scenario]]` entry in `perf/budgets.toml` linking `bench` and `row` to its `id` (`P1`..`Pn`), `metric` (`p50` | `p95` | `max`), `budget_ms`, and `enforce = true|false`.
   - `enforce = false`: reported as `over budget (target not met yet)` in the scorecard; only p50 regressions > 15% (and > 1.0 ms) against the same machine class fail the run.
   - Flip `enforce = true` in the PR that meets the budget.
3. Run `cargo xtask perf --quick` locally to verify the harness, then regenerate `cargo xtask scorecard`.
4. Only update `perf/baseline.json` via `cargo xtask perf --update-baseline` on an idle machine (check the 1/5/15-min load average recorded in the baseline header—noisy background load invalidates latency numbers).

---

## 3. The Self-Auditing Scorecard (`cargo xtask scorecard`)

`docs/scorecard.md` is generated deterministically from committed files (`xtask/src/scorecard.rs`). Never edit `docs/scorecard.md` by hand; CI enforces `cargo xtask scorecard --check`.

### What `xtask scorecard` Reads
1. **`perf/budgets.toml` + `perf/baseline.json`:** The performance scenarios table.
2. **`crates/io/tests/corpus.rs`:** Parses `const *: Source = Source { .. }` blocks and their doc comments to extract corpus file counts, `pass_floor`, and `roundtrip_floor`.
3. **`scorecard/*.toml`:** Per-area TOML checklists (e.g., `file_compat.toml`, `tools.toml`, `ui.toml`, `automation.toml`, `reliability.toml`, `distribution.toml`, `bugs.toml`).
   - Note: `target` and `note` fields must not contain `|` (breaks Markdown tables), and `id`s must be globally unique.
4. **Dead Preferences Audit:** Automatically scans `crates/engine/src/prefs.rs` for `Preferences` struct fields that are never read outside `prefs.rs` and `prefs_ui.rs` (target: 0 dead settings).
5. **Source Tree Audits:** Counts `#![deny(clippy::unwrap_used, ...)]` coverage across all workspace crates, checks for `panic_hunt.rs`, and reads live menu parity from `docs/parity-checklist.md`.

---

## 4. CI Pipeline Architecture (`.github/workflows/ci.yml`)

Understanding how CI runs tests helps diagnose failures and avoid slowing down builds:
- **Dynamic Platform Selection (`platforms` job):**
  - `ubuntu-latest` runs on every PR.
  - `windows-latest` splits into two parallel legs (`unit`: `--lib --bins`; `integration`: `--test '*'`) and runs on `main`, `full-ci`-labeled PRs, or when platform-sensitive paths (`apps/`, `crates/{ui-egui,gpu}/`, `Cargo.lock`, `build.rs`) change.
  - `web` (WebAssembly bundle + per-file size gate) runs when web or crate sources change.
  - `macos` can be reserved for release workflows or platform-specific changes.
- **Test Variants (`test-variants` job):**
  - **Optional feature matrix:** Verifies crates compile and return clear errors when optional native/codec features are disabled.
  - **External asset/font fixtures:** Checks out pinned test assets (e.g., CJK/Unicode fonts or domain symbol libraries) and sets `*_REQUIRED=1` so tests that skip locally when optional assets are absent are strictly enforced in CI.
- **Cache Discipline:**
  - Only `refs/heads/main` saves `rust-cache` and `sccache` entries; PRs restore `main`'s cache in read-only mode so PR caches do not evict `main` within GitHub's 10 GB cache limit.
  - Whenever running follow-up `nextest` steps (like `panic_hunt`), pass the **exact same** `--workspace --locked --features <features> --lib --bins --tests` flags and filter with `-E` so Cargo does not invalidate feature metadata and recompile the workspace.

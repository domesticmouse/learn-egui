# End-to-End Feature Tracing & Programmatic Inspection

Use these recipes to trace how any feature works from the UI or CLI down to pixel tiles, or to inspect the running engine.

---

## 1. Recipe: Trace a Menu Item or Engine Command

Suppose you want to understand how **Filter ▸ Blur ▸ Gaussian Blur…** works end to end:

1. **Find the canonical Command ID (`crates/ui-egui/src/menu_catalog.rs`):**
   Search `menu_catalog.rs` for `"Gaussian Blur"` $\to$ reveals command ID `"filter.blur.gaussianBlur"`.
2. **Locate the `CommandSpec` in `crates/engine`:**
   Grep for `"filter.blur.gaussianBlur"` in `crates/engine/src/` (or run `cargo run -p photocraft-cli -- commands --filter gaussianBlur`).
   - Check `crates/engine/src/commands.rs` to see every `*_cmds::specs()` module registered in the engine.
   - Inspect the `CommandSpec` fields: `enabled` predicate, `params` schema string, and `run` function.
3. **Check strict parameter validation (`crates/engine/src/param_check.rs`):**
   See whether the command ID is listed in `STRICT` (which enforces that unknown JSON keys are rejected).
4. **Follow `Session::edit` into the `L2` algorithm (`crates/algo`):**
   Inside `run`, follow how the command resolves the edit target (active pixel layer vs. Smart Object `SmartFilter` vs. layer mask vs. alpha channel), checks layer locks, calls into `photocraft_algo`, blends through `doc.selection`, and commits one COW snapshot via `Session::edit`.
5. **Check the UI dialog (if interactive):**
   Search `crates/ui-egui/src/filter_dialog.rs`, `dialogs.rs`, or `adjust_dialog.rs` for the command ID to see how live on-canvas preview and schema sliders are wired.

---

## 2. Recipe: Trace a Canvas Tool or Dock Panel

1. **Locate UI state in `crates/ui-egui/src/state.rs`:**
   - Every tool, panel tab, active dialog, and tool option is stored on `UiState` (serializable via `serde` so the control channel can inspect and mutate it).
2. **Trace canvas pointer/keyboard handling (`crates/ui-egui/src/canvas.rs`):**
   - `canvas.rs` translates viewport coordinates, dispatches pointer down/drag/up events to the active tool module (e.g., `brush_*.rs`, `crop_*.rs`, `transform_*.rs`, `text_*.rs`, `anchor_tools.rs`), and draws vector overlays (marching ants, transform handles, guides, brush cursors).
3. **Trace how the tool mutates the document:**
   - Tools never mutate `Document` directly in `ui-egui`; they invoke `session.execute(cmd_id, params)` (passing `"coalesce": "<gesture-id>"` during drags so intermediate frames merge into one undo step).

---

## 3. Recipe: Trace File Import / Export Fidelity

1. **Identify the format parser (`L0 Standalone` or `L3`):**
   - Native `.pcraft` $\to$ `crates/format/src/manifest.rs` & `convert.rs`.
   - Adobe `.psd` / `.psb` $\to$ `crates/psd/src/lib.rs`.
   - Flat formats (PNG, JPEG, TIFF, WebP, EXR, etc.) $\to$ `crates/codecs/src/lib.rs`.
   - Camera RAW $\to$ `crates/raw/src/lib.rs`.
   - Affinity $\to$ `crates/affinity/src/lib.rs`.
   - Adobe presets (`.abr`, `.atn`, `.asl`, `.grd`, `.pat`, `.aco`) $\to$ `crates/adobe-assets/src/lib.rs`.
2. **Trace conversion to/from `photocraft_doc::Document` (`crates/io`):**
   - `crates/io/src/psd_import.rs` & `psd_export.rs` map PSD layers, masks, adjustment blocks (`adjust_map.rs`), layer styles (`effects_map.rs`), text (`text_import.rs`, `text_styles_map.rs`), vector masks (`vector_map.rs`), and Smart Objects (`smart_map.rs`), while preserving unmodeled PSD blocks verbatim in `Layer::psd_blocks`.
   - Other multi-layer formats live in `crates/io/src/{tiff_layers,ora,kra,xcf,pdn,affinity}.rs`.
3. **Run corpus oracle tests:**
   ```sh
   cargo xtask test-corpus -p photocraft-io
   ```

---

## 4. Programmatic Inspection & Debugging Workflows

### A. Query the Headless CLI
```sh
# List all ~700 commands or filter by substring
cargo run -p photocraft-cli -- commands --filter layer.new

# Dump a file's full layer tree and metadata as JSON
cargo run -p photocraft-cli -- info path/to/doc.psd

# Execute a sequence of commands headlessly and save the result
cargo run -p photocraft-cli -- run path/to/in.psd \
  --cmd filter.blur.gaussianBlur --params '{"radius": 4.0}' \
  --out /tmp/out.png
```

### B. Interactive Headless JSON-Lines Session (`photocraft-cli serve`)
`photocraft-cli serve` keeps a persistent `Headless` session open over stdio or loopback TCP (`docs/control-protocol.md#headless-server`):
```sh
printf '{"id":1,"method":"doc.new","params":{"width":64,"height":64}}\n{"id":2,"method":"doc.inspect","params":{}}\n' \
  | cargo run -q -p photocraft-cli -- serve
```

### C. Visual UI Snapshot & Live Control Channel
- **Offscreen PNG snapshot (fast, headless, no window):**
  ```sh
  cargo run -p photocraft-ui-egui --example snapshot
  ```
- **Live GUI with TCP control (`--control`):**
  ```sh
  cargo run --release -p photocraft -- --new-instance \
    --control 7878 --control-token-file .private/control.token \
    --automation-read-root work --automation-write-root work
  ```
  Send JSON-lines requests (`ui.inspect`, `ui.screenshot`, `command.run`, `ui.pointer`) to `127.0.0.1:7878` as documented in `docs/control-protocol.md`.

### D. Check Parity, Scorecard & Performance Budgets
```sh
cargo xtask parity              # Regenerate docs/parity-checklist.md (menu coverage)
cargo xtask scorecard --check   # Verify docs/scorecard.md (checklists, perf budgets, corpus floors, dead prefs)
cargo xtask i18n-coverage       # Check UI localization coverage across registered languages
```

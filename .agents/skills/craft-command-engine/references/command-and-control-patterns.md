# CommandSpec, Execution & Automation Patterns

## 1. `CommandSpec` Structure (`crates/engine/src/commands.rs`)

```rust
pub type Run = fn(&mut Session, &Value) -> Result<Value>;
pub type Enabled = fn(&Session) -> std::result::Result<(), String>;

pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    /// Menu placement, e.g. `["Filter", "Blur"]`. Empty = not in menus.
    pub menu: &'static [&'static str],
    /// Default shortcut in egui-style notation (`Cmd+Shift+N`), mapped per platform by the UI.
    pub shortcut: Option<&'static str>,
    /// Human/agent-readable parameter description (JSON-ish).
    pub params: &'static str,
    pub enabled: Enabled,
    pub run: Run,
    /// Record in the session journal (false for queries).
    pub journal: bool,
}
```

## 2. Writing Informative `Enabled` Predicates

Instead of returning `bool`, `Enabled` returns `Result<(), String>` so both the GUI and AI agents (via MCP `command_list`) know *why* a command is unavailable:

```rust
fn has_pixel_layer(s: &Session) -> std::result::Result<(), String> {
    let l = crate::active_layer_of(s)?;
    if matches!(l.content, LayerContent::Raster(_)) {
        Ok(())
    } else {
        Err(format!(
            "active layer is {} {} layer, not a pixel layer",
            l.content.article(),
            l.content.kind_name()
        ))
    }
}
```

When `command_list` is queried, `commands::list_entry` exposes:
- `"enabled": bool`
- `"disabledReason": Option<String>`
- `"needsDocument": bool` (distinguishes "open a document first" from preconditions like "no selection")

## 3. Session Dispatch Lifecycle (`precheck`, `admit`, `execute`, `edit`)

When `Session::execute(id, params)` (or `Session::start` for background jobs in `crates/engine/src/jobs.rs`) runs:
1. **`Session::precheck(id, params)`:** Validates that the command exists, `params` is a JSON object/null, strict parameter keys pass `param_check::check`, and any explicit `"layer"` param resolves to an existing layer (`commands::explicit_layer` accepts either a `u64` `LayerId` or a layer name string).
2. **`Session::admit(id, spec, params)`:**
   - Injects the Channels panel target (`channel_cmds::inject_target`) and `__kind` for table-driven adjustment commands (`commands::inject_kind`).
   - Evaluates the command's `enabled` precondition (on the explicitly named `"layer"` if provided, without changing the active layer).
   - Rejects conflicting edits while a background job is running on the document (`job_conflict`) or when painting/moving a hidden layer (`hidden_target::refusal`).
3. **Live Stroke Commit Optimization (`offer_live_stroke` / `offer_live_retouch`):** Interactive canvas strokes offer their already-rendered tiles to the session before calling `paint.stroke`, `paint.pencil`, `paint.cloneStamp`, or `paint.historyBrush`; if the command's parameters match the live stroke, it adopts the rendered surface (`"fromLive": true`) while still going through normal authorization, journaling, and undo history.
4. **`Session::edit(label, closure)`:** Clones the `Arc<Document>` (O(1) copy-on-write tiles), runs the mutation, restricts to a targeted color channel if active, updates Color Sampler and symmetry coordinates if canvas geometry changed, computes tight pixel damage (`step_damage`), coalesces history steps when `"coalesce"` matches the prior step, and bumps `DocState::revision`.

## 4. Respecting Context Inside `run`

A pixel/filter command's `run` implementation should check:
1. **Parameter validity & strict key check:** Parse and bounds-check numbers (`is_finite()`, range limits, `int_i32`, `u32_id`, `finite_f32`) returning `EngineError::BadParams`. Commands listed in `STRICT` (`crates/engine/src/param_check.rs`) automatically reject any parameter key not documented in `CommandSpec::params` (except the universal `"coalesce"` and `"target"` keys).
2. **Edit target (Pixels vs. Smart Filter vs. Mask vs. Alpha Channel):** Check `is_mask_target(params)` or `channel_cmds::edits_channel(session)` first so filters/adjustments work seamlessly on layer masks, Quick Mask, and alpha channels; when targeting a Smart Object layer (`LayerContent::Smart`), append or update a non-destructive `SmartFilter` on the layer instead of rasterizing.
3. **Locks:** Call `check_pixels_unlocked(doc, layer_id)?` or `check_all_unlocked(doc, layer_id)?`.
4. **Selection Mask & Feathering:** If `doc.selection` is `Some(mask)`, blend the modified surface with the original surface weighted by the selection mask (and layer `locks.transparency`).
5. **Coalescing Interactive Edits:** Callers dragging a slider or typing pass `"coalesce": "<gesture-id>"` in `params`; `Session::execute` extracts this key so `Session::edit` merges consecutive steps with the same key into a single undo entry.

## 5. Multi-Frontend Dispatch & UI Shell Interception

Because `Session::execute(&mut self, id: &str, params: Value) -> Result<Value>` is the single engine entry point:
- **GUI (`photocraft-ui-egui/src/menus.rs::invoke`):**
  - Checks `modal_allows` (blocks edits while modal dialogs, Camera Raw, or unsaved-changes prompts are open, except `NAV_COMMANDS`).
  - Handles `SHELL_COMMANDS` (`view.zoomIn`, `window.toggle.*`, `window.workspace.*`, `window.theme.*`, `edit.freeTransform`) and `TOOLS` (`tools.select.*`).
  - Intercepts zero-arg menu invocations (e.g., `file.new` or `layer.duplicate` with `{}`) to open UI dialogs/file pickers, or prompts to rasterize a Type layer before running a filter (`rasterize_prompt::intercept_filter`), then dispatches to `app.run(id, params)` $\to$ `Session::execute`.
- **Headless CLI (`apps/photocraft-cli`):**
  - `photocraft-cli commands [--json] [--filter <text>]` lists and searches registered commands (`list_entry_matches` searches `id`, `label`, and `SEARCH_KEYWORDS`).
  - `photocraft-cli run in.psd --cmd <id> --params '<json>' --out out.png` runs without a display server.
- **Headless JSON-Lines Server (`photocraft-cli serve`, `crates/automation/src/rpc.rs`):** Persistent newline-delimited JSON-RPC over stdio or loopback TCP (`--port <port>`) without MCP framing (`engine.execute`, `engine.commands`, `doc.open`, `doc.save`, `doc.inspect`, `doc.render`, `batch`; passing `index` on `doc.*` methods targets that document for a single call without changing the active document).
- **TCP Control Server (`ui-egui/src/control.rs`):** Accepts authenticated JSON-lines requests (`command.run`, `command.list`, `ui.inspect`, `ui.screenshot`, `ui.pointer`, `ui.key`, `ui.menu.invoke`, `ui.gpu.simulateLoss`) to drive and visually verify a live app instance.
- **MCP Server (`crates/automation`):** Wraps `Session` (or bridges to a running GUI over TCP) with capability-restricted filesystem roots (`cap-std` via `AuthorizedWorkspace`), exposing `command_list`, `command_run`, `command_batch`, `doc_inspect`, and `doc_render_preview`.

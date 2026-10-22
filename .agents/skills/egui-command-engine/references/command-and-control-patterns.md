# CommandSpec, Execution & Automation Patterns

## 1. `CommandSpec` Structure (`crates/engine/src/commands.rs`)

```rust
pub type Run = fn(&mut Session, &Value) -> Result<Value>;
pub type Enabled = fn(&Session) -> std::result::Result<(), String>;

pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    /// Menu placement, e.g. `["Edit", "Transform"]`. Empty = not in menus.
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

Instead of returning `bool`, `Enabled` returns `Result<(), String>` so both the `egui` frontend (e.g., disabled menu item hover tooltips) and AI agents (via MCP `command_list`) know *why* a command is unavailable:

```rust
fn has_active_selection(s: &Session) -> std::result::Result<(), String> {
    let doc = s.active_doc().ok_or_else(|| "no open document".to_string())?;
    if !doc.selection.is_empty() {
        Ok(())
    } else {
        Err("nothing is selected".to_string())
    }
}
```

When `command_list` is queried, `commands::list_entry` exposes:
- `"enabled": bool`
- `"disabledReason": Option<String>`
- `"needsDocument": bool` (distinguishes "open a document first" from domain preconditions like "nothing is selected")

## 3. Session Dispatch Lifecycle (`precheck`, `admit`, `execute`, `edit`)

When `Session::execute(id, params)` (or `Session::start` for background jobs in `crates/engine/src/jobs.rs`) runs:
1. **`Session::precheck(id, params)`:** Validates that the command exists, `params` is a JSON object/null, strict parameter keys pass `param_check::check`, and any explicit target ID resolves to an existing entity.
2. **`Session::admit(id, spec, params)`:**
   - Evaluates the command's `enabled` precondition.
   - Rejects conflicting edits while a background job is running on the document (`job_conflict`).
3. **`Session::edit(label, closure)`:** Clones the `Arc<Document>` (O(1) copy-on-write data), runs the mutation closure, computes change regions/damage (`step_damage`), coalesces history steps when `"coalesce"` matches the prior step, and bumps `DocState::revision`.

## 4. Respecting Context Inside `run`

A command's `run` implementation should check:
1. **Parameter validity & strict key check:** Parse and bounds-check numbers (`is_finite()`, range limits) returning `EngineError::BadParams`. Commands listed in `STRICT` (`crates/engine/src/param_check.rs`) automatically reject any parameter key not documented in `CommandSpec::params` (except universal keys like `"coalesce"` and `"target"`).
2. **Target & Locks:** Verify that the target entity is unlocked and editable before mutating.
3. **Coalescing Interactive `egui` Edits:** Callers dragging an `egui::Slider`, `egui::DragValue`, or canvas handle pass `"coalesce": "<gesture-id>"` in `params`; `Session::execute` extracts this key so `Session::edit` merges consecutive steps from the same drag gesture into a single undo entry.

## 5. Multi-Frontend Dispatch & `egui` Shell Interception

Because `Session::execute(&mut self, id: &str, params: Value) -> Result<Value>` is the single engine entry point:
- **GUI (`ui-egui/src/menus.rs::invoke`):**
  - Checks `modal_allows` (blocks background edits while modal dialogs or unsaved-changes prompts are open, except navigation commands).
  - Handles `SHELL_COMMANDS` (`view.zoomIn`, `window.toggle.*`, `window.workspace.*`, `window.theme.*`) and `TOOLS` (`tools.select.*`).
  - Intercepts zero-arg menu invocations (e.g., `file.new` or `item.duplicate` with `{}`) to open `egui` dialogs or file pickers, then dispatches to `app.run(id, params)` $\to$ `Session::execute`.
- **Headless CLI (`apps/<app>-cli`):**
  - `<app>-cli commands [--json] [--filter <text>]` lists and searches registered commands.
  - `<app>-cli run in.ext --cmd <id> --params '<json>' --out out.ext` runs without a display server.
- **Headless JSON-Lines Server (`<app>-cli serve`, `crates/automation/src/rpc.rs`):** Persistent newline-delimited JSON-RPC over stdio or loopback TCP (`--port <port>`) without MCP framing (`engine.execute`, `engine.commands`, `doc.open`, `doc.save`, `doc.inspect`, `batch`).
- **TCP Control Server (`ui-egui/src/control.rs`):** Accepts authenticated JSON-lines requests (`command.run`, `command.list`, `ui.inspect`, `ui.screenshot`, `ui.pointer`, `ui.key`, `ui.menu.invoke`) to drive and visually verify a live `egui` instance.
- **MCP Server (`crates/automation`):** Wraps `Session` (or bridges to a running `egui` GUI over TCP) with capability-restricted filesystem roots (`cap-std` via `AuthorizedWorkspace`), exposing `command_list`, `command_run`, `command_batch`, and `doc_inspect`.

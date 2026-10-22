---
name: egui-command-engine
description: >-
  Use this skill when exploring, explaining, tracing, adding, or modifying
  commands in an engine-first egui application: understanding the three-tier
  command taxonomy (Engine CommandSpecs, Shell/Tool UI commands, and Automation
  transports), wiring egui menu items, implementing schema-driven dialogs, or
  writing panic_hunt fuzz tests.
---

# Engine-First Command Architecture & UI Wiring

In an engine-first `egui` architecture, **everything is a command**. Menu items, toolbar actions, keyboard shortcuts, interactive canvas/viewport gestures, panel controls, and external automation all flow through a unified command architecture in `crates/engine` and `crates/ui-egui`.

## Three-Tier Command Taxonomy

1. **Engine Commands (`CommandSpec` in `crates/engine/src/commands.rs` + `<domain>_cmds.rs`):**
   - Dispatched via `Session::execute(id, params)` (synchronous) or `Session::start(id, params)` (background jobs in `jobs.rs`).
   - Includes state-mutating commands (`journal: true`, recorded in undo/redo `History` via `Session::edit`) and read-only queries (`journal: false`, e.g. `session.inspect`, `document.inspect`, `command.list`, `job.status`).
2. **Shell & Tool Commands (`crates/engine/src/shell_commands.rs` & `crates/ui-egui/src/menus.rs`):**
   - Pure UI/window/view actions (`SHELL_COMMANDS`, re-exported as `menus::UI_COMMANDS`, e.g., `view.zoomIn`, `window.toggle.*`, `window.theme.*`) and toolbar tool-selection commands (`TOOLS`, `tools.select.<tool>`).
   - Declared in `crates/engine/src/shell_commands.rs` so headless sessions know every bindable shortcut ID for shortcut customization and conflict detection, and executed by `crates/ui-egui/src/menus.rs::invoke`.
   - `menus::invoke` also intercepts interactive menu clicks (e.g., `file.new` with empty params, or commands needing modal input) to open `egui` dialogs or file pickers before delegating to `Session::execute`.
3. **Transport & Automation Protocol Commands (`crates/automation` & `crates/ui-egui/src/control.rs`):**
   - External drivers wrap the engine registry: CLI (`<app>-cli commands` / `run`), headless JSON-RPC (`<app>-cli serve`: `engine.execute`, `engine.commands`, `doc.*`), MCP server (`command_list`, `command_run`, `command_batch`, `doc_inspect`), and the live GUI TCP control channel (`--control`: `command.run`, `command.list`, `ui.inspect`, `ui.screenshot`, `ui.pointer`, `ui.key`, `ui.menu.invoke`).

---

## Step-by-Step Checklist: Adding a Command

### 1. Define the Canonical Command ID
- Register or look up the menu item in `crates/ui-egui/src/menu_catalog.rs`.
- Using the exact ID from `menu_catalog.rs` automatically lights up the menu item in the `egui` menu bar without manual UI wiring.
- For non-menu actions (e.g., interactive drag gestures or sub-commands), use a consistent dotted camelCase convention (e.g., `node.transform.set`) and an empty menu path `&[]`.

### 2. Implement the Domain Logic in the Lowest Applicable Layer (`L0`–`L2`)
- Place pure domain logic and data transformations in `L0`–`L2` crates rather than inside `ui-egui` or command glue.
- Keep domain logic **pure and deterministic** so it can be unit-tested without spinning up a full session or UI.

### 3. Register the `CommandSpec` in `crates/engine`
- Create or update a focused command module `crates/engine/src/<domain>_cmds.rs` exposing `pub fn specs() -> Vec<CommandSpec>` and register it in `crates/engine/src/commands.rs`.
- Populate every `CommandSpec` field (see [command-and-control-patterns.md](./references/command-and-control-patterns.md)):
  - `id`, `label`, `menu`, `shortcut`
  - `params`: concise JSON schema doc string (e.g., `'{"scale":f32=1.0,"mode":"a|b"="a"}'`), read by agents and CLI tools via `command_list`. Add closed-schema commands to `STRICT` in `crates/engine/src/param_check.rs` so `Session::execute` rejects misspelled/unknown keys across all transports (`"coalesce"` and `"target"` are accepted universally).
  - `enabled`: `fn(&Session) -> Result<(), String>` returning a clear human/agent-readable disabled reason (shown in tooltips and returned by `command_list`).
  - `run`: `fn(&mut Session, &Value) -> Result<Value>` that validates parameters and records **one** undo step via `Session::edit`.
  - `journal`: `true` for state-mutating commands, `false` for read-only queries.

### 4. Validate Adversarial Input (Never Panic)
- Validate all JSON parameters before allocating or indexing: reject negative sizes, `NaN`/`inf`, zero divisors, or out-of-range IDs with `Err(EngineError::BadParams { .. })`.
- Never use `unwrap()`, `expect()`, or unchecked slice indexing `[i]`.

### 5. Wire `egui` Dialogs and State (If Parameterized)
- Parameterized commands can use schema-driven `egui` dialogs with live preview (`crates/ui-egui/src/dialogs.rs`).
- All UI state must live in `crates/ui-egui/src/state.rs` (`UiState`, serializable via `serde`) and use design tokens from `theme::Tokens` rather than hard-coded pixel/color constants.

### 6. Write Comprehensive Tests

Run the verification suite before finishing (see [egui-testing-infrastructure](../egui-testing-infrastructure/SKILL.md) for `panic_hunt`, menu parity ratchets, and MCP acceptance testing):

```sh
cargo test -p <app>-engine --lib --tests
cargo test -p <app>-engine --test panic_hunt -- --ignored
cargo clippy -p <app>-engine -p <app>-ui-egui --all-targets -- -D warnings
```

---
nav_id: "0029"
page_title: "Lesson 0029: Embedded egui MCP Inspector Workbench, Agent Playbooks & Act VII Scorecard — Learn Rust & egui"
act_badge: "Act VII • Agentic Hardware Review & MCP (rmcp)"
title: "0029. Embedded `egui` MCP Inspector Workbench, Agent Playbooks & Act VII Scorecard"
subtitle: "Embedding an offline MCP Inspector Workbench (`McpInspectorWorkbench`) inside the `egui` desktop app to switch between `HeadlessDuplex` and `BridgedLoopback` transports, log JSON-RPC 2.0 frames (`RpcFrameRecord`), execute 4-step Agent Review Playbooks, and seal the 30-lesson course scorecard."
card:
  num: "Lesson 0029"
  mutants: "548 / 548 Caught"
  title: "egui MCP Inspector Workbench & Agent Playbooks"
  summary: "Toggle between `HeadlessDuplex` and `BridgedLoopback` transports in `egui`, audit `RpcFrameRecord` JSON-RPC 2.0 logs, execute the 4-step Power Integrity Agent Playbook, and verify the complete 30-lesson course scorecard."
  egui_skill: "Embedded MCP Inspector Workbench, Transport Switcher & Agent Playbook Runner"
  rust_concept: "Dual-Transport State Machines (`McpTransportMode`), Auditable JSON-RPC Logs & Multi-Step Agent Playbooks"
  app_dir: "apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks"
---

## Why Every AI-Controllable Desktop App Needs an Embedded MCP Workbench

Debugging an MCP server through an external cloud LLM chat window is slow, expensive, and non-deterministic. By embedding an **MCP Inspector Workbench** (`McpInspectorWorkbench`) directly inside our `egui` desktop reviewer, hardware engineers and CI pipelines can toggle between **`McpTransportMode::HeadlessDuplex`** and **`McpTransportMode::BridgedLoopback`**, inspect every JSON-RPC 2.0 request/response frame (`RpcFrameRecord`), and execute deterministic multi-step **Agent Review Playbooks** (`run_power_integrity_playbook`) completely offline.{{sidenote: **Deterministic Agent Playbooks (`Tier 6` Acceptance):** Our 4-step Power Integrity Playbook chains `resources/read` (`kicad://board/summary`), `pcb_query_net` (`VCC_3V3`), `pcb_run_custom_drc` (`U1` clearance), and `pcb_add_review_pin` (+ cropped PNG capture when running in `BridgedLoopback` mode) into an auditable `AgentPlaybookReport`.}}

```mermaid
flowchart LR
    Mode["McpTransportMode<br/>HeadlessDuplex ⇄ BridgedLoopback"] --> Playbook["run_power_integrity_playbook()<br/>4-Step Deterministic Audit"]
    Playbook --> S1["1. resources/read<br/>kicad://board/summary"]
    S1 --> S2["2. tools/call<br/>pcb_query_net (VCC_3V3)"]
    S2 --> S3["3. tools/call<br/>pcb_run_custom_drc"]
    S3 --> S4["4. tools/call + Bridge<br/>Pin + 64×64 Cropped PNG"]
    style Playbook fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style S4 fill:#141a21,stroke:#3b8eea,color:#e6edf3
```

## Dual-Transport Workbench & 4-Step Power Integrity Playbook

```rust caption="McpTransportMode, RpcFrameRecord, and McpInspectorWorkbench in crates/pcb-doc/src/lib.rs"
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpTransportMode {
    HeadlessDuplex,
    BridgedLoopback,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AgentPlaybookReport {
    pub transport: McpTransportMode,
    pub frames_logged: usize,
    pub vcc_pad_count: usize,
    pub drc_violations: usize,
    pub pin_count: usize,
    pub snapshot_captured: bool,
    pub snapshot_bytes_len: usize,
}
```

{{widget:mcp-workbench-sim}}

:::exercise id="ex0029" title="Interactive Lab 2: Toggle MCP Transports & Execute the 4-Step Agent Playbook" badge="Skill Practice • MCP Workbench & Playbooks"
Complete `McpTransportMode::toggle` and step 4 of `McpInspectorWorkbench::run_power_integrity_playbook` in `crates/pcb-doc/src/lib.rs`.

```rust
impl McpTransportMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::HeadlessDuplex => {{blank:Self::BridgedLoopback|Switch from HeadlessDuplex to BridgedLoopback|Self::BridgedLoopback|185px}},
            Self::BridgedLoopback => Self::HeadlessDuplex,
        }
    }
}

// Inside McpInspectorWorkbench::run_power_integrity_playbook:
let (snapshot_captured, snapshot_bytes_len) = match self.transport {
    McpTransportMode::HeadlessDuplex => (false, 0),
    McpTransportMode::BridgedLoopback => {
        let snap = capture_cropped_region_png(board, 10, 10, 60, 50, {{blank:64, 64|Capture a 64x64 pixel cropped PNG region snapshot|64, 64|110px}})
            .map_err(|e| e.to_string())?;
        (true, snap.png_bytes.len())
    }
};
```

:::prompts
hint: Switch `Self::HeadlessDuplex` to `Self::BridgedLoopback`, and pass `64, 64` for the cropped PNG dimensions in `BridgedLoopback` mode.
success: Congratulations! You have completed all 30 lessons (`0000`–`0029`) of the Rust & `egui` Schematic and PCB Design Review System course!
:::
:::

:::verify id="verify-0029" title="Lab 3: Embedded MCP Inspector Workbench, Agent Playbooks & Final Act VII Scorecard (`0029`)" badge="cargo test --workspace + egui_kittest + cargo mutants" tier="Tier 1–7 (Full Course Release Gate)" caught="539 / 539 (100%)" missed="0 Missed"
Run the complete workspace test suite (including `mcp_inspector_workbench_and_power_integrity_playbook_preserve_invariants`) and mutate the transport switcher, playbook runner, and UI workbench methods:

```bash caption="Run final Act VII MCP Inspector Workbench and Agent Playbook mutation gate on 0029"
cargo test --manifest-path ./apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks/Cargo.toml --workspace --in-place -F "McpTransportMode::toggle|McpTransportMode::label|McpInspectorWorkbench::run_power_integrity_playbook|toggle_mcp_transport|run_agent_playbook"
```

- [ ] `cargo test --manifest-path ./apps/0029-egui-mcp-inspector-workbench-and-agent-playbooks/Cargo.toml --workspace` passes all **31 tests** (**3** in `kicad-sexpr`, **22** in `pcb-doc`, and **6** in `pcb-reviewer`).
- [ ] `mcp_inspector_workbench_and_power_integrity_playbook_preserve_invariants` verifies `McpTransportMode::toggle`, 4-frame `RpcFrameRecord` sequencing across both `HeadlessDuplex` and `BridgedLoopback` transports, and $64 \times 64$ cropped PNG capture.
- [ ] `cargo mutants` catches **100% of viable workbench and playbook mutants** (`0 missed`), sealing the entire 30-lesson curriculum (`0000`–`0029`) with **zero warnings** and **zero missed mutants**!
:::

:::quiz id="quiz-0029"
Q: What is the primary advantage of embedding `McpInspectorWorkbench` directly inside the `egui` desktop reviewer?
- [ ] It replaces the Rust compiler with an interpreted JavaScript runtime
- [x] Engineers can deterministically test and audit MCP tools completely offline
- [ ] It automatically uploads proprietary KiCad layouts to public training servers
- [ ] It disables bearer token verification on local loopback network sockets
why: An embedded MCP workbench lets hardware engineers and CI pipelines exercise every MCP resource, tool, prompt, and GUI bridge command deterministically and completely offline.
hint: Review how `McpInspectorWorkbench` executes `run_power_integrity_playbook` without external API keys.

Q: How does `run_power_integrity_playbook` behave differently when switched from `HeadlessDuplex` to `BridgedLoopback`?
- [ ] It skips the net topology and custom DRC steps to save cycles
- [ ] It returns an empty `RpcFrameRecord` log without inspecting the active board layout
- [x] It authenticates through `LoopbackControlBridge`, focuses the viewport, and captures a cropped PNG
- [ ] It deletes all existing review pins from the local Turso audit database
why: In `BridgedLoopback` mode, the playbook drives the live GUI bridge in addition to querying the engine, producing a verified $64 \times 64$ cropped PNG snapshot (`snapshot_captured == true`).
hint: Compare the `McpTransportMode::HeadlessDuplex` and `McpTransportMode::BridgedLoopback` arms in `run_power_integrity_playbook`.

Q: Across the full 30-lesson course (`0000`–`0029`), what invariant is enforced on every companion crate before a lesson is marked complete?
- [x] Every workspace passes `cargo test` with zero warnings and zero missed mutants under `cargo-mutants`
- [ ] Every crate must depend on an active external internet connection to run unit tests
- [ ] Only GUI code is tested while pure domain logic is excluded from mutation testing
- [ ] Warnings and panics are permitted as long as the binary compiles in release mode
why: Every single companion workspace from `0000` through `0029` enforces `#![forbid(unsafe_code)]`, `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, zero compiler warnings, and 100% caught mutants (`0 missed`).
hint: Check the `:::verify` block and `.cargo/mutants.toml` in each companion workspace.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [Model Context Protocol — Inspector & Debugging Guide](https://modelcontextprotocol.io/docs/tools/inspector) and [`egui_kittest` Documentation](https://docs.rs/egui_kittest/latest/egui_kittest/).
:::

:::teacher-prompt
**Act VII & Full 30-Lesson Curriculum (`0000`–`0029`) Complete!** You have built a complete, multi-crate **Rust & `egui` Schematic and PCB Design Review System** from immediate-mode fundamentals (`0000`) through custom 2D CAD viewports, headless `egui_kittest` verification, DRC overlays, Copy-on-Write `Arc` undo/redo, `CommandSpec` fuzzing, `nom` KiCad S-expression parsing, embedded Turso SQL transactions, and an official `rmcp` Model Context Protocol server + Inspector Workbench—all verified with **0 missed mutants**.
:::

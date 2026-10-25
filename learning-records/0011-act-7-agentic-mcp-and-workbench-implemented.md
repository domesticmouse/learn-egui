# Learning Record 0011 — Act VII (`0027`–`0030`) Implemented: Agentic Hardware Review & Model Context Protocol (`rmcp`)

- **Date:** 2026-10-25
- **Scope:** Implemented **Act VII (`0027`–`0030`)** of the Rust & `egui` Schematic and PCB Design Review System course, completing all 30 lessons (`0001`–`0030`) with official `rmcp` v1.4 Model Context Protocol integration, `schemars` JSON Schema 2020-12 introspection, constant-time bearer-authenticated loopback GUI control, magic-verified cropped PNG snapshots, an embedded `egui` MCP Inspector Workbench (`McpInspectorWorkbench`), four interactive browser simulators (`{{widget:spatial-query-sim}}`, `{{widget:headless-mcp-sim}}`, `{{widget:bridged-mcp-sim}}`, `{{widget:mcp-workbench-sim}}`), and **100% mutation coverage (`0 missed`)** across all four companion workspaces.

---

## 1. Companion Workspaces & Mutation Coverage Summary

| Lesson | Companion Workspace | Workspace Tests | New Viable Mutants Caught | Cumulative Workspace Mutants Caught |
| --- | --- | --- | --- | --- |
| [`0027`](../lessons/0027-llm-native-spatial-and-electrical-queries.html) | `apps/0027-llm-native-spatial-and-electrical-queries` | **28** (`3` in `kicad-sexpr`, `19` in `pcb-doc`, `6` in `pcb-reviewer`) | **21 / 21** (`7` unviable, `0` missed) | **497 / 497 (100%, 0 missed)** |
| [`0028`](../lessons/0028-headless-mcp-server-rmcp-and-mock-client.html) | `apps/0028-headless-mcp-server-rmcp-and-mock-client` | **29** (`3` in `kicad-sexpr`, `20` in `pcb-doc`, `6` in `pcb-reviewer`) | **10 / 10** (`15` unviable, `0` missed) | **507 / 507 (100%, 0 missed)** |
| [`0029`](../lessons/0029-bridged-mcp-control-channel-and-live-gui-snapshots.html) | `apps/0029-bridged-mcp-control-channel-and-live-gui-snapshots` | **30** (`3` in `kicad-sexpr`, `21` in `pcb-doc`, `6` in `pcb-reviewer`) | **26 / 26** (`1` unviable, `0` missed) | **533 / 533 (100%, 0 missed)** |
| [`0030`](../lessons/0030-egui-mcp-inspector-workbench-and-agent-playbooks.html) | `apps/0030-egui-mcp-inspector-workbench-and-agent-playbooks` | **31** (`3` in `kicad-sexpr`, `22` in `pcb-doc`, `6` in `pcb-reviewer`) | **6 / 6** (`2` unviable, `0` missed) | **539 / 539 (100%, 0 missed)** |

---

## 2. Key Architectural & Pedagogical Milestones in Act VII

1. **Lesson `0027` — LLM-Native Spatial, Topological & Custom DRC Queries (`schemars`)**:
   - Derived `schemars::JsonSchema` on `SpatialQueryParams`, `SpatialNeighborhoodReport`, `NetTopologyReport`, `CustomDrcParams`, and `CustomDrcReport` and exposed `spatial_query_json_schema()` via `schemars::schema_for!()`.
   - Implemented `BoardModel::query_spatial_neighborhood`, `BoardModel::query_net_topology`, and `BoardModel::run_custom_drc` (filtering copper traces via `PcbLayer::is_copper()`), wired into `SidePanel::right` (`"Query U1 Radius"`, `"Query VDD Net"`, `"Run Custom DRC"`).
2. **Lesson `0028` — Headless MCP Server (`rmcp` v1.4) & `tokio::io::duplex` Mock Client**:
   - Implemented `PcbMcpServer` using `#[tool_router]`, `#[tool]`, and `#[tool_handler(router = self.tool_router)]` over `Arc<Mutex<BoardModel>>`, exposing 4 typed MCP tools (`pcb_query_spatial`, `pcb_query_net`, `pcb_run_custom_drc`, `pcb_add_review_pin`), `read_kicad_resource` (`kicad://board/summary`, `kicad://bom`, `kicad://net/{name}`), and `build_review_prompt` (`review_power_integrity`, `triage_open_drc`).
   - Built `MockMcpClient` (`impl ClientHandler`) and `run_duplex_mcp_roundtrip` over `tokio::io::duplex(4096)` for deterministic in-memory MCP integration testing (`rmcp_headless_server_duplex_client_resources_and_prompts_work`).
3. **Lesson `0029` — Bridged MCP Control Channel (`127.0.0.1` + Bearer Token) & Cropped Live-GUI Snapshots**:
   - Implemented constant-time `verify_bearer_token` accumulating XOR byte differences (`usize::from(a ^ b)`) before branching.
   - Implemented `capture_cropped_region_png` with bounding-box validation (`min_x_mm >= max_x_mm`), pixel budget enforcement (`16_384` max pixels), and 8-byte `PNG_MAGIC_HEADER` prefixing (`\x89PNG\r\n\x1a\n`), dispatched through `LoopbackControlBridge::dispatch`.
4. **Lesson `0030` — Embedded `egui` MCP Inspector Workbench, Agent Playbooks & Final Course Scorecard**:
   - Embedded `McpInspectorWorkbench` directly inside `PcbReviewerApp` with transport switching (`McpTransportMode::HeadlessDuplex` $\leftrightarrow$ `McpTransportMode::BridgedLoopback`), 4-step `run_power_integrity_playbook`, and `RpcFrameRecord` audit telemetry.
   - Sealed the complete 30-lesson course (`0001`–`0030`) with **34 compiled HTML pages**, **31 workspace tests**, **zero compiler warnings**, and **539 / 539 caught mutants (`0 missed`)**.

---
name: egui-app-runner
description: >-
  Use this skill when building, running, launching, or debugging an engine-first
  egui application across runtime targets: the native desktop app (eframe/wgpu),
  headless CLI and batch runner, authenticated JSON TCP control channel and MCP
  server, offscreen egui visual snapshots, or WebAssembly browser builds (via
  Trunk or Docker). Also use when configuring GPU backend fallbacks or
  single-instance locks.
---

# Running, Driving & Debugging an egui Application

In a multi-crate Cargo workspace with separate binary targets (`<app>`, `<app>-cli`, `<app>-web`, `xtask`), always pass `-p <package>` to `cargo run` or `cargo build` to select the target binary:
- `<app>` (`apps/<app>`) — native desktop `eframe`/`egui`/`wgpu` GUI application
- `<app>-cli` (`apps/<app>-cli`) — headless CLI, persistent JSON-lines server (`serve`), and MCP server (`mcp`)
- `<app>-web` (`apps/<app>-web`) — WebAssembly browser application
- `xtask` (`xtask`) — workspace build, test, and verification runner (`cargo xtask ...`)

---

## 1. Run the Native Desktop App (`<app>`)

Use `--release` for smooth interactive frame rates (or configure `[profile.dev.package."*"] opt-level = 2` in `Cargo.toml` for responsive debug builds):

```sh
# Launch the desktop app
cargo run --release -p <app>

# Open one or more files directly on launch
cargo run --release -p <app> -- path/to/document.ext

# Force a separate instance if single-instance locking is enabled
cargo run --release -p <app> -- --new-instance
```

---

## 2. Run the Headless CLI (`<app>-cli`)

`apps/<app>-cli` drives the core `engine` and `io` crates without opening a window or requiring a display server:

```sh
# Inspect document metadata and state as JSON
cargo run --release -p <app>-cli -- info in.ext

# Execute engine commands on a document headlessly
cargo run --release -p <app>-cli -- run in.ext \
  --cmd item.create --params '{"name":"Example"}' \
  --out out.ext

# Search the command registry
cargo run --release -p <app>-cli -- commands --filter item
```

---

## 3. Drive the App Programmatically (Control Channel, Offscreen Snapshots & MCP)

### Offscreen Visual Snapshots (Headless `egui`)
Render `egui` UI layouts to an image without launching a window or stealing focus:
```sh
cargo run -p <app>-ui-egui --example snapshot
```

### Authenticated TCP Control Channel (`127.0.0.1`)
Start the desktop app with `--control` and a bearer token file, then send newline-delimited JSON-RPC requests to inspect `egui` state, invoke commands, simulate pointer/keyboard input, or capture screenshots:

```sh
# 1. Start the desktop app with control server enabled
cargo run --release -p <app> -- --control 7878 \
  --control-token-file .private/control.token \
  --automation-read-root work --automation-write-root work

# 2. Authenticate, run a command, and capture a screenshot
TOKEN=$(tr -d '\r\n' < .private/control.token)
printf '%s\n' \
  "{\"id\":\"auth\",\"method\":\"auth\",\"params\":{\"token\":\"$TOKEN\"}}" \
  '{"id":1,"method":"engine.execute","params":{"command":"item.create","params":{"name":"Demo"}}}' \
  '{"id":2,"method":"ui.screenshot","params":{"path":"work/shot.png"}}' \
  | nc 127.0.0.1 7878
```

### MCP Server (`rmcp` over stdio)

- **Headless MCP (in-process `Session`, no window):**
  ```sh
  cargo run --release -p <app>-cli -- mcp \
    --automation-read-root "$PWD/work" \
    --automation-write-root "$PWD/work"
  ```
- **Bridged MCP (live desktop `egui` UI + MCP server):**
  1. Launch the desktop app with `--control`, `--control-token-file`, and the `--automation-*-root` flags (in bridge mode, filesystem roots are configured on the desktop process; if the token file does not exist, the app creates it with a fresh 256-bit token and `0600` permissions):
     ```sh
     mkdir -p .private work
     cargo run --release -p <app> -- \
       --control 7878 \
       --control-token-file "$PWD/.private/control.token" \
       --automation-read-root "$PWD/work" \
       --automation-write-root "$PWD/work"
     ```
  2. Start the MCP bridge pointing at the desktop app's loopback port and token file:
     ```sh
     cargo run --release -p <app>-cli -- mcp \
       --bridge 127.0.0.1:7878 \
       --control-token-file "$PWD/.private/control.token"
     ```
  3. For MCP client configs (`mcpServers`), build `cargo build --release -p <app> -p <app>-cli` once and invoke the compiled `target/release/<app>-cli` binary directly so stdio startup doesn't wait on Cargo.
  - **Bridge vs. Headless tool availability:** `ui_*` tools (`ui_inspect`, `ui_set`, `ui_pointer`, `ui_menu_invoke`, `ui_screenshot`) and `control_call` work **only in bridge mode** (driving the live `egui` window). Engine/document tools (`command_run`, `command_list`, `doc_new`, `doc_open`, `doc_save`, `doc_inspect`, `jobs_list`, etc.) work in both.

---

## 4. Run the WebAssembly Browser App (`<app>-web`)

Prerequisites: `rustup target add wasm32-unknown-unknown` and `trunk`.

```sh
# Quick check that non-desktop crates compile cleanly for wasm32
cargo xtask wasm

# Serve locally with Trunk
cd apps/<app>-web && trunk serve --release
```

---

## 5. Troubleshooting, GPU Fallbacks & Environment Variables

For GPU startup recovery (`--safe-gpu`, `WGPU_BACKEND`), single-instance lock behavior (`instance.lock`), log rotation, live design token hot-reloading, and frame latency tracing, read [runtime-flags-and-debugging.md](./references/runtime-flags-and-debugging.md).

For running test suites, `egui_kittest` GPU serialization (`gpu_lock`), real-file corpus oracles, and performance benchmarks (`xtask perf`), use [egui-testing-infrastructure](../egui-testing-infrastructure/SKILL.md).

---
name: craft-app-runner
description: >-
  Use this skill when building, running, launching, or debugging PhotoCraft across
  all runtime targets: the native desktop app (photocraft), headless CLI and batch
  runner (photocraft-cli), authenticated JSON TCP control channel and MCP server,
  or WebAssembly browser build (photocraft-web via Trunk, packaging script, or Docker).
  Also use when configuring craft-fonts, GPU backend fallbacks, single-instance locks,
  or stroke latency tracing.
---

# Running, Driving & Debugging PhotoCraft

PhotoCraft is a multi-crate Cargo workspace with multiple binary packages (`photocraft`, `photocraft-cli`, `xtask`), so always pass `-p <package>` to `cargo run` or `cargo build` to select the target binary:
- `photocraft` (`apps/photocraft`) — native desktop egui/wgpu GUI app
- `photocraft-cli` (`apps/photocraft-cli`) — headless CLI, persistent JSON-lines server (`serve`), and MCP server (`mcp`)
- `photocraft-web` + `photocraft-web-boot` (`apps/photocraft-web*`) — WebAssembly browser application
- `xtask` (`xtask`) — workspace build, test, parity, and corpus runner (`cargo xtask ...`)

---

## 1. Run the Native Desktop App (`photocraft`)

Always use `--release` for interactive use (debug builds compile dependencies at `opt-level 2`, but pixel-heavy paths are significantly faster in release):

```sh
# Launch the desktop app
cargo run --release -p photocraft

# Open one or more files directly on launch
cargo run --release -p photocraft -- path/to/image.psd

# Force a separate instance if PhotoCraft is already running
cargo run --release -p photocraft -- --new-instance
```

### Bundling Optional CJK Fonts (`craft-fonts`)
Shared CJK/Japanese fonts (`BIZ UDPGothic`, `Shippori Mincho`, etc.) live in `storytold/craft-fonts` and are embedded at build time when `CRAFT_FONTS_DIR` points to an **absolute path**:

```sh
git clone https://github.com/storytold/craft-fonts ../craft-fonts
CRAFT_FONTS_DIR="$PWD/../craft-fonts" cargo run --release -p photocraft
```

---

## 2. Run the Headless CLI (`photocraft-cli`)

`apps/photocraft-cli` drives `photocraft-engine` and `photocraft-io` without opening a window:

```sh
# Convert between any supported formats (.psd, .pcraft, .png, .jpg, .tiff, .xcf, .ora, .kra, etc.)
cargo run --release -p photocraft-cli -- convert in.psd out.pcraft

# Inspect document metadata, color mode, bit depth, and layer tree as JSON
cargo run --release -p photocraft-cli -- info in.psd

# Execute engine commands on an existing or newly created document
cargo run --release -p photocraft-cli -- run in.png \
  --cmd layer.new.layer --params '{"name":"Ink"}' \
  --cmd filter.blur.gaussianBlur --params '{"radius":3}' \
  --out out.psd

# Batch-process a folder using an Actions JSON file
cargo run --release -p photocraft-cli -- batch --actions actions.json --in photos/ --out done/ --format jpg

# Search the command registry
cargo run --release -p photocraft-cli -- commands --filter blur
```

---

## 3. Drive the App Programmatically (Control Channel & MCP)

### Authenticated TCP Control Channel (`127.0.0.1`)
Start the desktop app with `--control` and a bearer token file, then send newline-delimited JSON RPC requests:

```sh
# 1. Start the desktop app with control server enabled
cargo run --release -p photocraft -- --control 7878 \
  --control-token-file .private/control.token \
  --automation-read-root work --automation-write-root work

# 2. Authenticate, run a command, and capture a screenshot
TOKEN=$(tr -d '\r\n' < .private/control.token)
printf '%s\n' \
  "{\"id\":\"auth\",\"method\":\"auth\",\"params\":{\"token\":\"$TOKEN\"}}" \
  '{"id":1,"method":"engine.execute","params":{"command":"layer.newAdjustmentLayer.hueSaturation","params":{"hue":30}}}' \
  '{"id":2,"method":"ui.screenshot","params":{"path":"work/shot.png"}}' \
  | nc 127.0.0.1 7878
```

For offscreen visual snapshots without launching a window or stealing focus:
```sh
cargo run -p photocraft-ui-egui --example snapshot
```

### MCP Server (`rmcp` over stdio)

- **Headless MCP (in-process `Session`, no window):**
  ```sh
  cargo run --release -p photocraft-cli -- mcp \
    --automation-read-root "$PWD/work" \
    --automation-write-root "$PWD/work"
  ```
- **Bridged MCP (live desktop egui UI + MCP server):**
  1. Launch the desktop app with `--control`, `--control-token-file`, and the `--automation-*-root` flags (in bridge mode, filesystem roots must be configured on the desktop `photocraft` process rather than on the bridging CLI; if the token file does not exist, PhotoCraft creates it with a fresh 256-bit token and `0600` permissions):
     ```sh
     mkdir -p .private work
     cargo run --release -p photocraft -- \
       --control 7878 \
       --control-token-file "$PWD/.private/control.token" \
       --automation-read-root "$PWD/work" \
       --automation-write-root "$PWD/work"
     ```
  2. Start the MCP bridge pointing at the desktop app's loopback port and token file:
     ```sh
     cargo run --release -p photocraft-cli -- mcp \
       --bridge 127.0.0.1:7878 \
       --control-token-file "$PWD/.private/control.token"
     ```
  3. For MCP client configs (`mcpServers`), build `cargo build --release -p photocraft -p photocraft-cli` once and invoke the compiled `target/release/photocraft-cli` binary directly so stdio startup doesn't wait on Cargo:
     ```json
     {
       "mcpServers": {
         "photocraft": {
           "command": "/abs/path/to/photocraft/target/release/photocraft-cli",
           "args": [
             "mcp",
             "--bridge",
             "127.0.0.1:7878",
             "--control-token-file",
             "/abs/path/to/photocraft/.private/control.token"
           ]
         }
       }
     }
     ```
  - **Bridge vs. Headless tool availability:** `ui_*` tools (`ui_inspect`, `ui_set`, `ui_pointer`, `ui_menu_invoke`, `ui_screenshot`) and `control_call` work **only in bridge mode** (driving the live window). `doc_select` and `doc_close` work **only in headless mode**. Engine/document tools (`command_run`, `command_list`, `doc_new`, `doc_open`, `doc_save`, `doc_inspect`, `jobs_list`, etc.) work in both.

---

## 4. Run the WebAssembly Browser App (`photocraft-web`)

Prerequisites: `rustup target add wasm32-unknown-unknown` and `trunk`.

```sh
# Quick check that L0–L6 compile cleanly for wasm32
cargo xtask wasm

# Serve locally with Trunk
cd apps/photocraft-web && trunk serve --release

# Or build and run the self-contained NGINX image via Docker (serves on http://localhost:8080)
docker build --load -t photocraft-web:local .
docker run --rm -p 8080:8080 photocraft-web:local
```

To build the production chunked static bundle (keeps all files under the 24 MiB CDN limit):
```sh
packaging/web/package.sh
```

---

## 5. Troubleshooting, GPU Fallbacks & Environment Variables

For GPU startup recovery (`--safe-gpu`, `WGPU_BACKEND`), single-instance lock behavior (`instance.lock`), log locations (`photocraft.log`), live design token hot-reloading, and brush stroke latency tracing (`PHOTOCRAFT_STROKE_TRACE`), read [runtime-flags-and-debugging.md](./references/runtime-flags-and-debugging.md).

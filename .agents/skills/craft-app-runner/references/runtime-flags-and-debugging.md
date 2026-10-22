# Runtime Flags, Environment Variables & Debugging Reference

## 1. CLI Flags (`photocraft` Desktop Binary)

| Flag | Purpose |
|---|---|
| `<files>...` | Open files on startup (forwards to existing instance unless isolated) |
| `--new-instance` | Bypass single-instance forwarding and open a separate process |
| `--safe-gpu` | Start with CPU canvas and software window adapter for this launch |
| `--control <port>` | Start the loopback JSON control server on `127.0.0.1:<port>` |
| `--control-token-file <path>` | Read (or generate) the 64-hex bearer token file for `--control` |
| `--automation-read-root <dir>` | Directory from which automation/control commands are allowed to read |
| `--automation-write-root <dir>` | Directory to which automation/control commands are allowed to write |
| `--present-mode <mode>` | Override swapchain present mode: `fifo`, `autovsync`, or `novsync` (`immediate`/`mailbox`) |
| `--frame-latency <1..3>` | Maximum queued swapchain frames for latency testing |
| `--stroke-trace <stderr\|file.csv>` | Print p50/p95 brush stroke frame timings (and optionally append per-frame CSV rows) |
| `--in-window-menus` | Force in-window menu bar instead of native macOS menu bar |

---

## 2. Environment Variables

| Variable | Effect |
|---|---|
| `CRAFT_FONTS_DIR=/abs/path/to/craft-fonts` | **Build-time:** Embeds shared fonts into `photocraft_text::CRAFT_FONTS`. Must be an absolute path (`$PWD/../craft-fonts`) because `crates/text/build.rs` executes inside `crates/text`. |
| `RUST_LOG` | Controls log verbosity on stderr and `photocraft.log` (e.g., `RUST_LOG=debug`, `RUST_LOG=info,photocraft_gpu=trace`, `RUST_LOG=photocraft*=debug`). |
| `PHOTOCRAFT_CONFIG_DIR` | Override the per-user settings/recovery/log directory (also isolates the single-instance lock). |
| `PHOTOCRAFT_NEW_INSTANCE=1` | Equivalent to `--new-instance`. |
| `PHOTOCRAFT_CPU_CANVAS=1` | Disable the `wgpu` shader canvas and use the CPU canvas path. |
| `PHOTOCRAFT_CPU_COMPOSE=1` | Keep the `wgpu` display shader active, but composite layers on the CPU (`photocraft-compose`). |
| `PHOTOCRAFT_GPU_TILE=2048` | Force GPU canvas tiling at a lower texture threshold to test tile seams. |
| `PHOTOCRAFT_FX_NOCACHE=1` | Bypass the CPU layer-effect map cache (`compose::effect_maps`). |
| `PHOTOCRAFT_FX_TRACE=1` | Log CPU time spent computing GPU effect shapes and Euclidean distance fields. |
| `PHOTOCRAFT_STROKE_TRACE=1` (or `=file.csv`) | Trace input-to-pixel frame breakdown (`interval`, `moves/frame`, `stroke`, `canvas`, `ui`, `cpu`, `wait`). |
| `WGPU_BACKEND` | Override wgpu backend (`metal`, `vulkan`, `dx12`, `gl`). |
| `PHOTOCRAFT_GPU_ADAPTER=<name>` | Linux: case-insensitive substring match to pick a specific GPU (e.g., `nvidia`). |
| `PHOTOCRAFT_NATIVE_WAYLAND=1` | Linux: stay on native Wayland even when a pen tablet is detected (by default, pen presence switches to XWayland for tablet pressure support). |
| `PHOTOCRAFT_LOCALE=<tag>` | Override UI language for one launch (e.g., `ja`, `de`, `fr`). |
| `PHOTOCRAFT_NO_PRINT=1` | Refuse real print spooling in `file.print` / `file.printOneCopy` (always set alongside `LPDEST=photocraft-no-such-printer` when fuzzing). |
| `PHOTOCRAFT_THEME_FILE=tokens.json` | **Debug builds only:** live-reload UI design token overrides (`theme::Tokens`) on save without recompiling. |

---

## 3. Single-Instance Lock & Forwarding

- By default, the first PhotoCraft instance acquires `instance.lock` and writes `instance.json` in the config directory (or `$XDG_RUNTIME_DIR/app/<app-id>/instance.sock` inside Flatpak; LaunchServices on macOS).
- Subsequent launches without isolation flags pass their file arguments to the running window so they open as new tabs, then exit.
- **When running via `cargo run` while an installed PhotoCraft is open:** pass `--new-instance` or set `PHOTOCRAFT_CONFIG_DIR=/tmp/pcraft-dev` so `cargo run` does not simply hand off to the installed binary and exit.
- Launches with `--control`, `--safe-gpu`, `--present-mode`, `--frame-latency`, `--stroke-trace`, `--in-window-menus`, or `--automation-*-root` automatically run as separate instances.

---

## 4. GPU Startup Marker & Crash Recovery

- **Startup marker (`gpu-starting.json`):** Written and locked before creating the `wgpu` device, then cleared after the first frames render. If a launch crashes inside the graphics driver, the next launch detects the unlocked marker and steps down to the next safer backend:
  - **macOS:** Metal $\to$ CPU
  - **Windows:** Vulkan $\to$ DX12 $\to$ CPU
  - **Linux:** Vulkan $\to$ GL $\to$ CPU
- **Document crash recovery (`RecoveryStore`):** Unsaved documents autosave in the background using content-addressed `.pcraft` tiles (`crates/format`). If the app crashes or loses the GPU device (`gpu_status::check_window`), autosaves are flushed and automatically reopened on the next launch.

---

## 5. Log Files

Desktop logs are written both to `stderr` and to `logs/photocraft.log` inside the settings directory:
- **macOS:** `~/Library/Application Support/Photocraft/logs/photocraft.log`
- **Linux:** `~/.config/photocraft/logs/photocraft.log`
- **Windows:** `%APPDATA%\Photocraft\logs\photocraft.log`

Each launch rotates the previous log to `photocraft.1.log` and `photocraft.2.log` so crash logs survive a restart.

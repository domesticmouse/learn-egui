# Runtime Flags, Environment Variables & Debugging Reference

## 1. Recommended CLI Flags (Desktop `eframe` Binary)

| Flag | Purpose |
|---|---|
| `<files>...` | Open files on startup (forwards to existing instance unless isolated) |
| `--new-instance` | Bypass single-instance forwarding and open a separate process |
| `--safe-gpu` | Start with software/fallback adapter for this launch |
| `--control <port>` | Start the loopback JSON control server on `127.0.0.1:<port>` |
| `--control-token-file <path>` | Read (or generate) the 64-hex bearer token file for `--control` |
| `--automation-read-root <dir>` | Directory from which automation/control commands are allowed to read |
| `--automation-write-root <dir>` | Directory to which automation/control commands are allowed to write |
| `--present-mode <mode>` | Override swapchain present mode: `fifo`, `autovsync`, or `novsync` (`immediate`/`mailbox`) |
| `--frame-latency <1..3>` | Maximum queued swapchain frames for latency testing |
| `--in-window-menus` | Force in-window `egui` menu bar instead of native OS menu bar |

---

## 2. Environment Variables

| Variable | Effect |
|---|---|
| `RUST_LOG` | Controls log verbosity on stderr and log files (e.g., `RUST_LOG=debug`, `RUST_LOG=info,eframe=debug`). |
| `WGPU_BACKEND` | Override `wgpu` backend (`metal`, `vulkan`, `dx12`, `gl`). |
| `APP_CONFIG_DIR` | Override the per-user settings/recovery/log directory (also isolates the single-instance lock). |
| `APP_NEW_INSTANCE=1` | Equivalent to `--new-instance`. |
| `APP_LOCALE=<tag>` | Override UI language for one launch (e.g., `ja`, `de`, `fr`). |
| `APP_THEME_FILE=tokens.json` | **Debug builds only:** live-reload `egui` design token overrides (`theme::Tokens`) on save without recompiling. |

---

## 3. Single-Instance Lock & Forwarding Pattern

- By default, the first desktop instance can acquire `instance.lock` and write `instance.json` in the config directory (or `$XDG_RUNTIME_DIR/app/<app-id>/instance.sock` on Linux).
- Subsequent launches without isolation flags pass their file arguments to the running window so they open as new tabs, then exit.
- **When running via `cargo run` while another instance is open:** pass `--new-instance` or override the config directory so `cargo run` does not hand off to the existing binary and exit.
- Launches with `--control`, `--safe-gpu`, `--present-mode`, `--frame-latency`, `--in-window-menus`, or `--automation-*-root` should automatically run as separate instances.

---

## 4. GPU Startup Marker & Crash Recovery Pattern

- **Startup marker (`gpu-starting.json`):** Written and locked before creating the `wgpu` device, then cleared after the first frames render. If a launch crashes inside the graphics driver, the next launch detects the unlocked marker and steps down to the next safer backend:
  - **macOS:** Metal $\to$ CPU/Fallback
  - **Windows:** Vulkan $\to$ DX12 $\to$ Fallback
  - **Linux:** Vulkan $\to$ GL $\to$ Fallback
- **Document crash recovery (`RecoveryStore`):** Unsaved documents can autosave in the background using content-addressed state (`crates/format`). If the app crashes or loses the GPU device, autosaves are flushed and automatically reopened on the next launch.

---

## 5. Log Files & Rotation

Desktop logs can be written both to `stderr` and to `logs/<app>.log` inside the OS-appropriate application support/config directory:
- **macOS:** `~/Library/Application Support/<App>/logs/<app>.log`
- **Linux:** `~/.config/<app>/logs/<app>.log`
- **Windows:** `%APPDATA%\<App>\logs\<app>.log`

Rotate the previous log on startup (`.1.log`, `.2.log`) so crash logs survive a restart.

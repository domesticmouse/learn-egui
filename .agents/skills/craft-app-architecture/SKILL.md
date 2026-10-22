---
name: craft-app-architecture
description: >-
  Use this skill when exploring, understanding, scaffolding, structuring, or
  auditing a multi-crate Rust application in the style of PhotoCraft (strict
  L0–L7 dependency layering, xtask layer enforcement, panic-free/unsafe-free
  lint gates, and Cargo profile tuning for native + WebAssembly targets).
---

# Craft-Style Application Architecture (`L0`–`L7`)

This skill guides you through setting up and maintaining a pure-Rust, engine-first desktop, CLI, and WebAssembly application following PhotoCraft's architectural standards.

## Architectural Principles

1. **Strict Acyclic Layering (`L0`–`L7`):** Every workspace crate is assigned a layer in `xtask/src/layers.rs`. A crate at layer $N$ may only depend on crates in layers $< N$ (plus explicit intra-layer chains).
2. **Standalone Format & Hardware Crates:** Parsers/encoders (`psd`, `codecs`, `raw`, `heif`, `affinity`, `adobe-assets`) and isolated OS helpers (`tablet`, `ipp`) are classified as `Standalone` at `L0` and must have **zero workspace dependencies** so they remain independently publishable (with the single explicit `STANDALONE_EXCEPTIONS` entry `("codecs", "heif")` behind `codecs`'s optional `heif` feature).
3. **UI Containment (`L6+` only):** No crate below `L6` may depend on UI or windowing crates (`egui`, `eframe`, `winit`, `rfd`, `craft-ui`, `bevy*`). `L0`–`L5` must compile headlessly and for `wasm32-unknown-unknown`.
4. **Never Crash (No Panic, No Unsafe):** Workspace-wide `unsafe_code = "forbid"` and `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]` on every library and binary crate. The sole isolated exception is `photocraft-tablet` (`crates/tablet`, `unsafe_code = "deny"` crate-wide), where `src/macos.rs` (AppKit `NSEvent` monitor) and `src/windows.rs` (`GetPointerPenInfo` barrel-button hook) allow `unsafe` with `SAFETY:` comments and platform tests; its X11 path and mapping code are safe.

---

## Step-by-Step Workflow

### 1. Organize the Cargo Workspace into 8 Layers

Structure the repository into `crates/*`, `apps/*`, and `xtask`:

- **`L0` Foundation:** `geom` (points, rects, affine, tile coords) $\to$ `cms` (ICC color management) $\to$ `color` (pixel formats, blend math) $\to$ `raster` (sparse copy-on-write tiled surfaces).
- **`L0` Standalone:** Format parsers/writers (`psd`, `codecs`, `raw`, `heif`, `affinity`, `adobe-assets`) and OS helpers (`tablet`, `ipp`) with no workspace dependencies.
- **`L1` Document Model:** `doc` — pure serializable data structures (layer tree, masks, effects, metadata). No rendering or UI logic.
- **`L2` Operations & Algorithms:** `ops` (undo/redo history), `algo` (filters/adjustments), `paint` (brush engine), `text` (shaping/rasterization), `vector` (paths/coverage).
- **`L3` Rendering & Native Format:** `compose` (CPU reference compositor), `gpu` (`wgpu` compositor), `format` (content-addressed native bundle).
- **`L4` I/O & Sandboxed Plugins:** `io` (maps standalone formats and XCF $\leftrightarrow$ `doc::Document`), `plugins` (`wasmi` sandboxed Wasm filters).
- **`L5` Engine Façade:** `engine` (`Session`, `CommandSpec` registry, background `Jobs`, `Preferences`).
- **`L6` Shells & Automation:** `ui-egui` (thin `egui` frontend + JSON control protocol), `automation` (`rmcp` MCP server + headless JSON-RPC).
- **`L7` Binaries (`Exempt`):** `apps/<app>` (desktop), `apps/<app>-cli` (headless CLI + MCP/RPC host), `apps/<app>-web` (browser Wasm), `apps/<app>-web-boot` (loads compressed Wasm chunks under host file-size caps), `xtask`.

### 2. Configure Workspace Lints and Cargo Profiles

Apply the workspace lint gates and build profiles in the root `Cargo.toml`.
See [layering-and-profiles.md](./references/layering-and-profiles.md) for the exact `Cargo.toml` configuration:
- Optimize dependencies in dev builds (`[profile.dev.package."*"] opt-level = 2`) so pixel/audio loops run at interactive speeds during development.
- Use `debug = "line-tables-only"` in `[profile.dev]` to halve link times and symbol sizes.
- Set `[profile.test.package.<ui-crate>] opt-level = 0` to speed up UI test compilation.
- Configure `[profile.wasm-release]` with `lto = "fat"` and per-crate `opt-level` overrides (`"z"` for data/parsers/UI, `3` for hot pixel crates).

### 3. Enforce Layering with `cargo xtask layers`

Whenever adding a crate or dependency:
1. Register the crate in `xtask/src/layers.rs` (`TABLE`).
2. Add the crate header lints (`#![forbid(unsafe_code)]` and `#![deny(clippy::unwrap_used, ...)]`) to its `src/lib.rs`.
3. Write or update `Cargo.toml` atomically if working alongside parallel agents.

### 4. Validation Steps

Run these commands to verify architectural compliance:

```sh
cargo xtask layers
cargo xtask wasm
cargo clippy --workspace --all-targets -- -D warnings
```

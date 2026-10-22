---
name: egui-app-architecture
description: >-
  Use this skill when exploring, understanding, scaffolding, structuring, or
  auditing a multi-crate Rust and egui application (strict L0–L7 dependency
  layering, xtask layer enforcement, panic-free/unsafe-free lint gates, and
  Cargo profile tuning for native + WebAssembly targets).
---

# Engine-First egui Application Architecture (`L0`–`L7`)

This skill guides you through setting up and maintaining a pure-Rust, engine-first desktop (`eframe`/`egui`), CLI, and WebAssembly application with strict architectural boundaries.

## Architectural Principles

1. **Strict Acyclic Layering (`L0`–`L7`):** Every workspace crate is assigned a layer in `xtask/src/layers.rs`. A crate at layer $N$ may only depend on crates in layers $< N$ (plus explicit intra-layer chains).
2. **Standalone Format & OS Helper Crates:** Reusable parsers/encoders (`codecs`, `formats`) and isolated OS helpers are classified as `Standalone` at `L0` and must have **zero workspace dependencies** so they remain independently publishable and testable.
3. **UI Containment (`L6+` only):** No crate below `L6` may depend on UI or windowing crates (`egui`, `eframe`, `winit`, `rfd`). `L0`–`L5` must compile headlessly and for `wasm32-unknown-unknown`.
4. **Never Crash (No Panic, No Unsafe):** Workspace-wide `unsafe_code = "forbid"` and `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]` on every library and binary crate. If an isolated OS integration crate requires FFI, isolate it in its own `L0` crate with `unsafe_code = "deny"` and explicit `SAFETY:` comments and platform tests.

---

## Step-by-Step Workflow

### 1. Organize the Cargo Workspace into Layers

Structure the repository into `crates/*`, `apps/*`, and `xtask` (adapt layers as needed for your domain):

- **`L0` Foundation:** Core primitives, geometry, math, and copy-on-write data containers.
- **`L0` Standalone:** Format parsers/writers and OS helpers with no workspace dependencies.
- **`L1` Document / Domain Model:** `doc` — pure serializable data structures (document tree, entities, metadata). No rendering or UI logic.
- **`L2` Operations & Algorithms:** `ops` (undo/redo history), `algo` (domain transformations, queries, and processing).
- **`L3` Rendering & Native Persistence:** CPU/GPU rendering backends (`wgpu`) and `format` (content-addressed or structured native bundle).
- **`L4` I/O & Plugins:** `io` (maps standalone formats $\leftrightarrow$ `doc::Document`), optional sandboxed plugins.
- **`L5` Engine Façade:** `engine` (`Session`, `CommandSpec` registry, background `Jobs`, `Preferences`).
- **`L6` Shells & Automation:** `ui-egui` (thin `egui` frontend + optional JSON control protocol), `automation` (MCP server + headless JSON-RPC).
- **`L7` Binaries (`Exempt`):** `apps/<app>` (desktop `eframe` binary), `apps/<app>-cli` (headless CLI + MCP/RPC host), `apps/<app>-web` (browser Wasm), `xtask`.

### 2. Configure Workspace Lints and Cargo Profiles

Apply the workspace lint gates and build profiles in the root `Cargo.toml`.
See [layering-and-profiles.md](./references/layering-and-profiles.md) for the exact `Cargo.toml` configuration:
- Optimize dependencies in dev builds (`[profile.dev.package."*"] opt-level = 2`) so rendering and compute loops run at interactive speeds during development.
- Use `debug = "line-tables-only"` in `[profile.dev]` to halve link times and symbol sizes.
- Set `[profile.test.package.<ui-crate>] opt-level = 0` to speed up UI test compilation.
- Configure `[profile.wasm-release]` with `lto = "fat"` and per-crate `opt-level` overrides (`"z"` for data/parsers/UI, `3` for hot compute crates).

### 3. Enforce Layering with `cargo xtask layers`

Whenever adding a crate or dependency:
1. Register the crate in `xtask/src/layers.rs` (`TABLE`).
2. Add the crate header lints (`#![forbid(unsafe_code)]` and `#![deny(clippy::unwrap_used, ...)]`) to its `src/lib.rs`.
3. Write or update `Cargo.toml` atomically if working alongside parallel agents.

### 4. Validation Steps

Run these commands to verify architectural compliance (see [egui-testing-infrastructure](../egui-testing-infrastructure/SKILL.md) for the full 7-tier verification, corpus, performance, and scorecard workflow):

```sh
cargo xtask layers
cargo xtask wasm
cargo clippy --workspace --all-targets -- -D warnings
```

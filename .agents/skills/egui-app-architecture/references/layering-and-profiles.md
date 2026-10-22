# Layering Enforcer & Workspace Profiles Reference

## 1. Root `Cargo.toml` Profiles and Lints

```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "xtask"]

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
dbg_macro = "warn"
todo = "warn"

# Optimize dependencies in dev builds so egui rendering and data processing stay responsive.
[profile.dev.package."*"]
opt-level = 2

[profile.dev]
opt-level = 1
debug = "line-tables-only"

# Keep the large UI crate unoptimized in test builds to cut compile time.
[profile.test.package.app-ui-egui]
opt-level = 0

[profile.release]
lto = "thin"
codegen-units = 1

# Browser build: minimize Wasm binary size while keeping hot compute loops fast.
[profile.wasm-release]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
strip = "debuginfo"

[profile.wasm-release.package."*"]
opt-level = "s"

# Large crates off the per-frame hot path (domain model, format parsers, UI toolkit)
# use "z" to reduce WebAssembly bundle size:
[profile.wasm-release.package.app-doc]
opt-level = "z"
[profile.wasm-release.package.app-format]
opt-level = "z"
[profile.wasm-release.package.app-ui-egui]
opt-level = "z"

# Hot compute crates can stay at opt-level 3 even in wasm-release:
[profile.wasm-release.package.app-core]
opt-level = 3
```

## 2. Crate Header Lints (`src/lib.rs`)

Every non-test crate starts with:

```rust
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
```

And `clippy.toml` at the workspace root allows those macros inside `#[cfg(test)]`:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
```

## 3. Layering Rules Model (`xtask/src/layers.rs`)

An `xtask` layer checker can classify every workspace crate into:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Regular layered crate (may only depend on layers < l, or earlier in INTRA_LAYER_ORDER).
    Layer(u8),
    /// Layer 0, and additionally may depend on no workspace crate at all.
    Standalone,
    /// Test tooling: may depend on anything up to L5; other crates may use it only as a dev-dependency.
    Testkit,
    /// Binaries and build tooling: exempt from the rules.
    Exempt,
}
```

Key checks performed by `check(crates: &[Crate]) -> Vec<Violation>`:
- **`Violation::Unregistered`**: Every workspace member must appear in `TABLE`.
- **`Violation::Upward`**: `to >= from` unless `to == from && intra_layer_allowed(from, to)`.
- **`Violation::StandaloneHasWorkspaceDep`**: A `Class::Standalone` crate cannot depend on any workspace crate unless listed in `STANDALONE_EXCEPTIONS`.
- **`Violation::TestkitAsNormalDep`**: `testkit` is only allowed when `DepKind::Dev`.
- **`Violation::UiBelowL6`**: Any crate with `layer < 6` depending on `["egui", "eframe", "winit", "egui_kittest", "rfd"]` is rejected so the core engine remains UI-agnostic.

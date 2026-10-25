# RESOURCES.md Format

`RESOURCES.md` is the curated set of trusted sources for this topic. Knowledge for explainers should be drawn from here, not from parametric guesses. Wisdom comes from the communities listed here.

## Structure

```md
# {Topic} Resources

## Knowledge

- [Official API Docs: `egui` on docs.rs](https://docs.rs/egui/latest/egui/)
  Primary reference by Emil Ernerfeldt covering the immediate-mode architecture, `Context`, `Ui`, `Painter`, `Scene`, and built-in widgets. Use for: authoritative explanations of widget signatures, coordinate spaces, and frame lifecycle.
- [Official User Guide: `cargo-mutants` (`mutants.rs`)](https://mutants.rs/)
  Official documentation by Martin Pool for Rust mutation testing. Use for: monitoring true assertion coverage in every lesson.

## Wisdom (Communities)

- [`emilk/egui` GitHub Discussions](https://github.com/emilk/egui/discussions)
  Official Q&A and architecture forum monitored by maintainers and experienced practitioners. Use for: design review of custom canvas widgets, performance tuning, and idiomatic patterns.
- [The Rust Programming Language Users Forum (`URLO`)](https://users.rust-lang.org/)
  High-signal, well-moderated forum for Rust learners and experts alike. Use for: borrow-checker questions when structuring application state across panels and canvases.
```

## Rules

- **High-trust only.** Prefer primary sources, recognised experts, peer-reviewed work, and communities with strong moderation. If a resource is marketing dressed as education, leave it out.
- **Annotate every entry.** A bare link is useless in three months. Add one line: what it covers and when to reach for it.
- **Group by Knowledge / Wisdom.** Mirrors the philosophy in [SKILL.md](../SKILL.md). It is fine for a resource to appear in only one group.
- **Surface gaps explicitly.** If no good resource exists for an area the mission needs, write a `## Gaps` section listing what is missing. This drives future search.
- **Prune ruthlessly.** A resource that turned out to be wrong, shallow, or off-mission should be removed, not buried. Better five sharp sources than thirty mediocre ones.
- **Record community preferences.** If the user has opted out of joining communities, note it here so future sessions don't keep proposing them.

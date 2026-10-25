# GLOSSARY.md Format

`reference/GLOSSARY.md` (compiled to `reference/glossary.html` by `build.py`) is the canonical language for this teaching workspace. All explainers, exercises, and learning records should adhere to its terminology. Building it is itself part of learning: compressing a concept into a tight definition is evidence the user understands it.

## Structure

```md
---
title: "Glossary of {Topic} Terms"
page_title: "Glossary — {Topic}"
nav_id: "glossary"
nav_label: "Glossary"
badge: "Mission: {Mission Name}"
subtitle: "Canonical definitions used across all lessons and reference sheets. Designed for quick screen lookup and clean printing."
card:
  title: "**Glossary of {Topic} Terms**"
  summary: "{One-sentence summary of what the glossary covers for the index reference table.}"
---

## 1. {Category Heading}

**Immediate-Mode GUI (IMGUI)**:
A UI paradigm where widgets are not persistent objects in memory; instead, application state lives in a plain Rust `struct` and every frame calls functions like `ui.checkbox(...)` that simultaneously query input and emit draw commands.
_Avoid_: Stateless UI, redraw-everything mode

**Mutable Borrow (`&mut T`)**:
An exclusive, read-write reference to a value of type `T` that allows a function to both inspect the current value and modify it in place without taking ownership.
_Avoid_: Mutable pointer, pass-by-reference copy
```

## Rules

- **Add a term only when the user understands it.** The glossary is a record of compressed knowledge, not a dictionary the user reads to learn. If the user has just been introduced to a concept, wait until they can use it correctly before promoting it here.
- **Be opinionated.** When several words exist for the same concept, pick the best one and list the rest as aliases to avoid. This is how language compresses.
- **Keep definitions tight.** One or two sentences. Define what the term IS, not what it does or how to do it.
- **Use the glossary's own terms inside definitions.** Once a term is in the glossary, prefer it everywhere, including inside other definitions. This is what makes complex terms easier to grasp later.
- **Group under subheadings** when natural clusters emerge (e.g. `## 1. Core GUI Paradigms`, `## 2. Essential Rust Mechanics`). A flat list or structured table is fine when terms cohere.
- **Flag ambiguities explicitly.** If a term is used loosely in the wider field, note the resolution.
- **Revise as understanding deepens.** A definition written in lesson one may be refined later. Update in place; do not leave stale entries.

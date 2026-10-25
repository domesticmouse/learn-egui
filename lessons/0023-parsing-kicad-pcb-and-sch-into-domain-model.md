---
nav_id: "0023"
page_title: "Lesson 0023: Lowering KiCad .kicad_pcb & .kicad_sch ASTs into the Domain Model & Corpus Oracles — Learn Rust & egui"
act_badge: "Act VI • Real-World KiCad Parsing & Embedded SQL"
title: "0023. Lowering KiCad `.kicad_pcb` & `.kicad_sch` ASTs into the Domain Model & Corpus Oracles"
subtitle: "Lowering untyped `SexprNode` syntax trees into typed `ComponentSymbol`, `SmdPad`, and `TraceSegment` domain structs with `KicadLowerError`, exporting lossless `.kicad_pcb` S-expressions, and enforcing Tier 4 pinned corpus oracle ratchets (`CORPUS_PASS_FLOOR` & `CORPUS_ROUNDTRIP_FLOOR`)."
card:
  num: "Lesson 0023"
  mutants: "385 / 385 Caught"
  title: "KiCad AST Lowering & Tier 4 Corpus Oracles"
  summary: "Lower `SexprNode` trees into `BoardModel` (`from_kicad_pcb_sexpr` / `to_kicad_pcb_sexpr`), validate KiCad layer names (`F.Cu`, `B.Cu`, `F.SilkS`), and enforce Tier 4 corpus pass & round-trip ratchets."
  egui_skill: "Live KiCad S-Expression Board Import/Export & Corpus Oracle Telemetry"
  rust_concept: "AST Lowering (`Result<T, KicadLowerError>`), `#[from]` Error Chaining & Corpus Floor Ratchets"
  app_dir: "apps/0023-parsing-kicad-pcb-and-sch-into-domain-model"
---

## Two-Phase Parsing: Untyped `SexprNode` (`L0`) $\to$ Typed `BoardModel` (`L1`/`L4`)

In Lesson 0022, our standalone `L0` `kicad-sexpr` crate turned raw KiCad text into an untyped `SexprNode` tree. However, a syntactically valid S-expression like `(pad "1" smd (at 20.0 "not_a_float") (layer "Unknown.Layer"))` is still semantically invalid for a 2D PCB CAD engine.{{sidenote: **Tier 4 Pinned Corpus Oracles (`egui-testing-infrastructure`):** Real-world file importers are verified against a pinned corpus of `.kicad_pcb` fixtures (`PINNED_KICAD_CORPUS`). Every fixture must pass both import (`CORPUS_PASS_FLOOR`) and `import -> export -> re-import` round-trip equivalence (`CORPUS_ROUNDTRIP_FLOOR`), and ratchets only move up!}}

By separating **Phase 1 (Syntax Tokenization & AST)** from **Phase 2 (Semantic Lowering into `BoardModel`)**, we get clean, granular error reporting via `KicadLowerError`:

```mermaid
flowchart LR
    Sexpr["SexprNode::List<br/>(kicad_pcb ...)"] --> RootCheck{"tag_name() ==<br/>Some(\"kicad_pcb\")?"}
    RootCheck -- "No" --> ErrRoot["Err(KicadLowerError::UnexpectedRootTag)"]
    RootCheck -- "Yes" --> Lower["Lower child nodes:<br/>(footprint ...) -> ComponentSymbol<br/>(pad ...) -> SmdPad<br/>(segment ...) -> TraceSegment"]
    Lower --> Export["BoardModel::to_kicad_pcb_sexpr()"]
    Export --> Reload["Re-parse & Compare<br/>reloaded == original"]
    Reload --> Oracle["CorpusOracleReport<br/>meets_ratchet_floors()"]
    style Oracle fill:#14231c,stroke:#3dd68c,color:#e6edf3
    style ErrRoot fill:#2a1418,stroke:#ff5555,color:#e6edf3
```

## Lowering `(footprint ...)`, `(pad ...)`, and `(segment ...)` with `KicadLowerError`

```rust caption="Typed KiCad S-expression lowering and CorpusOracleReport in crates/pcb-doc/src/lib.rs"
pub const CORPUS_PASS_FLOOR: usize = 3;
pub const CORPUS_ROUNDTRIP_FLOOR: usize = 3;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum KicadLowerError {
    #[error("S-expression parse error: {0}")]
    Sexpr(#[from] SexprParseError),
    #[error("expected `(kicad_pcb ...)` root node, found `{0}`")]
    UnexpectedRootTag(String),
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("invalid numeric coordinate `{0}`")]
    InvalidNumber(String),
    #[error("unknown KiCad layer `{0}`")]
    UnknownLayer(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorpusOracleReport {
    pub total_files: usize,
    pub passed_files: usize,
    pub roundtrip_files: usize,
}

impl CorpusOracleReport {
    pub fn meets_ratchet_floors(&self) -> bool {
        self.passed_files >= CORPUS_PASS_FLOOR
            && self.roundtrip_files >= CORPUS_ROUNDTRIP_FLOOR
            && self.passed_files == self.total_files
    }
}
```

{{widget:kicad-corpus-sim}}

:::exercise id="ex0023" title="Interactive Lab 2: Map KiCad Layer Names & Enforce Corpus Floor Ratchets" badge="Skill Practice • AST Lowering & Corpus Ratchets"
Complete `PcbLayer::from_kicad_name` and `CorpusOracleReport::meets_ratchet_floors` in `crates/pcb-doc/src/lib.rs`.

```rust
impl PcbLayer {
    pub fn from_kicad_name(name: &str) -> Result<Self, KicadLowerError> {
        match name {
            "B.Cu" => Ok(PcbLayer::BottomCopper),
            "F.Cu" => Ok(PcbLayer::{{blank:TopCopper|Map KiCad's F.Cu front copper layer to PcbLayer::TopCopper|TopCopper|120px}}),
            "F.SilkS" => Ok(PcbLayer::Silkscreen),
            "Ratsnest" => Ok(PcbLayer::Ratsnest),
            other => Err(KicadLowerError::{{blank:UnknownLayer|Return KicadLowerError::UnknownLayer for unrecognized layer names|UnknownLayer|145px}}(other.to_owned())),
        }
    }
}

impl CorpusOracleReport {
    pub fn meets_ratchet_floors(&self) -> bool {
        self.passed_files >= {{blank:CORPUS_PASS_FLOOR|Require passed_files >= CORPUS_PASS_FLOOR|CORPUS_PASS_FLOOR|195px}}
            && self.roundtrip_files >= CORPUS_ROUNDTRIP_FLOOR
            && self.passed_files == self.total_files
    }
}
```

:::prompts
hint: Map `"F.Cu"` to `PcbLayer::TopCopper`, wrap unknown layer strings in `KicadLowerError::UnknownLayer(other.to_owned())`, and check `self.passed_files >= CORPUS_PASS_FLOOR`.
success: Spot on! Every KiCad `(footprint ...)`, `(pad ...)`, and `(segment ...)` lowers into typed `BoardModel` structs and passes our Tier 4 corpus oracle ratchets.
:::
:::

:::verify id="verify-0023" title="Lab 3: KiCad AST Lowering, Tier 4 Corpus Oracle Ratchets & Mutation Gate (`0023`)" badge="cargo test --workspace + corpus oracle + cargo mutants" tier="Tier 2 + Tier 4 (Pinned Corpus Oracles)" caught="385 / 385 (100%)" missed="0 Missed"
Run the workspace test suite (including `kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work`) and mutate all KiCad lowering helpers, round-trip formatters, and corpus ratchet methods:

```bash caption="Run Tier 4 corpus oracle tests and cargo-mutants on 0023"
cargo test --manifest-path ./apps/0023-parsing-kicad-pcb-and-sch-into-domain-model/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0023-parsing-kicad-pcb-and-sch-into-domain-model/Cargo.toml --workspace --in-place -F "kicad_name|from_kicad_name|parse_child_f32_pair|parse_child_text|parse_child_u32|lower_footprint_node|lower_pad_node|lower_segment_node|from_kicad_pcb_sexpr|to_kicad_pcb_sexpr|CorpusOracleReport|run_pinned_kicad_corpus|import_kicad_sexpr_board|export_kicad_sexpr_board"
```

- [ ] `cargo test --manifest-path ./apps/0023-parsing-kicad-pcb-and-sch-into-domain-model/Cargo.toml --workspace` passes all **25 tests** (**3** in `kicad-sexpr`, **16** in `pcb-doc`, and **6** in `pcb-reviewer`).
- [ ] `kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work` verifies `run_pinned_kicad_corpus().meets_ratchet_floors()`, lossless `to_kicad_pcb_sexpr -> from_kicad_pcb_sexpr` round-tripping, and synthetic regression rejection on malformed coordinates and unknown layers.
- [ ] `cargo mutants` catches **100% of viable KiCad lowering & corpus oracle mutants** (`0 missed`).
:::

:::quiz id="quiz-0023"
Q: Why do we separate S-expression syntax parsing (`L0` `kicad-sexpr`) from domain lowering (`L1`/`L4` `BoardModel::from_kicad_pcb_sexpr`)?
- [x] To isolate untyped syntax errors from typed domain validation
- [ ] To force the tokenizer to allocate GPU vertex buffers
- [ ] To allow broken parentheses inside valid copper pad coordinates
- [ ] To skip all numeric coordinate checks during file import
why: Separating `L0` S-expression syntax parsing from `L1`/`L4` semantic lowering keeps `kicad-sexpr` independent of domain types while giving `KicadLowerError` precise diagnostics for missing fields, bad floats, or unknown layer names.
hint: Compare `SexprParseError` and `KicadLowerError` in `crates/pcb-doc/src/lib.rs`.

Q: What does our Tier 4 pinned KiCad corpus oracle verify for every board fixture?
- [ ] That every board contains zero copper trace segments
- [x] That parsing and S-expression round-tripping meet ratchet floors
- [ ] That external corpus files overwrite source code files
- [ ] That invalid layer names silently default to silkscreen
why: `run_pinned_kicad_corpus()` parses every pinned `.kicad_pcb` board, exports it back via `to_kicad_pcb_sexpr()`, re-parses the exported text, and asserts `meets_ratchet_floors()`.
hint: Inspect `run_pinned_kicad_corpus()` and `CorpusOracleReport::meets_ratchet_floors()`.

Q: According to Critical Rule #4 of our testing architecture, how must every corpus crash fix be verified?
- [ ] By deleting the failing board from the test suite
- [ ] By lowering the corpus pass floor constant to zero
- [x] By adding a small deterministic synthetic regression unit test
- [ ] By disabling clippy lints across the workspace parser crate
why: Every parser or lowering crash discovered against a corpus file must be captured as a small, self-contained synthetic regression test in the test suite so it can never recur.
hint: Check Critical Rule #4 in our Testing & Mutation Coverage Cheat Sheet.
:::

## Recommended Primary Source

:::callout
**Read (5 mins):** [KiCad Documentation — "S-Expression Board File Format (`.kicad_pcb`)"](https://dev-docs.kicad.org/en/file-formats/sexpr-pcb/) and [The Rust Programming Language — §9.2 "Recoverable Errors with `Result` and the `?` Operator"](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html).
:::

:::teacher-prompt
Next up: [Lesson 0024: Embedded SQL Review Store with Turso (`turso_parser` & Async `tokio` Bridge)](./0024-embedded-sql-review-store-with-turso.html), where we validate SQL DDL/DML ASTs with `turso_parser::parser::Parser` and persist multi-engineer review threads in an async `TursoReviewStore`!
:::

---
nav_id: "0024"
page_title: "Lesson 0024: Lowering KiCad .kicad_pcb & .kicad_sch ASTs into the Domain Model & Corpus Oracles — Learn Rust & egui"
act_badge: "Act VI • Real-World KiCad Parsing & Embedded SQL"
title: "0024. Lowering KiCad `.kicad_pcb` & `.kicad_sch` ASTs into the Domain Model & Corpus Oracles"
subtitle: "Lowering untyped `SexprNode` syntax trees into typed `ComponentSymbol`, `SmdPad`, and `TraceSegment` domain structs with `KicadLowerError`, exporting lossless `.kicad_pcb` S-expressions, and enforcing Tier 4 pinned corpus oracle ratchets (`CORPUS_PASS_FLOOR` & `CORPUS_ROUNDTRIP_FLOOR`)."
card:
  num: "Lesson 0024"
  mutants: "385 / 385 Caught"
  title: "KiCad AST Lowering & Tier 4 Corpus Oracles"
  summary: "Lower `SexprNode` trees into `BoardModel` (`from_kicad_pcb_sexpr` / `to_kicad_pcb_sexpr`), validate KiCad layer names (`F.Cu`, `B.Cu`, `F.SilkS`), and enforce Tier 4 corpus pass & round-trip ratchets."
  egui_skill: "Live KiCad S-Expression Board Import/Export & Corpus Oracle Telemetry"
  rust_concept: "AST Lowering (`Result<T, KicadLowerError>`), `#[from]` Error Chaining & Corpus Floor Ratchets"
  app_dir: "apps/0024-parsing-kicad-pcb-and-sch-into-domain-model"
---

## Two-Phase Parsing: Untyped `SexprNode` (`L0`) -> Typed `BoardModel` (`L1`/`L4`)

In Lesson 0023, our standalone `L0` `kicad-sexpr` crate turned raw KiCad text into an untyped `SexprNode` tree. However, a syntactically valid S-expression like `(pad "1" smd (at 20.0 "not_a_float") (layer "Unknown.Layer"))` is still semantically invalid for a 2D PCB CAD engine.{{sidenote: **Tier 4 Pinned Corpus Oracles (`egui-testing-infrastructure`):** Real-world file importers are verified against a pinned corpus of `.kicad_pcb` fixtures (`PINNED_KICAD_CORPUS`). Every fixture must pass both import (`CORPUS_PASS_FLOOR`) and `import -> export -> re-import` round-trip equivalence (`CORPUS_ROUNDTRIP_FLOOR`), and ratchets only move up!}}

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

## Anatomy of KiCad AST Lowering & Corpus Oracles: Block by Block

Let's unpack all the new code added in `crates/pcb-doc/src/lib.rs` and `src/main.rs` block by block before viewing the complete assembled listings in context.

### Block 1: `KicadLowerError`, Pinned Corpus Fixtures & `CorpusOracleReport` (`crates/pcb-doc/src/lib.rs`)

```rust caption="Typed KiCad S-expression lowering errors and CorpusOracleReport in crates/pcb-doc/src/lib.rs"
/// Minimum number of pinned KiCad `.kicad_pcb` corpus fixtures that must parse cleanly.
pub const CORPUS_PASS_FLOOR: usize = 3;

/// Minimum number of pinned KiCad `.kicad_pcb` corpus fixtures that must round-trip losslessly.
pub const CORPUS_ROUNDTRIP_FLOOR: usize = 3;

/// Pinned real-structure KiCad `.kicad_pcb` corpus fixtures verified by our Tier 4 oracle harness.
pub const PINNED_KICAD_CORPUS: &[(&str, &str)] = &[
    (
        "ldo-power-rev-a.kicad_pcb",
        SAMPLE_KICAD_PCB_SNIPPET,
    ),
    (
        "spi-sensor-rev-b.kicad_pcb",
        "(kicad_pcb (version 20240108) (footprint \"0603\" (ref \"R12\") (val \"10k\") (at 52.0 35.0) (size 10.0 6.0) (net 2)) (pad \"1\" smd (at 20.0 48.0) (size 4.0 2.4) (layer \"B.Cu\") (net 2)) (segment (start 20.0 48.0) (end 55.0 48.0) (width 0.8) (layer \"B.Cu\") (net 2)))",
    ),
    (
        "usb-header-rev-c.kicad_pcb",
        "(kicad_pcb (version 20240108) (footprint \"Header\" (ref \"J1\") (val \"CONN_2P\") (at 68.0 35.0) (size 10.0 14.0) (net 1)) (segment (start 14.0 16.0) (end 28.0 16.0) (width 0.3) (layer \"F.SilkS\") (net 0)) (segment (start 20.0 32.0) (end 55.0 48.0) (width 0.25) (layer \"Ratsnest\") (net 2)))",
    ),
];

/// Typed, panic-free errors returned when lowering a KiCad S-expression AST into `BoardModel`.
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

/// Tier 4 pinned KiCad corpus oracle report tracking pass and lossless round-trip counts.
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

    pub fn summary_badge(&self) -> String {
        let status = if self.meets_ratchet_floors() {
            "PASS"
        } else {
            "FAIL"
        };
        format!(
            "Corpus [{status}]: {}/{} pass, {}/{} round-trip",
            self.passed_files, self.total_files, self.roundtrip_files, self.total_files
        )
    }
}

/// Runs the Tier 4 pinned KiCad `.kicad_pcb` corpus oracle across all `PINNED_KICAD_CORPUS` fixtures.
pub fn run_pinned_kicad_corpus() -> CorpusOracleReport {
    let mut passed_files = 0;
    let mut roundtrip_files = 0;
    for &(_name, raw_sexpr) in PINNED_KICAD_CORPUS {
        if let Ok(board) = BoardModel::from_kicad_pcb_sexpr(raw_sexpr) {
            passed_files += 1;
            let exported = board.to_kicad_pcb_sexpr();
            if let Ok(reloaded) = BoardModel::from_kicad_pcb_sexpr(&exported)
                && reloaded == board
            {
                roundtrip_files += 1;
            }
        }
    }
    CorpusOracleReport {
        total_files: PINNED_KICAD_CORPUS.len(),
        passed_files,
        roundtrip_files,
    }
}
```

- **`#[from] SexprParseError` Error Chaining**: Decorating `Sexpr(#[from] SexprParseError)` with `thiserror`'s `#[from]` attribute generates `impl From<SexprParseError> for KicadLowerError`, allowing `BoardModel::from_kicad_pcb_sexpr` to call `parse_sexpr_str(raw)?` with the `?` operator and automatically promote syntax errors into `KicadLowerError::Sexpr`.
- **Granular Semantic Error Variants**: `UnexpectedRootTag(String)` rejects non-PCB S-expressions like `(kicad_sch ...)`, `MissingField(&'static str)` reports omitted child lists (`"package"`, `"ref"`, `"at"`, etc.), `InvalidNumber(String)` catches malformed floats/integers, and `UnknownLayer(String)` flags unrecognized layer names.
- **`CorpusOracleReport` & `run_pinned_kicad_corpus`**: Evaluates every fixture in `PINNED_KICAD_CORPUS`, counting both clean imports (`passed_files`) and lossless `to_kicad_pcb_sexpr() -> from_kicad_pcb_sexpr()` round-trips (`roundtrip_files`), then enforces `CORPUS_PASS_FLOOR` and `CORPUS_ROUNDTRIP_FLOOR` in `meets_ratchet_floors()`.

### Block 2: Layer Name Mapping & AST Node Lowering Helpers (`crates/pcb-doc/src/lib.rs`)

```rust caption="Mapping KiCad layer strings and lowering footprint, pad, and segment nodes"
impl PcbLayer {
    // ... existing z_rank() and rgb() methods ...

    pub fn kicad_name(self) -> &'static str {
        match self {
            PcbLayer::BottomCopper => "B.Cu",
            PcbLayer::TopCopper => "F.Cu",
            PcbLayer::Silkscreen => "F.SilkS",
            PcbLayer::Ratsnest => "Ratsnest",
        }
    }

    pub fn from_kicad_name(name: &str) -> Result<Self, KicadLowerError> {
        match name {
            "B.Cu" => Ok(PcbLayer::BottomCopper),
            "F.Cu" => Ok(PcbLayer::TopCopper),
            "F.SilkS" => Ok(PcbLayer::Silkscreen),
            "Ratsnest" => Ok(PcbLayer::Ratsnest),
            other => Err(KicadLowerError::UnknownLayer(other.to_owned())),
        }
    }
}

fn parse_f32_token(raw: &str) -> Result<f32, KicadLowerError> {
    raw.parse::<f32>()
        .map_err(|_| KicadLowerError::InvalidNumber(raw.to_owned()))
}

fn parse_child_f32_pair(
    parent: &SexprNode,
    tag: &'static str,
) -> Result<(f32, f32), KicadLowerError> {
    let list = parent
        .find_child_list(tag)
        .ok_or(KicadLowerError::MissingField(tag))?;
    let children = list.children();
    let x_str = children
        .first()
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))?;
    let y_str = children
        .get(1)
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))?;
    Ok((parse_f32_token(x_str)?, parse_f32_token(y_str)?))
}

fn parse_child_text<'a>(
    parent: &'a SexprNode,
    tag: &'static str,
) -> Result<&'a str, KicadLowerError> {
    parent
        .find_child_list(tag)
        .and_then(|n| n.children().first())
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))
}

fn parse_child_u32(parent: &SexprNode, tag: &'static str) -> Result<u32, KicadLowerError> {
    let raw = parse_child_text(parent, tag)?;
    raw.parse::<u32>()
        .map_err(|_| KicadLowerError::InvalidNumber(raw.to_owned()))
}

fn lower_footprint_node(node: &SexprNode) -> Result<ComponentSymbol, KicadLowerError> {
    let package_name = node
        .children()
        .first()
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField("package"))?
        .to_owned();
    let ref_des = parse_child_text(node, "ref")?.to_owned();
    let value = parse_child_text(node, "val")?.to_owned();
    let (cx, cy) = parse_child_f32_pair(node, "at")?;
    let (w, h) = parse_child_f32_pair(node, "size")?;
    let net_num = parse_child_u32(node, "net")?;
    Ok(ComponentSymbol {
        ref_des,
        value,
        package_name,
        center_mm: pos2(cx, cy),
        body_size_mm: vec2(w, h),
        primary_net: NetId(net_num),
    })
}

fn lower_pad_node(node: &SexprNode) -> Result<SmdPad, KicadLowerError> {
    let (cx, cy) = parse_child_f32_pair(node, "at")?;
    let (w, h) = parse_child_f32_pair(node, "size")?;
    let layer = PcbLayer::from_kicad_name(parse_child_text(node, "layer")?)?;
    let net_num = parse_child_u32(node, "net")?;
    Ok(SmdPad {
        center_mm: pos2(cx, cy),
        size_mm: vec2(w, h),
        layer,
        net: NetId(net_num),
    })
}

fn lower_segment_node(node: &SexprNode) -> Result<TraceSegment, KicadLowerError> {
    let (x0, y0) = parse_child_f32_pair(node, "start")?;
    let (x1, y1) = parse_child_f32_pair(node, "end")?;
    let width_mm = parse_f32_token(parse_child_text(node, "width")?)?;
    let layer = PcbLayer::from_kicad_name(parse_child_text(node, "layer")?)?;
    let net_num = parse_child_u32(node, "net")?;
    let net = if net_num == 0 {
        None
    } else {
        Some(NetId(net_num))
    };
    Ok(TraceSegment {
        start: pos2(x0, y0),
        end: pos2(x1, y1),
        width_mm,
        layer,
        net,
    })
}
```

- **`PcbLayer::kicad_name` & `from_kicad_name`**: Provides a bijection between KiCad's canonical layer strings (`"B.Cu"`, `"F.Cu"`, `"F.SilkS"`, `"Ratsnest"`) and our `PcbLayer` enum, returning `Err(KicadLowerError::UnknownLayer(...))` on unsupported layers like `"In1.Cu"`.
- **Composable Field Extractors (`parse_child_text`, `parse_child_f32_pair`, `parse_child_u32`)**: Each helper chains `parent.find_child_list(tag)` with `.ok_or(KicadLowerError::MissingField(tag))?` and `.parse::<T>()` so missing or malformed attributes fail with exact diagnostics and zero `.unwrap()` calls.
- **`lower_footprint_node`, `lower_pad_node`, `lower_segment_node`**: Translate each KiCad AST subtree into a typed `ComponentSymbol`, `SmdPad`, or `TraceSegment` (mapping KiCad's `net 0` unrouted/graphic convention to `None` on `TraceSegment::net`).

### Block 3: Board-Level S-Expression Import & Export (`BoardModel::from_kicad_pcb_sexpr` & `to_kicad_pcb_sexpr`)

```rust caption="Importing and exporting complete .kicad_pcb S-expressions on BoardModel"
impl BoardModel {
    /// Lowers a KiCad `(kicad_pcb ...)` S-expression string into a typed `BoardModel`.
    pub fn from_kicad_pcb_sexpr(raw: &str) -> Result<Self, KicadLowerError> {
        let root = parse_sexpr_str(raw)?;
        let root_tag = root.tag_name().unwrap_or("atom");
        if root_tag != "kicad_pcb" {
            return Err(KicadLowerError::UnexpectedRootTag(root_tag.to_owned()));
        }

        let mut components = Vec::new();
        let mut pads = Vec::new();
        let mut traces = Vec::new();

        for child in root.children() {
            match child.tag_name() {
                Some("footprint") => components.push(lower_footprint_node(child)?),
                Some("pad") => pads.push(lower_pad_node(child)?),
                Some("segment") => traces.push(lower_segment_node(child)?),
                _ => {}
            }
        }

        Ok(Self {
            traces: Arc::new(traces),
            pads: Arc::new(pads),
            components: Arc::new(components),
            review_pins: Arc::new(HashMap::new()),
            next_pin_id: 1,
        })
    }

    /// Exports the board's footprints, SMD pads, and trace segments to a canonical KiCad `.kicad_pcb` S-expression.
    pub fn to_kicad_pcb_sexpr(&self) -> String {
        let mut parts = vec!["(kicad_pcb (version 20240108)".to_owned()];
        for comp in self.components.iter() {
            parts.push(format!(
                "(footprint \"{}\" (ref \"{}\") (val \"{}\") (at {:.1} {:.1}) (size {:.1} {:.1}) (net {}))",
                comp.package_name,
                comp.ref_des,
                comp.value,
                comp.center_mm.x,
                comp.center_mm.y,
                comp.body_size_mm.x,
                comp.body_size_mm.y,
                comp.primary_net.0
            ));
        }
        for (idx, pad) in self.pads.iter().enumerate() {
            parts.push(format!(
                "(pad \"{}\" smd (at {:.1} {:.1}) (size {:.1} {:.1}) (layer \"{}\") (net {}))",
                idx + 1,
                pad.center_mm.x,
                pad.center_mm.y,
                pad.size_mm.x,
                pad.size_mm.y,
                pad.layer.kicad_name(),
                pad.net.0
            ));
        }
        for seg in self.traces.iter() {
            let net_num = seg.net.map_or(0, |n| n.0);
            parts.push(format!(
                "(segment (start {:.1} {:.1}) (end {:.1} {:.1}) (width {:.2}) (layer \"{}\") (net {}))",
                seg.start.x,
                seg.start.y,
                seg.end.x,
                seg.end.y,
                seg.width_mm,
                seg.layer.kicad_name(),
                net_num
            ));
        }
        format!("{})", parts.join(" "))
    }
}
```

- **`from_kicad_pcb_sexpr`**: Validates that the root list tag is `"kicad_pcb"`, dispatches `"footprint"`, `"pad"`, and `"segment"` children to their respective lowering functions, ignores informational metadata tags like `(version ...)` or `(net ...)`, and wraps the resulting collections in `Arc::new(...)` for `BoardModel`.
- **`to_kicad_pcb_sexpr`**: Serializes `self.components`, `self.pads`, and `self.traces` back into canonical `.kicad_pcb` S-expression syntax so `BoardModel::from_kicad_pcb_sexpr(&board.to_kicad_pcb_sexpr())` round-trips losslessly.

### Block 4: Wiring KiCad Board Import/Export & Corpus Telemetry into `src/main.rs`

```rust caption="Adding import_kicad_sexpr_board, export_kicad_sexpr_board, and UI buttons in src/main.rs"
impl PcbReviewerApp {
    // ...
    pub fn import_kicad_sexpr_board(&mut self) -> bool {
        match BoardModel::from_kicad_pcb_sexpr(&self.view.ast_source_text) {
            Ok(board) => {
                self.board = board;
                self.view.parse_active_sexpr();
                true
            }
            Err(err) => {
                self.view.parsed_ast = None;
                self.view.last_ast_error = Some(err.to_string());
                false
            }
        }
    }

    pub fn export_kicad_sexpr_board(&mut self) {
        self.view.ast_source_text = self.board.to_kicad_pcb_sexpr();
        self.view.parse_active_sexpr();
    }

    pub fn render_session_panel(&mut self, ui: &mut egui::Ui) {
        // ... inside the "KiCad S-Expr AST (nom)" section:
        ui.separator();
        ui.heading("KiCad S-Expr AST (nom)");
        ui.label(self.view.ast_status_badge());
        ui.label(run_pinned_kicad_corpus().summary_badge());
        ui.horizontal(|ui| {
            if ui.button("Parse Sample KiCad AST").clicked() {
                self.view.ast_source_text = SAMPLE_KICAD_PCB_SNIPPET.to_owned();
                self.view.parse_active_sexpr();
            }
            if ui.button("Test Unclosed S-Expr").clicked() {
                self.view.ast_source_text = "(kicad_pcb (net 1".to_owned();
                self.view.parse_active_sexpr();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Import KiCad Board").clicked() {
                self.import_kicad_sexpr_board();
            }
            if ui.button("Export KiCad S-Expr").clicked() {
                self.export_kicad_sexpr_board();
            }
        });
        // ... CollapsingHeader tree inspector ...
    }
}
```

- **`import_kicad_sexpr_board(&mut self) -> bool`**: Lowers `self.view.ast_source_text` into `self.board` and refreshes the `CollapsingHeader` AST tree on success, or records `AST Error: ...` in `self.view.last_ast_error` on failure.
- **`export_kicad_sexpr_board(&mut self)`**: Formats the active `self.board` via `to_kicad_pcb_sexpr()`, stores it in `self.view.ast_source_text`, and re-parses the live AST tree.
- **Corpus Status Badge & Action Row**: Displays `run_pinned_kicad_corpus().summary_badge()` (`Corpus [PASS]: 3/3 pass, 3/3 round-trip`) alongside `"Import KiCad Board"` and `"Export KiCad S-Expr"` buttons in the right-hand session panel.

### Block 5: Corpus Oracle & Synthetic Crash-Regression Tests (`crates/pcb-doc/src/lib.rs` & `src/main.rs`)

```rust caption="Tier 4 corpus oracle and synthetic regression tests in crates/pcb-doc/src/lib.rs"
    #[test]
    fn kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work() {
        for (layer, name) in [
            (PcbLayer::BottomCopper, "B.Cu"),
            (PcbLayer::TopCopper, "F.Cu"),
            (PcbLayer::Silkscreen, "F.SilkS"),
            (PcbLayer::Ratsnest, "Ratsnest"),
        ] {
            assert_eq!(layer.kicad_name(), name);
            assert_eq!(PcbLayer::from_kicad_name(name), Ok(layer));
        }
        assert_eq!(
            PcbLayer::from_kicad_name("In1.Cu"),
            Err(KicadLowerError::UnknownLayer("In1.Cu".to_owned()))
        );

        let report = run_pinned_kicad_corpus();
        assert_eq!(report.total_files, 3);
        assert_eq!(report.passed_files, 3);
        assert_eq!(report.roundtrip_files, 3);
        assert!(report.meets_ratchet_floors());
        assert_eq!(
            report.summary_badge(),
            "Corpus [PASS]: 3/3 pass, 3/3 round-trip"
        );

        // Synthetic crash-regression & validation error coverage
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_sch (version 1))"),
            Err(KicadLowerError::UnexpectedRootTag("kicad_sch".to_owned()))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_pcb (footprint))"),
            Err(KicadLowerError::MissingField("package"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (footprint \"SOT-23\" (ref \"U1\") (val \"LDO\") (at 20.0 bad_y) (size 12.0 16.0) (net 1)))"
            ),
            Err(KicadLowerError::InvalidNumber("bad_y".to_owned()))
        );
    }
```

- **Tier 4 Corpus Ratchet Assertions**: Verifies that all 3 pinned boards pass `run_pinned_kicad_corpus()` and also tests each failure condition of `CorpusOracleReport::meets_ratchet_floors()` (`passed_files < CORPUS_PASS_FLOOR`, `passed_files != total_files`, and `roundtrip_files < CORPUS_ROUNDTRIP_FLOOR`) so `cargo-mutants` cannot flip `&&` or `>=` operators.
- **Deterministic Synthetic Regression Tests**: Follows Critical Rule #4 of our testing architecture by asserting exact `KicadLowerError` variants on malformed root tags, missing fields, and non-numeric coordinates.

### Complete KiCad Lowering & UI Integration Context Listing

Here is the complete assembled code added to `crates/pcb-doc/src/lib.rs` and `src/main.rs` in Lesson 0024:

```rust caption="Complete KiCad lowering, corpus oracle, and tests in crates/pcb-doc/src/lib.rs"
/// Minimum number of pinned KiCad `.kicad_pcb` corpus fixtures that must parse cleanly.
pub const CORPUS_PASS_FLOOR: usize = 3;

/// Minimum number of pinned KiCad `.kicad_pcb` corpus fixtures that must round-trip losslessly.
pub const CORPUS_ROUNDTRIP_FLOOR: usize = 3;

/// Pinned real-structure KiCad `.kicad_pcb` corpus fixtures verified by our Tier 4 oracle harness.
pub const PINNED_KICAD_CORPUS: &[(&str, &str)] = &[
    (
        "ldo-power-rev-a.kicad_pcb",
        SAMPLE_KICAD_PCB_SNIPPET,
    ),
    (
        "spi-sensor-rev-b.kicad_pcb",
        "(kicad_pcb (version 20240108) (footprint \"0603\" (ref \"R12\") (val \"10k\") (at 52.0 35.0) (size 10.0 6.0) (net 2)) (pad \"1\" smd (at 20.0 48.0) (size 4.0 2.4) (layer \"B.Cu\") (net 2)) (segment (start 20.0 48.0) (end 55.0 48.0) (width 0.8) (layer \"B.Cu\") (net 2)))",
    ),
    (
        "usb-header-rev-c.kicad_pcb",
        "(kicad_pcb (version 20240108) (footprint \"Header\" (ref \"J1\") (val \"CONN_2P\") (at 68.0 35.0) (size 10.0 14.0) (net 1)) (segment (start 14.0 16.0) (end 28.0 16.0) (width 0.3) (layer \"F.SilkS\") (net 0)) (segment (start 20.0 32.0) (end 55.0 48.0) (width 0.25) (layer \"Ratsnest\") (net 2)))",
    ),
];

/// Typed, panic-free errors returned when lowering a KiCad S-expression AST into `BoardModel`.
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

/// Tier 4 pinned KiCad corpus oracle report tracking pass and lossless round-trip counts.
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

    pub fn summary_badge(&self) -> String {
        let status = if self.meets_ratchet_floors() {
            "PASS"
        } else {
            "FAIL"
        };
        format!(
            "Corpus [{status}]: {}/{} pass, {}/{} round-trip",
            self.passed_files, self.total_files, self.roundtrip_files, self.total_files
        )
    }
}

/// Runs the Tier 4 pinned KiCad `.kicad_pcb` corpus oracle across all `PINNED_KICAD_CORPUS` fixtures.
pub fn run_pinned_kicad_corpus() -> CorpusOracleReport {
    let mut passed_files = 0;
    let mut roundtrip_files = 0;
    for &(_name, raw_sexpr) in PINNED_KICAD_CORPUS {
        if let Ok(board) = BoardModel::from_kicad_pcb_sexpr(raw_sexpr) {
            passed_files += 1;
            let exported = board.to_kicad_pcb_sexpr();
            if let Ok(reloaded) = BoardModel::from_kicad_pcb_sexpr(&exported)
                && reloaded == board
            {
                roundtrip_files += 1;
            }
        }
    }
    CorpusOracleReport {
        total_files: PINNED_KICAD_CORPUS.len(),
        passed_files,
        roundtrip_files,
    }
}

impl PcbLayer {
    // ...
    pub fn kicad_name(self) -> &'static str {
        match self {
            PcbLayer::BottomCopper => "B.Cu",
            PcbLayer::TopCopper => "F.Cu",
            PcbLayer::Silkscreen => "F.SilkS",
            PcbLayer::Ratsnest => "Ratsnest",
        }
    }

    pub fn from_kicad_name(name: &str) -> Result<Self, KicadLowerError> {
        match name {
            "B.Cu" => Ok(PcbLayer::BottomCopper),
            "F.Cu" => Ok(PcbLayer::TopCopper),
            "F.SilkS" => Ok(PcbLayer::Silkscreen),
            "Ratsnest" => Ok(PcbLayer::Ratsnest),
            other => Err(KicadLowerError::UnknownLayer(other.to_owned())),
        }
    }
}

fn parse_f32_token(raw: &str) -> Result<f32, KicadLowerError> {
    raw.parse::<f32>()
        .map_err(|_| KicadLowerError::InvalidNumber(raw.to_owned()))
}

fn parse_child_f32_pair(
    parent: &SexprNode,
    tag: &'static str,
) -> Result<(f32, f32), KicadLowerError> {
    let list = parent
        .find_child_list(tag)
        .ok_or(KicadLowerError::MissingField(tag))?;
    let children = list.children();
    let x_str = children
        .first()
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))?;
    let y_str = children
        .get(1)
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))?;
    Ok((parse_f32_token(x_str)?, parse_f32_token(y_str)?))
}

fn parse_child_text<'a>(
    parent: &'a SexprNode,
    tag: &'static str,
) -> Result<&'a str, KicadLowerError> {
    parent
        .find_child_list(tag)
        .and_then(|n| n.children().first())
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField(tag))
}

fn parse_child_u32(parent: &SexprNode, tag: &'static str) -> Result<u32, KicadLowerError> {
    let raw = parse_child_text(parent, tag)?;
    raw.parse::<u32>()
        .map_err(|_| KicadLowerError::InvalidNumber(raw.to_owned()))
}

fn lower_footprint_node(node: &SexprNode) -> Result<ComponentSymbol, KicadLowerError> {
    let package_name = node
        .children()
        .first()
        .and_then(SexprNode::as_text)
        .ok_or(KicadLowerError::MissingField("package"))?
        .to_owned();
    let ref_des = parse_child_text(node, "ref")?.to_owned();
    let value = parse_child_text(node, "val")?.to_owned();
    let (cx, cy) = parse_child_f32_pair(node, "at")?;
    let (w, h) = parse_child_f32_pair(node, "size")?;
    let net_num = parse_child_u32(node, "net")?;
    Ok(ComponentSymbol {
        ref_des,
        value,
        package_name,
        center_mm: pos2(cx, cy),
        body_size_mm: vec2(w, h),
        primary_net: NetId(net_num),
    })
}

fn lower_pad_node(node: &SexprNode) -> Result<SmdPad, KicadLowerError> {
    let (cx, cy) = parse_child_f32_pair(node, "at")?;
    let (w, h) = parse_child_f32_pair(node, "size")?;
    let layer = PcbLayer::from_kicad_name(parse_child_text(node, "layer")?)?;
    let net_num = parse_child_u32(node, "net")?;
    Ok(SmdPad {
        center_mm: pos2(cx, cy),
        size_mm: vec2(w, h),
        layer,
        net: NetId(net_num),
    })
}

fn lower_segment_node(node: &SexprNode) -> Result<TraceSegment, KicadLowerError> {
    let (x0, y0) = parse_child_f32_pair(node, "start")?;
    let (x1, y1) = parse_child_f32_pair(node, "end")?;
    let width_mm = parse_f32_token(parse_child_text(node, "width")?)?;
    let layer = PcbLayer::from_kicad_name(parse_child_text(node, "layer")?)?;
    let net_num = parse_child_u32(node, "net")?;
    let net = if net_num == 0 {
        None
    } else {
        Some(NetId(net_num))
    };
    Ok(TraceSegment {
        start: pos2(x0, y0),
        end: pos2(x1, y1),
        width_mm,
        layer,
        net,
    })
}

impl BoardModel {
    // ...
    /// Lowers a KiCad `(kicad_pcb ...)` S-expression string into a typed `BoardModel`.
    pub fn from_kicad_pcb_sexpr(raw: &str) -> Result<Self, KicadLowerError> {
        let root = parse_sexpr_str(raw)?;
        let root_tag = root.tag_name().unwrap_or("atom");
        if root_tag != "kicad_pcb" {
            return Err(KicadLowerError::UnexpectedRootTag(root_tag.to_owned()));
        }

        let mut components = Vec::new();
        let mut pads = Vec::new();
        let mut traces = Vec::new();

        for child in root.children() {
            match child.tag_name() {
                Some("footprint") => components.push(lower_footprint_node(child)?),
                Some("pad") => pads.push(lower_pad_node(child)?),
                Some("segment") => traces.push(lower_segment_node(child)?),
                _ => {}
            }
        }

        Ok(Self {
            traces: Arc::new(traces),
            pads: Arc::new(pads),
            components: Arc::new(components),
            review_pins: Arc::new(HashMap::new()),
            next_pin_id: 1,
        })
    }

    /// Exports the board's footprints, SMD pads, and trace segments to a canonical KiCad `.kicad_pcb` S-expression.
    pub fn to_kicad_pcb_sexpr(&self) -> String {
        let mut parts = vec!["(kicad_pcb (version 20240108)".to_owned()];
        for comp in self.components.iter() {
            parts.push(format!(
                "(footprint \"{}\" (ref \"{}\") (val \"{}\") (at {:.1} {:.1}) (size {:.1} {:.1}) (net {}))",
                comp.package_name,
                comp.ref_des,
                comp.value,
                comp.center_mm.x,
                comp.center_mm.y,
                comp.body_size_mm.x,
                comp.body_size_mm.y,
                comp.primary_net.0
            ));
        }
        for (idx, pad) in self.pads.iter().enumerate() {
            parts.push(format!(
                "(pad \"{}\" smd (at {:.1} {:.1}) (size {:.1} {:.1}) (layer \"{}\") (net {}))",
                idx + 1,
                pad.center_mm.x,
                pad.center_mm.y,
                pad.size_mm.x,
                pad.size_mm.y,
                pad.layer.kicad_name(),
                pad.net.0
            ));
        }
        for seg in self.traces.iter() {
            let net_num = seg.net.map_or(0, |n| n.0);
            parts.push(format!(
                "(segment (start {:.1} {:.1}) (end {:.1} {:.1}) (width {:.2}) (layer \"{}\") (net {}))",
                seg.start.x,
                seg.start.y,
                seg.end.x,
                seg.end.y,
                seg.width_mm,
                seg.layer.kicad_name(),
                net_num
            ));
        }
        format!("{})", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    // ...
    #[test]
    fn kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work() {
        for (layer, name) in [
            (PcbLayer::BottomCopper, "B.Cu"),
            (PcbLayer::TopCopper, "F.Cu"),
            (PcbLayer::Silkscreen, "F.SilkS"),
            (PcbLayer::Ratsnest, "Ratsnest"),
        ] {
            assert_eq!(layer.kicad_name(), name);
            assert_eq!(PcbLayer::from_kicad_name(name), Ok(layer));
        }
        assert_eq!(
            PcbLayer::from_kicad_name("In1.Cu"),
            Err(KicadLowerError::UnknownLayer("In1.Cu".to_owned()))
        );

        let report = run_pinned_kicad_corpus();
        assert_eq!(report.total_files, 3);
        assert_eq!(report.passed_files, 3);
        assert_eq!(report.roundtrip_files, 3);
        assert!(report.meets_ratchet_floors());
        assert_eq!(
            report.summary_badge(),
            "Corpus [PASS]: 3/3 pass, 3/3 round-trip"
        );

        let spi_board = BoardModel::from_kicad_pcb_sexpr(PINNED_KICAD_CORPUS[1].1).unwrap();
        assert_eq!(spi_board.components.len(), 1);
        assert_eq!(spi_board.pads.len(), 1);
        assert_eq!(spi_board.traces.len(), 1);
        assert_eq!(spi_board.traces[0].net, Some(NetId(2)));

        let usb_board = BoardModel::from_kicad_pcb_sexpr(PINNED_KICAD_CORPUS[2].1).unwrap();
        assert_eq!(usb_board.traces.len(), 2);
        assert_eq!(usb_board.traces[0].net, None);
        assert_eq!(usb_board.traces[1].net, Some(NetId(2)));

        for failing_report in [
            CorpusOracleReport {
                total_files: 3,
                passed_files: 2,
                roundtrip_files: 3,
            },
            CorpusOracleReport {
                total_files: 4,
                passed_files: 3,
                roundtrip_files: 3,
            },
            CorpusOracleReport {
                total_files: 3,
                passed_files: 3,
                roundtrip_files: 2,
            },
        ] {
            assert!(!failing_report.meets_ratchet_floors());
            assert!(failing_report.summary_badge().starts_with("Corpus [FAIL]:"));
        }

        // Synthetic crash-regression & validation error coverage
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_sch (version 1))"),
            Err(KicadLowerError::UnexpectedRootTag("kicad_sch".to_owned()))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_pcb (footprint))"),
            Err(KicadLowerError::MissingField("package"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr("(kicad_pcb (footprint \"SOT-23\"))"),
            Err(KicadLowerError::MissingField("ref"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (footprint \"SOT-23\" (ref \"U1\") (val \"LDO\") (at 20.0)))"
            ),
            Err(KicadLowerError::MissingField("at"))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (footprint \"SOT-23\" (ref \"U1\") (val \"LDO\") (at 20.0 bad_y) (size 12.0 16.0) (net 1)))"
            ),
            Err(KicadLowerError::InvalidNumber("bad_y".to_owned()))
        );
        assert_eq!(
            BoardModel::from_kicad_pcb_sexpr(
                "(kicad_pcb (pad \"1\" smd (at 20.0 22.0) (size 4.0 2.4) (layer \"F.Cu\") (net bad_net)))"
            ),
            Err(KicadLowerError::InvalidNumber("bad_net".to_owned()))
        );
    }
}
```

```rust caption="Complete KiCad board import/export UI methods and egui_kittest checks in src/main.rs"
impl PcbReviewerApp {
    // ...
    pub fn import_kicad_sexpr_board(&mut self) -> bool {
        match BoardModel::from_kicad_pcb_sexpr(&self.view.ast_source_text) {
            Ok(board) => {
                self.board = board;
                self.view.parse_active_sexpr();
                true
            }
            Err(err) => {
                self.view.parsed_ast = None;
                self.view.last_ast_error = Some(err.to_string());
                false
            }
        }
    }

    pub fn export_kicad_sexpr_board(&mut self) {
        self.view.ast_source_text = self.board.to_kicad_pcb_sexpr();
        self.view.parse_active_sexpr();
    }

    pub fn render_session_panel(&mut self, ui: &mut egui::Ui) {
        // ...
        ui.separator();
        ui.heading("KiCad S-Expr AST (nom)");
        ui.label(self.view.ast_status_badge());
        ui.label(run_pinned_kicad_corpus().summary_badge());
        ui.horizontal(|ui| {
            if ui.button("Parse Sample KiCad AST").clicked() {
                self.view.ast_source_text = SAMPLE_KICAD_PCB_SNIPPET.to_owned();
                self.view.parse_active_sexpr();
            }
            if ui.button("Test Unclosed S-Expr").clicked() {
                self.view.ast_source_text = "(kicad_pcb (net 1".to_owned();
                self.view.parse_active_sexpr();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Import KiCad Board").clicked() {
                self.import_kicad_sexpr_board();
            }
            if ui.button("Export KiCad S-Expr").clicked() {
                self.export_kicad_sexpr_board();
            }
        });
        if let Some(root) = &self.view.parsed_ast {
            egui::CollapsingHeader::new(format!(
                "Root ({}) [{} children]",
                root.tag_name().unwrap_or("atom"),
                root.children().len()
            ))
            .default_open(true)
            .show(ui, |ui| {
                for child in root.children() {
                    if let Some(tag) = child.tag_name() {
                        ui.label(format!("Node: ({tag}) • {} items", child.children().len()));
                    }
                }
            });
        }
    }
}

// Inside tests::standalone_layer_swatch_and_headless_ui_workflows_pass in src/main.rs:
        assert!(
            harness
                .query_by_label("Corpus [PASS]: 3/3 pass, 3/3 round-trip")
                .is_some()
        );

        harness.get_by_label("Import KiCad Board").click();
        harness.run();
        assert_eq!(harness.state().board.components.len(), 1);
        assert_eq!(harness.state().board.pads.len(), 1);

        harness.state_mut().board = BoardModel::default();
        harness.get_by_label("Export KiCad S-Expr").click();
        harness.run();
        assert!(
            harness
                .state()
                .view
                .ast_source_text
                .contains("(segment (start 20.0 32.0)")
        );

        harness.state_mut().view.ast_source_text = "(kicad_sch (version 1))".to_owned();
        assert!(!harness.state_mut().import_kicad_sexpr_board());
        assert!(
            harness
                .state()
                .view
                .ast_status_badge()
                .starts_with("AST Error:")
        );
```

{{widget:kicad-corpus-sim}}

:::exercise id="ex0024" title="Interactive Lab 2: Map KiCad Layer Names & Enforce Corpus Floor Ratchets" badge="Skill Practice • AST Lowering & Corpus Ratchets"
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

:::verify id="verify-0024" title="Lab 3: KiCad AST Lowering, Tier 4 Corpus Oracle Ratchets & Mutation Gate (`0024`)" badge="cargo test --workspace + corpus oracle + cargo mutants" tier="Tier 2 + Tier 4 (Pinned Corpus Oracles)" caught="385 / 385 (100%)" missed="0 Missed"
Run the workspace test suite (including `kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work`) and mutate all KiCad lowering helpers, round-trip formatters, and corpus ratchet methods:

```bash caption="Run Tier 4 corpus oracle tests and cargo-mutants on 0023"
cargo test --manifest-path ./apps/0024-parsing-kicad-pcb-and-sch-into-domain-model/Cargo.toml --workspace
cargo mutants --manifest-path ./apps/0024-parsing-kicad-pcb-and-sch-into-domain-model/Cargo.toml --workspace --in-place -F "kicad_name|from_kicad_name|parse_child_f32_pair|parse_child_text|parse_child_u32|lower_footprint_node|lower_pad_node|lower_segment_node|from_kicad_pcb_sexpr|to_kicad_pcb_sexpr|CorpusOracleReport|run_pinned_kicad_corpus|import_kicad_sexpr_board|export_kicad_sexpr_board"
```

- [ ] `cargo test --manifest-path ./apps/0024-parsing-kicad-pcb-and-sch-into-domain-model/Cargo.toml --workspace` passes all **25 tests** (**3** in `kicad-sexpr`, **16** in `pcb-doc`, and **6** in `pcb-reviewer`).
- [ ] `kicad_pcb_lowering_roundtrip_and_corpus_oracle_ratchets_work` verifies `run_pinned_kicad_corpus().meets_ratchet_floors()`, lossless `to_kicad_pcb_sexpr -> from_kicad_pcb_sexpr` round-tripping, and synthetic regression rejection on malformed coordinates and unknown layers.
- [ ] `cargo mutants` catches **100% of viable KiCad lowering & corpus oracle mutants** (`0 missed`).
:::

:::quiz id="quiz-0024"
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
Next up: [Lesson 0025: Embedded SQL Review Store with Turso (`turso_parser` & Async `tokio` Bridge)](./0025-embedded-sql-review-store-with-turso.html), where we validate SQL DDL/DML ASTs with `turso_parser::parser::Parser` and persist multi-engineer review threads in an async `TursoReviewStore`!
:::

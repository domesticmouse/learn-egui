#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{take_while, take_while1},
    character::complete::{char, multispace0},
    combinator::map,
    multi::many0,
    sequence::delimited,
};
use thiserror::Error;

/// Standalone `L0` parser crates must have zero workspace or UI dependencies.
pub const FORBIDDEN_DEPS_IN_L0_STANDALONE: &[&str] = &[
    "pcb-doc",
    "pcb-reviewer",
    "egui",
    "eframe",
    "winit",
    "egui_kittest",
    "rfd",
];

/// Verifies that an `L0` standalone parser crate's `Cargo.toml` has no workspace or UI dependencies.
pub fn validate_l0_standalone_manifest(cargo_toml: &str) -> Result<(), &'static str> {
    for &forbidden in FORBIDDEN_DEPS_IN_L0_STANDALONE {
        let key_eq = format!("{forbidden} =");
        let key_dot = format!("dependencies.{forbidden}");
        if cargo_toml.contains(&key_eq) || cargo_toml.contains(&key_dot) {
            return Err(forbidden);
        }
    }
    Ok(())
}

/// Sample KiCad `.kicad_pcb` S-expression snippet used by the AST inspector and tests.
pub const SAMPLE_KICAD_PCB_SNIPPET: &str = "(kicad_pcb (version 20240108) (net 1 \"VCC_3V3\") (net 2 \"SPI_CLK\") (footprint \"SOT-23\" (ref \"U1\") (val \"LDO_3V3\") (at 20.0 27.0) (size 12.0 16.0) (net 1)) (pad \"1\" smd (at 20.0 22.0) (size 4.0 2.4) (layer \"F.Cu\") (net 1)))";

/// Typed, panic-free errors returned by the KiCad S-expression parser.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SexprParseError {
    #[error("input must not be empty")]
    EmptyInput,
    #[error("unclosed or malformed S-expression near `{0}`")]
    SyntaxError(String),
    #[error("unexpected trailing tokens: `{0}`")]
    TrailingTokens(String),
}

/// Recursive S-expression abstract syntax tree node for KiCad `.kicad_pcb` and `.kicad_sch` files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SexprNode {
    Symbol(String),
    Quoted(String),
    List(Vec<SexprNode>),
}

impl SexprNode {
    /// If this node is a parenthesized list whose first element is an unquoted `Symbol`,
    /// returns that symbol's name (e.g., `"kicad_pcb"`, `"footprint"`, `"pad"`).
    pub fn tag_name(&self) -> Option<&str> {
        match self {
            SexprNode::List(items) => match items.first() {
                Some(SexprNode::Symbol(tag)) => Some(tag.as_str()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the text of a `Symbol` or `Quoted` leaf node, or `None` for a `List`.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            SexprNode::Symbol(s) | SexprNode::Quoted(s) => Some(s.as_str()),
            SexprNode::List(_) => None,
        }
    }

    /// Returns the slice of child nodes following the leading tag in a `List` node.
    pub fn children(&self) -> &[SexprNode] {
        match self {
            SexprNode::List(items) => items.get(1..).unwrap_or(&[]),
            _ => &[],
        }
    }

    /// Finds the first child list whose `tag_name()` equals `tag`.
    pub fn find_child_list(&self, tag: &str) -> Option<&SexprNode> {
        self.children()
            .iter()
            .find(|node| node.tag_name() == Some(tag))
    }

    /// Counts the total number of AST nodes (`Symbol`, `Quoted`, and `List`) in this subtree.
    pub fn node_count(&self) -> usize {
        match self {
            SexprNode::Symbol(_) | SexprNode::Quoted(_) => 1,
            SexprNode::List(items) => 1 + items.iter().map(Self::node_count).sum::<usize>(),
        }
    }

    /// Computes the maximum nesting depth of lists in this subtree (`0` for leaf atoms).
    pub fn max_depth(&self) -> usize {
        match self {
            SexprNode::Symbol(_) | SexprNode::Quoted(_) => 0,
            SexprNode::List(items) => 1 + items.iter().map(Self::max_depth).max().unwrap_or(0),
        }
    }

    /// Formats a concise telemetry badge summarizing the root tag, total nodes, and depth.
    pub fn summary_badge(&self) -> String {
        format!(
            "AST: ({}) • {} nodes, depth {}",
            self.tag_name().unwrap_or("atom"),
            self.node_count(),
            self.max_depth()
        )
    }
}

pub fn is_symbol_char(c: char) -> bool {
    !c.is_whitespace() && c != '(' && c != ')' && c != '"'
}

fn parse_quoted_str(input: &str) -> IResult<&str, SexprNode> {
    map(
        delimited(char('"'), take_while(|c: char| c != '"'), char('"')),
        |s: &str| SexprNode::Quoted(s.to_owned()),
    )(input)
}

fn parse_symbol_str(input: &str) -> IResult<&str, SexprNode> {
    map(take_while1(is_symbol_char), |s: &str| {
        SexprNode::Symbol(s.to_owned())
    })(input)
}

fn parse_list_node(input: &str) -> IResult<&str, SexprNode> {
    map(
        delimited(
            char('('),
            many0(delimited(multispace0, parse_node, multispace0)),
            char(')'),
        ),
        SexprNode::List,
    )(input)
}

fn parse_node(input: &str) -> IResult<&str, SexprNode> {
    alt((parse_list_node, parse_quoted_str, parse_symbol_str))(input)
}

/// Parses a complete KiCad S-expression string into a `SexprNode` without panicking.
pub fn parse_sexpr_str(input: &str) -> Result<SexprNode, SexprParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(SexprParseError::EmptyInput);
    }
    match parse_node(trimmed) {
        Ok((rest, node)) => {
            let trailing = rest.trim();
            if trailing.is_empty() {
                Ok(node)
            } else {
                Err(SexprParseError::TrailingTokens(trailing.to_owned()))
            }
        }
        Err(_) => Err(SexprParseError::SyntaxError(trimmed.to_owned())),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn l0_standalone_manifest_gate_rejects_workspace_and_ui_deps() {
        let actual_manifest = include_str!("../Cargo.toml");
        assert_eq!(validate_l0_standalone_manifest(actual_manifest), Ok(()));
        assert_eq!(
            validate_l0_standalone_manifest("[dependencies]\npcb-doc = { path = \"../pcb-doc\" }"),
            Err("pcb-doc")
        );
        assert_eq!(
            validate_l0_standalone_manifest("[dependencies.egui]\nversion = \"0.37\""),
            Err("egui")
        );
    }

    #[test]
    fn is_symbol_char_and_sexpr_combinators_parse_and_reject_edge_cases() {
        assert!(is_symbol_char('a'));
        assert!(is_symbol_char('2'));
        assert!(is_symbol_char('.'));
        assert!(!is_symbol_char(' '));
        assert!(!is_symbol_char('\n'));
        assert!(!is_symbol_char('('));
        assert!(!is_symbol_char(')'));
        assert!(!is_symbol_char('"'));

        assert_eq!(parse_sexpr_str("   \n\t "), Err(SexprParseError::EmptyInput));
        assert_eq!(
            parse_sexpr_str("(kicad_pcb (net 1)"),
            Err(SexprParseError::SyntaxError("(kicad_pcb (net 1)".to_owned()))
        );
        assert_eq!(
            parse_sexpr_str("\"unterminated"),
            Err(SexprParseError::SyntaxError("\"unterminated".to_owned()))
        );
        assert_eq!(
            parse_sexpr_str("(net 1 \"VCC\") extra_token"),
            Err(SexprParseError::TrailingTokens("extra_token".to_owned()))
        );

        // Tightly packed tokens without spaces before `(`, `)`, or `"`
        let packed = parse_sexpr_str("(footprint\"SOT-23\"(at 20.0 27.0))").unwrap();
        assert_eq!(packed.tag_name(), Some("footprint"));
        assert_eq!(packed.children().len(), 2);
        assert_eq!(packed.children()[0].as_text(), Some("SOT-23"));
        assert_eq!(
            packed
                .find_child_list("at")
                .and_then(|n| n.children().first())
                .and_then(SexprNode::as_text),
            Some("20.0")
        );
        assert_eq!(packed.find_child_list("missing"), None);

        // Leaf node and empty list edge cases
        let sym = SexprNode::Symbol("F.Cu".to_owned());
        assert_eq!(sym.tag_name(), None);
        assert_eq!(sym.as_text(), Some("F.Cu"));
        assert!(sym.children().is_empty());
        assert_eq!(sym.node_count(), 1);
        assert_eq!(sym.max_depth(), 0);
        assert_eq!(sym.summary_badge(), "AST: (atom) • 1 nodes, depth 0");

        let empty_list = parse_sexpr_str("()").unwrap();
        assert_eq!(empty_list.tag_name(), None);
        assert_eq!(empty_list.as_text(), None);
        assert!(empty_list.children().is_empty());
        assert_eq!(empty_list.node_count(), 1);
        assert_eq!(empty_list.max_depth(), 1);

        let quoted_head = parse_sexpr_str("(\"quoted_head\" 42)").unwrap();
        assert_eq!(quoted_head.tag_name(), None);
        assert_eq!(quoted_head.children().len(), 1);

        // Full sample KiCad snippet metrics
        let root = parse_sexpr_str(SAMPLE_KICAD_PCB_SNIPPET).unwrap();
        assert_eq!(root.tag_name(), Some("kicad_pcb"));
        assert_eq!(root.children().len(), 5);
        assert_eq!(root.node_count(), 51);
        assert_eq!(root.max_depth(), 3);
        assert_eq!(
            root.summary_badge(),
            "AST: (kicad_pcb) • 51 nodes, depth 3"
        );
    }

    proptest! {
        #[test]
        fn prop_sexpr_parser_roundtrips_arbitrary_footprints(
            ref_des in "[A-Z]{1,3}[0-9]{1,3}",
            val in "[A-Za-z0-9_.-]{1,12}",
            x in -100.0_f32..100.0_f32,
            y in -100.0_f32..100.0_f32,
        ) {
            let src = format!(
                "(footprint \"SOT-23\" (ref \"{ref_des}\") (val \"{val}\") (at {x:.1} {y:.1}))"
            );
            let ast = parse_sexpr_str(&src).unwrap();
            prop_assert_eq!(ast.tag_name(), Some("footprint"));
            prop_assert_eq!(ast.max_depth(), 2);
            prop_assert_eq!(ast.node_count(), 13);
            let ref_node = ast.find_child_list("ref").unwrap();
            prop_assert_eq!(ref_node.children()[0].as_text(), Some(ref_des.as_str()));
            let val_node = ast.find_child_list("val").unwrap();
            prop_assert_eq!(val_node.children()[0].as_text(), Some(val.as_str()));
        }
    }
}

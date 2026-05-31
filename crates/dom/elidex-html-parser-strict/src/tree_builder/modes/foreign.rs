//! WHATWG HTML §13.2.6.5 "The rules for parsing tokens in foreign content"
//! plus the §13.2.6 tree-construction dispatcher branch that routes into it.
//!
//! Foreign content (inline SVG / MathML) is **not** a 22nd insertion mode: the
//! §13.2.6 dispatcher decides, per token, whether to use the current
//! insertion-mode's HTML-content rules or these foreign-content rules, based on
//! the adjusted current node's namespace and the integration points. The
//! dispatcher branch lives in [`in_foreign_content`] (queried by
//! [`super::super::TreeBuilder::dispatch`] before the mode match), and the
//! foreign-content token rules live in [`foreign_content`].
//!
//! # Strict semantics
//!
//! Every §13.2.6.5 "parse error" branch — a U+0000 NULL character, a DOCTYPE,
//! the breakout start tags (and `</br>` / `</p>`), a misnested end tag —
//! aborts with [`crate::StrictParseError`] rather than performing the spec's
//! recovery (FFFD substitution, pop-until-HTML-and-reprocess). Valid foreign
//! content reaches HTML content only through an integration point
//! (`<svg><foreignObject><div>`, `<math><mtext><b>`), which the dispatcher
//! routes back to the HTML-content rules.

use elidex_ecs::{Entity, Namespace};

use super::super::parse_state::is_html_whitespace;
use super::super::{foreign_adjust, parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::{TagToken, Token};
use crate::StrictParseError;

/// WHATWG HTML §13.2.6 tree-construction dispatcher: whether `token` is to be
/// processed by the foreign-content rules ([`foreign_content`]) rather than
/// the current insertion mode's HTML-content rules.
///
/// Returns `false` (→ HTML content) when any of the eight listed conditions
/// holds: the stack is empty; the adjusted current node is an HTML-namespace
/// element; the adjusted current node is a MathML text integration point and
/// the token is a non-`mglyph`/`malignmark` start tag or a character; the
/// adjusted current node is a MathML `annotation-xml` element and the token is
/// an `<svg>` start tag; the adjusted current node is an HTML integration
/// point and the token is a start tag or character; or the token is EOF.
/// Otherwise returns `true`. The adjusted current node is the current node for
/// document parsing (fragment-context is deferred, D-fc-d).
pub(crate) fn in_foreign_content(tb: &TreeBuilder, token: &Token) -> bool {
    // Condition 1: the stack of open elements is empty.
    let Some(node) = tb.state.current_node() else {
        return false;
    };
    let namespace = tb.dom.namespace_of(node);
    // Condition 2: the adjusted current node is in the HTML namespace.
    if namespace == Namespace::Html {
        return false;
    }
    match token {
        // Condition 8: an end-of-file token.
        Token::EndOfFile => return false,
        Token::StartTag(tag) => {
            // Condition 3: MathML text integration point + a start tag whose
            // name is neither "mglyph" nor "malignmark".
            if is_mathml_text_integration_point(tb, node)
                && tag.name != "mglyph"
                && tag.name != "malignmark"
            {
                return false;
            }
            // Condition 5: a MathML annotation-xml element + an "svg" start tag.
            if namespace == Namespace::MathMl
                && tb.dom.has_tag(node, "annotation-xml")
                && tag.name == "svg"
            {
                return false;
            }
            // Condition 6: an HTML integration point + a start tag.
            if is_html_integration_point(tb, node) {
                return false;
            }
        }
        Token::Character(_) => {
            // Conditions 4 & 7: a MathML text integration point or an HTML
            // integration point + a character token.
            if is_mathml_text_integration_point(tb, node) || is_html_integration_point(tb, node) {
                return false;
            }
        }
        // End tags, comments, and DOCTYPEs at a foreign node take the
        // foreign-content rules (end-tag walk / insert / reject).
        Token::EndTag(_) | Token::Comment(_) | Token::Doctype(_) => {}
    }
    true
}

/// Whether `entity` is a MathML text integration point (WHATWG HTML §13.2.6):
/// a MathML `mi` / `mo` / `mn` / `ms` / `mtext` element.
fn is_mathml_text_integration_point(tb: &TreeBuilder, entity: Entity) -> bool {
    tb.dom.namespace_of(entity) == Namespace::MathMl
        && tb.dom.with_tag_name(entity, |t| {
            matches!(t, Some("mi" | "mo" | "mn" | "ms" | "mtext"))
        })
}

/// Whether `entity` is an HTML integration point (WHATWG HTML §13.2.6): a
/// MathML `annotation-xml` element whose `encoding` is `text/html` or
/// `application/xhtml+xml` (ASCII case-insensitive), or an SVG `foreignObject`
/// / `desc` / `title` element.
fn is_html_integration_point(tb: &TreeBuilder, entity: Entity) -> bool {
    match tb.dom.namespace_of(entity) {
        Namespace::MathMl => {
            tb.dom.has_tag(entity, "annotation-xml")
                && tb.dom.with_attribute(entity, "encoding", |enc| {
                    enc.is_some_and(|e| {
                        e.eq_ignore_ascii_case("text/html")
                            || e.eq_ignore_ascii_case("application/xhtml+xml")
                    })
                })
        }
        Namespace::Svg => tb.dom.with_tag_name(entity, |t| {
            matches!(t, Some("foreignObject" | "desc" | "title"))
        }),
        Namespace::Html => false,
    }
}

/// WHATWG HTML §13.2.6.5 — handle `token` under the rules for parsing tokens
/// in foreign content.
pub(crate) fn foreign_content(
    tb: &mut TreeBuilder,
    token: &Token,
) -> Result<Flow, StrictParseError> {
    match token {
        // A U+0000 NULL is a parse error (the spec inserts U+FFFD as recovery);
        // strict rejects. It only reaches tree construction inside a CDATA
        // section, where the tokenizer emits it verbatim (§13.2.5.69).
        Token::Character('\u{0000}') => {
            Err(parse_error("unexpected-null-character-in-foreign-content"))
        }
        // Whitespace and other characters are inserted; non-whitespace also
        // clears the frameset-ok flag.
        Token::Character(ch) => {
            tb.insert_character(*ch);
            if !is_html_whitespace(*ch) {
                tb.state.frameset_ok = false;
            }
            Ok(Flow::Next)
        }
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        // A DOCTYPE in foreign content is a parse error; strict rejects.
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-foreign-content")),
        Token::StartTag(tag) => start_tag(tb, tag),
        Token::EndTag(tag) => end_tag(tb, tag),
        // EOF is routed to the HTML-content rules by the dispatcher
        // (condition 8 in `in_foreign_content`), so it never reaches here.
        Token::EndOfFile => {
            unreachable!("EOF is dispatched to HTML content, not foreign content")
        }
    }
}

/// The §13.2.6.5 "breakout" start-tag names: HTML elements that, in valid
/// content, cannot appear in foreign context and trigger the spec's
/// pop-out-and-reprocess recovery — which strict rejects.
const BREAKOUT_TAGS: &[&str] = &[
    "b",
    "big",
    "blockquote",
    "body",
    "br",
    "center",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "hr",
    "i",
    "img",
    "li",
    "listing",
    "menu",
    "meta",
    "nobr",
    "ol",
    "p",
    "pre",
    "ruby",
    "s",
    "small",
    "span",
    "strong",
    "strike",
    "sub",
    "sup",
    "table",
    "tt",
    "u",
    "ul",
    "var",
];

/// §13.2.6.5 start-tag handling.
fn start_tag(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    // Breakout start tags (and `<font>` carrying color/face/size) are a parse
    // error with pop-out recovery in the spec; strict rejects. Valid HTML
    // re-entry happens only through an integration point, which the dispatcher
    // routes to HTML content before reaching here.
    if BREAKOUT_TAGS.contains(&tag.name.as_str())
        || (tag.name == "font"
            && tag
                .attrs
                .iter()
                .any(|(name, _)| matches!(name.as_str(), "color" | "face" | "size")))
    {
        return Err(parse_error("breakout-start-tag-in-foreign-content"));
    }

    // "Any other start tag": insert a foreign element in the adjusted current
    // node's namespace (the current node for document parsing). `dispatch`
    // only routes here via `in_foreign_content`, which already established a
    // non-empty stack whose current node is foreign, so the lookup is total.
    let node = tb
        .state
        .current_node()
        .expect("foreign-content rules run only with a foreign current node");
    let namespace = tb.dom.namespace_of(node);
    insert_foreign_start_tag(tb, tag, namespace);
    Ok(Flow::Next)
}

/// WHATWG HTML §13.2.6.1 "insert a foreign element" for a start tag created in
/// `namespace`: apply the §13.2.6.1 / §13.2.6.5 element-name and attribute
/// adjustments to a copy of the token's name and attributes, insert the
/// element, and — per §13.2.6.5 — pop it again if the start tag was
/// self-closing. (The SVG `<script/>` special case, "act as a script end
/// tag", also reduces to that pop, since strict mode never executes scripts.)
/// Shared by the in-body `<math>` / `<svg>` entry (§13.2.6.4.7) and the
/// foreign "any other start tag" path (§13.2.6.5), so the insert-a-foreign-
/// element step has a single home (relevant once `#11-xml-namespace` makes
/// the foreign-attribute adjustment non-trivial).
pub(super) fn insert_foreign_start_tag(tb: &mut TreeBuilder, tag: &TagToken, namespace: Namespace) {
    let mut name = tag.name.clone();
    let mut attrs = tag.attrs.clone();
    foreign_adjust::adjust_foreign_start_tag(namespace, &mut name, &mut attrs);
    tb.insert_foreign_element(&name, &attrs, namespace);
    if tag.self_closing {
        tb.pop();
    }
}

/// §13.2.6.5 end-tag handling.
fn end_tag(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    // `</br>` and `</p>` are breakout end tags (parse error + recovery in the
    // spec); strict rejects.
    if matches!(tag.name.as_str(), "br" | "p") {
        return Err(parse_error("breakout-end-tag-in-foreign-content"));
    }
    any_other_end_tag(tb, &tag.name)
}

/// §13.2.6.5 "any other end tag". The conforming case is that the current
/// node's tag name (ASCII-lowercased) matches the token, so the current node
/// is popped. Every other shape is misnesting: the spec's step-2 parse error
/// (a mismatch it then recovers by popping through to a matching ancestor) and
/// the steps 3 / 6-7 HTML-namespace / fragment transitions are all recovery
/// or fragment-only paths, which strict rejects. (A `</script>` end tag whose
/// current node is the SVG `script` element lands here and pops — the same
/// outcome as the spec's dedicated SVG-script arm, minus script execution.)
fn any_other_end_tag(tb: &mut TreeBuilder, token_name: &str) -> Result<Flow, StrictParseError> {
    // `token_name` is ASCII-lowercase (the tokenizer lowercases end-tag
    // names); SVG elements store a camel-cased tag name, so compare
    // case-insensitively.
    let matches_current = tb.state.current_node().is_some_and(|node| {
        tb.dom.with_tag_name(
            node,
            |t| matches!(t, Some(name) if name.eq_ignore_ascii_case(token_name)),
        )
    });
    if matches_current {
        tb.pop();
        Ok(Flow::Next)
    } else {
        Err(parse_error("misnested-end-tag-in-foreign-content"))
    }
}

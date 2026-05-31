//! WHATWG HTML §13.2.6.4.7 The "in body" insertion mode.
//!
//! The richest mode, and the place where the strict parser's "no error
//! recovery" stance is most visible. The list of active formatting elements
//! and the adoption agency algorithm (§13.2.6.4.7) are not implemented: for
//! conforming nesting "reconstruct the active formatting elements" is a no-op
//! and the adoption agency degenerates to a plain pop, so the output tree is
//! identical; any input that would need non-trivial adoption agency is
//! non-conforming and is rejected (misnested formatting end tags abort in
//! [`any_other_end_tag`], nested `<a>`/`<nobr>` abort on the start tag).
//!
//! Foreign content (`<math>` / `<svg>`, §13.2.6.4.7 → §13.2.6.5) enters here:
//! the `<math>` / `<svg>` start tags insert a foreign element (MathML / SVG
//! namespace), after which the §13.2.6 dispatcher routes the subtree's tokens
//! to the foreign-content rules ([`super::foreign`]) until the foreign root is
//! popped.

use elidex_ecs::Namespace;

use super::super::parse_state::{is_html_whitespace, InsertionMode, Scope};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::states::State;
use crate::tokenizer::token::{TagToken, Token};
use crate::StrictParseError;

/// §13.2.6.4.7 — handle a token in the "in body" insertion mode.
pub(crate) fn in_body(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        // A U+0000 NULL never reaches tree construction: the strict tokenizer
        // rejects it as a parse error. Whitespace and other characters are
        // inserted; non-whitespace also clears the frameset-ok flag.
        // "Reconstruct the active formatting elements" is a no-op (no list).
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
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-body")),
        Token::StartTag(tag) => start_tag(tb, token, tag),
        Token::EndTag(tag) => end_tag(tb, token, tag),
        Token::EndOfFile => {
            // Inside a template, defer to the "in template" rules.
            if !tb.state.template_modes.is_empty() {
                return super::in_template::in_template(tb, token);
            }
            // An element still open that is not implicitly closeable at EOF is
            // a parse error; otherwise stop parsing.
            if has_unexpected_open_element(tb) {
                return Err(parse_error("eof-with-unclosed-element"));
            }
            Ok(Flow::Stop)
        }
    }
}

/// Dispatch an "in body" start tag.
//
// This is one spec dispatch table (§13.2.6.4.7 "in body" start tags); keeping
// it as a single `match` preserves the 1:1 mapping to the spec's rule list, so
// the length and the formatting-vs-"any other start tag" body coincidence
// (the active-formatting-list push is a no-op in strict mode) are intentional.
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
fn start_tag(
    tb: &mut TreeBuilder,
    token: &Token,
    tag: &TagToken,
) -> Result<Flow, StrictParseError> {
    match tag.name.as_str() {
        "html" => Err(parse_error("unexpected-html-start-tag-in-body")),
        // Head-content elements are handled with the "in head" rules.
        "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style"
        | "template" | "title" => super::in_head::in_head(tb, token),
        "body" => Err(parse_error("unexpected-body-start-tag")),
        "frameset" => Err(parse_error("unexpected-frameset-start-tag")),
        // Block-level elements that close an open p in button scope.
        "address" | "article" | "aside" | "blockquote" | "center" | "details" | "dialog"
        | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure" | "footer" | "header"
        | "hgroup" | "main" | "menu" | "nav" | "ol" | "p" | "search" | "section" | "summary"
        | "ul" => {
            close_open_p_in_button_scope(tb)?;
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            close_open_p_in_button_scope(tb)?;
            // A heading nested directly inside another heading is a parse
            // error (the spec pops the open heading as recovery).
            if tb.current_node_has_any_tag(&["h1", "h2", "h3", "h4", "h5", "h6"]) {
                return Err(parse_error("nested-heading"));
            }
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
        "pre" | "listing" => {
            close_open_p_in_button_scope(tb)?;
            tb.insert_html_element(tag)?;
            // A leading newline is dropped (§13.2.6.4.7).
            tb.state.skip_next_lf = true;
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        "form" => start_form(tb, tag),
        "li" => start_li(tb, tag),
        "dd" | "dt" => start_dd_dt(tb, tag),
        "plaintext" => {
            close_open_p_in_button_scope(tb)?;
            tb.insert_html_element(tag)?;
            tb.tokenizer.set_state(State::Plaintext);
            Ok(Flow::Next)
        }
        "button" => {
            // A button nested inside an open button is non-conforming.
            if tb.has_tag_in_scope("button", Scope::Default) {
                return Err(parse_error("nested-button"));
            }
            tb.insert_html_element(tag)?;
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        "a" => {
            // The spec consults the list of active formatting elements; strict
            // mode has none, so a nested anchor (always non-conforming) is
            // detected by checking the stack of open elements directly.
            if tb.has_element_on_stack("a") {
                return Err(parse_error("nested-anchor"));
            }
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
        // Other formatting elements: insert as ordinary elements (the active
        // formatting list / reconstruction is a no-op for conforming input).
        "b" | "big" | "code" | "em" | "font" | "i" | "s" | "small" | "strike" | "strong" | "tt"
        | "u" => {
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
        "nobr" => {
            if tb.has_tag_in_scope("nobr", Scope::Default) {
                return Err(parse_error("nested-nobr"));
            }
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
        "applet" | "marquee" | "object" => {
            tb.insert_html_element(tag)?;
            // "Insert a marker at the end of the list of active formatting
            // elements" — no-op (no list).
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        "table" => {
            // The Document is never quirks mode in strict, so always close an
            // open p in button scope.
            close_open_p_in_button_scope(tb)?;
            tb.insert_html_element(tag)?;
            tb.state.frameset_ok = false;
            tb.state.mode = InsertionMode::InTable;
            Ok(Flow::Next)
        }
        "area" | "br" | "embed" | "img" | "keygen" | "wbr" => {
            tb.insert_void_element(tag);
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        "input" => {
            tb.insert_void_element(tag);
            // frameset-ok stays "ok" only for a hidden input.
            let hidden = tag
                .attrs
                .iter()
                .any(|(name, value)| name == "type" && value.eq_ignore_ascii_case("hidden"));
            if !hidden {
                tb.state.frameset_ok = false;
            }
            Ok(Flow::Next)
        }
        "param" | "source" | "track" => {
            tb.insert_void_element(tag);
            Ok(Flow::Next)
        }
        "hr" => {
            close_open_p_in_button_scope(tb)?;
            // `<hr>` is valid `<select>` content: when a select is in scope the
            // spec generates implied end tags first (popping an open option), so
            // the `<hr>` becomes a sibling of the option, not its child.
            if tb.has_tag_in_scope("select", Scope::Default) {
                tb.generate_implied_end_tags();
                if tb.has_tag_in_scope("option", Scope::Default)
                    || tb.has_tag_in_scope("optgroup", Scope::Default)
                {
                    return Err(parse_error("misnested-hr-in-select"));
                }
            }
            tb.insert_void_element(tag);
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        // Legacy alias for img — a parse error.
        "image" => Err(parse_error("unexpected-image-start-tag")),
        "textarea" => {
            tb.parse_generic_rcdata(tag)?;
            // A leading newline is dropped (§13.2.6.4.7).
            tb.state.skip_next_lf = true;
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        "xmp" => {
            close_open_p_in_button_scope(tb)?;
            tb.state.frameset_ok = false;
            tb.parse_generic_rawtext(tag)?;
            Ok(Flow::Next)
        }
        "iframe" => {
            tb.state.frameset_ok = false;
            tb.parse_generic_rawtext(tag)?;
            Ok(Flow::Next)
        }
        "noembed" => {
            tb.parse_generic_rawtext(tag)?;
            Ok(Flow::Next)
        }
        // With scripting enabled, noscript content is raw text. With scripting
        // disabled it falls through to "any other start tag" (an ordinary
        // element).
        "noscript" if tb.state.scripting => {
            tb.parse_generic_rawtext(tag)?;
            Ok(Flow::Next)
        }
        "select" => {
            if tb.has_tag_in_scope("select", Scope::Default) {
                return Err(parse_error("nested-select"));
            }
            tb.insert_html_element(tag)?;
            tb.state.frameset_ok = false;
            Ok(Flow::Next)
        }
        "option" => start_option(tb, tag),
        "optgroup" => start_optgroup(tb, tag),
        "rb" | "rtc" => start_rb_rtc(tb, tag),
        "rp" | "rt" => start_rp_rt(tb, tag),
        // Table-structure and head start tags are out of place in body.
        "caption" | "col" | "colgroup" | "frame" | "head" | "tbody" | "td" | "tfoot" | "th"
        | "thead" | "tr" => Err(parse_error("misplaced-table-or-head-start-tag")),
        // The entry points into foreign content (§13.2.6.4.7): insert the
        // root foreign element, then the §13.2.6 dispatcher takes over.
        "math" => Ok(start_foreign_root(tb, tag, Namespace::MathMl)),
        "svg" => Ok(start_foreign_root(tb, tag, Namespace::Svg)),
        // Any other start tag — unknown / custom elements — is an ordinary
        // HTML element.
        _ => {
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
    }
}

/// §13.2.6.4.7 `<math>` / `<svg>` start tags — the entry into foreign content.
/// Runs the full sequence: reconstruct the active formatting elements (a
/// strict no-op — the list is not maintained), then insert the foreign root in
/// `namespace` via the shared §13.2.6.1 "insert a foreign element" step
/// ([`super::foreign::insert_foreign_start_tag`]), which adjusts the
/// attributes and honours a self-closing flag. Infallible: a foreign start tag
/// (unlike a non-void HTML one) never rejects a self-closing flag.
fn start_foreign_root(tb: &mut TreeBuilder, tag: &TagToken, namespace: Namespace) -> Flow {
    super::foreign::insert_foreign_start_tag(tb, tag, namespace);
    Flow::Next
}

/// Dispatch an "in body" end tag.
//
// The formatting end-tag arm and the "any other end tag" arm share a body:
// the conforming adoption agency degenerates to the same `any_other_end_tag`
// walk. The explicit arm is kept for 1:1 spec traceability (§13.2.6.4.7).
#[allow(clippy::match_same_arms)]
fn end_tag(tb: &mut TreeBuilder, token: &Token, tag: &TagToken) -> Result<Flow, StrictParseError> {
    match tag.name.as_str() {
        "template" => super::in_head::in_head(tb, token),
        "body" => end_body(tb, false),
        "html" => end_body(tb, true),
        "p" => {
            if !tb.has_tag_in_scope("p", Scope::Button) {
                return Err(parse_error("unexpected-end-p-no-open-p"));
            }
            close_p(tb)?;
            Ok(Flow::Next)
        }
        "li" => {
            if !tb.has_tag_in_scope("li", Scope::ListItem) {
                return Err(parse_error("unexpected-end-li"));
            }
            tb.generate_implied_end_tags_except("li");
            if !tb.current_node_has_tag("li") {
                return Err(parse_error("misnested-end-li"));
            }
            tb.pop_until_tag("li");
            Ok(Flow::Next)
        }
        "dd" | "dt" => {
            let name = tag.name.as_str();
            if !tb.has_tag_in_scope(name, Scope::Default) {
                return Err(parse_error("unexpected-end-dd-dt"));
            }
            tb.generate_implied_end_tags_except(name);
            if !tb.current_node_has_tag(name) {
                return Err(parse_error("misnested-end-dd-dt"));
            }
            tb.pop_until_tag(name);
            Ok(Flow::Next)
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];
            if !tb.has_any_tag_in_scope(HEADINGS, Scope::Default) {
                return Err(parse_error("unexpected-end-heading"));
            }
            tb.generate_implied_end_tags();
            if !tb.current_node_has_tag(tag.name.as_str()) {
                return Err(parse_error("misnested-end-heading"));
            }
            tb.pop_until_any(HEADINGS);
            Ok(Flow::Next)
        }
        "form" => end_form(tb),
        "address" | "article" | "aside" | "blockquote" | "button" | "center" | "details"
        | "dialog" | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure" | "footer"
        | "header" | "hgroup" | "listing" | "main" | "menu" | "nav" | "ol" | "pre" | "search"
        | "section" | "select" | "summary" | "ul" => {
            close_element_in_scope(tb, tag.name.as_str())?;
            Ok(Flow::Next)
        }
        "applet" | "marquee" | "object" => {
            close_element_in_scope(tb, tag.name.as_str())?;
            // "Clear the list of active formatting elements up to the last
            // marker" — no-op (no list).
            Ok(Flow::Next)
        }
        // Formatting end tags: the conforming adoption agency degenerates to
        // the "any other end tag" walk (a plain pop when properly nested).
        "a" | "b" | "big" | "code" | "em" | "font" | "i" | "nobr" | "s" | "small" | "strike"
        | "strong" | "tt" | "u" => {
            any_other_end_tag(tb, tag.name.as_str())?;
            Ok(Flow::Next)
        }
        "br" => Err(parse_error("unexpected-end-br")),
        _ => {
            any_other_end_tag(tb, tag.name.as_str())?;
            Ok(Flow::Next)
        }
    }
}

// ----- shared end-tag / start-tag helpers -----

/// Close an open `p` element in button scope, if there is one
/// (the "if … has a p element in button scope, then close a p element" guard
/// many block start tags share).
fn close_open_p_in_button_scope(tb: &mut TreeBuilder) -> Result<(), StrictParseError> {
    if tb.has_tag_in_scope("p", Scope::Button) {
        close_p(tb)?;
    }
    Ok(())
}

/// §13.2.6.4.7 "close a p element".
fn close_p(tb: &mut TreeBuilder) -> Result<(), StrictParseError> {
    tb.generate_implied_end_tags_except("p");
    if !tb.current_node_has_tag("p") {
        return Err(parse_error("misnested-p"));
    }
    tb.pop_until_tag("p");
    Ok(())
}

/// The block-element end-tag algorithm (default scope): close the named
/// element, rejecting a missing-open-element or misnested close.
fn close_element_in_scope(tb: &mut TreeBuilder, tag: &str) -> Result<(), StrictParseError> {
    if !tb.has_tag_in_scope(tag, Scope::Default) {
        return Err(parse_error("end-tag-without-open-element-in-scope"));
    }
    tb.generate_implied_end_tags();
    if !tb.current_node_has_tag(tag) {
        return Err(parse_error("misnested-end-tag"));
    }
    tb.pop_until_tag(tag);
    Ok(())
}

/// §13.2.6.4.7 "any other end tag" (also the conforming path for formatting
/// end tags). Walks the stack for a tag-name match; a match that is not the
/// current node after generating implied end tags, or a special element
/// reached before any match, is non-conforming and rejected.
fn any_other_end_tag(tb: &mut TreeBuilder, tag: &str) -> Result<(), StrictParseError> {
    for idx in (0..tb.state.open_elements.len()).rev() {
        let node = tb.state.open_elements[idx];
        if tb.entity_has_tag(node, tag) {
            tb.generate_implied_end_tags_except(tag);
            if tb.state.current_node() != Some(node) {
                return Err(parse_error("misnested-end-tag"));
            }
            tb.pop_until_entity(node);
            return Ok(());
        }
        if tb.is_special(node) {
            return Err(parse_error("unexpected-end-tag"));
        }
    }
    Err(parse_error("unexpected-end-tag-no-match"))
}

/// `</body>` (and, with `reprocess`, `</html>`): switch to "after body",
/// rejecting if there is no body in scope or an unexpected element is still
/// open.
fn end_body(tb: &mut TreeBuilder, reprocess: bool) -> Result<Flow, StrictParseError> {
    if !tb.has_tag_in_scope("body", Scope::Default) {
        return Err(parse_error("unexpected-end-body-no-body-in-scope"));
    }
    if has_unexpected_open_element(tb) {
        return Err(parse_error("unclosed-element-before-body-close"));
    }
    tb.state.mode = InsertionMode::AfterBody;
    Ok(if reprocess {
        Flow::Reprocess
    } else {
        Flow::Next
    })
}

/// `</form>` (§13.2.6.4.7).
fn end_form(tb: &mut TreeBuilder) -> Result<Flow, StrictParseError> {
    if tb.has_template_on_stack() {
        if !tb.has_tag_in_scope("form", Scope::Default) {
            return Err(parse_error("unexpected-end-form-no-form-in-scope"));
        }
        tb.generate_implied_end_tags();
        if !tb.current_node_has_tag("form") {
            return Err(parse_error("misnested-end-form"));
        }
        tb.pop_until_tag("form");
        return Ok(Flow::Next);
    }
    let node = tb.state.form_pointer.take();
    let Some(node) = node else {
        return Err(parse_error("unexpected-end-form-no-form-pointer"));
    };
    if !tb.has_entity_in_scope(node, Scope::Default) {
        return Err(parse_error("unexpected-end-form-not-in-scope"));
    }
    tb.generate_implied_end_tags();
    if tb.state.current_node() != Some(node) {
        return Err(parse_error("misnested-end-form"));
    }
    tb.remove_from_open_elements(node);
    Ok(Flow::Next)
}

/// `<li>` start tag (§13.2.6.4.7).
fn start_li(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    insert_list_item_like(tb, tag, &["li"], "misnested-li")
}

/// `<dd>` / `<dt>` start tag (§13.2.6.4.7).
fn start_dd_dt(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    insert_list_item_like(tb, tag, &["dd", "dt"], "misnested-dd-dt")
}

/// The shared §13.2.6.4.7 `<li>` / `<dd>` / `<dt>` stepping algorithm: set
/// frameset-ok to not ok, reverse-walk the stack closing the nearest matching
/// list-item element (generate implied end tags except it, reject a misnested
/// close, pop to it) but stop at a special element that is not
/// `address`/`div`/`p`; then close an open p in button scope and insert.
fn insert_list_item_like(
    tb: &mut TreeBuilder,
    tag: &TagToken,
    scan_tags: &[&str],
    misnest_err: &str,
) -> Result<Flow, StrictParseError> {
    tb.state.frameset_ok = false;
    for idx in (0..tb.state.open_elements.len()).rev() {
        let node = tb.state.open_elements[idx];
        if let Some(close_tag) = scan_tags
            .iter()
            .copied()
            .find(|&t| tb.entity_has_tag(node, t))
        {
            tb.generate_implied_end_tags_except(close_tag);
            if !tb.current_node_has_tag(close_tag) {
                return Err(parse_error(misnest_err));
            }
            tb.pop_until_tag(close_tag);
            break;
        }
        if tb.is_special(node) && !tb.entity_has_any_tag(node, &["address", "div", "p"]) {
            break;
        }
    }
    close_open_p_in_button_scope(tb)?;
    tb.insert_html_element(tag)?;
    Ok(Flow::Next)
}

/// `<form>` start tag (§13.2.6.4.7).
fn start_form(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    // Whether a template is open is invariant across the work below (closing a
    // p and inserting the form neither push nor pop a template), so check once.
    let in_template = tb.has_template_on_stack();
    if tb.state.form_pointer.is_some() && !in_template {
        return Err(parse_error("misnested-form"));
    }
    close_open_p_in_button_scope(tb)?;
    let form = tb.insert_html_element(tag)?;
    if !in_template {
        tb.state.form_pointer = Some(form);
    }
    Ok(Flow::Next)
}

/// `<option>` start tag (§13.2.6.4.7).
fn start_option(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    if tb.has_tag_in_scope("select", Scope::Default) {
        tb.generate_implied_end_tags_except("optgroup");
        if tb.has_tag_in_scope("option", Scope::Default) {
            return Err(parse_error("misnested-option"));
        }
    } else if tb.current_node_has_tag("option") {
        tb.pop();
    }
    tb.insert_html_element(tag)?;
    Ok(Flow::Next)
}

/// `<optgroup>` start tag (§13.2.6.4.7).
fn start_optgroup(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    if tb.has_tag_in_scope("select", Scope::Default) {
        tb.generate_implied_end_tags();
        if tb.has_tag_in_scope("option", Scope::Default)
            || tb.has_tag_in_scope("optgroup", Scope::Default)
        {
            return Err(parse_error("misnested-optgroup"));
        }
    } else if tb.current_node_has_tag("option") {
        tb.pop();
    }
    tb.insert_html_element(tag)?;
    Ok(Flow::Next)
}

/// `<rb>` / `<rtc>` start tag (§13.2.6.4.7).
fn start_rb_rtc(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    if tb.has_tag_in_scope("ruby", Scope::Default) {
        tb.generate_implied_end_tags();
        if !tb.current_node_has_tag("ruby") {
            return Err(parse_error("misnested-ruby"));
        }
    }
    tb.insert_html_element(tag)?;
    Ok(Flow::Next)
}

/// `<rp>` / `<rt>` start tag (§13.2.6.4.7).
fn start_rp_rt(tb: &mut TreeBuilder, tag: &TagToken) -> Result<Flow, StrictParseError> {
    if tb.has_tag_in_scope("ruby", Scope::Default) {
        tb.generate_implied_end_tags_except("rtc");
        if !tb.current_node_has_any_tag(&["rtc", "ruby"]) {
            return Err(parse_error("misnested-ruby"));
        }
    }
    tb.insert_html_element(tag)?;
    Ok(Flow::Next)
}

/// Elements that may remain open at EOF / `</body>` without being a parse
/// error (§13.2.6.4.7 EOF and `</body>` "if there is a node … that is not …").
const EOF_CLOSEABLE_TAGS: &[&str] = &[
    "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc", "tbody", "td", "tfoot",
    "th", "thead", "tr", "body", "html",
];

/// Whether the stack of open elements contains an element that is not
/// implicitly closeable at EOF / `</body>`.
fn has_unexpected_open_element(tb: &TreeBuilder) -> bool {
    tb.state.open_elements.iter().any(|&entity| {
        tb.dom.with_tag_name(entity, |tag| match tag {
            Some(name) => !EOF_CLOSEABLE_TAGS.contains(&name),
            None => true,
        })
    })
}

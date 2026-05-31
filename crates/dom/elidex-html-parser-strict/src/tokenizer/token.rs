//! Token types emitted by the strict HTML tokenizer.
//!
//! Per WHATWG HTML §13.2.5 "Tokenization", the tokenizer emits a stream
//! of tokens consumed by the tree builder. The tokenizer itself is
//! `EcsDom`-unreachable — it produces only these value types and never
//! touches the DOM (see the crate Layering mandate).
//!
//! Spec: <https://html.spec.whatwg.org/multipage/parsing.html#tokenization>

/// A single token produced by the tokenizer.
///
/// Mirrors the token kinds enumerated in WHATWG HTML §13.2.5: DOCTYPE,
/// start/end tags, comments, characters, and end-of-file. Strict mode
/// never emits the malformed-token recovery shapes the spec defines for
/// error handling — those paths abort with
/// [`crate::StrictParseError`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// A `<!DOCTYPE …>` token (§13.2.5.53 onward).
    Doctype(DoctypeToken),
    /// A `<tag …>` start tag (§13.2.5.6 Tag open state onward).
    StartTag(TagToken),
    /// A `</tag>` end tag (§13.2.5.7 End tag open state onward).
    EndTag(TagToken),
    /// A comment token (`<!-- … -->`, §13.2.5.43 onward).
    Comment(String),
    /// A single character token (§13.2.5.1 Data state etc.).
    Character(char),
    /// The end-of-file token, emitted once when input is exhausted.
    EndOfFile,
}

/// A start- or end-tag token.
///
/// Spec: WHATWG HTML §13.2.5.8 "Tag name state" and the attribute states
/// §13.2.5.32–39. Attribute names are unique by construction: the strict
/// tokenizer rejects a duplicate attribute (`duplicate-attribute`
/// parse-error, §13.2.5.33) rather than silently dropping it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagToken {
    /// Tag name, ASCII-lowercased per §13.2.5.8.
    pub(crate) name: String,
    /// Attribute (name, value) pairs in source order, names unique.
    pub(crate) attrs: Vec<(String, String)>,
    /// Whether the tag carried a `/` self-closing flag (§13.2.5.40).
    pub(crate) self_closing: bool,
}

/// A DOCTYPE token.
///
/// Spec: WHATWG HTML §13.2.5.53 "DOCTYPE state" through §13.2.5.68.
/// `force_quirks` records the spec's "force-quirks flag"; in strict mode
/// it is only ever set via the EOF/parse-error branches, which abort, so
/// a well-formed `<!DOCTYPE html>` always yields `force_quirks: false`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DoctypeToken {
    /// DOCTYPE name (e.g. `"html"`), ASCII-lowercased per §13.2.5.55.
    pub(crate) name: Option<String>,
    /// Public identifier (§13.2.5.59/60), present only with `PUBLIC`.
    pub(crate) public_id: Option<String>,
    /// System identifier (§13.2.5.65/66), present with `SYSTEM`/`PUBLIC`.
    pub(crate) system_id: Option<String>,
    /// The spec "force-quirks flag" (§13.2.5.53).
    pub(crate) force_quirks: bool,
}

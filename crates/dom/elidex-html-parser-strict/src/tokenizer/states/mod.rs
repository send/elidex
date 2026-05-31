//! The strict HTML tokenizer state machine.
//!
//! Implements the WHATWG HTML §13.2.5 "Tokenization" states reachable
//! under strict semantics (§13.2.5.1 Data state through §13.2.5.80
//! Numeric character reference end state). The two bogus *recovery*
//! states — §13.2.5.41 bogus comment and §13.2.5.68 bogus DOCTYPE — are
//! omitted: strict mode rejects their entry conditions
//! (`incorrectly-opened-comment`, the invalid-DOCTYPE sequences) rather
//! than recovering, so they are unreachable by construction. Per-state
//! handlers live in spec-family submodules ([`data`], [`tag`],
//! [`attribute`], [`comment`], [`doctype`], [`cdata`]); the
//! character-reference family (§13.2.5.72–80) lives in
//! [`super::char_ref`] alongside the named-entity table.
//!
//! # Strict semantics
//!
//! This tokenizer has **no error recovery**. Every WHATWG HTML §13.2.2
//! parse-error condition aborts immediately with
//! [`crate::StrictParseError`] (contrast: the tolerant html5ever path
//! silently recovers). The spec's recovery shapes — U+FFFD replacement
//! for U+0000, bogus-comment fallback, implicit tag closing — are
//! therefore unreachable in valid HTML5 and rejected when encountered.
//!
//! # Layering
//!
//! The tokenizer is `EcsDom`-unreachable: it produces only
//! [`super::token::Token`] values. The tree builder drives raw-text
//! state transitions via [`Tokenizer::set_state`] (e.g. switching to
//! RCDATA on `<title>`), the only seam between the two layers; the
//! appropriate-end-tag name is tracked automatically as start tags are
//! emitted, so no explicit last-start-tag plumbing crosses the seam.

use crate::tokenizer::token::{DoctypeToken, TagToken, Token};
use crate::StrictParseError;
use std::collections::VecDeque;

mod attribute;
mod cdata;
mod comment;
mod data;
mod doctype;
mod tag;

/// The tokenizer state, one variant per WHATWG HTML §13.2.5 state.
///
/// Variant order follows the spec section order (§13.2.5.1–§13.2.5.80).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    // §13.2.5.1–5 — text content states
    /// §13.2.5.1 Data state.
    Data,
    /// §13.2.5.2 RCDATA state.
    Rcdata,
    /// §13.2.5.3 RAWTEXT state.
    Rawtext,
    /// §13.2.5.4 Script data state.
    ScriptData,
    /// §13.2.5.5 PLAINTEXT state.
    Plaintext,
    // §13.2.5.6–8 — tag opening / name
    /// §13.2.5.6 Tag open state.
    TagOpen,
    /// §13.2.5.7 End tag open state.
    EndTagOpen,
    /// §13.2.5.8 Tag name state.
    TagName,
    // §13.2.5.9–11 — RCDATA less-than / end-tag
    /// §13.2.5.9 RCDATA less-than sign state.
    RcdataLessThanSign,
    /// §13.2.5.10 RCDATA end tag open state.
    RcdataEndTagOpen,
    /// §13.2.5.11 RCDATA end tag name state.
    RcdataEndTagName,
    // §13.2.5.12–14 — RAWTEXT less-than / end-tag
    /// §13.2.5.12 RAWTEXT less-than sign state.
    RawtextLessThanSign,
    /// §13.2.5.13 RAWTEXT end tag open state.
    RawtextEndTagOpen,
    /// §13.2.5.14 RAWTEXT end tag name state.
    RawtextEndTagName,
    // §13.2.5.15–31 — script data less-than / escape families
    /// §13.2.5.15 Script data less-than sign state.
    ScriptDataLessThanSign,
    /// §13.2.5.16 Script data end tag open state.
    ScriptDataEndTagOpen,
    /// §13.2.5.17 Script data end tag name state.
    ScriptDataEndTagName,
    /// §13.2.5.18 Script data escape start state.
    ScriptDataEscapeStart,
    /// §13.2.5.19 Script data escape start dash state.
    ScriptDataEscapeStartDash,
    /// §13.2.5.20 Script data escaped state.
    ScriptDataEscaped,
    /// §13.2.5.21 Script data escaped dash state.
    ScriptDataEscapedDash,
    /// §13.2.5.22 Script data escaped dash dash state.
    ScriptDataEscapedDashDash,
    /// §13.2.5.23 Script data escaped less-than sign state.
    ScriptDataEscapedLessThanSign,
    /// §13.2.5.24 Script data escaped end tag open state.
    ScriptDataEscapedEndTagOpen,
    /// §13.2.5.25 Script data escaped end tag name state.
    ScriptDataEscapedEndTagName,
    /// §13.2.5.26 Script data double escape start state.
    ScriptDataDoubleEscapeStart,
    /// §13.2.5.27 Script data double escaped state.
    ScriptDataDoubleEscaped,
    /// §13.2.5.28 Script data double escaped dash state.
    ScriptDataDoubleEscapedDash,
    /// §13.2.5.29 Script data double escaped dash dash state.
    ScriptDataDoubleEscapedDashDash,
    /// §13.2.5.30 Script data double escaped less-than sign state.
    ScriptDataDoubleEscapedLessThanSign,
    /// §13.2.5.31 Script data double escape end state.
    ScriptDataDoubleEscapeEnd,
    // §13.2.5.32–39 — attribute states
    /// §13.2.5.32 Before attribute name state.
    BeforeAttributeName,
    /// §13.2.5.33 Attribute name state.
    AttributeName,
    /// §13.2.5.34 After attribute name state.
    AfterAttributeName,
    /// §13.2.5.35 Before attribute value state.
    BeforeAttributeValue,
    /// §13.2.5.36 Attribute value (double-quoted) state.
    AttributeValueDoubleQuoted,
    /// §13.2.5.37 Attribute value (single-quoted) state.
    AttributeValueSingleQuoted,
    /// §13.2.5.38 Attribute value (unquoted) state.
    AttributeValueUnquoted,
    /// §13.2.5.39 After attribute value (quoted) state.
    AfterAttributeValueQuoted,
    // §13.2.5.40, 42 — self-closing / markup decl (§13.2.5.41 bogus
    // comment omitted: strict rejects its entry instead of recovering)
    /// §13.2.5.40 Self-closing start tag state.
    SelfClosingStartTag,
    /// §13.2.5.42 Markup declaration open state.
    MarkupDeclarationOpen,
    // §13.2.5.43–52 — comment states
    /// §13.2.5.43 Comment start state.
    CommentStart,
    /// §13.2.5.44 Comment start dash state.
    CommentStartDash,
    /// §13.2.5.45 Comment state.
    Comment,
    /// §13.2.5.46 Comment less-than sign state.
    CommentLessThanSign,
    /// §13.2.5.47 Comment less-than sign bang state.
    CommentLessThanSignBang,
    /// §13.2.5.48 Comment less-than sign bang dash state.
    CommentLessThanSignBangDash,
    /// §13.2.5.49 Comment less-than sign bang dash dash state.
    CommentLessThanSignBangDashDash,
    /// §13.2.5.50 Comment end dash state.
    CommentEndDash,
    /// §13.2.5.51 Comment end state.
    CommentEnd,
    /// §13.2.5.52 Comment end bang state.
    CommentEndBang,
    // §13.2.5.53–67 — DOCTYPE states (§13.2.5.68 bogus DOCTYPE omitted:
    // strict rejects its entry instead of recovering)
    /// §13.2.5.53 DOCTYPE state.
    Doctype,
    /// §13.2.5.54 Before DOCTYPE name state.
    BeforeDoctypeName,
    /// §13.2.5.55 DOCTYPE name state.
    DoctypeName,
    /// §13.2.5.56 After DOCTYPE name state.
    AfterDoctypeName,
    /// §13.2.5.57 After DOCTYPE public keyword state.
    AfterDoctypePublicKeyword,
    /// §13.2.5.58 Before DOCTYPE public identifier state.
    BeforeDoctypePublicIdentifier,
    /// §13.2.5.59 DOCTYPE public identifier (double-quoted) state.
    DoctypePublicIdentifierDoubleQuoted,
    /// §13.2.5.60 DOCTYPE public identifier (single-quoted) state.
    DoctypePublicIdentifierSingleQuoted,
    /// §13.2.5.61 After DOCTYPE public identifier state.
    AfterDoctypePublicIdentifier,
    /// §13.2.5.62 Between DOCTYPE public and system identifiers state.
    BetweenDoctypePublicAndSystemIdentifiers,
    /// §13.2.5.63 After DOCTYPE system keyword state.
    AfterDoctypeSystemKeyword,
    /// §13.2.5.64 Before DOCTYPE system identifier state.
    BeforeDoctypeSystemIdentifier,
    /// §13.2.5.65 DOCTYPE system identifier (double-quoted) state.
    DoctypeSystemIdentifierDoubleQuoted,
    /// §13.2.5.66 DOCTYPE system identifier (single-quoted) state.
    DoctypeSystemIdentifierSingleQuoted,
    /// §13.2.5.67 After DOCTYPE system identifier state.
    AfterDoctypeSystemIdentifier,
    // §13.2.5.69–71 — CDATA section states
    /// §13.2.5.69 CDATA section state.
    CdataSection,
    /// §13.2.5.70 CDATA section bracket state.
    CdataSectionBracket,
    /// §13.2.5.71 CDATA section end state.
    CdataSectionEnd,
    // §13.2.5.72–80 — character reference states
    /// §13.2.5.72 Character reference state.
    CharacterReference,
    /// §13.2.5.73 Named character reference state.
    NamedCharacterReference,
    /// §13.2.5.74 Ambiguous ampersand state.
    AmbiguousAmpersand,
    /// §13.2.5.75 Numeric character reference state.
    NumericCharacterReference,
    /// §13.2.5.76 Hexadecimal character reference start state.
    HexadecimalCharacterReferenceStart,
    /// §13.2.5.77 Decimal character reference start state.
    DecimalCharacterReferenceStart,
    /// §13.2.5.78 Hexadecimal character reference state.
    HexadecimalCharacterReference,
    /// §13.2.5.79 Decimal character reference state.
    DecimalCharacterReference,
    /// §13.2.5.80 Numeric character reference end state.
    NumericCharacterReferenceEnd,
}

/// The strict HTML tokenizer.
///
/// Construct with [`Tokenizer::new`], then pull tokens with
/// [`Tokenizer::next_token`] until [`Token::EndOfFile`]. Any parse error
/// aborts with [`StrictParseError`].
// The spec state machine carries several independent boolean flags
// (self-closing, end-tag, attribute-in-progress, EOF emitted); they model
// distinct §13.2.5 conditions and do not collapse into an enum.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Tokenizer {
    /// Preprocessed input (newlines normalized per §13.2.3.5).
    input: Vec<char>,
    /// Index of the next character to consume.
    pos: usize,
    /// Current tokenizer state.
    state: State,
    /// Return state for the character-reference states (§13.2.5.72).
    return_state: State,
    /// Pending output tokens (some states emit multiple characters).
    output: VecDeque<Token>,
    /// Whether the end-of-file token has been emitted.
    eof_emitted: bool,

    // --- current tag token under construction ---
    /// Name of the tag being built (ASCII-lowercased).
    tag_name: String,
    /// Attributes accumulated for the current tag.
    tag_attrs: Vec<(String, String)>,
    /// Self-closing flag for the current tag.
    tag_self_closing: bool,
    /// Whether the current tag is an end tag (`</…>`).
    tag_is_end: bool,
    /// Name of the attribute currently being built.
    cur_attr_name: String,
    /// Value of the attribute currently being built.
    cur_attr_value: String,
    /// Whether an attribute is currently being accumulated.
    building_attr: bool,

    // --- current comment / doctype under construction ---
    /// Data of the comment being built.
    comment: String,
    /// The DOCTYPE token being built.
    doctype: DoctypeToken,

    // --- shared scratch ---
    /// Temporary buffer (script-data escapes, character references).
    temp_buffer: String,
    /// Accumulated numeric character reference code (§13.2.5.78/79).
    char_ref_code: u32,
    /// Name of the last start tag emitted (appropriate-end-tag check).
    last_start_tag: Option<String>,
    /// First §13.2.3.5 input-stream parse error seen while consuming
    /// (control-character / noncharacter), recorded for strict reject.
    pending_input_error: Option<&'static str>,
    /// The terminal parse error, set the moment tokenization aborts. Once
    /// set, every [`Tokenizer::next_token`] call re-returns it and never
    /// yields another token (strict abort is permanent by construction).
    error: Option<StrictParseError>,
}

impl Tokenizer {
    /// Create a tokenizer over `html`, applying the §13.2.3.5
    /// "Preprocessing the input stream" newline normalization (CRLF and
    /// lone CR both collapse to LF). The tokenizer starts in the Data
    /// state; the tree builder may switch it via [`Tokenizer::set_state`].
    pub(crate) fn new(html: &str) -> Self {
        Tokenizer {
            input: preprocess(html),
            pos: 0,
            state: State::Data,
            return_state: State::Data,
            output: VecDeque::new(),
            eof_emitted: false,
            tag_name: String::new(),
            tag_attrs: Vec::new(),
            tag_self_closing: false,
            tag_is_end: false,
            cur_attr_name: String::new(),
            cur_attr_value: String::new(),
            building_attr: false,
            comment: String::new(),
            doctype: DoctypeToken::default(),
            temp_buffer: String::new(),
            char_ref_code: 0,
            last_start_tag: None,
            pending_input_error: None,
            error: None,
        }
    }

    /// Force the tokenizer into `state`. Used by the tree builder (A3) to
    /// enter RCDATA / RAWTEXT / script-data / PLAINTEXT per §13.2.5, and
    /// by tests to inject html5lib `initialStates`.
    pub(crate) fn set_state(&mut self, state: State) {
        self.state = state;
    }

    /// Record the last-start-tag name used by the appropriate-end-tag
    /// check in the RCDATA/RAWTEXT/script-data end-tag-name states
    /// (§13.2.5.11/14/17).
    ///
    /// Production never needs this: [`Self::next_token`] tracks the last
    /// start tag automatically as it emits one. It exists only so tests
    /// can mirror the html5lib `lastStartTag` setup parameter, hence the
    /// `#[cfg(test)]` gate.
    #[cfg(test)]
    pub(crate) fn set_last_start_tag(&mut self, name: &str) {
        self.last_start_tag = Some(name.to_string());
    }

    /// Pull the next token, running the state machine until one is
    /// produced.
    ///
    /// Aborts with [`StrictParseError`] on the first parse error — and in
    /// strict mode reaching EOF in most states (inside a tag, comment,
    /// DOCTYPE, character reference, etc.) *is* a parse error, so it
    /// returns `Err` rather than [`Token::EndOfFile`]. EOF is emitted as
    /// [`Token::EndOfFile`] only when it is reached in a non-error context
    /// (e.g. the Data state); once that has happened the tokenizer is
    /// terminal and every subsequent call returns [`Token::EndOfFile`]
    /// again.
    pub(crate) fn next_token(&mut self) -> Result<Token, StrictParseError> {
        // A parse error is terminal: once one has aborted tokenization,
        // re-return it and never yield another token, so "abort on the
        // first parse error" holds even if a caller keeps iterating.
        if let Some(err) = &self.error {
            return Err(err.clone());
        }
        loop {
            if let Some(t) = self.output.pop_front() {
                return Ok(t);
            }
            if self.eof_emitted {
                return Ok(Token::EndOfFile);
            }
            let step_result = self.step();
            // §13.2.3.5: a control / noncharacter in the input stream is a
            // parse error raised as the character enters the stream — i.e.
            // *before* any state-level error raised while processing that
            // same character — so it takes priority over `step_result`'s
            // error. Strict rejects at the point it is consumed.
            if let Some(name) = self.pending_input_error.take() {
                let err = self.parse_error(name);
                return Err(self.abort(err));
            }
            if let Err(err) = step_result {
                return Err(self.abort(err));
            }
        }
    }

    /// Record `err` as the terminal parse error and discard any tokens
    /// already queued this step, so a post-error token can never leak to a
    /// caller that keeps iterating past the abort. Returns `err` for
    /// propagation.
    fn abort(&mut self, err: StrictParseError) -> StrictParseError {
        self.output.clear();
        self.error = Some(err.clone());
        err
    }

    /// Dispatch one state transition to its handler.
    fn step(&mut self) -> Result<(), StrictParseError> {
        match self.state {
            State::Data => self.data_state(),
            State::Rcdata => self.rcdata_state(),
            State::Rawtext => self.rawtext_state(),
            State::ScriptData => self.script_data_state(),
            State::Plaintext => self.plaintext_state(),
            State::TagOpen => self.tag_open_state(),
            State::EndTagOpen => self.end_tag_open_state(),
            State::TagName => self.tag_name_state(),
            State::RcdataLessThanSign => self.rcdata_less_than_sign_state(),
            State::RcdataEndTagOpen => self.rcdata_end_tag_open_state(),
            State::RcdataEndTagName => self.rcdata_end_tag_name_state(),
            State::RawtextLessThanSign => self.rawtext_less_than_sign_state(),
            State::RawtextEndTagOpen => self.rawtext_end_tag_open_state(),
            State::RawtextEndTagName => self.rawtext_end_tag_name_state(),
            State::ScriptDataLessThanSign => self.script_data_less_than_sign_state(),
            State::ScriptDataEndTagOpen => self.script_data_end_tag_open_state(),
            State::ScriptDataEndTagName => self.script_data_end_tag_name_state(),
            State::ScriptDataEscapeStart => self.script_data_escape_start_state(),
            State::ScriptDataEscapeStartDash => self.script_data_escape_start_dash_state(),
            State::ScriptDataEscaped => self.script_data_escaped_state(),
            State::ScriptDataEscapedDash => self.script_data_escaped_dash_state(),
            State::ScriptDataEscapedDashDash => self.script_data_escaped_dash_dash_state(),
            State::ScriptDataEscapedLessThanSign => self.script_data_escaped_less_than_sign_state(),
            State::ScriptDataEscapedEndTagOpen => self.script_data_escaped_end_tag_open_state(),
            State::ScriptDataEscapedEndTagName => self.script_data_escaped_end_tag_name_state(),
            State::ScriptDataDoubleEscapeStart => self.script_data_double_escape_start_state(),
            State::ScriptDataDoubleEscaped => self.script_data_double_escaped_state(),
            State::ScriptDataDoubleEscapedDash => self.script_data_double_escaped_dash_state(),
            State::ScriptDataDoubleEscapedDashDash => {
                self.script_data_double_escaped_dash_dash_state()
            }
            State::ScriptDataDoubleEscapedLessThanSign => {
                self.script_data_double_escaped_less_than_sign_state()
            }
            State::ScriptDataDoubleEscapeEnd => self.script_data_double_escape_end_state(),
            State::BeforeAttributeName => self.before_attribute_name_state(),
            State::AttributeName => self.attribute_name_state(),
            State::AfterAttributeName => self.after_attribute_name_state(),
            State::BeforeAttributeValue => self.before_attribute_value_state(),
            State::AttributeValueDoubleQuoted => self.attribute_value_double_quoted_state(),
            State::AttributeValueSingleQuoted => self.attribute_value_single_quoted_state(),
            State::AttributeValueUnquoted => self.attribute_value_unquoted_state(),
            State::AfterAttributeValueQuoted => self.after_attribute_value_quoted_state(),
            State::SelfClosingStartTag => self.self_closing_start_tag_state(),
            State::MarkupDeclarationOpen => self.markup_declaration_open_state(),
            State::CommentStart => self.comment_start_state(),
            State::CommentStartDash => self.comment_start_dash_state(),
            State::Comment => self.comment_state(),
            State::CommentLessThanSign => self.comment_less_than_sign_state(),
            State::CommentLessThanSignBang => self.comment_less_than_sign_bang_state(),
            State::CommentLessThanSignBangDash => self.comment_less_than_sign_bang_dash_state(),
            State::CommentLessThanSignBangDashDash => {
                self.comment_less_than_sign_bang_dash_dash_state()
            }
            State::CommentEndDash => self.comment_end_dash_state(),
            State::CommentEnd => self.comment_end_state(),
            State::CommentEndBang => self.comment_end_bang_state(),
            State::Doctype => self.doctype_state(),
            State::BeforeDoctypeName => self.before_doctype_name_state(),
            State::DoctypeName => self.doctype_name_state(),
            State::AfterDoctypeName => self.after_doctype_name_state(),
            State::AfterDoctypePublicKeyword => self.after_doctype_public_keyword_state(),
            State::BeforeDoctypePublicIdentifier => self.before_doctype_public_identifier_state(),
            State::DoctypePublicIdentifierDoubleQuoted => {
                self.doctype_public_identifier_double_quoted_state()
            }
            State::DoctypePublicIdentifierSingleQuoted => {
                self.doctype_public_identifier_single_quoted_state()
            }
            State::AfterDoctypePublicIdentifier => self.after_doctype_public_identifier_state(),
            State::BetweenDoctypePublicAndSystemIdentifiers => {
                self.between_doctype_public_and_system_identifiers_state()
            }
            State::AfterDoctypeSystemKeyword => self.after_doctype_system_keyword_state(),
            State::BeforeDoctypeSystemIdentifier => self.before_doctype_system_identifier_state(),
            State::DoctypeSystemIdentifierDoubleQuoted => {
                self.doctype_system_identifier_double_quoted_state()
            }
            State::DoctypeSystemIdentifierSingleQuoted => {
                self.doctype_system_identifier_single_quoted_state()
            }
            State::AfterDoctypeSystemIdentifier => self.after_doctype_system_identifier_state(),
            State::CdataSection => self.cdata_section_state(),
            State::CdataSectionBracket => self.cdata_section_bracket_state(),
            State::CdataSectionEnd => self.cdata_section_end_state(),
            State::CharacterReference => self.character_reference_state(),
            State::NamedCharacterReference => self.named_character_reference_state(),
            State::AmbiguousAmpersand => self.ambiguous_ampersand_state(),
            State::NumericCharacterReference => self.numeric_character_reference_state(),
            State::HexadecimalCharacterReferenceStart => {
                self.hexadecimal_character_reference_start_state()
            }
            State::DecimalCharacterReferenceStart => self.decimal_character_reference_start_state(),
            State::HexadecimalCharacterReference => self.hexadecimal_character_reference_state(),
            State::DecimalCharacterReference => self.decimal_character_reference_state(),
            State::NumericCharacterReferenceEnd => self.numeric_character_reference_end_state(),
        }
    }

    // ----- input cursor helpers -----

    /// Consume the next input character (§13.2.5 "consume the next input
    /// character"). `None` signals EOF.
    ///
    /// The `consume()` cursor is clamped at the EOF sentinel
    /// `input.len() + 1`: once past the end, repeated consumes (e.g.
    /// reconsuming EOF across states) leave it there rather than growing
    /// without bound, so `reconsume_in` stays paired with a single
    /// position and EOF parse-error positions don't drift. (The
    /// lookahead-commit helper [`Self::advance`] is not clamped, but every
    /// caller gates it behind a successful [`Self::matches_ahead`] that
    /// proves the characters exist, so it never advances past the end.)
    pub(super) fn consume(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        if self.pos <= self.input.len() {
            self.pos += 1;
        }
        if self.pending_input_error.is_none() {
            if let Some(name) = c.and_then(input_stream_error) {
                self.pending_input_error = Some(name);
            }
        }
        c
    }

    /// Re-consume the character just consumed in the given state
    /// (§13.2.5 "reconsume in the … state").
    pub(super) fn reconsume_in(&mut self, state: State) {
        self.pos -= 1;
        self.state = state;
    }

    /// Switch state without reconsuming.
    pub(super) fn switch_to(&mut self, state: State) {
        self.state = state;
    }

    /// Peek the next `s.len()` characters from the cursor and compare
    /// ASCII-case-insensitively (or sensitively when `ci` is false) against
    /// `s` without consuming. Used by the markup-declaration-open lookahead
    /// (§13.2.5.42).
    pub(super) fn matches_ahead(&self, s: &str, ci: bool) -> bool {
        self.matches_ahead_at(self.pos, s, ci)
    }

    /// As [`Self::matches_ahead`] but starting from an explicit index, so a
    /// state can match a keyword that begins at an already-consumed
    /// character without rewinding the cursor (§13.2.5.56).
    pub(super) fn matches_ahead_at(&self, start: usize, s: &str, ci: bool) -> bool {
        for (offset, want) in s.chars().enumerate() {
            match self.input.get(start + offset) {
                Some(&got) => {
                    let eq = if ci {
                        got.eq_ignore_ascii_case(&want)
                    } else {
                        got == want
                    };
                    if !eq {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    /// Set the cursor to an absolute index (commit a
    /// [`Self::matches_ahead_at`] lookahead).
    pub(super) fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Advance the cursor by `n` characters (commit a [`Self::matches_ahead`]
    /// lookahead).
    pub(super) fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    // ----- emit helpers -----

    /// Queue a token for output.
    pub(super) fn emit(&mut self, token: Token) {
        self.output.push_back(token);
    }

    /// Queue a single character token.
    pub(super) fn emit_char(&mut self, c: char) {
        self.output.push_back(Token::Character(c));
    }

    /// Mark EOF and queue the end-of-file token.
    pub(super) fn emit_eof(&mut self) {
        self.eof_emitted = true;
        self.output.push_back(Token::EndOfFile);
    }

    /// Build a [`StrictParseError`] naming the WHATWG HTML §13.2.2 parse
    /// error and the index of the last consumed input character (§D-e:
    /// structured, minimal echo of user input).
    ///
    /// `self.pos` is the cursor (the *next* character to consume), so for
    /// the common consume-then-reject sites it sits one past the offending
    /// character; reporting `pos - 1` points at the character the error
    /// concerns and is consistent across sites. At EOF the cursor is the
    /// `len + 1` sentinel, so this reports `len` (just past the input).
    pub(super) fn parse_error(&self, name: &str) -> StrictParseError {
        let offset = self.pos.saturating_sub(1);
        StrictParseError {
            errors: vec![format!("{name} (input position {offset})")],
        }
    }

    // ----- tag construction helpers -----

    /// Begin a new start-tag token (§13.2.5.6).
    pub(super) fn new_start_tag(&mut self) {
        self.tag_is_end = false;
        self.tag_name.clear();
        self.tag_attrs.clear();
        self.tag_self_closing = false;
        self.building_attr = false;
        self.cur_attr_name.clear();
        self.cur_attr_value.clear();
    }

    /// Begin a new end-tag token (§13.2.5.7).
    pub(super) fn new_end_tag(&mut self) {
        self.new_start_tag();
        self.tag_is_end = true;
    }

    /// Append a character to the current tag name.
    pub(super) fn push_tag_name(&mut self, c: char) {
        self.tag_name.push(c);
    }

    /// Set the self-closing flag on the current tag (§13.2.5.40).
    pub(super) fn set_self_closing(&mut self) {
        self.tag_self_closing = true;
    }

    /// Begin a new attribute, finishing any in-progress one first.
    pub(super) fn start_attribute(&mut self) -> Result<(), StrictParseError> {
        self.finish_attribute()?;
        self.cur_attr_name.clear();
        self.cur_attr_value.clear();
        self.building_attr = true;
        Ok(())
    }

    /// Commit the in-progress attribute to the current tag, rejecting a
    /// `duplicate-attribute` parse error (§13.2.5.33).
    pub(super) fn finish_attribute(&mut self) -> Result<(), StrictParseError> {
        if self.building_attr {
            if self.tag_attrs.iter().any(|(n, _)| n == &self.cur_attr_name) {
                return Err(self.parse_error("duplicate-attribute"));
            }
            self.tag_attrs.push((
                std::mem::take(&mut self.cur_attr_name),
                std::mem::take(&mut self.cur_attr_value),
            ));
            self.building_attr = false;
        }
        Ok(())
    }

    /// Append to the current attribute name.
    pub(super) fn push_attr_name(&mut self, c: char) {
        self.cur_attr_name.push(c);
    }

    /// Append to the current attribute value.
    pub(super) fn push_attr_value(&mut self, c: char) {
        self.cur_attr_value.push(c);
    }

    /// Emit the current tag token (§13.2.5 "emit the current tag token"),
    /// committing the final attribute and validating end-tag invariants.
    pub(super) fn emit_current_tag(&mut self) -> Result<(), StrictParseError> {
        self.finish_attribute()?;
        let token = TagToken {
            name: std::mem::take(&mut self.tag_name),
            attrs: std::mem::take(&mut self.tag_attrs),
            self_closing: self.tag_self_closing,
        };
        if self.tag_is_end {
            // §13.2.5.7: end tags carrying attributes or a self-closing
            // flag are parse errors; strict mode rejects rather than
            // discarding them.
            if !token.attrs.is_empty() {
                return Err(self.parse_error("end-tag-with-attributes"));
            }
            if token.self_closing {
                return Err(self.parse_error("end-tag-with-trailing-solidus"));
            }
            self.emit(Token::EndTag(token));
        } else {
            self.last_start_tag = Some(token.name.clone());
            self.emit(Token::StartTag(token));
        }
        Ok(())
    }

    /// Whether the current end-tag token is an "appropriate end tag"
    /// (§13.2.5: its name matches the last start tag emitted).
    pub(super) fn is_appropriate_end_tag(&self) -> bool {
        match &self.last_start_tag {
            Some(name) => self.tag_is_end && &self.tag_name == name,
            None => false,
        }
    }

    // ----- comment / doctype construction helpers -----

    /// Begin a new, empty comment token.
    pub(super) fn new_comment(&mut self) {
        self.comment.clear();
    }

    /// Append a character to the current comment.
    pub(super) fn push_comment(&mut self, c: char) {
        self.comment.push(c);
    }

    /// Append a string to the current comment.
    pub(super) fn push_comment_str(&mut self, s: &str) {
        self.comment.push_str(s);
    }

    /// Emit the current comment token.
    pub(super) fn emit_comment(&mut self) {
        let data = std::mem::take(&mut self.comment);
        self.emit(Token::Comment(data));
    }

    /// Begin a new DOCTYPE token.
    pub(super) fn new_doctype(&mut self) {
        self.doctype = DoctypeToken::default();
    }

    /// Emit the current DOCTYPE token.
    pub(super) fn emit_doctype(&mut self) {
        let dt = std::mem::take(&mut self.doctype);
        self.emit(Token::Doctype(dt));
    }

    /// Append to the current DOCTYPE name (§13.2.5.55).
    pub(super) fn push_doctype_name(&mut self, c: char) {
        self.doctype.name.get_or_insert_with(String::new).push(c);
    }

    /// Append to the current DOCTYPE public identifier (§13.2.5.59/60).
    pub(super) fn push_doctype_public_id(&mut self, c: char) {
        self.doctype
            .public_id
            .get_or_insert_with(String::new)
            .push(c);
    }

    /// Append to the current DOCTYPE system identifier (§13.2.5.65/66).
    pub(super) fn push_doctype_system_id(&mut self, c: char) {
        self.doctype
            .system_id
            .get_or_insert_with(String::new)
            .push(c);
    }

    // ----- character-reference scratch helpers -----

    /// Set the character-reference return state (§13.2.5.72).
    pub(super) fn set_return_state(&mut self, state: State) {
        self.return_state = state;
    }

    /// Empty the temporary buffer (§13.2.5 "set the temporary buffer to
    /// the empty string").
    pub(super) fn clear_temp_buffer(&mut self) {
        self.temp_buffer.clear();
    }

    /// Append a character to the temporary buffer.
    pub(super) fn push_temp_buffer(&mut self, c: char) {
        self.temp_buffer.push(c);
    }

    /// Append a string to the temporary buffer.
    pub(super) fn push_temp_buffer_str(&mut self, s: &str) {
        self.temp_buffer.push_str(s);
    }

    /// Whether the temporary buffer equals `s`.
    pub(super) fn temp_buffer_is(&self, s: &str) -> bool {
        self.temp_buffer == s
    }

    /// Take the temporary buffer, leaving it empty.
    pub(super) fn take_temp_buffer(&mut self) -> String {
        std::mem::take(&mut self.temp_buffer)
    }

    /// Whether the character-reference return state is one of the
    /// attribute-value states (governs flush behaviour, §13.2.5.72).
    pub(super) fn in_attribute_return_state(&self) -> bool {
        matches!(
            self.return_state,
            State::AttributeValueDoubleQuoted
                | State::AttributeValueSingleQuoted
                | State::AttributeValueUnquoted
        )
    }

    /// "Flush code points consumed as a character reference" (§13.2.5.72):
    /// either append the temp buffer to the current attribute value (when
    /// returning to an attribute state) or emit it as character tokens.
    pub(super) fn flush_code_points_as_char_ref(&mut self) {
        let buf = std::mem::take(&mut self.temp_buffer);
        if self.in_attribute_return_state() {
            self.cur_attr_value.push_str(&buf);
        } else {
            for c in buf.chars() {
                self.emit_char(c);
            }
        }
    }

    /// Switch to the character-reference return state without reconsuming.
    pub(super) fn switch_to_return_state(&mut self) {
        self.state = self.return_state;
    }

    /// Reconsume the current input character in the return state
    /// (§13.2.5.72 "anything else").
    pub(super) fn reconsume_in_return_state(&mut self) {
        let rs = self.return_state;
        self.reconsume_in(rs);
    }

    // ----- numeric character-reference accumulator (§13.2.5.78/79) -----

    /// Reset the numeric character-reference code to `code`.
    pub(super) fn set_char_ref_code(&mut self, code: u32) {
        self.char_ref_code = code;
    }

    /// The accumulated numeric character-reference code.
    pub(super) fn char_ref_code(&self) -> u32 {
        self.char_ref_code
    }

    /// Multiply the accumulated code by `base` and add `digit`, saturating
    /// so an oversized reference cannot wrap (the end state rejects any
    /// value above U+10FFFF).
    pub(super) fn accumulate_char_ref_code(&mut self, base: u32, digit: u32) {
        self.char_ref_code = self
            .char_ref_code
            .saturating_mul(base)
            .saturating_add(digit);
    }

    // ----- input inspection helpers (used by the char-ref family) -----

    /// Length of the preprocessed input in characters.
    pub(super) fn input_len(&self) -> usize {
        self.input.len()
    }

    /// The character at index `i` (caller guarantees `i < input_len()`).
    pub(super) fn input_at(&self, i: usize) -> char {
        self.input[i]
    }

    /// Peek the character at index `i`, or `None` past the end.
    pub(super) fn peek_at(&self, i: usize) -> Option<char> {
        self.input.get(i).copied()
    }

    /// The index of the next character to consume.
    pub(super) fn pos(&self) -> usize {
        self.pos
    }
}

/// Classify a §13.2.3.5 input-stream parse error for `c`, if any.
///
/// Controls other than ASCII whitespace and U+0000 NULL are
/// `control-character-in-input-stream` errors; noncharacters are
/// `noncharacter-in-input-stream` errors. (U+0000 is reported per-state as
/// `unexpected-null-character`; U+000D is normalized away in
/// preprocessing; surrogates cannot occur in a Rust `&str`.)
fn input_stream_error(c: char) -> Option<&'static str> {
    let code = c as u32;
    let is_control = code <= 0x1F || (0x7F..=0x9F).contains(&code);
    let is_exempt = matches!(code, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20);
    if is_control && !is_exempt {
        return Some("control-character-in-input-stream");
    }
    if is_noncharacter(code) {
        return Some("noncharacter-in-input-stream");
    }
    None
}

/// WHATWG "noncharacter" test (used by §13.2.3.5 input-stream errors and
/// the §13.2.5.80 numeric character reference end state): the
/// U+FDD0–U+FDEF block and the last two code points of every plane.
pub(super) fn is_noncharacter(code: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&code) || (code & 0xFFFE) == 0xFFFE
}

/// The four ASCII whitespace characters the tokenizer treats specially
/// (§13.2.5): tab, line feed, form feed, space. Carriage return is
/// normalized to line feed during preprocessing, so it never appears.
pub(super) fn is_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{000C}' | ' ')
}

/// WHATWG HTML §13.2.3.5 "Preprocessing the input stream": normalize
/// newlines so that every U+000D CARRIAGE RETURN, whether or not followed
/// by U+000A LINE FEED, becomes a single U+000A. Charset decoding
/// (§13.2.3.1–4) is out of scope — strict mode takes `&str` input.
fn preprocess(html: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preprocess_normalizes_crlf_and_lone_cr() {
        assert_eq!(
            preprocess("a\r\nb\rc\nd"),
            vec!['a', '\n', 'b', '\n', 'c', '\n', 'd']
        );
    }

    /// Copilot R3: the cursor must not grow without bound when EOF is
    /// consumed repeatedly. After the EOF token, further `next_token`
    /// calls keep returning `EndOfFile` and `pos` stays clamped at the
    /// `len + 1` sentinel.
    #[test]
    fn cursor_clamped_at_eof_sentinel() {
        let mut t = Tokenizer::new("hi");
        loop {
            if t.next_token().expect("valid input") == Token::EndOfFile {
                break;
            }
        }
        let after_first_eof = t.pos();
        for _ in 0..10 {
            assert_eq!(t.next_token().expect("idempotent EOF"), Token::EndOfFile);
        }
        // `next_token` short-circuits on `eof_emitted` without consuming,
        // and even a direct over-consume cannot push past the sentinel.
        assert_eq!(t.pos(), after_first_eof);
        for _ in 0..5 {
            t.consume();
        }
        assert_eq!(t.pos(), t.input_len() + 1, "pos clamped at EOF sentinel");
    }

    /// Copilot R5: a parse error aborts permanently. A control character
    /// (`control-character-in-input-stream`) is emitted into the output
    /// queue in the same step that flags the error; the error must clear
    /// that token and every later call must re-return the error rather
    /// than leak the queued character.
    #[test]
    fn parse_error_aborts_permanently_without_leaking_tokens() {
        let mut t = Tokenizer::new("\u{000B}"); // vertical tab = control char
        let err = t.next_token().expect_err("control char rejected");
        assert!(err.errors[0].contains("control-character-in-input-stream"));
        for _ in 0..5 {
            let again = t.next_token().expect_err("error is terminal");
            assert_eq!(again.errors, err.errors, "no token may leak after abort");
        }
    }

    /// Copilot R6: a §13.2.3.5 input-stream error (here a control character
    /// in the after-DOCTYPE-name position) is raised as the character
    /// enters the stream, so it takes priority over the state-level
    /// `invalid-character-sequence-after-doctype-name` error that the same
    /// character would otherwise trigger.
    #[test]
    fn input_stream_error_takes_priority_over_state_error() {
        let mut t = Tokenizer::new("<!DOCTYPE html \u{000B}>");
        let err = loop {
            match t.next_token() {
                Ok(Token::EndOfFile) => panic!("expected strict reject"),
                Ok(_) => {}
                Err(e) => break e,
            }
        };
        assert!(
            err.errors[0].contains("control-character-in-input-stream"),
            "input-stream error must win over the state error: {:?}",
            err.errors
        );
    }
}

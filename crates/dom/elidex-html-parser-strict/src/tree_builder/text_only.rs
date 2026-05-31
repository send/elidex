//! WHATWG HTML §13.2.6.2 "Parsing elements that contain only text" — the
//! generic raw text and generic RCDATA element parsing algorithms, plus the
//! closely-related `<script>` raw-text take-over.

use super::parse_state::InsertionMode;
use super::TreeBuilder;
use crate::tokenizer::states::State;
use crate::tokenizer::token::TagToken;
use crate::StrictParseError;

impl TreeBuilder {
    /// Shared body of §13.2.6.2: insert the element, switch the tokenizer into
    /// `raw_state` (RAWTEXT / RCDATA / script data), save the original
    /// insertion mode, then switch to "text".
    ///
    /// The tokenizer already recorded this start tag as its last start tag on
    /// emit, so its appropriate-end-tag check closes the element correctly
    /// once `set_state` takes effect — no explicit `set_last_start_tag` call
    /// is needed in the document-parse flow.
    fn parse_text_element(
        &mut self,
        token: &TagToken,
        raw_state: State,
    ) -> Result<(), StrictParseError> {
        self.insert_html_element(token)?;
        self.tokenizer.set_state(raw_state);
        self.state.original_mode = Some(self.state.mode);
        self.state.mode = InsertionMode::Text;
        Ok(())
    }

    /// §13.2.6.2 "the generic raw text element parsing algorithm"
    /// (style / noframes / noscript-when-scripting / xmp / iframe / noembed).
    pub(super) fn parse_generic_rawtext(
        &mut self,
        token: &TagToken,
    ) -> Result<(), StrictParseError> {
        self.parse_text_element(token, State::Rawtext)
    }

    /// §13.2.6.2 "the generic RCDATA element parsing algorithm"
    /// (title / textarea).
    pub(super) fn parse_generic_rcdata(
        &mut self,
        token: &TagToken,
    ) -> Result<(), StrictParseError> {
        self.parse_text_element(token, State::Rcdata)
    }

    /// §13.2.6.4.4 `<script>` start tag: insert the element, switch the
    /// tokenizer to the script data state, and enter "text". The strict parser
    /// has no scripting engine, so the script-execution bookkeeping
    /// (parser document, force-async, prepare-the-script) is intentionally
    /// omitted; A3 only builds the tree.
    pub(super) fn parse_script_element(
        &mut self,
        token: &TagToken,
    ) -> Result<(), StrictParseError> {
        self.parse_text_element(token, State::ScriptData)
    }
}

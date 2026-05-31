//! WHATWG HTML §13.2.4.1 "reset the insertion mode appropriately".

use super::parse_state::InsertionMode;
use super::TreeBuilder;

impl TreeBuilder {
    /// Run the §13.2.4.1 "reset the insertion mode appropriately" 17-step
    /// stack walk, selecting the insertion mode from the open-elements stack.
    ///
    /// Shared by `</template>` closing (§13.2.6.4.4 / .16), table-section /
    /// caption / cell closing, and the declarative-shadow `</template>`
    /// path. Strict mode is never the fragment case, so the context-element
    /// substitution (step 3) and the fragment-only fallbacks (steps 13, 15)
    /// only fire on a malformed stack — they are kept for fidelity.
    pub(super) fn reset_insertion_mode_appropriately(&mut self) {
        let len = self.state.open_elements.len();
        for idx in (0..len).rev() {
            let node = self.state.open_elements[idx];
            // Step 3: `last` is true at the topmost (first) stack entry.
            let last = idx == 0;

            // Steps 4-9: table-context elements.
            if !last && self.entity_has_any_tag(node, &["td", "th"]) {
                self.state.mode = InsertionMode::InCell;
                return;
            }
            if self.entity_has_tag(node, "tr") {
                self.state.mode = InsertionMode::InRow;
                return;
            }
            if self.entity_has_any_tag(node, &["tbody", "thead", "tfoot"]) {
                self.state.mode = InsertionMode::InTableBody;
                return;
            }
            if self.entity_has_tag(node, "caption") {
                self.state.mode = InsertionMode::InCaption;
                return;
            }
            if self.entity_has_tag(node, "colgroup") {
                self.state.mode = InsertionMode::InColumnGroup;
                return;
            }
            if self.entity_has_tag(node, "table") {
                self.state.mode = InsertionMode::InTable;
                return;
            }
            // Step 10: a template resets to the current template insertion mode.
            if self.entity_has_tag(node, "template") {
                let mode = *self.state.template_modes.last().expect(
                    "a template on the stack of open elements has a template insertion mode",
                );
                self.state.mode = mode;
                return;
            }
            // Steps 11-14: head / body / frameset / html.
            if !last && self.entity_has_tag(node, "head") {
                self.state.mode = InsertionMode::InHead;
                return;
            }
            if self.entity_has_tag(node, "body") {
                self.state.mode = InsertionMode::InBody;
                return;
            }
            if self.entity_has_tag(node, "frameset") {
                self.state.mode = InsertionMode::InFrameset;
                return;
            }
            if self.entity_has_tag(node, "html") {
                // Step 14: before head if the head pointer is null, else after
                // head.
                self.state.mode = if self.state.head_pointer.is_none() {
                    InsertionMode::BeforeHead
                } else {
                    InsertionMode::AfterHead
                };
                return;
            }
            // Step 15: fragment-case fallback.
            if last {
                self.state.mode = InsertionMode::InBody;
                return;
            }
        }
    }
}

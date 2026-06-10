//! WHATWG HTML §13.2.4.1 "reset the insertion mode appropriately".

use super::parse_state::InsertionMode;
use super::TreeBuilder;

impl TreeBuilder {
    /// Run the §13.2.4.1 "reset the insertion mode appropriately" 17-step
    /// stack walk, selecting the insertion mode from the open-elements stack.
    ///
    /// Shared by `</template>` closing (§13.2.6.4.4 / .16), table-section /
    /// caption / cell closing, the declarative-shadow `</template>` path, and
    /// the §13.4 HTML fragment parsing algorithm (step 16).
    ///
    /// Step 3 fragment-case substitution: when the walk reaches the bottom of
    /// the stack (`last == true`) and the parser was created for fragment
    /// parsing ([`ParseState::fragment_context`](super::parse_state::ParseState::fragment_context)
    /// is `Some`), the spec evaluates the remaining steps against the *context
    /// element* rather than the (synthetic `<html>`) bottom node — that is how
    /// a `<td>` / `<tr>` / `<select>` context selects "in cell" / "in row" /
    /// "in body". For whole-document parsing `fragment_context` is `None`, so
    /// the bottom node is always the real `<html>` element and the existing
    /// 21-mode behaviour is unchanged. The `!last` guards on the td/th and head
    /// steps already encode the fragment-case "last is false" conditions
    /// (§13.2.4.1 steps 4 / 11), so no further fragment branching is needed.
    pub(super) fn reset_insertion_mode_appropriately(&mut self) {
        let len = self.state.open_elements.len();
        for idx in (0..len).rev() {
            // Step 3: `last` is true at the topmost (first) stack entry. In the
            // fragment case the spec then substitutes the context element for
            // the node at this last position.
            let last = idx == 0;
            let node = if last {
                self.state
                    .fragment_context
                    .unwrap_or(self.state.open_elements[idx])
            } else {
                self.state.open_elements[idx]
            };

            // A foreign-namespace fragment context matches none of the HTML
            // element-type cases below (the spec's element references in §13.4
            // are HTML-namespace), so it resets to "in body" — the §13.2.4.1
            // step-15 fallback; the foreign-content dispatcher then routes its
            // children. (Document parsing has no foreign context here.)
            if last && self.dom.namespace_of(node) != elidex_ecs::Namespace::Html {
                self.state.mode = InsertionMode::InBody;
                return;
            }

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

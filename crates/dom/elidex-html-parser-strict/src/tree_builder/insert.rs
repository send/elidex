//! WHATWG HTML §13.2.6.1 "Creating and inserting nodes" — strict node
//! insertion, plus the stack-of-open-elements and scope helpers the mode
//! handlers share.
//!
//! Strict mode never enables foster parenting, so the "appropriate place for
//! inserting a node" reduces to the current node (with the template-contents
//! redirect of §13.2.6.1 step 3). All `EcsDom` mutation is confined to this
//! module's marshalling helpers.

use elidex_ecs::{Attributes, Entity, Namespace, NodeKind, TextContent};

use super::parse_state::{self, Scope};
use super::{parse_error, TreeBuilder};
use crate::tokenizer::token::{DoctypeToken, TagToken};
use crate::StrictParseError;

impl TreeBuilder {
    // ----- the appropriate place + raw EcsDom marshalling -----

    /// WHATWG HTML §13.2.6.1 "the appropriate place for inserting a node".
    ///
    /// Foster parenting is never enabled in strict mode, so the adjusted
    /// insertion location is simply the current node (or the Document when the
    /// stack is empty, before the `html` element is pushed). Step 3 redirects
    /// insertion inside a `<template>` to its template contents: for an ordinary
    /// template that is its detached content `DocumentFragment`
    /// (`ContentTarget::ContentFragment`); for a declarative shadow template
    /// it is the attached shadow root (`ContentTarget::ShadowRoot`). Both
    /// kinds resolve uniformly here via `ContentTarget::entity`.
    pub(super) fn appropriate_place(&self) -> Entity {
        let target = self.state.current_node().unwrap_or(self.document);
        if self.dom.has_tag(target, "template") {
            if let Some(content) = self.state.template_content_targets.get(&target) {
                return content.entity();
            }
        }
        target
    }

    /// Append `child` to `parent`, asserting success. Strict construction only
    /// ever appends a freshly created node to a live ancestor, so
    /// [`EcsDom::append_child`] cannot legitimately reject (self-append /
    /// cycle / destroyed entity are all impossible here).
    pub(super) fn append(&mut self, parent: Entity, child: Entity) {
        if !self.dom.append_child(parent, child) {
            debug_assert!(false, "strict insert: append_child rejected unexpectedly");
        }
    }

    /// Create an HTML-namespace element named `tag` with `attrs` (in source
    /// order), append it at the appropriate place, and push it onto the stack
    /// of open elements (§13.2.6.1 "insert an HTML element"). Used for both
    /// real tokens and the synthetic start tags the spec inserts (e.g. an
    /// implied `tbody`).
    pub(super) fn insert_element_named(&mut self, tag: &str, attrs: &[(String, String)]) -> Entity {
        let parent = self.appropriate_place();
        let mut attributes = Attributes::default();
        for (name, value) in attrs {
            attributes.set(name.as_str(), value.as_str());
        }
        let element = self.dom.create_element(tag, attributes);
        self.append(parent, element);
        self.state.open_elements.push(element);
        element
    }

    /// Insert an HTML element for a non-void start-tag `token` (§13.2.6.1).
    ///
    /// A self-closing flag on a non-void HTML element is the
    /// `non-void-html-element-start-tag-with-trailing-solidus` parse error
    /// (§13.2.5.40 / §13.2.6); strict mode rejects rather than silently
    /// ignoring the solidus.
    pub(super) fn insert_html_element(
        &mut self,
        token: &TagToken,
    ) -> Result<Entity, StrictParseError> {
        if token.self_closing {
            return Err(parse_error(
                "non-void-html-element-start-tag-with-trailing-solidus",
            ));
        }
        Ok(self.insert_element_named(&token.name, &token.attrs))
    }

    /// WHATWG HTML §13.2.6.1 "insert a foreign element" — create a foreign
    /// (SVG / MathML) element named `tag` in `namespace` with `attrs`, append
    /// it at the appropriate place, and push it onto the stack of open
    /// elements. `tag` and `attrs` are the already-adjusted name and
    /// attributes (the §13.2.6.1 / §13.2.6.5 case tables run in
    /// [`super::foreign_adjust`] before this marshalling step).
    ///
    /// Unlike [`insert_html_element`](Self::insert_html_element), a foreign
    /// element's self-closing flag is **not** a parse error: §13.2.6.5 makes
    /// a self-closing foreign start tag valid and pops the element. The caller
    /// performs that pop; this helper only inserts.
    pub(super) fn insert_foreign_element(
        &mut self,
        tag: &str,
        attrs: &[(String, String)],
        namespace: Namespace,
    ) -> Entity {
        let parent = self.appropriate_place();
        let mut attributes = Attributes::default();
        for (name, value) in attrs {
            attributes.set(name.as_str(), value.as_str());
        }
        // Owner document for the foreign node's `AssociatedDocument`. In a
        // §13.4 fragment parse `self.document` is the throwaway document that is
        // despawned before the nodes are returned (and `take_fragment_children`
        // re-homes the whole returned subtree to the context's node document
        // anyway), so use that node document here too — a live owner. Document
        // parsing keeps `self.document` (the real result Document).
        let owner = match self.state.fragment_context {
            Some(context) => self.dom.owner_document(context),
            None => Some(self.document),
        };
        let element = self
            .dom
            .create_element_ns(tag, namespace, attributes, owner);
        self.append(parent, element);
        self.state.open_elements.push(element);
        element
    }

    /// Insert a void HTML element for `token` and immediately pop it
    /// (§13.2.6.4 "Insert an HTML element for the token. Immediately pop the
    /// current node off the stack of open elements."). Void elements
    /// acknowledge the self-closing flag, so a trailing solidus is permitted
    /// and has no effect.
    pub(super) fn insert_void_element(&mut self, token: &TagToken) -> Entity {
        let element = self.insert_element_named(&token.name, &token.attrs);
        // Pop via the cleanup-aware wrapper so every stack removal site keeps
        // the content-target override map bounded by one uniform path.
        self.pop();
        element
    }

    /// §13.2.6.1 "insert a comment" — at the appropriate place.
    pub(super) fn insert_comment(&mut self, data: &str) {
        let parent = self.appropriate_place();
        let comment = self.dom.create_comment(data);
        self.append(parent, comment);
    }

    /// Insert a comment as the last child of a specific node (the spec's
    /// "insert a comment as the last child of the Document object / the first
    /// element in the stack of open elements").
    pub(super) fn insert_comment_to(&mut self, parent: Entity, data: &str) {
        let comment = self.dom.create_comment(data);
        self.append(parent, comment);
    }

    /// §13.2.6.4.1 — append a DocumentType node to the Document for a
    /// conforming DOCTYPE token. Missing identifiers map to the empty string,
    /// as the spec requires. Quirks mode is never set: every DOCTYPE the
    /// strict parser accepts is, by construction, a no-quirks DOCTYPE (the
    /// quirks conditions are a subset of the parse-error conditions that abort
    /// first), so there is no document-mode state to write.
    pub(super) fn insert_doctype(&mut self, dt: &DoctypeToken) {
        let name = dt.name.as_deref().unwrap_or("");
        let public_id = dt.public_id.as_deref().unwrap_or("");
        let system_id = dt.system_id.as_deref().unwrap_or("");
        let node = self.dom.create_document_type(name, public_id, system_id);
        self.append(self.document, node);
    }

    // ----- character / text coalescing (§13.2.6.1 "insert a character") -----

    /// Buffer a character for text-node coalescing. The buffer is flushed by
    /// [`Self::flush_text`] before the next non-character token is processed.
    pub(super) fn insert_character(&mut self, ch: char) {
        self.pending_text.push(ch);
    }

    /// Flush the pending character run at the appropriate place, if non-empty.
    /// Idempotent: a no-op when the buffer is empty, so it is safe to call
    /// before every non-character token.
    pub(super) fn flush_text(&mut self) {
        if self.pending_text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.pending_text);
        let parent = self.appropriate_place();
        self.append_text(parent, &text);
    }

    /// Insert `text` at the appropriate place immediately (bypassing the
    /// [`Self::pending_text`] buffer). Used by the "in table text" mode to
    /// flush its accumulated whitespace run while the table is still the
    /// current node.
    pub(super) fn insert_text_run(&mut self, text: &str) {
        let parent = self.appropriate_place();
        self.append_text(parent, text);
    }

    /// §13.2.6.1 "insert a character": if the node immediately before the
    /// insertion location (the parent's last child) is a Text node, append
    /// `text` to it; otherwise create a new Text node. This is the DOM-level
    /// text coalescing that merges character runs split across intervening
    /// non-character tokens (e.g. `X</html> ` → one `"X "` text node).
    fn append_text(&mut self, parent: Entity, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.dom.get_last_child(parent) {
            if self.dom.node_kind(last) == Some(NodeKind::Text) {
                let mut combined = self
                    .dom
                    .world()
                    .get::<&TextContent>(last)
                    .map(|t| t.0.clone())
                    .unwrap_or_default();
                combined.push_str(text);
                self.dom.set_text_data(last, &combined);
                return;
            }
        }
        let node = self.dom.create_text(text);
        self.append(parent, node);
    }

    // ----- stack of open elements (§13.2.4.2) -----

    /// Pop the current node off the stack of open elements, returning it.
    /// Drops any `<template>` content-target override the element carried
    /// (**both** kinds — ordinary content-fragment and declarative shadow —
    /// unconditionally), keeping the override map bounded by the live stack. A
    /// kind branch here would leak ordinary entries past their stack lifetime.
    ///
    /// Returns the popped entity for the caller to inspect (e.g. `pop_until_tag`
    /// checks its tag), so it does NOT despawn it. A consumed declarative-shadow
    /// template (stack-only, never in the tree) is despawned at its consumption
    /// sites (`</template>` / fragment rollback), which capture its
    /// content-target *kind* before this clears the map entry; an ordinary
    /// template stays in the tree and is never despawned there.
    pub(super) fn pop(&mut self) -> Option<Entity> {
        let popped = self.state.open_elements.pop();
        if let Some(entity) = popped {
            self.state.template_content_targets.remove(&entity);
        }
        popped
    }

    /// Pop elements off the stack until an element with tag name `tag` has
    /// been popped (the spec's "pop elements … until a _tag_ element has been
    /// popped from the stack").
    pub(super) fn pop_until_tag(&mut self, tag: &str) {
        while let Some(entity) = self.pop() {
            if self.dom.has_tag(entity, tag) {
                break;
            }
        }
    }

    /// Pop elements off the stack until an element whose tag name is one of
    /// `tags` has been popped.
    pub(super) fn pop_until_any(&mut self, tags: &[&str]) {
        while let Some(entity) = self.pop() {
            if self.entity_has_any_tag(entity, tags) {
                break;
            }
        }
    }

    /// Whether there is a `template` element anywhere on the stack of open
    /// elements (§13.2.6.4.7 "if there is a template element on the stack").
    pub(super) fn has_template_on_stack(&self) -> bool {
        self.has_element_on_stack("template")
    }

    /// Whether there is an element with tag name `tag` anywhere on the stack
    /// of open elements.
    pub(super) fn has_element_on_stack(&self, tag: &str) -> bool {
        self.state
            .open_elements
            .iter()
            .any(|&entity| self.dom.has_tag(entity, tag))
    }

    /// Pop elements off the stack until the specific element `target` has been
    /// popped (the spec's "pop all the nodes from the current node up to and
    /// including _node_").
    pub(super) fn pop_until_entity(&mut self, target: Entity) {
        while let Some(entity) = self.pop() {
            if entity == target {
                break;
            }
        }
    }

    /// Remove a specific element from anywhere in the stack of open elements
    /// (the spec's "remove _node_ from the stack of open elements", used by
    /// `</form>` where the form may not be the current node).
    pub(super) fn remove_from_open_elements(&mut self, target: Entity) {
        self.state.open_elements.retain(|&entity| entity != target);
        self.state.template_content_targets.remove(&target);
    }

    /// Whether `entity` is in the "special" category (§13.2.4.2).
    pub(super) fn is_special(&self, entity: Entity) -> bool {
        self.dom.with_tag_name(
            entity,
            |tag| matches!(tag, Some(name) if parse_state::is_special_tag(name)),
        )
    }

    /// Pop the stack until the current node's tag is one of `boundary`
    /// (the spec's "clear the stack back to a … context" family). `html` and
    /// `template` are always the outer boundary; callers add the
    /// context-specific tags.
    fn clear_stack_to(&mut self, boundary: &[&str]) {
        while let Some(node) = self.state.current_node() {
            if self.entity_has_any_tag(node, boundary) {
                break;
            }
            self.pop();
        }
    }

    /// §13.2.6.4.9 "clear the stack back to a table context".
    pub(super) fn clear_stack_to_table_context(&mut self) {
        self.clear_stack_to(&["table", "template", "html"]);
    }

    /// §13.2.6.4.13 "clear the stack back to a table body context".
    pub(super) fn clear_stack_to_table_body_context(&mut self) {
        self.clear_stack_to(&["tbody", "tfoot", "thead", "template", "html"]);
    }

    /// §13.2.6.4.14 "clear the stack back to a table row context".
    pub(super) fn clear_stack_to_table_row_context(&mut self) {
        self.clear_stack_to(&["tr", "template", "html"]);
    }

    // ----- tag-name inspection helpers -----

    /// Whether `entity` is an HTML element with tag name `tag`.
    pub(super) fn entity_has_tag(&self, entity: Entity, tag: &str) -> bool {
        self.dom.has_tag(entity, tag)
    }

    /// Whether `entity` is an HTML element whose tag name is one of `tags`.
    pub(super) fn entity_has_any_tag(&self, entity: Entity, tags: &[&str]) -> bool {
        self.dom
            .with_tag_name(entity, |t| matches!(t, Some(name) if tags.contains(&name)))
    }

    /// Whether the current node is an HTML element with tag name `tag`.
    pub(super) fn current_node_has_tag(&self, tag: &str) -> bool {
        self.state
            .current_node()
            .is_some_and(|entity| self.dom.has_tag(entity, tag))
    }

    /// Whether the current node is an HTML element whose tag name is one of
    /// `tags`.
    pub(super) fn current_node_has_any_tag(&self, tags: &[&str]) -> bool {
        self.state
            .current_node()
            .is_some_and(|entity| self.entity_has_any_tag(entity, tags))
    }

    // ----- scope predicates (§13.2.4.2) -----

    /// Whether the stack has an HTML element with tag name `tag` in `scope`.
    /// The scope targets in the "in HTML content" rules are always HTML
    /// elements, so the target match is HTML-namespace-qualified (a foreign
    /// element of the same tag name does not satisfy it).
    pub(super) fn has_tag_in_scope(&self, tag: &str, scope: Scope) -> bool {
        parse_state::has_in_scope(&self.dom, &self.state.open_elements, scope, |ns, name| {
            ns == Namespace::Html && name == tag
        })
    }

    /// Whether the stack has an HTML element whose tag name is one of `tags`
    /// in `scope`.
    pub(super) fn has_any_tag_in_scope(&self, tags: &[&str], scope: Scope) -> bool {
        parse_state::has_in_scope(&self.dom, &self.state.open_elements, scope, |ns, name| {
            ns == Namespace::Html && tags.contains(&name)
        })
    }

    /// Whether the stack has the specific element `target` in `scope` (matched
    /// by entity identity — used for `</form>`).
    pub(super) fn has_entity_in_scope(&self, target: Entity, scope: Scope) -> bool {
        parse_state::has_entity_in_scope(&self.dom, &self.state.open_elements, target, scope)
    }
}

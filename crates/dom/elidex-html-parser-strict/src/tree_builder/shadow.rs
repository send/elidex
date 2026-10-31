//! `<template>` handling, including the WHATWG HTML §13.2.6.4.4 template start
//! tag's declarative shadow root branch (HTML §4.12.3 `shadowrootmode` + DOM §4.9
//! "attach a shadow root").
//!
//! Strict mode builds the tree in a single pass straight into `EcsDom`, so the
//! compat path's two-pass approach (build an `RcDom`, then post-hoc reparent
//! `template_contents` into a shadow root, see
//! `crates/dom/elidex-html-parser/src/convert.rs`) does not apply. Instead the
//! spec's stream algorithm is implemented directly: a declarative
//! `<template shadowrootmode>` is created stack-only (not appended to the
//! light DOM), a shadow root is attached to the host (the open element it
//! appears in), and the template's "template contents" is bound to that shadow
//! root so subsequent children route into it. The only `EcsDom` API shared
//! with the compat path is [`EcsDom::attach_shadow_with_init`]; host-tag
//! validation lives inside it.

use elidex_ecs::{Attributes, Entity, ShadowInit, ShadowRootMode, SlotAssignmentMode};

use super::parse_state::{ContentTarget, InsertionMode};
use super::{parse_error, TreeBuilder};
use crate::tokenizer::token::TagToken;
use crate::StrictParseError;

/// Parse the declarative-shadow attributes off a `<template>` start tag.
///
/// Returns `Some(ShadowInit)` only when `shadowrootmode` is present with a
/// valid `open`/`closed` value (HTML §4.12.3 enumerated attribute, ASCII
/// case-insensitive); otherwise `None`, i.e. the `shadowrootmode` is in the
/// "no shadow root" None state and the template is ordinary.
fn shadow_init_from_attrs(attrs: &[(String, String)]) -> Option<ShadowInit> {
    let mode_value = attrs
        .iter()
        .find(|(name, _)| name == "shadowrootmode")
        .map(|(_, value)| value)?;
    let mode = if mode_value.eq_ignore_ascii_case("open") {
        ShadowRootMode::Open
    } else if mode_value.eq_ignore_ascii_case("closed") {
        ShadowRootMode::Closed
    } else {
        return None;
    };
    let has = |name: &str| attrs.iter().any(|(n, _)| n == name);
    // §4.12.3 / the template start tag: slot assignment defaults to "named", "manual" only
    // when the enumerated attribute is in the Manual state.
    let slot_assignment = attrs
        .iter()
        .find(|(name, _)| name == "shadowrootslotassignment")
        .map_or(SlotAssignmentMode::Named, |(_, value)| {
            if value.eq_ignore_ascii_case("manual") {
                SlotAssignmentMode::Manual
            } else {
                SlotAssignmentMode::Named
            }
        });
    Some(ShadowInit {
        mode,
        delegates_focus: has("shadowrootdelegatesfocus"),
        slot_assignment,
        clonable: has("shadowrootclonable"),
        serializable: has("shadowrootserializable"),
        // Declarative shadow roots have no customElementRegistry
        // channel (HTML declarative-shadow attributes carry no such
        // member) — always the document registry.
        null_registry: false,
    })
}

impl TreeBuilder {
    /// WHATWG HTML §13.2.6.4.4 — the `<template>` start tag algorithm, shared by
    /// every mode that processes `<template>` "using the rules for the in head
    /// insertion mode".
    pub(super) fn template_start_tag(&mut self, token: &TagToken) -> Result<(), StrictParseError> {
        // `<template/>` is a non-void self-closing start tag — a parse error.
        if token.self_closing {
            return Err(parse_error(
                "non-void-html-element-start-tag-with-trailing-solidus",
            ));
        }
        // The marker / frameset-ok / insertion-mode / template-mode steps (run
        // for both ordinary and declarative templates). "Insert a marker at the
        // end of the list of active formatting elements" is a no-op: strict mode
        // does not maintain that list.
        self.state.frameset_ok = false;
        self.state.mode = InsertionMode::InTemplate;
        self.state.template_modes.push(InsertionMode::InTemplate);

        // The template start tag: declarative only if shadowrootmode is
        // present and valid, the parser allows declarative shadow roots, and
        // hostIsRoot is false. declarativeShadowHostElement is the *current
        // node* (not the adjusted current node), so a top-level template in
        // the fragment case never reaches the external context element;
        // hostIsRoot (the fragment root also has a redirection-map entry) is
        // "the stack holds one element", and then the template is a regular
        // element (the "Keep" branch). Document parsing and a nested
        // in-fragment host have a stack deeper than one.
        if self.allow_declarative_shadow && self.state.open_elements.len() > 1 {
            if let Some(init) = shadow_init_from_attrs(&token.attrs) {
                self.attach_declarative_shadow(token, init);
                return Ok(());
            }
        }

        // The "Keep" branch's regular template (also hostIsRoot, no/invalid
        // shadowrootmode, parser flag off). Per HTML §4.12.3 the element gets its
        // associated content `DocumentFragment` "when the template element is
        // created"; the §13.2.6.1 appropriate-place redirect then routes the
        // template's children into that fragment rather than the (in-tree)
        // template element.
        let template = self.insert_html_element(token)?;
        self.attach_ordinary_template_contents(template);
        Ok(())
    }

    /// Give an **ordinary** `<template>` its content fragment + §13.2.6.1
    /// appropriate-place redirect (HTML §4.12.3). Shared by the ordinary-template
    /// arm and the declarative-shadow graceful fallback (a failed DSD template
    /// becomes ordinary), so the fragment / owner-doc / redirect semantics stay
    /// identical at both sites (One-issue-one-way). Owner doc is explicit —
    /// correct for a nested template whose in-tree parent is an outer template's
    /// detached content fragment (where the tree-root walk would dead-end): the
    /// parse document for document parsing; in a §13.4 fragment parse the
    /// **context's** node document, as the throwaway document is destroyed and
    /// (for a documentless context, where the adopt is skipped) the stamp would
    /// dangle. `None` leaves the fragment unstamped, matching the tolerant backend.
    fn attach_ordinary_template_contents(&mut self, template: Entity) {
        let owner = match self.state.fragment_context {
            Some(context) => self.dom.owner_document(context),
            None => Some(self.document),
        };
        let fragment = self.dom.attach_template_contents(template, owner);
        self.state
            .template_content_targets
            .insert(template, ContentTarget::ContentFragment(fragment));
    }

    /// WHATWG HTML §13.2.6.4.4 template start tag — the "Keep" branch's
    /// declarative shadow root attach (hostIsRoot already ruled out).
    ///
    /// Infallible: a failed shadow attach falls back to an ordinary template,
    /// it is not a parse error.
    fn attach_declarative_shadow(&mut self, token: &TagToken, init: ShadowInit) {
        // declarativeShadowHostElement is the current node. The fallback
        // insertion location is the appropriate place's element. Both are
        // captured before the template is pushed.
        let host = self.state.current_node().unwrap_or(self.document);
        let fallback_parent = self.appropriate_place();

        // The declarative branch of the template start tag (§13.2.6.4.4): the
        // template element is created and pushed onto the stack of open elements
        // but never inserted, and its template contents are set to the shadow
        // root (below). The regular-template "insert an element at the adjusted
        // insertion location with template, insertionLocation, and true" calls
        // are the other branches; their `true` is not "stack only".
        let mut attributes = Attributes::default();
        for (name, value) in &token.attrs {
            attributes.set(name.as_str(), value.as_str());
        }
        let template = self.dom.create_element("template", attributes);
        self.state.open_elements.push(template);

        // "Attach a shadow root" to the host. Any failure
        // (host tag not allowed, or host already a shadow host) is the spec's
        // graceful fallback — append the template to the light DOM as an
        // ordinary element. On success the template's "template contents" is
        // the shadow root, so children inserted while the template is the
        // current node route into the shadow (via the appropriate-place
        // template-contents redirect).
        if let Ok(shadow_root) = self.dom.attach_shadow_with_init(host, init) {
            self.state
                .template_content_targets
                .insert(template, ContentTarget::ShadowRoot(shadow_root));
        } else {
            // Graceful fallback: the failed declarative shadow template
            // becomes an ordinary in-tree `<template>` (§4.12.3), so it gets
            // the same content fragment + redirect as the ordinary arm —
            // otherwise its children would land as light children and
            // `.content` would be empty (the tolerant fallback already routes
            // through the ordinary template arm).
            self.append(fallback_parent, template);
            self.attach_ordinary_template_contents(template);
        }
    }

    /// WHATWG HTML §13.2.6.4.4 — the `</template>` end tag algorithm.
    ///
    /// Returns the `unexpected-end-tag` parse error (strict reject) when there
    /// is no template on the stack or the implied-end-tag generation leaves a
    /// non-template current node (a misnested close).
    pub(super) fn template_end_tag(&mut self) -> Result<(), StrictParseError> {
        if !self.has_template_on_stack() {
            return Err(parse_error("unexpected-end-tag-template-no-open-template"));
        }
        // Step 1.
        self.generate_all_implied_end_tags_thoroughly();
        // Step 2: a non-template current node here is a misnested close.
        if !self.current_node_has_tag("template") {
            return Err(parse_error("unexpected-end-tag-template-misnested"));
        }
        // Step 3. The current node is the template being closed. If it is a
        // consumed declarative-shadow template (stack-only, never in the tree —
        // identified by a `ShadowRoot`-kind content-target), despawn it after the
        // pop so it does not dangle; capture it before pop clears the entry. An
        // ordinary `ContentFragment`-kind template is in the tree and must NOT be
        // despawned here (a missed kind branch = data loss).
        let consumed_shadow_template = self
            .state
            .current_node()
            .filter(|&t| self.state.is_consumed_shadow_template(t));
        self.pop_until_tag("template");
        if let Some(template) = consumed_shadow_template {
            let _ = self.dom.destroy_entity(template);
        }
        // Step 4, "clear the list of active formatting elements up to the last
        // marker", is a no-op (no active formatting list in strict mode).
        // Step 5.
        self.state.template_modes.pop();
        // Step 6.
        self.reset_insertion_mode_appropriately();
        Ok(())
    }
}

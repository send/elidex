//! `<template>` handling, including the WHATWG HTML §13.2.6.4.4 step 10
//! declarative shadow root branch (HTML §4.12.3 `shadowrootmode` + DOM §4.9
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

use elidex_ecs::{Attributes, ShadowInit, ShadowRootMode, SlotAssignmentMode};

use super::parse_state::InsertionMode;
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
    // §4.12.3 / step 4-5: slot assignment defaults to "named", "manual" only
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
    })
}

impl TreeBuilder {
    /// WHATWG HTML §13.2.6.4.4 — the `<template>` start tag algorithm (steps
    /// 1-10), shared by every mode that processes `<template>` "using the
    /// rules for the in head insertion mode".
    pub(super) fn template_start_tag(&mut self, token: &TagToken) -> Result<(), StrictParseError> {
        // `<template/>` is a non-void self-closing start tag — a parse error.
        if token.self_closing {
            return Err(parse_error(
                "non-void-html-element-start-tag-with-trailing-solidus",
            ));
        }
        // Steps 2-5 (run for both ordinary and declarative templates). Step 2,
        // "insert a marker at the end of the list of active formatting
        // elements", is a no-op: strict mode does not maintain that list.
        self.state.frameset_ok = false;
        self.state.mode = InsertionMode::InTemplate;
        self.state.template_modes.push(InsertionMode::InTemplate);

        // Steps 6-9: declarative only if shadowrootmode is present and valid,
        // the document allows declarative shadow roots, and the adjusted
        // current node is not the topmost element (i.e. not the fragment
        // root). In document parsing the adjusted current node is the current
        // node, so "not topmost" reduces to a stack depth greater than one.
        let adjusted_current_not_topmost = self.state.open_elements.len() > 1;
        if adjusted_current_not_topmost && self.allow_declarative_shadow {
            if let Some(init) = shadow_init_from_attrs(&token.attrs) {
                self.attach_declarative_shadow(token, init);
                return Ok(());
            }
        }

        // Step 9: ordinary template.
        self.insert_html_element(token)?;
        Ok(())
    }

    /// WHATWG HTML §13.2.6.4.4 step 10 — the declarative shadow root branch.
    ///
    /// Infallible: a failed shadow attach falls back to an ordinary template,
    /// it is not a parse error.
    fn attach_declarative_shadow(&mut self, token: &TagToken, init: ShadowInit) {
        // Step 10.1: the host is the adjusted current node (= current node in
        // document parsing). Step 6-7: the fallback insertion parent is the
        // appropriate place's element. Both are captured before the template
        // is pushed.
        let host = self.state.current_node().unwrap_or(self.document);
        let fallback_parent = self.appropriate_place();

        // Step 10.2: insert a foreign element with onlyAddToElementStack=true
        // — create the template and push it onto the stack, but do not append
        // it to the light DOM.
        let mut attributes = Attributes::default();
        for (name, value) in &token.attrs {
            attributes.set(name.as_str(), value.as_str());
        }
        let template = self.dom.create_element("template", attributes);
        self.state.open_elements.push(template);

        // Steps 10.9/10.10: attach a shadow root to the host. Any failure
        // (host tag not allowed, or host already a shadow host) is the spec's
        // graceful fallback — append the template to the light DOM as an
        // ordinary element. On success the template's "template contents" is
        // the shadow root, so children inserted while the template is the
        // current node route into the shadow (via the appropriate-place
        // template-contents redirect).
        match self.dom.attach_shadow_with_init(host, init) {
            Ok(shadow_root) => {
                self.state
                    .template_content_targets
                    .insert(template, shadow_root);
            }
            Err(_) => {
                self.append(fallback_parent, template);
            }
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
        // identified by its content-target entry), despawn it after the pop so
        // it does not dangle; capture it before pop clears the entry.
        let consumed_shadow_template = self
            .state
            .current_node()
            .filter(|t| self.state.template_content_targets.contains_key(t));
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

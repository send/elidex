//! Form submission and reset.

use elidex_ecs::{
    EcsDom, Entity, OutputDefaultValue, OutputValueOverride, TagType, MAX_ANCESTOR_DEPTH,
};

use crate::{FormControlKind, FormControlState};

/// The outcome of running the form submission algorithm
/// (WHATWG HTML §4.10.22.3) for a given submitter.
///
/// The submitter's **method** (§attr-fs-method: the submit button's
/// `formmethod` if present, otherwise the form's `method`) selects the
/// terminal branch — the `dialog` keyword (step 11) closes a `<dialog>`
/// and never navigates, so it carries no action/method/enctype.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormSubmission {
    /// Steps 12+ — navigate the form's target (GET/POST).
    Navigate {
        /// The form action URL (empty → caller substitutes the document URL).
        action: String,
        /// The HTTP method, normalized to `"get"` or `"post"`.
        method: String,
        /// The encoding type (e.g. "application/x-www-form-urlencoded").
        enctype: String,
        /// The collected form data entries.
        data: Vec<FormDataEntry>,
    },
    /// Step 11 — `method=dialog`: close the form's nearest ancestor
    /// `<dialog>` with `result`, firing no navigation.
    Dialog {
        /// The nearest ancestor `<dialog>` (step 11.2 "subject").
        subject: Entity,
        /// The result (steps 11.3-11.5): a submit button's optional value,
        /// or `None` when the submitter is not a submit button / has no
        /// `value` attribute. `None` leaves the dialog's `returnValue`
        /// unchanged; `Some` (incl. `Some("")`) sets it.
        result: Option<String>,
    },
}

/// Encode form data entries as `application/x-www-form-urlencoded`.
#[must_use]
pub fn encode_form_urlencoded(data: &[FormDataEntry]) -> String {
    data.iter()
        .map(|entry| {
            format!(
                "{}={}",
                percent_encode(&entry.name),
                percent_encode(&entry.value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Minimal percent-encoding for form data (application/x-www-form-urlencoded).
///
/// Per WHATWG URL §5.2: the application/x-www-form-urlencoded byte serializer
/// outputs bytes `0x2A` (`*`), `0x2D` (`-`), `0x2E` (`.`), `0x30`-`0x39` (`0-9`),
/// `0x41`-`0x5A` (`A-Z`), `0x5F` (`_`), `0x61`-`0x7A` (`a-z`) verbatim.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.bytes() {
        match ch {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(ch as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(char::from(HEX[(ch >> 4) as usize]));
                out.push(char::from(HEX[(ch & 0x0f) as usize]));
            }
        }
    }
    out
}

const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// Form submission attributes extracted from a `<form>` element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormAttrs {
    /// The form action URL.
    pub action: String,
    /// The HTTP method (lowercase: "get" or "post").
    pub method: String,
    /// The encoding type.
    pub enctype: String,
}

/// Read form submission attributes from a `<form>` element.
#[must_use]
pub fn read_form_attrs(dom: &EcsDom, form_entity: Entity) -> FormAttrs {
    dom.world()
        .get::<&elidex_ecs::Attributes>(form_entity)
        .ok()
        .map_or_else(
            || FormAttrs {
                action: String::new(),
                method: "get".to_string(),
                enctype: "application/x-www-form-urlencoded".to_string(),
            },
            |attrs| FormAttrs {
                action: attrs.get("action").unwrap_or("").to_string(),
                method: attrs.get("method").unwrap_or("get").to_ascii_lowercase(),
                enctype: attrs
                    .get("enctype")
                    .unwrap_or("application/x-www-form-urlencoded")
                    .to_string(),
            },
        )
}

/// Run the method-dispatch portion of the form submission algorithm
/// (WHATWG HTML §4.10.22.3 steps 10-11) for `form_entity` submitted by
/// the optional `submitter`.
///
/// Resolves the submitter's **effective method** (§attr-fs-method: the
/// submit button's `formmethod` if present — invalid → GET — otherwise
/// the form's `method`) and branches:
/// - `method=dialog` (step 11): returns [`FormSubmission::Dialog`] with
///   the nearest ancestor `<dialog>` and the submit button's optional
///   value as the result. **Returns `None`** when the form has no
///   ancestor `<dialog>` (step 11.1 silent return — the sole `None`).
/// - otherwise (steps 12+): returns [`FormSubmission::Navigate`] with the
///   collected entry list, the submitter's `formaction` override (if a
///   non-empty submit-button `formaction`), and the normalized method.
///
/// The submitter's name/value is appended to the entry list for the
/// navigate case per HTML §4.10.22.4 / §4.10.22.3 step 5.
#[must_use]
pub fn build_form_submission(
    dom: &EcsDom,
    form_entity: Entity,
    submitter: Option<Entity>,
) -> Option<FormSubmission> {
    let attrs = read_form_attrs(dom, form_entity);
    let method = resolve_effective_method(dom, &attrs.method, submitter);

    if method == "dialog" {
        // Step 11.1: no ancestor <dialog> → silent return (no submission).
        let subject = find_dialog_ancestor(dom, form_entity)?;
        // Steps 11.3-11.5: result = the submit button's optional value
        // (None when the submitter is not a submit button or has no
        // `value` attribute). Image-button (step 11.4 `"x,y"`) coords are
        // not tracked yet (slot `#11-input-image-state`); an image button
        // is a submit button here and falls through to its optional value.
        let result = submitter
            .filter(|&s| is_submit_button(dom, s))
            .and_then(|s| dom.with_attribute(s, "value", |v| v.map(str::to_owned)));
        return Some(FormSubmission::Dialog { subject, result });
    }

    let mut data = collect_form_data(dom, form_entity);
    // Add the submitter's name/value if it's a submit button with a name.
    if let Some(submitter_entity) = submitter {
        if let Ok(fcs) = dom.world().get::<&FormControlState>(submitter_entity) {
            if fcs.kind == FormControlKind::SubmitButton && !fcs.name.is_empty() {
                // A submit button submits its OPTIONAL VALUE — the `value`
                // content attribute's value, or empty when absent (HTML
                // §4.10.5.1.18: "The element's optional value is the value of
                // the element's value attribute, if there is one; otherwise
                // null").  NOT `fcs.value`, which holds the display LABEL: the
                // implementation-defined "Submit" string is substituted there
                // for an empty/absent value attribute (§4.10.5.1.18 "the
                // button's label … otherwise … 'Submit'"), and that label must
                // not leak into the submitted value (e.g. `btn.value = ""`
                // submits empty, not "Submit").
                let value = dom
                    .with_attribute(submitter_entity, "value", |v| v.map(str::to_owned))
                    .unwrap_or_default();
                data.push(FormDataEntry {
                    name: fcs.name.clone(),
                    value,
                });
            }
        }
    }

    // §4.10.22.3 step 12 (action) with the submit button's `formaction`
    // override (a non-empty `formaction` on a submit-button submitter
    // wins over the form's `action`).
    let action = submitter
        .filter(|&s| is_submit_button(dom, s))
        .and_then(|s| {
            dom.with_attribute(s, "formaction", |v| {
                v.filter(|fa| !fa.is_empty()).map(str::to_owned)
            })
        })
        .unwrap_or(attrs.action);

    Some(FormSubmission::Navigate {
        action,
        method,
        enctype: attrs.enctype,
        data,
    })
}

/// Resolve a submitter's **method** per WHATWG HTML §attr-fs-method:
/// if the submitter is a submit button with a `formmethod` attribute,
/// that attribute's state; otherwise the form owner's `method`.
///
/// Both `method` and `formmethod` are enumerated (`get`/`post`/`dialog`)
/// with an invalid-value default of GET; `formmethod` has no missing
/// default (absent → fall through to the form's `method`). Returns the
/// normalized keyword.
fn resolve_effective_method(dom: &EcsDom, form_method: &str, submitter: Option<Entity>) -> String {
    let raw = submitter
        .filter(|&s| is_submit_button(dom, s))
        .and_then(|s| dom.with_attribute(s, "formmethod", |v| v.map(str::to_owned)))
        .unwrap_or_else(|| form_method.to_string());
    match raw.to_ascii_lowercase().as_str() {
        "post" => "post".to_string(),
        "dialog" => "dialog".to_string(),
        // Missing/invalid value default = GET (§attr-fs-method).
        _ => "get".to_string(),
    }
}

/// Find the nearest `<dialog>` ancestor of `entity` (WHATWG HTML
/// §4.10.22.3 step 11.2 "nearest ancestor dialog element"). Mirrors
/// [`find_form_ancestor`].
#[must_use]
pub fn find_dialog_ancestor(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    let mut current = dom.get_parent(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let e = current?;
        let is_dialog = dom
            .world()
            .get::<&TagType>(e)
            .is_ok_and(|t| t.0 == "dialog");
        if is_dialog {
            return Some(e);
        }
        current = dom.get_parent(e);
    }
    None
}

/// Collected form data entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormDataEntry {
    /// The `name` attribute of the control.
    pub name: String,
    /// The value of the control.
    pub value: String,
}

/// Find the nearest `<form>` ancestor of an entity.
#[must_use]
pub fn find_form_ancestor(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    let mut current = Some(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let e = current?;
        let is_form = dom.world().get::<&TagType>(e).is_ok_and(|t| t.0 == "form");
        if is_form {
            return Some(e);
        }
        current = dom.get_parent(e);
    }
    None
}

/// WHATWG HTML §4.10.21.4 `requestSubmit(submitter?)` step 2.1:
/// a non-null submitter must be a submit button — `<button type=submit>`,
/// `<input type=submit>`, or `<input type=image>` (all classified as
/// `FormControlKind::SubmitButton` by `from_element`).
///
/// Caller maps `false` to `TypeError` per spec.
#[must_use]
pub fn is_submit_button(dom: &EcsDom, submitter: Entity) -> bool {
    dom.world()
        .get::<&FormControlState>(submitter)
        .is_ok_and(|fcs| fcs.kind == FormControlKind::SubmitButton)
}

/// WHATWG HTML §4.10.21.4 `requestSubmit(submitter?)` step 2.2:
/// the submitter's form owner must be `form`.  Delegates to the
/// canonical §4.10.18.3 form-owner resolution path
/// ([`resolve_form_owner_public`](crate::radio::resolve_form_owner_public)),
/// which combines the `form="..."` IDREF lookup with the tree-ancestor
/// fallback.  The empty-id edge case (`form` has no `id` or `id=""`)
/// is handled by `find_form_by_id`'s WHATWG DOM §4.2.5 / HTML §3.2.5
/// empty-IDREF short-circuit.
///
/// Caller maps `false` to `NotFoundError` `DOMException` per spec.
#[must_use]
pub fn is_form_owner(dom: &EcsDom, submitter: Entity, form: Entity) -> bool {
    crate::radio::resolve_form_owner_public(dom, submitter) == Some(form)
}

/// Collect form data from all submittable controls in a form.
///
/// Per HTML §4.10.15.3: collect entries from input, select, textarea
/// that have a name, are not disabled, and are submittable.
/// Also collects controls associated via `form` attribute (cross-tree).
#[must_use]
pub fn collect_form_data(dom: &EcsDom, form_entity: Entity) -> Vec<FormDataEntry> {
    let mut entries = Vec::new();
    // Collect descendants.
    walk_form_descendants(
        dom,
        form_entity,
        &mut |c| {
            collect_control_entry(dom, c, &mut entries);
        },
        0,
    );
    // Collect controls associated via `form` attribute (HTML
    // §4.10.15.3).  Empty `id=""` is filtered for symmetry with the
    // submitter-side lookup (WHATWG DOM §4.2.5 / HTML §3.2.5 —
    // empty IDREF is unreachable),
    // so a `<form id="">` does not silently sweep up cross-tree
    // controls that happen to carry `form_owner = Some(String::new())`.
    let form_id = dom
        .world()
        .get::<&elidex_ecs::Attributes>(form_entity)
        .ok()
        .and_then(|a| a.get("id").map(String::from))
        .filter(|s| !s.is_empty());
    if let Some(ref id) = form_id {
        let associated: Vec<Entity> = dom
            .world()
            .query::<(Entity, &FormControlState)>()
            .iter()
            .filter(|(_, fcs)| fcs.form_owner.as_deref() == Some(id.as_str()))
            .map(|(e, _)| e)
            .collect();
        for entity in associated {
            collect_control_entry(dom, entity, &mut entries);
        }
    }
    entries
}

/// Collect a single control's form data entry.
fn collect_control_entry(dom: &EcsDom, entity: Entity, entries: &mut Vec<FormDataEntry>) {
    let Ok(fcs) = dom.world().get::<&FormControlState>(entity) else {
        return;
    };
    if fcs.disabled || fcs.name.is_empty() || !fcs.kind.is_submittable() {
        return;
    }
    match fcs.kind {
        FormControlKind::Checkbox | FormControlKind::Radio => {
            if fcs.checked {
                // HTML §4.10.22.4 "Constructing the entry list" step 7: a
                // checkbox/radio submits the value of its `value` CONTENT
                // ATTRIBUTE **if specified** (including an explicitly empty
                // `value=""`), otherwise the string "on" — NOT the live value
                // (step 10's "value of the field element", used for
                // hidden/text).  The decision keys on attribute PRESENCE, so
                // it reads the attribute directly rather than the
                // `default_value` mirror (which cannot distinguish an absent
                // attribute from a present-but-empty one).  This also keeps a
                // dirty value-mode → checkbox/default-on type change correct:
                // the value IDL is then in default/on mode, so the content
                // attribute — not the dirty-frozen live value — is the
                // submission source, consistent with the IDL `value` getter.
                let value = dom
                    .with_attribute(entity, "value", |v| v.map(str::to_owned))
                    .unwrap_or_else(|| "on".to_string());
                entries.push(FormDataEntry {
                    name: fcs.name.clone(),
                    value,
                });
            }
        }
        FormControlKind::Select if fcs.multiple => {
            // HTML spec: for <select multiple>, submit all selected options.
            for opt in &fcs.options {
                if opt.selected && !opt.disabled {
                    entries.push(FormDataEntry {
                        name: fcs.name.clone(),
                        value: opt.value.clone(),
                    });
                }
            }
        }
        FormControlKind::File => {
            // HTML §4.10.22.4 "Constructing the entry list" step 8: a file
            // control submits its SELECTED FILES as `File` objects — NOT the
            // live value (step 10) and NOT the `value` content attribute,
            // which is inert in filename mode.  Routing file inputs through
            // the general `fcs.value` arm would emit a stale string backing
            // (e.g. `<input type=file value=secret>` seeds `fcs.value` at
            // creation), so submission is carved out here.
            //
            // The selected-files list is not yet modeled
            // (`#11-input-file-shell-staging`); step 8.1 ("no selected files")
            // creates an entry with an empty-name `File`, which this
            // string-valued stopgap represents as an empty value.  When real
            // `File` objects land this extends to step 8.2 (one entry per
            // selected file).
            entries.push(FormDataEntry {
                name: fcs.name.clone(),
                value: String::new(),
            });
        }
        _ => {
            // HTML §4.10.22.4 "Constructing the entry list" step 10: a
            // general control (text, hidden, …) submits "the value of the
            // field element" — the control's VALUE, i.e. its internal state
            // (`fcs.value`), per §4.10.18.1 "A form control's value".
            //
            // This is the LIVE value, NOT the `value` content attribute / the
            // default-mode IDL getter.  For a control dirtied in a value-mode
            // state and then type-changed into a default mode (hidden/submit),
            // §4.10.18.1 says the dirty value flag makes the value IGNORE the
            // default value, so a later default-mode `el.value = x` updates the
            // content attribute (and the IDL getter) while the submitted value
            // stays the dirty live value — getter ≠ submission is intended.
            // (Checkbox/radio are the deliberate exception, handled above per
            // step 7 = the content attribute.)
            entries.push(FormDataEntry {
                name: fcs.name.clone(),
                value: fcs.value.clone(),
            });
        }
    }
}

/// Walk form descendants recursively, calling `visitor` on each entity with
/// a `FormControlState`.
fn walk_form_descendants(
    dom: &EcsDom,
    entity: Entity,
    visitor: &mut dyn FnMut(Entity),
    depth: usize,
) {
    if depth >= MAX_ANCESTOR_DEPTH {
        return;
    }
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        if dom.world().get::<&FormControlState>(c).is_ok() {
            visitor(c);
        }
        walk_form_descendants(dom, c, visitor, depth + 1);
        child = dom.get_next_sibling(c);
    }
}

/// Reset all form controls within a form to their default values
/// (HTML §4.10.21.5).
///
/// `<output>` reset (HTML §4.10.13 step "reset algorithm"): clears the
/// value-mode override (`OutputValueOverride`) and replaces the element's
/// children with a single text node containing the default value
/// (read from `OutputDefaultValue` if explicitly set, otherwise the
/// empty string).  Slot `#11-tags-T2d-interactive`.
pub fn reset_form(dom: &mut EcsDom, form_entity: Entity) {
    let controls: Vec<Entity> = collect_form_entities(dom, form_entity);
    for entity in controls {
        // Distinguish `<output>` controls before borrowing the
        // FormControlState mutably so the same source-of-truth
        // (`fcs.kind`) drives both the per-kind reset and the
        // dispatch decision.  Re-reading `TagType` would add an
        // independent ECS lookup whose answer can drift from
        // `FormControlState::from_element`'s classification.
        let is_output = matches!(
            dom.world().get::<&FormControlState>(entity).map(|s| s.kind),
            Ok(FormControlKind::Output)
        );
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(entity) {
            fcs.reset_value();
        }
        if is_output {
            reset_output_value_mode(dom, entity);
        }
    }
}

/// `<output>` reset hook (HTML §4.10.13).  Clears the value-mode
/// override and re-renders the displayed text from the snapshotted
/// `OutputDefaultValue`.  Pristine default-mode elements (no
/// `OutputValueOverride` ever set) are left untouched: their
/// descendant text content already IS the default per spec, and
/// rewriting children unconditionally would wipe `<output>5</output>`
/// to the empty string when no explicit snapshot exists.
fn reset_output_value_mode(dom: &mut EcsDom, entity: Entity) {
    let was_in_value_mode = dom
        .world_mut()
        .remove_one::<OutputValueOverride>(entity)
        .is_ok();
    if !was_in_value_mode {
        return;
    }
    let default_text = dom
        .world()
        .get::<&OutputDefaultValue>(entity)
        .map(|d| d.0.clone())
        .unwrap_or_default();
    replace_children_with_text(dom, entity, &default_text);
}

/// Drop every child of `entity` and append a single fresh text node
/// containing `text` (skipped if `text` is empty — output's reset to
/// an empty default leaves the element child-less, matching the
/// `textContent.set` behaviour for empty strings).
///
/// No `session.release(child)` call accompanies the removal because
/// `reset_form` runs without a session reference; any session-side
/// identity-map entries for the removed text/comment children become
/// stale and are pruned at the next GC sweep.  This matches the
/// session-less mutation precedent (`update_pattern` /
/// `dom.set_attribute` direct calls).
fn replace_children_with_text(dom: &mut EcsDom, entity: Entity, text: &str) {
    let children: Vec<Entity> = dom.children(entity);
    for child in children {
        let _ = dom.remove_child(entity, child);
    }
    if !text.is_empty() {
        let text_node = dom.create_text(text);
        let _ = dom.append_child(entity, text_node);
    }
}

fn collect_form_entities(dom: &EcsDom, entity: Entity) -> Vec<Entity> {
    let mut result = Vec::new();
    walk_form_descendants(
        dom,
        entity,
        &mut |c| {
            result.push(c);
        },
        0,
    );
    result
}

#[cfg(test)]
#[path = "submit_tests.rs"]
mod tests;

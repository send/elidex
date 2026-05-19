//! Form submission and reset.

use elidex_ecs::{
    EcsDom, Entity, OutputDefaultValue, OutputValueOverride, TagType, MAX_ANCESTOR_DEPTH,
};

use crate::{FormControlKind, FormControlState};

/// Describes a form submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormSubmission {
    /// The form action URL.
    pub action: String,
    /// The HTTP method (GET or POST).
    pub method: String,
    /// The encoding type (e.g. "application/x-www-form-urlencoded").
    pub enctype: String,
    /// The collected form data entries.
    pub data: Vec<FormDataEntry>,
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

/// Build a `FormSubmission` from a form element.
///
/// Optionally includes a `submitter` entity (e.g. the submit button that was
/// clicked), which adds the submitter's name/value to the form data per
/// HTML §4.10.15.3 step 5.
#[must_use]
pub fn build_form_submission(
    dom: &EcsDom,
    form_entity: Entity,
    submitter: Option<Entity>,
) -> FormSubmission {
    let attrs = read_form_attrs(dom, form_entity);
    let mut data = collect_form_data(dom, form_entity);
    // Add the submitter's name/value if it's a submit button with a name.
    if let Some(submitter_entity) = submitter {
        if let Ok(fcs) = dom.world().get::<&FormControlState>(submitter_entity) {
            if fcs.kind == FormControlKind::SubmitButton && !fcs.name.is_empty() {
                data.push(FormDataEntry {
                    name: fcs.name.clone(),
                    value: fcs.value.clone(),
                });
            }
        }
    }
    FormSubmission {
        action: attrs.action,
        method: attrs.method,
        enctype: attrs.enctype,
        data,
    }
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
                entries.push(FormDataEntry {
                    name: fcs.name.clone(),
                    value: if fcs.value.is_empty() {
                        "on".to_string()
                    } else {
                        fcs.value.clone()
                    },
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
        _ => {
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
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn make_form_with_input(dom: &mut EcsDom, name: &str, value: &str) -> (Entity, Entity) {
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("name", name);
        attrs.set("value", value);
        let input = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(form, input);
        (form, input)
    }

    #[test]
    fn find_form_ancestor_direct_parent() {
        let mut dom = EcsDom::new();
        let (form, input) = make_form_with_input(&mut dom, "q", "test");
        assert_eq!(find_form_ancestor(&dom, input), Some(form));
    }

    #[test]
    fn find_form_ancestor_nested() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        let input = dom.create_element("input", {
            let mut a = Attributes::default();
            a.set("name", "q");
            a
        });
        let fcs = FormControlState::from_element("input", &{
            let mut a = Attributes::default();
            a.set("name", "q");
            a
        })
        .unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(div, input);
        let _ = dom.append_child(form, div);
        assert_eq!(find_form_ancestor(&dom, input), Some(form));
    }

    #[test]
    fn no_form_ancestor() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        assert_eq!(find_form_ancestor(&dom, input), None);
    }

    #[test]
    fn collect_form_data_basic() {
        let mut dom = EcsDom::new();
        let (form, _) = make_form_with_input(&mut dom, "q", "hello");
        let data = collect_form_data(&dom, form);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].name, "q");
        assert_eq!(data[0].value, "hello");
    }

    #[test]
    fn collect_form_data_skips_disabled() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("name", "q");
        attrs.set("disabled", "");
        let input = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(form, input);
        let data = collect_form_data(&dom, form);
        assert!(data.is_empty());
    }

    #[test]
    fn collect_form_data_skips_unnamed() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let attrs = Attributes::default();
        let input = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(form, input);
        let data = collect_form_data(&dom, form);
        assert!(data.is_empty());
    }

    #[test]
    fn checkbox_only_submitted_when_checked() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "checkbox");
        attrs.set("name", "agree");
        let cb = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(cb, fcs);
        let _ = dom.append_child(form, cb);

        // Unchecked — not submitted.
        assert!(collect_form_data(&dom, form).is_empty());

        // Check it.
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(cb) {
            fcs.checked = true;
        }
        let data = collect_form_data(&dom, form);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].value, "on");
    }

    #[test]
    fn reset_form_restores_defaults() {
        let mut dom = EcsDom::new();
        let (form, input) = make_form_with_input(&mut dom, "q", "original");
        // Modify value.
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(input) {
            fcs.value = "modified".to_string();
            fcs.dirty_value = true;
        }
        reset_form(&mut dom, form);
        let fcs = dom.world().get::<&FormControlState>(input).unwrap();
        assert_eq!(fcs.value, "original");
        assert!(!fcs.dirty_value);
    }

    #[test]
    fn radio_submitted_when_checked() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "radio");
        attrs.set("name", "color");
        attrs.set("value", "red");
        let r = dom.create_element("input", attrs.clone());
        let mut fcs = FormControlState::from_element("input", &attrs).unwrap();
        fcs.checked = true;
        let _ = dom.world_mut().insert_one(r, fcs);
        let _ = dom.append_child(form, r);
        let data = collect_form_data(&dom, form);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].value, "red");
    }

    #[test]
    fn buttons_not_submitted() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "submit");
        attrs.set("name", "btn");
        let btn = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(btn, fcs);
        let _ = dom.append_child(form, btn);
        let data = collect_form_data(&dom, form);
        assert!(data.is_empty());
    }

    #[test]
    fn password_submitted() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "password");
        attrs.set("name", "pw");
        attrs.set("value", "secret");
        let input = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(form, input);
        let data = collect_form_data(&dom, form);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].value, "secret");
    }

    #[test]
    fn reset_restores_default_checked() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "checkbox");
        attrs.set("name", "x");
        attrs.set("checked", "");
        let cb = dom.create_element("input", attrs.clone());
        let mut fcs = FormControlState::from_element("input", &attrs).unwrap();
        // Uncheck it (user action).
        fcs.checked = false;
        let _ = dom.world_mut().insert_one(cb, fcs);
        let _ = dom.append_child(form, cb);
        reset_form(&mut dom, form);
        // Should restore to default_checked = true.
        assert!(dom.world().get::<&FormControlState>(cb).unwrap().checked);
    }

    #[test]
    fn reset_clears_checked() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "checkbox");
        attrs.set("name", "x");
        let cb = dom.create_element("input", attrs.clone());
        let mut fcs = FormControlState::from_element("input", &attrs).unwrap();
        fcs.checked = true;
        let _ = dom.world_mut().insert_one(cb, fcs);
        let _ = dom.append_child(form, cb);
        reset_form(&mut dom, form);
        assert!(!dom.world().get::<&FormControlState>(cb).unwrap().checked);
    }

    #[test]
    fn encode_form_urlencoded_basic() {
        let data = vec![
            FormDataEntry {
                name: "q".into(),
                value: "hello world".into(),
            },
            FormDataEntry {
                name: "lang".into(),
                value: "en".into(),
            },
        ];
        assert_eq!(encode_form_urlencoded(&data), "q=hello+world&lang=en");
    }

    #[test]
    fn encode_form_urlencoded_special_chars() {
        let data = vec![FormDataEntry {
            name: "key".into(),
            value: "a=b&c".into(),
        }];
        assert_eq!(encode_form_urlencoded(&data), "key=a%3Db%26c");
    }

    #[test]
    fn encode_form_urlencoded_empty() {
        let data: Vec<FormDataEntry> = vec![];
        assert_eq!(encode_form_urlencoded(&data), "");
    }

    #[test]
    fn read_form_attrs_defaults() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let attrs = read_form_attrs(&dom, form);
        assert!(attrs.action.is_empty());
        assert_eq!(attrs.method, "get");
        assert_eq!(attrs.enctype, "application/x-www-form-urlencoded");
    }

    #[test]
    fn read_form_attrs_custom() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("action", "/submit");
        attrs.set("method", "POST");
        let form = dom.create_element("form", attrs);
        let fa = read_form_attrs(&dom, form);
        assert_eq!(fa.action, "/submit");
        assert_eq!(fa.method, "post");
    }

    #[test]
    fn build_form_submission_collects_data() {
        let mut dom = EcsDom::new();
        let mut form_attrs = Attributes::default();
        form_attrs.set("action", "/search");
        form_attrs.set("method", "GET");
        let form = dom.create_element("form", form_attrs);
        let mut input_attrs = Attributes::default();
        input_attrs.set("name", "q");
        input_attrs.set("value", "test");
        let input = dom.create_element("input", input_attrs.clone());
        let fcs = FormControlState::from_element("input", &input_attrs).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(form, input);
        let submission = build_form_submission(&dom, form, None);
        assert_eq!(submission.action, "/search");
        assert_eq!(submission.method, "get");
        assert_eq!(submission.data.len(), 1);
    }

    #[test]
    fn hidden_input_is_submittable() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "hidden");
        attrs.set("name", "csrf");
        attrs.set("value", "token123");
        let input = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.append_child(form, input);
        let data = collect_form_data(&dom, form);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].name, "csrf");
        assert_eq!(data[0].value, "token123");
    }

    #[test]
    fn select_multiple_submits_all_selected() {
        let mut state = FormControlState {
            kind: FormControlKind::Select,
            name: "colors".to_string(),
            multiple: true,
            options: vec![
                crate::SelectOption {
                    text: "R".into(),
                    value: "r".into(),
                    disabled: false,
                    group: None,
                    selected: true,
                },
                crate::SelectOption {
                    text: "G".into(),
                    value: "g".into(),
                    disabled: false,
                    group: None,
                    selected: false,
                },
                crate::SelectOption {
                    text: "B".into(),
                    value: "b".into(),
                    disabled: false,
                    group: None,
                    selected: true,
                },
            ],
            ..FormControlState::default()
        };
        state.value = "r".to_string();
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let sel = dom.create_element("select", Attributes::default());
        let _ = dom.world_mut().insert_one(sel, state);
        let _ = dom.append_child(form, sel);
        let data = collect_form_data(&dom, form);
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].value, "r");
        assert_eq!(data[1].value, "b");
    }

    #[test]
    fn submitter_entry_included() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut btn_attrs = Attributes::default();
        btn_attrs.set("type", "submit");
        btn_attrs.set("name", "action");
        btn_attrs.set("value", "save");
        let btn = dom.create_element("input", btn_attrs.clone());
        let fcs = FormControlState::from_element("input", &btn_attrs).unwrap();
        let _ = dom.world_mut().insert_one(btn, fcs);
        let _ = dom.append_child(form, btn);
        let submission = build_form_submission(&dom, form, Some(btn));
        // Submit buttons are not in the normal data, but the submitter is added.
        assert!(submission
            .data
            .iter()
            .any(|e| e.name == "action" && e.value == "save"));
    }

    #[test]
    fn percent_encode_asterisk() {
        let data = vec![FormDataEntry {
            name: "q".into(),
            value: "a*b".into(),
        }];
        // WHATWG URL §5.2: * (0x2A) is in the unreserved set.
        assert_eq!(encode_form_urlencoded(&data), "q=a*b");
    }

    // ---------------------------------------------------------------
    // is_submit_button / is_form_owner — WHATWG HTML §4.10.21.4 step 2
    // ---------------------------------------------------------------

    fn make_submit_button(dom: &mut EcsDom, parent: Entity) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("type", "submit");
        let btn = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(btn, fcs);
        let _ = dom.append_child(parent, btn);
        btn
    }

    #[test]
    fn is_submit_button_input_type_submit() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let btn = make_submit_button(&mut dom, form);
        assert!(is_submit_button(&dom, btn));
    }

    #[test]
    fn is_submit_button_input_type_text_false() {
        let mut dom = EcsDom::new();
        let (_, input) = make_form_with_input(&mut dom, "q", "test");
        assert!(!is_submit_button(&dom, input));
    }

    #[test]
    fn is_submit_button_no_form_control_state_false() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        assert!(!is_submit_button(&dom, div));
    }

    #[test]
    fn is_form_owner_tree_descendant() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let btn = make_submit_button(&mut dom, form);
        assert!(is_form_owner(&dom, btn, form));
    }

    #[test]
    fn is_form_owner_wrong_form_false() {
        let mut dom = EcsDom::new();
        let form_a = dom.create_element("form", Attributes::default());
        let form_b = dom.create_element("form", Attributes::default());
        let btn = make_submit_button(&mut dom, form_a);
        assert!(!is_form_owner(&dom, btn, form_b));
    }

    #[test]
    fn is_form_owner_cross_tree_via_form_attr() {
        // <form id=login> ... </form>  <button type=submit form="login">
        let mut dom = EcsDom::new();
        let mut form_attrs = Attributes::default();
        form_attrs.set("id", "login");
        let form = dom.create_element("form", form_attrs);
        // Submit button OUTSIDE the form, with form="login" attribute.
        let mut btn_attrs = Attributes::default();
        btn_attrs.set("type", "submit");
        btn_attrs.set("form", "login");
        let btn = dom.create_element("input", btn_attrs.clone());
        let mut fcs = FormControlState::from_element("input", &btn_attrs).unwrap();
        fcs.form_owner = Some("login".to_string());
        let _ = dom.world_mut().insert_one(btn, fcs);
        // Note: btn NOT appended to form.
        assert!(is_form_owner(&dom, btn, form));
    }

    #[test]
    fn is_form_owner_cross_tree_form_has_no_id_false() {
        // Edge case: form attribute path (b) unreachable when form has no id.
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut btn_attrs = Attributes::default();
        btn_attrs.set("type", "submit");
        btn_attrs.set("form", "login");
        let btn = dom.create_element("input", btn_attrs.clone());
        let mut fcs = FormControlState::from_element("input", &btn_attrs).unwrap();
        fcs.form_owner = Some("login".to_string());
        let _ = dom.world_mut().insert_one(btn, fcs);
        // btn detached AND form has no id → no ownership.
        assert!(!is_form_owner(&dom, btn, form));
    }

    #[test]
    fn is_form_owner_cross_tree_empty_id_false() {
        // Edge case: empty id is treated as no id per spec.
        let mut dom = EcsDom::new();
        let mut form_attrs = Attributes::default();
        form_attrs.set("id", "");
        let form = dom.create_element("form", form_attrs);
        let mut btn_attrs = Attributes::default();
        btn_attrs.set("type", "submit");
        btn_attrs.set("form", "");
        let btn = dom.create_element("input", btn_attrs.clone());
        let mut fcs = FormControlState::from_element("input", &btn_attrs).unwrap();
        fcs.form_owner = Some(String::new());
        let _ = dom.world_mut().insert_one(btn, fcs);
        assert!(!is_form_owner(&dom, btn, form));
    }
}

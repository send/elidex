//! Tests for `submit.rs` — form-submission entry construction (§4.10.22.4)
//! and submitter value resolution (§4.10.5.1.18). Split out to keep `submit.rs`
//! under the 1000-line review threshold (sibling-module pattern, cf. `lib_tests.rs`).

use super::*;
use elidex_ecs::{Attributes, EcsDom};

/// Unwrap a [`FormSubmission::Navigate`] (the common case in these tests),
/// returning `(action, method, enctype, data)`.
fn navigate(sub: Option<FormSubmission>) -> (String, String, String, Vec<FormDataEntry>) {
    match sub.expect("expected a submission") {
        FormSubmission::Navigate {
            action,
            method,
            enctype,
            data,
        } => (action, method, enctype, data),
        FormSubmission::Dialog { .. } => panic!("expected Navigate, got Dialog"),
    }
}

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

/// §4.10.22.4 step 8: a file control submits its selected files, NOT the live
/// value / `value` content attribute (inert in filename mode).  Guards the
/// stale-backing leak Codex flagged — `<input type=file value=secret>` seeds
/// `fcs.value` at creation, but submission must emit the empty-file entry
/// (step 8.1, no selected files modeled), not `secret`.
#[test]
fn collect_form_data_file_does_not_submit_value_attribute() {
    let mut dom = EcsDom::new();
    let form = dom.create_element("form", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type", "file");
    attrs.set("name", "f");
    attrs.set("value", "secret");
    let input = dom.create_element("input", attrs.clone());
    let fcs = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(fcs.kind, FormControlKind::File);
    let _ = dom.world_mut().insert_one(input, fcs);
    let _ = dom.append_child(form, input);

    let data = collect_form_data(&dom, form);
    assert_eq!(
        data.len(),
        1,
        "file control still produces an (empty-file) entry"
    );
    assert_eq!(data[0].name, "f");
    assert_eq!(
        data[0].value, "",
        "file submission is the empty selected-files list, not the stale `value` attribute"
    );
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
fn checkbox_submits_value_content_attribute_not_stale_live_value() {
    // HTML §4.10.22.4 step 7: a checkbox submits the value of its
    // `value` content attribute, NOT the live value (step 10's "value of
    // the field element", used for hidden/text).  A dirty value-mode →
    // checkbox/default-on type change decouples the two — the live value
    // is frozen by the dirty flag while the default/on IDL setter updates
    // the content attribute — so submission must follow the content
    // attribute to stay consistent with the IDL `value` getter.
    let mut dom = EcsDom::new();
    let form = dom.create_element("form", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("name", "c");
    attrs.set("value", "secret"); // the `value` content attribute
    let cb = dom.create_element("input", attrs.clone());
    let mut fcs = FormControlState::from_element("input", &attrs).unwrap();
    fcs.checked = true;
    // Diverge the live value from the content attribute (the dirty-frozen
    // live value a default/on type change would leave behind).
    fcs.value = "abc".to_string();
    let _ = dom.world_mut().insert_one(cb, fcs);
    let _ = dom.append_child(form, cb);

    let data = collect_form_data(&dom, form);
    assert_eq!(data.len(), 1);
    assert_eq!(
        data[0].value, "secret",
        "checkbox submits the `value` content attribute, not the stale live value"
    );
}

#[test]
fn checkbox_present_empty_value_attribute_submits_empty_not_on() {
    // HTML §4.10.22.4 step 7: a present-but-empty `value=""` is a
    // *specified* attribute, so it submits "" — NOT the "on" fallback
    // (which applies only when the attribute is absent).
    let mut dom = EcsDom::new();
    let form = dom.create_element("form", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("name", "c");
    attrs.set("value", ""); // explicitly present, empty
    let cb = dom.create_element("input", attrs.clone());
    let mut fcs = FormControlState::from_element("input", &attrs).unwrap();
    fcs.checked = true;
    let _ = dom.world_mut().insert_one(cb, fcs);
    let _ = dom.append_child(form, cb);

    let data = collect_form_data(&dom, form);
    assert_eq!(data.len(), 1);
    assert_eq!(
        data[0].value, "",
        "a present-but-empty value attribute submits empty, not 'on'"
    );
}

#[test]
fn clear_file_value_empties_live_backing() {
    // `file.value = ""` (the filename-mode empty setter → `clear_file_value`)
    // empties the live value backing.  Form submission is now independently
    // decoupled from this backing (§4.10.22.4 step 8 — see
    // `collect_form_data_file_does_not_submit_value_attribute`), so this
    // asserts `clear_file_value`'s remaining job directly on the state.
    let mut attrs = Attributes::default();
    attrs.set("type", "file");
    let mut fcs = FormControlState::from_element("input", &attrs).unwrap();
    fcs.value = "x".to_string(); // stale file backing
    fcs.clear_file_value();
    assert_eq!(fcs.value(), "", "clear_file_value empties the live backing");
}

#[test]
fn submit_button_submitter_submits_value_attribute_not_label() {
    // HTML §4.10.5.1.18: a submit button submits its OPTIONAL VALUE (the
    // `value` content attribute), while the implementation-defined "Submit"
    // string is only the display LABEL substituted for an empty/absent
    // value attribute.  The label must not leak into the submitted value.
    let build = |value_attr: Option<&str>| -> String {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let mut attrs = Attributes::default();
        attrs.set("type", "submit");
        attrs.set("name", "btn");
        if let Some(v) = value_attr {
            attrs.set("value", v);
        }
        let btn = dom.create_element("input", attrs.clone());
        let fcs = FormControlState::from_element("input", &attrs).unwrap();
        // `from_element` substitutes the "Submit" display label into the
        // live value when the value attribute is empty/absent.
        let _ = dom.world_mut().insert_one(btn, fcs);
        let _ = dom.append_child(form, btn);
        let (_, _, _, data) = navigate(build_form_submission(&dom, form, Some(btn)));
        data.into_iter()
            .find(|e| e.name == "btn")
            .map(|e| e.value)
            .expect("submitter entry present")
    };
    // Explicit empty value attribute → submit empty (NOT the "Submit"
    // display label).  This is the regression the IDL `value` setter path
    // would otherwise reintroduce via the content-attribute write.
    assert_eq!(build(Some("")), "");
    // Absent value attribute → submit empty (label is still "Submit").
    assert_eq!(build(None), "");
    // Non-empty value attribute → submit it verbatim.
    assert_eq!(build(Some("Go")), "Go");
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
    let (action, method, _, data) = navigate(build_form_submission(&dom, form, None));
    assert_eq!(action, "/search");
    assert_eq!(method, "get");
    assert_eq!(data.len(), 1);
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
    let mut state = FormControlState::default();
    state.kind = FormControlKind::Select;
    state.name = "colors".to_string();
    state.multiple = true;
    state.options = vec![
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
    ];
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
    let (_, _, _, data) = navigate(build_form_submission(&dom, form, Some(btn)));
    // Submit buttons are not in the normal data, but the submitter is added.
    assert!(data.iter().any(|e| e.name == "action" && e.value == "save"));
}

// ---------------------------------------------------------------
// method=dialog — WHATWG HTML §4.10.22.3 step 11 + §attr-fs-method
// ---------------------------------------------------------------

/// Unwrap a [`FormSubmission::Dialog`], returning `(subject, result)`.
fn dialog(sub: Option<FormSubmission>) -> (Entity, Option<String>) {
    match sub.expect("expected a submission") {
        FormSubmission::Dialog { subject, result } => (subject, result),
        FormSubmission::Navigate { .. } => panic!("expected Dialog, got Navigate"),
    }
}

/// Build `<dialog><form method=M>` with a submit button (optional `value`),
/// returning `(dialog, form, button)`.
fn dialog_form(
    dom: &mut EcsDom,
    form_method: &str,
    btn_formmethod: Option<&str>,
    btn_value: Option<&str>,
) -> (Entity, Entity, Entity) {
    let dlg = dom.create_element("dialog", Attributes::default());
    let mut form_attrs = Attributes::default();
    form_attrs.set("method", form_method);
    let form = dom.create_element("form", form_attrs);
    let _ = dom.append_child(dlg, form);
    let mut btn_attrs = Attributes::default();
    btn_attrs.set("type", "submit");
    btn_attrs.set("name", "go");
    if let Some(fm) = btn_formmethod {
        btn_attrs.set("formmethod", fm);
    }
    if let Some(v) = btn_value {
        btn_attrs.set("value", v);
    }
    let btn = dom.create_element("input", btn_attrs.clone());
    let fcs = FormControlState::from_element("input", &btn_attrs).unwrap();
    let _ = dom.world_mut().insert_one(btn, fcs);
    let _ = dom.append_child(form, btn);
    (dlg, form, btn)
}

#[test]
fn dialog_method_returns_dialog_with_submitter_value() {
    let mut dom = EcsDom::new();
    let (dlg, form, btn) = dialog_form(&mut dom, "dialog", None, Some("ok"));
    let (subject, result) = dialog(build_form_submission(&dom, form, Some(btn)));
    assert_eq!(subject, dlg);
    assert_eq!(result.as_deref(), Some("ok"));
}

#[test]
fn formmethod_dialog_overrides_form_post() {
    let mut dom = EcsDom::new();
    // form method=post, but the submit button's formmethod=dialog wins.
    let (dlg, form, btn) = dialog_form(&mut dom, "post", Some("dialog"), Some("v"));
    let (subject, result) = dialog(build_form_submission(&dom, form, Some(btn)));
    assert_eq!(subject, dlg);
    assert_eq!(result.as_deref(), Some("v"));
}

#[test]
fn dialog_method_with_no_ancestor_dialog_returns_none() {
    let mut dom = EcsDom::new();
    // <form method=dialog> NOT inside any <dialog>.
    let mut form_attrs = Attributes::default();
    form_attrs.set("method", "dialog");
    let form = dom.create_element("form", form_attrs);
    let btn = make_submit_button(&mut dom, form);
    assert!(
        build_form_submission(&dom, form, Some(btn)).is_none(),
        "§4.10.22.3 step 11.1: no ancestor dialog → silent return"
    );
}

#[test]
fn dialog_result_none_when_submitter_has_no_value_attr() {
    let mut dom = EcsDom::new();
    let (_, form, btn) = dialog_form(&mut dom, "dialog", None, None);
    let (_, result) = dialog(build_form_submission(&dom, form, Some(btn)));
    assert_eq!(result, None, "no value attr → returnValue unchanged");
}

#[test]
fn dialog_result_empty_string_when_value_empty() {
    let mut dom = EcsDom::new();
    let (_, form, btn) = dialog_form(&mut dom, "dialog", None, Some(""));
    let (_, result) = dialog(build_form_submission(&dom, form, Some(btn)));
    assert_eq!(
        result.as_deref(),
        Some(""),
        "value=\"\" → set returnValue to \"\""
    );
}

#[test]
fn dialog_result_none_when_no_submitter() {
    let mut dom = EcsDom::new();
    let (dlg, form, _) = dialog_form(&mut dom, "dialog", None, Some("ok"));
    let (subject, result) = dialog(build_form_submission(&dom, form, None));
    assert_eq!(subject, dlg);
    assert_eq!(result, None, "no submitter → result is null");
}

#[test]
fn invalid_form_method_defaults_to_get() {
    let mut dom = EcsDom::new();
    let mut form_attrs = Attributes::default();
    form_attrs.set("method", "frobnicate");
    let form = dom.create_element("form", form_attrs);
    let (_, method, _, _) = navigate(build_form_submission(&dom, form, None));
    assert_eq!(method, "get", "§attr-fs-method: invalid method → GET");
}

#[test]
fn formaction_override_on_submit_button() {
    let mut dom = EcsDom::new();
    let mut form_attrs = Attributes::default();
    form_attrs.set("action", "/form");
    form_attrs.set("method", "post");
    let form = dom.create_element("form", form_attrs);
    let mut btn_attrs = Attributes::default();
    btn_attrs.set("type", "submit");
    btn_attrs.set("formaction", "/override");
    let btn = dom.create_element("input", btn_attrs.clone());
    let fcs = FormControlState::from_element("input", &btn_attrs).unwrap();
    let _ = dom.world_mut().insert_one(btn, fcs);
    let _ = dom.append_child(form, btn);
    let (action, ..) = navigate(build_form_submission(&dom, form, Some(btn)));
    assert_eq!(action, "/override");
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

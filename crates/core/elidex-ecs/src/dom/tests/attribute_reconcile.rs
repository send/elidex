//! Tests for `EcsDom::reconcile_attribute_derived_components` — the
//! attribute-write chokepoint's derived-component reconcile. Covers the
//! `IframeData` re-derive half of slot
//! `#11-derived-component-attr-maintenance`: a generic `setAttribute` on a
//! live `<iframe>` keeps `IframeData` consistent with its content attributes.

use super::*;
use crate::components::{IframeData, IsModalDialog};

/// Build a live `<iframe>` carrying an `IframeData` derived from an initial
/// `src` — the post-parse / post-clone shape the chokepoint reconciles.
fn iframe_with_data(dom: &mut EcsDom) -> Entity {
    let mut attrs = Attributes::default();
    attrs.set("src", "https://a.example/");
    let el = dom.create_element("iframe", attrs);
    let derived = {
        let a = dom.world().get::<&Attributes>(el).unwrap();
        IframeData::from_attributes(&a)
    };
    dom.world_mut().insert_one(el, derived).unwrap();
    el
}

#[test]
fn set_attribute_rederives_iframe_data_src() {
    let mut dom = EcsDom::new();
    let el = iframe_with_data(&mut dom);
    assert_eq!(
        dom.world().get::<&IframeData>(el).unwrap().src.as_deref(),
        Some("https://a.example/")
    );
    // Generic setAttribute("src", …) now re-derives IframeData via the
    // chokepoint reconcile seam (was stale before the fix).
    dom.set_attribute(el, "src", "https://b.example/");
    assert_eq!(
        dom.world().get::<&IframeData>(el).unwrap().src.as_deref(),
        Some("https://b.example/")
    );
}

#[test]
fn remove_attribute_rederives_iframe_data_src_to_none() {
    let mut dom = EcsDom::new();
    let el = iframe_with_data(&mut dom);
    dom.remove_attribute(el, "src");
    assert_eq!(dom.world().get::<&IframeData>(el).unwrap().src, None);
}

#[test]
fn non_src_iframe_attr_write_rederives_whole_struct() {
    let mut dom = EcsDom::new();
    let el = iframe_with_data(&mut dom);
    // A non-src iframe attribute write re-derives the WHOLE struct (not just
    // a single field), so `name` updates while `src` is preserved.
    dom.set_attribute(el, "name", "myframe");
    let ifd = dom.world().get::<&IframeData>(el).unwrap();
    assert_eq!(ifd.name.as_deref(), Some("myframe"));
    assert_eq!(
        ifd.src.as_deref(),
        Some("https://a.example/"),
        "src must be preserved when an unrelated iframe attr changes"
    );
}

#[test]
fn set_attribute_does_not_attach_iframe_data_to_non_iframe() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    // Presence-gated: a non-iframe that receives a `src` write must NOT gain
    // an IframeData component.
    dom.set_attribute(el, "src", "https://a.example/");
    assert!(
        dom.world().get::<&IframeData>(el).is_err(),
        "non-iframe must not gain IframeData from a src write"
    );
}

/// Build a modal `<dialog>` (open attribute + `IsModalDialog` marker) — the
/// shape `showModal()` produces.
fn modal_dialog(dom: &mut EcsDom) -> Entity {
    let mut attrs = Attributes::default();
    attrs.set("open", "");
    let el = dom.create_element("dialog", attrs);
    dom.world_mut().insert_one(el, IsModalDialog).unwrap();
    el
}

#[test]
fn remove_open_attribute_clears_modal_marker() {
    let mut dom = EcsDom::new();
    let el = modal_dialog(&mut dom);
    // Removing `open` (HTML §4.11.4 dialog attribute-change → cleanup steps)
    // drops the modal marker: a dialog cannot be modal while closed.
    dom.remove_attribute(el, "open");
    assert!(
        dom.world().get::<&IsModalDialog>(el).is_err(),
        "removing `open` must clear the modal marker"
    );
}

#[test]
fn non_open_attribute_write_preserves_modal_marker() {
    let mut dom = EcsDom::new();
    let el = modal_dialog(&mut dom);
    // An unrelated attribute write on an open modal dialog must NOT disturb the
    // marker (the clear is gated on the `open` attribute specifically).
    dom.set_attribute(el, "id", "dlg");
    assert!(
        dom.world().get::<&IsModalDialog>(el).is_ok(),
        "non-`open` attribute write must preserve the modal marker"
    );
}

#[test]
fn rewriting_open_attribute_preserves_modal_marker() {
    let mut dom = EcsDom::new();
    let el = modal_dialog(&mut dom);
    // Writing `open` while it stays present (e.g. re-setting the empty value)
    // must NOT clear the marker — only its removal does.
    dom.set_attribute(el, "open", "");
    assert!(
        dom.world().get::<&IsModalDialog>(el).is_ok(),
        "rewriting `open` (still present) must preserve the modal marker"
    );
}

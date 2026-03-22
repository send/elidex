//! Form control initialization: bulk DOM walk and dynamic single-entity attach.

use elidex_ecs::{Attributes, EcsDom, ElementState, Entity, TagType};

use crate::{fieldset, select, FormControlKind, FormControlState};

/// Find the first element with `autofocus` attribute in the DOM tree.
#[must_use]
pub fn find_autofocus_target(dom: &EcsDom) -> Option<Entity> {
    dom.world()
        .query::<(Entity, &FormControlState)>()
        .iter()
        .find(|(_, fcs)| fcs.autofocus && !fcs.disabled)
        .map(|(e, _)| e)
}

/// Walk the DOM tree and attach `FormControlState` to form control elements.
///
/// Must be called after HTML parsing, before first render.
pub fn init_form_controls(dom: &mut EcsDom) {
    let entities: Vec<(Entity, String, Attributes)> = dom
        .world()
        .query::<(Entity, &TagType, &Attributes)>()
        .iter()
        .map(|(entity, tag, attrs)| (entity, tag.0.clone(), attrs.clone()))
        .collect();

    for (entity, tag, attrs) in &entities {
        if let Some(mut state) = FormControlState::from_element(tag, attrs) {
            finalize_control(dom, *entity, &mut state);
            let _ = dom.world_mut().insert_one(*entity, state);
        }
    }

    // Propagate fieldset disabled to descendant controls.
    fieldset::propagate_fieldset_disabled(dom);
}

/// Attach a `FormControlState` to a single entity (for dynamic JS element creation).
///
/// Returns `true` if a form control state was created and attached.
/// This is the single-entity equivalent of `init_form_controls()`.
pub fn create_form_control_state(dom: &mut EcsDom, entity: Entity) -> bool {
    let Some(tag) = dom
        .world()
        .get::<&TagType>(entity)
        .ok()
        .map(|t| t.0.clone())
    else {
        return false;
    };
    let attrs = dom
        .world()
        .get::<&Attributes>(entity)
        .ok()
        .map_or_else(Attributes::default, |a| (*a).clone());

    let Some(mut state) = FormControlState::from_element(&tag, &attrs) else {
        return false;
    };

    finalize_control(dom, entity, &mut state);
    // Check ancestor fieldset disabled state and propagate to this control.
    if !state.disabled && fieldset::is_fieldset_disabled(entity, dom) {
        state.disabled = true;
        if let Ok(mut es) = dom.world_mut().get::<&mut ElementState>(entity) {
            es.insert(ElementState::DISABLED);
        }
    }
    let _ = dom.world_mut().insert_one(entity, state);
    true
}

/// Apply post-creation initialization: textarea text content, select options, element state.
fn finalize_control(dom: &mut EcsDom, entity: Entity, state: &mut FormControlState) {
    // For <textarea>, read initial value from first child text content.
    if state.kind == FormControlKind::TextArea {
        if let Some(text) = first_child_text(dom, entity) {
            // HTML §4.10.7.2: strip one leading newline from textarea content.
            let text = text.strip_prefix('\n').unwrap_or(&text).to_string();
            state.set_value_initial(text);
        }
    }
    // For <select>, walk child <option>/<optgroup> elements.
    if state.kind == FormControlKind::Select {
        select::init_select_options(dom, entity, state);
    }
    // Set ElementState flags for CSS pseudo-class matching.
    apply_element_state_flags(dom, entity, state);
}

/// Set `ElementState` flags on the entity based on form control state.
fn apply_element_state_flags(dom: &mut EcsDom, entity: Entity, state: &FormControlState) {
    let mut elem_state = dom
        .world()
        .get::<&ElementState>(entity)
        .ok()
        .map_or(ElementState::default(), |s| *s);
    if state.disabled {
        elem_state.insert(ElementState::DISABLED);
    }
    if state.checked {
        elem_state.insert(ElementState::CHECKED);
    }
    if state.required {
        elem_state.insert(ElementState::REQUIRED);
    }
    if state.readonly {
        elem_state.insert(ElementState::READ_ONLY);
    }
    // All form controls start valid (validation runs later).
    elem_state.insert(ElementState::VALID);
    let _ = dom.world_mut().insert_one(entity, elem_state);
}

/// Get the text content of the first child text node (for textarea initial value).
fn first_child_text(dom: &EcsDom, entity: Entity) -> Option<String> {
    let child = dom.get_first_child(entity)?;
    dom.world()
        .get::<&elidex_ecs::TextContent>(child)
        .ok()
        .map(|tc| tc.0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::EcsDom;

    fn make_dom_with_input(tag: &str, attrs: &[(&str, &str)]) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let mut attr_map = Attributes::default();
        for (k, v) in attrs {
            attr_map.set(k.to_string(), v.to_string());
        }
        let entity = dom.create_element(tag, attr_map);
        (dom, entity)
    }

    #[test]
    fn init_form_controls_attaches_state() {
        let (mut dom, entity) = make_dom_with_input("input", &[("type", "text"), ("value", "hi")]);
        init_form_controls(&mut dom);
        let state = dom.world().get::<&FormControlState>(entity).unwrap();
        assert_eq!(state.kind, FormControlKind::TextInput);
        assert_eq!(state.value, "hi");
    }

    #[test]
    fn init_form_controls_textarea_with_text() {
        let mut dom = EcsDom::new();
        let ta = dom.create_element("textarea", Attributes::default());
        let text_node = dom.create_text("Hello");
        let _ = dom.append_child(ta, text_node);
        init_form_controls(&mut dom);
        let state = dom.world().get::<&FormControlState>(ta).unwrap();
        assert_eq!(state.value, "Hello");
        assert_eq!(state.cursor_pos, 5);
    }

    #[test]
    fn textarea_strips_leading_newline() {
        let mut dom = EcsDom::new();
        let ta = dom.create_element("textarea", Attributes::default());
        let text_node = dom.create_text("\nHello");
        let _ = dom.append_child(ta, text_node);
        init_form_controls(&mut dom);
        let state = dom.world().get::<&FormControlState>(ta).unwrap();
        assert_eq!(state.value, "Hello");
    }

    #[test]
    fn textarea_preserves_non_leading_newline() {
        let mut dom = EcsDom::new();
        let ta = dom.create_element("textarea", Attributes::default());
        let text_node = dom.create_text("Hello\nWorld");
        let _ = dom.append_child(ta, text_node);
        init_form_controls(&mut dom);
        let state = dom.world().get::<&FormControlState>(ta).unwrap();
        assert_eq!(state.value, "Hello\nWorld");
    }

    #[test]
    fn create_form_control_state_attaches_state() {
        let (mut dom, entity) =
            make_dom_with_input("input", &[("type", "text"), ("value", "hello")]);
        assert!(create_form_control_state(&mut dom, entity));
        let fcs = dom.world().get::<&FormControlState>(entity).unwrap();
        assert_eq!(fcs.kind, FormControlKind::TextInput);
        assert_eq!(fcs.value, "hello");
    }

    #[test]
    fn create_form_control_state_returns_false_for_div() {
        let (mut dom, entity) = make_dom_with_input("div", &[]);
        assert!(!create_form_control_state(&mut dom, entity));
    }
}

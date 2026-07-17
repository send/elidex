//! Form control state and input handling for elidex.
//!
//! Provides `FormControlState` ECS component for tracking the runtime state
//! of HTML form controls (`<input>`, `<button>`, `<textarea>`, `<label>`).
//! The `.value` property is distinct from the `value` HTML attribute per spec.
//!
//! The `FormControlState` component, its value model, and the pure
//! derivations (constraint validation / fieldset disabled-ancestry /
//! sanitization / step + datetime primitives / the `value` IDL value-mode
//! cluster / the selection + clipboard value-model helpers) live in the
//! engine-independent leaf crate `elidex-form-core` and are re-exported here
//! (see below) so `elidex_form::X` keeps resolving unchanged.
//! This crate keeps the higher *systems* (reconciler / init /
//! radio / submit / select / fieldset push-propagation / value-mode
//! migration) that need `&mut EcsDom` / `elidex-dom-api` / `elidex-script-session`.

pub mod ancestor_cache;
mod clone;
mod fieldset;
pub mod focus_snapshot;
mod inert_document;
mod init;
mod label;
pub mod radio;
mod reconciler;
mod select;
mod sizing;
mod submit;
mod value_mode;

// --- Re-export the elidex-form-core leaf crate (Slice 0a carve) ---
// Every form-core public item is re-exported so `elidex_form::X` keeps
// resolving unchanged for downstream crates; the crate split is
// source-transparent to consumers.
pub use elidex_form_core::util;
pub use elidex_form_core::{
    apply_step, clipboard_copy, clipboard_cut, clipboard_paste, collapse_selection,
    delete_selection, extend_selection, first_legend_child, form_control_key_input,
    form_control_key_input_action, is_constraint_validation_candidate, is_fieldset_disabled,
    is_in_first_legend, resolve_input_list, sanitize_for_type_change, select_all, validate_control,
    FormControlKind, FormControlState, KeyAction, SelectOption, SelectionDirection, StepError,
    ValidityState, ValueMode, ValueSetAction, MAX_PATTERN_LENGTH,
};
// `compile_pattern_regex` is `pub` in form-core, but stays crate-internal
// here (the reconciler `pattern`-arm calls it) — not elidex-form public API.
pub(crate) use elidex_form_core::compile_pattern_regex;

pub use ancestor_cache::AncestorCache;
pub use clone::apply_clone_form_state;
pub use fieldset::propagate_fieldset_disabled;
pub use focus_snapshot::{
    clear_focus_snapshot, record_focus_snapshot, take_focus_snapshot, FocusValueSnapshot,
};
pub use inert_document::parse_into_inert_document;
pub use init::{create_form_control_state, find_autofocus_target, init_form_controls};
pub use label::{find_label_target, is_label, is_labelable_element, resolve_label_for};
pub use radio::{
    find_radio_group, find_radio_group_scoped, is_radio_group_satisfied, radio_arrow_navigate,
    toggle_radio,
};
pub use reconciler::FormControlReconciler;
pub use select::{
    find_option_index_in_tree, find_option_select, init_select_options, is_option_disabled,
    navigate_select, option_value_string, select_get_value, select_option, select_selected_index,
    select_set_selected_index, select_set_value, select_uses_implicit_default,
};
pub use sizing::form_intrinsic_size;
pub use submit::{
    build_form_submission, collect_form_data, encode_form_urlencoded, find_dialog_ancestor,
    find_form_ancestor, is_form_owner, is_submit_button, read_form_attrs, reset_form, FormAttrs,
    FormDataEntry, FormSubmission,
};

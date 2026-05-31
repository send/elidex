//! WHATWG HTML §13.2.6.4 "The rules for parsing tokens in HTML content" — one
//! submodule per insertion mode (§13.2.6.4.1–.21).
//!
//! Each mode exposes a single `pub(crate) fn <mode>(tb, token) ->
//! Result<Flow, StrictParseError>`. Only the conforming branches are
//! implemented: any token that the spec routes through an error-recovery
//! branch (foster parenting, adoption agency, implicit misnested-tag closing,
//! stray/out-of-place tags) aborts with [`crate::StrictParseError`]. The
//! dispatch is the single `match` in [`super::TreeBuilder::dispatch`].

pub(super) mod after_after_body;
pub(super) mod after_after_frameset;
pub(super) mod after_body;
pub(super) mod after_frameset;
pub(super) mod after_head;
pub(super) mod before_head;
pub(super) mod before_html;
pub(super) mod in_body;
pub(super) mod in_caption;
pub(super) mod in_cell;
pub(super) mod in_column_group;
pub(super) mod in_frameset;
pub(super) mod in_head;
pub(super) mod in_head_noscript;
pub(super) mod in_row;
pub(super) mod in_table;
pub(super) mod in_table_body;
pub(super) mod in_table_text;
pub(super) mod in_template;
pub(super) mod initial;
pub(super) mod text;

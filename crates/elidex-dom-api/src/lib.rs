// The DomApiHandler/CssomApiHandler traits use `&str` return for method_name(),
// which clippy flags when implementations return string literals. The trait
// signature can't be changed here, so suppress this lint.
#![allow(clippy::unnecessary_literal_bound)]

//! DOM API handler implementations for elidex.
//!
//! This crate provides concrete implementations of the [`DomApiHandler`] and
//! [`CssomApiHandler`] traits defined in `elidex-script-session`. It is
//! engine-independent — no dependency on boa or any other JS engine.
//!
//! [`DomApiHandler`]: elidex_script_session::DomApiHandler
//! [`CssomApiHandler`]: elidex_script_session::CssomApiHandler

pub mod class_list;
pub mod computed_style;
pub mod document;
pub mod element;
pub mod registry;
pub mod style;
pub(crate) mod util;

// Re-export handlers for convenient access.
pub use class_list::{ClassListAdd, ClassListContains, ClassListRemove, ClassListToggle};
pub use computed_style::GetComputedStyle;
pub use document::{
    query_selector_all, CreateElement, CreateTextNode, GetElementById, QuerySelector,
};
pub use element::{
    collect_text_content, serialize_inner_html, AppendChild, GetAttribute, GetInnerHtml,
    GetTextContent, InsertBefore, RemoveAttribute, RemoveChild, SetAttribute, SetTextContent,
};
pub use style::{StyleGetPropertyValue, StyleRemoveProperty, StyleSetProperty};

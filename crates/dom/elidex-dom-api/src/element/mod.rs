//! Element-level DOM API handlers: appendChild, insertBefore, removeChild,
//! getAttribute/setAttribute/removeAttribute, textContent, innerHTML.

mod attrs;
pub mod enumerated_reflect;
pub mod href_accessor;
mod inheritance;
pub(crate) mod layout_query;
pub mod numeric_reflect;
mod option_disabled;
mod props;
pub(crate) mod tree;

pub use attrs::{camel_to_data_attr, data_attr_to_camel};
pub use attrs::{
    DatasetDelete, DatasetGet, DatasetKeys, DatasetSet, GetAttributeNames, GetClassName, GetId,
    HasAttribute, SetClassName, SetId, ToggleAttribute,
};
pub use inheritance::is_content_editable;
pub use layout_query::{
    GetBoundingClientRect, GetClientHeight, GetClientLeft, GetClientRects, GetClientTop,
    GetClientWidth, GetOffsetHeight, GetOffsetLeft, GetOffsetParent, GetOffsetTop, GetOffsetWidth,
    GetScrollHeight, GetScrollLeft, GetScrollTop, GetScrollWidth, ScrollIntoView,
};
pub use option_disabled::is_option_disabled;
pub use props::{GetAttribute, RemoveAttribute, SetAttribute};
pub use tree::{
    collect_text_content, serialize_inner_html, validate_attribute_name, AppendChild, GetInnerHtml,
    InsertAdjacentElement, InsertAdjacentHtml, InsertAdjacentText, InsertBefore, RemoveChild,
    ReplaceChild, SetInnerHtml,
};

// Tests are split by category to keep each file under the 1000-line
// convention.  `unused_must_use` is allowed because individual test
// bodies invoke mutation handlers (append_child / set_attribute /
// dataset.set / etc.) without binding the `Result` they return —
// the assertion target is always observed dom state, not the handler
// return value.
#[cfg(test)]
#[allow(unused_must_use)]
mod tests_attrs;
#[cfg(test)]
#[allow(unused_must_use)]
mod tests_tree;

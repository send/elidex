//! Element-level DOM API handlers: appendChild, insertBefore, removeChild,
//! getAttribute/setAttribute/removeAttribute, textContent, innerHTML.

mod attrs;
mod inheritance;
pub(crate) mod layout_query;
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

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests;

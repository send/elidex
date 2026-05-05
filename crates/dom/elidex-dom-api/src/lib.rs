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

pub mod char_data;
pub mod child_node;
pub mod class_list;
pub mod computed_style;
pub mod document;
pub mod element;
pub mod live_collection;
pub mod node_methods;
pub mod range;
pub mod registry;
pub mod style;
pub mod traversal;
pub mod tree_nav;
pub(crate) mod util;

// Re-export handlers for convenient access.
pub use char_data::{
    AppendData, CreateAttribute as CreateAttributeHandler, CreateComment as CreateCommentHandler,
    CreateDocumentFragment as CreateDocumentFragmentHandler, DeleteData, GetAttrName,
    GetAttrSpecified, GetAttrValue, GetAttributeNode, GetBody, GetCharacterSet, GetCompatMode,
    GetData, GetDoctype, GetDoctypeName, GetDoctypePublicId, GetDoctypeSystemId,
    GetDocumentElement, GetDocumentUrl, GetHead, GetLength, GetOwnerElement, GetReadyState,
    GetTitle, InsertData, RemoveAttributeNode, ReplaceData, SetAttrValue, SetAttributeNode,
    SetData, SetTitle, SplitText, SubstringData,
};
pub use child_node::{
    After, Append, Before, ChildNodeRemove, Closest, Matches, Prepend, ReplaceChildren, ReplaceWith,
};
pub use class_list::{
    ClassListAdd, ClassListContains, ClassListItem, ClassListLength, ClassListRemove,
    ClassListReplace, ClassListSupports, ClassListToggle, ClassListValueGet, ClassListValueSet,
};
pub use computed_style::{css_value_to_string, GetComputedStyle};
pub use document::{
    query_selector_all, CreateElement, CreateTextNode, GetElementById, QuerySelector,
};
pub use element::{
    camel_to_data_attr, collect_text_content, data_attr_to_camel, serialize_inner_html,
    validate_attribute_name, AppendChild, DatasetDelete, DatasetGet, DatasetKeys, DatasetSet,
    GetAttribute, GetAttributeNames, GetBoundingClientRect, GetClassName, GetClientHeight,
    GetClientLeft, GetClientRects, GetClientTop, GetClientWidth, GetId, GetInnerHtml,
    GetOffsetHeight, GetOffsetLeft, GetOffsetParent, GetOffsetTop, GetOffsetWidth, GetScrollHeight,
    GetScrollLeft, GetScrollTop, GetScrollWidth, HasAttribute, InsertAdjacentElement,
    InsertAdjacentHtml, InsertAdjacentText, InsertBefore, RemoveAttribute, RemoveChild,
    ReplaceChild, ScrollIntoView, SetAttribute, SetClassName, SetId, SetInnerHtml, ToggleAttribute,
};
pub use node_methods::{
    CloneNode, CompareDocumentPosition, Contains, GetRootNode, GetTextContentNodeKind, IsConnected,
    IsEqualNode, IsSameNode, Normalize, OwnerDocument, SetNodeValue, SetTextContentNodeKind,
};
pub use range::{adjust_ranges_for_removal, adjust_ranges_for_text_change, Range};
pub use style::{StyleGetPropertyValue, StyleRemoveProperty, StyleSetProperty};
pub use traversal::{
    NodeIterator, TreeWalker, SHOW_ALL, SHOW_COMMENT, SHOW_DOCUMENT, SHOW_ELEMENT, SHOW_TEXT,
};
pub use tree_nav::{
    GetChildElementCount, GetFirstChild, GetFirstElementChild, GetLastChild, GetLastElementChild,
    GetNextElementSibling, GetNextSibling, GetNodeName, GetNodeType, GetNodeValue,
    GetParentElement, GetParentNode, GetPrevElementSibling, GetPrevSibling, GetTagName,
    HasChildNodes,
};

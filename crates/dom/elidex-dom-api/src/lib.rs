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
pub mod css_namespace;
pub mod cssom_sheet;
pub mod document;
pub mod element;
pub mod consumer_dispatcher;
pub mod live_collection;
pub mod node_methods;
pub mod range;
pub mod registry;
pub mod selection;
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
    TokenListHandler, TokenListOp, CLASS_LIST_ADD, CLASS_LIST_CONTAINS, CLASS_LIST_ITEM,
    CLASS_LIST_LENGTH, CLASS_LIST_REMOVE, CLASS_LIST_REPLACE, CLASS_LIST_SUPPORTS,
    CLASS_LIST_TOGGLE, CLASS_LIST_VALUE_GET, CLASS_LIST_VALUE_SET, LINK_SIZES_ADD,
    LINK_SIZES_CONTAINS, LINK_SIZES_ITEM, LINK_SIZES_LENGTH, LINK_SIZES_REMOVE, LINK_SIZES_REPLACE,
    LINK_SIZES_SUPPORTS, LINK_SIZES_TOGGLE, LINK_SIZES_VALUE_GET, LINK_SIZES_VALUE_SET,
    OUTPUT_HTML_FOR_ADD, OUTPUT_HTML_FOR_CONTAINS, OUTPUT_HTML_FOR_ITEM, OUTPUT_HTML_FOR_LENGTH,
    OUTPUT_HTML_FOR_REMOVE, OUTPUT_HTML_FOR_REPLACE, OUTPUT_HTML_FOR_SUPPORTS,
    OUTPUT_HTML_FOR_TOGGLE, OUTPUT_HTML_FOR_VALUE_GET, OUTPUT_HTML_FOR_VALUE_SET, REL_LIST_ADD,
    REL_LIST_CONTAINS, REL_LIST_ITEM, REL_LIST_LENGTH, REL_LIST_REMOVE, REL_LIST_REPLACE,
    REL_LIST_SUPPORTS, REL_LIST_TOGGLE, REL_LIST_VALUE_GET, REL_LIST_VALUE_SET,
};
pub use computed_style::{css_value_to_string, GetComputedStyle};
pub use css_namespace::{CssEscape, CssSupports};
pub use cssom_sheet::{
    collect_stylesheet_owners, count_stylesheet_owners, CssRulesItemId, CssRulesLength, DeleteRule,
    InsertRule, RuleCssText, RuleSelectorText, RuleStyleCssText, RuleStyleGetPropertyValue,
    RuleStyleItem, RuleStyleLength,
};
pub use document::{
    query_selector_all, CreateElement, CreateTextNode, GetElementById, QuerySelector,
};
pub use element::href_accessor::{
    HyperlinkHashGet, HyperlinkHashSet, HyperlinkHostGet, HyperlinkHostSet, HyperlinkHostnameGet,
    HyperlinkHostnameSet, HyperlinkHrefGet, HyperlinkHrefSet, HyperlinkOriginGet,
    HyperlinkPasswordGet, HyperlinkPasswordSet, HyperlinkPathnameGet, HyperlinkPathnameSet,
    HyperlinkPortGet, HyperlinkPortSet, HyperlinkProtocolGet, HyperlinkProtocolSet,
    HyperlinkSearchGet, HyperlinkSearchSet, HyperlinkToString, HyperlinkUsernameGet,
    HyperlinkUsernameSet,
};
pub use element::{
    camel_to_data_attr, collect_text_content, data_attr_to_camel, serialize_inner_html,
    serialize_inner_html_with_options, serialize_outer_html, validate_attribute_name, AppendChild,
    DatasetDelete, DatasetGet, DatasetKeys, DatasetSet, GetAttribute, GetAttributeNames,
    GetBoundingClientRect, GetClassName, GetClientHeight, GetClientLeft, GetClientRects,
    GetClientTop, GetClientWidth, GetId, GetInnerHtml, GetOffsetHeight, GetOffsetLeft,
    GetOffsetParent, GetOffsetTop, GetOffsetWidth, GetScrollHeight, GetScrollLeft, GetScrollTop,
    GetScrollWidth, HasAttribute, InsertAdjacentElement, InsertAdjacentHtml, InsertAdjacentText,
    InsertBefore, RemoveAttribute, RemoveChild, ReplaceChild, ScrollIntoView, SerializeOptions,
    SetAttribute, SetClassName, SetId, SetInnerHtml, ToggleAttribute,
};
pub use consumer_dispatcher::ConsumerDispatcher;
pub use element::document_base::{compute_frozen_url, BaseUrlMaintainer};
pub use live_collection::{CollectionFilter, CollectionKind, LiveCollection};
pub use node_methods::{
    CloneNode, CompareDocumentPosition, Contains, GetRootNode, GetTextContentNodeKind, IsConnected,
    IsEqualNode, IsSameNode, Normalize, OwnerDocument, SetNodeValue, SetTextContentNodeKind,
};
pub use range::{
    adjust_ranges_for_removal, adjust_ranges_for_text_change, LiveRangeBridge, LiveRangeRegistry,
    Range, RangeId, RangePointError, END_TO_END, END_TO_START, START_TO_END, START_TO_START,
};
// `adjust_ranges_for_insertion` is intentionally NOT re-exported yet —
// it is a forward-stub for D-8 PR-A's `LiveRangeRegistry` (an in-crate
// consumer reachable via `crate::range::adjust_ranges_for_insertion`).
// PR-A will widen the visibility to `pub use` if/when an external
// consumer needs it, keeping the public API surface minimal until
// then.
pub use selection::{SelectionDirection, SelectionError, SelectionState, SelectionType};
pub use style::{StyleGetPropertyValue, StyleRemoveProperty, StyleSetProperty};
pub use traversal::{
    NodeIterator, NodeIteratorAdjuster, NodeIteratorState, TreeWalker, SHOW_ALL, SHOW_COMMENT,
    SHOW_DOCUMENT, SHOW_ELEMENT, SHOW_TEXT,
};
pub use tree_nav::{
    GetChildElementCount, GetFirstChild, GetFirstElementChild, GetLastChild, GetLastElementChild,
    GetNextElementSibling, GetNextSibling, GetNodeName, GetNodeType, GetNodeValue,
    GetParentElement, GetParentNode, GetPrevElementSibling, GetPrevSibling, GetTagName,
    HasChildNodes,
};

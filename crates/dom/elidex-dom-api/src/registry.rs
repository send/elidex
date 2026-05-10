//! Pre-populated registries for standard DOM/CSSOM API handlers.
//!
//! These factory functions create registries that contain all built-in handlers,
//! enabling engine-agnostic dispatch by name rather than direct handler references.

use elidex_plugin::PluginRegistry;
use elidex_script_session::{CssomApiHandler, DomApiHandler};

/// Type alias for a registry of DOM API handlers.
pub type DomHandlerRegistry = PluginRegistry<dyn DomApiHandler>;

/// Type alias for a registry of CSSOM API handlers.
pub type CssomHandlerRegistry = PluginRegistry<dyn CssomApiHandler>;

/// Create a registry pre-populated with all standard DOM API handlers.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn create_dom_registry() -> DomHandlerRegistry {
    let mut r: DomHandlerRegistry = PluginRegistry::new();

    // --- Document ---
    r.register_static("querySelector", Box::new(super::QuerySelector));
    r.register_static("getElementById", Box::new(super::GetElementById));
    r.register_static("createElement", Box::new(super::CreateElement));
    r.register_static("createTextNode", Box::new(super::CreateTextNode));
    r.register_static(
        "createDocumentFragment",
        Box::new(super::char_data::CreateDocumentFragment),
    );
    r.register_static("createComment", Box::new(super::char_data::CreateComment));
    r.register_static(
        "createAttribute",
        Box::new(super::char_data::CreateAttribute),
    );

    // --- Document properties ---
    r.register_static("document.URL.get", Box::new(super::GetDocumentUrl));
    r.register_static("document.readyState.get", Box::new(super::GetReadyState));
    r.register_static("document.compatMode.get", Box::new(super::GetCompatMode));
    r.register_static(
        "document.characterSet.get",
        Box::new(super::GetCharacterSet),
    );
    r.register_static(
        "document.documentElement.get",
        Box::new(super::GetDocumentElement),
    );
    r.register_static("document.head.get", Box::new(super::GetHead));
    r.register_static("document.body.get", Box::new(super::GetBody));
    r.register_static("document.title.get", Box::new(super::GetTitle));
    r.register_static("document.title.set", Box::new(super::SetTitle));
    r.register_static("doctype.get", Box::new(super::GetDoctype));
    r.register_static("doctype.name.get", Box::new(super::GetDoctypeName));
    r.register_static("doctype.publicId.get", Box::new(super::GetDoctypePublicId));
    r.register_static("doctype.systemId.get", Box::new(super::GetDoctypeSystemId));

    // --- Element — child mutations ---
    r.register_static("appendChild", Box::new(super::AppendChild));
    r.register_static("insertBefore", Box::new(super::InsertBefore));
    r.register_static("removeChild", Box::new(super::RemoveChild));
    r.register_static("replaceChild", Box::new(super::ReplaceChild));

    // --- Element — attributes ---
    r.register_static("getAttribute", Box::new(super::GetAttribute));
    r.register_static("setAttribute", Box::new(super::SetAttribute));
    r.register_static("removeAttribute", Box::new(super::RemoveAttribute));
    r.register_static("hasAttribute", Box::new(super::HasAttribute));
    r.register_static("toggleAttribute", Box::new(super::ToggleAttribute));
    r.register_static("getAttributeNames", Box::new(super::GetAttributeNames));
    r.register_static("className.get", Box::new(super::GetClassName));
    r.register_static("className.set", Box::new(super::SetClassName));
    r.register_static("id.get", Box::new(super::GetId));
    r.register_static("id.set", Box::new(super::SetId));

    // --- Attr node ---
    r.register_static("getAttributeNode", Box::new(super::GetAttributeNode));
    r.register_static("setAttributeNode", Box::new(super::SetAttributeNode));
    r.register_static("removeAttributeNode", Box::new(super::RemoveAttributeNode));
    r.register_static("attr.name.get", Box::new(super::GetAttrName));
    r.register_static("attr.value.get", Box::new(super::GetAttrValue));
    r.register_static("attr.value.set", Box::new(super::SetAttrValue));
    r.register_static("attr.ownerElement.get", Box::new(super::GetOwnerElement));
    r.register_static("attr.specified.get", Box::new(super::GetAttrSpecified));

    // --- Element — content ---
    r.register_static("textContent.get", Box::new(super::GetTextContentNodeKind));
    r.register_static("textContent.set", Box::new(super::SetTextContentNodeKind));
    r.register_static("innerHTML.get", Box::new(super::GetInnerHtml));
    r.register_static("innerHTML.set", Box::new(super::SetInnerHtml));
    r.register_static("insertAdjacentHTML", Box::new(super::InsertAdjacentHtml));

    // --- Layout query ---
    r.register_static(
        "getBoundingClientRect",
        Box::new(super::GetBoundingClientRect),
    );
    r.register_static("offsetWidth.get", Box::new(super::GetOffsetWidth));
    r.register_static("offsetHeight.get", Box::new(super::GetOffsetHeight));
    r.register_static("offsetTop.get", Box::new(super::GetOffsetTop));
    r.register_static("offsetLeft.get", Box::new(super::GetOffsetLeft));
    r.register_static("offsetParent.get", Box::new(super::GetOffsetParent));
    r.register_static("clientWidth.get", Box::new(super::GetClientWidth));
    r.register_static("clientHeight.get", Box::new(super::GetClientHeight));
    r.register_static("clientTop.get", Box::new(super::GetClientTop));
    r.register_static("clientLeft.get", Box::new(super::GetClientLeft));
    r.register_static("scrollWidth.get", Box::new(super::GetScrollWidth));
    r.register_static("scrollHeight.get", Box::new(super::GetScrollHeight));
    r.register_static("scrollTop.get", Box::new(super::GetScrollTop));
    r.register_static("scrollLeft.get", Box::new(super::GetScrollLeft));
    r.register_static("getClientRects", Box::new(super::GetClientRects));
    r.register_static("scrollIntoView", Box::new(super::ScrollIntoView));

    // --- Element — insertAdjacent ---
    r.register_static(
        "insertAdjacentElement",
        Box::new(super::InsertAdjacentElement),
    );
    r.register_static("insertAdjacentText", Box::new(super::InsertAdjacentText));

    // --- Dataset ---
    r.register_static("dataset.get", Box::new(super::DatasetGet));
    r.register_static("dataset.set", Box::new(super::DatasetSet));
    r.register_static("dataset.delete", Box::new(super::DatasetDelete));
    r.register_static("dataset.keys", Box::new(super::DatasetKeys));

    // --- Style (CSSStyleDeclaration §6.6) ---
    r.register_static("style.setProperty", Box::new(super::StyleSetProperty));
    r.register_static(
        "style.getPropertyValue",
        Box::new(super::StyleGetPropertyValue),
    );
    r.register_static("style.removeProperty", Box::new(super::StyleRemoveProperty));
    r.register_static("style.length", Box::new(super::style::StyleLength));
    r.register_static("style.item", Box::new(super::style::StyleItem));
    r.register_static("style.cssText.get", Box::new(super::style::StyleCssTextGet));
    r.register_static("style.cssText.set", Box::new(super::style::StyleCssTextSet));

    // --- CSSOM (formerly via CssomApiHandler — see §M4-12 design review CRIT-2) ---
    r.register_static("getComputedStyle", Box::new(super::GetComputedStyle));

    // --- CSS namespace (CSSOM §6.7) ---
    //
    // `CSS.escape` and `CSS.supports` are pure functions — they consult
    // neither `this` nor `dom`.  The custom-VM host file
    // (`vm/host/css_style_declaration.rs`) calls
    // `elidex_css::escape_ident` / `elidex_css::parse_declaration_block`
    // directly to skip the registry round-trip and the resulting
    // sentinel-entity dance.  The handlers (`CssEscape` / `CssSupports`)
    // remain in the dom-api crate as engine-independent reference
    // implementations and for any future engine that prefers the unified
    // dispatch path; they are intentionally NOT registered here.

    // --- ClassList ---
    r.register_static("classList.add", Box::new(super::ClassListAdd));
    r.register_static("classList.remove", Box::new(super::ClassListRemove));
    r.register_static("classList.toggle", Box::new(super::ClassListToggle));
    r.register_static("classList.contains", Box::new(super::ClassListContains));
    r.register_static("classList.replace", Box::new(super::ClassListReplace));
    r.register_static("classList.value.get", Box::new(super::ClassListValueGet));
    r.register_static("classList.value.set", Box::new(super::ClassListValueSet));
    r.register_static("classList.length", Box::new(super::ClassListLength));
    r.register_static("classList.item", Box::new(super::ClassListItem));
    r.register_static("classList.supports", Box::new(super::ClassListSupports));

    // --- RelList (slot #11-tags-T2a-url-bearing) ---
    r.register_static("relList.add", Box::new(super::RelListAdd));
    r.register_static("relList.remove", Box::new(super::RelListRemove));
    r.register_static("relList.toggle", Box::new(super::RelListToggle));
    r.register_static("relList.contains", Box::new(super::RelListContains));
    r.register_static("relList.replace", Box::new(super::RelListReplace));
    r.register_static("relList.value.get", Box::new(super::RelListValueGet));
    r.register_static("relList.value.set", Box::new(super::RelListValueSet));
    r.register_static("relList.length", Box::new(super::RelListLength));
    r.register_static("relList.item", Box::new(super::RelListItem));

    // --- LinkSizes (slot #11-tags-T2a-url-bearing, <link>.sizes) ---
    r.register_static("linkSizes.add", Box::new(super::LinkSizesAdd));
    r.register_static("linkSizes.remove", Box::new(super::LinkSizesRemove));
    r.register_static("linkSizes.toggle", Box::new(super::LinkSizesToggle));
    r.register_static("linkSizes.contains", Box::new(super::LinkSizesContains));
    r.register_static("linkSizes.replace", Box::new(super::LinkSizesReplace));
    r.register_static("linkSizes.value.get", Box::new(super::LinkSizesValueGet));
    r.register_static("linkSizes.value.set", Box::new(super::LinkSizesValueSet));
    r.register_static("linkSizes.length", Box::new(super::LinkSizesLength));
    r.register_static("linkSizes.item", Box::new(super::LinkSizesItem));

    // --- HTMLHyperlinkElementUtils mixin (anchor / area, slot #11-tags-T2a-url-bearing) ---
    r.register_static("hyperlink.href.get", Box::new(super::HyperlinkHrefGet));
    r.register_static("hyperlink.href.set", Box::new(super::HyperlinkHrefSet));
    r.register_static("hyperlink.origin.get", Box::new(super::HyperlinkOriginGet));
    r.register_static(
        "hyperlink.protocol.get",
        Box::new(super::HyperlinkProtocolGet),
    );
    r.register_static(
        "hyperlink.protocol.set",
        Box::new(super::HyperlinkProtocolSet),
    );
    r.register_static(
        "hyperlink.username.get",
        Box::new(super::HyperlinkUsernameGet),
    );
    r.register_static(
        "hyperlink.username.set",
        Box::new(super::HyperlinkUsernameSet),
    );
    r.register_static(
        "hyperlink.password.get",
        Box::new(super::HyperlinkPasswordGet),
    );
    r.register_static(
        "hyperlink.password.set",
        Box::new(super::HyperlinkPasswordSet),
    );
    r.register_static("hyperlink.host.get", Box::new(super::HyperlinkHostGet));
    r.register_static("hyperlink.host.set", Box::new(super::HyperlinkHostSet));
    r.register_static(
        "hyperlink.hostname.get",
        Box::new(super::HyperlinkHostnameGet),
    );
    r.register_static(
        "hyperlink.hostname.set",
        Box::new(super::HyperlinkHostnameSet),
    );
    r.register_static("hyperlink.port.get", Box::new(super::HyperlinkPortGet));
    r.register_static("hyperlink.port.set", Box::new(super::HyperlinkPortSet));
    r.register_static(
        "hyperlink.pathname.get",
        Box::new(super::HyperlinkPathnameGet),
    );
    r.register_static(
        "hyperlink.pathname.set",
        Box::new(super::HyperlinkPathnameSet),
    );
    r.register_static("hyperlink.search.get", Box::new(super::HyperlinkSearchGet));
    r.register_static("hyperlink.search.set", Box::new(super::HyperlinkSearchSet));
    r.register_static("hyperlink.hash.get", Box::new(super::HyperlinkHashGet));
    r.register_static("hyperlink.hash.set", Box::new(super::HyperlinkHashSet));
    r.register_static("hyperlink.toString", Box::new(super::HyperlinkToString));

    // --- Tree navigation ---
    r.register_static("parentNode.get", Box::new(super::GetParentNode));
    r.register_static("parentElement.get", Box::new(super::GetParentElement));
    r.register_static("firstChild.get", Box::new(super::GetFirstChild));
    r.register_static("lastChild.get", Box::new(super::GetLastChild));
    r.register_static("nextSibling.get", Box::new(super::GetNextSibling));
    r.register_static("previousSibling.get", Box::new(super::GetPrevSibling));
    r.register_static(
        "firstElementChild.get",
        Box::new(super::GetFirstElementChild),
    );
    r.register_static("lastElementChild.get", Box::new(super::GetLastElementChild));
    r.register_static(
        "nextElementSibling.get",
        Box::new(super::GetNextElementSibling),
    );
    r.register_static(
        "previousElementSibling.get",
        Box::new(super::GetPrevElementSibling),
    );

    // --- Node info ---
    r.register_static("tagName.get", Box::new(super::GetTagName));
    r.register_static("nodeName.get", Box::new(super::GetNodeName));
    r.register_static("nodeType.get", Box::new(super::GetNodeType));
    r.register_static("nodeValue.get", Box::new(super::tree_nav::GetNodeValue));
    r.register_static(
        "childElementCount.get",
        Box::new(super::GetChildElementCount),
    );
    r.register_static("hasChildNodes", Box::new(super::HasChildNodes));

    // --- Node methods ---
    r.register_static("contains", Box::new(super::Contains));
    r.register_static(
        "compareDocumentPosition",
        Box::new(super::CompareDocumentPosition),
    );
    r.register_static("cloneNode", Box::new(super::CloneNode));
    r.register_static("normalize", Box::new(super::Normalize));
    r.register_static("isConnected.get", Box::new(super::IsConnected));
    r.register_static("getRootNode", Box::new(super::GetRootNode));
    r.register_static("nodeValue.set", Box::new(super::SetNodeValue));
    r.register_static("ownerDocument.get", Box::new(super::OwnerDocument));
    r.register_static("isSameNode", Box::new(super::IsSameNode));
    r.register_static("isEqualNode", Box::new(super::IsEqualNode));

    // --- ChildNode mixin ---
    r.register_static("before", Box::new(super::Before));
    r.register_static("after", Box::new(super::After));
    r.register_static("remove", Box::new(super::ChildNodeRemove));
    r.register_static("replaceWith", Box::new(super::ReplaceWith));

    // --- ParentNode mixin ---
    r.register_static("prepend", Box::new(super::Prepend));
    r.register_static("append", Box::new(super::Append));
    r.register_static("replaceChildren", Box::new(super::ReplaceChildren));

    // --- Element selectors ---
    r.register_static("matches", Box::new(super::Matches));
    r.register_static("closest", Box::new(super::Closest));

    // --- CharacterData ---
    r.register_static("data.get", Box::new(super::GetData));
    r.register_static("data.set", Box::new(super::SetData));
    r.register_static("length.get", Box::new(super::GetLength));
    r.register_static("substringData", Box::new(super::SubstringData));
    r.register_static("appendData", Box::new(super::AppendData));
    r.register_static("insertData", Box::new(super::InsertData));
    r.register_static("deleteData", Box::new(super::DeleteData));
    r.register_static("replaceData", Box::new(super::ReplaceData));
    r.register_static("splitText", Box::new(super::SplitText));

    // --- CSSOM stylesheet (`#11-style-declaration` PR-B) ---
    r.register_static("cssRules.length", Box::new(super::CssRulesLength));
    r.register_static("cssRules.itemId", Box::new(super::CssRulesItemId));
    r.register_static("stylesheet.insertRule", Box::new(super::InsertRule));
    r.register_static("stylesheet.deleteRule", Box::new(super::DeleteRule));
    r.register_static("rule.cssText.get", Box::new(super::RuleCssText));
    r.register_static("rule.selectorText.get", Box::new(super::RuleSelectorText));
    r.register_static(
        "rule.style.getPropertyValue",
        Box::new(super::RuleStyleGetPropertyValue),
    );
    r.register_static("rule.style.length", Box::new(super::RuleStyleLength));
    r.register_static("rule.style.item", Box::new(super::RuleStyleItem));
    r.register_static("rule.style.cssText.get", Box::new(super::RuleStyleCssText));

    r
}

/// Create a registry pre-populated with all standard CSSOM handlers.
///
/// Empty in PR-A: `GetComputedStyle` was migrated to the DOM registry
/// alongside the rest of the `CSSStyleDeclaration` surface
/// (`#11-style-declaration` design review CRIT-2 — there is no
/// `invoke_cssom_api` bridge in the custom VM, only `invoke_dom_api`,
/// so a parallel registry adds dispatch divergence without engine
/// benefit).  Kept as an empty constructor so existing call sites
/// (boa bridge, wasm-runtime) continue to compile while the cssom
/// registry plumbing is dismantled crate-by-crate.
#[must_use]
pub fn create_cssom_registry() -> CssomHandlerRegistry {
    PluginRegistry::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn dom_registry_has_all_handlers() {
        let registry = create_dom_registry();
        // Verify a representative subset of handlers exist
        let expected = [
            "querySelector",
            "getElementById",
            "createElement",
            "createTextNode",
            "createDocumentFragment",
            "createComment",
            "createAttribute",
            "appendChild",
            "insertBefore",
            "removeChild",
            "replaceChild",
            "getAttribute",
            "setAttribute",
            "removeAttribute",
            "hasAttribute",
            "toggleAttribute",
            "getAttributeNames",
            "className.get",
            "className.set",
            "id.get",
            "id.set",
            "textContent.get",
            "textContent.set",
            "innerHTML.get",
            "insertAdjacentElement",
            "insertAdjacentText",
            "dataset.get",
            "dataset.set",
            "dataset.delete",
            "dataset.keys",
            "style.setProperty",
            "style.getPropertyValue",
            "style.removeProperty",
            "classList.add",
            "classList.remove",
            "classList.toggle",
            "classList.contains",
            "classList.replace",
            "classList.value.get",
            "classList.value.set",
            "classList.length",
            "classList.item",
            "classList.supports",
            "parentNode.get",
            "parentElement.get",
            "firstChild.get",
            "lastChild.get",
            "nextSibling.get",
            "previousSibling.get",
            "firstElementChild.get",
            "lastElementChild.get",
            "nextElementSibling.get",
            "previousElementSibling.get",
            "tagName.get",
            "nodeName.get",
            "nodeType.get",
            "nodeValue.get",
            "nodeValue.set",
            "childElementCount.get",
            "hasChildNodes",
            "contains",
            "compareDocumentPosition",
            "cloneNode",
            "normalize",
            "isConnected.get",
            "getRootNode",
            "ownerDocument.get",
            "isSameNode",
            "isEqualNode",
            "before",
            "after",
            "remove",
            "replaceWith",
            "prepend",
            "append",
            "replaceChildren",
            "matches",
            "closest",
            "data.get",
            "data.set",
            "length.get",
            "substringData",
            "appendData",
            "insertData",
            "deleteData",
            "replaceData",
            "splitText",
            "document.URL.get",
            "document.readyState.get",
            "document.compatMode.get",
            "document.characterSet.get",
            "document.documentElement.get",
            "document.head.get",
            "document.body.get",
            "document.title.get",
            "document.title.set",
            "doctype.get",
            "doctype.name.get",
            "doctype.publicId.get",
            "doctype.systemId.get",
            "getAttributeNode",
            "setAttributeNode",
            "removeAttributeNode",
            "attr.name.get",
            "attr.value.get",
            "attr.value.set",
            "attr.ownerElement.get",
            "attr.specified.get",
            "getClientRects",
            "scrollIntoView",
        ];
        for name in expected {
            assert!(
                registry.resolve(name).is_some(),
                "handler '{name}' not found in DOM registry"
            );
        }
    }

    #[test]
    fn dom_registry_has_get_computed_style() {
        // §M4-12 #11-style-declaration CRIT-2: `getComputedStyle` was
        // migrated to the DOM registry; the cssom registry is empty.
        let registry = create_dom_registry();
        let handler = registry.resolve("getComputedStyle").unwrap();
        assert_eq!(handler.method_name(), "getComputedStyle");
    }

    #[test]
    fn cssom_registry_is_empty() {
        let registry = create_cssom_registry();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn dom_registry_has_style_surface() {
        let registry = create_dom_registry();
        for name in [
            "style.length",
            "style.item",
            "style.cssText.get",
            "style.cssText.set",
        ] {
            assert!(
                registry.resolve(name).is_some(),
                "handler '{name}' missing from DOM registry"
            );
        }
    }

    #[test]
    fn css_namespace_handlers_not_in_registry() {
        // Engine-bound CSS namespace dispatch calls `elidex_css` directly
        // (see `create_dom_registry` doc comment for `CSS.escape`).  The
        // handlers exist as reference implementations but are not
        // registered.
        let registry = create_dom_registry();
        assert!(registry.resolve("CSS.escape").is_none());
        assert!(registry.resolve("CSS.supports").is_none());
    }

    #[test]
    fn unknown_name_returns_none() {
        let registry = create_dom_registry();
        assert!(registry.resolve("nonExistentMethod").is_none());
    }
}

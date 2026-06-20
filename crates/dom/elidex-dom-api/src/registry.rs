//! Pre-populated registries for standard DOM/CSSOM API handlers.
//!
//! These factory functions create registries that contain all built-in handlers,
//! enabling engine-agnostic dispatch by name rather than direct handler references.

use elidex_plugin::{PluginRegistry, SpecLevelPolicy};
use elidex_script_session::{CssomApiHandler, DomApiHandler};

/// Type alias for a registry of DOM API handlers.
pub type DomHandlerRegistry = PluginRegistry<dyn DomApiHandler>;

/// Type alias for a registry of CSSOM API handlers.
pub type CssomHandlerRegistry = PluginRegistry<dyn CssomApiHandler>;

/// Create a registry pre-populated with all standard DOM API handlers under
/// the default ([`BrowserCompat`](elidex_plugin::EngineMode::BrowserCompat))
/// policy — the full compat surface.
///
/// Convenience wrapper over [`create_dom_registry_with_policy`] for embedders
/// that do not select an [`EngineMode`](elidex_plugin::EngineMode) (boa bridge,
/// wasm-runtime, tests). The elidex-js VM uses the policy-aware variant.
#[must_use]
pub fn create_dom_registry() -> DomHandlerRegistry {
    create_dom_registry_with_policy(SpecLevelPolicy::default())
}

/// Create a registry pre-populated with the standard DOM API handlers the
/// `policy` installs (seam-4 of the A1 Web-API core/compat gate).
///
/// Handlers whose [`spec_level`](DomApiHandler::spec_level) the `policy`
/// excludes are **withheld at registration time** — so resolve stays a pure
/// name→handler map lookup and the policy is a single construction-time
/// decision (no per-call hot-path check). Every handler is `Living` today, so
/// `BrowserCompat` and `BrowserCore` currently produce the same set; when B0/B1
/// classify the live-collection handlers `Legacy`, `BrowserCore`/`App` will
/// withhold them here.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn create_dom_registry_with_policy(policy: SpecLevelPolicy) -> DomHandlerRegistry {
    let mut r: DomHandlerRegistry = PluginRegistry::new();

    // Register a handler only if the policy installs its DOM spec level.
    // (`spec_level()` is read before the handler is moved into the registry.)
    let mut reg = |name: &'static str, handler: Box<dyn DomApiHandler>| {
        if policy.installs_dom(handler.spec_level()) {
            PluginRegistry::register_static(&mut r, name, handler);
        }
    };

    // --- Document ---
    reg("querySelector", Box::new(super::QuerySelector));
    reg("getElementById", Box::new(super::GetElementById));
    reg("createElement", Box::new(super::CreateElement));
    reg("createTextNode", Box::new(super::CreateTextNode));
    reg(
        "createDocumentFragment",
        Box::new(super::char_data::CreateDocumentFragment),
    );
    reg("createComment", Box::new(super::char_data::CreateComment));
    reg(
        "createAttribute",
        Box::new(super::char_data::CreateAttribute),
    );

    // --- Document properties ---
    reg("document.URL.get", Box::new(super::GetDocumentUrl));
    // D-31: WHATWG DOM §4.4 Interface Node `baseURI` getter
    // (anchor `#dom-node-baseuri`).  Document-receiver flavour +
    // Node-receiver flavour share the same underlying reader.
    reg(
        "document.baseURI.get",
        Box::new(super::char_data::GetDocumentBaseURI),
    );
    reg(
        "node.baseURI.get",
        Box::new(super::char_data::GetNodeBaseURI),
    );
    reg("document.readyState.get", Box::new(super::GetReadyState));
    reg("document.compatMode.get", Box::new(super::GetCompatMode));
    reg(
        "document.characterSet.get",
        Box::new(super::GetCharacterSet),
    );
    reg(
        "document.documentElement.get",
        Box::new(super::GetDocumentElement),
    );
    reg("document.head.get", Box::new(super::GetHead));
    reg("document.body.get", Box::new(super::GetBody));
    reg("document.title.get", Box::new(super::GetTitle));
    reg("document.title.set", Box::new(super::SetTitle));
    reg("doctype.get", Box::new(super::GetDoctype));
    reg("doctype.name.get", Box::new(super::GetDoctypeName));
    reg("doctype.publicId.get", Box::new(super::GetDoctypePublicId));
    reg("doctype.systemId.get", Box::new(super::GetDoctypeSystemId));

    // --- Element — child mutations ---
    reg("appendChild", Box::new(super::AppendChild));
    reg("insertBefore", Box::new(super::InsertBefore));
    reg("removeChild", Box::new(super::RemoveChild));
    reg("replaceChild", Box::new(super::ReplaceChild));

    // --- Element — attributes ---
    reg("getAttribute", Box::new(super::GetAttribute));
    reg("setAttribute", Box::new(super::SetAttribute));
    reg("removeAttribute", Box::new(super::RemoveAttribute));
    reg("hasAttribute", Box::new(super::HasAttribute));
    reg("toggleAttribute", Box::new(super::ToggleAttribute));
    reg("getAttributeNames", Box::new(super::GetAttributeNames));
    reg("className.get", Box::new(super::GetClassName));
    reg("className.set", Box::new(super::SetClassName));
    reg("id.get", Box::new(super::GetId));
    reg("id.set", Box::new(super::SetId));

    // --- Attr node ---
    reg("getAttributeNode", Box::new(super::GetAttributeNode));
    reg("setAttributeNode", Box::new(super::SetAttributeNode));
    reg("removeAttributeNode", Box::new(super::RemoveAttributeNode));
    reg("attr.name.get", Box::new(super::GetAttrName));
    reg("attr.value.get", Box::new(super::GetAttrValue));
    reg("attr.value.set", Box::new(super::SetAttrValue));
    reg("attr.ownerElement.get", Box::new(super::GetOwnerElement));
    reg("attr.specified.get", Box::new(super::GetAttrSpecified));

    // --- Element — content ---
    reg("textContent.get", Box::new(super::GetTextContentNodeKind));
    reg("textContent.set", Box::new(super::SetTextContentNodeKind));
    reg("innerHTML.get", Box::new(super::GetInnerHtml));
    reg("innerHTML.set", Box::new(super::SetInnerHtml));
    reg("insertAdjacentHTML", Box::new(super::InsertAdjacentHtml));

    // --- Layout query ---
    reg(
        "getBoundingClientRect",
        Box::new(super::GetBoundingClientRect),
    );
    reg("offsetWidth.get", Box::new(super::GetOffsetWidth));
    reg("offsetHeight.get", Box::new(super::GetOffsetHeight));
    reg("offsetTop.get", Box::new(super::GetOffsetTop));
    reg("offsetLeft.get", Box::new(super::GetOffsetLeft));
    reg("offsetParent.get", Box::new(super::GetOffsetParent));
    reg("clientWidth.get", Box::new(super::GetClientWidth));
    reg("clientHeight.get", Box::new(super::GetClientHeight));
    reg("clientTop.get", Box::new(super::GetClientTop));
    reg("clientLeft.get", Box::new(super::GetClientLeft));
    reg("scrollWidth.get", Box::new(super::GetScrollWidth));
    reg("scrollHeight.get", Box::new(super::GetScrollHeight));
    reg("scrollTop.get", Box::new(super::GetScrollTop));
    reg("scrollLeft.get", Box::new(super::GetScrollLeft));
    reg("getClientRects", Box::new(super::GetClientRects));
    reg("scrollIntoView", Box::new(super::ScrollIntoView));

    // --- Element — insertAdjacent ---
    reg(
        "insertAdjacentElement",
        Box::new(super::InsertAdjacentElement),
    );
    reg("insertAdjacentText", Box::new(super::InsertAdjacentText));

    // --- Dataset ---
    reg("dataset.get", Box::new(super::DatasetGet));
    reg("dataset.set", Box::new(super::DatasetSet));
    reg("dataset.delete", Box::new(super::DatasetDelete));
    reg("dataset.keys", Box::new(super::DatasetKeys));

    // --- Style (CSSStyleDeclaration §6.6) ---
    reg("style.setProperty", Box::new(super::StyleSetProperty));
    reg(
        "style.getPropertyValue",
        Box::new(super::StyleGetPropertyValue),
    );
    reg("style.removeProperty", Box::new(super::StyleRemoveProperty));
    reg(
        "style.getPropertyPriority",
        Box::new(super::style::StyleGetPropertyPriority),
    );
    reg("style.length", Box::new(super::style::StyleLength));
    reg("style.item", Box::new(super::style::StyleItem));
    reg("style.cssText.get", Box::new(super::style::StyleCssTextGet));
    reg("style.cssText.set", Box::new(super::style::StyleCssTextSet));

    // --- CSSOM (formerly via CssomApiHandler — see §M4-12 design review CRIT-2) ---
    reg("getComputedStyle", Box::new(super::GetComputedStyle));

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

    // --- DOMTokenList families (classList / relList / linkSizes) ---
    // Single TokenListHandler factory dispatches by (attr_name, op);
    // /simplify H3 fold of slot `#11-tags-T2a-url-bearing` collapsed
    // 28 unit struct types into 28 const instances.
    reg("classList.add", Box::new(super::CLASS_LIST_ADD));
    reg("classList.remove", Box::new(super::CLASS_LIST_REMOVE));
    reg("classList.toggle", Box::new(super::CLASS_LIST_TOGGLE));
    reg("classList.contains", Box::new(super::CLASS_LIST_CONTAINS));
    reg("classList.replace", Box::new(super::CLASS_LIST_REPLACE));
    reg("classList.value.get", Box::new(super::CLASS_LIST_VALUE_GET));
    reg("classList.value.set", Box::new(super::CLASS_LIST_VALUE_SET));
    reg("classList.length", Box::new(super::CLASS_LIST_LENGTH));
    reg("classList.item", Box::new(super::CLASS_LIST_ITEM));
    reg("classList.supports", Box::new(super::CLASS_LIST_SUPPORTS));

    reg("relList.add", Box::new(super::REL_LIST_ADD));
    reg("relList.remove", Box::new(super::REL_LIST_REMOVE));
    reg("relList.toggle", Box::new(super::REL_LIST_TOGGLE));
    reg("relList.contains", Box::new(super::REL_LIST_CONTAINS));
    reg("relList.replace", Box::new(super::REL_LIST_REPLACE));
    reg("relList.value.get", Box::new(super::REL_LIST_VALUE_GET));
    reg("relList.value.set", Box::new(super::REL_LIST_VALUE_SET));
    reg("relList.length", Box::new(super::REL_LIST_LENGTH));
    reg("relList.item", Box::new(super::REL_LIST_ITEM));
    reg("relList.supports", Box::new(super::REL_LIST_SUPPORTS));

    reg("linkSizes.add", Box::new(super::LINK_SIZES_ADD));
    reg("linkSizes.remove", Box::new(super::LINK_SIZES_REMOVE));
    reg("linkSizes.toggle", Box::new(super::LINK_SIZES_TOGGLE));
    reg("linkSizes.contains", Box::new(super::LINK_SIZES_CONTAINS));
    reg("linkSizes.replace", Box::new(super::LINK_SIZES_REPLACE));
    reg("linkSizes.value.get", Box::new(super::LINK_SIZES_VALUE_GET));
    reg("linkSizes.value.set", Box::new(super::LINK_SIZES_VALUE_SET));
    reg("linkSizes.length", Box::new(super::LINK_SIZES_LENGTH));
    reg("linkSizes.item", Box::new(super::LINK_SIZES_ITEM));
    reg("linkSizes.supports", Box::new(super::LINK_SIZES_SUPPORTS));

    // `<output>.htmlFor` DOMTokenList family — slot `#11-tags-T2d-interactive`.
    reg("outputHtmlFor.add", Box::new(super::OUTPUT_HTML_FOR_ADD));
    reg(
        "outputHtmlFor.remove",
        Box::new(super::OUTPUT_HTML_FOR_REMOVE),
    );
    reg(
        "outputHtmlFor.toggle",
        Box::new(super::OUTPUT_HTML_FOR_TOGGLE),
    );
    reg(
        "outputHtmlFor.contains",
        Box::new(super::OUTPUT_HTML_FOR_CONTAINS),
    );
    reg(
        "outputHtmlFor.replace",
        Box::new(super::OUTPUT_HTML_FOR_REPLACE),
    );
    reg(
        "outputHtmlFor.value.get",
        Box::new(super::OUTPUT_HTML_FOR_VALUE_GET),
    );
    reg(
        "outputHtmlFor.value.set",
        Box::new(super::OUTPUT_HTML_FOR_VALUE_SET),
    );
    reg(
        "outputHtmlFor.length",
        Box::new(super::OUTPUT_HTML_FOR_LENGTH),
    );
    reg("outputHtmlFor.item", Box::new(super::OUTPUT_HTML_FOR_ITEM));
    reg(
        "outputHtmlFor.supports",
        Box::new(super::OUTPUT_HTML_FOR_SUPPORTS),
    );

    // --- HTMLHyperlinkElementUtils mixin (anchor / area, slot #11-tags-T2a-url-bearing) ---
    reg("hyperlink.href.get", Box::new(super::HyperlinkHrefGet));
    reg("hyperlink.href.set", Box::new(super::HyperlinkHrefSet));
    reg("hyperlink.origin.get", Box::new(super::HyperlinkOriginGet));
    reg(
        "hyperlink.protocol.get",
        Box::new(super::HyperlinkProtocolGet),
    );
    reg(
        "hyperlink.protocol.set",
        Box::new(super::HyperlinkProtocolSet),
    );
    reg(
        "hyperlink.username.get",
        Box::new(super::HyperlinkUsernameGet),
    );
    reg(
        "hyperlink.username.set",
        Box::new(super::HyperlinkUsernameSet),
    );
    reg(
        "hyperlink.password.get",
        Box::new(super::HyperlinkPasswordGet),
    );
    reg(
        "hyperlink.password.set",
        Box::new(super::HyperlinkPasswordSet),
    );
    reg("hyperlink.host.get", Box::new(super::HyperlinkHostGet));
    reg("hyperlink.host.set", Box::new(super::HyperlinkHostSet));
    reg(
        "hyperlink.hostname.get",
        Box::new(super::HyperlinkHostnameGet),
    );
    reg(
        "hyperlink.hostname.set",
        Box::new(super::HyperlinkHostnameSet),
    );
    reg("hyperlink.port.get", Box::new(super::HyperlinkPortGet));
    reg("hyperlink.port.set", Box::new(super::HyperlinkPortSet));
    reg(
        "hyperlink.pathname.get",
        Box::new(super::HyperlinkPathnameGet),
    );
    reg(
        "hyperlink.pathname.set",
        Box::new(super::HyperlinkPathnameSet),
    );
    reg("hyperlink.search.get", Box::new(super::HyperlinkSearchGet));
    reg("hyperlink.search.set", Box::new(super::HyperlinkSearchSet));
    reg("hyperlink.hash.get", Box::new(super::HyperlinkHashGet));
    reg("hyperlink.hash.set", Box::new(super::HyperlinkHashSet));
    reg("hyperlink.toString", Box::new(super::HyperlinkToString));

    // --- Tree navigation ---
    reg("parentNode.get", Box::new(super::GetParentNode));
    reg("parentElement.get", Box::new(super::GetParentElement));
    reg("firstChild.get", Box::new(super::GetFirstChild));
    reg("lastChild.get", Box::new(super::GetLastChild));
    reg("nextSibling.get", Box::new(super::GetNextSibling));
    reg("previousSibling.get", Box::new(super::GetPrevSibling));
    reg(
        "firstElementChild.get",
        Box::new(super::GetFirstElementChild),
    );
    reg("lastElementChild.get", Box::new(super::GetLastElementChild));
    reg(
        "nextElementSibling.get",
        Box::new(super::GetNextElementSibling),
    );
    reg(
        "previousElementSibling.get",
        Box::new(super::GetPrevElementSibling),
    );

    // --- Node info ---
    reg("tagName.get", Box::new(super::GetTagName));
    reg("nodeName.get", Box::new(super::GetNodeName));
    reg("nodeType.get", Box::new(super::GetNodeType));
    reg("nodeValue.get", Box::new(super::tree_nav::GetNodeValue));
    reg(
        "childElementCount.get",
        Box::new(super::GetChildElementCount),
    );
    reg("hasChildNodes", Box::new(super::HasChildNodes));

    // --- Node methods ---
    reg("contains", Box::new(super::Contains));
    reg(
        "compareDocumentPosition",
        Box::new(super::CompareDocumentPosition),
    );
    reg("cloneNode", Box::new(super::CloneNode));
    reg("normalize", Box::new(super::Normalize));
    reg("isConnected.get", Box::new(super::IsConnected));
    reg("getRootNode", Box::new(super::GetRootNode));
    reg("nodeValue.set", Box::new(super::SetNodeValue));
    reg("ownerDocument.get", Box::new(super::OwnerDocument));
    reg("isSameNode", Box::new(super::IsSameNode));
    reg("isEqualNode", Box::new(super::IsEqualNode));

    // --- ChildNode mixin ---
    reg("before", Box::new(super::Before));
    reg("after", Box::new(super::After));
    reg("remove", Box::new(super::ChildNodeRemove));
    reg("replaceWith", Box::new(super::ReplaceWith));

    // --- ParentNode mixin ---
    reg("prepend", Box::new(super::Prepend));
    reg("append", Box::new(super::Append));
    reg("replaceChildren", Box::new(super::ReplaceChildren));

    // --- Element selectors ---
    reg("matches", Box::new(super::Matches));
    reg("closest", Box::new(super::Closest));

    // --- CharacterData ---
    reg("data.get", Box::new(super::GetData));
    reg("data.set", Box::new(super::SetData));
    reg("length.get", Box::new(super::GetLength));
    reg("substringData", Box::new(super::SubstringData));
    reg("appendData", Box::new(super::AppendData));
    reg("insertData", Box::new(super::InsertData));
    reg("deleteData", Box::new(super::DeleteData));
    reg("replaceData", Box::new(super::ReplaceData));
    reg("splitText", Box::new(super::SplitText));

    // --- HTMLTable family (`#11-tags-T2c-table`) ---
    reg(
        "table.insertRow",
        Box::new(super::element::table_mutation::TableInsertRow),
    );
    reg(
        "table.deleteRow",
        Box::new(super::element::table_mutation::TableDeleteRow),
    );
    reg(
        "section.insertRow",
        Box::new(super::element::table_mutation::SectionInsertRow),
    );
    reg(
        "section.deleteRow",
        Box::new(super::element::table_mutation::SectionDeleteRow),
    );
    reg(
        "row.insertCell",
        Box::new(super::element::table_mutation::RowInsertCell),
    );
    reg(
        "row.deleteCell",
        Box::new(super::element::table_mutation::RowDeleteCell),
    );
    reg(
        "table.createTHead",
        Box::new(super::element::table_mutation::TableCreateTHead),
    );
    reg(
        "table.createTFoot",
        Box::new(super::element::table_mutation::TableCreateTFoot),
    );
    reg(
        "table.createCaption",
        Box::new(super::element::table_mutation::TableCreateCaption),
    );
    reg(
        "table.createTBody",
        Box::new(super::element::table_mutation::TableCreateTBody),
    );
    reg(
        "table.deleteTHead",
        Box::new(super::element::table_mutation::TableDeleteTHead),
    );
    reg(
        "table.deleteTFoot",
        Box::new(super::element::table_mutation::TableDeleteTFoot),
    );
    reg(
        "table.deleteCaption",
        Box::new(super::element::table_mutation::TableDeleteCaption),
    );
    reg(
        "table.tHead.set",
        Box::new(super::element::table_mutation::TableSetTHead),
    );
    reg(
        "table.tFoot.set",
        Box::new(super::element::table_mutation::TableSetTFoot),
    );
    reg(
        "table.caption.set",
        Box::new(super::element::table_mutation::TableSetCaption),
    );

    // --- CSSOM stylesheet (`#11-style-declaration` PR-B) ---
    reg("cssRules.length", Box::new(super::CssRulesLength));
    reg("cssRules.itemId", Box::new(super::CssRulesItemId));
    reg("stylesheet.insertRule", Box::new(super::InsertRule));
    reg("stylesheet.deleteRule", Box::new(super::DeleteRule));
    reg("rule.cssText.get", Box::new(super::RuleCssText));
    reg("rule.selectorText.get", Box::new(super::RuleSelectorText));
    reg(
        "rule.style.getPropertyValue",
        Box::new(super::RuleStyleGetPropertyValue),
    );
    reg(
        "rule.style.getPropertyPriority",
        Box::new(super::RuleStyleGetPropertyPriority),
    );
    reg("rule.style.length", Box::new(super::RuleStyleLength));
    reg("rule.style.item", Box::new(super::RuleStyleItem));
    reg("rule.style.cssText.get", Box::new(super::RuleStyleCssText));

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

    // --- A1 Web-API core/compat gate (seam-4) ---

    use elidex_ecs::{EcsDom, Entity};
    use elidex_plugin::{DomSpecLevel, EngineMode, JsValue};
    use elidex_script_session::{DomApiError, SessionCore};

    /// A mock `Legacy`-classified DOM handler (e.g. a live-collection getter
    /// once B0 demotes the family). Never invoked — only its `spec_level` and
    /// registration/withholding are exercised.
    struct MockLegacyHandler;
    impl DomApiHandler for MockLegacyHandler {
        fn method_name(&self) -> &str {
            "mockLegacy"
        }
        fn spec_level(&self) -> DomSpecLevel {
            DomSpecLevel::Legacy
        }
        fn invoke(
            &self,
            _this: Entity,
            _args: &[JsValue],
            _session: &mut SessionCore,
            _dom: &mut EcsDom,
        ) -> Result<JsValue, DomApiError> {
            unreachable!("mock handler is never invoked")
        }
    }

    #[test]
    fn policy_wrapper_matches_default() {
        // The no-arg convenience wrapper == the policy-aware variant under the
        // default (BrowserCompat) policy.
        let a = create_dom_registry();
        let b = create_dom_registry_with_policy(EngineMode::BrowserCompat.spec_level_policy());
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn living_handlers_survive_a_legacy_excluding_policy() {
        // Every real handler is `Living` today, so a BrowserCore (Legacy-
        // excluding) policy must withhold none of them — no over-prune.
        let compat = create_dom_registry_with_policy(EngineMode::BrowserCompat.spec_level_policy());
        let core = create_dom_registry_with_policy(EngineMode::BrowserCore.spec_level_policy());
        assert_eq!(compat.len(), core.len());
        assert!(core.resolve("querySelector").is_some());
        assert!(core.resolve("setAttribute").is_some());
    }

    #[test]
    fn legacy_handler_withheld_under_core_policy() {
        // Seam-4 end-to-end: a `Legacy`-classified handler registers under
        // BrowserCompat but is withheld under BrowserCore/App — the exact
        // `if policy.installs_dom(handler.spec_level()) { register }` gate that
        // `create_dom_registry_with_policy` applies (here with an injectable
        // mock, since the real handler set is all-`Living` today).
        for (mode, expect_present) in [
            (EngineMode::BrowserCompat, true),
            (EngineMode::BrowserCore, false),
            (EngineMode::App, false),
        ] {
            let policy = mode.spec_level_policy();
            let mut r: DomHandlerRegistry = PluginRegistry::new();
            let handler: Box<dyn DomApiHandler> = Box::new(MockLegacyHandler);
            if policy.installs_dom(handler.spec_level()) {
                r.register_static("mockLegacy", handler);
            }
            assert_eq!(
                r.resolve("mockLegacy").is_some(),
                expect_present,
                "{mode:?}: Legacy handler presence mismatch"
            );
        }
    }
}

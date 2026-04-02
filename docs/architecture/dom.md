# Architecture: DOM (html-parser, dom-api, a11y)

## elidex-html-parser

- **Three entry points**: `parse_html(&str)` (existing, UTF-8), `parse_tolerant(&[u8], charset_hint)` (byte input with charset auto-detection), `parse_strict(&str)` (rejects documents with parse errors).
- **Charset detection** (`charset.rs`): BOM always stripped first (`strip_bom()`), then encoding priority: HTTP charset hint → BOM encoding → `<meta charset>` prescan (1024 bytes) → `<meta http-equiv="Content-Type" content="…;charset=…">` prescan → UTF-8 default. Uses `encoding_rs` with `new_decoder_without_bom_handling()` to avoid encoding_rs's built-in BOM sniffing overriding our priority logic.
- **ParseResult**: Extended with `encoding: Option<&'static str>` (set by `parse_tolerant`, `None` for `parse_html`).
- **StrictParseError**: `Display` + `Error` impl, contains `Vec<String>` of html5ever error messages.
- **Dependencies**: `encoding_rs 0.8` for charset detection/transcoding.

## elidex-dom-api

- **Engine-independent DOM API handlers**: Concrete implementations of `DomApiHandler`/`CssomApiHandler` traits from `elidex-script-session`. No dependency on boa or any JS engine.
- **document.rs**: `QuerySelector` (CSS selector matching via `elidex_css::Selector::matches()`), `GetElementById`, `CreateElement`, `CreateTextNode`, `query_selector_all()` standalone function.
- **element.rs**: `AppendChild`, `InsertBefore`, `RemoveChild` (direct `EcsDom` operations), `Get/Set/RemoveAttribute`, `Get/SetTextContent`, `GetInnerHtml` (HTML serialization with escaping).
- **class_list.rs**: `ClassListAdd/Remove/Toggle/Contains` — operates on `Attributes` class string.
- **style.rs**: `StyleSetProperty/GetPropertyValue/RemoveProperty` — `InlineStyle` component operations. Auto-inserts `InlineStyle` if missing.
- **computed_style.rs**: `GetComputedStyle` (CssomApiHandler) — delegates to `elidex_style::get_computed_as_css_value()`.
- **util.rs**: `require_string_arg()`, `require_object_ref_arg()`, `escape_html()`, `escape_attr()`.

## elidex-a11y

- **build_tree_update()**: Walks ECS DOM pre-order → AccessKit `TreeUpdate`. `TREE_ROOT_ID = 0` sentinel for document root (safe because hecs entities are `NonZeroU64`). Skips `aria-hidden="true"` elements.
- **Role mapping**: `tag_to_role()` maps ~30 HTML tags, `aria_role_from_str()` maps ~60 ARIA role strings. Special cases: `img` with empty `alt` → GenericContainer, `a` without `href` → GenericContainer.
- **ACCNAME algorithm**: `compute_accessible_name()` — priority: `aria-labelledby` (id reference resolution) → `aria-label` → `alt` (img) → text content → `title`.
- **entity_to_node_id()**: `Entity.to_bits().get()` → `NodeId(u64)`.
- **Dependencies**: elidex-ecs, elidex-plugin (LayoutBox), accesskit 0.24.

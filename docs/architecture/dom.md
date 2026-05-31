# Architecture: DOM (html-parser, dom-api, a11y)

## elidex-html-parser

- **Three entry points**: `parse_html(&str)` (UTF-8, tolerant html5ever = §11.3 Tier-2), `parse_progressive(&[u8], charset_hint)` (byte input with charset auto-detection + design doc §11.3 strict-first dispatch: tries `parse_strict` for conforming HTML5, falls back to the tolerant html5ever backend on the first §13.2.2 parse error over the same decoded text), `parse_strict(&str)` (strict mode = §11.3 Tier-1, rejects documents with parse errors).
- **Charset detection** (`charset.rs`): BOM always stripped first (`strip_bom()`), then encoding priority: HTTP charset hint → BOM encoding → `<meta charset>` prescan (1024 bytes) → `<meta http-equiv="Content-Type" content="…;charset=…">` prescan → UTF-8 default. Uses `encoding_rs` with `new_decoder_without_bom_handling()` to avoid encoding_rs's built-in BOM sniffing overriding our priority logic.
- **ParseResult**: `encoding: Option<&'static str>` (detected encoding, set by `parse_progressive`; `None` for the bare-`&str` entry points `parse_html`/`parse_strict`) and `tier: ParseTier` (`Clean` = strict/Tier-1 produced the tree, `Recovered` = tolerant/Tier-2 backend — intrinsic to the producing backend, making the §11.3 strict-vs-fallback gradient observable).
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

# Architecture: Core (elidex-plugin, elidex-ecs)

## elidex-plugin

- **Traits**: `CssPropertyHandler`, `HtmlElementHandler`, `LayoutModel`, `NetworkMiddleware` (all `Send + Sync`).
- **PluginRegistry**: Generic (`Debug` impl), static-first lookup, `#[must_use]` on `resolve()`, same-name re-registration overwrites. `is_shadowed()` helper for dedup.
- **SpecLevel enums**: All `#[non_exhaustive]`, `Default` with `#[default]` on first variant.
- **Error types**: `define_error_type!` macro for DRY error boilerplate (`ParseError`, `HtmlParseError`, `NetworkError`).
- **JsValue**: `#[non_exhaustive]` enum (Undefined/Null/Bool/Number/String/ObjectRef) — cross-engine JS value type.
- **Network types**: `HttpRequest` (method/url/headers), `HttpResponse` (status/headers), `NetworkError` (kind/message), `NetworkErrorKind` enum.
- **ProcessModel**: `SiteIsolation`/`PerTab`/`Shared{max_renderers}`/`SingleProcess` — `#[non_exhaustive]`, Phase 3.5 implements `SingleProcess` only.
- **Sandbox types** (`sandbox.rs`): `FilesystemAccess` (None/ReadOnly/ReadWrite), `NetworkAccess` (None/SameOrigin/Full), `SandboxPolicy` (filesystem/network/ipc/gpu) with `strict()`/`permissive()`/`web_content()` constructors, `PlatformSandbox` (LinuxSeccomp/MacOSAppSandbox/WindowsRestricted/Unsandboxed). Type-only — enforcement deferred to OS process isolation phase.
- **Built-in handlers** (`handlers/`): Demo trait implementations for HTML and layout plugins. `create_html_element_registry()` (div/a/img/script/button), `create_layout_registry()` (block/flex/grid/table). Layout models use stub layout (actual dispatch remains in elidex-layout). CSS property handlers moved to dedicated plugin crates (elidex-css-{box,text,flex,grid,table,float}).
- **css_resolve module**: Shared resolution utilities re-exported for plugin crates — `resolve_length`, `resolve_dimension`, `resolve_to_px`, `resolve_calc_expr`, `resolve_non_negative_f32`, `resolve_i32`, `keyword_from`, `parse_length_unit`.

## elidex-ecs

- **Tree invariants**: No cycles (ancestor walk with depth counter, capped at 10,000), consistent sibling links, parent↔child consistency, destroyed entity safety. `#[must_use]` on all mutation methods.
- **Internal helpers**: `update_rel()`, `read_rel()`, `clear_rel()` for TreeRelation access. `is_child_of()` for parent validation. `all_exist()` for entity checks.
- **API**: `append_child`, `insert_before`, `replace_child` (validates before detach), `detach`, `destroy_entity`. Helpers: `get_parent`, `get_first_child`, `get_last_child`, `get_next_sibling`, `get_prev_sibling`, `contains`.
- **Attributes**: `get/set/remove/contains` accessors on `Attributes` struct.
- **Shadow DOM**: `ShadowRoot` (mode + host), `ShadowHost` (shadow_root), `ShadowRootMode` (Open/Closed), `SlotAssignment` (assigned_nodes), `SlottedMarker`, `TemplateContent` (marker) components. `attach_shadow(host, mode)` with WHATWG element whitelist (18 tags). `get_shadow_root(host)`. `composed_children(entity)` — shadow hosts return shadow tree children, slots return assigned nodes (or fallback), others return normal children.

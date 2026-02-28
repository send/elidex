# CLAUDE.md — elidex project notes

## Project Overview

elidex is an experimental browser engine written in Rust. Phase 0 (foundation) is complete.

### Workspace Structure

```
crates/
  elidex-plugin/        — Plugin traits, SpecLevel enums, PluginRegistry
  elidex-plugin-macros/ — Procedural macros (#[derive(SpecLevel)] etc.)
  elidex-ecs/           — ECS (hecs) based DOM prototype
  elidex-crawler/       — Web compatibility survey tool (binary crate)
  elidex-parser/        — HTML/XML parser (html5ever wrapper, charset detection)
  elidex-css/           — CSS parser, value types, selector engine
  elidex-style/         — Cascade, inheritance, style resolution
  elidex-layout/        — Block, inline, flexbox layout
  elidex-text/          — Text shaping, measurement, line breaking
  elidex-render/        — Rendering backend abstraction
  elidex-shell/         — Window management, event loop shell
  elidex-script-session/ — Script session abstraction (JS ↔ ECS DOM bridge)
  elidex-net/           — HTTP network stack (hyper, TLS, connection pool, cookies)
```

### Common Commands

```sh
mise run ci          # Run all CI checks locally (lint + test + deny)
mise run test        # cargo test --workspace
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
cargo doc --workspace --no-deps  # Build docs
```

### Key Files

- `SECURITY.md` — Vulnerability disclosure policy
- `CONTRIBUTING.md` — Contribution guidelines
- `.github/dependabot.yml` — Automated dependency updates (Cargo + Actions)
- `deny.toml` — License allow-list + supply chain (`unknown-registry`/`unknown-git` = deny)
- `docs/design/ja/29-survey-analysis.md` — JA/EN 900-site compatibility survey analysis (Ch. 29)

---

## Architecture Notes

### elidex-crawler

- **SSRF protection**: `validate_url()` checks scheme, private IPs (IPv4/IPv6), reserved hostnames. Custom reqwest redirect policy validates each hop.
- **robots.txt**: RFC 9309 subset. `Allow`/`Disallow` with longest-match-wins. `best_agent_match()` shared by `is_allowed`/`crawl_delay`. `Crawl-delay` validated (`is_finite() && >= 0.0`). Body fetch has 5s timeout + 512KB limit.
- **Concurrency**: Global semaphore + per-host semaphore (2 concurrent). Semaphore acquire uses `let...else` for graceful error handling. Host map evicts stale entries at 10,000 threshold via `Arc::strong_count`.
- **Content-Type**: MIME type extracted before `;` for exact match (`text/html`, `application/xhtml+xml`, `text/xml`).
- **Shared utilities**: `analyzer/util.rs` provides `strip_comments()` (CSS/JS) and `extract_tag_blocks()` (style/script), both `pub(crate)`. `MAX_EXTRACT_ITERATIONS` in `analyzer/mod.rs`.
- **Type alias**: `FeatureCount = HashMap<String, usize>` in `analyzer/mod.rs`, used across all feature structs.
- **Report aggregation**: `FeatureAggregator` pattern + `FEATURE_CSV_HEADER` constant deduplicates feature counting/CSV logic.
- **Error chain**: Retry errors use `format!("{e:#}")` to preserve full anyhow error chain.
- **`to_ascii_lowercase()` safety**: Verified — only modifies ASCII bytes, preserving byte positions. Safe to cross-index with original HTML.

### elidex-parser

- **Three entry points**: `parse_html(&str)` (existing, UTF-8), `parse_tolerant(&[u8], charset_hint)` (byte input with charset auto-detection), `parse_strict(&str)` (rejects documents with parse errors).
- **Charset detection** (`charset.rs`): BOM always stripped first (`strip_bom()`), then encoding priority: HTTP charset hint → BOM encoding → `<meta charset>` prescan (1024 bytes) → `<meta http-equiv="Content-Type" content="…;charset=…">` prescan → UTF-8 default. Uses `encoding_rs` with `new_decoder_without_bom_handling()` to avoid encoding_rs's built-in BOM sniffing overriding our priority logic.
- **ParseResult**: Extended with `encoding: Option<&'static str>` (set by `parse_tolerant`, `None` for `parse_html`).
- **StrictParseError**: `Display` + `Error` impl, contains `Vec<String>` of html5ever error messages.
- **Dependencies**: `encoding_rs 0.8` for charset detection/transcoding.

### elidex-ecs

- **Tree invariants**: No cycles (ancestor walk with depth counter, capped at 10,000), consistent sibling links, parent↔child consistency, destroyed entity safety. `#[must_use]` on all mutation methods.
- **Internal helpers**: `update_rel()`, `read_rel()`, `clear_rel()` for TreeRelation access. `is_child_of()` for parent validation. `all_exist()` for entity checks.
- **API**: `append_child`, `insert_before`, `replace_child` (validates before detach), `detach`, `destroy_entity`. Helpers: `get_parent`, `get_first_child`, `get_last_child`, `get_next_sibling`, `get_prev_sibling`, `contains`.
- **Attributes**: `get/set/remove/contains` accessors on `Attributes` struct.

### elidex-plugin

- **Traits**: `CssPropertyHandler`, `HtmlElementHandler`, `LayoutModel`, `NetworkMiddleware` (all `Send + Sync`).
- **PluginRegistry**: Generic (`Debug` impl), static-first lookup, `#[must_use]` on `resolve()`, same-name re-registration overwrites. `is_shadowed()` helper for dedup.
- **SpecLevel enums**: All `#[non_exhaustive]`, `Default` with `#[default]` on first variant.
- **Error types**: `define_error_type!` macro for DRY error boilerplate (`ParseError`, `HtmlParseError`, `NetworkError`).
- **JsValue**: `#[non_exhaustive]` enum (Undefined/Null/Bool/Number/String/ObjectRef) — cross-engine JS value type.
- **Network types**: `HttpRequest` (method/url/headers), `HttpResponse` (status/headers), `NetworkError` (kind/message), `NetworkErrorKind` enum.

### elidex-layout

- **Block layout**: `layout_block_inner()` handles block formatting context — width resolution, margin collapse (adjacent siblings, positive/negative), padding/border, vertical stacking. `is_block_level()` classifies display types.
- **Inline layout**: `layout_inline()` handles inline formatting context — text shaping, line breaking, line box construction.
- **Flexbox layout**: `flex.rs` implements CSS Flexbox Level 1 (simplified). `layout_flex()` entry point: box model resolution → item collection (`display:none` skipped) → `order` stable sort → line splitting (nowrap/wrap/wrap-reverse) → flexible length resolution (grow/shrink with frozen/unfrozen loop) → cross size resolution → main axis positioning (justify-content: 6 values) → cross axis alignment (align-items/align-self: stretch/flex-start/flex-end/center) → multi-line align-content distribution.
- **Phase 2 simplifications**: `baseline` alignment → `flex-start`, `flex-basis: content` → `auto`, `InlineFlex` treated as block-level.
- **Routing**: `block.rs` and `layout.rs` route `Display::Flex`/`InlineFlex` children to `flex::layout_flex()`.

### elidex-script-session

- **SessionCore**: Owns `IdentityMap` + `Vec<Mutation>`. `record_mutation()` buffers, `flush()` applies all via `apply_mutation()`. `get_or_create_wrapper()` for identity mapping, `release()` for cleanup.
- **IdentityMap**: Bidirectional `(Entity, ComponentKind) ↔ JsObjectRef`. Monotonic counter for unique refs. `get_or_create()` is idempotent. `release()` single ref, `release_entity()` all refs for an entity.
- **Mutation enum**: `AppendChild`, `InsertBefore`, `RemoveChild`, `ReplaceChild`, `SetAttribute`, `RemoveAttribute`, `SetTextContent`, `SetInnerHtml` (stub), `SetInlineStyle`, `RemoveInlineStyle`, `InsertCssRule`/`DeleteCssRule` (stubs).
- **apply_mutation()**: Delegates tree ops to `EcsDom`, attribute/style ops via `world_mut()`. `SetInlineStyle` auto-inserts `InlineStyle` component if missing. Returns `Option<MutationRecord>`.
- **DomApiHandler / CssomApiHandler traits**: `Send + Sync`, `method_name()`, `spec_level()` (default Living/Standard), `invoke(this, args, session, dom) -> Result<JsValue, DomApiError>`.
- **Types**: `JsObjectRef(u64)`, `ComponentKind` enum (Element/Style/ClassList/Attributes/Dataset/ChildNodes), `DomApiError` + `DomApiErrorKind` (NotFoundError/HierarchyRequestError/InvalidStateError/SyntaxError/TypeError/Other).

### elidex-net

- **NetClient**: Top-level API integrating transport, cookies, middleware, redirect, CORS, HTTPS-upgrade. `send()` for raw HTTP, `load()` for resource loading (http/data/file).
- **HttpTransport**: Sends requests via connection pool with timeout. Wraps hyper HTTP/1.1 and HTTP/2.
- **ConnectionPool**: Per-origin pooling. `OriginKey(scheme, host, port)`. H1: up to 6 idle connections. H2: single multiplexed `SendRequest` clone. 90s idle eviction.
- **Connector**: TCP+TLS with DNS-level SSRF protection. Uses `TokioIo<StreamWrapper>` for hyper compatibility. ALPN negotiation for H2.
- **TLS**: rustls with ring provider, webpki-roots, TLS 1.2/1.3, ALPN `h2, http/1.1`.
- **CookieJar**: `Set-Cookie` parsing via `cookie` crate. Domain/path matching (RFC 6265). `SameSite=Lax` default. Third-party blocking (simplified domain comparison, TODO: eTLD+1 in M2-7). Thread-safe via `Mutex`.
- **Redirect**: Follows 301/302/303/307/308 up to max 20. 301-303 change to GET. SSRF re-validation on each hop (skipped when `allow_private_ips`).
- **CORS**: `validate_cors()` checks `Access-Control-Allow-Origin` against request origin.
- **HTTPS upgrade**: `upgrade_to_https()` rewrites HTTP URLs to HTTPS.
- **MiddlewareChain**: Adapts plugin `NetworkMiddleware` trait to internal Request/Response types.
- **data_url**: RFC 2397 parser (plain text + base64).
- **ResourceLoader trait + SchemeDispatcher**: Routes http/https, data:, file:// with cookie injection and redirect following.
- **SSRF shared module**: `elidex_plugin::url_security` — `validate_url()` + `is_private_ip()`, shared by elidex-net and elidex-crawler.

### CI

- 5 jobs: `changes` (path filter), `check` (ubuntu/macos/windows: fmt + clippy + test), `doc` (cargo doc -D warnings), `deny` (standalone), `msrv` (1.88: check + test).
- Path-based skip: `dorny/paths-filter@v3` detects `rust` (crates/\*\*, Cargo.\*, toolchain/fmt/clippy config, mise.toml) and `config` (deny.toml, Cargo.\*) changes. Push to main always runs all jobs.
- Actions pinned: `actions/checkout@v4`, `Swatinem/rust-cache@v2`, `dorny/paths-filter@v3`, `taiki-e/install-action@v2`.
- `rust-toolchain.toml`: `channel = "stable"`. MSRV `1.88` in `Cargo.toml`.


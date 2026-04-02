# Architecture: Tools (crawler, WPT, benchmarks)

## elidex-crawler

- **SSRF protection**: `validate_url()` checks scheme, private IPs (IPv4/IPv6), reserved hostnames. Custom reqwest redirect policy validates each hop.
- **robots.txt**: RFC 9309 subset. `Allow`/`Disallow` with longest-match-wins. `best_agent_match()` shared by `is_allowed`/`crawl_delay`. `Crawl-delay` validated (`is_finite() && >= 0.0`). Body fetch has 5s timeout + 512KB limit.
- **Concurrency**: Global semaphore + per-host semaphore (2 concurrent). Semaphore acquire uses `let...else` for graceful error handling. Host map evicts stale entries at 10,000 threshold via `Arc::strong_count`.
- **Content-Type**: MIME type extracted before `;` for exact match (`text/html`, `application/xhtml+xml`, `text/xml`).
- **Shared utilities**: `analyzer/util.rs` provides `strip_comments()` (CSS/JS) and `extract_tag_blocks()` (style/script), both `pub(crate)`. `MAX_EXTRACT_ITERATIONS` in `analyzer/mod.rs`.
- **Type alias**: `FeatureCount = HashMap<String, usize>` in `analyzer/mod.rs`, used across all feature structs.
- **Report aggregation**: `FeatureAggregator` pattern + `FEATURE_CSV_HEADER` constant deduplicates feature counting/CSV logic.
- **Error chain**: Retry errors use `format!("{e:#}")` to preserve full anyhow error chain.
- **`to_ascii_lowercase()` safety**: Verified — only modifies ASCII bytes, preserving byte positions. Safe to cross-index with original HTML.

## elidex-wpt

- **WPT-style test harness**: JSON-based test case format for CSS conformance testing.
- **WptTestCase**: name, html, css, expected (selector → property → value string).
- **run_test_case()**: parse_html → parse_stylesheet → resolve_styles → find element by selector → compare computed values.
- **run_test_suite()**: batch runner returning `Vec<WptTestResult>`.
- **Built-in suites**: `suites::cascade::cascade_suite()` — 10 cascade/specificity/inheritance test cases.
- **Dependencies**: elidex-html-parser, elidex-css, elidex-ecs, elidex-style, elidex-plugin, serde, serde_json.

## Benchmarks (criterion)

- **elidex-css** (`benches/css_parse.rs`): `css_parse_10/100/1000_rules`.
- **elidex-style** (`benches/style_resolve.rs`): `resolve_100/1000_flat`, `resolve_deep_100`.
- **elidex-layout** (`benches/layout_bench.rs`): `block_100`, `flex_20`.
- **Run**: `mise run bench` or `cargo bench -p <crate>`.

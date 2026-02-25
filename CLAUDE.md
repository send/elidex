# CLAUDE.md — elidex project notes

## Project Overview

elidex is an experimental browser engine written in Rust. Currently in Phase 0 (foundation).

### Workspace Structure

```
crates/
  elidex-plugin/   — Plugin traits, SpecLevel enums, PluginRegistry
  elidex-ecs/      — ECS (hecs) based DOM prototype
  elidex-crawler/  — Web compatibility survey tool (binary crate)
```

### Common Commands

```sh
mise run ci          # Run all CI checks locally (lint + test + deny)
mise run test        # cargo test --workspace
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
cargo doc --workspace --no-deps  # Build docs
```

### Test Count

101 tests total: 52 crawler + 40 ECS + 9 plugin

### Key Files

- `SECURITY.md` — Vulnerability disclosure policy
- `CONTRIBUTING.md` — Contribution guidelines
- `.github/dependabot.yml` — Automated dependency updates (Cargo + Actions)
- `deny.toml` — License allow-list + supply chain (`unknown-registry`/`unknown-git` = deny)

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
- **Placeholder types**: All `#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]`.

### CI

- 5 jobs: `changes` (path filter), `check` (ubuntu/macos/windows: fmt + clippy + test), `doc` (cargo doc -D warnings), `deny` (standalone), `msrv` (1.75: check + test).
- Path-based skip: `dorny/paths-filter@v3` detects `rust` (crates/\*\*, Cargo.\*, toolchain/fmt/clippy config, mise.toml) and `config` (deny.toml, Cargo.\*) changes. Push to main always runs all jobs.
- Actions pinned: `actions/checkout@v4`, `Swatinem/rust-cache@v2`, `dorny/paths-filter@v3`, `taiki-e/install-action@v2`.
- `rust-toolchain.toml`: `channel = "stable"`. MSRV `1.75` in `Cargo.toml`.

---

## Code Review History

- **Review #2** (2026-02-25): 37 findings — ALL FIXED
- **Review #3** (2026-02-25): 27 findings (C3/H6/M10/L8) — ALL FIXED
- **Review #4** (2026-02-25): 59 findings (C3/H7/M23/L26) — ALL FIXED
- **Review #5** (2026-02-26): 19 findings (M7/L12) — ALL FIXED
- **Review #6** (2026-02-26): 6 findings (M2/L4) — ALL FIXED
- **Review #7** (2026-02-26): 0 findings — CLEAN (final review)
- **Refactoring R1** (2026-02-26): 17 items (H5/M8/L4) — ALL COMPLETE
- **Refactoring R2** (2026-02-26): 13 items (H2/M7/L4) — ALL COMPLETE
- **Refactoring R3** (2026-02-26): 6 items (M2/L4) — ALL COMPLETE (final)


# 29. Survey Analysis

## 29.1 Survey Overview

A web compatibility survey was conducted using elidex-crawler against major Japanese (JA) and English (EN) websites.

| Metric | JA | EN |
| --- | --- | --- |
| Total sites | 451 | 449 |
| Success (HTML fetched) | 387 (85.8%) | 412 (91.8%) |
| Failed | 64 (14.2%) | 37 (8.2%) |

### Failure Breakdown

| Failure Reason | JA | EN |
| --- | --- | --- |
| Blocked by robots.txt | 5 | 16 |
| Timeout | 26 | 5 |
| Non-HTML content | 3 | 1 |
| Other connection errors | 30 | 15 |

The higher timeout rate for JA is likely due to access restrictions on Japan-hosted servers from overseas locations or low-bandwidth connections. The higher robots.txt block rate for EN reflects major platforms (Facebook, Instagram, Twitter/X, LinkedIn, Reddit, Netflix, etc.) rejecting crawlers.

## 29.2 HTML Analysis

### Deprecated Tags

| Metric | JA | EN |
| --- | --- | --- |
| Sites using deprecated tags | 17 (4.4%) | 10 (2.4%) |

**JA Deprecated Tags:**

| Tag | Total Count | Site Count | Site % |
| --- | --- | --- | --- |
| `<font>` | 94 | 9 | 2.0% |
| `<big>` | 25 | 1 | 0.2% |
| `<center>` | 9 | 7 | 1.6% |
| `<nobr>` | 2 | 1 | 0.2% |

**EN Deprecated Tags:**

| Tag | Total Count | Site Count | Site % |
| --- | --- | --- | --- |
| `<center>` | 8 | 5 | 1.1% |
| `<font>` | 6 | 4 | 0.9% |
| `<nobr>` | 2 | 1 | 0.2% |
| `<blink>` | 1 | 1 | 0.2% |

Deprecated tag usage is low (JA 4.4% / EN 2.4%). `<font>` and `<center>` are the most common.

### Deprecated Attributes

| Metric | JA | EN |
| --- | --- | --- |
| Sites using deprecated attrs | 278 (71.8%) | 297 (72.1%) |

`width` and `height` attributes are overwhelmingly dominant (JA: 59.0%/56.8%, EN: 62.6%/62.8% of sites). These are primarily used as size hints on `<img>` tags, a pattern still recommended by modern browsers for preventing layout shift.

**JA Top 5 Attributes:**

| Attribute | Total Count | Site Count | Site % |
| --- | --- | --- | --- |
| `width` | 13,973 | 266 | 59.0% |
| `height` | 12,588 | 256 | 56.8% |
| `size` | 112 | 38 | 8.4% |
| `border` | 85 | 20 | 4.4% |
| `align` | 53 | 14 | 3.1% |

**EN Top 5 Attributes:**

| Attribute | Total Count | Site Count | Site % |
| --- | --- | --- | --- |
| `width` | 17,290 | 281 | 62.6% |
| `height` | 16,662 | 282 | 62.8% |
| `size` | 894 | 31 | 6.9% |
| `color` | 591 | 74 | 16.5% |
| `text` | 88 | 5 | 1.1% |

### Parser Errors

| Metric | JA | EN |
| --- | --- | --- |
| Sites with errors | 186 (48.1%) | 181 (43.9%) |
| Total error count | 2,309 | 1,463 |

**JA Top 5 Errors:**

| Error | Count |
| --- | --- |
| Found special tag while closing generic tag | 724 |
| Duplicate attribute | 405 |
| Unexpected token | 316 |
| No `<p>` tag to close | 121 |
| Unexpected open element | 105 |

**EN Top 5 Errors:**

| Error | Count |
| --- | --- |
| Duplicate attribute | 310 |
| Unexpected token | 262 |
| Saw ? in state TagOpen | 197 |
| Found special tag while closing generic tag | 186 |
| Saw = in state BeforeAttributeValue | 85 |

Approximately half of all sites produce parser errors, but these are all handled by html5ever's spec-compliant automatic error recovery. No unrecoverable errors were observed.

## 29.3 CSS Analysis

### Vendor Prefixes

| Metric | JA | EN |
| --- | --- | --- |
| Sites using prefixes | 70 (18.1%) | 177 (43.0%) |

**By Prefix:**

| Prefix | JA Site Count (%) | EN Site Count (%) |
| --- | --- | --- |
| `-webkit-` | 66 (14.6%) | 174 (38.8%) |
| `-ms-` | 40 (8.9%) | 109 (24.3%) |
| `-moz-` | 42 (9.3%) | 127 (28.3%) |
| `-o-` | 12 (2.7%) | 51 (11.4%) |

Vendor prefix usage is significantly higher on EN sites. `-webkit-` is the most prevalent and should be prioritized in elidex's compat layer.

### Non-Standard Properties

| Property | JA Site % | EN Site % |
| --- | --- | --- |
| `-webkit-appearance` | 4.7% | 17.6% |
| `-webkit-font-smoothing` | 2.0% | 17.4% |
| `-moz-osx-font-smoothing` | 1.8% | 12.9% |
| `-moz-appearance` | 2.4% | 12.5% |
| `-webkit-tap-highlight-color` | 2.2% | 10.2% |
| `-webkit-overflow-scrolling` | 1.8% | 9.4% |
| `-webkit-text-size-adjust` | 2.2% | 13.6% |
| `-ms-overflow-style` | 2.2% | 7.6% |
| `zoom` | 0.7% | 2.9% |

### Aliased Properties (Legacy Syntax)

| Alias | JA Site % | EN Site % |
| --- | --- | --- |
| `-webkit-box-align` | 2.7% | 14.0% |
| `-webkit-box-pack` | 3.5% | 13.6% |
| `word-wrap` | 3.5% | 13.8% |
| `-webkit-box-orient` | 5.5% | 12.7% |

Legacy flexbox syntax (`-webkit-box-*`) is still in active use. `word-wrap` is a legacy alias for `overflow-wrap`.

## 29.4 JavaScript Analysis

| Metric | JA | EN |
| --- | --- | --- |
| `document.write` usage | 48 (12.4%) | 22 (5.3%) |
| `document.all` usage | 0 (0.0%) | 0 (0.0%) |

`document.all` usage is zero. `document.write` is relatively high in JA (12.4%), but most instances are from ad scripts and analytics tags. Since elidex takes a strict-only approach, these API compatibilities are not required.

## 29.5 Compat Rule Priority

Based on the survey results, compatibility rules are prioritized as follows.

### P0 (Must Have)

- **width/height presentational hints:** Apply `<img>` `width`/`height` attributes as CSS initial values. Used by over 60% of sites and essential for preventing layout shift.
- **`-webkit-` aliases:** `-webkit-appearance`→`appearance`, `-webkit-box-*`→`flex`, etc. Nearly 40% of EN sites use these.

### P1 (Should Have)

- **`appearance` property:** Standardization support for `-webkit-appearance` and `-moz-appearance`.
- **font-smoothing:** `-webkit-font-smoothing` and `-moz-osx-font-smoothing`. Used by 17% of EN sites.
- **`-webkit-text-size-adjust`:** Mobile display control. Used by 13.6% of EN sites.

### P2 (Low Priority)

- **`<font>`/`<center>` tags:** Usage below 5%. Minimal impact from non-support under elidex's strict policy.
- **`document.write`:** Not supported under strict-only approach.
- **`-ms-`/`-o-` prefixes:** Legacy browser-specific.

## 29.6 Phase 0.5 Decision Gate

### Parser Error Recovery

The survey results show parser errors in approximately half of all sites, but the nature of these errors is entirely addressable by html5ever's spec-compliant error recovery algorithm.

Main error categories:
- **Structural errors** (special tag closing, unexpected open element): handled by html5ever's tree reconstruction algorithm
- **Attribute errors** (duplicate attribute, quoting issues): first value adopted, subsequent values ignored
- **Character reference errors**: best-effort decode

No unrecoverable errors (cases where the page is completely broken) were observed.

### LLM Fallback Decision

**Provisional Decision: No-Go**

Rationale:
1. Zero empirical evidence of unrecoverable parser errors
2. html5ever's automatic recovery is sufficient for practical use
3. The cost of LLM runtime fallback (latency, memory, complexity) does not justify the return

**Project decision (2026-03-05):** All LLM-related scope, including elidex-llm-diag, is paused for now. This applies to both broken-HTML recovery and developer diagnostics.

## 29.7 Implications for Phase 1

1. **Presentational hints support is essential:** `width`/`height` attributes are used on over 60% of sites. The Phase 1 CSS parser must support presentational hints.

2. **Phased vendor prefix support:** Prioritize `-webkit-` in the compat layer. Full implementation in the Phase 3 compatibility layer.

3. **Legacy flexbox syntax:** `-webkit-box-*`→`flex` mappings should be incorporated during the Phase 2 Flexbox layout implementation.

4. **Parser design impact:** Approximately half of all sites have errors, but html5ever's recovery is sufficient. elidex-parser-tolerant (Phase 2) should leverage html5ever's error recovery as-is, keeping custom recovery logic to a minimum.

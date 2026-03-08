
# 11. Parser Design

## 11.1 Dual Parser Strategy

Two parsers serve different use cases:

| Parser | Behavior | Use Case |
| --- | --- | --- |
| parse_strict | Rejects malformed HTML with detailed error messages, similar to a compiler. Reports unclosed tags with source locations. | elidex-app mode. Developers see errors at build/dev time and fix their markup. Produces the best performance. |
| parse_tolerant | Recovers from common errors (unclosed tags, nesting mistakes). Does NOT implement full HTML5 spec error recovery (e.g., adoption agency algorithm). | elidex-browser mode. Handles real-world websites that contain markup errors. Pragmatic subset of error recovery. |

The tolerant parser implements only the error recovery patterns that are actually needed on the modern web, as determined by crawler survey data. This is a deliberate departure from the HTML5 spec, which requires full error recovery for all historical patterns.

> **Phase 0 Survey Result (Ch. 29 §29.2):** All error categories found in the survey (structural errors ~48%, duplicate attributes, unexpected tokens, etc.) are automatically recoverable by html5ever's spec-compliant error recovery. No unrecoverable errors were observed.

## 11.2 CSS Parser

CSS parsing uses the cssparser crate as a foundation, with property parsing delegated to registered CssPropertyHandler plugins. Unknown properties are silently dropped (standard CSS behavior). Legacy properties handled by elidex-compat-css are transformed into standard equivalents before reaching the plugin registry.

## 11.3 Graduated Degradation Model

A fundamental design philosophy of elidex is that broken HTML should have a cost. The current web ecosystem imposes zero penalty for malformed markup—every major browser silently fixes everything, removing any incentive for authors to write correct HTML. Elidex introduces a natural performance gradient where correctness is rewarded with speed:

| Input Quality | Latency | Handler | Behavior |
| --- | --- | --- | --- |
| Valid HTML5 | μs | elidex-core (direct) | Parsed directly by the strict parser. Maximum performance. This is the happy path. |
| Minor errors | < 1ms | Rule-based recovery | Common patterns (unclosed tags, simple nesting mistakes) handled by deterministic rules. Nearly imperceptible overhead. |
| Severely broken | — | Error display (current) | If rule-based recovery cannot handle the input, show a diagnostic error page. LLM fallback remains a future extension point in design but is currently paused. |
| Unrecoverable | — | Error display | Show an honest error page with diagnostics rather than a garbled layout. |

> **Phase 0 Survey Result (Ch. 29 §29.6):** The 900-site survey found zero cases requiring Tier 3 (LLM fallback) or Tier 4 (unrecoverable). All parser errors were handled at Tier 2 (rule-based recovery) or below.

This gradient creates a positive feedback loop for the web ecosystem. Site authors who test in elidex will naturally write cleaner HTML to get faster load times, similar to how TypeScript’s type system nudges developers toward correctness without forbidding JavaScript’s flexibility.

## 11.4 LLM-Assisted Error Recovery (Design Record, Currently Paused)

> **Project decision (2026-03-05):** LLM-related features in this section (runtime fallback, offline rule generation, and developer diagnostics) are out of active implementation scope for now. The content below is retained as design record for possible future revisit.

The LLM fallback layer is a novel approach to HTML error recovery that replaces the hundreds of hand-written error recovery rules in traditional browser parsers with a small language model that can reason about broken markup.

### 11.4.1 Runtime Architecture

The LLM is invoked only when the rule-based tolerant parser encounters errors it cannot resolve. This ensures zero overhead for well-formed pages:

```rust
pub fn parse_tolerant(input: &str) -> ParseResult {
    // Stage 1: Try rule-based recovery (fast, deterministic)
    match rule_based_parse(input) {
        Ok(dom) => ParseResult::Clean(dom),
        Err(partial) if partial.is_recoverable() => {
            ParseResult::Recovered(partial.dom, partial.warnings)
        }
        Err(broken) => {
            // Stage 2: LLM fallback (slow, best-effort)
            match llm_repair(input, &broken.diagnostics) {
                Ok(repaired_html) => {
                    let dom = strict_parse(&repaired_html);
                    ParseResult::LlmRepaired(dom, broken.diagnostics)
                }
                Err(_) => ParseResult::Failed(broken.diagnostics)
            }
        }
    }
}
```

Key design constraints for the runtime LLM:

**Local inference only: ** The model runs locally via a lightweight inference runtime (e.g., llama.cpp/candle). No network calls to external APIs. Privacy and offline capability are non-negotiable.

**Small model: ** A model in the 1–3B parameter range, quantized (Q4/Q5), targeting < 1GB memory footprint. The task (HTML structural repair) is narrow enough that a small fine-tuned model can handle it.

**Deterministic post-processing: ** The LLM’s output is re-parsed by the strict parser. If the repaired HTML still doesn’t parse, the repair is rejected. The LLM never directly produces a DOM—it only produces candidate HTML that must pass strict validation.

**User-visible indicator: ** When LLM repair is triggered, a visible indicator (e.g., an amber icon in the address bar) informs the user that the page required AI-assisted recovery and may not render exactly as intended.

### 11.4.2 Offline Rule Generation

In addition to runtime repair, the LLM plays a role in the development pipeline by generating new rule-based recovery patterns:

```
[Offline pipeline - runs periodically]

elidex-crawler results (broken HTML corpus)
  │
  ▼  LLM analysis
Pattern classification + repair rule proposals
  │
  ▼  Human review
Approved rules merged into elidex-parser-tolerant
  │
  ▼  Next release
Fewer pages need LLM fallback at runtime
```

This creates a virtuous cycle: the LLM handles novel errors at runtime, those error patterns are harvested from telemetry, the LLM proposes deterministic rules for the most common patterns, and approved rules graduate into the fast rule-based parser. Over time, the LLM fallback is invoked less and less frequently as the rule-based parser learns the patterns the LLM discovered.

### 11.4.3 Ecosystem Incentive Design

The graduated degradation model serves an explicit ecosystem goal: incentivizing better HTML authorship. The performance gradient provides natural pressure toward correctness without breaking accessibility:

**For site authors: ** Sites with clean HTML5 are measurably faster in elidex. This is visible in performance metrics (Lighthouse-equivalent scoring) and directly perceived by users.

**For users: ** No site is completely broken. Even severely malformed pages get a best-effort render. The amber indicator provides transparency about why a page is slow.

**For the ecosystem: ** Unlike outright refusal to render (which would be a nonstarter for adoption), the cost is proportional to the severity of the problem. This mirrors how search engines use page speed as a ranking signal—it’s a gentle push, not a wall.

### 11.4.4 App Mode: LLM-Powered Developer Diagnostics

In elidex-app mode, the strict parser rejects malformed HTML at development time. Here, the LLM serves a different role: generating rich, context-aware error messages.

```
error[E0042]: Unclosed element
  --> ui/main.html:32:5
   \|
32 \|     <div class="container">
   \|     ^^^^^^^^^^^^^^^^^^^^^ this <div> is never closed
   \|
   = help: Consider adding </div> before line 45:
   \|
44 \|         </ul>
45 \| +     </div>
46 \|     </section>
   \|
   = note: The LLM inferred this from the document structure.
           The <div> likely wraps the <ul> starting at line 34.
```

This provides an Elm/Rust-compiler-like developer experience that no other web application runtime offers. The LLM’s reasoning ability makes the suggestions significantly better than what rule-based heuristics can produce, and since this runs only during development, latency is not a concern.

### 11.4.5 Conditional Adoption: Data-Driven Decision

The LLM fallback layer represents a significant engineering investment (model fine-tuning, inference runtime integration, memory overhead). Its adoption is therefore conditional on survey data. The decision process is:

```
Phase 0:   900 sites (JA 451 + EN 449) × top page
           → Measure legacy feature prevalence
           → Determine what elidex-core can omit

Phase 0.5: Expanded crawl (3,000–5,000 sites × 5–10 subpages)
           → Parse all pages with html5ever, record error events
           → Classify: rule-recoverable vs. unrecoverable errors
           → Measure: what % of pages have unrecoverable errors?

Decision gate:
  Unrecoverable error rate < ~2%  → Rule-based only (LLM not worth it)
  Unrecoverable error rate ≥ ~2%  → Invest in LLM fallback
```

> **Phase 0 Survey Result (Ch. 29):**
> - Parser error detection rate: JA ~48% / EN ~44% (per site)
> - Unrecoverable error rate: **0%** (all errors auto-recovered by html5ever)
> - Deprecated tag usage: JA 4.4% / EN 2.4% (< 5%)
> - Presentational hints (width/height): used on 60%+ of sites
> - Decision gate result: 0% < 2% → **No-Go confirmed** (Phase 0.5 skipped)

Phase 0.5 is triggered only after Phase 0 results are analyzed. If the initial survey shows that the modern web is overwhelmingly well-formed HTML5, the LLM layer may be unnecessary—and avoiding unnecessary complexity is itself aligned with the elidex philosophy.

> **Phase 0 Survey Result (Ch. 29 §29.6):** The survey confirmed that "the modern web is overwhelmingly well-formed HTML5." The LLM runtime fallback layer is deemed unnecessary.

An earlier plan treated elidex-app LLM diagnostics (Section 11.4.4) as independently valuable, but this track is also paused by the 2026-03-05 project decision.

# E0 (F6) — Shell style-compat selection from the engine-wide `EngineMode`

Plan date: 2026-06-23 JST
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → Program E (F6).
Anchor design (SSoT): `docs/plans/2026-06-web-api-compat-split-design.md` §5 (E0 row) +
§3.2b (EngineMode → per-layer policy, fixed at VM construction).
Base: clean off `origin/main` `dc00ffd0` (#396 landed); re-grepped at this HEAD.
Plan-review: **`/elidex-plan-review` requested** (cross-layer = `EngineMode` threaded
into the shell + whole-engine-consistency check; not edge-dense, but the A0 §5
integrity check is worth the cheapest gate).

---

> **Umbrella status note.** The umbrella scopes E0 as *investigate-only / defer, no
> code* (§2.6 / Program-E table) **because at authoring time no mode mechanism
> existed**. That precondition is now satisfied — A1 (#396 cluster) landed the
> engine-wide `EngineMode` (`spec_level.rs:94`) — so the **anchor design §5 E0 row
> supersedes** the umbrella's investigate-only framing and moves E0 to
> implementation. §5 AC done-ifies Program E / F6 in the umbrella at landing.

## 0. Problem (F6)

The shell resolves styles **unconditionally through the compat path**:
`re_render` (`lib.rs:778`), the two resolves in `run_scripts_and_finalize`
(`pipeline.rs:66`/`:108`), and the paged path (`pipeline.rs:135`) all call
`resolve_with_compat`, which always prepends the **legacy UA stylesheet**
(`legacy_ua_stylesheet()`) and applies **HTML presentational hints**
(`get_presentational_hints`). There is no way for a core/app session to resolve
against the modern UA baseline only. A1 introduced the engine-wide `EngineMode`
authority (`crates/core/elidex-plugin/src/spec_level.rs`) but it is **not wired
into the shell** (`rg EngineMode crates/shell` = 0). F6 = make the shell's
compat-vs-core style choice derive from that one `EngineMode`.

## 1. Design (fixed by A0 §5 / R3-6 — this memo applies it)

**1a. `StyleCompatPolicy`, derived *in parallel* from `EngineMode` (R3-6).**
Add to `elidex-plugin` (next to `SpecLevelPolicy`):

```rust
// elidex-plugin/src/spec_level.rs
pub struct StyleCompatPolicy { legacy_presentational: bool }   // private field
impl StyleCompatPolicy {
    /// Legacy UA stylesheet + HTML presentational-hint declarations participate
    /// in the cascade. `BrowserCompat` → true; `BrowserCore`/`App` → false
    /// (modern UA baseline only).
    pub fn presentational_compat(&self) -> bool { self.legacy_presentational }
}
impl EngineMode {
    pub fn style_compat_policy(self) -> StyleCompatPolicy {
        StyleCompatPolicy { legacy_presentational: matches!(self, EngineMode::BrowserCompat) }
    }
}
```

This mirrors `EngineMode::spec_level_policy() -> SpecLevelPolicy` exactly: one
engine-wide mode, **each layer derives its own policy** via its own
`EngineMode::*_policy()` method. The style policy is derived **parallel to** (never
via) the Web-API `SpecLevelPolicy` — so the CSS pipeline never depends on a Web-API
enum (R3-6 whole-engine consistency). Default (all-false) = `BrowserCompat` =
zero-behavior-change baseline, identical to the `SpecLevelPolicy` `Default` rationale.
`Deprecated`/`with_legacy_excluded` have no style analogue (no compile-time style
ceiling exists), so the type stays minimal; it can grow a field if a future mode
must split "legacy UA sheet" from "presentational hints" (today they toggle together).

**1b. `PipelineResult.engine_mode: EngineMode` — the shell's single authority.**
The shell is the embedder; it holds **one** `EngineMode` on `PipelineResult`
(the per-content state `re_render` mutates and the 4 builders produce). All 4
builders default it to `EngineMode::BrowserCompat` (production invariant). This is
the **single mode authority**: today only the style layer reads it (the production
JS engine is still boa, which is mode-agnostic and delete-destined — S5/D-26 PR7);
at the boa→elidex-js-VM cutover the **same field** feeds VM construction
(`ElidexJsEngine::new_with_mode(result.engine_mode)`). One value, one consumer now,
two post-cutover — never a second authority.

**1c. `resolve_with_mode` dispatcher (replaces `resolve_with_compat`).** One helper,
consulted at **every** shell resolve seam (One issue, one way). It branches on
`engine_mode.style_compat_policy().presentational_compat()`:

- **compat arm** (`BrowserCompat`) = today's exact call: `legacy_ua_stylesheet()`
  as an extra UA sheet + `&get_presentational_hints` + `Some(registry)`. **Byte-identical.**
- **core arm** (`BrowserCore`/`App`) = `resolve_styles_with_compat(dom, author,
  &[], &no_hints, viewport, medium, None)` — i.e. the **`elidex_style::resolve_styles`
  semantics** (empty extra-UA, no hints, no registry), but **medium-parametrized**
  so no `@media print` is silently dropped (the 3-arg `resolve_styles` hardcodes
  `Medium::Screen`). `no_hints` = a shell-local 1-line empty generator (elidex-style's
  is private; this is "no compat data", not a duplicated algorithm).

`medium` is threaded to **both** arms (no ignored param). All 4 call sites route
through it: `re_render` passes `result.engine_mode`; `run_scripts_and_finalize`
and `build_paged_pipeline` take an `engine_mode: EngineMode` param (builders pass
`BrowserCompat`).

**Note on `registry`:** `resolve_styles_with_compat`'s `_registry` arg is currently
**unused** (`_`-prefixed), so `Some(registry)` vs `None` is behaviorally inert today
— the *only* real compat-vs-core delta is the legacy UA sheet + presentational hints.
The core arm passes `None` for honesty (no registry-driven compat dispatch in core).

## 2. Five-axis pre-analysis (for `/elidex-plan-review`)

- **Axis 1 — Layering.** `StyleCompatPolicy` is pure decision data in `elidex-plugin`
  (shared vocabulary). **`elidex-style` stays mode-agnostic** — it does *not* import
  `EngineMode`/`StyleCompatPolicy`; the shell (embedder) holds the mode, derives the
  policy, and *selects which existing elidex-style entry to call*. The style/compat
  **algorithm stays put**: `legacy_ua_stylesheet`/`get_presentational_hints` remain in
  `elidex-dom-compat`, `resolve_styles*` in `elidex-style`. E0 moves **no algorithm**;
  it is a shell call-site mode selection (F6 counter-signal: crate boundary maintained,
  no compat-impl relocation).
- **Axis 2 — ECS-native / side-store.** None. `EngineMode` is shell-level session
  state on `PipelineResult` (a plain struct field), not per-entity state — not a
  component candidate (it is not a per-entity fact; same class as `viewport`). No new
  `Send+Sync` per-entity side-store. (Pre-cleared like A0 §4.3.)
- **Axis 3 — Pragmatic shortcut.** The core arm is **test-only** (no real session
  selects `BrowserCore`/`App` until `#11-async-core-storage-cookiestore`, per the
  `EngineMode` doc-comment). This is a *spec'd* precondition, not a shortcut: wiring
  the gate now while real-session selection waits on async-core is the A0-sanctioned
  sequencing. Production stays `BrowserCompat` → byte-identical.
- **Axis 4 — Spec.** No new spec surface. The compat layer (legacy UA sheet +
  presentational hints) already cites its anchors in `elidex-dom-compat`; E0 only gates
  *whether* it applies. Whole-engine core/compat/deprecated consistency (CLAUDE.md):
  CSS now joins HTML/DOM/Web-API/ES under the same `EngineMode`-derived policy pattern.
- **Axis 5 — Project context / file size.** `lib.rs` is 924 lines (near the 1000-line
  touch-time-split line). Net add is lean (~field + 4 one-line defaults + dispatcher
  branch + small tests); **no split needed** (no real new cohesion seam; the resolve
  helpers stay a cohesive unit). Additions kept minimal per the handoff.

## §3. Spec coverage map

E0 adds **no new spec branch** — it gates *whether* the already-implemented compat
surface applies. The single spec home for both gated features is HTML §15.2
(webref-verified; the legacy UA stylesheet and presentational hints both live there).
The gate is **binary** (compat = full §15.2 surface applied; core = withheld), so
each row's branch set is fully enumerated (Full enum ✓).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §15.2 The CSS user agent style sheet and presentational hints | legacy UA stylesheet application | BrowserCompat: applied / BrowserCore+App: withheld | `resolve_with_mode` (NEW) compat arm → `legacy_ua_stylesheet()` (shell, unchanged) | ✓ (binary gate) | no |
| WHATWG HTML §15.2 The CSS user agent style sheet and presentational hints | presentational-hint declarations | BrowserCompat: applied / BrowserCore+App: withheld | `resolve_with_mode` (NEW) compat arm → `get_presentational_hints` (shell, unchanged) | ✓ (binary gate) | yes (HTML attr → decl) |

**Breadth**: K=1 spec (html), M=2 entries → single-PR scope.
**Anchors** (HTML §15.2): section = `#the-css-user-agent-style-sheet-and-presentational-hints`
(legacy-UA-sheet row); the presentational-hint row has the finer dfn anchor
`#presentational-hints` (term "presentational hint", same §15.2 — no UA-sheet-specific
dfn exists, so row 1 keeps the section anchor). webref-verified 2026-06-23.

### §3.1 User-input touch audit

The presentational-hint row carries user-input flow: author-controlled HTML
attributes (`bgcolor`/`width`/`align`/…) convert to CSS declarations via
`get_presentational_hints` (in `elidex-dom-compat`, **unchanged**). E0 does **not**
touch that conversion — it only gates whether the conversion runs (compat) or is
skipped (core). So the user-input *surface* is unchanged; E0 changes only the
on/off. No new attribute parsing, no new sanitization branch. The legacy-UA-sheet
row is static rules (no user input).

## §4. Touch-set (re-grepped at `dc00ffd0`)

New symbols: `StyleCompatPolicy` (NEW), `StyleCompatPolicy::presentational_compat`
(NEW), `EngineMode::style_compat_policy` (NEW), `resolve_with_mode` (NEW) — all
introduced by E0. All other backticked symbols are existing (grep-verified
2026-06-23). The 4-builder / 4-resolve-site counts are verified below the table.

| File | Change |
|---|---|
| `crates/core/elidex-plugin/src/spec_level.rs` | + `StyleCompatPolicy` (NEW) + `EngineMode::style_compat_policy` (NEW) + unit tests |
| `crates/core/elidex-plugin/src/lib.rs` | export `StyleCompatPolicy` (NEW) at the `:66` re-export line |
| `crates/shell/elidex-shell/src/lib.rs` | `PipelineResult` (`:386`): + `engine_mode: EngineMode` field |
| `crates/shell/elidex-shell/src/lib.rs` | `resolve_with_compat` (`:334`) → `resolve_with_mode` (NEW) — add mode branch; compat arm unchanged |
| `crates/shell/elidex-shell/src/lib.rs` | `re_render` (`:685`): the resolve at `:778` → `resolve_with_mode(.., result.engine_mode)` |
| `crates/shell/elidex-shell/src/lib.rs` | 4 builders: set `engine_mode: BrowserCompat` in each `PipelineResult` literal; pass it to `run_scripts_and_finalize` |
| `crates/shell/elidex-shell/src/pipeline.rs` | `run_scripts_and_finalize` (`:50`): + `engine_mode` param; resolves (`:66`/`:108`) via `resolve_with_mode` |
| `crates/shell/elidex-shell/src/pipeline.rs` | `build_paged_pipeline` (`:124`): + `engine_mode` param (callers pass `BrowserCompat`); resolve (`:135`) via `resolve_with_mode` |
| `crates/shell/elidex-shell/src/tests.rs` | dispatcher both-arm unit test + `re_render` BrowserCore-vs-BrowserCompat integration test |

Enumeration artifacts (verified 2026-06-23):
- **4 `PipelineResult` literal builders** via `grep -n 'PipelineResult {' lib.rs` →
  lines 506 / 578 / 651 / 854.
- **4 `run_scripts_and_finalize` call sites** via `grep -n 'run_scripts_and_finalize(' lib.rs` →
  lines 487 / 562 / 635 / 837 (one per builder).
- **4 shell resolve sites** via `grep -rn 'resolve_with_compat' shell/src` →
  `lib.rs:778`, `pipeline.rs:66/108/135` (def at `lib.rs:334`).

iframes resolve through `re_render` (`content/iframe/thread.rs:107`,
`render.rs:36`) on their own `PipelineResult`, so they inherit the field
automatically; their builders default `BrowserCompat` (correct for production).

## §5. Invariants / AC

1. Shell resolution selects compat-vs-core from the **`EngineMode`-derived
   `StyleCompatPolicy`** (hard-wired `resolve_with_compat` default removed) — F6 close.
2. Style-compat policy derived **parallel** to (not via) the Web-API `SpecLevelPolicy`
   — `elidex-style` has no `EngineMode` dependency (R3-6).
3. `BrowserCompat` path = **byte-identical** (existing render/tests unchanged).
4. `BrowserCore`/`App` path = core resolution (no legacy UA, no hints); both arms
   covered by tests. Real session stays `BrowserCompat` (async-core precondition noted).
5. Compat algorithm stays in `elidex-dom-compat`; crate boundaries / no impl moved.
6. Single `EngineMode` authority (no style-only mode); same field feeds VM at cutover.

## §6. Open questions for plan-review

1. **Policy shape:** is a single-bool `StyleCompatPolicy` + one `presentational_compat()`
   query right, or should "legacy UA sheet" and "presentational hints" be separately
   queryable now (they toggle together for all three current modes)? (Recommend single;
   grow when a mode splits them.)
2. **Mode storage:** `PipelineResult.engine_mode` (per-content, builders default
   `BrowserCompat`) vs. an `App`-level field passed into builders. (Recommend
   `PipelineResult` — it is the only mode-reachable seam from `re_render`, and
   production is uniformly `BrowserCompat`; lift to `App` when App-mode becomes real.)
3. **Core-arm medium:** thread `medium` into the core arm via `resolve_styles_with_compat`
   (recommended, no medium-drop) vs. literally call the 3-arg `resolve_styles`
   (matches A0 §5 wording but hardcodes `Screen`; the only non-Screen site is the dead
   paged path). Confirm the medium-correct reading is preferred.

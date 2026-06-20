# Elidex Philosophy-Alignment Umbrella Plan

Plan date: 2026-06-20 JST
Status: **PLAN ONLY — no implementation, no code change beyond this doc.**
Follows up: `docs/audits/2026-06-elidex-philosophy-implementation-audit.md` (F1–F6).
Audience: Claude / maintainers (and Codex via `## Review guidelines`).

> This is an umbrella plan that decomposes the remediation of audit findings
> F1–F6 into a multi-PR program. Per `CLAUDE.md` ("Edge-dense work = multi-PR
> program + 実装前 plan-review 必須"), **F1, F2, and F3 are edge-dense and may
> NOT be fixed in a single PR**; each constituent PR must pass
> `/elidex-plan-review` before implementation. This doc is the umbrella; the
> per-PR plan-memos are written later, one per slice.
>
> The audit is a 2026-06-19 snapshot. Every finding below was **re-checked
> against current `main` (HEAD `2f4a9d5a`, 2026-06-20)** before being scoped.
> Re-check deltas — including one finding whose central framing was wrong — are
> recorded in §2. Do not anchor downstream work on the stale audit prose; anchor
> on §2.

---

## 1. Scope

### 1.1 In scope (this program)

Remediation of the six philosophy-drift findings from the audit:

| Finding | One-line | Audit sev | Re-checked sev (§2) |
|---|---|---|---|
| F1 | Sync `localStorage`/`sessionStorage` on the core VM surface | IMP | IMP (confirmed) |
| F2 | `document.cookie` on the core Document binding (+ stale "stub" doc) | IMP | IMP (confirmed) + clerical sub-fix |
| F3 | DOM write paths vs the `ScriptSession` mutation boundary | IMP | **IMP, reframed** — see §2.3 |
| F4 | iframe `contentDocument`/`contentWindow` parity null stubs | IMP | IMP (confirmed; **no defer slot**) |
| F5 | HTML tag→prototype routing hard-coded in the VM | MIN | MIN (confirmed) |
| F6 | Shell pipeline defaults to compat style resolution | MIN | MIN, **partly moot** — see §2.6 |

The program also covers the *infrastructure that does not yet exist* but is a
precondition for F1/F2: there is currently **no** `WebApiSpecLevel`/`DomSpecLevel`
runtime gate in the VM, and **no** `elidex-app`/`elidex-browser` mode mechanism
anywhere in the repo (§2.7). "Move X behind compat" is therefore not a move —
it is a build-the-boundary-first problem.

### 1.2 Out of scope (explicitly NOT this program)

- **Implementing the full iframe browsing-context model** (sub-frame
  browsing-context entity, cross-VM Document/Window proxies, same-origin access
  checks). That is gated behind the `world_id` / cross-DOM migration program and
  S5/boa removal (see `MEMORY.md` Active state). F4 here only decides
  *defer-slot-or-remove* and writes the eventual implementation plan; it does
  not implement proxies.
- **The bench-driven O(1) tag-dispatch lookup table** (master roadmap §H-7).
  F5 here only re-frames that slot as a plugin-metadata question; it does not do
  the optimization.
- **Building `elidex-app`.** F6's "app vs browser mode observability" premise
  presupposes a mode mechanism that does not exist; this program does not create
  elidex-app, only records the foundational gap.
- **The S0→S5 media-query / boa-removal flagship** (JS-side, in flight). This
  program must *coordinate* with it (§7) but does not touch its slices.
- **Any ECMAScript core/compat (LegacySemantics) work.** The audit's
  Non-Findings (`with`, sloppy direct-eval) are already aligned; ES core/compat
  is the separate LegacySemantics compat-plugin track and is out of scope here.

### 1.3 Boundary with the in-flight HTML-side / JS-side sessions

Two sessions are reportedly active in parallel. Their likely surfaces (from
`MEMORY.md` Active state) and the resulting boundaries:

- **JS-side session** — flagship S0 media-query → S5 boa removal; next slice is
  **media Slice 2b** (`matchMedia` + `MediaQueryList`/`MediaQueryListEvent`).
  Slice 2b registers `matchMedia` on `Window`, i.e. it **edits
  `crates/script/elidex-js/src/vm/host/window.rs`** — the *same file* F1 edits.
  → **F1 and Slice 2b collide on `window.rs`.** This program must not open a
  window.rs-editing PR while Slice 2b is open; see §7.
- **HTML-side session** — current task is **`#11-dialog-form-method`**
  (`<form method=dialog>` → close ancestor `<dialog>` + returnValue), touching
  `elidex-form/submit.rs`, `crates/shell/.../form_input.rs`, and the dialog-close
  path (VM-host today); the focus program **A2b** (click-ancestor climb +
  editing-host + iframe/legacy/trigger *focus* cutover) is queued after it.
  Neither touches iframe `contentDocument`/`contentWindow` accessors, the F6 style
  pipeline (`pipeline.rs`/`lib.rs` style resolution ≠ `form_input.rs`), or the
  attribute setters. → **nil** collision with F1/F2/F3/F5; **low** with F4
  (different file in the same crate) and F6 (different shell file). Confirm at
  PR-open time (§7).
- **Codex worktrees** (`/private/tmp/elidex-design-claude`,
  `/private/tmp/elidex-philosophy-audit`) correspond to already-merged PRs #365
  (CLAUDE.md philosophy) and #366 (the audit). Not active conflicts.

This program's PRs live primarily in `vm/host/` (JS-side territory) and in
engine-independent crates. Ownership and sequencing to avoid collisions are in
§3 and §7.

---

## 2. Current-Code Re-check (evidence as of HEAD `2f4a9d5a`)

Method: targeted reads + read-only sub-agent sweeps over `crates/script/elidex-js/`,
`crates/core/elidex-ecs/`, `crates/css/elidex-style/`, `crates/shell/`. Audit
line numbers had drifted a few lines (files last touched 2026-06-13…06-17, all
*before* the audit date); the drift is purely positional unless noted.

### 2.1 F1 — sync storage on the core surface — **CONFIRMED**

- `crates/script/elidex-js/src/vm/host/window.rs:522-525` — `WINDOW_STORAGE_ACCESSORS`
  table (`localStorage`, `sessionStorage`); **installed** onto `Window.prototype`
  at `window.rs:438` (`install_ro_accessors(...)`). (Audit cited :522 for the
  table — correct; the install site is :438.)
- `window.rs:530` — `native_window_get_local_storage` getter (direct VM host
  binding; `sessionStorage` sibling at `:541`). (Audit cited :527 = docstring.)
- `crates/script/elidex-js/src/vm/host/storage.rs:1` — `//! Storage interface
  … — VM thin binding`; the 5 methods + `length` accessor are installed in
  `register_storage_global` at `storage.rs:237-271`.
- `storage.rs:82-92` — opaque-origin deviation doc. **It already cites a defer
  slot**: `#11-storage-opaque-origin-securityerror` ("the per-VM sentinel bucket
  is a pre-existing pragmatic fallback … should throw `SecurityError`"). So the
  opaque-origin sub-issue is tracked; the *core-vs-compat placement* of `Storage`
  itself is not.

**Gate check (Axis 1b / whole-engine consistency):** `localStorage`/
`sessionStorage` are **unconditionally installed** — there is no per-API gate
(§2.7). Design doc `14-script-engines-webapi.md` §14.4.2 + ADR #16 classify them
as compat-only with async `elidex.storage` as the core equivalent.
**`elidex.storage` does not exist as a JS-visible namespace** (re-checked: grep
hits are Rust backend types, not a JS global). So F1 is not just "move it" — the
core replacement is also absent.

### 2.2 F2 — `document.cookie` on core Document — **CONFIRMED + clerical sub-fix**

- `crates/script/elidex-js/src/vm/host/document.rs:600` — `native_document_get_cookie`,
  a **real** impl delegating to `elidex_net::CookieJar::cookies_for_script`
  (`document.rs:628-631`). (Audit cited :594 = docstring.)
- `document.rs:646` — `native_document_set_cookie`, **real** setter forwarding to
  `CookieJar::set_cookie_from_script` (`:682`). It does *not* drop writes when a
  jar is bound. (Audit cited :639 = docstring.)
- `document.rs:1098-1100` — **stale comment still present**, verbatim: "`cookie`
  is currently a stub whose setter silently drops writes (see the setter
  docstring for the PR6 integration path)." Contradicted by the real setter
  above. A **second** stale comment exists at `navigator.rs:72-75` (same "silently
  drop" claim). → These two stale comments are a standalone clerical correction
  (Axis 3 docstring-contract ↔ body), separable from the larger migration.
- Core equivalent `CookieStore`/`cookieStore`: **not implemented** (zero hits).

### 2.3 F3 — DOM writes vs the `ScriptSession` boundary — **REFRAMED (audit framing partly wrong)**

The audit's *line-level* claims are accurate (modulo offsets): `attr_set`
(`element_attrs.rs:112`) and the reflected-IDL setters
(`html_input_proto.rs:460`, `html_button_proto.rs:183`,
`html_element_proto.rs:430`, `html_select_proto.rs:254`, and ~40 more — see the
write-site sweep in §A) do call `EcsDom::set_attribute` / `remove_attribute`
directly. But the audit's **inference is wrong on two counts**, confirmed by
direct read:

1. **Direct writes do NOT bypass observers/reconcilers.** `EcsDom::set_attribute`
   (`crates/core/elidex-ecs/src/dom/attribute.rs:101`) *is itself* the canonical
   chokepoint: it runs `reconcile_attribute_derived_components` + `rev_version`
   (`:200-201`) and then `dispatch_event(MutationEvent::AttributeChange)`
   (`:118`); `remove_attribute` (`:258`) does the same (`:274-294`).
   `dispatch_event` drives the single `ConsumerDispatcher`, which fans out to
   custom-element reactions (`attributeChangedCallback`), form-control reconcile,
   event-handler-attr, canvas, base-url, live-range, and node-iterator consumers.
   So custom elements / form state / style derivation / **live collections**
   (via `rev_version`) are **not** bypassed by direct calls.
2. **`setAttribute`/`getAttribute`/`removeAttribute` already have
   `invoke_dom_api` routes.** `element_attrs.rs:218` (`setAttribute`) and `:202`
   (`getAttribute`) dispatch through `dom_bridge::invoke_dom_api` to the
   engine-independent `elidex-dom-api` handlers, which themselves bottom out at
   the same `EcsDom::set_attribute`. The audit's claim that "only tree ops go
   through the bridge" is stale.

**The genuine, narrower issue** (this is what F3 should remediate):

- (a) **MutationObserver coverage gap.** The JS-level `MutationObserver`
  (WHATWG DOM §4.3) is *not* a `ConsumerDispatcher` consumer. It is fed only by
  `Vm::deliver_mutation_records` (`vm_api.rs:867`, an embedder API). The **only**
  production callers in `vm/host/` are `dom_inner_html.rs:148` and `:362`
  (innerHTML/fragment). Therefore a JS `el.setAttribute(...)`, reflected setter,
  `removeAttribute`, **or** tree mutation produces **no `MutationRecord`** in the
  elidex-js engine — `new MutationObserver(...)` does not observe them. The
  MutationObserver delivery tests construct `SessionRecord`s by hand and call
  `deliver_mutation_records` directly; none asserts that a JS-level mutation
  yields a record. This gap is **uniform across both the bridge path and the
  direct path** — it is *not* a direct-vs-bridge distinction.
- (b) **Two coexisting mutation-notification mechanisms** ("One issue, one way"
  drift): the `EcsDom` `ConsumerDispatcher` (what VM writes actually use) and the
  `SessionCore` mutation buffer + `flush` → `deliver_mutation_records` (what the
  shell's innerHTML path and the legacy/boa session path use). The new CLAUDE.md
  rule "ScriptSession as the sole Script↔ECS boundary" + ADR #17 + design §13
  describe a single session-mediated path; the elidex-js VM does **not** route
  writes through the `ScriptSession` mutation buffer at all (zero
  `record_mutation`/`session.flush` calls under `crates/script/elidex-js/src/vm/`).
- (c) **Asymmetry / Layering**: `setAttribute` goes through the bridge but
  `removeAttribute` (`native_element_remove_attribute`, `element_attrs.rs:226`)
  uses a file-local `attr_remove` helper (`:177`) even though a `"removeAttribute"`
  handler is registered. Reflected IDL setters do their attribute write VM-side
  rather than through a `DomApiHandler`.

→ F3 stays **IMP** and **edge-dense**, but the program target shifts from "stop
writes bypassing observers" (false) to **"(a) make JS DOM writes produce
MutationObserver records; (b) converge on one canonical mutation-notification
path between `ConsumerDispatcher` and the session buffer; (c) resolve the
bridge/direct + setAttribute/removeAttribute asymmetry."** This reframe is itself
the strongest argument for an audit-first, plan-review-gated approach: the
mechanism must be confirmed before any fix (this re-check already overturned the
original framing).

### 2.4 F4 — iframe parity stubs — **CONFIRMED; no defer slot**

- `html_iframe_proto.rs:31-40` — module doc calls `contentDocument`/
  `contentWindow` "**Parity null stubs**"; `:115` installs them (getter-only);
  `:312` and `:323` return `Ok(JsValue::Null)`.
- **No `#11-*` defer slot, no TODO/FIXME.** The only tracking is a narrative
  "tracked in the M4-12 cutover residual roadmap" (`:37`). This fails CLAUDE.md's
  requirement that a phase-constraint stub carry a *defer slot with why /
  trigger / date*. So F4 has a concrete, cheap first action: **decide
  remove-vs-formal-slot**, and if retained, register the slot.

### 2.5 F5 — hard-coded tag→prototype routing — **CONFIRMED (MIN)**

- `elements.rs:186` `tag_specific_html_prototype` + helpers at `:258`/`:362`/`:399`
  — ~68 `tag_matches_ascii_case` branches across 4 fns.
- **Defer slots are cited** (`#11-tags-T1-v2`, `#11-tags-T2a-url-bearing`,
  `#11-tags-T2b-passive`, `#11-tags-T2c-table`, `#11-tags-T2d-interactive`) and a
  perf-tradeoff comment points the O(1)-lookup optimization at master roadmap
  §H-7 (`elements.rs:188-193`, `:250-254`). So the *cost* is tracked; the
  *plugin-first shape* concern (CLAUDE.md "Plugin-first extensibility") is not.
  MIN, investigate-only for now.

### 2.6 F6 — shell defaults to compat resolution — **CONFIRMED but partly moot (MIN)**

- `pipeline.rs:64`/`:96`/`:121` call `resolve_with_compat`; `lib.rs:261`
  defines it (legacy UA sheet + presentational hints). Confirmed.
- A non-compat core path **exists but is unused in production**:
  `elidex-style::resolve_styles` (`lib.rs:88`) is called only by tests / benches /
  the WPT harness.
- **Key re-check delta:** there is **no `elidex-app`/`elidex-browser` crate, no
  `Mode` enum, and no compat-mode flag anywhere** (repo-wide grep negative). So
  the audit's "app/core paths may observe compat" premise has no mechanism to
  attach to today. F6 is therefore **investigate-only / defer** until a mode
  mechanism is introduced (which is the same precondition as F1/F2; §2.7).
- Minor adjacent observation (report-only, not a finding): the underlying
  `resolve_styles_with_compat` takes `_registry: Option<&…>` (unused) while the
  shell-side docstring claims it enables handler dispatch. Note for the F6
  investigation, not actioned here.

### 2.7 Cross-finding precondition: no compat-gate / mode mechanism exists

Confirmed across F1/F2/F6: there is **no** `WebApiSpecLevel`/`DomSpecLevel`
runtime gate in the VM, **no** compile-time per-API feature gate (the only `cfg`
is the whole-module `#![cfg(feature = "engine")]`), and **no** app/browser mode
switch. The design docs describe these (ADR #14/#16, §12.1.1, §14.4); the VM has
not grown them yet. **This is the structural root** behind F1, F2, and the
actionable half of F6: you cannot "exclude sync APIs in app mode" or "toggle a
method by `DomSpecLevel`" until that mechanism is designed. PR0 (§5) makes this
the first thing decided.

---

## 3. Ownership Map

Most findings live in `crates/script/elidex-js/src/vm/host/` (JS-side) and/or in
engine-independent crates. Assignment is by *who can land it without colliding*
and *which design lens dominates*.

### 3.1 → New cross-cutting session (owns the umbrella)

- **F1 + F2 design + compat infrastructure (Program A).** Spans the VM surface
  (`window.rs`, `document.rs`, `storage.rs`, `navigator.rs`), the *non-existent*
  compat gate/mode mechanism, and (per design §14.4.2) new compat crates
  (`elidex-api-storage-compat`, `elidex-api-cookies-compat`) + the async core
  equivalents (`elidex.storage`, CookieStore). Cross-cutting by nature; the VM
  edits must be sequenced against the JS-side `window.rs` work (§7).
- **F3 (Program B).** Architectural; spans `vm/host/`, `elidex-ecs`
  (`ConsumerDispatcher`), `elidex-script-session` (`SessionCore` buffer), and
  `elidex-api-observers`. Not a localized JS-side change.

### 3.2 → JS-side session (or close coordination with it)

- **F2 stale-comment correction** (`document.rs:1098`, `navigator.rs:72`): a
  clerical doc fix in JS-side files. Cheapest to hand to whoever next touches
  those files, OR land as an isolated micro-PR when the JS-side session is not
  mid-edit on `document.rs` (low collision; `document.rs` is not on the Slice 2b
  path). No plan-review.
- **Any actual VM-surface edits in Program A** (moving the accessors) — these are
  JS-side territory and **must not** be opened while media Slice 2b holds
  `window.rs`. Either the JS-side session executes them under the cross-cutting
  plan, or the cross-cutting session does them after Slice 2b lands + rebases.

### 3.3 → HTML-side session (or its adjacency)

- Nothing is naturally HTML-side. **F4's eventual implementation** touches iframe
  browsing-context, which is adjacent to the HTML-side focus program's
  iframe-focus cutover (A2b) and to the `world_id`/cross-DOM program — flag for
  awareness, but F4's *near-term* action (defer-slot decision) is doc-only and
  owner-agnostic.

### 3.4 → Investigate-only (no implementation this program)

- **F5** — already slot-tracked; the open question (plugin-metadata-driven
  dispatch vs VM-local table) is investigated and folded into §H-7's framing, not
  implemented.
- **F6** — moot until a mode mechanism exists; record the foundational gap, do not
  add a speculative policy parameter to a single-caller pipeline.
- **F1/F2 deep migration** — *investigate/design first* (PR0), implement only
  after the compat mechanism is designed and reviewed.

---

## 4. Proposed Multi-PR Program

Legend: **PR-R** = `/elidex-plan-review` required before implementation.
Acceptance criteria (AC) are gate conditions, not aspirations.

> Per CLAUDE.md base-case rule: once the umbrella is approved and a per-PR slice
> has passed plan-review with a single invariant-axis intersection, that slice is
> a terminal unit (allowed single PR) even though it touches the same subsystem.

### Program A — Web API core/compat split (F1, F2) — edge-dense

| PR | Purpose | Main files / crates | Do-not-touch | Depends on | Plan-review | AC |
|---|---|---|---|---|---|---|
| **A0 (= PR0, §5)** | Compat-split **audit + supported-surface inventory + boundary design** (doc only) | `docs/plans/` | any `.rs` | — | recommended (umbrella plan-review) | Inventory of every core-VM Window/Document/navigator API classified Modern/Legacy/Deprecated; decision on the gate/mode mechanism (§2.7); decision on where compat storage/cookie + async core equivalents live; explicit list of which subsequent PRs are needed |
| **A1** | Introduce the **compat-gate / spec-level mechanism** the VM lacks (per A0's decision) | `elidex-js` VM host registration plumbing; possibly `elidex-plugin` SpecLevel | algorithm bodies; storage/cookie semantics | A0 | **PR-R** | A mechanism exists by which an API can be classified + conditionally installed; localStorage/cookie still installed (no behavior change yet); mechanism covered by tests |
| **A2** | **Gate** `Storage` (`localStorage`/`sessionStorage`) **+ `StorageEvent`** behind the compat boundary — **gate-only; the async core `elidex.storage` is OUT of A2 scope** (deferred via `#11-async-core-storage-cookiestore`, A0 §4.2) | `vm/host/storage.rs`, `window.rs`, `vm/globals.rs`; opt. `elidex-api-storage-compat` (per A0 §4.1) | `document.rs` cookie code; async-core build | A1 | **PR-R** | Sync storage + `StorageEvent` reachable only under `BrowserCompat`; opaque-origin slot `#11-storage-opaque-origin-securityerror` re-evaluated; behavior unchanged for browser-compat; tests green |
| **A3** | **Gate** `document.cookie` behind the cookies-compat boundary **+ couple `navigator.cookieEnabled`** — **gate-only; CookieStore core path is OUT of A3 scope** (same async-core deferral) | `vm/host/document.rs`, `vm/host/navigator.rs`; opt. `elidex-api-cookies-compat` (per A0 §4.1) | storage code; async-core build | A1 (parallel to A2) | **PR-R** | `document.cookie` reachable only under `BrowserCompat`; `cookieEnabled` value tracks cookie reachability; cookie-file stale comments removed (full §1.5 sweep = independent F2 micro-PR, A0); tests green |

A2 and A3 may proceed in parallel after A1. The **window.rs edits in A2 must be
sequenced after JS-side media Slice 2b** (§7).

### Program B — ScriptSession mutation boundary / MutationObserver coverage (F3) — edge-dense

| PR | Purpose | Main files / crates | Do-not-touch | Depends on | Plan-review | AC |
|---|---|---|---|---|---|---|
| **B0** | **Mutation-path audit doc**: confirm §2.3 mechanism end-to-end (every write site; the two notification mechanisms; the exact MutationObserver gap), and choose the canonical path | `docs/plans/` (or extend `docs/audits/`) | any `.rs` | — | recommended | A complete write-site map (seeded by §A); an unambiguous statement of which path is canonical (session buffer vs `ConsumerDispatcher`) and why; the chosen target architecture for MutationObserver record production |
| **B1** | Make JS DOM mutations produce **MutationObserver records** through the chosen canonical path | `vm/host/` attribute + tree natives; `elidex-script-session`; `elidex-api-observers` | unrelated `EcsDom` internals | B0 | **PR-R** | A JS `el.setAttribute`/`removeAttribute`/reflected-setter/tree-mutation yields the correct `MutationRecord`(s) per WHATWG DOM §4.3; new JS-level (not hand-constructed) MutationObserver tests pass; no double-delivery with the innerHTML path |
| **B2** | **Unify** the bridge vs direct attribute paths + `setAttribute`/`removeAttribute` asymmetry into one canonical form (One issue, one way) | `vm/host/element_attrs.rs`, reflected-IDL setter macros, `dom_bridge.rs` | — | B1 | **PR-R** | Reflected setters + attribute API route through one documented mechanism; no "new seam + N legacy" coexistence; behavior unchanged; tests green |

B1 before B2: get correctness (records) right, then collapse the decision
surface. B2 may be merged into B1 if plan-review finds the unification is the
natural shape of the fix rather than a separable step.

### Program C — iframe browsing-context (F4)

| PR | Purpose | Main files / crates | Do-not-touch | Depends on | Plan-review | AC |
|---|---|---|---|---|---|---|
| **C0** | **Decide remove-vs-formal-slot** for `contentDocument`/`contentWindow`; if retained, register a defer slot (why/trigger/date) and replace the narrative comment with the slot id; write the eventual same-origin/cross-origin implementation plan | `vm/host/html_iframe_proto.rs` (comment/slot only, or removal); `docs/plans/` | proxy implementation | — | not required (slot decision); **PR-R** for the eventual impl | Either the stubs are removed, or a `#11-*` slot exists with all three elements and is cited in-code; implementation plan written; no behavior change beyond possible removal |
| C1+ | (Deferred — out of scope) same-origin/cross-origin proxy implementation | — | — | C0 + `world_id` program + S5/boa removal | **PR-R** | (future) |

### Program D — plugin-first tag dispatch (F5) — investigate-only

| PR | Purpose | Plan-review | AC |
|---|---|---|---|
| **D0** | Investigation note: can tag→prototype routing derive from plugin/registry metadata while keeping hot built-ins static? Fold conclusion into §H-7's framing (plugin-first, not just O(1)). No code. | no | A short finding appended to the roadmap/this plan: plugin-metadata feasibility + recommended shape; no implementation |

### Program E — shell style-compat mode policy (F6) — investigate-only

| PR | Purpose | Plan-review | AC |
|---|---|---|---|
| **E0** | Investigation note: enumerate every caller of the shell resolution pipeline; record that no app/browser mode mechanism exists (§2.7) and that compat is the sole production path; recommend deferring a policy parameter until a mode mechanism lands (shared with Program A's A0/A1). No code. | no | A short finding: caller list + the "defer until mode exists" recommendation; cross-reference to A0/A1 |

### 4.1 Dependency graph (high level)

```
A0 (PR0) ──► A1 ──► A2  (window.rs; after JS-side Slice 2b)
              └───► A3  (document.cookie; folds F2 clerical fix)
B0 ──► B1 ──► B2
C0  (independent; cheap)
D0  (independent; investigate)
E0  (depends conceptually on A0/A1's mode decision; investigate)
F2 clerical comment fix  (independent micro-PR; or folded into A3)
```

A0 (PR0) and B0 are both design/audit docs and can be authored concurrently. C0,
D0, E0, and the F2 clerical fix are independent and low-risk.

---

## 5. Recommended First PR

**PR0 = Program A's A0: "Web API core/compat split audit + supported-surface
inventory + boundary design" (docs/plans only).**

This matches the user's instinct, and the re-check confirms it is the safest and
highest-leverage first move:

- **Doc-only ⇒ zero collision** with the in-flight JS-side `window.rs` (Slice 2b)
  and HTML-side work. It can land immediately.
- **It unblocks the real IMP findings.** §2.7 showed there is *no* compat gate,
  *no* mode mechanism, and *no* async core equivalent (`elidex.storage`,
  CookieStore) yet — so F1/F2 cannot be "moved" until the boundary and mechanism
  are designed. Designing that is exactly A0.
- **Most of the inventory already exists** from the re-check (§2.1, §2.2, and the
  Window/Document/navigator surface enumeration in §A) and can be transcribed,
  saving effort.
- It is the natural artifact to run through `/elidex-plan-review` as the umbrella
  design gate before any code PR.

PR0 scope (no `.rs` changes):
1. Transcribe the full core-VM Window/Document/navigator API inventory (seed: §A),
   classify each Modern / Legacy / Deprecated against design §14.4 + §12.1.2.
2. Decide the gate/mode mechanism (the §2.7 gap): `WebApiSpecLevel`/`DomSpecLevel`
   runtime gate vs compile-time feature vs both; where it lives.
3. Decide compat placement (`elidex-api-storage-compat`/`-cookies-compat`) and the
   async core equivalents' ownership.
4. Emit the concrete A1/A2/A3 (and adjusted B/C/D/E) PR list with AC.

**Cheapest parallel "real" follow-ups (low collision, can run alongside PR0):**

- **F2 stale-comment correction** (`document.rs:1098`, `navigator.rs:72`) — a
  clerical doc fix that removes active reviewer-misleading misinformation; no
  plan-review; land when `document.rs` is not mid-edit by JS-side.
- **F4 C0 defer-slot decision** — cheap, doc/comment-only, owner-agnostic.

> Considered alternatives for "first PR" and why PR0 wins: starting with B0
> (F3 audit) is equally safe but does not unblock the two confirmed-IMP storage
> findings and depends on a subtler mechanism that this session's re-check only
> just corrected — better to let the corrected §2.3 settle and review B0 second.
> Starting with an actual F1/F2 move is rejected: it would touch `window.rs`
> (collision) and presupposes a compat mechanism that does not exist.

---

## 6. Plan-Review Questions (for `/elidex-plan-review`)

To run when each PR-R plan-memo is authored. Grouped by finding; F1–F3 emphasized
per the brief.

### 6.1 F1/F2 (Program A) — core/compat boundary

- **Axis 1b / whole-engine consistency:** Does the proposed boundary actually
  remove sync legacy APIs from the *core* surface, or just rename them? Where is
  the modern async equivalent (`elidex.storage`, CookieStore) and is the core
  truly usable without the compat shim?
- **Mechanism design (§2.7):** Is the gate a runtime `SpecLevel`, a compile-time
  feature, or both? Does it generalize to the *other* legacy APIs in the
  inventory (not just storage/cookie), or is it a one-off? (One issue, one way.)
- **No app/browser mode exists yet:** is the plan inventing a mode mechanism, and
  if so is that in-scope for this PR or a precondition PR? Avoid a half-built mode
  switch (strangler middle state).
- **Plugin-first:** should compat storage/cookie be a `DomApiHandler`/Web-API
  plugin behind a `SpecLevel`, per design §12.1.1 / §14.4, rather than a VM-local
  conditional?
- **Side-store→component:** does the compat storage backing introduce any
  per-entity `Send+Sync` side-store that should be an ECS component instead?
  (Storage/cookie are likely shared/session resources — exception (b) — but
  confirm.)
- **Spec citation (Axis 4):** WHATWG HTML **§12.2** (Web storage — *The API*;
  Storage interface §12.2.1, `sessionStorage` §12.2.2, `localStorage` §12.2.3,
  `StorageEvent` §12.2.4; opaque-origin `SecurityError` in the getter algorithms
  §12.2.2/§12.2.3), `document.cookie` = **§3.1.4** (Resource metadata management),
  the CookieStore spec, design §14.4.2/§14.4.3 — all cited in docstrings. (The old
  "§11.2" was a stale section number — A0 §1.5/§7 verified §12.2 via webref.)

### 6.2 F3 (Program B) — ScriptSession boundary + observer fan-out

This is the highest-risk plan-review. The §2.3 reframe must be re-verified, not
assumed:

- **Canonical-path decision:** Which is the single canonical
  mutation-notification mechanism — the `EcsDom` `ConsumerDispatcher` or the
  `SessionCore` mutation buffer + flush? CLAUDE.md says "ScriptSession is the sole
  Script↔ECS boundary," yet the VM currently never uses the session buffer and
  relies on `ConsumerDispatcher`. Does the plan move MutationObserver onto
  `ConsumerDispatcher`, or route VM writes through the session buffer? Is the
  *other* mechanism then removed (no coexistence)?
- **MutationObserver semantics:** Does the plan produce correct `MutationRecord`s
  for attributes (oldValue, attributeFilter), childList, and characterData per
  WHATWG DOM §4.3 — and avoid double-delivery with the existing innerHTML path
  (`dom_inner_html.rs:148/362`)?
- **Custom-element reactions:** `attributeChangedCallback` is currently driven by
  `ConsumerDispatcher`. If MutationObserver moves to the session buffer, are
  reactions still fired exactly once with correct ordering relative to
  microtasks? If the session buffer becomes canonical, does the CE reaction queue
  move with it?
- **Style invalidation:** `reconcile_attribute_derived_components` + inline-style
  derivation currently hang off `EcsDom::set_attribute`. Does the chosen path
  preserve them for every write site (including reflected setters and tree ops)?
- **Live queries / live collections:** `rev_version` bumps drive live
  `HTMLCollection`/`NodeList` re-evaluation. Does the plan keep version bumps on
  the canonical path for *all* mutation kinds?
- **Layering (Axis 1a):** Do reflected IDL setters and `removeAttribute` route
  through a `DomApiHandler` (engine-independent) rather than implementing the
  attribute-change algorithm in `vm/host/`? Is the bridge/direct asymmetry
  eliminated?
- **Re-check discipline:** does the plan-memo cite file:line evidence for the
  current mechanism (not the stale audit prose), per Axis 5 premise-correction?

### 6.3 F4 (Program C) — iframe

- **Defer-slot eligibility (4-question audit):** does the retained stub qualify
  for a slot, or should it be removed until the proxy model lands? If retained,
  are why/trigger/date all present, and is the trigger tied to the `world_id` /
  cross-DOM + S5/boa-removal precondition?
- **Spec correctness:** the null return matches cross-origin but is observably
  wrong for same-origin — does the plan acknowledge this and gate tests
  (same-origin / cross-origin / sandboxed / detached) for the eventual impl?

---

## 7. Risks / Coordination Notes

### 7.1 Collision risks with in-flight sessions

- **`window.rs` (HIGH): F1/A2 vs JS-side media Slice 2b.** Both edit
  `crates/script/elidex-js/src/vm/host/window.rs` (Slice 2b adds `matchMedia`; A2
  moves `localStorage`/`sessionStorage`). **Do not open A2 while Slice 2b is
  open.** Sequence: let Slice 2b land → rebase → then A2. PR0/B0/C0/D0/E0 and the
  F2 clerical fix do **not** touch `window.rs` and are unaffected.
- **`vm/host/` attribute files (MED): F3/B1-B2 vs JS-side.** B1/B2 touch
  `element_attrs.rs` and the reflected-IDL setter macros. The JS-side Slice 2b is
  on `window.rs`/`MediaQueryList`, not the attribute setters — low overlap today,
  but B-program is later and must re-check active branches at open-time
  (`git branch -r`, Axis 5).
- **`document.rs` (LOW): F2 clerical + A3 vs JS-side.** `document.rs` is not on the
  Slice 2b path; still confirm at open-time.
- **iframe (LOW): F4/C0 vs HTML-side.** The current HTML-side task
  (`#11-dialog-form-method`) and the queued focus A2b touch dialog/form and
  iframe *focus* cutover, not `contentDocument`/`contentWindow` accessors. C0 is
  comment/slot only. Confirm no overlap when A2b's scope is known.
- **Worktree isolation:** every code-touching PR in this program must be built in
  a dedicated worktree off `origin/main` (`git worktree add -b <branch> <dir>
  origin/main`), never in the shared main tree, per CLAUDE.md parallel-session
  rule. This plan PR itself is doc-only on `docs/plans/`.

### 7.2 Merge order

1. **PR0 (A0)** — umbrella design doc; run through `/elidex-plan-review`. Land
   first; it gates everything in Program A and informs E0.
2. **F2 clerical fix** and **C0 (F4 slot decision)** — independent, cheap; land
   any time (clerical fix prefers a window where `document.rs` is quiet).
3. **B0** — F3 audit doc; can be authored in parallel with PR0, land second.
4. **A1** → then **A2** (after Slice 2b) and **A3** in parallel.
5. **B1** → **B2**.
6. **D0**, **E0** — investigation notes, any time.

Each implementation PR (A1/A2/A3, B1/B2, C1+) is `/elidex-plan-review`-gated and
full-reviewed individually; never bundle two PR-R slices.

### 7.3 Stale-audit-memo guardrails (re-check before each PR)

The audit was a snapshot and **one of its central inferences (F3) was wrong** —
treat every finding's prose as a lead, not a spec. Before authoring each per-PR
plan-memo:

- **Re-grep the cited file:line.** Audit line numbers had already drifted ~6
  lines; assume more drift over time. Anchor on §2 of this doc, and re-verify §2
  against `main` at PR-open (this doc is also a snapshot — dated 2026-06-20).
- **Re-confirm the F3 mechanism (§2.3) by direct read** of `attribute.rs`
  (`set_attribute`/`dispatch_event`) and `vm_api.rs`/`mutation_observer.rs`
  (`deliver_mutation_records`) — do not carry the reframe forward on trust;
  Program B's correctness depends on it.
- **Re-check active branches** (`git branch -r`, open PRs) for convergence drift
  on the target files (Axis 5), especially `window.rs` / `vm/host/`.
- **Re-verify slot state:** `#11-storage-opaque-origin-securityerror` (F1) and the
  F5 tag slots / §H-7 must still be open and named as cited before referencing
  them; F4 currently has **no** slot (creating one is part of C0).
- **Watch for the precondition shifting:** if a mode/`SpecLevel` mechanism lands
  via some other program before A1, A1's scope collapses — re-baseline rather
  than re-implement.

---

## Appendix A — Re-check evidence index (seed for PR0 / B0)

Core-VM Web-API surface (HEAD `2f4a9d5a`), for transcription into the PR0
inventory. `(L)` = sync/legacy candidate.

- **Window** (`window.rs`, installed in `register_window_prototype` :411):
  methods `scrollTo`/`scroll`/`scrollBy`/`postMessage`/`getComputedStyle`/
  `getSelection` (:469-490); RO accessors `innerWidth`…`closed` (:501-517);
  RW `name` (:520); **storage `localStorage`/`sessionStorage` (:522-525) (L)**;
  event-handler IDL attrs (:445-451). `fetch` is a bare global
  (`fetch/mod.rs:91`), not a Window method. No `alert`/`confirm`/`prompt`, no
  `XMLHttpRequest`.
- **Document** (`document.rs`): methods `getElementById`…`getSelection`
  (:1017-1048) + traversal factories (:987); RO `documentElement`…`styleSheets`
  (:1050-1096); RW `title` (:1103) + **`cookie` (:1108) (L)**. No `document.write`/
  `writeln`/`open`/`close`/`all`/`execCommand`.
- **navigator** (`navigator.rs`, :32): UA/string fields (:32-68), bool fields incl
  `cookieEnabled=false`/`javaEnabled=false` (:76-89), `hardwareConcurrency` (:92),
  `languages` (:107), conditional `serviceWorker` (:119). No methods; no
  `clipboard`/`storage`/`permissions`/`mediaDevices`.

F3 write-site sweep (seed for B0) — direct `EcsDom` mutation sites in `vm/host/`:

- **Attribute API:** `element_attrs.rs:112` (`attr_set`), `:177` (`attr_remove`),
  `:414` (`setAttributeNode`), `:535` (`removeAttributeNode`); `attr_proto.rs:416`;
  `named_node_map.rs:345`/`:431`. (`element_attrs.rs:218` `setAttribute` and `:202`
  `getAttribute` go through `invoke_dom_api`.)
- **Reflected IDL setters (direct `set_attribute`):** `html_input_proto.rs:460`/
  `:544`/`:687`/`:853`; `html_input_value.rs:129`/`:182`/`:253`/`:501`/`:535`;
  `html_button_proto.rs:183`/`:246`/`:283`/`:324`/`:358`; `html_select_proto.rs:254`/
  `:299`/`:364`; `html_textarea_proto.rs:306`/`:373`/`:509`; `html_form_proto.rs:246`/
  `:296`/`:334`/`:371`/`:403`; `html_element_proto.rs:430`/`:714`/`:724`/`:750`/`:791`/
  `:835`/`:871`/`:904`; `html_iframe_proto.rs:241`/`:292`; `html_option_proto.rs:178`/
  `:255`/`:293`; `html_optgroup_proto.rs:106`/`:142`; `html_label_proto.rs:143`;
  `html_fieldset_proto.rs:135`/`:171`; `canvas/mod.rs:780`; `form_state_sync.rs:82`/`:111`.
- **Tree mutations via `invoke_dom_api` (bridge):** `node_proto.rs:709`
  (`appendChild`); `removeChild`/`insertBefore`/`replaceChild` registered handlers.
- **Tree mutations direct (NOT bridge):** `parentnode.rs:126/127/172/225/236/249`;
  `childnode.rs:199/355/356/429/430/491/518/522/523/551`;
  `element_insert_adjacent.rs:176/187/190/198/213/218`;
  `html_select_proto.rs:790/797/821/849/901/907`; `dom_bridge.rs:136`.
- **Notification mechanisms:** `EcsDom::set_attribute` (`attribute.rs:101`) /
  `remove_attribute` (`:258`) → `reconcile_attribute_derived_components` +
  `rev_version` + `dispatch_event` → `ConsumerDispatcher`. JS `MutationObserver`
  fed only by `Vm::deliver_mutation_records` (`vm_api.rs:867`), production callers
  only `dom_inner_html.rs:148`/`:362`.

> All Appendix A line numbers are a 2026-06-20 snapshot — re-grep at PR-open.

# S5 — THE FLIP + boa deletion (multi-PR umbrella plan-memo)

> **⚠ SUPERSEDED re world_id (2026-06-30):** this memo's "world_id strictly AFTER S5" framing (§0, §9
> keystone row, Q4) is **superseded** by `docs/plans/2026-06-agent-scoped-ecsdom-world.md` — there is no
> world_id program; `EcsDom` World = one per similar-origin window agent (B1), which lands with the
> friendly-iframe layer (post-S5) and eliminates cross-DOM aliasing by construction. The
> nav-scrub-as-S5-6-hard-gate broadening is **retracted** (the flip is cross-DOM-neutral; see that doc
> §6.2). The full keystone-row rewrite is deferred to the B1 implementation PR (trigger). See that doc §6
> for the reconciliation.

Anchor = **the ideal end-state**, not an incremental patch (`feedback_plan-memo-anchor-on-ideal-not-incremental`).
Parent program = `memory/boa-vm-cutover-plan.md` (S0→S5 umbrella). This memo **explodes the parent's
§5 last bullet ("S5 — THE FLIP + boa deletion")** into a dedicated multi-PR program, per CLAUDE.md
"Edge-dense work = multi-PR program + 実装前 plan-review 必須". All state below **re-verified against HEAD
`de47636e` (2026-06-27)** — the parent + S1/S2 plan file:line cites are 13–16 days stale and several were
falsified by this re-grep (recorded in §2).

> **This is a PLANNING artifact.** No behavior change. The gate for this umbrella is `/elidex-plan-review`
> (5-agent design review) **before** the first S5 PR implements. Per-PR fine-grained spec-step enumeration
> belongs in each PR's own plan-memo at impl time — this umbrella is at **program / surface granularity**.

---

## §1 Goal + ideal end-state

**Goal**: remove the boa/VM coexistence — the single largest JS-side decision tax ("boa or VM? is this a
migration surface?" on every JS PR), the largest edge-dense program in elidex (~−15k LoC, "larger than S2").

**Ideal end-state** (inherited from parent §1, unchanged):
- **One JS engine** = the bytecode VM (`elidex-js` with `features=["engine"]`). The shell drives it through
  the `HostDriver` + `ScriptEngine` traits via a single concrete `ElidexJsEngine`.
- **No `dyn ScriptEngine` strangler**, no runtime-selectable dual engine, no boa kept "just in case". The
  flip is a **hard swap + crate deletion** in one program (CLAUDE.md "no strangler middle state",
  One-issue-one-way). `elidex-js-boa` is a **wholesale crate removal** (`project_boa_runtime_deletion.md`).
- **Layering preserved**: every web-API *algorithm* surfaced by the flip (DOMParser fragment parse,
  XMLSerializer serialize, media-query eval, focusable-area) lives engine-indep; VM `host/` only marshals.
- **ECS-native preserved**: per-entity DOM facts (focus) are the canonical `ElementState::FOCUS` component
  (already converged, §2); per-VM browsing-context facts (origin / sandbox / nav buffers) are HostData
  shared-cross-cutting side-stores (the CLAUDE.md (b) exception), **not** components — and **not** migrated
  to components in this program (that is the world_id program, strictly AFTER S5; §0, §9).
- **VM strictly ≥ boa at flip time** — measured against `boa-vm-cutover-surface-parity-audit.md`'s
  regression set, NOT the stale roadmap §D-tier3 list (premise-corrected 4/4 in that audit).

---

## §0 Non-negotiable constraints (read first — these bound every PR below)

1. **world_id is strictly AFTER S5** (user-confirmed 2026-06-14, `project_world-id-cross-dom-migration.md`).
   The `HostDriver`/`ScriptEngine` signatures are **world_id-agnostic** (unchanged before/after world_id) →
   S5 lands without it. **Do NOT pull per-VM-side-store→per-entity-component migration into any S5 PR**
   (`document_origin` / sandbox / nav stay interim per-VM HostData; double-refactor ban). The world_id
   program is the *cleanest* right after boa is deleted (single-engine HostData, no dual-maintenance).
2. **boa-deletion is wholesale crate removal** — no pre-cleanup, no mirroring new work to boa
   (`project_boa_runtime_deletion.md`, `feedback_boa-findings-light-touch`). boa fixes only to keep CI green.
3. **`#11-async-core-storage-cookiestore` is a SECOND keystone, parallel to S5, NOT a sub-PR of S5.** It is
   the precondition that makes **non-compat `EngineMode` modes production-selectable**. S5 flips the shell to
   the VM in `BrowserCompat` mode (the only mode that ships sync `localStorage`/`document.cookie`); the
   async-core storage contract is what later flips a `BrowserCore`/`App` mode on. S5 does **not** block on it,
   and it does **not** block on S5 — they co-gate the mode-plumbing cohort (§6). Keep them separate programs.
4. **Edge-dense discipline**: this umbrella + per-PR plan-review is the *mechanism* that satisfies the
   mandatory rule. **Base case** (CLAUDE.md): a narrowly-scoped per-PR slice under this approved umbrella that
   has passed its own `/elidex-plan-review` is a terminal unit = an allowed single PR (a per-PR slice touching
   the same subsystem is **not** a re-split trigger — else infinite regress).

---

## §2 Current-state reconciliation (HEAD `de47636e`, re-verified)

The parent plan's "S1–S4 land while boa stays live; S5 = the flip" sequencing is **largely executed**. The
re-grep falsifies the parent's picture of "S5 = one big remaining slice": **most VM capability already
landed**; what remains is a **small VM-capability tail (S3/S4) + the flip itself + a security/navigation
enforcement-edge cohort**.

### Landed (VM capability ≥ boa for these surfaces — verified)
- **S0 media-query (the deep prerequisite) — DONE.** `elidex-css::media::{parse,eval,serialize,types}` is the
  canonical engine-indep evaluator (`evaluate(list, env)` at `media/eval.rs:68`); `@media` wired into the
  cascade (#378); VM `window.matchMedia` + `MediaQueryList` (#370); `set_media_environment` +
  `deliver_media_query_changes` back-channel on `HostDriver` (`engine.rs:364/379`). Slices #360/#364/#370/
  #372/#378.
- **S1a/b/c/d — DONE.** The parent's "S5 wires the shell bracket" assumption is **superseded**: the
  `HostDriver` trait (`elidex-script-session/src/engine.rs:123`) now carries the **full** shell-facing surface
  — `bind`/`unbind`/`with_bound` (batch-bind RAII, §4), `set_origin`/`origin`/`set_sandbox_flags`/
  `sandbox_flags`/`forms_allowed`/`popups_allowed`/`iframe_depth` (S1b security substrate, now real in
  `host_data.rs:184/215/233`), `set_current_url`/`take_pending_navigation`/`take_pending_history`/
  `set_session_history`/`history_length`/`set_navigation_referrer` (S1c navigation back-channel),
  `set_visibility`/`take_pending_scroll`/`set_scroll_offset` (S2 transport), the deliver_* drains, the
  install_* (network/idb/cookie). `ElidexJsEngine` impls `ScriptEngine` + `HostDriver`; `drain_reactions`/
  `drain_timers` are **implemented, not stubs** (`elidex-js/src/engine.rs:243/261`).
- **S2-focus convergence — DONE.** Focus is the canonical `ElementState::FOCUS` ECS component read via
  `elidex_dom_api::focus::current_focus` (with connectedness filter); VM `focused_entity` side-store **gone**
  (grep = 0 in `host_data.rs`); `activeElement`/`hasFocus` read the component (`host/document.rs:792/844`);
  `tab_index_default_for`/focusable-area moved to engine-indep `elidex-dom-api/src/focus/`; shell `focus.rs`
  routes through the reconciler reading the canonical bit. (The parent §2.3 "≥5 stores to converge" work is
  complete.)

### Remaining VM-capability gaps (vs boa — the only true "VM < boa" deltas left)
- **S3 — DOMParser / XMLSerializer ABSENT in VM** (grep: 0 files in `elidex-js`; boa has
  `globals/window/dom_parser.rs`). This is the **only HIGH/MED-stakes regression** left (common in libs).
- **S4 — VisualViewport / cookieStore ABSENT, Screen minimal** (grep: `VisualViewport`/`cookieStore` = 0 in
  `elidex-js`). LOW-stakes; judge real UA impact at review.

### The flip is NOT yet done
Shell still runs boa: `pipeline.rs:10` `use elidex_js_boa::JsRuntime`, `lib.rs:39/433` (`PipelineResult.runtime:
JsRuntime`), the CSSOM shadow-sync (`lib.rs:84/205/245` `CssomSheet`/`CssomMutation`/`sync_stylesheets_to_bridge`)
still feeds boa. `Cargo.toml:31` deps `elidex-js-boa`. The `engine` feature on `elidex-js` is enabled by
nothing but benches (`Cargo.toml:21` "shell still runs the boa engine (S5 cutover pending)").

**Net**: S5 is now **(a) close the last VM-capability gaps (S3/S4) + (b) flip + delete + (c) land the
security/navigation enforcement-edge cohort that only matters once the VM drives real shell traffic.**

---

## §3. Spec coverage map

The web-API surfaces this umbrella's PRs **implement / flip onto the VM**, at **program / surface
granularity** (one row per surface; per-step / per-branch enumeration belongs in each S5-N PR's OWN plan-memo
at impl time — `feedback_plan-scope-re-evaluation`). Citations webref-verified 2026-06-27 via
`.claude/tools/webref heading --exact` (the HTML rows re-checked against the preflight's own
`heading --exact` call; the CSSOM-View / Media Queries / Web Animations rows verified against their
level-suffixed shortnames `cssom-view-1` / `mediaqueries-5` / `web-animations-1`). "Touch" maps the surface to
its owning S5-N PR per §5 (PR decomposition) + §7 (edge matrix). "Full enum?" = does the row land the full
spec surface (✓) or a boa-parity-bounded / subset-first slice (✗, the tail goes to the PR's own defer ledger).

| Spec section | Step | Branch | Touch (どの S5-N PR / site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.4 Parsing HTML fragments | `DOMParser.parseFromString` (§8.5.1 The DOMParser interface) | all MIME → HTML-parsed (= boa, real XML = future feat, no regression) | **S5-1** — host/ marshal → existing `innerHTML` fragment seam | ✗ (boa-parity-bounded; real XML = future) | yes (markup string) |
| WHATWG HTML §13.3 Serializing HTML fragments | `XMLSerializer.serializeToString` (§8.5.8 The XMLSerializer interface) | element / text node | **S5-1** — reuse `elidex-dom-api::serialize_{outer,inner}_html` | ✓ | yes (DOM tree) |
| CSSOM-View §12.1 The VisualViewport Interface | `Window.visualViewport` / `VisualViewport` | offsetLeft/Top, width/height, scale, `resize`/`scroll` | **S5-2** — VM host/ additive surface | ✗ (presence-first; judge real UA impact at review) | no (shell-driven) |
| CSSOM-View §4.3 The Screen Interface | `Window.screen` / `Screen` completion | width / height / colorDepth / pixelDepth | **S5-2** — VM host/ additive surface | ✗ (presence-first) | no (shell-driven) |
| Cookie Store API §3 The CookieStore interface | `cookieStore` (webref shortname `cookiestore`; §2 = Concepts, §3 = The CookieStore interface — verified 2026-06-27) | get / set / delete / `change` event | **S5-2** — VM host/ additive surface (cookie-jar-backed) | ✗ (presence-first; Cookie Store API subset) | yes (cookie name/value) |
| CSSOM-View §4 Extensions to the Window Interface | `matchMedia(query)` (§4.2 The MediaQueryList Interface) | returns MediaQueryList + `change` event | **S5-3** — host/ marshal → canonical `elidex-css::media::evaluate`; keepalive-rooting the MQL EventTarget | ✗ (change-delivery edge; grammar = next row) | yes (query string) |
| Media Queries 5 §2 Media Queries | evaluate `<media-query-list>` | width/height/orientation/resolution/prefers-color-scheme/-reduced-motion | **S5-3** — canonical engine-indep `elidex-css::media` (already wired into cascade) | ✗ (subset-first; grammar grows) | yes (query string + `@media`) |
| WHATWG HTML §7.1.5 Sandboxing | sandboxing flag set (scripts/forms/popups/modals + `sandboxed origin browsing context flag` opaque-origin) | flag-gated method calls + sandboxed-fetch opaque origin | **S5-4** — sandbox-method gates + origin isolation at the live-engine seam | ✗ (the cluster's gated subset) | yes (sandboxed page script) |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | scripting-is-disabled event-handler attr processing | event-handler content-attr (§8.1.8.1) compile gated when scripting disabled | **S5-4** — scripting-disabled gate step 1 | ✓ | yes (event-handler attr) |
| WHATWG HTML §7.4.2.2 Beginning navigation | `navigate` + synchronous fragment navigation (§7.4.2.3.3 Fragment navigations) | session-history navigation + same-document fragment | **S5-5** — nav-origin-resync + sync-fragment at the live-engine seam | ✗ (the nav enforcement-edge subset) | yes (URL / location set) |
| WHATWG HTML §7.4.3 Reloading and traversing | session history traversal + `popstate` (§7.2.7.2 The PopStateEvent interface) | `history.go`/`back`/`forward` traversal; popstate dispatch | **S5-5** — drain-history-before-navigation + traversal/popstate fidelity | ✗ (traversal enforcement-edge subset) | yes (history state object) |
| WHATWG HTML §8.1.4.4 Calling scripts | clean up after running script (microtask checkpoint) | batch-bind bracket: bind → eval → clean-up → unbind | **S5-6** — the FLIP batch-bind bracket (§4) | ✓ | yes (script source) |
| WHATWG HTML §12.2.4 The StorageEvent interface | `storage` event broadcast / VM emit-site | cross-context dispatch | **S5-6** — VM emit-site at cutover (`#11-storage-event-broker`) | ✗ (emit-site only; mode-aware delivery = §6 follow-up) | yes (storage writes) |
| WHATWG HTML §6.6.6 Focus management APIs | `focus()`/`blur()` + synthetic focus/blur/change events (A2c) | focusing-steps (§6.6.4) + synthetic dom-event dispatch | **S5-6** — synthetic focus/blur/change at the host seam, now driven live | ✓ (reads canonical `ElementState::FOCUS`) | no (DOM/shell-driven) |
| Web Animations §6.8 The Animatable interface mixin | `Element.animate(keyframes, options)` | WAAPI animation creation | **S5-7** — post-flip fidelity surface | ✗ (WAAPI subset; timeline tail = own ledger) | yes (keyframe / options object) |
| WHATWG HTML §7.2.2.1 Opening and closing windows | `window.open` (window open steps) | auxiliary browsing-context creation | **S5-8** — browsing-context model (world_id-bound, §10 Q4) | ✗ (world_id-gated; stub-only pre-world_id) | yes (URL / features string) |
| WHATWG HTML §9.3 Cross-document messaging | `postMessage` (cross-document messaging) | cross-window message dispatch + origin check | **S5-8** — browsing-context model (world_id-bound, §10 Q4) | ✗ (world_id-gated) | yes (message data + targetOrigin) |

**NOT in the table (no implementation gap → not cutover work, verified §2)**: HTML §8 scripting / event-loop
(eval / microtasks / timers / observer·worker·SW·network drains — VM has all, **wiring only** via S1, landed) ;
CSSOM §6 `CSSStyleSheet`/`insertRule`/`document.styleSheets` (**VM already ideal**, DOM-as-truth) ;
`document.cookie` (**VM already has** `HostData::install_cookie_jar`) ; page visibility / scroll read-back
(S2 transport, landed).

**Breadth verdict — SPLIT (already satisfied by this umbrella's multi-PR decomposition).** K = 5 distinct
specs (WHATWG HTML, CSSOM-View, Media Queries 5, Web Animations, Cookie Store API) over M = 16 surface rows →
well past the preflight's `K≥4` SPLIT-RECOMMENDED line and brushing the `K≥6` SPLIT-DEFAULT band. **Either
way the verdict is SPLIT — and SPLIT is already achieved**: this is an *umbrella*
plan that **explodes the work into the S5-1..S5-8 multi-PR decomposition** (§5) with a per-PR-plan-review
mandate (§0.4, §5 "plan-review?" column). **No single-PR justification is owed** — the umbrella IS the split.
The breadth here is the *program* surface; each S5-N PR carries a *narrow* per-PR §3 (a handful of rows from
this table) into its own plan-review, where it lands well inside the single-PR band.

### §3.1 User-input touch audit

The flip processes the **same untrusted page inputs boa already does** — **no NEW trust boundary** is opened by
moving the engine; the VM host bindings already validate at marshal (the crypto/JWK work hardened it). The
untrusted-input sites that flow through the surfaces above (`feedback_trust-boundary-enumerate-upfront`):

- **script source** (HTML §8.1.4.4 Calling scripts, S5-6) — the page's `<script>` text + event-handler attrs;
  bracketed by the batch-bind window and gated by the sandbox `scripts_allowed` flag (HTML §7.1.5) and the
  scripting-disabled gate (HTML §8.1.3.4, S5-4). The load-bearing edge = the bound window must stay
  valid + unaliased for the whole batch (§4).
- **DOMParser markup string** (HTML §13.4, S5-1) — routes to the same `innerHTML` fragment seam boa uses, so
  it **inherits that seam's sanitization** (no new parse path).
- **media-query string** (CSSOM-View §4 `matchMedia` arg + `@media`, Media Queries 5 §2, S5-3) — parsed by the
  canonical `elidex-css::media` evaluator; invalid grammar shapes are enumerated up front by that crate (a
  pure `&str → bool` seam, already the cascade's parser — not a new untrusted surface at the flip).
- **cookie / storage writes** (Cookie Store API + HTML §12.2.4 StorageEvent, S5-2 / S5-6) — cookie name/value
  via the existing `CookieJar`; storage writes broadcast a `StorageEvent` whose data crosses contexts (the
  emit-site, not a new validation boundary — the values were already untrusted page strings).
- **postMessage data + targetOrigin** (HTML §9.3, S5-8) — cross-window message payload + origin check; the
  origin comparison is the trust gate, **world_id-bound** (§10 Q4) so it does not land pre-world_id.

No surface introduces a new sanitization obligation the boa path lacked; the audit's role here is to confirm
the flip is **trust-boundary-neutral** and to point each per-PR plan-review at its untrusted-input site.

---

## §4 The batch-bind safety corner (S5 owns the shell-side wiring)

This is the corner the parent §2.1 / S1 plan §2 flagged as the deepest, and it is **what the flip PR
actually wires**. VM `bind`/`unbind` (`vm_api.rs`) are **heavy browsing-context-cycle** operations
(`unbind` clears non-Node wrappers, live collections, rolls back IDB txns, tears down dispatcher/workers) —
**NOT** boa's cheap Rc-pointer swaps. So the model is **BATCH-BIND**: the shell brackets each engine-driving
batch (the `<script>` eval loop, each UA event dispatch, each frame drain) with **ONE** `bind`/`unbind`; the
trait methods **assume bound**. S1a already exposed the VM-side primitive — `ElidexJsEngine::bind`/`unbind`
(with a `"batch brackets must not nest"` assertion, `engine.rs:296`) + `with_bound` RAII (`engine.rs:327`).

**What S5 owns**: replacing boa's per-call self-bind driving in the shell with the **single outer batch
bracket** around `run_scripts_and_finalize`, each UA event dispatch path, and each content-thread frame drain.
This is the load-bearing edge of the flip — the bound window must stay valid + unaliased for the **whole**
batch (bind is non-re-entrant), and the post-script microtask checkpoint is WHATWG HTML §8.1.4.4
`#clean-up-after-running-script` (verified via webref), self-contained in `VmInner::eval`.

**Coupled invariants** — each pair's intersection named (`feedback_coupled-invariant-design-corner`):
- **bind-lifetime × assume-bound contract**: the trait methods (`eval`/`call_listener`/`drain_*`) read host
  pointers *without* re-binding, so those pointers must stay unaliased for the **whole** outer batch — a method
  that self-bound mid-batch (boa's model) would tear down cross-`<script>` wrapper/JS-state identity through
  the heavy `unbind`.
- **assume-bound × sandbox `scripts_allowed`**: the sandbox short-circuit (§7.1.5
  `#sandboxed-scripts-browsing-context-flag`, verified) reads per-VM sandbox flags off **bound** HostData, so
  it must run *inside* the bound window, not before bracketing (gating pre-bind would read absent/stale flags).
- **bind-lifetime × `scripts_allowed`**: a *disallowed* script must not leave a half-opened bracket — the batch
  bracket opens and closes regardless of whether any script ran, so the `scripts_allowed = false` path still
  pairs its `bind` with an `unbind` (the non-re-entrant `bind` must always see a clean prior close).

This corner is why the **FLIP PR (S5-6) is
plan-reviewed**, and why `#11-bound-safe-dispatch-dom-aliasing` (the "drive event dispatch under a bound
window" slot, `engine.rs:303`) is bundled into it.

---

## §5 PR decomposition + S5-cohort slot assignment

The ~14 S5-cohort slots (`project_open-defer-slots.md` "S5-boa-deletion cohort") are assigned to PRs by the
(a)/(b)/(c) rule: **(a)** FLIP-precondition (VM lacks an enforcement substrate the live shell will exercise) →
land BEFORE the flip; **(b)** bundle INTO the flip PR (only meaningful once the VM drives traffic); **(c)**
FLIP follow-up (post-flip fidelity, independently shippable). Plus the two remaining capability gaps (S3/S4).

| PR | Scope | Slots folded | Edge axes touched | plan-review? | Depends on |
|---|---|---|---|---|---|
| **S5-1 DOMParser+XMLSerializer** | VM DOMParser/XMLSerializer (last HIGH/MED capability gap). Marshal to existing `innerHTML` fragment seam (parse) + reuse `elidex-dom-api::serialize_{outer,inner}_html` (serialize). boa parity exactly (all MIME→HTML-parsed; real XML = future feat, no regression). | — | — (pure additive surface) | **no** (narrow additive, boa-parity-bounded; base case) | — (boa live) |
| **S5-2 minor window parity** | VisualViewport + cookieStore + Screen completion. Judge each on real UA impact at review (not auto-defer). | — | — | **no** (narrow additive; base case) | — (boa live) |
| **S5-3 EventTarget listener-keepalive rooting** | `#11-eventtarget-listener-keepalive-rooting` — a VM `EventTarget` alive ONLY by a listener (`matchMedia(q).addEventListener`) is GC-collected → §4.2 change delivery lost. **S5-flip-precondition** (inert while VM media path dormant, breaks the headline `deliver_media_query_changes` once the VM drives the shell). Ideal = GENERIC "EventTarget alive while listenered" unifying AbortSignal/observers/MQL. | `#11-eventtarget-listener-keepalive-rooting` | GC-rooting × listener lifecycle × MQL/observer/AbortSignal unification | **yes** (edge-dense: ≥3 unification axes) | — (VM-internal; boa live) |
| **S5-4 sandbox/security enforcement edge** | The sandbox-method gates + origin-isolation edges that only bite once the VM is the live engine: alert/confirm/prompt + window.open gating, sandboxed-fetch opaque-origin, iframe-origin-before-initial-scripts, scripting-disabled event-handler attr processing. **Layering**: the sandbox / scripting-disabled gate *predicate* (HTML §7.1.5 / §8.1.3.4) lands engine-indep — read the already-engine-side sandbox flags; precise crate (`elidex-dom-api` / `elidex-script-session`) decided at S5-4's own plan-review, **not** a fresh `host/` body. | `#11-vm-sandbox-method-gates-and-modals`, `#11-sandbox-fetch-opaque-origin-isolation`, `#11-iframe-origin-before-initial-scripts`, `#11-scripting-disabled-eventhandler-processing-step1`, `#11-worker-port-message-no-origin` | sandbox flags × origin × scripting-disabled × fetch isolation | **yes** (edge-dense security cluster: ≥3 invariant axes; sandbox bypass = security-by-structure) | S1b substrate (landed) |
| **S5-5 navigation/history enforcement edge** | The navigation-origin + history-traversal edges the live shell drives: nav-origin-resync, drain-history-before-navigation, synchronous fragment navigation, popstate/traversal fidelity. | `#11-vm-navigation-origin-resync`, `#11-s5-shell-drain-history-before-navigation`, `#11-synchronous-fragment-navigation`, `#11-history-state-traversal-popstate-fidelity` | origin × navigation × history-traversal × focus-reset | **yes** (edge-dense: nav × history × origin) | S1c back-channel (landed) |
| **S5-6 THE FLIP + boa deletion** | shell deps `elidex-js features=["engine"]`, drop `elidex-js-boa` dep; `PipelineResult.runtime: JsRuntime` → `ElidexJsEngine`; wire the **batch-bind brackets** (§4) around the eval loop / event dispatch / frame drains; delete the CSSOM shadow-sync (`stylesheets_to_cssom`/`sync_stylesheets_to_bridge`/`CssomMutation` — VM reads CSSOM live from EcsDom); **delete the `elidex-js-boa` crate**. Bundles the bound-window-dispatch slot + the storage-event VM emit-site. | `#11-bound-safe-dispatch-dom-aliasing`, `#11-storage-event-broker` (VM emit-site), `#11-vm-host-synthetic-dom-event-dispatch` (focus A2c synthetic blur/focus/change at the host seam, now driven live) | batch-bind safety × CSSOM-truth swap × storage-event emit × synthetic-event dispatch × focus A2c × the whole shell test oracle | **yes** (the highest-blast PR; the batch-bind corner + wholesale deletion) | **S5-1..S5-5 + C3** (§9) |
| **S5-7 element.animate (Web Animations)** | `#11-web-animations-element-animate` — `Element.animate()`/WAAPI. Post-flip fidelity surface; independently shippable once the VM is live. | `#11-web-animations-element-animate` | animation timeline × WAAPI (largely self-contained) | **judge at scope** (own plan-review if it touches the animation/timeline cross-cut) | S5-6 (flip done) |
| **S5-8 window.open / postMessage browsing-context model** | `#11-browsing-context-model-window-open-postmessage` — the broader auxiliary-browsing-context / cross-window postMessage model. **Strongly couples world_id / multi-doc** (cross-VM Window proxy identity) → likely lands WITH the world_id program, not pure S5. Registered here for cohort completeness; disposition = §10 Q4. | `#11-browsing-context-model-window-open-postmessage` (+ related off-cap `#11-windowproxy-browsing-context`/`#11-auxiliary-browsing-context-opener` from #412 C0, which are world_id-bound) | browsing-context entity × WindowProxy identity × cross-VM postMessage × opener | **yes** (edge-dense + world_id boundary) | **world_id program** (post-S5) — see §10 Q4 |

**Slot-to-PR coverage check.** The 14-slot **boa-deletion cohort** (`project_open-defer-slots.md`
"S5-boa-deletion cohort") maps as: S5-4 {sandbox-method-gates, sandbox-fetch-opaque, iframe-origin-before-scripts,
scripting-disabled-eventhandler, worker-port-no-origin} (5); S5-5 {nav-origin-resync, drain-history-before-nav,
synchronous-fragment-nav, history-traversal-popstate} (4); S5-6 {bound-safe-dispatch, storage-event-broker,
vm-host-synthetic-dom-event-dispatch} (3); S5-7 {web-animations} (1); S5-8 {browsing-context-window-open-postmessage}
(1) = **14 cohort slots ✓**. **S5-3 {keepalive} is +1 cross-listed S5-*prerequisite***
(`#11-eventtarget-listener-keepalive-rooting` lives in the observer KEEP-section tagged "S5-prerequisite" —
**NOT** a boa-deletion-cohort member). **Total PR-folded = 15** (14 cohort + 1 prerequisite).

---

## §6 mode-plumbing cohort + the async-core-storage 2nd keystone

These slots are gated by **S5 (the VM becoming the live engine, so an `EngineMode` actually flows shell→content)
AND/OR `#11-async-core-storage-cookiestore` (a non-compat mode becoming production-selectable)**. They are
**NOT S5 PRs** — they activate once both keystones are met, and are tracked here only so S5's sequencing
exposes the co-gate.

| Slot | Gated by | Why dormant until both | Lands as |
|---|---|---|---|
| `#11-async-core-storage-cookiestore` | **2nd keystone itself** (Program-A precondition) | `Vm::new_with_mode` is `#[cfg(test)]`; SW thread hard-derives `BrowserCompat`; until this lands, non-compat modes are not production-selectable | its own program (parallel to S5, co-gates the cohort) |
| `#11-storage-event-mode-aware-delivery` | S5 (`EngineMode` shell→content) **OR** async-core (core mode selectable) | shell broadcasts `StorageEvent` with no `EngineMode` metadata; no live consumer until a non-compat mode exists | mode-plumbing follow-up (post-both) |
| `#11-enginemode-full-session-threading` | S5 (`EngineMode` shell→content) **OR** async-core | E0 wired mode→style-compat at the resolution chokepoint only; iframe/nav/CSS-parse paths still hard-code `BrowserCompat` | mode-plumbing follow-up (post-both) |
| `#11-form-control-ua-rendering-fidelity` | a `BrowserCore` mode becoming selectable (async-core) | pre-existing UA-sheet bugs now also affect core arm; orthogonal to the flip | spec-fidelity follow-up (post-async-core) |
| `#11-navigator-spec-faithful-surface` (残 UA fields) | shell exposes a UA/compat source (E0/F6 family) | needs a real UA source, not fabricated interim | demand-gated follow-up |

**Sequencing of the keystones**: S5-6 (the flip) makes the shell pass `BrowserCompat` to a single live VM →
an `EngineMode` plumbing surface *exists* but has one value. `#11-async-core-storage-cookiestore` is the
**independent** program that makes a second value (`BrowserCore`/`App`) reachable. Only when **both** land
does the mode-plumbing cohort have a live consumer. **Recommendation**: keep the keystones decoupled; do not
gate S5 on async-core (S5 ships compat-only), do not gate async-core on S5 (it can land while boa is live —
it is about the storage contract, not the engine). The cohort is the *intersection* follow-up.

---

## §7 Edge matrix (the review-tail pre-empt)

Cohort-crossing invariant axes. Each cell = which PR touches that axis. This is the upfront map the
edge-dense discipline requires so per-PR plan-reviews can pre-empt the cross-PR review tail.

| Invariant axis | S5-3 keepalive | S5-4 sandbox | S5-5 nav/history | S5-6 FLIP | S5-7 animate | S5-8 win.open |
|---|---|---|---|---|---|---|
| **origin** (document_origin SoT, interim per-VM) | — | ✔ (opaque/sandbox fetch + iframe-origin-before-scripts) | ✔ (nav-origin-resync) | reads (batch context) | — | ✔ (cross-window origin checks) |
| **sandbox flags** (scripts/forms/popups/modals) | — | ✔ (the cluster) | — | gate read (eval bracket) | — | ✔ (popup/top-nav) |
| **navigation** (NavigationRequest back-channel) | — | — | ✔ (resync + sync-fragment) | drains (frame batch) | — | ✔ (aux context nav) |
| **history** (HistoryAction / traversal / popstate) | — | — | ✔ (drain-before-nav, traversal) | drains | — | — |
| **focus** (`ElementState::FOCUS`, A2c synthetic) | — | — | ✔ (nav focus-reset) | ✔ (synthetic dom-event dispatch, A2c) | — | ✔ (cross-frame focus, world_id) |
| **storage-event** (broadcast/emit) | — | — | — | ✔ (VM emit-site at cutover) | — | ✔ (cross-window) |
| **batch-bind safety** (bound window, non-re-entrant) | — | — | — | ✔ (the load-bearing wiring, §4) | — | — |
| **CSSOM source-of-truth** (EcsDom live vs boa shadow) | — | — | — | ✔ (delete shadow-sync) | reads | — |
| **GC rooting** (EventTarget/listener lifecycle) | ✔ (the slot) | — | — | reads (deliver_* live) | ✔ (animation keep-alive?) | — |
| **EngineMode** (shell→content, mode-plumbing §6) | — | — | — | ✔ (single `BrowserCompat` value flows) | — | — |
| **world_id / cross-VM identity** (post-S5, §0) | — | — | — | — | — | ✔ (S5-8 likely world_id-bound) |

**Densest intersections** (where review tail concentrates → plan-review them hardest): **S5-6 (FLIP)** crosses
batch-bind × CSSOM-truth × storage-emit × synthetic-event × focus-A2c × the whole shell oracle; **S5-5** crosses
origin × navigation × history × focus-reset; **S5-4** crosses sandbox × origin × scripting-disabled × fetch.
**S5-8** crosses the world_id boundary → likely defers into that program (§10 Q4).

---

## §8 Dependency DAG

```
                      (parallel, independent program — NOT a S5 PR)
                       #11-async-core-storage-cookiestore  ──────┐
                                                                 │ (2nd keystone)
   [landed]  S0 media · S1a/b/c/d · S2-focus/transport           │
                                                                 ▼
   C3 (shell-viewport device facts) ──[direct pre-flip gate]──┐  (both keystones met →
                                                              │   mode-plumbing cohort
   S5-1 DOMParser/XMLSerializer ──┐                           │   §6 follow-ups活性化)
   S5-2 minor window parity ──────┤  (VM-capability tail;     │
   S5-3 EventTarget keepalive ────┤   boa stays live;         │
   S5-4 sandbox enforcement ──────┤   independently shippable)│
   S5-5 nav/history enforcement ──┘                           │
                       │                                      │
                       └──────────────► S5-6 THE FLIP + boa deletion ◄── C3
                                              │
                                              ├──► S5-7 element.animate (post-flip fidelity)
                                              │
                                              └──► (world_id program) ──► S5-8 win.open/postMessage
```

- **S5-1..S5-5 are mutually independent** (all VM-capability-only, boa stays live) → parallelizable, any order.
- **S5-6 (FLIP) gates on S5-1..S5-5 done + C3 done** (C3 = the direct pre-flip viewport device-facts gate,
  currently plan-reviewed/impl-pending; §9). It is the join point.
- **S5-7 / S5-8 are post-flip** (S5-7 fidelity; S5-8 enters the world_id program).
- **async-core-storage is a SIDE program** (parallel, not on the S5 critical path); it co-gates the §6 cohort.

---

## §9 Keystone + precondition map

| Keystone / precondition | Role | Status | Gates |
|---|---|---|---|
| **C3 (shell-viewport device facts: dppx + prefers-color-scheme)** | **direct pre-flip gate** | plan-reviewed, **impl-pending** (worktree `elidex-pr-c3`, plan committed local, unpushed) | the FLIP (S5-6) — the VM matchMedia/`@media` device-facts path must have a real source before the VM is the live engine, else responsive sites regress at the flip |
| **`#11-async-core-storage-cookiestore`** (the "2nd keystone" framing) | makes non-compat `EngineMode` modes production-selectable | open (no-cap structural precondition) | the §6 mode-plumbing cohort (NOT S5 itself; S5 ships compat-only) |
| **world_id program** (`#11-wrapper-cache-cross-dom-discriminator` + family) | per-VM side-store → per-entity component migration | open, **strictly AFTER S5** (user-confirmed) | S5-8 (win.open/postMessage) + the `document_origin`/sandbox component migration that S5 deliberately does NOT do (§0) |

**Precondition statement**: `C3 → S5-6 FLIP`. `async-core-storage ∥ S5` (co-gate §6). `world_id ⟶ strictly after
S5` (and S5-8 rides it). The signatures are world_id-invariant, so S5 lands world_id-agnostic.

---

## §10 Open questions for /elidex-plan-review (+ where user judgment is needed)

- **Q1 (FLIP bundling vs splitting):** Is S5-6 (the flip + batch-bind wiring + CSSOM-shadow-sync deletion +
  crate removal) one PR, or does the **batch-bind shell wiring** split from the **crate deletion**? The
  deletion is mechanical (drop dep + delete crate) once nothing calls boa; the batch-bind shell wiring is the
  load-bearing edge (§4). Lean: **one PR** (deletion is the trivial tail of "nothing references boa anymore" —
  splitting would leave a strangler dual-engine moment, violating One-issue-one-way). Confirm at plan-review.
- **Q2 (S5-4/S5-5 granularity):** Are the sandbox cluster (S5-4, 5 slots) and the nav/history cluster (S5-5,
  4 slots) each one plan-reviewed PR, or do they sub-split? They are cohesive enforcement units, but each is
  ≥3-axis edge-dense. The base-case rule says a plan-reviewed narrow slice is terminal — but "5 slots in one
  PR" may exceed "narrow". **Recommend**: keep each as one umbrella-child PR, let its own plan-review decide if
  a slot peels off. (User input wanted on whether 5-slot S5-4 is too coarse.)
- **Q3 (S5-1/S5-2 plan-review skip):** S5-1 (DOMParser, boa-parity-bounded additive) and S5-2 (minor window
  parity) are marked **no plan-review** as base-case narrow additive PRs. Confirm this is within the
  edge-dense base-case exemption (they touch no cohort edge axis in §7).
- **Q4 (S5-8 disposition — the world_id boundary):** `#11-browsing-context-model-window-open-postmessage`
  couples cross-VM WindowProxy identity, which is world_id-bound (#412 C0 carved `#11-windowproxy-browsing-context`
  / `#11-auxiliary-browsing-context-opener` as world_id/multi-doc-bound). **Is S5-8 in the S5 program at all, or
  does it belong wholly to the world_id program?** Lean: **register in S5 cohort for completeness but land it
  inside the world_id program** (it cannot be done world_id-agnostic without a strangler stub). User judgment:
  accept S5-8 leaving the S5 critical path entirely.
- **Q5 (C3 hard-gate confirmation):** Confirm C3 (device facts) is a **hard** pre-flip gate vs a "ship flip,
  matchMedia device-facts temporarily bogus" softer line. The parent §7-Q1 leaned "S5 blocks on at least a
  minimal real matchMedia"; C3 delivers the device-facts source. Lean: **hard gate** (responsive sites are the
  HIGH-stakes regression class; a live VM with bogus `prefers-color-scheme`/dppx is a visible regression).
- **Q6 (oracle sufficiency):** The `elidex-shell` test suite is the behavioral equivalence oracle for S5-6.
  Per the audit's caveat, a surface a grep shows "present" may be behaviorally shallow, and DOMParser/matchMedia
  absence won't be caught by tests that don't exercise them. **Should S5-6 require a pre-flip regression-set
  checklist** (the audit's §A regression list + the S3/S4 surfaces) run against the VM before flipping? Lean:
  **yes** — make the audit regression set an explicit S5-6 acceptance gate, not just "shell tests green".

---

## §11 Per-PR workflow (each S5 PR)

Each PR: plan-verify grep against HEAD → (edge-dense ⇒ own `/elidex-plan-review` BEFORE impl) → impl in
isolated worktree → `/pre-push` (6-stage gate) → `/external-converge` (Codex) → squash merge. S5-6 (the flip)
additionally runs the §10-Q6 regression-set checklist as an acceptance gate. boa fixes only to keep CI green
(no feature mirroring). world_id migration stays out of every S5 PR (§0).

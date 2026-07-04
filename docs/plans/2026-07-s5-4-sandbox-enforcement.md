# S5-4 — sandbox / security enforcement edge cluster (the canonical gate-predicate program)

Per-PR-cluster plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`,
§5 row "S5-4 sandbox/security enforcement edge"). **Anchor = the ideal end-state**, not an incremental
patch (`feedback_plan-memo-anchor-on-ideal-not-incremental`).

S5-4 is a **FLIP-precondition** cluster (umbrella §5 type-(a): land BEFORE the S5-6 boa→VM flip). It
covers the sandbox-method gates + origin-isolation edges that only bite once the VM drives real shell
traffic: alert/confirm/prompt + window.open gating, sandboxed-fetch opaque-origin, iframe-origin-before-
initial-scripts, scripting-disabled event-handler processing, worker-port MessageEvent origin. The
cluster crosses **sandbox flags × origin × scripting-disabled × fetch isolation** (umbrella §7) —
edge-dense security work, hence this mandatory `/elidex-plan-review` before any impl (CLAUDE.md
"Edge-dense work = multi-PR program + 実装前 plan-review 必須"; sandbox bypass = security-by-structure).

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE impl. §0 answers umbrella Q2 (slice granularity),
> §4.1 settles the umbrella's open predicate-crate-home decision, §6 maps the edge matrix so
> plan-review can pre-empt the review tail.

> **Binding umbrella pre-decisions (inherited, not re-litigated here):** (1) the sandbox /
> scripting-disabled gate *predicate* lands **engine-independent** — never a fresh `vm/host/`
> algorithm body (Layering mandate; S5-3b/c precedent: keepalive tier rules → engine-indep
> `elidex-api-ws` / `elidex-api-observers`); (2) **boa = light-touch** (deletion-bound, D-26 PR7):
> boa parity defines the behavioral baseline, no boa-side feature work; (3) **no per-VM-side-store →
> component migration in any S5 PR** (umbrella §0.1; `document_origin` / sandbox flags stay interim
> per-VM HostData; the migration is the agent-scoped-World **B1** program, post-S5, PR #434
> `docs/plans/2026-06-agent-scoped-ecsdom-world.md`).

All file:line cites grep-verified against `main` HEAD `78d4d2e6` (2026-07-02; incl. the
plan-review fix pass F1–F14, same date/HEAD). Every spec § / anchor
webref-verified 2026-07-02 (sources: `html`, `fetch`, `service-workers`; §2.7 records the corrections
where the slot ledger's expected anchors were wrong).

---

## §0 Umbrella Q2 resolution — the 5-slot cluster SUB-SPLITS into 5 slices under ONE plan-review

Umbrella §10 Q2 asked: is 5-slot S5-4 one plan-reviewed PR, or does it sub-split? **Answer: sub-split
— 5 slices, derived from the edge matrix, not from convenience.** The rationale, both directions:

**Why not one PR.** The 5 slots share one *predicate substrate* (the sandbox-flag reads), but their
*enforcement sites* live in five nearly-disjoint subsystems with disjoint blast radii and disjoint
test oracles:

| Enforcement site | Subsystem touched | Oracle |
|---|---|---|
| method gates (alert/confirm/prompt/open) | VM `host/window.rs` natives + session back-channel + shell drains | VM integration tests + shell drain tests |
| fetch opaque-origin | `elidex-net` broker contract + VM fetch dispatch | net unit tests + VM fetch tests |
| iframe origin ordering | shell `content/iframe/load.rs` only | shell integration test |
| scripting-disabled handler processing | VM `event_handler_attrs.rs` + dispatch chokepoint | VM handler tests |
| worker-port MessageEvent origin | VM worker channel (`worker.rs` / `worker_thread.rs`) | VM worker tests |

One PR spanning VM natives + a net-broker type change + a shell load-path reorder + the worker channel
+ the handler compile path is exactly the #339 shape (implementation ~1 commit, review tail 30+
commits): every reviewer finding in any subsystem re-gates the whole security cluster. The 4
intersecting invariant axes make findings *likelier*, which multiplies the cost of bundling.

**Why not 5 separate plan-review cycles.** The slices are strongly *substrate-coupled* (all read the
same flag set; three consume the same canonical predicate home; the fetch slice's end-to-end test
depends on the ordering fix) — reviewing them piecemeal would re-litigate the shared decisions
(predicate home §4.1, origin-type unification §4.4, activation default §4.3.3) five times. And three
of the five slices (4b, 4e, and arguably 4a) are narrow single-subsystem fixes that would not
individually trigger the edge-dense rule.

**The structure** (base-case rule, CLAUDE.md / umbrella §0.4: a plan-reviewed narrowly-scoped per-PR
slice under an approved umbrella is a terminal unit):

| PR | Name | Closes slot | Depends on | Size |
|---|---|---|---|---|
| **S5-4a** | canonical sandbox/scripting predicate home + §8.1.8.1 gate completion | `#11-scripting-disabled-eventhandler-processing-step1` | — | S–M |
| **S5-4b** | iframe origin/flags installed BEFORE initial scripts | `#11-iframe-origin-before-initial-scripts` | — | S |
| **S5-4c** | VM sandbox method gates: alert/confirm/prompt + window.open + modals/popup/top-nav | `#11-vm-sandbox-method-gates-and-modals` (folds the F5 top-nav 2-flag concern) | S5-4a | **L** |
| **S5-4d** | sandboxed-fetch opaque-origin isolation (broker origin-type unification) | `#11-sandbox-fetch-opaque-origin-isolation` | S5-4b (soft — test fidelity, §5.4) | M |
| **S5-4e** | dedicated-worker port MessageEvent origin = "" | `#11-worker-port-message-no-origin` | — | S |

Dependency order: **{4a, 4b, 4e} parallel → {4c (after 4a), 4d (after 4b, soft)}**. Every slice is
independently shippable and boa stays live throughout (the cluster is VM/shell-substrate work; the
flip consumes it at S5-6).

**Plan-review economy**: this memo carries **all five slices at per-PR depth** (§5) — one 5-agent
review of this memo makes each slice a plan-reviewed terminal base case; no per-slice follow-up memos
are planned. **Exception hatch**: if plan-review judges S5-4c (the only L slice: new VM surface + a
session-contract extension + shell drain rewiring) too deep for a §5 section, it peels off into its
own memo — that is the one slice where the S5-3 precedent (program memo → per-arm memos) could
recur. **Recommendation: accept the 5-slice structure with S5-4c reviewed from this memo** — its §5.3
enumerates the full gate/target/activation matrix, which is what a dedicated memo would add.

Human-PM confirmation requested at plan-review: (a) the 5-slice split, (b) S5-4c staying in-memo,
(c) the S5-4e scope narrowing (§2.6 — SW messages are spec-required to CARRY origin, so the slot's
"empty origin" fix applies to the dedicated-worker channel only).

**✅ RESOLVED at plan-review — converged BEFORE the first S5-4a implementation commit (2026-07-02)**:
Q1 adjudicated — the 5-slice structure + S5-4c in-memo ACCEPTED (post-F1/F2 the §5.3.2 design is
specified to disposition-enum depth; a dedicated memo would add nothing). The §9-Q1 text is retained
for the record, marked resolved.

---

## §1 Scope + slot map

### §1.1 What S5-4 is

The VM today has the sandbox *substrate* (S1b: flags + origin threaded via `HostDriver`,
`set_origin` / `set_sandbox_flags`, landed) but not the *enforcement edges*: the VM has **no**
alert/confirm/prompt/window.open natives at all (§3.2), fetch derives its request origin from
`current_url` instead of the canonical opaque-aware `document_origin()` (§3.4), the shell installs an
iframe's origin **after** its initial scripts already ran (§3.3), the §8.1.8.1 run-time
scripting-disabled gate is only "suppressed by construction" (§3.5), and worker messages stamp a page
origin the spec says must be empty (§3.6). All five are inert or masked today because **boa is the
live engine**; at the flip each becomes a live sandbox-bypass or spec-conformance regression. S5-4
lands the enforcement BEFORE the flip so S5-6 swaps engines onto an already-gated surface.

### §1.2 The 5 covered defer slots (ledger verbatim → slice)

1. `#11-vm-sandbox-method-gates-and-modals` → **S5-4c**. `modals_allowed()` accessor + the
   popup/top-nav/modal gate wiring, landing atomically with the VM's `window.open` / `alert` /
   `confirm` / `prompt` natives (boa parity sites `globals/window/mod.rs:354-502`; window.open needs
   the S1c `NavigationRequest` back-channel + two new engine-agnostic channels, §4.3.2). Folds the F5
   top-nav 2-flag concern: `elidex_plugin::IframeSandboxFlags` has a single `ALLOW_TOP_NAVIGATION`
   bit; the spec's with/without-user-activation 2-flag fidelity + the
   `allow-top-navigation-by-user-activation` token are delivered here (§4.3.3).
2. `#11-sandbox-fetch-opaque-origin-isolation` → **S5-4d**. Route the fetch request origin through
   `document_origin()`'s opaque-ness (sandboxed iframe → `Origin: null` + all-cross-origin CORS +
   credential strip), which forces the `SecurityOrigin` ↔ `url::Origin` bridging decision on the
   `elidex_net::Request` broker contract (§4.4).
3. `#11-iframe-origin-before-initial-scripts` → **S5-4b**. Pre-existing shell ordering bug:
   `iframe/load.rs` builds + runs initial scripts before `make_in_process_entry` calls
   `set_origin` / `set_sandbox_flags` (§3.3) — the in-process paths violate the `set_origin` contract
   doc (`host_data.rs:1097-1104`: the embedder "installs it before scripts run").
4. `#11-scripting-disabled-eventhandler-processing-step1` → **S5-4a**. HTML §8.1.3.4 "scripting is
   disabled" must gate event-handler processing. **Anchor correction** (§2.7-C1): the ledger's
   "§8.1.8.1 step 1 compile gate" conflates TWO distinct gates — the *compile* gate is
   §8.1.8.1 *getting the current value of the event handler* **step 3.2** (already implemented), the
   missing piece is *the event handler processing algorithm* **step 1** (the invocation gate).
5. `#11-worker-port-message-no-origin` → **S5-4e**. Verified anchor (§2.6): HTML §9.4.4 Message
   ports, *message port post message steps* step 7.7 fires the MessageEvent with only `data` +
   `ports` initialized → `origin` stays the `MessageEventInit` default `""`. elidex stamps a
   page/script origin instead. Scope = the dedicated-worker channel; SW messages are the opposite
   case (origin spec-REQUIRED, §2.6) and already out of the delivered gap.

### §1.3 Non-goals (bounded out, with owners)

- **WindowProxy / auxiliary browsing-context creation** for `window.open` — S5-8 (B1-gated,
  umbrella Q4). S5-4c's `window.open` returns `null` always (boa parity, §3.2); only the *gates* +
  URL-routing back-channel land here.
- **Popup sandboxing-flag-set propagation** (§7.1.5 *sandbox propagates to auxiliary browsing
  contexts flag*, *one permitted sandboxed navigator*, choosing a navigable step 8, "create a new
  top-level traversable" case, substep 9) — requires an
  auxiliary-browsing-context object to stamp; S5-8/B1-bound (carve §8-D1).
- **Shell modal UI** for alert/confirm/prompt — NOT needed for spec conformance: §8.9.1 *cannot show
  simple dialogs* step 4 "Optionally, return true" sanctions a UA that never shows simple dialogs
  (§2.3). elidex opts in permanently; no slot owed (§5.3.1).
- **`EngineMode` threading / storage gating** — the structural bucket-routing IS delivered
  (`vm/host/storage.rs:103` routes bucket choice through `document_origin()`; opaque → sentinel
  bucket), but the spec-complete THROW surface is owned by two existing OPEN slots. Cite split
  (webref-verified): the §7.1.5 *sandboxed origin* flag prose only says the flag "prevents script
  from reading from or writing to the `document.cookie` IDL attribute, and blocks access to
  `localStorage`" — no throw there; the `SecurityError` THROW mandate lives at the accessor
  algorithms themselves — `document.cookie` = HTML §3.1.4 Resource metadata management
  (`#dom-document-cookie`), `localStorage` getter = HTML §12.2.3 The localStorage getter
  (`#dom-localstorage`). The owning slots — `#11-storage-opaque-origin-securityerror` +
  `#11-cookie-opaque-origin-securityerror` — were re-scoped by the ledger to M4-13 infra because
  they couple to about:blank origin-inheritance (throwing unconditionally on opaque would regress
  about:blank pages pre-inheritance). Disposition → §8.
- **Per-VM store → ECS component migration** for origin/sandbox — B1, umbrella §0.1 (§4.5).
- **CSP `sandbox` directive** (CSP-derived sandboxing flags, §7.1.5) — no CSP-policy plumbing exists
  beyond frame-ancestors; compat surface, out of the S5-4 gated subset (noted in §8 audit, no slot —
  demand-gated with the CSP program).

---

## §2 Spec substrate (webref-verified 2026-07-02)

### §2.1 HTML §7.1.5 Sandboxing (`html#sandboxing`) — the flag set + token parse

Verified dfns (all → §7.1.5 Sandboxing):

| Concept | Anchor |
|---|---|
| sandboxing flag set | `#sandboxing-flag-set` |
| sandboxed navigation browsing context flag | `#sandboxed-navigation-browsing-context-flag` |
| sandboxed auxiliary navigation browsing context flag | `#sandboxed-auxiliary-navigation-browsing-context-flag` |
| sandboxed top-level navigation **without** user activation browsing context flag | `#sandboxed-top-level-navigation-without-user-activation-browsing-context-flag` |
| sandboxed top-level navigation **with** user activation browsing context flag | `#sandboxed-top-level-navigation-with-user-activation-browsing-context-flag` |
| sandboxed origin browsing context flag | `#sandboxed-origin-browsing-context-flag` |
| sandboxed forms browsing context flag | `#sandboxed-forms-browsing-context-flag` |
| sandboxed scripts browsing context flag | `#sandboxed-scripts-browsing-context-flag` |
| sandboxed modals flag | `#sandboxed-modals-flag` |
| sandbox propagates to auxiliary browsing contexts flag | `#sandbox-propagates-to-auxiliary-browsing-contexts-flag` |
| one permitted sandboxed navigator | `#one-permitted-sandboxed-navigator` |
| parse a sandboxing directive | `#parse-a-sandboxing-directive` |
| `allow-top-navigation-by-user-activation` (attr-value, for=iframe/sandbox) | `#attr-iframe-sandbox-allow-top-navigation-by-user-activation` |

*Parse a sandboxing directive* (prose verified) — the token→flag mapping S5-4 gates on:

- *sandboxed auxiliary navigation* flag set **unless `allow-popups`** (⚠ there is **no** "sandboxed
  popups flag" — the popup gate is the auxiliary-navigation flag; §2.7-C3).
- *sandboxed top-level navigation without user activation* flag set **unless `allow-top-navigation`**.
- *sandboxed top-level navigation with user activation* flag set **unless
  `allow-top-navigation-by-user-activation` OR `allow-top-navigation`** (spec note: both tokens
  together = document conformance error, `allow-top-navigation` wins).
- *sandboxed origin* flag unless `allow-same-origin`; *forms* unless `allow-forms`; *scripts* unless
  `allow-scripts`; *modals* unless `allow-modals`.
- *sandboxed modals flag* prose: prevents `window.alert()` / `confirm()` / `print()` / `prompt()` /
  the `beforeunload` event. (elidex delivered surface = alert/confirm/prompt; print + beforeunload
  prompting are unimplemented UA surfaces — nothing to gate, noted in §5.3.)
- *sandboxed origin* flag prose: forces an opaque origin **and** blocks `document.cookie` +
  `localStorage` (already structurally delivered, §1.3).

### §2.2 HTML §8.1.3.4 Enabling and disabling scripting (`html#enabling-and-disabling-scripting`)

Prose verified. **Scripting is enabled** for an *environment settings object* `settings` when ALL of:
(1) the UA supports scripting; (2) the user has not disabled scripting for `settings`; (3) either
`settings`'s global is not a `Window`, **or** its associated `Document`'s **active sandboxing flag
set does not have the sandboxed scripts flag set**; (4) WebDriver BiDi scripting-enabled is true.
*Scripting is disabled* = the negation.

**Scripting is disabled for a platform object** `object` if ANY of: (a) disabled for `object`'s
relevant settings object; (b) `object` implements `Node` and its node document's **browsing context
is null**; (c) `object` implements `Window` and its associated Document's browsing context is null.
Clause (b) is the load-bearing edge for slot 4: it is the only path by which a *compiled* handler can
exist while scripting is disabled (§5.1.2).

### §2.3 Simple dialogs — HTML §8.9.1 (`html#simple-dialogs`), gate = `#cannot-show-simple-dialogs`

(⚠ §8.9.1, NOT §8.8 — §8.8 = Microtask queuing; §8.9 = User prompts; §2.7-C4.) *Cannot show simple
dialogs* for a Window, verified steps: **1.** active sandboxing flag set has the **sandboxed modals
flag** → true; **2.** relevant settings object's origin and **top-level origin** not same
origin-domain → true; **3.** termination nesting level nonzero → optionally true; **4.** *"Optionally,
return true"* (UA-choice, e.g. ignore-all-dialogs) ; **5.** false. Method steps (each verified):
`alert()` step 1 cannot-show → **return** (undefined); `confirm()` step 1 → **return false**;
`prompt()` step 1 → **return null**. These returns are exactly boa's current unconditional returns
(§3.2), so the VM can be simultaneously boa-parity AND spec-faithful by opting into step 4 (§5.3.1).

### §2.4 window.open + the popup / top-nav gates

- **window open steps** = §7.2.2.1 Opening and closing windows, `#window-open-steps` (verified). The
  steps themselves contain **no sandbox check** — gating is delegated:
- **Popup gate** = §7.3.1.7 Navigable target names, `#the-rules-for-choosing-a-navigable` (verified
  prose): step 3 snapshots `sandboxingFlagSet`; step 8's first-applicable-option list contains *"If
  sandboxingFlagSet has the **sandboxed auxiliary navigation browsing context flag** set"* → popup
  blocked (may report to console). Also relevant: step 8-case "no transient activation + popup
  blocker" (activation-coupled), and step 8's "create a new top-level traversable" case substep 9
  (flag-set propagation to the popup — S5-8, §1.3).
- **Top-nav gate** = §7.4.2.4 Preventing navigation, dfn *allowed by sandboxing to navigate* →
  `#allowed-to-navigate` (verified prose): when target is the source's top-level ancestor — step 3.2:
  has transient activation AND *with-user-activation* flag set → false; step 3.3: NO transient
  activation AND *without-user-activation* flag set → false; else true. This is the 2-flag fidelity
  the F5 concern deferred to S5-4.

### §2.5 Fetch — request origin, opaque serialization, credentials, CORS

- Request **origin** = Fetch §2.2.5 Requests, `#concept-request-origin` (default `"client"`).
- **Serializing a request origin** = §2.2.5 `#serializing-a-request-origin` (verified): if request's
  redirect-taint ≠ "same-origin" → `"null"`; else request's origin, **serialized**.
  *Byte-serializing* = `#byte-serializing-a-request-origin`.
- Origin **serialization of an opaque origin = `"null"`** = HTML §7.1.1 Origins,
  `#ascii-serialisation-of-an-origin` step 1 (verified prose).
- **`Origin` header** = Fetch §3.2 `#origin-header` (verified prose): used for all HTTP fetches whose
  **response tainting is "cors"**, and all whose method is neither GET nor HEAD; ABNF
  `origin-or-null = serialized-origin / %s"null"`.
- **credentials mode** = §2.2.5 `#concept-request-credentials-mode`; **response tainting** = §2.2.5
  `#concept-request-response-tainting`. Operative attach rule = Fetch §4.6 HTTP-network-or-cache
  fetch (`#http-network-or-cache-fetch`, verified prose): *includeCredentials* is true iff
  credentials mode is `"include"`, **or** `"same-origin"` AND response tainting is `"basic"` — i.e.
  the strip routes through the **response-tainting intermediary**, not a direct origin equality. It
  is still *structural* for a sandboxed document: tainting `"basic"` requires the request origin
  same-origin with the URL (mode navigate/websocket aside — not the `fetch()` surface), and an
  opaque origin is same-origin with nothing → tainting is never `"basic"` for HTTP(S) → cookies
  strip under `"same-origin"` (no per-call-site special case; §4.4). `"include"` continues to
  attach (subject to CORS allow-credentials — wildcard-with-credentials already rejected
  broker-side, `cors.rs:57`).

### §2.6 Message ports / workers — where origin must be EMPTY vs REQUIRED

- **Message port post message steps** = HTML §9.4.4 Message ports,
  `#message-port-post-message-steps` (verified prose): step 7.7 — *"Fire an event named message at
  messageEventTarget, using MessageEvent, with the **data** attribute initialized to messageClone and
  the **ports** attribute initialized to newPorts."* No `origin` initialization → the
  `MessageEventInit` default applies. Verified `MessageEvent` IDL: `initMessageEvent(..., optional
  USVString origin = "", ...)` — default **empty string**.
- **Worker.postMessage delegates to the port**: §10.2.6.3 Dedicated workers and the Worker interface
  (verified prose): *"The postMessage(...) methods on Worker objects act as if ... they immediately
  invoked the respective postMessage(...) on this's outside port"*. Same for
  `DedicatedWorkerGlobalScope.postMessage`. ⇒ dedicated-worker `message` events have
  **origin = ""** per spec.
- **Contrast — origin REQUIRED**: `window.postMessage` (HTML §9.3, delivered correctly today via
  `document_origin()`, §3.6) and **ServiceWorker.postMessage** = Service Workers §3.1.5
  `#dom-serviceworker-postmessage-message-options` (verified prose): step 6.2.2 *"Let origin be
  incumbentSettings's origin"*, initialized onto the `ExtendableMessageEvent`. ⇒ S5-4e must NOT touch
  the SW channel (§5.5).

### §2.7 Anchor corrections found (record for the plan-review spec axis)

- **C1 (slot 4 "step 1")**: §8.1.8.1's *compile* gate is *getting the current value of the event
  handler* (`#getting-the-current-value-of-the-event-handler`) **step 3.2** — "If document's active
  sandboxing flag set has its **sandboxed scripts browsing context flag** set, then return null"
  (checks the FLAG only). The **step 1** the slot names belongs to *the event handler processing
  algorithm* (`#the-event-handler-processing-algorithm`) — "If **scripting is disabled** for
  eventTarget, then return" (checks the FULL §8.1.3.4 predicate incl. platform-object clauses). Two
  different gates testing two different predicates; elidex has the first, lacks the second (§3.5).
- **C2 (slot 5 "§9.4.4-ish")**: confirmed §9.4.4 = "Message ports"; the operative algorithm is the
  *message port post message steps* step 7.7 (non-initialization of `origin`), reached from
  `Worker.postMessage` via §10.2.6.3. SW messages are the opposite polarity (§2.6).
- **C3 (no "sandboxed popups flag")**: the `allow-popups` token clears the **sandboxed auxiliary
  navigation browsing context flag**; the gate site is §7.3.1.7 choosing-a-navigable step 8, NOT the
  window open steps.
- **C4 (simple dialogs section)**: §8.9.1 Simple dialogs (under §8.9 User prompts). §8.8 = Microtask
  queuing — do not cite §8.8 for dialogs.
- **C5 (top-nav gate home)**: enforcement = §7.4.2.4 Preventing navigation
  (`#allowed-to-navigate`, dfn *allowed by sandboxing to navigate*), keyed on **sourceSnapshotParams'
  sandboxing flags + has-transient-activation** — not a method-local check.

### §2.8 Spec coverage map (cluster rows; per-slice branch detail in §5)

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §7.1.5 Sandboxing | parse a sandboxing directive | token→flag mapping for the 7 delivered tokens (+ new `allow-top-navigation-by-user-activation`) | S5-4c (`elidex-plugin` parse) | ✗ (delivered-token subset; rest §8 audit) | yes (`sandbox` attr string) |
| HTML §8.9.1 Simple dialogs | cannot show simple dialogs; alert/confirm/prompt step 1 | modals flag / origin-domain / step-4 UA-opt | S5-4c VM natives | ✓ (gate); presentation = step-4 opt-out | yes (message strings) |
| HTML §7.2.2.1 / §7.3.1.7 | window open steps → choosing a navigable | `_self`/`_parent`/`_top`/named/`_blank` × aux-nav flag | S5-4c VM native + back-channel | ✗ (returns null — WindowProxy = S5-8) | yes (url/target/features strings) |
| HTML §7.4.2.4 Preventing navigation | allowed by sandboxing to navigate step 3 | 2 top-nav flags × transient activation | S5-4c predicate + shell/VM gate sites | ✗ (top-nav-of-self subset; general ancestor matrix = S5-8/B1) | yes (nav target) |
| HTML §8.1.3.4 / §8.1.8.1 | scripting-disabled; processing-algorithm step 1 + get-current-value step 3.2 | settings-level ∧ platform-object clauses | S5-4a | ✓ (both gates; clause (b) per §5.1.2) | yes (event-handler attr source) |
| Fetch §2.2.5 / §3.2 + HTML §7.1.1 | request origin; serialize ("null"); Origin header; credentials mode | opaque vs tuple × credentials mode × redirect-taint | S5-4d | ✗ (delivered-CORS subset; broker already bounds it) | yes (fetch URL/init) |
| HTML §9.4.4 / §10.2.6.3 | message port post message steps step 7.7 | dedicated-worker both directions (+ messageerror) | S5-4e | ✓ (delivered worker surface) | yes (postMessage payload) |

**Breadth verdict**: K = 3 specs (HTML, Fetch, Service Workers-as-boundary-check) over M = 7 rows —
inside the SPLIT-RECOMMENDED band as a cluster, and **the split is §0's answer**: each slice carries
1–2 rows, well inside the single-PR band.

**User-input touch audit** (`feedback_trust-boundary-enumerate-upfront`): `sandbox` attr string
(existing `parse_sandbox_attribute` tokenizer — unknown tokens ignored, no new parse surface beyond
one added keyword); `window.open` url/target/features (url → existing `encoding-parse` seam; target →
closed 5-way dispatch; features → **ignored** at boa parity, tokenization = S5-8, §8-D1); alert/
confirm/prompt message strings (never rendered — step-4 opt-out; no sink); event-handler attr source
(existing compile seam, gate only *suppresses*); fetch URL/init (existing validated dispatch);
postMessage payload (existing structured-clone seam — S5-4e only *removes* a stamped field). **No new
trust boundary is opened; every slice narrows or corrects an existing one.**

---

## §3 Current-state code map (HEAD `78d4d2e6`)

### §3.1 The substrate (landed, S1b) — flags, origin, predicates

- **`IframeSandboxFlags`** — `crates/core/elidex-plugin/src/origin.rs:123-143`, `bitflags` u16,
  **6 allow-bits**: `ALLOW_SCRIPTS`, `ALLOW_SAME_ORIGIN`, `ALLOW_FORMS`, `ALLOW_POPUPS`,
  `ALLOW_TOP_NAVIGATION` (single bit — the F5 gap), `ALLOW_MODALS`. Positive "allow-token"
  representation (the inversion of the spec's restriction-flag set); `None` = unsandboxed,
  `Some(empty)` = maximum restriction. Parser `parse_sandbox_attribute` `origin.rs:150-164`.
- **`SecurityOrigin`** — `origin.rs:22-32`: `Tuple {scheme, host, port}` | `Opaque(u64)`
  (globally-unique counter); `from_url` :46, `opaque()` :85, `serialize()` :94 (**opaque → `"null"`**).
- **Per-VM state (interim per CLAUDE.md exception, B1-bound)** —
  `crates/script/elidex-js/src/vm/host_data.rs`: `sandbox_flags: Option<IframeSandboxFlags>` :196,
  `document_origin_override: Option<SecurityOrigin>` :229 (doc :208-226 records the component-home
  deferral), `fallback_opaque_origin` :242. Canonical resolver **`VmInner::document_origin()`** —
  `crates/script/elidex-js/src/vm/host/navigation.rs:292-309` (override → else `from_url(current_url)`,
  opaque fallback pinned for stability).
- **Existing predicates — duplicated 3×** (the One-issue-one-way violation §4.1 removes): identical
  `sandbox_flags.is_none_or(|f| f.contains(X))` bodies in (i) VM `host_data.rs` —
  `scripts_allowed` :1068, `forms_allowed` :1085, `popups_allowed` :1092; (ii) boa
  `crates/script/elidex-js-boa/src/bridge/iframe_bridge.rs` — :66/:76/:86 + **`modals_allowed`
  :96-101 (boa-ONLY — no VM, no trait equivalent)**; (iii) surfaced on the session trait
  `crates/script/elidex-script-session/src/engine.rs:283-318` (`forms_allowed` :306,
  `popups_allowed` :311 — **no `scripts_allowed`, no `modals_allowed`**).
- **Consumers**: eval gate `elidex-js/src/engine.rs:116-125` (scripts); handler-attr compile gate
  `vm/host/event_handler_attrs.rs:554-560`; shell form gate `content/form_input.rs:129`; shell link
  gates `content/event_handlers.rs:186-198` (popup) / :199-214 (top-nav, single-bit); boa window
  `globals/window/mod.rs:359` (popup) / :400 (top-nav) / :467,480,493 (modals).
- **ECS state**: only the raw attr string — `IframeData.sandbox: Option<String>`
  (`crates/core/elidex-ecs/src/components.rs:918`), parsed shell-side (`iframe/load.rs:335-340`).
  **No component holds `SecurityOrigin` or `IframeSandboxFlags`** (correct interim state, §4.5).
- **Crate DAG** (from Cargo.toml, load-bearing for §4.1): `elidex-plugin` = leaf (zero elidex deps);
  `elidex-net` → plugin ONLY; `elidex-script-session` → plugin (+css/ecs/…, NOT dom-api);
  `elidex-dom-api` → script-session → plugin.

### §3.2 boa parity sites — the behavioral baseline for S5-4c

`crates/script/elidex-js-boa/src/globals/window/mod.rs`:

- `register_window_open` :354-421 — entry gate `if !bridge.popups_allowed() { return null }` :359
  (⚠ boa gates ALL targets incl. `_self` — not spec-shaped; §5.3.2 fixes the shape in the VM);
  target dispatch :388-412: `_blank` → `queue_open_tab(url)`; `_self` →
  `set_pending_navigation {replace:false}`; `_parent`/`_top` → `ALLOW_TOP_NAVIGATION` gate :399-403 →
  `set_pending_navigation`; named → `set_pending_navigate_iframe(name, url)`. Returns null/undefined
  on every path (no WindowProxy). Features string ignored.
- `register_modals` :462-502 — alert :465-475 / confirm :477-488 / prompt :490-501; each gates on
  `modals_allowed()` and **returns the same value on both branches** (undefined / false / null) —
  i.e. the gate is behaviorally invisible today; the *shape* is the parity content.
- Back-channel (boa-bridge-only): `bridge/navigation.rs` — `queue_open_tab` :55,
  `drain_pending_open_tabs` :60, `set_pending_navigate_iframe` :65, `drain_pending_navigate_iframe`
  :74.
- **VM side: ABSENT.** `vm/host/window.rs` `WINDOW_METHODS` :601-625 = scroll*/postMessage/
  getComputedStyle/getSelection/matchMedia — **no alert/confirm/prompt/open**; explicit deferral
  comment :361-364. `HostData` has **no `modals_allowed`**; VM `NavigationState`
  (`vm/host/navigation.rs`: `pending_navigation` :129 single-slot last-wins, `pending_history` :145,
  referrer :151) has **no open-tab / named-iframe channel**.
- Shell drains — ALL sites enumerated: **(i)** `content/navigation.rs` — engine-agnostic
  `take_pending_navigation` :190 / `take_pending_history` :199, then **boa-bridge-only**
  `drain_pending_open_tabs` :206 → `OpenNewTab` and `drain_pending_navigate_iframe` :222 → hit:
  `iframe::navigate_iframe` :226 / **miss: else-branch `ContentToBrowser::OpenNewTab(url)`
  :227-229** — a named-target-miss → popup promotion EXISTS at HEAD; **(ii)**
  `content/event_loop.rs:145-148` — a SECOND boa-bridge-only `drain_pending_open_tabs` →
  `ContentToBrowser::OpenNewTab` site, existing for the pure-async case (its comment: a pure-async
  window.open with no DOM change would otherwise stall under Wait); **(iii)** `app/navigation.rs`
  (interactive app mode) — drains `take_pending_navigation` :18 / `take_pending_history` :30 ONLY
  (no open-tab / named-frame drains today). boa never reaches the miss-promotion sandboxed only
  because its entry gate :359 blocks ALL of window.open; a spec-shaped VM native that dropped that
  entry gate without gating that miss-branch would open a sandbox bypass at flip (§5.3.2 gates it by
  snapshot verdict).
- Shell modal surface: **none** (grep: only `<dialog>`/`method=dialog` form handling,
  `content/form_input.rs:147-170`). alert/confirm/prompt are end-to-end no-ops.

### §3.3 The iframe origin-ordering bug (S5-4b)

`crates/shell/elidex-shell/src/content/iframe/load.rs`, in-process path `load_iframe_from_url`
:82-206 — verified sequence: **(1)** :119 fetch; **(2)** :130-134 compute
`origin = apply_sandbox_origin(from_url, sandbox_flags, credentialless)` (opaque unless
`allow-same-origin`); **(3)** :136-138 cross-origin → OOP path (correct there, see below); **(4)**
:169-184 `build_pipeline_from_loaded` → `run_scripts_and_finalize` (`lib.rs:903`) — **initial scripts
RUN with `document_origin_override` still unset**; **(5)** :189 `make_in_process_entry` → :233-235
`set_sandbox_flags` / `set_origin` / `set_iframe_depth` — **after**. Same shape on the
srcdoc/about:blank/no-src paths (`load_iframe` :30-78 → entry at :75) and `blank_entry` :316-328.
Consequence: a sandboxed iframe's initial scripts observe the **URL-derived tuple origin instead of
the opaque origin** (`document_origin()` falls through to `current_url`) — an origin-isolation hole,
and the direct falsifier of the `set_origin` contract doc (`host_data.rs:1097-1104`). **The OOP path
is the correct template**: `make_out_of_process_entry` :254-310 sets flags :288-291 + origin :292-298
BEFORE `iframe_thread_main` :300 runs scripts.

**⚠ CORRECTION (S5-4b impl contact, 2026-07-02)**: the "OOP path is the correct template" claim is
falsified — initial scripts are evaluated INSIDE `build_pipeline_from_loaded` (~:272), i.e. BEFORE
the :288-298 installs, so the OOP path had the same ordering gap. And since a sandboxed URL-load
iframe's opaque origin ≠ parent routes it to the OOP path (`load.rs:136`), the URL-case harm was on
OOP. S5-4b fixed BOTH: the security installs converge on a single pre-eval chokepoint in the
pipeline builder (`FrameSecurity` threaded into `run_scripts_and_finalize`), in-process AND OOP.

### §3.4 Fetch request-origin hold-out (S5-4d)

`crates/script/elidex-js/src/vm/host/fetch/dispatch.rs` — `origin_for_request(source, _target)`
:219-230 is literally `Some(current_url.origin())`, with carve-out comments naming the slot at
:171-188 (`reject_same_origin_cross_origin`), :220-229, :324-327 (`attach_default_origin` — already
emits `Origin: null` for opaque *initiators* like `data:`, but derives from URL not document).
**Every other settings-origin reader already routes through `document_origin()`** — event_source.rs
:311, websocket.rs :394, pending_tasks.rs :584 (window.postMessage), storage.rs :103, history.rs
:199. Fetch is the lone hold-out. Broker contract:
`elidex_net::Request { origin: Option<url::Origin>, credentials: CredentialsMode, mode: RequestMode, … }`
(`crates/net/elidex-net/src/lib.rs:147-186`); cookie gate `should_attach_cookies` :206-215
(`SameOrigin` ⇒ attach iff `request.origin == request.url.origin()`; `None` ⇒ **attach
unconditionally** — embedder-load carve-out), invoked :465; CORS `validate_cors`
(`src/cors.rs:28`), preflight validator (`preflight/validator.rs:42`,
`url::Origin::ascii_serialization`). **No `SecurityOrigin` ↔ `url::Origin` bridge exists**, and
`url::Origin`'s opaque variant cannot be minted with a chosen identity — the type decision §4.4.

### §3.5 Scripting-disabled event-handler processing (S5-4a)

Engine-indep recording: `elidex-script-session/src/event_handler_consumer.rs` —
`EVENT_HANDLER_ATTRS` table :61-176, consumer stores raw source into the `EventListeners` component
(`set_inline_handler` :468 → `set_uncompiled`). VM compile: `vm/host/event_handler_attrs.rs` —
`ensure_event_handler_current` :521-585 is the reconcile point; the **compile gate exists**
:554-560 (early-return, no compile, when `!scripts_allowed()` — comment :526-553 correctly cites
§8.1.8.1 step 3.2). Dispatch chokepoint: `ScriptEngine::call_listener`
`elidex-js/src/engine.rs:143-164` (comment :156-161 names it "the scripting-disabled chokepoint").
**Missing**: *the event handler processing algorithm* step 1 — suppression of an
**already-compiled** callable when scripting is (or becomes) disabled; today "suppressed by
construction" only because no callable can be created without eval — which clause-(b)
(browsing-context-null, §2.2) breaks: a handler compiled while the document was live must not run
after its browsing context is nulled. Slot marker at :549. ⚠ do-not-conflate note: the parser-side
`scripting_disabled` flag (`mutation/html_fragment.rs:39`, html-parser crates,
`elidex-form/src/inert_document.rs:143`) is the §13 parser's noscript/template flag — a different
concept, untouched by S5-4.

### §3.6 Worker message origin stamping (S5-4e)

Both directions stamp a non-empty origin: parent→worker `vm/host/worker.rs:499-522`
(`current_url.origin().ascii_serialization()`, carve-out comment naming the slot :516);
worker→parent `vm/worker_thread.rs:243` (`script_url.origin()...`), stamped onto
`WorkerToParent::PostMessage { data, origin }` :245-247, drained at `worker.rs:149-157` into
`dispatch_message_event_at` (`worker_scope.rs:257-328` — builds the MessageEvent, `origin` slot from
the channel string, `ports` always empty). `messageerror` sites: `worker.rs:122-124,185`.
**Contrast (already correct)**: window.postMessage routes `compute_own_origin_sid` →
`vm.document_origin().serialize()` (`pending_tasks.rs:583-586`). boa mirrors the stamping
(`elidex-js-boa/src/worker_thread.rs:147`, `globals/worker_scope.rs:249-252`) — light-touch, adjusted
only as far as shared channel types force (§5.5). MessagePort/MessageChannel objects: **not
implemented** (all sites build empty `ports`) — the delivered "port message" surface IS the worker
channel.

---

## §4 Ideal architecture

### §4.1 The predicate home decision — `elidex-plugin` (the umbrella's binary is falsified by the DAG)

The umbrella left "`elidex-dom-api` vs `elidex-script-session`" open. **The codebase data flow rules
out both and selects `elidex-plugin`** (`crates/core/elidex-plugin/src/origin.rs`, or a sibling
`sandbox.rs` module):

1. **Reachability**: the gate has consumers in `elidex-net` (S5-4d: opaque-origin reasoning on the
   broker contract), `elidex-script-session` (trait surface), the VM, boa, and the shell.
   `elidex-net` depends on **plugin only** — it cannot see dom-api or script-session without a new
   (wrong-direction) edge. `elidex-script-session` cannot see dom-api (dom-api depends on session).
   `elidex-plugin` is the unique existing common ancestor.
2. **Cohesion**: the predicate is a pure function of `IframeSandboxFlags` — a type that, together
   with `SecurityOrigin` and `parse_sandbox_attribute`, ALREADY lives in `elidex-plugin::origin`.
   The flag set and its decision functions belong in one module (data + its laws).
3. **One issue, one way**: the identical `is_none_or(contains)` body exists 3× (§3.1). A single
   canonical set of functions in plugin, with HostData / boa bridge / session-trait impls
   **delegating**, kills the duplication in the same PR that adds new predicates — no
   "new seam + legacy branches" residue. (boa's delegation is a mechanical dedupe, not feature work
   — light-touch-compatible.)
4. **Layering mandate**: `elidex-plugin` is engine-independent core; VM `host/` bodies then only
   *marshal* (read `Option<IframeSandboxFlags>` off bound HostData, call the plugin predicate) —
   exactly the S5-3b/c shape (seam marshals, engine-indep crate rules).
5. **Plugin-first**: sandbox flags are already an `elidex-plugin` vocabulary type; the gate becomes
   part of the same extension mental model rather than a new ad hoc module.

**Shape** (final naming = impl detail; the review point is the seam):

> [S5-4a kickoff naming resolution: the pre-existing OS process-sandbox module was renamed to
> `elidex_plugin::process_sandbox`, and `IframeSandboxFlags` + `parse_sandbox_attribute` moved from
> `origin.rs` into `sandbox.rs`, so the flag set and its laws share the module — the
> `elidex_plugin::sandbox::*` cites in this memo are exact; §3.1's `origin.rs` line cites are pre-move.]

```rust
// elidex-plugin (engine-independent; the ONE canonical home)
impl IframeSandboxFlags { /* per-capability bit tests, incl. the 2-arg top-nav decision */ }
pub mod sandbox {
    // `None` = unsandboxed (all allowed); `Some(empty)` = max restriction — the existing contract.
    pub fn scripts_allowed(flags: Option<IframeSandboxFlags>) -> bool;
    pub fn forms_allowed(flags: Option<IframeSandboxFlags>) -> bool;
    pub fn popups_allowed(flags: Option<IframeSandboxFlags>) -> bool;      // aux-navigation, §2.4
    pub fn modals_allowed(flags: Option<IframeSandboxFlags>) -> bool;       // NEW (S5-4c consumer)
    pub fn top_navigation_allowed(flags: Option<IframeSandboxFlags>,
                                  has_transient_activation: bool) -> bool;  // NEW, 2-flag (§4.3.3)
    /// HTML §8.1.3.4 settings-level composition: UA-supports(=true) ∧ ¬user-disabled(hook, =false)
    /// ∧ ¬sandboxed-scripts-flag. Platform-object clause (b) composes at the caller (§5.1.2).
    pub fn scripting_enabled(flags: Option<IframeSandboxFlags>) -> bool;
}
```

Representation stays the **positive allow-token** form (the inversion of the spec's restriction
flags) — it is already the workspace-wide contract and the inversion is total for the delivered
subset; the doc-comment records the mapping to §7.1.5 flag names per bit. (Flipping to
restriction-flag representation would churn every existing reader for zero semantic gain — rejected
as pure-cost.)

### §4.2 Security by structure — each gate is a chokepoint, not call-site sprinkle

Per surface, the structural guarantee (no gated surface can reach its effect except through the
predicate):

| Surface | Chokepoint | Structure |
|---|---|---|
| classic scripts | `ScriptEngine::eval` (`engine.rs:116-125`, exists) | shell cannot run script except via `eval` |
| handler-attr compile | `ensure_event_handler_current` (exists, :554-560) | the ONLY raw-source→callable path |
| handler invocation | same reconcile point + processing-step-1 check before invoke (§5.1) | dispatch cannot reach a handler-derived callable except via the chokepoint |
| modals | `cannot_show_simple_dialogs(...)` helper called as step 1 of each of the 3 natives — natives return before any presentation branch exists | a future shell modal surface can only be driven from behind the gate |
| popups / open-tab | the open-tab **back-channel enqueue** is gated (not the shell drain): a blocked popup never enters `pending_open_tabs` | shell drains can't leak what was never queued |
| top-nav | `top_navigation_allowed` at the two producers (VM `window.open` `_top`/`_parent` arm; shell link-target site `event_handlers.rs:199-214`) | the only two top-nav producers |
| fetch credentials/origin | broker-side `should_attach_cookies` equality + serialize-at-header-attach (§4.4) | opaque strips credentials by type-level equality failure, not an if-branch |
| storage | `storage.rs:103` via `document_origin()` (exists) | bucket keyed by canonical origin |

### §4.3 S5-4c mechanism detail (the L slice)

**§4.3.1 Natives are marshal-only.** `alert`/`confirm`/`prompt`/`open` bodies in `vm/host/window.rs`
do: read bound-HostData flags → call plugin predicate → route to `NavigationState` channels / return.
No algorithm bodies in host/ (Layering mandate). Helper homes, settled: `modals_allowed(flags)` is
the `elidex_plugin::sandbox` pure predicate (§4.1); the *cannot show simple dialogs* composition =
that plugin predicate ∧ a documented UA policy constant (the permanent step-4 opt-in; step 2's
origin-domain check subsumed — §5.3.1), applied AT the native — marshal-scale (two condition reads),
not a host/ algorithm body and not a session helper.

The 5-way target dispatch is likewise NOT a host/ body: it is an engine-independent pure
*disposition* function, home = `elidex-script-session::navigation` (the owner of the channel types
it selects between), e.g.

```rust
// elidex-script-session::navigation (engine-independent)
pub enum WindowOpenDisposition {
    Blocked,                          // gate failed → silent null
    SelfNavigate,                     // _self → NavigationRequest
    TopNavigate,                      // _parent/_top, gate passed → NavigationRequest
    Named { aux_nav_allowed: bool },  // §5.3.2 snapshot verdict rides here (F1)
    OpenTab,                          // _blank/popup, gate passed → OpenTabRequest
}
pub fn window_open_disposition(target: &str,
                               flags: Option<IframeSandboxFlags>,
                               has_transient_activation: bool) -> WindowOpenDisposition;
```

computed over (target, flags, activation), calling the `elidex_plugin::sandbox` predicates. URL
parse failure → SyntaxError is decided AT the native as *boundary marshalling* — input conversion
at the JsValue boundary, same standing as WebIDL arg conversion — so the disposition fn owns the
COMPLETE 5-way outcome set over valid inputs (no dead `url_parsed` parameter; the seam boundary is
unambiguous). The `vm/host/` native only coerces JsValue args, calls the disposition fn, and
enqueues per the result — marshal-only, consistent with this section's claim. §5.3.2 routes through
this fn.

**§4.3.2 The back-channel goes engine-agnostic (session contract), killing the boa-bridge-only
drains.** Ideal end-state: `elidex-script-session::navigation` gains `OpenTabRequest(url)` +
`NamedFrameNavigation { name, url, aux_nav_allowed: bool }` (the §5.3.2 snapshot verdict) alongside
`NavigationRequest`/`HistoryAction`; `HostDriver`
gains `take_pending_open_tabs()` + `take_pending_frame_navigations()`; VM `NavigationState` gains the
two queues (FIFO like `pending_history` — multiple `window.open` calls in one task must all
surface); **BOTH content drain sites** (`content/navigation.rs:206-229` AND
`content/event_loop.rs:145-148`, the §3.2 enumeration) consume **the trait**, not the boa bridge —
a drain left boa-only at flip is E4's forbidden form, and the event_loop site is exactly that risk
if left unrewired (a pure-async VM `window.open(_blank)` with no DOM change would stall under
Wait). App-mode disposition: the interactive app path (`app/navigation.rs:18/:30`) also drains the
two new channels — it is the same trait call, so leaving it out of the 4c delivered surface would
need a reason that does not exist. boa keeps its private drains until S5-6 deletes the crate —
bounded pre-flip coexistence
force-resolved by the flip (the sanctioned staging, S5-3b §0.3 shape; the shell keeps the boa-typed
drain calls on the boa path only).

Two 4c-landing coupling notes: (a) S5-4c touches the umbrella §7 **"navigation" axis** (the
umbrella matrix row currently says "—") — updating that matrix row is a 4c landing deliverable;
(b) `process_pending_actions` (`content/navigation.rs:189-238`) is shared with S5-5's
drain-history-before-navigation work — whichever lands second re-checks drain ordering at that
site. Disjoint-concern flag: `content/navigation.rs:99` carries a pre-existing SW-navigation TODO
("construct document from SW response body") — untouched by 4c; its stated blocker (:81, "requires
M4-10 (elidex-js VM event loop)") is complete, so the TODO is stale — flag for owner-slot check at
4c landing.

**§4.3.3 Top-nav 2-flag fidelity (F5 fold).** Add `ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION = 1 << 6`;
`parse_sandbox_attribute` maps the token per §7.1.5 (both-tokens note: `allow-top-navigation`
implies it). Decision function mirrors §7.4.2.4 steps 3.2/3.3 in allow-form:
`top_navigation_allowed(flags, activation) = allowed(ALLOW_TOP_NAVIGATION) ∨ (activation ∧
allowed(ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION))`. **Activation source**: the workspace has NO
transient-activation tracking (grep: zero hits). Interim per-call-site truth: script-initiated
`window.open` → `false` (the stricter *without-activation* flag governs — conservative,
never-bypasses); the shell link-click site → `true` (a click IS activation at that site, statically).
The predicate takes the bool **parameter** so the seam is activation-ready; real
transient-activation tracking (HTML user-activation model) is carved (§8-D2).

### §4.4 S5-4d mechanism — unify the broker origin type on `SecurityOrigin`

The blocker: `Request.origin: Option<url::Origin>` cannot faithfully carry the document's opaque
origin — `url::Origin`'s opaque variant is freshly-unique per construction (no identity-stable mint
from `SecurityOrigin::Opaque(u64)`), so "convert at the boundary" would break same-origin equality
across two requests from the same sandboxed document. Two-type coexistence + a lossy bridge is a
permanent decision tax (One-issue-one-way). **Ideal: the broker contract speaks the engine's origin
type** — `Request.origin: Option<elidex_plugin::SecurityOrigin>` (`elidex-net` already depends on
plugin, §3.1 DAG; zero new edges):

- `should_attach_cookies`: `SameOrigin` ⇒ attach iff `request.origin ==
  Some(SecurityOrigin::from_url(&request.url))` — opaque ≠ tuple **by type**, credentials strip
  structurally (§2.5). The equality is the delivered stand-in for the spec's tainting route (Fetch
  §4.6 *includeCredentials*: `"same-origin"` attaches iff response tainting is `"basic"`, and
  `"basic"` requires exactly this same-origin relation on the delivered surface) — the strip
  traces through the tainting intermediary, not an ad hoc origin check. `None` = embedder-load
  carve-out unchanged (trust boundary documented).
- `Origin` header / preflight / CORS context: serialize via `SecurityOrigin::serialize()`
  (opaque → `"null"`), replacing `url::Origin::ascii_serialization` at `attach_default_origin`,
  `preflight/validator.rs:42`, `cors.rs` context.
- VM `origin_for_request` → `vm.document_origin()`; `FetchCorsMeta.request_origin` +
  `reject_same_origin_cross_origin` follow (all three carve-out comment sites §3.4 close together).
- Redirect-taint interplay: *serializing a request origin* step 2 (redirect-taint ≠ same-origin →
  `"null"`) — the existing `redirect_tainted` machinery stays the outer clause; opaque origin makes
  the inner serialization `"null"` as well (edge row §6-E7).
- boa fetch sets `origin: None` today (`globals/fetch/mod.rs:140`) — mechanical type-adjust only
  (light-touch); worker/SW construction sites (`worker_thread.rs:126`, `worker_scope.rs:447`,
  `sw_thread.rs:135`) convert `from_url` at the same sites (workers have tuple script origins —
  behavior-neutral).

### §4.5 ECS-native lens

Origin + sandbox flags are **browsing-context-level facts** (not single-entity facts of a DOM node)
currently held as per-VM HostData — the documented interim placement (`host_data.rs:208-229`), whose
component migration is **B1-gated** (`#11-browsing-context-state-ecs-components`, folded per PR #434
§5 req 5) and explicitly banned from S5 (umbrella §0.1). S5-4's design is **storage-home-neutral by
construction**: every predicate is a pure function over the flag VALUE (`Option<IframeSandboxFlags>`
in, bool out), so when B1 moves the value from HostData to a component on the document/browsing-
context entity, the predicates and every gate site survive verbatim — only the read-site changes.
This memo therefore adds **zero** new per-VM side-store state beyond the two FIFO back-channel queues
(§4.3.2), which are per-browsing-context *event queues* (transient work items, the CLAUDE.md (b)
shared-cross-cutting shape — same standing as `pending_history`), not per-entity facts. The ECS-side
`IframeData.sandbox: Option<String>` (raw attr) stays the parse input; no duplicate parsed-flag
component is introduced pre-B1 (avoids a dual-SoT).

---

## §5 Per-slice plan

> **§5.0 Touch-set line counts (1000-line touch-time discipline)**: `vm/host_data.rs` = **1953**
> — S5-4a must run the touch-time cohesion-seam assessment at kickoff (standalone prereq split PR
> if a real seam exists; note the §5.1 delegation itself REDUCES lines).
> **✅ ASSESSED at 4a kickoff (2026-07-02): no split** — a single `HostData` struct + a single
> field-accessor `impl` is a 一枚岩 cohesive unit (no real seam; splitting would be line-count
> mechanics, and the struct-level seam question coincides with the already-deferred VmInner
> sub-struct refactor — `memory/vm-inner-substruct-deferral.md`, no slot). The S5-4a delegation
> itself reduces the file;
> `content/event_handlers.rs` = 994 (the 4c re-key may cross 1000 — monitor);
> `vm/host/window.rs` = 784 + four new natives (monitor).

### §5.1 S5-4a — canonical predicate home + §8.1.8.1 gate completion

**Scope**: (1) `elidex_plugin::sandbox` module (§4.1 shape) with `scripts_allowed` / `forms_allowed`
/ `popups_allowed` / `scripting_enabled` + unit-tested truth tables; (2) delegate the 3 duplicate
bodies (VM `host_data.rs:1068/1085/1092`, boa `iframe_bridge.rs:66/76/86`, and the session-trait impl
docs point at the canonical home) — **no behavior change** on these; (3) the *processing algorithm
step 1* invocation gate at the dispatch chokepoint. `modals_allowed` / `top_navigation_allowed` land
in S5-4c WITH their consumers (dead-code discipline: no unconsumed predicate ships).

**§5.1.2 The step-1 gate + clause (b)**: at `call_listener` / `ensure_event_handler_current`
(`engine.rs:143-164` → `event_handler_attrs.rs:521`), before invoking a handler-derived callable:
settings-level `scripting_enabled(flags)` ∧ platform-object clause (b) (target implements Node and
node document's browsing context is null → disabled). Impl-verify item (open question §9-Q3): the
clause-(b) data source — the C0 (#412) browsing-context null-stubs are the candidate representation;
if the VM cannot yet observe "browsing context is null" for a document, the clause lands
settings-level-only with the platform-object refinement carved (§8-D3) rather than faked. The
compile gate (:554-560) is *re-keyed verbatim* onto the canonical predicate — note it tests the
**flag clause only** (spec step 3.2 tests the sandbox flag, NOT full scripting-disabled — §2.7-C1;
the comment already says so, keep it).

**⚠ CORRECTION (external review, PR #444 Codex R2)**: (1) the platform-object composition was
relocated to engine-indep `elidex-script-session::scripting` (Layering: the `VmInner` predicate is
now a marshal wrapper reading `HostData` state and delegating); (2) clause (b) hardened with the
**adopt-equivalent tree-root rule** — elidex's insertion path lacks DOM §4.2.3 insert adoption
(`append_child` relinks without re-homing `AssociatedDocument`), so a `DOMParser` node appended
into the bound document kept a stale owner and was wrongly suppressed; the predicate now treats a
node whose composed tree root IS the bound document as having the bound document as its node
document. The missing insertion-adoption is carved as `#11-cross-document-adopt-on-insert`.
R5 surfaced the inserted-then-removed adoption-persistence edge (a node adopted into the active
document then removed should stay ENABLED, since DOM §4.2.3 adoption is sticky): the gate uses the
live composed-tree-root proxy and fails closed on that edge; a spec-correct fix needs DOM adoption,
out of S5-4a scope → deferred to `#11-cross-document-adopt-on-insert`.

**Tests**: plugin truth tables (None/Some(empty)/each bit); VM: sandboxed-no-allow-scripts doc —
handler attr present → getter yields null + dispatch runs nothing (compile gate); compiled-then-
disabled — handler compiled while enabled, then dispatch under clause-(b) conditions → suppressed
(step 1) — this row is conditional on §9-Q3 resolving "representable"; if D3 carves instead, the
test accompanies D3, not 4a; non-handler `addEventListener` listeners NOT suppressed (step 1 gates
event *handlers*, not all listeners — the processing algorithm is handler-specific).

**Edges**: E1 (compile vs invoke predicates differ), E5 (parser `scripting_disabled` non-conflation
— assert untouched), boa delegation compile-only.

### §5.2 S5-4b — iframe origin/flags installed BEFORE initial scripts

**Scope**: reorder the in-process iframe paths so `set_sandbox_flags` + `set_origin` +
`set_iframe_depth` precede the first eval — on **all four** in-process shapes: `load_iframe_from_url`
(:169→:189 inversion), the srcdoc / about:blank / no-src `load_iframe` arms (:46/:55/:66 → :75), and
`blank_entry` (:316-328). The OOP path (:288-300) is the template and stays untouched. Mechanically
this moves the security installs from `make_in_process_entry` to between pipeline-construction and
`run_scripts_and_finalize` (exact plumbing — pass origin/flags into the build call vs split the
entry constructor — is impl detail; the reviewed invariant is the ORDER).

**Tests**: shell integration — sandboxed (no `allow-same-origin`) iframe whose initial script reads
its origin observes `"null"`/opaque, not the URL tuple. PRIMARY oracle = the **storage-bucket
sentinel** (opaque origin → sentinel bucket, `storage.rs:103` — observable in-process); the
postMessage-origin oracle only if an in-process iframe→parent delivery site exists (none found at
HEAD — OOP forwarding only). Unsandboxed iframe unchanged; srcdoc + blank paths covered; OOP path
regression-pinned.

**⚠ CORRECTION (S5-4b impl contact, 2026-07-02)**: the storage-bucket sentinel cite
(`storage.rs:103`) is VM-side; the shell's live engine pre-flip is boa, whose localStorage keys off
`current_url`-derived `cached_origin` (not `set_origin`) — so the storage sentinel is unobservable
in shell until the S5-6 flip. Delivered oracles instead: the boa eval gate (sandboxed script does
not run) + the WS mixed-content gate (opaque origin passes where tuple throws), both falsified by
HEAD-order simulation. **Registered S5-6 flip deliverable**: add the storage-bucket-sentinel shell
test once the VM is live.

**Edges**: E2 (origin×script ordering — THE slice), E6 (S5-4d test fidelity dep). No engine change;
no new state.

### §5.3 S5-4c — VM sandbox method gates + modals + window.open

**✅ LANDED (2026-07-04)** — **⚠ POST-LANDING DESIGN CORRECTION (Codex R1+R2 convergence)**: the
§4.3.2 **two-queue** back-channel (`pending_open_tabs` + `pending_frame_navigations`) was replaced by
a **single ordered queue** `pending_window_open: VecDeque<WindowOpenIntent>` (`WindowOpenIntent =
Popup(OpenTabRequest) | NamedFrame(NamedFrameNavigation)`), draining via ONE
`HostDriver::take_pending_window_opens` routed by ONE shell `route_window_opens`. Two independent
queues were the root defect behind two Codex findings: R1 (an async pump drained only one queue → a
named `window.open` from a timer/postMessage stranded forever) and R2 (cross-call order lost → a later
`_blank` surfaced before an earlier named MISS). A single ordered FIFO dissolves both (call order
preserved by construction; one drain method makes "drain only one queue" unrepresentable) and satisfies
the memo's own §4.3.2 "multiple `window.open` calls in one task must all surface" + CLAUDE.md
"One issue, one way". The §4.3.2 two-queue design text (above, incl. its `take_pending_open_tabs()` /
`take_pending_frame_navigations()` / `route_frame_navigations` seam names) is SUPERSEDED by this note —
those intermediate names do NOT exist in the delivered code; the real seams are
`take_pending_window_opens` / `route_window_opens` over the one `WindowOpenIntent` queue. — implemented
as spec'd below with these impl-contact refinements (stated in terms of the FINAL delivered API):
(1) **App-mode is drain-AND-DROP, not routing** (refines §4.3.2's "same trait call" claim): inline
interactive `app/navigation.rs` drains the ordered window.open queue (`take_pending_window_opens`) but has
no new-tab facility (`ChromeAction::NewTab` is a threaded-mode-only no-op inline) and no iframe registry,
so it drains-to-drop for leak-prevention only; real routing lives in the content-thread
`process_pending_actions`. (2) All `window.open` routing (popup + named, in call order) is ONE
`pub(crate) route_window_opens` in `content/navigation.rs` (Popup → `OpenNewTab`; NamedFrame HIT → ungated
`navigate_iframe`; MISS → `OpenNewTab` iff `aux_nav_allowed`), shared by both drain pumps; the MISS-gate
is unit-testable on synthesized intents (the boa path can only produce `aux_nav_allowed: true`).
(3) Drain sites re-parse the channel `url: String` into `url::Url` (VM/boa resolve to absolute
pre-enqueue; parse-failure skips). (4) boa's `JsRuntime` gained ONE engine-agnostic
`take_pending_window_opens` wrapper concatenating its two private bridge drains (popups then named —
best-effort order matching boa's prior effective order; `aux_nav_allowed: true` by construction, entry
gate already passed) so the shell drain is signature-identical to `HostDriver` and the S5-6 flip swaps
the runtime type without touching it (E4). (5) The link-top-nav re-key end-to-end regression is pinned at the predicate seam only — no
shell click-simulation harness exists and blocked/allowed both terminate in `send_display_list`
(indistinguishable on the channel); gap documented in-test. `event_handlers.rs` = 997 lines post-edit
(under 1000, no restructure). **Post-review refinements** (pre-push `/code-review` + `/elidex-review`):
(6) **Empty-url urlRecord is threaded as `Option`, NOT pre-resolved to about:blank** (§7.2.2.1 steps
3-4/15.3/16.1 — webref-verified): `window.open("", "_self")`/`_top`/`_parent` and a named-target HIT are
NO-OPs (an existing navigable is navigated only for a non-null urlRecord, step 16.1), while a `_blank`/popup
or named-MISS *new* navigable defaults to about:blank (step 15.3). This corrected a real bug in the first
draft (empty-url `_self` destroyed the current document). `NamedFrameNavigation.url` is `Option<String>`
so the existing-vs-new choice stays the shell's (resolved at frame-tree lookup). A whitespace-only url is
NOT empty (JS-empty check, spec step 4) — it URL-parses to the document URL, a deliberate divergence from
boa's non-spec `trim()` guard. (7) `HostDriver::modals_allowed` was NOT added to the session trait after
all (deviation from the §5.3 scope-1 "session-trait surface" line): the modals gate is entirely
engine-internal (the `alert`/`confirm`/`prompt` natives), the shell has no modal gate to drive, so — like
`scripts_allowed` (also engine-internal, off the trait) — it lives only as `HostData::modals_allowed`;
a trait method would be dead surface. (8) `window.rs` crossed 1000 (784→1021) via the four natives → the
dialog/open group was split into the sibling `vm/host/window_dialogs.rs` (touch-time cohesion seam;
window.rs back to 791). (9) **Single ordered `window.open` queue** (Codex R2 root fix — see the
DESIGN CORRECTION note above): the two back-channels collapsed into one `pending_window_open:
VecDeque<WindowOpenIntent>`, one `take_pending_window_opens` drain, one `route_window_opens` routing
home — call order preserved by construction across popup + named intents, and the async-pump drain gap
(R1) becomes unrepresentable. (10) The queue's overflow spam-clamp (`MAX_PENDING_WINDOW_OPENS`, drops
the NEW intent past the bound) is pinned by test. (11) **Codex R5 drain-correctness triple**: (a)
`process_pending_actions` now drains the (non-destructive) window.open queue BEFORE the pipeline-replacing
own-context navigation — a same-turn `window.open('_blank'); window.open('_self')` no longer strands the
popup (the two channels stay distinct effect classes: own-context nav = last-wins `pending_navigation`
shared with `location.*`; other-context open = FIFO `pending_window_open`, so no queue unification needed —
the bug was drain ORDER); (b) `process_pending_actions` now returns `true` only for an OWN-CONTEXT navigation (`location.*` /
`window.open('_self'/'_top')` / history), NOT for `window.open` tab/iframe opens — those act on OTHER
contexts and are applied but must not suppress a co-located link's default `<a href>` navigation (a browser
opens the popup AND navigates the link). `route_window_opens` returns
`WindowOpenOutcome { navigated_iframe, any_effect }`; `any_effect` gates only the display-list flush (a no-op
MISS / empty-url HIT / blocked scheme sends nothing), so Codex F1's no-op-suppresses-link case AND the
broader real-popup-suppresses-link case are both closed; (c) all window.open tab/iframe URLs route through the shell's `resolve_nav_url` chokepoint, so a
`javascript:`/`vbscript:` `window.open` is blocked like link/location navigation. **CLOSES
`#11-vm-sandbox-method-gates-and-modals`.**

**Scope**: (1) `ALLOW_MODALS` predicate + `ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION` bit + token
parse + `top_navigation_allowed` (§4.3.3) in `elidex-plugin`; `modals_allowed` VM accessor
(`HostData`) + session-trait surface (parity with `forms_allowed`/`popups_allowed` — the trait gap
§3.1); (2) the four VM natives (marshal-only, §4.3.1); (3) the engine-agnostic open-tab /
named-frame back-channel on the session contract + VM queues + shell drain rewiring (§4.3.2);
(4) the shell link-target top-nav site (`event_handlers.rs:199-214`) re-keyed onto
`top_navigation_allowed(flags, true)`.

**§5.3.1 Modals — spec-faithful headless.** Each native runs the *cannot show simple dialogs*
composition as its step 1: `elidex_plugin::sandbox::modals_allowed(flags)` (pure predicate, §4.1) ∧
the documented UA policy constant (permanent step-4 opt-in), composed at the native — marshal-scale
(§4.3.1; the home is settled: plugin predicate + native-site composition, no session helper).
elidex's UA permanently opts into step 4 ("Optionally, return true") — so
presentation never happens and returns are alert→undefined / confirm→false / prompt→null on BOTH
branches: simultaneously **boa-parity** (§3.2) and **spec-conformant**. The gate is still landed as
a real chokepoint (security-by-structure: a future shell modal surface can only attach behind it);
step 2 (origin vs top-level-origin same origin-domain) is *subsumed* by the permanent step-4 opt-out
— it can never be observed while step 4 always fires first-class, so we do NOT thread top-level
origin to the VM for it (threading = demand-gated with a real presentation surface; noted, no slot —
the step-4 opt-out is conformant on its own).

**§5.3.2 window.open — spec-shaped dispatch (fixing boa's shape), null-returning.** The native is
marshal-only: coerce JsValue args → parse/resolve url (existing seam; failure → SyntaxError per
§7.2.2.1 step 4.2, thrown at the native BEFORE dispatch — boundary marshalling, not a disposition
input, §4.3.1) → resolve target (default `_blank`) → call `window_open_disposition` (§4.3.1) →
enqueue per the result. The disposition, per spec order not per boa: `_self` → `SelfNavigate` →
`NavigationRequest` (NO popup gate — boa's entry-gate-everything :359 is not spec-shaped;
choosing-a-navigable resolves `_self` to currentNavigable before any flag check); `_parent`/`_top` →
`top_navigation_allowed(flags, false)` (script-initiated = no activation, §4.3.3) → `TopNavigate` →
`NavigationRequest`, else `Blocked`; named → `Named { aux_nav_allowed: popups_allowed(flags) }` —
HTML §7.3.1.7 step 3 **snapshots** the source's sandboxingFlagSet, so the verdict is taken at call
time and enqueued on `NamedFrameNavigation { name, url, aux_nav_allowed }` (§4.3.2). The shell's
existing-frame lookup stays shell-side: on HIT, `navigate_iframe` — **ungated**, spec-correct, but
the premises must be recorded: the spec's hit path DOES carry sandbox-relevant conditions — *find a
navigable by target name* (§7.3.1.7) consults *allowed by sandboxing to navigate* at its match
steps (currently "optionally", tracked by whatwg/html#10849), and window-open-steps step 16's
navigate enforces §7.4.2.4 for real. Both are discharged today only because `find_iframe_by_name`
(`content/iframe/lifecycle.rs:270-287`) searches only the current document's iframes → the source
is an ancestor of the target → §7.4.2.4 step 2 ("If source is an ancestor of target, then return
true") holds unconditionally. If S5-8/B1 widens the named lookup beyond descendants, "HIT ungated"
must be revisited (clause registered in the §8-D1 fold). On MISS, promote to
`OpenNewTab` ONLY if `aux_nav_allowed`, else drop (spec: "may report to a developer console") — the
promotion EXISTS at HEAD (`content/navigation.rs:227-229`, §3.2) and left ungated it becomes a
sandbox bypass at flip. The named hit/miss asymmetry with `_blank` (enqueue-time gating) is the
spec's own structure: the aux-nav flag gates only the create-a-new-traversable case of step 8.
`_blank`/popup → `popups_allowed` gate → `OpenTab` → enqueue `OpenTabRequest`, else `Blocked`.
Features string ignored (boa parity; tokenization → §8-D1). **Return null on every path** (WindowProxy
= S5-8). Blocked popup = silent null (spec: "may report to a developer console").

**Tests**: per-gate × per-target matrix (flags None / empty / each relevant bit): open-`_blank`
blocked/allowed → queue empty/populated; `_top` with/without both top-nav bits; `_self` never
popup-gated; named-MISS matrix — sandboxed no-`allow-popups` doc, `window.open(url, "nonexistent")`
→ NO `OpenNewTab`; with `allow-popups` → promoted to `OpenNewTab`; named-HIT stays ungated either
way; modals return-value triple on both branches; shell drain integration (VM engine drives
`OpenNewTab` / `navigate_iframe`); trait-conformance test that boa path still drains via its own
channels (until S5-6).

**Edges**: E3 (target×flag×activation matrix), E4 (back-channel strangler bounded by flip), E8
(batch-bind: flag reads happen inside the bound window — umbrella §4 coupled invariant; the natives
read bound HostData by construction).

### §5.4 S5-4d — fetch opaque-origin isolation

**Scope**: the §4.4 type unification (`Request.origin: Option<SecurityOrigin>`) + re-keying the
three VM dispatch sites onto `document_origin()` + serializer swaps (header/preflight/CORS) + the
cookie-equality gate. Soft-dep on S5-4b: without it a sandboxed iframe's *initial* scripts still
fetch under the pre-override origin (the ordering bug), so the end-to-end sandboxed-fetch test only
holds after 4b; the unit surface (net crate) is independent.

**Tests**: net unit — `should_attach_cookies` matrix (tuple-same / tuple-cross / opaque / None) ×
credentials mode; `Origin: null` header for opaque initiator on cors-tainted + non-GET/HEAD;
preflight serialization; VM — sandboxed doc `fetch()` produces request with opaque origin (identity-
stable across two fetches from the same doc: same `Opaque(u64)`); `mode: SameOrigin` request from
opaque doc rejected; redirect-taint still yields `"null"` for tuple origins (E7).

**Edges**: E6, E7; embedder `None` carve-out documented as a trust boundary (unchanged behavior,
re-asserted in tests).

### §5.5 S5-4e — dedicated-worker port MessageEvent origin = ""

**Scope**: drop the origin stamping on the dedicated-worker channel both directions
(`worker.rs:499-522` + `worker_thread.rs:243-247` + messageerror sites :122-124,185): the
`MessageEvent` for worker `message`/`messageerror` is built with `origin = ""` (§2.6). Ideal:
**delete the `origin` field from the worker channel messages** (`WorkerToParent::PostMessage` /
`ParentToWorker::PostMessage`) — dead payload once unread ("dead code deleted"); boa shares the
channel types → mechanical field-removal on the boa sites (light-touch-compatible; if the channel
types turn out boa-owned duplicates, VM-side only and boa keeps its copy until S5-6). **Do NOT
touch**: window.postMessage (§9.3, origin required, correct today) and the SW channel
(`ExtendableMessageEvent` origin spec-REQUIRED, SW §3.1.5 — §2.6; verify-only test pin).

**Tests**: VM worker round-trip — `onmessage` event `.origin === ""` both directions; messageerror
origin `""`; window.postMessage origin regression-pinned (`document_origin().serialize()`); SW
message path untouched (pin current behavior + a comment citing SW §3.1.5).

**Edges**: E9 (worker channel disjoint from all other slices — zero intersection, why it stays a
micro-PR).

---

## §6 Edge matrix (review-tail pre-empt; slices × invariant axes)

Axes = the umbrella's four (sandbox flags / origin / scripting-disabled / fetch isolation) + the
cluster-local cross-cuts surfaced by this memo.

| # | Edge (intersection named) | 4a | 4b | 4c | 4d | 4e |
|---|---|---|---|---|---|---|
| E1 | **compile-gate ≠ invoke-gate predicates** (§8.1.8.1 step 3.2 = flag-only vs processing step 1 = full §8.1.3.4) — wiring both to ONE predicate would be spec-wrong in either direction | ✔ owns | — | — | — | — |
| E2 | **origin × initial-script ordering** (`set_origin` contract; opaque origin must be observable to the FIRST eval) | — | ✔ owns | — | reads (E6) | — |
| E3 | **sandbox flags × nav target × transient activation** (5-way target dispatch × 3 gate kinds × 2-flag top-nav; conservative no-activation default for script paths; **named-miss × snapshot verdict**: the aux-nav verdict is snapshotted at enqueue per §7.3.1.7 step 3 and consumed at the shell miss-branch — flag changes between call and drain must NOT re-evaluate. The snapshot discipline covers the FLAG axis; the NAME axis retains a pre-existing shell-architecture deviation: spec resolves HIT/MISS synchronously at call time, elidex resolves at drain time in `process_pending_actions` — a same-task insert/remove of a named iframe can flip HIT↔MISS vs spec; no sandbox bypass either way since the verdict rides the payload) | — | — | ✔ owns | — | — |
| E4 | **back-channel strangler bound** (VM channels on the trait + boa private drains coexist ONLY until S5-6 deletes boa — force-resolved, S5-3b §0.3 shape; a drain left boa-only at flip = the forbidden form) | — | — | ✔ owns | — | — |
| E5 | **scripting-disabled ≠ parser `scripting_disabled`** (§8.1.3.4 vs §13 noscript flag — name collision, zero shared semantics) | ✔ guard | — | — | — | — |
| E6 | **fetch isolation × ordering fix** (4d's end-to-end sandboxed-fetch oracle is only truthful post-4b; unit oracle independent) | — | ✔ | — | ✔ | — |
| E7 | **opaque origin × redirect-taint** (two independent `"null"` producers in *serializing a request origin* — must compose, not shadow) | — | — | — | ✔ owns | — |
| E8 | **flag-read × batch-bind window** (umbrella §4: sandbox reads come off BOUND HostData; the new natives/gates read inside the bracket by construction — no pre-bind gating) | ✔ | — | ✔ | ✔ | — |
| E9 | **worker channel disjointness** (no shared state with any other slice — parallel-safe) | — | — | — | — | ✔ |
| E10 | **B1 storage-home neutrality** (predicates pure over the VALUE; no new per-VM per-entity fact; queues = event-queue shape) | ✔ | — | ✔ | ✔ | — |
| E11 | **boa light-touch boundary** (delegation/type-adjust = mechanical, allowed; feature mirroring = forbidden; boa modal/open behavior is the parity BASELINE only) | ✔ | — | ✔ | ✔ | ✔ |
| E12 | **modals gate observability** (permanent step-4 opt-out makes the gate behaviorally invisible TODAY — the test oracle is the chokepoint's return-shape + structure, not a UI diff) | — | — | ✔ owns | — | — |

**Densest slice = S5-4c** (E3+E4+E8+E11+E12) — as predicted by the umbrella; it is why 4c is the one
slice with a peel-off hatch (§0).

---

## §7 Test strategy (supported-surface declaration)

Boa stays the live shell engine throughout S5-4, so the oracles are engine-level + targeted shell
integration (same posture as S5-3a/b/c):

- **`elidex-plugin` unit** (engine-indep, the canonical rules): token-parse table (incl. new token,
  both-tokens conformance note), per-predicate truth tables, top-nav 2×2×2
  (flags-combo × activation).
- **VM integration** (`cargo test -p elidex-js --all-features`): per-slice suites per §5 (gate
  matrices, handler compile/invoke suppression, worker origin round-trip, fetch request-shape).
- **`elidex-net` unit**: cookie-attach matrix, Origin-header serialization, preflight/CORS with
  `"null"`.
- **Shell integration** (`cargo test -p elidex-shell`): iframe ordering (4b), drain wiring (4c),
  link-gate re-key regression (4c).
- **WPT subset declaration**: the cluster's supported surface maps to
  `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_*`-family semantics and
  `html/webappapis/scripting/events/` compile-gate cases — tracked as engine-independent equivalents
  (elidex-wpt harness scope judged at impl; the unit/integration coverage above is the regression
  gate per "Supported-surface testing").
- Per-PR workflow: plan-verify grep against HEAD → impl in isolated worktree → `/pre-push` →
  `/external-converge` → squash merge (umbrella §11).

---

## §8 Deferred carves (+ audits; cap ≤3 — actual: 4c = 1 new (D2) + 1 fold-append (D1); 4a = 1 new (D4, external-review predicate-hardening) [D3 NOT created — §9-Q3 resolved representable]; others 0)

- **D1 (FOLD — no new slot minted)**: popup sandboxing-flag-set propagation (§7.1.5 propagate-flag
  + choosing a navigable step 8, "create a new top-level traversable" case, substep 9), *one
  permitted sandboxed navigator*, features tokenization (noopener/noreferrer). These facets FOLD
  into the existing `#11-browsing-context-model-window-open-postmessage` slot — appending them to
  that slot's ledger text at 4c landing is a registered landing deliverable. (Named-target-miss →
  popup promotion is NOT in the fold: it is HANDLED by the §5.3.2 snapshot-verdict gate.) The fold
  ALSO carries the §5.3.2 named-HIT revisit clause: "HIT ungated" rests on the descendant-only
  `find_iframe_by_name` lookup (source = ancestor of target → §7.4.2.4 step 2 discharges); if
  S5-8/B1 widens the named lookup, BOTH `navigate_iframe` callers riding that discharge must gate
  via §7.4.2.4: the drain path (`content/navigation.rs:226`) AND the link-click named-target arm
  (`content/event_handlers.rs:219-229`, `navigate_iframe` at :223 — user-gesture-only; its MISS
  falls through to self-navigation, not popup promotion, so no gap today).
  **Audit**: spec-core? yes (§7.3.1.7/§7.1.5) but requires an auxiliary-browsing-context OBJECT that
  does not exist pre-S5-8/B1 — not implementable without the model; one-way? yes — the S5-4c
  dispatch leaves the `_blank`/named arms as the exact insertion points; pragmatic-debt? no (nothing
  faked — behavior is "no popup object exists", boa-parity); repeat-signal? this is the S5-8
  boundary the umbrella already drew (Q4). **Trigger**: S5-8 / B1 window.open program.
  **Re-eval**: at S5-8 plan-memo (fold note carries no own calendar date — the host slot's cadence
  governs).
- **D2 `#11-transient-activation-tracking`** (carved by S5-4c): HTML user-activation model (transient
  activation state, consume-on-use) as a real tracked fact; today the top-nav/popup activation input
  is per-call-site static truth (§4.3.3 — script=false, link-click=true). **Audit**: spec-core? yes
  (HTML §6.4 family; also feeds the popup-blocker option in choosing-a-navigable); one-way? yes —
  `top_navigation_allowed` takes the bool parameter, tracking swaps the argument source only;
  pragmatic-debt? the conservative default under-permits (sandbox never bypassed; a
  user-activated script `window.open('_top')` inside `allow-top-navigation-by-user-activation` is
  wrongly blocked — fail-closed, acceptable interim); repeat-signal? activation will recur at
  fullscreen/clipboard/autoplay gates. **Trigger**: first user-activation-gated API beyond top-nav,
  or S5-8. **Re-eval**: S5-8 plan-memo, calendar backstop **2026-09-30**.
- **D3 `#11-scripting-disabled-platform-object-clauses`** — **NOT CREATED (§9-Q3 resolved
  "representable" at 4a kickoff, 2026-07-02; the clause shipped in 4a)**. Original conditional
  carve, kept for the record (CONDITIONAL — carved by S5-4a **only
  if** §9-Q3 resolves "not yet representable"): §8.1.3.4 platform-object clauses (b)/(c)
  (browsing-context-null) refinement of the step-1 gate. **Audit**: spec-core? yes (§2.2); one-way?
  yes — the step-1 gate composes `settings ∧ platform_object(target)`, the clause slots in;
  pragmatic-debt? settings-level-only under-gates exactly the detached-document edge; repeat-signal?
  C0 (#412) carved the browsing-context null-stub family this rides on. **Trigger**: C0
  null-stub → real browsing-context-null representation landing. **Re-eval**: at the
  `#11-browsing-context-state-ecs-components` / B1 disposition, calendar backstop **2026-10-31**.
  If Q3 resolves "representable now", D3 is not created and the clause ships in 4a.
- **D4 `#11-cross-document-adopt-on-insert`** (carved by S5-4a — external-review PR #444 Codex
  R2/R4/R5 predicate hardening; a DIFFERENT, newly-surfaced carve than the not-created D3): elidex
  does not implement DOM §4.2.3 insertion adoption (`AssociatedDocument` is not mutated on
  cross-document insert), so the §8.1.3.4 clause-(b) gate (the scripting-disabled-for-a-platform-object
predicate, invoked at §8.1.8.1 step 1) uses a composed-tree-root
  effective-document proxy — correct for a node's live tree position but a best-effort approximation
  for the whole class of nodes **MOVED between documents** (adoption is sticky yet elidex has no
  "was-adopted" state). The imperfect facets span BOTH directions: **over-suppress (fails CLOSED)** a
  node adopted into the active document then removed, and **under-suppress** a live main-document node
  appended into a detached foreign (DOMParser-built) subtree — plus their reverses. No single
  tree/owner-document rule closes all facets (each refinement trades which facet is wrong), so the
  proxy is intentionally **not refined per-facet**; all facets are bounded here. **Audit**: spec-core?
  yes (DOM §4.2.3 adopt on insert); one-way? yes — implementing
  adoption re-homes `AssociatedDocument` and the proxy collapses to a direct `owner_document` read;
  pragmatic-debt? the fail-closed edge is exotic (a DOMParser/foreign-doc node inserted-then-removed,
  then its handler dispatched) and safe-direction for a security gate; repeat-signal? this defect
  underlies R2/R4/R5 (three predicate rounds) AND the sibling `owner_document` consumers
  (`selection.rs`, `document_base.rs`, the `ownerDocument` getter, `href`/`baseURI`) noted in the
  R2/R4 same-pattern audits. **Trigger**: a DOM adoption implementation, or a WPT/site exercising
  DOMParser-node adoption + event dispatch. **Re-eval**: **2026-10-31**.

- **Slot-trigger disposition (existing slots, no new carve)**:
  `#11-storage-opaque-origin-securityerror` + `#11-cookie-opaque-origin-securityerror` (§1.3) —
  both slots' trigger ("sandboxed-iframe opaque-origin plumbing lands") is SATISFIED by S5-4b/4d
  landing. A promotion audit at that landing is a **registered landing deliverable**: promote if the
  about:blank origin-inheritance coupling is resolved by then, else re-defer with an updated date.
  NOT folded into this cluster — genuine cross-PR boundary (inheritance semantics = M4-13 infra,
  per the slots' own ledger re-scope; the umbrella's S5-5 row does not own about:blank
  inheritance).

**Not carved (dispositioned in-memo, no slot)**: shell modal UI (spec-conformant step-4 opt-out,
§5.3.1); top-level-origin threading for cannot-show step 2 (unobservable behind step 4, demand-gated
with a presentation surface); remaining §7.1.5 flags (pointer-lock / automatic-features /
document.domain / downloads / custom-protocols / orientation-lock / presentation — each gates a UA
feature elidex does not implement; nothing to gate, re-audited when the feature lands); CSP sandbox
directive (§1.3); SW message origin (already-correct polarity, §2.6). Defer-ledger reconciliation
(closing the 5 covered slots + registering D1/D2/±D3) is a landing deliverable of the respective
slices, not a side-effect.

---

## §9 Open questions for `/elidex-plan-review`

- **Q1 (Q2-confirmation, PM)** — **✅ RESOLVED at plan-review (§0): ACCEPTED** (5-slice structure +
  S5-4c in-memo; post-F1/F2 the §5.3.2 design reaches disposition-enum depth — a dedicated memo
  would add nothing). Original question, kept for the record: accept the 5-slice structure + S5-4c
  staying in this memo (§0)?
  The alternative (4c peels into its own memo after this review) costs one extra review cycle and is
  the right call only if the §5.3 matrix is judged under-specified.
- **Q2 (predicate home)**: confirm `elidex-plugin` over the umbrella's dom-api/script-session binary
  (§4.1). The DAG argument looks decisive (net-reachability), but if plan-review sees the sandbox
  module outgrowing plugin's vocabulary-crate role (e.g. once activation tracking lands), the
  fallback is a dedicated engine-indep `elidex-security` crate — judged premature now (one module,
  one concern; a crate for 6 functions is structure without load).
- **Q3 (clause-(b) representability, 4a)** — **✅ RESOLVED at 4a kickoff (2026-07-02):
  representable now.** Clause (b) ships in 4a via the bound-document proxy: the VM models exactly
  ONE top-level browsing context whose active document is the bound `document_entity`, so "node
  document's browsing context is null" = `EcsDom::owner_document(target)` (self for a Document
  node) resolves to a document ≠ `HostData::document_entity_opt()` — the same single-BC query
  shape as `native_node_get_is_connected` (`node_proto.rs`). D3 is NOT carved. Two caveats,
  recorded in-code at the predicate (`VmInner::scripting_disabled_for_platform_object`):
  (a) detached-iframe documents (the spec's motivating clause-(b) case) cannot arise in the
  single-browsing-context VM model — moot/unreachable, not un-gated; (b) `<template>` content
  nodes are false NEGATIVES (owner resolves to the main document) until
  `#11-template-contents-owner-document` gives template contents a real inert owner document —
  the gate is correct over the `AssociatedDocument` data and self-heals when that slot lands.
  Original question, kept for the record: can the VM observe "node document's browsing context is
  null" today via the C0 (#412) null-stubs, or does D3 carve? Impl-verify at 4a kickoff — the memo
  deliberately leaves both paths specified (§5.1.2, §8-D3).
- **Q4 (`_parent` vs `_top` routing fidelity, 4c)**: boa routes BOTH to `set_pending_navigation`
  (i.e. navigates the iframe's own context in today's single-navigable shell model, §3.2). S5-4c
  keeps that routing (boa parity) while fixing the GATE (2-flag). Is parity-routing acceptable
  pre-S5-8, or must `_parent`/`_top` thread a target-navigable discriminator through
  `NavigationRequest` now? Lean: parity (the discriminator is browsing-context-model work = S5-8;
  adding it gate-less would be dead plumbing) — but this is a judgment call plan-review should
  ratify.
- **Q5 (worker channel type ownership, 4e)**: if `WorkerToParent`/`ParentToWorker` are shared
  types, field deletion touches boa (mechanical — allowed?); if boa-owned duplicates exist, VM-only.
  Grep at impl; both paths light-touch-compatible (§5.5), flagged so the review agrees on the
  boundary.
- **Q6 (S5-4b plumbing shape)**: pass origin/flags into pipeline-build vs split
  `make_in_process_entry` into install-then-finalize halves — impl detail, but the reviewer should
  confirm the invariant statement ("security installs precede first eval on ALL in-process paths,
  OOP template") is the right review surface rather than a specific plumbing.

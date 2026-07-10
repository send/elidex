# S5-6 — THE FLIP: boa→VM cutover + wholesale `elidex-js-boa` deletion

Per-PR plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`, §5 row
"S5-6 THE FLIP + boa deletion"; §7 edge-matrix S5-6 column; §4 the batch-bind corner). **Anchor = the
ideal end-state** — ONE engine, no strangler, no dual-engine moment
(`feedback_plan-memo-anchor-on-ideal-not-incremental`).

S5-6 is the umbrella's **keystone join point**: all gates are MET — S5-1 (#420 DOMParser/XMLSerializer),
S5-2 (#423 VisualViewport/cookieStore/Screen), S5-3 keepalive COMPLETE (#440/441/442, hard pre-flip gate
PASSED), S5-4 sandbox COMPLETE 5/5 (#444/446/447/448/445), S5-5 nav/history COMPLETE (#449/451/452),
C3 device facts landed (#415). What remains is exactly this plan — **two PRs under this one memo**
(§0.1): S5-6a lands the flip-inert capability prereq (the six missing VM/trait pieces + the
re-collection seam), then S5-6b swaps the shell's engine type, wires the batch-bind brackets, deletes
the boa-feeding CSSOM shadow-sync, converges every boa-coupled surface onto the
`HostDriver`/`ScriptEngine` trait or shell-owned state, and **deletes the crate** (39,193 LoC /
121 files). S5-6b crosses **batch-bind safety × CSSOM-truth swap × storage-emit × history-publish ×
SW-thread × the whole shell test oracle** (umbrella §7) — the highest-blast PR of the program, hence
this mandatory `/elidex-plan-review` before any impl (CLAUDE.md "Edge-dense work").

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE impl. §0 resolves umbrella Q1 (bundling) and
> proposes the A2c peel; §5 is the deliverable ledger (the load-bearing section — every migration
> surface enumerated with re-verified cites); §8 is the acceptance gate (umbrella §10-Q6, committed
> in §11); §11 carries the open questions.

> **Umbrella Q1 — RESOLVED by this memo (open question with a lean, not a pre-decision)**: the
> **flip + crate deletion stay ONE PR (S5-6b)** — splitting deletion from flip would leave a
> strangler dual-engine moment (One-issue-one-way); the deletion is the trivial tail of "nothing
> references boa anymore" AND is itself the E4 strangler audit (§8). Round 2 additionally peeled a
> **flip-inert capability prereq (S5-6a)** in front of it (§0.1) — that peel does not touch Q1's
> inseparability: S5-6a adds dormant capability only, and flip+deletion remain atomic within S5-6b.
> This resolution is ratified by this memo's plan-review.
>
> **Binding umbrella pre-decisions (inherited, not re-litigated):** (2) **boa-deletion is wholesale crate removal** — no pre-cleanup, no boa fixes beyond
> CI-green-minimum (`project_boa_runtime_deletion.md`, `feedback_boa-findings-light-touch`). (3) **No
> per-VM-side-store → component migration** (umbrella §0.1; B1 agent-scoped World, post-S5, PR #434).
> (4) **S5 ships compat-only**: the flip passes `BrowserCompat` as the single live `EngineMode` value
> (umbrella §6; `#11-async-core-storage-cookiestore` stays a parallel program — neither blocks the
> other). (5) **VM strictly ≥ boa at flip time** — with the ruler **premise-corrected** (round-2
> review, §8-1): the parity audit's §A regression set is itself under-enumerated (it missed
> `element.animate`/WAAPI entirely — no row, no deliberate-exclusion record — and its own caveats
> admit grep misses), so the effective flip-time oracle = audit §A **+ this memo's §3.4 B-sweep +
> the full shell suite**, with `#11-web-animations-element-animate` (B22) the ONE documented
> exception (S5-7, first post-flip cohort). The §8 acceptance gate runs that composite oracle.

All file:line cites grep-verified against this worktree at HEAD `7b76d722` (2026-07-10, the S5-5c
merge). Every spec § / anchor webref-verified 2026-07-10 (`.claude/tools/webref heading --exact` /
`dfn`); §2.3 records the corrections where the task framing's cite or claim was imprecise.

---

## §0 Scope resolution — TWO PRs under this one memo, what peels

### §0.1 The decomposition (round-2 plan-review resolution): S5-6a prereq → S5-6b THE FLIP

The §3.4 sweep surfaced **six genuinely missing VM/trait capabilities** (+1 test-oracle accessor,
G3-iii). The umbrella's own assignment rule (a) — *FLIP-preconditions land BEFORE the flip* — puts
that class in its own PR, so this plan is **TWO PRs under this ONE memo** (all designs reviewed here;
the S5-4c E4 precedent covers the bounded trait+boa-private coexistence between them):

**S5-6a "flip-inert capability prereq"** (lands FIRST, boa stays live; narrowly additive, no separate
plan-review — it implements this memo's §4.2/§4.3 designs verbatim):
- The VM+trait halves of the 6 ADDs: B3 storage-change drain, B4 `install_web_storage`, B6 IDB
  versionchange emit drain, B13 pending-focus, B16 parent-message queue, B21 IDB versionchange
  deliver (§4.3.2).
- `elidex-dom-api::collect_document_stylesheets` + the per-owner `CollectedStylesheet` component
  cache (§4.2).
- The `extract_inline_scripts` unification in `elidex-navigation` **including the shell call-site
  swap** (§4.3.7/H8: 6a lands the entry AND points `pipeline.rs:444,528,614` +
  `content_iframe_security_tests.rs:396` at it — boa-compatible output makes it a live-oracle 6a
  change; boa's `extract_scripts` export goes caller-less and dies with the crate in 6b).
- The VM console-capture test-oracle accessor (§3.4-B26, the G3-iii disposition).
All flip-inert (no live shell consumer until S5-6b) → VM/per-crate tests are the oracle.

**S5-6b "THE FLIP"** (rebases on S5-6a):
1. **Dep + type swap**: `elidex-shell` drops `elidex-js-boa`, gains `elidex-js` with
   `features=["engine", "compat-webapi"]` (both mandatory — §5 item 1); `PipelineResult.runtime:
   JsRuntime → ElidexJsEngine`; single construction chokepoint swap (§5 items 1–2).
2. **Batch-bind brackets** (umbrella §4 — THE load-bearing edge; §4.1).
3. **CSSOM shadow-sync deletion** (§4.2): shadow deleted; the cascade reads the S5-6a re-collection.
4. **All call-site convergence** (§3.4 B1–B26 rows onto trait/shell-owned state), the SW-thread swap,
   oracle migration + live tests — incl. the three parity-preserving bundles that cannot wait
   (§1.2: `#11-bound-safe-dispatch-dom-aliasing`, `#11-storage-event-broker` VM emit-site,
   `#11-session-history-index-vm-publish`) and the registered flip deliverables carried from S5-4b
   (storage-bucket sentinel), S5-4c (E4 strangler bound), S5-5b/5c (bind-around-history-delivery,
   live popstate/hashchange tests, post-handler re-drain, `(index,length)` publish).
5. **Crate deletion**: `crates/script/elidex-js-boa/` (39,193 LoC, 121 files) + workspace membership +
   prose sweep (§5 item 12). **Q1's flip+deletion inseparability holds WITHIN S5-6b** — the
   strangler-free property is unchanged: S5-6a adds only dormant capability, S5-6b flips and deletes
   in one PR.

### §0.2 PEEL: `#11-vm-host-synthetic-dom-event-dispatch` (A2c) → S5-6-post (recommendation, §11-Qa)

The umbrella §5 bundled the focus-A2c synthetic focus/blur/change slot into S5-6. **This memo proposes
the peel.** Rationale: the flip PR's oracle is *behavioral equivalence with boa* (§8) — every other
bundled item is parity-**preserving** wiring (without it, something boa does today silently stops).
A2c is behavior-**ADDING** new VM host code: the VM currently has **no** synthetic focus-dispatch site —
only the deferred-marker doc at `crates/script/elidex-js/src/vm/host/html_element_proto.rs:256-268`
("Focus EVENT dispatch (`focus`/`focusin`) is deferred: a VM host method cannot fire a DOM event
through the 3-phase listener walk yet … slot `#11-vm-host-synthetic-dom-event-dispatch`") and the
FocusEvent shape at `vm/host/event_shapes.rs:134-137`. **boa also lacks it** — so omitting it at flip
is regression-free, and bundling it would mix a "prove nothing changed" PR with a "new behavior + new
tests" PR, muddying the equivalence oracle exactly where the blast radius is highest. Peel target:
**S5-6-post**, an immediate post-flip fidelity slice (own plan-review if it crosses the dispatch/focus
axes, which it does — the 3-phase-walk-from-a-native primitive is the same one `el.click()` needs).
The slot is an existing carve being **routed**, not a new one.

### §0.3 OUT (own PRs, sequenced around the flip)

- **`#11-session-history-task-queue-model` (S5-5d, flip-gated)**: NOT in the flip PR. **Kicks off
  FIRST after S5-6b lands** (the first post-flip PR) — the flip makes its trigger reachable (boa's
  single-slot back-channel made a multi-action turn unreachable; the VM `Vec` drain + live popstate
  handlers make it real and testable). The flip PR carries only the minimal post-handler re-drain the
  S5-5b memo §9 registered (§5 item 10), explicitly as an interim the D5 model supersedes.
  **S5-7 (`#11-web-animations-element-animate`) lands in the same first post-flip cohort** (closing
  the B22 exception window) — ordering within the cohort: S5-5d first (correctness), S5-7 with it
  (§8-2, §11-Qc aligned). **A2c (S5-6-post, §0.2) queues BEHIND that first cohort** — it is
  behavior-adding fidelity with no exception window to close, so S5-5d/S5-7 outrank it.
- **`#11-keepalive-event-loop-step1-snapshot`** (WS-only, soft target): NOT bundled; its own small
  plan-reviewed PR, non-blocking, can land before or in parallel.

---

## §1 Scope + slot map

### §1.1 What S5-6 is

The shell today runs boa end-to-end: `pipeline.rs:14` `use elidex_js_boa::{extract_scripts, JsRuntime}`,
construction at `pipeline.rs:161` `JsRuntime::with_network(network_handle)`, `PipelineResult.runtime:
JsRuntime` (`lib.rs:440`), and a shell that drives **27 boa-coupled surface rows** (the §3.4 B1–B26 + B23b
inventory: per-turn bridge drains, pre-eval installs, read-backs, the SW thread entry) plus the CSSOM
shadow-sync (§3.3). The VM (`ElidexJsEngine`, `elidex-js` `features=["engine", "compat-webapi"]` —
item 1) implements the **full** `HostDriver` + `ScriptEngine` surface
(`elidex-script-session/src/engine.rs:58-500`, 50 methods — §3.5) with `bind`/`unbind`/`with_bound`
batch brackets (`elidex-js/src/engine.rs:308/336/342`). S5-6 swaps the type at the single chokepoint,
brackets every engine-driving batch, converges every boa-coupled surface onto the trait or shell-owned
state (adding the six genuinely missing pieces — §3.4/§4.3.2), replaces the CSSOM shadow with the
DOM→cascade re-collection (§4.2), swaps the SW thread entry, and deletes the crate. Everything S5-1…S5-5 landed as "flip-inert"
(VM-tested, boa-stubbed) goes live and gets its live-shell test here.

### §1.2 The 3 bundled slots (why each cannot wait)

1. **`#11-bound-safe-dispatch-dom-aliasing`** — the batch-bind safety corner (umbrella §4). The VM's
   own `bind` doc names it: `engine.rs:317-325` "Known soundness gap — event dispatch … Driving event
   dispatch under a batch bracket relies on the bound `*mut dom` and the `&mut ctx.dom` reborrows
   inside the shared `script_dispatch_event` referring to the same `EcsDom` — a Stacked-Borrows
   aliasing violation … The principled fix is a bound-safe dispatch API designed when the shell wires
   dispatch bracketing (S5)". The flip IS that wiring — deferring the fix would ship the aliasing
   violation as the live engine's hot path. Cannot wait by construction. **Layer home**: the
   bound-safe fix lands on the SHARED `script_dispatch_event` seam in `elidex-script-session`
   (engine-indep — the seam every shell dispatch already rides), NOT a VM-private dispatch variant;
   the concrete bound-safe API shape (dispatch reading the already-bound pointers instead of taking a
   fresh `&mut ctx.dom` reborrow) is decided at impl **on that seam**, keeping the one dispatch path
   One-issue-one-way.
2. **`#11-storage-event-broker` (VM emit-site)** — the shell's cross-tab StorageEvent pipeline is:
   originating content thread drains `runtime.bridge().drain_storage_changes()` →
   `ContentToBrowser::StorageChanged` (`content/event_loop.rs:96-104`) → browser broadcast
   (`app/content_messages.rs:228` → `BrowserToContent::StorageEvent`) → receiving content thread
   `dispatch_storage_event` (`content/mod.rs:672`). The VM has **no storage-change out-queue**
   (grep `StorageChange|storage_change` in `elidex-js` + `elidex-script-session` = 0) — without the
   emit drain, every `localStorage.setItem` stops broadcasting at flip = cross-tab storage events
   silently vanish. (The RECEIVE side does NOT vanish — §2.3-C2.) Parity regression; cannot wait.
3. **`#11-session-history-index-vm-publish`** (S5-5b §9 carve, trigger = "the S5-6 flip") — the shell
   publishes only `set_history_length(len)` at **8** nav sites (`app/navigation.rs:172,256,341,695`;
   `content/navigation.rs:310,415,867,886`), but the VM's `pushState` derives the next length from its
   stored `current_index` — a persistent-VM same-document nav then a later `pushState` computes from a
   stale index. The fix is the **whole-surface** `set_session_history(index, length)` publish (trait
   `engine.rs:292`) — flip-inert until the VM index is consulted, i.e. live exactly at the flip;
   publishing index on only some sites would desync the rest. Cannot wait.

### §1.3 Registered flip deliverables carried from sibling memos (verbatim owners)

- **S5-4b** (`2026-07-s5-4-sandbox-enforcement.md` §5.2 CORRECTION): "**Registered S5-6 flip
  deliverable**: add the storage-bucket-sentinel shell test once the VM is live" — the sandboxed
  (no `allow-same-origin`) iframe's localStorage keys the opaque-origin **sentinel bucket**
  (`vm/host/storage.rs:95-111` `current_origin` → `opaque_origin_sentinel`), unobservable under boa
  (boa keys off `current_url`-derived origin). §7 test.
- **S5-4c** (E4): "back-channel strangler bound — VM channels on the trait + boa private drains coexist
  ONLY until S5-6 deletes boa; **a drain left boa-only at flip = the forbidden form**". §8's audit; the
  boa-private drain inventory is §3.4.
- **S5-5b §9**: (a) "the live-shell popstate/hashchange test once the VM is the engine"; (b) "**Bind
  the VM around the history-step delivery**" — the shell calls `deliver_history_step_events` directly
  on the runtime at 4 sites (`content/navigation.rs:451,478`; `app/navigation.rs:297,316`) outside any
  bracket; the VM impl gates on `is_bound()` → post-flip an unbound call silently no-ops. Must be the
  SAME uniform bracket decision that binds MQL/resize/intersection/mutation delivery, NOT a
  history-only bracket; (c) the **scroll-vs-popstate-handler ordering** + **post-handler re-drain**
  folds (§5 item 10); (d) the `(index,length)` publish (§1.2-3).
- **S5-5c §7**: the live-shell state round-trip test (pushState → traversal → `history.state`
  restored) + the cross-doc seed already wired engine-agnostically (`pipeline.rs:224-228`
  `set_history_state` — boa stub, VM lights up at flip).
- **S5-5a/§3.2 (cluster memo)**: "the **boa relative-nav base** divergence is DEFERRED to the S5-6
  flip, which erases it by construction — the VM resolves at enqueue" → add the **both-orders
  regression test** the boa path couldn't have hosted (relative `location.href=` after a same-turn
  `pushState`: resolves against the setter-time URL, in both orderings). §7 test.

### §1.4 Non-goals (bounded out, with owners)

- A2c synthetic focus/blur/change (§0.2 peel → S5-6-post).
- The task-queued session-history model (S5-5d, §0.3).
- `#11-storage-event-mode-aware-delivery` + `#11-enginemode-full-session-threading` — the §6
  mode-plumbing cohort (umbrella §6; needs the async-core 2nd keystone).
- `#11-cookiestore-structured-spec-faithful` — cookieStore fidelity beyond the S5-2 presence-first
  subset (own slot).
- window.open WindowProxy / postMessage browsing-context model — S5-8, B1-bound (umbrella §10-Q4).
- Any per-VM-side-store → component migration (B1, umbrella §0.1).
- **Quota wiring** (`app/sw_coordinator.rs:382-389` `quota_estimate`, `TODO(M4-8.5): use
  OriginStorageManager's QuotaManager`): the flip's SW rewiring CREATES the per-origin
  `OriginStorageManager` state that TODO waited for (§4.3.6), but connecting `navigator.storage.
  estimate()` to the real QuotaManager is a separate storage-accounting concern with its own
  correctness surface — justify-don't-absorb; the TODO's owner is valid and live: **M4-8.5
  "browser.sqlite 永続化 + SW spec gap 補完" is a tracked phase4-plan milestone line**
  (`memory/phase4-plan.md:163,168`), not a dangling tag — no slot needed.
- Fixing anything **in** boa (light-touch: CI-green-minimum only until the deletion commit).

---

## §2 Spec substrate (webref-verified 2026-07-10, source `html`)

### §2.1 Section ↔ title pairs (all lookup-verified)

| § | Title | Anchor / dfn |
|---|---|---|
| §8.1.4.4 | Calling scripts | `#calling-scripts`; dfn *clean up after running script* → `#clean-up-after-running-script` |
| §12.2.1 | The Storage interface | `#the-storage-interface` (the setItem/removeItem/clear ALGORITHM home: no-change-no-fire, broadcast-excluding-originating) |
| §12.2.4 | The StorageEvent interface | `#the-storageevent-interface` (the interface SHAPE only; parent §12.2 "The API" `#storage`) |
| §7.1.5 | Sandboxing | `#sandboxing` |
| §8.1.3.4 | Enabling and disabling scripting | `#enabling-and-disabling-scripting` |
| §7.4.6.2 | Updating the document | `#updating-the-document` (dfn *update document for history step application* — the popstate/hashchange hub, verified in the S5-5 memo §2) |
| §6.6.6 | Focus management APIs | `#focus-management-apis` (the A2c peel's surface AND B13's `window.focus()` — `#dom-window-focus`) |
| §4.12.1 | The script element | `#the-script-element` (the inline-script extraction seam's classification surface, §4.3.7) |
| §4.13.6 | Custom element reactions | `#custom-element-reactions` (the CE reaction queue model, §4.3.1) |
| DOM §4.2.3 | Mutation algorithms | `#mutation-algorithms` (where the spec enqueues CE reactions — *remove* steps 13/14 (disconnected + descendant callback reactions) — and creates transient registered observers — *remove* step 15; §4.3.1) |
| IndexedDB-3 §4.2 | Event interfaces | `#events`; dfn *fire a version change event* → `#fire-a-version-change-event` (the cross-tab versionchange drain/deliver pair, §4.3.2) |

### §2.2 Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §8.1.4.4 Calling scripts | *clean up after running script* (microtask checkpoint) after each script/callback | batch bracket: bind → eval/dispatch/drain → clean-up → unbind; per-callback checkpoints self-contained in `VmInner` (`eval`, `drain_timers` per-timer, `drain_reactions`) | the batch-bind brackets (§4.1) | ✓ (the bracket + checkpoint discipline) | yes (script source) |
| HTML §12.2.1 The Storage interface (+ §12.2.4 interface shape) | `storage` event broadcast: emit at the originating mutation (§12.2.1 setItem step 3.2 "If oldValue is value, then return" + step 7 "Broadcast this…"), deliver on other same-origin Documents (*broadcast a Storage object* step 3 — excludes the originating storage; step-verified via webref body 2026-07-10) | setItem / removeItem / clear; no-change no-fire; never on the originating document | VM emit drain (§4.3.2) + existing engine-agnostic receive path | ✗ (emit-site only; mode-aware delivery = umbrella §6 cohort follow-up) | yes (storage key/value) |
| IndexedDB-3 §4.2 Event interfaces | *fire a version change event* — cross-tab `versionchange` on open connections when another context opens with a higher version | emit (open-with-higher-version) + deliver (other tab's connections) | VM emit drain + receive deliver (§4.3.2, the same new group) | ✗ (drain/deliver wiring; in-VM event machinery landed) | yes (db name / version) |
| HTML §4.12.1 The script element | script type classification for the inline-extraction seam (classic JS vs non-JS `type` skip) | inline classic scripts; external `src` = loader path | `elidex-navigation` unified extraction (§4.3.7) | ✗ (classic inline subset — the pre-existing seam's scope, unchanged) | yes (script source) |
| HTML §4.13.6 Custom element reactions + DOM §4.2.3 Mutation algorithms | CE reaction enqueue for externally-flushed records (the spec model enqueues INSIDE the mutation algorithms — *remove* steps 13/14; the record-driven conversion is a boa-parity interim — §4.3.1) | connected / disconnected / attributeChanged from records; in-bracket native mutations = dispatcher custody | item 6 (`deliver_mutation_records` extension) | ✗ (record-driven interim; algorithm-site enqueue = `#11-ce-reaction-mutation-observer-ordering`) | yes (DOM mutations) |
| HTML §6.6.6 Focus management APIs | `window.focus()` method (`#dom-window-focus`) → the pending-focus flag → shell `FocusWindow` | script-initiated window focus only (no steal-gating change) | B13 (S5-6a ADD + S5-6b converge) | ✗ (flag-relay parity with boa; focusing-steps fidelity = the S2 focus program) | yes (script call) |
| HTML §9.3.3 Posting messages | iframe→parent `postMessage` routing (`#dom-window-postmessage-options`) | `iframe_depth > 0` → parent-directed FIFO; depth 0 → self-delivery | B16 (S5-6a ADD + S5-6b converge) | ✗ (boa-parity context-routed interim; real WindowProxy targeting = S5-8/B1) | yes (message data) |
| HTML §7.1.5 Sandboxing | sandbox flag reads gate method calls | `scripts_allowed` / modals / popups / top-nav — ALL landed (S5-4) | flip only **re-brackets**: flag reads happen off BOUND HostData inside the bracket (umbrella §4 coupled invariant 2) | ✓ (gate-read placement) | yes (sandboxed page script) |
| HTML §8.1.3.4 Enabling and disabling scripting | scripting-disabled gates (S5-4a landed) | compile-gate + invoke-gate | bracket placement only (gates read bound state) | ✓ | yes (event-handler attr) |
| HTML §7.4.6.2 Updating the document | popstate (step 6.4.3, sync) + hashchange (step 6.4.5, queued task) delivery goes LIVE | fragment nav / same-doc traversal / cross-doc seed | bind-around-delivery (§4.3.4) + live tests | ✓ (the S5-5 matrix, now live) | yes (history state object) |

**Breadth verdict**: K = 3 specs (HTML, DOM, IndexedDB-3) over M = 11 rows (counting basis: one row
per spec surface whose behavior this plan wires live or newly implements — bracket/checkpoint,
storage emit+deliver, IDB versionchange, sandbox/scripting gate placement, history delivery, script
extraction, CE record-interim, window.focus, parent postMessage), and every row is **wiring/bracketing of an
already-landed algorithm surface** (S5-1…S5-5 carried the per-step enumeration) — inside the
single-PR band as the umbrella's terminal base case. The breadth of this PR is *mechanical* (the §5
ledger), not spec-algorithmic.

### §2.3 Corrections found during cite re-verification (record for the plan-review spec axis)

- **C1 (stale HTML §numbers in touched shell comments)**: the shell's storage comments cite the
  pre-renumber §11.x — `content/mod.rs:667` "WHATWG HTML §11.2.1", `ipc.rs:375` "§11.2.1",
  `app/tab.rs:41` "§11.2.1", and the VM's own `storage_event.rs:1` "§11.4.2" / `:19` "§11.2.5".
  Replacement mapping follows the §2.1 SHAPE/ALGORITHM split: the four **dispatch/broadcast-semantics**
  comments (`content/mod.rs:667`, `ipc.rs:375`, `app/tab.rs:41`, `storage_event.rs:19`) re-cite to
  **§12.2.1 The Storage interface** (setItem step 7 "Broadcast this…" + *broadcast a Storage object*
  step 3's excluding-the-originating-storage clause — both step-verified via webref body 2026-07-10);
  only `storage_event.rs:1` (the interface shape) re-cites to **§12.2.4 The StorageEvent interface**.
  (The MEMORY `project_html-section-renumber-sweep` tracks the untouched remainder.) The flip rewords
  the cites its diff touches.
- **C2 (task-framing correction — the StorageEvent "deliver site" DOES exist VM-side)**: the framing
  "VM has interface/constructor only with NO deliver site" is imprecise. The **receive** path is
  engine-agnostic and VM-complete: `dispatch_storage_event` (`content/mod.rs:672`) builds
  `EventPayload::Storage` and rides the generic `pipeline.dispatch_event` → `script_dispatch_event`,
  and the VM's UA-dispatch shape table handles `EventPayload::Storage`
  (`vm/host/event_shapes.rs:345,536`). What the VM lacks is the **EMIT drain** (§1.2-2) — the
  `storage_event.rs:17-25` module doc says exactly this ("This file ships only the constructor +
  class definition; the dispatch path is left absent" — where "dispatch" = the cross-VM fan-out,
  "tracked at `#11-storage-event-broker`"). So the flip wires **emit**, brackets **receive**, and
  tests both.
- **C3 (cite drift, minor)**: the umbrella's `engine.rs:303` for the bound-safe-dispatch slot is now
  `engine.rs:317-325` (the `bind` soundness-gap note); the umbrella's `lib.rs:39/433` are now
  `lib.rs:46/440`; the task framing's `pipeline.rs:241` = the `dispatch_lifecycle_events` call
  (verified; the dispatch bodies are `pipeline.rs:305-406`: readystatechange ×2, DOMContentLoaded
  :317, load :344, and `dispatch_unload_events` beforeunload :390 / unload :406). The framing's
  "188 `#[test]`s across 9+ shell test files" is **141** across the 9 named files / **232**
  crate-wide (§7.1).

### §2.4 User-input touch audit

Carried verbatim from umbrella §3.1: the flip processes the **same untrusted page inputs boa already
does** — script source (bracketed + sandbox-gated), markup strings (S5-1 seam), media-query strings
(canonical `elidex-css::media`), cookie/storage writes (existing jar/manager; the StorageEvent payload
was already an untrusted page string crossing contexts via the existing IPC broadcast). **No NEW trust
boundary opens by moving the engine**; the VM host bindings already validate at marshal. The one
security-relevant *improvement* is structural: sandbox/scripting-disabled gates now read **bound**
HostData inside the bracket by construction (umbrella §4), and opaque-origin storage keys the sentinel
bucket (S5-4b, observable + tested at last).

---

## §3 Current-state code map (HEAD `7b76d722`)

### §3.1 Dep + construction chokepoint

- `crates/shell/elidex-shell/Cargo.toml:31` `elidex-js-boa.workspace = true`; **no `elidex-js` dep**.
- The `engine` feature on `elidex-js` (`Cargo.toml:32`) is enabled by nothing but two benches
  (`Cargo.toml:201-213`, `required-features = ["engine"]` on `event_dispatch` + `dom_collection`);
  the feature doc `:21` says "shell still runs the boa engine (S5 cutover pending)".
- `lib.rs:46` `use elidex_js_boa::JsRuntime`; `lib.rs:440` `pub runtime: JsRuntime` on
  `PipelineResult`.
- **Single construction site**: `pipeline.rs:161` `JsRuntime::with_network(network_handle)` inside
  `run_scripts_and_finalize` (`pipeline.rs:130`), fed by the 4 pipeline builders
  (`pipeline.rs:444,528,614,708` — the first three call `extract_scripts` at those same lines; the
  4th consumes `LoadedDocument.scripts` from `elidex-navigation`).
- Workspace: root `Cargo.toml:49` members + `:196` path entry.

### §3.2 Engine-driving surfaces (the bracket sites)

- **Script eval loop**: `run_scripts_and_finalize` — pre-eval install block (origin / sandbox / cookie
  jar / referrer / viewport / `set_history_state` `pipeline.rs:224-228`), then per-script `eval`
  (`:230-233`), `drain_timers` (`:236`), `flush_with_ce_reactions` (`:238`),
  `dispatch_lifecycle_events` (`:241` → bodies `:305-406`: readystatechange ×2, DOMContentLoaded
  `:317`, load `:344`), second CE flush (`:242`).
- **Post-construction dispatch chokepoint**: `PipelineResult::dispatch_event` (`lib.rs:488-491`) —
  EVERY shell dispatch rides it (`script_dispatch_event` at `lib.rs:490`): input events
  (`content/event_handlers.rs`), `dispatch_storage_event` (`content/mod.rs:672`),
  `dispatch_message_event` (`:693`), resize, MQL change. Plus `PipelineResult::eval_script`
  (`lib.rs:494`) and `drain_timers` (`lib.rs:500-502`). Unload: `dispatch_unload_events`
  (`pipeline.rs:380-406`, beforeunload `:390` / unload `:406`).
- **Frame drain**: `re_render` (`lib.rs:522-`) — CSSOM take + re-sync (`:524-528`, dies §3.3), then
  `session.flush` → boa `enqueue_ce_reactions_from_mutations` (`:541,556-558`) +
  `drain_custom_element_reactions_public` (`:542,560`) in the bounded
  `MAX_CE_STABILIZATION_ROUNDS` loop.
- **Bind primitive** (VM side, landed S1a): `ElidexJsEngine::bind` (`engine.rs:308`,
  `debug_assert!(!self.bound, "… batch brackets must not nest")` `:310-312`), `unbind` (`:336`),
  `with_bound` RAII (`:342`); the trait contract `elidex-script-session/src/engine.rs:156-186`.

### §3.3 CSSOM shadow-sync (deleted at flip)

`lib.rs:91` `stylesheets_to_cssom` (builds `elidex_js_boa::bridge::CssomSheet` list), `:212`
`sync_stylesheets_to_bridge`, `:221-223` the helper doc — verbatim "**(Dies with the boa path at the
S5 cutover.)**", `:250-293` `apply_cssom_mutations` over `elidex_js_boa::bridge::CssomMutation`
(`InsertRule :260` / `DeleteRule :293`), consumed at `re_render` `:524-528`
(`take_cssom_mutations` + re-sync). Test oracles: `tests.rs:849-866` (two `apply_cssom_mutations`
`InsertRule` tests). The VM needs none of it: CSSOM reads live from EcsDom
(`elidex-dom-api/src/cssom_sheet.rs`), `insertRule`/`deleteRule` mutate the DOM-owned sheet and
re-resolution picks them up — the shadow copy is **deleted, not ported** (§4.2 names the replacement
oracle).

### §3.4 boa-private bridge channels the shell drives (the E4 inventory — every row must land on the trait or be deleted)

The FULL sweep — **two grep patterns, both required**: `bridge()` + `elidex_js_boa` (round 1) AND the
inherent-method pattern `\bruntime\.[a-z_]+\(` (round 2 — inherent `JsRuntime` calls the first
pattern structurally missed), across `elidex-shell/src` incl. `content/iframe/` + `app/`, non-test,
**reconciled row-by-row against `memory/boa-vm-cutover-surface-parity-audit.md` §B** (§8-1): 27 rows
(B1–B26 + the B23b name-shadowed roll-up).
Classification: **CONVERGE** (existing trait method — named) / **DELETE** (the VM model or shell
ownership makes it unnecessary — why stated) / **ADD** (genuinely new trait/VM surface; each
investigated for an existing VM equivalent first).

| # | boa seam (shell call site) | VM / trait state at HEAD | Flip action (§4.3) |
|---|---|---|---|
| B1 | `bridge().take_cssom_mutations()` (`lib.rs:524`) | VM: DOM-as-truth, no channel needed | DELETE with §3.3 (+ the §4.2 re-collection replaces the cascade feed) |
| B2 | `enqueue_ce_reactions_from_mutations` + `drain_custom_element_reactions_public` (`lib.rs:541-560`; same 2-call pair in `flush_with_ce_reactions`, `pipeline.rs:105-111`) | VM: bind-scoped `ConsumerDispatcher` + `ScriptEngine::drain_reactions` (trait `:96`) — external-records CE gap, §4.3.1 | REWIRE + **extend `deliver_mutation_records` for record→CE** (§4.3.1) |
| B3 | `bridge().drain_storage_changes()` (`content/event_loop.rs:96`) | **VM: ABSENT** (no out-queue) | **ADD** trait drain group + VM enqueue (§4.3.2) |
| B4 | `elidex_js_boa::bridge::local_storage::flush_dirty_stores()` (`event_loop.rs:143`) | VM: `WebStorageManager::flush_dirty` exists (`elidex-storage-core/web_storage.rs:450`) but `HostData::install_web_storage` (`host_data.rs:992`) has **zero production callers** — VM falls back to per-VM in-memory | **ADD** install + shell-owned manager + per-turn flush (§4.3.3) |
| B5 | `bridge().drain_post_messages()` — TOP-LEVEL site (`event_loop.rs:88`, delivered to self) | VM: same-window `postMessage` is VM-internal (`vm/host/window.rs:610-611` → `pending_tasks::native_window_post_message`, delivered by the task drain) | DELETE the top-level site (VM self-delivers); the parent-side OOP drain `iframes.drain_oop_messages()` (`event_loop.rs:84`) is engine-indep, unchanged. **The iframe-side twin is B16 — a separate, REAL surface** |
| B6 | `bridge().drain_idb_versionchange_requests()` (`event_loop.rs:106-121`) | **VM: ABSENT** (no cross-tab versionchange request queue; the VM has only in-VM `versionchange` event machinery — `vm/mod.rs:2301` prototype) | **ADD** to the §4.3.2 drain group, at **both** boa fire sites — open-upgrade AND `deleteDatabase` (§4.3.2) — (a drain left boa-only = the E4 forbidden form) |
| B7 | `bridge().drain_sw_register_requests()` (`event_loop.rs:123-141`) | VM: routes SW register through the EXISTING trait `drain_sw_client_requests` (`engine.rs:231`; VM queue doc `vm/mod.rs:2465`) | CONVERGE onto the trait method |
| B8 | `bridge().current_url()` (`event_loop.rs:124`) | trait `current_url` (`:262`) | CONVERGE (type-level) |
| B9 | `deliver_media_query_changes(changed, session, dom, doc)` — shell-computed args (`content/mod.rs:621-627`; callers `event_loop.rs:441,471` after `re_evaluate_media_queries` `:466`) + `set_viewport` (`event_loop.rs:412`) | VM model INVERTED: `set_media_environment` (trait `:433`) + no-arg `deliver_media_query_changes` (`:448`; VM-internal eval, `vm/host/media_query.rs:412`) | REWIRE (§4.3.5) |
| B10 | `set_history_state` seed (`pipeline.rs:224-228`, boa no-op stub) + `deliver_history_step_events` ×4 (boa no-op stubs) + `set_history_length` ×8 | trait methods live (`:317`, `:336`, `:292`); VM delivery gates on `is_bound()` | bracket + `(index,length)` publish (§4.3.4) |
| B11 | `elidex_js_boa::sw_thread::sw_thread_main` (`app/sw_coordinator.rs:192`, 4 params) | VM twin `vm/sw_thread.rs:92` — **6 params** (§4.3.6) | REWIRE with new coordinator state |
| B12 | `extract_scripts` (`pipeline.rs:14,444,528,614`; `content_iframe_security_tests.rs:396`) | engine-indep twin already in `elidex-navigation` (§4.3.7) | UNIFY into `elidex-navigation`, delete boa's |
| B13 | `bridge().take_pending_focus()` (`event_loop.rs:157` → `ContentToBrowser::FocusWindow`; boa: `window.focus()` sets the flag, `bridge/viewport.rs:139-146`) | **VM: ABSENT** — no `window.focus()` native (grep `window.rs` = 0; element `.focus()` exists, window-level does not). Spec: HTML §6.6.6 `window.focus()` (`#dom-window-focus`; Window IDL `undefined focus()` webref-verified) | **ADD** (marshal-scale): `window.focus()` native → `HostData` pending-focus flag → trait `take_pending_focus() -> bool`; shell site converges |
| B14 | `drain_realtime_events()` + `dispatch_realtime_events(...)` — shell pumps WS/SSE events INTO boa (`event_loop.rs:162-171`) | VM model INVERTED: `tick_network` (trait `:202`) drives VM-internal WS/ES delivery (`websocket_dispatch.rs` / `event_source_dispatch.rs`; S5-3b keepalive-rooted) | DELETE the pump (tick_network already in the turn bracket) |
| B15 | `shutdown_all_realtime()` (`event_loop.rs:276`; `content/navigation.rs:197` pre-rebuild) + `shutdown_all_workers()` (`event_loop.rs:277`) | VM: `unbind` tears both down — `drain_realtime_for_unbind` closes ws/sse conns (`vm_api.rs:422-450`) + worker teardown | CONVERGE onto `unbind`/engine drop at pipeline replacement/shutdown (explicit calls die) |
| B16 | **iframe-side** `drain_post_messages()` (`content/iframe/thread.rs:113` → `IframeToBrowser::PostMessage` forward to parent — the in-process AND OOP iframe→parent messaging seam; boa context-routes ONE queue: top-level→self, iframe→parent) | **VM: no parent-directed out-queue** — VM `parent`/`top` resolve to `globalThis` (single-window stubs, `window.rs:346-409`), so an iframe VM's `parent.postMessage` would SELF-deliver = wrong target. Spec: HTML §9.3.3 Posting messages (`#dom-window-postmessage-options`; Window IDL `postMessage` overloads webref-verified) | **ADD** (marshal-scale, boa-parity): when `iframe_depth > 0`, the postMessage native enqueues a `pending_parent_messages` FIFO → trait drain; `thread.rs:113` converges. The real WindowProxy model replaces this at S5-8/B1 (the routing-by-depth mirrors boa's own context-routed queue) |
| B17 | `bridge().client_id()` (`content/navigation.rs:141`, SW fetch attribution; boa = per-runtime UUIDv4, `bridge/sw.rs:60`) | VM: no window-side client id (SW-side takes ids as params) | DELETE via **shell-owned**: the shell mints the client UUID at pipeline construction (`ContentState`/`PipelineResult` field) — coherent by construction with the `ClientSnapshot` ids the coordinator seeds into the SW (`initial_clients`, §4.3.6): ONE generator |
| B18 | `bridge().cookie_jar_clone()` (`content/navigation.rs:199`; `app/navigation.rs:360`; `iframe/lifecycle.rs:356` — read-back to thread the jar into the next build) | trait has `install_cookie_jar` (`:500`), deliberately no getter | DELETE via **shell-owned**: the shell constructs the jar and passes it in already — retain the `Arc<CookieJar>` on `PipelineResult` (engine-indep field) instead of reading it back through the engine |
| B19 | `set_credentialless` (`pipeline.rs:194`) + `credentialless()` read-back (`iframe/thread.rs:222` `PreEvalFrameInputs`) | **VM: ABSENT** — and correctly so: boa persists it as browsing-context config whose only effects are (a) the shell's rebuild round-trip and (b) storage partitioning keyed **through the opaque origin** (`bridge/mod.rs:300-305`, `document_state.rs:120`); the VM's behavior derives entirely from `set_origin` (S5-4b) | DELETE via **shell-owned**: `credentialless` lives on the shell's iframe entry / `PreEvalFrameState` SoT (browsing-context config = shell side-store, CLAUDE.md exception (b)); no engine surface. Impl-time check: confirm no JS-observable boa read exists (grep shows none) |
| B20 | device-facts read-backs `device_pixel_ratio()`/`color_scheme()` (`event_loop.rs:244-252` diff-guard; `app/navigation.rs:368-369`; `iframe/lifecycle.rs:349-360`) + pre-eval writes `set_viewport`/`set_device_pixel_ratio`/`set_color_scheme` (`pipeline.rs:211-216`) | writes → `set_media_environment`/`set_screen_dimensions` (trait `:433`/`:467`); reads → nothing (trait has no getters, deliberately) | CONVERGE writes (incl. the **initial pre-eval seed**, §4.3.5/F4); DELETE read-backs via **shell-owned** `DeviceFacts` snapshot (content thread already tracks `applied_facts_seq` + viewport cell; app-mode gains a `DeviceFacts` field) |
| B21 | receive-half `dispatch_idb_versionchange(...)` (`event_loop.rs:544-560`, boa inherent — fires `versionchange` on this tab's open connections from the cross-tab broadcast) | VM: in-VM `versionchange` machinery exists (`idb_version_change_event_prototype`, `vm/mod.rs:2301`) but **no external deliver entry** | **ADD** the deliver method to the §4.3.2 group (`deliver_idb_versionchange(db, old, new)`) — emit + receive land together (IndexedDB-3 §4.2 *fire a version change event*) |
| B22 | `bridge().drain_script_animations()` (`content/mod.rs:707-711` → shell `AnimationEngine`; boa has REAL `element.animate` — `globals/element/accessors/animate.rs` + `bridge/animation.rs`) | **VM: no WAAPI surface** (S5-7 owns it, umbrella §5 — post-flip by design) | DELETE at flip; **S5-7 re-establishes** the surface + its own drain design. This is the ONE known VM<boa surface at flip — umbrella-scheduled (S5-7 depends on S5-6), recorded as a §8 acceptance-gate exception with S5-7 as the immediate follow-up |
| B23 | Type-level trait converges (already 1:1): `set_cookie_jar :164→install_cookie_jar`, `set_origin pipeline.rs:173,192→:343`, `set_sandbox_flags :191→:352`, `set_iframe_depth :193→:387`, `set_referrer :200→set_navigation_referrer :301`, `set_history_state :228→:317`, `sw_controller_scope content/navigation.rs:116→:243`, `origin() iframe/lifecycle.rs:175,257→:348`, `iframe_depth() lifecycle.rs:350→:384`, `popups_allowed/sandbox_flags event_handlers.rs:190,201→:370/:356`, `forms_allowed form_input.rs:129→:363`, `set_visibility event_loop.rs:485→:398`, `set_scroll_offset content/mod.rs:224→:410`, `take_pending_scroll content/mod.rs:255→:405`, `sync_dirty_canvases content/mod.rs:242-245` (boa form takes `&mut dom`; trait `:206` is no-arg **assume-bound** → a bracketed sub-span inside the `re_render` region, §4.1) | trait methods exist | CONVERGE (mechanical type swap; canvas-sync additionally needs the §4.1 bracket placement) |
| B23b | **Name-shadowed inherent family** (inclusion criterion: boa inherents whose name AND signature already equal the trait method — S5-4c/S5-5a canonicalized them — so the call sites compile unchanged after the type swap): `set_current_url` (`pipeline.rs:168`, `content/navigation.rs:306,384,863` — the 4th is the S5-5c pushState/replaceState commit path), `take_pending_window_opens` (`content/navigation.rs:527`, `app/navigation.rs:50`, `event_loop.rs:154`), `take_pending_history` (`content/navigation.rs:553`, `app/navigation.rs:59`), `take_pending_navigation` (`content/navigation.rs:594`, `app/navigation.rs:89`) | trait-identical signatures (the deliberate S5-4c/S5-5a shape work) | CONVERGE (zero-diff at the call site; listed so the E4 audit's "every surface classified" claim is checkable, not implied) |
| B24 | `runtime.drain_and_dispatch_worker_events(session, dom, document) -> bool` (`event_loop.rs:181-185`, returns needs_render) | trait `drain_worker_messages` (`engine.rs:226`, no-arg no-return; VM impl `elidex-js/engine.rs:398` — the VM self-dispatches worker messages to JS listeners) | CONVERGE + the **needs_render replacement = the §4.3.8 version-delta signal** (the per-call bool dies with the boa shape) |
| B25 | observer delivery inherents with args: `deliver_mutation_records(records, session, dom, doc)` (`content/mod.rs:334`), `deliver_resize_observations(session, dom, doc)` (`:342`), `deliver_intersection_observations(session, dom, doc, viewport)` (`:354`) | trait forms exist: `:190` (records-only), `:193`/`:197` (no-arg, assume-bound; VM computes the viewport-intersection input off bound state) | CONVERGE (arg-shape change + turn-bracket placement; the mutation-records input shrinks per §4.3.1 — flush is empty under the VM) |
| B26 | **console test oracle**: `runtime.console_output().messages()` (`tests.rs:407,428,797,828` — boa inherent capture buffer) | **VM: ABSENT** — the VM console is a print native (`vm/natives.rs:742` `console_output(ctx, args, prefix)` formats + prints; no retrievable buffer). The parity audit §B's "console capture … exist → NOT cutover gaps" was a grep-presence over-claim (§8-1) | **ADD (test-oracle accessor, G3-iii lean)**: a marshal-scale VM console-capture buffer + accessor (`Vm::console_messages()` — the print native tees into a bounded per-VM buffer; feasibility trivial, the native already has the formatted string in hand). Lands in S5-6a; the 4 oracles swap accessor, not assertion |

**Six rows are genuine VM-capability ADDs** — B3 (storage-change drain), B4 (web-storage install),
B6 (IDB-versionchange emit drain), B13 (window.focus pending-focus), B16 (iframe parent-message
out-queue), B21 (IDB-versionchange receive deliver) — all shell-facing drain/install/flag plumbing
(marshal-scale), not algorithms — **plus one test-oracle accessor** (B26 console capture). All seven
land in **S5-6a** (§0.1). Everything else is call-site convergence onto landed trait surface
(CONVERGE) or dies with the boa model / moves to shell-owned state (DELETE), in **S5-6b**.

### §3.5 VM-side surface (what the flip drives)

`HostDriver` + `ScriptEngine` (`elidex-script-session/src/engine.rs`): eval `:58` / `call_listener`
`:76` / `run_microtasks` `:93` / `drain_reactions` `:96` / `drain_timers` `:99`; bind group
`:156-186`; deliver group (`deliver_mutation_records :190`, resize `:193`, intersection `:197`,
`tick_network :202`, `sync_dirty_canvases :206`, SW client `:210-231`, `next_timer_deadline :238`);
navigation/history group `:257-336`; security group `:343-387`; visibility/scroll `:398-410`;
media/screen/viewport `:433-486`; install group `:492-500` (network / idb / cookie-jar — **no
web-storage yet**). `ElidexJsEngine` implements all of it (`elidex-js/src/engine.rs`); `drain_reactions`
= `settle_tasks_and_reactions` (`engine.rs:103-106` = `drain_tasks()` + `flush_ce_reactions()`).
`Vm::deliver_mutation_records` (`vm_api.rs:950`) is documented "cutover-ready API: the boa-driven shell
still invokes the boa-side … the VM-side wiring lands with the boa→VM cutover (M4-12 D-26 / PR7)".

---

## §4 Ideal architecture

### §4.1 The batch-bind brackets (umbrella §4 — reproduced as binding design)

VM `bind`/`unbind` are **heavy browsing-context-cycle** operations (`vm_api.rs`: unbind clears
non-Node wrappers + live collections, rolls back IDB txns, tears down dispatcher/workers) — NOT boa's
cheap per-call Rc swaps. The model is **BATCH-BIND**: the shell brackets each engine-driving batch
with **ONE** `bind`/`unbind` (`with_bound` RAII, `engine.rs:342`); the trait methods assume bound;
`bind` is non-re-entrant (`engine.rs:308-312` debug_assert). The post-script microtask checkpoint is
HTML §8.1.4.4 *clean up after running script* (`#clean-up-after-running-script`, webref-verified),
self-contained per-callback in `VmInner` (`eval`; `drain_timers` runs the full checkpoint **per fired
timer**, `engine.rs:277-283` doc; `drain_reactions` is the post-dispatch checkpoint,
`engine.rs:258-273` doc).

**Coupled invariants** (umbrella §4, each pair's intersection named):

- **bind-lifetime × assume-bound contract**: the trait methods (`eval`/`call_listener`/`drain_*`/
  `deliver_*`) read host pointers *without* re-binding, so `ctx.session`/`ctx.dom` must stay valid +
  **unaliased** for the whole outer batch — a method that self-bound mid-batch (boa's model) would
  tear cross-`<script>` wrapper/JS-state identity through the heavy unbind; and the shell must not
  touch `session`/`dom` directly while a bracket is open (the `with_bound` `# Safety` contract,
  `engine.rs:342-360`).
- **assume-bound × sandbox `scripts_allowed`**: the sandbox short-circuit (HTML §7.1.5
  `#sandboxed-scripts-browsing-context-flag`) reads per-VM sandbox flags off **bound** HostData, so it
  runs *inside* the bound window, never before bracketing (pre-bind gating would read absent/stale
  flags). All S5-4 gates already read bound state by construction — the flip only places the bracket
  around them.
- **bind-lifetime × `scripts_allowed`**: a *disallowed* script must not leave a half-open bracket —
  the bracket opens and closes regardless of whether any script ran (`with_bound`'s RAII guard makes
  the unpaired form unrepresentable).

**Bracket placement** (one bracket per batch; §3.2 sites):

| Batch | Bracket span |
|---|---|
| `<script>` eval loop | ONE bracket around `run_scripts_and_finalize`'s engine-driving span: pre-eval installs → per-script `eval` loop → `drain_timers` → CE settle → lifecycle dispatch (`:230-242`, dispatch bodies `:305-370`) — NOT per-script (cross-script wrapper identity; unbind is heavy) |
| UA / lifecycle event dispatch | `PipelineResult::dispatch_event` (`lib.rs:488`) becomes the bracketed chokepoint (open bracket → `script_dispatch_event` → `drain_reactions` per its contract → close); direct `pipeline.rs` dispatch sites ride the eval-loop bracket above; `dispatch_unload_events` gets its own bracket |
| Frame drain / `re_render` | bracket the engine-driving sub-spans: [`sync_dirty_canvases` — the trait form is no-arg assume-bound (`:206`), unlike boa's `&mut dom` form at `content/mod.rs:242-245` (B23), so it becomes its own bracketed sub-span at the top of the region] and [`deliver_mutation_records` + `drain_reactions`] after each `session.flush`; `session.flush` / style / layout / paint run **outside** any bracket (they take `&mut dom` — the unaliased contract forbids overlap; the E1×F11 intersection) |
| Timer drains | `PipelineResult::drain_timers` (`lib.rs:500`) + `pipeline.rs:236` ride their batch's bracket |
| Per-turn deliver/drain pump (`event_loop.rs`) | ONE bracket per message-turn around the engine-driving span: dispatch + `tick_network` + `deliver_*` (MQL / resize / intersection / history-step) + `take_pending_*` / storage drains — the **uniform** delivery-binding decision S5-5b §9 requires (no per-channel brackets) |

**Bracket/flush interleaving pin (F11)**: `re_render` / `session.flush` / style / layout are **never**
invoked with a bracket open — they take `&mut dom`, which the bound-pointer contract forbids. Where a
handler's effects require a follow-up engine pass (the §4.3.4 post-handler re-drain), the pass runs as
a **sibling bracket AFTER the owning bracket closes**, with any `re_render` between the two brackets:
`[bracket A: deliver/dispatch] → close → re_render (unbound) → [bracket B: re-drain take_pending_*] →
close → re_render if effects`. Sibling-bracket (not bracket-extension) because the re-drain's inputs
are only complete once the handler's DOM effects are flushed — and flushing requires being unbound.

**The cross-batch wrapper-identity edge (surfaced for plan-review — §11-Qe).** Because `unbind`
clears non-Node wrapper caches (cross-DOM-safety clearing, `vm_api.rs` "cross-DOM references and must
be cleared on unbind"), a JS-held wrapper compared against a re-fetched one **across batches** can
violate WebIDL `[SameObject]` (the extended attribute promises the SAME object on every getter
access, with no task-scoped carve-out — the spec is violated across batches, full stop). What the
interim ships is a narrower **tested contract**: identity holds within a batch (one script task) —
an engineering judgment about the observable blast radius (in-task comparisons dominate real usage),
NOT a spec attribution.
boa's persistent caches did preserve cross-task identity, so this is a (narrow) behavior delta at the
flip. Interim disposition: **accept + document + pin with a test** (the post-flip 1:1 VM↔DOM pairing
makes the clearing semantically unnecessary but structurally required pre-B1; the agent-scoped-World
B1 program (PR #434) makes same-World rebind identity-safe by construction and migrates wrappers to
full-keyed components — the structural fix). Carve `#11-cross-batch-wrapper-identity` (NEW, B1-fold
candidate) if plan-review agrees the interim is acceptable; the alternative (skip the clear on
same-DOM rebind) is B1-territory double-refactor (umbrella §0.1 ban).

**Named flip invariant — ONE Vm per navigation (F18).** The flip keeps **navigation = a fresh engine
per pipeline build** (the single construction chokepoint, §3.1). This is not an implementation
accident: the B1 reconciliation's **retraction of the nav-scrub-as-S5-6-hard-gate is CONDITIONED on
it** — "the flip keeps one-Vm-per-navigation (navigation = new Vm), so the flip is
cross-DOM-NEUTRAL; cross-DOM aliasing only becomes reachable via friendly iframes (post-S5)"
(`memory/project_world-id-deferral-contract-next.md`, the nav-scrub retraction bullet; PR #434
`docs/plans/2026-06-agent-scoped-ecsdom-world.md` §6.2). An impl-time "optimization" that reuses the
engine across navigations would re-open live cross-DOM wrapper aliasing (the exact hazard the E2
clearing guards) with no gate left standing. Engine reuse is **forbidden in this PR** and until B1;
pin with a comment at the construction chokepoint.

### §4.2 CSSOM: delete the shadow, DOM is the truth — WITH the DOM→cascade re-collection

Everything in §3.3 is deleted — `stylesheets_to_cssom`, `sync_stylesheets_to_bridge`,
`apply_cssom_mutations`, the `re_render` take/re-sync block, both helpers.

**⚠ RETRACTION (plan-review F1)**: this memo's first draft claimed `re_render`'s style
re-resolution "sees script CSSOM mutations with no sync step at all". **That was wrong.** The
cascade does NOT read the DOM's sheets: `re_render` Phase 2 resolves over the build-time snapshot
`PipelineResult.stylesheets` (`lib.rs:614` `stylesheet_refs` → `resolve_with_mode` `:615`), and that
snapshot's ONLY mutator at HEAD is the to-be-deleted `apply_cssom_mutations` (`lib.rs:526`). The VM's
`insertRule`/`deleteRule` do write back to the DOM **owner sources**
(`elidex-dom-api/src/cssom_sheet.rs` "Mutator round-trip": `<style>` text via the `apply_replace_all`
primitive, records discarded; `<link>` via the `LinkStylesheet` component) — but nothing re-collects
those sources into cascade input. Deleting the shadow-sync without a replacement would silently break
`insertRule`'s rendered effect at flip.

**The replacement — DOM→cascade re-collection (version-compared, ECS-native):**

- **⚠ Round-2 correction (G1)**: the round-1 draft proposed a `SessionCore` take-flag set at the
  `flush_sheet_mutation` chokepoint. Dropped — (a) the chokepoint lives in `elidex-dom-api` while the
  flag would live on `SessionCore`, an unverified wiring convention a new write-site could silently
  miss; (b) the ECS-idiomatic form is **version counter + lazy detection**, which dissolves the
  wiring question entirely. **No flag, no chokepoint convention.**
- **Version signals (verified)**: exactly the ones `cssom_sheet.rs` already keys its own CSSOM cache
  on — `sheet_version(entity, dom)` (`cssom_sheet.rs:68-76`): the `LinkStylesheet` component's
  monotonic counter for `<link>`, else the subtree `EcsDom::inclusive_descendants_version` for
  `<style>`. Both fire on the mutator write-back paths by construction: the `<style>` write-back is
  an `apply_replace_all` text replace ("a direct `<style>.textContent` write diverges it" +
  "the cascade picks up the change and `EcsDom::rev_version` fires" — module doc `:13-36`), and the
  `<link>` write-back writes the `LinkStylesheet` component, bumping its counter.
- **`<link>` write-back must ALSO bump the root version (round-4 I2 — option A chosen)**: the
  `<link>` arm of `flush_sheet_mutation` (`cssom_sheet.rs:135-159`) writes source + `version += 1`
  through a **raw component borrow** — bumps nothing — so the §4.3.8 root-version delta never sees a
  link-sheet `insertRule`/`deleteRule`: an **async** turn (worker/timer) whose only effect is a
  link-sheet CSSOM mutation would skip `needs_render`, and since the re-collection runs INSIDE
  `re_render`, the change would be lost until the next unrelated render (sync event paths mask this
  — they re-render unconditionally). **Fix: the `<link>` arm additionally calls
  `rev_version(sheet_entity)`** (one line, one signal — lands with the S5-6a re-collection).
  Self-trigger verification: the `<link>` owner's per-owner compare key is the **LinkStylesheet
  counter, not `inclusive_descendants_version`** (`sheet_version`'s component-first branch,
  `:71-75`), so the added bump cannot dirty the collect compare, and the `CollectedStylesheet` stamp
  stays a raw write — E11's false-positive direction stays closed. Discipline coherence: the
  `<style>` arm ALREADY bumps (via `apply_replace_all`); a CSSOM source write IS a document-visible
  style mutation, so instrumenting the `<link>` arm makes the two write-back arms symmetric, not a
  new instrumentation class. (Option B — widening the turn-end compare to a per-owner sheet-version
  aggregate outside `re_render` — rejected: a second change-signal shape for one producer.)
- **Re-collection seam**: a new engine-indep `elidex-dom-api::collect_document_stylesheets(&mut
  EcsDom) -> …` that **lazily compares per-owner versions against a cached stamp** and re-parses
  (`elidex_css::parse_stylesheet`) only changed owners. `re_render` calls it **every frame** —
  O(#owners) version compares + `Arc` bumps on the no-change path, the same O(1)-per-owner
  discipline `cssom_sheet.rs` documents. **Cache home (CLAUDE.md side-store rule)**: the
  parsed-stylesheet + version stamp is per-entity, `Send + Sync`, derived state → an **ECS
  component on the owner entity** (`CollectedStylesheet { parsed: Arc<Stylesheet>, version: u64 }`
  — the `Arc` is a /simplify-stage representation refinement, not a design change: cache hits and
  cascade-input assembly are pointer copies, never a deep `Stylesheet` clone) — SameObject-free
  derived data, despawn = automatic cleanup (at document teardown — mid-life removal is a detach,
  see the scope-bound bullet/I1), no entity-keyed side map. `PipelineResult.stylesheets` remains
  the assembled cascade-input Vec (`Arc`-shared with the stamps), rebuilt from the components when
  any owner's version moved.
- **Stamp write must not self-trigger (round-3 H2, investigated)**: the version instrumentation is
  **explicit, not hecs-level** — `rev_version(entity)` (`elidex-ecs/src/dom/mod.rs:710`, the
  Servo-style ancestor-propagating bump) is called only from the DOM mutation methods
  (`dom/attribute.rs:228,309`, `dom/text_data.rs:55,124,189`, `dom/tree/mutation.rs:44-455`,
  `dom/tree/teardown.rs:146,235`); hecs has no write hooks, so a **raw component write via
  `dom.world_mut().insert_one(entity, …)` bumps NOTHING** — neither `inclusive_descendants_version`
  nor the document-root version. That is the chosen write seam: the `CollectedStylesheet` stamp is
  written through the non-instrumented derived-data path (`world_mut().insert_one` — the PRODUCTION
  precedents are the style/layout derived-state writes: `elidex-style/src/pseudo.rs:64`,
  `elidex-style/src/walk.rs:233,238`, `elidex-layout/src/layout/anonymous_table.rs:155` — all
  verified; round-4 I3 swapped out the earlier `computed_style.rs` cite, which is `#[cfg(test)]`
  fixture code), hence the signature takes `&mut EcsDom`, and the write is **non-self-triggering by
  construction** (no compare-before-write dance needed). Pinned by the §7 no-change-frame oracle
  (zero re-parse AND zero §4.3.8 delta on an idle frame — the E11 edge).
- **Scope bound (restated, round-3 H7 + round-4 I1 — the walk EXCEEDS boa parity, deliberately)**:
  the owner enumeration is a **document-connectedness-filtered, document-ORDER tree walk from the
  document root** — NOT a bare hecs query, for two verified reasons: (i) **removal = DETACH, not
  despawn** — `EcsDom::remove_child` (`dom/tree/mutation.rs:57-88`) only `detach`es (the node
  survives for re-insertion / held wrappers; the sole `despawn_subtree` caller is parser document
  teardown, `elidex-html-parser-strict/tree_builder/mod.rs:232`), so a script-removed `<style>`
  remains a live queryable entity still carrying its `CollectedStylesheet` — the connectedness
  filter, not entity death, is what drops it from the cascade input; (ii) **document order matters
  for cascade correctness** (stylesheet order is cascade order), which a hecs query does not
  provide. Under that walk: a script-**appended** `<style>` IS picked up (new owner, no stamp →
  parse), a script-**removed** owner drops out via the connectedness filter (its component is
  reclaimed only at document teardown — the despawn-cleanup property is real but end-of-life-only),
  and a `<style>.textContent` **edit** IS picked up (it bumps
  `inclusive_descendants_version` via the instrumented text mutation — `text_data.rs:55,124,189`).
  None of these restyled under boa — the version-keyed walk covers them **by construction**, at no
  extra machinery, and they are spec-correct improvements shipped with tests, not parity risks. What
  stays deferred is exactly `#11-link-stylesheet-dynamic-fetch` (post-B2, user-confirmed): a
  script-inserted `<link rel=stylesheet>` triggers **no fetch** — its `LinkStylesheet` component is
  only populated by the document loader, so an unfetched dynamic `<link>` has nothing to collect
  (the gap is the FETCH, not the collection).

**All `result.stylesheets` read sites (grep-enumerated) and their post-flip source:**

| Site | Today | Post-flip |
|---|---|---|
| `lib.rs:614-615` (`re_render` Phase 2 cascade) | build-time snapshot, patched by `apply_cssom_mutations` | the refreshed cache (re-collection above) |
| `lib.rs:526,528` (apply + re-sync) | the shadow mechanism | DELETED |
| `pipeline.rs:498,575,661,754` (`sync_stylesheets_to_bridge` at the 4 builders) | boa bridge seed | DELETED |
| build-time cascade (`pipeline.rs:150,245,280` `resolve_with_mode` over the `stylesheets` param) | parse-time collection | unchanged (parse-time collection stays the initial input) |

**Replacement test oracle** (for `tests.rs:849-866`): drive `insertRule` via script in a pipeline
test and assert the **rendered outcome** (re-resolved style / display-list effect) — the observable
the shadow-struct equality oracle was a proxy for — plus a re-collection unit oracle
(`collect_document_stylesheets` picks up a written-back `<style>`/`LinkStylesheet` source) and the
existing VM-level CSSOM tests.

### §4.3 Back-channel rewiring map (the One-issue-one-way convergence)

#### §4.3.1 CE reactions (investigated seam — with the unbound-flush ownership gap named)

The VM's in-band CE-reaction enqueue seam is **bind-scoped**: `Vm::bind` constructs the typed
`ConsumerDispatcher` (`vm_api.rs:258`) — whose `CustomElementReactionConsumer`
(`consumer_dispatcher.rs:75-85`) enqueues Connected / Disconnected / AttributeChanged reactions onto
`HostData::ce_reaction_queue` (`host_data.rs:536-538`) — and installs it **on the EcsDom**
(`vm_api.rs:280` `dom.set_mutation_dispatcher`); `unbind` clears it (`vm_api.rs:487`
`clear_mutation_dispatcher`). Reactions drain via `VmInner::flush_ce_reactions`
(`vm/host/custom_elements/flush.rs:40`, bounded waves), reachable from the shell as
`ScriptEngine::drain_reactions` (`elidex-js/engine.rs:258` → `settle_tasks_and_reactions` `:103`).

**The gap (plan-review F2)**: under the §4.1 bracket model, `session.flush(&mut dom)` runs
**outside** the bracket (the `&mut dom` aliasing contract forbids overlap; moving the flush inside is
rejected) — i.e. **after `unbind` cleared the dispatcher** — so no VM consumer hears the flushed
mutations. And the external-records entry does not cover CE: **`Vm::deliver_mutation_records`
(`vm_api.rs:950` → `mutation_observer.rs:421`) is observer-ONLY** — per record `notify_one`
(`mutation_observer.rs:563`: observer record queues + the DOM §4.2.3 Mutation algorithms *remove*
step 15 transient-registered-observer creation) then `deliver_pending_mutation_records`; no CE path
anywhere in it. So a shell-flushed record's CE reaction has **no owner** at HEAD.

**Mutation-source partition — the no-double-enqueue invariant (round-2 G2 investigation, verified):**
a mutation rides **exactly one custody chain**, by construction:

- **VM-native mutations** go through the session's `apply_*` primitives which write the EcsDom
  **immediately** (`element_attrs.rs:133-137`: `apply_set_attribute` "calls the
  `EcsDom::set_attribute` chokepoint (full fan-out preserved)") — fan-out reaches the bind-installed
  dispatcher (CE custody) and the VM queues observer records itself; they **never enter
  `SessionCore::pending`**. The flush doc states this outright: the per-op `notify_records` scratch
  "is drained by the VM per bridge op (so it is empty here under the VM)", and `pending` is the
  buffered path of "a non-draining embedder (the boa runtime …)" (`session.rs:114-126`). **Under the
  VM, `session.flush` returns records only for externally-buffered/externally-built mutations — for
  VM-native work it is EMPTY.**
- **External records** (layout-derived or shell-buffered — the `vm_api.rs:950` doc's own case) reach
  CE only through the `deliver_mutation_records` extension below; the dispatcher cannot double-hear
  them because flush runs unbound (dispatcher cleared at `unbind`, `vm_api.rs:487` — the F11 pin is
  itself part of the invariant).

So double-enqueue is **unrepresentable** while both legs hold; a **double-fire pin test** (§7.2)
guards the legs (one bound native mutation + one external-record delivery in the same turn ⇒ exactly
ONE CE reaction + ONE observer record each).

**Design**: extend the external-records entry to own the record→CE conversion — inside the same
`deliver_mutation_records` trait call, before the observer delivery, run the record-driven CE
enqueue. **Single-homing pin (round-3 H1)**: the record→CE classification (record → Connected /
Disconnected / AttributeChanged against `observed_attributes`) is **ONE factored implementation**
called from both entry points — the `CustomElementReactionConsumer` (mutation-event leg) and the
`deliver_mutation_records` extension (record leg) — never a parallel re-implementation. **Crate
home: engine-indep `crates/dom/elidex-custom-elements`**, beside the consumer that already owns the
`observed_attributes` gating (`consumer.rs:56` `CustomElementReactionConsumer` + its per-entity
classification `:208`); the record-leg entry at `vm/host/mutation_observer.rs:421` stays
marshalling + call-through only (Layering mandate).

**Spec framing (G2b)**: the spec enqueues CE reactions **inside the mutation algorithms themselves**
(DOM §4.2.3 Mutation algorithms; HTML §4.13.6 Custom element reactions — both webref-verified
2026-07-10), not from records. The record-driven conversion is a **boa-parity interim**, owned by the
EXISTING slot `#11-ce-reaction-mutation-observer-ordering` (algorithm-site enqueue + CE↔MO relative
ordering) — this PR does not carve anything new for it (§10).

Net shape: the shell's boa 2-call CE loop (`lib.rs:541-560`) largely **dissolves** rather than
converts — VM-native mutations settle CE inside the VM's own checkpoints (`flush_ce_reactions`
bounded waves, `flush.rs:27`), so the shell-side `MAX_CE_STABILIZATION_ROUNDS` loop and its flush
records shrink to the external-record case: `session.flush` (outside brackets, usually empty) →
[bracket: `deliver_mutation_records(&records)` (record→CE + observers) + `drain_reactions`] only when
records exist. The boa-only `enqueue_ce_reactions_from_mutations` /
`drain_custom_element_reactions_public` names die with the crate. **Consequence for the shell's OWN
record consumers** (focusable-cache invalidation, iframe add/remove detection from records,
`needs_render` — `content/mod.rs` re_render doc + `:360+`): they starve when flush goes empty →
they move to the §4.3.8 version-delta signal (same root, one signal).

#### §4.3.2 The new trait surfaces (the six ADDs — one cohesive design)

New cohesive `HostDriver` method-groups (Accretion, `engine.rs:127-130` — one home, incremental
membership), payload types on `elidex-script-session` (engine-indep, mirroring
`WindowOpenIntent`/`HistoryStepEvents` precedents):

- **Storage-change emit drain** (B3): `take_pending_storage_changes() -> Vec<StorageChange>`. VM
  enqueue in the `Storage` natives (`vm/host/storage.rs` set/remove/clear) — **fire only on an actual
  value change per HTML §12.2.1 setItem step 3.2 "If oldValue is value, then return"**; never delivered
  to the originating document (§12.2.1 broadcast excludes the originating storage — enforced by the
  existing shell broadcast topology, which only fans out to OTHER tabs). **Changed-ness/oldValue
  derive engine-indep from the storage backend's return** — `WebStorageManager::local_set ->
  Result<Option<String>, StorageError>` returns the previous value (its doc literally says "for
  StorageEvent `oldValue` pairing", `web_storage.rs:324-334`) and `local_remove -> Option<String>`
  (`:368`) likewise (the in-memory `SessionStorageState::set` `:524` has the same shape) — so the
  host native does compare + enqueue-marshalling only; **no storage-core extension needed**. The
  payload additionally carries the **storage-BUCKET `origin` string** (the shell's broadcast-targeting
  key): the enqueue site's `current_origin` bucket key — the per-VM opaque-origin **sentinel** for
  sandboxed/opaque documents — NOT a re-derivable `origin().serialize()`, which collapses every
  opaque document to `"null"` and would alias unrelated sandboxed iframes' broadcasts across the
  S5-4b isolation boundary.
- **IDB versionchange, emit + receive TOGETHER** (B6 + B21, IndexedDB-3 §4.2 *fire a version change
  event*): `take_pending_idb_versionchange_requests() -> Vec<IdbVersionChangeRequest>` — VM enqueue
  at **BOTH boa fire sites**: the open-with-higher-version path (`new_version = Some`; note a fresh
  0→1 creating open also rides this branch, so every fresh-db open emits a request — boa same) AND
  `deleteDatabase` (`new_version = None`, IndexedDB-3 §5.3 *delete a database* step 6;
  existence-gated per step 4 — deleting a nonexistent database broadcasts nothing, spec-correct
  where boa enqueued unconditionally) — + the receive-half deliver `deliver_idb_versionchange(db,
  old_version, new_version)` (fires `versionchange` on this VM's open connections — the VM's in-VM
  event machinery exists, `vm/mod.rs:2301`; only the external entry is missing). The IPC
  correlation `request_id` is NOT engine surface — the shell mints it at the drain→IPC seam (S5-6b),
  where boa minted it in the bridge; it never crosses back into the engine. The boa-inherent
  receive site (`event_loop.rs:544-560` `dispatch_idb_versionchange`) converges onto the deliver.
  **Adjacent TODO dispositions (H10)**: the browser-side `TODO(M4-10)` at
  `app/content_messages.rs:257-258` (wait for `IdbConnectionsClosed` from all tabs / timeout before
  `IdbUpgradeReady` vs `IdbBlocked`, W3C IndexedDB §2.4) is broker-protocol sequencing **above** the
  drain/deliver seam — it is exactly the "coordinate connection-close acks beyond the existing
  `IdbConnectionsClosed` reply" coupling named as **D2's fire condition** (§10-D2): if wiring the
  deliver half forces that sequencing work, D2 fires; otherwise the TODO stays M4-10-owned. The
  `content/navigation.rs:170-172` "TODO: construct document from SW response body" is the SW
  navigation-interception response path — untouched by the flip (the SW thread swap changes the
  engine inside the SW, not the interception protocol); scoped out, M4-10 family.
- **Pending-focus drain** (B13): `window.focus()` native → `HostData` flag →
  `take_pending_focus() -> bool` (boa parity: `bridge/viewport.rs:139-146`).
- **Parent-message drain** (B16): iframe-depth-routed `pending_parent_messages` FIFO →
  `take_pending_parent_messages() -> Vec<ParentMessage>` (named payload: `data` — the boa-parity
  `ToString` wire form — + `target_origin` **verbatim**); `iframe/thread.rs:113` converges. The
  §9.3.3 targetOrigin gate is applied at the **receiving** side (S5-6b): it compares against the
  TARGET (parent) window's origin, which the iframe VM cannot know — the sender only
  syntax-validates (boa gated against the SENDER's own origin, a deletion-bound divergence).
  Replaced wholesale by the S5-8/B1 WindowProxy model.
- **Web-storage install** (B4): `install_web_storage(Arc<WebStorageManager>)` on the install group
  (§4.3.3).

All queues are FIFO event-queue-shaped per-VM HostData (B1-neutral, transient intent standing);
all drains/delivers run inside the turn bracket. Shell: `event_loop.rs:96-121` swaps its boa drains
onto the trait calls. The storage **receive** side is already engine-agnostic (§2.3-C2). This closes
`#11-storage-event-broker`'s emit half; mode-aware delivery stays the umbrella §6 follow-up.
(sessionStorage broadcast is out of scope: elidex has no second same-session context pre-S5-8, and
boa's drain was localStorage-only — parity preserved; it rides the S5-8 browsing-context hand-off,
§10.)

#### §4.3.3 localStorage persistence (investigated seam)

VM `Storage` natives write through `HostData::web_storage = Arc<WebStorageManager>`
(`elidex-storage-core`; same on-disk layout as boa — `{data_dir}/elidex/localStorage/
{origin_hash}.json`, boa `bridge/local_storage.rs:4` vs storage-core `web_storage.rs:15,240` — so
**user data carries across the flip**; verify hash+JSON format identity at impl, it is the same
SHA-256-hex + serde_json scheme). Writes mark origins dirty; persistence = `WebStorageManager::
flush_dirty` (`web_storage.rs:450`, per-origin lock, retry-on-failure). At HEAD
`HostData::install_web_storage` (`host_data.rs:992`) has **zero callers** → production VM would fall
back to per-VM in-memory `fallback_local_storage` = silent persistence loss at flip. Wiring: add
`install_web_storage(Arc<WebStorageManager>)` to the `HostDriver` install group (`:492-500` — cookie
jar precedent, shared cross-cutting session resource, CLAUDE.md exception (b)); the shell owns ONE
process-wide manager (constructed beside the cookie jar), installs it at pipeline construction, and
`event_loop.rs:143` becomes `manager.flush_dirty()` (shell-side call on the shell-owned manager — no
trait method needed for the flush itself; the boa global-registry call dies). **Why
convention-guarded rather than construction-required (H11)**: the in-memory fallback IS the VM's
hermetic test path — `install_web_storage` has zero callers *including the VM's own unit tests*
(§3.4-B4 grep), which all run on `fallback_local_storage` with no disk I/O; a required constructor
input would force every VM unit test to build a disk-backed manager (or reintroduce the same
`Option` internally). So the fallback stays, guarded twice at the production seam: the debug
assertion at shell construction + the §5-item-7 shell persistence test (F14).

#### §4.3.4 History delivery + publish

- Bracket the 4 `deliver_history_step_events` sites + the `set_history_state` seed inside the uniform
  turn/eval brackets (§4.1) — the S5-5b-registered "bind the VM around history delivery" deliverable;
  the VM impl's `is_bound()` gate then passes by construction.
- Swap all **8** `set_history_length(len)` sites to `set_session_history(nav_controller.
  current_index(), nav_controller.len())` (trait `:292`) — whole-surface, closing
  `#11-session-history-index-vm-publish`; add the `NavigationController` current-index getter it
  needs (the slot's own trigger note).
- **Post-handler re-drain** (S5-5b §9 fold, interim until S5-5d): after a bracketed
  `deliver_history_step_events` in `fragment_navigate` / the traversal path, re-drain the
  navigation/history/window-open intents a popstate/hashchange handler enqueued (one extra
  `process_pending_actions` pass, run as a **sibling bracket after the delivery bracket closes** —
  the §4.1 F11 pin) + re-render if the handler mutated DOM / queued scrolls — and apply
  the *navigate-to-fragment* step-15 scroll AFTER popstate returns (scroll-vs-handler ordering), via
  the existing `re_render` post-layout scroll seam. Explicitly interim: D5
  (`#11-session-history-task-queue-model`, S5-5d) replaces the inline pass with the task-queued model;
  the flip's version is the minimal live-correctness form, test-backed now that handlers actually run.
- boa relative-nav base: erased by deletion; add the both-orders regression test (§1.3).

#### §4.3.5 Media-query / screen / viewport delivery (the model inversion)

The shell currently **computes** MQL changes itself against the boa bridge
(`re_evaluate_media_queries` at `event_loop.rs:466`; `dispatch_media_query_changes`
`content/mod.rs:621` → boa `observers.rs:210` taking `changed: &[(u64, bool)]`). The VM model inverts
this: the shell **pushes facts, the engine evaluates** — `set_media_environment(MediaEnvironment)`
(trait `:433`; from the C3-landed device-facts transport: `apply_device_facts` / `SetViewport` /
`SetDeviceFacts` sites `event_loop.rs:430-475`) then no-arg `deliver_media_query_changes()` (`:448` →
`vm/host/media_query.rs:412`, canonical `elidex-css::media` eval + CSSOM-View §4.2
matches-before-change ordering + the S5-3 keepalive rooting). Same shape for
`set_screen_dimensions` (`:467`) + `deliver_visual_viewport_events` (`:486` →
`visual_viewport.rs:265`) — the S5-2 surfaces whose live shell producers were explicitly deferred to
S5-6. All inside the turn bracket. The shell-side `re_evaluate_media_queries`/`changed`-list plumbing
is deleted.

**Initial seed (F4)**: the pre-eval install block already seeds boa's viewport + device facts before
the first script (`pipeline.rs:211-216` `set_viewport`/`set_device_pixel_ratio`/`set_color_scheme` —
the C3 "born with the right dppx/prefers-color-scheme" invariant). The flip MUST carry that seed onto
the VM surface: an initial `set_media_environment` (viewport size + dppx + color-scheme from the same
`viewport`+`device_facts` construction inputs) + `set_screen_dimensions` in the pre-eval install
block, else the first paint's `matchMedia`/`devicePixelRatio` reads a default environment — a C3
regression. VisualViewport's initial state derives from the same environment (no separate seed
call). Install-block completeness sweep (every boa pre-eval install → its VM equivalent):
cookie jar `:164`→`install_cookie_jar` ✓, `set_current_url :168` ✓, origin `:171-175,192`→`set_origin`
✓, sandbox `:191` ✓, iframe depth `:193` ✓, credentialless `:194`→shell-owned (B19), referrer
`:200`→`set_navigation_referrer` ✓, viewport+facts `:211-216`→**this seed**, history state
`:228`→`set_history_state` ✓ — no other seed is missing.

#### §4.3.6 SW thread (investigated parity)

**Not signature-identical.** boa: `sw_thread_main(script_url, scope, channel, network_handle)` — 4
params (`elidex-js-boa/sw_thread.rs:33-38`; call `app/sw_coordinator.rs:192`). VM twin:
`vm/sw_thread.rs:92-99` — **6 params**, adding `cache_conn: Arc<Mutex<SqliteConnection>>` (the DR-A
shared per-origin Cache API connection = `OriginStorageManager::cache_connection`,
`elidex-storage-core/origin_manager.rs:186` — shared so window VM and SW observe one cache) and
`initial_clients: Vec<ClientSnapshot>` (seeds `clients.matchAll()`). The shell has **neither today**
(grep `SqliteConnection|cache_conn|ClientSnapshot` in `elidex-shell/src` = 0): the coordinator gains
per-origin `OriginStorageManager` state (constructed at registration, keyed by the SW origin) and
passes the registering page's client snapshot(s) (the coordinator's `SwHandle`/registration knowledge;
window-side updates continue via the existing trait `deliver_sw_client_update`/`seed_sw_client`
`:210-224`). Mode: the VM entry **hard-derives `BrowserCompat`** (`sw_thread.rs:79-91` doc, F10 —
deliberate, correct for the flip per umbrella §6; `#11-async-core-storage-cookiestore` threads a real
mode later). Everything else (channel protocol `elidex-api-sw`, lifecycle, fetch interception) is the
same `LocalChannel<SwToContent, ContentToSw>` contract — drop-in beyond the two new args.

#### §4.3.7 `extract_scripts` (investigated landing spot)

boa's `extract_scripts` (`script_extract.rs:18`) is a **poorer near-duplicate** of a seam
`elidex-navigation` already owns: `resource.rs:111` `collect_scripts` walks the same
EcsDom (`TagType`/`Attributes`/`TextContent`), additionally handling external `src` scripts and the
non-JS `type` filter, feeding `LoadedDocument.scripts: Vec<ResolvedScript>` (`loader.rs:31-32`) —
which the 4th pipeline builder already consumes (`pipeline.rs:690-708`). One-issue-one-way: **delete
boa's, converge on `elidex-navigation`** — expose a pub inline-extraction entry (e.g.
`extract_inline_scripts(dom, document) -> Vec<ResolvedScript>` reusing the same walker) and point
`pipeline.rs:444,528,614` + `content_iframe_security_tests.rs:396` at it. Spec surface: HTML
**§4.12.1 The script element** (`#the-script-element`, webref-verified 2026-07-10) — the `type`
classification (classic JS vs skip) the `elidex-navigation` walker already implements is that
section's script-type model; the seam's scope (classic inline subset) is unchanged by the move.
Layering-correct: script extraction is document-load resource resolution (engine-independent,
`elidex-navigation`'s exact charter) — not VM `host/` (no marshalling), not `elidex-script-session`
(no Script↔ECS boundary concern; the session consumes sources, it does not gather them).
**Slice boundary (H8)**: the entry AND the 4 call-site swaps land in **S5-6a** (boa-compatible
output = live oracle while boa still runs); S5-6b only deletes the then-caller-less boa export with
the crate.

#### §4.3.8 The shell-visible change signal — ONE version-delta, replacing per-call bools + record taps

Three boa-shaped "did the engine change anything?" signals die at the flip: (a) B24's
`drain_and_dispatch_worker_events -> bool` needs_render return (the trait form is no-return); (b) the
flush-record stream feeding the shell's focusable-cache invalidation + iframe add/remove detection
(`content/mod.rs` re_render doc + `:360+`), which goes empty under the VM (§4.3.1); (c) assorted
per-drain `needs_render |=` bools. **One replacement (One-issue-one-way)**: snapshot the
**document-root `inclusive_descendants_version`** (`EcsDom::inclusive_descendants_version(root)`,
`dom/mod.rs:755` — the read side of the `rev_version` bump, whose ancestor propagation reaches the
root from any in-document mutation, `dom/mod.rs:710-748`) at turn start, compare after the bracket
closes; a delta drives `needs_render`, the focusable-cache invalidation (wholesale, cheap relative
to a changed turn), and an idempotent iframe **diff-scan** (the registry already knows its current
set; the initial-scan walker `iframe::scan_initial_iframes` generalizes to a diff). A
detached-subtree mutation does NOT move the root version — correct for these consumers (rendering /
in-document iframes / focusables are document-tree facts). Rejected alternative: a
`take_flushed_mutation_records` tap teeing the VM's observer records to the shell — a second record
custody duplicating observer state for consumers that only need "changed / which iframes", the exact
side-store shape the version counter dissolves.

**Bump-site coverage (round-3 H3 — every mutation class the old flush-record stream carried, ×
instrumentation site, code-verified):**

| Mutation class (old record source) | `rev_version` bump site | Covered |
|---|---|---|
| childList tree ops (insert / remove / replace / move / replace-all) | `dom/tree/mutation.rs:44,71,120,198,455` + teardown `dom/tree/teardown.rs:146,235` | ✓ |
| attribute writes (set / remove — incl. parser + `Attr`-node paths via the `EcsDom::set_attribute` chokepoint) | `dom/attribute.rs:228,309` | ✓ |
| characterData writes (Text / Comment data, `textContent` replace) | `dom/text_data.rs:55,124,189` | ✓ |
| worker-/timer-/listener-driven JS mutations | not a separate class — they ride the same `apply_*` → `EcsDom` chokepoints above | ✓ |

**No uninstrumented class found** — every record-bearing mutation class bumps the root version.
(One suppression exists by design: `version_propagation_suppressed` during `despawn_subtree`
teardown, where the PARENT bump at `teardown.rs:146,235` still fires — the subtree's own doomed
nodes are skipped, not the document-visible change.)

### §4.4 EngineMode note

The flip makes the shell pass `BrowserCompat` to a single live VM (`pipeline.rs` builders already
thread `EngineMode::BrowserCompat`; `PipelineResult.engine_mode` exists) — an `EngineMode` plumbing
surface *exists* with one value. `#11-async-core-storage-cookiestore` (2nd keystone, parallel program)
makes a second value reachable; only then does the §6 mode-plumbing cohort activate. Nothing here
blocks or is blocked by it (umbrella §6 sequencing, re-affirmed).

### §4.5 ECS-native / B1 neutrality

No new per-entity state anywhere in this PR. The two added VM queues (storage-change,
IDB-versionchange) are transient FIFO intents on per-VM HostData (event-queue standing — the
`pending_history`/`pending_window_open` precedent); `Arc<WebStorageManager>` and the SW `cache_conn`
are shared cross-cutting session resources (CLAUDE.md exception (b)). Wrapper identity stays per-VM
HostData per the interim contract (§4.1 edge; B1 migrates it with full `WrapperKey` keying, PR #434
§5). **One-Vm-per-navigation is a named invariant of this PR** (§4.1/F18 — the condition the
nav-scrub-gate retraction rests on; engine reuse across navigations is forbidden until B1).
`credentialless` / client-id / cookie-jar retention / device-facts snapshots move to **shell-owned**
state (B17–B20) — browsing-context config and session resources are shell side-stores, not engine
surface (the same exception (b)).

---

## §5 Deliverable ledger (the checklist — all cites re-verified at HEAD `7b76d722`)

> **Slice mapping (§0.1)**: the VM/trait halves of items 6–9's ADDs + the item-4 re-collection seam +
> the item-9 extraction unification + the B26 console accessor = **S5-6a**; everything else
> (swap / brackets / convergence / deletion / oracle migration / live tests) = **S5-6b**.

| # | Deliverable | Sites (verified) | Done-when |
|---|---|---|---|
| 1 | **Dep swap** | `elidex-shell/Cargo.toml:31` drop `elidex-js-boa`, add `elidex-js = { workspace = true, features = ["engine", "compat-webapi"] }` — **BOTH features**: the elidex-js feature doc mandates "a *browser* embedder must enable `engine` + `compat-webapi` together" (`elidex-js/Cargo.toml:15-22`), and `vm/host/storage.rs:55` is `#![cfg(all(feature = "engine", feature = "compat-webapi"))]` — `engine` alone ships a shell with NO Web Storage. What `compat-webapi` turns on for the shell: the Legacy Web-API glue families — Web Storage natives + `HostData` storage fields (A2) + the `elidex-storage-core/web-storage` backend (weak-dep), `document.cookie` accessor glue (A3, `vm/host/document.rs` gated blocks), and lifting the hard `Legacy`-exclusion ceiling the VM applies when the feature is off (`vm/init.rs`; gated sites across `window.rs`/`document.rs`/`structured_clone.rs`/dispatch tables). Item 12's sweep ALSO rewords this feature-doc paragraph ("the shell still runs the boa engine (S5 cutover pending)" — falsified by this row). (`engine` currently bench-only: `:201-213`) | shell compiles against `ElidexJsEngine`; both features load-bearing in CI |
| 2 | **Runtime type swap** | `lib.rs:46` use; `lib.rs:440` `PipelineResult.runtime`; chokepoint `pipeline.rs:161` `JsRuntime::with_network` (single site; builders `:444,528,614,708`) → `ElidexJsEngine` construction + `install_network_handle`/`install_idb_backend`/`install_cookie_jar`/`install_web_storage` + the **initial `set_media_environment`/`set_screen_dimensions` seed in the pre-eval install block** (§4.3.5/F4) + the shell-owned state moves (client UUID, cookie-jar retention, `DeviceFacts` snapshot, iframe `credentialless` — B17–B20). **One-Vm-per-navigation pinned by comment at the chokepoint (F18)** | one construction seam, all installs + seeds threaded; no engine reuse across navigations |
| 3 | **Batch-bind brackets** | eval loop `pipeline.rs:230-242` (+ lifecycle bodies `:305-406`); dispatch chokepoint `lib.rs:488-491`; `eval_script :494`; `drain_timers lib.rs:500-502`/`pipeline.rs:236`; `re_render` engine sub-spans `lib.rs:522-560`; event-loop turn bracket (`event_loop.rs` pump); non-nesting honored (`engine.rs:308-312`) | every trait call runs inside exactly one bracket; §4.1 coupled invariants hold; `#11-bound-safe-dispatch-dom-aliasing` closed via the bound-safe dispatch shape (the `engine.rs:317-325` gap note resolved + comment updated) |
| 4 | **CSSOM shadow-sync deletion + DOM→cascade re-collection** | deletion: `lib.rs:91,212,221-223,250-293,524-528` + `pipeline.rs:498,575,661,754`; oracles `tests.rs:849-866`; replacement (§4.2, G1 form): `elidex-dom-api::collect_document_stylesheets` with lazy per-owner version compare (`sheet_version` signals, `cssom_sheet.rs:68-76`) + the `CollectedStylesheet {parsed, version}` ECS component cache on owner entities; `re_render` calls it every frame | shadow code + tests deleted; `insertRule` rendered-outcome oracle green through the re-collection path; no-change frame = O(#owners) compares, zero re-parse (pinned); every §4.2 read site's post-flip source verified |
| 5 | **Back-channel production callers live** | B9 MQL inversion (`content/mod.rs:621`; `event_loop.rs:441,466,471`) → `set_media_environment` + `deliver_media_query_changes`; `set_screen_dimensions` + `deliver_visual_viewport_events` producers wired (S5-2 deferral) **+ the initial pre-eval seed (F4, item 2)**; history delivery bracketed (4 sites §1.3); B13 pending-focus + B14 realtime-pump deletion + B15 unbind-teardown convergence + B24 worker-drain converge + B25 observer-delivery converges (`content/mod.rs:334,342,354`) + the §4.3.8 version-delta signal replacing the per-call needs_render bools and the record-fed shell consumers | every `deliver_*` has a live bracketed shell producer; VM `is_bound` gates pass by construction; first-paint matchMedia reads real device facts; iframe diff-scan + focusable invalidation run off the version delta |
| 6 | **CE reactions** | `lib.rs:540-560` boa loop **dissolves** (§4.3.1: VM-native mutations settle CE inside the VM's own bounded checkpoints; `session.flush` is empty for them under the VM — `session.rs:114-126`); **extend `deliver_mutation_records` with the record→CE enqueue** for external records (observer-only at HEAD; single-homed classification factored into engine-indep `elidex-custom-elements` beside `CustomElementReactionConsumer` — H1; the `mutation_observer.rs:421` entry stays call-through); boa-parity interim owned by `#11-ce-reaction-mutation-observer-ordering` | CE + observer callbacks fire on the VM path for BOTH custody chains; **double-fire pin test green** (§7.2); no parallel record→CE re-implementation (one factored fn in `elidex-custom-elements`, two callers) |
| 7 | **Storage + IDB cross-tab** | emit drains ADD (§4.3.2: trait group + `vm/host/storage.rs` enqueue + `event_loop.rs:96-121` swap; changed-ness/oldValue from the `WebStorageManager` set/remove returns — F10, no storage-core extension); IDB versionchange emit (both fire sites — open-upgrade + `deleteDatabase`, §4.3.2) + receive (B6+B21, `deliver_idb_versionchange` replacing `event_loop.rs:544-560`; the shell mints the IPC `request_id` at the drain→IPC seam); persistence ADD (§4.3.3: `install_web_storage` + shell manager + `event_loop.rs:143` → `flush_dirty`); receive path bracketed (`content/mod.rs:672`); S5-4b sentinel shell test; **fallback disposition (F14)**: `fallback_local_storage` stays the test/unconfigured fallback ONLY — production oracle = a shell test pinning that pipeline-constructed engines persist through the manager (+ a debug assertion at the shell construction seam that the install ran), so a future construction site cannot silently regress to in-memory | cross-tab StorageEvent + versionchange round-trip through two VM pipelines; localStorage persists to the same on-disk files; sandboxed iframe hits the sentinel bucket; production path provably manager-backed |
| 8 | **SW thread** | `app/sw_coordinator.rs:192` → `elidex_js::vm::sw_thread::sw_thread_main` with coordinator-owned `cache_conn` + `initial_clients` (§4.3.6) | SW lifecycle/fetch tests green on the VM SW; BrowserCompat hard-derive documented at the call |
| 9 | **Dispatch + messaging + extract_scripts** | `script_dispatch_event` sites under brackets (item 3); top-level `drain_post_messages` site deleted (B5, VM self-delivers) + iframe parent-message out-queue ADD (B16, `iframe/thread.rs:113` converges); `drain_script_animations` site deleted (B22 — WAAPI = `#11-web-animations-element-animate`, S5-7, the §8 exception); `extract_scripts` unified into `elidex-navigation` (§4.3.7, HTML §4.12.1; `pipeline.rs:14,444,528,614`, `content_iframe_security_tests.rs:396`) | boa exports unreferenced; one script-extraction seam; iframe→parent postMessage round-trips on the VM |
| 10 | **Flip-inert behaviors going live** (tests, no new mechanism beyond §4.3.4's re-drain) | live popstate/hashchange firing + `history.state` deserialize (S5-5b/5c registered); scroll-vs-popstate ordering + post-handler re-drain (interim, D5-superseded); relative-nav both-orders test (boa divergence erased); `(index,length)` publish ×8 | the S5-5 §2.2 event matrix observable in the live shell; stale-index repro fixed |
| 11 | **Shell test oracle migration** | 232 `#[test]` crate-wide / 141 in the 9 pipeline-driving files; **four toucher families** (§7.1/H6): oracle-mechanism (`tests.rs:849-866` CssomMutation; `content_iframe_security_tests.rs:396` extract_scripts; `tests.rs:407,428,797,828` console → B26), stub pins (re-baseline per Qd), test-injector hooks (history-drain/window-open setters → eval-driven + `#[cfg(test)]` injection seam), getter oracles ×21 (→ shell-owned B19/B20 state / rendered outcome; I4-corrected count) + the ~16-site mechanical receiver-swap bucket | full suite green on the VM; no test imports `elidex_js_boa`; the injection seam is `#[cfg(test)]`-only |
| 12 | **Crate deletion + prose sweep** | `crates/script/elidex-js-boa/` (39,193 LoC / 121 files); root `Cargo.toml:49,196`; doc-comment reword sweep — 27 non-shell files reference the crate name in comments only (verified comment-only: `elidex-net` broker/lib, `elidex-storage-core/web_storage.rs`, `elidex-wasm-runtime/host/state.rs`, 21 `elidex-js` host/vm files incl. `sw_thread.rs`/`event_shapes.rs`/`custom_elements/flush.rs`, 2 test files). **F13 — the sweep is PROSE, not just the crate-name token**: `grep -rin "boa"` over non-deleted crates and reword every now-false present-tense claim — "the shell still runs the boa engine (S5 cutover pending)" (`elidex-js/Cargo.toml:15-22`, partially falsified by item 1 anyway), "the boa-driven shell still invokes the boa-side …" (`vm_api.rs` deliver docs, e.g. `:975` vicinity), "Mirrors the boa-side `bridge/local_storage.rs` pattern" (`elidex-storage-core/web_storage.rs:154-158` — now a dangling comparison), "VM twin of `elidex_js_boa::…`" framings (`vm/sw_thread.rs:5,70`). Historical/incident mentions (memo/ledger/ADR prose) stay | `grep -r elidex.js.boa crates/` = 0 outside git history; no present-tense "boa is live/pending-cutover" prose survives; `cargo build --workspace --all-features` green |

---

## §6 Edge matrix (review-tail pre-empt; this PR's expansion of the umbrella §7 S5-6 column)

| # | Edge (intersection named) | Owned by |
|---|---|---|
| E1 | **bind-lifetime × assume-bound × `scripts_allowed`** (umbrella §4's three coupled invariants — bracket placement, gate-read-inside-bracket, RAII pairing) | item 3 |
| E2 | **unbind-clears × cross-batch wrapper identity** (`[SameObject]` across turns; boa caches persisted, VM clears — the flip's one behavior delta class; interim accept + pin, B1 structural fix) | §4.1 / Qe |
| E3 | **CSSOM-truth swap × cascade feed × test oracle** (the shadow was the cascade's ONLY mutation feed — deleting it without the §4.2 dirty-gated re-collection silently breaks `insertRule` rendering; the shadow-struct oracle dies with its mechanism; replacement = rendered outcome through the re-collection) | items 4, 11 |
| E4 | **back-channel strangler bound** (S5-4c's forbidden form: a drain left boa-only at flip — §3.4 B1–B26 is the exhaustive inventory, two grep patterns + audit-§B reconcile; 6 trait ADDs + 1 accessor in S5-6a, the rest CONVERGE-or-DELETE in S5-6b; the crate deletion makes any miss a compile error) | items 5-9, §8 |
| E5 | **storage/IDB emit × receive duality** (storage: emit = new VM drain, receive = existing engine-agnostic payload path; IDB versionchange: BOTH halves new at the trait — wiring only one half ships half an event either way) | item 7 |
| E6 | **history `(index,length)` publish × 8 length-only sites** (partial publish = desync; whole-surface or nothing) | item 10 |
| E7 | **flip-inert → live** (every S5-4/5 "VM-tested now, live at S5-6" assertion converts to a live-shell test HERE or silently loses its oracle) | items 7, 10, §7 |
| E8 | **MQL model inversion** (shell-computed changed-list → engine-evaluated; CSSOM-View §4.2 matches-before-change ordering preserved by the VM path; deleting the shell evaluator without wiring `set_media_environment` = dead matchMedia) | item 5 |
| E9 | **SW entry parity** (4→6 params; missing `cache_conn` = SW Cache API loses the shared-conn DR-A invariant; missing `initial_clients` = `clients.matchAll()` empty) | item 8 |
| E10 | **EngineMode single value** (BrowserCompat everywhere; accidentally threading a second value pre-async-core would unshield the `#[cfg(test)]` gates) | §4.4 |
| E11 | **§4.2 stamp-write × version-compare × §4.3.8 root-version delta — BOTH directions** (false-POSITIVE: a `CollectedStylesheet` write that bumped its own compare keys would re-parse every frame AND fire spurious `needs_render` — prevented by the non-instrumented `insert_one` write seam; false-NEGATIVE (round-4 I2): a `<link>`-sheet CSSOM write-back that bumped NOTHING would be invisible to the §4.3.8 delta and lose its render on an async turn — prevented by the `<link>`-arm `rev_version` bump, §4.2 option A. Pinned from both sides: idle-frame zero re-parse/zero delta + the async-turn link-insertRule test) | items 4-5, §7.2 |

Densest: E1×E2 (the bracket) and E4×E7 (the strangler/oracle sweep) — where plan-review should press
hardest.

---

## §7 Test strategy (supported-surface declaration + oracle migration)

### §7.1 Oracle inventory (verified counts)

232 `#[test]` across `elidex-shell` (141 in the 9 pipeline-driving files: `tests.rs` 47,
`content_tests.rs` 21, `content_iframe_security_tests.rs` 17, `viewport_tests.rs` 15,
`content_window_open_tests.rs` 13, `content_fragment_nav_tests.rs` 12,
`content_history_drain_tests.rs` 12, `app_fragment_nav_tests.rs` 4, + `content_test_support.rs`
harness; remainder = chrome/ipc/quota/key_map/app/iframe/scroll/focus unit files, mostly
engine-independent). Most drive via `PipelineResult` → the swap is **type-level** (no assertion
change). One residual bucket needs no rewrite: **~16 trait-covered accessor sites in test files**
(`bridge().origin()`/`sandbox_flags()`/`set_origin()` — count grep-verified) take a mechanical
receiver swap only (the `.bridge()` receiver dies with the crate; assertions unchanged,
compile-error-guarded). Boa-internal touchers needing REWRITE — **four families** (round-3 H6
corrected the premise; the first two were the round-2 list):

1. **Oracle-mechanism touchers**: `tests.rs:849-866` (§4.2 replacement),
   `content_iframe_security_tests.rs:396` (§4.3.7 seam — swapped already at S5-6a, H8),
   `tests.rs:407,428,797,828` (console → B26 accessor).
2. **Stub pins**: boa no-fire popstate/hashchange, `deliver_history_step_events` no-op,
   `set_history_state` no-op — **invert** into fires-correctly assertions (Qd).
3. **Test-injector hooks** (bridge setters with NO trait counterpart — the trait has `take_*` only,
   deliberately): `set_pending_history`/`set_pending_navigation`
   (`content_history_drain_tests.rs:97-98,167-168,199,228,458-459` — the 12-test S5-5a drain suite
   deliberately bypasses eval to isolate drain-ORDER semantics), `set_pending_navigate_iframe`
   (`content_window_open_tests.rs:217`), `queue_open_tab` + `set_pending_navigation` (`:296-297`).
   **Migration decision: eval-driven where the scenario allows it** (a script enqueues the intents —
   tests the real path: `pushState('/a'); location.href='/b'` expresses most drain-order fixtures),
   **plus a `#[cfg(test)] `injection seam on `ElidexJsEngine`** for the fixtures eval cannot
   express (mid-drain states, e.g. a bare `HistoryAction::Back` with a preset cursor, or an intent
   that must exist WITHOUT its synchronous VM side-effects — precisely why these tests bypassed eval
   under boa too). Justification: real-path first; the injection seam is test-only surface,
   compile-gated, mirroring the hooks' current role without widening the production trait.
4. **Getter oracles against a deliberately getterless trait**:
   `device_pixel_ratio()`/`color_scheme()`/`viewport_width()`/`viewport_height()` ×19
   (`viewport_tests.rs` ×13 — the grep's 14th hit at `:839` is a test-fn NAME, not a call site;
   `content_tests.rs` ×6), `scroll_y()` (`content_fragment_nav_tests.rs:183`), `credentialless()`
   (`content_iframe_security_tests.rs:376`). Replacement oracle = the **shell-owned state B19/B20
   create** (the `DeviceFacts` snapshot / `PreEvalFrameState` / shell scroll state) or the rendered
   outcome — the trait stays getterless (B20's deliberate shape); the tests assert the shell's own
   source of truth instead of reading it back through the engine.

### §7.1a S5-6a land-time oracles (per ADD — round-3 H4)

Every S5-6a surface lands **VM/engine-indep-tested at 6a** (dead-code discipline: with no live shell
consumer until 6b, **the test IS the connection**):

| ADD | 6a land-time oracle |
|---|---|
| B3 storage-change drain | setItem/removeItem/clear enqueue with correct key/old/new/url + the bucket-`origin` (a tuple origin serializes; an opaque document carries its per-VM sentinel, never `"null"`); **same-value setItem does NOT enqueue** (§12.2.1 setItem step 3.2); drain empties FIFO in order |
| B4 `install_web_storage` | post-install, Storage natives route through the manager (value visible via `WebStorageManager::local_get`), NOT `fallback_local_storage`; un-installed VM still falls back (hermetic-test pin, H11) |
| B6 IDB emit drain | open-with-higher-version enqueues `IdbVersionChangeRequest {db, old, Some(new)}` (a fresh 0→1 creating open included); `deleteDatabase` enqueues `{db, old, None}` and a nonexistent-db delete enqueues NOTHING (§5.3 step 4 gate); drain shape/order pinned |
| B13 pending-focus | `window.focus()` sets the flag; `take_pending_focus()` returns true ONCE then false (drain semantics) |
| B16 parent-message queue | `iframe_depth > 0` ⇒ postMessage enqueues to the parent FIFO (self-delivery suppressed); depth 0 ⇒ self-delivers, FIFO stays empty |
| B21 IDB deliver | `deliver_idb_versionchange(db, old, new)` fires `versionchange` on this VM's open connections (in-VM listener observes it); no open connection ⇒ no-op |
| B26 console accessor | `console.log/warn/error` tee into the capture buffer; accessor returns them in order; buffer bounded |
| §4.2 re-collection | unit oracle in 6a: `collect_document_stylesheets` picks up a written-back `<style>`/`LinkStylesheet` source, re-parses ONLY the changed owner (a hit = an `Arc` bump — the /simplify-stage `Arc<Stylesheet>` form), an idle pass does zero re-parse + zero stamp-key movement, AND the `<link>`-arm write-back moves the root version (the I2 option-A bump) without dirtying the per-owner compare (E11, both directions) |
| §4.3.7 extraction | the 4 swapped call sites green against the `elidex-navigation` entry with boa still live (H8 — the one 6a change with a live shell oracle) |

### §7.2 New flip tests (the live conversions + additions)

- **Live popstate/hashchange** (S5-5b registered): fragment nav in the live shell fires popstate
  (state=null, sync) + hashchange (task, old/new URLs); pushState fires neither; same-doc traversal
  fires popstate with restored state; cross-doc traversal seeds `history.state`, fires neither.
- **State round-trip live** (S5-5c registered): pushState → back → `history.state` deep-equals; scroll
  restored.
- **Storage**: cross-tab StorageEvent round-trip (two content pipelines, same origin: setItem in A →
  `storage` event in B with key/old/new/url; NOT in A); no-change write fires nothing; sentinel-bucket
  shell test (S5-4b registered): sandboxed no-`allow-same-origin` iframe's localStorage isolates to
  the sentinel bucket; persistence: setItem → flush → manager state observable (+ on-disk format
  identity spot-check).
- **Relative-nav both-orders** (S5-5a deferral): `pushState('/sub/'); location.href='rel'` and the
  reverse order — resolves against the setter-time URL (VM enqueue-time resolution).
- **MQL live change delivery**: `SetDeviceFacts` → matchMedia `change` fires with post-change
  `matches` (C3 facts → VM eval; the §8 audit row's end-to-end form); **first-paint device facts**
  (F4): a pipeline built with HiDPI/dark facts → the FIRST script's `devicePixelRatio`/`matchMedia`
  reads them (the initial-seed pin).
- **New drain/deliver surfaces** (§4.3.2): `window.focus()` → `FocusWindow` notify (B13); iframe
  `parent.postMessage` → parent document `message` event, in-process + OOP (B16); cross-tab IDB
  `versionchange` round-trip emit→broadcast→deliver (B6+B21).
- **Bracket discipline**: debug-assert regression pin (nested bind panics in debug); a
  `scripts_allowed=false` turn still opens+closes its bracket (gate-inside-bracket pin).
- **Wrapper-identity pin** (E2): document the cross-batch delta with an explicit test asserting the
  CURRENT contract (identity within a task; re-fetch across tasks may differ — cite the carve).
- **CE + observers on the VM path**: a custom element upgraded by parser + reacting to script
  mutation through the VM's own checkpoints; MutationObserver callback timing unchanged;
  **double-fire pin** (§4.3.1): one bound VM-native mutation + one external-record delivery in the
  same turn ⇒ exactly ONE CE reaction + ONE observer record each (guards both partition legs).
- **Re-collection + change-signal**: `insertRule` → owner version moves → exactly the changed owner
  re-parses (no-change frame = zero re-parse, pinned); script appends an `<iframe>` → the §4.3.8
  version-delta diff-scan loads it (the record-fed detection's replacement oracle); focusable-cache
  invalidates on a `tabindex` mutation turn; **async-turn link-insertRule (the I2 false-negative —
  easy to miss because sync paths mask it)**: a timer/worker callback whose ONLY effect is
  `insertRule` on a `<link>`-owned sheet → the root-version delta fires → re-render happens → the
  rule is visible (would silently lose its render without the §4.2 option-A bump); removal
  regression: a script-removed `<style>` (detached, still-live entity) drops out of the cascade
  input via the connectedness filter (I1).
- **SW**: existing coordinator tests re-run on the VM SW thread (lifecycle, fetch interception,
  clients seeding via `initial_clients`).

### §7.3 Posture

Engine-level VM tests continue as-is (`cargo test -p elidex-js --all-features`); the shell suite is
the **behavioral-equivalence oracle** (§8). Tests run per-crate during impl (`-p elidex-shell`,
`-p elidex-js`, `-p elidex-script-session`, `-p elidex-navigation`); `mise run ci` (--all-features —
now compiling the shell WITH the engine feature, no longer bench-only) before push. WPT-subset
declarations inherit from the S5-1…S5-5 memos (this PR adds wiring, not spec surface).

---

## §8 Acceptance gate (umbrella §10-Q6, committed §11)

1. **Pre-flip regression checklist** — run `memory/boa-vm-cutover-surface-parity-audit.md` §A against
   the VM BEFORE flipping, each row with its landed evidence: matchMedia/MediaQueryList incl. change
   delivery via C3 device facts (S0 + S5-3 #440-442 + C3 #415), DOMParser/XMLSerializer (S5-1 #420),
   Screen (S5-2 #423), VisualViewport (S5-2 #423), cookieStore (S5-2 #423). A row failing = fix
   VM-side first, do not flip around it. **Premise-correction (round-2, G4)**: §A is itself
   under-enumerated — it MISSED `element.animate`/WAAPI (no row, no deliberate-exclusion record;
   the audit's own caveats admit grep misses) — so "VM strictly ≥ boa measured against §A" was
   measured against an under-enumerated ruler. The **effective flip-time oracle = §A + the fresh
   §3.4 B-sweep (B1–B26) + the full shell suite**, with B22 the ONE documented exception. **Audit §B
   reconciled row-by-row against §3.4** (this memo's sweep): §B's "PRESENT in VM … `console_output`
   … IDB `versionchange` … all exist → NOT cutover gaps" was a **grep-presence over-claim** — the
   `versionchange` hits are the in-VM event machinery (no cross-tab queue/deliver, B6/B21) and the
   `console_output` hit is a print native, not a capture buffer (B26); §B's two GENUINELY-ABSENT
   rows (CSSOM shell-sync, cookie-jar setter) are superseded by this memo's §4.2 (DOM-as-truth
   re-collection — no `set_stylesheets` port) and B18/`install_cookie_jar`.
2. **Full shell test suite green post-flip** — the equivalence oracle (§7). **ONE documented
   exception**: `element.animate`/WAAPI (B22, slot `#11-web-animations-element-animate`) — boa has a
   real surface, the VM's lands at S5-7 (umbrella-scheduled post-flip fidelity, S5-7 depends on
   S5-6). The bounded regression window is recorded in the PR description; **S5-7 lands in the first
   post-flip cohort** (with S5-5d — §0.3 ordering); no other VM<boa surface is accepted.
3. **E4 strangler audit** — no back-channel left boa-only: **the crate deletion IS the audit**
   (any missed drain = compile error), plus a grep sweep for dead trait methods / dead
   `elidex_script_session` payload types the rewiring orphaned, plus §3.4 (B1–B26) checked
   row-by-row in the PR description.
4. **`mise run ci`** (--all-features) — noting `engine` + `compat-webapi` are now load-bearing for
   the shell (item 1).
5. **Defer-ledger reconciliation** — closed slots marked, routed slots re-homed, new carves ≤3
   (§10). **Also (F19)**: `#11-cookiestore-structured-spec-faithful` (carved at #423, own project
   file) is missing from `project_open-defer-slots.md` — register it there as part of this
   reconciliation.

---

## §9 1000-line touch audit (CLAUDE.md touch-time split discipline)

Measured at HEAD (`wc -l`):

| File | Lines | Flip growth | Judgment |
|---|---|---|---|
| `lib.rs` | 666 | net ~0 (CSSOM −130, brackets +) | no split |
| `pipeline.rs` | 805 | small ± | no split |
| `content/mod.rs` | 836 | small ± (MQL inversion −) | no split |
| `content/event_loop.rs` | 623 | moderate + (turn bracket + drain swaps) | no split; headroom ample |
| `content/navigation.rs` | 889 | small + (brackets, publish swap) | no split; monitor — next substantive nav growth should take the drain/dispatch seam |
| `app/navigation.rs` | 719 | small + | no split |
| `app/sw_coordinator.rs` | 453 | moderate + (cache_conn/clients state) | no split |
| `elidex-js/src/engine.rs` | 614 | small + (2 drains + install) | no split |
| `elidex-script-session/src/engine.rs` | 501 | small + (method group) | no split |
| `tests.rs` | 982 | ± (CSSOM oracles −, some +) | **watch**: if migration pushes it past 1000, the real seam is scenario-family (CSSOM/CE/lifecycle) — split then (touch-time), not pre-emptively |
| `content_tests.rs` | 993 | ± | same watch as `tests.rs` |
| `viewport_tests.rs` | **1141 (already >1000)** | ~14 getter-oracle swaps (§7.1 family 4) — NOT pure type-level (round-3 H6 corrected the earlier wording), but each swap is a one-line assertion-source change (bridge getter → shell `DeviceFacts` snapshot), likely <50 substantive LoC net | judgment unchanged: no prereq split for THIS PR (mechanical assertion-source swaps, no scenario growth; new live tests land in their scenario files). **Watch entry sharpened**: if the oracle swaps + any drive-by exceed ~50 substantive LoC at impl contact, take the real seam (viewport transport vs MQL scenarios) as a touch-time split IN the flip PR series |
| other test files | 162–665 | new tests land here | no split |

**Recommendation (Qb)**: **no standalone prereq split PR** — nothing the flip substantively grows is
over the line with a real seam; the two ~990 files are watch-listed with a named seam if impl pushes
them over.

---

## §10 Out-of-scope / defer ledger

**Routed (existing slots, NOT new carves — not cap-counted):**
- `#11-vm-host-synthetic-dom-event-dispatch` → S5-6-post peel (§0.2, Qa).
- `#11-session-history-task-queue-model` (D5) → S5-5d, plan-reviewed kickoff immediately post-flip
  (§0.3; the flip's §4.3.4 re-drain is its documented interim).
- `#11-keepalive-event-loop-step1-snapshot` → own parallel PR.
- `#11-storage-event-mode-aware-delivery`, `#11-enginemode-full-session-threading` → umbrella §6
  mode-plumbing cohort (post-both-keystones).
- `#11-cookiestore-structured-spec-faithful` → own slot, demand-gated.
- `#11-web-animations-element-animate` → S5-7, first post-flip cohort (the B22 exception window's
  closer).
- `#11-ce-reaction-mutation-observer-ordering` → owns the algorithm-site CE enqueue that supersedes
  the §4.3.1 record-driven interim (existing slot; this memo adds its second trigger).
- S5-8 browsing-context model (`#11-browsing-context-model-window-open-postmessage`,
  `#11-windowproxy-browsing-context`) + wrapper-component migration → B1 / agent-scoped World program
  (post-S5). **The sessionStorage cross-context broadcast (§4.3.2 out-of-scope note) rides this
  hand-off** — it needs a second same-session context to exist first.

**Landing bookkeeping (G6-4)**: the four NEW shell-owned browsing-context states this PR creates —
client UUID (B17), retained `Arc<CookieJar>` (B18), `credentialless` on `PreEvalFrameState` (B19),
the `DeviceFacts` snapshot (B20) — are registered into the
`#11-browsing-context-state-ecs-components` program inventory
(`memory/project_browsing-context-state-ecs-components.md`) at landing, so the B1-era
policy-container componentization sweeps them.

**New carves (cap ≤3 — actual: 1, +1 conditional):**
- **D1 `#11-cross-batch-wrapper-identity`** (§4.1-E2): `[SameObject]` for non-Node wrappers across
  batch brackets — interim = accept + pin (unbind's cross-DOM-safety clearing; 1:1 VM↔DOM post-flip
  makes it semantically unnecessary but structurally required pre-B1). **Audit**: spec-core? yes
  (WebIDL `[SameObject]`); one-way? yes — B1's full-keyed wrapper components (PR #434 §5 req 6/7)
  subsume it, no interim mechanism to unwind; pragmatic-debt? narrow (identity-comparison across
  tasks of re-fetched non-Node wrappers — rare in real sites); repeat-signal? the B1 motivation
  itself. **Trigger**: B1 implementation PR. **Re-eval**: at B1 kickoff; backstop 2026-12-31.
- **D2 `#11-idb-cross-tab-versionchange-vm` (conditional carve — fires only if impl contact shows
  either IDB half exceeds the §4.3.2 drain/deliver-group shape)**: both halves — emit
  (`take_pending_idb_versionchange_requests`) AND receive (`deliver_idb_versionchange`) — are
  in-scope by default (B6+B21, item 7); the carve exists so a discovered coupling (e.g. the
  versionchange requires `IdbBackend` cooperation beyond a HostData queue, or the deliver must
  coordinate connection-close acks beyond the existing `IdbConnectionsClosed` reply) does not get
  hacked into the flip PR. **Why deferred (if fired)**: the flip's oracle is boa-parity wiring;
  a backend-coupled redesign is its own bounded slice, and the boa-parity interim (drain stub +
  documented no-op) keeps the E4 audit honest. **Re-eval trigger**: item 7 impl contact; also the
  IndexedDB multi-tab test work. **Backstop**: 2026-10-31.

**Not carved (dispositioned in-memo)**: on-disk localStorage format identity (impl-time spot-check,
item 7); stale §11.x storage cites beyond the touched sites (existing renumber-sweep owns them, §2.3-C1);
`viewport_tests.rs` >1000 (watch-listed with named seam, §9).

---

## §11 Open questions for `/elidex-plan-review`

- **Qa (A2c peel)** — accept peeling `#11-vm-host-synthetic-dom-event-dispatch` to S5-6-post (§0.2)?
  Deviation from the umbrella §5 bundling, argued on the equivalence-oracle purity (behavior-adding vs
  parity-preserving; boa also lacks it → regression-free omission). **Recommend: accept.**
- **Qb (prereq split) — RESOLVED at round 2 (G5), superseding round 1's "no prereq split"**: the
  1000-line audit still warrants no mechanical file split (§9 unchanged; `tests.rs`/`content_tests.rs`
  watch-listed), but the **capability prereq DOES split**: S5-6a (the 6 ADDs + re-collection seam +
  extraction unification + console accessor, flip-inert, boa live) lands BEFORE S5-6b (THE FLIP +
  deletion) — the umbrella's own type-(a) assignment rule applied to what the §3.4 sweep surfaced.
  Both slices are planned by THIS memo (§0.1); S5-6a needs no separate plan-review.
- **Qc (S5-5d timing)** — kick off `#11-session-history-task-queue-model` DURING the flip (parallel
  worktree, lands right after) vs AFTER it merges. The flip's §4.3.4 re-drain is D5's documented
  interim either way; starting during risks re-basing over the flip's `process_pending_actions`
  changes. **Recommend: after S5-6b merges, S5-5d kicks off FIRST in the post-flip cohort, with S5-7
  (`#11-web-animations-element-animate`) landing in the same cohort** (§0.3/§8-2 aligned); the flip
  PR's landing note hands off the interim sites by name.
- **Qd (oracle migration strategy)** — for tests that pinned boa **stubs** (no-fire popstate,
  `deliver_history_step_events` no-op, `set_history_state` no-op): re-baseline to assert the
  spec-correct live behavior (deleting the stub pin), never keep a dual-mode assertion. Type-level
  swap everywhere else. Any test whose assertion cannot be expressed engine-agnostically post-flip is
  a design smell to surface, not to shim. **Recommend: re-baseline, single-behavior assertions.**
- **Qe (cross-batch wrapper identity, E2/§4.1)** — ratify the interim: accept unbind's wrapper-cache
  clearing (identity within a batch, possible re-fetch inequality across batches), pin with a test,
  carve D1 to B1. Alternatives rejected in-memo: skip-clear-on-same-DOM-rebind (B1-territory
  double-refactor, umbrella §0.1 ban); session-long bracket (violates the unaliased-pointers contract
  across shell style/layout writes). **Recommend: accept + pin + D1.**
- **Qf (bracket granularity ratify)** — confirm §4.1's placement table (per-batch, not per-call; ONE
  turn bracket in the event loop; flush/style/layout outside brackets) as the uniform binding model —
  the SAME decision S5-5b §9 requires for history delivery, applied to every deliver/drain group at
  once (One-issue-one-way).

---

## §12 Workflow (two PRs, §0.1)

**S5-6a (prereq, lands first)**: own worktree/branch off main → implements this memo's §4.2/§4.3
designs (no separate plan-review — reviewed here) → per-crate tests (`-p elidex-js`,
`-p elidex-script-session`, `-p elidex-dom-api`, `-p elidex-custom-elements`,
`-p elidex-navigation`; the §7.1a oracle table) → `/pre-push` → `/external-converge` → squash merge.
boa stays live; the trait+boa-private coexistence is bounded by S5-6b (the S5-4c E4 precedent).
**Escape hatch (H12)**: if S5-6a impl contact deviates from this memo's §4.2/§4.3 designs (a seam
moves crate, a signature changes shape, a partition leg fails), it returns to `/elidex-plan-review`
BEFORE landing — the no-separate-review grant covers the reviewed designs, not their replacements.

**S5-6b (THE FLIP)**: impl in THIS worktree (`elidex-pr-s5-6`, branch `s5-6-flip`), **rebased on the
merged S5-6a** → plan-verify grep against merge-base HEAD at kickoff (this memo's cites re-checked if
main moved) → `/pre-push` (6-stage gate; cargo fmt → `mise run ci` with the now-load-bearing
`engine`+`compat-webapi` features → /simplify → /code-review → /review → /elidex-review) →
`/external-converge` (Codex, loop to TERMINAL — highest-blast PR of the program) → CI green visually
confirmed → squash merge. The §8 acceptance gate runs on S5-6b, in order: §A checklist (+ the §8-1
composite-oracle addenda) BEFORE the flip commit lands in the PR; the strangler grep + full suite
after. boa receives no fixes beyond CI-green-minimum at any point (deletion-bound).

Landing deliverables: defer-ledger reconciliation (§10, incl. the F19 cookiestore registration + the
G6-4 shell-owned-state inventory entries), the S5-5d/S5-7 first-cohort hand-off note (Qc), memory
ledger update.

**Stale concurrent branch note (G6-7)**: `origin/media-prefers-producers` (last commit `a14f3123`,
2026-06-21, unmerged) predates C2 #411 / C3 #415, which landed the same shell↔content device-facts
area after it — likely superseded. **Verify liveness at S5-6b impl start** (a superseded branch gets
flagged for deletion, not rebased over); NOT a rebase blocker.

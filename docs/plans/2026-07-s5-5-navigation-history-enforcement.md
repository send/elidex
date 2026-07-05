# S5-5 — navigation / history enforcement edge cluster (the same-document primitive program)

Per-PR-cluster plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`,
§5 row "S5-5 navigation/history enforcement edge"; §7 edge matrix nav/history column). **Anchor = the
ideal end-state**, not an incremental patch (`feedback_plan-memo-anchor-on-ideal-not-incremental`).

S5-5 is a **FLIP-precondition** cluster (umbrella §5 type-(a): land BEFORE the S5-6 boa→VM flip; boa
stays the live engine throughout, so this is VM-capability + shell-correctness work). It covers the
navigation/history edges the live shell drives once the VM is the engine: same-document (fragment /
pushState / traversal) navigation as a first-class shell path, session-history state threaded
end-to-end, popstate/hashchange actually fired, drain/apply order made spec-faithful, and
document-origin kept correct across a no-rebuild navigation. The cluster crosses **origin ×
navigation × history-traversal × focus-reset × scroll × two events × state-round-trip** (umbrella §7)
— edge-dense work with **no single canonical algorithm already in the tree**, hence this mandatory
`/elidex-plan-review` before any impl (CLAUDE.md "Edge-dense work = multi-PR program + 実装前
plan-review 必須").

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE impl. §0 answers umbrella Q2 (slice granularity)
> for S5-5; §2 pins the event-firing matrix from the actual algorithm prose; §4 settles the
> async-queue fidelity mapping + the origin-resync mechanism; §6 maps the edge matrix so plan-review
> can pre-empt the review tail.

> **Binding umbrella pre-decisions (inherited, not re-litigated here):** (1) the navigation
> *algorithms* (same-document determination, scroll-to-fragment, traversal/entry model, event-firing
> hub) land **engine-independent** — the VM `host/{location,history,navigation}.rs` stay marshal-only
> (Layering mandate; S5-4 precedent: sandbox predicate + window.open disposition → engine-indep
> `elidex_plugin::sandbox` / `elidex_script_session::navigation`); (2) **boa = light-touch**
> (deletion-bound, D-26 PR7): boa's current behavior is the parity baseline, and boa is touched ONLY
> to keep CI compiling — a contract field it gains is passed `None`, a new back-channel it gains is a
> no-op stub; no boa-side feature work; (3) **no per-VM-side-store → component migration in any S5
> PR** (umbrella §0.1; `document_origin` / `NavigationState` stay interim per-VM HostData; the
> migration is the agent-scoped-World **B1** program, post-S5, PR #434
> `docs/plans/2026-06-agent-scoped-ecsdom-world.md`); (4) **session history is a browsing-context /
> navigable fact**, not a per-entity DOM fact — the shell-owned `NavigationController` stays a
> legitimate shell side-store (CLAUDE.md ECS-native exception (b)), NOT an ECS component.

All file:line cites grep-verified against `main` HEAD `31c1f76d` (2026-07-04, the S5-4d merge). Every
spec § / anchor / algorithm-step webref-verified 2026-07-04 (source: `html` monolithic multipage);
§2.7 records the corrections where the slot ledger's or umbrella's cite shorthand was imprecise.

---

## §0 Umbrella Q2 resolution — the 4-slot cluster SUB-SPLITS into 3 slices under ONE plan-review

Umbrella §10 Q2 asked: is 4-slot S5-5 one plan-reviewed PR, or does it sub-split? **Answer: sub-split
— 3 slices, derived from the shared-core / consumer structure, not from convenience.** This is the
load-bearing decomposition call, and the coupling here is *tighter* than S5-4 (whose 5 slices lived in
five nearly-disjoint subsystems). Here three of the four slots are **consumers of one shared core**,
so the split must avoid a strangler while still not bundling ≥3 intersecting invariant axes into one
high-blast PR.

### §0.1 The architectural fact that drives the split

Fragment navigation (§7.4.2.3.3), pushState/replaceState (§7.4.4 URL and history update steps), and
history traversal (§7.4.3 → §7.4.6.1 apply the history step) are **three consumers of one shared
core**:

- the **same-document session-history-entry model** (`HistoryEntry` with its state + scroll fields
  actually populated — §3.4);
- **finalize a same-document navigation** (§7.4.2.3.3) — the commit primitive shared *verbatim* by
  fragment nav and the URL-and-history-update steps (webref-verified: "This is used by both fragment
  navigations and by the URL and history update steps"); in elidex this is
  `NavigationController::push`/`replace`;
- **update document for history step application** (§7.4.6.2) — the **event-firing hub** that fires
  popstate + hashchange and restores state + scroll; in elidex this becomes ONE shell→engine
  back-channel (§4.3).

A split that carved out a *shared-core-only* slice (the entry model + finalize + update-document hub
with NO live consumer) would be a **strangler middle state** — CLAUDE.md One-issue-one-way explicitly
forbids "a shared-core slice with no live consumer". So the shared core must land **with its first
consumer**.

### §0.2 Why not one PR, why not four

**Why not one PR.** One PR covering all four slots crosses origin × navigation × history × focus ×
scroll × 2 events × state-round-trip × contract-change × the shell drain (×2 navigation impls, §3.6) ×
VM marshalling — the #339 shape (implementation ~1 commit, review tail 30+). The ≥3 intersecting
invariant axes make findings *likelier*, multiplying the bundling cost; the edge-dense rule bans it.

**Why not four (one-per-slot).** Slot 1 (origin-resync) is not a standalone mechanism at all — it is a
*corollary invariant* of the same-document path (§4.4), discharged by-construction the moment ANY
no-rebuild path exists. Splitting slot 4 (state) from its traversal consumer would strand a dead half
(restoring state without firing popstate is unobservable; firing popstate without restored state is
spec-wrong) — a strangler. So four slices over-fragment.

### §0.3 The structure — 3 slices (base-case rule)

Base-case rule (CLAUDE.md / umbrella §0.4: a plan-reviewed narrowly-scoped per-PR slice under an
approved umbrella is a terminal unit):

| PR | Name | Closes slot(s) | Depends on | Size |
|---|---|---|---|---|
| **S5-5a** | drain history before navigation (+ Vec-drain canonicalization) | `#11-s5-shell-drain-history-before-navigation` | — | **S** |
| **S5-5b** | synchronous fragment navigation = the shared same-document primitive + hashchange/popstate firing hub + origin stable-by-construction | `#11-synchronous-fragment-navigation`, `#11-vm-navigation-origin-resync` | S5-5a (soft — drain order) | **L** |
| **S5-5c** | session-history state + traversal popstate/scroll fidelity (second consumer of the 5b primitive) | `#11-history-state-traversal-popstate-fidelity` | **S5-5b** (hard — the shared core) | **L** |

Dependency order: **S5-5a (independent, first) → S5-5b → S5-5c**. 5a de-risks by establishing the
`process_pending_actions` drain order that 5b/5c assume (and the S5-4c memo §4.3.2 already flagged
"`process_pending_actions` is shared with S5-5's drain-history-before-navigation work — whichever
lands second re-checks drain ordering"). 5b lands the shared core WITH fragment nav as its **first
live consumer** (no strangler). 5c **accretes** traversal as the **second consumer** and extends the
primitive (popstate-with-restored-state + scroll restore) — incremental membership on the 5b seam
(the `HostDriver` "Accretion" model, `engine.rs:127-130`), NOT a dual impl. Every slice is
independently shippable and boa stays live throughout.

**Plan-review economy**: this memo carries **all three slices at per-PR depth** (§5) — one 5-agent
review of this memo makes each slice a plan-reviewed terminal base case. **Exception hatch**: if
plan-review judges **S5-5c** (state serialization + the StructuredSerializeForStorage fidelity
decision, §4.5) too deep for a §5 section, it peels off into its own memo — the one slice where the
S5-3/S5-4c precedent (cluster memo → one peeled slice) could recur. **Recommendation: accept the
3-slice structure with 5b and 5c reviewed from this memo** (§5.2/§5.3 reach mechanism depth; a
dedicated memo would add nothing that §4.3–§4.5 do not already specify).

Human-PM confirmation requested at plan-review: (a) the 3-slice split (vs the 2-slice fold of 5a→5b,
§9-Q1); (b) 5b + 5c staying in-memo; (c) the engine-agnostic-now vs flip-inert classification (§4.6 /
§9-Q4) — the event *firing* is flip-inert (VM-fired, boa-stubbed), only the shell same-document *path*
is engine-agnostic-now.

---

## §1 Scope + slot map

### §1.1 What S5-5 is

The shell today treats **every** navigation as a full pipeline rebuild — including fragment-only
navigations, which §7.4.2.3.3 says are *synchronous same-document* navigations (no reload). It threads
**no** history state object to the shell (the pushState state lives only in the VM's `current_state:
JsValue` and is dropped from the `HistoryAction`, §3.3), fires **neither** popstate nor hashchange
anywhere (grep-verified: the two events are constructable + bindable but have **no** firing site,
§3.5), restores **neither** state nor scroll on traversal (§3.4), and drains a same-turn
history-mutation-then-navigation in the **wrong order** (navigation first, stranding the history
mutation, §3.2). All of this is inert or masked today because **boa is the live engine** and boa
itself does none of it (parity baseline); at the S5-6 flip each becomes a live navigation-fidelity
regression. S5-5 lands the enforcement BEFORE the flip so S5-6 swaps engines onto an already-correct
navigation surface.

### §1.2 The 4 covered defer slots (ledger verbatim → slice)

1. `#11-synchronous-fragment-navigation` → **S5-5b**. Fragment nav (same path, different `#fragment`)
   currently does a FULL pipeline reload (`handle_navigate` rebuilds regardless — the `is_fragment_only`
   flag only skips the SW check, §3.1). §7.4.2.3.3 says it is a synchronous same-document navigation:
   no reload, `update document for history step application` (fires events), `scroll to the fragment`,
   `finalize a same-document navigation`. This slot lands the **shared same-document primitive**.
2. `#11-history-state-traversal-popstate-fidelity` → **S5-5c**. The structured-serialized state object
   is NOT threaded (`HistoryAction::PushState { url, title }` drops it; `HistoryEntry.classic_history_api_state`
   is scaffolded-but-unfilled, §3.4); popstate/hashchange are never fired; traversal restores neither
   state nor scroll. Second consumer of the 5b primitive.
3. `#11-s5-shell-drain-history-before-navigation` → **S5-5a**. When one script turn produces both a
   history mutation (sync pushState) and a navigation, the shell drains navigation FIRST and
   early-returns, stranding the pushState entry (§3.2). §7.4.4 makes the URL/history update
   synchronous, so the pushState entry must commit before the async cross-document navigation
   supersedes. Standalone, low-blast; ships first.
4. `#11-vm-navigation-origin-resync` → **S5-5b** (corollary). After a same-navigable navigation with
   no pipeline rebuild, the persistent VM's `document_origin()` must stay correct (fetch/WS/ES/
   postMessage key on it). Today DORMANT because every navigation rebuilds the pipeline (fresh
   origin); becomes live exactly when same-document nav (no rebuild) is introduced — i.e. in 5b.
   S5-4d (#448) explicitly deferred this to S5-5. **Closed by-construction** (§4.4), not by an active
   resync call.

### §1.3 Non-goals (bounded out, with owners)

- **The Navigation API** (`navigation.*`, `NavigationHistoryEntry`, `navigate`/`navigatesuccess`/
  `navigateerror` events, `navigation.entries()`) — a whole separate modern surface. The
  `HistoryEntry.navigation_api_key`/`_id`/`_state` fields exist (§3.4) but the API is unimplemented and
  out of the S5-5 classic-History subset. Demand-gated with its own program; no slot owed here.
- **bfcache / `pageshow`/`pagehide` persisted state, `hasUAVisualTransition`** — elidex has no
  back/forward cache; `update document for history step application` step 6.4.3 initializes
  `hasUAVisualTransition` to true only for a UA visual-transition, which elidex never does → always
  `false`. No slot (trivially constant).
- **Cross-document traversal document reconstruction from a persisted entry** — a traversal to a
  cross-document entry rebuilds the pipeline (the existing `handle_navigate` path); reconstructing the
  *exact* prior document (vs re-fetching) is bfcache, out of scope.
- **`history.scrollRestoration` writable setter + `"manual"` mode suppression** — the getter is a stub
  returning `"auto"` (§3.3); 5c implements the `Auto` capture/restore behavior. The writable setter +
  `Manual`-mode suppression is a bounded follow-on (defer candidate §8-D3).
- **The fragment-navigation focusing step** (§7.4.6.4 scroll-to-the-fragment step 3.6 "run the focusing
  steps for target" + step 3.7 "move the sequential focus navigation starting point") — 5b lands the
  *scroll* to the indicated element; the focus-move-to-indicated-element is a refinement (defer
  candidate §8-D2), gated on the focusing-steps surface (S2 focus program).
- **Per-VM `NavigationState` → ECS component migration** — B1, umbrella §0.1 (§4.7).
- **Two-navigation-impl unification** (`content/navigation.rs` thread-mode vs `app/navigation.rs`
  inline-mode, §3.6) — both must gain the same-document path; unifying the two shell navigation
  drivers is a shell-architecture refactor beyond S5-5 (defer candidate §8-D4; One-issue-one-way
  tension noted).

---

## §2 Spec substrate (webref-verified 2026-07-04, source `html`)

### §2.1 The §heading ↔ title ↔ anchor triples (Axis 4 — all lookup-verified)

| § | Title | Section anchor | Key algorithm dfn (anchor) |
|---|---|---|---|
| §7.2.5 | The History interface | `#the-history-interface` | shared history push/replace state steps → `#shared-history-push/replace-state-steps` |
| §7.2.7.2 | The PopStateEvent interface | `#the-popstateevent-interface` | — |
| §7.2.7.3 | The HashChangeEvent interface | `#the-hashchangeevent-interface` | — |
| §7.4.2.3.3 | Fragment navigations | `#scroll-to-fragid` | navigate to a fragment → `#navigate-fragid`; finalize a same-document navigation → `#finalize-a-same-document-navigation` |
| §7.4.3 | Reloading and traversing | `#reloading-and-traversing` | traverse the history by a delta → `#traverse-the-history-by-a-delta` |
| §7.4.4 | Non-fragment synchronous "navigations" | `#navigate-non-frag-sync` | URL and history update steps → `#url-and-history-update-steps` |
| §7.4.6.1 | Updating the traversable | `#updating-the-traversable` | apply the history step → `#apply-the-history-step`; activate history entry → `#activate-history-entry` |
| §7.4.6.2 | Updating the document | `#updating-the-document` | update document for history step application → `#update-document-for-history-step-application` (the popstate/hashchange fire hub) |
| §7.4.6.4 | Scrolling to a fragment | `#scrolling-to-a-fragment` | scroll to the fragment → `#scroll-to-the-fragment-identifier` |

### §2.2 The event-firing matrix (from the actual algorithm prose — the load-bearing fact)

The single event-firing hub is **update document for history step application** (§7.4.6.2). Verified
steps (paraphrase; the load-bearing branch is verbatim below):

- step 6 (`documentsEntryChanged` = the doc's latest entry ≠ the target entry) → 6.3 **restore the
  history object state** (sets `history.state`); if 6.4 (`documentIsNew` false = same document):
  - **6.4.3 fire `popstate`** at the relevant global object with `state` = the (just-restored) history
    object state;
  - 6.4.4 **restore persisted state** (scroll);
  - **6.4.5 if oldURL's fragment ≠ entry's URL's fragment**, queue a task to **fire `hashchange`** with
    oldURL + newURL.
- step 8 (`documentIsNew` true = a fresh document, i.e. cross-document nav) → 8.3 "try to scroll to the
  fragment" — **no popstate, no hashchange**.

**Who calls the hub, and who does not** — the three consumers:

- **navigate to a fragment** (§7.4.2.3.3) step 14 **calls** update-document-for-history-step-application
  (verbatim step 14 note: "This algorithm will be called twice as a result of a single fragment
  navigation: once synchronously … history.state is nulled out, and various events are fired; and once
  asynchronously … no events are fired"). Fragment nav step 11.1 (push) **"Set history's state to
  null"**, and step 6 note: **"The classic history API state is never carried over."** ⇒ fragment nav
  fires `popstate` with **state = null** AND `hashchange` (fragment changed).
- **URL and history update steps** (§7.4.4, the pushState/replaceState core) **do NOT** call the hub.
  Verbatim §7.4.4 note: *"only fragment navigation contains a synchronous call to update document for
  history step application … this means that popstate events fire for fragment navigations, but not
  for history.pushState() calls."* Step 8 note: setting the URL here *"does not cause a hashchange
  event to be fired."* ⇒ pushState/replaceState fire **neither**.
- **apply the history step** (§7.4.6.1, traversal) calls the hub → fires popstate (restored state) +
  hashchange (if the fragment differs), unless the target is cross-document (documentIsNew=true ⇒ step
  6.4 skipped; step 6.3 still restores state, so history.state is set but no popstate/hashchange).

**The matrix S5-5 implements** (this is the review surface — the fragment-nav popstate is
counterintuitive vs pre-2021 mental models and is the #1 Axis-4 ratify-point):

| Operation | `popstate` | `hashchange` | `history.state` after | scroll |
|---|---|---|---|---|
| **Fragment nav** (`location.hash=`, `<a href="#x">`, `href=` differing only in fragment) | **YES**, state = **null** | **YES** (fragment changed) | null | scroll-to-fragment (§7.4.6.4) |
| **pushState** | NO | NO | serializedData | unchanged |
| **replaceState** | NO | NO | serializedData | unchanged |
| **Traversal → same-document entry** | **YES**, state = **restored** from entry | **YES** iff oldURL frag ≠ newURL frag | restored | restored persisted |
| **Traversal → cross-document entry** | NO (new document) | NO | **restored (step 6.3)** | scroll-to-fragment |

### §2.3 The shared commit primitive — finalize a same-document navigation (§7.4.2.3.3)

Verbatim scope note: *"This is used by both fragment navigations and by the URL and history update
steps, which are the only synchronous updates to session history."* Steps: if `entryToReplace` is null
→ clear forward history, append the entry at current-step + 1; else replace in place; then apply the
push/replace history step. In elidex this is exactly `NavigationController::push` (clear-forward +
append, `navigation.rs:75-101`) / `replace` (`navigation.rs:108-116`).

### §2.4 pushState/replaceState + state serialization (§7.2.5 → §7.4.4)

**shared history push/replace state steps** (§7.2.5, verified): step 2 fully-active gate (SecurityError);
**step 3 `serializedData = StructuredSerializeForStorage(data)`** — the state serialization point;
step 5 encoding-parse url relative to settings + can-have-url-rewritten (SecurityError); step 10 run
the URL and history update steps with serializedData. **URL and history update steps** (§7.4.4,
verified): step 3 newEntry's `serialized state = serializedData if non-null else activeEntry's classic
state`; **step 4 if `is initial about:blank` → historyHandling = "replace"** ("pushState() on an
initial about:blank Document behaves as a replaceState()"); step 6 (push) increment index, length =
index + 1; step 7 restore history object state (if serializedData non-null); step 8 set URL (**no
hashchange**); step 13 finalize a same-document navigation. Eviction (§7.2.5 note, verbatim): a UA may
limit state objects; over the limit it removes the entry immediately after the first entry for that
Document — a **FIFO buffer for eviction, LIFO for navigation** (elidex's `MAX_HISTORY_ENTRIES = 50`
drop-oldest, `navigation.rs:43`).

### §2.5 scroll to the fragment (§7.4.6.4) + traverse the history by a delta (§7.4.3)

**scroll to the fragment** (verified): resolve the document's *indicated part* (id/name match, or "top
of the document" for empty fragment) → scroll target into view (`block:"start"`), run the focusing
steps for the target, move the sequential-focus starting point. S5-5b routes the *scroll* through the
existing viewport-scroll transport (§4.6); the focusing step is a refinement (§1.3, §8-D2). **traverse
the history by a delta** (verified): queue a task on the traversable to apply the history step for the
delta-resolved target step; a delta that resolves out of range is a no-op — matching
`NavigationController::go` returning `None` (`navigation.rs:144-158`).

### §2.6 Anchor / cite corrections found (record for the plan-review spec axis)

- **C1 (fragment nav fires popstate)**: the umbrella §5 row's shorthand "popstate/traversal fidelity"
  and the task framing "fragment nav fires hashchange; traversal fires popstate" **understate** the
  spec: per §7.4.2.3.3 step 14 → §7.4.6.2 step 6.4.3, **fragment navigation fires popstate too**
  (state = null). This is the modern (post-navigables-rewrite) unified behavior and matches current
  Chrome/Firefox. The matrix (§2.2) is authoritative. **Algorithm-traced + confirmed 2026-07-04**: for
  a fresh fragment nav, *update document for history step application* step 2 `documentsEntryChanged` =
  (document's latest entry ≠ the new entry) = TRUE, step 1 `documentIsNew` = (latest entry is null) =
  FALSE ⇒ step 6.4.3 fires popstate; the state is null because the new entry never carries the classic
  history API state (step 6.3 restore → StructuredDeserialize(null) → null). Verified-by-trace, not
  asserted.
- **C2 (§7.4.6.2 hub section)**: the event-firing hub is the dfn *update document for history step
  application* (`#update-document-for-history-step-application`) under **§7.4.6.2 "Updating the
  document"** (`#updating-the-document`) — **NOT §7.4.6.1**. §7.4.6.1 "Updating the traversable"
  (`#updating-the-traversable`) holds *apply the history step* + *activate history entry* (which stay
  §7.4.6.1); the popstate/hashchange fire inside the §7.4.6.2 hub (*update document…*), not in
  *apply…* or *activate…*.
- **C3 (§7.4.4 section title)**: §7.4.4 = "Non-fragment synchronous 'navigations'"
  (`#navigate-non-frag-sync`); the operative algorithm is *URL and history update steps*
  (`#url-and-history-update-steps`). Do not cite "§7.4.4 pushState" — pushState is a §7.2.5 method that
  *delegates* to §7.4.4.
- **C4 (§7.4.2.3.3 section vs algorithm anchor)**: the section is `#scroll-to-fragid`; the algorithm is
  *navigate to a fragment* (`#navigate-fragid`); the shared commit is *finalize a same-document
  navigation* (`#finalize-a-same-document-navigation`). All three distinct.

### §2.7 Spec coverage map (cluster rows; per-slice branch detail in §5)

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §7.4.4 URL and history update steps | drain order: sync history update before async navigation | pushState/replaceState vs same-turn location nav | S5-5a | ✓ (the ordering) | yes (state/url/target) |
| HTML §7.4.2.3.3 navigate to a fragment | same-document determination; scroll; event-firing hub | fragment-only vs cross-document; push vs replace | S5-5b | ✗ (fragment subset; Navigation API = non-goal) | yes (url / location set / `<a href>`) |
| HTML §7.4.6.4 scroll to the fragment | indicated part → scroll (focus-move deferred) | id-match / name-match / top-of-document / null | S5-5b | ✗ (scroll delivered; focusing-step §8-D2) | yes (fragment string) |
| HTML §7.4.6.2 update document for history step application | popstate 6.4.3 / hashchange 6.4.5 / restore state 6.3 / restore scroll 6.4.4 | fragment-nav (null state) vs traversal (restored) vs cross-doc (none) | S5-5b (hub + fragment) → S5-5c (traversal/state/scroll) | ✓ (the fire + restore matrix) | yes (state object) |
| HTML §7.2.5 shared history push/replace state steps | StructuredSerializeForStorage; url gate; eviction FIFO | push vs replace; null vs opaque state; empty-string url special case | S5-5c (serialize → thread) | ✗ (StructuredSerializeForStorage fidelity §4.5) | yes (state object / url) |
| HTML §7.4.3 traverse the history by a delta | delta resolution; out-of-range no-op | back / forward / go(±n) / go(0) | S5-5c (state/scroll on commit) | ✓ (delta clamp) | yes (delta) |
| HTML §7.1.1 origin | document origin stable across same-document nav | override preserved / same-URL-tuple derivation | S5-5b (by-construction, §4.4) | ✓ (URL gate ≠ origin, §7.2.5) | — |

**Breadth verdict**: K = 1 spec (HTML) over M = 7 rows — inside the SPLIT-RECOMMENDED band as a
cluster; each slice carries 1–3 rows, inside the single-PR band. **User-input touch audit**
(`feedback_trust-boundary-enumerate-upfront`): every input rides an EXISTING validated seam — url
strings → the `resolve_nav_url` chokepoint (`app/navigation.rs:283`, blocks javascript:/vbscript:) +
the VM `resolve_url` seam (`location.rs:55`); the state object → the existing `structured_clone`
serialize seam (§4.5); fragment strings → `url::Url::fragment()`; delta → the existing `to_int32`
coercion (`history.rs:132`). **No new trust boundary is opened; every slice narrows or corrects an
existing one** (a fragment nav stops hitting the network at all; a stranded pushState stops being
dropped).

---

## §3 Current-state code map (HEAD `31c1f76d`)

### §3.1 Fragment nav does a full reload (S5-5b)

`crates/shell/elidex-shell/src/content/navigation.rs`, `handle_navigate` :19-186 — verified sequence:
**(1)** :27-36 compute `is_fragment_only` (current URL split-on-`#` == target split-on-`#` AND target
has a fragment); **(2)** :42 `if !is_fragment_only { …SW check… }` — the flag's **only** consumer is
the SW-skip; **(3)** :123 `elidex_navigation::load_document(url, …)` runs **regardless of
`is_fragment_only`** — a full network fetch; **(4)** :141 `build_pipeline_from_loaded` → **(5)** :151
`state.pipeline = new_pipeline` — the whole document (network + parse + render + a **fresh VM**) is
rebuilt for a fragment change. So a `#foo` navigation re-fetches + re-parses + drops all script state.
Consequence (the slot): no synchronous same-document semantics, no scroll-to-fragment, no hashchange,
and (a side bug) focus is wrongly reset because the EcsDom is rebuilt (:152-153 comment). **The
same-origin fragment case is the clean target of the same-document path** (§4.2).

### §3.2 Drain order strands a same-turn history mutation (S5-5a)

`content/navigation.rs`, `process_pending_actions` :200-245 — verified order: **(1)** :212
`take_pending_window_opens` (drained first, S5-4c); **(2)** :227 `take_pending_navigation` → on Some,
:231 `handle_navigate` then :233 **`return true` (early)**; **(3)** :236 `take_pending_history` →
`handle_history_action`. So a same-turn `pushState('/a'); location.href='/b'` enqueues
`pending_history = [PushState('/a')]` (VM already ran the sync URL update) + `pending_navigation =
'/b'`; the drain hits the navigation at step (2), early-returns, and **never reaches step (3)** — the
pushState entry is silently dropped from the shell's `NavigationController`. §7.4.4 makes the pushState
update **synchronous** (it happened during the script), so its entry must be committed **before** the
async navigation supersedes. **The fix is a reorder: history before navigation** (mirroring why
window-opens already drain first — a pipeline-replacing navigation strands anything queued after it).

⚠ **boa/VM drain-signature drift (a flip-reconciliation this slice owns)**: the shell is
**boa-concrete** (`PipelineResult.runtime: JsRuntime`, `lib.rs:440`), and boa's
`take_pending_history() -> Option<HistoryAction>` (`runtime/mod.rs:496`, `bridge/navigation.rs:40`)
drains **one**, so the call site `if let Some(action) = …take_pending_history()` (:236) applies **at
most one** history action per turn — dropping the rest even without the reorder bug. The
`HostDriver`/VM contract is `take_pending_history() -> Vec<HistoryAction>` (`engine.rs:436`,
`script-session/engine.rs:273`), draining ALL in FIFO. At the S5-6 flip the `Vec` return breaks the
`if let Some` site. S5-5a canonicalizes the shell drain to **iterate the `Vec` in FIFO order** and
adjusts boa's concrete `take_pending_history` to return `Vec` (mechanical light-touch — boa's single
slot yields a 0/1-element Vec; the change also stops boa's drop-all-but-one, harmless as boa is
deletion-bound). Same reconciliation applies to `set_history_length(len)` (boa `runtime/mod.rs:546`)
vs the trait's `set_session_history(index, length)` (§3.5 — deferred to the natural site, §5.1).

**boa relative-nav base — a deletion-bound divergence DEFERRED to the S5-6 flip (NOT fixed in 5a).**
Orthogonal to the drain signature: boa's `location.href=` / `assign` / `replace` store the **raw** arg
(`globals/location.rs`) and the shell resolves it against `pipeline.url` at drain time, so once 5a's
history-before-navigation reorder commits a same-turn `pushState` **before** the navigation drains, a
*relative* boa nav resolves against the pushState-mutated URL rather than the setter-time URL. The
**VM is correct by construction** — it resolves at enqueue (`vm/host/location.rs:136` — encoding-parse
relative to the entry settings object, HTML §7.2.4 "The Location interface") AND its `pushState` updates
`current_url` synchronously. Fully correcting *boa* would need TWO boa changes (resolve-at-enqueue **and**
`pushState` updating `current_url`, per the R3 `location.rs:155` thread) — an **edifice on the
deletion-bound engine** that §0 pre-decision (2) forbids (and the #396 self-root-check flags). So the
boa-only relative-nav base is **deferred to the S5-6 flip** (D-26 PR7 boa deletion), which erases it by
construction — **no `#11-` slot** (the flip removes the divergence; the VM shell path is unaffected).

### §3.3 State is dropped; popstate/hashchange never fired; scrollRestoration stubbed (S5-5b/c)

- **VM pushState/replaceState** — `crates/script/elidex-js/src/vm/host/history.rs`, `state_mutate`
  :147-257: runs the can-have-url-rewritten gate :201 (`elidex_plugin::can_have_url_rewritten`),
  synchronously sets `current_url` :224 + `current_state` :225 (bare `JsValue`), bumps index/length via
  `record_push_state` :239, then enqueues `HistoryAction::PushState { url, title }` :250 — **the state
  object is NOT on the action** (§7.2.5 step 3 `StructuredSerializeForStorage` is unimplemented; module
  doc :32-34 names the deferred slot). `current_state` is GC-rooted (`navigation.rs:135-138`).
- **`HistoryAction`** — `crates/script/elidex-script-session/src/navigation.rs`, `PushState`/
  `ReplaceState` :57-69 carry only `url: Option<String>` + `title` — **no state field**.
- **`history.scrollRestoration`** — `history.rs`, `native_history_get_scroll_restoration` :75-83
  always returns `"auto"`; **no setter** (RO accessor table :299-303).
- **Event constructors exist, no firing** — `crates/script/elidex-js/src/vm/host/events_extras.rs`:
  `native_hash_change_event_constructor` :440-480 (reuses the UA-dispatch `hash_change` shape,
  oldURL/newURL string slots), `native_pop_state_event_constructor` :486-519 (`state: any` slot,
  GC-rooted); registered :168/:178. **No firing site anywhere** (grep-verified: constructors +
  registration only). Event-handler IDL attrs registered `HandlerScope::Window`:
  `crates/script/elidex-script-session/src/event_handler_consumer.rs` — `onhashchange` :153, `onpopstate`
  :163.
- **UA-dispatch path (the hashchange fire mechanism)** — `elidex_plugin::EventPayload` (`event_types.rs:136`)
  has a **`HashChange(HashChangeEventInit)`** variant (:171, init struct :192) routed by
  `prototype_for_payload` (`events_misc.rs:288`) — so a UA-dispatched hashchange can ride the existing
  `EventPayload` window-dispatch path. **There is NO `PopState` variant** (`events_misc.rs` match falls
  through to `_ => event_prototype`): popstate carries `state: any` (a live `JsValue`), which the
  engine-independent `EventPayload` enum cannot hold — popstate needs a **VM-specific** delivery
  carrying the serialized state (§4.3, the key split-driver).

### §3.4 Session-history controller — scaffolded but unpopulated (S5-5b/c)

`crates/shell/elidex-navigation/src/navigation.rs`: `HistoryEntry` :17-34 already carries
`scroll_restoration: ScrollRestorationMode` :27, `scroll_position: Option<(f64,f64)>` :29,
`classic_history_api_state: Option<String>` :31 (comment: "JSON string"), `navigation_api_key`/`_id`/
`_state` :23/:25/:33 — **but `push` :75-101 and `replace` :108-116 set state/scroll to `None`/default
and NOTHING populates them**. `push` truncates forward entries :78 + drop-oldest evict :95-100 (=
`finalize a same-document navigation`, §2.3); `go_back`/`go_forward`/`go` :120/:128/:144 move the index
and return **`&url::Url` only** — they do NOT expose the target entry's state/scroll. So threading state
+ scroll needs: (a) `push`/`replace` to accept a serialized state + optional scroll; (b) a read path
for the current/target entry's state + scroll on traversal. `NavigationController` = `Vec<HistoryEntry>
+ Option<usize> index`, `MAX_HISTORY_ENTRIES = 50` :43 — synchronous, single-navigable (no async
traversal queue; §4.1 justifies the fidelity).

### §3.5 The intent contract + drain trait (the mirror surface)

- **Contract** — `crates/script/elidex-script-session/src/navigation.rs`: `NavigationRequest {url,
  replace}` :33; `HistoryAction` :49 (Back/Forward/Go/PushState/ReplaceState); the S5-4c one-queue
  precedent (`WindowOpenIntent` :131 + `window_open_disposition` :200 — the canonical form to MIRROR:
  a pure engine-indep decision fn + typed intents on the session seam, engine natives marshal-only).
- **Drain trait** — `crates/script/elidex-script-session/src/engine.rs`, `HostDriver`: navigation
  back-channel :245-301 (`set_current_url` :257 — doc :252-256 explicitly "Commits **only** the URL —
  an integrator must call `set_origin` alongside it after a **cross-origin** navigation"; `current_url`
  :262; `take_pending_navigation` :267; `take_pending_history` :273 = `Vec`; `take_pending_window_opens`
  :285; `set_session_history(index,length)` :292; `history_length` :296; `set_navigation_referrer`
  :301); the media back-channel :391-413 (`set_media_environment` :398 + `deliver_media_query_changes`
  :413 — **the state-push + deliver-turn split to MIRROR** for the history event delivery, §4.3); the
  **Accretion** doc :127-130 ("grows one cohesive method-group per capability … one home, incremental
  membership, never two ways") sanctions adding the S5-5 history-event method-group here. VM impl —
  `crates/script/elidex-js/src/engine.rs`: `set_current_url` :424 (comment :420-422 restates the
  origin-resync slot), `take_pending_history` :436 (`Vec`), `set_session_history` :444, `origin` :476
  (`document_origin()`), `deliver_media_query_changes` :559.
- **`document_origin()`** — `crates/script/elidex-js/src/vm/host/navigation.rs:347-364`: override →
  else `from_url(current_url)`, opaque → per-VM `fallback_opaque_origin`. Doc :340-346 names the
  resync slot: "the shell re-pushing `set_origin` alongside `set_current_url` after a content-thread
  navigation … is shell-side at the S5 flip → `#11-vm-navigation-origin-resync`". §4.4 shows this is a
  by-construction closure for same-document nav, not an active call.

### §3.6 Two shell navigation implementations (S5-5b/c touch BOTH)

- **Thread mode** — `content/navigation.rs`: `handle_navigate` :19, `handle_history_action` :345,
  `apply_push_replace_state` :381 (resolves url, same-origin check :387-396, updates `pipeline.url`
  :397, `push_or_replace` :398, `set_current_url` :402, `set_history_length` :406 — **drops state**,
  fires nothing). Helpers on `ContentState` (`content/mod.rs`): `notify_navigation` :146,
  `send_url_changed` :134, `send_navigation_state` :126, `push_or_replace` :155.
- **Inline app mode** — `app/navigation.rs`: `process_pending_navigation` :12 (drains navigation :18,
  history :30, window-opens drain-and-drop :43), `navigate` :53, `navigate_to_history_url` :75 (full
  reload for back/forward), `handle_history_action` :145, `apply_state_change` :255 (near-duplicate of
  `apply_push_replace_state` — **also drops state, fires nothing**), `resolve_state_url` :234,
  **`resolve_nav_url` :283 (the chokepoint, `BLOCKED_NAV_SCHEMES` :277)**.

The two are near-duplicates (One-issue-one-way tension) — both must gain the same-document path. 5b/5c
apply the change to both; the shared *primitive* (the entry-model population + the event-delivery
back-channel) is engine-independent (`elidex-navigation` + the session contract), so the duplication is
confined to the two thin drain drivers, not the algorithm. Unifying the drivers is out of scope
(§8-D4).

---

## §4 Ideal architecture

### §4.1 Fidelity mapping — collapse the async traversable queue onto the synchronous Vec+index

The spec models a **traversable navigable** with an async **session history traversal queue**;
synchronous navigations (fragment / URL-and-history-update) run OUTSIDE it and reconcile via *finalize
a same-document navigation*; *apply the history step* is invoked on the queue and *update document for
history step application* is called **twice** for a fragment nav (sync best-guess values + async final
values). **elidex maps this onto the synchronous `NavigationController` (Vec + index), and does NOT
import the traversal queue.** Justification (the fidelity is *ideal*, not a shortcut):

1. The traversal queue exists to **serialize concurrent traversals and resolve races across MULTIPLE
   navigables in one traversable**. elidex's `NavigationController` owns **one** navigable (the
   top-level browsing context; iframes are separate pipelines with their own controllers, and the VM
   models a single browsing context — `navigation.rs:83-91`). With one navigable there is no
   cross-navigable race to serialize.
2. The **double-application** (sync best-guess + async final) exists only because the async queue
   defers the real index/length. elidex applies once, synchronously, on the content thread's post-turn
   drain, reading the **real** `(index, length)` from the `NavigationController` immediately — so a
   single application with final values, firing the events once. Same observable result, no best-guess
   phase.
3. What is observable — the entry model, the event-firing matrix (§2.2), state/scroll round-trip — is
   preserved exactly. What is dropped — the *multi-navigable* queue, the twice-call — is unobservable
   machinery for a race elidex's single-navigable model cannot have.

**⚠ Correction (post-R1–R4): the single-navigable *task boundary* must be KEPT — dropping it is the root
of the review tail.** Points 1–2 correctly drop the *multi-navigable* serialization and the
*double-application* (both are machinery for a cross-navigable race the single-navigable model cannot
have). But §7.4.3 *traverse the history by a delta* also models, even for ONE navigable, a **task
boundary**: it **queues a task** to run *apply the history step* (§7.4.6.1), so a traversal runs as a
*later* task, while §7.4.4 *URL and history update steps* (pushState/replaceState) run **synchronously in
the current task**. So `history.back(); history.pushState(…)` is spec-modeled as "pushState commits
synchronously THIS task; back() runs as a LATER task" — they **phase-separate**, they do not collide. The
§4.1 collapse onto the synchronous `Vec+index` silently dropped that single-navigable task boundary too,
draining a same-turn traversal *and* the sync updates in ONE synchronous pass; the R1–R4 clauses in
`process_pending_actions` (`break` / fall-through / cursor-restore / the final `return true`) are ad-hoc
reconstructions of the missing boundary. Keeping it is **cheap on the single-writer content thread**
(defer *apply the history step* to a post-drain content-thread task); its omission is what these review
rounds hand-reconstructed. 5a is the correct-for-boa **narrow order fix** under the collapsed model
(boa's single-slot back-channel makes ≤1 action/turn reachable, §3.2); the faithful task-queued model
lands in a later plan-reviewed slice (§8-D5, `#11-session-history-task-queue-model`).

This mirrors S5-4d's mapping of the spec's async fetch pipeline onto elidex's simpler broker. **Ratify
at plan-review** (§9-Q2): is the Vec+index + synchronous-apply the right fidelity for the *entry/event*
observables (yes), while the *task boundary* is kept as the §8-D5 follow-on? (Known residual: a *same-turn
traversal-then-sync-update — `history.back(); pushState(…)` — is applied in one synchronous pass rather
than phase-separated across tasks; bounded, §6-E7, §8-D5.)

### §4.2 The same-document primitive — a first-class shell path (S5-5b)

Today `handle_navigate` is the sole path and always rebuilds. The ideal introduces a **same-document
branch** that is the elidex realization of *navigate to a fragment* + *finalize a same-document
navigation* + *update document for history step application*, engine-independent in the shell/navigation
layer:

- **Same-document determination** (engine-indep pure fn, home = `elidex-navigation` alongside the
  controller it feeds, mirroring `window_open_disposition`'s home next to its channels): given the
  current document URL and a target URL, classify — **Fragment** (the target **equals the current
  document URL when compared excluding fragments, AND the target's fragment is non-null** — the
  *navigate* algorithm §7.4.2.2 step 15, conjuncts 3-4: "url equals … exclude fragments set to true"
  AND "url's fragment is non-null" — covering fragment add / change / empty / identical-including-
  fragment), **CrossDocument** (differs in path / query / scheme / host, OR the target has **no**
  fragment — a full rebuild; fragment **removal** `/a#x → /a` and identical-with-no-fragment `/a → /a`
  are CrossDocument, since their target fragment is null). The reload distinction (`location.reload()`)
  is a call-site `cursor_op` fact, not a URL-classification. This **preserves, not generalizes,** the
  `is_fragment_only` logic (§3.1): its `url.fragment().is_some()` clause is exactly step 15's "url's
  fragment is non-null" and is already spec-correct — `/a#x → /a` (removal, target fragment null) is
  CrossDocument (a full reload, matching real browsers) and `/a#x → /a#` (emptied, target fragment
  `Some("")`) is Fragment (same-document), so nothing "must be generalized". 5b keeps the clause and
  fixes the **wiring** (§3.1: the flag's only consumer was the SW-skip; the document rebuilt
  regardless), upgrading the crude `split('#')` string compare to the `url` crate's `equals(exclude
  fragments)` compare (a robustness refinement, semantically identical for valid serialized URLs).
  *(This predicate was corrected during the S5-5b plan-review per the 5b memo §4 — the earlier
  "fragments-differ / must be generalized" framing was spec-wrong; this SoT is reconciled to navigate
  §7.4.2.2 step 15.)* (pushState/replaceState are a *separate* same-document entry point — the URL-and-history-
  update steps — not this classifier; they are already synchronous VM-side, §3.3, and only need the
  entry-model + drain wiring, not the fragment path.)
- **On Fragment**: do NOT `load_document`, do NOT rebuild the pipeline. Instead: (1) update the VM's
  `current_url` via `set_current_url` (so `location.*`/`document.URL` read the new fragment URL —
  §4.4 shows origin stays correct); (2) commit the session-history entry via
  `NavigationController::push`/`replace` (= finalize a same-document navigation); (3) **scroll to the
  fragment** through the viewport-scroll transport (§4.6); (4) fire the events via the back-channel
  (§4.3) — popstate (null state) + hashchange. Focus is NOT reset (the document persists — §4.7); the
  current wrong focus-reset (§3.1) disappears because there is no rebuild.
- The primitive lands in **5b with fragment nav as its first consumer**; **5c reuses it** for
  traversal (the traversal path calls the same entry-commit + event-delivery back-channel, extended to
  carry restored state + scroll — §4.5).

### §4.3 The event-firing hub — one shell→engine back-channel (mirror `deliver_media_query_changes`)

*Update document for history step application* is an engine boundary crossing: the shell decides WHICH
events fire and with what data (from the entry model), the engine reconstructs `history.state` and
fires at the window. This is the **state-push + deliver-turn** shape of the media back-channel
(`set_media_environment` + `deliver_media_query_changes`, `engine.rs:391-413`). Ideal end-state — a new
cohesive method-group on `HostDriver` (Accretion, §3.5), e.g.:

```rust
// elidex-script-session::engine (HostDriver) — the history-event delivery group
/// Deliver the popstate/hashchange of a same-document history-step application
/// (WHATWG HTML §7.4.6.2 "update document for history step application"): the
/// shell computes which fire from its session-history entry model; the engine
/// reconstructs history.state from `popstate_state` and fires at the Window.
fn deliver_history_step_events(&mut self, ev: HistoryStepEvents);

// elidex-script-session::navigation (engine-independent)
pub struct HistoryStepEvents {
    /// Fire popstate with this state (§7.4.6.2 step 6.4.3). `Some(None)` = fire
    /// with state=null (fragment nav); `Some(Some(bytes))` = fire with the
    /// StructuredDeserialize of the restored entry state (traversal); `None` =
    /// do not fire popstate (documentsEntryChanged is false).
    pub popstate_state: Option<Option<SerializedState>>,
    /// Fire hashchange with (oldURL, newURL) (§7.4.6.2 step 6.4.5) — `Some` iff
    /// the fragment differs.
    pub hashchange: Option<(String, String)>,
}
```

- **VM impl** (`vm/host/`, marshal-only): deserialize `popstate_state` (or use null) → set
  `navigation.current_state` → build + fire a `PopStateEvent` at the Window; if `hashchange` present,
  build + fire a `HashChangeEvent`. **Event timing (§7.4.6.2 step 6.4.3 vs 6.4.5 — load-bearing):
  popstate is "*Fire an event*" = SYNCHRONOUS within the deliver, but hashchange is "*queue a global
  task on the DOM manipulation task source … to fire*" = ENQUEUED, not synchronous. So the back-channel
  fires popstate synchronously and enqueues hashchange as a task — never both synchronously; at minimum
  popstate strictly-before-hashchange.** Fire uses the existing UA-dispatch machinery (the `EventPayload`
  window-dispatch for hashchange; a direct PopStateEvent build+dispatch for popstate, since
  `EventPayload` has no PopState variant — §3.3). This is the layering-correct site: the *fire* is
  JsValue-construction + event dispatch (host/ marshalling); the *decision* of which fires is the
  shell's (engine-indep).
- **boa impl** (light-touch): **no-op stub** (boa is deletion-bound; it never fired these events, so a
  stub is not a regression — §4.6). ⇒ the firing is **flip-inert**: VM-tested now, live at S5-6.
- 5b uses it with `popstate_state = Some(None)` + `hashchange = Some(…)` (fragment nav); 5c uses it
  with `popstate_state = Some(Some(restored))` + `hashchange = maybe` (traversal). **One method, two
  consumers — the One-issue-one-way form.**

Why NOT route popstate through `EventPayload` like hashchange: popstate's `state: any` is a live
`JsValue` the engine-independent `EventPayload` cannot carry (§3.3). The back-channel carries the
**serialized** state and the engine reconstructs — so popstate is intrinsically a VM-reconstruction, and
that is exactly why the *decision* stays engine-indep while the *reconstruction+fire* is VM-side.

### §4.4 Origin stable-by-construction across same-document nav (S5-5b, closes slot 1)

Slot `#11-vm-navigation-origin-resync` is **not an active mechanism** — it closes by construction, and
the memo's job is to prove + test the invariant, not add a resync call:

- The origin stays stable **NOT** because the nav is "same-origin by definition" — per §7.2.5 *can have
  its URL rewritten* is a **URL-component gate** (scheme / username / password / host / port), and its
  spec note is verbatim: *"only the URL of the Document matters, and not its origin. They can mismatch
  in cases like about:blank Documents with inherited origins, in sandboxed iframes, or when the
  document.domain setter has been used."* The URL gate is **not** an origin gate.
- It stays stable because `document_origin()` (`navigation.rs:347-364`) resolves to: (a) any installed
  **override** (opaque / sandboxed / inherited) — which `set_current_url` never touches — so the
  sandboxed-opaque / about:blank-inherited / `document.domain` cases key the **preserved** origin across
  the URL update; or (b) for a no-override top-level document (`content/navigation.rs:148` passes
  `None`), `from_url(current_url)` derives the **same URL-tuple origin**, since the URL differs only in
  fragment (fragment nav) or keeps scheme/host/port (pushState's §7.2.5 URL gate, `history.rs:201`).
- ⇒ after a same-document nav updates `current_url`, `document_origin()` is **unchanged**, so
  fetch/WS/ES/postMessage stay correctly keyed. **No `set_origin` re-push is needed** for the
  same-navigable case.

The active resync (`set_origin` alongside `set_current_url`) the `HostDriver::set_current_url` doc
anticipates (:252-256) is only needed for a hypothetical **cross-document navigation that reuses the
VM** — which S5-5 does NOT introduce (cross-document nav rebuilds the pipeline → fresh VM → fresh
origin). That case is S5-8/B1 (auxiliary/cross-doc contexts). **5b closes slot 1** with (i) the
by-construction invariant documented at the same-document path, and (ii) a regression test: `fetch()` /
`new WebSocket()` after a fragment nav in both a top-level and a sandboxed-opaque iframe key on the
correct (unchanged) origin. **Ratify** (§9-Q5): close by-construction vs add the defensive resync call
now.

### §4.5 State serialization + round-trip (S5-5c)

The state object must survive a **cross-document traversal** (back to a pushState'd entry in a
different document = a pipeline rebuild = a fresh VM), so it cannot stay a live `JsValue` — it must be
serialized into the engine-independent `HistoryEntry`. Flow:

1. **VM pushState/replaceState** (`history.rs` `state_mutate`): after the existing sync `current_state`
   set, **StructuredSerializeForStorage** the state (§7.2.5 step 3) via the existing
   `vm/host/structured_clone.rs` serialize seam — legitimate host/ marshalling (JsValue ↔ serialized
   form, same standing as `structuredClone`/postMessage) — and put the serialized form on
   `HistoryAction::PushState/ReplaceState { serialized_state, url, title }` (contract change; **boa
   passes `None`** — light-touch).
2. **Shell** stores it in `HistoryEntry.classic_history_api_state` via an extended
   `NavigationController::push`/`replace` that accepts the serialized state (+ the scroll position,
   §4.6).
3. **On same-document traversal** (`go_back`/`go_forward`/`go`): expose the target entry's serialized
   state + scroll (extend the return past `&url::Url`) and deliver them via `deliver_history_step_events`
   (§4.3): the VM StructuredDeserializes → `history.state` → fires popstate; the shell applies the
   scroll.
4. **`history.state` seed on document construction** (the cross-document-traversal case — by
   construction, NOT a traversal special case): document construction initializes `history.state` from
   the **current** session-history entry's serialized state (`nav_controller.current()`'s
   `classic_history_api_state`) — null-by-construction for normal nav / fragment / iframe / initial
   load, restored only when the current entry was pushState'd (i.e. a traversal landed on it). It rides
   the **existing pre-eval install block in `run_scripts_and_finalize` (`pipeline.rs:160-215`)** — the
   SAME block S5-4b (#446) added `referrer` to (`pipeline.rs:194-199`); `history.state` is one more seed
   of identical shape alongside cookie-jar / `current_url` / origin / sandbox / referrer / viewport.
   This placement is **spec-REQUIRED, not incidental**: §7.4.6.2 step 6.3 "restore the history object
   state" is gated on `documentsEntryChanged` (TRUE even for a fresh document — latest entry null ≠
   target entry) and **runs regardless of `documentIsNew`**, sequenced BEFORE step 8.4 "scripts may
   run", so the seed must precede the initial `eval` (`pipeline.rs:~217`) — a post-build seed would let
   the reconstructed document's initial scripts read a null `history.state` first (spec-wrong). Only
   popstate (6.4.3) + hashchange (6.4.5) are gated on `documentIsNew=false` (step 6.4), so a
   cross-document traversal **restores state (step 6.3) but fires neither event** (§2.2 matrix).
   Threading cost = one null-defaulted param on `build_pipeline_from_loaded` (6 call sites; 5 pass null
   by construction, only the `content/navigation.rs` `is_history_nav` cross-doc branch carries a value
   read from `nav_controller.current()`). **In-scope for 5c** — a bounded state-seed, NOT bfcache
   document reconstruction (§1.3's non-goal stays scoped to the *exact prior document*); the seam is
   proven (S5-4b precedent), so the "carve a slot if too coupled" fallback does NOT trigger.

**Serialized form** (`SerializedState`): the `HistoryEntry.classic_history_api_state` field is already
typed `Option<String>` ("JSON string", §3.4). The ideal end-state is full `StructuredSerializeForStorage`
(handles Blob/File/Map/Date/cycles); the **interim reuses the same JSON-shortcut** the worker
postMessage path already uses (`worker_scope.rs:341-361` — an explicitly-tracked deviation from full
StructuredSerialize), so the `Option<String>` field type is honored and no new serialization primitive
is invented. **Full StructuredSerializeForStorage fidelity folds into the existing worker-shortcut slot
family** (§8-D1), not a new invention. **Ratify** (§9-Q3): JSON-shortcut interim (matching the worker
precedent + the pre-typed field) vs blocking on full StructuredSerializeForStorage.

### §4.6 Scroll capture + restore (S5-5b scroll-to-fragment; S5-5c persist/restore)

Scroll routes through the **existing viewport-scroll transport** (`HostDriver::take_pending_scroll` /
`set_scroll_offset`, `engine.rs:534-540`; the content thread's `viewport_scroll`,
`content/navigation.rs:157`), never a new channel:

- **5b scroll-to-fragment**: on a Fragment nav, compute the indicated part (id/name match, or top for
  empty fragment — §2.5) in the shell/layout layer (engine-indep — the DOM + layout own element
  geometry) and set the viewport scroll offset via the existing transport. (The focusing step is
  deferred, §1.3/§8-D2.) **Scroll-application currency (load-bearing): the fragment's element-resolution
  + offset must be buffered and resolved+applied through `re_render`'s post-layout scroll-application
  (`content/mod.rs:252` pending-scroll drain, `:298-307` clamp-against-content-size + echo to `scrollX`/
  `scrollY` + document-root `ScrollState`), NOT set inline in the post-render `process_pending_actions`
  drain.** `process_pending_actions` is itself POST-render (both its only call sites —
  `content/event_handlers.rs:172→174` click / `:387→389` key — run `re_render` immediately before, and
  it is never called from the async `run_event_loop`), so this turn's DOM mutations are already flushed
  and layout is current at the drain — the hazard is NOT stale layout; it is that a scroll set inline in
  the drain and then shipped via `send_display_list()` ships a display list with the offset
  **un-applied** (the clamp / echo / `ScrollState` machinery lives only in `re_render`). Anchor on that
  same post-layout scroll seam (`content/mod.rs:245-308`, the Codex R6/F4 "apply script scrolls after
  layout is refreshed" precedent): scroll-to-fragment is **strictly harder than `scrollTo`** — it
  RESOLVES an element (not just clamps an offset), so it inherits that seam's post-layout offset
  application and must route the element-resolution through it too. §7.4.2.3.3 step 15 remains relevant
  for the id-not-yet-parsed case: *"if the scrolling fails because the Document is new and the relevant
  ID has not yet been parsed, then the second asynchronous call … will take care of scrolling."*
- **5c persist/restore**: capture the current scroll offset **on leaving** an entry (write
  `HistoryEntry.scroll_position` before a traversal moves the index) and restore it **on arriving**
  (apply the target entry's `scroll_position` via the scroll transport), per *restore persisted state*
  (§7.4.6.2 step 6.4.4). Default `ScrollRestorationMode::Auto` (§3.4). Manual-mode suppression + the
  writable setter are deferred (§8-D3).

### §4.7 ECS-native lens + focus

- **Session history = a browsing-context/navigable fact**, held in the shell-owned
  `NavigationController` — a **legitimate shell side-store** (CLAUDE.md ECS-native exception (b):
  browsing-context/session-level resource, not a single-entity fact), NOT an ECS component. This is the
  correct home; no migration.
- **Focus**: cross-document nav resets focus **by construction** (the rebuild yields a fresh empty
  `EcsDom` — `content/navigation.rs:152-153`), needing no active reset. Same-document nav does **NOT**
  reset focus (the document + its `ElementState::FOCUS` component persist) — and 5b's no-rebuild fixes
  the current *wrong* focus-reset on fragment nav (§3.1). The only in-scope focus interaction is the
  scroll-to-fragment focusing step (§7.4.6.4 step 3.6), which would MOVE focus to the indicated element
  through the canonical `ElementState::FOCUS` component (`elidex_dom_api::focus`), never an ad-hoc
  reset — and it is deferred as a refinement (§8-D2). So S5-5 adds **zero** ad-hoc focus state.
- **Storage-home neutrality**: no new per-VM per-entity state. The VM gains a `serialized_state` on an
  existing FIFO intent (transient event-queue shape, the `pending_history` standing) — B1-migration-
  neutral (§4.1 of the S5-4 memo's precedent).

### §4.8 Layering ledger (per surface)

| Surface | Home | Layer |
|---|---|---|
| same-document determination (fragment vs cross-doc) | `elidex-navigation` pure fn | engine-indep |
| finalize-same-document (entry commit) | `NavigationController::push`/`replace` | engine-indep (shell side-store) |
| event-firing DECISION (which fire, with what) | shell drain + `HistoryStepEvents` (`elidex-script-session::navigation`) | engine-indep |
| event RECONSTRUCT + FIRE | VM `vm/host/` (JsValue build + dispatch) | marshalling (host/) |
| state StructuredSerialize/Deserialize | VM `vm/host/structured_clone.rs` | marshalling (host/) |
| scroll-to-fragment (indicated part → offset) | shell/layout | engine-indep |
| scroll transport | existing `take_pending_scroll`/`set_scroll_offset` | engine boundary (exists) |
| origin stability | `document_origin()` unchanged | by-construction (no code) |

**No new algorithm in `vm/host/`** — the natives (`location.rs`/`history.rs`) stay
marshal-only; the same-document / traversal / event-decision algorithms are engine-indep; only the
event reconstruction+fire and state serialize/deserialize (both JsValue↔host marshalling) are host/.

---

## §5 Per-slice plan

> **§5.0 Touch-set line counts (1000-line touch-time discipline)**: all touched files are under 1000
> (`content/navigation.rs` = 424, `app/navigation.rs` = 294, `elidex-navigation/navigation.rs` = 386,
> `vm/host/history.rs` = 303, `script-session/engine.rs` = 466, `events_extras.rs` = 716). **No
> touch-time split obligation.** Monitor `events_extras.rs` (+ a fire path) and `engine.rs` (+ a
> method group) — both stay well under 1000; if a fire-path helper cluster forms in `events_extras.rs`,
> a `vm/host/history_events.rs` sibling is the natural cohesion seam (assess at 5b kickoff).

### §5.1 S5-5a — drain history before navigation (+ Vec-drain canonicalization)

**Scope**: (1) reorder `process_pending_actions` (`content/navigation.rs:200`) so the **history drain
precedes the navigation drain** (window-opens → **history** → navigation), applying each history action
in FIFO order before the pipeline-replacing navigation; (2) canonicalize the shell drain to iterate
`take_pending_history() -> Vec` (fixing the boa `Option`-drains-one drift, §3.2) — adjust boa's concrete
`take_pending_history`/`take_pending_navigation` shapes mechanically (light-touch) so the call site is
type-swap-stable at S5-6; (3) apply the same reorder to the inline `app/navigation.rs:12`
`process_pending_navigation`. **No same-document restructure** (that is 5b) — 5a only fixes ORDER +
drain-completeness on the existing (rebuild-based) history handling.

**§5.1.0 R-loop additions (Codex R1/R2/R3/R4, folded into 5a).** Correctness threads surfaced in
external review and land within 5a's scope. Post-R4 root-cause re-derivation: R1–R4 are **ad-hoc
reconstructions of the single-navigable task boundary the §4.1 collapse dropped** (§4.1 Correction). 5a
keeps the collapsed synchronous model — correct for the boa-reachable surface (single-slot back-channel
⇒ ≤1 action/turn, §3.2) — and fixes the ONE live-reachable defect narrowly, deferring the faithful
task-queued model to §8-D5. The threads:
**(a) supersede-signal contract** — the FIFO history drain must STOP once a same-turn traversal
(`back`/`forward`/`go`) *successfully* rebuilds the pipeline, else the remaining intents (captured from
the navigated-away document) replay onto the fresh page. So `handle_navigate` / `handle_history_action`
(content) + `navigate_to_history_url` / `handle_history_action` (inline) return a `bool` = "a traversal
load succeeded and replaced the pipeline", and the drain loops act on it. This is a **control-flow
return, no new persistent state**: a no-op or *failed-load* traversal returns `false` so the loop
CONTINUES (a failed load must not drop trailing same-turn history — Codex R2, else a stale-runtime
document loses its `pushState`).
**(a′) #283 — a supersede must `return true`, not `break`** (this slice's own regression, the ONE
live-reachable finding): 5a's history-before-navigation reorder placed the history drain BEFORE the
`take_pending_navigation()` drain, so `break`ing out of the history loop on a supersede *fell through* to
`take_pending_navigation()` — which, after a successful traversal, reads the **freshly-loaded** runtime
and drains a `location.*` the new page's initial scripts queued (`build_pipeline_from_loaded` runs them
before returning). The fix is `break` → **`return true`**: a successful traversal has already shipped its
display list, so the drain returns immediately and never touches the fresh page's nav. This restores
symmetry with the pre-5a order (which returned right after the history action) and matches the normal-nav
path. Reachability: boa's single-slot back-channel makes a multi-action FIFO turn unreachable pre-flip,
so #283 is the only live one; **#259** (drop a trailing FIFO PushState after a supersede) is post-flip
only, and **#448** (reentrant `restore_index` clobber) needs the dead SW path — both re-eval at §8-D5.
**(b) traversal atomicity → peek-then-commit** (Codex R3, superseding R3's own capture/restore):
`NavigationController::go_back`/`go_forward`/`go` moved the index BEFORE `handle_navigate` ran, so a
FAILED traversal load (which under (a) does NOT supersede) left the cursor at the failed-target index and
let a trailing same-turn `pushState` commit from the wrong index. Rather than the R3 eager-move +
`restore_index` rollback (which Codex #448 flags can clobber reentrant mutations), the traversal is made
**atomic by construction**: `NavigationController` gains `peek_back`/`peek_forward`/`peek_go` (return the
would-be target index+URL WITHOUT moving the cursor) + `commit_index` (commit the cursor to a peeked
index), and `handle_history_action` peeks → loads → commits the cursor **iff the load succeeded**. The
cursor never moves speculatively, so there is no rollback path to clobber (`current_index`/`restore_index`
are retired). One-issue-one-way: `go_back`/`go_forward`/`go` become thin eager `peek`+`commit` wrappers
for the chrome-button path.
**(c) boa relative-nav base** — a boa-only, deletion-bound divergence DEFERRED to the S5-6 flip (§3.2;
the VM is correct by construction, so it is NOT fixed on the boa engine in 5a).

**§5.1.1 Spec basis**: §7.4.4 makes the URL/history update synchronous (it happened during the script),
so a same-turn `pushState(); location.href=` must commit the pushState entry (already reflected in the
VM's `current_url`) to the `NavigationController` **before** the async navigation supersedes — exactly
why window-opens already drain first (a pipeline-replacing effect strands anything queued after it).

**Tests**: `pushState('/a'); location.href='/b'` in one turn → session history contains `/a` before the
`/b` navigation (was: `/a` dropped). `pushState('/a'); pushState('/b')` → both entries commit in order
(was: boa dropped `/b`). `replaceState` + navigation ordering. A pure-navigation turn is unchanged.

**Edges**: E1 (history-before-nav ordering — the slice); E7 (traversal + navigation same-turn — 5a's
`return true`-on-supersede makes the traversal win cleanly, #283; the faithful fix is the task boundary,
§8-D5). Adds a **supersede-signal control-flow contract** (§5.1.0(a/a′): a `bool`
"successful-traversal-rebuild" return threaded through `handle_navigate` / `handle_history_action` /
`navigate_to_history_url` + a **return-true-on-rebuild** drain) — a return value, **no new persistent
state** — plus **traversal atomicity by peek-then-commit** (§5.1.0(b): `peek_back`/`peek_forward`/`peek_go`
+ `commit_index`, the cursor moving iff the load succeeds — retiring the R3 `current_index`/`restore_index`
rollback). The §3.2 boa relative-nav base is DEFERRED to the flip, not fixed here. NOT edge-dense (single
axis) → base-case narrow, terminal under this memo's plan-review; the deeper task-queued model is the
edge-dense follow-on (§8-D5).

### §5.2 S5-5b — synchronous fragment navigation + the shared same-document primitive

**Scope**: (1) the same-document classifier (`elidex-navigation` pure fn, §4.2) promoting the
`is_fragment_only` logic; (2) the **Fragment branch** in `handle_navigate` (+ the inline
`app/navigation.rs` path): no `load_document`/rebuild — `set_current_url` + `NavigationController`
commit + scroll-to-fragment + fire; (3) the **event-firing back-channel** `deliver_history_step_events`
+ `HistoryStepEvents` on the session contract (§4.3), VM impl (fire popstate-null + hashchange), boa
no-op stub; (4) scroll-to-fragment via the viewport transport (§4.6); (5) the origin-stable-by-
construction invariant + test (§4.4). Closes `#11-synchronous-fragment-navigation` +
`#11-vm-navigation-origin-resync`.

**§5.2.1 Mechanism** (per §4.2/§4.3): a Fragment-classified navigation runs the elidex realization of
navigate-to-a-fragment: push/replace the entry (finalize-same-document), fire via the hub back-channel
with `popstate_state = Some(None)` (state nulled, §2.2) + `hashchange = Some((old, new))` (fragment
changed). The VM native path is untouched — `location.href=`/`location.hash=`/`<a href="#x">` already
enqueue a `NavigationRequest` (`location.rs:121`); the shell's drain classifies it Fragment and takes
the same-document branch instead of `handle_navigate`'s rebuild. The events are **flip-inert** (VM-fired,
boa-stubbed — §4.6): pre-flip the live boa shell does the no-rebuild + NavigationController + scroll
(engine-agnostic, observable) but does not fire (boa stub); at S5-6 the VM fires. The **shell
same-document path is engine-agnostic-now**; the **firing is flip-inert**.

**§5.2.2 Same-document determination edge cases** (the precise predicate = equals-excluding-fragments
AND the target's fragment is **non-null**, navigate §7.4.2.2 step 15, §4.2): fragment **add** (`/a →
/a#x`); **change** (`/a#x → /a#y`); **empty** (`/a#x → /a#`, and `/a → /a#` — an empty fragment ⇒
top-of-document scroll, §2.5); **remove** (`/a#x → /a` — target fragment **null** ⇒ **CrossDocument**,
a full rebuild that fires **nothing** on the fragment path, matching real browsers' reload-on-fragment-
removal); **identical excluded** (`/a → /a`, both fragments null — a **rebuild / CrossDocument**, NOT
fragment nav); path-or-query differ (`/a → /b`, `/a → /a?q`) ⇒ CrossDocument (rebuild, unchanged). Both
the removal and emptied cases are already classified **correctly** by `is_fragment_only`'s
`fragment().is_some()` clause (removal → CrossDocument, emptied → Fragment); only the **wiring** was
broken (§4.2), not the predicate. `push`/`replace`/`reload`
rides `NavigationRequest.nav_type` (`NavigationType {Push,Replace,Reload}`; `location.replace()` →
`Replace`, `location.reload()` → `Reload` — a **distinct** type, §7.4.3 `isSameDocument=false`, not a
replace). App-mode honors all three; the thread-mode drain currently collapses `Replace → Push` for the
cursor op (deferred, `#11-thread-mode-drain-replace-honoring`).

**Tests** (VM + shell integration): fragment nav does NOT re-fetch (network-request oracle: zero
requests) + fires hashchange with correct old/new URL + fires popstate with state=null; `location.href`
after fragment nav reads the new fragment URL; scroll lands on the `#id` element; **scroll-application**:
`location.hash='x'` for an off-screen `#x` → the resolved fragment offset **reaches the display list /
is applied** (clamped + echoed to `scrollX`/`scrollY`), not shipped un-applied; **focus persists**
across fragment nav (was: reset); origin unchanged after fragment nav in a top-level doc AND a
sandboxed-opaque iframe (fetch/WS key correctly — §4.4); cross-document nav still rebuilds (regression
pin). boa path: no-rebuild + NavigationController correct, no fire (flip-inert pin).

**Edges**: E2 (fragment vs cross-document classification — the slice), E3 (origin-by-construction), E5
(focus-persist-not-reset), E6 (flip-inert firing vs engine-agnostic path). Edge-dense (≥3 axes) but
terminal under **this memo's** plan-review (base-case rule; S5-4c precedent — the dense slice stayed
in-memo).

### §5.3 S5-5c — session-history state + traversal popstate/scroll fidelity

**Scope**: (1) StructuredSerializeForStorage the pushState/replaceState state (§4.5) via the existing
`structured_clone` seam + add `serialized_state` to `HistoryAction::PushState/ReplaceState` (boa passes
`None`); (2) extend `NavigationController::push`/`replace` + a traversal read path to store/expose the
serialized state + scroll on entries (§3.4); (3) on **same-document** traversal, deliver restored state
+ fire popstate (via the 5b back-channel, `popstate_state = Some(Some(restored))`) + hashchange (if
fragment differs) + restore scroll; (3b) **`history.state` seed on document construction** — the
cross-doc-traversal case rides the `run_scripts_and_finalize` pre-eval install block
(`pipeline.rs:160-215`, the S5-4b referrer-seed precedent), seeding from `nav_controller.current()`
(§4.5 step 4), restoring state but firing no popstate; (4) scroll capture-on-leave /
restore-on-arrive (§4.6); (5) the same
in `app/navigation.rs`. Depends on **5b** (the back-channel + same-document primitive). Closes
`#11-history-state-traversal-popstate-fidelity`.

**§5.3.1 State round-trip** (§4.5): pushState serializes → HistoryAction → `HistoryEntry.classic_history_api_state`;
traversal back reads it → `deliver_history_step_events` → VM deserialize → `history.state` + popstate.
The VM keeps its synchronous `current_state: JsValue` for immediate `history.state` reads after
pushState (§3.3); a traversal overwrites it with the deserialized restored value on the back-channel —
coherent (both represent the same entry's state).

**§5.3.2 Traversal classification**: a traversal whose target entry is **same-document** (same URL
modulo fragment as the current) applies the same-document path (no rebuild — restore state/scroll + fire
popstate/hashchange); a **cross-document** target rebuilds (the existing `handle_navigate`
`is_history_nav=true` path, §3.6) and **seeds** the fresh VM's `history.state` from the current entry on
construction (§4.5 step 4 — §7.4.6.2 step 6.3 restore runs regardless of `documentIsNew`), restoring
state but firing **no** popstate (popstate/hashchange are gated on `documentIsNew=false`, step 6.4;
§2.2). This reuses the 5b classifier.

**Tests**: `pushState({n:1}); pushState({n:2}); history.back()` → popstate fires with `{n:1}` +
`history.state === {n:1}`; back across a fragment-only difference fires popstate + hashchange; back to a
cross-document entry rebuilds + fires no popstate + **seeds `history.state`** (cross-doc
`pushState({n:1})` → navigate away → `history.back()` → the fresh document's `history.state === {n:1}`,
no popstate); scroll restored on same-document back; `go(0)` reload;
state survives serialize/deserialize (structured value round-trip); boa passes `None` (compile pin).

**Edges**: E4 (state serialize/deserialize round-trip + cross-document survival), E6 (flip-inert
firing), E8 (StructuredSerializeForStorage fidelity — JSON-shortcut interim). Edge-dense; terminal
under this memo's plan-review, with the §0 peel-off hatch if the §4.5 serialization design is judged
under-specified.

---

## §6 Edge matrix (review-tail pre-empt; slices × invariant axes)

Axes = the umbrella's four for this column (origin / navigation / history-traversal / focus-reset) +
the cluster-local cross-cuts this memo surfaces (scroll / event-firing / state-round-trip / flip-inert
boundary / drain-order).

| # | Edge (intersection named) | 5a | 5b | 5c |
|---|---|---|---|---|
| E1 | **drain order: sync history before async navigation** (§7.4.4 — a pushState entry stranded by a same-turn cross-doc nav); + the boa `Option`-drains-one vs VM `Vec` reconciliation | ✔ owns | reads | reads |
| E2 | **same-document vs cross-document classification** (fragment-only ⇒ no rebuild; path/query differ ⇒ rebuild) — the shared classifier | — | ✔ owns | reuses |
| E3 | **origin stable across no-rebuild nav** (`document_origin()` unchanged: override preserved / same-origin URL derivation) — by-construction, NOT an active resync | — | ✔ owns | reads |
| E4 | **state serialize/deserialize round-trip + cross-document survival** (live JsValue → serialized `HistoryEntry` → rebuilt-VM deserialize) | — | — | ✔ owns |
| E5 | **focus persists on same-document nav ≠ reset on cross-document** (same-doc keeps `ElementState::FOCUS`; 5b's no-rebuild fixes the wrong reset) | — | ✔ owns | reads |
| E6 | **engine-agnostic-now (shell same-document path) vs flip-inert (event firing)** — boa stubs the fire, VM fires; the shell path is observable pre-flip, the events are not | — | ✔ | ✔ |
| E7 | **traversal + navigation same-turn coexistence** (both rebuild the pipeline; the second drain runs on the fresh runtime → one wins). 5a's `return true`-on-supersede makes the *traversal* win cleanly — it does NOT drain the freshly-loaded page's nav (#283 fix); but a cross-document traversal + a location nav still collide in ONE synchronous drain pass under the collapsed model (§4.1). The faithful fix is the task-queued traversal (a *later* task) — reframed slot §8-D5 `#11-session-history-task-queue-model`; same-document nav (5b) removes it for the *fragment/pushState* cases | ✔ (`return true`; #283) | narrows | narrows |
| E8 | **StructuredSerializeForStorage fidelity** (full structured-clone vs the JSON-shortcut interim matching the worker precedent + the pre-typed `Option<String>` field) | — | — | ✔ owns |
| E9 | **fragment-nav popstate is counterintuitive** (§2.2 C1: fragment nav fires popstate-with-null, not just hashchange — the modern unified behavior; wiring only hashchange would be spec-wrong) | — | ✔ guard | reads |
| E10 | **two navigation impls** (`content/` + `app/` near-duplicates both need the same-document path; the *primitive* is engine-indep so the duplication is confined to the thin drivers) | ✔ | ✔ | ✔ |

**Densest slice = S5-5b** (E2+E3+E5+E6+E9) — the shared-core landing; it is why 5b is the slice with
the peel-off consideration alongside 5c (§0). E7 (traversal+nav coexistence) is the one axis no slice
fully closes: 5a fixes its live-reachable facet (#283 `return true`), and the faithful resolution — the
task-queued traversal boundary — is the reframed §8-D5 slot (`#11-session-history-task-queue-model`),
which subsumes #259/#283/#448 + E7 and lands in a later plan-reviewed slice (edge-dense).

---

## §7 Test strategy (supported-surface declaration)

Boa stays the live shell engine throughout S5-5, so the oracles are engine-level VM tests + targeted
shell integration (the S5-3/S5-4 posture), with the **engine-agnostic-now vs flip-inert** split
(§4.6) made explicit per assertion:

- **Engine-agnostic-now** (observable in the live boa shell): no-rebuild on fragment nav (network-request
  count = 0), `NavigationController` entry/state correctness, scroll landing, focus persistence, origin
  stability (fetch/WS keying). These are shell-integration tests that pass pre-flip.
- **Flip-inert** (VM-tested now, live at S5-6): popstate/hashchange **firing** + state deserialize →
  `history.state`. VM integration tests (`cargo test -p elidex-js --all-features`) drive
  `deliver_history_step_events` and assert the events fire with the right state/URLs; a shell test pins
  boa's no-fire (stub) as the pre-flip baseline. **Registered S5-6 flip deliverable**: add the live
  shell popstate/hashchange test once the VM is the engine (mirrors S5-4b's storage-sentinel deferral).
- **`elidex-navigation` unit**: the same-document classifier truth table — **add / change / empty /
  identical-incl-fragment ⇒ Fragment (SameDocument); removal / path-or-query differ / identical-
  excluding-fragment ⇒ CrossDocument** — exercising the equals-excluding-fragments-AND-target-
  fragment-non-null predicate (navigate §7.4.2.2 step 15), with explicit rows for the
  fragment-**removal** (`/a#x → /a` ⇒ **CrossDocument**, target fragment null) and **emptied**
  (`/a#x → /a#` ⇒ Fragment, target fragment `Some("")`) cases — both classified **correctly** by
  `is_fragment_only`'s non-null-fragment clause, NOT "missed" — and the identical-exclusion
  (`/a → /a` ⇒ CrossDocument, ≠ Fragment); `NavigationController` push/replace with state +
  scroll; traversal read path exposing target-entry state/scroll; eviction FIFO with state.
- **WPT subset declaration**: the supported surface maps to `html/browsers/history/the-location-*` +
  `html/browsers/history/the-History-object/*` (pushState/replaceState/popstate/hashchange) +
  `html/browsers/browsing-the-web/history-traversal/*` — tracked as engine-independent equivalents
  (elidex-wpt harness scope judged at impl; the unit/integration coverage above is the regression gate
  per "Supported-surface testing").
- Per-PR workflow: plan-verify grep against HEAD → impl in isolated worktree → `/pre-push` →
  `/external-converge` → squash merge (umbrella §11).

---

## §8 Deferred carves (+ audits; cap ≤3 per PR — actual: 5a = 1 (D5, reframed); 5b = 3 (D2, D6, D7); 5c = 2 (D1, D3); shared audit D4)

- **D1 `#11-history-state-structured-serialize-fidelity`** (carved by S5-5c, or FOLD into the existing
  worker-shortcut slot family): full `StructuredSerializeForStorage` for `history.state` (Blob / File /
  Map / Date / cyclic graphs) vs the JSON-shortcut interim (§4.5). **Audit**: spec-core? yes (§7.2.5
  step 3); one-way? yes — the interim serializes/deserializes through one seam; upgrading swaps the seam
  body, the `HistoryEntry` field + round-trip unchanged; pragmatic-debt? the interim drops non-JSON
  state shapes (rare for classic History state — usually plain objects) — matches the **already-tracked**
  worker-postMessage JSON-shortcut deviation (`worker_scope.rs:341-361`), so it folds there rather than
  minting a fresh invention; repeat-signal? the same shortcut recurs at worker/SW/IndexedDB storage.
  **Trigger**: full structured-clone-to-storage-bytes work (the worker-shortcut slot's trigger).
  **Re-eval**: with the worker-shortcut slot; backstop **2026-10-31**.
- **D2 `#11-fragment-navigation-focusing-step`** (carved by S5-5b): §7.4.6.4 scroll-to-the-fragment step
  3.6 "run the focusing steps for target" + step 3.7 "move the sequential focus navigation starting
  point" — 5b lands the scroll, not the focus move. **Audit**: spec-core? yes (§7.4.6.4); one-way? yes —
  the focus move routes through the canonical `ElementState::FOCUS` at the same scroll-to-fragment site;
  pragmatic-debt? the interim scrolls-without-focusing (a `#id` jump does not move keyboard focus to the
  target) — safe, common-case-correct (most fragment jumps are scroll-only); repeat-signal? the focusing
  steps are the S2 focus program's surface. **Trigger**: the focusing-steps / sequential-focus-navigation
  surface (S2 focus). **Re-eval**: at the S2 focus program; backstop **2026-10-31**.
- **D3 `#11-history-scroll-restoration-manual-mode`** (carved by S5-5c): the writable
  `history.scrollRestoration` setter + `"manual"` mode suppression of auto scroll restore (§1.3). 5c
  implements the `Auto` capture/restore; the getter stays `"auto"` (§3.3) until the setter lands.
  **Audit**: spec-core? yes (§7.4.1.1 scroll restoration mode); one-way? yes — the mode is already a
  `HistoryEntry` field (`scroll_restoration`, §3.4); the setter writes it + the restore consults it;
  pragmatic-debt? interim always auto-restores (a page opting out of scroll restoration is not honored —
  minor, rare); repeat-signal? none. **Trigger**: a site/WPT exercising manual scroll restoration.
  **Re-eval**: 2026-10-31.
- **D4 (audit, no new slot — feeds the shell-arch backlog)**: `content/navigation.rs` +
  `app/navigation.rs` are near-duplicate navigation drivers (§3.6); S5-5b/c apply the same-document
  change to both. Unifying the two shell navigation drivers is a shell-architecture refactor (the
  primitive is already engine-indep, so only the thin drivers duplicate). **Audit**: one-issue-one-way
  tension is real but the duplication is confined + the shared algorithm is single-homed; unifying is
  out of S5-5 scope and out of the flip critical path. **Disposition**: note for the shell-arch backlog;
  not carved as a `#11-` slot (no spec surface — pure code-org), re-audited when the inline/thread shell
  split is next touched.
- **D5 `#11-session-history-task-queue-model`** (reframed from `#11-traversal-navigation-same-turn-race`;
  carved by S5-5a): **implement the spec's task-queued traversal** — *apply the history step* (§7.4.6.1)
  as a **deferred post-drain content-thread task**, so a same-turn traversal phase-separates from the
  synchronous history/navigation updates (*URL and history update steps* §7.4.4 commit in the current
  task; *traverse the history by a delta* §7.4.3 queues the traversal as a *later* task) instead of
  colliding in ONE synchronous `process_pending_actions` drain pass. This is the faithful single-navigable
  task boundary the §4.1 collapse dropped (§4.1 Correction) and that R1–R4 hand-reconstructed with ad-hoc
  clauses. **This one (renamed) slot subsumes**: **#283** (drain fall-through onto the freshly-rebuilt
  runtime — *fixed* narrowly in 5a by `return true`, but the structural fix is the task boundary);
  **#259** (drop a trailing FIFO PushState after a supersede — post-flip only, unreachable under boa's
  single-slot back-channel, re-evals here); **#448 + the R5-review SW-pump held-peek vector** (the SW-fetch
  synchronous message pump in `handle_navigate` can re-dispatch a nav-mutating message during its blocking
  wait, mutating `nav_controller.entries` mid-traversal: #448's `restore_index`-clobber form was retired in
  5a by peek-then-commit, but peek-then-commit's OWN exposure — a held `target_index` staling before
  `commit_index` — remains; both are the SAME SW-reentrancy root, **unreachable today** [SW controller path
  dead — `sw_controller_scope()` always `None`], `commit_index`'s `debug_assert` backstops the out-of-range
  case, and both re-land at the SW-interception / async-event-loop wiring, M4-10); the **chrome-button
  traversal** stays eager (`event_loop.rs` `go_back()`/`go_forward()` move BEFORE `handle_navigate`, with no
  rollback on a failed load — a pre-existing atomicity gap, behaviourally unchanged by 5a, that the unified
  task-queued model closes for all traversal entry points); and the **E7** traversal+navigation same-turn
  race. **Audit**: spec-core? yes (the task boundary is §7.4.3/§7.4.6.1, not optional machinery — only the
  *multi-navigable* serialization is elidex-inapplicable); one-way? yes — the task-queued model subsumes all
  the folded concerns (drain-order, FIFO, reentrancy, chrome atomicity) at one seam; pragmatic-debt? 5a's
  collapsed synchronous drain is correct for the boa-reachable surface (≤1 action/turn) but not for the
  post-flip multi-action turn; repeat-signal? every review round R1–R5 re-derived a facet of the missing
  boundary. **Lands in a future plan-reviewed slice** (5c, or a
  dedicated 5d — edge-dense per CLAUDE.md, must not ride a narrow PR): #259 re-evals there; #448 re-evals
  at the S5-6 flip / SW-interception (M4-10) reentrant wiring. **Trigger**: the multi-action drain
  (post-flip), the SW-interception reentrant path, or a site/WPT exercising mixed-turn traversal+nav.
  **Re-eval**: at 5c/5d kickoff; backstop **2026-10-31**.
- **D6 `#11-thread-mode-drain-replace-honoring`** (carved by S5-5b, plan-review R2): the `NavigationType`
  enum CONVEYS `Replace`, but the thread-mode drain maps `Replace → HistoryCursorOp::Push` (collapsing the
  replace-vs-push cursor distinction), so a thread-mode `location.replace()` navigation adds a
  session-history entry instead of replacing it — a **pre-existing** gap (NOT 5b-introduced; the drain
  hardcoded `Push` pre-5b), and app-mode already honors the full enum. **Audit**: spec-core? yes
  (historyHandling push/replace, §7.4.4/§7.4.2.2); one-way? yes — extend the same `nav_type → cursor_op`
  map the `Reload → Keep` case uses (needs a `HistoryCursorOp` replace-equivalent, touching the 5a-owned
  drain); pragmatic-debt? interim = thread-mode `location.replace()` pushes (a `history.length` off-by-one,
  rare/minor; app-mode is correct); repeat-signal? the nav-type conveyance (reload fixed, replace the
  sibling). **Trigger**: a site/WPT exercising thread-mode `location.replace()` history semantics, or the
  next thread-mode nav-drain touch. **Re-eval**: backstop **2026-10-31**.
- **D7 `#11-iframe-fragment-navigation`** (carved by S5-5b, plan-review R2): iframe fragment navigations
  still full-rebuild. The iframe nav path is a **distinct 3-arg** `handle_navigate(pipeline, url, channel)`
  (`content/iframe/thread.rs`), separate from the top-level/app-mode `handle_navigate` 5b touches; 5b's
  `#11-synchronous-fragment-navigation` closure covers **top-level + app-mode only**. **Audit**: spec-core?
  yes (§7.4.2.3.3 applies per-navigable, iframes included); one-way? yes — the same-document primitive
  (classifier + branch + back-channel) is engine-indep, so the iframe path consumes it once wired;
  pragmatic-debt? interim = iframe `#frag` navs rebuild (loses same-document semantics + focus/scroll, but
  safe); repeat-signal? the OOP-iframe nav surface (S5-4b iframe origin, S5-8 browsing-context).
  **Trigger**: iframe same-document nav fidelity work / the OOP-iframe surface (S5-8). **Re-eval**: backstop
  **2026-10-31**.

**Not carved (dispositioned in-memo, no slot)**: `hasUAVisualTransition` (always false, §1.3);
Navigation API (non-goal, own program, §1.3); bfcache / cross-document-entry document reconstruction
(non-goal); pushState-on-initial-about:blank → replace (§2.4 step 4 — small, folds into 5c's
same-document entry handling if `is initial about:blank` is representable, else a one-line audit note —
**verify at 5c kickoff**, §9-Q6). Defer-ledger reconciliation (closing the 4 covered slots + registering
D1/D2/D3/D5/D6/D7) is a landing deliverable of the respective slices.

---

## §9 Open questions for `/elidex-plan-review`

- **Q1 (decomposition, PM + design)** — accept the **3-slice** split (5a / 5b / 5c, §0)? The alternative
  is a **2-slice fold** (5a's drain-order into 5b) — defensible only if plan-review judges 5a too thin
  to stand alone; but 5a is independently correct + valuable (fixes a live drop-the-history bug) +
  de-risks 5b/5c by establishing the drain order they assume, so **recommend 3**. The further
  alternative (1 PR) is rejected (#339-shape blast radius, §0.2). Also confirm 5b + 5c stay in-memo (vs
  the §0 peel-off hatch for 5c).
- **Q2 (async-queue fidelity)** — ratify the §4.1 mapping: elidex's **synchronous Vec+index
  `NavigationController` + one-shot apply** is the right fidelity, and the spec's session-history
  **traversal queue + twice-called update-document** is unobservable machinery for a multi-navigable
  race the single-navigable model cannot have. Any observable behavior that *requires* the queue?
  (Known residual: the pathological same-turn traversal-then-sync-update, E7/D5.)
- **Q3 (state serialization fidelity, 5c)** — ratify the **JSON-shortcut interim** for
  `StructuredSerializeForStorage(history.state)` (matching the existing worker-postMessage precedent +
  the pre-typed `Option<String>` field), with full structured-clone-to-storage folded into the
  worker-shortcut slot (§4.5 / D1)? Or block 5c on full StructuredSerializeForStorage?
- **Q4 (engine-agnostic-now vs flip-inert, PM)** — ratify the classification (§4.6): the shell
  same-document **path** (no-rebuild / NavigationController / scroll) is engine-agnostic-now (observable
  in the live boa shell), while the event **firing** (popstate + hashchange) is **flip-inert** (VM-fired,
  boa no-op stub, VM-tested now, live at S5-6). This means fragment nav in the live boa shell will
  not-fire the events until the flip — a non-regression (they are unfired today) but a visible-only-after-
  flip behavior. Accept, or does plan-review want boa to fire hashchange via `EventPayload` pre-flip
  (rejected as boa feature-work under light-touch, but a genuine fork)?
- **Q5 (origin-resync mechanism, 5b)** — close slot 1 **by-construction** (`document_origin()` unchanged
  by-construction — installed override preserved / same-URL-tuple derivation, §4.4; invariant +
  regression test), or add the defensive
  `set_origin`-alongside-`set_current_url` call now? Lean **by-construction** (the active resync is dead
  plumbing until a cross-document same-VM nav exists = S5-8/B1); ratify.
- **Q6 (fragment-nav popstate — the spec ratify-point, 5b)** — confirm the §2.2/C1 matrix: a **fresh
  fragment navigation fires popstate (state = null)** in addition to hashchange (§7.4.2.3.3 step 14 →
  §7.4.6.2 step 6.4.3; the §7.4.4 note is verbatim). This is counterintuitive vs pre-2021 mental models
  and is the load-bearing Axis-4 fact; wiring only hashchange (or only popstate) would be spec-wrong in
  either direction. Also confirm the pushState-on-initial-about:blank → replace edge (§2.4 step 4) is a
  5c fold vs a §8 audit note (representability of `is initial about:blank` — verify at 5c kickoff).

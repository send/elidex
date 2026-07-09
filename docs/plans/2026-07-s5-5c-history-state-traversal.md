# S5-5c — session-history state + traversal popstate / scroll fidelity

Per-PR plan-memo (impl-PR-同梱) for the **S5-5c** slice of the S5-5 navigation/history cluster — the
**second consumer of the 5b same-document primitive**.

- **Cluster SoT** (decomposition §0, spec substrate §2, edge matrix §6, deferred carves §8):
  `docs/plans/2026-07-s5-5-navigation-history-enforcement.md`. This memo carries the **5c slice at
  impl-ready depth**, re-grounded on the current HEAD, and **corrects one load-bearing design decision in
  the cluster memo** (§4.1 — the serialized-state representation, now that 5b has landed the wire type).
- **5b memo** (the primitive this slice reuses): `docs/plans/2026-07-s5-5b-fragment-navigation.md`. 5c
  reuses 5b's `classify_navigation` classifier (**only on the fresh-fragment `Push` path** — the traversal
  path uses per-entry `document_sequence` identity, §0-CR CR-1), the `deliver_history_step_events`
  back-channel, and the `fragment_navigate` no-rebuild primitive — **incremental membership on the 5b seam**
  (cluster §0.3, the
  `HostDriver` "Accretion" model), NOT a dual impl.
- **Umbrella**: `docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md` (§5 row S5-5). 5c is a
  **FLIP-precondition** slice — boa stays the live engine; VM-capability + shell-correctness work landing
  BEFORE the S5-6 boa→VM flip.
- **Gate**: `/elidex-plan-review` BEFORE impl (CLAUDE.md "Edge-dense work = 実装前 plan-review 必須"; 5c is
  edge-dense — cluster §6 E4+E6+E8).
- **Anchor = the ideal end-state** (`feedback_plan-memo-anchor-on-ideal-not-incremental`).

**Re-grounding**: cluster-memo cites were verified against **pre-5a** HEAD `31c1f76d`; the 5b memo against
`539a09ba`. **S5-5b (#451, `a904ea81`) landed the same-document primitive** (`classify_navigation`,
`fragment_navigate`, `deliver_history_step_events`, `HistoryStepEvents`, `SerializedState = Vec<u8>`, the
`reconstruct_history_state` traversal stub) + shifted line numbers. **All cites here are grep-verified
against HEAD `a904ea81` (2026-07-09)**; every spec §/anchor/algorithm step is `webref`-verified 2026-07-09
(source `html` multipage); §4.1's representation correction is verified against the landed 5b wire type.

---

## §0-CR — R1 code-review corrections (document-identity redesign) — SUPERSEDES §6.4's classifier

> Authored 2026-07-09 after the first implementation's `/pre-push` Stage-4 `/code-review high` returned
> **6 confirmed findings**, the headline one **design-level**. This section is the authoritative
> correction and **supersedes §6.4/§4.5's `classify_navigation`-for-traversal mechanism** and §6.3/§6.6's
> serialize/scroll placement where noted. It is the input to a **re-`/elidex-plan-review`** (edge-dense: a
> new invariant axis — per-entry document identity — is introduced). Matches
> `feedback_plan-review-verify-preserve-existing-spec-claims`: the plan's reuse of an existing helper
> (`classify_navigation`) was spec-wrong; re-verified against `webref html`.

### CR-1 (headline) — traversal same-document determination = per-entry **document identity**, NOT URL

§6.4 said a traversal classifies same-vs-cross-document via 5b's `classify_navigation(current_url,
target_url)`. **That is spec-wrong for a traversal.** `classify_navigation` is the *navigate* algorithm
step-15 predicate (equal-excluding-fragments **AND** target fragment non-null) — correct for a **fresh
fragment navigation**, but a **traversal**'s same-document-ness is a fact about **document identity**, not
URLs:

- `pushState(s1,'','/products'); pushState(s2,'','/products/2'); back()` — the entries have **different
  paths but are the same document** (the entire point of SPA `pushState` routing). URL comparison →
  CrossDocument → full network rebuild, **no popstate**. Broken.
- A no-fragment `pushState` back (the memo's own §5.3.2 / §9 headline test) → CrossDocument → rebuild.
- **There is no URL-based fix**: `url_equals_excluding_fragments` alone would mis-classify "same URL,
  *different* document" (a fresh-nav to `/a`, later `back()` to an earlier `/a` entry) as same-document →
  a stale document. Document identity is irreducible.

**Correction — the `document_sequence` model** (spec's session-history-entry *document* field, modeled as
a monotonic id):

- `HistoryEntry` gains `document_sequence: u64`. Two entries are the same document ⇔ equal
  `document_sequence`.
- `NavigationController`: a `next_document_sequence` counter. **Document-identity re-stamping is symmetric
  across ALL THREE entry operations (push / replace / reload)** — NOT only push (plan-review R2 Axis-2
  CRIT: the first draft's blanket "`replace` keeps sequence" mis-stamped the one cross-document `replace`
  caller, re-introducing the stale-document bug on the replace axis):
  - **`push(url)` = a NEW document** (fresh `document_sequence`) — cross-document navigation, initial load.
  - **`push_same_document(url)` = the CURRENT document** (new entry **inherits** the current entry's
    `document_sequence`) — `pushState` + the fresh **fragment** push.
  - **`replace(url)` = a NEW document** (replace the current entry's URL in place AND stamp a fresh
    `document_sequence`) — `location.replace()` (app-mode `NavigationType::Replace`), reached only AFTER
    the same-document early-return, so it is by construction a cross-document replace (it rebuilds).
  - **`replace_same_document(url)` = the CURRENT document** (replace in place, KEEP the sequence) —
    `replaceState` + the equal-URL fragment replace.
  - **reload re-stamps** (plan-review Axis-2 IMP): `NavigationType::Reload` replaces the navigable's
    *document* (isSameDocument=false) without moving the cursor or creating an entry, so a
    `restamp_current_document()` gives the current entry a fresh `document_sequence` — else a neighbor entry
    that shared the pre-reload sequence (a prior fragment/pushState) mis-classifies SameDocument against the
    reloaded entry on a later traversal. Content-mode reload (`HistoryCursorOp::Keep`) re-stamps too.
- **Spec basis** (plan-review Axis-4, webref `html`): §7.4.6.1 *apply the history step* step 14.10 —
  "or **targetEntry's document is displayedDocument**: This is a same-document navigation" — same-document-ness
  is **Document object identity**, and step 12.8 keys rebuild-vs-reuse off the SHE *document* field being
  null/present, **never URL comparison**. `document_sequence` is a faithful monotonic-id proxy for the
  session-history-entry *document* field (§7.4.1.1).
- Traversal classifier (replaces `TraversalRestore`): `resolve_traversal(target_index) -> TraversalKind`:
  ```
  enum TraversalKind { SameDocument { state: Option<Vec<u8>>, scroll: Option<(f64,f64)> }, Rebuild }
  ```
  - `target_index == current` (a `go(0)`) → **`Rebuild`** (History.go step 4 = reload — see CR-2), NOT a
    same-document no-op.
  - `entries[target_index].document_sequence == entries[current].document_sequence` (different entry, same
    document) → **`SameDocument { state, scroll }`** (read the peeked target entry — DR-1).
  - else (different document) → **`Rebuild`**.
- **Both shells drop `classify_navigation` from the traversal path** and match on `resolve_traversal`;
  `classify_navigation` stays ONLY for the fresh-fragment-nav (`Push`) path.
- **Write-chain** (plan-review Axis-2 enumerated all 7 push + 5 replace sites, verified against HEAD):
  - `push` (NEW doc): `content/navigation.rs handle_navigate Push` (cross-doc, after the same-doc
    early-return), `app/navigation.rs navigate Push`, `content/mod.rs` initial load.
  - `push_same_document` (inherit): `content same_document_step` FragmentNav, `content push_or_replace`
    (pushState), `app same_document_step` FragmentNav, `app apply_state_change` (pushState).
  - `replace` (NEW doc): `app/navigation.rs navigate NavigationType::Replace` (`location.replace()`, the
    ONE cross-document replace).
  - `replace_same_document` (keep): `content same_document_step` equal-URL, `content push_or_replace`
    (replaceState), `app same_document_step` equal-URL, `app apply_state_change` (replaceState).
    (Thread-mode collapses Replace→Push §10-D6, so all `content` replace callers are same-document.)
  - `restamp_current_document`: (a) reload (`content handle_navigate Keep`, `app navigate Reload`); AND
    (b) **a CROSS-document `Commit` traversal / `go(0)` reload** (`content handle_navigate` Commit arm after
    `commit_index`; `app traverse_to` Rebuild arm after `commit_index`) — R2 code-review CRIT: a rebuilt
    traversal target is a FRESH document, so without re-stamping it keeps the `document_sequence` it shared
    with its former pushState/fragment siblings, and a later traversal to a sibling mis-classifies
    same-document (stale document under a swapped URL). A `Commit` reaches the rebuild arm ONLY when
    `resolve_traversal` returned `Rebuild` (the SameDocument case early-returns), so re-stamping every such
    `Commit` is correct.
  - **reload re-seeds `history.state`** (R2 code-review IMP): `content handle_navigate` `Keep` seeds
    `history_state` from the CURRENT entry's serialized state (`current_serialized_state()`), matching the
    `go(0)` reload — a reload restores the entry's classic state.
  A missed / mis-assigned site mis-stamps identity → mis-classifies a later traversal (the F1 bug in the
  other direction) — this is the load-bearing coupled invariant (§2 new J9).

### CR-2 — `history.go(0)` = reload, not a same-document no-op (F3)

`webref html` History.go step 4: **"If delta is 0, then reload navigable … and return."** So `go(0)`
reloads (a `Rebuild`), regardless of fragment — the old `TraversalRestore::NoOp` (return `false`, no
reload) dropped the pre-5c reload for fragment URLs. Subsumed by CR-1: `resolve_traversal` maps
`target_index == current` → `Rebuild`. (`documentsEntryChanged=false` is unreachable through the JS API
otherwise: `back`/`forward` never target the current entry, `go(n≠0)` targets a different one.)

### CR-3 — interim serialize DEGRADES, does not throw, for cloneable-but-not-JSON state (F5)

`StructuredSerializeForStorage` **succeeds** for BigInt / cyclic / Map / Date (all structured-cloneable);
only `JSON.stringify` throws on them (verified plan-review Axis-4: structured clone handles cycles via a
memory map + supports BigInt/Map/Date; the shared push/replace steps step 3 says only "Rethrow any
exceptions" = a genuine serialize-time user throw). The interim mapping "representability failure →
DataCloneError" therefore **regresses** working pages (`pushState({v:10n})` throws where browsers succeed;
the memo's own "cyclic → DataCloneError" framing in §4.2 is likewise wrong). The JSON shortcut cannot match
structured-clone's error set in **either** direction, so the interim **never throws for a representability
failure** — it **degrades to `None`** (no restorable state; a same-turn `history.state` read still sees the
live `current_state`, and a cross-document traversal restores `null` — the D1 gap). Only a **user exception
thrown *during* serialization** (a throwing `toJSON`/getter — a `ThrowValue`) still propagates. Signature:
`structured_serialize_for_storage(ctx, value) -> Result<Option<Vec<u8>>, VmError>` (`Ok(None)` =
un-representable, `Err` = user `ThrowValue` only). `HistoryAction.serialized_state` is already
`Option<Vec<u8>>`, so the `None` threads unchanged.

- **The OPPOSITE deviation, explicitly slotted** (plan-review Axis-3 IMP — do NOT under-document): degrading
  instead of throwing means a **genuinely non-cloneable** top-level value — a `function` / `symbol`, which
  the spec **requires** to throw DataCloneError — now `JSON.stringify`s to `undefined` → `Ok(None)` →
  `pushState(function(){})` **succeeds silently with null state** where the spec mandates a throw. This is a
  script-observable contract deviation, not merely a value-fidelity gap. It is a **distinct D1 sub-gap**
  (`#11-history-state-structured-serialize-fidelity` §10-D1 audit gains it: the interim drops BOTH
  directions — cloneable-non-JSON wrongly can't restore, AND non-cloneable wrongly doesn't throw). The JSON
  shortcut cannot distinguish the two (both surface as "JSON can't encode"), so neither is fixable without
  the full structured-clone walker (D1). **Honest interim posture**: pushState never throws for a
  representability reason; correct DataCloneError-for-non-cloneable + full-fidelity-restore both land at D1.
- **Tests**: **remove** the J2 "DataCloneError before URL side-effect" invariant + the
  `cyclic_state_throws_data_clone_error` test (it encoded the wrong behavior). **Add** (a) "cyclic/BigInt
  pushState **succeeds**, state degrades to `null` on restore"; (b) "`pushState(function(){})` **succeeds**
  with null state (interim; D1-owned to become a DataCloneError throw)" — pinning the interim contract so
  the D1 flip is a visible test change.

### CR-4 — WebIDL argument-conversion order: coerce url/title BEFORE serialize (F6)

WebIDL converts the `pushState(data, unused, url)` arguments left-to-right (`unused`→DOMString,
`url`→USVString?) **before** the algorithm runs (StructuredSerializeForStorage is algorithm step 3). So the
order is: coerce `url`/`title` to strings (WebIDL) → serialize `data` (step 3) → parse URL + gate (step 5).
The first impl serialized before the `url.toString()` coercion, so a throwing `url.toString()` + a throwing
`toJSON` surfaced the wrong exception. Reorder `state_mutate`: `to_string(url)`/`to_string(title)` first,
then serialize, then parse+gate. (The serialize-before-side-effect ordering CR-1/§6.3 still holds — the
parse+gate is step 5, after serialize.)

### CR-5 — chrome Back/Forward buttons route through the same-document-aware traversal (F2)

The toolbar Back/Forward path (`app/navigation.rs` `handle_chrome_action`) eagerly commits
(`go_back`/`go_forward`) then always rebuilds (`navigate_to_history_url`), so a same-document toolbar back
rebuilds where JS `history.back()` now applies in place — an observable "toolbar back ≠ JS back" split
(One-issue-one-way) and a non-atomic eager commit. **Fix (app-mode, cheap):** route it through
`traverse_to` (peek → `resolve_traversal` → same-doc-restore or atomic rebuild-then-commit). **Content-mode
(threaded):** the chrome-back path is assessed at impl — if it already routes through
`handle_history_action` (`Back`/`Forward`) it is same-document-aware by construction; if it is a distinct
eager path, it is fixed the same way. **Structural (not effort) deferral criterion** (plan-review Axis-3/Axis-5): route
content-mode chrome-back through `resolve_traversal` **in-PR** UNLESS it cannot do so atomically without the
D5 task-boundary work (`#11-session-history-task-queue-model`) — i.e. defer ONLY if the content chrome path
structurally requires the deferred task-queue model, not merely if it is "more work". **No new `#11` slot
is minted** (the §10 cap is already at 3 = D1/D3/D8): a genuine deferral folds into the **existing no-slot
§8-D4 driver-unification audit**, keeping the per-PR cap ≤3. **Ratify the disposition at re-plan-review.**

### CR-6 — scroll capture-on-leave fires on ANY leave, not only a traversal (F4)

Capture-on-leave was wired only into the traversal arms, so the common "scroll `/a` → click a fragment/link
→ `back()`" flow found `scroll_position = None` (the fresh-nav-leave never captured). **Fix:** capture the
departing entry's scroll before **any** cursor-advancing operation. Content-mode: a single
`capture_scroll_on_leave` at the **top of `handle_navigate`** (before the same-document gate) covers
`Push` (fresh nav + fragment) / `Commit` (traversal) / `Keep` (reload) uniformly — replacing the two
per-arm captures in `handle_history_action`. App-mode: a `capture_scroll_on_leave` at the top of `navigate`
+ `traverse_to`.

### CR — findings NOT changing the design

None beyond F1–F6. The verified `/simplify` improvements already applied (shared `resolve_traversal_restore`
→ now `resolve_traversal`; `step` by-value) are retained, adjusted to the `TraversalKind` shape.

---

## §1 Scope + slots

### §1.1 What 5c is

5c threads the **classic History `state` object** and **scroll position** end-to-end through the shell's
session-history model, and makes a **same-document history *traversal*** restore that state + scroll and
fire `popstate` (+ `hashchange` when the fragment differs) — the second consumer of the 5b same-document
primitive. Concretely: (1) `StructuredSerializeForStorage` the pushState/replaceState state object into
the engine-independent `HistoryEntry`; (2) store + expose the serialized state + scroll on session-history
entries; (3) on a **same-document** traversal, restore state + scroll and fire popstate via the 5b
back-channel (`popstate_state = Some(entry_state)` — inner `Some(bytes)`/`None`, filling the
`reconstruct_history_state` stub 5b left); (4) on a **cross-document** traversal, seed the rebuilt document's `history.state` at the pre-eval
install seam (restore-without-fire); (5) capture scroll-on-leave / restore scroll-on-arrive.

### §1.2 Slots closed

- **`#11-history-state-traversal-popstate-fidelity`** — the structured-serialized state object is NOT
  threaded (`HistoryAction::PushState/ReplaceState` carry only `url`/`title`, §5.2;
  `HistoryEntry.classic_history_api_state` is scaffolded-but-unfilled, §5.3); popstate is never fired on
  traversal (traversal takes the full-rebuild path, never `deliver_history_step_events`, §5.4); traversal
  restores neither state nor scroll (`scroll_position`/`scroll_restoration` fields exist but are never
  written or read, §5.3).

### §1.3 Non-goals (inherit cluster §1.3 / 5b §1.3)

- **Navigation API** (`navigation.*`, `NavigationHistoryEntry`, `navigation.entries()`,
  `NavigationActivation`) — separate modern surface; the `HistoryEntry.navigation_api_*` fields exist but
  the API is out of the classic-History subset. No slot owed.
- **The D5 task-queued traversal model** (`#11-session-history-task-queue-model`) — the spec's
  **task-boundary** phase-separation of a same-turn traversal-then-sync-update. **DEFERRED to a dedicated
  S5-5d**, NOT folded into 5c (§4.3 — the load-bearing scope decision). 5c stays on the collapsed
  synchronous model 5a/5b use; the E7 residual is documented + inherited.
- **`pushState-on-initial-about:blank → replace`** (§7.4.4 step 4) — NOT representable today (no
  `is initial about:blank` flag exists, §5.6); **deferred as a bounded carve** minting the shared flag when
  first load-bearing (§4.4 / §10-D8), NOT smuggled into 5c.
- **Full `StructuredSerializeForStorage` fidelity** (Blob / File / Map / Date / cyclic graphs to storage
  bytes) — 5c ships the **JSON-shortcut interim** (§4.2 / Q3); full fidelity → D1
  (`#11-history-state-structured-serialize-fidelity`, folds with the worker `#11-worker-structured-serialize`
  slot).
- **`history.scrollRestoration` writable setter + `"manual"`-mode suppression** — the getter is an
  `"auto"` stub (§5.5); 5c implements the `Auto` capture/restore only. Setter + `Manual` suppression →
  **D3** (`#11-history-scroll-restoration-manual-mode`, cluster §8-D3).
- **The fragment-navigation focusing step** (§7.4.6.4 step 3.6/3.7) — inherited non-goal (5b **D2**).
- **bfcache / cross-document-entry *document* reconstruction / `hasUAVisualTransition`** — non-goals
  (5c seeds `history.state` on a *rebuilt* document, NOT the exact prior document; `hasUAVisualTransition`
  always `false`).
- **iframe traversal same-document** — the iframe nav path is a distinct 3-arg `handle_navigate`
  (5b **D7** `#11-iframe-fragment-navigation`); 5c's closure is top-level + app-mode only.
- **The post-handler re-render / re-drain + `(index, length)` VM publish cluster** — 5b already routed
  these to the **S5-6 flip deliverable** + the `#11-session-history-index-vm-publish` carve (5b §9); 5c's
  traversal firing inherits that flip-inert deferral (§6.6), does NOT re-open it per-facet.
- Per-VM state → ECS component migration — B1 (post-S5, umbrella §0.1).

---

## §2 Coupled-invariant enumeration (edge-dense — Pre-condition #3)

5c is edge-dense (3 intersecting axes: state-round-trip × event-firing × scroll, each crossing the
engine-boundary flip-inert seam). The invariants it **simultaneously** satisfies, and each load-bearing
**pairwise intersection**:

**Invariants**
- **J1 one serialized representation** — `SerializedState = Vec<u8>` is the SINGLE serialized form from VM
  serialize → `HistoryAction` → `HistoryEntry` → `HistoryStepEvents` → VM deserialize; no second
  representation (the field is aligned to `Vec<u8>`, §4.1). D1 (full fidelity) swaps the seam **body**, not
  the field/wire type (E8).
- **J2 serialize-order fidelity** — `StructuredSerializeForStorage` runs at §7.2.5 step 3, **before** the
  §7.2.5 step-5 URL parse + can-have-url-rewritten gate (step 6 = the separate navigation-allowed gate).
  The interim signature is `-> Result<Option<Vec<u8>>, VmError>` (§0-CR CR-3): a representability failure
  (cyclic / BigInt / non-JSON) **degrades to `Ok(None)`** (no throw — the JSON shortcut cannot match
  structured-clone's error set in either direction), and only a **user exception thrown *during* serialize**
  (a throwing `toJSON`/getter — a `ThrowValue`) propagates, doing so **before** any URL side-effect (§6.3).
  What J2 pins is the **ordering** (whatever the serialize step does — degrade or user-throw — it does so
  before the URL parse+gate); the degrade-vs-throw semantics themselves are §0-CR CR-3.
- **J3 state round-trip coherence** — the VM's synchronous `current_state: JsValue` (set at pushState,
  §5.2) and the deserialized restored value delivered on traversal represent the **same** entry's state; a
  traversal overwrites `current_state` with the deserialized restored value (§6.4), never diverging.
- **J4 traversal event matrix** — same-document traversal fires popstate(**restored**) + hashchange(iff
  frag differs); cross-document traversal fires **neither** but **restores** `history.state` (§7.4.6.2 step
  6.3 runs regardless of `documentIsNew`; step 6.4 popstate/hashchange gated on `documentIsNew=false`) —
  the E4 matrix (§4.5).
- **J5 seed-before-scripts** — the cross-document-traversal `history.state` seed is installed at the
  **pre-eval** chokepoint (§7.4.6.2 step 6.3 restore precedes step 8.4 "scripts may run"), so the rebuilt
  document's *initial* scripts read the restored `history.state`, not null (§6.5).
- **J6 restore-WITHOUT-fire ≠ deliver** — the cross-document seed is a distinct restore-only path (NO
  popstate), NOT a `deliver_history_step_events` call (which fires popstate — spec-wrong for a new
  document, `documentIsNew=true`) (§6.5).
- **J7 scroll persist/restore currency** — scroll captured-on-leave (before the cursor moves) + restored
  on-arrive rides the existing viewport transport + `re_render` **post-layout** scroll-application seam (the
  5b I6 precedent), never inline in the drain (§6.6).
- **J8 flip-inert state, engine-agnostic scroll** — the state VALUE round-trip is flip-inert end-to-end
  (boa passes `None`, §6.7); the scroll persist/restore + same-document-traversal no-rebuild classification
  is engine-agnostic-now (observable in the live boa shell) (E6, §6.7).
- **J9 per-entry document identity** (§0-CR CR-1) — every entry-creating/replacing/reloading op stamps
  `document_sequence` **consistently**: `push`/`replace` = a NEW document (fresh sequence),
  `push_same_document`/`replace_same_document` = **inherit** the current entry's sequence, `reload` =
  **re-stamp** the current entry. A traversal is same-document **iff** the target entry's `document_sequence`
  equals the current entry's — Document object identity (§7.4.6.1 step 14.10 / step 12.8), **NOT** URL
  comparison. A missed / mis-assigned stamp mis-classifies a later traversal (the load-bearing write-chain,
  §0-CR CR-1) — the new invariant axis the re-plan-review adds.

**Intersections (load-bearing)**
- **J1 × J4** — the SAME `Vec<u8>` the classifier-driven traversal delivers as `popstate_state =
  Some(Some(bytes))` (same-doc) is the value the cross-doc seed restores; one representation feeds both the
  fire path and the restore-only path.
- **J3 × J4** — `current_state` coherence holds across the traversal fire: `reconstruct_history_state`
  sets `navigation.current_state = deserialize(bytes)` (5b's line 93, already there for the null case)
  BEFORE firing popstate, so a synchronous popstate handler reads `history.state === popstate.state`.
- **J5 × J6** — the seed is restore-before-scripts (J5) AND restore-without-fire (J6): a cross-document
  traversal's initial scripts read the restored `history.state` but NO popstate fires (the fresh document
  is `documentIsNew=true`) — conflating the seed with `deliver_history_step_events` would spuriously fire
  popstate on document load (the classic double-fire bug).
- **J4 × J7** — same-document traversal fires popstate AND restores persisted scroll (§7.4.6.2 step 6.4.3
  popstate, step 6.4.4 restore-persisted-state); the popstate is SYNC while the scroll rides the post-layout
  `re_render` seam — a popstate handler that scrolls must not be clobbered by the restore, and the restore
  must not be clobbered by the handler. **This ordering is flip-inert** (boa fires no popstate) → inherited
  by the S5-6 flip deliverable (5b §9, the scroll-vs-popstate-handler ordering fold), NOT solved per-facet
  in 5c (§6.6/§6.7).
- **J2 × J8** — the serialize (J2) is VM-side (boa passes `None`), so the whole round-trip's *value* is
  flip-inert; the serialize-order fidelity (representability failure degrades to `Ok(None)`; a user
  `ThrowValue` propagates before the URL side-effect — §0-CR CR-3) is a VM-test assertion now, live at S5-6.
- **J1 × J8** — aligning the entry field to `Vec<u8>` (J1) is engine-agnostic **structural** plumbing
  (the field/controller change is observable regardless of engine), even though the state VALUE flowing
  through it is flip-inert (boa fills `None`). The plumbing lands now; the value lights up at the flip.
- **J9 × J4** — the traversal event matrix (§4.5) keys off **J9's `document_sequence` classification**
  (`resolve_traversal`), **NOT** URL comparison: same-document (equal sequence) fires popstate(restored) +
  frag-gated hashchange, differing sequence (or `go(0)` = reload) rebuilds and fires neither. Classifying by
  URL (the superseded `classify_navigation`-for-traversal model) mis-keys the matrix (§0-CR CR-1) — a
  different-path-same-document `pushState` back would rebuild + drop popstate.

---

## §3 Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §7.2.5 shared history push/replace state steps | step 3 `StructuredSerializeForStorage(data)`; step 5 url parse + can-have-url-rewritten gate; eviction FIFO | push vs replace; null vs value; empty-string url | `state_mutate` (`vm/host/history.rs:147`) serialize insert (§6.3) | ✗ (JSON-shortcut interim §4.2; full = D1) | yes (state object / url) |
| HTML §2.7.5 StructuredSerializeForStorage(value) | serialize `data` to bytes (forStorage=true) | JSON-representable → `Ok(Some(bytes))` vs not → **`Ok(None)` degrade** (§0-CR CR-3; interim never throws DataCloneError — D1 sub-gap); user throw → `Err(ThrowValue)` | shared JSON-shortcut byte seam (§6.2) | ✗ (JSON subset; §4.2 / D1) | yes (state object) |
| HTML §7.4.4 URL and history update steps | step 3 newEntry serialized-state; step 7 restore-if-non-null; step 8 set-url-no-hashchange | pushState/replaceState (5c serialize + thread) | `HistoryAction::PushState/ReplaceState.serialized_state` (§6.2) + `NavigationController::push/replace` (§6.2) | ✓ (the serialize/thread) | yes (state / url) |
| HTML §7.4.3 traverse the history by a delta | delta resolution; out-of-range no-op | back / forward / go(±n) / go(0) | reuse 5a peek-then-commit; 5c adds same-doc restore path (§6.4) | ✓ (delta clamp — 5a) | yes (delta) |
| HTML §7.4.6.2 update document for history step application | step 6.3 restore-state; step 6.4.3 popstate; step 6.4.4 restore-scroll; step 6.4.5 hashchange; step 8.4 scripts-may-run | same-doc traversal (restored) vs cross-doc (seed-only) | `deliver_history_step_events` `Some(entry_state)` (§6.4) + the pre-eval seed (§6.5) | ✓ (the restore + fire matrix, §4.5, incl. no-state + `go(0)` branches) | yes (state object) |
| HTML §7.4.6.4 scroll to the fragment | restore persisted scroll (§6.4.4) on same-doc traversal | id-match / stored offset / top | scroll capture-on-leave / restore-on-arrive via viewport transport (§6.6) | ✗ (Auto mode; Manual → D3) | yes (scroll offset) |

**Breadth**: K=1 (HTML), M=6 → **ok (single-PR scope)**.

### §3.1 User-input touch audit (`feedback_trust-boundary-enumerate-upfront`)

Every input rides an EXISTING validated seam — **no new trust boundary; 5c narrows one** (a dropped
state stops being dropped):
- the **state object** → the shared JSON-shortcut serialize seam (§6.2). A throwing `toJSON`/getter
  surfaces as a script-observable `ThrowValue` (`Err`, propagated). A representability failure
  (cyclic/BigInt/depth) **degrades to `Ok(None)`** — **no throw** (§0-CR CR-3): the JSON shortcut cannot
  reproduce structured-clone's `DataCloneError` set, so an un-representable state becomes **un-restorable**
  rather than an exception (and the **opposite deviation** — a `function`/`symbol` the spec requires to
  throw silently succeeding with null state — is the D1-owned sub-gap). The seam reuses the worker JSON
  core (`natives_json::stringify_to_string`, via `worker_scope.rs`) but with **degrade-not-throw**
  semantics (it does NOT inherit the worker `serialize_message` throw-on-failure contract).
- **url** strings → the existing `state_mutate` parse + `can_have_url_rewritten` gate
  (`vm/host/history.rs:180-208`, SecurityError on cross-origin rewrite) — unchanged.
- **delta** → the existing `to_int32` coercion + `NavigationController::peek_go`'s out-of-range clamp
  (5a) — unchanged.
- the **serialized bytes** on traversal → engine-internal (produced by the VM's own serialize, never
  external); `StructuredDeserialize` catches a decode exception → `state = null` (§7.4.6.2 restore step 2,
  webref-verified) — a **backstop**, not a trust boundary (the bytes never leave the process).

---

## §4 THE design corrections + scope decisions (the plan-review ratify-points)

### §4.1 Headline — the serialized-state representation: align the entry field to `Vec<u8>`

> The load-bearing design finding of authoring 5c against the **landed** 5b wire. It **contradicts the
> cluster memo's §4.5 "honor the `Option<String>` field"** — which predates the 5b `SerializedState =
> Vec<u8>` wire — and is the #1 plan-review ratify-point (matches
> `feedback_plan-review-verify-preserve-existing-spec-claims`: re-verify a plan's "existing shape" claims).

The cluster memo §4.5 said: *"the `HistoryEntry.classic_history_api_state` field is already typed
`Option<String>` … the interim reuses the same JSON-shortcut … so the `Option<String>` field type is
honored."* But **5b landed the delivery wire as `SerializedState = Vec<u8>`**
(`script-session/navigation.rs:111`; `HistoryStepEvents.popstate_state: Option<Option<SerializedState>>`
:128) — a `Vec<u8>`, not a `String`. Meanwhile the pre-existing shell-side
`HistoryEntry.classic_history_api_state` is `Option<String>` (`elidex-navigation/navigation.rs:31`). **Two
representations for one value** — the exact drift One-issue-one-way retires.

**5c aligns `HistoryEntry.classic_history_api_state: Option<String>` → `Option<Vec<u8>>`** (= the
`SerializedState` alias), so there is **one** serialized representation end-to-end:

```
VM StructuredSerializeForStorage → HistoryAction.serialized_state: Option<Vec<u8>>
  → NavigationController stores HistoryEntry.classic_history_api_state: Option<Vec<u8>>
  → HistoryStepEvents.popstate_state: Some(Some(Vec<u8>))  (traversal)  OR  the seed reads the Vec<u8>
  → VM StructuredDeserialize(Vec<u8>) → history.state
```

- **Why not keep `Option<String>` + convert at the boundary.** A `String↔Vec<u8>` UTF-8 conversion is
  lossless *only* for the JSON-shortcut interim (JSON text is valid UTF-8). Full
  `StructuredSerializeForStorage` (D1) produces **arbitrary bytes** (Blob/ArrayBuffer/binary) that a
  `String` field cannot hold — so keeping `String` would force D1 to *also* change the field type,
  re-touching every write/read site. `Vec<u8>` now makes D1 a **seam-body swap** (change the serialize/
  deserialize functions), field + wire + controller unchanged (J1). This is the ideal-over-pragmatic +
  One-issue-one-way choice.
- **Cost**: the `classic_history_api_state` field type + its 3 write sites (verified 2026-07-09 — `push`/`replace` param,
  `state_mutate` non-write) + read sites (traversal expose, the seed). Bounded; §5.3 enumerates.
- **Cluster-memo reconciliation** (One-issue-one-way — **sweep ALL sites, not just §4.5**): the stale
  `Option<String>` framing lives in **≥3** cluster locations, so the 5c landing edits **all** of them, else
  the cluster keeps contradictory framing (the exact drift this correction invokes One-issue-one-way to
  retire): (a) cluster **§4.5 prose** ("so the `Option<String>` field type is honored", ~line 679); (b) the
  **E8 edge-matrix row** ("the pre-typed `Option<String>` field", ~line 941); (c) **§9-Q3** ("the pre-typed
  `Option<String>` field", ~line 1108). All three → `Option<Vec<u8>>` aligned to the 5b `SerializedState`
  wire. **Plan-review ratifies the correction** (§7 Q-SERIAL-REP).

### §4.2 State serialization fidelity — the JSON-shortcut interim (Q3)

There is **no** JsValue↔bytes serializer in the tree today: `structured_clone.rs` is a **fused**
StructuredSerialize+Deserialize (`clone_value(vm, input) -> JsValue`, an in-memory clone, `:75`), never
producing bytes. The **only** serialize-to-portable-form precedent is the worker JSON-shortcut
(`worker_scope.rs` `serialize_message`, a JSON `String` standing in for full StructuredSerialize, slot
`#11-worker-structured-serialize`).

5c's interim: **`StructuredSerializeForStorage(state)` = JSON-encode → UTF-8 `Vec<u8>`; `StructuredDeserialize`
= `String::from_utf8` → JSON-parse → JsValue** — the SAME JSON-shortcut, sharing the worker path's JSON
encode/decode core (One-issue-one-way — one JSON-shortcut, two consumers). Both JSON-shortcuts stand in for
full StructuredSerialize and share the same D1 trigger, so **D1 upgrades one shared seam** (full
StructuredSerializeForStorage-to-bytes) for both worker + history; the field/wire (`Vec<u8>`) is unchanged.

- **Fidelity gap** (D1): the interim drops non-JSON shapes (Blob/File/Map/Date/cyclic — **degraded to
  `Ok(None)` → `null` on restore, NOT a `DataCloneError` throw**; §0-CR CR-3; the opposite deviation — a
  non-cloneable `function`/`symbol` silently succeeding where the spec mandates DataCloneError — is the
  D1-owned sub-gap). Classic History state is almost always plain objects, so the interim is
  common-case-correct; full fidelity is D1. `StructuredSerializeForStorage` differs from `StructuredSerialize`
  only in disallowing SharedArrayBuffer / `forStorage=true` handling — moot for the JSON subset (JSON handles
  neither), noted for D1.
- **One-issue-one-way audit**: 5c does NOT force the worker path to refactor onto a new shared seam
  (scope) — but it homes the history serialize/deserialize as a `Vec<u8>` JSON-shortcut **beside** the
  worker one and registers the convergence (D1 folds both). Whether to extract a shared
  `structured_serialize_for_storage`/`structured_deserialize` fn now vs at D1 is a §7 ratify-point
  (Q-SERIAL-HOME); **lean: extract the shared byte seam now** (both call it), since 5c must write the
  byte serializer anyway and a parallel history-local JSON encoder would be the very duplication the audit
  flags. Ratify.

### §4.3 The D5 task-queued model boundary — dedicated S5-5d, NOT in 5c (the scope decision)

**Decision: `#11-session-history-task-queue-model` (D5) lands in a dedicated S5-5d, NOT folded into 5c.**
The philosophy lens converges (decide-not-ask, `feedback_decide-via-philosophy-before-asking`); recorded
here as a plan-review ratify-point (§7 Q-D5), not an open question.

- **Edge-dense rule** (CLAUDE.md): D5 subsumes #259/#283/#448 + E7 + chrome-button atomicity — a subsystem
  with ≥3 intersecting invariant axes and no canonical algorithm in the tree (it restructures the
  `process_pending_actions` drain into a task boundary). It "must not ride a narrow PR" and needs its own
  plan-review. 5c is *itself* edge-dense (J1–J8); bundling D5 in is the #339 mega-PR shape.
- **One-issue-one-way**: 5c and 5d touch **disjoint** seams. 5c = the entry-model / serialize / event-**data**
  ("what" state/scroll a traversal restores + fires); 5d = the drain / task-boundary ("when" a traversal
  runs relative to same-turn sync updates). 5c is **complete-and-shippable on the collapsed synchronous
  model** 5a/5b already use — it leaves no dead half (the synchronous same-document traversal restores +
  fires correctly; 5d later refines its *timing* relative to a same-turn sync update). NOT a strangler.
- **No new live-reachable E7 facet**: boa's single-slot back-channel makes a multi-action turn (traversal
  + pushState) **unreachable pre-flip** (cluster §3.2); D5's own trigger is "the multi-action drain
  (post-flip)". So 5c's NEW same-document-traversal firing introduces no live-reachable E7 collision.
- **Reinforced by 5b's deferral**: 5b already routed the whole post-handler re-render / re-drain /
  inline-`drain_tasks` / `(index,length)`-publish cluster into the **S5-6 flip deliverable** + D5 as
  flip-inert (5b §9). 5c's traversal firing **inherits** that deferral verbatim (§6.6) — it does not
  re-open any facet. The E7 residual (a same-turn `history.back(); pushState()` applied in one synchronous
  pass rather than phase-separated) is documented, bounded, and owned by 5d.

### §4.4 `pushState-on-initial-about:blank → replace` — deferred (not representable)

§7.4.4 step 4 (webref-verified 2026-07-09): *"If document's is initial about:blank is true, then set
historyHandling to 'replace'"* ("pushState() on an initial about:blank Document behaves as a
replaceState()"). The 5b memo handed this to "5c kickoff" (5b §7) to **verify representability**.

**Verified: NOT representable.** No `is_initial_about_blank` flag/predicate exists anywhere in
`crates/shell` + `crates/script` (grep: only URL literals + a doc comment at `vm/host/navigation.rs:97-98`).
`NavigationState::new()` seeds `current_url = about:blank` but no boolean distinguishing the *initial*
about:blank from a navigated-to one.

Per the cluster memo §8's own conditional ("folds into 5c *if* `is initial about:blank` is representable,
else defer"), and One-issue-one-way (the flag is **shared infrastructure** — also load-bearing for
§7.4.6.2 step 7.4's `NavigationActivation` "previousEntryForActivation's document's initial about:blank is
false" and the navigate algorithm), 5c **defers** it as a bounded carve **D8**
(`#11-initial-about-blank-flag`): mint the flag once, deliberately, when first load-bearing — not smuggled
into 5c's state/traversal core. Interim: a pushState during the (rare) initial-about:blank window pushes
rather than replaces — a `history.length` off-by-one in an uncommon pre-navigation window (§10-D8).

### §4.5 The traversal event matrix 5c implements (spec-traced 2026-07-09)

| Operation | popstate | hashchange | `history.state` after | scroll |
|---|---|---|---|---|
| **pushState / replaceState** (5c serializes) | NO | NO | **serializedData** (synchronous `current_state` + serialized onto the entry) | unchanged |
| **Traversal → same-document entry (equal `document_sequence`), *with* state** | **YES**, state = **restored** (`StructuredDeserialize`) | **YES** iff oldURL frag ≠ newURL frag | restored | restored persisted |
| **Traversal → same-document entry (equal `document_sequence`), *no* state** (plain-nav / boa-`None` entry) | **YES**, state = **null** (`Some(None)`) | **YES** iff oldURL frag ≠ newURL frag | null | restored persisted |
| **Traversal → cross-document entry (differing `document_sequence`)** | NO (`documentIsNew=true`) | NO | **restored (step 6.3, the pre-eval seed)** | scroll-to-fragment (existing rebuild) |
| **`go(0)` = reload (History.go step 4; §0-CR CR-2)** | NO (`documentIsNew=true` — reload replaces the document) | NO | **re-seeded from the entry (step 6.3, pre-eval seed — the document is reloaded, NOT unchanged)** | scroll-to-fragment (reload rebuild) |

**Load-bearing (Axis-4 branch completeness)**: popstate firing is **state-agnostic** — §7.4.6.2 step 6.4.3
fires whenever `documentsEntryChanged` (step 6) ∧ `documentIsNew=false` (step 6.4), *regardless of whether
the entry carries state*. So the delivered value is the **general form `popstate_state =
Some(target_entry.classic_history_api_state.clone())`** (outer `Some` = "same-document traversal ⇒ fire";
inner `None` = null state; inner `Some(bytes)` = `StructuredDeserialize`), NOT the state-present-only
`Some(Some(restored))`. Same-document-ness is keyed on **`document_sequence` identity** (§0-CR CR-1), not
URL. And a **`go(0)` traversal is a reload** (History.go step 4 = "If delta is 0, then reload navigable";
§0-CR CR-2) — it replaces the document (`documentIsNew=true`) → **fires no popstate**, but is **NOT** a
no-op: the document is reloaded and `history.state` re-seeded from the entry (step 6.3). The §9 `go(0)` test
pins the **reload**; an impl that treats `go(0)` as a same-document no-op (the superseded model), or that
fires popstate on it, is spec-wrong.

Trace (webref 2026-07-09, `#update-document-for-history-step-application`): step 2
`documentsEntryChanged`=(latest entry ≠ entry); step 6 gated on it — step 6.3 **restore the history object
state** (`StructuredDeserialize(entry's classic history API state)` → `history.state`) runs whenever the
entry changed, **regardless of `documentIsNew`**; step 6.4 gated on `documentIsNew=false` (same document) →
6.4.3 fire popstate SYNC, 6.4.4 restore persisted (scroll), 6.4.5 queue hashchange iff frag differs; step 8
gated on `documentIsNew=true` (fresh document) → 8.3 scroll-to-fragment, **8.4 "scripts may run"** (so step
6.3's restore precedes it — J5). ⇒ **cross-document traversal restores state (6.3) but fires neither event
(6.4 skipped)**; **same-document traversal fires both** (frag-gated hashchange). pushState/replaceState fire
neither (§7.4.4 note verbatim: *"popstate events fire for fragment navigations, but not for
history.pushState() calls"*).

---

## §5 Current-state (post-5b, HEAD `a904ea81` — re-grounded)

### §5.1 The 5b same-document primitive (what 5c reuses)

- `crates/script/elidex-js/src/vm/host/history_events.rs` (242 lines):
  `deliver_history_step_events(popstate_state: Option<Option<Vec<u8>>>, hashchange: Option<(String,String)>)`
  :63 — `is_bound()`-gated (flip-inert :72-79); restores `navigation.current_state = state` :93 then fires
  popstate SYNC :94 + `drain_microtasks` :98; enqueues + `drain_tasks` hashchange :106-114. **The traversal
  arm is stubbed**: `reconstruct_history_state(state: Option<Vec<u8>>)` :201-207 — `None => Null` (5b
  fragment), **`Some(_bytes) => JsValue::Null` :205 (the 5c placeholder — "StructuredDeserialize(_bytes).
  Unreachable in 5b")**. The fire path already **GC-roots an Object-valued state slot** (:154-164 comment:
  "a 5c `history.state` object") — so a real state object needs no new rooting.
- `crates/script/elidex-js/src/engine.rs` (VM): `deliver_history_step_events(ev)` :462-468 forwards to the
  above; `set_session_history(index, length)` :444 sets `current_index = index` :448; `origin()` :486.
- `crates/script/elidex-js-boa/src/runtime/observers.rs:283`: `deliver_history_step_events(_ev) {}` —
  **no-op stub** (flip-inert confirmed).
- `crates/shell/elidex-shell/src/content/navigation.rs`: `classify_navigation` (5b, `elidex-navigation`
  pure fn) + `fragment_navigate(state, current, target) -> bool` :248-322 (the no-rebuild primitive; fires
  `popstate_state: Some(None)` :286-292 + hashchange :310-320) — gated to fresh navs (`cursor_op == Push`).

### §5.2 pushState/replaceState drops the state (the write side)

`crates/script/elidex-js/src/vm/host/history.rs` (303 lines), `state_mutate` :147-257: reads `state =
args.first()` :152; the §7.2.5-step-5 URL parse + `can_have_url_rewritten` gate :168-210; synchronously
sets `current_url` :224 + `current_state = state` (bare `JsValue`) :225; `record_push_state()` :239 (push
only); enqueues `HistoryAction::PushState { url, title }` / `ReplaceState { url, title }` :244-255 — **NO
`serialized_state`** (§7.2.5 step 3 `StructuredSerializeForStorage` unimplemented). `native_history_get_scroll_restoration`
:75 returns `"auto"` stub; accessor table :302 registers the getter only (RO — no setter, D3).

`HistoryAction::PushState`/`ReplaceState` (`script-session/navigation.rs:91-103`) carry only `url:
Option<String>` + `title` — **no state field** (5c adds `serialized_state: Option<SerializedState>`).
`SerializedState = Vec<u8>` :111; `HistoryStepEvents { popstate_state: Option<Option<SerializedState>>,
hashchange: Option<(String,String)> }` :124-132.

### §5.3 `HistoryEntry` — state/scroll scaffolded but unpopulated (the store side)

`crates/shell/elidex-navigation/src/navigation.rs` (647 lines): `HistoryEntry` :17-31 carries
`scroll_restoration: ScrollRestorationMode` :27, `scroll_position: Option<(f64,f64)>` :29,
**`classic_history_api_state: Option<String>`** :31 (comment "JSON string"). `push(url: url::Url)` :75
hardcodes `scroll_restoration: default` :87, `scroll_position: None` :88, `classic_history_api_state: None`
:89; `replace(url)` :108 leaves them untouched. `MAX_HISTORY_ENTRIES = 50` :43. Traversal read path:
`peek_back`/`peek_forward` :125/132 + `peek_go(delta)` :140 return `Option<(usize, &url::Url)>` (URL only);
`commit_index(index)` :158; the eager `go_back`/`go_forward`/`go` :173/182/196 return `Option<&url::Url>`;
`current_url()` :203 / `current_title()` :236. **So state/scroll are never written by push/replace and
never exposed by the traversal read path** — 5c populates + exposes them (§6.2, incl. the §4.1 field
re-type to `Option<Vec<u8>>`).

### §5.4 Traversal takes the full rebuild path (never fires popstate)

`content/navigation.rs`: `handle_history_action(state, action) -> bool` :577-626 — Back/Forward
`peek_back`/`peek_forward` → `handle_navigate(state, &url, HistoryCursorOp::Commit(target_index), None)`
:582-605; Go `peek_go` → same :606-615; PushState/ReplaceState → `apply_push_replace_state` :616-624.
`handle_navigate` :48-232 — the `Commit(index)` arm rebuilds via `build_pipeline_from_loaded` :178 then
moves the cursor :210-214. **A traversal NEVER reaches `fragment_navigate` (gated `cursor_op == Push`
:68/213) and NEVER calls `deliver_history_step_events`** — no same-document-traversal branch exists; this
is what 5c adds (§6.4). `apply_push_replace_state(state, url_str, replace)` :632-675 (state-dropping).

### §5.5 The pre-eval install seam (the seed site, S5-4b precedent)

`crates/shell/elidex-shell/src/pipeline.rs` (774 lines): `run_scripts_and_finalize` :130-… installs
`PreEvalFrameState` (:30-90: sandbox_flags / origin / iframe_depth / credentialless / referrer) at :189-200
**BEFORE the first eval** (the S5-4b `#446` referrer-seed chokepoint), then seeds viewport/device facts
:211-214. `build_pipeline_from_loaded` :655 — **6 call sites** (`pipeline.rs:762`, `app/navigation.rs:283`,
`content/mod.rs:571`, `content/navigation.rs:178`, `iframe/load.rs:207`, `iframe/load.rs:305`). The seed
threads a null-defaulted `Option<Vec<u8>>` through them; only `content/navigation.rs`'s cross-doc traversal
arm carries a value (the **peeked target entry**'s state, `entries[target_index]` — NOT `current()`; §6.5
DR-1 read-source ordering). Of the 5 that pass null: 4 are null **by construction** (the initial-load,
standalone-URL, and 2 iframe callers have no traversal target). The **5th — `app/navigation.rs`'s cross-doc
traversal rebuild — passes null by CHOICE, not by construction** (R2 elidex-review Axis-3 MIN): the target
entry carries the state and app-mode could read it exactly like content-mode, but the seed is deliberately
NOT threaded — it is flip-inert (boa passes `None`) AND app-mode's rebuild path is **enrolled in the §8-D4
driver-unification audit**, which threads app-mode's cross-doc seed once at S5-6 rather than duplicating
content-mode's plumbing pre-flip. (`restamp` is still threaded now — identity is engine-agnostic; only the
flip-inert *state seed* defers to D4.)
**No `set_history_state` bridge/`HostDriver` method exists** — 5c adds one (restore-only, §6.5).

### §5.6 `is initial about:blank` not representable (§4.4)

Grep `is_initial` / `initial_about_blank` across `crates/shell` + `crates/script` → **zero** fields/fns;
only URL literals + the `vm/host/navigation.rs:97-98` doc comment. `NavigationState::new()` :199-209 seeds
`current_url = about:blank`, `history_length = 1`, `current_index = 0`, `current_state = Null` — no
initial-about:blank boolean. ⇒ §7.4.4 step 4 deferred (D8).

### §5.7 App-mode (5c touches both shells)

`crates/shell/elidex-shell/src/app/navigation.rs` (511 lines): `handle_history_action(action) -> bool`
:327-396 (Back/Forward/Go peek-then-`navigate_to_history_url`-then-`commit_index` :336-377;
PushState/ReplaceState → `apply_state_change` :378-394); `navigate_to_history_url(url) -> bool` :243-258
(pure cross-doc rebuild — **fires no popstate, restores no state**); `apply_state_change(interactive, url,
replace)` :472-488 (**drops state entirely** — no state arg, never touches `classic_history_api_state`);
`resolve_state_url` :451-466. Note: app-mode fragment navs DO fire popstate (5b, :206-232), but traversals
do not. Both shells' traversal paths gain the same-document-restore branch; the shared *primitive* is
engine-indep, so the duplication is confined to the two thin drivers (cluster §8-D4).

---

## §6 Ideal architecture (5c)

### §6.1 Layering ledger (per surface)

| Surface | Home | Layer |
|---|---|---|
| `StructuredSerializeForStorage` / `StructuredDeserialize` (JsValue ↔ `Vec<u8>`) | VM `vm/host/` (shared byte seam, beside `structured_clone`) | marshalling (host/) |
| serialized-state representation (`Vec<u8>`) | `HistoryAction.serialized_state` + `HistoryEntry.classic_history_api_state` + `HistoryStepEvents` | engine-indep contract / shell side-store |
| entry-model store/expose (state + scroll) | `NavigationController::push`/`replace` + a traversal entry read path | engine-indep (shell side-store) |
| traversal same-document classification | per-entry `document_sequence` identity + `resolve_traversal` classifier (`elidex-navigation`); `classify_navigation` stays only on the fresh-fragment `Push` path (§0-CR CR-1) | engine-indep |
| event-firing DECISION (which fire, with what) | shell drain + `HistoryStepEvents` | engine-indep |
| event RECONSTRUCT + FIRE (popstate restored-state; hashchange) | VM `deliver_history_step_events` / `reconstruct_history_state` (fill the stub) | marshalling (host/) |
| cross-doc `history.state` seed (restore-WITHOUT-fire) | new `HostDriver::set_history_state` at the pre-eval seam | engine boundary (new method) / marshalling |
| scroll capture-on-leave / restore-on-arrive | shell drain + existing viewport transport / `re_render` post-layout seam | engine-indep / engine boundary (exists) |

**No new algorithm in `vm/host/`** (Layering mandate): the natives stay marshal-only; serialize/deserialize
+ reconstruct+fire + the seed-restore are all JsValue↔host marshalling; the traversal classification +
event-decision + entry-model are engine-indep.

### §6.2 The serialized-state seam + entry-model threading

1. **Shared byte serialize/deserialize** (VM `vm/host/`, §4.2): `structured_serialize_for_storage(ctx,
   value) -> Result<Option<Vec<u8>>, VmError>` (JSON-encode → UTF-8 bytes; a **representability failure →
   `Ok(None)` degrade, NOT a throw**; only a user `ThrowValue` during serialize → `Err` — §0-CR CR-3) +
   `structured_deserialize(vm, &[u8]) -> JsValue` (`from_utf8` →
   JSON-parse; decode-failure → `Null` per §7.4.6.2 restore step 2). **One encoder, two thin wrappers
   (Q-SERIAL-HOME)**: both call the SAME existing JSON core the worker path already uses —
   `natives_json::stringify_to_string` (`crates/script/elidex-js/src/vm/natives_json.rs`; `serialize_message`
   delegates to it at `worker_scope.rs:376`) — the history wrapper just packages `into_bytes()`/`from_utf8`.
   NOT a parallel history-local JSON encoder (the very duplication §4.2's audit flags). Homed beside
   `structured_clone.rs`; the `String` (worker) vs `Vec<u8>` (history) split is a thin type-wrapper the spec
   itself distinguishes (`StructuredSerialize` vs `StructuredSerializeForStorage`), converging fully at D1.
2. **`HistoryAction::PushState`/`ReplaceState`** gain `serialized_state: Option<SerializedState>` (boa
   passes `None` — light-touch).
3. **`HistoryEntry.classic_history_api_state`** re-typed `Option<String>` → `Option<Vec<u8>>` (§4.1);
   `NavigationController::push`/`replace` accept `(url, serialized_state: Option<Vec<u8>>, scroll:
   Option<(f64,f64)>)` and store them; a traversal entry read path (`entry(index) -> &HistoryEntry`, or
   extend `peek_*` to return the entry's state+scroll) exposes the target entry's `classic_history_api_state`
   + `scroll_position` at commit. **Write-chain completeness (a data-flow enumeration)**: the value's path
   from `HistoryAction::PushState.serialized_state` to `push`/`replace` runs through **two intermediate
   callers that currently drop state and MUST be threaded** — thread-mode `apply_push_replace_state`
   (`content/navigation.rs:632`) → `push_or_replace` (`content/mod.rs:155`, signature `(url, replace)`), and
   app-mode `apply_state_change` (`app/navigation.rs:472`, no state arg). If either is missed, the traversal
   read of `classic_history_api_state` returns `None` forever (field never populated). These are the "3 write
   sites" of §4.1 made explicit.

### §6.3 pushState/replaceState serialize (write, `state_mutate`)

**WebIDL argument-conversion order (§0-CR CR-4)**: `pushState(data, unused, url)` coerces its args
left-to-right (`unused`→DOMString, `url`→USVString?) **before** the algorithm runs, so `state_mutate` must
**coerce `url`/`title` to strings FIRST** (`to_string(url)`/`to_string(title)`), THEN serialize `data`
(§7.2.5 step 3), THEN parse URL + gate (step 5). A throwing `url.toString()` must surface before a throwing
`toJSON`; the first impl (serialize-before-coercion) got this backwards.

Insert `StructuredSerializeForStorage` at §7.2.5 **step 3** order — after the WebIDL string coercions,
**before** the step-5 URL parse+gate (step 6 = the separate navigation-allowed gate) (J2): serialize
`state` (read at :152) into `Result<Option<Vec<u8>>, VmError>` right after reading it. A **representability
failure degrades to `Ok(None)`** (no throw — §0-CR CR-3; `HistoryAction.serialized_state` is already
`Option<Vec<u8>>`, so the `None` threads unchanged); only a **user `ThrowValue` during serialize**
propagates, and does so **before** any `current_url` side-effect. Then the existing sync `current_state =
state` (:225, kept for immediate `history.state` reads) proceeds and the serialized bytes (or `None`) ride
the enqueued `HistoryAction::PushState/ReplaceState.serialized_state`. **boa**: passes `None` (no serialize
— deletion-bound light-touch).

### §6.4 Same-document traversal — restore + fire (the second consumer of the 5b primitive)

A traversal (thread-mode `handle_history_action` Commit arm / app-mode `handle_history_action`) resolves
the target entry via **`resolve_traversal(target_index) -> TraversalKind`** (§0-CR CR-1), driven by
**per-entry document identity** (`document_sequence`), **NOT** by `classify_navigation` / URL comparison —
`classify_navigation` is the *navigate* algorithm's fresh-fragment predicate (equal-excluding-fragments ∧
target fragment non-null) and stays **ONLY** on the `Push` path; a traversal's same-document-ness is a fact
about the session-history-entry *document* field (§7.4.6.1 *apply the history step* step 14.10 / step 12.8 —
**Document object identity**, never URL comparison), modeled as `document_sequence`:

```
enum TraversalKind { SameDocument { state: Option<Vec<u8>>, scroll: Option<(f64,f64)> }, Rebuild }
```

- `target_index == current` (a `go(0)`) → **`Rebuild`** (a **reload** — History.go step 4 = "If delta is
  0, then reload navigable"; §0-CR CR-2), **NOT** a same-document no-op. This subsumes the old
  `documentsEntryChanged=false` no-op: `go(0)` is the only JS-reachable identical-entry traversal, and it
  reloads (`back`/`forward` never target the current entry, `go(n≠0)` targets a different one).
- `entries[target_index].document_sequence == entries[current].document_sequence` (a **different** entry,
  **same** document) → **`SameDocument { state, scroll }`** (read from the **peeked target entry** — §6.5
  DR-1) → the **no-rebuild path**, reusing/generalizing 5b's `fragment_navigate`: `set_current_url` +
  `commit_index` (5a) + restore scroll (§6.6) + `deliver_history_step_events` with the **general form
  `popstate_state = Some(target_entry.classic_history_api_state.clone())`** + `hashchange = Some(...)` iff
  the fragment differs. The outer `Some` = "same-document traversal ⇒ fire popstate" (**state-agnostic**,
  §4.5); the inner `Option` = the entry's serialized state — `Some(bytes)` for a pushState'd entry, **`None`
  for a plain-nav / boa-`None` entry ⇒ popstate fires with `state = null`** (`Some(None)`, NOT `None` —
  `None` would *skip* popstate, spec-wrong for a same-document traversal). This **fills the
  `reconstruct_history_state` stub** (`history_events.rs:205`): `Some(bytes) => structured_deserialize(vm,
  &bytes)` (the `None` arm at :203 already yields `Null`). 5b's `fragment_navigate` fires `Some(None)`; the
  traversal parameterizes the inner `Option` from the entry — **one primitive, the popstate-state +
  scroll-intent parameterized** (One-issue-one-way; incremental membership, NOT a fork). `current_state`
  coherence (J3) holds: `reconstruct_history_state` sets `navigation.current_state` (to the deserialized
  value or `Null`) before firing (5b line 93), so `history.state === popstate.state` even on the no-state
  branch (DR-3).
- else (a **different** `document_sequence` — a genuinely cross-document entry) → **`Rebuild`**: the
  existing rebuild path (`build_pipeline_from_loaded`) + the pre-eval seed (§6.5); fires **no**
  popstate/hashchange (`documentIsNew=true`).

**Both shells drop `classify_navigation` from the traversal path** and match on `resolve_traversal`; it
survives only on the fresh-fragment-nav (`Push`) path (§0-CR CR-1).

### §6.5 Cross-document `history.state` seed — restore-WITHOUT-fire (J5/J6)

A cross-document traversal rebuilds the pipeline (fresh VM). §7.4.6.2 step 6.3 restores `history.state`
**before** step 8.4 "scripts may run", **without** firing popstate (step 6.4 skipped, `documentIsNew=true`).
So the seed is a distinct **restore-only** path, NOT a `deliver_history_step_events` call (which fires — J6):

- New `HostDriver::set_history_state` (NEW) `(&mut self, serialized: Option<Vec<u8>>)` (VM: `current_state =
  structured_deserialize(bytes)` or `Null`; **boa no-op stub** — flip-inert), installed at the pre-eval
  chokepoint in `run_scripts_and_finalize` (`pipeline.rs:189-200`, the S5-4b referrer-seed seam), so the
  rebuilt document's initial scripts read the restored `history.state` (J5).
- Threading: a null-defaulted `Option<Vec<u8>>` on `build_pipeline_from_loaded` (6 call sites, verified
  2026-07-09 §5.5; 5 pass null, only `content/navigation.rs`'s cross-doc traversal arm carries a value).
  **Read-source (DR-1 — data-flow ordering)**: the seed value is the **peeked target entry**'s
  `classic_history_api_state` — `entries[target_index]` via the §6.2 `entry(index)` accessor, where
  `target_index` is the index `peek_back`/`peek_forward`/`peek_go` already returns (carried in
  `HistoryCursorOp::Commit(target_index)`) — **NOT `nav_controller.current()`**. Post-5a peek-then-commit
  moves the cursor (`commit_index`) **after** the rebuild (`build_pipeline_from_loaded` at
  `content/navigation.rs:178` → `commit_index` at :212; app-mode :352→:354), so at build/seed time
  `current()` still points at the entry being **left** (it would seed the departing document's state).
  (There is no `current()` accessor anyway — only `current_url()`/`current_title()`, `navigation.rs:203/236`.)
  Rides the existing `PreEvalFrameState`-shaped install (one more seed alongside origin/referrer/viewport).

### §6.6 Scroll capture-on-leave / restore-on-arrive (J7; Auto mode)

Routes through the **existing** viewport transport (`take_pending_scroll`/`set_scroll_offset`,
`script-session/engine.rs:389/394`) + `re_render`'s **post-layout** scroll-application seam (the 5b I6 /
Codex-R6-F4 precedent), never a new channel or an inline-in-drain set:

- **Capture-on-leave**: write the current viewport scroll offset into `entries[current_index].scroll_position`
  (§7.4.6.2 step 6.4.4 "restore persisted state" reads what leave captured). **Write-site ordering (a
  data-flow trap)**: the capture must read `state.viewport_scroll` **in `handle_history_action`, before it
  calls `handle_navigate`** — NOT merely "before `commit_index`". On the cross-document rebuild path
  `handle_navigate` resets `state.viewport_scroll = ScrollState::default()` (`content/navigation.rs:194`)
  **earlier** than `commit_index` (:212), so a capture placed between :194 and :212 reads `(0,0)` and the
  entry restores to top. Same-document traversal (no rebuild, no reset) is unaffected, but the single
  capture site must precede the reset for both.
- **Restore-on-arrive**: apply the target entry's `scroll_position` via the transport, resolved through
  `re_render`'s clamp-against-content-size + `scrollX`/`scrollY` echo + document-root `ScrollState` (so the
  shipped display list carries the applied offset, not an un-applied one).
- **Auto mode** only (`ScrollRestorationMode::default()`); the writable setter + `Manual`-suppression → D3.
- **Scroll-vs-popstate-handler ordering is flip-inert** (boa fires no popstate → no handler `scrollTo` to
  order against): inherited by the S5-6 flip deliverable (5b §9), NOT solved in 5c.

### §6.7 Engine-boundary classification (J8 — the flip-inert vs engine-agnostic split)

- **Flip-inert** (VM-tested now, live at S5-6): the state VALUE round-trip is flip-inert **end-to-end**
  because **boa passes `None`** on every `serialized_state` (§6.3) — so the entry stores `None`, a traversal
  restores `null`, and the seed reads `None`, on the boa-live path. Only the VM path serializes real state.
  Also flip-inert: popstate firing on same-document traversal (boa's `deliver_history_step_events` no-op),
  the cross-doc seed (boa `set_history_state` no-op), and the inherited post-handler-effect / index-publish
  cluster (5b §9).
- **Engine-agnostic-now** (observable in the live boa shell): the scroll capture/restore on traversal
  (scroll is not JS-state-dependent — the shell owns the viewport), the `NavigationController` state/scroll
  **plumbing** (structural; the field re-type + push/replace params land now, J1×J8), and the
  same-document-traversal **no-rebuild classification** (a same-doc back/forward stops rebuilding — a
  network-request oracle sees zero re-fetch, focus persists).

### §6.8 ECS-native lens

- **Session history = a browsing-context/navigable fact** in the shell-owned `NavigationController` — a
  legitimate shell side-store (CLAUDE.md ECS-native exception (b)), NOT an ECS component. The
  `serialized_state` on `HistoryEntry`/`HistoryAction` is a transient serialized blob on the existing
  side-store + FIFO intent — B1-migration-neutral (no new per-VM per-entity state; the VM's
  `current_state: JsValue` is the pre-existing transient the seed/traversal overwrites, §5.2).
- **`document_sequence`** (§0-CR CR-1) is another per-entry side-store **scalar** on `HistoryEntry` (a `u64`
  document-identity id) + a `next_document_sequence` counter on `NavigationController` — an accepted shell
  side-store (browsing-context/navigable fact, ECS-native exception (b)), NOT an ECS component; it re-stamps
  by-value on push/replace/reload with no per-VM per-entity state, so it is B1-migration-neutral.
- **Focus**: a same-document traversal (no rebuild) preserves `ElementState::FOCUS` (like 5b); a
  cross-document traversal rebuilds → fresh EcsDom (by-construction reset). Zero ad-hoc focus state.

---

## §7 Design decisions (the plan-review ratify-points)

| Decision | Resolution proposed | Basis |
|---|---|---|
| **Q-SERIAL-REP** (§4.1 — headline) | align `HistoryEntry.classic_history_api_state: Option<String>` → **`Option<Vec<u8>>`** (= the landed `SerializedState` wire); one representation end-to-end; correct cluster §4.5; D1 = seam-body swap | §4.1 vs the 5b `Vec<u8>` wire |
| **Q3** state serialize fidelity | **JSON-shortcut interim** to `Vec<u8>` (UTF-8 JSON, sharing the worker JSON-shortcut core); full `StructuredSerializeForStorage`-to-bytes → **D1** (folds with `#11-worker-structured-serialize`) | §4.2 |
| **Q-SERIAL-HOME** | extract the shared `structured_serialize_for_storage`/`structured_deserialize` **byte seam now** (both history + worker converge at D1), NOT a parallel history-local JSON encoder | §4.2 / §6.2 One-issue-one-way |
| **Q-D5** task-queue boundary | **dedicated S5-5d**, NOT in 5c (edge-dense → own plan-review; disjoint seam; 5c ships on the collapsed synchronous model; no live-reachable E7 facet; inherits 5b's flip-inert deferral) | §4.3 lens-converge |
| **Q-ABOUTBLANK** | `pushState-on-initial-about:blank → replace` (§7.4.4 step 4) **deferred** (D8) — not representable (no flag); the flag is shared infra, minted deliberately when first load-bearing | §4.4 / §5.6 |
| **Q-SEED** | cross-doc `history.state` seed = new **restore-WITHOUT-fire** `HostDriver::set_history_state` at the pre-eval seam (S5-4b precedent), boa no-op; NOT a `deliver_history_step_events` call (would spuriously fire popstate on load) | §6.5 J6 |
| **Q-FLIPINERT** | ratify: the state VALUE round-trip is **flip-inert end-to-end** (boa passes `None`); scroll persist/restore + no-rebuild-traversal classification is **engine-agnostic-now** | §6.7 |
| **Q-SCROLL** | Auto-mode capture-on-leave / restore-on-arrive via the existing viewport transport + `re_render` post-layout seam; Manual + writable setter → **D3** | §6.6 |
| **Q-MATRIX** | ratify the §4.5 traversal matrix: same-doc traversal fires popstate(restored)+hashchange(frag-gated); cross-doc restores state (6.3) but fires neither (6.4 skipped) | §4.5 spec trace |

---

## §8 Edge matrix (5c owned edges — cluster §6)

| # | Edge | 5c discharge |
|---|---|---|
| **E4** | state serialize/deserialize round-trip + cross-document survival | **owns**: §6.2/§6.3/§6.5, one `Vec<u8>` representation (J1), serialize-order (J2), seed-before-scripts (J5), restore-without-fire (J6) |
| **E6** | engine-agnostic-now (scroll/plumbing) vs flip-inert (state value + firing) | **owns**: §6.7 boa-passes-`None` end-to-end; VM-tested now |
| **E8** | `StructuredSerializeForStorage` fidelity (JSON-shortcut interim vs full) | **owns**: §4.2, shared byte seam, D1 fold |
| E1 | drain order (5a owns) | reads: the serialize rides the existing history drain |
| E2 | same-document classification (5b owns) | reuses `classify_navigation` **only on the fresh-fragment `Push` path**; the traversal same-doc-vs-cross-doc decision uses per-entry `document_sequence` identity + `resolve_traversal` (§0-CR CR-1 / §6.4) |
| E3 | origin stable across no-rebuild (5b owns) | reads: same-doc traversal no-rebuild keeps origin (by-construction) |
| E5 | focus persists on same-doc nav (5b owns) | reads: same-doc traversal no-rebuild keeps `ElementState::FOCUS` (§6.8) |
| E7 | traversal+nav same-turn (D5/5d owns) | narrows: same-doc traversal removes the rebuild for same-doc cases; the residual → 5d (§4.3) |
| E9 | fragment-nav popstate counterintuitive (5b owns) | reads: traversal's restored-state popstate is the sibling (§4.5) |
| E10 | two nav impls | applies the traversal-restore branch to both shells (§5.7) |

Edge-dense (E4+E6+E8) — terminal under **this memo's** plan-review (base-case rule; the cluster §0
peel-off hatch is NOT triggered — §4.1/§4.2 fully specify the serialization design).

---

## §9 Test strategy (supported-surface; engine-agnostic-now vs flip-inert split)

Boa stays the live shell engine; oracles = engine-level VM tests + targeted shell integration (the
S5-3/S5-4/5b posture), with the engine-agnostic-now vs flip-inert split (§6.7) explicit per assertion:

- **`elidex-navigation` unit** (engine-agnostic): `push`/`replace` store `serialized_state` + `scroll_position`;
  the traversal read path exposes the target entry's state+scroll; eviction FIFO with state; the field is
  `Option<Vec<u8>>`.
- **Engine-agnostic-now** (passes in the live boa shell): a same-document back/forward does **NOT** re-fetch
  (network-request oracle: zero requests) + focus persists; scroll captured-on-leave is restored-on-arrive
  (off-screen offset **reaches the display list / clamped + echoed**, not un-applied); a cross-document
  traversal still rebuilds (regression pin); **`go(0)` reloads** (History.go step 4; §0-CR CR-2 —
  network-request oracle sees a **re-fetch**, NOT a no-op; fires no popstate) — the §4.5/§6.4 `go(0)` pin
  asserts the **reload/rebuild**, not the superseded same-document no-op.
- **Flip-inert** (VM-tested now, live at S5-6): `cargo test -p elidex-js --all-features` drives
  `deliver_history_step_events(Some(Some(bytes)), …)` and asserts popstate fires with `history.state ===
  StructuredDeserialize(bytes)` (round-trip: `pushState({n:1}); pushState({n:2}); back()` → popstate `{n:1}`
  + `history.state === {n:1}`); same-doc-traversal-across-a-fragment fires popstate + hashchange; the
  cross-doc seed (`pushState({n:1})` → navigate away → `back()` → the fresh document's initial script reads
  `history.state === {n:1}`, **no** popstate); serialize→deserialize value round-trip; **a cyclic/BigInt
  `pushState` succeeds and degrades to `null` on restore** (§0-CR CR-3 — NOT a DataCloneError throw); **a
  `pushState(function(){})` succeeds with null state** (the D1-owned opposite-deviation pin — flips to a
  DataCloneError throw at D1); a user `ThrowValue` during serialize propagates **before** the URL
  side-effect (J2); boa passes `None` (compile pin) + boa's no-op
  stubs (`deliver_history_step_events` / `set_history_state`) as the pre-flip baseline. **Registered S5-6
  flip deliverable** (mirrors 5b): the live-shell traversal-popstate test + the scroll-vs-popstate-handler
  ordering, once the VM is the engine.
- **WPT subset**: `html/browsers/history/the-History-object/*` (pushState/replaceState/popstate) +
  `html/browsers/browsing-the-web/history-traversal/*` — engine-independent equivalents (harness scope
  judged at impl; the unit/integration above is the regression gate per "Supported-surface testing").
- Workflow: plan-verify grep vs HEAD → impl in the `s5-5c-history-state-traversal` worktree → `/pre-push`
  → `/external-converge` → squash merge (umbrella §11).

---

## §10 Deferred carves + audits (cap ≤3; actual 5c = 3 carves [D1, D3, D8] — §0-CR CR-5 mints NO 4th slot)

- **D1 `#11-history-state-structured-serialize-fidelity`** (carve; FOLDS with `#11-worker-structured-serialize`):
  full `StructuredSerializeForStorage`-to-bytes for `history.state` (Blob/File/Map/Date/cyclic) vs the
  JSON-shortcut interim (§4.2). **CR-3 opposite-deviation sub-gap** (§0-CR CR-3): the interim drops **BOTH**
  directions — a cloneable-but-non-JSON value (BigInt/cyclic/Map/Date) wrongly **can't restore** (degrades
  to `null`), AND a genuinely non-cloneable top-level value (`function`/`symbol`, which the spec
  **requires** to throw `DataCloneError`) wrongly **doesn't throw** (`JSON.stringify` → `undefined` →
  `Ok(None)` → `pushState(function(){})` silently succeeds with null state — a script-observable contract
  deviation). The JSON shortcut cannot distinguish the two (both surface as "JSON can't encode"), so neither
  is fixable without the full structured-clone walker (this D1); the D1 flip is pinned by the §9
  `pushState(function(){})` test. **Audit**: spec-core? yes (§7.2.5 step 3 / §2.7.5); one-way? yes — the
  interim serializes/deserializes through one shared byte seam (§6.2), upgrading swaps the seam **body**,
  the `Vec<u8>` field/wire unchanged; pragmatic-debt? drops non-JSON state shapes (rare for classic History
  state — usually plain objects), matching the already-tracked worker JSON-shortcut deviation; repeat-signal?
  the same shortcut recurs at worker/SW/IndexedDB storage. **Trigger**: full structured-clone-to-storage-bytes
  work (the worker-shortcut slot's trigger — the two converge). **Re-eval**: with the worker-shortcut slot;
  backstop **2026-10-31**.
- **D3 `#11-history-scroll-restoration-manual-mode`** (carve; cluster §8-D3): the writable
  `history.scrollRestoration` setter + `"manual"`-mode suppression of auto scroll-restore. 5c implements
  `Auto` capture/restore; the getter stays `"auto"` (§5.5). **Audit**: spec-core? yes (§7.4.1.1 scroll
  restoration mode); one-way? yes — the mode is already a `HistoryEntry` field (`scroll_restoration`); the
  setter writes it + the restore (§6.6) consults it; pragmatic-debt? interim always auto-restores (a page
  opting out is not honored — minor, rare); repeat-signal? none. **Trigger**: a site/WPT exercising manual
  scroll restoration. **Re-eval**: backstop **2026-10-31**.
- **D8 `#11-initial-about-blank-flag`** (carve; NEW, §4.4): mint the shared `is initial about:blank` flag
  (§7.4.4 step 4 `pushState → replace`; also §7.4.6.2 step 7.4 `NavigationActivation` + the navigate
  algorithm). **Audit**: spec-core? yes (§7.4.4 step 4); one-way? yes — one per-VM boolean (set at
  `NavigationState::new`, cleared on the first real navigation) consulted at the pushState→replace coercion +
  the future Navigation-API sites; pragmatic-debt? interim = a pushState during the (rare) initial-about:blank
  window pushes rather than replaces (a `history.length` off-by-one in a pre-navigation window); repeat-signal?
  yes — the flag is load-bearing for ≥3 spec sites (verified 2026-07-09 — pushState-replace §7.4.4 step 4, NavigationActivation §7.4.6.2 step 7.4, navigate), so
  minting it once deliberately beats smuggling a fragment into 5c. **Trigger**: the Navigation API surface, or
  a site/WPT exercising pushState on the initial about:blank. **Re-eval**: backstop **2026-10-31**.
- **Chrome Back/Forward button routing (§0-CR CR-5, F2)** — **NO new slot** (cap stays at 3): app-mode is
  fixed **in-PR** by routing `handle_chrome_action` through `traverse_to` (peek → `resolve_traversal` →
  same-doc-restore or atomic rebuild-then-commit); content-mode is routed **in-PR** too, UNLESS it
  **structurally** requires the D5 task-queue model to route atomically (not merely "more work") — in which
  case it folds into the **existing no-slot §8-D4 driver-unification audit**, NOT a minted 4th `#11` slot.
  Ratify the disposition at re-plan-review.
- **Cluster/5b carves referenced (not 5c's)**: D2 (fragment-nav focusing step, 5b) / D4 (nav-driver
  unification audit, no slot) / D5 (task-queue model → **S5-5d**, §4.3) / D6 (thread-mode replace-honoring,
  5b) / D7 (iframe fragment nav, 5b) / `#11-session-history-index-vm-publish` (5b, S5-6 flip).

**Not carved (dispositioned in-memo, no slot)**: `hasUAVisualTransition` (always false); Navigation API
(non-goal, own program); bfcache / cross-document-entry *document* reconstruction (non-goal — 5c seeds a
*rebuilt* document, not the exact prior one). Defer-ledger reconciliation (closing
`#11-history-state-traversal-popstate-fidelity` + registering D1/D3/D8) is a landing deliverable.
**Cluster §8 carve-count sync**: the cluster memo §8 header states "5c = 2 (D1, D3)" (~line 958) and its
"Not carved" note (~line 1086) anticipated about:blank as "a one-line audit note"; D8
(`#11-initial-about-blank-flag`) promotes it to a carve, so the 5c landing updates the cluster §8 count
line to **5c = 3 (D1, D3, D8)** alongside the §4.1 `Option<String>`-sweep reconciliation.

**Touch-set line counts** (post-5b, HEAD `a904ea81`): `content/navigation.rs` 675, `app/navigation.rs` 511,
`elidex-navigation/navigation.rs` 647, `vm/host/history.rs` 303, `vm/host/history_events.rs` 242,
`script-session/engine.rs` 485, `script-session/navigation.rs` 440, `pipeline.rs` 774. **All under 1000.**
⚠ **`elidex-navigation/navigation.rs`**: 647 = pristine HEAD; the in-progress 5c working tree is **~839**,
and the §0-CR `document_sequence` document-identity redesign (the new `document_sequence` field +
`next_document_sequence` counter + `push_same_document`/`replace_same_document`/`restamp_current_document`
+ `resolve_traversal`/`TraversalKind`) lands it at **~870** — still **<1000**, so no prereq split is owed,
but this file is the closest to the ceiling and should be re-checked at impl.
⚠ **`structured_clone.rs` = 1063 (already >1000)**: 5c homes the new byte serialize/deserialize seam
**beside** it — assess at impl whether the serialize/deserialize byte functions form a cohesion seam for a
`structured_serialize.rs` sibling (the split-on-touch discipline: a real seam → a standalone prereq split
BEFORE the feature; a fused-clone-adjacent helper cluster is the candidate). **Decide the split at 5c
kickoff** per the touch-time discipline (source-file, >50-LoC-add to a >1000 file = the Axis-5 backstop
too). **Default lean**: a standalone `structured_serialize.rs` **prereq split PR** (not bundled into the 5c
feature PR) IF the JSON-shortcut pair + D1's future full-fidelity graph-walker forms a real cohesion seam
(likely — StructuredSerializeForStorage-to-bytes is a distinct concern from the in-memory `clone_value`);
an in-file `beside` add only if it stays a <50-LoC thin wrapper over `natives_json::stringify_to_string`.

---

## §11 Open questions for `/elidex-plan-review`

The design decisions §7 are the ratify-points; the genuinely open calls for plan-review:

- **Q-SERIAL-REP (§4.1 — headline)**: ratify aligning `HistoryEntry.classic_history_api_state` to
  `Option<Vec<u8>>` (one representation end-to-end, matching the landed 5b `SerializedState` wire) vs the
  cluster memo's `Option<String>`; ratify the **cluster-memo edit** (the full `Option<String>` sweep —
  §4.5 + E8 + §9-Q3 — plus the §8 carve-count `5c = 2 → 3`) as part of the 5c landing.
- **Q-SERIAL-HOME (§4.2/§6.2)**: extract the shared `structured_serialize_for_storage`/`structured_deserialize`
  byte seam **now** (history + worker converge at D1), or keep a history-local JSON encoder + converge only
  at D1? Lean: extract now (avoids the parallel-encoder duplication).
- **Q-D5 (§4.3)**: confirm D5 (`#11-session-history-task-queue-model`) → **dedicated S5-5d**, NOT 5c (5c
  stays on the collapsed synchronous model; edge-dense; disjoint seam; no live-reachable E7 facet).
- **Q-ABOUTBLANK (§4.4)**: confirm `pushState-on-initial-about:blank → replace` → **D8** (mint the shared
  flag when first load-bearing), NOT folded into 5c (not representable; shared infra).
- **Q-SEED (§6.5)**: ratify the cross-doc `history.state` seed as a **restore-WITHOUT-fire**
  `set_history_state` at the pre-eval seam (NOT a `deliver_history_step_events` call — which would fire
  popstate on document load, `documentIsNew=true`).
- **Q-FLIPINERT (§6.7)**: ratify that the state VALUE round-trip is **flip-inert end-to-end** (boa passes
  `None`), while scroll persist/restore + the no-rebuild-traversal classification is engine-agnostic-now.
- **Q-SPLIT-CLONE (§10)**: confirm the `structured_clone.rs` (1063, >1000) touch — home the new byte seam
  beside it and assess a `structured_serialize.rs` prereq split at kickoff — respects the touch-time
  discipline (a standalone split PR if a real cohesion seam, not bundled into the 5c feature PR).
- **Q-MATRIX (§4.5)**: confirm the traversal event matrix (same-doc fires popstate-restored + frag-gated
  hashchange; cross-doc restores state via step 6.3 but fires neither).
</content>
</invoke>

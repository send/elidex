# S5-5b — synchronous fragment navigation + the shared same-document primitive

Per-PR plan-memo (impl-PR-同梱) for the **S5-5b** slice of the S5-5 navigation/history cluster.

- **Cluster SoT** (decomposition §0, spec substrate §2, edge matrix §6, deferred carves §8):
  `docs/plans/2026-07-s5-5-navigation-history-enforcement.md`. This memo carries the **5b slice at
  impl-ready depth**, re-grounded on the current HEAD, and **corrects one load-bearing spec error in the
  cluster memo's classifier design** (§4).
- **Umbrella**: `docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md` (§5 row S5-5). 5b is a
  **FLIP-precondition** slice — boa stays the live engine; VM-capability + shell-correctness work landing
  BEFORE the S5-6 boa→VM flip.
- **Gate**: `/elidex-plan-review` BEFORE impl (CLAUDE.md "Edge-dense work = 実装前 plan-review 必須"; 5b is
  the densest slice — cluster §6 E2+E3+E5+E6+E9).
- **Anchor = the ideal end-state** (`feedback_plan-memo-anchor-on-ideal-not-incremental`).

**Re-grounding**: cluster-memo cites were verified against **pre-5a** HEAD `31c1f76d`; **S5-5a (#449,
`539a09ba`) restructured the drain** (peek-then-commit, Vec-drain, `HistoryCursorOp`,
return-true-on-supersede) + shifted line numbers. **All cites here are grep-verified against HEAD
`539a09ba` (2026-07-05)**; every spec §/anchor is `webref heading --exact`-verified 2026-07-05 (source
`html` multipage); the §4 correction is verified by algorithm trace + an empirical `url`-crate check.

---

## §1 Scope + slots

### §1.1 What 5b is

5b introduces the **same-document (fragment) navigation path** as a first-class shell branch: a
fragment-classified navigation does **no** pipeline rebuild — it updates the VM `current_url`, commits the
session-history entry (*finalize a same-document navigation*), scrolls to the fragment, and fires the
history-step events (popstate-null + hashchange) via a new engine back-channel — and the persistent VM's
`document_origin()` stays correct **by construction**. It lands the **shared same-document primitive** with
fragment nav as its first live consumer (5c reuses it for traversal — cluster §0.1, no strangler).

### §1.2 Slots closed

- **`#11-synchronous-fragment-navigation`** — fragment nav today does a FULL pipeline reload (the
  `is_fragment_only` flag's only consumer is the SW-skip; `load_document` runs regardless — §5.1).
- **`#11-vm-navigation-origin-resync`** (corollary, closed **by-construction** — §6.5): after a
  same-navigable no-rebuild nav the VM `document_origin()` must stay correct (fetch/WS/ES/postMessage key
  on it). Dormant today (every nav rebuilds → fresh origin); becomes live exactly when the no-rebuild path
  is introduced (= 5b). S5-4d (#448) explicitly deferred it here.

### §1.3 Non-goals (inherit cluster §1.3)

- **Navigation API** (`navigation.*`, `navigate`/`navigatesuccess`, `navigation.entries()`) — separate
  modern surface; out of the classic-History subset. No slot owed.
- **The fragment-navigation focusing step** (§7.4.6.4 scroll-to-the-fragment step 3.6 "run the focusing
  steps" + 3.7 "move the sequential focus navigation starting point") — 5b lands the *scroll*; the
  focus-move is a refinement (carve **D2**, cluster §8-D2, gated on the S2 focus surface).
- **`history.state` threading + traversal state/scroll restore** — 5c (5b fires popstate with **state =
  null**, the *complete correct* fragment-nav behavior; no state serialization here).
- **`location.hash` setter as a distinct property + its §dom-location-hash step-8 redundant-set bailout** —
  the VM registers only a whole-`href` RW accessor (`hash` is RO). 5b drives fragment nav through the
  *existing* `NavigationRequest` enqueue (`location.href=` / `<a href>` / `assign`). **Audit A1** (§10).
- **bfcache / cross-document-entry reconstruction / `hasUAVisualTransition`** — non-goals
  (`hasUAVisualTransition` always `false`).
- **iframe fragment navigations** — the iframe nav path is a distinct 3-arg `handle_navigate`
  (`content/iframe/thread.rs:193`); 5b's `#11-synchronous-fragment-navigation` closure is **top-level +
  app-mode only**. Iframe `#fragment` navs keep rebuilding → carve **§10-D7**.
- **Thread-mode `location.replace()` honoring** — the `NavigationType` enum conveys `Replace`, but
  thread-mode still commits it as push (pre-existing); honoring it → carve **§10-D6**.
- Per-VM state → ECS component migration — B1 (post-S5, umbrella §0.1).

---

## §2 Coupled-invariant enumeration (edge-dense — Pre-condition #3)

5b is edge-dense (5 intersecting axes). The invariants it **simultaneously** satisfies, and each
load-bearing **pairwise intersection** (the cross-cut a prose "既存 seam を再利用" one-liner would drop):

**Invariants**
- **I1 same-document determination** — Fragment ⟺ navigate §7.4.2.2 step 15's four conjuncts hold:
  `documentResource null AND response null` (bodyless/responseless — a **call-site** guard, §6.3) AND
  `equals-excluding-fragments(cur, tgt) AND tgt.fragment ≠ null` (the **URL-pure** classifier); else
  CrossDocument (E2, §4).
- **I2 origin stability** — `document_origin()` unchanged across the no-rebuild nav (E3, §6.5).
- **I3 focus persistence** — `ElementState::FOCUS` survives; zero ad-hoc reset (E5, §6.6).
- **I4 event-firing correctness** — popstate(null) always; hashchange iff frag differs; popstate SYNC,
  hashchange ENQUEUED (E9, §4.4 / §6.3).
- **I5 engine-boundary flip-inertness** — VM fires, boa stubs; the shell path is engine-agnostic-now (E6,
  §6.3).
- **I6 scroll-application currency** — element-resolution + offset applied POST-layout via `re_render`, not
  inline in the drain (§6.4).
- **I7 nav-type ≠ classification** — navigation TYPE (`NavigationType { Push, Replace, Reload }`, §6.3) is
  orthogonal to the URL classification. **Reload** must be distinguished (nav_type→`Keep` + `cursor_op ==
  Push` guard, §6.3) so a `location.reload()` of a fragment-URL rebuilds; push-vs-**replace** honoring is a
  pre-existing thread-mode drop (§5.2(b)), deferred (§10-D6). The classifier stays type-agnostic.

**Intersections (load-bearing)**
- **I1 × I2** — the SameDocument branch is the SINGLE site that both skips rebuild AND leaves
  `document_origin()` derivation untouched (`set_current_url`-only; override never touched). Origin
  stability is a **corollary of the classification**, not a separate mechanism → the slot closes
  by-construction (§6.5), no active resync.
- **I1 × I3** — the same no-rebuild branch preserves `ElementState::FOCUS`; today's wrong focus-reset **is**
  the rebuild. Focus persistence is a corollary of not rebuilding the EcsDom.
- **I1 × I4** — the classifier decides Fragment-vs-CrossDoc (rebuild-or-not); the event **hub**
  independently decides which events fire (hashchange gated on frag-differ INSIDE update-document step
  6.4.5). **Classification output ≠ event-firing decision** — conflating them is exactly the cluster-memo
  error (§4: "fragments-differ" tried to make the classifier carry the hashchange gate).
- **I4 × I5** — popstate is a VM-reconstruction of live `JsValue` state fired SYNC; hashchange rides
  `EventPayload` ENQUEUED; boa stubs both ⇒ firing is flip-inert AND popstate-strictly-before-hashchange
  must hold across ONE back-channel call.
- **I4 × I2** — the events fire at the **persistent** VM's Window whose origin is unchanged (I2) — event
  delivery and origin-keying share the same not-rebuilt VM.
- **I1 × I6** — scroll-to-fragment RESOLVES an element (strictly harder than `scrollTo`'s clamp); because
  the classification means no rebuild, layout is the EXISTING document's (current at the drain), so
  resolution is against live layout and the offset must ride the post-layout `re_render` seam.
- **I1 × I7** — the URL classifier (I1) is nav-type-agnostic; the branch separately gates nav-type via the
  `NavigationType` enum (§6.3): a **Reload** must NOT take the no-rebuild path even though `target ==
  current` classifies `SameDocument` (Reload→`Keep`, excluded by `cursor_op == Push`); **Replace** honoring
  is a deferred pre-existing drop (§10-D6). Entangling nav-type into the URL classifier (as the
  reload-via-URL-guard shortcut would) is the anti-pattern the enum avoids; entry-point (not URL) is the
  spec discriminant (HTML §7.4.3, reload `isSameDocument=false`).

---

## §3 Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §7.4.2.2 Beginning navigation | navigate step 15 (fragment-nav trigger — 4 conjuncts) | `equals-excl-frag AND tgt-frag-non-null` (classifier) ⊕ `request.is_none()` (documentResource) ⊕ vacuous (response) ⊕ `cursor_op==Push` (fresh; excl reload/traversal) ⇒ Fragment; else CrossDocument | `classify_navigation` (NEW, URL pair) + 3 Fragment-branch guards INSIDE `handle_navigate` (§6.3) | ✓ (all 4 conjuncts, factored classifier + 3 guards) | yes (url + POST body → CrossDocument) |
| HTML §7.4.2.3.3 Fragment navigations | navigate-to-a-fragment 11.1 state=null / 12 set-url / 14 hub / 15 scroll / 17 finalize | push (replace deferred §10-D6) | Fragment branch: thread-mode inside `handle_navigate`, app-mode inside `navigate` (`content/navigation.rs` + `app/navigation.rs`) | ✗ (fragment subset; Navigation API non-goal; replace §10-D6; iframe §10-D7) | yes (url / `location.href=` / `<a href>`) |
| HTML §7.4.3 Reloading and traversing | reload = distinct nav-type (not navigate step 15) | `location.reload()` ⇒ `NavigationType::Reload` → drain `Keep` → rebuild | `NavigationType {Push,Replace,Reload}` (NEW enum, replaces `NavigationRequest.replace`) + drain Reload→`Keep` (§6.3) | ✓ (reload excluded from fragment path; href-identical still popstate; app-mode collision resolved) | yes (`location.reload()`) |
| HTML §7.4.6.2 Updating the document | update-document 6.4.3 popstate / 6.4.5 hashchange / 6.3 restore-state | fragment-nav (state=null) — 5b; traversal (restored) — 5c | `deliver_history_step_events` (NEW) hub | ✓ (fragment-nav fire matrix) | yes (state — null in 5b) |
| HTML §7.4.6.4 Scrolling to a fragment | indicated part → scroll (focus-move deferred D2) | id-match / name-match / top-of-document (empty `#`) | scroll-to-fragment via `re_render` post-layout seam | ✗ (scroll delivered; focusing-step §10-D2) | yes (fragment string) |
| HTML §7.1.1 Origins | document origin stable across same-document nav | override preserved / same-URL-tuple derivation | `document_origin()` unchanged (by-construction) | ✓ (URL-fragment change ≠ origin change) | — |
| HTML §7.4.4 Non-fragment synchronous "navigations" | URL-and-history-update note (pushState fires no popstate/hashchange) | pushState/replaceState = NOT the 5b fire path (5c) | matrix guard (5b asserts these do NOT fire on the fragment path) | ✓ (the negative — E9 guard) | — |

**Breadth**: K=1 (HTML), M=7 → **ok (single PR scope)**.

### §3.1 User-input touch audit (`feedback_trust-boundary-enumerate-upfront`)

Every input rides an EXISTING validated seam — **no new trust boundary is opened; 5b narrows one**:
- url strings → the `resolve_nav_url` chokepoint (`crates/shell/elidex-shell/src/app/navigation.rs:360`,
  `BLOCKED_NAV_SCHEMES` = `["javascript","vbscript"]` at :354) + the VM `resolve_url` seam
  (`crates/script/elidex-js/src/vm/host/location.rs:55`).
- fragment strings → `url::Url::fragment()` (a fragment nav stops hitting the network entirely).
- nav-type → the **NEW** `NavigationType {Push,Replace,Reload}` enum replacing `NavigationRequest.replace`
  (`crates/script/elidex-script-session/src/navigation.rs:50`); set engine-internally by the VM setters
  (`location.href=`→Push / `replace()`→Replace / `reload()`→Reload; boa the same). No new external input —
  the nav-type is engine-internal, riding the already-validated `NavigationRequest` seam.

---

## §4 THE classifier correction (Axis-4 headline — corrects cluster §4.2 / §5.2.2 / §7)

> The load-bearing finding of authoring 5b. It **contradicts the landed cluster memo's classifier
> predicate**, is verified by direct spec trace + empirical `url`-crate check, and is the #1 plan-review
> ratify-point. Matches `feedback_plan-review-verify-preserve-existing-spec-claims` (re-verify a plan's
> "existing impl behavior" spec claims at Axis 4).

### §4.1 The cluster memo's predicate is spec-wrong

Cluster §4.2/§5.2.2/§7 define the classifier as **`equals-excluding-fragments AND fragments-DIFFER`** and
assert the existing `is_fragment_only` "misses `/a#x → /a` (removal) and `/a#x → /a#` (emptied)" and "must
be generalized". **Both claims are wrong.** The spec's fragment-navigation trigger is the **navigate**
algorithm (§7.4.2.2) **step 15**, verbatim:

```
15. If all of the following are true:
      * documentResource is null;
      * response is null;
      * url equals navigable's active session history entry's URL with exclude fragments set to true; and
      * url's fragment is non-null,
    then: 1. Navigate to a fragment; 2. Return.
```

The predicate is **`equals-excluding-fragments AND target-URL-fragment-is-NON-NULL`** — not
`fragments-differ`. Divergence in two cases:

1. **Fragment removal `/a#x → /a`** (target fragment = **null**): the cluster predicate (frags differ)
   classifies it **Fragment** + fires hashchange. The **spec** (target fragment null ⇒ step 15 fails)
   classifies it **CrossDocument** — a full reload. **Real browsers reload on fragment removal.**
   Implementing the cluster predicate turns a reload into a silent no-rebuild + a spurious hashchange.
2. **Identical-including-fragment `/a#x → /a#x`** via `location.href` (target fragment `x`, non-null): the
   cluster predicate (frags equal) → **Reload/CrossDocument**. The **spec** (step 15 satisfied) →
   **Fragment** (popstate state=null, **no** hashchange since `x==x`, re-scroll). Rare, but still wrong.

### §4.2 The existing `is_fragment_only` predicate is already spec-correct

`crates/shell/elidex-shell/src/content/navigation.rs:54-63` computes (verbatim, line 61-62):

```rust
current.as_str().split('#').next() == url.as_str().split('#').next()
    && url.fragment().is_some()
```

`url.fragment().is_some()` **is exactly step 15's "url's fragment is non-null"**. Empirically (url 2.x,
2026-07-05): `Url::parse("http://x/a").fragment() == None`; `…"/a#").fragment() == Some("")`;
`…"#x").fragment() == Some("x")`. So the existing predicate classifies:

| current | target | tgt fragment | existing `is_fragment_only` | spec step 15 |
|---|---|---|---|---|
| `/a` → `/a#x` (add) | | `Some("x")` | **true** (Fragment) | Fragment ✓ |
| `/a#x` → `/a#y` (change) | | `Some("y")` | **true** (Fragment) | Fragment ✓ |
| `/a#x` → `/a` (**removal**) | | `None` | **false** (CrossDoc) | **CrossDoc ✓** |
| `/a#x` → `/a#` (**emptied**) | | `Some("")` | **true** (Fragment) | Fragment ✓ |
| `/a` → `/a#` | | `Some("")` | **true** (Fragment) | Fragment ✓ |
| `/a#x` → `/a#x` (identical) | | `Some("x")` | **true** (Fragment) | Fragment ✓ |
| `/a` → `/a` | | `None` | **false** (CrossDoc) | CrossDoc ✓ |
| `/a` → `/b` | | — | **false** (CrossDoc) | CrossDoc ✓ |

The existing predicate is **spec-correct on every case the cluster memo claims it botches**. The cluster's
"generalization to fragments-differ" is a **regression**.

### §4.3 What 5b actually changes (the real bug is wiring, not the predicate)

The real defect (§5.1): `is_fragment_only` is computed correctly but its **only consumer is the SW-skip
gate** (`if !is_fragment_only { …SW… }`) — `load_document` + rebuild run **regardless**. So 5b:

1. **Preserves** the `fragment().is_some()` (= step-15 non-null) clause; does **not** add "fragments-differ"
   or drop the non-null clause.
2. **Promotes** the bool into a two-way classifier (`SameDocument` / `CrossDocument`) and **upgrades** the
   crude `split('#').next()` string compare to the `url` crate's `equals(exclude fragments)` — a
   **robustness** refinement (default-port / percent-encoding / normalization), semantically identical for
   valid serialized URLs.
3. **Wires** `SameDocument` to the no-rebuild path (the actual fix).
4. hashchange-yes/no is **not** a classifier output — the event **hub** decides it (update-document step
   6.4.5: fire iff `oldURL.fragment ≠ newURL.fragment`), so `/a#x → /a#x` fires popstate but not hashchange,
   falling out of the clean predicate.
5. **Gates the non-URL conjuncts at the call site** — step 15's `documentResource is null` (via
   `request.is_none()`), `response is null` (**vacuous** — `handle_navigate` has no `response` param), and
   the fresh-nav distinction (`cursor_op == Push`, excluding reload/traversal) are **not** URL facts, so the
   URL-pure classifier does not encode them; the Fragment branch (§6.3) does. The `request.is_none()` guard
   is **defensive** in the current tree (the only `Some(request)` caller — `content/form_input.rs` POST —
   strips the fragment via `build_submission_url`'s `set_fragment(None)`, so a body-bearing nav reaches the
   classifier fragment-less ⇒ CrossDocument by conjunct-4 alone); it is the spec-faithful backstop, not
   relied on. The **reload** distinction (`cursor_op`) is by contrast load-bearing — a 5b-introduced
   regression without it (§5.2(a)/§6.3).

### §4.4 The event matrix 5b implements (spec-traced 2026-07-05)

| Operation | popstate | hashchange | history.state after | scroll |
|---|---|---|---|---|
| **Fragment nav** (add / change / emptied / identical-via-href) | **YES**, state = **null** | **YES** iff `oldFrag ≠ newFrag` | null | scroll-to-fragment |
| pushState / replaceState | NO | NO | serializedData (5c) | unchanged |

Trace (webref 2026-07-05): *navigate to a fragment* step 11.1 "Set history's state to null" + step-6 note
"The classic history API state is never carried over" ⇒ state=null; step 12 sets the active document URL;
step 14 → *update document for history step application* (the hub); step 15 *scroll to the fragment*; step
17 *finalize a same-document navigation* (shared commit). Hub: documentsEntryChanged=(latest entry ≠ new
entry)=TRUE, documentIsNew=(latest entry null)=FALSE ⇒ **6.4.3 fire popstate synchronously** (state = restore
= StructuredDeserialize(null) = null) + **6.4.5 queue a global task on the DOM manipulation task source** to
fire hashchange iff `oldFrag ≠ newFrag`. §7.4.4 note verbatim: "only fragment navigation contains a
synchronous call to update document for history step application … popstate events fire for fragment
navigations, but not for history.pushState() calls." **Timing (load-bearing)**: popstate = SYNC "Fire an
event"; hashchange = ENQUEUED "queue a global task … to fire" ⇒ **popstate strictly-before-hashchange**.

### §4.5 Cluster-memo reconciliation (One-issue-one-way)

The corrected predicate has ONE home. **The 5b landing edits the cluster memo** (§4.2 predicate → step-15
non-null; §5.2.2 removes "remove ⇒ Fragment/hashchange" + corrects identical-incl-fragment; §7 truth table
`remove ⇒ CrossDocument`) so the SoT is right. **Plan-review ratifies the correction BEFORE the
cluster-memo edit** (§11 Q-CLASSIFIER).

---

## §5 Current-state (post-5a, HEAD `539a09ba` — re-grounded)

### §5.1 Fragment nav rebuilds; the flag only gates the SW-skip (the slot)

`crates/shell/elidex-shell/src/content/navigation.rs`:
- `handle_navigate` — signature :46-51: `pub(super) fn handle_navigate(state, url: &url::Url, cursor_op:
  HistoryCursorOp, request) -> bool` (returns `true` iff load succeeded + pipeline replaced, post-5a).
- `is_fragment_only` compute :54-63 (predicate line 61-62, §4.2); **only consumer** = the SW-skip gate `if
  !is_fragment_only { …sw_controller_scope()… }` :69-145 (a 30s blocking message-pump wait :110-142 — SW
  path dead today, `sw_controller_scope()` always `None`).
- `load_document` :150 runs **regardless**; `build_pipeline_from_loaded` :168-177; `state.pipeline =
  new_pipeline` :178 — a full fetch + parse + **fresh VM** for a fragment change. **The Fragment branch
  slots in after the classification, before :150, with an early return.**
- `HistoryCursorOp` enum :17-28 (5a): `Push` / `Commit(usize)` / `Keep`. Cursor-move `match cursor_op`
  :200-204, before `set_current_url` :205 / `set_history_length` :206-209 / `notify_navigation` :210.

### §5.2 The drain calls `handle_navigate` — and collapses nav-type to `Push`

`process_pending_actions` :236 (`-> bool`), body to :340. Order (5a): **window-opens → history →
navigation**: `take_pending_window_opens()` :249; HISTORY `take_pending_history()` :275 (Vec), `for action
in &pending_history` :277-299, `if handle_history_action(…) { return true; }` :278/297; NAVIGATION
`take_pending_navigation()` :313 (Option, last-wins), `resolve_nav_url` :314, `handle_navigate(…,
HistoryCursorOp::Push, None)` :317, `return true` :318; pure-history tail :332-335.

**⚠ The drain hardcodes `HistoryCursorOp::Push` at :317, dropping `NavigationRequest.replace` AND carrying
no reload distinction.** Two consequences 5b must account for (grep-verified against
`crates/script/elidex-js/src/vm/host/location.rs`, `NavigationRequest` = `{url, replace}` only,
`script-session/navigation.rs:47-50`):
- **(a) reload asymmetry — 5b-INTRODUCED regression risk.** JS `location.reload()` (`location.rs:261-274`)
  enqueues `NavigationRequest{url: current, replace: true}` → the drain calls `handle_navigate(current,
  Push, None)`. The **chrome** reload button instead uses `HistoryCursorOp::Keep`
  (`content/event_loop.rs:521-534`). So JS-reload is `Push`, chrome-reload is `Keep` — an asymmetry that is
  harmless pre-5b (everything rebuilds) but **breaks once 5b adds the no-rebuild path**: for a fragment-URL
  page (`/a#x`), a JS reload's `target == current` ⇒ classifier `SameDocument` ⇒ the Fragment branch would
  **skip the rebuild** (reload does nothing). 5b MUST distinguish reload (§6.3, the `NavigationType` enum
  fix, which also fixes the pre-existing reload-pushes-a-history-entry bug — `Push` currently
  `nav_controller.push`es on every reload).
- **(b) replace drop — PRE-EXISTING gap.** Because the drain drops `nav_req.replace`, `location.replace()`
  navigations already `Push` (add an entry) rather than replace — a pre-existing thread-mode gap **not**
  introduced by 5b. 5b's fragment nav inherits `Push` (like every current nav); honoring
  `NavigationRequest.replace` for navigations is a separable cross-cutting concern → **deferred** (§10-D6,
  One-issue-one-way: not bundled into the fragment slice).

**The SameDocument classification + Fragment branch live INSIDE `handle_navigate`** (§6.3), NOT at the drain:
that is where the existing `is_fragment_only` (:54-63) lives, where the `request` body-guard is evaluable
(the drain has no body; the only `Some(request)` caller is `content/form_input.rs`, §6.3), and where the
SW-skip gate (:69) it feeds must be preserved. The drain is only extended to map reload → `Keep` (§6.3).

### §5.3 The event constructors exist; no firing site (the flip-inert surface)

- `crates/script/elidex-js/src/vm/host/events_extras.rs`: `native_hash_change_event_constructor` :440-480
  (reads `well_known.old_url`/`new_url`, slots `[oldURL, newURL]`, shape `hash_change`);
  `native_pop_state_event_constructor` :486-519 (reads `well_known.state` default Null, slot `[state]`,
  shape `pop_state_event`). **NO firing site** (repo-wide grep: no dispatch/enqueue; no
  `"hashchange"`/`"popstate"` type-string literals). Constructible from JS, never UA-fired.
- Event-handler IDL attrs `HandlerScope::Window`
  (`crates/script/elidex-script-session/src/event_handler_consumer.rs`: `onhashchange`, `onpopstate`).
- **`elidex_plugin::EventPayload::HashChange(HashChangeEventInit)`** — `crates/core/elidex-plugin/src/
  event_types.rs:171` (init :192-198; routed `events_misc.rs:288`) — a UA-dispatched hashchange **rides the
  existing `EventPayload` window-dispatch**. **NO `PopState` variant** in `elidex_plugin` (grep-confirmed):
  popstate carries `state: any` (live `JsValue`) the engine-indep `EventPayload` cannot hold ⇒ popstate
  needs **VM-specific** delivery (§6.3, the split-driver).

### §5.4 The back-channel mirror surface (the media pattern to mirror)

`crates/script/elidex-script-session/src/engine.rs` (`HostDriver`, :131-466):
- Navigation group (header :245-250): `set_current_url` :257, `current_url` :261-262,
  `take_pending_navigation -> Option<NavigationRequest>` :267, `take_pending_history -> Vec<HistoryAction>`
  :273, `set_session_history(index, length)` :292, `history_length` :295-296.
- **Media group** = the **state-push + deliver-turn** shape to mirror: `set_media_environment(...)`
  :398-405 ("Does NOT fire change on its own") + `deliver_media_query_changes(&mut self)` :413. VM impl
  `crates/script/elidex-js/src/engine.rs`: `set_media_environment` :542-557 / `deliver_media_query_changes`
  :559-561.
- **Scroll transport** (reuse): `take_pending_scroll -> Option<(f64,f64)>` :369-370 / `set_scroll_offset(x,
  y)` :375; VM impl :534-540.
- **"Accretion" doc** :126-130 ("one cohesive method-group per capability … one home, incremental
  membership, never two ways") — sanctions the 5b history-event method-group.
- **`WindowOpenIntent` + `window_open_disposition`** (`script-session/navigation.rs:142-151` / :211-237) =
  the S5-4c canonical form to MIRROR (engine-indep decision + typed intent on the session seam, natives
  marshal-only).

### §5.5 `document_origin()` (the by-construction origin surface)

`crates/script/elidex-js/src/vm/host/navigation.rs:347-364`: **override**
(`host_data.document_origin_override()`) :349-353 → `SecurityOrigin::from_url(&current_url)` :354 → opaque
fallback `fallback_opaque_origin()` :359-361. `set_current_url` **never touches the override**. VM impl
`origin()` `engine.rs:475-477`.

### §5.6 Two shell nav impls (5b touches BOTH)

- **Thread mode** — `content/navigation.rs` (above).
- **Inline app mode** — `crates/shell/elidex-shell/src/app/navigation.rs`: `process_pending_navigation`
  :12-75 (`-> bool`, same window→history→nav order), `navigate` :82-101, `navigate_to_history_url`
  :108-123 (`-> bool`), `handle_history_action` :192-261 (`-> bool`), `apply_state_change` :332-348,
  `resolve_state_url` :311-326, **`resolve_nav_url` :360-371** (chokepoint; `BLOCKED_NAV_SCHEMES` :354).

Both must gain the SameDocument branch. The shared *primitive* (classifier + entry commit + event-delivery
back-channel) is engine-indep, so the duplication is confined to the two thin drivers (cluster §8-D4 —
unifying the drivers is out of scope).

---

## §6 Ideal architecture (5b)

### §6.1 Layering ledger (per surface)

| Surface | Home | Layer |
|---|---|---|
| same-document classifier (Fragment vs CrossDocument) | `elidex-navigation` pure fn (next to `NavigationController`) | engine-indep |
| nav-type (`NavigationType` enum) | `NavigationRequest.nav_type` field (contract, replaces `replace:bool`) + VM setters set it + drains map (Reload→`Keep`) + app-mode `navigate` honors it | engine-indep contract / host/ marshal / shell map |
| unload gating (same-doc ⇒ no unload) | `classify_navigation == CrossDocument` guards `dispatch_unload_events` (`event_loop.rs:282`) | engine-indep classifier at the shell caller |
| finalize-same-document (entry commit) | `NavigationController::push` (replace deferred §10-D6) | engine-indep (shell side-store) |
| event-firing DECISION (which fire, with what) | shell drain + `HistoryStepEvents` (`elidex-script-session::navigation`) | engine-indep |
| event RECONSTRUCT + FIRE (popstate build+dispatch; hashchange enqueue) | VM `vm/host/` | marshalling (host/) |
| scroll-to-fragment (indicated part → offset) | shell/layout | engine-indep |
| scroll transport | existing `take_pending_scroll`/`set_scroll_offset` | engine boundary (exists) |
| origin stability | `document_origin()` unchanged | by-construction (no code) |

**No new algorithm in `vm/host/`** (Layering mandate): natives stay marshal-only; classifier /
event-decision / scroll-resolution are engine-indep; only event reconstruction+fire is host/.

### §6.2 The same-document classifier (engine-indep, `elidex-navigation`)

A pure fn, home in `elidex-navigation` alongside the `NavigationController` (mirroring
`window_open_disposition`'s home next to its channels):

```rust
// elidex-navigation — the same-document determination (WHATWG HTML navigate §7.4.2.2 step 15)
pub enum NavClass { SameDocument, CrossDocument }               // (NEW)

/// `current` = active document URL; `target` = requested URL.
/// SameDocument IFF URLs equal excluding fragments AND target fragment is non-null (navigate step 15).
/// CrossDocument otherwise (rebuild — covers true cross-doc AND same-URL reload).
pub fn classify_navigation(current: &url::Url, target: &url::Url) -> NavClass {   // (NEW)
    if url_equals_excluding_fragments(current, target) && target.fragment().is_some() {  // (NEW helper)
        NavClass::SameDocument
    } else {
        NavClass::CrossDocument
    }
}
```

- `url_equals_excluding_fragments` (NEW) uses the `url` crate's serialization comparison with the fragment
  cleared (robust vs the crude `split('#')`). Push-vs-replace is **not** a classifier output; today the
  thread-mode drain drops `NavigationRequest.replace` so the fragment commit pushes (§6.3 step 3 / §5.2(b),
  the deferred §10-D6 concern), not a classifier matter (I7).
- The classifier is deliberately **URL-pure** (engine-indep, no request access): navigate step 15's other
  three conjuncts are gated OUTSIDE the classifier — `documentResource is null` via the `request.is_none()`
  call-site guard (§6.3), `response is null` **vacuously** (no `response` param on `handle_navigate`), and
  the fresh-vs-reload/traversal distinction via `cursor_op == Push` (§6.3). So the full 4-conjunct step-15
  gate holds by-construction across `classify_navigation`-result + the three branch guards, keeping the
  classifier engine-indep.
- **Replaces** the inline `is_fragment_only` bool at `content/navigation.rs:54-63` (thread-mode); app-mode
  has **no** fragment-detection today (`app/navigation.rs` always rebuilds), so the classifier is added
  **fresh** there. Single-homed in `elidex-navigation` (both shells call it — §5.6).
- **Truth-table unit tests** (§9) pin every §4.2 row incl. the corrected **removal ⇒ CrossDocument** and
  **emptied ⇒ SameDocument**.

### §6.3 The Fragment branch + the event-firing hub back-channel

The classification + Fragment branch live **INSIDE `handle_navigate`** (§5.2 — where `is_fragment_only`
:54-63 lives, where `request` is evaluable, and where the SW-skip gate :69 is preserved), NOT at the drain.
The branch is entered iff **all three** hold (navigate step 15, the full 4-conjunct gate factored across
the URL classifier + two call-site guards):

- `classify_navigation(cur, tgt) == SameDocument` — the URL conjuncts (equals-excl-frag + target-frag-non-null);
- `request.is_none()` — the `documentResource is null` conjunct. A POST form submit to a same-page
  `#fragment` carries a body ⇒ **CrossDocument** (the POST is sent), never a fragment skip. **Defensive
  today** (the only `Some(request)` caller — `content/form_input.rs` POST — already strips the fragment via
  `build_submission_url`'s `set_fragment(None)`, so a body-bearing nav reaches the classifier fragment-less
  ⇒ `CrossDocument` by conjunct-4 alone); the guard is the spec-faithful by-construction backstop, not
  relied on. (`response is null` — the fourth conjunct — holds **vacuously**: `handle_navigate` has no
  `response` parameter, so a pre-fetched-response navigation cannot reach this site; it is not checked by
  `request.is_none()`.);
- `cursor_op == HistoryCursorOp::Push` — a **fresh** navigation. This excludes **reload** (`Keep`, see the
  reload fix below), **chrome-button traversal** (`Keep`), and **JS traversal** (`Commit`, deferred to 5c —
  the traversal same-document path). Without this, a JS `location.reload()` on a fragment-URL page
  (`target == current` ⇒ `SameDocument`) would skip the rebuild (§5.2(a)).

**Nav-type fix (§5.2, plan-review R2 — bundled, spec-faithful; root fix for reload + the app-mode
collision).** Replace `NavigationRequest`'s `replace: bool` with **`nav_type: NavigationType { Push,
Replace, Reload }`** (NEW enum; single-homes the nav-type, mirroring `HistoryCursorOp`, dissolving the
non-orthogonal two-bool). Setters (VM `vm/host/location.rs`, light-touch; boa the same, deletion-bound):
`location.href=`/`assign`/`<a href>` → `Push`; `location.replace()` → `Replace`; `location.reload()` →
`Reload`.
- **Thread-mode drain** maps `nav_type → cursor_op`: `Reload → Keep`, `Push`/`Replace → Push` (thread-mode
  still collapses Replace→Push for the cursor op — the deferred §10-D6; the enum only CONVEYS the
  distinction). The Fragment branch's `cursor_op == Push` guard thus excludes `Reload` (`Keep`) +
  traversal (`Commit`).
- **App-mode** changes `navigate(url, replace: bool)` → `navigate(url, nav_type: NavigationType)` (app-mode
  **honors** the type, so the enum distinguishes `Reload` from `Replace` — the collision the two-bool could
  not: `location.reload()` and `location.replace('#x')` were both `replace:true`). App-mode's Fragment
  branch gates on `nav_type != Reload`; a `Reload` rebuilds with no cursor move (Keep-equivalent). **ALL
  `navigate` call sites must supply the correct type — the sibling-site sweep** (grep-verified
  `crates/shell/elidex-shell/src/app/`): the drain `navigation.rs:69` (`nav_req.nav_type`, pass-through),
  the link-click `events.rs:105` (→ `Push`), the chrome address-bar `navigation.rs:272` (→ `Push`), and —
  **the round-3 IMP** — the chrome **reload** `navigation.rs:295` (currently `navigate(&url, true)` where
  `true` meant reload, the exact two-bool collision) → **`NavigationType::Reload`, NOT `Replace`** (else
  app-mode chrome-reload of a `/a#x` URL would take the fragment branch + skip rebuild, the same regression
  the enum fixes for JS reload). App-mode's **traversal** path is a **separate** fn `navigate_to_history_url`
  (`inline.rs:248`, `navigation.rs:217/234/286`) — it does NOT get a fragment branch (that is 5c
  traversal-same-document; thread-mode's analogue is `Commit`, Push-excluded).

This (i) makes `location.reload()` rebuild (`Reload → Keep → rebuild`), (ii) fixes the pre-existing
**reload-pushes-a-history-entry** bug (`Keep` = no cursor move, correct for reload), (iii) keeps
`location.href = currentURL` (href-identical, `Push`) firing popstate per §4 — it is `Push` not `Reload`,
distinguished by ENTRY POINT (HTML §7.4.3 reload is a distinct algorithm, `isSameDocument=false`), avoiding
the URL-guard deviation, and (iv) resolves the **app-mode reload/replace collision** (nav_type distinguishes
them where `replace:bool` could not).

**Caller-audit — unload gating (§5.6, plan-review R2 IMP).** `handle_navigate` is called from several sites;
the address-bar path `BrowserToContent::Navigate` (`content/event_loop.rs:291`, `Push`) dispatches
`dispatch_unload_events` at `:282-290` **unconditionally, BEFORE** `handle_navigate`. A same-page
`#fragment` address-bar nav (`page → page#frag`) is `SameDocument`, so unload/beforeunload would fire before
the Fragment branch — **spec-wrong** (*navigate to a fragment* has `isSameDocument=true`, fires no unload;
browser-UI navs DO take the fragment path — webref navigate note) and it would make the fragment nav
beforeunload-cancelable. **Fix**: gate the `:282` `dispatch_unload_events` on `classify_navigation(current,
target) == CrossDocument` (call the pure engine-indep classifier at the caller, before unload; a
`SameDocument` address-bar nav skips unload and lets the Fragment branch handle it). Other callers: drain
`:317` (`Push`, no pre-unload) ✓; link-click `content/event_handlers.rs:238` (`<a href="#x">`, `Push`) ✓;
GET/POST form `content/form_input.rs` (POST strips fragment + body; GET changes the query ⇒ CrossDocument) ✓;
chrome `GoBack`/`GoForward`/`Reload` (`Keep`) ✓; iframe `content/iframe/thread.rs:193` is a **distinct 3-arg**
`handle_navigate` (out of scope, §1.3/§10-D7).

On the Fragment branch:

1. **No `load_document`, no rebuild** (the document + its `EcsDom` incl. `ElementState::FOCUS` persist —
   fixing the wrong focus-reset, I3).
2. `set_current_url(Some(target))` (so `location.*`/`document.URL` read the new URL; origin stays correct
   by-construction, I2/§6.5).
3. **Commit the entry** via `NavigationController::push` (= *finalize a same-document navigation*), feeding
   `set_history_length`. **Push-vs-replace note (§5.2(b))**: the thread-mode drain currently drops
   `NavigationRequest.replace` (hardcodes `Push`), so `location.replace('#x')` fragment navs commit as a
   push — **inheriting** the pre-existing all-navs-push behavior, no new bug. Honoring
   `NavigationRequest.replace` for navigations is a separable pre-existing gap → **deferred (§10-D6)**;
   when it lands, the same `replace` reaches this commit and selects `NavigationController::replace`.
4. **Scroll to the fragment** via the existing viewport transport (§6.4).
5. **Fire** via the new back-channel with `popstate_state = Some(None)` + `hashchange = Some((old, new))`
   iff `oldFrag ≠ newFrag`.
6. **Early return** (mirrors `handle_navigate`'s `return true`).

The Fragment branch applies to **both** shells' FRESH-nav path only — thread-mode `handle_navigate`,
app-mode `navigate` (§5.6 — sibling sweep; app-mode is GET-only so the `request` guard is vacuous there).
App-mode's traversal `navigate_to_history_url` does **NOT** get a Fragment branch (that is 5c
traversal-same-document; thread-mode's analogue is `Commit`, Push-excluded). The VM native path is otherwise
untouched (`location.href=`/`assign`/`<a href="#x">` already enqueue the `NavigationRequest`; the
`NavigationType` is set at the setter — `Push`/`Replace`/`Reload`).

The **event hub** is a new cohesive method-group on `HostDriver` (Accretion, §5.4), mirroring the
state-push + deliver-turn media shape:

```rust
// elidex-script-session::engine (HostDriver) — the history-step event delivery group
/// WHATWG HTML §7.4.6.2 "update document for history step application": the shell computes which fire from
/// its session-history entry model; the engine reconstructs history.state and fires at the Window.
fn deliver_history_step_events(&mut self, ev: HistoryStepEvents);              // (NEW)

// elidex-script-session::navigation (engine-independent)
pub struct HistoryStepEvents {                                                 // (NEW)
    /// `Some(None)` = fire popstate with state=null (fragment nav, 5b); `Some(Some(bytes))` =
    /// StructuredDeserialize(restored) (5c traversal); `None` = do not fire popstate.
    pub popstate_state: Option<Option<SerializedState>>,
    /// `Some` iff the fragment differs (step 6.4.5).
    pub hashchange: Option<(String, String)>,
}
```

- **5b uses** `Some(None)` + `hashchange` iff frag differs. 5c reuses the **same method** with
  `Some(Some(restored))` — One-issue-one-way, one method two consumers.
- **VM impl** (`vm/host/`, marshal-only): build a `PopStateEvent` (state=null for 5b) and **fire
  synchronously** at the Window (direct build+dispatch — `EventPayload` has no PopState variant, §5.3); if
  `hashchange` present, **enqueue** a hashchange task (via `EventPayload::HashChange` window-dispatch).
  **popstate SYNC, hashchange ENQUEUED** (I4/§4.4) — popstate strictly-before-hashchange. **Fire-path
  cohesion**: if the popstate build+dispatch + hashchange enqueue forms a cluster in `events_extras.rs`
  (716 lines), split a `vm/host/history_events.rs` sibling (assess at impl; §10 line-count).
- **boa impl** (light-touch): **no-op stub** (deletion-bound; never fired these ⇒ not a regression) ⇒ the
  firing is **flip-inert** (VM-tested now, live at S5-6); the shell same-document path is
  **engine-agnostic-now** (I5).

Why NOT route popstate through `EventPayload` like hashchange: popstate's `state: any` is a live `JsValue`
the engine-indep `EventPayload` cannot carry (§5.3) ⇒ popstate is intrinsically a VM-reconstruction, which
is exactly why the *decision* stays engine-indep while the *reconstruct+fire* is host/.

### §6.4 Scroll-to-the-fragment (via the post-layout scroll seam — I6)

Scroll routes through the **existing** viewport transport (`take_pending_scroll`/`set_scroll_offset`, §5.4),
never a new channel:

- Compute the **indicated part** (id → name (`<a name>`) → "top of document" for empty `#` — §7.4.6.4) in
  the **shell/layout** layer (engine-indep — DOM + layout own geometry) and set the viewport offset via the
  transport.
- **Scroll-application currency (load-bearing, I6)**: the element-resolution + offset must be resolved+
  applied through `re_render`'s **post-layout** scroll application (`crates/shell/elidex-shell/src/content/
  mod.rs` pending-scroll drain → clamp-against-content-size + echo to `scrollX`/`scrollY` + document-root
  `ScrollState`), **not** set inline in the post-render `process_pending_actions` drain. The drain is
  itself POST-render (its click/key call sites `re_render` immediately before), so layout is current — the
  hazard is NOT stale layout; it is that a scroll set inline in the drain + shipped via
  `send_display_list()` ships a display list with the offset **un-applied** (the clamp/echo/`ScrollState`
  machinery lives only in `re_render`). Anchor on that same post-layout scroll seam (the Codex R6/F4 "apply
  script scrolls after layout is refreshed" precedent). Scroll-to-fragment is strictly harder than
  `scrollTo` — it RESOLVES an element — so it inherits the seam's post-layout offset application and routes
  element-resolution through it too.
- Empty fragment (`#`) → top-of-document. Focus-move deferred (D2). §7.4.2.3.3 step 15's async-scroll
  fallback (id not yet parsed on a *new* document) is a cross-doc concern; for 5b the document is EXISTING
  (id parsed), scroll succeeds synchronously.

### §6.5 Origin stable-by-construction (closes `#11-vm-navigation-origin-resync`; I2)

Not an active mechanism — closes by construction; 5b proves + tests the invariant:

- Per §7.2.5 *can have its URL rewritten* is a **URL-component gate** (scheme/username/password/host/port),
  **not** an origin gate (spec note verbatim: "only the URL of the Document matters, and not its origin.
  They can mismatch in cases like about:blank … sandboxed iframes, or when the document.domain setter has
  been used").
- `document_origin()` (§5.5) resolves to (a) any installed **override** (opaque/sandboxed/inherited) —
  which `set_current_url` never touches — so the sandboxed-opaque / about:blank-inherited / `document.domain`
  cases key the **preserved** origin; or (b) for a no-override top-level doc,
  `SecurityOrigin::from_url(current_url)` derives the **same URL-tuple origin** (a fragment nav changes only
  the fragment).
- ⇒ after a same-document nav updates `current_url`, `document_origin()` is **unchanged** ⇒
  fetch/WS/ES/postMessage stay correctly keyed. **No `set_origin` re-push.** The active resync the
  `set_current_url` doc anticipates is only for a cross-document nav that reuses the VM — which 5b does NOT
  introduce (cross-doc rebuilds → fresh VM → fresh origin; that is S5-8/B1).
- **Closed by** (i) the documented invariant + (ii) a regression test: `fetch()` / `new WebSocket()` after
  a fragment nav in **both** a top-level doc AND a sandboxed-opaque iframe key on the correct (unchanged)
  origin (§9).

### §6.6 ECS-native lens + focus

- **Session history = a browsing-context/navigable fact**, held in the shell-owned `NavigationController`
  — a legitimate shell side-store (CLAUDE.md ECS-native exception (b): browsing-context/session resource,
  not a single-entity fact), NOT an ECS component. Correct home; no migration.
- **Focus**: same-document nav does NOT reset focus (`ElementState::FOCUS` persists) — 5b's no-rebuild
  **fixes** the wrong focus-reset (§5.1, I3). The only in-scope focus interaction is the scroll-to-fragment
  focusing step, deferred (D2) — routed through canonical `ElementState::FOCUS`, never an ad-hoc reset. 5b
  adds **zero** ad-hoc focus state.
- **Storage-home neutrality**: no new per-VM per-entity state (5b's popstate state is null; delivery is a
  transient turn). B1-migration-neutral.

---

## §7 Design decisions (the plan-review ratify-points; cluster §9 Q4/Q5/Q6 + the §4 correction)

| Decision | Resolution proposed | Basis |
|---|---|---|
| **Q-CLASSIFIER** (§4) | predicate = **equals-excluding-fragments AND target-fragment-non-null** (navigate step 15); **removal ⇒ CrossDocument**; PRESERVE the existing `fragment().is_some()` clause; correct cluster §4.2/§5.2.2/§7 | §4 spec trace + empirical url-check |
| **Q4** engine-agnostic-now vs flip-inert | shell same-document **path** engine-agnostic-now (observable in live boa shell); event **firing** flip-inert (VM-fired, boa no-op stub, live at S5-6). Non-regression | §6.3 |
| **Q5** origin-resync | close by-construction (invariant + test); no defensive `set_origin` (dead until cross-doc same-VM nav = S5-8/B1) | §6.5 |
| **Q6** fragment-nav popstate matrix | fresh fragment nav fires popstate(null) + hashchange; pushState/replaceState fire neither | §4.4 |
| **Q-NAVTYPE** (plan-review R2) | `NavigationType {Push,Replace,Reload}` enum (replaces `replace:bool`, single-homes nav-type) BUNDLED in 5b — reload fix (Reload→`Keep`, spec-faithful: href-identical still fires popstate) + app-mode collision fix. **replace**-honoring DEFERRED (§10-D6). Bundle-not-prereq per cluster §0.1 (nav-type modeling is inert without the fragment consumer) | §5.2 / §6.3 lens-converge |
| **Q-UNLOAD** (plan-review R2 IMP) | gate `dispatch_unload_events` (`event_loop.rs:282`) on `classify_navigation == CrossDocument` — a same-page `#fragment` address-bar nav fires NO unload (spec: fragment nav `isSameDocument=true`); pins the address-bar Push caller | §6.3 caller-audit |

`pushState-on-initial-about:blank → replace` (§7.4.4 step 4) is a **5c** concern, noted for 5c kickoff.

---

## §8 Edge matrix (5b owned edges — cluster §6)

| # | Edge | 5b discharge |
|---|---|---|
| **E2** | same-document vs cross-document classification | **owns**: §6.2 classifier, corrected predicate (§4), truth-table tests (§9) |
| **E3** | origin stable across no-rebuild nav | **owns**: §6.5 by-construction + top-level & sandboxed-opaque fetch/WS test |
| **E5** | focus persists on same-document nav | **owns**: §6.6 no-rebuild ⇒ `ElementState::FOCUS` persists |
| **E6** | engine-agnostic-now vs flip-inert firing | **owns**: §6.3 boa stub / VM fire; §9 split per assertion |
| **E9** | fragment-nav popstate is counterintuitive | **guard**: §4.4/§9 assert both fire; wiring only-one is spec-wrong |
| E1 | drain order (5a owns) | reads: §5.2 fragment branch at the nav drain, post-history |
| E7 | traversal+nav same-turn (D5) | narrows: fragment nav removes the rebuild for fragment cases |
| E10 | two nav impls | applies the branch to both shells (§5.6) |

Densest slice (E2+E3+E5+E6+E9) — terminal under this memo's plan-review (base-case rule; S5-4c precedent).

---

## §9 Test strategy (supported-surface; engine-agnostic-now vs flip-inert split)

Boa stays the live shell engine; oracles = engine-level VM tests + targeted shell integration.

- **`elidex-navigation` unit — the classifier truth table** (the §4 correction's regression gate): every
  §4.2 row — add/change/emptied/identical-via-href ⇒ SameDocument; **removal (`/a#x → /a`) ⇒ CrossDocument**;
  identical-no-fragment ⇒ CrossDocument; path/query differ ⇒ CrossDocument. Pins `url::fragment()` `Some("")`
  (emptied) vs `None` (removal).
- **Engine-agnostic-now** (passes in the live boa shell): fragment nav does **NOT** re-fetch (network-request
  oracle: **zero** requests); `NavigationController` gains one entry (push — replace deferred §10-D6); scroll
  lands on the `#id` element AND **scroll-application** (`location.href='#x'` for off-screen `#x` → resolved
  offset **reaches the display list / clamped + echoed to `scrollX`/`scrollY`**, not shipped un-applied);
  **focus persists** across fragment nav (was: reset); origin unchanged after fragment nav in **both** a
  top-level doc AND a sandboxed-opaque iframe (fetch/WS key correctly, §6.5); cross-document nav (incl.
  **fragment removal**) still rebuilds (regression pin); **`location.reload()` on a fragment-URL page**
  (`/a#x`) **rebuilds** (network-request oracle: re-fetch happens; `history.length` does NOT grow — reload→
  `Keep`, §6.3/§5.2(a)), NOT a no-rebuild skip (the 5b-introduced-regression pin); **an address-bar nav to a
  same-page `#fragment`** (`BrowserToContent::Navigate`, `page → page#frag`) fires **NO** `unload`/`beforeunload`
  AND does not rebuild (the unload-gating IMP, §6.3 caller-audit) — while a **cross-document** address-bar
  nav still fires unload + rebuilds (regression pin); **app-mode reload of a fragment-URL rebuilds without
  history growth via BOTH entry points** — JS `location.reload()` (`process_pending_navigation`) AND the
  **chrome `ChromeAction::Reload`** (`handle_chrome_action`, `navigation.rs:295` — the round-3-IMP pin: it
  must map to `NavigationType::Reload`, not `Replace`) — AND app-mode `location.replace('#x')` is
  distinguished from reload (`NavigationType`, §6.3 — the collision pin); **a body-bearing navigation to a same-page `#fragment`**
  (`handle_navigate` with `request = Some(...)`) is **CrossDocument** (full nav / body sent), NOT a fragment
  skip (the `request.is_none()` defensive guard, §4.3 point 5 / §6.3).
- **Flip-inert** (VM-tested now, live at S5-6): popstate/hashchange **firing** — VM integration (`cargo test
  -p elidex-js --all-features`) drives `deliver_history_step_events` and asserts popstate fires
  **synchronously** with **state=null** + hashchange **enqueued** with correct old/new URLs
  (popstate-strictly-before-hashchange); identical-via-href fires popstate but **NOT** hashchange
  (`oldFrag==newFrag`). A shell test pins boa's **no-fire** (stub) as the pre-flip baseline. **Registered
  S5-6 flip deliverable**: the live-shell popstate/hashchange test once the VM is the engine (mirrors
  S5-4b's storage-sentinel deferral). **Fold into that deliverable the scroll-vs-popstate-handler
  ordering** (flip-inert today — boa stubs the events, so no popstate handler runs): the *navigate to a
  fragment* step-15 fragment scroll must WIN over a popstate handler's own `scrollTo` (the handler's
  queued scroll is applied by `re_render`'s `take_pending_scroll` after the directly-set fragment offset,
  so today it would override — spec-wrong once popstate fires live), and the fragment offset is resolved
  against pre-popstate layout (§6.4 — a handler mutating layout above the target would stale it). Both are
  only observable + testable when popstate fires live (S5-6); revisit the `re_render` scroll-application
  order there.
- **WPT subset**: `html/browsers/history/the-location-*` (fragment) + the popstate/hashchange subset —
  engine-independent equivalents (harness scope judged at impl; the unit/integration above is the
  regression gate per "Supported-surface testing").
- Workflow: plan-verify grep vs HEAD → impl in the `s5-5b-fragment-navigation` worktree → `/pre-push` →
  `/external-converge` → squash merge (umbrella §11).

---

## §10 Deferred carves + audits (cap ≤3; actual 5b = 3 carves [D2/D6/D7] + 1 audit [A1] — at cap)

- **D2 `#11-fragment-navigation-focusing-step`** (carve; cluster §8-D2): §7.4.6.4 scroll-to-the-fragment
  step 3.6 "run the focusing steps" + 3.7 — 5b lands the *scroll*, not the focus-move. **Audit**: spec-core
  yes (§7.4.6.4); one-way yes (focus-move routes through canonical `ElementState::FOCUS` at the same site);
  pragmatic-debt: interim scrolls-without-focusing (safe, common-case-correct); repeat-signal: the S2 focus
  program's surface. **Trigger**: the focusing-steps surface (S2). **Re-eval**: S2 focus program; backstop
  **2026-10-31**.
- **Audit A1 (no slot)** — `location.hash=` setter: the VM registers only a whole-`href` RW accessor (`hash`
  is RO); a dedicated `hash` setter + §dom-location-hash step-8 bailout is a separate Location-surface
  concern. 5b drives fragment nav via `location.href=`/`<a>`/`assign` (existing `NavigationRequest`).
  **Disposition**: verify at impl whether `location.hash=` is a cheap in-scope-adjacent add or a separate
  follow-on; if separate, note for the Location-surface backlog (no `#11-` slot minted now — verify-at-impl).
- **D6 `#11-thread-mode-drain-replace-honoring`** (carve; NEW, plan-review R2): 5b's `NavigationType` enum
  CONVEYS `Replace`, but the thread-mode drain still maps `Replace → HistoryCursorOp::Push` (collapsing the
  replace-vs-push cursor distinction), so `location.replace()` navigations add a history entry instead of
  replacing — a **pre-existing** gap (NOT 5b-introduced; 5b's fragment nav inherits `Push`). Honoring it is
  the natural sibling to the reload fix on the same `nav_type → cursor_op` seam (map `Replace` → a replace
  cursor op → the §6.3 Fragment branch's step-3 `NavigationController::replace` lights up; app-mode already
  honors it). **Audit**: spec-core? yes (historyHandling push/replace, HTML §7.4.4/§7.4.2.2); one-way? yes
  (extend the same drain `nav_type → cursor_op` map the `Reload → Keep` case uses); pragmatic-debt? interim
  = thread-mode `location.replace()` pushes (a `history.length` off-by-one, rare/minor; app-mode is already
  correct); repeat-signal? yes — the nav-type conveyance (reload fixed, replace the sibling). **Trigger**: a
  site/WPT exercising thread-mode `location.replace()` history semantics, or the next thread-mode nav-drain
  touch. **Re-eval**: backstop **2026-10-31**. (One-issue-one-way: deliberately NOT bundled — a separable
  cross-cutting concern touching all navigations, not just fragment.)
- **D7 `#11-iframe-fragment-navigation`** (carve; NEW, plan-review R2): iframe fragment navigations still
  full-rebuild. The iframe nav path is a **distinct 3-arg** `handle_navigate(pipeline, url, channel)`
  (`content/iframe/thread.rs:193`), separate from the top-level/app-mode `handle_navigate` 5b touches; 5b's
  `#11-synchronous-fragment-navigation` closure covers **top-level + app-mode only**. An iframe same-page
  `#fragment` nav is a same-document nav per spec but keeps reloading. **Audit**: spec-core? yes (§7.4.2.3.3
  applies per-navigable, iframes included); one-way? yes (the same-document primitive — classifier + branch +
  back-channel — is engine-indep, so the iframe path consumes it once wired); pragmatic-debt? interim = iframe
  `#frag` navs rebuild (loses same-document semantics + focus/scroll, but safe); repeat-signal? the OOP-iframe
  nav surface (S5-4b iframe origin, S5-8 browsing-context). **Trigger**: iframe same-document nav fidelity
  work / the OOP-iframe surface (S5-8). **Re-eval**: backstop **2026-10-31**.
- **Cluster carves referenced (5a/5c's, not 5b's)**: D1 (StructuredSerialize) / D3 (scrollRestoration
  manual) / D5 (task-queue model) — cluster §8.

**Touch-set line counts** (post-5a): `content/navigation.rs` 563, `app/navigation.rs` 371,
`elidex-navigation/navigation.rs` 476, `vm/host/navigation.rs` 365, `script-session/engine.rs` 466,
`script-session/navigation.rs` 390, `events_extras.rs` 716, VM `engine.rs` 596. **All under 1000 — no
touch-time split obligation.** Monitor: `events_extras.rs` (+ fire path) → split `vm/host/history_events.rs`
if a cluster forms (§6.3); `content/navigation.rs` (563 + the bounded Fragment branch insert).

---

## §11 Open questions for `/elidex-plan-review`

- **Q-CLASSIFIER (§4 — headline)**: ratify the spec correction — the URL predicate is
  `equals-excluding-fragments AND target-fragment-non-null` (navigate step 15 conjuncts 3-4), NOT the
  cluster memo's `fragments-differ`; **removal ⇒ CrossDocument/reload** (real-browser); the existing
  `fragment().is_some()` clause is spec-correct and is **preserved** (5b fixes wiring, not the predicate).
  The full step-15 gate is factored (§6.3): 2 URL conjuncts (classifier) + `documentResource is null`
  (`request.is_none()` defensive guard) + `response is null` (vacuous — no `response` param) + fresh-nav
  (`cursor_op == Push`, excluding reload/traversal). Ratify the **cluster-memo edit** (§4.5) as part of the
  5b landing.
- **Q-NAVTYPE (plan-review R2)**: ratify the **`NavigationType {Push,Replace,Reload}` enum** (replaces
  `NavigationRequest.replace`, single-homes nav-type) BUNDLED in 5b — reload fix (`Reload → Keep`,
  spec-faithful: href-identical still fires popstate, avoiding the URL-guard deviation) + the app-mode
  reload/replace collision fix — and **bundled not prereq** per cluster §0.1 (the nav-type modeling is inert
  without the fragment consumer, so a prereq would be a no-consumer strangler). Replace-**honoring** is
  DEFERRED (§10-D6, thread-mode only; app-mode already honors it). Accept, or fold replace-honoring in now
  (rejected: cross-cutting, not fragment-specific)?
- **Q-UNLOAD (plan-review R2 IMP)**: ratify gating `dispatch_unload_events` (`event_loop.rs:282`) on
  `classify_navigation == CrossDocument` so a same-page `#fragment` address-bar nav fires no unload
  (spec-faithful: fragment nav `isSameDocument=true`). Confirm this address-bar caller fix is in-scope for
  5b (it exposes the unconditional pre-unload).
- **Q4**: ratify engine-agnostic-now (shell path) vs flip-inert (event firing); accept that fragment nav in
  the live boa shell does not fire the events until S5-6 (non-regression). Or does plan-review want boa to
  fire hashchange via `EventPayload` pre-flip (rejected as boa feature-work under light-touch)?
- **Q5**: close `#11-vm-navigation-origin-resync` by-construction (invariant + test), or add the defensive
  `set_origin`-alongside-`set_current_url` now? Lean by-construction.
- **Q6**: confirm the fragment-nav popstate matrix (§4.4).
- **Q-SCROLL**: ratify routing scroll-to-fragment through the `re_render` post-layout scroll seam (not
  inline in the drain), element-resolution riding the same seam (§6.4).
- **Q-SPLIT**: confirm 5b stays a single terminal PR under this memo's plan-review (base-case rule), with
  the fire-path `vm/host/history_events.rs` split assessed at impl (§10).

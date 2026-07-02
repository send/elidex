# S5-3c — Observer (Mutation / Resize / Intersection) keepalive arm (the active-observation predicate)

Per-PR plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`)
and the S5-3 program memo (`docs/plans/2026-06-s5-3-eventtarget-listener-keepalive-rooting.md`,
§7 split decision: **S5-3c = observers**; §5.2 = the observer over-rooting reconciliation this memo
settles). **Anchor = the ideal end-state**, not an incremental patch
(`feedback_plan-memo-anchor-on-ideal-not-incremental`).

S5-3a (#430 `3345949e`) landed the keepalive-**predicate seam**
(`crates/script/elidex-js/src/vm/gc/keepalive.rs`) + the `MediaQueryList` arm + the
`AbortSignal.timeout` membership root. S5-3b (#440 `113a6c75`) added the **`WebSocket` + `EventSource`**
state-tiered arms (tier rules delegated to engine-indep `elidex-api-ws`). S5-3c is the **third and
LAST arm**: route the three observers (`MutationObserver` / `ResizeObserver` / `IntersectionObserver`)
through the seam with an **active-observation predicate**, replacing today's **construct-time for-life
root** (a leak / over-root). This is the arm that **closes the hard pre-flip gate**
`#11-eventtarget-keepalive-registrant-coverage` — after S5-3c, every non-Node keepalive registrant is on
the seam and the S5-6 flip is unblocked on this axis.

> **⚠ DESIGN inheritance (read with the parent):** the parent S5-3 memo's `world_id` framing is
> **SUPERSEDED by the agent-scoped `EcsDom` World program** (PR #434 `deb6eaf6`,
> `docs/plans/2026-06-agent-scoped-ecsdom-world.md`). Throughout this memo the keepalive
> component-on-entity migration (`#11-eventtarget-keepalive-component-migration`) is **B1-gated**
> (1-agent = 1-World makes per-entity identity stable without a discriminator), **not** world_id-gated.
> Do not reintroduce world_id framing. (Note the S5-3a/b `keepalive.rs` module doc **already carries** the
> B1 supersession block at `keepalive.rs:32-36` — "⚠ SUPERSEDED 2026-06-30: world_id retracted →
> agent-scoped `EcsDom` World (PR #434) …"; the only pre-#434 residual is the single "world_id-gated"
> adjective on `keepalive.rs:31`, which S5-3c tightens as a one-word stale-comment deliverable, §12.)

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE impl, per CLAUDE.md "Edge-dense work = multi-PR
> program + 実装前 plan-review 必須" and the umbrella S5-3c row (`plan-review? = yes`). This memo maps
> the edge matrix (§8) + coupled invariants (§2.4) so plan-review can pre-empt the review tail, settles
> the D1-vs-D2 `is_observing` design (§5.2 — **the** central ECS-native decision), and confirms the
> step-1-snapshot **decoupling** (§2.5).

All file:line cites grep-verified against `main` HEAD `113a6c75` (2026-07-02). Every spec § prose /
anchor webref-verified 2026-07-02 (sources: `dom`, `resize-observer-1`, `intersection-observer`,
`performance-timeline`).

---

## §0 Read-first (scope + the central reframe, inherited + the direction FLIP)

### §0.1 What S5-3c is
A **FLIP-precondition** (umbrella §5 type-(a): land BEFORE the S5-6 boa→VM flip), **VM-internal**, boa
stays live, **no external dependency**. The deliverable extends the existing keepalive seam with a
**direct membership mark** for the three observers (Mutation / Resize / Intersection) — marked directly
in `keepalive_survivors` like the `AbortSignal.timeout` membership root, **not** a `KeepaliveClass`
listener-predicate variant (shape B, settled §4.2 / Q2) — whose spec-faithful predicate is
**"the observer has ≥1 active observation"** (DOM §4.3 registered-observer-list membership;
the RO/IO `[[observationTargets]]` internal-slot analogue) — **OR-ed, MutationObserver only, with
"has ≥1 pending undelivered record"** (the §2.1 ⚠ CORRECTION; RO/IO have no such clause).
A never-observed or disconnected observer
with no retained JS reference becomes **collectible** — fixing the latent leak that today a
`new MutationObserver(cb)` (or `.disconnect()`ed observer) is **immortal until `Vm::unbind`** (§1).

It is **inert today** (boa is the live engine; the VM observer delivery path is dormant), but it
**gates** the flip via the hard gate `#11-eventtarget-keepalive-registrant-coverage`: leaving the
observer root off the seam at the flip = exactly the forbidden indefinite strangler (S5-3 §0.3). The
over-root is a **pre-existing** behavior — S5-3c **migrates** it onto the seam; it does not introduce
divergence.

### §0.2 The central reframe (inherited, non-negotiable) — membership, NOT any-listener
The seam is a **per-registrant keepalive PREDICATE**, never an "any-listener roots the target" rule
(DOM §2.8 "Observing event listeners", `dom#observing-event-listeners`: listener presence must not be
observable; there is no general listener-keepalive rule — S5-3 §2). For the observers the keepalive is
**not a listener test at all** — an observer's callback fires from **observation** deliveries, not from
`addEventListener` on the observer object. So the observer predicate is **registry-membership**: kept
alive iff it **currently observes ≥1 target** — OR, MutationObserver only, still has a queued
undelivered record (the §2.1 ⚠ CORRECTION; also membership-shaped — a registry fact, never a listener
test). §2–§5 must not regress to "any listener" or to "construct-time root".

### §0.3 The DIRECTION FLIP vs S5-3a/b (state this so the test oracle is right)
S5-3a/b fixed **UNDER-rooting**: a listener-held MQL / WS / ES was *wrongly collected* (its only anchor
was a listener whose callback was rooted but whose target was not). The headline test there asserts the
target **survives**. **S5-3c is the OPPOSITE**: observers are **OVER-rooted** today — the
`(callback, instance)` binding is rooted **at construction for life** (`host_data.rs:1692-1733`
`gc_root_object_ids`), and `.disconnect()` **never releases it** (`mutation_observer.rs:249-261` clears
the *observation* but not the *binding*) → **immortal-until-unbind leak**. S5-3c is an **over-root / LEAK
fix**: construct-time root → active-observations predicate. **This flips the test oracle** — the
**headline** S5-3c test asserts a never-observed / disconnected unreferenced observer **IS collected**
(§9). Getting this framing wrong (copying the S5-3a "assert survives" oracle) would test the exact
opposite of the fix.

### §0.4 Strangler-safety (inherited) + this is the closer
S5-3c is **bounded in-program staging under the hard pre-flip gate**
`#11-eventtarget-keepalive-registrant-coverage` (S5-3 §10): **all of S5-3a/b/c MUST land before S5-6**.
S5-3c is the **last** arm — landing it **retires** the coverage gate (all registrants on the seam) so
the seam+legacy coexistence is fully force-resolved at the flip, the sanctioned staged delivery, NOT
the forbidden indefinite strangler. S5-3c is flip-MANDATORY but flip-*order*-independent relative to
S5-3b (both fix pre-existing latent bugs; neither blocks the other, but the flip blocks on both).

---

## §1 The bug — precise GC mechanics (cited, `113a6c75`)

The three observers are non-Node `EventTarget`s bound to a per-VM `(callback, instance)` `ObjectId`
pair. Unlike the MQL/WS/ES under-root, the observer pair is rooted **too much**:

1. **Construct-time for-life root.** `new MutationObserver(cb)` inserts an `ObserverBinding { callback,
   instance }` into `HostData::mutation_observer_bindings` at construction
   (`mutation_observer.rs:197-203`; RO/IO analogues in `resize_observer.rs` / `intersection_observer.rs`).
   The binding is rooted **unconditionally** by `HostData::gc_root_object_ids`
   (`host_data.rs:1712-1723`), which flat-maps the three `*_observer_bindings` values into
   `[b.callback, b.instance]` and chains them into the root iterator seeded at the mark pass. So **both
   ObjectIds survive every GC for the observer's whole lifetime, whether or not it observes anything.**

2. **`disconnect()` does NOT release the binding.** `native_mutation_observer_disconnect`
   (`mutation_observer.rs:249-261`) calls `observers.disconnect(dom, id)` — which clears the
   *observation* (`mutation/mod.rs:339-344` `retain_observations(dom, |o| o.observer != id)` +
   `records.clear()`) — but **never touches `mutation_observer_bindings`**. So a
   constructed-then-disconnected (or never-`observe()`d) observer with no retained JS reference stays
   **immortal until `Vm::unbind`**. The field doc-comment states this exact leak:
   `host_data.rs:274-280` ("`disconnect()` per spec only clears observation targets, not the binding …
   Long-lived VMs that churn many observers accumulate dead entries; weak-rooting / sweep-time cleanup
   is tracked at `#11-mutation-observer-extras`"); and `Vm::unbind` **intentionally retains** the map
   (`vm_api.rs:685-696`, keyed by VM-monotonic `observer_id`).

3. **Why it is spec-WRONG (over-root).** DOM §4.3 keeps a `MutationObserver` reachable through its
   nodes' **registered observer lists** (`dom#registered-observer-list`); membership = an **active
   observation**. RO/IO keep the observer reachable through the Document's observer processing while it
   has entries in `[[observationTargets]]` (`resize-observer-1#dom-resizeobserver-observationtargets-slot`
   / `intersection-observer#dom-intersectionobserver-observationtargets-slot`). **None of the three keeps
   the observer alive with an EMPTY target set.** A no-observation observer with no JS reference **must
   be collectible** (its callback can never fire — it observes nothing). The construct-time for-life root
   makes the *presence of a constructed-but-idle observer* observable through GC behavior — the §2.8
   over-rooting direction, and a real unbounded leak on observer-churning pages.

**The ideal end-state**: the observer joins the keepalive seam with a predicate **"has ≥1 active
observation"**, so the `(callback, instance)` pair is rooted **exactly while** the observer observes,
and released (collectible) the moment its last observation ends (`disconnect()` / `unobserve()` of the
last target / despawn of the sole observed entity). The binding-map **row** is then pruned in the sweep
by the instance's mark bit (§4.3) so the struct itself does not leak either.

---

## §2 Why spec-faithful = an active-observation membership predicate (webref-verified)

### §2.1 MutationObserver — DOM §4.3 registered-observer-list (`dom`, webref 2026-07-02)
DOM §4.3 "Mutation observers" (`#mutation-observers`) defines the **registered observer** / **registered
observer list** (`#registered-observer` / `#registered-observer-list`) as a list **on each node** — the
observer is reachable *from* the nodes it observes. There is **NO MutationObserver "garbage collection"
dfn** in DOM (`dfn 'garbage collection' dom` → the only GC dfn is §3.2.1 AbortSignal, not observers).
So keepalive = **membership in ≥1 node's registered observer list** = the observer has ≥1 active
registration. §4.3.1 `observe()` steps 7–8 (webref prose): **step 7** is the **already-observed** branch
(*"For each registered of target's registered observer list: if registered's observer is this: [7.1]
remove all transient registered observers … [7.2] set registered's options to options"*) — i.e. it
**replaces** the existing registration **in place**; **step 8** is the *"Otherwise: Append a new
registered observer …"* branch, which fires **only when not already observed**. So a re-`observe()` of an
already-observed target does **not** add a second registration. This matters for the count design (§5.2
D1): the same (observer, target) pair is a single membership fact, not two.

**Spec-basis asymmetry (record for plan-review).** MutationObserver keepalive is **INFERRED** from DOM
§4.3 registered-observer-list membership (there is no explicit MutationObserver-lifetime prose — the only
GC dfn in DOM is §3.2.1 AbortSignal). By contrast RO/IO carry **EXPLICIT** two-condition lifetime prose
(RO §3.5 / IO §3.3 "Lifetime", §2.2/§2.3). The predicate is **identical across all three** ("has ≥1 active
observation" = "is observing ≥1 target"), but the spec basis differs per class — inferred (MO) vs explicit
(RO/IO).

**⚠ SECOND keepalive clause for MutationObserver — the queued-record liveness (CORRECTION, `/code-review`
2026-07-02).** Registered-observer-list membership is **NOT the whole predicate**. DOM §4.3.2 "Queuing a
mutation record" (`#queue-a-mutation-record`, step 4.2 "Enqueue record to observer's record queue" + step
5 "Queue a mutation observer microtask") + §4.3 "notify mutation observers" (`#notify-mutation-observers`,
step 6.1 "Let records be a clone of mo's record queue" → step 6.4 "invoke mo's callback") mean an observer
with a **queued but undelivered record** must stay alive to deliver it — **even once its last observation
has ended** (its sole observed target despawned, or it was `disconnect()`ed, after the record queued but
before the notify microtask ran). In that window the observer is NOT in any node's registered observer
list (membership zero), yet dropping it loses the queued records: the delivery path (`take_records` then a
missing binding lookup) silently discards them, and the callback never fires = observable data loss. So
the **full MutationObserver keepalive predicate = "has ≥1 active observation OR has ≥1 pending undelivered
record"**. This is the exact analogue of the SSE §9.2.9 "task queued on the remote event task source"
strong-reference clause that S5-3b's `es_keepalive` `has_queued_task` INCLUDED — the same "queued task
awaiting the notify microtask ⇒ strong reference" liveness, which the membership-only analysis above
missed. The pending-record disjunct is engine-indep (`mutation::observers_with_pending_records(&self) ->
HashSet<u64>`, reading the registry's `records` queue — HostData only, **no World**, so it holds bound AND
unbound), keyed on **non-empty `records`** (not stale `pending` membership: `takeRecords()` empties
`records` without touching `pending`, so a drained observer has nothing to deliver and needs no keepalive).
**RO/IO have NO analogous clause** (§2.2/§2.3): their delivery is synchronous (gather-then-deliver in one
`deliver_*` call, no persistent cross-checkpoint entry queue), so active-observation membership is their
sole signal.

### §2.2 ResizeObserver — resize-observer-1 §3.5 Lifetime + §3.2.2 internal slots (`resize-observer-1`, webref 2026-07-02)
**PRIMARY keepalive basis = §3.5 "ResizeObserver Lifetime" (`#lifetime`)**, which states the exact
normative predicate verbatim: *"A ResizeObserver will remain alive until both of these conditions are met:
there are no scripting references to the observer; the observer is not observing any targets."* So the
observer is collectible **only** when it has no JS ref AND observes nothing — precisely the S5-3c predicate
(keep iff ≥1 observation, as an ADDITIONAL root beyond the JS ref). The **mechanism** for "is observing"
is the `[[observationTargets]]` / `[[activeTargets]]` internal slot (§3.2.2 "ResizeObserver",
`#resize-observer-slots`; `#dom-resizeobserver-observationtargets-slot` /
`#dom-resizeobserver-activetargets-slot`): the Document owns a **list of resize observers** and processes
each, and an RO with a non-empty `[[observationTargets]]` participates. (There is **NO separate "garbage
collection" section** in resize-observer-1 — `dfn 'garbage collection' resize-observer-1` → no hit — but,
UNLIKE MutationObserver, keepalive here is not merely INFERRED from list membership: §3.5 states the
two-condition lifetime EXPLICITLY.) Keepalive = `[[observationTargets]]` non-empty = has ≥1 active
observation. Note the §3.5 clause *"there are no scripting references"* directly substantiates §2.5's
"the predicate is an ADDITIONAL root; a JS-referenced observer survives independently" point and the
negative control (cross-ref).

### §2.3 IntersectionObserver — intersection-observer §3.3 Lifetime + §3.1.3 internal slot (`intersection-observer`, webref 2026-07-02)
**PRIMARY keepalive basis = §3.3 "IntersectionObserver Lifetime" (`#lifetime`)**, which states the exact
normative predicate verbatim: *"An IntersectionObserver will remain alive until both of these conditions
hold: There are no scripting references to the observer; The observer is not observing any targets."* So —
identical to RO §3.5 — collectible **only** when it has no JS ref AND observes nothing, matching the S5-3c
predicate exactly. The **mechanism** for "is observing" is the `[[ObservationTargets]]` internal slot
(§3.1.3 "IntersectionObserver", `#intersection-observer-private-slots`;
`#dom-intersectionobserver-observationtargets-slot`): the observer is processed while it has observation
targets. (There is **NO separate "garbage collection" section** — `dfn 'garbage collection'
intersection-observer` → no hit — but, UNLIKE MutationObserver, keepalive is stated EXPLICITLY by §3.3's
two-condition lifetime, not merely inferred from slot membership.) Keepalive = `[[ObservationTargets]]`
non-empty = has ≥1 active observation. The §3.3 *"There are no scripting references"* clause substantiates
§2.5's additive-root point and the negative control (cross-ref).

### §2.4 Coupled-invariant enumeration (edge-dense canonical home)
This is an edge-dense arm; the simultaneously-satisfied invariants live here (§8's matrix is the
per-kind expansion). The three observer arms must satisfy, **together**:

- **GC-rooting** — the predicate runs in `keepalive_survivors` (called at `collect.rs:1233`, marked at
  `:1234-1238`) and `mark_object`s survivors **before** `trace_work_list` (`:1315`) and **before** the
  sweep (`:1388`+).
- **observation-lifecycle** — the observation truth lives as per-entity `*ObservedBy` components
  (`mutation/mod.rs:114` `MutationObservedBy(Vec<MutationObservation>)`; `intersection.rs:80`;
  `resize.rs:102`); membership toggles at `observe` / `unobserve` / `disconnect` /
  `add_transient_observers` / transient-clear / **entity despawn** (auto-drop, §5.2).
- **per-class-predicate** — the observer arm registers its own spec-faithful rule = "**has ≥1 active
  observation**", **owned by engine-indep `elidex-api-observers`** (§4.4 layering), the seam only
  marshals (reads the binding maps, calls the query, marks survivors).
- **active-state** — the in-flight condition IS the observation membership (there is no separate
  readyState axis, unlike WS/ES; membership *is* the active state).
- **callback-root duality** — a surviving observer must keep **BOTH** its `instance` AND its `callback`
  ObjectId rooted (the callback is what actually fires; the instance is the callback's `this` +
  2nd arg), and release **BOTH** when the last observation ends (§4.2, §8 edge 4).
- **binding-row-lifecycle** — the binding-map ROW (`HashMap<u64, ObserverBinding>`) must be **swept**
  by the instance's mark bit (§4.3), not just leave the ObjectIds un-rooted — else the stale row leaks
  the struct and holds a dangling/reusable `instance` ObjectId.
- **unbind-lifecycle** — the binding maps stay per-VM and are **retained** across unbind (keyed by
  VM-monotonic `observer_id`, `vm_api.rs:685-696`); the seam's sweep prune (not unbind) is what shrinks
  them. **CRITICAL nuance (against the code):** `Vm::unbind` (`host_data.rs:1212-1215`) **NULLs `dom_ptr`**
  (+ bumps `bind_epoch`) but does **NOT despawn the `EcsDom` World** — the World is externally owned,
  passed by raw pointer at `bind` (`host_data.rs:1174-1209`). So the `*ObservedBy` observation components
  **PERSIST across a mere unbind** (they vanish only when the external owner despawns/replaces the document
  = a NAVIGATION, which is distinct from `Vm::unbind` — unbind "closes every BATCH … not only a
  navigation", `vm_api.rs:798-806`). During the unbound window `dom_ptr` is null → the World is
  **UNREADABLE** (`dom_shared` would assert), so the observation membership **cannot be evaluated**;
  therefore the keepalive pass **keeps ALL observer bindings while unbound** (fail-safe, mirroring the MQL
  arm at `keepalive.rs:236-256` which keeps every `document`-tagged MQL across an unbound inter-batch GC),
  and evaluates the precise membership predicate **only while bound** (§4.2).
- **B1-home** — the rooted thing is a per-VM `ObjectId` (side-store→component exception (a)); component
  migration is B1-gated (§6). Note the nuance: the **observation TARGET tracking already uses the ideal
  ECS-native per-entity `*ObservedBy` component** — only the observer *wrapper's* `ObjectId` binding is
  per-VM.

Key pairwise intersections (one line each):
- **GC-rooting × observation-lifecycle** — the mark pass reads the *current* observation membership to
  decide survival; the truth source is the per-entity `*ObservedBy` components (D2) or a maintained
  count (D1) (§5.2).
- **observation-lifecycle × entity-despawn** — an observed entity's despawn drops its `*ObservedBy`
  component **with no registry notification** (`teardown.rs:133` `world.despawn`); D2 reads live truth
  (despawn-safe by construction), D1 must add a despawn→decrement hook or it stale-leaks (§5.2 = THE
  discriminator).
- **callback-root duality × binding-row-lifecycle** — a surviving observer pushes **both** binding
  ObjectIds (so both survive) AND its row is kept because its `instance` bit is now set; a
  non-observing-but-JS-referenced observer keeps its row via the JS-root mark on `instance`; a
  non-observing + unreferenced observer has `instance` unmarked → its row is pruned (§4.3).
- **per-class-predicate × active-state** — membership *is* the active state (no readyState disjunct);
  observe-replaces (§2.1) and transient observers must be counted/queried exactly (D1 exactness burden
  / D2 handles by construction, §5.2).
- **unbind-lifecycle × GC-rooting** — the unbound-GC fail-safe **keeps all bindings** (the World is
  unreadable so the precise predicate cannot run) so a subsequent rebind can resume delivery; the
  **bound**-GC evaluates the predicate precisely (marks only genuinely-observing observers). This mirrors
  the MQL arm's "keep all `document`-tagged across unbound" and the AbortSignal.timeout unconditional
  membership mark (`keepalive.rs:261`, no `is_bound` guard) — the observer arm must NOT skip-to-collect
  while unbound (§4.2).

### §2.5 Step-1-snapshot INDEPENDENCE — S5-3c is DECOUPLED (settle the flagged coupling)
The reopened slot `#11-keepalive-event-loop-step1-snapshot` (S5-3b §10 / Codex #440 R4-R7) text says it
is "cross-cutting with the S5-3c observer arm keying the same snapshot". **SETTLE: it is NOT.** The
WS-only step-1-snapshot gap exists because **WebSockets §7 keys its keepalive tiers to the readyState
"as of the last time the event loop reached step 1"** (a step-1 SNAPSHOT). The three observer specs have
**NO "garbage collection" section** (webref §2.1-2.3, all three `dfn 'garbage collection'` → no hit; RO/IO
instead carry EXPLICIT §3.5/§3.3 Lifetime prose, §2.2/§2.3) and **NO "as of the last time the event loop
reached step 1" KEEPALIVE-SNAPSHOT tiering language** — the WS §7-specific construct. (Narrow phrasing, so
the absence claim is literally true: DOM §4.3's body DOES contain ordinary numbered "step 1" list items in
its queue/notify algorithms, and intersection-observer §3.4.1 is literally titled "HTML Processing Model:
Event Loop" — but that IO §3.4.1 is a rendering / Update-the-rendering substep, **not** a GC-keepalive
tiering. What is absent is the step-1 keepalive-snapshot tiering, not the substrings "step 1" / "event
loop".) So observer keepalive is **LIVE-membership-keyed**: reading the *current* observation membership
during a mid-turn GC is spec-correct — there is **no step-1 snapshot requirement** to honor, and hence
no snapshot to key or ordering constraint to satisfy.

The one apparent hazard — a **transiently-zero-observation** observer that JS still references, seen
mid-turn by an allocation-triggered GC — is a **non-issue**: the predicate is an **ADDITIONAL** root, not
the only one (RO §3.5 / IO §3.3 Lifetime state this directly: an observer stays alive while there are
"scripting references to the observer", independent of the observation clause). A JS-referenced observer
survives via its **JS root** (the `instance` ObjectId is reachable from a live JS value → marked in
`mark_roots`), independent of the keepalive predicate. So a mid-mutation transient-zero window causes **no
early collection**. (A NOT-JS-referenced observer with zero live observations *should* be collectible —
that is the fix, not a bug.)

**A SECOND window is separately covered — the UNBOUND-GC window.** A genuinely-observing but
no-JS-ref observer seen by an **unbound** inter-batch GC is NOT wrongly collected: during unbound the World
is unreadable (`dom_ptr` null), so the pass **keeps all observer bindings** (the keep-all-during-unbound
branch, §4.2, mirroring the MQL arm at `keepalive.rs:236-256`), deferring precise collection to the next
**bound** GC. (An observation persists across unbind — `Vm::unbind` does not despawn the World, §2.4 — so
without this branch the earlier "is_bound()-skip" framing would have pruned a still-observing observer's
binding, an under-root regression. This branch is why it does not.) The **step-1-snapshot decoupling
conclusion is UNAFFECTED** by either window: observers remain live-membership-keyed (bound) / keep-all
(unbound), neither of which is a step-1 snapshot dependency.

> **CONCLUSION (record in plan-review + the slot ledger): S5-3c is INDEPENDENT of the
> `#11-keepalive-event-loop-step1-snapshot` slice** — no snapshot dependency, no ordering constraint,
> no shared mechanism. The slot text's "cross-cutting with S5-3c keying the same snapshot" clause is
> **incorrect** and should be struck at S5-3c landing (§10 deliverable). The step-1 slot is WS-only.

---

## §3 Spec coverage map (keepalive RULES × condition-branches)

This is a **GC-keepalive** arm, so the rows are keepalive **RULES × membership condition-branch** (one
mark-roots pass consulting the predicate). "Touch" names the predicate / mark-pass site from §5.

| Spec section | Step / condition | Branch | Touch (predicate / mark-pass site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| DOM §2.8 Observing event listeners (`#observing-event-listeners`) | general default (no bare-listener keepalive) | — | seam does NOT root on bare-listener presence; observers are membership-rooted, not listener-rooted | ✓ | yes (page) |
| DOM §4.3 registered-observer-list (`#registered-observer-list`) | MutationObserver, ≥1 active registration | has ≥1 `MutationObservedBy` observation for this observer | direct membership mark in `keepalive_survivors` → `mutation::observing_observer_ids` (D2) / count (D1) | ✓ | yes (`observe`) |
| DOM §4.3.2 "Queuing a mutation record" (`#queue-a-mutation-record`) step 4.2/5 + §4.3 "notify mutation observers" (`#notify-mutation-observers`) | MutationObserver, ≥1 pending undelivered record (queued, awaiting the notify microtask) | has a NON-EMPTY `records` queue — **stays alive even with zero active observations** (target despawned / disconnected mid-window) | SECOND disjunct in `keepalive_survivors` (OR-ed with membership) → `mutation::observers_with_pending_records` (registry-only, no World) | ✓ | yes (`observe` → mutate) |
| DOM §4.3.1 `observe()` (`#interface-mutationobserver`) step 7 | observe an already-observed target | REPLACE existing registration (no double-membership) | predicate reads current membership (D2 set / D1 replace-not-add count) | ✓ | yes |
| DOM §4.2.3 "Mutation algorithms" — remove algorithm step 15 / §4.3 transient registered observers | transient observation | transient membership counts while present, cleared at notify step 6.3 | same predicate reads `transient:true` entries as membership (D2 automatic; D1 counts add/clear) | ✓ | no (engine-driven) |
| resize-observer-1 §3.2.2 `[[observationTargets]]` (`#dom-resizeobserver-observationtargets-slot`) | ResizeObserver, `[[observationTargets]]` non-empty | has ≥1 `ResizeObservedBy` observation | direct membership mark in `keepalive_survivors` → `resize::observing_observer_ids` (D2) / count (D1) | ✓ | yes (`observe`) |
| intersection-observer §3.1.3 `[[ObservationTargets]]` (`#dom-intersectionobserver-observationtargets-slot`) | IntersectionObserver, `[[ObservationTargets]]` non-empty | has ≥1 `IntersectionObservedBy` observation | direct membership mark in `keepalive_survivors` → `intersection::observing_observer_ids` (D2) / count (D1) | ✓ | yes (`observe`) |
| DOM §4.3 registered-observer-list (`#registered-observer-list`) — membership end via entity despawn | entity despawn of sole observed target → observation implicitly ends | membership drops (component auto-vanishes with the entity) | D2: not scanned (collectible); D1: needs a despawn→decrement hook (§5.2) | ✓ | no (tree mutation) |
| PerformanceObserver — performance-timeline §4 (`#performanceobserver`) | (NOT implemented in VM) | reference-only | n/a — no `ObjectKind` / registry; "born into seam when it lands" (cf. parent §3 XHR row) | n/a | n/a (future) |

### §3.1 Breadth + split verdict
**K = 4** distinct specs (DOM, resize-observer-1, intersection-observer, performance-timeline);
**M = 8** rows. This is the **observer row-subset** the parent S5-3 §3 table (K=6 / M=13) carved into
its own slice (parent row "DOM §4.3 registered-observer-list … observer predicate replaces
construct-root"). The three implemented observers share **one mechanism** (per-entity `*ObservedBy`
membership) and **one seam site** (a direct membership mark in `keepalive_survivors`, shape B §4.2), so
the breadth here is bounded to one predicate over three kinds. S5-3c is a **single PR** (the narrow observer slice under the
approved umbrella + this plan-review = a terminal base-case, §7), not a re-split. PerformanceObserver is
a reference-only future row (§9 confirms out of scope so reviewers don't flag its absence).

### §3.2 User-input touch audit
User-controllable inputs: the page's `new MutationObserver(cb)` / `.observe(target, init)` /
`.disconnect()` / `.unobserve(target)` calls (RO/IO analogues), plus tree mutations that despawn observed
entities. The predicate reads the **observation membership** (page-driven via `observe`/`unobserve`) but
tests only **presence of ≥1 observation** — a pure `has_observation(observer_id) → bool` over per-VM /
per-entity state the page already produced. There is **no injection surface**: an unregistered /
never-observed id simply fails the membership test and is collected. The callback `ObjectId` is
page-supplied but is only *marked*, never *invoked*, by the seam. So the arm opens **no new trust
boundary** (cf. umbrella §3.1: the flip is trust-boundary-neutral).

---

## §4 The ideal — extend the seam (mechanism design)

### §4.1 Where it hooks (same seam as S5-3a/b — `&VmInner`, NOT `GcRoots`)
S5-3a/b's landed seam reads `&VmInner` directly in `keepalive_survivors` (`keepalive.rs:230`), reusing
per-VM state maps rather than threading them into the `GcRoots` snapshot. S5-3c follows it. The observer
binding maps are `pub(crate)` fields on `HostData` (`host_data.rs:281`/`:297`/`:308`), reachable from
`gc/keepalive.rs` as `vm.host_data.as_deref().map(|hd| &hd.mutation_observer_bindings)` — the **same
`&VmInner` borrow** the MQL/WS/ES arms use. **No `GcRoots` change, no new side-store** — the seam adds
only the predicate-consulting mark pass **and** (the required deliverable) the binding-row sweep prune
(§4.3). The construct-time root in `gc_root_object_ids` (`host_data.rs:1712-1723`) is **removed** — that
removal is the behavior change; the predicate + sweep prune replace it.

### §4.2 The marshalling layer (seam MARSHALS, engine-indep crate RULES) — the dispatch shape
**Plan-review Q (analogous to parent §4.2 / Q5): shape A vs shape B.** The parent S5-3 §4.2 shape-A list
included an `Observer` variant of `KeepaliveClass`. Two candidate shapes:

- **(shape A) a `KeepaliveClass::Observer(ObserverKind)` variant** — the survivor loop dispatches per
  kind, the arm's `keepalive` reads *its* binding map and calls *its* registry's membership query. This
  matches the existing `KeepaliveClass` enum-dispatch (the WS/ES arms are variants), and `ObserverKind`
  (`Mutation`/`Resize`/`Intersection`) already exists as the `ObjectKind::Observer { kind, observer_id }`
  discriminator (`observer_common.rs:52` import; brand-check `require_observer_receiver`
  `observer_common.rs:75-93`). **BUT** the `KeepaliveClass::keepalive(self, vm, target: ObjectId) -> bool`
  signature is **ObjectId-keyed** (fits MQL/WS/ES whose side-stores are `ObjectId`-keyed). Observer
  binding maps are **`u64`-keyed** (`observer_id`), with 2 ObjectId *values* per row, and the survivor
  must push **both** ObjectIds. So a per-ObjectId `-> bool` predicate is a poor fit — the survivor loop
  needs the `u64` key + the binding pair, not a single target ObjectId.

- **(shape B, RECOMMENDED) iterate the three binding maps inline in `keepalive_survivors`, no new
  `keepalive`-enum variant** — for each binding map, compute its kind's observing-id set (D2) **once**,
  then for each `(observer_id, binding)` row whose `observer_id ∈ set`, push **both** `binding.instance`
  and `binding.callback`. This mirrors the shape `keepalive_survivors` **already uses** for
  `AbortSignal.timeout` membership (`keepalive.rs:261` `keep.extend(vm.pending_timeout_signals.values()
  .copied())`) — a **membership** registrant marked directly, *not* through the per-ObjectId `keepalive`
  dispatch (the module doc `keepalive.rs:51-56` already distinguishes "listener-predicate" registrants
  [dispatched via `KeepaliveClass`] from "membership" registrants [marked directly]). Observers are
  **membership** registrants (registration in the registered-observer-list *is* the anchor), so they
  belong in the direct-mark family, NOT the `KeepaliveClass` listener-predicate enum.

> **Recommendation: shape B** (direct membership mark, mirroring `AbortSignal.timeout`). It is the
> **One-issue-one-way** fit: the seam already has a membership-mark family and the module doc already
> classifies observers as membership registrants; the `KeepaliveClass` enum is explicitly the
> *listener*-test family, which observers are not (their keepalive is not a listener test — §0.2). This
> **reconciles** the parent §4.2 shape-A "add an `Observer` enum variant" note: that note predates the
> S5-3a landing that split listener-predicate from membership; under the landed seam, observers are a
> membership registrant, not a `KeepaliveClass` variant. **Surfaced as a plan-review Q** because it
> revises a parent framing.

Sketch (impl owns exact form; the D1/D2 choice §5.2 fills `observing_ids`):

```rust
// keepalive.rs — keepalive_survivors, appended after the WS/ES membership marks:
if let Some(hd) = vm.host_data.as_deref() {
    if hd.is_bound() {
        // BOUND GC — evaluate the PRECISE membership predicate. Membership sets
        // derived ONCE per kind from live observation truth (D2) — an engine-indep
        // query owned by elidex-api-observers; NO spec-rule in vm/gc.
        // (D1 variant: read a maintained count instead — §5.2.)
        let dom = hd.dom_shared();                         // &EcsDom (read-only, GC holds &VmInner)
        let mo_ids = elidex_api_observers::mutation::observing_observer_ids(dom);
        let ro_ids = elidex_api_observers::resize::observing_observer_ids(dom);
        let io_ids = elidex_api_observers::intersection::observing_observer_ids(dom);
        for (oid, b) in &hd.mutation_observer_bindings {
            if mo_ids.contains(oid) { keep.push(b.instance); keep.push(b.callback); }
        }
        for (oid, b) in &hd.resize_observer_bindings {
            if ro_ids.contains(oid) { keep.push(b.instance); keep.push(b.callback); }
        }
        for (oid, b) in &hd.intersection_observer_bindings {
            if io_ids.contains(oid) { keep.push(b.instance); keep.push(b.callback); }
        }
    } else {
        // UNBOUND GC — dom_ptr is null, so the EcsDom World is UNREADABLE and the
        // membership predicate CANNOT be evaluated. FAIL-SAFE = KEEP ALL observer
        // bindings (mirrors the MQL arm keepalive.rs:236-256 which keeps every
        // document-tagged MQL across an unbound inter-batch GC, and the unconditional
        // AbortSignal.timeout membership mark at keepalive.rs:261). Vm::unbind does
        // NOT despawn the externally-owned World (host_data.rs:1212-1215 only NULLs
        // dom_ptr), so the *ObservedBy observations PERSIST across unbind; a
        // genuinely-observing no-JS-ref observer must survive so the next
        // same-document rebind's deliver can fire it. This over-keep is TRANSIENT and
        // SELF-CORRECTING: the next BOUND GC runs the precise predicate and collects
        // genuinely-idle observers. It is NOT the immortal-until-unbind leak S5-3c
        // fixes (that was per-life rooting at EVERY GC incl. bound; here idle
        // observers are still collected at the frequent bound GCs — unbound GCs are
        // rare, GC usually fires during bound script execution).
        keep.extend(hd.mutation_observer_bindings.values().flat_map(|b| [b.instance, b.callback]));
        keep.extend(hd.resize_observer_bindings.values().flat_map(|b| [b.instance, b.callback]));
        keep.extend(hd.intersection_observer_bindings.values().flat_map(|b| [b.instance, b.callback]));
    }
}
```

**Callback-root duality (parent §8 edge 4) — the exact survivor shape.** Unlike WS/ES (push a single
target ObjectId), each surviving observer pushes **BOTH** `b.instance` **and** `b.callback`. Both are
marked, so the sweep retains both AND `trace_work_list` traces their fan-out (e.g. the callback's
closed-over upvalues). Releasing them together when the last observation ends is automatic: no
observation ⇒ `observer_id ∉ set` ⇒ neither pushed ⇒ both collectible (unless independently JS-rooted).

**The unbound branch — WHY keep-all, not skip-to-collect (the load-bearing correction).** An earlier
framing guarded the whole mark pass on `is_bound()` and reasoned "unbound GC ⇒ no observations reachable ⇒
no observer marked ⇒ an unreferenced observer is collectible (observes nothing)". **That is WRONG against
the code.** `Vm::unbind` (`host_data.rs:1212-1215`) NULLs `dom_ptr` but does **NOT despawn** the
externally-owned `EcsDom` World (bound by raw pointer at `host_data.rs:1174-1209`) — and `unbind` "closes
every BATCH … not only a navigation" (`vm_api.rs:798-806`). So the `*ObservedBy` observations **PERSIST**
across a mere unbind (they vanish only on the actual document despawn/replace = navigation, distinct from
`Vm::unbind`). During the unbound window `dom_ptr` is null → the World is **UNREADABLE** (not "empty"),
so the predicate cannot be evaluated. Skipping-to-collect would prune a **genuinely-observing** no-JS-ref
observer's binding row; on rebind the observation still exists on the entity but the callback/binding are
gone → the observer **can never fire again** = a NEW under-root regression introduced by the over-root fix.
The **precedent-forced fix** (One-issue-one-way with the landed arms): the MQL arm KEEPS every
`document`-tagged MQL across an unbound inter-batch GC (`keepalive.rs:236-256` — a listener-only MQL must
survive so the next same-document rebind's `deliver` can fire it), and the `AbortSignal.timeout` membership
mark (`keepalive.rs:261`) is likewise **unconditional** (no `is_bound` guard). The observer arm **mirrors
both**: bound → precise D2 predicate; unbound → **keep ALL bindings** (fail-safe). Precise collection is
deferred to the next **bound** GC — self-correcting, and NOT the immortal-until-unbind leak (idle observers
are still collected at the frequent bound GCs; unbound GCs are rare).

> **Parity caveat (not byte-identical to the MQL arm — the mirror is DIRECTIONAL).** The MQL arm's
> unbound keep is still *filtered* — it keeps only `document`-tagged MQLs (`media_query.rs` `keepalive_worthy`:
> `self.document.is_some() && …`), so an unbound-created `document == None` MQL is still collected while
> unbound. `ObserverBinding` (`observer_common.rs:122`) carries only `{callback, instance}` — **no document
> tag** — so the observer arm cannot apply that refinement and keeps **every** binding while unbound
> (strictly coarser than the MQL subset). This is **safe**: the over-keep is transient (the next bound GC
> runs the precise predicate) and never worse than the pre-fix all-GC rooting it replaces. The mirror is at
> the level of *direction* (keep-across-unbound, don't skip-to-collect), not a document-tag-filtered parity.

### §4.3 Binding-map ROW pruning (a REQUIRED deliverable — do NOT omit)
Removing the bindings from `gc_root_object_ids` un-roots the ObjectIds, but the binding-map **rows**
(`HashMap<u64, ObserverBinding>`) still persist until `unbind` — a **residual struct leak + dangling
`ObjectId` hazard** (a stale row holds a swept, index-reusable `instance` ObjectId; a later GC could
mark a *different* object that reused that index). The fix mirrors the established sweep pattern
`vm_event_listeners.retain(|id, _| bit_get(marks, id.0))` (`collect.rs:1746`): after the sweep marks are
final, prune each `*_observer_bindings` by the **instance's** mark bit:

```rust
// collect.rs sweep tail (new — parallel to the vm_event_listeners retain at :1746):
if let Some(hd) = self.host_data.as_deref_mut() {
    hd.mutation_observer_bindings.retain(|_, b| bit_get(marks, b.instance.0));
    hd.resize_observer_bindings.retain(|_, b| bit_get(marks, b.instance.0));
    hd.intersection_observer_bindings.retain(|_, b| bit_get(marks, b.instance.0));
}
```

Keying on `b.instance.0` (the wrapper's mark bit) gives the exact three-way lifecycle:
- **observing** → predicate marked `instance` (§4.2) → bit set → row **kept** (+ callback kept);
- **not observing but JS-referenced** → `instance` marked via its JS root (independent of the predicate)
  → bit set → row **kept** (so a later `observe()` on the retained `mo` still finds its binding + fires);
- **not observing + not JS-referenced** → `instance` unmarked → bit clear → row **pruned** (the leak fix;
  the swept observer's binding struct is reclaimed).

**Ordering edge (§8 edge 3):** this prune runs in the **sweep** (after `mark_roots` + keepalive mark +
`trace_work_list` have set every live bit); the keepalive **mark** runs earlier in `collect_garbage`
(`:1234-1238`). So by the time the prune keys on `b.instance.0`, the observing / JS-referenced instances
already have their bit set. Verify at impl that the prune is placed after the mark-complete point (co-
located with the `vm_event_listeners.retain` at `:1746`, which has the same requirement). Prune the
`callback` implicitly: a row is kept iff its `instance` bit is set (keying on `instance` is sufficient
and correct because `instance` is the wrapper identity), and a kept row **never orphans its callback** —
the callback's bit is set whenever the instance survives, via **either** the keepalive predicate push
(§4.2, the observing case) **or** the `ObjectKind::Observer` **ownership trace edge** (`gc/trace.rs`) for
the **idle-but-JS-referenced** case where the predicate pushes nothing yet the instance is JS-reachable
(so a later `observe()` still finds a live callback). *(Impl note: the trace edge is load-bearing for the
NEGATIVE CONTROL — an idle JS-referenced observer whose callback would otherwise be collected; the earlier
"pushed in lockstep, no separate keying" framing held only for the observing case. The callback is an
ownership edge of the instance, mirroring the TreeWalker/NodeIterator filter + Selection→Range fan-out.)*
Note: this replaces the deferred `#11-mutation-observer-extras` "weak-rooting / sweep-time
cleanup" the field doc (`host_data.rs:279`) tracks — S5-3c **delivers** that cleanup for the keepalive
subset (call it out in §10 / the field-doc reframe).

### §4.4 Layering — `elidex-api-observers` owns the membership query
Per CLAUDE.md "VM host/ は engine-bound 責務のみ" / "新規 algorithm を host/ に書く前に engine-independent
crate を確認": the "which observers currently observe ≥1 target" membership query is a **spec/domain
algorithm** (it is the DOM §4.3 registered-observer-list / RO-IO `[[observationTargets]]` membership
computed from the `*ObservedBy` components), not engine-bound marshalling. It lives in
**`elidex-api-observers`** as a new fn per module:

```rust
// elidex-api-observers/src/mutation/mod.rs (+ intersection.rs, resize.rs analogues)
/// Observer ids (raw u64) that currently have ≥1 active observation — the DOM §4.3
/// registered-observer-list membership (RO/IO: [[observationTargets]] non-empty),
/// derived from the live per-entity `MutationObservedBy` components in one archetype
/// query. Despawn-safe by construction: a despawned entity's component is gone, so a
/// stale (observer, despawned-target) pair is never scanned.
pub fn observing_observer_ids(dom: &EcsDom) -> std::collections::HashSet<u64> {
    let mut ids = std::collections::HashSet::new();
    for (_e, comp) in dom.world().query::<(Entity, &MutationObservedBy)>().iter() {
        for obs in &comp.0 {
            ids.insert(obs.observer.raw());
        }
    }
    ids
}
```

The **seam owns only**: read the binding maps, call `observing_observer_ids`, intersect keys, push the
survivor ObjectIds, and (sweep) prune the rows by mark bit. **No spec-rule branching in `vm/gc`.** These
queries get their **own engine-indep unit tests** in `elidex-api-observers` (empty world → empty set;
one `observe` → the id present; `unobserve`/`disconnect` → id absent; despawn of sole target → id absent;
transient observation → id present while transient present), independent of the VM (§9).

---

## §5 Per-class predicate detail + the D1-vs-D2 decision

### §5.1 Per-kind table (spec § + elidex predicate + wiring site)

| Class | Spec § (webref) | elidex keepalive predicate | Wiring site | Replaces / fixes |
|---|---|---|---|---|
| **MutationObserver** | DOM §4.3 registered-observer-list (`dom#registered-observer-list`; no GC-note) **+ §4.3.2 queued-record / §4.3 notify** (pending-record liveness) | **has ≥1 active observation OR ≥1 pending undelivered record** (any `MutationObservedBy` entry with `observer == this`, permanent OR transient; OR a non-empty `records` queue) | **rule** `mutation::observing_observer_ids(dom) -> HashSet<u64>` (World, D2) **OR-ed with** `mutation::observers_with_pending_records() -> HashSet<u64>` (registry-only, no World); **seam** iterates `mutation_observer_bindings` (`host_data.rs:281`), pushes `[instance, callback]` for ids in EITHER set | **FIXES the construct-time for-life over-root / leak** (`gc_root_object_ids` `:1712-1723`; `disconnect` no-release `mutation_observer.rs:249-261`) **AND the queued-record data-loss** (pending record dropped if wrapper swept mid-window — `/code-review` 2026-07-02) |
| **ResizeObserver** | resize-observer-1 §3.5 "Lifetime" (`#lifetime`, EXPLICIT two-condition keepalive prose) — mechanism §3.2.2 `[[observationTargets]]` (`#dom-resizeobserver-observationtargets-slot`) | **has ≥1 active observation** (any `ResizeObservedBy` entry with `observer == this`) | **rule** `elidex_api_observers::resize::observing_observer_ids(dom)` (NEW); **seam** iterates `resize_observer_bindings` (`host_data.rs:297`) | same over-root fix (RO binding) |
| **IntersectionObserver** | intersection-observer §3.3 "Lifetime" (`#lifetime`, EXPLICIT two-condition keepalive prose) — mechanism §3.1.3 `[[ObservationTargets]]` (`#dom-intersectionobserver-observationtargets-slot`) | **has ≥1 active observation** (any `IntersectionObservedBy` entry with `observer == this`) | **rule** `elidex_api_observers::intersection::observing_observer_ids(dom)` (NEW); **seam** iterates `intersection_observer_bindings` (`host_data.rs:308`) | same over-root fix (IO binding) |
| **PerformanceObserver** | performance-timeline §4 (`#performanceobserver`) | — (not implemented in VM: no `ObjectKind`, no registry) | — | **reference-only / OUT OF SCOPE** — born into the seam when the interface lands (cf. parent §3 XHR row) |

### §5.2 The `is_observing` implementation — **D1 vs D2 (THE central ECS-native decision)**

The parent S5-3 §5.2 recommended **D1** (an incremental per-observer active-observation COUNT on the
registry side, O(1) `is_observing`) and characterized the D2 alternative (query the components per GC) as
an **O(N²) per-GC full-entity scan**. **The code investigation refutes that O(N²) characterization and
inverts the recommendation.** Both designs are presented; **D2 is recommended**; the choice is a
plan-review decision (Q3).

#### D1 — incremental active-observation count per observer (parent §5.2's pick)
Maintain a `HashMap<observer_id, usize>` (or a `usize` per `RegisteredObserver`) on each registry;
`is_observing(oid) = count > 0`. O(1) per registrant at GC.
- **Increment**: `observe` (`mutation/mod.rs:256`), `add_transient_observers` (`:186`), and the RO/IO
  `observe` (`resize.rs:133` / `intersection.rs:150`).
- **Decrement**: `unobserve` (`resize.rs:167` / `intersection.rs:177`), `disconnect`
  (`mutation/mod.rs:339` via `retain_observations` `:561`; RO `resize.rs:179`; IO `intersection.rs:189`),
  `retain_observations`-family clears (`mutation/mod.rs:561`), transient-clear
  (`clear_transient_observers` `:582` / `by_source` `:592` / `clear_all` `:602`).
- **Exactness burdens D1 must get right** (each a divergence risk):
  1. **observe-replaces-existing** (DOM §4.3.1 step 7, §2.1): observing an already-observed target
     REPLACES the registration (no new membership) — `observe` must **not** double-count. But
     `mutation/mod.rs:280-328` matches existing `reg_id`s and updates-in-place vs. adds — the count
     would have to branch identically (a second source of truth for the same "is this a new membership?"
     decision → the exact divergence CLAUDE.md warns of).
  2. **transient add/clear** must be counted exactly (add at `add_transient_observers`, subtract at each
     of the 3 spec-distinct transient clears) — a 4-site accounting the transient lifecycle already
     found subtle (the #413 transient work).
  3. **THE HARD CASE — entity despawn.** When an observed DOM-node entity is despawned, its `*ObservedBy`
     component **auto-vanishes with the entity** (`teardown.rs:133` `world.despawn`) **WITHOUT routing
     through any registry decrement chokepoint** — the code investigation confirms **there is NO
     despawn→registry-decrement path** (`teardown.rs` `destroy_entity` / `despawn_subtree` make **zero**
     observer-registry calls; the module docs `intersection.rs:5-8` / `resize.rs:5-7` state "a despawned
     entity drops its observations automatically"). So a D1 count goes **stale-high** on despawn → the
     observer stays rooted though it observes nothing → a **residual leak that partially defeats the very
     fix S5-3c ships**. To make D1 correct you must **add a despawn→decrement hook** in `teardown.rs`
     (a new cross-cutting chokepoint that must decrement per-observer counts for every observation on the
     despawned entity) — new coupling between the ECS teardown and the observer registries that does not
     exist today, and that must itself be kept exact across `despawn_subtree`, shadow teardown, and
     unbind-world scrub.

#### D2 — derive the observing-id SET once per GC from live components (RECOMMENDED)
A single engine-indep pass per kind: `observing_observer_ids(dom) -> HashSet<u64>` (§4.4) runs **one
hecs archetype query** over the entities carrying `*ObservedBy` and flat-maps their observations'
`observer` ids into a set. `is_observing(oid) = set.contains(oid)`.
- **Cost is LINEAR, not O(N²).** The parent's O(N²) fear assumed a **per-observer independent re-scan**
  (scan all entities once *per observer*). D2 is a **single shared pass**: `HashSet` built in **one**
  archetype traversal that visits **only** entities carrying the component (hecs archetype query =
  contiguous, visits observed entities only, **not** all DOM entities), cost = **O(total live
  observations)** per GC (three queries, one per kind). Observations are few (a page has a handful of
  observers watching tens of nodes) and GC is infrequent; this is cheap. The seam then does O(observers)
  set-membership lookups over the (small) binding maps. **This is the query pattern the registries
  ALREADY use** for `disconnect` (`mutation/mod.rs:563` `world_mut().query::<(Entity, &mut
  MutationObservedBy)>()`) and `gather_observations` (`intersection.rs:273` / `resize.rs:226`), so D2
  adds no new access shape — just a read-only variant.
- **D2 is DESPAWN-SAFE BY CONSTRUCTION.** It reads **live** component truth: a despawned entity's
  `*ObservedBy` is gone, so its (observer, target) pair is simply **never scanned** — the observer's
  membership drops to zero automatically the instant the sole observed entity despawns, with **no hook**
  in `teardown.rs`. This is the exact hard case that forces D1 to grow a new chokepoint.
- **D2 is drift-free, self-correcting, and handles the exactness burdens for free.** No parallel count to
  maintain (no divergence surface). observe-replaces (§2.1) is automatic (the component already holds one
  registration per (observer, target), replaced in place — the set dedups by `observer` id regardless).
  Transient add/clear is automatic (transient entries live in the same component; present ⇒ in the set).
- **D2 is MORE ECS-native** (CLAUDE.md side-store→component rule: **do not maintain a divergeable
  side-store when you can query the components**). The observation truth already lives as the ideal
  per-entity `*ObservedBy` component (the `#213` outlier precedent, `ecs-native-side-store-audit`); a
  parallel count would be exactly the redundant side-store the rule forbids. **D2 reads the canonical
  truth; D1 mirrors it and risks divergence** (the despawn stale-high being the concrete divergence).
- **Feasibility VERIFIED (the one D2 precondition).** The keepalive pass needs read access to the EcsDom
  World during GC. `keepalive_survivors(vm: &VmInner)` holds `&VmInner` → `vm.host_data.as_deref()` →
  `&HostData` → **`HostData::dom_shared(&self) -> &EcsDom`** (`host_data.rs:1317`, the established
  immutable-World accessor for `&HostData`-holding read paths). The seam passes that `&EcsDom` **into**
  the engine-indep `observing_observer_ids(dom)` fn — the `.world().query::<…>()` call-shape lives
  **ONLY inside `elidex-api-observers`** (which owns the private `*ObservedBy` component type + the
  query); `vm/gc` calls `observing_observer_ids(dom)` alone and **never** `dom.world().query(...)`
  directly (§4.4 layering). No `&mut EcsDom` is live during the mark pass (the collector holds `&VmInner`
  immutably here, exactly as the MQL arm already reaches `document_entity_opt` `host_data.rs:1593`).
  Branch on `is_bound()` (`:1241`): while **bound** the precise D2 query runs (the `dom_shared` read is
  valid); while **UNBOUND** `dom_ptr` is null so the World is **UNREADABLE** (`dom_shared` would assert) —
  the pass does **NOT** skip-to-collect. Instead it **keeps ALL observer bindings** (fail-safe,
  MQL-arm-consistent `keepalive.rs:236-256` + the unconditional AbortSignal.timeout mark, §4.2), because
  the observations **persist** across a mere unbind (the World is not despawned — §2.4). **Do not
  conflate "unbound VM" with "despawned world":** an unbind NULLs `dom_ptr` without despawning the
  externally-owned World, so "unbound ⇒ observes nothing ⇒ collectible" is WRONG; the precise predicate is
  simply deferred to the next bound GC. **No borrow conflict; D2 is feasible on the landed `&VmInner`
  seam.**

#### §5.2.1 The pending-record disjunct — registry-only, so it holds bound AND unbound (CORRECTION)
The MutationObserver predicate is **not** the D2 membership query alone: it is OR-ed with the
**pending-record** disjunct `mutation::observers_with_pending_records() -> HashSet<u64>` (§2.1's second
clause; DOM §4.3.2 queued-record / §4.3 notify). Unlike `observing_observer_ids` (which reads the World's
`*ObservedBy` components), this reads **only the registry's `records` queue** — a HostData-owned side-store
with **NO World access**. Two consequences:
- **It is valid regardless of `is_bound()`.** The bound branch evaluates it explicitly (OR-ed with the D2
  membership set). The unbound branch's keep-all fail-safe already covers every binding (so pending-record
  observers are covered there too), so no separate unbound handling is needed — but the disjunct is *not*
  gated on `dom_shared` (it never touches the World), so it could be evaluated in either branch safely.
- **Key on non-empty `records`, NOT `pending` membership.** `takeRecords()` empties an observer's `records`
  queue (`take_records`) but deliberately leaves it in the `pending` notifySet (so notify step 6.3's
  transient clear still runs — `mutation/mod.rs` `pending` doc). A drained observer has nothing to deliver,
  so it needs no pending-record keepalive; keying on the record queue (the precise "has undelivered data"
  signal) avoids over-keeping a stale-pending, empty-queue observer.

> **Plan-review decision (Q3) — recommend D2.** D2 is the ideal / ECS-native / spec-faithful choice: a
> single cheap linear query of the canonical per-entity truth, despawn-safe **by construction**, drift-
> free, and needing **no new despawn chokepoint** (D1 needs one, or it stale-leaks — partially defeating
> the fix). It reconciles the parent §5.2 by **correcting** its O(N²) characterization (that assumed a
> per-observer re-scan; D2 is one shared pass) and inverting the D1 recommendation. **Recommendation:
> D2.** Surfaced because it revises the parent's explicit D1 pick and rests on the "no despawn→decrement
> path exists" code finding. If plan-review nonetheless prefers D1 (e.g. to avoid a per-GC World query),
> the despawn-hook follow-up must be carved as a slot (§10) — but the recommendation is D2, which needs
> no such slot.

---

## §6 ECS-native lens + B1 home constraint

The rooted thing is a per-VM `ObjectId` pair (the observer wrapper's `(callback, instance)` binding).
Under CLAUDE.md's side-store→component rule it is the **per-VM-identity-handle exception (a)**: the
values are `Send` (`ObjectId(u32)`) but their meaning is per-VM (JS wrapper + callback identities), and
the binding maps are keyed by VM-monotonic `observer_id` (`vm_api.rs:685-696` retains them across unbind
precisely because they are per-VM identity handles, no `Entity`/recycled-`ObjectId` aliasing). So S5-3c
lands the **per-VM HostData binding + predicate form** (the same per-VM-now / component-later pattern
S5-2/S5-3a/S5-3b used).

**Corrected unbind semantics (record for plan-review).** `Vm::unbind` NULLs `dom_ptr` but does **NOT**
despawn the externally-owned `EcsDom` World (`host_data.rs:1212-1215` vs the raw-pointer bind at
`:1174-1209`); so the `*ObservedBy` observation components **PERSIST across unbind** (they vanish only on
navigation = document despawn/replace). The binding maps' **retention** across unbind
(`vm_api.rs:685-696`) is therefore **consistent** with the keepalive pass's keep-all-during-unbound branch
(§4.2): both the binding row AND its observation survive the unbound window, so a same-document rebind can
resume delivery. The unbound window is UNREADABLE (no `dom_ptr`), which is exactly why the precise
predicate is deferred to the next bound GC rather than evaluated as "observes nothing".

**The interesting nuance (record for plan-review):** the observation **TARGET tracking already uses the
IDEAL ECS-native pattern** — the registered-observer list lives as a **per-entity `*ObservedBy`
component** (`mutation/mod.rs:114` / `intersection.rs:80` / `resize.rs:102`; the `#213` side-store→
component outlier, `ecs-native-side-store-audit-2026-05-21.md`). The **D2 design leans directly into
that** (it queries those components). Only the observer **wrapper's `ObjectId` binding** is the per-VM
part still in a HostData side-store — and *that* is the exception-(a) handle whose component migration is
B1-gated.

The component-migration ideal is tracked by the **existing** slot
`#11-eventtarget-keepalive-component-migration` (S5-3 §10), **B1-gated** (agent-scoped `EcsDom` World,
PR #434 — `world_id` SUPERSEDED): under 1-agent = 1-World per-entity identity is stable, so migrating the
observer wrapper binding to a marker-component-on-entity becomes safe without a discriminator. S5-3c adds
the observers as new registrants under that *same* deferred slot — **no new component owed pre-B1, no new
slot for the home question.**

**ECS axis confirmation for plan-review**: the predicate reads (a) per-VM observer-wrapper bindings
(exception (a)) + (b) the canonical per-entity `*ObservedBy` components (D2, the ideal ECS-native truth).
No per-entity DOM fact is mis-stored in a side-store; **D2 explicitly avoids** introducing a divergeable
count side-store (the D1 anti-pattern the rule forbids).

---

## §7 Scope (single PR, base-case — plan-review confirm)

S5-3c is a **single PR**: the narrow observer arm (3 kinds, 1 mechanism) under the approved S5 umbrella +
the S5-3 §7 split, having passed `/elidex-plan-review` = a **terminal base-case** (CLAUDE.md base-case
rule: a narrowly-scoped per-PR slice under an approved umbrella + plan-review is an allowed single PR;
the slice touching the same subsystem is not a re-split trigger). It is edge-dense (an over-root behavior
change + the D1/D2 ECS-native decision + the binding-row sweep prune) — which is why it gets this
plan-review, **not** why it must split further. **Scope = all three observer kinds atomically** — they
share the `ObserverBinding` infra + the `gc_root_object_ids` construct-root + must ALL be off that root
before S5-6 (leaving one on the construct-root = the forbidden partial strangler). No prereq split is
owed: `keepalive.rs` (295 LoC), `elidex-api-observers/{mutation/mod.rs,intersection.rs,resize.rs}` are
all under the 1000-line touch-time threshold; `collect.rs` is large but the added sweep-prune is a small,
cohesive addition co-located with the existing `vm_event_listeners.retain` (confirm the touch site is
under threshold at impl — if `collect.rs` itself needs a touch-time split, that is a standalone prereq
PR, not bundled).

---

## §8 Edge matrix (review-tail pre-empt)

| Invariant axis | MutationObserver | ResizeObserver | IntersectionObserver |
|---|---|---|---|
| **GC-rooting (seam mark)** | ✔ membership mark in `keepalive_survivors` (shape B, direct) | ✔ same | ✔ same |
| **membership source** | `MutationObservedBy` entries (permanent + transient) | `ResizeObservedBy` entries | `IntersectionObservedBy` entries |
| **per-class predicate (engine-indep)** | `mutation::observing_observer_ids` (D2) | `resize::observing_observer_ids` | `intersection::observing_observer_ids` |
| **active-state** | membership ≥1 (no readyState axis) **OR ≥1 pending undelivered record** | membership ≥1 | membership ≥1 |
| **pending-record liveness** | ✔ SECOND disjunct — non-empty `records` queue keeps the observer alive **even at zero observations** (queued record awaiting the notify microtask, DOM §4.3.2/§4.3); registry-only (no World) | ✖ NONE — synchronous gather-then-deliver, no persistent cross-checkpoint entry queue | ✖ NONE — same (VM buffers no entries; `takeRecords()` returns empty) |
| **callback-root duality** | push BOTH `instance` + `callback`; release both at last observation **AND after the last pending record delivers** | same | same |
| **binding-row prune** | sweep `mutation_observer_bindings.retain(by instance mark bit)` | `resize_observer_bindings.retain(...)` | `intersection_observer_bindings.retain(...)` |
| **despawn-safety** | D2: component gone ⇒ membership drops (safe); D1: stale-high (needs hook). **Refined by the pending-record disjunct**: despawn of the sole target ⇒ collectible **ONLY IF no pending records** — a despawned-target observer with a queued record STILL survives (to deliver it) | same, minus the pending-record refinement (no such disjunct) | same |
| **behavior-change** | **YES** — never-observed / disconnected unreferenced observer becomes COLLECTIBLE (over-root/leak fix) | **YES** — same | **YES** — same |
| **unbind-lifecycle (per-VM)** | binding map RETAINED on unbind (`vm_api.rs:685-696`); observations PERSIST (World not despawned, `host_data.rs:1212-1215`); unbound GC KEEPS ALL bindings (World unreadable → fail-safe, §4.2); shrunk by sweep prune at a BOUND GC | same | same |
| **B1-home (component defer)** | exception (a) per-VM now → component after B1 (target tracking ALREADY ECS-native) | same | same |

**Cross-cutting edges plan-review must scrutinize:**
1. **membership × observe-replaces** (DOM §4.3.1 step 7, §2.1): re-`observe()`ing an already-observed
   target must NOT create a second membership. D2 is automatic (set dedups by `observer` id; the
   component holds one registration per (observer, target), replaced in place). D1 must branch its
   increment on the replace-vs-add decision (`mutation/mod.rs:280-328`) — a divergence risk. Test:
   `observe(n); observe(n)` then `disconnect()` → collectible (no residual over-count).
2. **membership × transient observers** (DOM §4.2.3 "Mutation algorithms" — remove algorithm step 15 /
   §4.3 notify step 6.3): a transient observation counts as membership while present and is cleared at
   notify step 6.3 (`clear_transient_observers` `mutation/mod.rs:582`). D2: transient entries are in the
   same component ⇒ in the set automatically. D1: must add/subtract at every transient chokepoint. Test:
   an observer whose ONLY live membership is a transient survives while the transient exists, collectible
   after step-6.3 clear (if not otherwise observing / referenced).
3. **seam × sweep-prune ordering** (§4.3): the keepalive **membership mark** runs in `collect_garbage`
   (`:1234-1238`, before `trace_work_list` `:1315` and the sweep `:1388`+); the **binding-row prune**
   runs in the sweep tail (co-located with `vm_event_listeners.retain` `:1746`), keying on the now-final
   `instance` mark bit. Verify the prune is placed **after** every mark is set (mark-roots + keepalive +
   trace) so an observing / JS-referenced instance's bit is set before the retain keys on it — else a
   live observer's row is wrongly pruned.
4. **callback-root duality × partial reference** (parent §8 edge 4): a surviving observer must keep
   **both** ObjectIds; a page holding a ref to only the callback (not the instance) with an active
   observation still survives because the predicate marks BOTH (the observation, not the JS ref, is the
   anchor). Test: observing observer with no JS ref at all → both survive + callback still fires on the
   next delivery.
5. **despawn × D1/D2 discriminator** (§5.2): despawn of the sole observed entity ends the observation
   with NO registry notification (`teardown.rs:133`). D2 passes for free (component gone ⇒ not scanned);
   D1 only passes if a despawn→decrement hook exists (which it does not today). This is the **D1-vs-D2
   discriminator test** (§9 test d).
6. **unbound GC** (§4.2 branch): a GC while `!is_bound()` (post-unbind, pre-rebind) **cannot** read the
   observations — `dom_ptr` is null so the World is UNREADABLE (not "empty"), and `Vm::unbind` does NOT
   despawn the externally-owned World (`host_data.rs:1212-1215`) so the observations PERSIST across
   unbind. **FAIL-SAFE = KEEP ALL observer bindings** (mirroring the MQL arm `keepalive.rs:236-256` + the
   unconditional AbortSignal.timeout mark `:261`). Skipping-to-collect here would prune a
   genuinely-observing no-JS-ref observer's binding → an under-root regression (it could never fire again
   on rebind). The over-keep is TRANSIENT: the next **bound** GC runs the precise predicate and collects
   genuine idles. Test: an OBSERVING but unreferenced observer survives an unbound GC (binding row
   retained) and RESUMES delivery after rebind; separately, an IDLE unreferenced observer is collected at
   the next BOUND GC (§9). (This is the case the earlier "empty set ⇒ collectible" framing MISSED.)
8. **pending-record × despawn** (§2.1 second clause, `/code-review` 2026-07-02): the membership-only
   predicate MISSED that a MutationObserver with a queued-but-undelivered record must survive **even at
   zero observations**. The intersection cell "despawn of sole target → collectible" is REFINED to
   "collectible **ONLY IF no pending records**". A page that observes N, mutates N (queuing a record +
   joining `pending`), then drops the JS ref AND despawns N before the notify microtask: N's
   `MutationObservedBy` vanishes (membership zero), but the record is still queued — dropping the wrapper
   loses it (delivery `take_records` then a missing-binding lookup silently discards → callback never
   fires = data loss). Fix = the OR-ed `observers_with_pending_records` disjunct (registry-only, no World).
   RO/IO are UNAFFECTED (no persistent entry queue; synchronous delivery). Test: §9's pending-record
   regression (observe → queue-record → despawn → GC → survives → deliver → callback fires); negative
   control: a `takeRecords()`-drained observer (empty queue) with no observation IS collected.
7. **step-1-snapshot independence** (§2.5): confirm S5-3c has **no** dependency on / coupling with
   `#11-keepalive-event-loop-step1-snapshot` (WS-only; observers are live-membership-keyed). The absent
   thing is the **"as of the last time the event loop reached step 1" KEEPALIVE-SNAPSHOT tiering**
   language (the WS §7 construct) — not the substrings "step 1" / "event loop" per se: DOM §4.3's body
   has ordinary numbered "step 1" list items (queue/notify algorithms) and intersection-observer §3.4.1 is
   literally titled "HTML Processing Model: Event Loop" (but that IO §3.4.1 is a rendering / Update-the-
   rendering substep, NOT GC-keepalive). Reviewers should NOT block S5-3c on that slot, and the slot
   text's "cross-cutting with S5-3c" clause is struck at landing (§10).

---

## §9 Test strategy (VM-test oracle — boa is the live engine)

S5-3c is exercised by **VM tests** (`elidex-js` `engine`-feature suite) + **engine-indep unit tests**
(`elidex-api-observers`). Test infra: `with_bound_vm(|vm| …)`, `vm.inner.collect_garbage()` to force GC,
the existing observer test helpers (`tests_mutation_observer.rs` etc.). **The oracle is FLIPPED vs
S5-3a/b** (§0.3): the headline asserts an idle observer **IS collected**.

**Engine-indep unit tests (`elidex-api-observers`, pure — the D2 query):**
- `mutation::observing_observer_ids`: empty world → empty set; after `observe(id, n, init)` → `{id.raw()}`;
  after `unobserve`/`disconnect` → absent; **despawn of the sole observed entity** → absent (the
  despawn-safety proof at the unit level); a **transient** observation → present while present, absent
  after `clear_transient_observers`; two observers on distinct targets → both present.
- `resize::observing_observer_ids` / `intersection::observing_observer_ids`: same matrix (observe →
  present; unobserve/disconnect → absent; despawn → absent).

**VM tests (the decisive behavior — the flipped oracle):**
- **(a) HEADLINE (over-root/leak fix, the flip):** `new MutationObserver(cb)` with **no `observe`** and
  **no retained reference** → drop the ref → `collect_garbage()` → assert the observer is **COLLECTED**
  (its `mutation_observer_bindings` row is **pruned**; its `instance`/`callback` ObjectIds are reclaimed).
  Today this LEAKS (immortal until unbind); this test flips it. Repeat for RO + IO.
- **(b) observing survives + still fires:** `let mo = new MutationObserver(cb); mo.observe(el, {childList:true});`
  drop the `mo` JS ref → GC → assert the observer **survives** (row retained), then mutate `el` →
  `deliver_pending_mutation_records()` → assert `cb` fired (the callback stayed rooted via the predicate,
  not a JS ref). Repeat for RO (resize) + IO (intersection) via their `gather_observations` delivery.
- **(c) observe-then-disconnect collectible:** `mo.observe(el, …); mo.disconnect();` drop ref → GC →
  assert **COLLECTED** (disconnect ended the only observation; the fix releases the binding). Repeat
  RO/IO with `unobserve` of the sole target.
- **(d) DESPAWN discriminator (the D1-vs-D2 test):** `mo.observe(el, …)` (sole target), drop the `mo`
  ref, then **despawn `el`** (remove it from the tree so its entity is destroyed) → GC → assert the
  observer is **COLLECTIBLE** (its sole observation vanished with the entity). **D2 passes for free; D1
  passes only if a despawn→decrement hook was added.** Repeat RO/IO. (This is the test that operationally
  distinguishes the two designs — include it regardless of the choice.)
- **(d') PENDING-RECORD × DESPAWN regression (the queued-record data-loss fix, `/code-review` 2026-07-02):**
  `mo.observe(N, {childList:true})`, drop the `mo` JS ref, then **queue a MutationRecord WITHOUT running
  the notify microtask** (`vm.inner.queue_mutation_record(record)` — enqueues into the `records` queue +
  joins `pending`), then **despawn N** (its `MutationObservedBy` vanishes → observation membership drops to
  zero) → `collect_garbage()` → assert the observer **SURVIVES** (binding row retained, on the
  pending-record clause ALONE — no active observation) → `deliver_pending_mutation_records()` → assert the
  callback **FIRES** with the queued record (`records.length === 1`), i.e. the record is **not silently
  dropped**. Without the disjunct the binding row is pruned and the record is lost. **Companion negative
  control:** an observer whose queue the page **drained via `takeRecords()`** (empty `records`) with no
  observation + no JS ref IS **collected** — proving the disjunct keys on non-empty `records`, not stale
  `pending` membership, so a drained observer is not over-kept. (Engine-indep half in `elidex-api-observers`
  `tests_core.rs`: after `notify`, `observers_with_pending_records` contains the id; after `take_records`,
  it does not.) **MO-only** — RO/IO owe no analogous test (no persistent entry queue).
- **(e) UNBOUND-GC keep-all + rebind-resume (the DECISIVE fail-safe test — §4.2 unbound branch):** an
  **OBSERVING** but **UNREFERENCED** observer (`mo.observe(el, …)`, drop the `mo` JS ref, `el` still in
  the tree) → **unbind** the VM → `collect_garbage()` **while unbound** → assert the observer's
  `*_observer_bindings` **row is RETAINED across the unbound GC** (kept by the keep-all-during-unbound
  branch, NOT collected) → **rebind** the same document → mutate `el` → deliver → assert `cb`
  **fires** (delivery RESUMES post-rebind). This is the case the earlier `is_bound()`-skip framing would
  have wrongly pruned (an under-root regression); this test proves the fail-safe. Repeat RO/IO.
- **NEGATIVE CONTROL (no over-collection):** an observer with a **live JS reference** but **zero
  observations** → GC → assert it **SURVIVES** (via its JS root — the predicate is additive, §2.5; RO
  §3.5 / IO §3.3 Lifetime "no scripting references" clause), AND a subsequent `mo.observe(el, …)`
  **works** and delivers (the retained binding row was kept because `instance` was JS-marked, §4.3). This
  guards against the over-root fix over-shooting into under-rooting. **Companion (leak fix still holds):**
  an **IDLE unreferenced** observer, after a **BOUND** GC, IS **collected** (test (a)/(c)) — the unbound
  keep-all (test (e)) is transient and self-corrects at the next bound GC; the two together prove the fix
  neither under-roots (e) nor fails to collect idles (a/c). Note: the unbound window is the case the
  earlier "empty set ⇒ collectible" framing missed.
- **transient membership:** an observer whose only membership is a transient registered observer
  (created via a subtree removal, DOM §4.2.3 "Mutation algorithms" — remove algorithm step 15) survives
  while the transient is present, collectible after the notify step-6.3 clear (if not otherwise observing
  / referenced).
- **binding-row prune correctness:** after (a)/(c), assert the specific `*_observer_bindings` row for the
  collected observer's `observer_id` is **absent** (not just the ObjectIds unrooted) — the required §4.3
  deliverable.

**Out of scope (reviewers should not flag):** PerformanceObserver (no VM `ObjectKind` — reference-only
§5.1); the post-flip observer delivery *producers* themselves (`deliver_pending_mutation_records` /
`gather_observations` are already VM-resident — S5-3c only ensures their observer targets survive to be
delivered to).

---

## §10 Deferred slots + open questions (per-PR cap ≤3)

### Slots
- **`#11-eventtarget-keepalive-registrant-coverage`** (HARD pre-flip gate, S5-3 §10) — S5-3c is the
  **LAST arm; landing it RETIRES this gate**. After S5-3c, all non-Node keepalive registrants (MQL, WS,
  ES, the 3 observers, AbortSignal.timeout membership) are on the seam and no construct-time /
  force-close divergent root remains off it. **S5-3c deliverable: mark this slot CLOSED** in
  `project_open-defer-slots.md` (the S5-6 flip is unblocked on this axis).
- **`#11-eventtarget-keepalive-component-migration`** (B1-gated, S5-3 §10) — **stays B1-gated**. S5-3c
  adds the observer wrapper bindings as new registrants under this *existing* slot; the
  marker-component-on-entity migration is deferred to the B1 program (§6). Not created new; not closed.
- **`#11-mutation-observer-extras`** (EXISTING, `host_data.rs:279`) — S5-3c **partially discharges** its
  "weak-rooting / sweep-time cleanup" clause for the keepalive subset (the §4.3 binding-row sweep prune
  IS that cleanup, delivered). **S5-3c deliverable: reframe** the field doc-comment
  (`host_data.rs:274-280`) — the "never shrunk … accumulate dead entries" leak is FIXED for the collected
  case by the sweep prune; the residual `#11-mutation-observer-extras` scope is the non-keepalive extras
  (primitive→ToObject init parsing, `Symbol.iterator` filter, etc.), not the binding leak.
- **`#11-keepalive-event-loop-step1-snapshot`** (WS-only, reopened by #440) — **S5-3c does NOT touch or
  depend on it** (§2.5). **S5-3c deliverable: strike the slot text's "cross-cutting with the S5-3c
  observer arm keying the same snapshot" clause** — the observers are live-membership-keyed (no step-1
  language in DOM §4.3 / RO / IO), so there is no shared snapshot. The slot stays WS-scoped and
  plan-review-gated for its own slice. (This is a text correction in the slot ledger, not new scope.)
- **(conditional) `#11-observer-keepalive-despawn-decrement-sync`** (NEW — **only if plan-review picks D1
  over the D2 recommendation**): D1's active-observation count stale-highs on entity despawn because
  `teardown.rs:133` has no registry-decrement path (§5.2). If D1 is chosen, carve this slot for the
  despawn→decrement hook (a `teardown.rs` chokepoint that decrements per-observer counts for the
  despawned entity's observations, kept exact across `despawn_subtree` + shadow teardown). **Not created
  if D2 is chosen** (D2 is despawn-safe by construction — no follow-up owed). Per-PR cap respected:
  D2-path carves **0** new slots; D1-path carves **1**.

### Open questions for `/elidex-plan-review`
- **Q1 (the reframe / spine + DIRECTION FLIP):** Confirm the observer keepalive is a **membership
  predicate** ("has ≥1 active observation", DOM §4.3 / RO-IO `[[observationTargets]]`), and that S5-3c is
  an **OVER-root / LEAK fix** (opposite direction to S5-3a/b's under-root fix) — so the headline test
  asserts a never-observed / disconnected unreferenced observer **IS collected** (§0.3, §9a). Lean:
  **yes** (the only form consistent with §2.8 + the no-GC-note membership specs; and the field-doc
  `host_data.rs:279` already names the leak).
- **Q2 (dispatch shape, revises parent §4.2):** Confirm **shape B** — observers are **membership**
  registrants marked directly in `keepalive_survivors` (mirroring `AbortSignal.timeout` at
  `keepalive.rs:261`), NOT a new `KeepaliveClass::Observer` *listener-predicate* enum variant (the
  parent §4.2 shape-A note predates the S5-3a listener-vs-membership split; the module doc
  `keepalive.rs:51-56` already classifies observers as membership). Lean: **shape B** (One-issue-one-way;
  observers' keepalive is not a listener test).
- **Q3 (D1 vs D2 — THE central ECS-native decision):** §5.2 — maintain an incremental active-observation
  **count** per observer (D1, parent §5.2's pick, O(1) but divergeable + needs a despawn hook that does
  not exist), or derive the observing-id **set** once per GC from the live `*ObservedBy` components (D2,
  single linear archetype query, despawn-safe by construction, drift-free, MORE ECS-native)?
  **Recommendation: D2** — it corrects the parent's O(N²) characterization (that assumed a per-observer
  re-scan; D2 is one shared pass), needs no new despawn chokepoint, and reads the canonical component
  truth rather than a divergeable side-store (CLAUDE.md side-store→component rule). Feasibility verified:
  `dom_shared` (`host_data.rs:1317`) gives the immutable `&EcsDom` the pass needs, no borrow conflict on
  the `&VmInner` seam. If plan-review picks D1, carve `#11-observer-keepalive-despawn-decrement-sync`
  (§10). Lean: **D2**.
- **Q4 (binding-row sweep prune):** Confirm the required §4.3 deliverable — the seam must **sweep-prune**
  each `*_observer_bindings` by the instance's mark bit (`retain(|_, b| bit_get(marks, b.instance.0))`,
  parallel to `vm_event_listeners.retain` `collect.rs:1746`), so the binding STRUCT does not leak and no
  stale dangling `instance` ObjectId persists — not merely removing the construct-root. Lean: **yes**
  (removing the root without the prune leaves a struct leak + dangling-ObjectId hazard).
- **Q5 (step-1-snapshot decoupling):** Confirm S5-3c is **INDEPENDENT** of
  `#11-keepalive-event-loop-step1-snapshot` (§2.5 — observers are live-membership-keyed; no step-1
  language in the three observer specs; the JS-root makes transient-zero windows benign), so the slot
  text's "cross-cutting with S5-3c" clause is struck. Lean: **independent** (webref-confirmed absence of
  step-1 language; the step-1 gap is WS-only because only WS §7 keys tiers to the step-1 snapshot).
- **Q6 (atomic 3-kind scope):** Confirm all three observer kinds land in **one** PR (shared
  `ObserverBinding` infra + shared construct-root; leaving any one on the construct-root before S5-6 = a
  partial strangler), a terminal base-case under the umbrella. Lean: **atomic single PR**.
- **Q7 (PerformanceObserver out-of-scope):** Confirm PerformanceObserver (performance-timeline §4, no VM
  `ObjectKind` / registry) is **correctly out of scope** (reference-only, "born into the seam when it
  lands", cf. parent §3 XHR row), so reviewers don't flag its absence. Lean: **out of scope**.

---

## §11 Verified-cites note (read before plan-review)

Spec §/anchors webref-verified 2026-07-02:
- **DOM** §2.8 "Observing event listeners" (`#observing-event-listeners`); §4.3 "Mutation observers"
  (`#mutation-observers`), `registered observer list` (`#registered-observer-list`) / `registered
  observer` (`#registered-observer`); §4.3.1 "Interface MutationObserver" (`#interface-mutationobserver`,
  `observe()` **step 7** = already-observed REPLACE-in-place [7.1 remove transients, 7.2 set options],
  **step 8** = "Otherwise: Append a new registered observer" — verified via `body dom
  dom-mutationobserver-observe`); §4.2.3 "Mutation algorithms" (`#mutation-algorithms`), remove algorithm
  step 15 (transient append); §4.3.2 "Queuing a mutation record" (`#queueing-a-mutation-record`); §4.3.3
  "Interface MutationRecord". **No MutationObserver "garbage collection" dfn in DOM** (the only GC dfn is
  §3.2.1 AbortSignal) — confirmed via `dfn 'garbage collection' dom`. The §4.3 body has ordinary numbered
  "step 1" list items (queue/notify algorithms) but **no "as of the last time the event loop reached step
  1" KEEPALIVE-SNAPSHOT tiering** language (the WS §7 construct).
- **resize-observer-1** **§3.5 "ResizeObserver Lifetime" (`#lifetime`) — the EXPLICIT normative keepalive
  predicate: "A ResizeObserver will remain alive until both of these conditions are met: there are no
  scripting references to the observer; the observer is not observing any targets."** (verified via `body
  resize-observer-1 lifetime`); §3.2.2 "ResizeObserver" (`#resize-observer-slots`); `[[observationTargets]]`
  (`#dom-resizeobserver-observationtargets-slot`) / `[[activeTargets]]`
  (`#dom-resizeobserver-activetargets-slot`). **No separate "garbage collection" section** (`dfn 'garbage
  collection' resize-observer-1` → no hit; keepalive is stated by §3.5, not inferred).
- **intersection-observer** **§3.3 "IntersectionObserver Lifetime" (`#lifetime`) — the EXPLICIT normative
  keepalive predicate: "An IntersectionObserver will remain alive until both of these conditions hold:
  There are no scripting references to the observer; The observer is not observing any targets."**
  (verified via `body intersection-observer lifetime`); §3.1.3 "IntersectionObserver"
  (`#intersection-observer-private-slots`); `[[ObservationTargets]]`
  (`#dom-intersectionobserver-observationtargets-slot`). **No separate "garbage collection" section**
  (`dfn 'garbage collection' intersection-observer` → no hit). Note §3.4.1 is titled "HTML Processing
  Model: Event Loop" but is a rendering / Update-the-rendering substep, NOT a GC-keepalive tiering.
- **performance-timeline** §4 "The PerformanceObserver interface" (`#performanceobserver`) — reference-
  only (not VM-implemented).

Code cites grep-verified against `main` HEAD `113a6c75` (2026-07-02):
- Seam: `keepalive.rs:51-56` (listener-predicate vs membership family doc), `:230` (`keepalive_survivors`
  sig), `:261` (`pending_timeout_signals` direct membership mark = the shape-B model), `:1-42`
  (module doc naming the S5-3c observer arm delegating to `elidex-api-observers`).
- Construct-root + leak: `host_data.rs:1692-1733` (`gc_root_object_ids`, `:1712-1723` the three
  `*_observer_bindings` flat-map), `:281`/`:297`/`:308` (the three binding-map fields), `:274-280` (the
  leak field-doc + `#11-mutation-observer-extras` pointer). `mutation_observer.rs:197-203` (construct-time
  binding insert), `:249-261` (`disconnect` clears observation, NOT binding). `vm_api.rs:685-696`
  (bindings RETAINED on unbind, keyed by monotonic `observer_id`).
- Registries (engine-indep, D2 substrate): `mutation/mod.rs:114` (`MutationObservedBy(Vec<
  MutationObservation>)`), `:95-108` (`MutationObservation` carries `observer: MutationObserverId`),
  `:116-141` (`MutationObserverRegistry` = `records` BTreeMap + `pending` Vec + id allocators — **NO
  observer→nodes reverse index**), `:256` (`observe`), `:186` (`add_transient_observers`), `:280-328`
  (observe replace-vs-add), `:339` (`disconnect` via `retain_observations`), `:561-572`
  (`retain_observations` = `world_mut().query::<(Entity, &mut MutationObservedBy)>()`), `:582`/`:592`/
  `:602` (transient clears). `intersection.rs:80` (`IntersectionObservedBy`), `:71-75`
  (`IntersectionObservation`), `:93-98` (registry = id counter + per-observer `RegisteredObserver`, no
  reverse index), `:150`/`:177`/`:189` (`observe`/`unobserve`/`disconnect`), `:273` (`gather_observations`
  query). `resize.rs:102` (`ResizeObservedBy`), `:90-97` (`ResizeObservation`), `:111-114` (registry = id
  counter + `registered` HashSet, no reverse index), `:133`/`:167`/`:179`
  (`observe`/`unobserve`/`disconnect`), `:226` (`gather_observations` query). Id newtypes `u64` w/
  `raw()`/`from_raw()`: `mutation/mod.rs:17-30`, `resize.rs:23-36`, `intersection.rs:21-34`.
- **Despawn chokepoint (the D1/D2 discriminator):** `crates/core/elidex-ecs/src/dom/tree/teardown.rs`
  `destroy_entity` (`:51-150`, `world.despawn(entity)` at `:133`) + `despawn_subtree` (`:190-241`) make
  **ZERO observer-registry calls** — the `*ObservedBy` components auto-vanish with the entity, **no
  registry decrement**. Module docs `intersection.rs:5-8` / `resize.rs:5-7` state "a despawned entity
  drops its observations automatically". **Confirmed: no despawn→registry-decrement path exists.**
- GC mark/sweep: `collect.rs:25` (`collect_garbage`), `:1233` (`keepalive_survivors` call), `:1234-1238`
  (mark survivors via `mark_object`), `:1315` (`trace_work_list`), `:1388` (sweep), `:1746`
  (`vm_event_listeners.retain(|id,_| bit_get(marks, id.0))` = the row-prune pattern to mirror),
  `:1917`/`:1931` (WS/ES `retain` precedent). `ObjectId(pub(crate) u32)` `value.rs:40-42`; `bit_get`
  `gc/mod.rs:64-67`.
- D2 feasibility: `HostData::dom_shared(&self) -> &EcsDom` (`host_data.rs:1317`, immutable-World accessor
  for `&HostData` read paths, asserts `is_bound`); `is_bound` `:1241`; MQL arm precedent
  `document_entity_opt` `:1593`.
- Unbind-does-NOT-despawn-World (the F1 keep-all basis): `HostData::bind` binds the World by raw pointer
  (`host_data.rs:1174-1209`, `self.dom_ptr = dom`); `HostData::unbind` **only NULLs `dom_ptr`** + bumps
  `bind_epoch` (`:1212-1215`) — it does NOT despawn the externally-owned World. `Vm::unbind` "closes every
  BATCH … not only a navigation" (`vm_api.rs:798-806`), so observations persist across unbind and the
  World is UNREADABLE (not empty) during the unbound window. Fail-safe precedents: MQL keep-all-unbound
  (`keepalive.rs:236-256`), AbortSignal.timeout unconditional mark (`keepalive.rs:261`, no `is_bound`
  guard).

**No cite drift found vs the prompt's stated lines** — the prompt's approximate line numbers all resolved
to the correct symbols at `113a6c75`; exact numbers are pinned above (e.g. `gc_root_object_ids` flat-map
= `host_data.rs:1712-1723` not `:1692-1724`; `keepalive_survivors` = `keepalive.rs:230`; binding fields =
`:281`/`:297`/`:308`). The one **framing correction** vs the parent: parent §5.2's D2-as-O(N²) is
**pessimistic** — D2 is a single shared archetype pass (O(total live observations)), not a per-observer
re-scan (§5.2).

---

## §12 Workflow

plan-verify grep against `113a6c75` (done) → **`/elidex-plan-review` (this memo) BEFORE impl** → impl in
this worktree (order: `elidex-api-observers` `observing_observer_ids` fns + unit tests [D2] → seam
membership mark in `keepalive_survivors` [shape B] + remove the construct-root from `gc_root_object_ids`
→ binding-row sweep prune in `collect.rs` → stale-comment/field-doc reframes → VM tests) → `/pre-push`
(6-stage) → `/external-converge` (Codex) → squash merge. boa untouched (VM-internal). B1 component
migration stays out (`#11-eventtarget-keepalive-component-migration`, deferred).

**Stale-artifact reframes (S5-3c landing deliverables, not side-effects):**
1. `keepalive.rs` module doc — the observer arm is now **landed** (currently `:41-42` "The remaining
   observer arm marshals to `elidex-api-observers` (S5-3c)" → mark done). **NOTE the B1 supersession is
   ALREADY present** at `keepalive.rs:32-36` ("⚠ SUPERSEDED 2026-06-30: world_id retracted → agent-scoped
   `EcsDom` World (PR #434) … under B1 (1-agent=1-World) per-entity identity is stable, so that migration
   becomes safe without a discriminator"); the **only** pre-#434 residual is the single "world_id-gated"
   **adjective at `:31`** — tighten just that one word to "B1-gated" (a one-word touch, NOT a
   reframe-of-pre-#434-wording, which the :32-36 block already did).
2. `KeepaliveClass` doc `keepalive.rs:66-71` — "The remaining non-Node EventTargets migrate … the
   Mutation / Resize / Intersection observers (active-observation membership, S5-3c …)" → mark the
   observer arm landed (note it is a **membership** registrant marked directly, per Q2 shape B, not a
   `KeepaliveClass` variant).
3. `host_data.rs:274-280` field-doc — the "never shrunk … accumulate dead entries … weak-rooting /
   sweep-time cleanup is tracked at `#11-mutation-observer-extras`" leak → reframe: the collected-observer
   binding leak is **FIXED** by the §4.3 sweep prune; residual `#11-mutation-observer-extras` = non-
   keepalive extras only.
4. **Defer-ledger** (`project_open-defer-slots.md`): mark `#11-eventtarget-keepalive-registrant-coverage`
   **CLOSED** (S5-3c = last arm, gate satisfied); strike the "cross-cutting with S5-3c" clause in
   `#11-keepalive-event-loop-step1-snapshot` (§2.5); reframe `#11-eventtarget-listener-keepalive-rooting`
   / `#11-mutation-observer-extras` per above; keep `#11-eventtarget-keepalive-component-migration`
   B1-gated (observers added as registrants). After S5-3c the **whole S5-3 keepalive program is complete
   on the delivered surface** — every non-Node EventTarget keepalive is on the seam.

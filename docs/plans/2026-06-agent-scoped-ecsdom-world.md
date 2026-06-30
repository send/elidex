# Agent-scoped `EcsDom` World — the cross-DOM identity architecture (supersedes world_id)

Anchor = **the ideal end-state** (`feedback_plan-memo-anchor-on-ideal-not-incremental`). The ideal is
**not** "tag entities with a `world_id` so a `Vm` can safely hold entities from two `EcsDom` worlds." The
ideal is the **dual**: make a `Vm` *never hold entities from two worlds* — by scoping the `EcsDom` World
to the **similar-origin window agent** (one World per agent, hosting that agent's many document
subtrees). Then cross-DOM aliasing — the entire motivation for `world_id` — **cannot occur by
construction**, and the `world_id` discriminator (`#11-wrapper-cache-cross-dom-discriminator`) is
**unnecessary → superseded**.

This is a **roadmap-defining architecture decision**, docs-only, landing on its own (analogous to
`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`). It is **not** bundled with impl. After it lands,
S5-3a's (PR #430) deferral comments are **retargeted to cite it** (a separate cite-update step — §6.4).

> **Status (2026-06-30)**: this replaces the earlier draft of this branch, which framed the same slot as
> a *two-facet `world_id` deferral contract* (nav-scrub = S5-6 hard gate + full discriminator after S5).
> A deep design dialogue **reversed that thesis**: agent-scoped World dissolves the problem rather than
> deferring its discriminator. The nav-scrub-as-pre-flip-gate is **retracted** (§6.2). The slot
> `#11-wrapper-cache-cross-dom-discriminator` is **superseded, not deferred**.

---

## §0 Read first — the one-line decision

`EcsDom` World = **one per similar-origin window agent** (the dfn in HTML §8.1.2.1), **site-keyed by
default** (so a *same-site cross-origin* frame/popup is **same-agent**, §1.4 terminology), origin-keyed
when the §8.1.2.2 algorithm keys by origin (an honored `Origin-Agent-Cluster`, cross-origin isolation,
opaque). One World hosts the agent's **many same-agent document subtrees** (the embedder + its same-agent
frames + same-agent popups in the same browsing-context group). Cross-agent (cross-site / sandboxed-opaque
/ a COOP-`noopener` BCG split) content lives in a **separate World + separate `Vm`** (+ OOP for
cross-origin), reached only by value or by a restricted proxy.

**The one line**: *one agent = one World = one `Vm`.* A `Vm` therefore never holds entities from two
Worlds, so the within-`Vm`-cross-World reference — the **exact** state `world_id` exists to discriminate
— **never exists to discriminate**.

### §0.1 Why this is the ideal, stated as a contrast

| | **B2 — world_id discriminator** (the superseded path) | **B1 — agent-scoped World** (this decision) |
|---|---|---|
| World grain | one World **per document** | one World **per agent** (many docs) |
| A `Vm` holds | entities from **N Worlds** | entities from **1 World** |
| Cross-DOM aliasing | **occurs**, managed by a read-time `world_id`-mismatch → dangling check + a per-VM-side-store → per-entity-component migration the check unblocks | **cannot occur** — distinct entities within one World; hecs `generation` already handles staleness |
| New machinery | `EcsDom::world_id: u64` counter + origin-world tag on every retained handle + a mismatch check threaded through **every** entity→wrapper resolve site (the world_id migration memo estimated dozens) + a navigation-scrub | **none** — extends the *existing* within-World multi-document grain |
| First-in-codebase | introduces the **first** within-`Vm` cross-World references | introduces **none** (no cross-World refs anywhere — §1.3) |

B1 is the ideal because it **removes a problem class** instead of **adding a mechanism to police it**
(Ideal-over-pragmatic; One-issue-one-way: the canonical answer to "how do two documents' entities
coexist safely in one script realm" is "they live in one World," not "they live in two Worlds plus a
discriminator").

---

## §1 The decisive invariant — why B1 obviates world_id

### §1.1 The aliasing mechanism is real, and it is **specifically** a two-Worlds artifact

Cross-DOM aliasing exists for one reason: two separate `EcsDom::new()` Worlds **share entity-index
space** *and* each restarts hecs `generation` at 1, so the same `index|generation` bitpattern is
**simultaneously live in both Worlds**. A retained `Entity{index:5,gen:1}` captured in world-A, resolved
after a `Vm::unbind`→`bind` to world-B, lands on a **live-but-different** node in world-B. The aliasing
is the **stale entity bits themselves** (lesson #195, `vm/vm_api.rs`), not a stale pointer — which is
why a component-lookup-on-stale-bits does not save you, and why B2 needs a *read-time* discriminator.

The load-bearing word is **two**. The collision requires two Worlds with overlapping index spaces in
scope of the same resolver. Remove the second World from the `Vm`'s reach and the collision is
unconstructable.

### §1.2 Within ONE World, hecs `generation` already detects use-after-despawn (verified)

The premise B2 rests on — "elidex does not use hecs `generation` for stale detection" — is true *only
across Worlds* (each World restarts the counter). **Within one World it is false**: hecs reliably
detects use-after-despawn.

Verified against `hecs-0.11.0` this session:

- **despawn bumps the generation.** `World::despawn` → `Entities::free` bumps `meta.generation`
  (`entities.rs:384`: a `NonZeroU32::new(u32::from(meta.generation).wrapping_add(1))` cycling `1..=u32::MAX`,
  skipping 0) after asserting the handle's generation matches (`entities.rs:380`).
- **stale handles are rejected.** `contains` / `get` gate on `meta.generation == entity.generation`
  (`entities.rs:412`, `:439`) and reject a mismatch (`:464`).

So in a single World, a parent-held wrapper to a **despawned** child-iframe-document node is correctly
**dangling** — `dom.contains(entity)` returns false, the wrapper resolves to `None` — **without any
`world_id`**. (hecs `Entity::to_bits` packs `(generation as u64) << 32 | id` into a `NonZeroU64`
(`entities.rs:~44`; `generation: NonZeroU32`, `entities.rs:~21`) — no spare bits to carry a `world_id` even
if we wanted one, which independently rules out the "pack world_id into Entity" variant of B2.) Generation
handles the *despawn* hazard; B1 ensures despawn is the *only*
hazard by never admitting a second World's live-but-different bits.

**The temporal dimension (rebind), not just the spatial (reference).** §1.3 establishes that no `Vm` holds
two Worlds' entities *at once* (the spatial hazard). The staleness `bind_epoch` guards in-tree today
(`static_range_proto.rs:371`, `vm_api.rs:574-585`) is the *temporal* sibling: a wrapper retained across a
`Vm::unbind`→`bind` that re-binds a **different** `EcsDom` resolves stale bits generation cannot catch —
the rebind is **not** a despawn (same index, same generation, a different world). **B1 eliminates this too,
by construction**: a `Vm` is bound to exactly one World — its agent's — for its whole life. In the
**current through-S5 pipeline**, *every* navigation is a whole-pipeline replacement (a fresh engine +
`EcsDom`, owned together and never rebound — `pipeline.rs`, a new `JsRuntime` today, the `Vm` post-flip);
under **B1**, **same-agent** navigation *reuses* the agent's World/`Vm` (§5 req 1) and only a **cross-agent**
navigation allocates a new pair — but **either way the live `Vm` never swaps its World for a different
one** (same-agent nav keeps it; cross-agent nav is a *new* `Vm`). A World's document-root membership may
grow/shrink (docs created-in / despawned-from, §5 req 1), but the World identity under a live `Vm` is
stable. So every `unbind`→`bind` re-binds the *same* World; there is no cross-World rebind, and
`bind_epoch`'s cross-world detection role disappears (§5 req 6 / §7 Q7).
The one residual within-World hazard B1 inherits from hecs is **ABA on 32-bit `generation` wraparound** (an
index despawned/respawned 2³²−1 times — the full `NonZeroU32` cycle — recurs at the same bitpattern) —
accepted as out of scope (a retained wrapper does not survive ~4 billion despawns of one index), and
distinct from the cross-World hazard B1 removes.

### §1.3 The construction proof: no `Vm` ever holds two Worlds → no discriminator is needed

> **1 agent = 1 World = 1 `Vm`.** Every entity a `Vm` can resolve belongs to its single World. Two
> documents in the same agent share that one World (distinct entities, generation-checked). Two
> documents in different agents are in different `Vm`s (different processes/threads for cross-origin),
> exchanging only values/proxies — **no entity bits cross**. Therefore the within-`Vm`-cross-World
> reference that `world_id` discriminates **is never created**.

This is not aspirational: it is the **existing** within-World grain, merely extended from the document to
the agent. elidex already runs **multiple document entities in one World**:

- `AssociatedDocument(Entity)` links every node to its owning document (`components.rs:810`); the page
  World already holds a 2nd persistent document for fragment/inert parsing
  (`elidex-form/src/inert_document.rs:110-121` builds into "the `dom`'s existing world… without
  clobbering the page's `document_root`"; covered by `dom/tests/associated_document.rs`).
- `adopt_subtree` re-homes a subtree **within one World** by re-stamping `AssociatedDocument` per node
  (`dom/tree/teardown.rs:285-291`) — and a codebase-wide sweep finds **no primitive that moves an
  entity between two Worlds** at all. B2 would be the *first* code to introduce within-`Vm` cross-World
  references; B1 introduces none.
- "Currently focused area" is already scoped **per-document by a membership filter inside one shared
  World** (`focus/sot.rs:53-56` `current_focus(dom, document)` + the `is_in_document` ancestor-walk,
  `:70-88`), over a per-entity `ElementState::FOCUS` flag (`components.rs:426`) — exactly B1's pattern.

So B1 = the grain elidex **already uses for multi-document**, lifted to the agent boundary. The
multi-root style/layout, focus-per-doc-membership, and `adopt_subtree` machinery are the *same*
mechanisms, given a wider span.

### §1.4 The taxonomy of every cross-frame interaction (the dichotomy that closes it)

Every cross-frame interaction is exactly one of:

- **within-World** — **same agent** (embedder + same-agent frames/popups in the same browsing-context
  group). Shared heap, distinct entities, generation-checked. *Trivial in B1*; *needs `world_id` in B2*.
- **cross-`Vm`** — **different agent** (cross-site, sandboxed→opaque, or a COOP/`noopener` browsing-context-
  group split). Separate World + `Vm` (+ OOP). By-value (`structured-clone`) or restricted proxy; **no
  entity bits cross**. *Identical in B1/B2*.

> **Terminology — *same-agent* (the World boundary) ≠ *same-origin* (the access boundary).** The World is
> the **similar-origin window agent** (HTML §8.1.2.1), **site-keyed by default** (HTML §8.1.2.2). So a
> **same-site cross-origin** frame/popup is **same-agent** — it shares the World + heap (one agent = one
> heap) — even though it is **not** same-origin. What "same-origin" gates is **synchronous DOM access**
> ("friendly" scripting), enforced as an **in-World access check** (cross-origin access is restricted until
> `document.domain` matches, §4.3), **not** a separate World. Conversely a **COOP/`noopener` browsing-
> context-group switch** (HTML §7.1.3.2) puts an otherwise-same-origin window in a **different** agent →
> different World. So World membership = "same agent in the same browsing-context group", which this doc
> writes **same-agent**; reserve **same-origin** for the access check. (The prose below is corrected to
> this; a few "same-origin" usages describing *friendly* access remain deliberate.)

`world_id` is the discriminator for the **first** category's hazard — a hazard B1 makes
**unconstructable** and B2 keeps and polices. §4 walks the full sweep to show no corner escapes this
dichotomy.

---

## §2 B1 vs B2 — why B1, not "world_id done well"

### §2.1 Shared-`Vm` for same-agent iframes is **forced regardless** — that is the hinge

Same-origin "friendly" iframes script each other **synchronously** with **full object identity**
(`iframe.contentWindow.document.body === ` the same node object the child sees; `parent.fn(childNode)`
passes a *live* node, not a copy). Object identity **cannot cross heaps** — a wrapper's identity and a
node's entity bits are only meaningful in the realm/World that owns them. Therefore **same-agent iframes
must share one `Vm` and one heap**. This is not a B1 choice; it is what "friendly iframe" *means*. (It is
also why #412 C0 had to **stub** same-origin friendly-iframe sync scripting — the current
separate-`EcsDom`+separate-`Vm`-per-iframe model, `crates/shell/elidex-shell/src/content/iframe/load.rs:44-46,238-244`
(cited `iframe/load.rs` below), **cannot** express it.)

**One heap, many realms — `Vm` must be multi-realm.** "Shared heap" is **not** "shared realm". An agent
is one heap/event-loop hosting **multiple Window globals/realms** that synchronously access each other
(HTML §8.1.2.1) — each same-agent document has its **own** Window, global scope, and per-realm prototype
chain. So the shared `Vm` must host **N realms** (one per same-agent Window), not collapse them onto a
singleton global. This is **design req 7** (the multi-realm requirement) — without it, folding frames into
one `Vm` would alias `contentWindow` / globals / prototypes / `document` to the parent. It is the dual
boundary to the World: **entities are shared (one World), realms are not (one per Window)**; B1 makes the
*entity* boundary the agent and keeps the *realm* boundary per-Window.

Given that same-agent content shares one `Vm`, the only remaining question is the **World grain inside
that shared `Vm`**:

- **B2**: keep one World *per document* inside the shared `Vm` → the `Vm` now resolves entities from
  several Worlds → reintroduce the cross-World aliasing → need `world_id` to police it.
- **B1**: make the World the *agent* → one World inside the shared `Vm` → no cross-World refs → no
  discriminator.

B1 is the choice that does **not** manufacture the very hazard the shared-`Vm` requirement otherwise
creates.

### §2.2 What B1 costs vs what it avoids

B1 is a deeper change than "world_id if the iframe layer were frozen" — **but the iframe layer is being
built right now** (S5 / FLIP + `#11-windowproxy-browsing-context` /
`docs/plans/2026-06-iframe-browsing-context-plan.md`), friendly-iframe sync is currently **stubbed**
(#412 C0), and friendly iframes **cannot be done correctly with separate `Vm`s** (§2.1). So B1 is not
"extra work to avoid world_id" — it is **"build the iframe layer once, the right way."** Net:

- **Avoids** the entire `world_id` apparatus: the `EcsDom::world_id` counter, the origin-world tag on
  every retained handle, the mismatch check threaded through every entity→wrapper resolve site, and the
  navigation-scrub.
- **Avoids** cross-`Vm` *synchronous* forwarding for same-agent frames (the iframe plan §2's "separate
  `VmInner` + cross-VM forwarding" path, which it applied to same-origin frames) — which could never carry
  object identity anyway.
- **Inverts** the planned iframe direction (same-agent → *shared* World+`Vm`, not separate-`Vm`
  forwarding). This is a genuine reshape of `#11-windowproxy-browsing-context` / the iframe plan §2, owned
  by this decision (§6.3).
- **Folds** `#11-browsing-context-state-ecs-components` (the per-document policy/origin cluster becomes
  per-document-root components in B1 — §5 req 5).

The honest summary: B1 trades "implement and forever maintain a discriminator" for "set the World grain
correctly when the iframe layer is built." Since the iframe layer is being built now, the trade is
favorable and the timing is exactly right.

---

## §3 Spec coverage map (preflight completeness gate)

This is an **architecture decision**, not a web-spec algorithm implementation — so this table is **not** a
step-by-step impl map but the **spec provenance of the document/agent model B1 reshapes**: every spec
surface whose ECS placement this decision determines must appear here. Completeness check per
`feedback_plan-scope-re-evaluation`. All §↔title pairs **webref-verified 2026-06-30** (`html`, `dom`,
`CSP3`, `cssom-view-1`); "Touch" = the existing elidex site the decision moves or reshapes; "Reshaped?" =
does this decision fully specify that surface's ECS grain.

| Spec section | Model element (what B1 places in the ECS) | Grain under B1 | Touch (current elidex site) | Reshaped? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.2 Agents and agent clusters (the *similar-origin window agent* dfn = §8.1.2.1; the *agent cluster key* dfn = §8.1.2.2) | the **agent** = the World identity; an agent = one heap hosting **multiple realms/globals** (§8.1.2.1) | World = one per similar-origin window agent; `Vm` = **multi-realm** (req 7) | **absent today** (no agent/site concept; origin/sandbox per-VM `HostData`, `host_data.rs:184,215`; `Vm` singleton `global_object`) | ✓ | no (engine structure; values from origin/headers) |
| WHATWG HTML §7.1.2 Origin-keyed agent clusters | `Origin-Agent-Cluster` → origin-keyed vs site-keyed, resolved against the BCG *historical agent cluster key map* | World keying chosen at document creation (req 1) | absent today | ✓ | yes (response header) |
| WHATWG HTML §7.3.2.3 Groupings of browsing contexts (browsing-context group + *historical agent cluster key map*) | BCG bounds agent membership; the key map keeps OAC keying consistent within a group | per-document-root entity tagged with its agent/BCG (req 1 / req 5) | absent today | ✓ | yes (headers) |
| WHATWG HTML §7.1.3 Cross-origin opener policies (§7.1.3.2 BCG switch) | COOP / `noopener` → **new** BCG → **different** agent → **separate** World+`Vm` | popup/navigation isolation = World boundary (req 1, §4.3) | absent today (COOP unmodeled) | ✓ | yes (COOP header) |
| WHATWG HTML §7.1.4 Cross-origin embedder policies | COOP **+ COEP** set the BCG's cross-origin-isolation mode → §8.1.2.2 origin-keys when it is non-`none` | a cross-origin-isolated agent → origin-keyed World (req 1) | absent today (no COEP handling in `crates/`) | ✓ | yes (COEP header) |
| WHATWG HTML §7.3.2 Browsing contexts | browsing context ↔ document-root entity; WindowProxy indirection | per-document-root entity in the agent's World; WindowProxy = context indirection (not a raw entity) | iframe = separate `EcsDom`+`Vm` today (`iframe/load.rs:44-46`); WindowProxy null-stub (#412 C0) | ✓ | no (structure) |
| CSSOM-View §5 Extensions to the Document Interface (`elementFromPoint`) | per-document hit-test result stops at the container (does **not** pierce the frame) | DOM-API boundary preserved despite shared World (§4.1) | — (clarifies §5 req 3 scope) | ✓ | yes (coords) |
| WHATWG HTML §7.1.5 Sandboxing (incl. *determine the creation sandboxing flags*, *iframe sandboxing flag set*) | sandbox flags incl. embedder→embeddee union; sandbox→opaque-origin → **own** World | per-document-root component; opaque ⇒ separate World/agent | `apply_sandbox_origin` post-build (`iframe/load.rs:410-424`); embedder→embeddee **union absent** (pre-existing gap, §5 req 2) | ✓ | yes (`sandbox` attr) |
| WHATWG HTML §7.1.7 Policy containers | the policy container is its **own struct** (items per §7.1.7), **distinct** from the origin + sandbox flag set; computed **before** document creation | per-document-root components; computed pre-build (req 2) | **concept absent**; load order = World built first, policy stitched post (`lib.rs:879-916`, `iframe/load.rs:46-52`) | ✓ | yes (headers) |
| WHATWG HTML §7.5.1 Shared document creation infrastructure | "create and initialize a Document object" takes the **creation parameters** (the full §7.5.1 set — policy container, origin, sandbox flags, permissions policy, …) | document-create assigns World by the resulting agent | World-first/policy-post today (must reverse, req 2) | ✓ | no (structure) |
| WHATWG HTML §7.4.2.2 Beginning navigation + §7.4.1.2 Document state | navigation = **deactivate / cache-or-despawn** old doc + create new; a **new World/`Vm` only when the agent key changes** (same-agent nav reuses the World; BFCache keeps the old doc non-fully-active, §7.4.1.2) | docs created-in / deactivated / despawned, **never moved** between Worlds | one-`Vm`-per-navigation (`pipeline.rs`); the flip keeps this | ✓ | yes (URL/load) |
| WHATWG HTML §6.6.2 Data model (focus) | focusable area / focused element scoped per document in one World | per-entity `ElementState::FOCUS` + per-doc membership filter (already B1) | `focus/sot.rs:53-88`, `components.rs:426` | ✓ | yes (Tab/click) |
| WHATWG DOM §4.5 Interface Document (`adopt`) | `adoptNode` / document adoption | within-World re-home (`adopt_subtree`); wired onto `adoptNode` (req 3) | `adopt_subtree` (`dom/tree/teardown.rs:285-291`) | ✓ | yes (`adoptNode`) |
| CSP3 §6.4.2 `frame-ancestors` | ancestor-chain walk + origin **values** (browsing-context structure, not entity crossing) | neutral — context walk + values | partial today (`origin.rs:167-298` parse only) | ✓ | yes (CSP header) |

**Breadth**: spec = 4 (HTML, DOM, CSP3, CSSOM-View), 13 model rows → at the `coverage-map` K≥4 / M<20
soft "SPLIT-RECOMMENDED" line, but a decision doc is **one cohesive artifact by construction** (it exists
to be the single citeable agent/World framing — splitting the agent model across PRs would defeat its
purpose and re-introduce the per-PR re-derivation it exists to kill). The breadth grew because the
spec-fidelity pass (Codex R1) pulled in the COOP / BCG / agent-cluster-keying / multi-realm / DOM-API-
boundary facets the model genuinely spans — these are **one model**, not separable concerns. **No split
owed** (positive appeal per `feedback_ideal-over-pragmatic`).

**Trust boundary (user-input audit)**: *this docs-only decision* changes no parse/eval/marshal path. But
it does **flag a new trust boundary the B1 implementation must review**: B1's World assignment depends on
parsing **new response headers *before* document creation** — the headers that feed the agent-keying +
cross-origin-isolation decision (`Origin-Agent-Cluster`, COOP, **COEP** — `crossOriginIsolated` needs both
COOP *and* COEP — and whatever else §8.1.2.2 / §7.3.2.3 consult; the **authoritative input set is the
spec's**, not this list). These are **absent today** (the §3 rows mark them "absent / unmodeled"; no
`Origin-Agent-Cluster` / COEP handling exists in `crates/`), so they are **new header inputs** (not
pre-existing), and the B1 impl PR owes a **security / data-flow review** of that whole new header-parse
surface (secure-context gating, BCG-key consistency, opaque / cross-origin-isolated forcing — §5 req 1).
The `sandbox` attribute and CSP *are* already consumed (existing surface). The decision's own safety claim
is *structural* (the cross-DOM aliasing class becomes unconstructable; cross-agent content is forced into a
separate `Vm`/process), strengthening the boundary — but that does **not** excuse the new header-parse
surface from impl-time review.

---

## §4 The cross-frame sweep — no corner breaks B1

Comprehensive walk (per CLAUDE.md "trust boundary: enumerate upfront"): every cross-frame surface, classed
by whether a DOM **entity** crosses a frame boundary. The thesis holds iff every surface is either
*within-World* (B1-trivial), *neutral* (no entity crossing), or *B1-absorbed* (handled by
agent-granularity + dynamic membership + policy-first ordering).

### §4.1 Category-3 — same-agent cross-document **raw-node** reference (B1-trivial / B2-needs-world_id)

These pass a *live node from another document* — the surfaces that motivated `world_id`. In B1 they are
**all** within one World, reusing existing mechanisms:

| Surface | B1 mechanism |
|---|---|
| `iframe.contentDocument.*`, `contentWindow.document` | the embedded same-agent doc is **another `AssociatedDocument` in the same World** |
| `adoptNode` / `importNode`, `node.ownerDocument` | `adopt_subtree` (within-World re-home) — already exists |
| `getComputedStyle(childDoc node)` | multi-root style over the agent's document subtrees |
| **internal** hit-test / event routing descending into iframes | the internal walk descends into same-World iframe subtrees |
| Intersection / Resize / Mutation observers of cross-doc nodes | observer config holds same-World entities (generation-checked) |
| focus chain + sequential nav across same-agent frames | `focus/sot.rs` per-doc membership, extended to span the agent's docs (req 3) |

> **⚠ `Document.elementFromPoint` is NOT in this category — it stops at the container.** Per CSSOM-View §5,
> `elementFromPoint(x,y)` hit-tests boxes in **that document's** viewport, so a point over a nested
> browsing context resolves to the **iframe/container element in the parent document**, not an inner
> child-document node (it does **not** pierce the frame boundary). The shared World must **not** make this
> DOM API descend — only the **internal** hit-test (event routing, above) descends. Same-World sharing is
> an *entity-coexistence* property, not a license to flatten per-document API boundaries (cf. §4.3 BFCache,
> §5 req 3).

### §4.2 Neutral — **no DOM entity crosses** (identical in B1 and B2)

| Surface | Why neutral |
|---|---|
| `postMessage`, `BroadcastChannel`, `MessageChannel` | structured-clone **by value**; DOM nodes are **uncloneable** — no entity crosses |
| event bubbling | intra-document; "1 World ≠ 1 tree" — same-World trees stay topologically disjoint |
| Workers | a **separate agent, no DOM** → `world_id` is **non-applicable** (corrects the world_id memo's "Workers reason" for world_id — workers never hold DOM entities at all) |
| `SharedArrayBuffer` | shared over an **agent *cluster*** (HTML §8.1.2.2), which spans multiple agents incl. dedicated workers — so it is *cluster*-scoped, **not** a World=agent reinforcement (workers have no DOM entities; the cluster ⊋ the window agent) |
| Range / Selection | cannot span documents by spec — no cross-doc entity pair |
| drag-drop `DataTransfer`, event-loop / microtask, storage / IDB | values / per-origin backends; no entity crossing |

### §4.3 B1-absorbed — agent-granularity + dynamic membership + policy-first ordering

| Surface | How B1 absorbs it |
|---|---|
| `window.open` / `opener` (popup) | a popup joins the opener's World **only if it stays in the opener's browsing-context group AND same agent** — a **COOP / `noopener` BCG switch** (HTML §7.1.3.2) puts it in a **new** group → **separate** World+`Vm` (must not share heap across a COOP isolation boundary). Same-group same-agent popup = dynamic membership (*transition → §7 Q2*) |
| `document.domain` | an **in-World origin-field relaxation** (HTML §7.1.1.2) — it changes the effective origin for the same-origin access check; it does **NOT** reshape the agent-cluster key or change World membership (the windows were **already** same-agent, §1.4). So it is **not** a membership transition (not a §7 Q2 concern) — just an origin/access-check field update. Absent today anyway |
| BFCache | **per-document(-navigable) lifecycle inside the World**, NOT a whole-agent-World freeze — the World can hold still-active opener/sibling/parent documents that must keep running; cache/restore the **bf-cached document subtree**, leaving co-resident active documents untouched (req 7 multi-realm makes per-Window suspension expressible) |
| `Origin-Agent-Cluster` | World **keying decision at document creation** via the agent-cluster keying rule (req 1) |
| sandbox → opaque origin | the sandboxed doc gets its **own** World/agent (req 2) |

**Structural absorption — the dichotomy holds whether a row stays or leaves.** The §1 dichotomy holds
**either way**: a row whose content is *same-agent* (the `document.domain` flip, a same-group same-agent
popup) stays in **one** World (no cross-World reference); a row whose content is *cross-agent* (a **COOP /
`noopener` BCG-split popup**, a **sandbox→opaque** document) **deliberately goes to a separate World+`Vm`**
— that is the cross-agent boundary across which **no entity crosses** (by-value/proxy), exactly the §1.4
"cross-`Vm`" leg. So "absorbed" does **not** mean "kept in the opener's World"; for the cross-agent rows it
means "correctly allocated a *new* World." The one genuinely *open* piece is the **dynamic
membership-transition mechanism**: a same-agent popup *joining* the opener's World at runtime, or a COOP
split *spawning* a new World mid-session, mutates a World's document-root membership — and what happens to
a retained wrapper *across* that transition is open design question **§7 Q2**. (`document.domain` is **not**
a membership transition — it is an in-World origin-field relaxation, the windows were already same-agent.)
The static "1 agent = 1 World" proof covers the steady state; the membership-*transition* invariant is the
open part.

### §4.4 WindowProxy / context indirection (structurally the same in B1 and B2)

`window.parent` / `top` / `frames` / `opener`, `event.source`, CSP `frame-ancestors` ancestor walk —
all operate over **browsing-context indirection + origin values**, never a raw cross-document entity.
B1 does not change their shape; they were never the `world_id` hazard.

**Sweep verdict**: every surface lands in §4.1–§4.4. The only category that ever carried a raw
cross-document entity (§4.1) is within-World in B1. **No corner requires a within-`Vm` cross-World
reference.** ∎

---

## §5 B1's design requirements (the decision's substance)

These are the obligations the B1 implementation (post-S5, when the iframe layer is built for real) must
meet. They are the deliverable — the contract is "build to these."

> **Altitude note (req 1–2) — decision-level only, no spec enumeration.** This is a **decision** doc: it
> fixes *which agent/document fact the World keys on* and the *direction* of the document-creation order,
> and **cites the governing HTML algorithm** — but it deliberately does **NOT** enumerate the exact field
> lists, parameter sets, or keying triggers (per §3 "not a step-by-step impl map"). The complete and
> precise versions — the *obtain a similar-origin window agent* algorithm and all its origin-keying
> triggers (§8.1.2.2), the full policy-container item list and clone/history rules (§7.1.7), the complete
> set of document **creation parameters** (§7.5.1), the document-state lifecycle (§7.4.1.2) — are the **B1
> implementation plan-memo's** job (with its own §3 spec-coverage-map + plan-review). Where this doc names
> example items it is **illustrative, read the cited section for the authoritative set.** (This altitude
> discipline is what keeps the decision from drifting into — and mis-reproducing — impl detail.)

1. **World = similar-origin window agent** (HTML §8.1.2.1), assigned by **HTML's *obtain a similar-origin
   window agent* algorithm** (HTML §8.1.2.2) — **not** a naive "site default + current OAC header"
   shortcut. The decision-level requirements the impl must honor:
   - **site-keyed by default**; the algorithm origin-keys for **several** reasons (an honored
     `Origin-Agent-Cluster` opt-in §7.1.2, the browsing-context group's cross-origin-isolation mode, an
     opaque origin, …) — the impl must take the **full trigger set + secure-context/BCG-consistency rules
     from §8.1.2.2**, not treat OAC as the sole trigger or do a per-response re-key.
   - membership is **browsing-context-group-scoped**: a **COOP / `noopener` BCG switch** (§7.1.3.2) moves a
     window into a **new** group → a **different** agent → a **separate** World (do not share a heap across
     a COOP isolation boundary — §4.3 popup row).
   - **same-site cross-origin** windows in the same group are **same-agent** → **same World** (§1.4
     terminology), with cross-origin DOM access gated by the **in-World access check**; `document.domain`
     is an in-World *origin-field relaxation* (§7.1.1.2), **not** a World-membership change (the windows
     were already same-agent — §4.3).

   **Dynamic membership**: a World spans top-level contexts (opener + same-group same-agent popups) and
   **non-contiguous** same-agent frames; a tab may host several Worlds (one per agent present). Documents
   are **created-in / deactivated / despawned** within a World and **never moved between Worlds**. A new
   World/`Vm` is allocated **only when the agent key changes** — same-agent navigation reuses the World;
   a navigated-away document is **cached-or-despawned**, not unconditionally torn down (BFCache keeps it
   non-fully-active for reactivation, §7.4.1.2; co-resident same-agent opener/sibling docs keep running).

2. **Creation-parameters-first ordering (reverses today's).** Compute the document's **creation parameters**
   (this doc's umbrella term — HTML has no `creation parameters` dfn; the spec bundle is the *navigation
   params* §7.4.2.1 the §7.5.1 algorithm consumes)
   — the full set HTML §7.5.1 "create and initialize a Document object" takes (the **policy container**
   §7.1.7, the **origin** incl. sandbox→opaque, the **sandboxing flag set** incl. the embedder→embeddee
   §7.1.5 union [**currently ABSENT** — a pre-existing gap to fix], the **permissions policy**, …; the
   *authoritative list* is §7.5.1, and the **policy container is its own struct** [§7.1.7] distinct from the
   origin and sandbox fields) — **before** creating the document, assign the World by the resulting agent,
   then build the document into it. Current order (World built first, policy stitched post-build:
   `lib.rs:879-916`, `iframe/load.rs:46-52,410-424`) is **backwards** for B1, because a sandboxed-opaque
   document belongs in a *different* World than its embedder. B1 is the motivation to build the
   creation-parameters abstraction elidex lacks today. (Exact field list + semantics + clone/history rules
   → B1 plan-memo, per the altitude note.)

3. **Extend the existing within-World mechanisms to span the agent's document subtrees**:
   `AssociatedDocument` multi-doc, focus-per-doc-membership (`focus/sot.rs`), `adopt_subtree`, multi-root
   style/layout. Wire `adoptNode` (DOM §4.5) onto `adopt_subtree`; extend focus sequential-nav, hit-test,
   `getComputedStyle`, and observer descent to reach into same-World iframe subtrees.

4. **`WindowProxy` has two modes — by *agent* boundary, not *origin* boundary.** `postMessage` stays
   by-value. A restricted `WindowProxy` (cross-origin property allowlist) is needed for **both**
   cross-origin *same-agent* and cross-agent access — but the mechanism differs:
   - **same-agent cross-origin** (same-site, same BCG): an **in-`Vm` restricted proxy** — the target Window
     is a *realm in the same heap* (req 7), so the proxy is a same-`Vm` access-checked view, **not** cross-
     `Vm` forwarding (and `document.domain` can later relax it in-`Vm`);
   - **cross-agent** (cross-site / different BCG / sandboxed-opaque): a **cross-`Vm` forwarding proxy** (no
     entity crossing) to the separate World+`Vm` (+ OOP).

   **Same-agent friendly access REQUIRES the shared heap** (§2.1) → collapse same-agent iframes from the
   current separate-`EcsDom`+separate-`Vm` (`iframe/load.rs`) into the **one shared World + one `Vm`** — the
   `Vm` hosting **multiple realms** (req 7), not a single global.

5. **Per-document-state cluster → per-document-root components.** the **creation parameters** (req 2's full
   §7.5.1 set — origin, sandboxing flags, policy container, permissions policy, URL, referrer, …) become
   **per-entity components on each document-root entity**, replacing the interim per-VM `HostData`
   (`host_data.rs:184,215`, whose own comments already name the per-entity-component target). **This
   subsumes `#11-browsing-context-state-ecs-components`.**

6. **`world_id` (`#11-wrapper-cache-cross-dom-discriminator`) is NOT built → supersede.** The
   wrapper-identity-component migration (`wrapper_store` → a per-entity `WrapperRefs` component) and the
   keepalive-component migration (`#11-eventtarget-keepalive-component-migration`) become **safe without
   any discriminator** — but *because B1 dissolves the precondition*, **not** by the generic "`Send+Sync`
   per-entity ⇒ component" rule. `wrapper_store` holds per-VM JS-wrapper `ObjectId`s (`wrapper_intern.rs`:
   "NOT an ECS component — aliases across DOMs if hosted on the entity") = CLAUDE.md **exception (a) per-VM
   identity handle**; exception (a)'s hazard is **exactly** the cross-DOM aliasing B1 makes unconstructable
   (§1.1–§1.3: no cross-World reference, and §1.2 no cross-World *rebind*). With the aliasing precondition
   gone, the handle is a per-entity fact like any other → component-eligible, and within one World hecs
   `generation` + liveness cover all staleness (§1.2). `bind_epoch` loses its cross-world role; it remains
   only as `StaticRange`'s within-World freshness check, or folds into ordinary generation/liveness checks
   (§7 Q7) — it is **not** generalized into a `world_id`-style cross-World discriminator.
   **⚠ But the wrapper is per-*realm*, not per-entity (req 7).** With multiple realms in one `Vm`, one
   entity can be exposed through several Windows, and Web IDL platform objects are **realm-associated**
   (each realm has its own `Node.prototype` etc.) — so `WrapperRefs` must be keyed by **(entity, realm)**,
   not a single unqualified `ObjectId` per entity, or cross-frame access returns a wrapper with the wrong
   prototype/realm. The component is `entity → { realm → ObjectId }` (the *entity* is the shared-World fact
   B1 unblocks; the *realm* discriminator is orthogonal to `world_id` — it is an explicit first-class
   concept, not an implicit index-space collision, so it needs no cross-World machinery).

7. **The `Vm` is multi-realm — one heap, N Window globals/realms.** An agent is one heap hosting
   **multiple realms** that synchronously access each other (HTML §8.1.2.1); each same-agent document has
   its **own** Window, global scope, and per-realm prototype chain — **except** for the spec's
   **Window-reuse cases**, which keep the *same* Window/realm across a document swap: most importantly the
   **initial `about:blank`** document and the first same-origin document that replaces it **share one
   Window** (HTML §7.5.1), so `contentWindow` identity + expandos survive that initial navigation. The
   model is therefore "**realm ≈ per Window, with spec-defined Window-reuse**" — the exact reuse rules are
   the B1 plan-memo's (per the altitude note). The current `Vm` (singleton `global_object` + singleton
   prototype slots) must generalize to **N realms** (one per Window, modulo reuse), with: per-realm
   globals/prototypes; per-`(entity, realm)` wrapper identity (req 6); `iframe.contentWindow` resolving to
   the **child's** realm/global (not aliasing the parent); per-document/per-Window lifecycle (so BFCache
   suspends one Window without freezing the agent, §4.3).
   This is the dual of the World boundary: **entities shared (one World), realms not (one per Window)** —
   B1 makes the *entity* boundary the agent and keeps the *realm* boundary per-Window. (This is the
   foundational requirement that makes "shared `Vm`" correct rather than parent-aliasing; it does **not**
   reintroduce a `world_id`-class hazard — realms are explicit, not an index-space artifact.)

---

## §6 Honest cost + roadmap impact

### §6.1 What this supersedes / folds / reshapes

- **Supersedes** `#11-wrapper-cache-cross-dom-discriminator` and the world_id program memo
  (`project_world-id-cross-dom-migration.md` — its "world_id is genuinely needed" was contingent on
  *multiple EcsDom Worlds per `Vm`*, which B1 avoids).
- **Folds** `#11-browsing-context-state-ecs-components` (req 5).
- **Reshapes** `#11-windowproxy-browsing-context` / `docs/plans/2026-06-iframe-browsing-context-plan.md`
  §2 (same-origin: shared World+`Vm`, not separate-`Vm` cross-VM forwarding — §6.3).
- **Makes safe-without-world_id** `#11-eventtarget-keepalive-component-migration` and
  `#11-wrapper-identity-component-migration` (the `wrapper_store`→`WrapperRefs` component migration, req 6).

**Slot-ledger disposition (record at landing).** This decision touches **4 numbered `#11-` slots + 2
non-numbered items** — supersede ×2 (`#11-wrapper-cache-cross-dom-discriminator` + the non-numbered
world_id program memo), fold ×1 (`#11-browsing-context-state-ecs-components`), reshape ×1
(`#11-windowproxy-browsing-context`), make-safe ×2 (`#11-eventtarget-keepalive-component-migration` +
`#11-wrapper-identity-component-migration`). The landing-memo / defer-ledger pass
(`project_open-defer-slots.md`) **must** record this disposition + the net cap delta (this removes/folds ≥2
open slots) — the disposition itself (supersede = keep as pointer for history, not delete; annotate the
world_id program memo as superseded, not silently abandoned) is settled by §7 Q6.

### §6.2 The nav-scrub-as-pre-flip-gate is **RETRACTED**

The S5-2-era broadening that made "nav-scrub = S5-6 hard pre-flip gate" (and the earlier draft of this
branch that codified it) is **withdrawn**. Reasoning: the flip keeps **one `Vm` per navigation**
(navigation = a new `Vm`, `pipeline.rs`), so the **flip is cross-DOM-neutral** — it does not introduce a
within-`Vm` second World. Cross-DOM aliasing becomes reachable **only via friendly iframes**, which are
**stubbed until post-S5** (#412 C0). Therefore **near-term (through S5, iframes stubbed) one-doc-one-World
holds**: no aliasing in production, no nav-scrub, no pre-flip world_id gate. The S5-6 flip is **not**
gated on any world_id-adjacent work. (This is the single most important correction over the earlier
draft: the problem the nav-scrub was guarding does not manifest at the flip.)

### §6.3 Roadmap sequencing

- **Through S5 (incl. the flip)**: nothing here is on the critical path. One-doc-one-World is the
  production reality; the MQL keepalive cross-`EcsDom` residual (S5-3a R6) is **test-only / non-production**
  (§6.4). Land this decision docs-only; cite it.
- **The B1 implementation lands with the friendly-iframe layer (post-S5)** — i.e. when #412 C0's stub is
  replaced. That is the first point a second same-agent document shares a `Vm`, and B1 is *how* it does so
  correctly. So B1 is not a separate "world_id program after S5"; it **is** the friendly-iframe / browsing-
  context buildout, done with the correct World grain. The iframe-plan §2 reshape (§6.1) is its umbrella.
- **Honest deeper-change caveat**: B1 touches document-creation ordering (req 2, policy-container-first),
  the iframe in-process model (req 4, collapse to shared `Vm`), the per-document-state cluster (req 5), and
  — the largest single item — generalizing the **singleton-global `Vm` to multi-realm** (req 7), with the
  full agent-cluster / BCG / COOP keying (req 1). These are real and larger than dropping a `world_id`
  counter in. **But they are the same work the friendly-iframe layer requires anyway** — multi-realm in
  particular is *unavoidable* for friendly iframes under **either** B1 or B2 (a shared `Vm` with one global
  aliases all frames regardless of World grain), so it is **not** a B1 cost. And B2 would need *all of
  req 1–5, 7 too* **plus** the `world_id` discriminator. B1 is strictly less total machinery for a strictly
  cleaner invariant.

### §6.4 S5-3a (PR #430) cite-update (separate step, owed after this lands)

S5-3a is code-converged (HEAD `4501b2b5`, Codex R1→R6; the **R6 thread `PRRT_kwDORYj7cc6M7gpt`** —
"prevent MQL keepalive across unknown rebinds" — is the deferred cross-world horn). Its deferral comments
currently cite "`#11-wrapper-cache-cross-dom-discriminator`, strictly AFTER S5." **⚠ Split by tree**: the
**S5-3a-*new* symbols** — `keepalive_worthy` / `deliverable_to` docstrings, `vm/gc/keepalive.rs`, the
`KeepaliveClass` doc, `tests/tests_match_media_keepalive.rs` — live on the *unmerged* `s5-3a-keepalive-seam`
branch @ `4501b2b5` (absent from `main`; names may shift before S5-3a merges → re-discover at cite-update
time). The **general `world_id` slot cites already on `main`** (`vm/host_data.rs`, `vm/host/media_query.rs`,
…) are the on-`main` code comments **§6.5 has already forward-pointed in this PR** (the complete sweep) —
their full *rewrite* still rides B1; §6.4 owns the unmerged S5-3a-new symbols, §6.5 owns the on-`main` set.
After S5-3a merges, **retarget its symbols to cite THIS doc** with the corrected framing:

- cross-DOM aliasing is **non-production near-term** (one-doc-one-World holds through S5; the flip is
  cross-DOM-neutral, §6.2);
- the **long-term resolution is the agent-scoped World (B1), NOT a world_id discriminator** — when
  friendly iframes land, the MQL registry's entities are same-World and generation-checked, so the R6
  horn **dissolves** (it required two Worlds in one `Vm`);
- the MQL keepalive cross-`EcsDom` residual is therefore **test-only** (only a synthetic two-`EcsDom`
  rebind exercises it). **Resolve the R6 thread** citing this decision.
- the earlier "nav-scrub = S5-6 hard gate" retarget is itself superseded by §6.2 (no such gate).

### §6.5 Umbrella + iframe-plan reconciliation (forward-pointer folded in; full rewrite deferred with trigger)

**Five SSoT surfaces** — **three planning docs** (S5 umbrella, iframe plan, philosophy umbrella) +
**`CLAUDE.md`** + the **review-axis SSoT `.claude/skills/elidex-review/axes.md`** — encode the
now-superseded slot, and **after this lands they would actively contradict it** — a strangler/decision-tax
mid-state One-issue-one-way forbids. Found by a repo-wide grep audit (Codex R2
surfaced the philosophy-umbrella + `CLAUDE.md`; R9 the `axes.md` review-SSoT — the fix-scope sibling
sweep, finally run **whole-repo** to be exhaustive):

- **`CLAUDE.md`** (the SSoT `AGENTS.md` points reviewers to) — the "Side-store→component 判定ルール"
  exception (a) names the **`world_id` discriminator as the unlock condition** for the wrapper-side-store →
  component migration. Left as-is, future reviews enforce the obsolete world_id path.
- **`.claude/skills/elidex-review/axes.md:81`** (the review-axis SSoT `AGENTS.md` tells reviews to apply) —
  same exception (a) text names `world_id` as the unlock; a future B1 / wrapper-component-migration review
  would keep enforcing the superseded prerequisite.
- **S5 umbrella** (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`) — "world_id strictly AFTER S5"
  (§0, §9 keystone row, Q4).
- **iframe plan** (`docs/plans/2026-06-iframe-browsing-context-plan.md`) — §2 "separate `VmInner` + cross-VM
  forwarding" for same-origin, which §2.1/§6.3 of this decision **invert** (same-origin = shared World+`Vm`).
- **philosophy-alignment umbrella** (`docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md`) — F4/C1+
  gated on "C0 + `world_id` program + S5/boa removal".

**Resolution (this PR, minimal):** a short **`⚠ SUPERSEDED` forward-pointer** (one inline line on
`CLAUDE.md`/`axes.md`, a short block-quote on the planning docs — wording tailored per surface) is folded
into **all five SSoT surfaces (3 docs + `CLAUDE.md` + `axes.md`)** at this decision's landing — "⚠ world_id-related framing here is SUPERSEDED by
`docs/plans/2026-06-agent-scoped-ecsdom-world.md` (§6)" — so the contradiction window is closed atomically
(cross-doc consistency in one commit). (Four further docs — `web-api-compat-a2-storage-demotion`,
`web-api-compat-split-design`, `shell-viewport-delivery-pr-c2`, `s5-2-window-parity` — mention "the
world_id program" only as a *future-program label* in a deferral trigger, not as a live current path; they
get the program-name update [world_id → agent-scoped World] at the B1-impl propagation, not a contradiction
pointer now.)

**On-`main` code comments — the COMPLETE forward-pointer sweep is done in this PR.** A repo grep finds
the `world_id` discriminator / `#11-wrapper-cache-cross-dom-discriminator` /
`#11-browsing-context-state-ecs-components` / nav-scrub-`S5-6`-gate cited across **~26 in-tree code-comment
blocks in 16 files (+91 comment-only lines, 0 deletions)** (`vm_api.rs`, `host_data.rs`, `wrapper_intern.rs`,
`mod.rs`, `gc/collect.rs`,
`host/{screen,visual_viewport,media_query,window,html_iframe_proto,html_dialog_proto,navigation}.rs`,
`api/elidex-api-observers/src/intersection.rs`, and 3 test files). The earlier draft tried to split these
into "load-bearing (point now) vs incidental (defer)" — but that per-comment judgment proved **unreliable**
(the Codex loop kept finding "incidental" ones that were in fact active instructions a future editor
follows, e.g. the per-document-state cluster comment and the nav-scrub-`S5-6`-gate comments that directly
contradict §6.2). Per One-issue-one-way / §6.5's "close atomically", **this PR adds a terse,
per-disposition `⚠ SUPERSEDED 2026-06-30 → agent-scoped World (§6)` forward-pointer to *every* such block**
(three wordings — world_id-retracted / slot-FOLDED / nav-scrub-RETRACTED; comment-only, +91 lines, no
code/logic touched — it does make the PR touch `crates/`, accepted because the alternative ships live
obsolete guidance). The forward-POINTER (all sites) is in this PR; the full comment
**REWRITE** (rephrasing the now-superseded rationale, removing `world_id`) still rides the B1 implementation
that removes `world_id`.

**Deferred (PM, trigger = the friendly-iframe / B1 implementation PR, post-S5):** the *full*
rewrite — rewriting the umbrella §9 keystone row, inverting the iframe-plan §2 design prose, and the
code-comment full-rewrite (the pointers already land here) — is design-affecting (iframe-plan §2 is itself
plan-review-grade) and lands with the implementation, not in this decision. The forward-pointers
hold the invariant in the meantime; the rewrite is not silently abandoned (it has a named trigger).

---

## §7 Open questions for /elidex-plan-review

1. **World keying default — site vs origin.** Default site-keyed (similar-origin window agent) with
   origin-keyed opt-in via `Origin-Agent-Cluster` (§5 req 1) matches the spec. Confirm elidex adopts the
   spec default rather than always-origin-keyed (which would be simpler but would break `document.domain`
   and same-site-cross-origin friendly access). Recommend: spec default (site-keyed).

2. **Dynamic World membership mechanism** (the §4.3 transition invariant). A World spans non-contiguous
   same-agent frames + same-group same-agent popups, joined/left dynamically (popup open, navigation, a
   **COOP/`noopener` BCG switch** spawning a new World — §5 req 1; **not** `document.domain`, which is only
   an in-World origin-field flip). What owns membership?
   **ECS-native idiom translation**: an OO membership-*registry* → a per-document-root **`agent-id` /
   `bcg-id` component** queried into the membership set — the same shape as focus-per-doc-membership
   (`focus/sot.rs:53-88`'s `is_in_document` ancestor-walk). Lean: component-on-entity (per ECS-native +
   req 5). The **transition invariant** from §4.3 — a retained wrapper's validity across a membership
   change (popup join, a COOP split spawning a *new* World) — must be specified *with* the chosen mechanism
   (this is the one §4 corner the static proof does not cover).

3. **Policy-container-first ordering rollout (§5 req 2).** Reversing "World-first / policy-post" to
   "policy-first / build-into-the-right-World" is a load-order change touching `lib.rs` /
   `iframe/load.rs`. Is the policy-container abstraction built as a **prereq split** (CLAUDE.md
   touch-time-split / edge-dense) ahead of the iframe collapse, or within it? (Leans prereq split — it is
   a real cohesion seam and the embedder→embeddee sandbox-union gap fix rides it.)

4. **Cross-origin `WindowProxy` forwarding (§5 req 4).** B1 keeps cross-`Vm` proxy forwarding for
   cross-origin `window.parent`/`top`. Confirm this stays the restricted by-value/proxy surface (no entity
   crossing) and that it composes with the iframe-plan §2 reshape rather than re-deriving it.

5. **Sibling-family membership.** Confirm `#11-eventtarget-keepalive-component-migration` (carved by
   S5-3a) and `#11-browsing-context-state-ecs-components` are correctly **subsumed by / folded into** this
   decision (req 5, req 6) rather than surviving as independent slots.

6. **Supersede vs retire bookkeeping.** Confirm the right disposition of
   `#11-wrapper-cache-cross-dom-discriminator`: mark **superseded by this decision** (kept as a pointer for
   history) vs deleted outright. And confirm the world_id program memo
   (`project_world-id-cross-dom-migration.md`) is annotated as superseded, not silently abandoned.

7. **`bind_epoch` disposition (§5 req 6).** Keep `bind_epoch` as `StaticRange`'s within-World freshness
   check, or fold it into ordinary generation/liveness checks once the World grain guarantees uniqueness?
   (Either is sound under B1; this is a cleanup judgment, not a correctness fork.)

8. **Multi-realm `Vm` rollout (§5 req 7) — the largest impl fork.** The `Vm` must go from a singleton
   `global_object` + singleton prototype slots to **N realms** (one per same-agent Window): per-realm
   globals/prototypes, per-`(entity, realm)` wrapper identity (req 6), `contentWindow` → child realm,
   per-Window lifecycle (BFCache suspend-one). Is this built as a **prereq split** (a multi-realm `Vm`
   refactor landing before the iframe collapse) or within the friendly-iframe PR? (Leans prereq split — it
   is a large cohesion seam, is needed by friendly iframes under B1 *or* B2, and gates req 3/4/6.) Confirm
   the realm discriminator is modeled as an **explicit first-class concept** (not packed into `Entity`,
   which has no spare bits — §1.2) so it shares nothing with the superseded `world_id`. **2b write-path
   obligation handed to the B1 plan-memo**: req 6's `WrapperRefs: entity → { realm → ObjectId }` adds a
   *realm* axis on top of the entity axis, so the B1 plan owes the realm-slot **write/cleanup
   reconciliation** — who populates a `(entity, realm)` slot (realm/Window creation) and who drops it
   (realm despawn → drop that slot; entity despawn → whole-row drop via generation/liveness) — exactly as
   §7 Q2 owes the membership-*transition* invariant. (Decision-level: the `(entity, realm)` shape is fixed
   here; the producer/cleanup wiring is the plan-memo's.)

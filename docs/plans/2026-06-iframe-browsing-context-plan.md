# iframe Browsing-Context Implementation Plan

Slot: `#11-windowproxy-browsing-context`
Status: deferred stub — see C0 / F4 in the philosophy-alignment umbrella
Why deferred: sub-frame browsing-context entity model and cross-VM Document/Window proxy identity are not yet implemented (see §2)
Trigger: agent-scoped `EcsDom` World / cross-DOM program + S5/boa removal (was "`world_id`"; see SUPERSEDED note below)
Revisit: when the agent-scoped World / B1 implementation begins (post-S5; see SUPERSEDED note below)

> **⚠ DESIGN SUPERSEDED re same-*agent* frames (2026-06-30):** this plan's §2 "separate `VmInner` +
> cross-VM forwarding" path is **inverted for same-agent frames** by
> `docs/plans/2026-06-agent-scoped-ecsdom-world.md` (B1: agent-scoped `EcsDom` World). **The boundary is the
> agent, not the origin** (that doc §1.4 / §5 req 4): **same-agent** frames — which include **same-site
> cross-origin** frames in the same browsing-context group **that the *obtain a similar-origin window agent*
> algorithm site-keys** (HTML §8.1.2.2; *not only same-origin*) — share **one World + one `Vm`** (object
> identity cannot cross heaps — that doc §2.1) and use an **in-`Vm` restricted `WindowProxy`**, **not**
> separate `Vm`s with cross-VM forwarding. So **§2.4's cross-VM forwarding is correct for *cross-agent*
> frames** — cross-site, different BCG, sandboxed-opaque, **and an *origin-keyed* same-site cross-origin
> frame** (an honored `Origin-Agent-Cluster` or a non-`none` cross-origin-isolation mode → a **different**
> agent, that doc §5 req 1) — but **not** for a *site-keyed* same-site cross-origin same-BCG frame. And `world_id` is **superseded**, not a precondition (within one World, hecs `generation`
> handles staleness). **Read *every* `world_id` reference in this plan — incl. the §3 preconditions and the
> §5 ECS-native table's `WindowProxy` `ObjectId` rule ("post-`world_id`" / "until `world_id` lands") — as the
> agent-scoped World (B1) program: renamed, not canceled, so the gates/rules remain valid as B1** (the full
> per-row rewrite is deferred with this plan's revisit). The §2/§2.4 design-prose rewrite is the trigger for this plan's revisit (the B1
> implementation PR, post-S5). See that doc §1.4 / §4.1 (cross-frame sweep) / §5 req 4 / §6.3.

---

## 1. What this plan covers

Two stub families share the same underlying deferral:

1. **`HTMLIFrameElement.contentDocument` / `contentWindow`**
   (`crates/script/elidex-js/src/vm/host/html_iframe_proto.rs`)
   — both always return `null`.

2. **Window browsing-context accessors**
   `self` / `parent` / `top` / `frames` / `frameElement` / `opener` /
   `length` / `closed`
   (`crates/script/elidex-js/src/vm/host/window.rs`)
   — `self`/`parent`/`top`/`frames` resolve to `globalThis`;
   `frameElement`/`opener` return `null`; `length` returns `0`;
   `closed` returns `false`.

These stubs return correct values **only for a genuine top-level window with
no parent, no opener, and no child frames**.  For any other context:

- `contentDocument` returning `null` is spec-correct for **cross-origin** frames
  but wrong for same-origin frames (§7.3.1.3 step 3).
- `contentWindow` returning `null` is **wrong for all frames** — even cross-origin
  frames must receive a restricted `WindowProxy` (§7.3.1.3 content-window steps
  have no origin gate; cross-origin restriction comes from proxy traps §7.2.3).
- `parent` / `top` / `frameElement` / `opener` are wrong for any
  sub-frame or opened window context.
- `frames` attribute getter is already spec-correct (§7.2.2 returns `this`'s
  own global object); only `frames[i]` indexed access (an exotic WindowProxy
  operation, §7.2.3) is missing and is deferred under the same slot.

They share the same underlying deferral: the real implementation requires the same
underlying sub-frame browsing-context entity model.

---

## 2. Missing model

The following components must exist before either family can return correct
values for same-origin frames.

### 2.1 Sub-frame browsing-context entity

Each `<iframe>` element entity must carry an ECS component (the
"content navigable" component) that holds:

- the nested `EcsDom` / document entity (the *active document*),
- the `EngineMode` and sandboxing flags derived from the `sandbox` attribute.

The iframe's **effective origin is NOT a static stored field**: the
`contentDocument` access check (§7.3.1.3 step 3) compares
`document's origin` against `container's node document's origin`, where
`document` is the active document at check time.  A navigation to another
origin changes the active document's origin and must be reflected
immediately in subsequent `contentDocument` checks — there is no cached
"inherited origin" that stays valid across navigations.

Without this, `contentDocument` has no document to return and
`length` / `frames` cannot enumerate child frames.

### 2.2 Document / Window proxy identity

The WHATWG HTML spec defines `WindowProxy` as an exotic object that
forwards most operations to the current `Window` of the browsing context
(HTML §7.2.3).  In an ECS + VM architecture this requires:

- a stable JS object id (`ObjectId`) per browsing-context entity that
  survives document navigation,
- a cross-VM forwarding mechanism when same-origin access is allowed and the
  child frame runs in a separate `VmInner`,
- `SameObject` identity: repeated reads of `.contentWindow` on the same
  iframe element **while it retains the same content navigable** must return
  the same `ObjectId`.  **Exception — reattachment**: HTML §4.8.5 destroying
  steps destroy the old child navigable when the iframe is removed from the
  DOM; the post-connection steps create a new child navigable when the iframe
  is reinserted.  A reinserted `<iframe>` gets a new browsing context, so
  `contentWindow` MUST return a new (different) `ObjectId` after reattachment.
  C1+ must NOT attach the `ObjectId` to the iframe element entity across
  detach/reattach cycles; it must invalidate and re-allocate after reattachment.

This depends on the **agent-scoped World (B1) program**
(`docs/plans/2026-06-agent-scoped-ecsdom-world.md`), which supersedes the former `world_id` discriminator
(`#11-wrapper-cache-cross-dom-discriminator`) — see the SUPERSEDED note above.

### 2.3 Same-origin access checks (contentDocument only)

`contentDocument` must return `null` for cross-origin frames (spec-correct
today) and the actual `Document` object for same-origin frames (currently
wrong).  Per §7.3.1.3 step 3: compare the **active document's** origin
against the **container's node document's** origin (= the `<iframe>`
element's `ownerDocument`; NOT the caller document's origin — the
comparison is container-stable, not caller-relative); if not
same-origin-domain, return `null`.

`contentWindow` is **NOT origin-gated**: §7.3.1.3 `content window` steps
return the active `WindowProxy` directly (step 2), with no origin check.
Cross-origin callers receive a WindowProxy whose proxy traps enforce the
cross-origin access restrictions (§7.2.3 / WHATWG HTML WindowProxy exotic
object); they do NOT receive `null`.

This requires (2.1)'s active-document reference to be current (post-navigation).

### 2.4 Cross-VM proxy semantics

When the child frame runs in a separate `VmInner`, `contentWindow` returns
a `WindowProxy` exotic object.  For **same-origin** frames the proxy forwards
`[[Get]]` / `[[Set]]` to the child VM's global.  For **cross-origin** frames
the proxy still targets the child browsing context, but its traps enforce the
cross-origin access restrictions from §7.2.3 — safelisted operations
(`closed`, `parent`, `top`, `postMessage`, etc.) must still reach the child
context; all non-safelisted accesses throw `SecurityError`.  Both cases
require a cross-VM forwarding channel; the difference is that same-origin
allows transparent property access while cross-origin limits to the safelist.
The mechanics of both cases depend on how `world_id` / cross-DOM entity
identity is solved (S5 scope).

### 2.5 Coupled-invariant matrix

These four sub-models interact; C1+ design must hold all invariants simultaneously:

| Event / scenario | §2.1 entity state | §2.2 WindowProxy identity | §2.3 origin check | §2.4 cross-VM |
|---|---|---|---|---|
| Child navigation (same slot, new document) | active document pointer updates | `ObjectId` stays stable (WindowProxy persists across navigation, §7.2.3) | origin re-derived from new active document at next access | forwarding target updates to new VM/global if VM changes |
| iframe removed from DOM (detach) | entity enters detached state; content navigable = null | `ObjectId` keeps existing wrapper alive until GC; **prior `contentWindow` reference becomes a detached `WindowProxy`** (browsing context = null) → `w.closed === true` (§7.2.2.1); **do not check `w.contentDocument`** — `contentDocument` is an `HTMLIFrameElement` attribute, not a `Window`/`WindowProxy` member; instead check `iframe.contentDocument` (§7.3.1.3 step 1 → null) or `w.closed` | contentDocument → null (step 1); contentWindow on fresh access → null (step 1); prior-held `WindowProxy` stays alive but its Window's browsing context = null | forwarding terminates; proxy becomes inert |
| sandbox `allow-same-origin` toggle (attribute mutation post-load) | **attribute mutation does NOT affect the active navigable's flags** — applied sandbox flags are snapshotted at navigation time and stored with the content navigable; C1+ must snapshot at navigation, not re-read from the attribute on every access.  C1+ therefore tracks two states: **(1) the iframe element's pending sandboxing flag set** (derived from the current `sandbox` attribute value, updated immediately on attribute mutation) and **(2) the active navigable's applied flag snapshot** (frozen at navigation time; not changed by attribute mutations).  The next navigation reads (1) to compute a new (2).  `iframe.sandbox = "allow-scripts"; iframe.src = newUrl` navigates with the updated flags because the navigation reads (1) at its start. | unchanged | origin / opaque-origin determination uses the **snapshotted** flags in (2), not the live attribute (1); re-reading the attribute would allow scripts to flip `contentDocument` access without a reload | unchanged |
| Script holds `WindowProxy` reference across navigation | no entity change | same `ObjectId`; `[[Get]]` forwards to new active Window | not applicable (no origin gate on contentWindow) | forwarding target must update atomically with navigation |
| Cross-origin access to `frameElement` | unchanged | not applicable | `frameElement` getter uses caller ↔ container doc origin check (§7.2.2.4), not active-document check | not applicable |

C1+ plan-review must verify these interactions before implementation begins.

---

## 3. Trigger / gate

| Precondition | Status |
|---|---|
| agent-scoped World (B1) program (supersedes the `world_id` discriminator, `#11-wrapper-cache-cross-dom-discriminator`) | deferred (着手 = S5 後) |
| S5 / boa removal (D-26 PR7) | deferred |

C1+ (same-origin/cross-origin proxy implementation) must not begin until both
are resolved.  The sub-frame browsing-context entity model (§2.1) is NOT a
precondition — it is C1+'s **first internal task**: C1 slice 1 implements the
entity model itself, then uses it to implement the accessor behavior.

---

## 4. Targeted tests

When C1+ begins, the test plan must distinguish the following cases:

| Case | Expected `contentDocument` | Expected `contentWindow` |
|---|---|---|
| Same-origin iframe (same effective script origin) | `Document` object (non-null) | `WindowProxy` (non-null, same-origin) |
| Cross-origin iframe | `null` (origin-gated per §7.3.1.3 step 3) | `WindowProxy` (non-null, cross-origin restricted via proxy traps §7.2.3) |
| Sandboxed iframe without `allow-same-origin` | `null` | `WindowProxy` (non-null, sandboxed — proxy traps deny most access) |
| Sandboxed iframe with `allow-same-origin` | `Document` if origins match | `WindowProxy` (non-null) |
| Detached iframe (removed from DOM, no content navigable) | `null` (§7.3.1.3 step 1) | `null` (§7.3.1.3 content-window step 1) |

Analogous cases for `parent` / `top` / `frameElement`:

| Case | Expected `parent` | Expected `top` | Expected `frameElement` |
|---|---|---|---|
| Top-level window | `globalThis` (`WindowProxy` of self) | `globalThis` | `null` |
| Same-origin child frame | `WindowProxy` of parent | `WindowProxy` of top | iframe element |
| Cross-origin child frame | opaque `WindowProxy` (limited access) | opaque `WindowProxy` | `null` (cross-origin) |

Cases for `frames` / `length` / `closed` (`#11-windowproxy-browsing-context` scope), plus `opener` (`#11-auxiliary-browsing-context-opener` scope — included for completeness):

| Case | Expected `frames` | Expected `length` | Expected `opener` | Expected `closed` |
|---|---|---|---|---|
| Top-level window with no child frames | `globalThis` | `0` | `null` | `false` |
| Top-level window with N child frames | `globalThis` (still — `frames` is an alias; `frames[i]` is the WindowProxy of child i) | `N` | `null` | `false` |
| Window opened via `window.open()` | — | — | opener `WindowProxy` (or `null` if cross-origin + no-opener) | `false` |
| Closed window | — | — | — | `true` |

`opener` is included in the **current-window accessor** group but its real
implementation depends on `window.open()` support, which is out of this
slot's scope.  For tracking purposes, `opener` correctness is owned by a
separate auxiliary-browsing-context slot (`#11-auxiliary-browsing-context-opener`
— to be carved when `window.open()` is tackled); C1+ may implement the
sub-frame accessors (`parent`/`top`/`frameElement`/`frames`/`length`) while
leaving `opener` as a null stub with ownership explicitly transferred.

`frames[i]` indexed access is a §7.2.3 exotic operation on the WindowProxy and depends on the sub-frame entity model.

---

## 5. ECS-native design notes

This section maps the OO concepts from §2 to ECS primitives for C1+.

| OO concept | ECS-native form |
|---|---|
| BrowsingContext object (owns a Document) | component on the iframe element entity |
| `WindowProxy` exotic object identity | `ObjectId` component (post-`world_id`; see CLAUDE.md Side-store→component 判定ルール — (a) per-VM identity handle exception applies until `world_id` lands) |
| SameObject guarantee for `.contentWindow` | component get: same entity and same content navigable generation → same `ObjectId`; after detach/reattach the old content navigable is destroyed and a new one created (§4.8.5), so C1+ must NOT reuse the old `ObjectId` for the new navigable — invalidate and re-allocate on each new content navigable (see §2.2 reattachment exception) |
| Cross-VM proxy forwarding | marker component + system query that dispatches to child VM; not a direct VM call |
| `contentDocument` origin check | compare active document's origin vs **container's node document's origin** (§7.3.1.3 step 3; `contentWindow` has no origin gate — never skip proxy creation for `contentWindow`) |

No new per-entity side-store (`HashMap<entity, _>`) should be introduced for
browsing-context state; the sub-frame entity itself is the handle.

---

## 6. Layering check

No existing `elidex-dom-api` / `elidex-script-session` API implements
sub-frame browsing-context entity management or cross-VM `WindowProxy`
forwarding today.  C1+ must introduce new engine-independent helpers in one of:

- `elidex-dom-api` — same-origin access check (origin comparison logic)
- `elidex-script-session` — `WindowProxy` identity map (extends the existing
  Identity Map for cross-frame proxy registration)

Prototype installation and `ObjectId` allocation remain in `vm/host/` per the
Layering mandate.  Cross-VM forwarding dispatch must route through an
engine-independent trait, not a direct `VmInner` call.

---

## 7. References

- WHATWG HTML §7.2.2 — The `Window` object; §7.2.2.4 — Accessing related windows (browsing-context accessors)
- WHATWG HTML §7.2.3 — The `WindowProxy` exotic object
- WHATWG HTML §4.8.5 — `HTMLIFrameElement` (`contentDocument`, `contentWindow`)
- CLAUDE.md `#11-wrapper-cache-cross-dom-discriminator` (world_id gate)
- `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` — Program C
- `docs/plans/2026-06-web-api-compat-split-design.md` §1.1 / §5 (C0 scope)
- `memory/project_world-id-cross-dom-migration.md` (world_id program)

---

## 8. Open design questions for C1+ plan-review

The following are intentionally left as **open design questions** in this C0 plan document.
They fall within the C1+ implementation scope and must be resolved during the mandatory
`/elidex-plan-review` pass that gates C1+ before implementation begins:

- Exact cross-origin `WindowProxy` proxy trap semantics: safelisted operations
  (`closed`, `parent`, `top`, `postMessage`, etc.), `SecurityError` throw conditions,
  `postMessage` routing across VMs, and §7.2.3 exotic trap implementation details.
- sandbox flag two-state implementation: precise algorithm for when the pending
  sandboxing flag set (iframe element attribute) is read to produce the navigable's
  applied flag snapshot (§4.8.5), and edge cases for rapid attribute-mutation +
  navigation sequences.
- Coupled-invariant matrix completeness: additional edge cases beyond those in §2.5
  (e.g. cross-origin attribute mutation, `about:blank` initial document, sandboxed
  opaque-origin frames, gate ordering for cross-VM forwarding).
- Full test matrix coverage for all Window browsing-context accessors
  (`parent`/`top`/`frameElement`/`frames`/`length`/`closed`) across all
  same-origin/cross-origin/sandboxed/detached/reattached scenarios.
- `world_id` integration details (§2.2 `ObjectId` per navigable generation,
  cross-VM identity discriminator, GC coordination).

This C0 PR scope is limited to: (a) formal slot citation in-code with why/trigger/date,
(b) this plan document as a C1+ implementation guide, and (c) stub return values
unchanged.  C1+ plan-review owns the resolution of the above.

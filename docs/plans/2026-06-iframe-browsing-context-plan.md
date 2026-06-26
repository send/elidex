# iframe Browsing-Context Implementation Plan

Slot: `#11-windowproxy-browsing-context`
Status: deferred stub — see C0 / F4 in the philosophy-alignment umbrella
Why deferred: sub-frame browsing-context entity model and cross-VM Document/Window proxy identity are not yet implemented (see §2)
Trigger: `world_id` / cross-DOM program + S5/boa removal
Revisit: when the `world_id` / S5 program begins

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

Both families are spec-correct for top-level / cross-origin contexts but
observably wrong for same-origin sub-frames.  They are one browsing-context
family: the real implementation requires the same underlying model.

---

## 2. Missing model

The following components must exist before either family can return correct
values for same-origin frames.

### 2.1 Sub-frame browsing-context entity

Each `<iframe>` must correspond to a browsing-context entity (or ECS
component on the iframe element entity) that carries:

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
  iframe element must return the same `ObjectId`.

This depends on the `world_id` discriminator described in CLAUDE.md
`#11-wrapper-cache-cross-dom-discriminator`.

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

When the child frame runs in a separate `VmInner`, `contentWindow` must
return a `WindowProxy` exotic object that forwards `[[Get]]` / `[[Set]]` to
the child VM's global.  The mechanics depend on how `world_id` / cross-DOM
entity identity is solved (S5 scope).

### 2.5 Coupled-invariant matrix

These four sub-models interact; C1+ design must hold all invariants simultaneously:

| Event / scenario | §2.1 entity state | §2.2 WindowProxy identity | §2.3 origin check | §2.4 cross-VM |
|---|---|---|---|---|
| Child navigation (same slot, new document) | active document pointer updates | `ObjectId` stays stable (WindowProxy persists across navigation, §7.2.3) | origin re-derived from new active document at next access | forwarding target updates to new VM/global if VM changes |
| iframe removed from DOM (detach) | entity enters detached state; content navigable = null | `ObjectId` keeps existing wrapper alive until GC | contentDocument → null (step 1); contentWindow → null (step 1) | forwarding terminates; proxy becomes inert |
| sandbox `allow-same-origin` toggle | sandbox flags re-read from attribute each time | unchanged | effective origin may shift from opaque → real depending on flags | unchanged |
| Script holds `WindowProxy` reference across navigation | no entity change | same `ObjectId`; `[[Get]]` forwards to new active Window | not applicable (no origin gate on contentWindow) | forwarding target must update atomically with navigation |
| Cross-origin access to `frameElement` | unchanged | not applicable | `frameElement` getter uses caller ↔ container doc origin check (§7.2.2.4), not active-document check | not applicable |

C1+ plan-review must verify these interactions before implementation begins.

---

## 3. Trigger / gate

| Precondition | Status |
|---|---|
| `world_id` discriminator (`#11-wrapper-cache-cross-dom-discriminator`) | deferred (着手 = S5 後) |
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

Cases for `frames` / `length` / `opener` / `closed` (also in `#11-windowproxy-browsing-context` scope):

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

`frames[i]` indexed access is a §7.2.2 exotic operation on the WindowProxy and depends on the sub-frame entity model.

---

## 5. ECS-native design notes

This section maps the OO concepts from §2 to ECS primitives for C1+.

| OO concept | ECS-native form |
|---|---|
| BrowsingContext object (owns a Document) | component on the iframe element entity |
| `WindowProxy` exotic object identity | `ObjectId` component (post-`world_id`; see CLAUDE.md Side-store→component 判定ルール — (a) per-VM identity handle exception applies until `world_id` lands) |
| SameObject guarantee for `.contentWindow` | component get: same entity → same `ObjectId` |
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

# iframe Browsing-Context Implementation Plan

Slot: `#11-windowproxy-browsing-context`
Status: deferred stub — see C0 / F4 in the philosophy-alignment umbrella
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

- the nested `EcsDom` / document entity,
- the `EngineMode` / origin pair inherited from the parent,
- the sandboxing flags derived from the `sandbox` attribute.

Without this, `contentDocument` has no document to return and
`length` / `frames` cannot enumerate child frames.

### 2.2 Document / Window proxy identity

The WHATWG HTML spec defines `WindowProxy` as an exotic object that
forwards most operations to the current `Window` of the browsing context
(HTML §7.3.2).  In an ECS + VM architecture this requires:

- a stable JS object id (`ObjectId`) per browsing-context entity that
  survives document navigation,
- a cross-VM forwarding mechanism when same-origin access is allowed and the
  child frame runs in a separate `VmInner`,
- `SameObject` identity: repeated reads of `.contentWindow` on the same
  iframe element must return the same `ObjectId`.

This depends on the `world_id` discriminator described in CLAUDE.md
`#11-wrapper-cache-cross-dom-discriminator`.

### 2.3 Same-origin access checks

`contentDocument` must return `null` for cross-origin frames (spec-correct
today) and the actual `Document` object for same-origin frames (currently
wrong).  The check is: compare the `origin` of the iframe's browsing context
against the `origin` of the caller's browsing context; if not same-origin,
return `null`.

This requires (2.1) to know the iframe's origin.

### 2.4 Cross-VM proxy semantics

When the child frame runs in a separate `VmInner`, `contentWindow` must
return a `WindowProxy` exotic object that forwards `[[Get]]` / `[[Set]]` to
the child VM's global.  The mechanics depend on how `world_id` / cross-DOM
entity identity is solved (S5 scope).

---

## 3. Trigger / gate

| Precondition | Status |
|---|---|
| `world_id` discriminator (`#11-wrapper-cache-cross-dom-discriminator`) | deferred (착手 = S5 後) |
| S5 / boa removal (D-26 PR7) | deferred |
| Sub-frame browsing-context entity model | not started |

C1+ (same-origin/cross-origin proxy implementation) must not begin until all
three are resolved.

---

## 4. Targeted tests

When C1+ begins, the test plan must distinguish the following cases:

| Case | Expected `contentDocument` | Expected `contentWindow` |
|---|---|---|
| Same-origin iframe (same effective script origin) | `Document` object (non-null) | `WindowProxy` (non-null) |
| Cross-origin iframe | `null` | `null` |
| Sandboxed iframe without `allow-same-origin` | `null` | `null` |
| Sandboxed iframe with `allow-same-origin` | `Document` if origins match | `WindowProxy` if origins match |
| Detached iframe (removed from DOM) | `null` | `null` |

Analogous cases for `parent` / `top` / `frameElement`:

| Case | Expected `parent` | Expected `top` | Expected `frameElement` |
|---|---|---|---|
| Top-level window | `globalThis` (`WindowProxy` of self) | `globalThis` | `null` |
| Same-origin child frame | `WindowProxy` of parent | `WindowProxy` of top | iframe element |
| Cross-origin child frame | opaque `WindowProxy` (limited access) | opaque `WindowProxy` | `null` (cross-origin) |

---

## 5. References

- WHATWG HTML §7.3 — The `Window` object (browsing-context accessors)
- WHATWG HTML §7.3.2 — `WindowProxy` exotic object
- WHATWG HTML §4.8.5 — `HTMLIFrameElement` (`contentDocument`, `contentWindow`)
- CLAUDE.md `#11-wrapper-cache-cross-dom-discriminator` (world_id gate)
- `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` — Program C
- `docs/plans/2026-06-web-api-compat-split-design.md` §1.1 / §5 (C0 scope)
- `memory/project_world-id-cross-dom-migration.md` (world_id program)

# S5-6b Stage 2f-4 — parent-message receive-gate + the final three `bridge()` sites (B5 / B16 / thread.rs:221)

Per-PR-slice plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`)
and the S5-6 flip memo (`docs/plans/2026-07-s5-6-flip-boa-deletion.md` — §3.4 rows **B5** (`:366`),
**B16** (`:377`), **B19** (`:380`); the authoritative B16 design lives at §719-731 "Parent-message
drain"). This memo scopes the **last three `runtime.bridge()` production call sites** of the flip
(the boa bridge accessor is deleted): `content/event_loop.rs:88` (B5), `content/iframe/thread.rs:114`
(B16), `content/iframe/thread.rs:221` (`PreEvalFrameInputs` reads). It does **not** re-derive the
send-side, which landed in S5-6a (`native_window_post_message` resolution + `ParentMessage` wire); it
designs the **receive-side gate** the send-side was built to feed.

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE any impl. This is **edge-dense**: it crosses
> ≥3 intersecting invariant axes — **cross-origin-message-leak security boundary × opaque-origin
> identity (sentinel keys, never lossy `"null"`) × OOP-vs-in-process transport × send-resolution/
> receive-gate split** — and there is no off-the-shelf canonical algorithm for "one fail-closed gate
> for two transports". CLAUDE.md "Security by structure, not review convention" + "One issue, one way"
> + "Edge-dense work … 実装前 plan-review 必須". §6 carries the open questions; §7 is the sub-commit
> split; §8 is the carve list.

All cites grep-verified against worktree `s5-6b-flip` HEAD **`4d54ca07`** (2026-07-12). Spec anchors
webref-verified 2026-07-12 (`.claude/tools/webref body html window-post-message-steps` /
`… posting-messages`): §9.3.3 "Posting messages" (anchor `#posting-messages`, `web-messaging.html`),
algorithm "window post message steps" (anchor `#window-post-message-steps`).

---

## Decision on record — ONE fail-closed gate at ONE receiving chokepoint, fed by BOTH transports normalised to `Vec<ParentMessage>`; expose the parent's `storage_origin_key` on the `HostDriver` trait

The §9.3.3 targetOrigin gate is a **cross-origin-message-leak boundary**: it decides whether an
iframe's `postMessage` is delivered to the parent window. The send side (S5-6a) already resolved the
sender's `targetOrigin` to an **identity-preserving origin key** on the `ParentMessage.target_origin`
wire (steps 4–5, opaque sender → per-VM sentinel, opaque URL target → fail-closed-at-send). The
**receive side (this memo)** applies §9.3.3 **step 8.1** — "if the `targetOrigin` argument is not `*`
**and** targetWindow's associated Document's origin is not same origin with `targetOrigin`, then
return". The first-principles ideal (CLAUDE.md):

- **Security by structure** — the gate is a **single free function** applied at a **single receiving
  chokepoint** that every delivery path (OOP IPC + in-process shared-thread) is funnelled through,
  **fail-closed** (a message whose `target_origin` neither equals the parent key nor is `"*"` is
  dropped, never dispatched). No per-transport ad-hoc check a future edit can forget.
- **One issue, one way** — OOP and in-process iframe→parent messaging **converge onto ONE gate
  function + ONE `Vec<ParentMessage>` normalisation**. The gate input (parent `storage_origin_key`
  vs `ParentMessage.target_origin`, honouring `"*"`) is **identical**; only the transport differs, so
  the transport difference is absorbed by a single drain that yields `ParentMessage` from both, and
  the gate sees one input shape.
- **Identity-preserving, never lossy `"null"`** — the gate compares `storage_origin_key`s (opaque →
  per-VM sentinel `opaque_origin_sentinel`), so distinct opaque origins never alias. This is a
  **different value** from `ParentMessage.origin` (the DISPLAYED sender origin → `MessageEvent.origin`,
  where opaque IS `"null"` per §7.1.1). The two are kept crisply separate — the `ParentMessage` struct
  doc (`host_effects.rs:122-158`) is the SoT and this memo does **not** change it.

**Rejected alternatives (explicit):**

1. **Gate at the send site only** — REJECTED. §9.3.3 step 8.1 compares against *targetWindow's*
   (the parent's) Document origin, which the sending iframe VM cannot know (its `parent`/`top` resolve
   to `globalThis` stubs, `window.rs:346-409`). The send site can only resolve the *key* (steps 4–5);
   the same-origin comparison is structurally a receive-side operation. (This is exactly why the
   send-side native comments "the §9.3.3 origin gate is NOT applied here", `pending_tasks.rs:520-528`.)
2. **Two mechanisms (an OOP path and a separate in-process path)** — REJECTED (One-issue-one-way).
   The gate is identical for both; a second code path is a decision-tax ("which gate does this
   transport use?") and a place for the two to drift (one gets a fix the other misses — a latent
   leak). One drain → `Vec<ParentMessage>` → one gate.
3. **Derive the parent key from `origin()` in the shell** — REJECTED. The parent key must be the
   *same* serialization the send side produced (`storage_origin_key`: tuple → `serialize()`, opaque →
   per-VM sentinel). Re-deriving it in the shell would re-implement the opaque-sentinel logic outside
   the VM (leak of VM identity semantics into the shell + divergence risk). Expose
   `storage_origin_key` on the trait so the parent-side key is **identical-by-construction** to the
   send-side key.
4. **Stub the in-process path** ("only OOP delivers parent messages for now") — REJECTED (Ideal over
   pragmatic). §6.1 shows in-process iframe→parent `postMessage` is a **pre-existing absent seam**
   (never delivered under boa either); the clean design closes it through the same gate, it does not
   leave a second unhandled path.

---

## §1 Scope — the three `bridge()` sites

| Part | Site (HEAD `4d54ca07`) | Surface | Memo row | Weight |
|---|---|---|---|---|
| **B5** | `content/event_loop.rs:88-94` | Delete the top-level self-`postMessage` drain (`bridge().drain_post_messages()`); the VM self-delivers depth-0 messages internally with the §9.3.3 gate applied inline. `needs_render` rides the §4.3.8 version-delta. | B5 (`:366`) | trivial |
| **B16** | `content/iframe/thread.rs:114` (+ `event_loop.rs:83-86`, `iframe/mod.rs:112-133`, `iframe/types.rs:62,132`) | The iframe→parent receive-side opaque-origin `targetOrigin` gate: extend the IPC wire, normalise OOP+in-process onto `Vec<ParentMessage>`, apply ONE fail-closed gate at the parent chokepoint. | B16 (`:377`) | **edge-dense (the real surface)** |
| **thread.rs:221** | `content/iframe/thread.rs:221-227` | Migrate `PreEvalFrameInputs` reads off `bridge()`: `sandbox_flags`/`iframe_depth` → trait; `credentialless` → shell-owned (B19); `referrer` → shell-owned. Latent/dead path (`#11-oop-iframe-navigate-completeness`). | B19 (`:380`) + B23 | mechanical (dead path) |

**Explicitly out of scope** (carved, §8): the real `WindowProxy` browsing-context targeting model
(S5-8/B1 — replaces the depth-routing wholesale); nested in-process iframe → intermediate-parent
routing (rides S5-8; boa-parity routes to the content-thread top-level document); the OOP `Navigate`
runtime correctness (`#11-oop-iframe-navigate-completeness` — referrer-chain + cookie-jar handoff);
structured (non-`ToString`) message serialization (S5-8/B1). **The `#[cfg(test)]` `bridge()` call
sites** (`viewport_tests.rs`, `content_iframe_security_tests.rs`, `content_history_drain_tests.rs`,
etc.) are a **separate test-migration slice**, not part of the three *production* compile
errors this memo closes; §6.5 flags the sequencing (verified 2026-07-12: `cargo build -p
elidex-shell --all-features --tests` = **58 total errors** — ~50 `no method bridge` + ~8 COUPLED
non-bridge: DeviceFacts-arity `ContentState::new`, a test already expecting the shell-owned
`referrer: Option<url::Url>` field this memo adds, trait-in-scope `eval`/`take_pending_window_opens`,
`console_output` — so the slice is NOT purely mechanical).

**Layering-check (CLAUDE.md Layering mandate).** Is any of the gate engine-independent algorithm that
belongs in `elidex-dom-api`/a shared crate? **No.** The gate is a **two-line string comparison over
VM-marshalled origin keys** (`parent_key == target_origin || target_origin == "*"`) — pure
shell-transport routing, not a DOM/CSSOM/form/selector algorithm. The identity-bearing work
(`storage_origin_key`'s opaque sentinel = per-VM identity) is **VM-marshalling** and belongs on the
`HostDriver` trait (VM impl), exactly where the send-side `Vm::storage_origin_key` already sits. So
the usual crate-mapping table is intentionally **N/A**: the gate = shell free-fn, the key = trait
accessor (VM impl); nothing routes to `elidex-dom-api`/`elidex-form`/`elidex-css`.

**ECS-native side-store check (CLAUDE.md "Side-store→component 判定ルール").** Should
`pending_parent_messages` be ECS components instead of a per-VM HostData FIFO? **No.** It is
`effect_queues.parent_messages: Vec<ParentMessage>` (`host_data/effect_queues.rs:104-113`) — a
**transient per-VM outbound effect queue**, NOT per-entity state. It has no entity key (it is a
browsing-context-scoped outbound message list drained each turn), and it is the same class as the
sibling effect queues (`storage_changes` / `idb_versionchange` / top-level `post_messages`). It is
the canonical **FIFO trait event-queue** the flip already uses (CLAUDE.md "FIFO trait event-queue …
reuse it, do not add a parallel channel"), landed in S5-6a. **No change** — this memo reuses the
existing `take_pending_parent_messages()` drain, it adds no side-store.

---

## §2 Coupled invariants

The receive-gate sits at the intersection of four axes. Each pairwise intersection is named so
plan-review can check them independently.

- **security-boundary × opaque-origin-identity.** The gate's correctness depends on BOTH sides using
  the **same** serialization. The send side resolved `target_origin` to `storage_origin_key` form
  (tuple → `serialize()`; `/` → sender sentinel; opaque URL → fail-closed, `pending_tasks.rs:558-576`).
  The receive side must produce the parent key the *same* way, or a tuple-vs-tuple comparison could
  mismatch on formatting and an opaque-vs-opaque comparison could alias. **Intersection resolution**:
  expose `HostDriver::storage_origin_key` (VM impl = `Vm::storage_origin_key`), so both keys come from
  one function — identical by construction. A lossy `"null"` on the gate path is *structurally
  impossible* because neither side ever emits `"null"` for a gate key (send fails-closed on opaque
  URL targets; `storage_origin_key` uses the per-VM sentinel, `navigation.rs:395-405`).

- **OOP-transport × in-process-transport.** OOP loses `target_origin` across the IPC boundary unless
  the wire carries it (`IframeToBrowser::PostMessage` / `OopPostMessage` lack it today,
  `types.rs:62,132`); in-process carries the full `ParentMessage` (which has `target_origin`) with no
  serialization loss. **Intersection resolution**: extend the OOP IPC struct with `target_origin`, and
  have the parent drain **normalise both transports to `Vec<ParentMessage>`** so the single gate sees
  one input shape regardless of transport (One-issue-one-way).

- **send-resolution × receive-gate (the split).** §9.3.3 forces the split: steps 4–5 (resolve
  `targetOrigin`) run in the sender's realm (incumbentSettings's origin); step 8.1 (the same-origin
  comparison) runs against targetWindow (the parent). Neither side alone can gate. **Intersection
  resolution**: `ParentMessage.target_origin` is the seam contract — send writes it (steps 4–5),
  receive reads it (step 8.1). This memo must NOT re-resolve on the receive side (the sender already
  did steps 4–5) — it only compares.

- **fail-closed × dead-path.** thread.rs's OOP `Navigate` handler is a dead path (no production
  sender), and the in-process gate is newly-closed by this memo. Yet the gate must be fail-closed **by
  construction** so a future live sender cannot leak. **Intersection resolution**: the gate is a single
  structural chokepoint (§5) — even a not-yet-exercised transport routes through it; there is no
  "temporarily ungated" delivery path.

---

## §3 Spec coverage map

webref-verified §↔title pair: **§9.3.3 "Posting messages"** (anchor `#posting-messages`,
`web-messaging.html`); algorithm **"window post message steps"** (anchor `#window-post-message-steps`).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §9.3.3 window post message steps | step 4 — "if `targetOrigin` is `/`, set to incumbentSettings's origin" | `/` → sender `storage_origin_key` (opaque→sentinel) | **send** (done S5-6a): `pending_tasks.rs:572` (`None if target_origin_str == "/" => ctx.vm.storage_origin_key()`) | ✓ | yes (targetOrigin arg) |
| WHATWG HTML §9.3.3 window post message steps | step 5 — "otherwise if not `*`: parse URL; SyntaxError on failure; set to parsedURL's origin" | URL tuple → `ascii_serialization()`; URL opaque (`data:`) → **fail-closed at send** (drop) | **send** (done S5-6a): `pending_tasks.rs:562-568` | ✓ | yes |
| WHATWG HTML §9.3.3 window post message steps | step 5 guard-negation (`*`) — targetOrigin IS `*`, so step 5 is skipped (its guard is "otherwise if not `*`") and targetOrigin rides unchanged | `*` → gate returns true (any origin) | **send+receive**: wire `target_origin == "*"` | ✓ | yes |
| WHATWG HTML §9.3.3 window post message steps | step 7 — "StructuredSerializeWithTransfer(message, transfer); rethrow" | throwing `toString` surfaces before the gate/drop | **send**: `pending_tasks.rs:544-546` (clone-before-match) | ✓ | yes |
| WHATWG HTML §9.3.3 window post message steps | **step 8.1** — "if `targetOrigin` is not `*` **and** targetWindow's Document's origin is not same origin with `targetOrigin`, then return" | (a) `*` → deliver; (b) `parent_key == target_origin` → deliver; (c) else → **return (drop)** | **RECEIVE (this memo)**: `parent_message_allowed(parent_key, target_origin)` (NEW) at the §5 chokepoint | ✓ | no |
| WHATWG HTML §9.3.3 window post message steps | step 8.7 — "fire `message` at targetWindow; origin ← origin, data ← messageClone" | `MessageEvent.origin` = `ParentMessage.origin` (DISPLAYED, opaque→`"null"`) | **RECEIVE**: `dispatch_message_event` (`content/mod.rs:670`) | ✓ | yes (message data) |

**Routing enum (depth-based, boa-parity interim).** `native_window_post_message` routes by
`iframe_depth` (`pending_tasks.rs:534-539`): **depth 0** → VM-internal self-delivery (`dispatch_post_message`
`:332`, gate applied inline via `match_target_origin` `:596`); **depth > 0** → `enqueue_parent_message`
onto the FIFO (`:580-583`) → this memo's receive gate. Both branches gate against §9.3.3 step 8.1;
only the *transport* to the gate differs. The real `WindowProxy` targeting replaces depth-routing at
S5-8/B1 (§8).

**Spec takeaway**: the send side resolved steps 4–7 (S5-6a); this memo is **step 8.1 only** (the gate)
+ step 8.7 dispatch (already wired via `dispatch_message_event`). No re-resolution on receive.

---

## §4 Per-part design rulings

### Part B5 — delete the top-level self-message drain (`event_loop.rs:88-94`)

**Ruling (DELETE).** Under the VM a depth-0 `window.postMessage` is **VM-internal**: the native
self-delivers via `dispatch_post_message` (`pending_tasks.rs:332`) with the §9.3.3 step 8.1 gate
applied inline (`match_target_origin` `:596`, `compute_own_origin_sid` `:686`), delivered by the VM
task drain. So the shell no longer drains or dispatches top-level self-messages. Concretely:

- **DELETE** `event_loop.rs:88-91` (`let self_messages = …bridge().drain_post_messages(); for (data,
  origin) in &self_messages { dispatch_message_event(state, data, origin); }`). The `top_level_
  post_message_self_delivers_and_fifo_stays_empty` test (`tests_engine_s6a.rs:601`) pins that a
  depth-0 send self-delivers and leaves the parent FIFO empty — so nothing to drain shell-side.
- **`needs_render` (`:92-94`)** references `self_messages`. **Ruling: fold into the §4.3.8 version-
  delta.** The self-message DOM effect (a handler that mutates the DOM) moves the document-root
  version; the inclusive-descendants version-delta already wired at `event_loop.rs:216-223`
  (`inclusive_descendants_version(document) != last_render_dom_version`) restores `needs_render` — the
  identical pattern the realtime/worker drains use (`:198-206`, keystone stage 2d-2). The OOP-message
  half of the `:92-94` condition is likewise subsumed: the OOP dispatch at `:83-86` runs *before* the
  `:216` check in the same turn, so its DOM mutation is caught by the version-delta too. **Ruling:
  DELETE the entire `:92-94` `needs_render` block**; both self- and OOP-message-driven re-render ride
  the one version-delta signal (One-issue-one-way: one `needs_render` source).
  - **§6.6 verification obligation**: confirm no message-handler side effect needs a re-render
    *without* a DOM-version bump (a handler that only mutates scroll/canvas has its own signals —
    `update_caret_blink`/scroll/canvas paths — so the version-delta suffices for the DOM case). If
    plan-review finds a counterexample, keep a narrow `!post_messages.is_empty()` disjunct; the ideal
    is DELETE.

### Part B16 — the iframe→parent receive gate (the real surface)

**Ruling.** ONE gate function, ONE receiving chokepoint, BOTH transports normalised to
`Vec<ParentMessage>`. Four coordinated edits:

**(1) The gate function — `parent_message_allowed` (NEW, shell free-fn).** Defined beside
`dispatch_message_event` (`content/mod.rs`) or in `content/iframe/mod.rs`:

```
/// §9.3.3 "Posting messages" step 8.1: deliver iff the sender's resolved
/// targetOrigin is `*`, or it equals the parent window's origin key. The keys
/// are identity-preserving `storage_origin_key`s (opaque → per-VM sentinel), so
/// distinct opaque origins never alias — there is NO `"null"` special case.
fn parent_message_allowed(parent_key: &str, target_origin: &str) -> bool {
    target_origin == "*" || parent_key == target_origin
}
```

Semantics (cite §9.3.3 step 8.1): `"*"` → true (any origin); else same-origin string equality on the
`storage_origin_key`s. **NO `"null"` case** — the send side fails-closed on opaque URL targets and
uses the per-VM sentinel for `/`, so a gate key is never `"null"` (the DISPLAYED `"null"` lives only
on `ParentMessage.origin`, which is the `MessageEvent.origin`, not a gate input).

**(2) Expose the parent key — `HostDriver::storage_origin_key` (NEW trait method).** ADD
`fn storage_origin_key(&self) -> String;` to the trait (`script-session/src/engine.rs`, beside
`origin()` `:450`), impl on `ElidexJsEngine` (`elidex-js/src/engine.rs`) delegating to the existing
`Vm::storage_origin_key` (`navigation.rs:395`, currently `pub(crate)` — reachable from the same-crate
engine impl, mirroring how the `origin()` impl delegates to the VM). Rationale: the parent key must be
byte-identical to the send-side serialization; the trait accessor makes it so by construction (§2
security×identity axis; rejected-alt 3). **Not** derived from `origin()` (would re-implement the
opaque sentinel in the shell).

**(3) Extend the OOP IPC wire with `target_origin`.** Write-site audit (CLAUDE.md — include struct-
literal ctors):
- `IframeToBrowser::PostMessage` (`types.rs:62-67`): add `target_origin: String`.
- `OopPostMessage` (`types.rs:130-139`): **DELETE the struct**, converge on `ParentMessage` (see (4)).
  Its only consumer is `drain_oop_messages`; its `entity` field is already dead (`#[allow(dead_code)]`)
  — dropping it is boa-parity-preserving (nested-parent routing rides S5-8, §8). One-issue-one-way:
  no shell struct that duplicates `ParentMessage`.
- **Forward site** `thread.rs:114`: `for (data, origin) in …bridge().drain_post_messages()` →
  `for msg in pipeline.runtime.take_pending_parent_messages()` then
  `channel.send(IframeToBrowser::PostMessage { data: msg.data, origin: msg.origin, target_origin: msg.target_origin })`.
- **Construct site** `iframe/mod.rs:121-127`: destructure `PostMessage { data, origin, target_origin }`
  and push a `ParentMessage { data, origin, target_origin }` (not an `OopPostMessage`).

**(4) Normalise both transports + gate at ONE chokepoint (`event_loop.rs:83-86`).** Reshape
`IframeRegistry::drain_oop_messages` → `drain_parent_messages(&mut self) -> Vec<ParentMessage>`
(`iframe/mod.rs:112`) that walks ALL entries:
- **OOP** entry → `try_recv` `PostMessage { data, origin, target_origin }` → push
  `ParentMessage { data, origin, target_origin }` (was `OopPostMessage`).
- **InProcess** entry → `ip.pipeline.runtime.take_pending_parent_messages()` → extend (this **closes
  the in-process gap**, §6.1). Draining into an owned `Vec` releases the `state.iframes` borrow before
  dispatch (the borrow-discipline reason `drain_oop_messages` already returns a `Vec`).

Then `event_loop.rs:83-86` becomes the single gated dispatch loop:

```
let parent_key = state.pipeline.runtime.storage_origin_key();
for msg in state.iframes.drain_parent_messages() {
    if parent_message_allowed(&parent_key, &msg.target_origin) {
        dispatch_message_event(state, &msg.data, &msg.origin);
    }
}
```

`parent_key` is computed once (the parent is `state.pipeline` for both transports); the gate is
fail-closed (default = drop); `dispatch_message_event` (`content/mod.rs:670`) fires the `MessageEvent`
at `state.pipeline.document` (the parent window) with `MessageEvent.origin = msg.origin`.

### Part thread.rs:221 — migrate `PreEvalFrameInputs` reads off `bridge()`

**Ruling (per field).** The site builds `PreEvalFrameInputs { sandbox_flags, credentialless,
iframe_depth, referrer }` (`thread.rs:222-227`) for the OOP `Navigate` rebuild. It is a **dead path**
(`#11-oop-iframe-navigate-completeness` — no production sender for `BrowserToIframe::Navigate`,
`thread.rs:228-244`); the migration must **compile-converge** it, runtime correctness already carved.

| Field | Disposition | Source | Why |
|---|---|---|---|
| `sandbox_flags` | **trait read** | `pipeline.runtime.sandbox_flags()` (trait `:458`) | already a `HostDriver` method (VM-observable: sandbox eval gate) — mechanical B23 swap |
| `iframe_depth` | **trait read** | `pipeline.runtime.iframe_depth()` (trait `:486`) | already a `HostDriver` method (VM-observable: `MAX_IFRAME_DEPTH`) — mechanical B23 swap |
| `credentialless` | **shell-owned (B19)** | new `PipelineResult.credentialless: bool` | B19: browsing-context config, **no engine surface** (VM behaviour derives from `set_origin`, S5-4b); CLAUDE.md exception (b). The VM has no `credentialless()` — `pipeline.rs:237-241` already dropped `set_credentialless`. Retain shell-side (the `cookie_jar` B18 precedent, `lib.rs:216-223`). |
| `referrer` | **shell-owned** | new `PipelineResult.referrer: Option<url::Url>` | The trait has **only a setter** (`set_navigation_referrer` `:403`) — **no getter** (verified: `grep referrer engine.rs` = setter only). The trait *deliberately* has no read-back getters for shell-owned config (B18 cookie_jar "deliberately no getter"; B20 device-facts "no getters, deliberately"). Adding a getter would violate that design; the referrer is shell-constructed (`compute_referrer` → `set_navigation_referrer`), so **retain the value shell-side** (same B18/B20 pattern), don't read it back. |

So `PipelineResult` gains two shell-owned fields (`credentialless: bool`, `referrer: Option<url::Url>`),
populated at iframe build (where the shell already has the `PreEvalFrameState` — `pipeline.rs:51-74`),
and `thread.rs:221-227` reads `sandbox_flags`/`iframe_depth` from `pipeline.runtime`,
`credentialless`/`referrer` from `pipeline.<field>`. The `let bridge = pipeline.runtime.bridge();`
line (`:221`) is deleted.

- **Rejected alt**: retain the full `PreEvalFrameState` on `PipelineResult` — REJECTED: it also
  bundles `sandbox_flags`/`iframe_depth`, which are on the trait, creating a **dual source of truth**
  (One-issue-one-way violation). Retain only the two fields the VM lacks a getter for.
- **Dead-path caveat**: even the referrer *sourcing* here is already wrong (should be the frame's
  previous-document URL per §7.4.2 navigation referrer chain, not the embedder's original referrer —
  `thread.rs:229-237`). This memo does **not** fix that (carved); it only moves the read off the
  deleted `bridge()`.

---

## §5 Receive-chokepoint enumeration (fail-closed, once-firing)

The gate must be the **single** point every iframe→parent message passes through, applied exactly once
per message, defaulting to drop.

| # | Transport | Producer | Normalisation → gate | Once-firing / fail-closed |
|---|---|---|---|---|
| 1 | **OOP** (cross-origin, separate thread) | iframe thread `thread.rs:114` `take_pending_parent_messages()` → `IframeToBrowser::PostMessage {data,origin,target_origin}` (extended wire) | parent `drain_parent_messages` reconstructs `ParentMessage` from IPC → gate at `event_loop.rs:83-86` | drained once per `try_recv`; gate default-drop |
| 2 | **In-process** (same-origin, parent thread) | InProcess `ip.pipeline.runtime.take_pending_parent_messages()` (**NEW drain — closes the §6.1 gap**) | same `drain_parent_messages` extends the same `Vec<ParentMessage>` → same gate | `take_*` empties the FIFO once; gate default-drop |
| 3 | depth-0 self (top-level) | — (VM-internal) | **NOT** a shell chokepoint — self-delivery + gate is inline in the VM (`dispatch_post_message` + `match_target_origin`, B5 deletes the shell drain) | gated inside the VM; no shell path |

**Once-ness / no-double-fire.** Both `take_pending_parent_messages()` (in-process) and `try_recv`
(OOP) *consume* their source, so a message cannot be drained twice. The gate is applied exactly once,
in the single `event_loop.rs:83-86` loop. **No-leak argument** (plan-review must verify): every
iframe→parent message is produced by `enqueue_parent_message` (`effect_queues.rs:104`, the ONLY
enqueue site — grep-confirm) and consumed only by `take_pending_parent_messages` (the ONLY drain);
both transports route that drain through `drain_parent_messages` → the one gate. A reviewer should
adversarially search for a second dispatch of a `ParentMessage.data` that bypasses
`parent_message_allowed` (should be impossible: `dispatch_message_event` for parent messages is only
called from the gated loop).

---

## §6 Open design questions (for plan-review)

### §6.1 (CRITICAL) In-process iframe→parent drain — gap-close vs pre-existing seam?

**Investigation finding: in-process iframe→parent `postMessage` is a PRE-EXISTING ABSENT seam — this
memo CLOSES it through the same gate.** Evidence:

1. **No in-process drain exists today.** `tick_iframe_timers` (`iframe/render.rs:56-71`) drains
   **timers only** for InProcess iframes; it never drains `take_pending_parent_messages`. The
   `drain_post_messages`/`bridge()` path exists **only in the OOP thread** (`thread.rs:114`). So an
   in-process (same-origin) iframe whose JS calls `parent.postMessage` — which, under the VM,
   enqueues onto its FIFO when `iframe_depth > 0` — is **never delivered** to the parent. This holds
   under boa too (boa's `drain_post_messages` for the in-process pipeline is equally undrained).
   **Grep-confirmed**: no `take_pending_parent_messages`/`drain_parent` call in `iframe/render.rs`,
   `iframe/lifecycle.rs`, or `event_loop.rs` for InProcess.
2. **Closing it is the ideal (Ideal over pragmatic + One-issue-one-way).** The parent for an
   in-process iframe is the same content-thread `state.pipeline` (top-level document) as for OOP
   (boa-parity: flat `state.iframes` registry, no nested WindowProxy pre-S5-8). So the in-process
   drain produces `ParentMessage`s gated against the *same* `parent_key` and dispatched via the *same*
   `dispatch_message_event`. The §4-Part-B(4) `drain_parent_messages` normalisation is precisely the
   convergence point: OOP + in-process → one `Vec<ParentMessage>` → one gate.
3. **In-process is CLEANER than OOP** — it drains `ParentMessage` directly (already has
   `target_origin`), needing **no** IPC struct extension. Only OOP needs the wire widening (§4-Part-B(3)).

**Plan-review CONFIRMED** (Axis 2 + Axis 5, 2026-07-12): (a) an InProcess iframe's pipeline IS stamped
`iframe_depth > 0` at build — `build_load_context` sets `depth = parent_depth + 1` (`lifecycle.rs:383`,
`parent_depth = runtime.iframe_depth()`) → `pre_eval_state.iframe_depth = ctx.depth` (`load.rs:411`) →
`runtime.set_iframe_depth(state.iframe_depth)` at the pre-eval install seam (`pipeline.rs:236`, runs
BEFORE first eval, S5-4b invariant). So a first-level in-process iframe (parent depth 0) is stamped
depth 1 → its `parent.postMessage` takes the ENQUEUE branch (`pending_tasks.rs:536`), not depth-0
self-delivery → the FIFO is populated → the 2f4-d acceptance test premise holds. The write-path is
**pre-existing** (2f4-d adds only the drain, no depth stamp). (b) that draining `ip.pipeline` inside
`state.iframes` while dispatching at `state.pipeline`
is borrow-clean via the owned-`Vec` return (it is — mirrors `drain_oop_messages`); (c) whether the
in-process drain belongs in `drain_parent_messages` (unified, preferred) vs a sibling of
`tick_iframe_timers` (rejected — would split the gate). **Ruling: unified `drain_parent_messages`.**

### §6.2 `storage_origin_key` on the trait — accessor vs derive-from-`origin()`?

§4-Part-B(2) rules **ADD the trait accessor**. Plan-review should confirm: (a) `Vm::storage_origin_key`
(`pub(crate)`, `navigation.rs:395`) is reachable from the `ElidexJsEngine` trait impl (same crate —
yes); (b) no existing trait method already exposes it under another name (grep = none; `origin()`
returns `SecurityOrigin`, not the key string); (c) the opaque-sentinel semantics
(`opaque_origin_sentinel`, per-VM) is the right identity for the gate (it is — it is exactly what the
send-side `/`-resolution uses, `pending_tasks.rs:572`).

### §6.3 IPC write-site completeness

Confirm the full `target_origin` write-site set (CLAUDE.md write-site audit incl. struct-literal
ctors): `IframeToBrowser::PostMessage` def (`types.rs:62`); forward ctor (`thread.rs:114`); `OopPostMessage`
deletion + `ParentMessage` ctor at the construct site (`iframe/mod.rs:122`); the parent gate read
(`event_loop.rs:83-86`). Are there other `IframeToBrowser::PostMessage` constructors or
`OopPostMessage` literals (grep)? (grep at HEAD shows exactly these; re-verify post-reshape.)

### §6.4 The gate function — exact semantics + no `"null"` case

Confirm `parent_message_allowed`: `target_origin == "*"` → true; else `parent_key == target_origin`.
Cite §9.3.3 step 8.1 (verified verbatim, §3). Confirm there is **no** `"null"` special case: the send
side never emits `"null"` on the gate path (opaque URL target → fail-closed at send; `/` → sentinel;
tuple → serialize), and `storage_origin_key` emits the per-VM sentinel for opaque, not `"null"`
(`navigation.rs:400-404` — note the `"null"` fallback there is only for the *host-data-absent*
pure-test path, unreachable when bound). Plan-review: verify the bound-path invariant (a real parent
always has `host_data`, so its key is a sentinel, never the `"null"` literal fallback).

### §6.5 Production vs test `bridge()` sites — sequencing (RESOLVED)

This memo closes the **three production** `bridge()` errors (`cargo build -p elidex-shell
--all-features` = 3-4, verified 2026-07-12). The **`#[cfg(test)]` `bridge()` sites**
(`content_iframe_security_tests.rs:127`, `viewport_tests.rs:848`, `content_history_drain_tests.rs:96`,
etc.) still reference the deleted accessor. **RESOLVED**: `cargo build -p elidex-shell --all-features
--tests` = **58 total errors** (verified 2026-07-12) — **~50** `error[E0599] no method bridge` **plus
~8 COUPLED non-bridge** errors (a DeviceFacts-arity `ContentState::new` `E0061`; a test already
expecting the shell-owned `referrer: Option<url::Url>` this memo's 2f4-e adds; trait-not-in-scope
`eval`/`take_pending_window_opens`; `console_output`). So `mise run ci` (nextest compiles all tests)
does NOT pass until they migrate — the test-migration is a **HARD prerequisite of the Stage 3 merge
gate** (green CI), not an optional carve. It is **mostly mechanical** (the ~50 `bridge()` reads of
`device_pixel_ratio`/`origin`/`sandbox_flags`/`color_scheme`/… → trait methods / shell-owned fields)
**but NOT purely so** (the ~8 coupled errors — esp. the `referrer` type coupling to 2f4-e — mean the
slice must land AFTER 2f-4's shell-owned fields exist). It **must NOT bundle into 2f-4's edge-dense
gate commits** (blast-radius — a ~58-site sweep in the security-gate PR obscures both). **Ruling**: a
distinct slice `#11-shell-test-bridge-migration`, sequenced AFTER 2f-4 (production 0 errors + the
`referrer`/`credentialless` shell fields it depends on) and BEFORE Stage 3 (CI gate), as its own
commit(s) in the flip branch. Plan-review: confirm this sequencing (not "defer indefinitely" — it is
flip-required before merge).

### §6.6 B5 `needs_render` — does the version-delta fully cover message-driven re-render?

§4-Part-B5 rules DELETE the `:92-94` `needs_render` block (fold into the version-delta). Plan-review:
confirm no message handler needs a re-render *without* a DOM-version bump (a DOM mutation moves
`inclusive_descendants_version`; scroll/canvas/caret have their own signals). If a counterexample
exists, keep a narrow OOP disjunct; ideal = DELETE.

---

## §7 Sub-commit split + acceptance

Base-case terminal slice under the approved S5 umbrella + this plan-review (CLAUDE.md "base case =
narrowly-scoped per-PR slice") → **one PR**, internally sequenced so each sub-commit is independently
reviewable and the gate infrastructure lands before its consumers.

**Base-case vs edge-dense reconciliation** (plan-review Axis 3 MIN): the gate box + §2 declare this
work edge-dense across 4 axes, yet §7 keeps it a single PR. No contradiction — the 4 axes are
**constraints on ONE artifact** (the single `parent_message_allowed` gate + its one chokepoint), not
4 separable features; §2 maps them precisely to show they converge, which is the base-case exclusion's
intent (a narrowly-scoped, plan-reviewed slice under an approved umbrella). The two mechanical non-gate
surfaces bundled here (B5 top-level-drain DELETE + thread.rs:221 dead-path migration) are the **same
three `bridge()` compile errors** this PR exists to close — co-located by the flip's error-set, not
separable features — so bundling them is not a blast-radius violation (unlike the ~58-site test sweep,
which is a genuinely separate surface, §6.5). The security-critical gate still gets the full
`/external-converge` at push (§7 push gate).

| Sub-commit | Content | Acceptance |
|---|---|---|
| **2f4-a** | B5: delete the top-level self-message drain (`event_loop.rs:88-91`) + the `:92-94` `needs_render` block (rides version-delta). | Compiles; `top_level_post_message_self_delivers_and_fifo_stays_empty` still green; a self-`postMessage` handler that mutates the DOM still re-renders (version-delta). |
| **2f4-b** | Gate infra: `HostDriver::storage_origin_key` (trait + VM impl) + `parent_message_allowed` free-fn. | Trait method returns the send-side-identical key (unit test: tuple → `serialize()`, opaque → sentinel). Gate unit tests: `*` allows; equal keys allow; distinct keys drop; distinct opaque sentinels drop (no alias). |
| **2f4-c** | B16 OOP wire: extend `IframeToBrowser::PostMessage` + `OopPostMessage`→`ParentMessage` convergence; reshape `drain_oop_messages`→`drain_parent_messages` (OOP half); `thread.rs:114` `bridge()`→`take_pending_parent_messages`; gated dispatch loop at `event_loop.rs:83-86`. **On touch (D-17 cite discipline, plan-review Axis 4 IMP):** correct the `dispatch_message_event` docstring (`content/mod.rs:669`) — it cites `§9.4.3` ("The MessageEventTarget mixin", wrong) but this is the §9.3.3 **step 8.7** window-postMessage fire step; fix to `§9.3.3 step 8.7` while touching the dispatch path. | OOP iframe→parent: same-origin `targetOrigin` delivers; mismatched `targetOrigin` dropped; `"*"` delivers; `MessageEvent.origin` = sender displayed origin (opaque→`"null"`); `dispatch_message_event` docstring cites §9.3.3 step 8.7. |
| **2f4-d** | B16 in-process (§6.1 gap-close): `drain_parent_messages` in-process half (`take_pending_parent_messages` for InProcess entries). | In-process iframe→parent: same-origin delivers through the SAME gate; mismatch dropped. Test: an InProcess iframe `parent.postMessage(msg, parentOrigin)` reaches the parent `message` listener; `parent.postMessage(msg, "https://evil")` does not. |
| **2f4-e** | thread.rs:221 migration: `sandbox_flags`/`iframe_depth`→trait; `PipelineResult.credentialless`/`referrer` shell-owned fields (populated at build); read them in `handle_navigate`; delete `let bridge = …`. | Compiles; `content_iframe_security_tests.rs` sandbox/credentialless assertions green (after the §6.5 test-migration provides the shell-side reads). Dead-path caveat re-noted in code. |

**Acceptance gate.** After 2f4-e the **three production `bridge()` errors are closed** (grep
`\.bridge()` over `crates/shell/elidex-shell/src/**` non-test = 0). The §9.3.3 gate is the single
fail-closed chokepoint for both transports. **Test-migration co-requisite** (§6.5): the `#[cfg(test)]`
`bridge()` sites must be migrated (separate slice or bundled) before `mise run ci` passes.

**Push gate**: `mise run ci` + `/pre-push` (6-stage) + `/external-converge` (Codex). This is an
**edge-dense security keystone** (cross-origin-message-leak boundary), so the external pass is the
full convergence loop, not a single shot.

---

## §8 Carve list

- **`#11-browsing-context-model-window-open-postmessage`** (S5-8/B1, pre-existing registered slot,
  flip-memo B16 `:377` "replaced wholesale at S5-8/B1", `host_effects.rs:118-120`) — the real
  `WindowProxy` browsing-context targeting model replaces depth-routing + the flat-registry parent
  resolution. This memo's depth-routing + top-level-parent gate is the **boa-parity interim**; nested
  in-process iframe → intermediate-parent routing and structured (non-`ToString`) serialization ride
  this slot. (No new carve.)
- **`#11-oop-iframe-navigate-completeness`** (pre-existing, `thread.rs:228`) — the OOP `Navigate`
  handler's dead-path gaps: per-navigation referrer chain (§7.4.2) + cookie-jar/network-handle
  handoff. This memo compile-converges the `PreEvalFrameInputs` reads off `bridge()` but does **not**
  fix the runtime sourcing (already carved). (No new carve.)
- **`#11-shell-test-bridge-migration`** (NEW slice — **required, not deferred**; §6.5 RESOLVED) — the
  `#[cfg(test)]` `runtime.bridge()` reads (~47 bridge + ~8 coupled non-bridge, of the **57** `--tests`
  build errors post-2f-4; the 58th resolved as 2f-4 added the `referrer` field a coupled test expected)
  that must swap onto trait methods / shell-owned fields / observable assertions once boa is deleted.
  **Flip-required before the Stage 3 merge gate** (`mise run ci` compiles all tests). NOT indefinitely
  deferred: sequenced AFTER 2f-4 (needs its `referrer`/`credentialless` shell fields) and BEFORE
  Stage 3, as its own commit(s) in the flip branch — separated from 2f-4's edge-dense gate by
  blast-radius, not scope-dodge.
  - **✅ SCOPING RE-CONFIRMED MECHANICAL (2026-07-12, post-2f-4 investigation; an earlier draft of this
    note over-flagged it as design-bearing — corrected).** The eval-oracle infra ALREADY EXISTS:
    `ElidexJsEngine::vm(&mut self) -> &mut Vm` is **`pub`** (`engine.rs:71`), and under it `Vm::eval(src)
    -> Result<JsValue,_>` (`vm_api.rs:16`) + `Vm::console_messages() -> Vec<(String,String)>`
    (`vm_api.rs:119`) are both `pub`. `window.devicePixelRatio` / `matchMedia` read the
    `set_media_environment` facts (`vm_api_viewport.rs:34`) and `Vm::eval` reads globals with NO
    dom-binding, so post-build `runtime.vm().eval("String(window.devicePixelRatio)")` is a valid
    observable oracle. The ONLY infra gap is cross-crate value **marshalling** (`JsValue::String` is an
    interned `StringId` needing the string table, which shell can't reach) → add **two thin `pub`
    embedder-test oracles** on `ElidexJsEngine` mirroring the crate-internal `eval_string`
    (`tests_engine_s6a.rs:45`): `pub fn eval_string(&mut self, src: &str) -> String` +
    `pub fn eval_f64(&mut self, src: &str) -> f64` (marshal via the already-in-scope
    `self.vm().inner.strings`). That is a small addition, NOT a feature — **no plan-memo needed**.
    The 57 errors then split mechanical / observable-rewrite:
    - **Mechanical (~19)**: `.bridge().origin()` ×9 / `.sandbox_flags()` ×5 / `.set_origin()` ×2 →
      HostDriver trait methods (`runtime.origin()` etc. + `use elidex_script_session::HostDriver;` per
      test file); `.bridge().credentialless()` ×1 → `pipeline.credentialless` (2f4-e field);
      `.bridge().viewport_width/height()` → `pipeline.viewport.{width,height}`;
      `IframeToBrowser::PostMessage { data, origin }` pattern (`content_iframe_security_tests.rs:431`)
      → add `target_origin` (or `..`); `ContentState::new` (`content_test_support.rs:161`) → add the
      8th arg `crate::ipc::DeviceFacts::default()` (E0061).
    - **Observable-rewrite (~38) — the pub `runtime.vm()` oracle + two thin marshalling accessors**:
      1. **device-facts assertions** (`.bridge().device_pixel_ratio()` ×8 + `.color_scheme()` ×6, all
         in `viewport_tests.rs`): device facts reach the VM via a **fused setter with NO getter** (the
         shell-owned pattern, `engine.rs:551` `apply_device_facts(dppx, color_scheme, …)`), so there is
         **no trait getter to swap to**. Ideal (CLAUDE.md "Ideal over pragmatic" + "Supported-surface
         testing") = assert the **web-observable** value via JS eval (`window.devicePixelRatio`,
         `matchMedia('(prefers-color-scheme: dark)').matches`). Infra ALREADY present — `runtime.vm()`
         is pub → `runtime.vm().eval("String(window.devicePixelRatio)")`; add the two thin
         `eval_string`/`eval_f64` marshalling oracles (above) and rewrite the 14 assertions, e.g.
         `assert_eq!(rt.bridge().device_pixel_ratio(), 2.0)` → `assert_eq!(rt.eval_f64("window.devicePixelRatio"), 2.0)`;
         `assert_eq!(rt.bridge().color_scheme(), ColorScheme::Dark)` →
         `assert_eq!(rt.eval_string("matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'"), "dark")`.
      2. **`console_output().messages()` ×4** (`tests.rs`): boa's `ConsoleOutput::messages()` is gone;
         `runtime.vm().console_messages() -> Vec<(String,String)>` (`vm_api.rs:119`) is the drop-in
         (reachable via the pub `runtime.vm()`), then adapt the assertion `.iter().any(|m| m.1.contains(…))`
         to the `(level, message)` tuple shape (it already reads `m.1` = message — near-verbatim).
      3. **`p.runtime.eval(src, &mut session, &mut dom, document)` sites** (`content_window_open_tests.rs`,
         boa 4-arg signature): rewrite to `p.runtime.vm().eval(src)` (the VM eval reads globals with no
         `ScriptContext` needed — SIMPLER than the boa form).
      4. **`.bridge().scroll_y()` ×1 / `.set_pending_navigate_iframe()` ×1**: NOT on the trait NOR the
         shell (grep = none) — investigate per-site; likely assert removed/relocated surface (redirect
         to the shell scroll state / navigation queue, or delete if the asserted behaviour moved).
    - **Implication**: fully mechanical modulo the two thin oracles (T1) + the 14 observable rewrites +
      the 2 scroll/nav investigations. No plan-memo needed. Trait-method / shell-field / oracle targets
      all grep-verified against HEAD `fdebd93a` (2026-07-12).
- **`#11-pending-tasks-postmessage-step-renumber`** (NEW cite-sweep carve, plan-review Axis 4 MIN) —
  `pending_tasks.rs` carries stale FLAT §9.3.3 step numbers from an older spec revision that no longer
  exist in the current "window post message steps" algorithm (which tops at step 8 / sub-steps 8.1-8.7,
  webref-verified 2026-07-12): `:597` "step 9" (actually **step 8.1**, the same-origin gate), plus
  `:71`/`:326`/`:342`/`:467` cite "step 12"/"step 14"/"step 11"/"step 13". This memo's §3 correctly
  cites "step 8.1" for the inline depth-0 gate (`match_target_origin` `:596`) while the adjacent code
  comment says "step 9" — a reviewer following the memo hits the drift. **2f-4 does not modify the
  send-side** (`pending_tasks.rs` body unchanged), so this is a separate cite-sweep, not fixable
  on-touch here.
  - *Why deferred*: pre-existing S5-6a code drift (not introduced by 2f-4); 2f-4's touch is the
    receive-side (shell), not `pending_tasks.rs`; a whole-file step-renumber is out of this gate's scope.
  - *Re-evaluation trigger*: any PR that next modifies `pending_tasks.rs`'s postMessage path (D-17
    fix-on-touch), or a dedicated §9.3.3 cite-sweep.
  - *Re-evaluation date*: trigger-gated (next `pending_tasks.rs` postMessage touch).

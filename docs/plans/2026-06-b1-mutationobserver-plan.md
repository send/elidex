# B1 — VM `MutationObserver` record production + spec-faithful delivery (plan-memo)

Plan date: 2026-06-21 JST
Status: **PLAN / DESIGN — pre-implementation. No `.rs` change in this PR-of-record yet.**
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → **Program B (F3 reframe)**, the
canonical-path PR following the B0 audit.
Parent audit (SSoT, merged): `docs/plans/2026-06-scriptsession-mutation-path-audit.md` (B0, PR #367
`bb6d4389` + #374 `a2a9542e`). B0 establishes the factual map + names the coupled invariants; **this
memo resolves the §4.1 coupled-invariant corner and picks the mechanism** (B0 §4.6 hands that choice to
B1's `/elidex-plan-review`).
Gate: this memo must pass `/elidex-plan-review` (RULE — edge-dense subsystem, ≥3 intersecting invariant
axes: `synchronous-apply` × `ConsumerDispatcher` fan-out × `ScriptSession`-seam-ownership ×
record-coalescing × dual-runtime) **before** any implementation commit. Per CLAUDE.md "Edge-dense work =
multi-PR program + 実装前 plan-review 必須" and the umbrella. The B0→B1 overlay gates apply
(generator-layer / altitude check; pre-Ask written-lens — PR #377).

> **What B1 builds (scope ceiling).** B1 makes the **elidex-js VM**'s `MutationObserver` *fed by real DOM
> mutations* and *delivered on the spec's microtask checkpoint*, by (1) splitting record **production**
> (synchronous, at the engine-independent DOM-algorithm seam) from record **delivery** (the existing
> `Microtask::NotifyMutationObservers`, extended with WHATWG DOM §4.3 "notify mutation observers" steps
> 2–6), and (2) covering the **single-node childList family** (`appendChild` / `insertBefore` /
> `removeChild` / `replaceChild`) + migrating the already-coalesced `innerHTML` / `outerHTML` records onto
> the same delivery path. It is the **mechanism-proving slice**; it resolves §4.1 and establishes the one
> canonical record path. The remaining write-site families — direct tree ops + loop-coalescing (B1.2),
> characterData (B1.3), attribute-write convergence (B2 — B0 §4.5) — are **named follow-on slices, each its
> own `/elidex-plan-review`** (§5). B1 marks its own coverage boundary explicitly (§3.4, no silent cap).

> **Runtime caveat (read first).** The **production shell still runs boa** (`pipeline.rs` constructs
> `elidex_js_boa::JsRuntime`; S5/boa removal = D-26 PR7, not yet done). B1's canonical mechanism targets the
> **post-S5 VM**; it is exercised by VM tests and is **dormant in the production shell until S5** (consistent
> with matchMedia / Web-Animations VM-dormancy). boa keeps its existing, separate (incomplete) MO wiring
> until deleted — boa-specific paths are **known-to-differ, scope-out** per
> `memory/feedback_boa-findings-light-touch`. B1 does **not** touch `elidex-js-boa`.

---

## §A. Spec coverage map (preflight hard-gate)

> B1 implements **WHATWG DOM algorithm prose** (unlike A1, which was infra-only). The map names the
> algorithm sections B1 realizes + the engine site each attaches to. All anchors webref-verified
> 2026-06-21 (§10). B1's *covered observable surface* is deliberately bounded to single-node childList +
> innerHTML/outerHTML (the rest are follow-on slices, §5) — the map's "Full enum?" column states that
> boundary honestly.

| Spec section | Step / concept | Branch covered in B1 | Touch (engine site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| DOM §4.3 "notify mutation observers" (`#notify-mutation-observers`) | steps 2–6 (MO record callback delivery) | full (delivery is family-agnostic) | `vm/natives_promise.rs` `Microtask::NotifyMutationObservers` arm `:333` | ✓ (delivery covers all families at once) | no — records are engine-built |
| DOM §4.3 "queue a mutation observer microtask" (`#queue-a-mutation-observer-compound-microtask`) | flag + schedule | full | new VM `queue_mutation_record` (NEW) wrapper (production side) | ✓ | no |
| DOM §4.3.2 "queue a mutation record" (`#queue-a-mutation-record`) | ancestor-walk / kind / filter / oldValue gating | **already implemented** (`MutationObserverRegistry::notify`) — B1 only *feeds* it | `elidex-api-observers/src/mutation.rs` `notify` `:202` | ✓ (pre-existing, re-verified) | no |
| DOM §4.3.2 "queue a tree mutation record" (`#queue-a-tree-mutation-record`) | childList record shape | **single-node** appendChild/insertBefore/removeChild/replaceChild + innerHTML/outerHTML (coalesced) | `elidex-dom-api` child-node + `apply_set_inner_html`/`apply_set_outer_html` (return record) | ✗ — direct tree ops (ParentNode/ChildNode/insertAdjacent/select) + Range → **B1.2** (§5) | no |
| DOM §4.3.3 Interface MutationRecord (`#interface-mutationrecord`) | record fields | childList fields (target/addedNodes/removedNodes/prev+nextSibling) | `MutationRecord` (`mutation/mod.rs:135` / `mutation.rs:55`) | ✗ — `attributeNamespace` stays deferred to `#11-mutation-observer-extras` | no |
| DOM §4.2.3 "replace all" (`#concept-node-replace-all`) | suppressObservers → 1 coalesced record | innerHTML/outerHTML only (already coalesced by `apply_*`) | (reuse) | ✗ — replaceChildren / textContent-on-Element loop-coalescing → **B1.2** | no |

**Breadth**: K=1 spec (DOM); M=6 algorithm rows; observable surface B1 *covers* = single-node childList +
innerHTML/outerHTML (a deliberately narrow, mechanism-proving slice — coherent because none of these needs
*new* loop-coalescing: innerHTML/outerHTML already emit one coalesced record via `apply_*`, the four
single-node ops emit one record each). The follow-on families each re-run their own coverage map at their
plan time (§5). This is a **single-PR scope** by the edge-dense base-case rule (narrowly-scoped slice under
the approved umbrella).

**User-input flow**: none new. `MutationObserver.observe()` parsing (`MutationObserverInit`) already exists
(`mutation_observer.rs:212`, re-verified). B1 produces records from **engine-side** mutation algorithms; no
new untrusted-input parse or sanitization site is introduced. Attribute/character values flowing into
`oldValue` are already-stored DOM state, not freshly-parsed page input.

---

## §0. Decisions this memo commits to (the §4.1-corner resolution + B0-delegated calls)

These are **lens-collapsed** (Ideal-over-pragmatic + ECS-native-first + One-issue-one-way + Layering
mandate) and surfaced in §7 as Open Questions for the 5-agent gate to falsify — not unilateral closes.

1. **Mechanism = "synchronous write at the EcsDom chokepoint; record produced + enqueued synchronously at
   the engine-independent DOM-algorithm seam; callbacks delivered on the §4.3 microtask" (§2).** This is the
   structure B0 §4.4 hinted at (separate *where the write applies* from *where the record originates*),
   realized concretely. It satisfies all three §4.1 coupled invariants simultaneously (§2.4). **Both poles
   B0 enumerated are rejected as-is**: Pole A (MO as a `ConsumerDispatcher` consumer) is rejected because the
   `MutationEvent` stream is the wrong altitude for §4.3.3-correct records (§2.5); naive Pole B (record into
   the session buffer, apply at flush) is rejected because it breaks read-your-writes + re-forks the #181
   chokepoint (§2.6).

2. **Record-production locus = the engine-independent DOM-algorithm layer, which *returns* the record(s);
   `vm/host/` forwards them to the VM `queue_mutation_record` wrapper (marshalling-only).** This generalizes
   the **existing `innerHTML` pattern** (`apply_set_inner_html` returns a `MutationRecord`, the host
   delivers it — `dom_inner_html.rs:145-149`) to every family. Keeps the Layering mandate intact
   (`vm/host/` = marshalling; algorithm + record-shape in `elidex-dom-api` / `elidex-script-session`).
   For the families whose write-sites currently call `EcsDom::*` **directly** from `vm/host/`, the MO-record
   work *is* the Layering-convergence work — done family-by-family in the follow-on slices (§5), so MO
   coverage and the `vm/host/`-marshalling-only convergence land together, not as two passes.

3. **Delivery = extend the existing `Microtask::NotifyMutationObservers` arm; do NOT invent a new drain
   point (§2.3).** The arm already does §4.3 step 1 (clear `mutation_observer_microtask_queued`) + step 7
   (slotchange) — B1 adds steps 2–6 (MO callbacks). `MutationObserverRegistry::notify` already implements
   §4.3.2 (ancestor walk / gating); the **only** missing wiring is (a) §4.3.2 step 5 "queue a mutation
   observer microtask" at production and (b) §4.3 steps 2–6 in the drain. One drain point, spec-shaped.

4. **Retire the two synchronous `deliver_mutation_records` self-deliveries (innerHTML `:148`, outerHTML
   `:362`) in favour of the microtask path (§3.1, One-issue-one-way + spec timing fix).** Today innerHTML
   delivers callbacks **synchronously inside the setter** — a §4.3 timing bug (MO callbacks must run on the
   microtask checkpoint, never synchronously). Migrating them to `queue_mutation_record` both unifies the
   record path (no "direct-delivery vs buffered" fork — B0 §0) and fixes the timing.

5. **`deliver_mutation_records`'s conflated notify+callback is *split*, not removed; the `HostDriver` trait
   method is preserved (plan-review F3); the MO tests are rewritten to drive real JS mutations (§6, §7-AC).**
   `deliver_mutation_records` is a method on the `HostDriver` trait (`elidex-script-session/src/engine.rs:182`),
   implemented by the VM wrapper (`ElidexJsEngine`, `engine.rs:327` → `VmInner`) and exercised by the S1d smoke
   test (`tests_engine_s1d.rs:107`); **no production code calls the VM's trait impl** (the shell drives **boa**
   concretely via a *separate inherent 4-arg* `deliver_mutation_records`, `elidex-js-boa/src/runtime/observers.rs:20`
   → `content/mod.rs:258` — **boa does not implement the `HostDriver` trait**, verified). So B1 keeps the trait
   method (contract member) and **repoints the VM impl's internals** onto `queue_mutation_record` + the microtask
   (callbacks move from synchronous to the microtask checkpoint) — safe because no live caller depends on the
   VM impl's synchronous timing; **boa is untouched** (its inherent method is a different function that merely
   shares the name). This closes the B0-named **test-invisible gap** (current tests hand-build `SessionRecord`s
   and call `deliver_mutation_records` directly — none asserts "a JS mutation yields a record"); the rewritten
   tests drive real mutations + rely on the post-`eval` microtask drain (`interpreter.rs:41`, §6 item 9).

6. **B1 makes nothing observable that needs loop-coalescing it cannot yet do.** The four single-node ops
   each emit exactly one §4.3.2 record; innerHTML/outerHTML reuse the already-coalesced `apply_*` record.
   replaceChildren / textContent-on-Element / Range (which need §4.2.3 suppressObservers loop-coalescing)
   are **explicitly out of B1** → B1.2 (§5). This keeps B1 spec-correct *within its surface* rather than
   shipping an over-producing (N-records-where-1-is-owed) partial.

---

## §1. Verified anchors (re-grepped at HEAD `a2a9542e`, 2026-06-21)

Every site re-grepped against `main` HEAD this session (4-agent parallel grep-diff). **B0 anchor drifts are
recorded inline** (B0 is a snapshot; re-grep at PR-open per B0 §6).

### 1.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (engine-internal; NOT the MO path)

| Symbol / site | Verified location | Behavior / B0-drift |
|---|---|---|
| `EcsDom::set_attribute` | `core/elidex-ecs/src/dom/attribute.rs:101` (dispatch `:118`) | fires `MutationEvent::AttributeChange { old_value, new_value }` — **carries oldValue**; same-value writes still fire (DOM §4.3.2) |
| `EcsDom::remove_attribute` | `attribute.rs:258` (dispatch `:294`, gated `:287`) | fires only when `old_value.is_some()`; `rev_version` unconditional |
| `EcsDom::set_attribute_without_dispatch` | `attribute.rs:146` | **suppresses the entire fan-out** (incl. any MO record); used by value-mode migration + by consumers (re-entry guard) |
| tree fire-sites + shadow gate | `core/elidex-ecs/src/dom/tree/mutation.rs` `fire_after_insert:282` (gate `:289`, dispatch `:298`), `fire_after_remove:308` (gate `:343`, dispatch `:363`) | gate suppresses when node **or** parent is a shadow root; **no per-event `suppressObservers` flag** — granular per-node events |
| `EcsDom::dispatch_event` | `core/elidex-ecs/src/dom/mod.rs:191` | single `Box<dyn MutationDispatcher>`, take-and-restore, **re-entry guard** `debug_assert!(dispatch_depth==0)` → a dispatcher consumer cannot mutate/re-enter |
| `EcsDom::set_text_data` | `mod.rs:332` (write `:336`, dispatch `:340-344`) | **overwrites old text BEFORE notify**; `MutationEvent::TextChange` carries only `new_utf16_len` — **no old text** |
| `MutationEvent` enum (7 variants) | `core/elidex-ecs/src/dom/mutation_event.rs:107` | Insert/Remove/TextChange/ReplaceData/SplitText/NormalizeMerge/AttributeChange; **no namespace field** (deferred `#11-mutation-observer-extras`, prose `:293-298` — B0 said `:295-298`) |
| `ConsumerDispatcher` (7 consumers; verified 2026-06-21 — fields `consumer_dispatcher.rs:44-84`, dispatch `:141-147`) | **`crates/script/elidex-js/src/vm/consumer_dispatcher.rs`** — fields `:44-84`, dispatch `:141-147` | **B0 PATH DRIFT** — B0 cited `core/elidex-ecs/src/dom/consumer_dispatcher.rs` (does not exist); the dispatcher lives in the **VM crate** (relocated to break a cargo cycle when `FormControlReconciler` was added). 6 derived-state reconcilers + `CustomElementReactionConsumer` (a *script-visible* tap). **MO is NOT a consumer.** |
| dispatcher install | `crates/script/elidex-js/src/vm/vm_api.rs:279` (`Vm::bind`) | installed **only** here; **boa + shell install none** (re-confirmed: zero `set_mutation_dispatcher` in `elidex-js-boa`/`elidex-shell`; other repo hits are parser/teardown take-and-restore) |

> **Decisive layering fact (corrects B0).** Because `ConsumerDispatcher` lives in **`elidex-js`** (the VM /
> script-binding crate), and already hosts a *script-visible* tap (`CustomElementReactionConsumer`), Pole A
> would **not** put MO "inside the engine-internal `EcsDom` layer" — that part of B0 §4.1 invariant-3 framing
> is softened. Pole A is still rejected, but for the **record-altitude** reasons in §2.5, not a crate-locus
> reason.

### 1.2 Mechanism B — `SessionCore` buffer + flush; the MO registry + delivery

| Symbol / site | Verified location | Behavior / B0-drift |
|---|---|---|
| `SessionCore::record_mutation` | `crates/script/elidex-script-session/src/session.rs:79` | pure buffer (`pending.push`); applies later |
| `SessionCore::flush` | `session.rs:88` | drains `pending`, `apply_mutation` each, returns `Vec<Option<MutationRecord>>`; **does not deliver to MO** |
| `apply_set_attribute` (chokepoint-bypass) | `mutation/mod.rs:288-313` | mutates `Attributes` directly (NOT `EcsDom::set_attribute`), manually reconciles — **loses the consumer fan-out** (invariant 2 / #181 tension) |
| `apply_append_child` / `apply_insert_before` / `apply_remove_child` / `apply_replace_child` | `mutation/mod.rs:212` / `:232` / `:254` / `:268` | build childList `MutationRecord`s with exposed-sibling helpers (shadow-safe) |
| `apply_set_inner_html` / `apply_set_outer_html` | `elidex-script-session` (re-exported; used `dom_inner_html.rs:22`) | each returns **one coalesced** `MutationRecord` (replace-all shape) — the existing pattern B1 generalizes |
| `MutationObserverRegistry::notify` (§4.3.2) | `crates/api/elidex-api-observers/src/mutation.rs:202` | **already** does the inclusive-ancestor walk via `MutationObservedBy` component (registered-observer-list, §4.3 `#registered-observer`) + kind/attributeFilter/oldValue gating; **does NOT schedule the microtask** (§4.3.2 step 5 missing). Its own doc `:242-249` states the intended design: "*will surface … once ECS attribute-change events are translated into session MutationRecords*" |
| `MutationObservedBy` component | `mutation.rs:85` | ECS-native registered-observer-list (§4.3 `#registered-observer`), auto-cleaned on despawn |
| `MutationRecord` shape | session-side `mutation/mod.rs:135`; registry-side `mutation.rs:55` | both carry target/addedNodes/removedNodes/prev+nextSibling/attributeName/oldValue; **no `attributeNamespace`** (deferred); attr names lowercased |
| `VmInner::deliver_mutation_records` | `crates/script/elidex-js/src/vm/host/mutation_observer.rs:418` | **conflates** notify (`notify_one` loop `:431-433`) + callback delivery (`deliver_to_observer_callbacks` `:448`). B1 splits these. |
| `Microtask::NotifyMutationObservers` drain | `crates/script/elidex-js/src/vm/natives_promise.rs:333-344` | **slotchange-only stub**: clears `mutation_observer_microtask_queued` (step 1) + dispatches slotchange (step 7); **no MO callback delivery** (steps 2–6). B0 said `:342` = the slotchange line inside the arm; arm opens `:333`. |
| two flush paths | delivering: shell `re_render` (`lib.rs:606`) → `content/mod.rs:258`; MO-silent: `flush_with_ce_reactions` (`pipeline.rs:25`) | confirms B0 "not every flush delivers to MO" — **shell-side, boa-era; moot post-S5** |
| direct-delivery producers (2) | `dom_inner_html.rs:148` (innerHTML+setHTMLUnsafe), `:362` (outerHTML) | self-deliver **synchronously** (the §4.3 timing bug B1 fixes) |

### 1.3 The covered/mutator/gap sets (grep-diff conclusion)

- **Covered set today** (reaches `deliver_mutation_records`) = exactly the **2** direct producers
  (innerHTML/outerHTML/setHTMLUnsafe). The boa iframe `record_mutation`→flush path + the 2 dom-api
  `record_mutation` call-sites (`element/tree.rs:416`/`:476`) are **boa-era / unreachable from the VM**
  (the VM's `insertAdjacentHTML` is not even installed — `well_known.rs:341-342`; the bridge `SetInnerHtml`
  handler is bypassed by the VM which calls `apply_set_inner_html` directly). **The VM has ZERO
  `record_mutation` calls.**
- **Mutator set** (script-reachable DOM writes, `vm/host/` + `elidex-dom-api`) = the full §1.4 table.
- **Gap set = mutator set − covered set = essentially everything except innerHTML/outerHTML.** This is the
  B0 finding restated against live code: the VM MutationObserver is **largely silent**.

### 1.4 Write-site mutator set (re-grepped; route = bridge | direct)

> "bridge" = `dom_bridge::invoke_dom_api` → an `elidex-dom-api` `DomApiHandler`; "direct" = `EcsDom::*` from
> `vm/host/`. **ECS-event?** = does the underlying EcsDom primitive fire a `MutationEvent` (Mechanism A).
> **MO record today?** = uniformly **No** except innerHTML/outerHTML. B1's slice column = which slice covers it.

| Family / op | Site (vm/host unless noted) | Route | ECS-event? | Slice |
|---|---|---|---|---|
| `appendChild`/`removeChild`/`insertBefore`/`replaceChild` | `node_proto.rs:709/726/747/763` | bridge | yes | **B1** |
| `innerHTML`/`setHTMLUnsafe`/`outerHTML` | `dom_inner_html.rs:148/362` | (direct apply_*) | — | **B1** (migrate delivery) |
| ParentNode `prepend`/`append`/`replaceChildren` | `parentnode.rs:79/138/181` | direct | yes | B1.2 (+ replace-all coalescing) |
| ChildNode `before`/`after`/`replaceWith`/`remove` | `childnode.rs:269/367/441/534` | direct | yes | B1.2 |
| `insertAdjacentElement`/`Text` | `element_insert_adjacent.rs:230/269` | direct | yes | B1.2 |
| `<select>.add`/`remove`/`options.length=` | `html_select_proto.rs:691/833/871` | direct | yes | B1.2 |
| Range `deleteContents`/`extractContents`/`insertNode` | `range_proto_mutation.rs:73/102/125` | direct | yes | B1.2 (+ live-range) |
| `setAttribute` | `element_attrs.rs:218` | bridge (`element/props.rs:43`) | yes (AttributeChange) | **B2** |
| `removeAttribute`/`toggleAttribute`/`setAttributeNode`/`removeAttributeNode`/`Attr.value=`/`id=`/`className=`/NamedNodeMap | `element_attrs.rs:155/547/353/459`, `attr_proto.rs:416`, `named_node_map.rs:345/431` | direct | yes | **B2** (convergence) |
| reflected IDL setters (input/button/select/textarea/form/element/iframe/option/optgroup/fieldset/label/canvas) | `html_*_proto.rs` (many; see §3.3), `canvas/mod.rs:780` | direct | yes | **B2** |
| **`HTMLInputElement.value=`** | `html_input_value.rs:104-150` | direct (value-mode) | **mode-dependent** | **B2** — `SetLiveValue` arm must stay **record-free** (8kHF), only `SetContentAttr` arm owes a record |
| value-mode type migration | `elidex-form/src/value_mode.rs:222` (`set_attribute_without_dispatch`) | direct | **no (suppressed)** | **B2** — §4.9 record-owed-ness derivation |
| `data=`/`appendData`/`insertData`/`deleteData`/`replaceData` | `character_data_proto.rs:189/223/236/250/264` | bridge (`char_data_handlers.rs`) | Text: yes; **Comment: no** | B1.3 |
| `normalize` | `node_methods_extras.rs:270` | bridge | yes (NormalizeMerge+Remove) | B1.3 |
| `textContent=` / `nodeValue=` | `node_proto.rs:568/535` → `text_content.rs` | bridge | NodeKind-dependent | B1.3 (Text branch) / B1.2 (Element replace-all branch) |
| `splitText` | `text_proto.rs:95` (`split_text_at_offset:119`) | direct | yes (Insert+SplitText) | B1.3 |

---

## §2. The mechanism — resolving the §4.1 coupled-invariant corner

### 2.1 The data path (ideal, ECS-native)

```
JS mutation call (vm/host, marshalling-only)
   │
   ├─▶ engine-indep DOM algorithm (elidex-dom-api handler / elidex-script-session apply_*)
   │      • capture oldValue / sibling context  (BEFORE the write)
   │      • apply the write SYNCHRONOUSLY through the EcsDom chokepoint
   │        (EcsDom::set_attribute / append_child / set_text_data …)
   │        → fires MutationEvent → ConsumerDispatcher fan-out (UNCHANGED: reconcile + CE tap)
   │      • build the §4.3.3-shaped MutationRecord(s) (coalesced per §4.2.3 where applicable)
   │      • RETURN the record(s) to the caller
   │
   └─▶ VM queue_mutation_record(record)   [marshalling-only forward]
          • MutationObserverRegistry::notify(record)   (§4.3.2: ancestor-walk + gating + enqueue)
          • if any observer was enqueued → set mutation_observer_microtask_queued
            + push Microtask::NotifyMutationObservers   (§4.3.2 step 5 = "queue a mutation observer microtask")

… microtask checkpoint …
Microtask::NotifyMutationObservers   (§4.3 "notify mutation observers")
   1. clear mutation_observer_microtask_queued                 (already present)
   2–6. for each observer with pending records: take queue → invoke JS callback  (NEW)
   7. fire pending slotchange                                  (already present)
```

The write **applies synchronously at the chokepoint** (read-your-writes + fan-out preserved); the record is
**produced + enqueued synchronously at the algorithm seam** (correct oldValue/coalescing, seam-owned); the
**callback fires on the microtask** (correct §4.3 timing, runtime-uniform). This is exactly B0 §4.4's
"records the MO entry at the seam while keeping the synchronous write + dispatcher fan-out at the chokepoint".

### 2.2 Production side — `queue_mutation_record` (NEW)

New VM-side wrapper `Vm::queue_mutation_record` / `VmInner::queue_mutation_record` (NEW) (replaces the
notify-half of `Vm::deliver_mutation_records`, which exists at `vm_api.rs:867` and is retired — §3/§6):
- `notify_one(record)` → `MutationObserverRegistry::notify` (existing, §4.3.2).
- **`notify` is extended to report whether it enqueued anything** (return the count, or a bool). If >0,
  set `vm.mutation_observer_microtask_queued` (the existing flag) and push `Microtask::NotifyMutationObservers`
  — this *is* §4.3.2 step 5 "queue a mutation observer microtask" (idempotent via the flag). If 0 (the common
  no-observer case), skip scheduling (cheap — no empty microtask per unobserved mutation).
- Scheduling lives **VM-side** (the microtask queue is a VM concern); the registry stays engine-independent
  and VM-agnostic (no scheduling in `elidex-api-observers`).

### 2.3 Delivery side — extend `Microtask::NotifyMutationObservers`

The arm (`natives_promise.rs:333`) gains §4.3 steps 2–6 **before** the existing slotchange (step 7), reusing
the callback-invocation core currently in `deliver_mutation_records` (`observers_with_records` →
`deliver_to_observer_callbacks` → `take_records` → `build_mutation_records_array`, `mutation_observer.rs:442-469`).
The re-entrancy discipline already encoded for slotchange (clear flag first, re-entrant `observe`/`disconnect`
from callbacks see post-take state via the up-front id snapshot — `:435-446`) carries over verbatim. **One
microtask delivers both MO callbacks and slotchange**, matching §4.3 (which interleaves them in one algorithm).

### 2.4 Why this satisfies all three §4.1 coupled invariants

1. **`synchronous-apply` (read-your-writes)** ✓ — the write still applies at the EcsDom chokepoint inside the
   synchronous native call; only **callback delivery** is deferred to the microtask (which is *required* by
   §4.3, not a regression). The record **enqueue** is synchronous too, so `oldValue`/siblings are captured at
   the correct instant.
2. **`ConsumerDispatcher` fan-out preservation** ✓ — writes go through `EcsDom::set_attribute` /
   tree primitives unchanged; the 7 consumers (verified 2026-06-21, incl. CE) still fire. B1 adds **no** dispatcher consumer and
   touches **no** `EcsDom` primitive. It explicitly does **not** route writes through the buffered
   `apply_set_attribute` (which bypasses the chokepoint — invariant 2 / #181).
3. **`ScriptSession`-seam-ownership** ✓ — record production is at the engine-independent DOM-algorithm seam,
   enqueued into the seam-owned `MutationObserverRegistry`, delivered at the seam's microtask checkpoint;
   `vm/host/` stays marshalling-only. ADR #17 ("the seam owns MO records") is honored without ADR #14 being
   mis-read as "put MO production in the `EcsDom` layer" (B0 §5).

### 2.5 Why NOT Pole A (MO as a `ConsumerDispatcher` consumer)

Even though the dispatcher lives in `elidex-js` (so the crate-locus objection is moot — §1.1), Pole A is
**wrong altitude** for §4.3.3-correct records:
- **Coalescing impossible without a transaction boundary the event stream lacks.** §4.2.3 replace-all emits
  **one** record by removing/inserting children with `suppressObservers=true` and queuing a single tree
  record. The EcsDom layer fires **N granular** Insert/Remove `MutationEvent`s with no `suppressObservers`
  notion (`mutation.rs` has no such flag). A per-event consumer would over-produce (N records where 1 is
  owed) unless `suppressObservers` batching is pushed **into `elidex-ecs`** — re-introducing the invariant-3
  inversion at the algorithm-substrate level.
- **`oldValue` not in the event stream.** `TextChange` carries only `new_utf16_len` (`set_text_data`
  overwrites before notify). `characterDataOldValue` would require either re-reading (already-overwritten) or
  widening every text event with an allocation even when unobserved.
- **Comment/PI characterData fires no event at all** (the Comment branch only bumps `rev_version`,
  `char_data_handlers.rs:59-73`) → a consumer would silently miss it.
- The re-entry guard (`debug_assert!(dispatch_depth==0)`) means a consumer must *queue*, not act — so even
  Pole A would need a separate delivery step; it buys nothing over producing at the algorithm.

Producing at the algorithm seam sidesteps all four (coalescing granularity known; oldValue captured pre-write;
Comment handled at the handler that knows it's a characterData op; no re-entry concern).

### 2.6 Why NOT naive Pole B (record into the session buffer, apply at flush)

- Breaks **invariant 1**: a synchronous `setAttribute(); getAttribute()` cannot defer the apply to flush.
- The buffered `apply_set_attribute` **bypasses the chokepoint** (invariant 2 / #181) — re-forking the write
  path lesson #181 collapsed.
- The session buffer + flush is retained **only** for the genuinely-deferred fragment path (innerHTML/outerHTML
  apply synchronously *then* return a record — they do not buffer-and-defer the DOM write), and for boa until
  S5. B1 does **not** extend `record_mutation`-buffering to per-op attribute/tree/text writes.

> **The resolving move** is precisely *"record at the seam, write at the chokepoint, deliver at the
> microtask"* — neither pole as-is. The record's **content** comes from the algorithm (Pole-B-like ownership);
> the write's **application** stays at the chokepoint (Pole-A-like synchronicity); delivery is the spec's
> microtask (neither pole addressed timing).

---

## §3. Record production — B1's covered surface

### 3.1 innerHTML / outerHTML (migrate delivery, no shape change)

`dom_inner_html.rs:148`/`:362`: replace `ctx.vm.deliver_mutation_records(&[rec])` with
`ctx.vm.queue_mutation_record(rec)`. The record is unchanged (already a coalesced replace-all childList
record from `apply_set_inner_html`/`apply_set_outer_html`). Effect: callbacks move from synchronous-in-setter
to the microtask checkpoint (§4.3 timing fix) and onto the unified path (One-issue-one-way: the
"direct-delivery producer" category from B0 §0 is dissolved).

### 3.2 Single-node childList (appendChild / insertBefore / removeChild / replaceChild)

These already route **bridge** → an `elidex-dom-api` child-node handler (`node_proto.rs:709/726/747/763`).
Today the handler applies the tree mutation through the EcsDom primitive (firing Insert/Remove) but produces
**no** `MutationRecord` (the on-point slot reference for the single-node gap is `document.rs:251`, which cites
the unbuilt `#11-tree-mutation-record-pipeline` — **which B1 implements for this family**; the
`child_node/mutations.rs` doc-comment describes the *direct* ParentNode/ChildNode family = B1.2, and stays
accurate after B1). B1:
- The handler **captures the sibling context before the write** and, after applying through the chokepoint,
  produces a §4.3.2 "queue a tree mutation record" childList record (`addedNodes`/`removedNodes` +
  `previousSibling`/`nextSibling`, using the existing exposed-sibling helpers so shadow roots never leak as
  siblings — `apply_append_child`/`apply_insert_before` already model this shape and are the reuse target).
- `replaceChild` = **one** coalesced childList record (`addedNodes:[new]`, `removedNodes:[old]`), per
  §4.2.3 "replace" (`#concept-node-replace`) step 14 "queue a tree mutation record … with nodes, removedNodes,
  previousSibling, and referenceChild" (the inner remove + insert run with `suppressObservers=true`,
  webref-verified 2026-06-21). B1 reuses the existing `apply_replace_child` (`mutation/mod.rs:268`), which
  already builds exactly this one-record shape (`added_nodes:[new_child]`, `removed_nodes:[old_child]`). So
  replaceChild stays 1:1 (one op → one record) — no special handling.

> **The record out-channel (plan-review F2 — committed mechanism).** `DomApiHandler::invoke` returns
> `Result<JsValue, DomApiError>` (verified `elidex-dom-api/src/element/tree.rs:23-29`) — there is **no**
> MutationRecord return path today (the only existing producer, innerHTML, does *not* use the bridge; it calls
> `apply_set_inner_html` directly in the host and gets the record back as a value, `dom_inner_html.rs:145-149`).
> B1 commits the **ScriptSession-seam-faithful** channel (dry-run candidate (b), §4.1 invariant 3): a
> **session-owned `notify_records: Vec<MutationRecord>` scratch** on `SessionCore` — **distinct from the
> deferred `pending` apply-buffer** (it holds records for *already-applied* synchronous ops, so it does **not**
> re-introduce Pole B deferred-apply). The engine-independent child-node handler pushes its produced record
> onto `session.notify_records` (the handler already has `&mut` session/dom via the bridge ctx). The VM
> **drains `notify_records` once, at the `invoke_dom_api` boundary** (`dom_bridge.rs`, after the handler
> returns) and calls `queue_mutation_record` for each — a **single host-side drain point** covering every
> bridge op uniformly (no per-op host code; One-issue-one-way). `vm/host/` stays marshalling-only (push is in
> the engine-indep handler; drain is mechanical forwarding). The direct-apply host path (innerHTML/outerHTML)
> already holds the record as a value and calls `queue_mutation_record` directly — same sink, two feeders that
> converge on `queue_mutation_record`. *(This refines the earlier "bridge return channel" phrasing, which did
> not name the seam; the channel is the session scratch + bridge-boundary drain, not a widened `invoke` return
> type.)*

### 3.3 Explicitly NOT in B1 (the coverage boundary — no silent cap)

B1 leaves these **MO-silent** and says so (logged in the AC + a code comment at the seam, per
"supported-surface testing" / no-silent-caps):
- **Direct tree ops** (ParentNode/ChildNode/insertAdjacent/select) and **all loop-coalescing** ops
  (replaceChildren, textContent-on-Element, Range) → **B1.2**.
- **Attributes** (setAttribute + the direct attribute-write surface) → **B2**.
- **characterData** (data/appendData/…/normalize/splitText/textContent-on-Text/nodeValue, incl. the Comment
  branch + `characterDataOldValue` capture-timing + live-range coupling) → **B1.3**.
- **`attributeNamespace`** record field + `observe()` primitive-`ToObject` → stays `#11-mutation-observer-extras`.
- **Transient registered observers** (§4.3 step 6.3 / `#transient-registered-observer`, for subtree tracking
  of removed nodes) — open slot **`#11-mutation-transient-observers`**; the current registry models only
  persistent `MutationObservedBy`. B1 does not add transient observers; picked up by B1.2 (coupled to the
  removal record path). Flagged, not silently dropped.

### 3.4 Coverage-boundary honesty

B1 ships a **complete mechanism** + a **deliberately partial observable surface**. The AC asserts the covered
ops deliver correct records *and* that the uncovered families are tracked (the follow-on slices exist with
named scope). This is the supported-subset discipline, not a silent truncation.

---

## §4. The §4.2/§4.3 named invariants — disposition

B0 §4.2/§4.3 named invariants; B1's mechanism choice fixes how each is satisfied (those in B1's surface) or
where it is owed (deferred slices). Listed so the plan-review can confirm none is mis-handled.

| Named invariant (B0) | Disposition in B1's mechanism |
|---|---|
| `synchronous-apply` (read-your-writes) | §2.4(1) — write at chokepoint synchronous; only callback deferred (spec-required). |
| `ConsumerDispatcher` fan-out preservation | §2.4(2) — untouched; no consumer added, no primitive changed. |
| `ScriptSession`-seam-ownership | §2.4(3) — production at algorithm seam, registry seam-owned, microtask delivery. |
| `record-shape & coalescing` (§4.2.3) | **B1 surface**: single-node = 1 record each; innerHTML/outerHTML reuse coalesced `apply_*`. **Loop-coalescing (replaceChildren / textContent-Element / Range) = B1.2** (where suppressObservers→1-record is derived). B1 introduces no over-producing op. |
| `non-dispatching attribute write` (`set_attribute_without_dispatch`, §4.9) | **B2** — the value-mode migration's record-owed-ness (§4.9 "handle attribute changes") vs the `input.value` live-value write that must stay record-free (8kHF) is a B2 derivation; B1 does not touch attributes. |
| `move-record` / CE-timing / shadow-root boundary | **shadow**: delivery gating is in `notify`'s ancestor walk (re-verified §1.2) + exposed-sibling helpers; B1's single-node records inherit it. **move-record** (already-parented insert) + **CE-timing across moves** = B1.2 (move semantics live with the direct tree ops). |
| `characterData oldValue capture-timing` | **B1.3** — capture old data *before* `set_text_data` at the char-data handler (since records are produced at the algorithm, not the event, no `EcsDom` primitive change is needed). |
| `characterData + live-range` coupling (§4.10 ↔ §5.5) | **B1.3** — the char-data handler that produces the record also drives the live-range adjustment (already on the dispatcher path); B1.3 unifies them. |
| `boa buffered iframe write` | scope-out (boa, S5). B1 touches no boa path. |
| `dual-runtime delivery` | B1's microtask path is VM-only; boa keeps its separate shell-driven delivery until S5. The VM path is runtime-uniform for the post-S5 world. Stated, not silently assumed. |
| CE-reaction preservation (Mechanism B not MO-only) | B1 changes **delivery wiring**, not record *production* for CE: CE reactions ride Mechanism A (ConsumerDispatcher, VM) + the boa flush-scan — neither is altered by B1 (B1 adds `queue_mutation_record` alongside, does not reroute CE). Re-verified: CE has dual drivers (`consumer.rs:56` + boa `ce.rs:145`); B1 touches neither. |

---

## §5. Slicing — the B1 program (each slice `/elidex-plan-review`-gated)

Per CLAUDE.md edge-dense multi-PR rule. B1 (this memo) resolves §4.1 + ships the mechanism; the follow-ons
are **terminal base-case slices under this approved umbrella**, each with its own plan-review before impl.

| Slice | Scope | Key coupled invariants it owns | Sequencing |
|---|---|---|---|
| **B1** (this memo) | `queue_mutation_record` + microtask delivery (steps 2–6) + retire synchronous deliver + single-node childList + innerHTML/outerHTML migration + **rewrite MO tests to drive real mutations** | mechanism (all 3 §4.1) + single-node childList shape | **first** (mechanism gate) |
| **B1.2** | direct tree ops (ParentNode/ChildNode/insertAdjacent/select) + **replace-all loop-coalescing** (replaceChildren / textContent-on-Element) + Range mutations + **move-record** semantics + transient registered observers (`#11-mutation-transient-observers`) | §4.2.3 suppressObservers coalescing; move/shadow; live-range (Range) | after B1 |
| **B1.3** | characterData (data/appendData/insertData/deleteData/replaceData/normalize/splitText/textContent-Text/nodeValue) + **Comment/PI branch** + `characterDataOldValue` capture-timing + §4.10↔§5.5 live-range coupling | oldValue capture-timing; characterData+live-range | after B1 (parallel-able with B1.2) |
| **B2** (B0 §4.5) | attribute-write convergence: setAttribute + the direct attribute surface → record-producing engine-indep algorithms; value-mode `input.value` record-free boundary; §4.9 migration record | reflected-IDL-setter recording; VM-local Attr-detach; #181 convergence | after B1 (B0 §4.6: B2 target shape depends on B1 mechanism) |

Each follow-on inherits B1's mechanism verbatim (production-returns-record + `queue_mutation_record` +
microtask) and adds only its family's algorithm + the convergence of its direct write-sites onto
engine-independent record-producing handlers (the MO-coverage and Layering-convergence are the same work).

---

## §6. File-level change plan (B1 only; post-review commit)

> Listed for blast-radius scoping. No code until review passes. Re-grep §1 at PR-open.

1. **`crates/api/elidex-api-observers/src/mutation.rs`** — `notify` returns whether it enqueued (count or
   bool), so the VM can decide to schedule the microtask (§9 Q6: cheaper than a `has_pending_records()` delta
   walk). No behavior change to the gating logic.
2. **`crates/script/elidex-script-session/src/session.rs`** — add the **`notify_records: Vec<MutationRecord>`
   scratch** on `SessionCore` (F2 channel — distinct from `pending`) + a `push_notify_record` / `take_notify_records`
   pair. Records here are for *already-applied* synchronous ops (no deferred apply).
3. **`crates/dom/elidex-dom-api/`** (child-node handlers for appendChild/insertBefore/removeChild/replaceChild) —
   capture siblings before the EcsDom write, apply through the chokepoint, **push** the §4.3.2 tree record onto
   `session.notify_records` (reuse `apply_append_child`/`apply_insert_before`/`apply_replace_child` shapes).
   Engine-independent (record-shape stays out of `vm/host/`).
4. **`crates/script/elidex-js/src/vm/host/mutation_observer.rs`** — **split** `deliver_mutation_records`'s
   internals: keep `notify_one`; extract the callback-invocation block (`:442-469`) into a reusable
   `deliver_pending_mutation_records(vm)` the microtask calls. Add `VmInner::queue_mutation_record(record)`
   (NEW) = `notify_one` + (if enqueued) set flag + push `Microtask::NotifyMutationObservers`.
5. **`crates/script/elidex-js/src/vm/host/dom_bridge.rs`** — at the `invoke_dom_api` boundary (after the handler
   returns), **drain `session.notify_records` once** → `queue_mutation_record` per record. Single host-side drain
   covering every bridge op (mechanical forward; `vm/host/` marshalling-only).
6. **`crates/script/elidex-js/src/vm/vm_api.rs`** + **`crates/script/elidex-js/src/engine.rs`** — public
   `Vm::queue_mutation_record` (NEW; host-facing marshalling forward). **Do NOT remove the `deliver_mutation_records`
   `HostDriver` trait method** (F3: it is a member of the `HostDriver` trait `elidex-script-session/src/engine.rs:182`,
   impl'd only by the VM `ElidexJsEngine` `engine.rs:327` + exercised by the S1d test `tests_engine_s1d.rs:107`;
   **no production caller drives the VM impl** — the shell drives boa's *separate inherent 4-arg* method, not this
   trait method; **boa does not implement `HostDriver`**). Instead, **repoint the VM impl's internals** onto
   `queue_mutation_record` + the microtask (callbacks no longer fire synchronously) — safe, no live caller
   depends on the old synchronous timing; boa is untouched. `mutation_observer_microtask_queued` (slotchange
   flag) is confirmed the right flag to reuse (it already gates this microtask). **Size note (F11)**: `vm_api.rs`
   is 1192 lines; B1's net change is ~neutral (thin add + internal repoint, no large block) — no split needed.
7. **`crates/script/elidex-js/src/vm/natives_promise.rs`** — extend the `NotifyMutationObservers` arm (`:333`)
   with §4.3 "notify mutation observers" steps 2–6 (call `deliver_pending_mutation_records`) **before** the
   slotchange step (step 7), preserving the clear-flag-first re-entrancy discipline (`:341`).
8. **`crates/script/elidex-js/src/vm/host/dom_inner_html.rs`** — `:148`/`:362`: `deliver_mutation_records(&[rec])`
   → `queue_mutation_record(rec)`.
9. **Tests** — rewrite `crates/script/elidex-js/src/vm/tests/tests_mutation_observer/*` to **drive real JS
   mutations** (`appendChild`/`removeChild`/`insertBefore`/`replaceChild`/`innerHTML`) and assert the delivered
   records — closing the test-invisible gap. The microtask is **already drained** at end of each `eval`
   (`interpreter.rs:41` `drain_microtasks()`, def `natives_promise.rs:274`), so a JS mutation's callback fires
   before the post-`eval` read (F4 — no new drive helper needed; AC asserts no synchronous-in-setter delivery by
   reading state *mid-eval* via a second eval). Keep one synthetic-record unit test for the registry gating (or
   move it to `elidex-api-observers`).

**Files B1 does NOT touch**: any `EcsDom` primitive (`elidex-ecs`); `elidex-js-boa` (boa scope-out); the
attribute / characterData / direct-tree write-sites (B2 / B1.3 / B1.2); the shell `content/mod.rs` /
`pipeline.rs` boa delivery (boa-era, S5). `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` is **read,
not modified** (no MO consumer added).

---

## §7. Testing / Acceptance criteria

1. **A JS mutation yields the correct record on the microtask checkpoint** (the B0 test-invisible gap closed):
   - `observe(parent,{childList:true})` + `parent.appendChild(c)` → after a microtask checkpoint, one
     `childList` record with `addedNodes:[c]`, correct `previousSibling`.
   - `insertBefore` / `removeChild` analogues; `replaceChild` → **one** coalesced record (added+removed, §3.2).
   - `subtree:true` on an ancestor receives the record (ancestor-walk); a non-subtree ancestor does not.
   - `el.innerHTML = "…"` / `el.outerHTML = "…"` → one coalesced record, **delivered on the microtask, not
     synchronously inside the setter** (assert no records visible before the checkpoint).
2. **Microtask timing** — callbacks fire exactly once per checkpoint even across many mutations (idempotent
   `queue a mutation observer microtask` via the flag); re-entrant `observe`/`disconnect` from a callback sees
   post-take state (carry-over of the existing slotchange discipline).
3. **No double-delivery / no regression** — innerHTML/outerHTML no longer self-deliver synchronously (the VM's
   `deliver_mutation_records` trait impl is repointed onto the queue+microtask path, not removed — F3);
   slotchange still fires from the same microtask; CE reactions unaffected (separate driver). Existing
   VM/slotchange suites pass; boa's *separate inherent* `deliver_mutation_records` + the shell's call site are
   untouched.
4. **Coverage-boundary asserted** — a test (or doc-comment + tracking) confirms the uncovered families
   (attributes/characterData/direct-tree) are still silent **and** tracked to B1.2/B1.3/B2 (no silent cap).
5. **No-observer fast path** — a mutation with no registered observer schedules no microtask (assert the
   microtask queue stays empty).

`mise run ci` green; per-crate `cargo test -p elidex-api-observers -p elidex-script-session -p elidex-js
--all-features`.

---

## §8. Collision / sequencing

- **A1 (PR #376, `webapi-compat-a1`) — ZERO file overlap** (verified 2026-06-21 via `gh pr view 376 --json
  files`). A1 edits `elidex-plugin/{lib,spec_level}.rs` / `elidex-dom-api/registry.rs` / `elidex-js/Cargo.toml` /
  `engine.rs` / `lib.rs` / `vm/{globals,init,mod,sw_thread,worker_thread}.rs` / `vm/host/{window,worker}.rs` +
  tests. B1 = `mutation_observer.rs` / `vm_api.rs` / `natives_promise.rs` / `dom_inner_html.rs` /
  `dom_bridge.rs` / `node_proto.rs` / `session.rs` / `elidex-dom-api` child-node handlers / `elidex-api-observers`.
  **No file is in both sets** (the earlier "`vm_api.rs` shared" claim was wrong — A1 does not touch `vm_api.rs`).
  Independent. **Re-confirm at PR-open** (A1 may have merged).
- **media Slice3** (CSS at-media cascade / shell `prefers-*` producer) — no overlap by scope; not yet a
  populated branch (the local `media-query-slice3-cascade` holds unrelated work), so the no-overlap claim rests
  on scope, not a diff.
- **`#11-tree-mutation-record-pipeline` (3-facet slot — partial absorption, mechanism superseded).** The slot
  bundles (a) dynamic-iframe load on scripted `appendChild`, (b) custom-element connected/disconnected callbacks
  on scripted tree mutations, **and (c) MutationObserver childList**. B1 closes **only facet (c) for the
  single-node family**, and via a **different mechanism** than the slot's original "route through
  `session.record_mutation`" note (B1 uses production-returns-record + `queue_mutation_record`, §2.6). So at
  PR-open: update the slot to **not** mark it closed-by-B1 — record that facet (c)'s single-node half landed
  (mechanism superseded), facet (c)'s direct-tree half → B1.2, and facets (a)/(b) remain owed (iframe-load +
  scripted-CE-callback, not B1.2-tree-op work). Update the references (`document.rs:251`, `content_tests.rs:464`)
  accordingly.
- **Worktree isolation** — implementation builds in the dedicated worktree `elidex-b1-mo` off `origin/main`
  (this memo's branch `b1-mutationobserver`). main-direct commit forbidden.
- **Dual-runtime dormancy** — B1's path runs in the VM only; the production shell (boa) gains it at S5. Stated
  in the PR description so reviewers don't expect production-shell behavior change.

---

## §9. Open questions + resolutions

> **Status after `/elidex-plan-review` (2026-06-21, 5-agent, 0 CRIT / 3 IMP / 8 MIN).** The review **confirmed
> the mechanism** (Q1/Q2 — all 5 axes found the record-at-seam / write-at-chokepoint / deliver-at-microtask
> split sound: Layering-clean, ECS-native, valid edge-dense slice, spec-faithful) and **resolved** Q3/Q4/Q5/Q6.
> Step 4.5 re-check verified the F2 channel (Layering + data-flow CLEAN) and corrected the F3 rationale.

1. **Mechanism (§2) — RESOLVED (review-confirmed).** Production-returns-record + `queue_mutation_record` +
   microtask is the faithful §4.1 resolution; no axis found a dominating fourth structure. Both poles rejected
   as in §2.5/§2.6.
2. **Delivery locus — RESOLVED (review-confirmed).** Extending the existing `Microtask::NotifyMutationObservers`
   arm is spec-faithful (§4.3 "notify mutation observers" puts MO callbacks + slotchange in one algorithm), not
   a shortcut.
3. **`deliver_mutation_records` trait — RESOLVED (F3 + Step 4.5).** Keep the `HostDriver` trait method (member
   of the contract, impl'd by the VM + S1d test); **repoint only the VM impl's internals** onto queue+microtask.
   No production caller drives the VM impl (the shell drives boa's *separate inherent 4-arg* method, not the
   trait); boa is untouched. Removal would break the trait contract; repoint is safe.
4. **`replaceChild` record count — RESOLVED (F1, webref-verified).** §4.2.3 "replace" (`#concept-node-replace`)
   step 14 queues **one** coalesced tree record (added+removed); inner remove/insert run with
   `suppressObservers=true`. B1 reuses `apply_replace_child` (already one-record). The earlier "two records"
   reading was a spec error caught at plan-review.
5. **Slice boundary — RESOLVED (review-confirmed, F-Axis3/5 FP).** "Single-node childList + innerHTML/outerHTML"
   is a valid edge-dense base-case terminal slice (Axis 3 + Axis 5 confirmed the B1.2/B1.3/B2 deferrals are
   cross-PR scope boundaries with explicit slots, not scope-cuts). The `appendChild`-observed /
   `el.append()`-silent asymmetry is logged (§3.3 / §7-AC4), not hidden. B1 stays mechanism-pure + narrow.
6. **`notify` return-for-scheduling — RESOLVED (F-Axis2 FP, §6 item 1).** Threading "did-enqueue" out of
   `notify` is the clean choice (avoids a separate `has_pending_records()` delta walk); scheduling stays
   VM-side, registry stays engine-independent. Not a side-store violation (observer-keyed queues, not per-entity).

---

## §10. Citation appendix (webref-verified 2026-06-21)

| Concept | Source | Anchor |
|---|---|---|
| notify mutation observers (steps 1–7) | DOM §4.3 | `#notify-mutation-observers` |
| queue a mutation observer microtask | DOM §4.3 | `#queue-a-mutation-observer-compound-microtask` |
| registered observer / registered observer list | DOM §4.3 | `#registered-observer` |
| transient registered observer | DOM §4.3 | `#transient-registered-observer` |
| queue a mutation record (gating algorithm) | DOM §4.3.2 | `#queue-a-mutation-record` |
| queue a tree mutation record | DOM §4.3.2 | `#queue-a-tree-mutation-record` |
| Interface MutationRecord (record fields) | DOM §4.3.3 | `#interface-mutationrecord` |
| replace all (suppressObservers → 1 record) | DOM §4.2.3 | `#concept-node-replace-all` |
| handle attribute changes (B2) | DOM §4.9 | `#handle-attribute-changes` |
| CharacterData replace data (B1.3) | DOM §4.10 | `#concept-cd-replace` |
| live range / live range pre-remove steps (B1.2/B1.3) | DOM §5.5 | `#concept-live-range` / `#live-range-pre-remove-steps` |

> Design SSoT cross-refs (B0 §5): `docs/design/ja/12-dom-cssom.md` §12.1.2 (seam-recorded MO);
> `docs/design/ja/28-adr.md` ADR #17 (seam owns MO records — invariant 3) / ADR #14 (MO ↔ ECS change-detection
> *substrate*, not a license to produce in `EcsDom`); lesson #181 (`attribute.rs:5-15` chokepoint, in tension
> with naive seam-only routing — honored by writing at the chokepoint, §2.4(2)).

---

## §11. As-built notes (implementation — `b1-mutationobserver`)

Recorded so plan and landed code agree. Three refinements; none changes B1's contract (mechanism,
single-node childList coverage, microtask delivery for JS mutations). `mise run ci` green; new + existing MO
suites pass (51 existing + 7 new integration).

1. **`apply_*` builders made `pub` and reused by the VM handlers (refines §3.2 / §6 item 3).** The four
   `apply_append_child` / `apply_insert_before` / `apply_remove_child` / `apply_replace_child`
   (`mutation/mod.rs`) already apply-through-the-chokepoint **and** build the record; the dom-api child-node
   handlers (`element/tree.rs`) now call them directly (replacing their bare `dom.append_child` calls) and
   `session.push_notify_record(record)`. One record source shared by the deferred-flush path (`apply_mutation`)
   and the synchronous bridge path — *One issue, one way*. (`Mutation::AppendChild`-family records are
   unwired in production, so no double-apply.)

2. **The embedder API `deliver_mutation_records` stays SYNCHRONOUS; only the internal JS-mutation path defers
   (refines §0.5 / §6 item 6 / §9 Q3).** The plan said "repoint the VM trait impl onto queue+microtask". As
   built, `VmInner::deliver_mutation_records` keeps its synchronous contract (`notify_one` each +
   `deliver_pending_mutation_records()` now) — the **embedder's call site *is* its chosen checkpoint** (mirrors
   how the shell drives boa's delivery post-layout). The split is by *caller*, not two-ways-to-do-one-thing:
   internal producers (innerHTML/outerHTML/bridge childList) use `queue_mutation_record` (→ §4.3 microtask,
   deferred — the spec timing fix); the host-driver entry delivers when called. This keeps the 22 synthetic
   gating tests valid (they cover attribute/characterData record *shapes* B1 cannot yet produce via real
   mutations) and required **no `engine.rs` change** (the trait impl delegates unchanged). `deliver_pending_mutation_records`
   is the extracted callback-delivery half shared by both the synchronous API and the microtask arm.

3. **Record out-channel = `SessionCore.notify_records` + bridge drain + flush leak-guard (refines §3.2 F2
   callout / §6 items 2/5).** As planned: handlers push to the session scratch; `invoke_dom_api`
   (`dom_bridge.rs`) drains it once via `host_data.session().take_notify_records()` → `ctx.vm.queue_mutation_record`.
   **Added leak-guard**: `SessionCore::flush` clears `notify_records` — because boa **also** routes
   `appendChild` through these dom-api handlers (`globals/element/core.rs` → `dom_registry().resolve`) but never
   drains the scratch; without the per-turn clear boa would accumulate records forever. The VM drains
   synchronously per bridge op, so the guard is a no-op for it. (Removed at S5 with boa.)

4. **Microtask delivery (as planned).** `Microtask::NotifyMutationObservers` arm (`natives_promise.rs`) extended
   with §4.3 steps 2–6 (`deliver_pending_mutation_records`) before the slotchange step 7; the existing
   `mutation_observer_microtask_queued` flag (shared with slotchange) coalesces one microtask per checkpoint.
   `notify` (`elidex-api-observers`) now returns whether it enqueued, so `queue_mutation_record` skips
   scheduling for the no-observer common case.

5. **Tests (refines §6 item 9).** Existing 51 MO tests pass unchanged (the synchronous embedder API preserved
   their semantics). New `tests_mutation_observer/integration.rs` drives **real JS mutations** end-to-end:
   appendChild/insertBefore/removeChild/replaceChild/innerHTML/subtree + the no-observer fast path + microtask
   deferral probes (callback NOT synchronous in the setter) + the §4.2.3 single-coalesced-record assertion for
   replaceChild. This closes the B0 test-invisible gap.

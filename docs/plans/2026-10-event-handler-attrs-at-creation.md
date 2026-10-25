# EH-attrs-at-creation — event-handler content attributes processed once, at the element's creation (umbrella memo)

Standalone program, label **`EH-attrs-at-creation`**. Owner: Layout lane. It sits **outside** the IBP-sandbox program and
is not part of that umbrella's DAG (PM decision 17 (i), restated by PM decision 21). It has two nested PRs, **slice a**
and **slice b** (§1). IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 3 `0f99538d`, branch
`layout-ibp-sandbox-handler-gates`) depends on slice a (§1.3 edge x1); §L shows that no landing order can put N1 first.
Rev 1's label `EH-insert-once` and file `2026-10-event-handler-insert-once.md` are retired. The branch keeps its name
`layout-eh-insert-once` (not pushed).

**Approval scope.** Approving this umbrella **decides**: the landing-order verdict (§L); the program's nested-PR set,
their order and edges (§1.2–§1.3, PM decision 21); which slice owns each invariant axis, each coupled pair, each contract
clause and each gap (§1.1, §2, §0.5's landing qualifiers, §7); each fact's home and representation kind (§4.1's
*Home/kind* line); the D-items (§0.4); the spec-semantics decisions (which algorithm, which steps, which outcome: §0.5,
§3's *Step* and *Branch* columns); the classification of slice a as an **approved interim, required by dependency**, and
the two items slice b deletes (§1.4, D-8, D-3; PM decision 23); slice a's processing timings and their no-worse-than-HEAD
cells (§0.5 K-0); the C1 branches (§1.6); the E-pin paths and the red-first rule (§6.2's *path* column and its first
paragraph); the per-slice DoD (§10); and the slots and records (§8). §4's mechanisms below the *Home/kind* line, §5's
populations and named risks, §6's cell shapes and oracles, §3's *Touch* column and §6.5's estimates are **input to each
slice's own plan-review**, which re-derives them. A review finding against that input changes this umbrella only when it
changes a decision; otherwise it is recorded in the owning slice's plan. The per-PR plan for slice a is a separate later
memo; this umbrella is meant to be precise enough that it can be derived from §0.5, §1, §2, §4.1–§4.2, §6.1–§6.2 and
§10.1.

**Approval vehicle**: open (§11 O10).

**Rev 4** (2026-10-25). Folds PM decision 23 and the umbrella plan-review R1 (CRIT 0 / IMP 9 / MIN 18; disposition
§12). (1) §L (new) settles the landing order with cells: N1 on HEAD without slice a returns null from the getter of a
compiled or IDL-set handler moved into a DOMParser document, where HEAD returns a function; so no N1-first order is
no worse than HEAD, and slice a is required. (2) "Slice a is not an interim" is withdrawn everywhere: slice a is an
approved interim, required by dependency, closed by slice b under `#11-eh-attrs-creation-time-call-sites` (§1.4, §8.2).
(3) K-0 (new) states "no step worse than HEAD" as an observable invariant, and enumerates slice a's three processing
timings, each with its cell. To keep it, the bind pass runs once per document, at its first bind (D-5; rev 3's R10 is
now the decision). The Insert arm also keeps HEAD's light-tree walk; rev 3's shadow-including Insert walk is rejected
(R11). Together these remove R1's third timing. (4) O6 is decided: slice b deletes the Insert arm (D-3). (5) The C1
branches are written out (§1.6). *Rev 3* (`399f4534`): PM decision 21 (program, slices a / b, D-8, pins by path). *Rev 2*
(`00b863dd`): PM decision 18 (landing-order comparison, R8 over rev 1's `Pending`; G1 in scope). *Rev 1* (`28efbb67`):
first draft.

## §L. Landing-order verdict (PM decision 23, Phase 1)

**Question.** Is there a landing order that puts IBP-sandbox N1 before slice a and is no worse than HEAD on any
observable case? Two claims in this memo's history disagree. (i) N1's plan-review R1 F1 (PM decision 17) said that
under N1's D-N3 arm 3, the Insert arm's re-raw on every move makes a compiled or IDL-set handler that is moved into a
DOMParser document disappear. (ii) Rev 2 listed "T3 → N1 → C1 → the whole work" as a viable landing (§9.3 landing 1).

**Columns.** *HEAD* = `fd6955e8`. It has two gates. The compile gate is flag-only: `ensure_event_handler_current`
returns before any read only when the realm lacks `allow-scripts` (`event_handler_attrs.rs:550-556`); the main page and
a DOMParser document share the page realm, so the gate never fires for either. The invoke gate is the §8.1.3.4
composition with the node clause, evaluated **before** the reconcile on every dispatch path (`dispatch_target.rs:211`,
`engine.rs:215`, `natives_promise.rs:540`; composition `scripting.rs:83-113`). The getter is `read_event_handler`
(`event_handler_attrs.rs:638`), which reconciles (`:649`) and passes no invoke gate. *N1-alone* = N1 rev 3 applied to
HEAD without slice a. N1 deletes the invoke gate (D-N1), makes the compile gate the composition (D-N2), and reorders the
reconcile so that arm 3 drops a stale callable when the value is raw and the gate fires, keeping the source (D-N3,
N1 memo `0f99538d:261-272`). The consumer's Insert arm is HEAD's in both columns: every insertion, a move included,
calls `set_uncompiled` for every `on*` attribute (H1). `set_uncompiled` leaves the stored callable in place
(`event_listener.rs:207-219`), so after a move the callable is stale and the value is raw. *Spec* = WHATWG HTML §8.1.8.1:
an insertion runs no event-handler steps, so a callback value survives the move. N1's D2 (an approved node-memo
decision) says a callback value runs in a DOMParser document (N1 memo C8b).

`dp` is a DOMParser document. Its consumers are restored after the build (`inert_document.rs:198-199`), so a later
`dp.body.append(x)` fires Insert (H2). `el`, `el2` are live elements.

| # | scenario, step | HEAD | N1-alone | spec | N1-alone vs HEAD |
|---|---|---|---|---|---|
| L1 | setup: live `t`; `t.setAttribute('onclick','A')`; dispatch (compiles `fA`, runs) | `fA` runs | `fA` runs | `fA` runs | equal |
| L1.1 | `dp.body.append(t)`; getter `t.onclick` | Insert re-raws `A`. The compile gate does not fire, so the getter compiles a **new function `fA′`** | Insert re-raws `A`; arm 3 (raw, node clause fires) drops `fA`: **`null`** | `fA` (a function) | **worse**: HEAD's `typeof` is `'function'` like the spec's, and has `A`'s behaviour; N1-alone's is `null` |
| L1.2 | dispatch `click` on `t` | invoke gate suppresses: nothing runs | arm 3 → `null`: nothing runs | `fA` runs | equal |
| L1.3 | `el.append(t)`; getter; dispatch | re-raw; a new function; `A` runs | re-raw; arm 4 compiles a new function; `A` runs | `fA`; `fA` runs | equal |
| L2 | setup: `el.innerHTML='<div id=d onclick="A">'`; `d.onclick = fn` | `d.onclick === fn` | same | same | equal |
| L2.1 | `dp.body.append(d)`; getter | re-raw `A`; the getter compiles `A` over `fn`: **a function (not `fn`)** | re-raw `A`; arm 3 drops `fn`: **`null`** | `fn` | **worse**: N1-alone's value is not closer to `fn` than HEAD's, and it loses `typeof === 'function'`, which HEAD and the spec share |
| L2.2 | dispatch | nothing (invoke gate) | nothing (arm 3) | `fn` runs | equal |
| L2.3 | `el.append(d)`; getter; dispatch | re-raw; a new `A` function; `A` runs | re-raw; arm 4; `A` runs | `fn`; `fn` runs | equal |
| L3 | live → live after an IDL set: L2's setup, `el2.append(d)`; getter; dispatch | re-raw; a new `A` function; `A` runs | the same (live: neither clause fires; arm 4) | `fn`; `fn` runs | equal |
| L4 | identity live → live: `f = t.onclick` (compiled); `el2.append(t)`; `t.onclick === f` | `false` | `false` | `true` | equal |
| L5 | R-a (N1 R2; cell C4's shape): `c = createElement('div')`, never inserted; `c.setAttribute('onclick','A')`; dispatch (detached, page-owned: compiles `fA`, runs); `f = c.onclick`; `dp.body.append(c)`; getter | re-raw; a new function `fA′` | re-raw; arm 3 drops `fA`: **`null`** | `f` | **worse** (as L1.1) |
| L5.1 | then dispatch | nothing | nothing | `f` runs | equal |
| L6 | R-b (N1 R2; cell C5's shape): `dp` `<div id=d onclick="A">` (no `EventListeners` entry, H5); `d.onclick = fn`; getter; dispatch | `fn`; the invoke gate suppresses | `fn`; `fn` runs (C8b (i)) | `fn`; `fn` runs | closer |
| L6.1 | (i) `dp2.body.append(d)`; getter | re-raw `A`; compiles `A` over `fn`: a function | re-raw `A`; arm 3 drops `fn`: **`null`** | `fn` | **worse** (as L2.1) |
| L6.2 | (i) then dispatch | nothing | nothing | `fn` runs | equal |
| L6.3 | (ii) `el.append(d)`; getter; dispatch | re-raw; `A` function; `A` runs | the same | `fn`; `fn` runs | equal |
| L6.4 | (iii) `c = d.cloneNode(); c.onclick = fn; el.append(c)`; getter; dispatch | re-raw over `fn`; `A` runs | the same | `fn` runs | equal |
| L7 | shadow-root-direct child: live `x` with compiled `fA` (as L1's setup); `sr.appendChild(x)`, `sr` the shadow root of a live host; getter; dispatch | no Insert (`mutation.rs:289`); `fA`; `fA` runs | the same (arm 2) | `fA`; `fA` runs | equal |
| L7.1 | as L7, but the host is in `dp` | `fA`; the invoke gate suppresses | arm 2: `fA`; `fA` runs | `fA`; `fA` runs | closer |
| L8 | N1's C8c shape: live `A` compiled; `dp.body.append(t)`; `t.setAttribute('onclick','B')`; getter; dispatch; then `el.append(t)`; dispatch | getter compiles `B` (a function); nothing runs; then `B` runs | getter `null`; nothing runs; then `B` runs | `null`; nothing; then `B` runs | closer, then equal. Only C8c's second half stops discriminating (N1 NR7), a test-quality loss with no observable regression |

**Verdict: NO such order exists.** Rows L1.1, L2.1, L5 and L6.1 decide it. They are one mechanism: a compiled or
IDL-set callback on an element that carries an `on*` content attribute, read after a move into a DOMParser document.
HEAD returns a function there: the right source with a new identity (L1.1, L5), or the attribute's function in place of
`fn` (L2.1, L6.1). N1-alone returns `null` and drops the stored callable, where the spec returns the callback. In every
row dispatch is equal, so the regression is on the getter, and it sits squarely on the population N1 exists to fix
(its D2 / H4: a callback value persists). Every order that lands N1 before slice a passes through the N1-alone state on
main. Rev 2's landing 1 is one of those orders, so its "viable" is withdrawn (§9.3). Claim (i) holds; claim (ii) is
withdrawn. Slice a's K-1 removes the re-raw, so after slice a, L1.1, L2.1, L5 and L6.1 give the spec's callback under N1
(slice a cells C2, C5; N1 cells C8b (iv), C8c).

PM decision 23: with the verdict "no order", slice a goes first as an approved interim, required by dependency (§1.4).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`fd6955e8`** coordinate (the branch cut). origin/main is `ad80b5ca` at rev 3, and
`git diff --stat fd6955e8 ad80b5ca -- crates` is empty, so the coordinates hold on it too. Every code reference comes
from `/usr/bin/grep -n` (never the default ugrep) or `sed -n` over that tree, run on 2026-10-25 by this rev's author.
Every § number ↔ title pair and every algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`) run on
2026-10-25. N1's memo is cited at its rev 3 head `0f99538d`
(`git show 0f99538d:docs/plans/2026-10-ibp-sandbox-handler-gates.md`), and the IBP-sandbox umbrella at the same commit
(`git show 0f99538d:docs/plans/2026-09-ibp-sandbox.md`). Both are read-only here.

### §0.2 Premises re-measured

| id | claim | command | result |
|---|---|---|---|
| H1 | The Insert arm re-delivers baked `on*` attributes on **every** `MutationEvent::Insert` | `sed -n 373,386p` and `sed -n 427,463p crates/script/elidex-script-session/src/event_handler_consumer.rs` | **true.** `handle` (`:373`) routes `Insert` to `handle_insert` (`:427`), which walks `node` plus `traverse_descendants` (`:432`), reads every `Attributes` entry named in `EVENT_HANDLER_ATTRS` (`:61`) and calls `set_inline_handler` (`:468`) → `set_uncompiled`. No state check |
| H2 | `MutationEvent::Insert` fires for every insertion, a move included, except under a shadow root | `sed -n 282,298p crates/core/elidex-ecs/src/dom/tree/mutation.rs` | **true.** `fire_after_insert` (`:282`) returns early when `node` or `parent` is a shadow root (`:289`), otherwise dispatches `Insert`. The filter serves every consumer (doc `:267-280`); a deeper insertion inside a shadow tree (parent a normal element) is not filtered |
| H3 | `set_uncompiled` makes a compiled handler raw again | `sed -n 521,581p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs`; `sed -n 207,219p crates/script/elidex-script-session/src/event_listener.rs` | **true.** `ensure_event_handler_current` (`:521`) takes the source, compiles, and overwrites the callable: a new function identity, and an IDL-set callable is replaced. `set_uncompiled` leaves the stored callable in place until then |
| H4 | The IDL setter supersedes inline source | `sed -n 664,692p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** `activate_event_handler` (`:664`) calls `clear_uncompiled`. A later Insert undoes it (H1) |
| H5 | The DOMParser build suppresses every consumer | `sed -n 110,125p` and `sed -n 192,200p crates/dom/elidex-form/src/inert_document.rs` | **true.** `take_mutation_dispatcher` `:112`, restored `:198-199`. Untouched DOMParser nodes carry no `EventListeners` entry (N1 memo H-P13) |
| H6 | Fragment parses are built dispatch-suppressed and fire Insert at caller placement | `sed -n 318,333p crates/core/elidex-ecs/src/dom/tree/teardown.rs` | **true.** `begin_detached_fragment` (`:318`) takes the dispatcher (`:325`) and builds in a throwaway document (`:326`) |
| H7 | Every script attribute write reaches the AttributeChange arm while bound | §5.0 census | **true** for bound writes; the only non-dispatching writer that can name an `on*` attribute is a creation bake (§5.0 result (a)) |
| H8 | Clone copies an explicit component set only | `sed -n 13,36p crates/core/elidex-ecs/src/dom/tree_clone.rs` | **true.** The clone-policy table (`:19-34`) enumerates; `EventListeners` is in the "deliberate non-copy" row (`:30`) |
| H9 | Production parses the document **before** the VM binds, and bind seeds only the base URL (G1) | `sed -n 839,880p crates/shell/elidex-shell/src/pipeline.rs`; `sed -n 290,320p crates/script/elidex-js/src/vm/vm_api.rs`; `sed -n 113,137p crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | **true.** `build_pipeline_from_loaded` (`:839`) destructures an already-parsed `dom` (`:850-858`) and calls `run_scripts_and_finalize` (`:189`), whose bracket (`:355`) binds. `Vm::bind` builds `ConsumerDispatcher::new` (`vm_api.rs:292`) and calls `initialize_consumers` (`:313`) before installing it (`:314`). `initialize_consumers` (`consumer_dispatcher.rs:135-137`) calls only `base_url.initialize_from_tree`; its doc (`:113-134`) says other consumers need no init. Nothing wires a parsed `<button onclick>` until that node is inserted after bind |
| H10 | Light-tree walks skip shadow roots; a shadow-inclusive walker exists | `sed -n 294,302p` and `sed -n 15,60p crates/core/elidex-ecs/src/dom/tree/walkers.rs` | **true.** `traverse_descendants` (`:294`) skips `ShadowRoot` entities. `for_each_shadow_inclusive_descendant` (`:33`) visits light then shadow children (`:47-57`), skips nodes deeper than `MAX_ANCESTOR_DEPTH` (`:39-41`), and does **not** visit template contents (it follows `get_shadow_root` and `children_iter` only) |
| H11 | `remove_listener_and_prune_back_ref` keeps the `EventListeners` entry | `sed -n 49,61p crates/script/elidex-js/src/vm/inner.rs` | **true** |
| H12 | Bind/unbind keeps the listener store and the Window entity | `sed -n 213,230p crates/script/elidex-js/src/vm/vm_api.rs` | **true.** The first bind creates the Window (`create_window_root`, `:227`); later binds reuse it. `HostData` (owning `listener_store`) persists |
| H13 | **Every** bind rebuilds the dispatcher and re-runs `initialize_consumers`; every unbind removes it | `sed -n 28,36p crates/script/elidex-js/src/vm/lifecycle.rs`; `/usr/bin/grep -rn --include='*.rs' 'with_bound(' crates/shell \| wc -l` | **true.** Unbind calls `clear_mutation_dispatcher` (`lifecycle.rs:34`); bind installs a fresh one (`vm_api.rs:292-314`). The shell has 15 `with_bound(` call sites (initial scripts `pipeline.rs:355`, per-event brackets `lib.rs:299-529`, `pipeline.rs:133`, `:165`). So a bind-time pass would run once per bracket unless it is gated per document (D-5) |
| H14 | At `initialize_consumers` the Window entity exists | `sed -n 213,230p` and `:313` of `vm_api.rs` | **true.** `we` is resolved or created (`:222-229`) before the dispatcher is built and initialised. `resolve_handler_target` (`event_handler_consumer.rs:328-334`) finds it through `dom.window_entity()` |
| H15 | Template contents are owned by the template's own document, and the clause-(b) composition does not suppress them | `sed -n 539,580p` and `sed -n 820,856p crates/core/elidex-ecs/src/dom/mod.rs`; `sed -n 95,113p crates/script/elidex-script-session/src/scripting.rs` | **true.** `attach_template_contents` (`mod.rs:560`) creates the fragment with `owner_doc` = the template's document (C2 approximation, `:551-557`; slot `#11-template-contents-owner-document`). For a node under that fragment, `scripting_disabled_for_platform_object` takes the composed root (`scripting.rs:104`), which is the fragment, not a Document, so it falls back to `owner_document` (`:108`), which follows the fragment's `AssociatedDocument` (`mod.rs:851-853`) to the page document: not suppressed. The doc comment names this false negative (`scripting.rs:59-60`) |
| H16 | Every shell pipeline builder parses first and binds in `run_scripts_and_finalize` | `/usr/bin/grep -n 'run_scripts_and_finalize\|parse_progressive_str' crates/shell/elidex-shell/src/pipeline.rs`; `/usr/bin/grep -rn 'build_pipeline_from_loaded(\|build_pipeline_interactive_shared(' crates/shell/elidex-shell/src` | **true.** `build_pipeline_interactive` (`:565`; parse `:567`, bind `:588`), `build_pipeline_interactive_with_network` (`:646`; `:656`, `:675`), `build_pipeline_interactive_shared` (`:735`; `:747`, `:771`), `build_pipeline_from_loaded` (`:839`; `:872`). Production main-document callers of `build_pipeline_from_loaded`: `content/mod.rs:673`, `content/navigation.rs:276`, `app/navigation.rs:323`. So one bind-time pass covers every builder |
| H17 | The three iframe paths | `sed -n 52,81p` and `/usr/bin/grep -n 'build_pipeline\|fn build_iframe_pipeline' crates/shell/elidex-shell/src/content/iframe/load.rs` | **true.** URL-loaded same-origin: `build_pipeline_from_loaded` (`load.rs:221`). URL-loaded out-of-process: `build_pipeline_from_loaded` (`:327`). srcdoc / about:blank / no-src (`:62-79`, taken when `iframe_data.srcdoc` is present or `src` is absent, `:54`): `build_iframe_pipeline` (`:465`) → `build_pipeline_interactive_shared` (`:471`; `pipeline.rs:735`), **not** `build_pipeline_from_loaded`. All three reach `run_scripts_and_finalize` (H16) |
| H18 | cloneNode's path | `sed -n 201,215p crates/script/elidex-js/src/vm/host/node_methods_extras.rs`; `/usr/bin/grep -n 'clone_node_with_shadow_honor\|clone_node_shallow\|attach_template' crates/dom/elidex-dom-api/src/node_methods/clone.rs`; `/usr/bin/grep -n 'fn clone_node_shallow\|builder.add(attrs)' crates/core/elidex-ecs/src/dom/tree_clone.rs` | VM `native_node_clone_node` (`node_methods_extras.rs:201`) → `invoke_dom_api("cloneNode")` → `elidex-dom-api` `node_methods/clone.rs:154` → `clone_node_with_shadow_honor` (`:227`) → `clone_recording` → `EcsDom::clone_node_shallow` (`clone.rs:323`; `tree_clone.rs:251`) → `clone_node_shallow_unchecked` (`tree_clone.rs:270`), whose silent `Attributes` copy is `builder.add(attrs)` at `tree_clone.rs:368`. The template post-pass attaches a fresh fragment (`clone.rs:422`) |
| H19 | The VM holds the policy the dispatcher is built with | `/usr/bin/grep -n 'spec_level_policy' crates/script/elidex-js/src/vm/mod.rs crates/script/elidex-js/src/vm/vm_api.rs crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | **true.** `VmInner::spec_level_policy` (`vm/mod.rs:2715`) is passed to `ConsumerDispatcher::new` (`vm_api.rs:296`), which builds `EventHandlerAttributeConsumer::with_policy` from it (`consumer_dispatcher.rs:97`, `:107`) |
| H20 | `AttributeChange` fires after the `Attributes` write | `sed -n 125,130p crates/core/elidex-ecs/src/dom/attribute.rs` | **true.** `set_attribute` (`:125`) calls `write_attribute_no_dispatch` (`:126`) and fires afterwards |
| H21 | HEAD's two gates: the compile gate is flag-only; the invoke gate precedes the reconcile on every dispatch path; the getter passes only the compile gate | `sed -n 548,557p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs`; `sed -n 196,224p crates/script/elidex-js/src/vm/host/dispatch_target.rs`; `/usr/bin/grep -rn --include='*.rs' 'scripting_disabled_for_platform_object' crates/script/elidex-js/src`; `sed -n 638,655p` of `event_handler_attrs.rs` | **true.** Compile gate `:550-556` (`!hd.scripts_allowed()` → return). Invoke gate before `reconcile_handler`: `dispatch_target.rs:211`, `engine.rs:215`, `natives_promise.rs:540`. `read_event_handler` (`:638`) reconciles (`:649`) with no invoke gate (§L) |
| H22 | Placement into template contents by `innerHTML` fires Insert | `sed -n 88,104p crates/script/elidex-script-session/src/mutation/html_fragment.rs` | **true.** `placement_target` is the contents fragment (`:93`); `append_child(placement_target, node)` (`:102`) fires Insert (H2: a DocumentFragment parent is not filtered). HEAD's Insert arm raws those values; the clause-(b) composition does not suppress them (H15) |
| H23 | Crate graph for slice b's call sites | `/usr/bin/grep -n '^elidex' crates/dom/elidex-html-parser/Cargo.toml crates/dom/elidex-html-parser-strict/Cargo.toml crates/script/elidex-script-session/Cargo.toml crates/dom/elidex-form/Cargo.toml crates/dom/elidex-dom-api/Cargo.toml`; `/usr/bin/grep -rn 'attach_template_contents(' --include='*.rs' crates` | `elidex-html-parser-strict` → `elidex-ecs` only; `elidex-html-parser` → `elidex-ecs`, `elidex-custom-elements`, `elidex-html-parser-strict`; `elidex-script-session` → `elidex-html-parser` (`Cargo.toml:30`); `elidex-form` and `elidex-dom-api` → `elidex-script-session`. `begin/finish_detached_fragment` are called from `elidex-html-parser/src/lib.rs:142`, `:163` and strict `tree_builder/mod.rs:178`, `:262`. `attach_template_contents` (`elidex-ecs` `dom/mod.rs:560`) is called from `convert.rs:288`, strict `tree_builder/shadow.rs:137`, `elidex-dom-api` `document.rs:264`, `node_methods/clone.rs:422` |
| H24 | `native_body_weh_set` returns before any write when no Window is bound | `sed -n 458,479p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** The `window_entity()` early return is `:468-470`; `activate_event_handler` follows (`:472`) |
| H25 | Handler compile uses the global scope only | `sed -n 718,725p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** `lazy_compile_handler` wraps the source as `(function (event) {…})` and runs it as a script: no element, form or document scope (§5.5 K6) |

### §0.3 Premise errors (reported)

- **P1: `importNode` / `adoptNode` do not exist** in elidex (`/usr/bin/grep -rn -i --include='*.rs' -E
  'importnode|adoptnode' crates` → nothing). Insertion adoption itself is slot `#11-cross-document-adopt-on-insert`. This
  memo analyses only elidex's internal adopt, `EcsDom::adopt_subtree` (`teardown.rs:285`), used by fragment teardown.
- **P2: the Window-reflecting body set is not delegated** (G4). `onblur` `:68`, `onerror` `:90`, `onfocus` `:91`,
  `onload` `:99`, `onresize` `:125`, `onscroll` `:126` are `HandlerScope::Global`, so `resolve_handler_target` (`:328`)
  keeps them on the body. Merged by the PM into slot `#11-event-handler-target-determination` (facet 4). Not here.
- **P3: a docstring overstates a scripting rule** (`inert_document.rs:78-84`): the attribute change steps have no
  scripting condition; *get the current value of the event handler* step 3.2 nulls a raw value. The correction belongs to
  the slice that touches that file (slice b); slice a does not touch it (MINOR until then).
- **P4: `EcsDom::clone_attributes` is not the cloneNode writer** (`tree_clone.rs:57`; one caller, the test
  `crates/core/elidex-ecs/src/dom/tests/clone.rs:14`). The cloneNode path is H18.
- **P5 (rev 2 brief): the iframe coordinates name URL-loaded iframes only.** `load.rs:221` / `:327` are the
  same-origin-URL and out-of-process paths; srcdoc iframes are built by `build_pipeline_interactive_shared` (H17).
- **P6 (rev 2 brief): `vm_api.rs:312-314`.** `ConsumerDispatcher::new` is at `:292`; `initialize_consumers` at `:313`,
  the install at `:314`.
- **P7 (rev 2 brief): `tree_clone.rs:368` is a line inside `clone_node_shallow_unchecked`** (defined at `:270`), not its
  definition (H18).
- **P8 (rev 2 brief): there is a third R8 blocker** (template contents, §9.2 (c)).
- **P9 (rev 2 brief): H-P13 need not be rewritten** (§9.2 (a)).
- **P10 (rev 3 brief): the "rev 2 premise errors" on `vm_api.rs:292` and `tree_clone.rs:270` / `:368` were already
  corrected in rev 2** (P6, P7, H9, H17 of rev 2). Rev 3 re-measured them (H9, H18) and they hold; nothing to fix.
- **P11 (rev 3 brief): slice b's prerequisites include N1**, not only T3 and C1. §9.2 row (a): a raw value that slice b
  writes at a DOMParser node's creation reads null only under N1's compile-gate node clause. Decision 21's order (slice
  a → N1 → …, and C1's handling decided after N1 lands) satisfies it transitively; §1.3 records it as its own edge (x4)
  so that a later re-ordering cannot drop it silently.
- **P12 (rev 3 brief): "the only thing slice b deletes is one guard condition" holds only if O6 keeps the Insert arm.**
  O6's "delete" alternative (rev 2 §10 O6) would also delete slice a's Insert-arm processing call. **Resolved in rev 4**:
  O6 is decided against keeping the arm, and slice b deletes two items, the guard line and the Insert arm (§1.4).
- **P13 (rev 3 brief): the brief's "G2 part in slice a" is "shadow-inclusive walks".** The remaining G2 case (an
  element inserted directly under a shadow root fires no Insert, H2) is slice b's (§7). **Narrowed in rev 4**: only the bind pass
  walks shadow trees (D-10); the shadow content of a host inserted after bind is slice b's too (B6 (iii)).
- **P14 (rev 4 brief): "mark at placement under a shadow root" has no hook.** A placement whose parent is a shadow root
  fires no `MutationEvent` to any consumer (`mutation.rs:289`, H2). Changing that filter changes every consumer's
  contract (Range live-tracking, custom elements), which G2's disposition keeps out of this program. The brief's
  purpose (no third processing timing; A2 IMP-1) is met instead by D-5 (the bind pass runs at a document's first bind
  only) and D-10 (the Insert arm keeps HEAD's light-tree walk). Nothing is marked at such a placement (§0.5 K-0, cell U9).
- **P15 (rev 4 brief, from R1 A1): "parser crates depend only on elidex-ecs"** is exact for `elidex-html-parser-strict`
  only. `elidex-html-parser` also depends on `elidex-custom-elements` and the strict crate (H23). The conclusion still
  holds: neither parser crate can call `elidex-script-session`, which depends on `elidex-html-parser` (a cycle).
- **P16 (rev 4 brief, from R1 A5): the R1 record files the stale G7 text and §8.1's missing slots as "FP-known".** They
  are real staleness, and rev 4 fixes them (§12 rows A5-x1, A5-x2).

### §0.4 Decisions

- **D-1: once-key.** `HandlerAttrsProcessed` (NEW), a unit marker component on the **origin** element (never on the
  handler's target, the Window for `<body>` delegation). Meaning: *this element carries no handler content attribute
  whose creation-time value is still unprocessed.* Sparse: inserted only on an element that had a handler content
  attribute when processed. Absent = unprocessed (fails toward HEAD's delivery, never toward loss).
- **D-2: processing.** One engine-independent function, `process_handler_content_attrs` (NEW), runs the attribute
  change steps (WHATWG HTML §8.1.8.1) for every handler content attribute E carries, at most once per element (D-1),
  subject to the session's `SpecLevelPolicy`, with the target from `resolve_handler_target`.
  `process_handler_content_attrs_in_subtree` (NEW) applies it over a shadow-including subtree (H10); in slice a its only
  caller is the bind pass (D-10).
- **D-3: timing (rewritten in rev 4; O6 decided).** The end state (slice b) has three processing points, each the
  earliest point at which a consumer sees E: E's **creation**, for every creation path that runs while a consumer
  exists; the **first bind of E's document**, for elements created before any consumer existed (the pre-bind parse,
  G1); and E's **first handler write** (D-4). Insertion processes nothing in the end state: slice b deletes the Insert
  arm. Slice a has the last two of those points, plus one interim point: **E's first Insert over HEAD's light-tree walk**
  (D-10) stands in for creation on post-bind creation paths. Slice b deletes that interim point. A creation path that
  slice b fails to process is caught by cells that read E before any insertion, and by a `debug_assert!` that slice b's
  plan places (§6.3 B7). No release fallback is kept (rev 3's O6 "keep" alternative is withdrawn). Insertion never
  re-processes (D-1).
- **D-4: writers.** A handler write (the AttributeChange arm, and both VM element setters) first ensures E is
  processed (`note_handler_write`), then writes. Its shape is fixed by D-8.
- **D-5: bind (rewritten in rev 4; R10 adopted, PM decision 23).** `EventHandlerAttributeConsumer::initialize_from_tree`
  (NEW) processes the bound document's shadow-including tree **once per document**, at its first bind, and marks the
  Document entity with `PreBindHandlerAttrsProcessed` (NEW, a unit marker component). Later binds of the same document
  find the marker and do nothing. `ConsumerDispatcher::initialize_consumers` calls it. Elements that exist at a
  document's first bind are exactly the elements created while no consumer existed: no script runs between the parse
  and that bind (H9). Elements created after it are created while a consumer is installed, except elements created
  while unbound between brackets. That gap is G7, registered as `#11-unbound-attribute-writes-bypass-consumers`
  (latent, not reachable today). So the bind pass never reaches an element a consumer could have seen, and the third
  timing R1 found (A2 IMP-1) cannot occur. The pass stays after slice b (the pre-bind parse has no consumer at creation,
  H9). In branch A of §1.6, slice b also walks template contents (§4.3).
- **D-6: VM surface.** The two element setters (`native_event_handler_set` `:344`, `native_body_weh_set` `:459`) call
  `note_handler_write` before `activate_event_handler`, passing `VmInner::spec_level_policy` (H19). In
  `native_body_weh_set` the call is placed **after** the no-Window early return (`:468-470`, H24), so a setter that
  writes nothing also processes nothing. Marshalling only (Layering mandate).
- **D-7: program shape (PM decision 21).** Two nested PRs, slice a and slice b, with the scope, edges and order of §1.
- **D-8: the guard is one line, and it is the only line slice b deletes from `note_handler_write`.** Slice a's
  `note_handler_write` is
  ```text
  if origin is not an element, or is marked: return
  if origin carries a deliverable handler content attribute other than on<event_type>: return   // the guard
  process_handler_content_attrs(policy, origin, dom)
  ```
  Slice b deletes the guard line and nothing else in this function. When the guard passes, processing touches at most
  `on<event_type>` itself, and the write that follows overwrites it (H20: an AttributeChange write already holds the new
  value; an IDL write replaces the raw value through `activate_event_handler`). So the state right after the write
  equals HEAD's (K-0 timing T-a3). The guard is recorded under slot `#11-eh-attrs-creation-time-call-sites` (§8.2).
  Slice b also deletes the Insert arm (D-3); that is a separate function.
- **D-9: G1 pins by path, red first.** Slice a carries end-to-end pins on three paths: the main document, a URL-loaded
  iframe, and a srcdoc iframe (§6.2). Each pin is run on the pre-fix base and its failure is recorded before the fix
  lands; each has a non-vacuity control that is green on that base.
- **D-10: walk sets (rev 4; PM decision 23).** In slice a the Insert arm keeps HEAD's walk set: `node` plus
  `traverse_descendants(node)` (light tree), with the once check per element. Only the bind pass walks shadow trees
  (`for_each_shadow_inclusive_descendant`). The shadow-including Insert walk of rev 3 is rejected (R11). It would process
  shadow content at a host's insertion, an event at which HEAD processes none of it. An element placed directly under a
  shadow root that took a guarded write (two or more deliverable attributes) would then lose that write. HEAD keeps it.

### §0.5 Contract (single citable statement, with landing qualifiers)

For an element E, "creation-time attributes" are the `Attributes` written without `MutationEvent::AttributeChange` at
E's creation (the four parser bakes and the clone copy, §5.0 (a)). "Handler state" is the `EventListeners` entry for an
event type on E's resolved target, plus its stored callable. "Processed" = the attribute change steps have run for E's
creation-time handler content attributes (D-1). An "observable step" is an IDL getter read or a dispatch.

- **K-0, no step worse than HEAD.** *Slice a.* For every element and every observable step, slice a's outcome equals
  HEAD's outcome at the same step, or equals the spec's where HEAD's does not. There is one exception class, **E-G1**:
  a pre-bind (initial-HTML) element gets exactly the outcome that HEAD gives the same markup inserted after bind. Where
  that outcome itself diverges from the spec, the divergence is HEAD's existing post-bind defect, which G1's fix now
  reaches. E-G1 has two members, each registered with the trigger "slice a's landing": (i) the Window-reflecting body
  set goes onto the `<body>`. For example, an initial `<body onfocus="…">` runs on a focus event dispatched at the body,
  where the spec puts it on the Window (§8.1 facet 4). (ii) The missing handler scope chain: an initial
  `<select onchange="form.submit()">` throws a `ReferenceError` on `form` (H25), where the spec resolves the form owner.
  The onerror five-argument form belongs to the same slot (§8.1). Slice a has exactly three processing timings, and
  each comes with a cell that shows it is no worse than HEAD:
  - **T-a1, first bind of a document** (D-5). It processes the document's shadow-including tree; template contents are
    excluded. HEAD processes none of these elements, and no script has run since their creation (H9), so no handler
    state exists to overwrite: every outcome is the spec's or E-G1. Cells E1 (HEAD: no `data-hit`; slice a: hit; spec:
    hit), E7, U7. Later binds process nothing: cell U9 shows that a shadow-root-direct element that took a guarded
    IDL write keeps it across a host move and a second bind (HEAD-equal, and the spec's outcome for that write).
  - **T-a2, Insert** (D-10). It processes the unmarked elements of HEAD's own walk set at the Insert at which HEAD
    processes them, with HEAD's policy filter and target resolution. On an unmarked element the outcome equals HEAD's
    (cells C6, C13). A marked element is skipped, where HEAD re-raws; that skip is the spec's outcome (cells C1, C3, C8,
    U1, U2).
  - **T-a3, first handler write** (D-4, D-8). If the guard passes, E's only deliverable attribute is the one being
    written. Processing raws it and the write overwrites it, so the state right after the write equals HEAD's (cell
    U5 (i)). Its later effect is T-a2's skip (cells C4, C5, C11). If the guard returns, nothing is processed and the
    outcome is HEAD's (cells U5 (ii), C16).

  No other timing exists in slice a: no processing at placement under a shadow root (P14), none at a later bind (D-5),
  and none at a read.
- **K-1, once.** E's creation-time handler content attributes are processed at most once. After that, no insertion,
  move, or rebind writes handler state derived from them. A compiled callable keeps its identity across moves and
  rebinds, in the live document and into a DOMParser document alike. *From slice a, for every element.*
- **K-2, an AttributeChange write wins.** Processing never overwrites a handler written by `setAttribute` /
  `removeAttribute` of `on<T>` on E. Its value survives, including a callable compiled from it. *From slice a for every
  element with at most one deliverable handler attribute; from slice b for every element.*
- **K-3, an IDL write wins.** Processing never overwrites a callable or null assigned through `E.on<T>` (or a `<body>`'s
  delegated WindowEventHandlers accessor), whatever wrote E's `Attributes` silently. *Same qualifier as K-2.*
- **K-4, timing.** For every creation path that runs while a consumer exists, processing happens at creation, before
  any script can observe E. For elements created with no consumer (the pre-bind parse), it happens at the first bind of
  their document, before that bind's first script. Processing writes the internal raw uncompiled handler on the resolved
  target. *The bind arm from slice a; the creation arm from slice b. In branch B of §1.6, template contents are
  processed when they leave the template contents, not at creation (slot `#11-eh-template-contents-at-creation`).*
- **K-5, inert documents read null.** A raw value that processing writes on a node whose node document has a null
  browsing context (a DOMParser document, template contents) reads as null and never runs. *Slice a: the bind pass writes
  no such value (it reaches neither, U8 (i), (ii)). Slice a's Insert arm writes HEAD's raw values when an unmarked
  element is placed into one: into template contents through `tpl.innerHTML` (`html_fragment.rs:93`, `:102`, H22) or a
  script append into `tpl.content`, and into a DOMParser document after its build (`inert_document.rs:198-199`). Those
  values read non-null, and in template contents they run. That is the accepted window W-K5 (§1.5; cells C14, C15),
  HEAD-equal. From N1, the DOMParser half reads null (N1's node clause). From slice b in branch A of §1.6, for every
  node. In branch B, every node except template contents and their clones (slot `#11-eh-template-contents-at-creation`).*
- **K-6, bind completeness (G1).** At the first bind of a document, every element of its shadow-including tree
  (template contents excluded) is processed before any script of that bind runs. Later binds of the same document
  process nothing. *From slice a.*

Red cells per clause (§6): K-0 guards U5 (ii), U9, C6, C13, C16 (no red: K-0 is HEAD-equality or improvement); K-1 C1–C3,
C8, U1, U2, E3; K-2 C4, C11, U5 (i), B1, B8; K-3 C5, B1, B8; K-4 E1, E2, U7, B6; K-5 B4, B5 (window pins C14, C15);
K-6 E1, E2, E4–E7, U7.

## §1. Program shape (decided)

### §1.1 The nine invariant axes and their slices

| axis | source | slice a | slice b |
|---|---|---|---|
| I1 once (per element, across moves and rebinds) | K-1, H13 | owns it: marker, once check in the Insert arm and the bind pass; the document marker (D-5) | extends it to the creation-time call sites; deletes the Insert arm (D-3) |
| I2 writer order (a write is never overwritten) | K-2, K-3 | restricted (D-8 guard) | full (guard deleted) |
| I3 origin key (state on the origin, not the target) | D-1 | owns it | — |
| I4 freshness (new elements start unprocessed; clone does not copy) | H8 | owns it (clone-policy row) | clone processed at its creation |
| I5 coverage (every creation path, the pre-bind tree, shadow trees, template contents) | K-4, K-6, G1, G2 | the pre-bind tree, shadow-including (bind pass); first insertion over HEAD's light walk (D-10) | creation-time call sites (shadow content included), template contents (§1.6) |
| I6 policy | `event_handler_consumer.rs:405`, `:447` | owns it (same `policy.installs` filter in every path) | — |
| I7 inert-document reads (DOMParser, template) | K-5, H5, H15, H22 | the bind pass does not reach them (U8 (i), (ii)); the Insert arm keeps HEAD's W-K5 (C14, C15); C2's move into a DOMParser document keeps a callable | owns it (needs N1, C1 or §1.6 branch B) |
| I8 target determination (step 3) | §9.2 (b) | not reached (the bind pass walks only the active document) | owns it (needs T3) |
| I9 layering / re-entry | Layering mandate; `mutation_event.rs` | owns it for the VM setters and the bind pass | owns it for the new call sites (O9) |

**Count: 9** (≥ 3): edge-dense, so the work is a multi-PR program and each slice gets its own `/elidex-plan-review`
before implementation (CLAUDE.md *Edge-dense work* (a), (b)). Slice a holds seven axes (I1–I6, I9) and slice b seven (I1,
I2, I4, I5, I7, I8, I9; it owns I7 and I8 and extends the other five). Neither is re-split, under the base case (c):
each is a narrowly-scoped per-PR slice of this approved umbrella, and its own plan-review decides whether it recurses.

### §1.2 Nested PRs (decided)

| nested PR | scope | prerequisites |
|---|---|---|
| **slice a** `EH-attrs-at-creation-a` (branch `layout-eh-insert-once`) | D-1 marker `HandlerAttrsProcessed` on the origin element; D-2 functions; the Insert arm processes once over HEAD's light walk (K-1, D-10); the bind-time pass, once per document, shadow-including (D-5, G1, K-6); D-8 restricted writer marking in the AttributeChange arm and both VM setters (D-6); clone-policy row; K-0's cells; G1 pins on three paths (§6.2); the G6 docstring | none |
| **slice b** `EH-attrs-at-creation-b` | creation-time call sites for every bound creation path (fragment build, DOMParser build, clone, template contents per §1.6; shape O9), shadow content included (G2 rest); D-8 guard deleted (unrestricted writers); the Insert arm deleted (D-3; §1.6 branch B keeps it for template contents only); G5; K2; the `inert_document.rs` docstring (P3); the creation-path detector (B7) | T3, C1's handling decided (PM decision 21); N1 (P11) |

**Order (PM decision 21; §L):** slice a → IBP-sandbox N1 → … ; slice b after T3 and N1. **Slice b's start: after C1's
handling is decided** (§1.6). The PM decides C1's handling once slice a and N1 have landed.

### §1.3 Edges (decided)

| # | edge | kind | reason |
|---|---|---|---|
| x1 | slice a → IBP-sandbox N1 (external) | semantic precondition | §L: without slice a, N1 returns null from the getter of a compiled or IDL-set handler moved into a DOMParser document, where HEAD returns a function (L1.1, L2.1, L5, L6.1). N1's C8b (iv) needs that handler to survive the move, and C8c's second half discriminates only when a re-insertion does not turn the value raw (N1 memo NR7, `0f99538d:575`). Slice a's K-1 gives both (cells C2, C3) |
| x2 | slice a → slice b | code predecessor + data flow | slice b extends slice a's marker, `process_handler_content_attrs` and `note_handler_write` with creation-time call sites, deletes the D-8 guard and deletes the Insert arm |
| x3 | T3 (`#11-event-handler-target-determination` step 3; owner Layout) → slice b | semantic precondition | slice b processes a DOMParser (or fragment-throwaway) `<body>` at creation; without step 3 its WindowEventHandlers attributes land on the **page** Window (§9.2 (b), I8) |
| x4 | N1 → slice b | semantic precondition (P11) | slice b writes raw values at DOMParser-node creation; they read null only under N1's compile-gate node clause (§9.2 (a), I7) |
| x5 | C1's **handling decided** (`#11-template-contents-owner-document`; no driver; deliberately deferred) → slice b | decision precondition; in branch A also C1's landing | branch A: slice b processes template contents at creation, which reads null only once C1 carries the inert owner per node (§9.2 (c), H15). Branch B: slice b leaves template contents to the kept Insert arm (§1.6) |

Acyclic. x4 is satisfied transitively by the order (x1, and x5's decision point after N1 lands); it is listed so that a
re-ordering cannot drop it.

### §1.4 Slice a is an approved interim, required by dependency (PM decision 23)

CLAUDE.md "One issue, one way" forbids leaving a strangler state, in which a new mechanism coexists with the one it
replaces. Slice a is such a state for **timing**. Post-bind creations are processed at first insertion; slice b's end
state processes them at creation. The D-8 guard restricts the writers for the same reason. Rev 3 said slice a is not
an interim, and that is withdrawn (R1 A3 IMP-1, IMP-2). Slice a is an interim that the PM approves because a dependency
requires it:

- **Why it cannot be skipped.** N1 requires K-1 (§L: no N1-first order is no worse than HEAD). Slice b requires T3, N1
  and C1's handling (§1.3 x3–x5), so the end state cannot land before N1. The only order that satisfies both is
  slice a → N1 → slice b.
- **Kept by slice b unchanged:** `HandlerAttrsProcessed` and its meaning (D-1); `process_handler_content_attrs` and the
  subtree form (D-2); the bind pass and `PreBindHandlerAttrsProcessed` (D-5; still the only point for the pre-bind
  parse); the clone-policy row; the `note_handler_write` call sites in the AttributeChange arm and both VM setters (D-6).
- **Extended by slice b:** new call sites of `process_handler_content_attrs` at creation (O9).
- **Deleted by slice b (the interim's whole surface):** (1) the D-8 guard line in `note_handler_write`; (2) the Insert
  arm: `handle_insert` and the `MutationEvent::Insert` match arm of the consumer's `handle` (`event_handler_consumer.rs:373`, `:381`) (D-3). In §1.6
  branch B, (2) is deferred to the template-contents residue slot.
- **Interim behaviour, all HEAD-equal (K-0):** §1.5's windows.
- **Closure record:** slot `#11-eh-attrs-creation-time-call-sites` (§8.2) names the interim, what deletes it and when.

### §1.5 What slice a leaves open until slice b (accepted windows)

Every window below exists at HEAD with the same outcome. Slice a introduces none except E-G1 (K-0).

- **K-2 / K-3 residual:** a handler write on a post-bind element with **two or more** deliverable handler attributes,
  made before the element's first insertion, is overwritten at that insertion (the D-8 guard; B1, B8). HEAD overwrites
  it at every insertion.
- **G5 / K3:** a never-inserted clone reads null (B2).
- **K2:** the body-clone cross-origin write (B3).
- **G2 rest:** an element placed directly under a shadow root after bind, or inside a subtree so placed, is not
  processed. Neither is the shadow content of a host inserted after bind (B6).
- **G3:** a DOMParser `<body>` with a WindowEventHandlers attribute is delivered to the page Window at its first
  insertion (HEAD; B5).
- **W-K5, inert containers:** an unmarked element placed into template contents (by `tpl.innerHTML` or a script append)
  or into a DOMParser document gets HEAD's raw values. They read non-null, and in template contents they run (K-5;
  cells C14, C15; B4). Parser-built template contents are processed when moved out (HEAD).
- **Depth cap:** the bind pass skips elements deeper than `MAX_ANCESTOR_DEPTH` (`walkers.rs:39-41`). HEAD processes no
  pre-bind element, so this is HEAD-equal. Slice b's creation-time sites carry no cap.
- **E-G1:** facet 4 and the scope chain, now reached by initial HTML (K-0; §5.5 K5, K6; §8.1).

### §1.6 C1's handling: the two branches (PM decision 21's "slice b after C1's handling is decided")

| | **branch A: the PM lands C1** | **branch B: the PM decides not to land C1** |
|---|---|---|
| slice b's start | after C1 lands | after the decision |
| template contents at creation | processed (call site at `attach_template_contents`; the bind pass also walks pre-bind template contents, §4.3); they read null under C1's per-node inert owner | **not** processed at creation. The Insert arm is kept, and it is their only processing point when they leave the template contents (HEAD's) |
| the Insert arm | deleted (§1.4 (2)) | kept unchanged. After slice b, every other bound creation path marks at creation, so the arm processes only template contents leaving their fragment, plus any omitted path. B7's pre-insertion cells catch omitted paths, because the arm cannot mask a read made before insertion |
| K-4 | unqualified | qualified: template contents are processed when they leave the template contents |
| K-5 | unqualified | qualified: template contents and their clones read non-null (W-K5 persists, HEAD-equal) |
| slot | `#11-eh-attrs-creation-time-call-sites` closes at slice b's landing | the same slot closes at slice b's landing for every defect except template contents, which move to the new slot `#11-eh-template-contents-at-creation` (§8.2; its trigger is C1's landing; it owns the Insert arm's deletion) |
| §10.3 | K-0–K-6 unqualified | K-4 and K-5 carry the slot's qualifier; the rest unqualified |

## §2. Coupled invariants

### §2.1 All pairs (36), with owner

*Owner* = the slice whose plan resolves the pair; *a → b* = slice a resolves it for its scope and slice b re-derives it
for the creation-time call sites.

| pair | intersect? | owner | resolution |
|---|---|---|---|
| I1×I2 | yes | a → b | A processed element is marked; writers act on its entries directly (HEAD). `note_handler_write` on a marked element is a no-op. Cells C1, C4, U5 (iii) |
| I1×I3 | yes | a | The marker is per origin. A second `<body>` (a clone) is unmarked and processes once to the Window. Cells C8, U2 |
| I1×I4 | yes | a → b | Clone does not copy the marker (H8; clone-policy row). Slice b processes the clone at its creation. Cells C9, U6, B2 |
| I1×I5 | yes | a → b | The bind pass and the Insert arm call the same per-element function. The document marker makes the bind pass run once per document, across H13's repeated binds (D-5). The Insert arm walks HEAD's set (D-10). Slice b's creation sites call the same function and the Insert arm goes. Cells E3, U7, U9 |
| I1×I6 | yes | a | The marker is set when the element has any handler content attribute, excluded or not, so an excluded-only element is not rescanned. Cell U3 |
| I1×I7 | yes | a, b | Slice a: a compiled callable keeps its identity when moved into a DOMParser document (C2, N1's C8b (iv)). Slice b: a DOMParser node processed at creation is marked, so a later move into the page does not re-process it; its raw value compiles there (the spec's outcome) |
| I1×I8 | yes | b | A DOMParser `<body>` processed at creation under T3 writes nothing and is marked; a move into the page does not deliver it (insertion runs no change steps). Cell B5 |
| I1×I9 | yes | a | The once check is inside `process_handler_content_attrs`; the VM has no once logic |
| I2×I3 | yes | a | `note_handler_write` takes the `<body>` receiver for a delegated write (D-6). Cell C8 |
| I2×I4 | yes | a | A clone starts unmarked; a write on the source never marks the clone. Cell C9 |
| I2×I5 | yes | a → b | Slice a: a write on an unmarked element whose only deliverable attribute is the written one processes and marks it, so first insertion skips it (C4, C5, C11). With two or more, the guard returns and the element's next HEAD-walk Insert processes all of them, as HEAD does (§1.5). No later bind and no shadow-including Insert walk reaches it (D-5, D-10; U9). Slice b deletes the guard (B1, B8) |
| I2×I6 | yes | a | "Deliverable" = in `EVENT_HANDLER_ATTRS` and installed by the policy. An excluded name never reaches `note_handler_write` from the AttributeChange arm (returns at `:405-407`); its IDL setter is not installed |
| I2×I7 | yes | b | Unrestricted ensure-before-write on a DOMParser node materialises its siblings as raw values, which read null only under N1 (x4). This is why the guard exists in slice a (R7) |
| I2×I8 | yes | b | The same ensure on a DOMParser `<body>` would deliver its WindowEventHandlers siblings to the page Window without T3 (x3) |
| I2×I9 | yes | a | Both VM setters call the engine-independent `note_handler_write` (D-6) |
| I3×I4 | yes | a | A clone of a `<body>` is a new origin: unmarked, processed once to the Window at its first insertion. Cells U2, C9 |
| I3×I5 | yes | a | The bind pass resolves `<body>` WindowEventHandlers rows to the Window, which exists at bind (H14). Cell E4 |
| I3×I6 | yes | a | An excluded WindowEventHandlers attribute (`onstorage` under `BrowserCore`) marks the `<body>` and writes nothing to the Window. Cell U3 |
| I3×I7 | yes | b | The marker for a DOMParser `<body>` is on that body; its target is decided by T3, its reads by N1 |
| I3×I8 | yes | b | Slice a keeps HEAD's target (no step 3); the bind pass reaches only the active document, where step 3 does not fire. Slice b processes inactive-document bodies at creation, which needs step 3 |
| I3×I9 | yes | a | The origin is chosen in engine-independent code; `native_body_weh_set` passes the `<body>` receiver, not the Window (§4.2 row 4) |
| I4×I5 | yes | a → b | Fragment-parsed nodes and clones are processed at first insertion in slice a (H6 placement), at creation in slice b |
| I4×I6 | no | — | Clone does not copy the marker whatever the policy; the policy filter applies at processing (I1×I6, I1×I4 each alone) |
| I4×I7 | yes | b | A clone of a DOMParser node keeps the DOMParser document as owner (`clone_subtree` threads `owner_document(src)`, `tree_clone.rs:208`); processed at creation, its raw values read null under N1. A clone of template content keeps the page document as owner, which is why C1 is needed (§9.2 (c), §1.6) |
| I4×I8 | yes | b | A clone of a DOMParser `<body>` processed at creation resolves through step 3 to null |
| I4×I9 | yes | a | The marker's non-copy is a row of `elidex-ecs`'s clone-policy table that names an `elidex-script-session` component; `elidex-ecs` copies an explicit set, so no `elidex-ecs` code changes (§4.2 row 6) |
| I5×I6 | yes | a | The bind pass and the Insert arm apply the same `policy.installs` filter as HEAD (`:447`). Cell U3 |
| I5×I7 | yes | a, b | Slice a's bind pass starts at `dom.document_root()` (`dom/mod.rs:425`), so it never reaches a DOMParser document (created by `create_document_node`, which does not set the root, `:413`), and it does not walk template contents (H10). H-P13 stays true. Cell U8. The Insert arm keeps W-K5 (C14, C15). Slice b reaches both at creation (x4, x5, §1.6) |
| I5×I8 | yes | b | Slice b's DOMParser and fragment-build call sites run while the node document is inactive; step 3 must fire there (for the fragment build, before `finish_detached_fragment`'s adopt, O9) |
| I5×I9 | yes | a → b | The bind pass runs inside `initialize_consumers`, before the dispatcher is installed, so no `MutationEvent` re-enters (`vm_api.rs:313-314`). Slice b's call sites: O9, Q6 |
| I6×I7 | no | — | The policy filter is independent of the node document |
| I6×I8 | no | — | The policy filter runs before target resolution, on the name only |
| I6×I9 | yes | a | The VM passes `VmInner::spec_level_policy`, the value the dispatcher's consumer is built with (H19), so the writers and the consumer apply one policy |
| I7×I8 | yes | b | Both are null-reading for inert documents; slice b needs both prerequisites (x3, x4, x5) |
| I7×I9 | yes | b | The DOMParser call site is in `elidex-form`, the clone site in `elidex-dom-api` (both may call `elidex-script-session`, H23); the fragment-build and parser template sites cannot (O9) |
| I8×I9 | no | — | Step 3 lives in `resolve_handler_target` (engine-independent); no VM code is involved |

Count: 36 pairs; 32 intersect, 4 do not (I4×I6, I6×I7, I6×I8, I8×I9). Of the 32: slice a alone 14, slice b alone 10,
*a → b* 6, *a, b* 2 (re-count: `awk '/^### §2.1/{f=1} /^## §3/{f=0} f && /^\| I[0-9]×I[0-9]/' <memo> | awk -F'|' '$3 ~ /yes/
{print $4}' | sort | uniq -c`).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | event handler content attribute *attribute change steps* 1–5.6 | step 1 non-handler name → return; step 2 determine the target; step 3 null target → return; step 4 value null → deactivate; step 5.1 CSP inline check (not modelled: elidex has no CSP, and HEAD's `set_inline_handler` has none); step 5.5 set raw uncompiled handler; step 5.6 activate | a: `process_handler_content_attrs` (NEW) runs them for creation-time attributes; `handle_attribute_change` `crates/script/elidex-script-session/src/event_handler_consumer.rs:389` for writes (calls `note_handler_write` first). b: the same function at creation | ✓ (5.1 listed, not modelled) | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determining the target of an event handler* 1–4 | non-body → self; not WEH and not the reflecting set → self; step 3 inactive node document → null; else relevant global | `resolve_handler_target` `event_handler_consumer.rs:328` (unchanged by either slice; step 3 is T3 and the reflecting set is facet 4, both slot `#11-event-handler-target-determination`; b depends on T3) | ✗ | no |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter 1–4 | target null → return; value null → deactivate; else set value + activate | a: `native_event_handler_set` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:344`, `native_body_weh_set` `:459` (after the no-Window return `:468-470`) → `note_handler_write` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2 | raw value in a null-browsing-context document → null | `ensure_event_handler_current` `event_handler_attrs.rs:521` (unchanged; N1's node clause; template contents need C1; b's K-5; §L's columns) | ✗ | no |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | WindowEventHandlers rows | WEH rows on `<body>` delegated (HEAD) | `EVENT_HANDLER_ATTRS` `event_handler_consumer.rs:61`; cells C8, E4 use `onhashchange` | ✗ | no |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4 (*handle attribute changes*) | every attribute present at creation runs the attribute change steps once | `process_handler_content_attrs` (NEW): the realisation of those per-attribute runs | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* step 17.2, "Append attribute to element" | parser path: every token attribute is appended at creation | parser bakes `Attributes` at spawn (`crates/dom/elidex-html-parser/src/convert.rs:258`; strict `tree_builder/insert.rs:61`, `:119`, `shadow.rs:162`); a: processed at the first bind (pre-bind) or at first insertion; b: at fragment / DOMParser build | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", step 13 "Keep", sub-step 1 (declarative shadow root) | the shadow root is attached while parsing; its children are parsed into it | strict `tree_builder/shadow.rs:137`, `:162`; a: pre-bind DSD is processed by the bind pass (E7); fragment DSD is not processed at host insertion (D-10, HEAD-equal); b: at creation | ✗ | yes |
| WHATWG HTML §13.2.6.4.7 The "in body" insertion mode | start tag "html" step 2.2, start tag "body" step 3.2 | an attribute not already present is added to the existing element | no elidex-side writer (§5.0); not a creation-time attribute | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | step 4, the fragment's new `Document` | fragment elements are created in a throwaway document | `begin_detached_fragment` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:318` (unchanged in a; b's call site, O9) | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a single node* step 2.5; *clone a node* steps 5 (children) and 6.7 (shadow children) | a clone's attributes are appended at its own creation; the recursion clones each child the same way | a: clone-policy table `crates/core/elidex-ecs/src/dom/tree_clone.rs:19-34` (row added: marker non-copy); b: processing at clone time for every node of the cloned subtree (H18 path) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *template contents*; *appropriate template contents owner document*; the template *cloning steps* step 2 | contents are not descendants; their owner document has a null browsing context; a template clone's contents are cloned into the copy's contents | a: not walked by the bind pass (H10); the Insert arm keeps W-K5 (H22); b: §1.6 | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* | node document changes; no event-handler step | `EcsDom::adopt_subtree` `teardown.rs:285` (unchanged; the marker survives) | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert*, *insertion steps* | HTML defines no event-handler insertion steps | a: `handle_insert` `event_handler_consumer.rs:427` → process once over HEAD's walk set (D-10); b: deleted (D-3; §1.6 branch B keeps it) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including descendant* | walks include shadow trees | a: the bind pass only, `for_each_shadow_inclusive_descendant` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:33` | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` | `text/html` branch only | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged in a; b's call site) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter | fragment parse + replace children | fragment placement fires Insert (H6; unchanged in a; b processes at build) | ✗ | yes |
| WHATWG HTML §8.5.5 The outerHTML property | setter | fragment parse + replace self | as innerHTML (H6) | ✗ | yes |
| WHATWG HTML §8.5.2 HTML parsing and insertion methods | `setHTMLUnsafe` | fragment parse with declarative shadow roots allowed | as innerHTML (H6); its DSD content is §13.2.6.4.4's row | ✗ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author-controlled markup or script values. The mechanism stores no user string: the markers are
unit components, and the handler sources it writes are HEAD's (`set_inline_handler`). Slice a's bind pass newly makes
initial-HTML handler sources reachable by the compiler, which is the spec's behaviour (they are raw values at creation);
they compile only through HEAD's compile gate, with HEAD's global-only scope (H25; E-G1 (ii)). No row adds a parse of
user input.

## §4. Mechanism

**Home/kind (decided):** the once-key is a unit marker **component** on the origin element, and the bind-pass key a unit
marker **component** on the Document entity. Both are homed in
`crates/script/elidex-script-session/src/event_handler_consumer.rs` beside the consumer. Each is `Send + Sync`,
per-entity, and not a per-VM identity handle, so per CLAUDE.md's side-store→component rule each is a component. Despawn
removes them; adopt keeps the element marker; clone does not copy it. Everything below this paragraph is input to the
slices' plan-reviews.

### §4.1 Items (slice a)

```rust
// crates/script/elidex-script-session/src/event_handler_consumer.rs (NEW items)
/// The attribute change steps (WHATWG HTML §8.1.8.1) have run for this element's
/// creation-time handler content attributes (WHATWG DOM §4.9 "append an attribute"
/// step 4). Absent = unprocessed. Sparse; on the origin element, never the target.
pub struct HandlerAttrsProcessed;
/// The bind pass has processed this Document's pre-bind tree (D-5). One per document.
pub struct PreBindHandlerAttrsProcessed;
pub fn process_handler_content_attrs(policy: SpecLevelPolicy, element: Entity, dom: &mut EcsDom);
pub fn process_handler_content_attrs_in_subtree(policy: SpecLevelPolicy, root: Entity, dom: &mut EcsDom);
pub fn note_handler_write(policy: SpecLevelPolicy, origin: Entity, event_type: &str, dom: &mut EcsDom);
impl EventHandlerAttributeConsumer { pub fn initialize_from_tree(&mut self, dom: &mut EcsDom); }
```

- **`process_handler_content_attrs(policy, element, dom)`**: return if `element` is not an element or is marked. Collect
  its attributes named in `EVENT_HANDLER_ATTRS` (attribute order). If none, return (no marker). For each: skip it if the
  policy excludes it (`:447`'s filter); otherwise resolve the target (`resolve_handler_target`) and call
  `set_inline_handler`. Then insert the marker. Collect-then-write (HEAD's borrow discipline).
- **`process_handler_content_attrs_in_subtree(policy, root, dom)`**: collect `root` and its shadow-including descendants
  (`for_each_shadow_inclusive_descendant`, H10), then process each. Template contents are not visited (H10). Slice a's
  only caller is `initialize_from_tree`.
- **`handle_insert`** (rewritten, not new): collect `node` plus `traverse_descendants(node)` (HEAD's set, D-10), then
  call `process_handler_content_attrs` on each.
- **`note_handler_write(policy, origin, event_type, dom)`**: D-8's three lines.
- **`initialize_from_tree`**: if `dom.document_root()` is `Some(root)` and `root` lacks `PreBindHandlerAttrsProcessed`:
  process its subtree, then insert that marker on `root`.

### §4.2 Rewrite mapping (HEAD → slice a)

| # | site | HEAD | slice a |
|---|---|---|---|
| 1 | `handle_insert` `event_handler_consumer.rs:427-463` | light-tree walk, re-delivers on every Insert | the same walk; `process_handler_content_attrs(policy, e, dom)` per element (once) |
| 2 | `handle_attribute_change` `:389-421` | after the policy check (`:405`), resolves and writes | after the policy check, `note_handler_write(policy, node, event_type, dom)`, then HEAD's write and removal arms unchanged |
| 3 | `native_event_handler_set` `event_handler_attrs.rs:344` | `activate_event_handler(..)` | `note_handler_write` first (no-op for Document / Window / Worker receivers), then the same call; callable and null alike |
| 4 | `native_body_weh_set` `:459` | delegates to the Window | after the no-Window return (`:468-470`), `note_handler_write` on the **`<body>` receiver** (I3), then delegates |
| 5 | `ConsumerDispatcher::initialize_consumers` `crates/script/elidex-js/src/vm/consumer_dispatcher.rs:135-137`, doc `:113-134` | `base_url` only; doc says other consumers need no init | adds `self.event_handler_attrs.initialize_from_tree(dom)`; the doc states why (pre-bind parse, H9) and that it runs once per document (D-5) |
| 6 | `tree_clone.rs:30` "deliberate non-copy" row | — | adds `HandlerAttrsProcessed` (elidex-script-session): DOM §4.4 *clone a single node* step 2.5 appends the clone's attributes at its own creation |
| 7 | module docs `event_handler_consumer.rs:1-33`; `consumer_dispatcher.rs:58-66` field doc | "Arm 2 walks the inserted subtree's baked `on*` attributes" | the processing rule (D-3), the interim Insert arm, the bind arm |
| 8 | `crates/script/elidex-script-session/src/lib.rs:36-41` | re-exports | adds `HandlerAttrsProcessed`, `PreBindHandlerAttrsProcessed`, `note_handler_write` (VM), the two process functions (tests) |
| 9 | `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs:21-23` (G6) | says the deferred session flush writes `Attributes` without entering the chokepoints | corrected: `apply_set_attribute` routes through `EcsDom::set_attribute` (`mutation/mod.rs:897-909`). Docstring only |

`inert_document.rs` is not touched in slice a (P3 stays MINOR until slice b touches it).

### §4.3 Slice b's mechanism (input to its plan)

- Delete the D-8 guard line (§1.4).
- Call `process_handler_content_attrs_in_subtree` at the end of each bound creation path: the fragment build (before
  `finish_detached_fragment`'s adopt, so step 3 sees the throwaway document), the DOMParser build, the clone (H18), and,
  in §1.6 branch A, template-contents attachment. The call-site shape is O9.
- Delete the Insert arm (D-3). In §1.6 branch B, keep it unchanged and record its deletion under
  `#11-eh-template-contents-at-creation`.
- In branch A, the bind pass also walks template contents of the pre-bind tree (R1 A1's cross-axis note: without this
  the pre-bind parse's template contents would have no processing point once the Insert arm is gone). The walker is
  slice b's choice (H10's walker does not follow template contents).
- Place the creation-path detector (B7).
- Correct `inert_document.rs:78-84` (P3).

### §4.4 Alternatives rejected (and one adopted)

- **R-D3: rev 1's mechanism** (`HandlerAttrDelivery::Pending { superseded }` → `Delivered` at first insertion).
  Rejected. It adds a component whose only job is to bridge slice a's interim, and slice b would delete it. That makes
  the interim larger than the minimum the dependency requires: one guard line plus HEAD's Insert arm (§1.4). With G1 it
  would also be a second timing beside bind (§9.2 G1 paragraph). Its "absence = undelivered, fail toward HEAD" property
  is kept by D-1.
- **R1: a new `MutationEvent` (or AttributeChange) at every spawn.** AttributeChange at creation fans out to every
  consumer (custom-element `attributeChangedCallback`, canvas, base URL, form, MutationObserver records), changing each
  one's meaning. A dedicated creation event is one of slice b's candidate call-site shapes (O9), not slice a's.
- **R2: key on the target's entry.** The Window's entries are shared by every `<body>` and by `window.on*`, so a body
  clone would never deliver; the IDL null leaves no entry (`activate_event_handler` `:683-692`).
- **R3: a connectedness filter.** Remove-then-append re-delivers; a DOMParser document is connected; not the spec's
  condition.
- **R4: a positive "unprocessed" marker at spawn.** The spawn sites are below `elidex-script-session`, and a forgotten
  site leaves dead handlers. D-1's absence-means-unprocessed fails toward HEAD's delivery.
- **R5: compare source text.** No spec basis; "once" as a value coincidence.
- **R6: a session or `HostData` set.** Violates the side-store→component rule.
- **R7: unrestricted ensure-before-write in slice a.** On a DOMParser node it materialises raw siblings, which HEAD's
  getter compiles (needs N1), and on a DOMParser `<body>` it delivers WEH siblings to the page Window (needs T3). It is
  slice b's form, after those land.
- **R9: a dense marker** (mark every element at first sight, so writers need no rule). It avoids D-8's guard, but it
  costs one archetype move per element ever inserted or bound, for a marker whose information is "nothing to do".
- **R10 (adopted in rev 4 as D-5; a decision change made here, in the umbrella, under PM decision 23): bind pass at the
  first bind only.** Rev 3 rejected it because it misses elements created or written while unbound after the first
  bind (G7). G7 is now registered as latent and not reachable today (`#11-unbound-attribute-writes-bypass-consumers`),
  and rev 3's per-bind pass had the defect R1 A2 IMP-1 found: a later bind processes a shadow-root-direct element
  after a guarded write. R10's form, keyed per document by `PreBindHandlerAttrsProcessed`, removes that timing and
  resolves rev 3's O3 (one walk per document).
- **R11 (rev 4): the shadow-including Insert walk (rev 3).** Rejected under K-0 (D-10). A host's insertion would
  process shadow content that HEAD never processes at that event. On an element that took a guarded write, that
  overwrites the write. The improvement it bought (C10: a post-bind host's shadow content delivered) moves to slice b
  (B6).
- **R12 (rev 4): mark at placement under a shadow root.** No hook (P14).

### §4.5 ECS-native and Layering checks

- Idiom mapping: the OO "element.attributesProcessed flag" → a marker component on the entity; "document.initialized"
  → a marker component on the Document entity; "process the document at attach" → a system over the bound tree run at
  the dispatcher's install point, gated by the document marker; "writers notify the element" → writers calling one
  engine-independent function. No registry, no new dispatcher variant, no side-store.
- `vm/host/` adds two calls into `elidex_script_session` (marshalling). `consumer_dispatcher.rs` is the composer, not
  host code; it gains one delegate call, like `base_url`'s.
- Inside `dispatch` the consumer writes only `EventListeners` and the element marker (I9); the document marker is
  written only by `initialize_from_tree`, before install.
- Forward compatibility (R1 A2 MIN-2): the bind pass identifies the active document through the `EcsDom` singleton
  `document_root()`, as HEAD's `resolve_handler_target` does through `window_entity()`. Under an agent-scoped World
  with several documents, the bound document must be passed explicitly. That is the world-model program's change
  (PR #434, `docs/plans/2026-06-agent-scoped-ecsdom-world.md`; `#11-browsing-context-state-ecs-components`), and Q2
  checks the singleton assumption at every bind site today.

### §4.6 Docstring citation table (Axis 4)

| doc site | citation |
|---|---|
| `HandlerAttrsProcessed`, `process_handler_content_attrs` | WHATWG HTML §8.1.8.1 Event handlers (attribute change steps); WHATWG DOM §4.9 Interface Element (*append an attribute* step 4) |
| `PreBindHandlerAttrsProcessed`, `initialize_from_tree`, the `initialize_consumers` doc | WHATWG HTML §13.2.6.1 Creating and inserting nodes (*create an element for a token* step 17.2: attributes appended at creation) |
| `process_handler_content_attrs_in_subtree` | WHATWG DOM §4.8 Interface ShadowRoot (*shadow-including descendant*); WHATWG HTML §4.12.3 The template element (contents are not descendants) |
| `handle_insert` (`:423-426`, now "WHATWG DOM §4.2.3 insert") | WHATWG DOM §4.2.3 Mutation algorithms (*insert*: HTML defines no event-handler insertion steps, so insertion processes nothing that is already processed); the interim note naming `#11-eh-attrs-creation-time-call-sites` |
| `handle_attribute_change` (`:387-388`) | WHATWG HTML §8.1.8.1 (attribute change steps); unchanged citation, plus the `note_handler_write` line |
| module docs `event_handler_consumer.rs:1-33` (Arm 2's paragraph `:11-18`) | WHATWG DOM §4.2.3 Mutation algorithms; WHATWG DOM §4.9 Interface Element; WHATWG HTML §13.2.6.1 Creating and inserting nodes; WHATWG HTML §8.5.4 The innerHTML property, §8.5.5 The outerHTML property, §8.5.2 HTML parsing and insertion methods (replacing the rev-0 "§8.5 DOM-parsing APIs") |
| `note_handler_write` | WHATWG HTML §8.1.8.1 (IDL setter steps; attribute change steps 4–5) |
| `native_event_handler_set`, `native_body_weh_set` docs | WHATWG HTML §8.1.8.1 (IDL setter); §8.1.8.2 Event handlers on elements, Document objects, and Window objects (the `<body>` delegation) |
| clone-policy row | WHATWG DOM §4.4 Interface Node (*clone a single node* step 2.5) |
| `attribute_reconcile.rs:21-23` (G6) | none (a code-path statement, no spec claim) |

## §5. Populations and named risks (input)

### §5.0 `Attributes` writers: silent vs firing (census, reproducible)

A **silent writer** is a production code path that writes an element's `Attributes` without firing
`MutationEvent::AttributeChange` for that write. Population: every hit of the patterns below in `crates/`.

**Classification rule.** A hit `path:line` is **test** if `path` matches the ERE
`/tests/|/tests\.rs$|/tests_[^/]*\.rs$|_tests\.rs$|/benches/|test_helpers|test_support`, or if the file contains
`#[cfg(test)]` and `line` is greater than the line of its first occurrence. Otherwise it is **prod**. Command (run from
the repo root; `P` = prod, `T` = test):

```sh
c() { /usr/bin/grep -rnE --include='*.rs' "$1" crates | while IFS=: read -r f l _; do
  if echo "$f" | /usr/bin/grep -qE '/tests/|/tests\.rs$|/tests_[^/]*\.rs$|_tests\.rs$|/benches/|test_helpers|test_support'
  then echo "T $f:$l"; continue; fi
  t=$(/usr/bin/grep -n -m1 '#\[cfg(test)\]' "$f" | cut -d: -f1)
  if [ -n "$t" ] && [ "$l" -gt "$t" ]; then echo "T $f:$l"; else echo "P $f:$l"; fi; done; }
c '<pattern>' | /usr/bin/grep -c '^P'    # and '^T'; `| /usr/bin/grep '^P'` lists the prod sites
```

Run at `fd6955e8` on 2026-10-25:

| pattern (ERE) | total | test | prod | production sites → class |
|---|---|---|---|---|
| `get::<&mut (elidex_ecs::)?Attributes>` | 21 | 17 | 4 | `attribute.rs:215` (`write_attribute_no_dispatch`), `:305` (`remove_attribute`: **fires** when present, `:318`), `tree_clone.rs:68` (`clone_attributes`: **silent**, no prod caller), `content/iframe/lifecycle.rs:358` (iframe `src` rewrite: **silent**, `src` only; slot `#11-iframe-lifecycle-direct-attribute-writes`, §8.1; the file's `:516` hit is under `#[cfg(test)]` `:458`) |
| `insert_one\([^)]*(attrs\|Attributes\|attributes)` | 7 | 5 | 2 | `attribute.rs:224` (`write_attribute_no_dispatch`), `tree_clone.rs:72` (`clone_attributes`) |
| `builder\.add\(attrs\)` | 1 | 0 | 1 | `tree_clone.rs:368` (in `clone_node_shallow_unchecked` `:270`, the cloneNode path: **silent**, all attributes) |
| `\.create_element(_with_owner\|_ns)?\(` | 2210 | 2187 | 23 | 17 pass `Attributes::default()` (`teardown.rs:327`, `inert_document.rs:120`, `document.rs:235`, `entity_spawn.rs:57`, `select.rs:329`, `document_props.rs:337`, `anonymous_table.rs:159`, `grid.rs:422`, `:467`, `table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359`, `:448`, `:512`); 2 are shims (`dom/mod.rs:335`, `:380`); **4 bake parser attributes (silent)**: `elidex-html-parser/src/convert.rs:258`, `elidex-html-parser-strict/src/tree_builder/insert.rs:61`, `:119`, `tree_builder/shadow.rs:162` |
| `\.set_attribute_without_dispatch\(` | 1 | 0 | 1 | `crates/dom/elidex-form/src/value_mode.rs:64`: **silent**, `value` only |
| `\.clone_attributes\(` | 1 | 1 | 0 | test only (`dom/tests/clone.rs:14`) |
| `\.set_attribute\(` | 141 | 134 | 7 | all **fire** via the `attribute.rs:125` chokepoint: `mutation/mod.rs:903`, `select/mod.rs:156`, `:182`, `element/attrs.rs:208`, `:256`, `char_data/attr.rs:172`, `:333` |
| `\.remove_attribute\(` | 24 | 18 | 6 | 5 **fire** when present (`mutation/mod.rs:925`, `select/mod.rs:159`, `:178`, `dialog.rs:80`, `char_data/attr.rs:243`); 1 is a comment (`vm/host/attr_proto.rs:44`) |

Parser attribute merging has no elidex-side writer. In the spec this is the "in body" insertion mode's `html` start-tag
step 2.2 and `body` start-tag step 3.2 (WHATWG HTML §13.2.6.4.7 The "in body" insertion mode): an attribute not already
present is added to the existing element.
`/usr/bin/grep -rn --include='*.rs' -i -E "already present|merge_attr|add_missing|set_attribute|write_attribute"
crates/dom/elidex-html-parser-strict/src crates/dom/elidex-html-parser/src` lists no prod hit by the rule above.

Result:
- (a) The silent writers that can write an `on*` attribute are exactly the creation-time bakes: the four parser sites
  and `clone_node_shallow_unchecked` (plus the prod-unreachable `clone_attributes`). These define "creation-time
  attributes" in §0.5.
- (b) The other silent writers write fixed non-handler names (`value`, `src`).
- (c) Every post-creation `on*` write fires AttributeChange while a dispatcher is installed (7 + 5 firing sites, the table above; verified 2026-10-25).
  While **unbound** (no dispatcher, H13) a firing write is not observed; that is G7, registered as
  `#11-unbound-attribute-writes-bypass-consumers` (latent, not reachable today; §7).

### §5.1 Handler-state writers and readers

Commands: `/usr/bin/grep -rn --include='*.rs' -E
"set_uncompiled|clear_uncompiled|take_uncompiled|uncompiled_source|mark_cleared|is_handler_cleared" crates` and
`/usr/bin/grep -n -E "add_event_handler|find_event_handler" crates/script/elidex-script-session/src/event_handler_consumer.rs
crates/script/elidex-js/src/vm/host/event_handler_attrs.rs`.

| op | production sites | role in slice a |
|---|---|---|
| `set_uncompiled` | `event_handler_consumer.rs:474` (in `set_inline_handler`, from `:411` and the Insert arm) | `:411` unchanged; the Insert-arm and bind calls go through `process_handler_content_attrs` only |
| `clear_uncompiled` | `event_handler_attrs.rs:677`, `:687` (`activate_event_handler`) | unchanged; preceded by `note_handler_write` |
| `take_uncompiled` | `event_handler_attrs.rs:567` | unchanged (read-side compile) |
| `mark_cleared` | `event_handler_consumer.rs:487` (`clear_inline_handler`, from `:418`) | unchanged |
| `is_handler_cleared` | `event_handler_attrs.rs:568` | unchanged |
| `uncompiled_source` | none in production (`event_listener.rs:282`) | the store oracle for C2, U1, U2 |
| `add_event_handler` | `event_handler_consumer.rs:473`, `event_handler_attrs.rs:674` | unchanged |

Readers of `ensure_event_handler_current` (`/usr/bin/grep -rn --include='*.rs' 'ensure_event_handler_current('
crates/script/elidex-js/src`): `dispatch_target.rs:183`, `natives_promise.rs:548`, `engine.rs:233`. All read.

### §5.2 `MutationEvent::Insert` consumers

Command: `/usr/bin/grep -rn --include='*.rs' -E "MutationEvent::Insert" crates` (non-test arms): `LiveRangeBridge`
(`range/live.rs:121`), `BaseUrlMaintainer` (`document_base.rs:293`), `FormControlReconciler` (`reconciler.rs:44`),
`CustomElementReactionConsumer` (`consumer.rs:77`), this consumer (`event_handler_consumer.rs:381`). None reads
`EventListeners` (`consumer_dispatcher.rs:58-66`). No existing test moves an element carrying a handler content
attribute (`/usr/bin/grep -rln --include='*.rs' -E "setAttribute\(.on[a-z]+|onclick=" crates/script/elidex-js/src` →
`tests_event_handler_attrs.rs`, `tests_call_listener.rs`; neither does). No shell test asserts an initial-HTML inline
handler (`/usr/bin/grep -rn --include='*.rs' 'onclick=' crates/shell` → nothing), which is why G1 survived.

### §5.3 Creation paths and their processing point

| path | site | slice a | slice b |
|---|---|---|---|
| main-document parse (pre-bind) | `pipeline.rs:850-858` (`build_pipeline_from_loaded`; production callers H16) | **first bind** (G1) | same |
| URL-loaded iframe parse (pre-bind) | `load.rs:221` (same-origin), `:327` (out-of-process), both `build_pipeline_from_loaded` | **first bind** (G1) | same |
| srcdoc / about:blank iframe parse (pre-bind) | `load.rs:465` → `:471` → `pipeline.rs:735` | **first bind** (G1) | same |
| fragment (innerHTML / outerHTML / setHTMLUnsafe) | `teardown.rs:318`; `convert.rs:258`; strict `insert.rs:61`, `:119` | first insertion (placement, H6); none when placed directly under a shadow root (HEAD) | fragment build end |
| DOMParser | `inert_document.rs:110-200` | first insertion after return (W-K5 when that insertion is into a DOMParser document) | build end (after N1, T3) |
| `createElement` + `setAttribute` | `dom/mod.rs:344` + AttributeChange | the write processes and marks (D-8) | same |
| `cloneNode` | H18 | first insertion | clone time |
| declarative shadow (parser) | strict `shadow.rs:137`, `:162` | first bind (pre-bind parse); not processed in a fragment parse (the host's Insert walks the light tree, D-10) | creation |
| template contents | `dom/mod.rs:560` | placement into contents by `tpl.innerHTML` / script append (W-K5), or when moved out (Insert per child, HEAD) | §1.6 |

### §5.4 Interaction matrix (slice a)

| interaction | slice a outcome |
|---|---|
| innerHTML / outerHTML / setHTMLUnsafe | fresh elements, processed once at placement (light walk) |
| cloneNode | fresh, unmarked (I4); processed at first insertion; a write on the clone whose only deliverable attribute is the written one marks it (C5, C11) |
| importNode / adoptNode | not exposed (P1; `#11-cross-document-adopt-on-insert`); `adopt_subtree` keeps the marker |
| shadow trees (G2) | pre-bind shadow trees are processed by the bind pass (E7). After bind, an element inserted under a normal element inside a shadow tree fires Insert and is processed (HEAD's walk). An element placed directly under a shadow root, and a host's shadow content at the host's insertion, are not processed (HEAD-equal; U9; slice b's B6) |
| `<body>` WEH delegation | marker on the `<body>`; target the Window (`:328`). Cells C8, E4 |
| policy-excluded attributes | filtered as `:445-449`; they count toward the marker. Cell U3 |
| attribute removed while outside the tree | AttributeChange clears; processing reads `Attributes`, so the removed name is never processed (C12) |
| repeated binds | the bind pass runs at the first bind of each document only (document marker; E3, U7, U9) |
| inert containers | W-K5 (C14, C15) |

### §5.5 Named risks

- **K1: an unknown creation path.** In slice a, an element baked by such a path is processed at its first insertion,
  as at HEAD. After slice b (branch A) there is no Insert arm, so such an element would never be processed. B7's
  pre-insertion cells for each known path and the `debug_assert!` detect an omitted known path. An unknown path is an
  addition to §5.0's census, which slice b's plan re-runs.
- **K2: body-clone cross-origin write** (`c = document.body.cloneNode(); window.onhashchange = fn; append(c)`): the spec
  processes `onhashchange` at clone creation and `fn` wins; slice a processes at insertion and the raw value wins.
  Pre-existing; closes in slice b (B3).
- **K3: detached reads** (G5): a never-inserted clone of a live element reads null (spec: a compiled function).
  Pre-existing; closes in slice b (B2).
- **K4: walk cost.** The Insert arm keeps HEAD's walk and adds one `get::<&HandlerAttrsProcessed>` per element. The
  bind pass walks a document's shadow-including tree once, at its first bind (D-5), like the base-URL maintainer's
  `initialize_from_tree` (`document_base.rs:276-286` → `initialize_base_url_for_document` `:203`), which runs on every
  bind. Rev 3's O3 is resolved.
- **K5: G1 exposes G4 (E-G1 (i)).** Initial-HTML `<body onload>` / `onfocus` / … are now delivered, onto the **body**
  (P2), as HEAD already does for the same markup inserted after bind. `load` never arrives at the body, and
  focus-type events at the body run them. The fix is facet 4 of `#11-event-handler-target-determination`, whose trigger
  the PM set to slice a's landing (§8.1).
- **K6: G1 exposes the scope chain (E-G1 (ii)).** Initial-HTML handlers compile with the global scope only (H25), as
  HEAD's post-bind handlers do. `onchange="form.submit()"` does not resolve `form` and throws a `ReferenceError`; the
  spec's scope chain (document, form owner, element) is missing. Owner: `#11-event-handler-attribute-edge-cases` (core
  VM; §8.1). Fixing it is out of scope.

## §6. Test and coverage plan

*Red* = must fail on the slice's pre-fix base and pass after; the implementer runs each red cell against that base
first and records the failure in the PR description (red-first). *Guard* = passes at both, asserting HEAD-equality on a
shape where HEAD's outcome is also the spec's. *Window pin* = passes at both, asserting HEAD's outcome on a shape
where it is **not** the spec's; it names the cell that inverts it and is never cited as the correct answer. The "HEAD"
predictions come from H1 / H3 / H9 / H21 and are not measurements. Cell shapes and oracles are input; the assignment of
cells to slices is decided. Slice a's pre-fix base is its branch base (`fd6955e8` at this rev).

### §6.1 Slice a: VM integration and unit cells

VM, new file `crates/script/elidex-js/src/vm/tests/tests_event_handler_delivery.rs` (NEW), registered in
`crates/script/elidex-js/src/vm/tests/mod.rs`, reusing the `with_el` / `entity_of` shape of
`tests_event_handler_attrs.rs:38-76` with `el` connected. Dispatch is `dispatchEvent(new Event('click'))` (the VM has no
`click()`, N1 memo H-P14).

| cell | kind | shape | HEAD | oracle |
|---|---|---|---|---|
| C1 | red | `el.innerHTML = '<div id=t onclick="A">'`; `t.onclick = fn`; `el.append(t)` (move) | move re-raws A | `t.onclick === fn`; dispatch runs `fn` |
| C2 | red | `t = createElement('div')`; `el.append(t)`; `t.setAttribute('onclick', S)`; dispatch (compiles); `f = t.onclick`; `doc2.body.appendChild(t)` (N1 C8b(iv) shape; §L L1) | move re-raws S | store: `click` entry `uncompiled_source == None`; `t.onclick === f` |
| C3 | red | baked A via innerHTML; `f = t.onclick`; move twice live→live | each move recompiles | `t.onclick === f` after each |
| C4 | red | `c = createElement('div')` (never inserted); `c.setAttribute('onclick', A)`; dispatch on `c` (compiles); `f = c.onclick`; `doc2.body.append(c)` (N1 R2's R-a; §L L5) | first insert re-raws A | store `uncompiled_source == None`; `c.onclick === f` |
| C5 | red | DOMParser `<div id=d onclick="A">`; `d.onclick = fn`; variants: (i) `dp2.body.append(d)`, (ii) `el.append(d)`, (iii) `c = d.cloneNode()`, `c.onclick = fn`, `el.append(c)` (N1 R2's R-b, single-attribute form; §L L6) | first insert raws A over `fn` | `d.onclick === fn` (`c.onclick === fn`); dispatch runs `fn` in (ii), (iii) |
| C6 | guard | DOMParser `<div id=d onclick="A">` appended into `el` | delivered at insert | `typeof d.onclick === 'function'`; dispatch runs A |
| C7 | guard | delivered `t`, `f = t.onclick`; `t.setAttribute('onclick', B)` | — | `t.onclick !== f`; dispatch runs B |
| C8 | red | `b = createElement('body')`; `el.append(b)`; `b.setAttribute('onhashchange', X)`; `window.onhashchange = fn`; move `b` | move re-raws X onto the Window | `window.onhashchange === fn`; store: marker on `b`, none on the Window |
| C9 | guard | `t` delivered and IDL-set to `fn`; `c = t.cloneNode()`; insert `c` | — | before insert: `c` unmarked; after: `c.onclick` is compiled A (not `fn`) |
| C11 | red | `c = createElement('div')`; `c.setAttribute('onclick', A)`; `c.onclick = fn`; `el.append(c)` | first insert raws A over `fn` | `c.onclick === fn` |
| C12 | guard | clone of `onclick=A`; `c.removeAttribute('onclick')`; insert | — | `c.onclick === null` |
| C13 | guard | parser tiers: `el.innerHTML = '<button onclick=…>'` (strict) and a foster-parenting `<table>` string (tolerant fallback) | delivered | dispatch runs the body on both |
| C14 | window pin (W-K5, template) | `tpl.innerHTML = '<b id=x onclick="window.hit=1">'`; `tpl.content.getElementById('x').dispatchEvent(new Event('click'))` | runs | `window.hit === 1` (spec: does not run; inverted by B4 in §1.6 branch A) |
| C15 | window pin (W-K5, DOMParser) | `dp = parseFromString('<div id=x onclick="A">')`, `dp2 = parseFromString('')`; `x = dp.getElementById('x')` (unmarked; no entry, H5); `x.onclick` read (null); `dp2.body.append(x)`; getter | before: `null`; after: a function (Insert raws `A`; the flag-only compile gate compiles it) | after the append `typeof x.onclick === 'function'` (spec: null; inverted by N1's node clause, so once N1 lands this cell is replaced by N1's C8) |
| C16 | guard (K-0, T-a3 when the guard returns) | DOMParser `<div id=d onclick="A" onmouseover="C">`; `d.setAttribute('onclick', 'B')`; `d.onmouseover` | `null` (no `mouseover` entry: the build suppressed consumers, H5) | `d.onmouseover === null` (HEAD's and the spec's; an unguarded write would raw `C` and the flag-only gate would compile it) |

C10 (rev 3: a post-bind host's shadow content delivered at the host's insertion) is withdrawn from slice a (R11). Its
shape is slice b's B6 (iii).

Unit, new file `crates/script/elidex-script-session/src/tests_event_handler_delivery.rs` (NEW), `#[cfg(test)]` in
`lib.rs`, driving `EcsDom` with `EventHandlerAttributeConsumer` installed, through real `append_child` /
`set_attribute` calls so `Attributes` and the events agree.

| cell | kind | shape | oracle |
|---|---|---|---|
| U1 | red | element spawned with `onclick=A`, appended; `take_uncompiled`; appended elsewhere | `uncompiled_source == None` (HEAD `Some("A")`) |
| U2 | red | `<body onhashchange=X>` with a Window; append; `take_uncompiled` on the Window entry; move | Window entry `None`; marker on body only |
| U3 | guard | `BrowserCore`, `<body onstorage=…>` appended twice | no `storage` entry; body marked after the first append |
| U4 | guard | element with no handler attributes appended | unmarked |
| U5 | red (i) / guard (ii), (iii) | (i) unmarked element with only `onclick=A`: `set_attribute(onclick, B)` → marked, and the `click` entry is raw `B` (HEAD's state after the write); then append leaves the `click` taken-state intact. (ii) unmarked, never-inserted element with `onclick=A onmouseover=C`: `set_attribute(onclick, B)`, then append | (i) marked; raw `B`; after `take_uncompiled` and the append, still `None`. (ii) after the append: `click` raw `B`, `mouseover` raw `C`: HEAD's state, and the spec's for this shape (no compile in between). (iii) `note_handler_write` on a marked element → no change |
| U6 | guard | `clone_node_shallow` of a marked element | clone unmarked |
| U7 | red (G1) | tree built **without** a dispatcher (pre-bind shape) containing `<button onclick=A>`; call `initialize_from_tree` twice, with `take_uncompiled` between | after the first call the entry is raw A, the button marked, the document marked; after the second the entry is still `None` (not re-raw) |
| U8 | guard | `initialize_from_tree` with (i) a DOMParser-shaped second document (`create_document_node`) holding `<div onclick>`, (ii) a `<template>` whose contents hold `<div onclick>`, (iii) a shadow host with `<button onclick>` in its shadow root | (i), (ii) no entry and unmarked (H-P13, H10); (iii) delivered |
| U9 | guard (K-0; R1 A2 IMP-1) | after a first `initialize_from_tree`: a connected host `h` with shadow root `sr`; `x` spawned with `onclick=A onmouseover=C` and appended directly to `sr` (no Insert, H2); `note_handler_write(x, "click")` (returns at the guard), then a `click` handler entry with no source on `x` (the IDL write's store state, as `activate_event_handler` leaves it); then `h` moved; then `initialize_from_tree` again | `x` unmarked; the `click` entry still has no source (the write survives: HEAD's outcome and the spec's). The cell asserts nothing about `mouseover` |

### §6.2 Slice a: end-to-end pins (G1), by path

New file `crates/shell/elidex-shell/src/content_initial_handler_tests.rs` (NEW), registered in
`crates/shell/elidex-shell/src/content/mod.rs` beside `iframe_security_tests` (`:796-797`). `synth_cross_origin_loaded`
(`content_iframe_security_tests.rs:399`) and `run_oop_and_collect_posts` (`:423`) move into `content_test_support.rs`
(288 lines) so both files use them.

**Red-first rule (decided).** Each pin is committed and run on slice a's pre-fix base before the fix commit, and its
failure output is recorded in the PR description. A pin counts as red only if its **non-vacuity control** (a value the
initial script itself writes, so the harness and the script ran) is green on that base. Consumer-level cells (U7) do
not substitute for any pin.

Shared markup (`M`): `<body><button id=b onclick="document.body.dataset.hit=(+document.body.dataset.hit||0)+1"></button><script>document.body.dataset.ran=1;document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script></body>`.
Control: `data-ran == "1"`.

| path | pin | builder (H16, H17) | shape | base (pre-fix) | oracle |
|---|---|---|---|---|---|
| main document | E1 | `build_pipeline_from_loaded` (`pipeline.rs:839`; production callers `content/mod.rs:673`, `content/navigation.rs:276`, `app/navigation.rs:323`) | `synth_cross_origin_loaded(M, url)` passed to `build_pipeline_from_loaded` | `data-ran == "1"`, no `data-hit` | `data-hit == "1"` after the build (script-initiated dispatch inside the initial-scripts bind); `probe_attr` (`content_test_support.rs:273`)-style read |
| main document | E2 | same | then `PipelineResult::dispatch_event` (`lib.rs:293`) with a click at `b` | `data-hit` absent | `data-hit == "2"` (UA dispatch in its own bracket) |
| main document | E3 | same | `eval_script("window.f=document.getElementById('b').onclick; typeof window.f")`; `dispatch_event` (another bind); `eval_script("document.getElementById('b').onclick===window.f")` | first read `"object"` | `"function"`, then `true` (K-1 across H13's rebinds; the later bind processes nothing, D-5) |
| main document | E4 | same | `<body onhashchange="document.body.dataset.h=1"><script>document.body.dataset.ran=1;dispatchEvent(new Event('hashchange'))</script>` | `data-ran == "1"`, no `data-h` | `data-h == "1"` (delivered to the Window at bind, H14) |
| main document | E7 | same | `<div id=h><template shadowrootmode=open><button id=b onclick="…as M…"></button></template></div><script>document.body.dataset.ran=1;h.shadowRoot.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>` | `data-ran == "1"`, no `data-hit` | `data-hit == "1"` (the bind pass walks shadow trees) |
| URL-loaded iframe | E5 | `build_pipeline_from_loaded` via the out-of-process entry (`load.rs:327`) | `make_out_of_process_entry(synth_cross_origin_loaded(…, "https://other.example/"), PreEvalFrameState { unsandboxed, … })` with `<button id=b onclick="postMessage('eh-hit','*')"></button><script>postMessage('ran','*');document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>`; `run_oop_and_collect_posts` | posts = `["ran"]` | posts contain `"ran"` and `"eh-hit"` (`"ran"` is the control) |
| URL-loaded iframe | E5b (conditional, Q5) | `build_pipeline_from_loaded` via the same-origin URL entry (`load.rs:221`) | E5's markup through a hermetic same-origin loader, if Q5 finds one | as E5 | as E5. Whether E5b is written is decided in slice a's per-PR plan (from Q5's answer) and recorded there and in §10.1's DoD line; if not written, the plan states that `:221` and `:327` call the same builder, which E5 pins |
| srcdoc iframe | E6 | `build_iframe_pipeline` (`load.rs:465`) → `build_pipeline_interactive_shared` (`load.rs:471`; `pipeline.rs:735`) — **not** `build_pipeline_from_loaded` | `build_test_content_state_with_url` (`content_test_support.rs:147`) with `<iframe srcdoc='…M…'>` (the shape of `content_iframe_security_tests.rs:107`); read the in-process child's body | child `data-ran == "1"`, no `data-hit` | child `data-hit == "1"` |

### §6.3 Slice b's cells (assigned; shapes are input to its plan)

| cell | kind | shape | oracle | needs |
|---|---|---|---|---|
| B1 | red | DOMParser `<div id=d onclick=A onmouseover=C>`; `d.setAttribute('onclick', X)`; compile; insert; and the IDL form `d.onclick = fn` (N1 R2's R-a / R-b, multi-attribute) | compiled X (or `fn`) survives; `mouseover` raw C | N1 |
| B2 | red (G5) | `c = t.cloneNode()` of a live `onclick=A` element, never inserted | `typeof c.onclick === 'function'` | — |
| B3 | red (K2) | `c = document.body.cloneNode(); window.onhashchange = fn; el.append(c)` | `window.onhashchange === fn` | — |
| B4 | red (I7; inverts C14) | `tpl.content.querySelector('b').dispatchEvent(new Event('click'))` before a move out; then moved out | null / does not run before; compiles and runs after | C1 (branch A); in branch B it stays a window pin under `#11-eh-template-contents-at-creation` |
| B5 | red (I8) | DOMParser `<body onpageshow=…>`, before and after a move into the page | no Window entry either time | T3 |
| B6 | red (G2 rest) | (i) `sr.appendChild(x)` of an `onclick` element directly under a shadow root; (ii) `sr.innerHTML`; (iii) rev 3's C10: `h = createElement('div')`, `sr = h.attachShadow({mode:'open'})`, `sr.innerHTML = '<button id=b onclick="…">'`, `el.append(h)` | processed at creation; dispatch runs it | — |
| B7 | red (detector; D-3) | for each creation path of §5.3's slice-b column, a read before any insertion (`typeof x.onclick === 'function'` on the fresh node), plus the `debug_assert!` that slice b's plan places, exercised by a test-only creation path that skips processing | each path's pre-insertion read is a function; the assertion fires on the test-only path | — |
| B8 | red | live post-bind `c` with `onclick=A onmouseover=C`, never inserted; `c.onclick = fn` (and the `setAttribute` form); `el.append(c)` | `c.onclick === fn` (the D-8 guard's residual) | — |

Rev 3's "U5 (ii) inverted" row is withdrawn: U5 (ii) no longer pins the guard's internal state (§12 A3-IMP-1), and B8 is
the red cell for the residual.

### §6.4 Kept green, mutation guards, WPT (input)

Kept green: `tests_event_handler_attrs.rs` (35 tests: `/usr/bin/grep -c '#\[test\]'
crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` = 35) and the consumer's `spec_level_tests`
(`event_handler_consumer.rs:500`).

Mutation guards (slice a): dropping the marker check in `process_handler_content_attrs` fails C1–C3, C8, U1, U2, E3,
U7; deleting the bind delegate fails E1, E2, E4–E7, U7; dropping the document-marker check (a pass at every bind)
fails U9; a light-tree bind pass fails E7, U8 (iii); a shadow-including Insert walk fails U9 (the host move reaches `x`);
a `note_handler_write` that never processes fails C2, C4, C5, C11, U5 (i); one without the guard fails C16 (the
DOMParser node's `mouseover` sibling is materialised and compiles); the bind pass starting anywhere but `document_root`
(or walking template contents) fails U8 (i) / (ii).

WPT: no subset is claimed (engine-unit, VM-integration and shell end-to-end only).

### §6.5 Touch-time 1000-line check, slice a (`wc -l` at `fd6955e8`)

| file | lines | after (est.) | action |
|---|---|---|---|
| `crates/script/elidex-script-session/src/event_handler_consumer.rs` | 590 | ≈ 670 | none; tests go to the new file |
| `crates/script/elidex-script-session/src/lib.rs` | 62 | ≈ 69 | none |
| `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | 726 | ≈ 732 | none |
| `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | 153 | ≈ 165 | none |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 197 | 198 | none |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | ≈ 426 | none |
| `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs` | 66 | ≈ 66 | none (docstring only, G6) |
| `crates/shell/elidex-shell/src/content/mod.rs` | 858 | 862 | none |
| `crates/shell/elidex-shell/src/content_test_support.rs` | 288 | ≈ 340 | none (receives the two hoisted helpers) |
| `crates/shell/elidex-shell/src/content_iframe_security_tests.rs` | 667 | ≈ 615 | none (helpers move out) |

`crates/shell/elidex-shell/src/tests.rs` (1082 lines) is not touched; the pins go to the new file. No touched file
exceeds 1000 lines.

## §7. Gaps: dispositions (decided)

Inclusion criterion (PM decision 18 (2)): a gap whose root cause is the delivery mechanism this program builds is in
scope; any other becomes a slot proposal.

- **G1, initial-HTML handlers never wired** (H9). **In scope, slice a** (D-5; E1–E7, U7). Root cause: elements created
  before any consumer existed are never processed (I5). Confirmed by static trace (H9, H16, H17) and by the absence of
  any shell test (§5.2). Its exposures E-G1 (K-0) are registered elsewhere (§8.1).
- **G2, shadow-tree elements never delivered** (H2, H10). **In scope.** Slice a: the pre-bind shadow trees, through the
  bind pass (E7, U8 (iii)). Slice b: the rest (B6), by creation-time processing. `fire_after_insert`'s shadow-root filter
  (`mutation.rs:289`) is **not** changed: it serves every consumer, and slice b does not need it (P14).
- **G3, target step 3 not modelled.** Slot `#11-event-handler-target-determination` (facet 1). Not here; it is slice
  b's prerequisite T3 (x3).
- **G4, the Window-reflecting body set.** Slot `#11-event-handler-target-determination` (facet 4, merged by the PM;
  trigger: slice a's landing). Not here. Interaction: K5, E-G1 (i).
- **G5, reads never deliver** (K3). **In scope, slice b** (B2). Root cause: processing deferred past creation.
- **G6, stale docstring** `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs:21-23` (says the deferred session
  flush writes `Attributes` without the chokepoints; `apply_set_attribute` now routes through `EcsDom::set_attribute`,
  `mutation/mod.rs:897-909`). **Fixed in slice a** (§4.2 row 9; docstring only), so no slot is needed.
- **G7, writes and creations while unbound are not observed.** With no dispatcher installed (H13), an attribute write
  or a node insertion fires `MutationEvent`s to no consumer. Root cause: dispatcher absence while unbound, not this
  mechanism. **Registered** by the PM as `#11-unbound-attribute-writes-bypass-consumers` (latent: not reachable today;
  owner `EH-attrs-at-creation`; re-eval 2026-12-31). D-5's first-bind-only pass relies on it staying latent (§8.1).

## §8. Slots, records, hand-offs

**Registration timing**: a gap that exists at HEAD is registered when the PM accepts the proposal; a gap created by code
a slice adds is registered at that slice's landing. The PM registers every slot; this memo writes defect texts only.

### §8.1 Existing slots cited

- `#11-event-handler-target-determination` (registered by the PM 2026-10-25; facets (1) step 3, (2) step 4, (3)
  frameset, (4) the Window-reflecting body set). Facet (1) is **T3**, slice b's prerequisite (x3). Facet (4) is G4; its
  trigger is "slice a's landing makes it production-reachable" (K5, E-G1 (i)).
- `#11-template-contents-owner-document` (carved from #380) is **C1**, slice b's x5 (§1.6). No driver; deliberately
  deferred; the PM decides its handling after slice a and N1 land.
- `#11-cross-document-adopt-on-insert`: importNode / adoptNode and insertion adoption (P1). Not touched.
- `#11-unbound-attribute-writes-bypass-consumers` (G7; registered; latent, not reachable today; owner
  `EH-attrs-at-creation`; re-eval 2026-12-31). D-5 depends on its staying latent: if it becomes reachable, the
  elements it creates are unprocessed at bind, and the slot's fix must cover them.
- `#11-iframe-lifecycle-direct-attribute-writes` (registered; owner IBP-sandbox). Its only production writer is
  `content/iframe/lifecycle.rs:358`, which writes `src` only (§5.0); `:516` is under `#[cfg(test)]` (`:458`). It writes
  no `on*` attribute, so it does not enter "creation-time attributes".
- `#11-event-handler-attribute-edge-cases` (re-registered by the PM; facets: the handler scope chain, the onerror
  five-argument form; owner core VM; trigger: slice a's exposure; re-eval 2026-11-30). Slice a's G1 fix makes initial
  HTML reach it (K6, E-G1 (ii)): for example, `onchange="form.submit()"` does not resolve `form`. Out of scope here.

### §8.2 Proposed slot texts (for the PM; defects only)

> **`#11-eh-attrs-creation-time-call-sites`** — owner: Layout lane (program `EH-attrs-at-creation`). **What it
> records:** the approved interim of slice a (PM decision 23). In elidex, an element created after bind is processed at
> its first insertion, over HEAD's light-tree walk, instead of at its creation, and a handler write is restricted by a
> guard in `note_handler_write`
> (`crates/script/elidex-script-session/src/event_handler_consumer.rs`). **Defects** (the observable behaviour, both
> at HEAD and after slice a): (1) a handler write (`setAttribute` / `removeAttribute` of `on<T>`, or
> `E.on<T> = …`) on a never-inserted element with two or more deliverable handler content attributes is overwritten at
> the element's first insertion (at HEAD: for every element and at every insertion); (2) a never-inserted clone of an
> element with a handler content attribute reads `null` from `on<T>`; (3) after `c = document.body.cloneNode()`, a
> `window.on<T>` assignment is overwritten by `c`'s attribute at `c`'s first insertion; (4) an element with a handler
> content attribute placed directly under a shadow root after bind, or in a host's shadow tree when the host is
> inserted after bind, is never processed; (5) DOMParser and template-content elements are processed at their first
> insertion, not at creation, and an unmarked element placed into template contents or a DOMParser document reads a
> non-null handler (W-K5). **Classification:** all five are **pre-existing**. Each exists at HEAD with the same
> outcome or a wider population, and slice a narrows (1) to multi-attribute elements and makes no other worse (K-0).
> The guard line and the Insert-arm processing call are slice a's own code, but each carries a pre-existing defect and
> creates none. So under §8's rule the slot is **registered when the PM accepts this text** (at this umbrella's
> approval), not at slice a's landing. **Why deferred:** the end state processes at creation. That writes raw values on
> DOMParser nodes (null-reading only under N1, x4), on DOMParser bodies (needs T3, x3), and on template contents (needs
> C1, x5). N1 itself needs slice a first (§L). **Closes:** at slice b's landing. Slice b deletes the guard line and the
> Insert arm (§1.4). In §1.6 branch B, defect (5)'s template half moves to `#11-eh-template-contents-at-creation`.
> **Trigger:** T3 and N1 landed, and C1's handling decided. **Re-eval:** 2026-12-31. **Confidence:** medium. The order is
> fixed (PM decisions 21, 23), but slice b's start waits on T3 (owner Layout, not started) and on C1's handling (no
> driver).

> **`#11-eh-template-contents-at-creation`** (proposed only for §1.6 branch B; register when the PM decides not to land
> C1) — owner: Layout lane (program `EH-attrs-at-creation`). **Defect:** template contents (and clones of them) are
> processed when they leave the template contents, through the consumer's Insert arm, not at creation. Contents placed
> by `tpl.innerHTML` or a script append read a non-null handler and run it while inert (WHATWG HTML §4.12.3: the
> template contents owner document has a null browsing context). **Classification:** pre-existing (HEAD-equal; slice b
> leaves it). **Why deferred:** processing at creation without C1's per-node inert owner would make parser-built
> contents read non-null where HEAD reads null (worse than HEAD). **Closes:** after C1 lands, processing at creation
> plus deleting the Insert arm. **Trigger:** `#11-template-contents-owner-document` landed. **Re-eval:** at the C1
> decision + 90 days. **Confidence:** low (C1 has no driver).

### §8.3 Records for existing slots (for the PM)

- `#11-template-contents-owner-document`: add "`EH-attrs-at-creation` slice b" as a dependant (x5): in branch A slice b
  cannot process template contents at creation until the inert owner is carried per node (§9.2 (c)); in branch B the
  dependant is `#11-eh-template-contents-at-creation`.
- `#11-event-handler-target-determination`: add "`EH-attrs-at-creation` slice b" as a dependant of facet (1) (x3). Facet
  (4)'s trigger (slice a's landing) is already set; record E-G1 (i) as its first production exposure (K5).
- `#11-event-handler-attribute-edge-cases`: record E-G1 (ii) (K6) as the exposure its trigger names.
- `#11-unbound-attribute-writes-bypass-consumers`: record that D-5 relies on its latency (§8.1).

### §8.4 Hand-off: what N1's rev 4 must change (O8; N1 is not edited here)

Coordinates are N1 memo `0f99538d` lines unless stated.

1. **Rename the prerequisite.** Every `EH-insert-once` becomes "`EH-attrs-at-creation` slice a", and the path
   `docs/plans/2026-10-event-handler-insert-once.md` becomes `docs/plans/2026-10-event-handler-attrs-at-creation.md`:
   `:21`, `:29`, `:88`, `:188`, `:202`, `:282`, `:322`, `:358`, `:407`, `:447`, `:476`, `:559`, `:575`, `:587`, `:623`,
   `:633`, `:634`, `:671`, `:728`, `:733` (`:540` is revision history; N1's idiom decides).
2. **Restate P-N5 / H-P16 against K-1** (§0.5): "creation-time handler content attributes are processed at most once;
   no insertion, move or rebind writes handler state from them", not "delivered at first insertion".
3. **R2 G2 (R-a):** cite slice a's K-1 / K-2 with cell C4 for a single-attribute element; the multi-attribute case is
   slice b's B1. N1 decides whether the single-attribute form suffices for its cells (O5).
4. **R2 G3 (R-b):** cite K-3 with cell C5 (i)–(iii) for a single-attribute element; multi-attribute is B1.
5. **H-P13 stays** (P9); add U8 (i) as the evidence that slice a's bind pass does not reach a DOMParser document.
6. **NR7 (`:575`)**: "on a base without slice a", and cite this memo's §L (rows L1.1, L2.1, L5, L6.1) as the
   measurement behind it: without slice a, N1 is worse than HEAD on the getter.
7. **The `:623` row** ("the Insert arm turns handlers raw on every move"): fixed by slice a (K-1).
8. **§9 prerequisites (`:671`)**: the re-measure obligation on the post-slice-a base (N1 plan-review R2 G4) names
   slice a's landing commit.
9. **Check, not edit:** whether any N1 cell builds initial HTML through a shell pipeline. From slice a the bind pass
   delivers its inline handlers (G1), which may change such a cell's HEAD prediction.
10. **IBP-sandbox umbrella, "Prerequisite outside the DAG" paragraph** (`git show 0f99538d:docs/plans/2026-09-ibp-sandbox.md`
    line 38; it lands with N1): replace the label `EH-insert-once`, the retired path
    `docs/plans/2026-10-event-handler-insert-once.md`, and the contract text "a baked attribute becomes a raw value once
    per element, never on a later move" with "`EH-attrs-at-creation` slice a"
    (`docs/plans/2026-10-event-handler-attrs-at-creation.md`; contract K-1: "creation-time handler content attributes
    are processed at most once; no insertion, move or rebind writes handler state from them"), and cite §L for "N1
    depends on it landing first".

## §9. Landing-order comparison and the R8-vs-rev-1 decision record (kept from rev 2; amended in rev 4)

### §9.1 The question

Rev 1 built an interim (`Pending { superseded }` → `Delivered` at first insertion) and named R8 (the change steps at
write time, creation included) as the end state. CLAUDE.md "One issue, one way" forbids landing an interim that a later
PR deletes. So rev 2 asked: is there a landing order under which R8 lands with no interim? Rev 4 answers it with §L: no.
R8 needs T3, N1 and C1's handling (below), and N1 needs slice a (§L).

### §9.2 Each blocker, its removal, and the evidence

R8's blockers are exposures that R8 creates on populations whose spec outcome HEAD cannot yet produce.

| # | population | exposure if R8 lands on `fd6955e8` | evidence | removed by |
|---|---|---|---|---|
| (a) | DOMParser nodes with handler attributes | parse-time raw entries; HEAD's compile gate reads the HostData flag only, so `d.onclick` compiles (spec: null, *get the current value* step 3.2) | N1 memo §4.2 / D-N2 (the node clause is added to the compile gate by N1); `scripting.rs:83-113` (the clause exists, but at HEAD the compile gate does not consult it) | **N1 landing first.** H-P13 does not need rewriting: it describes N1's base, which then does not contain R8's DOMParser processing. N1's own DOMParser cells write handlers after the parse (N1 memo H-P13), so none depends on R8 |
| (b) | a DOMParser (or fragment-throwaway) `<body>` with a WindowEventHandlers attribute | parse-time delivery to the **page** Window: `resolve_handler_target` returns `dom.window_entity()` for any `<body>` (`event_handler_consumer.rs:328-334`) | `webref body html determining-the-target-of-an-event-handler`: step 3 "If eventTarget's node document is not an active document, then return null." HEAD has no step 3 (slot facet 1) | **T3 landing first.** T3 can reuse the effective-node-document rule in `scripting.rs:100-111` against `EcsDom::document_root` (`dom/mod.rs:425`). With T3, R8 is spec-exact here: a DOMParser body's WEH attribute writes nothing, and moving that body into the page does not deliver it |
| (c) | template contents and their clones | parse-time raw entries on template-content elements; the composition does not suppress them (H15), so after N1 they compile, and `tpl.content.querySelector('b').dispatchEvent(new Event('click'))` runs the inline handler (spec: the template contents owner document has a null browsing context, `webref body html appropriate-template-contents-owner-document` step 1.1.1, so null). A clone of template content keeps the page document as owner (`clone_subtree` threads `owner_document(src)`, `tree_clone.rs:208`; the template *cloning steps* step 2 clone contents into the copy's contents) | H15 | **C1 landing first** (§1.6 branch A), or the residue slot (§1.6 branch B). A narrower clause-(b) facet ("composed root is a template-contents fragment") does not suffice: a clone of template content is not under such a fragment, so the inert owner must be carried per node, which is C1's part (a). C1's part (b) (adopt into the inert owner) is needed for `tpl.innerHTML = …` results |
| (d) | the pre-bind parse (G1) | none; R8 needs a bind-time pass because no consumer exists at parse (H9) | H9, H13 | nothing: the bind pass is K-4's own arm in the end state, not part of slice a's interim surface (§1.4). D-5 runs it once per document |

**G1's effect on the comparison.** G1 adds the bind pass (d). Under R8 it is the creation-equivalent point for pre-bind
elements: no script runs between the parse and the first bind, so processing at that bind is observably processing at
creation. Under rev 1's D-3 it would be a **second** delivery timing beside first insertion, which is the strangler mix
"One issue, one way" forbids. So G1 strengthens the case for R8, and it forces D-1's per-element once-key whatever the
mechanism.

### §9.3 Verdict (rev 2), the decision (rev 3), the correction (rev 4)

Rev 2 concluded "R8 lands with no interim mechanism: yes" and compared two landings:

1. **No split**: order T3 → N1 → C1 → the whole work. **Withdrawn in rev 4 (§L):** it lands N1 first, and N1-alone is
   worse than HEAD on the getter of a compiled or IDL-set handler moved into a DOMParser document (L1.1, L2.1, L5,
   L6.1). Rev 2 recorded this as "N1 cannot carry C8b (iv)", a cost to N1's cells, and did not compare it with HEAD.
   It is a regression on main.
2. **Two slices** (rev 2's recommendation): slice a now, with no prerequisite, carrying G1 and N1's dependency; slice b
   after T3, N1 and C1's handling.

**PM decision 21 adopts landing 2** (§1). **PM decision 23** corrects its classification: slice a is an approved
interim, required by dependency (§1.4). Rev 2's "no interim mechanism" holds only in the narrow sense that slice a adds
no component that slice b deletes. Slice a's timing is interim, and so are the guard and the Insert arm.

## §10. Definition of done

### §10.1 Slice a

- `HandlerAttrsProcessed`, `PreBindHandlerAttrsProcessed`, `process_handler_content_attrs`,
  `process_handler_content_attrs_in_subtree`, `note_handler_write`, and
  `EventHandlerAttributeConsumer::initialize_from_tree` exist in `event_handler_consumer.rs` and are re-exported from
  `lib.rs`. `handle_insert` contains no direct `set_inline_handler` call (`/usr/bin/grep -n 'set_inline_handler('
  crates/script/elidex-script-session/src/event_handler_consumer.rs` → the definition, `handle_attribute_change`, and
  `process_handler_content_attrs` only).
- Walk sets (D-10): `/usr/bin/grep -n 'traverse_descendants\|for_each_shadow_inclusive_descendant'
  crates/script/elidex-script-session/src/event_handler_consumer.rs` → `traverse_descendants` in `handle_insert` only,
  `for_each_shadow_inclusive_descendant` in `process_handler_content_attrs_in_subtree` only.
- `initialize_from_tree` checks and sets `PreBindHandlerAttrsProcessed` on `dom.document_root()` (D-5).
- `note_handler_write` has D-8's shape; its guard line carries a comment naming `#11-eh-attrs-creation-time-call-sites`.
  `handle_insert`'s doc names the same slot as the interim's closure.
- `initialize_consumers` calls `event_handler_attrs.initialize_from_tree`; its doc no longer says the consumer needs no
  init.
- Both VM element setters call `note_handler_write` with `VmInner::spec_level_policy` before `activate_event_handler`;
  in `native_body_weh_set`, after the no-Window return (D-6). No other VM code touches either marker outside tests.
- Clone-policy row added; module and field docs updated (§4.6); the G6 docstring corrected.
- Cells C1–C9, C11–C16, U1–U9 and pins E1–E7 present (E5b per slice a's plan's recorded Q5 decision). Every red cell and
  every pin was run red on the pre-fix base, with its control green, and the failures recorded in the PR description.
  All pass at the head.
- `cargo test -p elidex-script-session --all-features`, `cargo test -p elidex-js --all-features`,
  `cargo test -p elidex-shell --all-features`, `cargo test -p elidex-ecs --all-features`, and `mise run ci` are green.
- `#11-eh-attrs-creation-time-call-sites` (§8.2) was registered at this umbrella's approval; slice a's PR cites it.

### §10.2 Slice b

- Its own per-PR plan-memo has passed `/elidex-plan-review`; its base contains T3 and N1, and C1's handling is decided
  (branch A: C1 has landed; branch B: `#11-eh-template-contents-at-creation` is registered).
- The D-8 guard line is deleted, and nothing else in `note_handler_write` changes (`git diff` of that function = one
  removed line plus its comment).
- Branch A: `handle_insert` and the `MutationEvent::Insert` match arm of the consumer's `handle` (`event_handler_consumer.rs:373`, `:381`) are deleted;
  `/usr/bin/grep -n 'MutationEvent::Insert' crates/script/elidex-script-session/src/event_handler_consumer.rs` → no
  non-test hit. Branch B: both are unchanged, and their doc names `#11-eh-template-contents-at-creation`.
- Every bound creation path of §5.3's slice-b column calls `process_handler_content_attrs_in_subtree` at creation (O9's
  chosen shape); in branch B, template-contents attachment does not.
- `inert_document.rs:78-84` corrected (P3).
- Cells B1–B8 present, red-first on slice b's base (B4 in branch A only); slice a's cells and pins stay green, except the
  window pins C14 (inverted by B4 in branch A) and C15 (replaced by N1's C8 once N1 has landed).
- `#11-eh-attrs-creation-time-call-sites` is closed by the PM.

### §10.3 Program

- Both slices have landed in the §1.3 order, each with its own reviewed plan-memo.
- Branch A: K-0 to K-6 hold at slice b's landing without qualifiers. Branch B: K-4 and K-5 carry the
  `#11-eh-template-contents-at-creation` qualifier; the others hold without qualifiers.
- **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-10-event-handler-attrs-at-creation.md` exits 0.

## §11. Open for plan-review

Decided (no longer open):
- **O1, split:** decided by PM decision 21 (D-7, §1).
- **Order:** decided by PM decision 21 (§1.2); settled with cells by §L (PM decision 23).
- **O2, One issue, one way:** decided by PM decision 23. Slice a is an approved interim, required by dependency (§1.4),
  and its closure is recorded in `#11-eh-attrs-creation-time-call-sites`.
- **O3, walk cost:** resolved by D-5 (one bind-pass walk per document). R9 stays rejected.
- **O6, the Insert arm after slice b:** decided by PM decision 23. It is not kept: slice b deletes it (D-3), and §1.6
  branch B is the only exception. Omitted creation paths are detected by B7.
- **O7, G7:** registered as `#11-unbound-attribute-writes-bypass-consumers` (§8.1).
- **O8, the N1 edits:** routed to N1's rev 4; the list is §8.4.

Open:
- **O4: writer completeness.** Is §5.1 + §5.0 complete for handler state? In particular any VM path that writes
  `EventListeners` handler state outside `activate_event_handler`.
- **O5: N1's R-a / R-b.** In slice a they hold only for elements with one deliverable handler attribute (C4, C5). Is that
  sufficient for N1's landing, given that N1's own cells do not use multi-attribute elements (N1 memo §6)?
- **O9 (slice b): the creation-time call-site shape (re-posed in rev 4).** The callers split by what they can call (H23):
  - **Direct-call-incapable:** the fragment build (`begin/finish_detached_fragment`, called from `elidex-html-parser`
    `lib.rs:142`, `:163` and strict `tree_builder/mod.rs:178`, `:262`); parser-built template contents
    (`attach_template_contents` from `convert.rs:288`, strict `shadow.rs:137`); parser-built declarative shadow
    content (strict `shadow.rs:162`). The parser crates cannot depend on `elidex-script-session` (a cycle: P15), and
    the session's own fragment apply (`html_fragment.rs`) runs only after `finish_detached_fragment`'s adopt, where step
    3 no longer sees the throwaway document. For these callers O9 is not a two-way choice. The processing signal must
    come from `elidex-ecs` (for example, a `MutationEvent` variant fired at the fragment's finish, before the adopt, and
    at `attach_template_contents`, through the dispatcher that `elidex-ecs` already owns) or from a parser hook. Any
    parser hook is a new extension point, so CLAUDE.md's *Plugin-first extensibility* requires it to be expressed in the
    `elidex-plugin` model, not as an ad hoc callback.
  - **Direct-call-capable:** the clone (`elidex-dom-api`, which already runs `apply_clone_form_state` synchronously at
    clone time with no `MutationEvent`, `tree_clone.rs:16`), the DOMParser build (`elidex-form`, as
    `BaseUrlMaintainer`'s `initialize_base_url_for_document` at `inert_document.rs:196`), and the dom-api template sites
    (`document.rs:264`, `clone.rs:422`).
  - **The question:** one shape for all callers (the `elidex-ecs` event, since the incapable callers force it), or the
    event for the incapable callers plus direct calls for the capable ones. "One issue, one way" weighs toward one
    shape; slice b's plan decides, with Q6.
- **O10: approval vehicle.** Whether this umbrella lands as a docs-only approval PR (the IBP-sandbox idiom) or with
  slice a's PR. The PM decides.

Questions for a scout (facts this rev could not establish by its own reads):

- **Q1** (answered): G7's reachability. The PM registered it as latent, not reachable today (§8.1).
- **Q2** At every bind site, is the bound `document` equal to `dom.document_root()` (main pipeline and the three iframe
  builders)? The bind pass, its document marker and T3 all assume it (§4.5).
- **Q3** Does elidex fetch or fire `load` / `error` for an `<img>` in template contents or in a clone of them? (The size
  of §9.2 (c)'s exposure.)
- **Q4** (answered by H23): `elidex-form` and `elidex-dom-api` both depend on `elidex-script-session`; the parser crates
  cannot.
- **Q5** Is there a hermetic harness for the same-origin URL iframe path (`load.rs:221`)? (E5b.)
- **Q6** Does any `MutationEvent` consumer match exhaustively, so that a new variant (O9's event shape) would need edits
  there?

## §12. R1 review disposition (umbrella plan-review R1 on `399f4534`; CRIT 0 / IMP 9 / MIN 18)

Each finding was re-verified against `fd6955e8` and webref before disposition.

| id | sev | finding (short) | disposition |
|---|---|---|---|
| A1-IMP | IMP | O9's direct-call shape is unbuildable for the fragment build and template contents; Q4 asks only about form / dom-api | **fixed**: H23 (crate graph verified; P15 corrects one detail); O9 re-posed with the direct-call-incapable callers and the Plugin-first constraint; Q4 answered |
| A1-MIN | MIN | `note_handler_write` placement vs `native_body_weh_set`'s no-Window return (`:468-470`) | **fixed**: D-6, §4.2 row 4, §10.1 (after the return; H24) |
| A1-note | note | pre-bind template contents are skipped by the bind pass, so K-4 / §10.3 "without qualifiers" may fail after slice b | **fixed**: §4.3 (branch A walks pre-bind template contents), §1.6, §10.3 |
| A3-IMP-1 | IMP | §1.4 vs §9.2: slice a has its own timings that slice b replaces; U5 (ii) pins the residual; "not an interim" fails for timing | **fixed**: §1.4 renamed and rewritten (PM decision 23); K-0 enumerates the timings; U5 (ii) rewritten to a HEAD = spec shape; B8 is the residual's red |
| A3-IMP-2 | IMP | §0 decides "not an interim" while O2 keeps it open; §1.4's "exactly one line" depends on O6 | **fixed**: O2 and O6 decided (§11); the approval scope names the approved interim and its two deleted items |
| A3-IMP-3 | IMP | the post-slice-b Insert-arm fallback is a silent release shim; D-3 pre-empts O6 | **fixed**: D-3 rewritten (no fallback; detection by B7); O6 decided |
| A3-IMP-4 | IMP | the no-C1 branch is unspecified | **fixed**: §1.6 (both branches), x5, K-4, K-5, §8.2 residue slot, §10.2, §10.3 |
| A3-MIN-1 | MIN | §1.5's universal claim vs K5 | **fixed**: K-0's exception class E-G1; §1.5 lists it |
| A3-MIN-2 | MIN | E5b's conditional recorded only in the PR description | **fixed**: §6.2 E5b row, §10.1 (decided and recorded in slice a's plan) |
| A3-MIN-3 | MIN | G6 has no slot | **fixed**: G6 fixed in slice a (§4.2 row 9), so no slot is needed |
| A3-MIN-4 | MIN | the guard slot lacks a re-eval date | **fixed**: §8.2 slot text (re-eval, confidence, trigger, owner) |
| A3-MIN-5 | MIN | the R10 fallback would change decided K-6 / D-5, so it must return to the umbrella | **fixed**: R10 is decided here (D-5, §4.4), in the umbrella, under PM decision 23 |
| A4-1 | MIN | §3 lacks declarative shadow root parsing | **fixed**: §3 row §13.2.6.4.4 (start tag "template", step 13 "Keep", sub-step 1; webref) |
| A4-2 | MIN | clone recursion and the template cloning steps not cited | **fixed**: §3 DOM §4.4 row (*clone a node* steps 5, 6.7), §4.12.3 row (cloning steps step 2), §9.2 (c) |
| A4-3 | MIN | §13.2.6.1 / §13.4 rows lack step numbers | **fixed**: step 17.2; step 4 (webref) |
| A4-4 | MIN | §3 row 1's Branch skips step 2 and step 5.1 | **fixed**: both listed; 5.1 marked not modelled |
| A4-5 | MIN | outerHTML = §8.5.5, setHTMLUnsafe = §8.5.2 | **fixed**: separate §3 rows; §4.6 module-docs row |
| A4-6 | MIN | §5.0's parser attribute merging uncited | **fixed**: §13.2.6.4.7 steps 2.2 / 3.2 in §5.0 and a §3 row |
| A4-7 | MIN | the §4.6 table omits rewritten docstring sites | **fixed**: rows for `handle_insert`, `handle_attribute_change`, module docs, the setters, the document marker, G6 |
| A2-IMP-1 | IMP | a shadow-root-direct element is processed at the next bind (third timing); with two or more attributes an IDL `fn` is overwritten | **fixed, by a different means than the brief's**: D-5 (first bind only) and D-10 (light Insert walk); "mark at placement" has no hook (P14); cell U9 |
| A2-IMP-2 | IMP | slice a's Insert arm writes raw values into inert containers; K-5's slice-a wording contradicts the data flow; no pin | **fixed**: K-5 rewritten; W-K5 in §1.5; window pins C14, C15; H22 |
| A2-MIN-1 | MIN | the shadow-inclusive walker's depth cap narrows HEAD's walk | **fixed**: the Insert arm keeps HEAD's uncapped walk (D-10); the cap now applies to the bind pass only, which HEAD lacks (§1.5 depth cap) |
| A2-MIN-2 | MIN | active vs inert is distinguished by `EcsDom` singletons, which breaks under an agent-scoped World | **rejected for this umbrella (recorded, §4.5)**: no multi-document World exists at `fd6955e8`; the world-model program owns the singleton's replacement, and Q2 checks today's assumption |
| A5-IMP-1 | IMP | §8.4 misses the IBP-sandbox umbrella's `:38` paragraph | **fixed**: §8.4 item 10 (verified at `0f99538d` line 38) |
| A5-IMP-2 | IMP | the C1-not-landed branch vs x5 / §10.3 / K-5 (= A3-IMP-4) | **fixed**: as A3-IMP-4 |
| A5-MIN-1 | MIN | the guard slot's own vs pre-existing classification and registration timing contradict §1.5 / §8; no re-eval or confidence | **fixed**: §8.2 (pre-existing, with reasons; registered at acceptance) |
| A5-MIN-2 | MIN | facet (4) becomes production-reachable via slice a; no record | **fixed**: §8.1, §8.3 (trigger cited), K5, E-G1 (i) |
| A5-MIN-3 | MIN | G1 exposes initial-HTML handlers to the missing scope chain | **fixed**: K6, E-G1 (ii), §8.1 `#11-event-handler-attribute-edge-cases` (`onchange="form.submit()"`); fixing it is out of scope |
| A5-x1 | (FP-known) | G7's stale "pending / scout running" text | **fixed** (P16): rev-3 paragraph, §4.4 R10, §7 G7, §11 O3 / O7 / Q1, §5.0 (c) |
| A5-x2 | (FP-known) | §8.1 lacks the two registered slots; the ledger's `lifecycle.rs:516` | **fixed**: §8.1 cites both; `:516` is under `#[cfg(test)]` (`:458`), §5.0 |

Counts: IMP 9 fixed, 0 rejected. MIN 18: 17 fixed, 1 rejected with reason (A2-MIN-2). The two FP-known items (outside
the 27) are fixed.

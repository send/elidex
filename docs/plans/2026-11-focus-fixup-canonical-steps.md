# `focus-fixup-canonical-steps` — the render-time focus fixup runs ungated, through the canonical focus update steps (per-PR plan-memo)

Slot **`#11-focus-fixup-gated-on-session-records`** (owner Layout, registered PM 245). Scope set by **PM decision 248**
(items 1–6, quoted by number below as "248 (n)"); the one-frame `:focus` lag question is **PM 242 (1)**. Branch
`layout-focus-fixup-ungated`, worktree `wt-focus-fixup`. Rev 1, 2026-11-03, draft for `/elidex-plan-review`.

**Measured defect.** `crate::re_render` (`crates/shell/elidex-shell/src/lib.rs:559`) ran the "update the rendering" step-17
focus fixup `reconcile_focus` (`crates/dom/elidex-dom-api/src/focus/sot.rs:178`) only when `session.flush` returned
records. VM-native mutations write the `EcsDom` directly and produce none, so `d.hidden = true` on the focused element's
ancestor left it focused (probe `f740a890`, now `crates/shell/elidex-shell/src/content_focus_fixup_tests.rs`). The branch
removed the gate (`0e0d7c93`); `/code-review` then showed that the ungated call is a **silent** reset (no `change` /
`blur` / `focusout`, the holder's change-on-blur snapshot never consumed), which for VM-native mutations is a regression
against `origin/main` — there the holder stayed focused and a later user blur still fired `change`.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

All `file:line` below are at branch HEAD `68c17ccf` (= `origin/main` `d7d34035` + `2163f113` probe, `0e0d7c93` gate
removal, `68c17ccf` /simplify). Two bases are used for red-first runs (§6): **base A** = `d7d34035` (gated fixup) and
**base B** = `68c17ccf` (ungated silent fixup). Re-run: `git -C <wt> log --oneline d7d34035..68c17ccf` → 3 commits.

### §0.2 Premises re-measured

| ID | Premise | Measurement (re-runnable) |
|---|---|---|
| P1 | `reconcile_focus` has exactly one non-test caller, `lib.rs:635` | `git grep -n 'reconcile_focus(' -- crates` → `lib.rs:635` + `focus/tests.rs` lines 72/731/740/751/758/767/780 (7 test sites, 3 test fns), verified 2026-11-03 |
| P2 | The iframe paths do not call `reconcile_focus`; they reach the fixup only through `crate::re_render` | `content/iframe/render.rs:38` (`re_render_all_iframes`, gated on `needs_render`), `content/iframe/thread.rs:107` (`iframe_thread_main`, per frame) |
| P3 | `reconcile_focused_iframe` is a different function (the `focused_iframe` side-field reconcile), defined in `content/event_handlers.rs:774`, called from `content/mod.rs:351` (pass 1) and `:423` (pass 2) | `git grep -n 'reconcile_focused_iframe' -- crates/shell` |
| P4 | Every other `crate::re_render` caller has no `ContentState`: `app/events.rs:83`, `:183`; `app/inline.rs:102`, `:132`, `:215`; `app/navigation.rs:94`, `:233`, `:466`; `content/iframe/thread.rs:107`; `app_test_support.rs:62` | `git grep -n 'crate::re_render(' -- crates/shell` (10 call sites besides `content/mod.rs:357`, verified 2026-11-03) |
| P5 | The canonical §6.6.4 transition exists: `focusing_steps` (`focus/update_steps.rs:106`), `unfocusing_steps` (`:165`), private `focus_update_steps` (`:175`), sink trait `FocusEventSink`; the shell sink `ShellFocusSink` (`content/focus.rs:108`, impl `:112-155`) dispatches `blur`+`focusout` / `focus`+`focusin` and consumes the snapshot in `dispatch_change_on_blur` (`:177`) | read at HEAD |
| P6 | `ancestor_cache.invalidate_all()` is gated on `mutation_records` (`lib.rs:599-601`); its only readers are the radio paths in `content/event_handlers.rs` (`:139`, `:609`, `:654`, `:660`) through `elidex_form::radio` (`radio.rs:77`, `:132`, `:204`) | `git grep -n 'ancestor_cache\|get_form_owner' -- crates` |
| P7 | The version API: `EcsDom::rev_version` (`crates/core/elidex-ecs/src/dom/mod.rs:656`) propagates to the document root; `EcsDom::inclusive_descendants_version` (`:701`) reads it; the read-time-validation precedent is `LiveCollection`'s `refresh_if_stale` (`crates/dom/elidex-dom-api/src/live_collection/mod.rs:231-235`, `cached_version: Option<u64>` `:112`) | read at HEAD |
| P8 | VM `focus()` gates on `is_focusable && is_in_document` (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:302-305`) and writes the bit directly (`set_focus_bit`), seeding the snapshot; VM `blur()` is bit-level (`:351`, `focus::blur`); `is_focusable` (`focus/predicate.rs:205`) does not enforce C2 (`:176-180`) nor fieldset-inherited disabled (`:182-185`) | read at HEAD |

### §0.3 Premise errors (in the brief)

- **E1** — "`reconcile_focused_iframe` in content/mod.rs": it is defined in `content/event_handlers.rs:774`; `content/mod.rs`
  only calls it (`:351`, `:423`). No consequence for the design.
- **E2** — "focusing steps for the viewport vs unfocusing steps — choose": the choice is forced by the spec. The
  unfocusing steps' step 6 ("If old focus target is not a focusable area, then return") returns for exactly the target
  step 17 handles, so they cannot implement the fixup. elidex's `unfocusing_steps` omits step 6 (its guard is step 5
  only, `update_steps.rs:165-171`), so calling it would "work" only through that omission — and would break silently when
  A2c adds step 6 for VM `blur()`. Pre-existing omission; recorded for A2c (§8).
- **E3** — The literal step-17 chain fires one event the brief does not list: `focus` at the **Window**. Derivation in
  §4.1 (S7); decided as a hand-off to A3 in D-9. PM-visible (§10).
- **E4** — Item 2 asks to "move it to a version-delta gate like the consumers [that] moved to
  `ContentState::last_render_dom_version`". The ideal is not a gate at `re_render`: the cache's readers run between
  frames (`toggle_radio_with_events`, `event_handlers.rs:650-663`, reads the cache after `set_focus`'s listeners ran), so
  any render-time invalidation leaves that window stale. D-8 validates at read time instead. The brief's question "where
  the baseline lives" is answered there (on the consumer).

### §0.4 Decisions (this PR)

- **D-1 The fixup is the focusing steps for the viewport** (248 (1)). Step mapping in §4.1. Not the unfocusing steps (E2).
- **D-2 One function moves focus to the viewport.** `focusing_steps_for_viewport` (NEW, private in `update_steps.rs`)
  implements focusing steps 1–8 for the viewport target in the single-element model (steps 1–4 vacuous, step 5 = "no
  holder → return", steps 6–8 = `focus_update_steps(sink, Some(old), None)`). Two callers: `focus_fixup_steps` (NEW, pub,
  the step-17 decision) and `unfocusing_steps` step 8 (which the spec also defines as "run the focusing steps for
  topDocument's viewport"; today it inlines `focus_update_steps(sink, Some(old_target), None)` at `update_steps.rs:170`).
  One-issue-one-way: "focus the viewport" has one implementation.
- **D-3 The predicate is `is_focusable`, unchanged** (248 (6)). It is the VM `focus()` gate's predicate (P8); a fixup that
  rejected more than the writer accepts would focus-then-reset every frame, now with events. C2 (`is_focusable_area`) and
  the shell's fieldset overlay (the shell `is_focusable`, `content/focus.rs:204`) are excluded for that reason; each
  has its slot (§7).
- **D-4 `reconcile_focus` is deleted.** Its one non-test caller (P1) is replaced; its test uses move to
  `update_steps.rs`'s test module as `focus_fixup_steps` cells using the existing `TestSink` (C14): two whole fns and the
  fixup half of a third (§5.2). `raw_focus_holder`
  (`sot.rs:16`) loses its second reader and stays for `blur` (`sot.rs:146`) and `current_focus`. The iframe paths need no
  change (P2): each pipeline's `crate::re_render` runs its own fixup. `reconcile_focused_iframe` (P3) is unchanged in code;
  its two call-site comments are corrected (F5, §5.1).
- **D-5 Shell entry.** `content::focus::focus_fixup` (NEW, `pub(crate)`, `content/focus.rs`) constructs
  `ShellFocusSink { pipeline }` and calls `elidex_dom_api::focus::focus_fixup_steps` (NEW). `crate::re_render` calls it. The sink
  needs only `&mut PipelineResult`, so every caller in P4 gets the same behaviour; no `ContentState` dependency.
- **D-6 Position** (248 (4)). **This PR does not move the position; the move goes with the being-rendered PR.** Inside
  the pre-style block the call moves up by one statement, from after `prune_dead_animation_entities` (`lib.rs:606`) to
  directly after the session settle (flush + CE loop, `lib.rs:576-596`), so entities a fixup listener removes are pruned
  this frame (C7 covers the listener-mutation side). Relative order to style resolution, layout and display-list build is
  unchanged.
- **D-7 No second settle after the fixup.** The fixup's listeners are VM callbacks; VM-native writes reach the `EcsDom`
  directly and settle MutationObserver / CE at the VM's own checkpoints (`lib.rs:566-571` comment). A session-pending
  write from a listener, if any path produces one (Q1), is applied at the next frame's flush — the same as for a
  listener run by any UA event dispatched outside `re_render`. Adding an unconditional re-flush would be code no cell can
  turn red.
- **D-8 The form-owner cache validates itself at read time** (248 (2), E4). `AncestorCache` (`crates/dom/elidex-form/src/ancestor_cache.rs:14`)
  gains `cached_version: Option<u64>` (NEW field) and `get_form_owner` (`:30`) clears the map when
  `dom.inclusive_descendants_version(root)` differs, `root` = `dom.document_root()` (`elidex-ecs` `dom/mod.rs:422`) —
  `LiveCollection`'s pattern (P7), `Option` for the same wraparound reason. `lib.rs:598-601` is deleted; `invalidate`
  (`:49`, no caller: `git grep -n '\.invalidate(' -- crates/dom/elidex-form crates/shell` → none outside the file) is
  deleted; `invalidate_all` becomes the private clear. Where the baseline lives: **on the consumer**, not a `ContentState`
  field, not a `PipelineResult` field and not a document component — (i) it is the cache's validity stamp, read by no
  one else, so the side-store rule's subject (per-entity state) is not this value; a document component would carry one
  consumer's private bookkeeping on a shared entity, and a `PipelineResult` field is the same side-store one level out;
  (ii) it works for every pipeline (P4) with no plumbing; (iii) it closes the between-frames window (E4). Reading
  `dom.document_root()` adds no coupling `rev_version` does not already have (it reads the same root, `dom/mod.rs:664`);
  B1's agent-scoped World re-homes both together.
- **D-9 The Window `focus` event of the literal chain is not fired** (E3). The single-element model (A2a,
  `update_steps.rs:14-23`) fires events only at elements; the chain and its Document/Window entries are PR-A3's. This PR
  fires `change` (if edited) and `blur` at the holder, related target null — the literal spec's events at the element —
  plus the sink's `focusout`, as on every elidex blur.
- **D-10 Commits.** (1) dom-api: `focusing_steps_for_viewport`, `focus_fixup_steps`, `unfocusing_steps` step 8 re-route,
  `reconcile_focus` deletion, relocated cells; (2) shell: `focus_fixup`, `lib.rs` call, comments F4/F5, shell cells;
  (3) form: `AncestorCache` read-time validation, `lib.rs:598-601` deletion, its cells. Each commit builds and passes its
  crate's tests.

## §1. Shape and verdict

Edge-dense by CLAUDE.md's rule (§2 has ten coupled invariants spanning event dispatch, script re-entry, iframe passes,
form state and render position), so plan-review is mandatory. **Single PR**: the algorithm is the approved A2a seam
(`#357`), reused; the parts that would widen the PR are already homed elsewhere — the position move and computed-style
predicate (being-rendered / CV-focus), the focus chain and Window events (A3), C2 (`#11-shadow-focus-delegation`), the
fieldset predicate (`#11-focusable-area-fieldset-inherited-disabled`), VM event dispatch (A2c). Breadth K = 1 spec, M = 12
rows (§3) → preflight "single PR scope". The form-owner cache (D-8) is a different concept found by 248 (2)'s sweep and
kept in this PR by that decision; it is a separate commit with its own cells.

## §2. Coupled invariants

- **I1 Ungated trigger** — every `crate::re_render` (all callers, P4) runs `focus_fixup_steps` once, regardless of
  `session.flush`'s output.
- **I2 Canonical transition** — the fixup moves focus only through `focus_update_steps(old, None)` via a sink: step 2.1
  `change` while `old` is still designated, then the bit cleared, then `blur` + `focusout` at `old`, related null.
- **I3 Snapshot consumed once** — the holder's `FocusValueSnapshot` is taken by `commit_change_on_blur`
  (`take_focus_snapshot`, `content/focus.rs:178`) exactly once per transition.
- **I4 Script re-entry inside `re_render`** — `change` / `blur` / `focusout` listeners run after the settle and before
  style; their VM-native writes are visible to this frame's style, layout and display list; MutationObserver / CE for
  them settle at the VM checkpoint after each callback (Q2); a reentrant `focus()` wins (`update_steps.rs:204`, `:223`).
- **I5 Observers see the pre-fixup holder** — MutationObserver callbacks for mutations made before the frame (VM-native:
  delivered at the script's own microtask checkpoint; external: `deliver_records_and_drain`, `lib.rs:580`) run before
  the fixup and read `activeElement` = the holder. Kept.
- **I6 Per-document fixup** — each pipeline's fixup reads and writes only its own `EcsDom` and dispatches into its own
  runtime: the parent in `ContentState::re_render` (`content/mod.rs:357`), in-process children in
  `re_render_all_iframes` (`:355`, only when `needs_render`), OOP children per frame (`iframe/thread.rs:107`). Parent pass 2
  (`:423`) runs after the parent fixup and blurs the child of a no-longer-focused `<iframe>`.
- **I7 Form-owner answers are current at read time** — `get_form_owner` never returns an owner computed before the
  document tree's last mutation (D-8).
- **I8 One predicate for writer and fixup** — the fixup resets exactly what the VM `focus()` gate would refuse
  (`is_focusable`, D-3); the shell UA writer (`focus_target`, `content/focus.rs:52`) accepts a subset of that.
- **I9 Position** — before style; ResizeObserver delivery (`content/mod.rs:433`) after it (not spec order — obligation,
  §8).
- **I10 Blurred control's UI state** — the cleared `FOCUS` bit is read by this frame's display-list build, so no caret
  is painted for the blurred control; IME composition state (`FormControlState.composition_text`) is left as every UA
  blur leaves it (Q3).

Pairs:
- **I1 × I2** — ungated + event-firing: an idle frame must fire nothing. `focus_fixup_steps` returns before the
  transition when there is no holder or the holder is focusable (C4, C14 (b)/(c)).
- **I1 × I8** — running every frame turns any writer/fixup predicate mismatch into a per-frame focus-then-reset loop with
  events; I8 removes it for the VM writer (C18 pins the C2 case).
- **I2 × I3** — step 2.1 *is* the consume; base B's silent reset skipped it, leaving the snapshot for a later unrelated
  blur (C6).
- **I2 × I4** — the transition's listener calls are the re-entry points; reentrant-wins is checked after step 2.1 and
  after step 2.4, so a listener's `other.focus()` stands and the outer transition designates nothing (new = viewport) (C9).
- **I2 × I5** — the frame's earlier MutationObserver deliveries precede the fixup (I5); records from the fixup's own
  listeners are delivered at their own checkpoint, after the fixup, and see the post-fixup `activeElement` (C8, C10).
- **I2 × I6** — a child pipeline's sink dispatches into the child runtime; the parent's fixup blurs the `<iframe>` element
  in the parent; the child is blurred by pass 2 through the child's own canonical `unfocusing_steps` (`blur_iframe_focus`
  → `blur_current`, `event_handlers.rs:755-762`) (C11).
- **I2 × I8** — a listener's reentrant `focus()` goes through VM `focus()` (bit-only, no events): the reentrant target wins
  without `focus` / `focusin`, and if it fires from `change`, `old` gets no `blur` (`update_steps.rs:184-203`). Pre-existing
  (A2c), not widened: base A/B fire nothing at all.
- **I2 × I10** — the bit is cleared at step 2.2 (before `blur`), which is before `build_display_list_with_scroll`
  (`lib.rs:705`), so the caret disappears this frame, as at base B.
- **I3 × I4** — a `change` listener writing `value` cannot cause a second `change`: the snapshot is already taken.
- **I4 × I6** — listener mutations in the parent land before `ContentState`'s version snapshot (`content/mod.rs:367-372`),
  so `focusable_cache`, the iframe rescan (`:441`) and `event_loop.rs:424-432`'s render signal see them this frame.
- **I4 × I7** — a listener that moves a radio or writes `form` bumps the root version; the next `get_form_owner` sees it,
  whether or not a `re_render` intervened (C12, C13).
- **I4 × I9** — listener writes are styled this frame only because the fixup is before style; the being-rendered move must
  either re-run style for them or accept a one-frame lag (obligation O1, §8).
- **I4 × FU** — a listener's script-time style read (`getComputedStyle`, once FU exists) reaches FU's forced update, which
  must not include step 17 (CV umbrella FU-5, `docs/plans/2026-10-content-visibility.md:239-241`); no recursion.
- **I5 × I9** — spec step 16 (ResizeObserver) precedes step 17, so RO callbacks should see the pre-fixup holder; at HEAD
  and after this PR they see the post-fixup one (same as base A/B). Obligation O1.
- **I6 × I1** — an in-process child runs its fixup only on frames where `needs_render` is set (`iframe/render.rs:37`); a
  child mutation that does not set it waits for the next flagged frame (adjacent class, §4.4 row S9; Q4).
- **I6 × I8** — an `<iframe>` is focusable by its tab-index default; `hidden` on it (or an ancestor) makes it
  non-focusable by the same predicate (C11).
- **I7 × I1** — after D-8 the cache's correctness no longer depends on `re_render` running at all.
- **I3 × I6**, **I5 × I6**, **I7 × I2**, **I7 × I3**, **I8 × I9**, **I10 × I6** — none beyond the shared pipeline.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 | focused area of doc not a focusable area → focusing steps for doc's viewport; focusable or no holder → nothing | `lib.rs` `re_render` → `content::focus::focus_fixup` (NEW) → `focus_fixup_steps` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 | "set the navigation API's focus changed during ongoing navigation to false" | unmodelled (no Navigation API; `update_steps.rs:227-230` records the same for step 4.1.1) | ✓ | no |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 notes | "will usually fire blur events, and possibly change events"; removal = the synchronous fixup, no events | events: I2; removal: `EcsDom::fire_after_remove` unchanged | ✓ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* steps 16 → 17 | ResizeObserver broadcast before the fixup | not met: RO delivered after (`content/mod.rs:433`); obligation O1 | ✗ | no |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 2 | docs order: a container document before its children, each step over all docs | not met: elidex runs a whole `re_render` per pipeline, children first (`content/mod.rs:355-357`); unchanged, §7 | ✗ | no |
| WHATWG HTML §6.6.4 Processing model | focusing steps 1–5, target = viewport | 1 viewport is a focusable area; 2 n/a; 3 not a navigable container; 4 DOM anchor = Document, `inert` unmodelled; 5 viewport already focused (no holder) → return | `focusing_steps_for_viewport` (NEW) | ✓ | no |
| WHATWG HTML §6.6.4 Processing model | focusing steps 6–8; focus update steps step 1 | old chain [holder] (holder not focusable → chain stops), new chain [viewport, Document]; no common tail; single-element model → `focus_update_steps(old, None)` | `update_steps.rs:175` (unchanged) | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1–2.4 | edited text control → `change` (snapshot consume); blur target = holder; related blur target null (last new entry is a Document) | `ShellFocusSink::commit_change_on_blur` / `fire_focus_event` (`content/focus.rs:121`, `:136`) | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 4 | Document entry: not a focusable area, focus event target = its Window → `focus` at Window (not fired, D-9); viewport entry: designated (bit cleared), no event; reentrant-wins gate | `update_steps.rs:220-245`; the Window event → A3 | ✗ | yes |
| WHATWG HTML §6.6.4 Processing model | unfocusing steps steps 5, 6, 8 | 6 "not a focusable area → return" (why D-1 rejects them; omitted in elidex, E2); 8 focusing steps for topDocument's viewport | `unfocusing_steps` step 8 → `focusing_steps_for_viewport` (NEW) | ✗ | yes |
| WHATWG HTML §6.6.2 Data model | focusable area criteria; focus chain | predicate = `is_focusable`: C1, C3 (direct `disabled` only), C5 (attribute-reachable); C2, fieldset-inherited C3, CSS C5 not enforced | `focus/predicate.rs:205` (unchanged) | ✗ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | `change` when the control loses focus | change applies except buttons and Hidden-state inputs; the sink does not check "applies" (a text→hidden `type` flip after an edit fires `change`) | `dispatch_change_on_blur` (`content/focus.rs:177`, unchanged; risk R3) | ✗ | yes |

Numbers and titles: `.claude/tools/webref coverage-map html 8.1.7.3 html 6.6.4 html 6.6.2 html 4.10.5.5` (§8.1.7.3
Processing model `#event-loop-processing-model`; §6.6.4 Processing model `#focus-processing-model`; §6.6.2 Data model
`#data-model`; §4.10.5.5 Common event behaviors `#common-input-element-events`); `webref dfn html 'update the rendering'`
→ §8.1.7.3; `'focusing steps'`, `'unfocusing steps'`, `'focus update steps'` → §6.6.4; `'focusable area'`, `'focus chain'`
→ §6.6.2. Step texts: `.claude/tools/webref body html update-the-rendering`, `focusing-steps`, `focus-update-steps`,
`unfocusing-steps`, `focus-chain`, `common-input-element-events` (2026-11-03). UI Events' focus-event order is the sink's,
unchanged, and not cited here. Breadth: K = 1, M = 12.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: any script write that changes the holder's focusability (`hidden` on
it or an ancestor, `disabled`, `type`, `tabindex`, `contenteditable`, `href` removal) — now fires author listeners
(`change`, `blur`, `focusout`) from inside `re_render`; those listeners can mutate the DOM, move focus, and register
observers (I4). User input: the edited value compared by `commit_change_on_blur`. Radio `name` / `form` attributes and
tree moves reach `get_form_owner` (D-8). Every listener runs on the renderer main thread inside the existing `with_bound`
bracket discipline (`PipelineResult::dispatch_event`, `lib.rs:293`); `crate::re_render` is never entered inside a bracket
(its `session.flush` would alias, `lib.rs:560-563`), so the dispatch nests nothing. Adjacent pre-existing surface:
the radio group scan includes radios outside the document tree (`radio.rs:118-123` has no same-tree filter) — not widened
(§7).

## §4. Mechanism

### §4.1 The fixup — HEAD → this PR

`focus_fixup_steps` (NEW, `update_steps.rs`):

```rust
pub fn focus_fixup_steps(sink: &mut dyn FocusEventSink) {
    let doc = sink.document();
    let Some(holder) = current_focus(sink.dom(), doc) else { return };
    if is_focusable(sink.dom(), holder) { return; }
    focusing_steps_for_viewport(sink);
}
```

`current_focus` = `raw_focus_holder` + `is_in_document`, i.e. HEAD's `reconcile_focus` test (`sot.rs:184-188`) exactly.

Step mapping (S1–S9; spec text from §3's commands):

| # | Spec | elidex |
|---|---|---|
| S1 | step 17: focused area not a focusable area | `focus_fixup_steps`'s two early returns |
| S2 | focusing step 1 (viewport is a focusable area) | vacuous |
| S3 | focusing steps 2–4 (null target, navigable container, inert) | vacuous / `inert` unmodelled (as `update_steps.rs:126-128`) |
| S4 | focusing step 5 (already the focused area) | `focusing_steps_for_viewport`: `current_focus` None → return |
| S5 | focusing steps 6–8; focus update step 1 | `focus_update_steps(sink, Some(old), None)` |
| S6 | focus update step 2 (change, blur at holder, related null) | `update_steps.rs:177-217`; `new = None` → related null |
| S7 | focus update step 4 for the Document entry (`focus` at Window) | not fired (D-9) |
| S8 | focus update step 4 for the viewport entry (designate) | the bit cleared at step 2.2 (A2a browser-parity ordering, `update_steps.rs:208-215`) |
| S9 | step 17's navigation-API flag | unmodelled |

`unfocusing_steps` (`update_steps.rs:165-171`): its body after the step-5 guard becomes `focusing_steps_for_viewport(sink)`;
behaviour unchanged (the guard already established `current_focus == Some(old_target)`), pinned by the existing unfocusing
cells in `update_steps.rs`'s test module.

`crate::re_render` after the settle block: `crate::content::focus::focus_fixup(result);` then the prune, then Phase 1.
The comment at `lib.rs:608-634` is rewritten: canonical steps, events listed, the step-17 note, the position obligation
(O1) and I5.

### §4.2 The form-owner cache — HEAD → this PR

D-8. `get_form_owner(&mut self, entity, dom)` keeps its signature (no caller changes: `radio.rs:77`, `:132`, `:204`).
First statement: compare `dom.document_root().map(|r| dom.inclusive_descendants_version(r))` with `cached_version`;
on mismatch clear and store. A world with no document root (`elidex-form` unit tests on detached trees) reads `None`
and never clears — the same as HEAD's never-invalidated test caches. Doc comment `:9-12` ("Invalidated when the DOM tree
changes (via `MutationObserver` or explicit call)") is rewritten.

### §4.3 ECS-native and Layering checks

- The step-17 decision and the viewport transition are engine-independent (`elidex-dom-api`); the shell contributes only
  the sink (event dispatch + `elidex-form` snapshot), as A2a set up. `vm/host/` is untouched.
- No new side-store: the FOCUS bit stays the SoT; the cache stamp is consumer validity (D-8 (i)).
- Version counter = the existing `inclusive_descendants_version`; no new counter.
- Observation, not changed here: `AncestorCache.form_owner` (`HashMap<Entity, Option<Entity>>` on `PipelineResult`,
  `lib.rs:255`) is itself the CLAUDE.md side-store class (per-entity, `Send + Sync`, not an identity handle) — its
  ECS-native form is a per-control form-owner component maintained by HTML's form-owner reset. PM item (§10 M3).

### §4.4 Concept sweep — "a consumer that depends on DOM changes but is gated on session records"

Scope: `crate::re_render` (`lib.rs:559-711`), the functions it calls in the same pass, and `ContentState::re_render`
(`content/mod.rs:313-460`). Method: every conditional in those bodies read for its gate's subject; plus
`git grep -n 'mutation_records\|records.is_empty()' -- crates/shell/elidex-shell/src`.

| # | Site | Gate | Disposition |
|---|---|---|---|
| S1 | `lib.rs:579-596` `deliver_records_and_drain` + CE stabilization loop | flush records | **correct** — the records are the subject (external-record MO/CE delivery); VM-native records deliver in the VM |
| S2 | `lib.rs:598-601` `ancestor_cache.invalidate_all()` | flush records | **hit (F1)** — fixed by D-8 (gate deleted, read-time validation) |
| S3 | `lib.rs:635` focus fixup | (gate removed by `0e0d7c93`) | **hit (the slot)** — D-1/D-5 |
| S4 | `lib.rs:606` `prune_dead_animation_entities` | none | correct (unconditional, comment `:603-605`) |
| S5 | `lib.rs:655-665` `collect_document_stylesheets` | per-owner version | correct (version-compared, DOM-as-truth) |
| S6 | `lib.rs:677-710` keyframes, `resolve_with_mode`, transitions, animations, layout, display list | none | correct (unconditional) |
| S7 | `content/mod.rs:367-372` version snapshot → `focusable_cache` (`:406-408`), `rescan_iframes_by_diff` (`:440-444`) | version delta | correct (already moved, the `re_render` doc's claim holds) |
| S8 | `content/mod.rs:351`, `:423` `reconcile_focused_iframe` | none (reads the FOCUS bit) | correct; comments stale (F5) |
| S9 | `content/mod.rs:355` → `iframe/render.rs:37` in-process child `re_render` | `needs_render` flag | **adjacent class, not this concept** (a writer-set dirty flag, not session records): a write into a same-origin child's DOM by a parent script, if possible, would not set it (Q4); out of scope, slot candidate (§10 M4) |
| S10 | `content/mod.rs:447` `check_lazy_iframes`, `:433` `deliver_layout_observations` | none | correct |
| S11 | `lib.rs:545-558` `re_render` doc ("ancestor-cache invalidation" listed as an internal record consumer) | — | doc updated with S2 |
| S12 | `pipeline.rs:147-170` build-time record delivery | flush records | correct (delivery; outside `re_render`, listed for completeness) |

Hits: S2, S3 (fixed here); adjacent S9 (§10). No other record-gated DOM consumer.

## §5. Populations and named risks

### §5.1 Docstring and comment edits

- **F4** `sot.rs:152-177` (moved with the deleted function's doc into `focus_fixup_steps`'s doc): "`disabled` lands"
  overclaims — fieldset-inherited disabled is not in the dom-api predicate (`predicate.rs:182-185`); the new doc says
  "a direct `disabled`", names slot `#11-focusable-area-fieldset-inherited-disabled`, and drops "Silent".
- `sot.rs:1-5` module doc and `mod.rs:35-38` module doc: "the asynchronous focusability fixup (`reconcile_focus`)" →
  `focus_fixup_steps` in `update_steps`; `mod.rs:26-29` ("Event dispatch … stays with the caller; these helpers only
  reconcile the `FOCUS` bit") narrowed to `sot`.
- `sot.rs:24-51` (`current_focus` doc) and `sot.rs:130-145` (`blur` doc) name `reconcile_focus` → re-pointed.
- **F5** `content/mod.rs:310-312` (`ContentState::re_render` doc: "Invalidates `focusable_cache` when mutations affect …
  (childList, tabindex, disabled, etc.)" → "on any document-tree version change"); `:338-350` (pass 1: "the iframe element
  made non-focusable by a *buffered* mutation … can only be reconciled after `crate::re_render` flushes + GCs the bit" →
  any mutation, VM-native included, reconciled by the ungated fixup inside `crate::re_render`); `:410-422` (pass 2: "that
  buffered attribute is only flushed + reconciled … INSIDE `crate::re_render`" → the fixup's canonical transition, which
  also fired `blur` at the `<iframe>` element).
- `lib.rs:545-558` (`re_render` doc, S11) and `lib.rs:608-634` (§4.1).
- `content_focus_fixup_tests.rs:1-3` module doc: `reconcile_focus` → the canonical fixup.
- `ancestor_cache.rs:9-12`, `:43` (§4.2).

### §5.2 Touch list with line counts (`wc -l` at `68c17ccf`)

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/shell/elidex-shell/src/lib.rs` | 731 | call moved/replaced, gate deleted, comments | < 1000 |
| `crates/shell/elidex-shell/src/content/mod.rs` | 865 | comments only (F5) | < 1000 |
| `crates/shell/elidex-shell/src/content/focus.rs` | 397 | `focus_fixup` (NEW, ~15 lines) | < 1000 |
| `crates/shell/elidex-shell/src/content_focus_fixup_tests.rs` | 97 | F6 + cells C5–C12, C18 (~+250) | < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 550 | two fns (~40), step-8 re-route, relocated + new cells (~+120) | ~710, < 1000 |
| `crates/dom/elidex-dom-api/src/focus/sot.rs` | 202 | `reconcile_focus` deleted, docs | shrinks |
| `crates/dom/elidex-dom-api/src/focus/mod.rs` | 57 | module doc | — |
| `crates/dom/elidex-dom-api/src/focus/tests.rs` | 999 | 3 fns relocated out (~−75); `current_focus_keeps_holder_until_async_fixup` (`:41`) keeps its first half, its fixup half moves | shrinks; **no growth allowed** (any net add crosses 1000 → standalone split first) |
| `crates/dom/elidex-form/src/ancestor_cache.rs` | 114 | field + check, `invalidate` deleted, cells | < 1000 |
| `crates/shell/elidex-shell/src/content/event_handlers.rs` | 965 | not touched (`reconcile_focused_iframe` unchanged; its doc is accurate) | — |

Re-run: `wc -l` on the paths above. `focus/predicate.rs` (329) and `elidex-form/src/radio.rs` (500) are read, not edited.

### §5.3 Named risks

- **R1 Script now runs inside `re_render`.** Listener cost and listener-driven DOM growth land in the frame. Bounded by
  one transition per pipeline per frame (no loop: step 17 runs once; a listener-created non-focusable holder waits for
  the next frame, as in the spec).
- **R2 Cache stamp read per lookup.** One `TreeRelation` read per `get_form_owner` (`read_rel`); the radio scan
  (`radio.rs:118-140`) calls it once per candidate — O(1) each, against HEAD's O(1) hit.
- **R3 "change event applies" unchecked** (§3 last row). The fixup now reaches the text→hidden flip case every frame;
  before, only a later UA blur did. Pre-existing in `dispatch_change_on_blur`; the A2 program's step 2.1 owns the
  condition (§8).
- **R4 F8 rejected — per-frame cost of the ungated fixup.** Per frame: `raw_focus_holder` iterates only archetypes
  carrying `ElementState` and stops at the first FOCUS holder (`sot.rs:16-22`, `find`); then one `is_in_document` walk and
  one `is_focusable` evaluation (its hidden-subtree walk) for that one holder, both capped at `MAX_ANCESTOR_DEPTH`
  (`sot.rs:81`); the full-holder sweep in `set_focus_bit` runs only on a reset. And `re_render` runs only when a render is
  needed (`event_loop.rs:252-447`'s `needs_render`; in-process children on `needs_render`). Base A ran the same work on
  every frame with records. No figure is given (machine-dependent); the argument is the bound above.

## §6. Test and coverage plan

Red-first: each NEW cell is first run on base A (`d7d34035`) and base B (`68c17ccf`) with the fix absent; the expected
result is in the "A / B" column ("red" = fails as the defect predicts; "green" = pin or control). The PR body records the
observed result per cell and base. Shell cells live in `content_focus_fixup_tests.rs` (fixture helpers `focused_on_b`,
`by_id`, `focused` exist); iframe cells reuse the `<iframe srcdoc>` in-process fixture of
`content/iframe/lifecycle.rs:587-589`. Events are observed by listeners writing to a global array read back with
`eval_script`.

| Cell | Setup → assertion | A / B |
|---|---|---|
| C1 (F6) `vm_native_hidden_ancestor_runs_focus_fixup` | existing; assertion → `current_focus == None` | red / green |
| C2 (F6) `vm_native_disabled_focused_button_runs_focus_fixup` | existing; → `== None` | red / green |
| C3 (F6) `control_session_buffered_hidden_runs_focus_fixup` | existing control; → `== None` | green / green |
| C4 `focusable_focus_survives_idle_rendering_updates` | existing negative; add: no `blur` recorded | green / green |
| C5 (NEW) `focus_fixup_fires_blur_then_focusout_at_holder` | `b` has `blur`/`focusout` listeners recording type, `relatedTarget`, `document.activeElement`; VM `d.hidden = true`; frame → `["blur","focusout"]`, related null, `activeElement === body` in both | red / red |
| C6 (NEW) `focus_fixup_fires_change_for_edited_text_control` | `<input id=b>` focused by UA `set_focus` (seeds snapshot); value edited through `elidex_form`'s edit path; VM `b.disabled = true`; frame → `["change","blur","focusout"]`, `change` listener sees `activeElement === b`, `take_focus_snapshot(b)` → None | red / red |
| C7 (NEW) `focus_fixup_listener_mutation_is_rendered_this_frame` | `blur` listener appends `<p id=late>x</p>`; frame → `#late` has a `LayoutBox` after the same `re_render` | red / red |
| C8 (NEW) `focus_fixup_listener_mutation_is_observed` | MutationObserver on `body` (childList); `blur` listener appends; after the frame the callback has run once and saw `activeElement === body` | red / red (Q2) |
| C9 (NEW) `focus_fixup_reentrant_focus_wins` | `blur` listener calls `c.focus()` (`c` focusable); frame → `current_focus == c`, `b` saw exactly one `blur`, the outer transition designated nothing after | red / red |
| C10 (NEW pin) `mutation_observer_sees_pre_fixup_active_element` | MutationObserver on `d` (attributes) records `activeElement.id`; VM `d.hidden = true`; frame → `"b"`, and focus None after | green / green |
| C11 (NEW, F7) `vm_native_hidden_focused_iframe_blurs_parent_and_child` | in-process `<iframe srcdoc="<input id=i>">`; focus into it (parent bit on the iframe, `focused_iframe`, child bit on `i`); parent VM `iframe.hidden = true`; `ContentState::re_render` → parent `current_focus == None`, `focused_iframe == None`, child `current_focus == None`, parent `blur` fired at the iframe element | red / green-except-`blur` (B resets silently: the `blur` assertion is red) |
| C12 (NEW, F1 shell) `radio_group_follows_vm_native_form_move` | forms `f1` {`ra`}, `f2` {`rb` checked}, same `name`; UA click `ra` (warms the cache: `ra` → `f1`); VM `f2.append(ra); ra.checked = false`; one frame; UA click `ra` → `ra` checked and `rb` unchecked (one checked radio in `f2`'s group) | red / red |
| C13 (NEW, F1 unit, `ancestor_cache.rs`) `get_form_owner_follows_tree_move_without_explicit_invalidate` | document root; `ra` in `f1`, cached; `append_child(f2, ra)`; `get_form_owner(ra)` → `f2` (no `invalidate_all` call) | red / red |
| C14 (NEW + relocated, `update_steps.rs`) `focus_fixup_steps_*` with `TestSink` | (a) non-focusable holder (HEAD's five arms: `hidden` on self, `hidden` on an ancestor, `disabled`, `type=hidden`, `href` removed — `tests.rs:719-770`) → events `[(h, Blur, None)]`, `changes == [h]`, focus None; (b) focusable holder → no events (`tests.rs:772-785`); (c) no holder → none; (d) holder in a `cloneNode` document → none, bit kept; (e) `reentrant_on_blur` → reentrant target holds focus | relocated: green / green on `reconcile_focus`; new events: n/a (new API) |
| C15 existing `unfocusing_steps` cells (`update_steps.rs` test module) | unchanged; must pass after the step-8 re-route | green |
| C16 OOP iframe | no in-test OOP harness found (Q5); covered structurally by I6 (`iframe/thread.rs:107` calls the same `crate::re_render`) | — |
| C17 caret (I10) | in C6, the frame's display list carries no caret item for `b` — only if a caret display-item probe exists (Q3); else structural (position before `build_display_list_with_scroll`, unchanged) | green / green |
| C18 (NEW pin, 248 (6)) `delegates_focus_host_focused_by_vm_survives_fixup` | `<div id=h tabindex=0>` with an open `delegatesFocus` shadow root and a focusable delegate; VM `h.focus()`; two frames → `current_focus == h`, no `blur` | green / green |

Mutation guards (run on the final head; each must turn the named cells red):

| Guard | Mutation | Red cells |
|---|---|---|
| MG1 | wrap the `focus_fixup` call in `if !mutation_records.is_empty()` (needs the variable back) | C1, C2, C11 |
| MG2 | in `focus_fixup_steps`, replace `focusing_steps_for_viewport(sink)` with `set_focus_bit(sink.dom(), None)` | C5, C6, C7, C8, C9, C14 (a)/(e) |
| MG3 | predicate `is_focusable` → `is_focusable_area` | C18 |
| MG4 | delete the `cached_version` comparison in `get_form_owner` | C12, C13 |
| MG5 | move the `focus_fixup` call after `layout_tree` (`lib.rs:703`) | C7 |
| MG6 | move the `focus_fixup` call before the first `session.flush` (`lib.rs:576`) | C3 |
| MG7 | in `unfocusing_steps`, replace the step-8 call with a bare `return` | C15 |

Coverage: engine-independent cells in `elidex-dom-api` (C14, C15) and `elidex-form` (C13); integration in
`elidex-shell` (rest). No WPT subset is claimed (the focus WPTs need the chain, A3).

## §7. Out of scope and slots

- Position after style + computed-style predicate + RO order → being-rendered / CV-focus (obligation O1, §8).
- Window `focus` (S7) and the focus chain → PR-A3 (no in-repo plan; the program record is the memory `project_canonical-focus-update-steps`).
- C2 in the predicate → `#11-shadow-focus-delegation` / A2 (c).
- Fieldset-inherited disabled → `#11-focusable-area-fieldset-inherited-disabled` (A3; coordinated with
  `docs/plans/2026-07-form-derived-state-reconciliation.md` `:4`).
- VM `focus()` / `blur()` event dispatch and unfocusing step 6 → A2c, `#11-vm-host-synthetic-dom-event-dispatch`.
- OOP iframe focus messages → `#11-oop-iframe-focus-lifecycle`.
- `user validity` at step 2.1.1 → `#11-focus-change-user-validity`.
- Per-pipeline document order (§3 row 5), radio same-tree filter (§3.1), "change applies" (R3), the form-owner side-store
  (§4.3), the in-process child `needs_render` class (S9) — §10 M2–M5.

## §8. Edges, hand-offs and records

- **O1 — obligation on the being-rendered PR** (`#11-focusable-area-being-rendered`, CV-focus; 248 (4)). This PR does not
  move the position; the move goes with the being-rendered PR. That PR moves the fixup after style/layout because (a) its
  predicate reads computed style, which exists only after Phase 2, and (b) ResizeObserver delivery
  (`content/mod.rs:433`, `deliver_layout_observations`) must see the pre-fixup holder — spec step 16 precedes 17 (I5 × I9).
  It also decides the one-frame `:focus` lag (PM 242 (1)): after the move the cleared bit is not in the frame's style
  unless the frame re-styles, and the fixup's listener writes (I4 × I9) face the same choice. Its plan cites this
  paragraph and C7 (which it will have to re-state).
- **CV umbrella** (`docs/plans/2026-10-content-visibility.md`, approved #541) names `reconcile_focus` at 12 sites (verified 2026-11-03
  via `git grep -c 'reconcile_focus' -- docs/plans/2026-10-content-visibility.md`, e.g. `:89`, `:241`, `:394`, `:463`, `:690`);
  after this PR the symbol is `focus_fixup_steps` and it fires events. Not edited here (approved text, HEAD-anchored
  coordinates); CV-focus's per-PR plan re-derives at its base. Its K8 ("render-time readers … read the last style pass,
  which they run after", `:360`) is O1's target state, not HEAD's.
- **FU** (`#11-forced-style-layout-flush-on-script-read`): FU-5's "not `reconcile_focus`" carries to `focus_fixup_steps`;
  the new fact is that the fixup runs script, whose style reads reach FU's point (I4 × FU).
- **Canonical-focus A2b / A2c / A3**: A2c adds unfocusing step 6 and VM event dispatch — `focus_fixup_steps` does not use
  `unfocusing_steps`, so step 6 cannot break it (E2); after A2c a reentrant VM `focus()` from a fixup listener fires its own
  events (I2 × I8). A3's chain adds S7 (Window `focus`) and may re-home `focusing_steps_for_viewport` under a target enum.
- **PR body records** (248 (6), R4): (i) VM `focus()` gates on `is_focusable && is_in_document`
  (`html_element_proto.rs:302`); `is_focusable` is C2-blind (`predicate.rs:176-180`) like the fixup, so a `delegatesFocus`
  host focused via VM `focus()` is not reset — consistent (C18; C2 = `#11-shadow-focus-delegation` / A2 (c)); (ii) F8's
  rejection with R4's reasoning; (iii) D-9 / E3; (iv) O1 verbatim; (v) red-first results per cell and base; (vi) the
  concept-sweep table §4.4.

## §9. Definition of done

1. `git grep -n 'reconcile_focus\b' -- crates` → only `reconcile_focused_iframe` hits (use
   `git grep -n -w 'reconcile_focus' -- crates` → 0).
2. `git grep -n 'mutation_records' -- crates/shell/elidex-shell/src/lib.rs` → only the settle block (no consumer gate).
3. `git grep -n 'invalidate_all\|\.invalidate(' -- crates/shell` → 0.
4. Cells C1–C15, C18 green on the final head; C5–C13 red on the bases as tabled (results in the PR body); MG1–MG7 each
   red on their cells, then reverted.
5. Comments F4, F5 and §5.1's sites updated; `focus/tests.rs` ≤ 999 lines.
6. `cargo fmt --all`; `cargo test -p elidex-dom-api -p elidex-form -p elidex-shell --all-features`; `mise run ci` green
   (`grep 'ERROR task failed'` on its log → none).
7. PR body carries §8's records; the slot `#11-focus-fixup-gated-on-session-records` reported closed to the PM, with O1
   appended to `#11-focusable-area-being-rendered` (PM transcribes).

## §10. Open items

**Questions for scout** (facts the design rests on; each names what changes if the answer differs):

- **Q1** Can any path reachable from a VM event listener call `SessionCore::record_mutation` (e.g. a `DomApiHandler` such
  as `SetInnerHtml`, `crates/dom/elidex-dom-api/src/element/markup.rs:56`, invoked by the VM)? VM `innerHTML` calls
  `apply_set_inner_html` directly (`vm/host/dom_inner_html.rs:22`). If yes: D-7 reverses — factor the settle block into
  one helper and call it again after the fixup.
- **Q2** Does `PipelineResult::dispatch_event` → `script_dispatch_event` run a microtask checkpoint after the listeners
  (HTML "clean up after running a callback")? C8 depends on it; if not, C8 is a finding against the dispatch path, owned
  outside this PR.
- **Q3** Is there a caret display-item probe usable in shell tests (C17), and does any path cancel the platform IME
  composition when focus leaves a text control (I10)?
- **Q4** Can a parent script write into an in-process child's `EcsDom` (same-origin `contentDocument`) without setting
  `needs_render` (§4.4 S9)?
- **Q5** Is there a test harness for the OOP iframe thread (C16)?
- **Q6** For every `PipelineResult` constructor (`pipeline.rs:620-632`, `:703-715`, `:799-811`, `:899-911`), is
  `pipeline.document == pipeline.dom.document_root()`? D-8 reads `document_root()`; if they differ, `AncestorCache`
  takes the document at construction instead.

**Needs a PM decision**:

- **M1** D-9: accept that the fixup does not fire the literal chain's Window `focus` (hand-off to A3, Chromium measurement
  there), or require it here.
- **M2** Register a slot for the per-pipeline rendering order (§3 row 5: children's whole `re_render` before the parent's,
  where the spec runs each step over all docs, parents first)?
- **M3** Register a slot for `AncestorCache.form_owner` as a side-store (§4.3), and for the radio group's missing
  same-tree filter (§3.1)?
- **M4** After Q4: register the in-process child `needs_render` class (S9)?
- **M5** R3 ("change applies"): fold into the A2 program's step-2.1 work, or a slot?

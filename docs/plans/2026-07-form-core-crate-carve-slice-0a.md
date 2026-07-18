# Plan — `elidex-form-core` crate carve (form derived-state Slice 0a)

**Parent**: `docs/plans/2026-07-form-derived-state-reconciliation.md` §6 Slice 0a (umbrella, `/elidex-plan-review`-converged R1-R3).
**Lane**: DOM/form (L3). **Branch**: `domform-form-state-reconciliation` (program's first PR carries umbrella-plan + this plan + this impl).
**Base**: `4a127cbb`. **Status**: Slice-0a plan-memo, `/elidex-plan-review` **CONVERGED** (single round: 0 CRIT / 0 IMP / 4 MIN, all applied). Ready to implement.
**Nature**: a **behavior-invariant crate split** — moves existing code, changes no logic. Not "mechanical" (E0116 coherence chains + a re-export contract), hence this own plan-review per the umbrella's edge-dense rule.

---

## §1 Goal — carve the leaf crate that breaks the `css→form` cycle

The umbrella's O1: `elidex-css` cannot call form predicates because `css→form→dom-api→css` would cycle (`elidex-dom-api/Cargo.toml:17`→css, `elidex-form/Cargo.toml`→dom-api; `elidex-css` depends on neither today). Resolution: carve a new **leaf** crate `elidex-form-core` (deps `elidex-ecs`+`elidex-plugin`+`regex`+`url`+`tracing` only) holding the `FormControlState` component + its coherence-bound value model + the pure derivations. `elidex-css` (Slice 1) then depends on `elidex-form-core` (a leaf with no path back to css) — acyclic.

**This slice (0a) is the move ONLY.** It creates `elidex-form-core`, relocates the enumerated code, and makes `elidex-form` re-export it so every downstream crate is untouched. It does **not** author the new §3.1 pull predicates (`is_effectively_disabled`/`is_checked`/`is_indeterminate`/`is_required`/`is_read_only`) and does **not** touch `elidex-css` — those land in **Slice 1**, where `matching.rs` consumes them (authoring-where-consumed; no dead code in 0a). This refines the umbrella §6 wording "0a … author the §3.1 predicates" → **0a moves; Slice 1 authors+wires** (see §12).

**Success = behavior-invariant**: every moved test passes unchanged, all 5 dependent crates compile with zero source churn, `mise run ci` green (incl. the `-D warnings` `doc` job and `deny`).

---

## §2 Coupled invariants (edge-dense: this is why 0a gets its own review)

A crate split intersects four invariant axes; each pairwise gap is where a prose-only "just move the files" plan leaks (the value-mode E0116 chain below is exactly such a leak — the umbrella's enumeration missed it; a code scout found it).

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Acyclicity** — `elidex-form-core` must be a true leaf (deps ⊆ `elidex-ecs`/`elidex-plugin`/`regex`/`url`/`tracing`) | No DOWN module may import a STAY-UP module (would need `form-core→form` = re-cycle), nor reach `elidex-dom-api`/`elidex-css`/`elidex-script-session`. **Verified clean** (scout §3: zero blockers). |
| **I2** | **Rust coherence (E0116)** — an *inherent* `impl T` must live in the crate defining `T` | Moving `FormControlState`/`FormControlKind` drags their **entire** inherent impls down, and any type those impls *return* must also be down. This is the load-bearing constraint: `value_mode.rs`'s `impl FormControlKind { value_idl_mode }` → `ValueMode` → `ValueSetAction` all forced DOWN (§5.3). No custom traits exist in `elidex-form` (`grep 'trait '` = ∅) ⇒ **no E0117 orphan risk**. |
| **I3** | **API stability (re-export facade)** — 5 crates (`elidex-render`/`elidex-shell`/`elidex-layout-block`/`elidex-js`/`elidex-a11y`) import `elidex_form::X` | Every moved `pub` item MUST be re-exported from `elidex-form` so `elidex_form::X` paths keep resolving with zero churn (§6). Missing one = a downstream compile break. |
| **I4** | **Behavior-invariance** — this is a move, not a rewrite | Moved tests must pass **unchanged**; split modules' inline tests split without losing coverage; no branch/logic edit. Any behavior delta is a bug in the carve. |

**Pairwise intersections (load-bearing):**
- **I1 × I2** — coherence *forces* code down (value-mode cluster), and acyclicity *forbids* the moved code calling back up. Together they fix the exact DOWN set: the type + its full inherent-impl transitive closure, all of which must be leaf-safe (verified: the value-mode cluster is pure, no EcsDom/up refs).
- **I2 × I3** — the facade must re-export the E0116-forced items too (`ValueMode`/`ValueSetAction` + `.value_idl_mode()`/`.idl_get()`/`.idl_set_action()`), not just the "obvious" `FormControlState`/`validate_control` — `elidex-js` calls them.
- **I1 × I4** — the 3 SPLIT modules (lib.rs/fieldset.rs/value_mode.rs) are *edited* (not file-moved), and their inline test modules exercise both halves → the test split is where behavior-invariance is most at risk.

---

## §3 Spec coverage map

Slice 0a **relocates** spec-implementing modules; it changes **no** branch logic, so every row is Touch=relocate-only / behavior-invariant. (This table documents the moved spec surface for trace-ability; there is no new spec-step enumeration because no behavior changes.)

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §4.10.5 The input element (value / step / datetime microsyntax) | value sanitization + step primitives | relocate `input.rs`/`datetime.rs`/`sanitize.rs` → form-core | relocate-only (no branch change) | n/a — behavior-invariant | yes (values flow through, unchanged) |
| HTML §4.10.21.2 Constraint validation | `validate_control` / `ValidityState` / candidacy | relocate `validation/mod.rs` → form-core | relocate-only | n/a | yes (unchanged) |
| HTML §4.10.19.5 Enabling and disabling form controls: the disabled attribute (fieldset pull-half) | `is_fieldset_disabled` (pull half) | split `fieldset.rs`, pull → form-core | relocate-only | n/a | no |

**Breadth**: K=1 spec (HTML), M=3 entries → single-PR scope (a refactor, not a spec-surface expansion).

### §3.1 User-input touch audit

No opcode/native/branch is added or altered. User-controllable input (form values, `disabled`/`readonly`/`pattern` attributes) flows through the **same** relocated code paths — the carve is transparent to input handling. Adjacent pre-existing behavior: unchanged (relocate-only). Exposure delta: none.

---

## §4 Move-list (scout-verified 2026-07-15, file:line)

`crates/dom/elidex-form/src/` — 30 files, 14,459 LoC. Classification:

**MOVE-DOWN whole-file** (→ `elidex-form-core`, deps verified leaf-safe):
- `util.rs` (156) — pure char/UTF-16 helpers, zero crate deps. `pub mod` (public API — re-export as a module).
- `datetime.rs` (722) + `datetime_tests.rs` (465) — imports only `crate::FormControlKind`.
- `input.rs` (845) + `input_tests.rs` (633) + `input_step_tests.rs` (588) — step-grid + key-input + `resolve_input_list` (pure EcsDom **reads**, no mutation → leaf-safe).
- `sanitize.rs` (308) + `sanitize_tests.rs` (451) — deps `elidex_plugin::CssColor` + input/datetime/FCS (all DOWN).
- `validation/mod.rs` (430) + `validation/tests.rs` (745) + `validation/datetime_validation_tests.rs` (330) — **entire module pure** (`validate_control`/`ValidityState`/candidacy/`check_*`/`email_regex`); the validation *binding* systems live in `vm/host/` (untouched), so this is a whole-file move, **not** a split.

**SPLIT** (edited in place, see §5): `lib.rs` (996), `fieldset.rs` (285), `value_mode.rs` (581).

**MOVE-DOWN, added by the `/simplify` gate** (see §11 J1 — the original enumeration mis-drew this boundary): `selection.rs` (+ inline tests), `clipboard.rs`. Both are **value-model modules, not systems**: zero `EcsDom` refs, every fn takes `&`/`&mut FormControlState`; `selection.rs` imports only `crate::util`+`crate::FormControlState`, `clipboard.rs` only `crate::selection`+`crate::FormControlState` — all DOWN. By §10.1's own criterion ("every module that takes `&mut EcsDom` is a system and STAYS UP") they belong DOWN. Confirmed leaf-safe empirically: both moved as pure `git mv` with **0 changed lines and zero import edits**.

**STAY-UP** (systems / bindings / dom-api- or script-session-coupled): `reconciler.rs`, `init.rs`, `radio.rs`, `submit.rs`(+tests), `select/mod.rs`(+tests, `elidex_dom_api` dep), `sizing.rs`, `label.rs`, `clone.rs` (`elidex_dom_api::ClonedFrom`), `focus_snapshot.rs`, `ancestor_cache.rs`, `inert_document.rs` (`elidex_script_session` — hard UP).

**Judgment item kept UP**: `sizing.rs` (`form_intrinsic_size`) — dependency-clean for DOWN, but keeping it UP **costs nothing** (it touches none of the type's private state) and no leaf consumer needs intrinsic-size. Contrast `selection.rs`/`clipboard.rs`, where keeping them UP *did* cost (§11 J1).

---

## §5 The three splits (edit-in-place, each with a test-module split)

### §5.1 `lib.rs` — type model ↓ / crate facade ↑
- **DOWN** (the FCS data model + pure API): `MAX_PATTERN_LENGTH` (`lib.rs:68`), `compile_pattern_regex` (`:74-86`), `enum FormControlKind` + `impl FormControlKind` (`:89-349`), `struct SelectOption` (`:351-364`), `enum SelectionDirection` (`:366-376`), `struct FormControlState` (`:378-468`), `impl Default for FormControlState` (`:470-512`), `impl FormControlState` inherent block (`:514-992`). Header `use elidex_ecs::Attributes; use std::sync::Arc;`.
- **STAY-UP**: the `mod`/`pub mod` tree (`:7-26`) + the `pub use` facade (`:28-62`) — rewritten so moved targets become `pub use elidex_form_core::{…}` re-exports (§6).
- `lib_tests.rs` (832) → **DOWN** (exercises the FCS type).

### §5.2 `fieldset.rs` — pull ↓ / push ↑
- **DOWN**: `first_legend_child` (`:29-43`), `is_in_first_legend` (`:45-57`), `is_fieldset_disabled` (`:59-95`). `is_fieldset_disabled` calls the other two (intra-core after move).
- **STAY-UP**: `propagate_fieldset_disabled` (`:11-27`), `disable_descendants` (`:97-133`) — mutate FCS + `ElementState`. `propagate_fieldset_disabled` calls `first_legend_child` → a **UP→DOWN** call (sound: `elidex-form` depends on `elidex-form-core`).
- Inline `mod tests` (`:135-285`) exercises **both** halves → split: pull-tests → form-core, push-tests → stay in `elidex-form`'s `fieldset.rs`.

### §5.3 `value_mode.rs` — E0116-forced type cluster ↓ / migration system ↑ ⚠
The non-obvious one (scout §4). `impl FormControlKind { fn value_idl_mode }` (`:107-170`) is **inherent** → E0116 requires it in `FormControlKind`'s crate (form-core). It returns `ValueMode` → `ValueMode` must be DOWN; `ValueMode`'s own `impl` (`:64-105`) follows (E0116); `idl_set_action` returns `ValueSetAction` → that enum too.
- **DOWN**: `enum ValueMode` (`:27-42`), `enum ValueSetAction` (`:50-62`), `impl ValueMode` (`:64-105`), `impl FormControlKind { value_idl_mode }` (`:107-170`). All pure (no EcsDom).
- **STAY-UP**: `apply_type_change_value_migration` (`:197-259`, `pub(crate)`) — a write system (`&mut EcsDom`). Its inline `mod tests` (`:261-581`) imports `FormControlReconciler` (UP) → stay UP.

---

## §6 Re-export facade contract (MANDATORY — I3)

`elidex-form` MUST re-export every moved `pub` item so `elidex_form::X` resolves unchanged for the 5 dependents. Verified-referenced-downstream (esp. `elidex-js`): `FormControlState`, `FormControlKind`, `ValidityState`, `validate_control`, `is_constraint_validation_candidate`, `is_fieldset_disabled`, `ValueMode`, `ValueSetAction` (+ methods `.value_idl_mode()`/`.idl_get()`/`.idl_set_action()`, auto-resolved once types re-export), `apply_step`, `resolve_input_list`, `form_control_key_input_action`, `KeyAction`, `StepError`. Also re-export the `pub`-but-no-current-external-consumer items: `SelectOption`, `SelectionDirection`, `first_legend_child`, `is_in_first_legend`, `sanitize_for_type_change`, `form_control_key_input`, `MAX_PATTERN_LENGTH`, and **`pub mod util`** (a module path — `elidex_form::util` is public API; re-export as `pub use elidex_form_core::util;`).

Verification: after the carve, `grep -rn 'elidex_form::' crates/ --include='*.rs'` (outside elidex-form itself) must resolve entirely against the facade — the plan-review + CI compile is the check.

**Facade lifetime — a migration seam, not permanent "API stability"** (`/simplify` A5). The blanket re-export is what makes 0a a *provable* zero-churn move, and is correct for this slice. But from **Slice 1** on, `elidex-css` imports `elidex_form_core::X` directly while the other 5 consumers import `elidex_form::X` — **one type, two canonical paths**, re-litigated at every import ([[feedback_duplicated-decision-surface-blocks-converge]]). Label it a seam with an end-slice, not a permanent API: a later slice should decide whether consumers migrate to `elidex_form_core::` directly and the facade shrinks to only what `elidex-form` genuinely re-exports. Left unstated, the dual path becomes permanent by default. **Resolved (this landing, `/elidex-review` F2)**: registered as durable slot **`#11-form-core-facade-shrink`** ([[project_open-defer-slots]]) — the shrink-vs-keep-as-stable-boundary decision triggers at **Slice 1's first `elidex_form_core::` import** (the moment the dual path is born); deliberately **not** folded into Slice 5 (whose tail-delete is cross-lane-gated on focus-A3 + L4, independent of the facade). So the seam now has a named end-condition + a tracked home, not "permanent by default".

---

## §7 Cargo + workspace

- **New** `crates/dom/elidex-form-core/Cargo.toml`: `name = "elidex-form-core"`; deps `elidex-plugin`/`elidex-ecs`/`regex`/`url`/`tracing` (workspace); `[lints] workspace = true`; version/edition/etc. = workspace. **Drops** `elidex-dom-api` + `elidex-script-session` (the two deps that made `elidex-form` non-leaf).
- **`elidex-form/Cargo.toml`**: keep all current deps **+ add `elidex-form-core.workspace = true`**.
- **Root `Cargo.toml`** `[workspace]`: add `"crates/dom/elidex-form-core"` to `members` (alongside the other `crates/dom/*`), and a `elidex-form-core = { path = "crates/dom/elidex-form-core" }` entry under `[workspace.dependencies]` (matching the repo's workspace-dep convention).
- **Naming**: `-core` suffix precedent = `elidex-storage-core` (leaf/foundation). Consistent.
- **Unaffected dependents** (re-export keeps them stable): `elidex-render`, `elidex-shell`, `elidex-layout-block`, `elidex-js`, `elidex-a11y`. **`elidex-css` unchanged in 0a** (it gains the `elidex-form-core` dep in Slice 1, not here).

---

## §8 Doc-link repairs (exactly 2 carve-induced breaks — scout §7)

Both are DOWN-module doc comments linking to STAY-UP items (unreachable after the move, since form-core can't depend on form):
1. `validation/mod.rs:419` — `[…](crate::radio::is_radio_group_satisfied)` → `radio` STAYS UP. **Repair**: de-link to plain prose (form-core cannot reference `radio`).
2. `sanitize.rs:42` — `[`crate::clipboard_paste`]` → `clipboard` STAYS UP. **Repair**: de-link to plain prose.

Both fail CI's `-D warnings` `doc` job if left. **Verify the exact set at impl** by `grep -nE '(///|//!).*crate::' <moved modules>` (belt-and-suspenders over this scout list). Pre-existing (not carve-induced, out of scope): `datetime.rs`'s `crate::input::sanitize_value` mis-link (both DOWN → still resolves-ish, unchanged); `vm/mod.rs:1104` prose `elidex_form::validation::validate_control` (a private-mod path that was never real — optionally fix to `elidex_form::validate_control`, doc-only).

---

## §9 Test plan + verification

Behavior-invariance is the whole contract, so verification is **differential**, not new-assertion:
- Moved test files (`lib_tests`/`datetime_tests`/`input_tests`/`input_step_tests`/`sanitize_tests`/`validation/tests`/`validation/datetime_validation_tests`) pass **unchanged** in `elidex-form-core`.
- Split-module inline tests (`fieldset` pull-tests, `value_mode` — but value_mode's tests stay UP): the pull-half tests move to form-core, push-half tests stay; **no assertion lost** (diff the test bodies pre/post).
- `cargo test -p elidex-form-core --all-features` + `cargo test -p elidex-form --all-features` both green.
- `mise run ci` green: 3-OS check, clippy, nextest, **doc job (`-D warnings`** — catches unrepaired doc-links), **deny** (the new crate's `regex`/`url`/`tracing` are already workspace-approved).
- The 5 dependent crates compile with **zero source edits** (the facade proof) — `cargo build -p elidex-js -p elidex-render -p elidex-shell -p elidex-layout-block -p elidex-a11y`.

---

## §10 Layering check

- **`elidex-form-core` is an engine-independent leaf** (deps `elidex-ecs`+`elidex-plugin`+`regex`+`url`+`tracing`) — no VM/host, no dom-api, no css. ✓
- Moving `validate_control` / `is_fieldset_disabled` / sanitization **down** into a leaf crate **reinforces** the CLAUDE.md Layering mandate (algorithm in engine-independent crate) — these were already engine-independent; the carve makes the crate boundary match the layering. ✓
- **No new `vm/host/` code**; the validation *binding* systems (`vm/host/validity_state.rs`, `html_form_proto.rs`) are untouched. ✓
- **No B1-core touch** (that's Slice 1's constant deletion). 0a is confined to `crates/dom/elidex-form*` + `Cargo.toml`s. ✓

### §10.1 ECS-native check

- **Component data DOWN, mutating systems UP** — the ECS-native split by construction. `FormControlState` (the ECS component) + its `&self`/`&mut self` value-model methods (data-methods, no `&mut EcsDom`) + the pure derivations move to the leaf crate; every module that takes `&mut EcsDom` (reconciler / init / radio / submit / select / the fieldset push-half / the value-mode migration) is a **system** and STAYS UP. No system is dragged into the leaf.
- **No OO pattern introduced** — `grep 'trait ' crates/dom/elidex-form/src/` = ∅; the carve adds no observer/registry/subscriber/inheritance shape. `FormControlState` stays an ECS component (not regressed to a side-store). The re-export facade is a `pub use`, not a dynamic-dispatch shim.
- **No new component read/write** — 0a relocates existing code; sub-check 2b is N/A (the pull predicates that read FCS are Slice 1). The only cross-boundary calls are the two SPLIT modules' UP→DOWN calls (push→pull, migration→classifier), which preserve the existing data-flow (§5.2/§5.3).

---

## §11 Open / judgment items (for `/elidex-plan-review`)

- **J1 — RESOLVED by the `/simplify` gate (the original lens was wrong).** The draft kept `selection.rs`/`sizing.rs` UP on the lens *"no leaf consumer today → a boundary should be pulled by a consumer, not pushed speculatively."* That lens is **right for `sizing.rs` and wrong for `selection.rs`/`clipboard.rs`** (which J1 never even mentioned), for a reason it cannot see: **the deciding invariant is not "is there a leaf consumer?" but "does the module touch the type's private state?"** — because that is what `pub(crate)` *meant*. `selection.rs`+`clipboard.rs` accounted for **55/66 (83%)** of the raw private-field writes, so keeping them UP forced **4 permanent widenings** (`cursor_pos`/`selection_start`/`selection_end`/`char_count`) that no other non-test code touches — and `set_cursor()`/`set_selection()` snap to char boundaries, so pub fields would let any crate write an unsnapped byte offset **past that guard**. Moving them DOWN is *more* move-only (2 verbatim `git mv`), and let all 4 fields revert to `pub(crate)`. `sizing.rs` stays UP: it touches no private state, so it costs nothing. **Residual widening = `value` + `dirty_value` only**, genuinely justified (`clone.rs:127-128`, STAYS UP, restores raw cloned state per the #466 cloning-steps contract — the real "system mutates component data" case), plus 4 methods with real STAY-UP callers.
  - **Lesson for later slices**: when a carve forces a visibility widening, that is a *boundary* signal, not an encapsulation question. Fix the boundary; do not add setters (ECS components take pub data, not OO accessors).
- **J2 — test-split fidelity** for `fieldset.rs` (pull-tests → core, push-tests → up). Confirm at impl that no shared test helper straddles the split (if one does, it moves down or duplicates).
- **J3 — `pub mod util` re-export**: `elidex_form::util` is a public *module* path. Re-exporting a module (`pub use elidex_form_core::util;`) vs re-declaring — confirm the module-path form keeps `elidex_form::util::next_char_boundary` resolving.
- **J4 — workspace-dep style**: confirm the repo uses `[workspace.dependencies]` path entries (vs direct path deps in each crate) so `elidex-form-core` is declared consistently.
- **J5 — RESOLVED IN 0a (`/elidex-review` F3; was: hand-off to Slice 0b).** The draft deferred this as "an *edit* not a move", but F3 flagged the inconsistency: J1's "a widening is a boundary signal → fix the boundary" lesson was applied to `selection.rs`/`clipboard.rs` in this same PR, so leaving `compile_pattern_regex`'s `pub` widening — whose only effect was to feed a hand-rolled `reconciler.rs:235` inline duplicating `FormControlState::update_pattern` (so `update_pattern` shipped as **dead `pub`**) — is a half-applied lesson + a One-issue-one-way / "dead code は残さず削除" violation. **Verified provably behavior-identical on all arms** (Some / None / compile-error→`Some(None)` / written-fields = `pattern`+`cached_pattern_regex` only; the `!=` guard is **preserved**, so the method recomputes only when the value changed = byte-identical incl. the no-op skip). Applied: the inline is now `fcs.update_pattern(new_value)`; `compile_pattern_regex` reverts `pub`→**`pub(crate)`** (its true minimum — still `crate::`-reached by `validation/tests.rs`); the `pub(crate) use …compile_pattern_regex` alias is deleted. Post-edit grep: **zero `compile_pattern_regex` refs outside form-core**; `update_pattern` now has its one real production caller. Stays inside 0a's contract because *behavior-invariant* is the contract and a provably-identical edit satisfies it (same class as the `/simplify` selection/clipboard moves). This also moots the MIN that J5's defer-home was mis-cited (the reconciler-side dup was never a "0b IDL-hand-mirror = VM-host" item — it is resolved here, not deferred).
  - **Remaining 0b hand-off (MIN "value/dirty_value latent bypass").** The residual `value`+`dirty_value` `pub` widening (J1 — kept because `clone.rs:127-133` restores raw cloned state per the #466 cloning-steps contract) could likewise be eliminated by an FCS `adopt_value_state(value, dirty)` method covering clone's read+write pair, reverting **both** to `pub(crate)`. Unlike F3 this is an *edit touching the #466 clone contract* (not provably a no-op without an F3-style per-arm proof of the clone read+write pair) → left to **Slice 0b**, where its own plan-review scopes the clone-contract interaction. Recorded so the residual widening reads as known-deferred-with-a-fix-path, not sanctioned-permanent.
- **J6 — pre-existing duplication surfaced (not introduced) by the move**: form-core now hosts **two** `replace_selection` — the method `FormControlState::replace_selection` (snaps, folds CRLF for TextArea, sets `dirty_value`) and the free fn `selection::replace_selection` (none of those). The divergence is deliberate-but-deferred (`#11-textarea-edit-path-newline-normalization`). Merging them is a **behavior** decision, not a move → **Slice 0b/1**. Recorded so the two-impl state reads as known-deferred, not sanctioned.

---

## §12 Relationship to the umbrella

The umbrella §6 Slice 0a cell says "move … + **author the §3.1 predicates**". This plan **refines** that: **0a = move-only** (behavior-invariant), and the new pull predicates (`is_effectively_disabled` etc.) are **authored in Slice 1**, where `matching.rs` + `submit.rs:353` consume them (authoring-where-consumed → no dead code in the 0a→1 window; and 0a stays a pure, easily-verified move). Consequently **0a does not touch `elidex-css`** — the `elidex-css → elidex-form-core` dependency edge is added in Slice 1 with the first `matching.rs` call. **Umbrella §6 reconciled** (this commit, 3 edits — not "one-line"): (a) the 0a cell drops `is_effectively_disabled`/the-§3.1-predicates from 0a's moved set and states MOVE-ONLY + no-`elidex-css`-touch; (b) the Slice 1 cell now authors the predicates + adds the `elidex-css`→`elidex-form-core` dep; (c) the closing "Slice 2 depends on 0a (exported predicate)" → "depends on Slice 1 (which authors it)".

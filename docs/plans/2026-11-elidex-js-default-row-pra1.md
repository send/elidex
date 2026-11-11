# PR-A1 — make `elidex-js`'s `default` row compile (U0 PR-A1, per-PR plan-memo)

**Program**: U0, the per-feature-profile build matrix (`docs/plans/2026-11-per-profile-build-matrix-umbrella.md`,
ratified PM 597). **This PR**: U0 §6's PR-A1 row and its "PR-A1 hand-off" block. **Owner**: VM P4 lane (PM 520 (1)).
**Shape**: F1-b, the interim (U0 §5; PM 520): `default = []` stays, every fix is a `cfg` fix, and `elidex-plugin`
stays optional (PM 584). **Order**: lands before L3's brand-check Rev 5 (PM 640). **Baseline**: origin/main
`8344af2b`. Every `file:line` below is at `8344af2b`. Paths are relative to `crates/script/elidex-js/src/` unless
they start with `crates/` or `docs/`.

**The defect, stated as a defect.** `elidex-js` declares `default = []`, and that configuration does not compile.
`cargo clippy -p elidex-js -- -D warnings` stops with 17 errors, and `cargo doc` stops at the first one, so the
row has no rustdoc output at all. The engine-less configuration is a declared row of U0 (I2), so U0's matrix
(PR-M1) cannot land while it is red. Nothing in CI runs this row today: the `check` job's default-features step is
`cargo clippy --workspace -- -D warnings` (`.github/workflows/ci.yml:111-112`), and the workspace unifies
`elidex-js`'s features with `elidex-shell`'s (`engine` + `compat-webapi`), so the row has been red unseen.

## §0. Status, history, premises

### §0.1 Status and history

- **Draft 1 (2026-11-11)**, written at `8344af2b`, before `/elidex-plan-review`. Inputs: U0 §5, §6 (PR-A1 row and
  hand-off), §8, §9; the driver's survey at `8344af2b`
  (`elidex-pm-trial/handoff/vm-p4-pra1/survey-8344af2b.md`); PM 640, PM 584, PM 564, PM 626. Every measurement in
  this memo was re-run for it (§5.1 gives the commands).
- U0 is not edited. U0's text does not ask for a link from its PR-A1 row or hand-off to this memo (U0 `:671-674`
  says only that "PR-A1's own plan" decides the destinations), so no pointer is added.
- **Rev 1.1 (2026-11-11), PM 649**: A1-8 / §7.2 item 1 provisionally (b); the `#11-host-data-full-decomposition`
  record goes to a PM-side ledger facet (PMO); §5.3 re-measured at `bea9ef19` (25, unchanged), owners VM P4 21 /
  L3 4; the window and the §5.6 PR-body line added.

### §0.2 Premises re-measured

Toolchain `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`env -u RUSTUP_TOOLCHAIN rustc --version`), macOS, at
`8344af2b`.

| Premise (source) | Measured | Holds? |
|---|---|---|
| 17 clippy errors in the `default` row (survey) | 17: E0432 `vm/init.rs:11`; E0433 `vm/lifecycle.rs:45`; 2 unused imports `vm/natives_json/mod.rs:13`, `:15`; E0599 `vm/lifecycle.rs:34`, `:45`; 10 × E0609 `vm/lifecycle.rs:46-63`; E0433 `vm/gc/collect.rs:47` | yes |
| rustdoc stops at the same E0432 (survey) | `cargo doc` rc 101 at `vm/init.rs:11`, 0 doc warnings reported | yes |
| "Ten engine_feature-only HostData fields are named" (survey item 5) | 10 E0609 errors naming **9** distinct fields (`node_iterator_states_shared` twice, `:47` and `:63`) | **no** — §0.3 |
| `vm/gc/collect.rs` is 2074 lines (U0 §6) | 2078 at `8344af2b` (U0 measured at `69fbe61f`; Layout's C1-pre moved it) | stale, not false |
| The natives_json consumers' gating is unknown (survey "Uncertain") | every consumer is engine-gated: `vm/host/indexeddb/value.rs`, `vm/host/request_response/response_statics.rs`, `vm/host/service_worker/deliver.rs`, `vm/host/structured_serialize.rs`, `vm/host/worker_scope.rs` — each a `#[cfg(feature = "engine")]` module in `vm/host/mod.rs` (`:228`, `:314`, `:322`, `:334`, `:375`) | resolved |
| Whether `new_with_mode`'s test path needs `engine` (survey "Uncertain") | yes: its parameter type is `EngineMode`; all its callers are engine-gated (`lib.rs:40-41` `mod engine`, `lib.rs:76-77` `mod tests_webapi_gate`, `vm/tests/tests_document_wrapper.rs:10` `#![cfg(feature = "engine")]`; `git grep -n 'new_with_mode' -- crates/script/elidex-js/src`) | resolved (fix A1-3) |

### §0.3 Premise errors

1. **Survey item 5**: "Ten engine_feature-only HostData fields are named. (C0 says 9; that is wrong.)" The survey is
   the one that is wrong: 10 E0609 diagnostics, 9 distinct fields. C0's 9 was a field count.
2. **Survey fix option for `collect.rs:45-49`** ("by not naming the optional type"): the branch has no reader in
   the `default` configuration at all. Its only consumer, the `active_cssom_rule_ids` field of `GcRoots`
   (`vm/gc/roots.rs:206-208`), and the struct literal's field (`vm/gc/collect.rs:1141-1142`) are both engine-gated.
   Re-typing the branch would leave an unused variable. The fix removes the branch (§2, A1-6).
3. **"The count is a lower bound" (U0 §5)** is confirmed. Once name resolution passes, a later compiler phase
   reports 6 further errors in the `default` row (5 `dead_code`, 1 `clippy::needless_pass_by_value`), and one
   of the fixes for those surfaces a seventh (an unused import). §1.3 lists all seven; each is named in the PR body
   (U0 §6 acceptance).
4. **The stub's diagnostic name.** rustc names the stub `engine_feature::HostData` (the diagnostic's path), because
   both the engine module and the stub module are called `engine_feature` (`vm/host_data/mod.rs:23` and `:681`).
   This is not a defect PR-A1 owns. It is noted because a reader of the diagnostics can take the stub for the engine
   type.

## §1. Scope

### §1.1 In scope

The 17 diagnostics of §0.2, the 7 that surface behind them (§1.3), and the doc statements that these fixes make
false (§2, A1-3 and A1-9). Every fix is a `cfg` fix (kind (b)). The full list is §2's fix table.

### §1.2 Out of scope

- **The crate split (F1-c).** It is a separate program, owned by VM P4 and not planned here (U0 §5). §6 records,
  for that program, the items this PR gates. They are core items that exist only for the host.
- **`HostData`'s decomposition** (`#11-host-data-full-decomposition`): PR-A1 moves nothing (§4).
- **Rustdoc warnings in the `default` row.** They are listed (§5.3) and handed to VM P4's PR-D… (U0 §6,
  PM 561/634/649). PR-A1 fixes doc *statements* that its own code change makes false. It does not fix link
  forms. §5.3 gives the reason.
- **The `default` row's tests** (PR-T, PR-T0…). PR-A1 checks only that the row's lib test target still compiles
  (§5.1). This is not PR-T's census.
- **CI**. The row's gate is PR-M1's `feature-profiles` job. PR-A1 adds no trip-wire (§5.4).
- **`vm/gc/collect.rs`'s split.** That decision is already recorded: the file belongs to umbrella 7b
  (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:84`, "`vm/gc/collect.rs` 2074 (**umbrella 7b**, through the
  children it mints for the work)"). PR-A1 takes that decision and makes no second one. Its touch removes 5 lines
  (2078 → 2073), so it does not grow the file.

### §1.3 Diagnostics: the 17, and the 7 behind them

| # | Diagnostic (at `8344af2b`, `default` row) | Group |
|---|---|---|
| D1 | E0432 `elidex_plugin` — `vm/init.rs:11` | L1 EngineMode import |
| D2 | E0433 `elidex_api_observers` — `vm/lifecycle.rs:45` | L2 unbind's bound-world cleanup |
| D3–D4 | E0599 `dom()` on the stub — `vm/lifecycle.rs:34`, `:45` | L2 |
| D5–D14 | E0609 on the stub — `vm/lifecycle.rs:46`, `:47`, `:51`, `:52`, `:53`, `:54`, `:59`, `:60`, `:61`, `:63` | L2 |
| D15 | E0433 `elidex_ecs` — `vm/gc/collect.rs:47` | L3 dead no-engine branch |
| D16–D17 | unused imports `parse_json_str` / `stringify_for_structured_shortcut`, `stringify_to_string` — `vm/natives_json/mod.rs:13`, `:15` | L4 host-only core items |
| N1 | `dead_code`: `parse_json_str` never used — `vm/natives_json/parse.rs:501` | L4 |
| N2 | `dead_code`: `stringify_for_structured_shortcut` never used — `vm/natives_json/stringify.rs:423` | L4 |
| N3 | `dead_code`: variant `Microtask::NotifyMutationObservers` never constructed — `vm/natives_promise.rs:59` | L4 |
| N4 | `dead_code`: `create_illegal_constructor_function` never used — `vm/shape_ops.rs:322` | L4 |
| N5 | `dead_code`: `native_illegal_constructor_unreachable` never used — `vm/value.rs:749` | L4 |
| N6 | `clippy::needless_pass_by_value`: `global_scope_kind` — `vm/init.rs:163` | L1 |
| N7 | unused import `super::super::VmInner` — `vm/natives_json/parse.rs:13` (appears only once N1's fix gates its sole user) | L4 |

N1–N6 were measured with D1–D17 fixed. N7 was measured with N1–N6 fixed as well. With all 24 fixed, the row is
clippy-clean (§5.2).

## §2. Design

### §2.1 The leak classes and the one mechanism decision

The 24 diagnostics fall into two classes.

- **Class I: core code names an item that only `engine` compiles.** D1–D15. The named item is either from an
  optional dependency (`elidex_plugin`, `elidex_api_observers`, `elidex_ecs`) or engine-only `HostData` API
  (`dom()` and per-bind fields that the stub does not have).
- **Class II: a core item has readers only in engine-gated code.** D16–D17, N1–N5, N7, and N6 (a parameter that is
  read only under `engine`). In the `default` configuration such an item is dead code. CLAUDE.md's rule for dead
  code is "接続するか削除": either it gets a reader in that configuration or it is removed from that configuration.

**The mechanism decision.** The question is: should the default `HostData` stub be met by gating at the use site,
or by an engine/non-engine seam in the layer that owns the state?

The decision: **gate at the use site (kind (b)). The stub's API is the configuration-total interface, and it does
not grow.** The stub (`vm/host_data/mod.rs:680-704`) offers `new`, `unbind`, `is_bound` and `gc_root_object_ids`,
and those are the only `HostData` items that ungated core code may name. Everything else on `HostData` is
engine-only, and only engine-gated code names it. The same rule applies to Class II: a core item whose readers are
all engine-gated becomes engine-gated itself, together with every site in that configuration that names it.
Silencing the lint with `allow(dead_code)` or `allow(unused_*)` is not used, and PR-A1 removes the one such
`cfg_attr` its fixes make unnecessary (A1-3).

Reasons, in the authority order of CLAUDE.md's 至上命題:

1. **The spec is silent.** ECMA-262 §4.2 leaves host-defined facilities to the host and does not say whether an
   implementation builds without one (U0 §5's annotation; §3 row 1). So the decision rests on elidex's
   principles.
2. **Owner and symmetry.** The `if hd.is_bound()` block in `Vm::unbind` (`vm/lifecycle.rs:30-67`) undoes what
   `Vm::bind` set up. `Vm::bind` installs the mutation dispatcher (`vm/vm_api.rs:324`, `set_mutation_dispatcher`),
   and `Vm::bind` is wholly `#[cfg(feature = "engine")]` (`vm/vm_api.rs:201`). The cleanup is therefore the engine
   half of an engine-only pair. Its gate belongs beside its counterpart's, in the function that already carries an
   engine block for the VM-side half (`vm/lifecycle.rs:90`). In the `default` configuration the stub's
   `is_bound()` is constantly `false` (`vm/host_data/mod.rs:696-698`) and no `bind` exists, so the gated block was
   unreachable there anyway. The gate states that as a compile-time fact and adds no behaviour.
3. **Contract 2 of U0's hand-off rules out the relocation candidate (kind (a)).** The candidate moves the clearing
   into `HostData::unbind` (`vm/host_data/engine_feature/bind.rs:118`). Contract 2 forbids adding a new home for a
   cleanup concern, and it forbids splitting one concern across a boundary the base does not have. The block is one
   coupled sequence. The bridge drop must come first so that `node_iterator_states_shared` is uniquely owned when it
   is cleared (the `debug_assert_eq!(Arc::strong_count(..), 1)` at `:62-66` spans both steps). Two of its four
   concerns are state that `HostData` does not own: the `EcsDom`'s dispatcher hook and the world's
   `MutationObservedBy` components. So the candidate has two forms, and both break contract 2:
   - Moving the whole sequence would make `HostData::unbind` a new home for the transient-observer concern, and
     would give `HostData` a dependency on `elidex_api_observers`.
   - Moving only the field clears would split the coupled sequence across the `Vm` / `HostData` boundary.
   Consolidating the homes is `#11-host-data-full-decomposition`'s job (contract 2 and contract 3).
4. **Growing the stub would mean a false API.** To compile, a stub `dom()` and stub fields would have to name
   `elidex_ecs::EcsDom`, the Range/NodeIterator/Selection state types, or stand-ins for them. Naming the real types
   is kind (c), which PM 584 rules out. Stand-ins would be an API that claims a DOM exists where none can.
5. **One issue, one way.** At the base the crate has two ways of handling host-only code in core files:
   - the gate, as in `#[cfg(feature = "engine")] use super::bit_get;` (`vm/gc/collect.rs:15-16`) and the
     engine-gated `GcRoots` fields;
   - a suppression, the one `cfg_attr(not(feature = "engine"), allow(unused_variables))` at `vm/init.rs:162`.
   PR-A1 uses the gate everywhere and removes the suppression, which leaves one way.
6. **The split, later.** U0 §5 keeps F1-b's `cfg` annotations as the map of the seam that the F1-c split will cut.
   Gating in place keeps every engine-only item visibly marked where it lives, and §6 lists them. A relocation
   would decide now where host lifecycle code lives, and that is the split's decision.

**Rejected alternatives**, each with the reason:
- Kind (a), relocation into `HostData::unbind`: reason 3.
- Growing the stub: reason 4.
- Kind (c) for any of the three candidates: §7.
- Gating all of `Vm::unbind` to match `Vm::bind`: this removes a public function from the `default`
  configuration, changes the stub's API (its `unbind` / `is_bound` would lose their callers) and changes the
  ungated caller `teardown_document` (`vm/lifecycle.rs:666`). All three are shape decisions that belong to the
  split, not to F1-b.
- `allow(dead_code)` / `allow(unused_imports)` for Class II: reason 5, and CLAUDE.md's dead-code rule.

### §2.2 Fix table

Kind key: (a) relocate, (b) gate in place, (c) non-optional dependency (U0 §6 hand-off, quoted in §7). Every fix
below is (b). No fix combines kinds.

| Fix | Leak and the state's owner | Kind and reason | Destination (file, function) | Guard: test or measurement |
|---|---|---|---|---|
| **A1-1** | D1. `use elidex_plugin::EngineMode` is ungated. `EngineMode` is the engine-wide core/compat authority, read only through `spec_level_policy()` under `engine` (`vm/init.rs:177`). Owner: `elidex-plugin` (optional, `Cargo.toml:80`). | (b). Every reader is engine-only. Kind (c) is not needed (§7). | `vm/init.rs:11`: the import gets `#[cfg(feature = "engine")]`. | `default` clippy (D1 gone); app and browser clippy and rustdoc unchanged (§5.2) |
| **A1-2** | N6, and D1's last ungated reader. `new_with_scope` takes `global_scope_kind` and `engine_mode`, and both are stored or read only under `engine` (`vm/init.rs:177`, `:839-844`). In `default` both parameters are dead. Owner: `VmInner` (engine-gated fields). | (b). Each parameter gets `#[cfg(feature = "engine")]` (cfg on a function parameter is stable). The `cfg_attr(not(feature = "engine"), allow(unused_variables))` at `:162` is deleted. | `vm/init.rs:162-163`, `new_with_scope`. `Vm::new` (`:30-32`) passes the two arguments under `engine` and none otherwise: two `cfg`-ed `let vm = …;` statements and a tail `vm`. This form was measured clippy-clean in all three rows. It adds one `cfg(not(feature = "engine"))` site, and A1-6 removes one (§6). | `default` clippy (N6 gone, no `let_and_return`); app and browser clippy rc 0 (§5.2) |
| **A1-3** | Doc and test surface of A1-1/A1-2. `Vm::new_with_mode` (`vm/init.rs:47-50`, `#[cfg(test)]`) takes `EngineMode`, so the `default` row's lib tests would not compile once A1-1 gates the import. The `new_with_scope` doc says "In non-`engine` builds the scope kind / mode are unused … but accepted to keep one shared construction body" (`:158-160`), which A1-2 makes false. | (b). `#[cfg(test)]` becomes `#[cfg(all(test, feature = "engine"))]`. All callers are engine-gated (§0.2). The doc sentence is rewritten to say the parameters exist only under `engine`. | `vm/init.rs:47` (`Vm::new_with_mode`); `vm/init.rs:158-160` (doc of `new_with_scope`) | `cargo test -p elidex-js --lib --no-run` in `default` rc 0 (§5.2) |
| **A1-4** | D2–D14. The `if hd.is_bound() { … }` block of `Vm::unbind` (`vm/lifecycle.rs:30-67`) names `elidex_api_observers` and engine-only `HostData` API. Owners: the `EcsDom` (mutation-dispatcher hook), the bound world (`MutationObservedBy` components), and `HostData` (per-bind fields). The base has 2 homes and 4 concerns (U0 hand-off contract 2). | (b). Mechanism decision §2.1, reasons 2–4. | `vm/lifecycle.rs:30`, `Vm::unbind`. The `if` statement gets `#[cfg(feature = "engine")]` (cfg on a statement). It stays where it is, before `hd.unbind()` at `:68`. Homes stay 2 and concerns stay 4. Nothing moves (§4). | `default` clippy (D2–D14 gone); engine behaviour: `vm/tests/tests_mutation_observer/lifecycle.rs` (`:25`, `:57`, `:89`, `:118`, `:179`), `vm/tests/tests_mutation_observer/transient.rs`, `vm/tests/tests_shadow_dom/events.rs:577` under `--all-features` (the `check` job) |
| **A1-5** | (none: the order contract). The order contract is A1-4's invariant, not a separate fix. §2.3 states it. | — | — | §2.3 |
| **A1-6** | D15. The `cfg(not(feature = "engine"))` branch of `collect_garbage` (`vm/gc/collect.rs:45-49`) builds an empty `HashMap<elidex_ecs::Entity, HashSet<u64>>`. Its only reader is engine-gated (`vm/gc/roots.rs:206-208`, `vm/gc/collect.rs:1141-1142`). Owner: the bound session's CSSOM (`SessionCore`, engine). | (b), the degenerate form: in the `default` configuration the branch names nothing that is read, so it is removed and the engine branch (`:40-44`) stays as it is. | `vm/gc/collect.rs:45-49` deleted, in `VmInner::collect_garbage` (`:25`). Split decision: §1.2. | `default` clippy (D15 gone, no unused variable) |
| **A1-7** | D16–D17, N1, N2, N7. `parse_json_str` (`vm/natives_json/parse.rs:501`) and `stringify_for_structured_shortcut` (`vm/natives_json/stringify.rs:423`) are JSON helpers in core, and their only readers are in `vm/host/` (§0.2). `stringify_to_string` keeps a core reader (`vm/natives_json/stringify.rs:368`). Owner: the language core's JSON module (ECMA-262 §25.5). The helpers serve HTML's structured-serialize shortcut and worker messages. | (b). They are core algorithms used by the host, which is the outbound direction the Layering mandate allows, so they stay in core. In `default` they have no reader, so they are gated. | `vm/natives_json/mod.rs:13-16`: the re-exports split, with `native_json_parse` / `native_json_stringify` ungated, and `parse_json_str`, `stringify_for_structured_shortcut` and `stringify_to_string` re-exported under `engine`. `vm/natives_json/parse.rs:501` (`parse_json_str`) and its import `:13` (`VmInner`); `vm/natives_json/stringify.rs:423` (`stringify_for_structured_shortcut`) — each gated. | `default` clippy (5 gone); JSON behaviour: `vm/tests/tests_json.rs`, `vm/tests/tests_json_shortcut_date.rs` under `--all-features` |
| **A1-8** | N3. `Microtask::NotifyMutationObservers` (`vm/natives_promise.rs:59`) is the DOM's "notify mutation observers" microtask (WHATWG DOM §4.3). Only `vm/host/` constructs it (`vm/host/html_slot_proto.rs:68`, `vm/host/mutation_observer.rs:464`). Owner: the host's mutation-observer machinery. | (b). Kind (a) needs an existing seam, and the core `Microtask` enum has no host-microtask extension point. Creating one is the split's decision (§6). Provisionally (b), PM 649. Re-examine A1-8 when the slot is ruled (`#11-vm-inbound-host-call-membership-test`, §2.2 "The membership test"). | `vm/natives_promise.rs:59` (variant, in `enum Microtask` `:31`); `vm/natives_promise.rs:333` (its arm in `drain_queue_pass` `:311`, gated as a whole, so the inner `#[cfg(feature = "engine")]` block at `:339` becomes redundant and is removed — one gate per site); `vm/gc/roots.rs:440` (its arm in `mark_roots` `:266`). The third site that names it, `vm/lifecycle.rs:354`, is already inside `Vm::unbind`'s engine block (`:90`). | `default` clippy (N3 gone); `vm/tests/tests_shadow_dom/events.rs:577` (`unbind_clears_pending_notify_mutation_observers_microtask`) under `--all-features` |
| **A1-9** | N4, N5. `create_illegal_constructor_function` (`vm/shape_ops.rs:322`) and `native_illegal_constructor_unreachable` (`vm/value.rs:749`) build Web IDL interface objects that have no constructor operation (Web IDL §3.7.1). Every caller is in `vm/host/` (`git grep -n -e create_illegal_constructor_function -e native_illegal_constructor_unreachable -- crates/script/elidex-js/src`). The doc at `vm/shape_ops.rs:320-321` says "Not engine-gated (sibling parity with [`Self::create_constructor_only_function`])", which the gate makes false. | (b). The sibling `create_constructor_only_function` (`:299`) has a core reader and stays ungated, so the parity the doc claims never held in `default`. | `vm/shape_ops.rs:322` and `vm/value.rs:749`, each gated; `vm/shape_ops.rs:320-321` rewritten to state the gate and why. `CallShape::IllegalConstructor` and the admission gate stay ungated: no `default` diagnostic names them. | `default` clippy (2 gone); `.claude/tools/native-ctor-guard-trip-wire.sh` unaffected (it matches guard idioms, not these items); illegal-constructor tests under `--all-features` |

**Layering check** (U0 hand-off), one line per fix:
- A1-1, A1-2, A1-3: (b). No relocation. They are not calls into `vm/host/`.
- A1-4: (b). Not an algorithm. It reaches `elidex_api_observers` and `HostData`, not `vm/host/`.
- A1-6: (b). It removes code and adds none.
- A1-7: (b). Algorithms that stay in core, read by `vm/host/` (the outbound direction).
- A1-8: (b). Its drain arm calls `vm/host/html_slot_proto.rs` from core, a call already engine-gated at the base
  (`:339`). PR-A1 moves the gate up one level and adds no call.
- A1-9: (b). Web IDL helpers in core, read by `vm/host/`.
- No fix relocates an algorithm, so no engine-independent API needed checking (PM 578 (iii)).

**The membership test.** The slot `#11-vm-inbound-host-call-membership-test` is **undecided**
(`docs/plans/2026-07-vm-p4-slot-ledger.md:311`, "has **no membership test**"). Per the hand-off (PM 585, PM 588),
this plan raised it to PM and does not wait for it (§7.2). For each site, here is whether the slot's scope (core
`vm/` → `vm/host/` calls) reaches it, and the class:
- Reached: only A1-8's drain arm. Provisionally (b), PM 649. Re-examine A1-8 when the slot is ruled. (b) because
  the call already exists and is already gated.
- Not reached: A1-1, A1-2, A1-3, A1-4, A1-6, A1-7, A1-9. These are imports, parameters, a `HostData` / external
  crate cleanup, a dead branch, and core items called *from* `vm/host/`.

Under design invariant I-5's own boundary ("the **`engine` feature gate + host-binding dependency**, not directory
name", `docs/plans/2026-07-vm-p4-design-invariants.md:102-104`), every PR-A1 gate moves code that is already
engine-bound into the engine-gated surface. None adds host knowledge to core.

### §2.3 The unbind order contract (PM 640 condition)

**PR-A1 does not change the order contract.** The PM 640 condition applies only if the plan changes it, but the
three parts are stated here so the claim can be checked.

- **Before** (`8344af2b`): in `Vm::unbind` (`vm/lifecycle.rs:15`), when `hd.is_bound()`, the following run first:
  `clear_mutation_dispatcher()` (`:34`), `clear_all_transient_observers(hd.dom())` (`:45`), the per-bind field
  clears (`:46-61`) and the `strong_count == 1` assertion (`:62-66`). Then `hd.unbind()` (`:68`) zeroes `dom_ptr`.
  The comment at `:17-22` states this order.
- **After**: the same statements, in the same order, at the same place. The only change is a
  `#[cfg(feature = "engine")]` attribute on the `if` statement at `:30`. In the `engine` configuration the
  compiled sequence is identical. In `default` the block does not exist, and there `is_bound()` was constantly
  `false` and no `bind` exists, so the block never ran.
- **Why it is safe**: the attribute removes code only from a configuration in which that code could not run, and
  it reorders nothing in the configuration in which it does run. The engine rows' clippy and rustdoc output is
  unchanged from the base (§5.2).
- **`#11-bound-dispatch-soundness-by-contract`**: its premise includes this order. The VM's `dom_ptr` and the live
  `&mut EcsDom` coexist for the whole bound bracket, and `hd.unbind()` ends it. PR-A1 leaves that premise as written: no order changes, no
  new reader of `dom_ptr`, and no change to `HostData::bind` / `unbind`, `Vm::bind` or `with_bound`.

## §3. Spec coverage map

PR-A1 is a build-shape PR. It changes which configuration compiles an item, and it changes no algorithm in any
configuration that already compiled. So the spec rows are few, and they say where each gated item's behaviour is
defined, not a step that PR-A1 implements. Row 1 is the reason the decision rests on elidex's principles (§2.1
reason 1). The other rows name the spec of each gated item's subject, to show that each gated item belongs to a
host facility and not to the language core. **No row is fully enumerated (✗)**: PR-A1 implements no step.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §4.2 Hosts and Implementations | #sec-hosts-and-implementations | host-defined facilities come from the host; silent on a host-less build | §2.1 reason 1 (whole PR) | ✗ | no |
| ECMA-262 §25.5 The JSON Object | #sec-json-object | `JSON.parse` / `JSON.stringify` stay in every configuration (`native_json_parse` / `native_json_stringify` ungated) | A1-7 `vm/natives_json/mod.rs:13-16` | ✗ | no |
| WHATWG HTML §2.7.4 StructuredSerializeInternal | #structuredserializeinternal | the JSON shortcut helpers serve the host's structured clone, so they exist only with the host | A1-7 `vm/natives_json/stringify.rs:423`, `vm/natives_json/parse.rs:501` | ✗ | no |
| WHATWG DOM §4.3 Mutation observers | #notify-mutation-observers | the notify microtask is queued only by the DOM host | A1-8 `vm/natives_promise.rs:59`, `:333`; `vm/gc/roots.rs:440` | ✗ | no |
| WHATWG DOM §4.3 Mutation observers | #transient-registered-observer | transient registrations exist only in a bound world, so the unbind scrub is engine-only | A1-4 `vm/lifecycle.rs:45` | ✗ | no |
| Web IDL §3.7.1 Interface object | #interface-object | interface objects are host-installed; the illegal-constructor helpers serve only them | A1-9 `vm/shape_ops.rs:322`, `vm/value.rs:749` | ✗ | no |

**Breadth**: K=4 sources, M=6 rows (`preflight.py`, §5.5). K=4 puts the plan at SPLIT-RECOMMENDED. That is not a
reason to split this PR. The four specs label the subjects of items that PR-A1 gates, and PR-A1 implements no step of
any of them. The unit is one row turning green. A slice would land with the row still red, so it would gate nothing on its
own. The section numbers come from `.claude/tools/webref`:
- `heading ecma262 4.2`, `heading ecma262 25.5`, `heading webidl 3.7.1`;
- `dfn html StructuredSerializeInternal` → §2.7.4;
- `dfn dom "notify mutation observers"` and `dfn dom "transient registered observer"` → §4.3.

### §3.1 User-input touch audit

No row has a user-input flow. A1-7 gates two helpers, and the input path of `JSON.parse` (`native_json_parse`) is
unchanged in every configuration.

## §4. Coupled invariants (edge-dense: cfg gating × Layering × unbind order × GC rooting × HostData decomposition)

The axes:
- **G**, cfg gating: each branch names only items its configuration compiles. No suppression.
- **L**, Layering: I-5, and the `vm/host/` mandate.
- **O**, unbind order: §2.3.
- **R**, GC rooting: the roots and marks agree with what exists in each configuration.
- **H**, `HostData` decomposition: the stub's API is the configuration-total interface, and nothing moves.

Two more axes from U0:
- **M**, rows: `default` / `app` / `browser`.
- **S**, 1000-line: the touch-time split rule.

| Pair | Intersection, and how PR-A1 meets it |
|---|---|
| G×L | Gating marks engine-bound code as engine-bound by I-5's own definition (the gate is the boundary). No gate adds host knowledge to core, and no fix puts code into `vm/host/` (contract 1). A1-8's existing core→host call is the one site the membership test would reach (§2.2). |
| G×O | The `cfg` sits on the whole `if hd.is_bound()` statement, not on parts of it. Partial gating (for example the field clears without the bridge clear) could split the sequence the `strong_count` assertion spans. Unit = the whole statement. |
| G×R | A1-8 gates the variant together with its `mark_roots` arm (`vm/gc/roots.rs:440`), so the match stays exhaustive in both configurations. A1-6 removes a value that rooting never read in `default`. `GcRoots.proto_roots`' 176 `None` fills (U0 §8.2) are not touched: their alignment is not a leak. |
| G×H | The stub is unchanged (`vm/host_data/mod.rs:680-704`). The gate stops core code from reaching past the stub's four items, so the stub stays what U0 §8.2 calls it: a no-state iterator provider. |
| L×O | The order lives in `Vm::unbind`, which is not in `vm/host/` (contract 1). It stays there, and the relocation alternative is rejected (§2.1 reason 3). |
| L×R | `collect_garbage` and `mark_roots` are core GC. Their engine-only parts were already gated per field. A1-6 and A1-8 extend the same per-site gating and add no host call to GC. |
| L×H | `HostData` stays the engine-bound per-VM side store of CLAUDE.md's exception (a). No `HostData` state moves into a component or out of `HostData`. |
| O×R | `Vm::unbind`'s engine block strips queued `NotifyMutationObservers` tasks (`vm/lifecycle.rs:350-355`), and `mark_roots` marks the microtask queue. Both name the variant only under `engine` after A1-8, so in `default` neither can disagree with the other. |
| O×H | The order contract spans `Vm` (the bridge clear and field clears) and `HostData::unbind` (`dom_ptr` zeroing). PR-A1 keeps that two-home split as it is (contract 2), and consolidating it is the decomposition slot's job. |
| R×H | The stub's `gc_root_object_ids` (an empty iterator) is the only `HostData` rooting `default` sees, and it is unchanged. |
| M×G | Every fix is attribute-only in the engine rows, apart from `Vm::new`'s statement form (A1-2), the two doc sentences (A1-3, A1-9) and the redundant inner gate (A1-8). Measured: app and browser clippy rc 0 before and after, and their rustdoc warning sets are identical by (file, message) (194 / 193; §5.2). |
| M×S | `vm/gc/collect.rs` (2078) is the only touched file over 1000 lines. Its split is umbrella 7b's (§1.2), and PR-A1 shrinks it. The other touched files are under 1000 lines (§5.1). |

**`#11-host-data-full-decomposition` (contract 3).** **PR-A1 moves nothing.** No `HostData` field, method, `impl`
or stub item changes place or shape. The only `HostData`-related change is the `cfg` on A1-4's statement in
`Vm::unbind`, and it does not change which function clears which `HostData` field. Record for the slot, via PM: "PR-A1
moved nothing. The unbind cleanup is still 2 homes (`Vm::unbind`, `HostData::unbind`) carrying 4 concerns, and
its engine-only block is now `#[cfg(feature = "engine")]` at `vm/lifecycle.rs:30`." The slot still has no Owner
(U0 §8; ledger), so the record goes to a facet on the PM-side ledger, written by the PMO (PM 649).

## §5. Tests and measurements

### §5.1 Commands (re-runnable)

All runs use `env -u RUSTUP_TOOLCHAIN CARGO_TARGET_DIR=<scratch>/pra1-target`, with no pipes, and each logs to a
file.

- **Rows.** `default` = `-p elidex-js`. `app` = `-p elidex-js --features engine`. `browser` =
  `-p elidex-js --features engine,compat-webapi`.
- **Compile columns.**
  - clippy: `cargo clippy <row> -- -D warnings`;
  - rustdoc: `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items <row>` (the U0 column).
  - To list the warnings, run the same command without `-D warnings`.
- **Default lib tests compile.** `cargo test -p elidex-js --lib --no-run`.
- **Browser, all targets.** `cargo clippy -p elidex-js --all-targets --features engine,compat-webapi -- -D warnings`.
- **File sizes.** `wc -l` on the touched files: `vm/init.rs` 860, `vm/lifecycle.rs` 668, `vm/gc/collect.rs` 2078,
  `vm/gc/roots.rs`, `vm/natives_json/mod.rs` 63, `vm/natives_json/parse.rs`, `vm/natives_json/stringify.rs`,
  `vm/natives_promise.rs`, `vm/shape_ops.rs`, `vm/value.rs`. All but `collect.rs` are under 1000 lines.
  `git ls-files crates/script/elidex-js/src/vm | xargs wc -l` gives the rest.

### §5.2 Results at `8344af2b`, and with the fix table applied (macOS)

The fix table was applied as a scratch probe in this worktree and then reverted. `git checkout -- crates`, then
`git status --porcelain` → empty. Results:

| Row / column | Base `8344af2b` | With A1-1…A1-9 |
|---|---|---|
| `default` clippy | rc 101, 17 errors (§1.3 D1–D17) | rc 0 |
| `default` rustdoc | rc 101 at E0432, no output | builds. 49 distinct (file, line, message) warnings (§5.3) |
| `default` `test --lib --no-run` | not reached | rc 0 |
| `app` clippy | rc 0 | rc 0 |
| `browser` clippy | rc 0 | rc 0 |
| `browser` clippy `--all-targets` | — | rc 0 |
| `app` rustdoc (warnings) | 194 | 194, the same set by (file, message) |
| `browser` rustdoc (warnings) | 193 | 193, the same set by (file, message) |

Intermediate probes, which give each class its control:
- With D1–D17 fixed and nothing else, the `default` row reports exactly N1–N6.
- With N1–N6 fixed as well, it reports N7.
- With N7 fixed, it reports nothing.

So reverting any one fix brings back its own diagnostic. In the PR, the guard for every fix is the `default` row's
clippy column.

**Three OS** (U0 "pre-matrix three-OS evidence", PM 558 (1)):
- A scratch commit on PR-A1's branch adds the three rows × two compile columns (and the `default` row's
  `test --lib --no-run`) as a temporary step of the `check` job. A later commit removes it.
- The PR body records the run id and that head's sha.
- At merge, PM checks (i)–(iii) of U0 §6.
- The touched files contain no `cfg(target_os)` / `cfg(unix)` / `cfg(windows)` site (`/usr/bin/grep -c` on each →
  0). Their per-OS results can differ only through code they do not touch, and the three-OS run is what covers that.
- Engine behaviour (A1-4, A1-7, A1-8, A1-9) on three OS is the `check` job's `--all-features` test run, which the
  PR runs anyway.

### §5.3 The `default` row's rustdoc warnings (handed to PR-D…)

There are 49 distinct (file, line, message) warnings. 24 also occur in the `app` or `browser` row at the same code
and are already PR-D… population. The other **25 are the `default` row's own**, all `unresolved link` warnings in
24 doc lines over 11 files (verified 2026-11-11: the set difference of the three rows' warning lists, §5.1).

**Re-measured at `bea9ef19`** (PM 649; current main, including #622), with the fix table applied by the same probe
(apply, measure, `git checkout -- crates`, `git status --porcelain` → empty): the same 49, and the same 25 own
warnings, line for line. #622 removed the 4 `vm/host/mod.rs:230` warnings, which belong to the `app` / `browser`
population, not to these 25; the 4 at `vm/host/mod.rs:53-68` remain.

They are listed here and, per U0 §6, go in PR-A1's body (§5.6) and are handed to PR-D… by owner (PM 561). Owners
(PM 649): every site outside `vm/host/` is VM P4; the sites in `vm/host/` are L3. PR-A1 does not fix them. They are
added to VM P4's PR-D… (due 12-04) after PR-A1 lands. Line numbers are with the fix
table applied. They differ from `8344af2b` only in `vm/init.rs` (+2 lines above `:26`, +1 above `:162`).

| File | Lines (message: `unresolved link to …`) | Count | Owner |
|---|---|---|---|
| `vm/coerce.rs` | `:376` `enforce_range_unsigned_short` | 1 | VM P4 |
| `vm/coerce_format.rs` | `:18` `super::host::dataset::collect_keys` | 1 | VM P4 |
| `vm/gc/mod.rs` | `:23` `keepalive` | 1 | VM P4 |
| `vm/gc/roots.rs` | `:46` `super::super::VmInner::subclass_array_ctors` | 1 | VM P4 |
| `vm/host/mod.rs` | `:53` `node_proto`, `:59` `node_methods_extras`, `:62` `element_proto`, `:68` `dom_bridge` | 4 | L3 DOM/form |
| `vm/init.rs` | `:26` `EngineMode::BrowserCompat`, `:162` `elidex_plugin::SpecLevelPolicy` | 2 | VM P4 |
| `vm/inner.rs` | `:14` `super::VmInner::remove_listener_and_prune_back_ref` | 1 | VM P4 |
| `vm/natives_json/stringify.rs` | `:3` and `:36` `stringify_for_structured_shortcut` | 2 | VM P4 |
| `vm/ops_element.rs` | `:25` `…::write_element_raw`, `:26` `…::read_element_raw`, `:36` `classify_typed_array_number_key`, `:37` `classify_typed_array_string_key` | 4 | VM P4 |
| `vm/shape_ops.rs` | `:495` `Self::install_bound_accessor_pair` | 1 | VM P4 |
| `vm/temp_root.rs` | `:15` (2: `VmInner::push_temp_root`, `VmTempRoot`), `:131`, `:134`, `:151`, `:154`, `:170` | 7 | VM P4 |

Totals: 25 warnings over 11 files (verified 2026-11-11 at `8344af2b` and again at `bea9ef19`, the table above).
Owners: VM P4 21, L3 4 (PM 649).

**The window.** From PR-A1's merge until PR-D… lands, the `default` row's rustdoc column stays red (these 25).
CI does not go red: no CI job runs the `default` row before U0 PR-M1 (§5.4).

**Provenance.**
- 23 of the 25 are inherent to the row: the link target is compiled out of `default` at the base too.
- 2 are created by PR-A1's own gate: `vm/natives_json/stringify.rs:3`, `:36` link to
  `stringify_for_structured_shortcut`, which A1-7 gates.

PR-A1 still hands these two to PR-D… and does not fix them itself. All 25 have one shape: an ungated doc comment
that links to an item compiled out of `default`. The form that fixes that shape, for example a plain code span
instead of a link, or an engine-only doc attribute, is one decision for the whole population. If PR-A1 picked a form
for those two, the program would have two forms (One issue, one way). PR-A1 fixes only doc *statements* that its own
change makes false (A1-3, A1-9). Those are content, not link form.

### §5.4 Keeping the row green: what PR-A1 relies on from PR-M1

PR-A1 adds no trip-wire and no CI step, apart from the temporary scratch step of §5.2, which does not merge. The
property "the `default` row is green" gets exactly one mechanism, PR-M1's `feature-profiles` job, which runs every
row in both compile columns on three OS (U0 §6 PR-M1). A temporary check that PR-M1 then retires would be a second
mechanism for the same property.

The cost is a window: from PR-A1's landing to PR-M1's, no CI runs the row, so an `elidex-js` PR can turn it red
again unseen. U0 already holds that case. PR-M1's acceptance is every row green on its own three-OS run, so a
regression in the window shows up there as a red row and goes to PM under the C0 rule (U0 §4.4 steps 2–3). It does
not land over the row. What PR-A1 relies on from PR-M1:
1. the `default` row is a declared row in both compile columns;
2. the rustdoc column runs with `--document-private-items`;
3. it runs on three OS;
4. the deriver has no mode that skips a row (U0 PR-M1 acceptance).

### §5.5 Plan-stage checks

- `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-elidex-js-default-row-pra1.md` →
  rc 0, no hard-fail; K=4, M=6, SPLIT-RECOMMENDED (answered under §3's breadth note); grep-pass 0 hard, 0 soft.
- `python3 .claude/tools/ecs_mission_ratchet.py` → 0 new.

### §5.6 PR body requirements

The PR body carries:
- the seven later-phase diagnostics N1–N7, each named (§0.3 item 3, §1.3);
- the three-OS run id and that head's sha (§5.2);
- §5.3's 25 warnings, by owner;
- the line: "default 行の rustdoc は 25 件 (→ PR-D…、PM 649)".

## §6. Observation for the split umbrella (F1-c)

This section is a record only, not a plan. It is input for `#11-elidex-js-no-default-features-support`, which is
re-evaluated on 2026-11-30. These are the core items PR-A1 marks as host-only. A crate split decides where each one
lives:
- `Vm::unbind`'s bound-world cleanup (`vm/lifecycle.rs:30-67`), the inverse of the engine-only `Vm::bind`;
- `new_with_scope`'s `global_scope_kind` / `engine_mode` parameters;
- the JSON helpers for structured serialization;
- `Microtask::NotifyMutationObservers` in the core microtask enum. It is a host microtask with no host-microtask
  seam;
- the Web IDL illegal-constructor helpers in `vm/shape_ops.rs` / `vm/value.rs`, beside
  `CallShape::IllegalConstructor`, which stays in core.

U0 §8.2's counts move as follows. `cfg(not(feature = "engine"))` sites:
- `vm/gc/collect.rs`: −1 (180 → 179 at `8344af2b`; U0 counted 179 at `69fbe61f`; A1-6);
- `vm/init.rs`: +1 (A1-2's `Vm::new`);
- crate total: unchanged, 188 at `8344af2b` by U0's command (`git grep -n 'not(feature = "engine")' -- crates | wc -l`; U0 counted 187 at `69fbe61f`).

U0 §8.2 is stated "at `69fbe61f`" and is not edited.

## §7. Returned to PM

### §7.1 Kind (c) sites: none

U0 hand-off, quoted: "**(c) Non-optional dependency**: a shape change, decided neither in PR-A1 nor in this
umbrella. PR-A1's plan names the site and returns it to PM (PM 520); PM's ruling is applied in PR-A1 if PM names it,
otherwise in PR-A1′."

Each candidate the brief and U0 name has a kind (b) fix, so none has (c) as its *only* fix:

| Candidate | Why not (c) |
|---|---|
| `vm/init.rs:11` `elidex_plugin` (U0's "known (c) candidate") | Every reader of `EngineMode` is engine-only (A1-1, A1-2, A1-3). `elidex-plugin` stays optional (PM 584). |
| `vm/lifecycle.rs:45` `elidex_api_observers` | The call is part of a block that cannot run without `engine` (A1-4). |
| `vm/gc/collect.rs:47` `elidex_ecs` | The branch has no reader in `default`, so it is removed (A1-6). |

So **PR-A1′ is not needed** unless PM rules otherwise.

### §7.2 Items for PM (items 1–3 ruled by PM 649)

1. **`#11-vm-inbound-host-call-membership-test`** (raised per the Layering check, PM 585, PM 588): Provisionally (b),
   PM 649. Re-examine A1-8 when the slot is ruled. A1-8's drain arm is the one site the slot's scope reaches; it does
   not reach A1-1, A1-2, A1-3, A1-4, A1-6, A1-7 or A1-9 ((b) for each). PR-A1 does not wait.
2. **`#11-host-data-full-decomposition` record** (contract 3): "PR-A1 moved nothing" (§4). The slot has no Owner;
   the record goes to a facet on the PM-side ledger, written by the PMO (PM 649).
3. **PR-D… for the `default` row**: the 25 warnings of §5.3, owners VM P4 21 and L3 4 (PM 649). They are added to
   VM P4's PR-D… (due 12-04) after PR-A1 lands. The window is stated in §5.3.
4. **Order contract**: unchanged (§2.3). The PM 640 condition does not apply, and the bound-dispatch slot's premise
   is untouched.

## §8. Touched functions (PM 626 overlap list; `file:line` at `8344af2b`)

| File | Item (line) | Fix |
|---|---|---|
| `vm/init.rs` | `use elidex_plugin::EngineMode` (`:11`) | A1-1 |
| `vm/init.rs` | `Vm::new` (`:30`) | A1-2 |
| `vm/init.rs` | `Vm::new_with_mode` (`:47-50`) | A1-3 |
| `vm/init.rs` | `new_with_scope` (doc `:158-160`, attribute `:162`, signature `:163`) | A1-2, A1-3 |
| `vm/lifecycle.rs` | `Vm::unbind` (`:15`; the statement at `:30`) | A1-4 |
| `vm/gc/collect.rs` | `VmInner::collect_garbage` (`:25`; lines `:45-49` removed) | A1-6 |
| `vm/gc/roots.rs` | `mark_roots` (`:266`; arm `:440`) | A1-8 |
| `vm/natives_json/mod.rs` | the re-exports (`:13-16`; no function) | A1-7 |
| `vm/natives_json/parse.rs` | import `:13`; `parse_json_str` (`:501`) | A1-7 |
| `vm/natives_json/stringify.rs` | `stringify_for_structured_shortcut` (`:423`) | A1-7 |
| `vm/natives_promise.rs` | `enum Microtask` (`:31`; variant `:59`); `drain_queue_pass` (`:311`; arm `:333`, inner gate `:339`) | A1-8 |
| `vm/shape_ops.rs` | `create_illegal_constructor_function` (doc `:320-321`, fn `:322`) | A1-9 |
| `vm/value.rs` | `native_illegal_constructor_unreachable` (`:749`) | A1-9 |

**Not touched**:
- `HostData::unbind` (`vm/host_data/engine_feature/bind.rs:118`) and `HostData::bind` (`:80`);
- the stub (`vm/host_data/mod.rs:680-704`);
- `Vm::bind` (`vm/vm_api.rs:203`);
- `Vm::teardown_document` and the engine block of `Vm::unbind` (`vm/lifecycle.rs:90-…`);
- `NativeContext` `host` / `host_opt` / `host_if_bound`;
- every interpreter entry, `ObjectKind::HostObject`, and every file under `vm/host/`.

## §9. Landing list (PMO, via PM)

1. `#11-elidex-js-default-features-build-broken`: no close at PR-A1 (U0 §8: it closes when PR-M1 lands with every
   row-fixing PR landed). Note on the slot: "PR-A1 landed `<sha>`. The `default` row is clippy-clean on three OS
   (run `<id>`)."
2. `#11-host-data-full-decomposition`: the "moved nothing" record of §4, to a facet on the PM-side ledger, written
   by the PMO (PM 649).
3. `#11-vm-typed-array-family-layering-and-gate` (VM P4): its "Until then" premise ends when PR-A1 lands (U0 §8.1).
   PM routes it.
4. PR-D… (`default` row share): §5.3's 25 warnings (VM P4 21, L3 4), added to VM P4's PR-D… (due 12-04) when
   PR-A1 lands (PM 649).
5. `#11-elidex-js-no-default-features-support`: a pointer to §6 of this memo, as the split's input.
6. `#11-vm-inbound-host-call-membership-test`: §7.2 item 1's site list, as input for whenever the slot is decided.

# PR-A1 — make `elidex-js`'s `default` row compile (U0 PR-A1, per-PR plan-memo)

**Program**: U0, the per-feature-profile build matrix (`docs/plans/2026-11-per-profile-build-matrix-umbrella.md`,
ratified PM 597). **This PR**: U0 §6's PR-A1 row and its "PR-A1 hand-off" block. **Owner**: VM P4 lane (PM 520 (1)).
**Shape**: F1-b, the interim (U0 §5; PM 520): `default = []` stays, every fix is a `cfg` fix, and `elidex-plugin`
stays optional (PM 584). **Order and date** (PM 605, 640, 642, 650, 651):
- prereq: `vm/value.rs` split (PM 651, PM 586 root 4 seam), lands first (VM P4, due 11-18) → **PR-A1, due 11-21**
  (PM 651) → default 行 rustdoc の PR (11-25, PM 650) → U0 PR-M1 (11-27). PR-A1 is built on top of the split.
- PM 605 placed PR-A1 in VM P4's queue after P0a-1's ratify and before Slice F (its date then, 11-27, is replaced by
  PM 651's 11-21).
- PM 640: L3's brand-check Rev 5 waits for PR-A1 only if Rev 5's files include `Vm::unbind` / `HostData::unbind`.
  PM 642: Rev 5's PR-1 list includes neither, so Rev 5 does not wait.

**Baseline**: origin/main `8344af2b`. Every `file:line` below is at `8344af2b` (the touched files are byte-identical
at `bea9ef19`: `git diff --stat 8344af2b bea9ef19 --` on them → empty), except `vm/value.rs`, which A1-9 touches on
the post-split file and cites by function name. Paths are relative to `crates/script/elidex-js/src/` unless they
start with `crates/` or `docs/`.

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
- **Rev 1.2 (2026-11-11), plan-review R1 (5 axes, `96c25a53`) + PM 650 + PM 651**: the 25 (now 26) `default`-row
  rustdoc warnings leave PR-D… for one separate PR, the default 行 rustdoc の PR (VM P4, L3's 4 included; after PR-A1,
  before PR-M1; due 11-25; PM 650); the suppression population re-derived by property, adding A1-10
  (`vm/natives_timer.rs`, §2.1 reason 5); `vm/value.rs` split as a standalone prereq before PR-A1 (PM 651); the
  three-OS rustdoc evidence designed (§5.2); PR-A1 due 11-21 (PM 651); the T axis added to §4; minor fixes.

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

The 17 diagnostics of §0.2, the 7 that surface behind them (§1.3), the no-engine suppressions found by property
(§2.1 reason 5: A1-2's and A1-10's), and the doc statements that these fixes make false (§2, A1-3, A1-9 and
A1-10). Every fix is a `cfg` fix (kind (b)). The full list is §2's fix table.

### §1.2 Out of scope

- **The crate split (F1-c).** It is a separate program, owned by VM P4 and not planned here (U0 §5). §6 records,
  for that program, the items this PR gates. They are core items that exist only for the host.
- **`HostData`'s decomposition** (`#11-host-data-full-decomposition`): PR-A1 moves nothing (§4).
- **Rustdoc warnings in the `default` row.** They are listed (§5.3) and go to the default 行 rustdoc の PR (VM P4,
  L3's 4 included; after PR-A1, before PR-M1; due 11-25; PM 650), not to PR-D…. PR-A1 fixes doc *statements* that
  its own code change makes false. It does not fix link forms. §5.3 gives the reason.
- **The `default` row's tests** (PR-T, PR-T0…). PR-A1 checks only that the row's lib test target still compiles
  (§5.1). This is not PR-T's census.
- **CI**. The row's gate is PR-M1's `feature-profiles` job. PR-A1 adds no trip-wire (§5.4).
- **`vm/gc/collect.rs`'s split.** That decision is already recorded: the file belongs to umbrella 7b
  (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:84`, "`vm/gc/collect.rs` 2074 (**umbrella 7b**, through the
  children it mints for the work)"). PR-A1 takes that decision and makes no second one. Its touch removes 5 lines
  (2078 → 2073), so it does not grow the file.
- **`vm/value.rs`'s split.** prereq: value.rs split (PM 651, PM 586 root 4 seam), lands first. It is a standalone
  pure-move PR, owned by VM P4, due 11-18, and is not planned here. Its seam is the one already fixed: the PR-0
  section of `git show dada49f7:docs/plans/2026-11-vm-host-receiver-brand-check.md` (§5.1 table, `vm/value.rs` row)
  moves the frame state (`Upvalue` / `UpvalueState`, `FrameKind`, `CallFrame` and its `impl`, `HandlerEntry`) to
  `vm/frame.rs`, leaves `NativeContext` where it is, and brings `value.rs` to about 967 lines. PM 651 rules that the
  split record's "umbrella 3" entry for `value.rs` (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:82`,
  "`vm/value.rs` 1187 (**umbrella 3**, …)") is not taken. PR-A1 designs no seam. A1-9's touch
  (`native_illegal_constructor_unreachable`, which the seam does not move) lands on the post-split `value.rs`.

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

Behind a suppression, a diagnostic does not appear at all, so the 24 do not include the dead code that an `allow`
hides. Those sites come from §2.1 reason 5's property grep instead. A1-10 is the one such site found, and removing
its `allow` brings up diagnostics N8–N10, which are listed here so the PR body names them too:

| # | Diagnostic (`default` row, with the `allow` at `vm/natives_timer.rs:242` removed) | Group |
|---|---|---|
| N8 | `dead_code`: methods `next_timer_deadline`, `drain_timer_fire_checkpoint`, `drain_timers`, `fire_current_timer` never used — `vm/natives_timer.rs:259` | L5 timer drive |
| N9 | `dead_code`: `TimerEntry`'s field `repeat` never read — `vm/natives_timer.rs:26` | L5 |
| N10 | unused import `super::VmInner` — `vm/natives_timer.rs:12`, and `dead_code`: constant `MIN_INTERVAL_REPEAT_MS` never used — `:66` (each appears once the previous gate is in place) | L5 |

## §2. Design

### §2.1 The leak classes and the one mechanism decision

The 24 diagnostics, and N8–N10 behind A1-10's suppression, fall into two classes.

- **Class I: core code names an item that only `engine` compiles.** D1–D15. The named item is either from an
  optional dependency (`elidex_plugin`, `elidex_api_observers`, `elidex_ecs`) or engine-only `HostData` API
  (`dom()` and per-bind fields that the stub does not have).
- **Class II: a core item has readers only in engine-gated (or, for N8–N10, engine- or test-gated) code.** D16–D17,
  N1–N5, N7, N8–N10, and N6 (a parameter that is
  read only under `engine`). In the `default` configuration such an item is dead code. CLAUDE.md's rule for dead
  code is "接続するか削除": either it gets a reader in that configuration or it is removed from that configuration.

**The mechanism decision.** The question is: should the default `HostData` stub be met by gating at the use site,
or by an engine/non-engine seam in the layer that owns the state?

The decision: **gate at the use site (kind (b)). The stub's API is the configuration-total interface, and it does
not grow.** The stub (`vm/host_data/mod.rs:680-704`) offers `new`, `unbind`, `is_bound` and `gc_root_object_ids`,
and those are the only `HostData` items that ungated core code may name. Everything else on `HostData` is
engine-only, and only engine-gated code names it. The same rule applies to Class II: a core item whose readers are
all engine-gated becomes engine-gated itself, together with every site in that configuration that names it.
Silencing the lint with `allow(dead_code)` or `allow(unused_*)` is not used, and PR-A1 removes the two existing
suppressions whose reason is the no-engine build (A1-2, A1-10; reason 5).

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
   - a suppression whose reason is the no-engine build.
   PR-A1 uses the gate everywhere and removes every such suppression, which leaves one way. The population is
   defined by that property, not by diagnostics, because a suppressed site emits none. It was derived like this:
   - Grep: `git grep -n -B4 -e 'allow(dead_code' -e 'allow(unused' -e 'cfg_attr(' -- crates/script/elidex-js/src`
     gives 11 sites. A control grep, `git grep -n -e 'expect(dead_code' -e 'expect(unused' -- crates/script/elidex-js/src`,
     gives 0.
   - Each site is classified by its comment, its module's gate, and a measurement. The measurement is a scratch
     probe on top of A1-1…A1-9: remove the `dead_code` part of the `allow`, run `default` and `app` clippy, then
     `git checkout -- crates` and check that `git status --porcelain` is empty.

   | Site | Reason (comment / measurement) | In the population? |
   |---|---|---|
   | `vm/init.rs:162` `cfg_attr(not(feature = "engine"), allow(unused_variables))` | no-engine by its own cfg | yes: A1-2 |
   | `vm/natives_timer.rs:242` `#[allow(dead_code)] impl VmInner` | comment `:239-241`: "allow explicitly for the no-feature lib build". Removed: `default` reports N8–N9, `app` reports nothing | yes: **A1-10** |
   | `vm/well_known.rs:60` `WellKnownStrings` | comment: "interpreter and future built-ins". Removed: `app` reports 81 dead fields, `default` 860 | no, mixed: the `allow` is needed in every row (§6, §7.2 item 5) |
   | `vm/well_known.rs:1549` `WellKnownSymbols` | removed: no diagnostic in either row | no: inert in every row |
   | `vm/pools.rs:71` `StringPool`'s `len` | removed (`dead_code` only): no diagnostic in either row | no: inert in every row |
   | `vm/host/event_shapes.rs:186`, `:219` | module is `#![cfg(feature = "engine")]` (`:44`). Removed: `app` reports both fields | no: engine-only module |
   | `vm/host/fetch/dispatch.rs:572`; `vm/host/readable_stream/mod.rs:83`, `:105`, `:173` | modules engine-gated (`vm/host/mod.rs:152-153`, `:311-312`); comments name later stages | no: not compiled in `default` |

   After A1-2 and A1-10, the grep finds no suppression whose reason is the no-engine build. The one mixed site,
   `well_known.rs:60`, stays. Its `allow` also hides 779 fields in `default` that only engine code reads. Gating
   them field by field would change the generated well-known table, and §7.2 item 5 returns that to PM.
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
- For A1-10, gating the whole timer surface (the globals, the heap and its `VmInner` fields) out of `default`. That
  would decide the host-less row's global surface, and it touches `vm/mod.rs` (2797 lines). It is returned to PM
  (§7.2 item 5 (a)), not taken.

### §2.2 Fix table

Kind key: (a) relocate, (b) gate in place, (c) non-optional dependency (U0 §6 hand-off, quoted in §7). Every fix
below is (b). No fix combines kinds.

| Fix | Leak and the state's owner | Kind and reason | Destination (file, function) | Guard: test or measurement |
|---|---|---|---|---|
| **A1-1** | D1. `use elidex_plugin::EngineMode` is ungated. `EngineMode` is the engine-wide core/compat authority, read only through `spec_level_policy()` under `engine` (`vm/init.rs:177`). Owner: `elidex-plugin` (optional, `Cargo.toml:80`). | (b). Every reader is engine-only. Kind (c) is not needed (§7). | `vm/init.rs:11`: the import gets `#[cfg(feature = "engine")]`. | `default` clippy (D1 gone); app and browser clippy and rustdoc unchanged (§5.2) |
| **A1-2** | N6, and D1's last ungated reader. `new_with_scope` takes `global_scope_kind` and `engine_mode`, and both are stored or read only under `engine` (`vm/init.rs:177`, `:839-844`). In `default` both parameters are dead. Owner: `VmInner` (engine-gated fields). | (b). Each parameter gets `#[cfg(feature = "engine")]` (cfg on a function parameter is stable). The `cfg_attr(not(feature = "engine"), allow(unused_variables))` at `:162` is deleted. | `vm/init.rs:162-163`, `new_with_scope`. `Vm::new` (`:30-32`) passes the two arguments under `engine` and none otherwise: two `cfg`-ed `let vm = …;` statements and a tail `vm`. This form was measured clippy-clean in all three rows. It adds one `cfg(not(feature = "engine"))` site, and A1-6 removes one (§6). | `default` clippy (N6 gone, no `let_and_return`); app and browser clippy rc 0 (§5.2) |
| **A1-3** | Doc and test surface of A1-1/A1-2. `Vm::new_with_mode` (`vm/init.rs:47-50`, `#[cfg(test)]`) takes `EngineMode`, so the `default` row's lib tests would not compile once A1-1 gates the import. The `new_with_scope` doc says "In non-`engine` builds the scope kind / mode are unused … but accepted to keep one shared construction body" (`:158-160`), which A1-2 makes false. | (b). `#[cfg(test)]` becomes `#[cfg(all(test, feature = "engine"))]`. All callers are engine-gated (§0.2). The doc sentence is rewritten to say the parameters exist only under `engine`. | `vm/init.rs:47` (`Vm::new_with_mode`); `vm/init.rs:158-160` (doc of `new_with_scope`) | `cargo test -p elidex-js --lib --no-run` in `default` rc 0 (§5.2) |
| **A1-4** | D2–D14. The `if hd.is_bound() { … }` block of `Vm::unbind` (`vm/lifecycle.rs:30-67`) names `elidex_api_observers` and engine-only `HostData` API. Owners: the `EcsDom` (mutation-dispatcher hook), the bound world (`MutationObservedBy` components), and `HostData` (per-bind fields). The base has 2 homes and 4 concerns (U0 hand-off contract 2). | (b). Mechanism decision §2.1, reasons 2–4. | `vm/lifecycle.rs:30`, `Vm::unbind`. The `if` statement gets `#[cfg(feature = "engine")]` (cfg on a statement). It stays where it is, before `hd.unbind()` at `:68`. Homes stay 2 and concerns stay 4. Nothing moves (§4). | `default` clippy (D2–D14 gone). Engine rows: a positive `cfg` is the identity in `engine`, so the compiled block is unchanged, and `app` / `browser` clippy and rustdoc are unchanged (§5.2). That is the guard. Pre-existing gap: no engine test asserts the `:30` block's clears (the `tests_mutation_observer` lifecycle / transient tests check no panic, bindings and drain only; `vm/tests/tests_shadow_dom/events.rs:577` covers the `:90` block, A1-8's). The U0 hand-off does not require a test for a (b) fix, so PR-A1 adds none. |
| **A1-5** | (none: the order contract). The order contract is A1-4's invariant, not a separate fix. §2.3 states it. | — | — | §2.3 |
| **A1-6** | D15. The `cfg(not(feature = "engine"))` branch of `collect_garbage` (`vm/gc/collect.rs:45-49`) builds an empty `HashMap<elidex_ecs::Entity, HashSet<u64>>`. Its only reader is engine-gated (`vm/gc/roots.rs:206-208`, `vm/gc/collect.rs:1141-1142`). Owner: the bound session's CSSOM (`SessionCore`, engine). | (b), the degenerate form: in the `default` configuration the branch names nothing that is read, so it is removed and the engine branch (`:40-44`) stays as it is. | `vm/gc/collect.rs:45-49` deleted, in `VmInner::collect_garbage` (`:25`). Split decision: §1.2. | `default` clippy (D15 gone, no unused variable) |
| **A1-7** | D16–D17, N1, N2, N7. `parse_json_str` (`vm/natives_json/parse.rs:501`) and `stringify_for_structured_shortcut` (`vm/natives_json/stringify.rs:423`) are JSON helpers in core, and their only readers are in `vm/host/` (§0.2). `stringify_to_string` keeps a core reader (`vm/natives_json/stringify.rs:368`). Owner: the language core's JSON module (ECMA-262 §25.5). Readers: `parse_json_str` (the deserialize side, HTML §2.7.7 StructuredDeserialize) is read by IndexedDB (`vm/host/indexeddb/value.rs:463`), service-worker delivery (`vm/host/service_worker/deliver.rs:359`), the worker scope (`vm/host/worker_scope.rs:277`) and the structured-serialize shortcut (`vm/host/structured_serialize.rs:97`). `stringify_for_structured_shortcut` (the serialize side, HTML §2.7.4) is read by `vm/host/structured_serialize.rs:78` and `vm/host/worker_scope.rs:386`. | (b). The parse helper is a core algorithm used by the host. `stringify_for_structured_shortcut` is a host-semantics mode inside the shared core serializer (the `reject_date` field of `JsonSerializer` `vm/natives_json/stringify.rs:38`, its Date check in `serialize_property` `:61-63` and `stringify_impl`'s `reject_date` parameter `:435` stay ungated), and that is pre-existing (§6). PR-A1 adds no call and moves nothing. In `default` neither has a reader, so both are gated. | `vm/natives_json/mod.rs:13-16`: the re-exports split, with `native_json_parse` / `native_json_stringify` ungated, and `parse_json_str`, `stringify_for_structured_shortcut` and `stringify_to_string` re-exported under `engine`. `vm/natives_json/parse.rs:501` (`parse_json_str`) and its import `:13` (`VmInner`); `vm/natives_json/stringify.rs:423` (`stringify_for_structured_shortcut`) — each gated. | `default` clippy (5 gone); JSON behaviour: `vm/tests/tests_json.rs`, `vm/tests/tests_json_shortcut_date.rs` under `--all-features` |
| **A1-8** | N3. `Microtask::NotifyMutationObservers` (`vm/natives_promise.rs:59`) is the DOM's "notify mutation observers" microtask (WHATWG DOM §4.3). Only `vm/host/` constructs it (`vm/host/html_slot_proto.rs:68`, `vm/host/mutation_observer.rs:464`). Owner: the host's mutation-observer machinery. | (b), on the diff fact: PR-A1 adds no call, no variant and no dependency, and removes the variant from `default`. The existing core→host calls of the drain arm (`vm/host/html_slot_proto.rs`'s two functions, called at `vm/natives_promise.rs:346-347` and `:352`) are already under the engine gate at `:339`. Kind (a) needs an existing seam, and the core `Microtask` enum has no host-microtask extension point. Creating one is the split's decision (§6). Provisionally (b), PM 649. Re-examine A1-8 when the slot is ruled (`#11-vm-inbound-host-call-membership-test`, §2.2 "The membership test"). | `vm/natives_promise.rs:59` (variant, in `enum Microtask` `:31`); `vm/natives_promise.rs:333` (its arm in `drain_queue_pass` `:311`, gated as a whole, so the inner `#[cfg(feature = "engine")]` block at `:339` becomes redundant and is removed — one gate per site); `vm/gc/roots.rs:440` (its arm in `mark_roots` `:266`). The third site that names it, `vm/lifecycle.rs:353`, is already inside `Vm::unbind`'s engine block (`:90`). | `default` clippy (N3 gone); `vm/tests/tests_shadow_dom/events.rs:577` (`unbind_clears_pending_notify_mutation_observers_microtask`) under `--all-features` |
| **A1-9** | N4, N5. `create_illegal_constructor_function` (`vm/shape_ops.rs:322`) and `native_illegal_constructor_unreachable` (`vm/value.rs`, cited by name: the prereq split moves lines, not this function) build Web IDL interface objects that have no constructor operation (Web IDL §3.7.1). Every caller is in `vm/host/` (`git grep -n -e create_illegal_constructor_function -e native_illegal_constructor_unreachable -- crates/script/elidex-js/src`). The doc at `vm/shape_ops.rs:320-321` says "Not engine-gated (sibling parity with [`Self::create_constructor_only_function`])", which the gate makes false. | (b). The sibling `create_constructor_only_function` (`:299`) has a core reader and stays ungated, so the parity the doc claims never held in `default`. | `vm/shape_ops.rs:322` and, on the post-split `vm/value.rs`, `native_illegal_constructor_unreachable`, each gated; `vm/shape_ops.rs:320-321` rewritten to state the gate and why. `CallShape::IllegalConstructor` and the admission gate stay ungated: no `default` diagnostic names them. | `default` clippy (2 gone); `.claude/tools/native-ctor-guard-trip-wire.sh` unaffected (it matches guard idioms, not these items); illegal-constructor tests under `--all-features` |
| **A1-10** | The suppression at `vm/natives_timer.rs:242` (§2.1 reason 5), and N8–N10 behind it. `#[allow(dead_code)] impl VmInner { next_timer_deadline, drain_timer_fire_checkpoint, drain_timers, fire_current_timer }` (`:243-…`). Readers of `drain_timers`: `engine.rs:344`, `vm/sw_thread.rs:474`, `vm/worker_thread.rs:227` (all engine), and `vm/tests/tests_timer.rs` (ungated, so a `default` test reader). `next_timer_deadline`'s only reader is `engine.rs:539`. `TimerEntry`'s `repeat` field (`:26`) is read only by `drain_timers` (`:388`). The constant `MIN_INTERVAL_REPEAT_MS` (`:66`) and `schedule_timer`'s `repeat` parameter (`:102`) feed only that field. Owner: the VM's timer heap (HTML §8.7 timers; the host's event loop drives it). | (b). The gate follows the readers: `cfg(any(test, feature = "engine"))` where `tests_timer` reads, and `cfg(feature = "engine")` where only engine reads. The `allow` is removed. | `vm/natives_timer.rs`: `:242` `allow` → `#[cfg(any(test, feature = "engine"))]` on the `impl`, and `#[cfg(feature = "engine")]` on `next_timer_deadline` (`:259`). `cfg(any(test, feature = "engine"))` goes on the import `:12`, the field `:26`, the constant `:66`, `schedule_timer`'s parameter `:102`, its `let interval_repeat` (`:143`), the struct-literal field `:155`, and the argument at each of the two call sites (`native_set_timeout` `:167`, `native_set_interval` `:176`; cfg on a call argument). The comment `:239-241` is rewritten to state the gate and its readers. That is 10 attributes in one file. In `default` without `test`, `setInterval` and `setTimeout` schedule the same entry, because nothing fires either one (§6, §7.2 item 5). | `default` clippy (N8–N10 gone; rc 0); `default` `test --lib --no-run` rc 0 (`tests_timer` compiles); `app` / `browser` clippy rc 0 and rustdoc sets unchanged (§5.2). Timer behaviour: `vm/tests/tests_timer.rs` under `--all-features` |

**Layering check** (U0 hand-off), one line per fix:
- A1-1, A1-2, A1-3: (b). No relocation. They are not calls into `vm/host/`.
- A1-4: (b). Not an algorithm. It reaches `elidex_api_observers` and `HostData`, not `vm/host/`.
- A1-6: (b). It removes code and adds none.
- A1-7: (b). Algorithms that stay in core, read by `vm/host/` (the outbound direction).
- A1-8: (b). Its drain arm calls `vm/host/html_slot_proto.rs` from core, a call already engine-gated at the base
  (`:339`). PR-A1 moves the gate up one level and adds no call.
- A1-9: (b). Web IDL helpers in core, read by `vm/host/`.
- A1-10: (b). Core timer-heap code, read by `engine.rs`, the worker / service-worker threads and `tests_timer`. It
  adds no call and moves nothing.
- No fix relocates an algorithm, so no engine-independent API needed checking (PM 578 (iii)).

**The membership test.** The slot `#11-vm-inbound-host-call-membership-test` is **undecided**
(`docs/plans/2026-07-vm-p4-slot-ledger.md:311`, "has **no membership test**"). Per the hand-off (PM 585, PM 588),
this plan raised it to PM and does not wait for it (§7.2). Two boundaries are in use, so each site is listed
under both.

Boundary 1 is the slot's own scope: directory, core `vm/` → `vm/host/` calls
(`docs/plans/2026-07-vm-p4-slot-ledger.md:311`).
- Reached: only A1-8's drain arm. Provisionally (b), PM 649. Re-examine A1-8 when the slot is ruled. It is (b) on
  the diff fact (A1-8's row): no call, variant or dependency is added.
- Not reached: A1-1, A1-2, A1-3, A1-4, A1-6, A1-7, A1-9 and A1-10. These are imports, parameters, a `HostData` /
  external-crate cleanup, a dead branch, the timer heap, and core items called *from* `vm/host/`.

Boundary 2 is design invariant I-5's: "the **`engine` feature gate + host-binding dependency**, not directory name"
(`docs/plans/2026-07-vm-p4-design-invariants.md:102-104`).
- Reached under I-5: A1-8's drain arm, and A1-4's block. That block is core code calling into engine-bound state:
  `hd.dom()`, `clear_all_transient_observers` (`elidex_api_observers`), and `HostData`'s engine fields.
- A1-4 adds no call; it only gates a block that already exists. Neither site adds a call or a dependency.
- The slot's membership test is undecided. This plan does not judge whether those calls are permitted. It hands
  both sites to the slot as input (§9 item 6).

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
| WHATWG HTML §2.7.4 StructuredSerializeInternal | #structuredserializeinternal | the serialize-side shortcut helper serves the host's structured clone, so it exists only with the host | A1-7 `vm/natives_json/stringify.rs:423` | ✗ | no |
| WHATWG HTML §2.7.7 StructuredDeserialize | #structureddeserialize | the deserialize-side helper serves the host's readers (IndexedDB, worker / service-worker messages, the shortcut), so it exists only with the host | A1-7 `vm/natives_json/parse.rs:501` | ✗ | no |
| WHATWG HTML §8.7 Timers | #timers | the timer heap is driven by the host's event loop; in `default` without `test` nothing drives it | A1-10 `vm/natives_timer.rs:242`, `:26`, `:102` | ✗ | no |
| WHATWG DOM §4.3 Mutation observers | #notify-mutation-observers | the notify microtask is queued only by the DOM host | A1-8 `vm/natives_promise.rs:59`, `:333`; `vm/gc/roots.rs:440` | ✗ | no |
| WHATWG DOM §4.3 Mutation observers | #transient-registered-observer | transient registrations exist only in a bound world, so the unbind scrub is engine-only | A1-4 `vm/lifecycle.rs:45` | ✗ | no |
| Web IDL §3.7.1 Interface object | #interface-object | interface objects are host-installed; the illegal-constructor helpers serve only them | A1-9 `vm/shape_ops.rs:322`, `vm/value.rs:749` | ✗ | no |

**Breadth**: K=4 sources, M=8 rows (`preflight.py`, §5.5). K=4 puts the plan at SPLIT-RECOMMENDED.
SPLIT-RECOMMENDED (K=4) を PM 650 で単一 PR と裁定. That is not a reason to split this PR. The four specs label the subjects of items that PR-A1 gates, and PR-A1 implements no step of
any of them. The unit is one row turning green. A slice would land with the row still red, so it would gate nothing on its
own. The section numbers come from `.claude/tools/webref`:
- `heading ecma262 4.2`, `heading ecma262 25.5`, `heading webidl 3.7.1`;
- `dfn html StructuredSerializeInternal` → §2.7.4; `dfn html StructuredDeserialize` → §2.7.7;
- `heading html 8.7` → §8.7 Timers (`#timers`);
- `dfn dom "notify mutation observers"` and `dfn dom "transient registered observer"` → §4.3.

### §3.1 User-input touch audit

No row has a user-input flow. A1-7 gates two helpers, and the input path of `JSON.parse` (`native_json_parse`) is
unchanged in every configuration. A1-10 leaves `setTimeout` / `setInterval`'s argument handling (`ToNumber` on the
delay, the callable check) ungated in every configuration. It gates only the interval period, which in `default`
without `test` nothing reads.

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

One more axis, from R1:
- **T**, test target: `cfg(test)` × row. Test-only items and test readers exist per row.

| Pair | Intersection, and how PR-A1 meets it |
|---|---|
| G×L | By the diff: no gate adds a call, a variant or a dependency, and no fix puts code into `vm/host/` (contract 1). Under the slot's directory scope only A1-8's existing core→host call is reached. Under I-5's boundary, A1-4's existing block is reached as well. Both go to the slot as input; PR-A1 rules on neither (§2.2). |
| G×O | The `cfg` sits on the whole `if hd.is_bound()` statement, not on parts of it. Partial gating (for example the field clears without the bridge clear) could split the sequence the `strong_count` assertion spans. Unit = the whole statement. |
| G×R | A1-8 gates the variant together with its `mark_roots` arm (`vm/gc/roots.rs:440`), so the match stays exhaustive in both configurations. A1-6 removes a value that rooting never read in `default`. `GcRoots.proto_roots`' 176 `None` fills (U0 §8.2) are not touched: their alignment is not a leak. |
| G×H | The stub is unchanged (`vm/host_data/mod.rs:680-704`). The gate stops core code from reaching past the stub's four items, so the stub stays what U0 §8.2 calls it: a no-state iterator provider. |
| L×O | The order lives in `Vm::unbind`, which is not in `vm/host/` (contract 1). It stays there, and the relocation alternative is rejected (§2.1 reason 3). |
| L×R | `collect_garbage` and `mark_roots` are core GC. Their engine-only parts were already gated per field. A1-6 and A1-8 extend the same per-site gating and add no host call to GC. |
| L×H | PR-A1 changes no `HostData` field's home; the classification is `#11-host-data-full-decomposition`'s. The per-bind fields A1-4's block clears (`selection_state`, `tree_walker_states`, `live_range_registry`, `node_iterator_states_shared`, `selectionchange_pending`) are not per-VM identity handles. The selection state is a candidate for `#11-browsing-context-state-ecs-components`. |
| O×R | `Vm::unbind`'s engine block strips queued `NotifyMutationObservers` tasks (`vm/lifecycle.rs:350-355`), and `mark_roots` marks the microtask queue. Both name the variant only under `engine` after A1-8, so in `default` neither can disagree with the other. |
| O×H | The order contract spans `Vm` (the bridge clear and field clears) and `HostData::unbind` (`dom_ptr` zeroing). PR-A1 keeps that two-home split as it is (contract 2), and consolidating it is the decomposition slot's job. |
| R×H | The stub's `gc_root_object_ids` (an empty iterator) is the only `HostData` rooting `default` sees, and it is unchanged. |
| M×G | Every fix is attribute-only in the engine rows, apart from `Vm::new`'s statement form (A1-2), the doc sentences (A1-3, A1-9, A1-10) and the redundant inner gate (A1-8). Measured: app and browser clippy rc 0 before and after, and their rustdoc warning sets are identical by (file, message) (§5.2). |
| M×O, M×R, M×H, M×L | Each follows from G×X row by row. In the engine rows every gate is the identity (M×G), so the order, roots, `HostData` and Layering facts of G×O, G×R, G×H and G×L hold unchanged there. In `default`, those rows already state what exists. |
| M×S | Two touched files are over 1000 lines (§5.1's table). `vm/gc/collect.rs` (2078): its split is umbrella 7b's (§1.2), and PR-A1 shrinks it. `vm/value.rs` (1176): prereq: value.rs split (PM 651, PM 586 root 4 seam), lands first. PR-A1 touches the post-split file (about 967). Every other touched file, `vm/natives_timer.rs` (440) included, is under 1000. |
| T×G | A1-3 narrows `cfg(test)` to `cfg(all(test, feature = "engine"))`, because every caller of `new_with_mode` is engine-gated. A1-10 uses `cfg(any(test, feature = "engine"))`, because `tests_timer` is a `default`-row test reader of `drain_timers`. `next_timer_deadline` has no test reader in `default`, so it gets `cfg(feature = "engine")`. Each gate follows that item's measured reader set. Measured: `default` `test --lib --no-run` rc 0 with A1-1…A1-10 (§5.2). The `default` row's test *census* stays PR-T's (§1.2). |

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
- **File sizes.** `wc -l` on every touched file, at `8344af2b` (identical at `bea9ef19`):

  | File | Lines | Fixes | > 1000? |
  |---|---|---|---|
  | `vm/init.rs` | 860 | A1-1, A1-2, A1-3 | no |
  | `vm/lifecycle.rs` | 668 | A1-4 | no |
  | `vm/gc/collect.rs` | 2078 | A1-6 | **yes**: umbrella 7b's split (§1.2); PR-A1 shrinks it |
  | `vm/gc/roots.rs` | 619 | A1-8 | no |
  | `vm/natives_json/mod.rs` | 63 | A1-7 | no |
  | `vm/natives_json/parse.rs` | 568 | A1-7 | no |
  | `vm/natives_json/stringify.rs` | 586 | A1-7 | no |
  | `vm/natives_promise.rs` | 961 | A1-8 | no |
  | `vm/shape_ops.rs` | 619 | A1-9 | no |
  | `vm/value.rs` | 1176 → about 967 after the prereq (measured at the prereq) | A1-9 | **yes** at the base. Prereq: value.rs split (PM 651, PM 586 root 4 seam), lands first |
  | `vm/natives_timer.rs` | 440 | A1-10 | no |
- **Rustdoc warning sets.** Each row's rustdoc command without `-D warnings`. The log is parsed into a set of
  distinct (file, line, message) triples, each `warning:` line paired with the `-->` location that follows it. A row's *own* warnings are the
  `default` triples whose (file, message) occurs in neither `app` nor `browser`.

### §5.2 Results at `8344af2b`, and with the fix table applied (macOS)

The fix table was applied as a scratch probe in this worktree and then reverted. `git checkout -- crates`, then
`git status --porcelain` → empty. Results:

| Row / column | Base `8344af2b` | With A1-1…A1-9 (Rev 1.1) |
|---|---|---|
| `default` clippy | rc 101, 17 errors (§1.3 D1–D17) | rc 0 |
| `default` rustdoc | rc 101 at E0432, no output | builds. 49 distinct (file, line, message) warnings (§5.3) |
| `default` `test --lib --no-run` | not reached | rc 0 |
| `app` clippy | rc 0 | rc 0 |
| `browser` clippy | rc 0 | rc 0 |
| `browser` clippy `--all-targets` | — | rc 0 |
| `app` rustdoc (warnings) | 194 | 194, the same set by (file, message) |
| `browser` rustdoc (warnings) | 193 | 193, the same set by (file, message) |

**Rev 1.2, with A1-1…A1-10, at `bea9ef19`'s code** (same probe procedure; then `git checkout -- crates` and
`git status --porcelain` → empty):
- every clippy column, `browser` `--all-targets` and `default` `test --lib --no-run`: rc 0, 0 warnings;
- every rustdoc build: rc 0;
- `default` rustdoc: 50 distinct triples, 26 own (§5.3; the 26th is A1-10's);
- `app` / `browser` rustdoc: 78 / 78 distinct triples. Both sets are identical, line for line, to Rev 1.1's
  re-measurement with A1-1…A1-9 at `bea9ef19`. So A1-10 adds nothing to the engine rows.

Intermediate probes, which give each class its control:
- With D1–D17 fixed and nothing else, the `default` row reports exactly N1–N6.
- With N1–N6 fixed as well, it reports N7.
- With N7 fixed, it reports nothing.

So reverting any one fix brings back its own diagnostic. In the PR, the guard for every fix is the `default` row's
clippy column.

**Three OS** (U0 "pre-matrix three-OS evidence", PM 558 (1)):
- **Runs.** On PR-A1's branch, a scratch commit adds temporary steps to the `check` job, which already runs on all
  three OS. A later commit removes them.
  - Run B, the base: the steps sit on PR-A1's base (the post-split main), before any fix commit. It carries only
    the `app` / `browser` steps, because `default` does not build at the base (§5.2).
  - Run H: the steps sit on the fixed head. Condition (ii) refers to run H.
- **Steps.** Per row (`default`, `app`, `browser`):
  - clippy `-D warnings`, expected green;
  - for `default` only, `test --lib --no-run`;
  - a **rustdoc listing step**: the U0 rustdoc command without `-D warnings`, with
    `--message-format=json` written to `rustdoc-<row>-<os>.json`, uploaded as a workflow artifact, with the
    triple count echoed to the log.
- **What each listing step emits.** For each OS and row, the warning list as the set of distinct (file, line,
  message) triples. The step fails only if rustdoc does not build.
- **Expected red.** Why a non-`-D` listing step, and not `continue-on-error` or ordering:
  - The `default` rustdoc column is red by design until the default 行 rustdoc の PR lands.
  - `continue-on-error` would also swallow a hard build error, so a step's red would stop meaning "this row
    failed".
  - Ordering cannot help: placing the red step last still hides any later row, and there are three rustdoc steps.
  - With the listing step, every step that runs is expected green and its rc is its result, so U0 conditions
    (i)–(iii) keep their meaning. The rustdoc acceptance, "no warning outside the named population", is a set
    statement, and the listing measures it directly.
- **Comparison.** Lists are compared by set, keyed by (file, line, message), against macOS's at the same head:
  - **`default`.** Linux's and Windows's run-H sets are compared with macOS's run-H set. macOS's set is §5.3's
    26, re-checked against the local measurement.
  - Owners of a per-OS increment follow the rule of PM 649/650: a site outside `vm/host/` is VM P4's, a site
    inside it is L3's. Both go to the default 行 rustdoc の PR.
  - **`app` / `browser`.** On each OS, run H's set must equal run B's, keyed by (file, message), because the fixes
    shift lines only in the touched files. This is the U0 acceptance "without adding a diagnostic to `app` or
    `browser`".
  - An `app` / `browser` triple that is in run B and run H on Linux or Windows but not on macOS is pre-existing
    and outside PR-A1. U0 left Linux / Windows unmeasured in the PR-D… population. Such a triple goes to PM under
    the C0 rule (U0 §4.4 step 2). PR-A1 does not fix it.
  - A triple in run H but not run B is PR-A1's own, and PR-A1 fixes it before landing.
- **PR body.** It records run B's and run H's ids and head shas, and the per-OS sets of all three rows with their
  increments over macOS, split by owner.
- At merge, PM checks (i)–(iii) of U0 §6.
- The touched files contain no `cfg(target_os)` / `cfg(unix)` / `cfg(windows)` site (`/usr/bin/grep -c` on each →
  0). Their per-OS results can differ only through code they do not touch, and the three-OS run is what covers that.
- Engine behaviour (A1-4, A1-7, A1-8, A1-9, A1-10) on three OS is the `check` job's `--all-features` test run, which the
  PR runs anyway.

### §5.3 The `default` row's rustdoc warnings (handed to the default 行 rustdoc の PR, PM 650)

There are 49 distinct (file, line, message) warnings. 24 also occur in the `app` or `browser` row at the same code
and are already PR-D… population. The other **25 are the `default` row's own**, all `unresolved link` warnings in
24 doc lines over 11 files (verified 2026-11-11: the set difference of the three rows' warning lists, §5.1).

**Re-measured at `bea9ef19`** (PM 649; current main, including #622), with the fix table applied by the same probe
(apply, measure, `git checkout -- crates`, `git status --porcelain` → empty): the same 49, and the same 25 own
warnings, line for line. #622 removed the 4 `vm/host/mod.rs:230` warnings, which belong to the `app` / `browser`
population, not to these 25; the 4 at `vm/host/mod.rs:53-68` remain.

**Rev 1.2, with A1-10** (§5.2): the same 25, plus one created by A1-10's gate,
`vm/natives_timer.rs:6` (`unresolved link to VmInner::drain_timers`). So 26 own, 50 distinct in all.

They are listed here and, per U0 §6, go in PR-A1's body (§5.6). PM 650 takes them out of PR-D…. They go to one
separate PR, the default 行 rustdoc の PR:
- owner: VM P4, L3's 4 included (One issue, one way; the PM tells L3);
- order: PR-A1 → that PR → U0 PR-M1;
- due: 11-25.
The owner split by site (PM 649) is kept as the table's column: a site outside `vm/host/` is VM P4's, a site in
`vm/host/` is L3's. PR-A1 does not fix them. Line numbers are with the fix table applied. They differ from
`8344af2b` only in `vm/init.rs` (+2 lines above `:26`, +1 above `:162`). `vm/natives_timer.rs:6` is unmoved.

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
| `vm/natives_timer.rs` | `:6` `VmInner::drain_timers` (A1-10, Rev 1.2) | 1 | VM P4 |
| `vm/temp_root.rs` | `:15` (2: `VmInner::push_temp_root`, `VmTempRoot`), `:131`, `:134`, `:151`, `:154`, `:170` | 7 | VM P4 |

Totals: 26 warnings over 12 files. The 25 were verified 2026-11-11 at `8344af2b` and again at `bea9ef19`. The 26th was
measured at `bea9ef19` with A1-1…A1-10 (Rev 1.2). Owners: VM P4 22, L3 4. PM 650 ruled on 21 + 4; the 26th is
VM P4's by the same rule, and PM is told (§7.2 item 3). All 26 go to the default 行 rustdoc の PR.

**The window** (PM 650). From PR-A1's merge until the default 行 rustdoc の PR lands, the `default` row's rustdoc
column is red (these 26). That PR lands before PR-M1, so the red column never meets PR-M1's `feature-profiles` job.
PM 649 said "no CI job runs the row before PR-M1". That holds only because PR-M1 lands after the fix, and PM 650
corrects it on that basis.

**Provenance.**
- 23 of the 26 are inherent to the row: the link target is compiled out of `default` at the base too.
- 2 are created by PR-A1's own gate: `vm/natives_json/stringify.rs:3`, `:36` link to
  `stringify_for_structured_shortcut`, which A1-7 gates.
- 1 is created by A1-10's gate: `vm/natives_timer.rs:6` links to `VmInner::drain_timers`, which is compiled out of
  `default` without `test`.

PR-A1 still hands these to the default 行 rustdoc の PR and does not fix them itself. All 26 have one shape: an
ungated doc comment that links to an item compiled out of `default`. The form that fixes that shape, for example a plain code span
instead of a link, or an engine-only doc attribute, is one decision for the whole population. If PR-A1 picked a form
for those two, the program would have two forms (One issue, one way). PR-A1 fixes only doc *statements* that its own
change makes false (A1-3, A1-9, A1-10). Those are content, not link form. The same reasoning covers A1-10's
`vm/natives_timer.rs:6`.

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

The local `mise` `check-default` task (`mise.toml:14-19`) is not extended in PR-A1. `mise.toml` and the CI shape
belong to the CI/tools lane under U0 (PR-M1 retires `check-default`), and U0 §4.4 step 4 ratified the window; PM 651
(3) hands the question to the PR-M1 plan.

### §5.5 Plan-stage checks

- `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-elidex-js-default-row-pra1.md` →
  rc 0, no hard-fail; K=4, M=8, SPLIT-RECOMMENDED (answered under §3's breadth note); grep-pass 0 hard, 0 soft.
- `python3 .claude/tools/ecs_mission_ratchet.py` → 0 new.

### §5.6 PR body requirements

The PR body carries:
- every fix with its kind (U0 §6 acceptance): A1-1, A1-2, A1-3, A1-4, A1-6, A1-7, A1-8, A1-9, A1-10, each (b); no
  (a), no (c) (§7.1). A1-5 is the order contract, not a fix;
- the later-phase diagnostics N1–N7 (§0.3 item 3, §1.3), and N8–N10 behind A1-10's suppression (§1.3), each named;
- run B's and run H's ids and head shas, and the per-OS rustdoc sets with their increments by owner (§5.2);
- §5.3's 26 warnings, by owner;
- the line: "default 行の rustdoc は 26 件 → default 行 rustdoc の PR (PR-A1 の後、PR-M1 の前、11-25、PM 650)".

## §6. Observation for the split umbrella (F1-c)

This section is a record only, not a plan. It is input for `#11-elidex-js-no-default-features-support`, which is
re-evaluated on 2026-11-30. These are the core items PR-A1 marks as host-only. A crate split decides where each one
lives:
- `Vm::unbind`'s bound-world cleanup (`vm/lifecycle.rs:30-67`), the inverse of the engine-only `Vm::bind`;
- `new_with_scope`'s `global_scope_kind` / `engine_mode` parameters, whose readers are engine-only today (see the
  `engine_mode` note below);
- the JSON helpers for structured serialization. Also pre-existing and left ungated: the structured-serialize mode
  inside the shared core serializer, i.e. the `reject_date` field of `JsonSerializer` and its Date check
  (`vm/natives_json/stringify.rs:38`, `:61-63`) and `stringify_impl`'s `reject_date` parameter (`:435`). This is host
  semantics in core;
- `Microtask::NotifyMutationObservers` in the core microtask enum. It is a host microtask with no host-microtask
  seam;
- the Web IDL illegal-constructor helpers in `vm/shape_ops.rs` / `vm/value.rs`, beside
  `CallShape::IllegalConstructor`, which stays in core;
- the timer drive (A1-10). The timer globals are HTML §8.7 and are registered in every configuration
  (`vm/globals.rs:107-110`). In `default` without `test`, nothing drains the heap, so `setTimeout` / `setInterval`
  accept callbacks that never fire, and the interval period is compiled out. Whether a host-less build exposes the
  timer globals at all is the split's decision (§7.2 item 5);
- the well-known string table (`vm/well_known.rs:60`): 779 of its fields are read only by engine code. The `allow`
  that covers them is needed in every row (§2.1 reason 5).

`engine_mode` (A1-2): its readers are engine-only today. `EngineMode` itself is the whole-engine core/compat
authority (`crates/core/elidex-plugin/src/spec_level.rs:78-81`), so where the parameter lives is the split's
decision too, not a host-only fact.

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

### §7.2 Items for PM (items 1–3 ruled by PM 649; item 3 re-ruled by PM 650)

1. **`#11-vm-inbound-host-call-membership-test`** (raised per the Layering check, PM 585, PM 588): Provisionally (b),
   PM 649. Re-examine A1-8 when the slot is ruled. Under the slot's directory scope (ledger `:311`), A1-8's drain
   arm is the one site reached. Under I-5's boundary, A1-4's block is reached as well (§2.2). Neither adds a call.
   No other fix is reached ((b) for each). PR-A1 does not wait.
2. **`#11-host-data-full-decomposition` record** (contract 3): "PR-A1 moved nothing" (§4). The slot has no Owner;
   the record goes to a facet on the PM-side ledger, written by the PMO (PM 649).
3. **The `default` row's rustdoc warnings** → default 行 rustdoc の PR (PR-A1 の後、PR-M1 の前、11-25、PM 650),
   owner VM P4, L3's 4 included. For PM: the count is now 26, not 25. A1-10 adds `vm/natives_timer.rs:6`, which is
   VM P4's (§5.3). The window is stated in §5.3.
4. **Order contract**: unchanged (§2.3). The PM 640 condition does not apply, and the bound-dispatch slot's premise
   is untouched.
5. **New in Rev 1.2, for the split (not blocking PR-A1)**:
   - (a) In `default` without `test`, the timer globals schedule entries that nothing drains (§6). PR-A1 gates only
     what has no reader (A1-10). Whether the host-less row should expose the HTML §8.7 timer globals at all would
     change that row's global surface and touch `vm/mod.rs` (2797 lines), so it is returned, not decided.
   - (b) `vm/well_known.rs:60`'s mixed `allow` (§2.1 reason 5): the 779 `default`-only dead fields would need
     per-entry gating of the generated table.
   - Both go to `#11-elidex-js-no-default-features-support` (§9 item 5) unless PM names another home.

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
| `vm/value.rs` (post-split) | `native_illegal_constructor_unreachable` (by name; the prereq split moves lines) | A1-9 |
| `vm/natives_timer.rs` | import `:12`; `TimerEntry` field `repeat` (`:26`); `MIN_INTERVAL_REPEAT_MS` (`:66`); `schedule_timer` (`:99`; parameter `:102`, `let` `:143`, field `:155`); `native_set_timeout` (`:162`; call `:167`); `native_set_interval` (`:171`; call `:176`); `impl VmInner` (comment `:239-241`, attribute `:242`); `next_timer_deadline` (`:259`) | A1-10 |

**Not touched**:
- `HostData::unbind` (`vm/host_data/engine_feature/bind.rs:118`) and `HostData::bind` (`:80`);
- the stub (`vm/host_data/mod.rs:680-704`);
- `Vm::bind` (`vm/vm_api.rs:203`);
- `Vm::teardown_document` and the engine block of `Vm::unbind` (`vm/lifecycle.rs:90-…`);
- `NativeContext` `host` / `host_opt` / `host_if_bound`;
- every interpreter entry, `ObjectKind::HostObject`, and every file under `vm/host/`.

## §9. Landing list (PMO, via PM)

Landing order: prereq: value.rs split (PM 651, PM 586 root 4 seam), lands first (VM P4, 11-18) → PR-A1 (11-21) →
default 行 rustdoc の PR (11-25) → U0 PR-M1 (11-27).

1. `#11-elidex-js-default-features-build-broken`: no close at PR-A1 (U0 §8: it closes when PR-M1 lands with every
   row-fixing PR landed). Note on the slot: "PR-A1 landed `<sha>`. The `default` row is clippy-clean on three OS
   (run `<id>`)."
2. `#11-host-data-full-decomposition`: the facet is already on the PM-side ledger (`project_open-defer-slots.md:181`,
   PM 649). The landing action is to repoint its source anchor (§4 @ `13400790`) to the merge commit. No new facet.
3. `#11-vm-typed-array-family-layering-and-gate` (VM P4): its "Until then" premise ends when PR-A1 lands (U0 §8.1).
   PM routes it.
4. The default 行 rustdoc の PR: §5.3's 26 warnings (VM P4 22, L3 4), due 11-25, after PR-A1 and before PR-M1
   (PM 650). The PM tells L3 that its 4 are in VM P4's PR.
5. `#11-elidex-js-no-default-features-support`: a pointer to §6 of this memo, as the split's input.
6. `#11-vm-inbound-host-call-membership-test`: §7.2 item 1's site list, as input for whenever the slot is decided:
   A1-8's drain arm (directory scope and I-5), and A1-4's block (reached under I-5).

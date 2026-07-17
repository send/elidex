# Plan — `input.valueAsDate` + `valueAsNumber` date-types (`#11-input-value-as-date`)

**Slot**: `#11-input-value-as-date` (UNBLOCKED 2026-07-15 by the VM `Date` builtin, #467 `3e252148`).
**Branch**: `vm-input-value-as-date` / worktree `/Users/kazuaki/repos/send.sh/elidex-wt-valdate`.
**Lane**: core VM.

## §1 Stacking (revised after plan-review F1 — the original "no L3 collision" claim was FALSE)

This PR is **stacked on the L3 lane's `elidex-form-core` carve (Slice 0a)** — but on that carve **replayed
onto current `main`**, not on the L3 branch as-is. `origin/domform-form-state-reconciliation` forks at
`7bafcb9f` (#466), i.e. **before** #467 `3e252148`, so basing on its head directly **drops the `Date`
builtin this PR depends on** (verified 2026-07-15: `natives_date` / `ObjectKind::Date` absent there).

**Base** = `main` (`3e252148`) + L3's Slice-0a commits cherry-picked. The replay is clean — L3 touches
`crates/dom/elidex-form*`, main-since-#466 touches `vm/` + css (disjoint file sets). Both prerequisites
verified present on the stack: `natives_date` ✓ and `elidex-form-core/src/datetime.rs` ✓ (the pre-carve
`datetime.rs` under `elidex-form` is gone, moved by the carve).

**Landing order**: Slice 0a merges to `main` first (it is implemented + CI-green, awaiting its own PR; and
being pre-#467 it must rebase onto current `main` regardless). This PR then rebases onto the resulting
`main` and opens against `main`. The stack here is a working snapshot — L3's branch is still moving (it has
since gained `/simplify` fixes), so re-verify the base before opening the PR.

**Why stacked, not authored into the pre-carve file**: the conversion algorithms this PR adds are pure,
coherence-bound derivations — *exactly* the class Slice 0a relocates into the leaf crate — so
`elidex-form-core` **is** their home independent of scheduling (ideal-over-pragmatic). Authoring them into
the soon-to-be-moved pre-carve file would make this PR's export-chain work (§4.2) get restructured again by
the carve. Stacking keeps implementation **non-blocking** per [[feedback_split-on-touch-prereq-workflow]]
(prereq split first, feature stacks on it) while landing the code directly in its final home.

## §2 Goal + coupled invariants

Complete the `<input>` date/time value-accessor family blocked on the absence of a VM `Date` builtin
(module doc `html_input_value.rs:12` names the slot). Two coupled gaps, one coherent family (the slot
itself scopes it as "valueAsDate + valueAsNumber-date-types"):

1. **`valueAsDate`** — get returns null / set throws, stubs at `html_input_value.rs:381`/`:392`.
2. **`valueAsNumber` for the date types** — only `Number`/`Range` are handled (`:335-338` get → `NaN`;
   `:365-376` set → `InvalidStateError`); date/month/week/time/datetime-local return `NaN` / throw today,
   a spec gap (they DO apply per §4.10.5.1.7-.11).

### Coupled invariants (this work sits at their intersection)

| # | Invariant | |
|---|---|---|
| **I1** | **Applies-matrix** is per-*accessor* (valueAsDate ⊂ valueAsNumber) | see §2.1 |
| **I2** | **Conversion identity** — "string→number" and "string→Date object" are *distinct* spec algorithms per type | see §2.2 |
| **I3** | **Direction duality** — each accessor has get (parse) + set (serialize), with different error branches | §2.3 |
| **I4** | **Crate boundary** — algorithms engine-independent (`elidex-form-core`), host marshalling-only | §4 |

Pairwise intersections (where a prose-only plan would leak):
- **I1 × I2** — Month is in *both* applies-sets but its two conversions **diverge** (month-count vs
  first-of-month ms): the accessor's gate and its conversion cannot share one helper. **The load-bearing corner.**
- **I1 × I3** — the set path must distinguish *not-applies* (→`InvalidStateError`) from
  *applies-but-unparseable* (→`""`/`NaN`); one predicate can't answer both, so the gate is checked before conversion.
- **I2 × I4** — because the two conversions differ per type, the "string→Date" algorithm is first-class
  engine-indep code in `elidex-form-core`, NOT derived host-side from the number conversion.
- **I3 × I4** — the set direction writes through `FormControlState`'s `set_value` (called via receiver,
  `html_input_value.rs:367`), the existing dirty-value+sanitize chokepoint — no new write path.

### §2.1 Spec surface (webref-confirmed, HTML §4.10.5.4 Common input element APIs + per-type §4.10.5.1.7-.11)

`valueAsDate` (`#dom-input-valueasdate`):
- **get**: if it does not apply for the current type state → `null`. Else run *"convert a string to a Date
  object"* for that state on `value`; return the Date if produced, else `null`.
- **set**: if it does not apply → **`InvalidStateError`**. Else if the new value is not `null` and not a
  Date object → **`TypeError`**. Else if `null` OR a Date with a **NaN time value** → set `value` to `""`.
  Else run *"convert a Date object to a string"* for that state → set `value`.

`valueAsNumber` (`#dom-input-valueasnumber`):
- **get**: if it does not apply → `NaN`. Else *"convert a string to a number"* on `value` (number or `NaN`).
- **set**: if the new value is infinite → **`TypeError`** (checked *before* the applies gate). Else if it
  does not apply → **`InvalidStateError`**. Else if `NaN` → `value=""`. Else *"convert a number to a
  string"* → set `value`.

### §2.2 Applies-matrix (I1, webref-confirmed — the two accessors differ)

| type | valueAsNumber applies | valueAsDate applies |
|------|:--:|:--:|
| Date | ✓ | ✓ |
| Month | ✓ | ✓ |
| Week | ✓ | ✓ |
| Time | ✓ | ✓ |
| DatetimeLocal | ✓ | **✗** (spec: "valueAsDate … do not apply") |
| Number, Range | ✓ | ✗ |

### §2.3 ⚠ Conversion identity (I2) — the two conversions are NOT the same number

`valueAsDate` must **NOT** naively reuse the `valueAsNumber` conversion:

| type | "string → number" (valueAsNumber) | "string → **Date object**" (valueAsDate) |
|------|---|---|
| Date | ms epoch→that day 00:00 UTC | Date at that instant (**same ms**) |
| Week | ms epoch→Monday 00:00 UTC | Date at Monday 00:00 UTC (**same ms**) |
| Time | ms since midnight | Date on **1970-01-01** at that time (**same ms**, 1970-01-01 epoch=0) |
| Month | **number of MONTHS since 1970-01** | Date at **first-of-month 00:00 UTC** (**DIVERGES** — months vs ms) |

So "convert a string to a Date object" / "convert a Date object to a string" are **first-class per-type
algorithms**, added alongside the existing number ones — not derived from them.

## §3. Spec coverage map

Section numbers webref-verified 2026-07-15 (`.claude/tools/webref coverage-map` — corrected an assumed
per-type order: the sequence is Date .7 → **Month .8 → Week .9 → Time .10** → datetime-local .11; and the
accessor home is **§4.10.5.4** "Common input element APIs", not §4.10.5.5 "Common event behaviors").

| Spec section | Step | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.5.4 Common input element APIs | valueAsDate get | apply→convert-string-to-Date; not-apply→`null` | `native_input_get_value_as_date` (`crates/script/elidex-js/src/vm/host/html_input_value.rs`) | ✓ | yes (`value`) |
| WHATWG HTML §4.10.5.4 Common input element APIs | valueAsDate set | not-apply→InvalidStateError; not-null∧not-Date→TypeError; null∨NaN-Date→`""`; else convert-Date-to-string | `native_input_set_value_as_date` (same file) | ✓ | yes (Date arg + `value`) |
| WHATWG HTML §4.10.5.4 Common input element APIs | valueAsNumber get | apply→convert-string-to-number; not-apply→`NaN` | `native_input_get_value_as_number` (fix: add date-type arms) | ✓ | yes (`value`) |
| WHATWG HTML §4.10.5.4 Common input element APIs | valueAsNumber set | infinite→TypeError (before gate); not-apply→InvalidStateError; NaN→`""`; else convert-number-to-string | `native_input_set_value_as_number` (fix: add date-type arms) | ✓ | yes (number arg) |
| WHATWG HTML §4.10.5.1.7 Date state | 4 per-type algorithms | string↔number (epoch-ms) / string→Date (**same ms**) / Date→string | `crates/dom/elidex-form-core/src/datetime.rs` (string→number exists; string→Date, Date→string NEW) | ✓ | yes (`value`) |
| WHATWG HTML §4.10.5.1.8 Month state | 4 per-type algorithms | string→number = **month-count**; string→Date = **first-of-month 00:00 UTC ms** (⚠ DIVERGE); Date→string; number→string | `crates/dom/elidex-form-core/src/datetime.rs` | ✓ | yes (`value`) |
| WHATWG HTML §4.10.5.1.9 Week state | 4 per-type algorithms | string→Date = Monday 00:00 UTC (= string→number ms) | `crates/dom/elidex-form-core/src/datetime.rs` | ✓ | yes (`value`) |
| WHATWG HTML §4.10.5.1.10 Time state | 4 per-type algorithms | string→Date = 1970-01-01 UTC + time (= string→number ms-since-midnight) | `crates/dom/elidex-form-core/src/datetime.rs` | ✓ | yes (`value`) |
| WHATWG HTML §4.10.5.1.11 Local Date and Time state | valueAsNumber only | valueAsDate does **not** apply (get→null via gate / set→InvalidStateError); string↔number epoch-ms | `crates/dom/elidex-form-core/src/datetime.rs` (string→number exists) | ✓ | yes (`value`) |

**Breadth**: K=1 spec (html), M=9 rows above (verified via `webref coverage-map` 2026-07-15) → K<4, M<20.

### §3.1 User-input touch audit

- **valueAsDate/valueAsNumber getters** — `value` (user-set content attr or prior IDL write) flows into the
  convert algorithms; all `datetime.rs` convert fns return `Option` (parse-total, no panic on malformed value).
- **valueAsDate setter** — the arg is an arbitrary user JS value; brand-check `ObjectKind::Date` *before*
  reading its `tv`; non-Date → TypeError, no unwrap/panic. NaN-`tv` handled explicitly (→`""`).
- **valueAsNumber setter** — arg is user-controlled f64; `infinite`→TypeError, `NaN`→`""` per §4.10.5.4.
- **Adjacent pre-existing** — all writes go through `FormControlState`'s `set_value` (existing dirty-value +
  sanitization chokepoint); exposure delta = none (no new write path, only new date-type read/convert arms).

## §4 Ideal design (Layering-first)

**Engine-independent conversions live in the leaf crate `elidex-form-core` (`src/datetime.rs`); VM host =
marshalling-only** (CLAUDE.md Layering mandate — `host/` does JsValue↔Entity + `ObjectKind::Date`
construct/read, nothing algorithmic).

### §4.1 `elidex-form-core/src/datetime.rs` — ADD (the two missing spec algorithms)

- `pub fn convert_string_to_date_ms(kind: FormControlKind, s: &str) -> Option<f64>` **(NEW)** — the spec
  *"convert a string to a Date object"*, returning the object's **time value in ms** (host wraps it in
  `ObjectKind::Date`). Per-type: Date/Week/Time reuse the existing epoch-ms parse; **Month = first-of-month
  midnight UTC** via the existing `CivilDate`/`days_from_civil` path (NOT the month-count).
- `pub fn convert_date_ms_to_string(kind: FormControlKind, ms: f64) -> Option<String>` **(NEW)** — the spec
  *"convert a Date object to a string"* (valid date/month/week/time string in UTC for `ms`). Non-finite `ms`
  → `None` (caller maps to `value=""`).
- Both gated to `matches!(kind, Date|Month|Week|Time)`; `None` for non-applying types. Reuse the `CivilDate`
  decomposition already in the file; no new date-math primitives.

### §4.2 Export chain (plan-review F4 — the original `pub(crate)` spec would NOT compile)

`elidex-js` depends on **`elidex-form`** (`Cargo.toml:141`), **not** on `elidex-form-core` directly, and
form-core's `mod datetime;` (`lib.rs:18`) is **private** with `convert_string_to_number` (`:498`) /
`convert_number_to_string` (`:607`) both **`pub(crate)`** — i.e. *no* convert fn is reachable outside
form-core today. The host is their first external caller, so expose all **four** (2 NEW + 2 existing):

1. `datetime.rs`: the 4 fns → `pub`.
2. `elidex-form-core/src/lib.rs`: `pub use datetime::{convert_string_to_date_ms, convert_date_ms_to_string,
   convert_string_to_number, convert_number_to_string};` — item re-export, module stays private (matches the
   existing `pub use fieldset::{…}` / `pub use input::{…}` idiom).
3. `elidex-form/src/lib.rs`: add the 4 to the Slice-0a I3 facade block (`pub use elidex_form_core::{…}`,
   `lib.rs:37`) so the `elidex_form::` re-export stays the single downstream path (One-issue-one-way — no
   new `elidex-js → elidex-form-core` dep edge).
4. Host calls `elidex_form::convert_*`.

### §4.3 VM host `html_input_value.rs` — marshalling only

- `native_input_get_value_as_date`: apply-gate (Date/Month/Week/Time) else `null`;
  `convert_string_to_date_ms(kind, value)` → `Some(ms)` → `ctx.vm.create_date(ms)` (`inner.rs:229`) →
  `JsValue::Object`; `None` → `null`.
- `native_input_set_value_as_date`: not-apply → `InvalidStateError`; arg not `null`/`ObjectKind::Date` →
  `TypeError`; `null` or `Date(NaN)` → `set_value("")`; else read `ObjectKind::Date(ms)` →
  `convert_date_ms_to_string(kind, ms)` → `set_value(s)`.
- **Fix `valueAsNumber`** get/set to route the date types through `convert_string_to_number` /
  `convert_number_to_string` (add Date/Month/Week/Time/DatetimeLocal arms; keep Number/Range). Closes the
  date-types half of the family and makes valueAsNumber↔valueAsDate consistent for date/week/time.
- All value writes go through the existing `set_value` chokepoint — no direct field pokes.
- The per-accessor applies-gate stays an inline `match state.kind` in host (plan-review F5, accept-as-is):
  it is *type dispatch*, not date/time algorithm, and mirrors the established sibling
  `native_input_get_value_as_number` (`:335-338`) pattern; the Layering mandate scopes host to brand-check +
  arg coercion + marshalling, which this is.

### §4.4 `create_date` for NaN

`VmInner::create_date(f64::NAN)` is the correct "Date representing the NaN time value" (invalid Date);
`ObjectKind::Date(t)` read yields `t.is_nan()` → the set-path `""` branch. `create_date` stores the raw `tv`
without TimeClip rejection (verified at `inner.rs:229`, #467).

## §5 Layering / ECS-native check

- No date/time algorithm in `host/` — all of it in `elidex-form-core`; host calls it + marshals
  `ObjectKind::Date`. ✓ Layering mandate.
- `FormControlKind` + `FormControlState` are the existing SoT (ECS component, read by the sibling
  valueAsNumber accessor already); no new side-store, no new `ObjectKind` variant. ✓

## §6 Non-goals / defer

- **Local-timezone `valueAsDate`** — N/A; all four applying types are UTC-defined by spec. Not a defer, a
  spec fact. (Distinct from the `#11-vm-date-local-timezone` slot, which is about the `Date` builtin.)
- **`datetime-local` valueAsDate** — spec says it does not apply → `InvalidStateError`/`null` by the gate.
  (Its valueAsNumber DOES apply and is fixed here.)
- No new `#11-` slot anticipated. If `datetime.rs` reveals a missing per-type parse edge, carve narrowly.

## §7 Tests (engine-independent unit + VM)

- `elidex-form-core` `datetime_tests.rs`: round-trip `convert_string_to_date_ms`↔`convert_date_ms_to_string`
  for date/month/week/time incl. the **Month first-of-month ≠ month-count** distinction (the I1×I2 corner —
  the key regression), epoch boundaries, out-of-range → `None`.
- VM `tests_html_input_proto.rs`: `valueAsDate` get (each applying type → Date instant; datetime-local →
  null / InvalidStateError), set (Date → value string; `null`/NaN-Date → `""`; non-Date → TypeError;
  not-apply → InvalidStateError); `valueAsNumber` date-types get/set now non-NaN; valueAsNumber↔valueAsDate
  consistency for date/week/time; **Month divergence pinned**.

## §8 Slicing — single PR, positive appeal against the edge-dense split default (plan-review F2)

The work is edge-dense (§2 coupled invariants I1-I4), so CLAUDE.md's **default is split**; a single PR needs
a positive appeal, which the breadth heuristic (§3: K=1/M=9) does **not** supply. The appeal:

- **Splitting the `elidex-form-core` conversions from their sole callers ships dead code** — nothing else
  calls `convert_string_to_date_ms`; a conversions-only PR would land an unreachable API ("dead code は接続
  するか削除").
- **Splitting `valueAsDate` from the `valueAsNumber` date-arms ships an incoherent intermediate** — the two
  accessors share one applies-matrix and one conversion family; landing valueAsDate while valueAsNumber still
  returns `NaN` for the same types is two truths about one surface (One-issue-one-way), and the
  cross-accessor consistency invariant (I1×I2, Month divergence) is only testable with both present.
- **(b) canonical-algorithm-absent does not hold** — WHATWG §4.10.5.1.7-.11 give the exact per-type
  algorithms; the edge-density is a *known finite matrix* (5 types × 2 accessors), fully enumerated in §2.2/§2.3,
  not an unmapped design space. The MANDATORY plan-review limb is honored (this document); the split limb is
  answered by coherence, not by size.

Terminal unit: 4 form-core fns exposed + 2 NEW + 4 host accessors + tests.

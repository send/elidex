# VM P4 Slice F — the `vm/host/structured_clone.rs` split (per-PR plan, rev 1)

**Row.** Slice **F**, `#11-vm-structured-clone-file-split`, the §5 row of
`docs/plans/2026-07-vm-p4-slice-plan.md` (`git grep -n '^| \*\*F\*\* |' -- docs/plans/2026-07-vm-p4-slice-plan.md`
→ `:962` at the base). The row is the source of truth for the seam, the acceptance, the declared green
guard, `Deps` (—) and the plan-review obligation. This memo restates none of them. It adds only what the
row leaves to the per-PR plan: the exact cut, the file names, the wiring, the measurement commands, and
the landing obligations. It is F's mandatory `/elidex-plan-review` input. It is outside the
`2026-07-vm-p4-*` family glob, so family memos cite it by path in backticks and never by a relative link
(§5, L-7).

**Base.** `origin/main` = `f9cf8c40`. Every `file:line` is at that revision unless it names another one.
`vm/host/structured_clone.rs` is byte-identical between the row's baseline `658cc302` and `f9cf8c40`
(`git diff --stat 658cc302 f9cf8c40 -- crates/script/elidex-js/src/vm/host/structured_clone.rs` → no
output), so the row's line cites and this memo's agree, with the exception recorded as premise error P-1. Paths are
relative to `crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`. §6 lists the
command behind each figure.

**Probe.** The cut in §1 was applied by the recipe in §1.6 to a throwaway
`git worktree add --detach … f9cf8c40` (session scratchpad `f-probe`). It was committed there only, as
`f16d4692`, and was not pushed. Under `rustc 1.99.1 (58cf8f9c3 2026-10-13)` it is `cargo fmt`-stable, and
`cargo clippy -p elidex-js --all-features --all-targets -- -D warnings` is clean. With
`cargo test -p elidex-js --all-features --lib`, the probe and the base each pass all 6524 tests. One
load-dependent timeout on the first probe run is recorded in §4.1. The figures in §1 and §2 are that probe's output.

**Kind.** A terminal prerequisite row, terminal by necessity. A pure move has nothing to decompose. Pc's
`Deps` names it, and the NativeError program's PR-E1 runs after it (§5, L-3).

---

## §0. Scope

### §0.1 What the row leaves to this plan

1. The cut itself: by function and by line, which file keeps which half, and the new module's name (§1.1–§1.2).
2. The edits outside the move: the `use` lines, the `mod` line and the one cross-module call path (§1.3).
   The visibility question is answered as "none changes" (§1.4).
3. Acceptance made concrete. The row's "hunk- and line-conserving" is a multiset property, and §2.2 shows
   by mutant that a multiset check cannot see where the seam was drawn. This plan therefore adds two
   anchored block identities and an exact moved-line count (§2.1).
4. The declared green guard as a command (§2.1, G-4).
5. The disposition of `#11-vm-structured-clone-layering-seam`'s trigger, which is this review (§2.3).
6. Landing obligations on the family memos and the ledgers. The family memos are not edited by this
   memo (§5).

### §0.2 Premises re-verified at `f9cf8c40`

| Premise | Verified by | Result |
|---|---|---|
| The file is 1077 lines | `wc -l vm/host/structured_clone.rs` | true |
| The algorithm half runs from `clone_value` `:75` to `data_clone_error` `:909`, which ends at `:914` | `git grep -n '^pub\|^fn' -- <file>`; `sed -n 909,915p` | true (`:915` is the blank separator) |
| `:916`-`:1077` is the `// Global registration` banner plus the binding layer | `sed -n 916,918p` | true. `impl VmInner` is at `:920`, `native_structured_clone` at `:944`, `validate_transfer` at `:965`, `ensure_empty_transfer_list` at `:998` |
| `register_structured_clone_global` is at `:922` (row F) | `git grep -n 'fn register_structured_clone_global' -- <file>` | **false, §0.3 P-1**: the fn line is `:925` |
| `ensure_empty_transfer_list` has a cross-module consumer at `vm/host/pending_tasks.rs:673` | `git grep -n 'ensure_empty_transfer_list' -- crates/script/elidex-js/src` | true. That is its only consumer outside the file; the other hits are the definition and the in-file caller at `:975` |
| Nothing in the algorithm half calls into the binding half | `sed -n 58,914p <file> \| /usr/bin/grep -c 'ensure_empty_transfer_list\|validate_transfer\|native_structured_clone'` | 0 |
| The binding half uses one algorithm item, `clone_value` (`pub(crate)`) | `sed -n 915,1077p <file> \| /usr/bin/grep -n 'clone_value\|data_clone_error\|clone_recursive'` | true. The only hit is `:962`, `clone_value(ctx.vm, value)` |
| `NativeContext` is used only by the binding half | per-half `/usr/bin/grep -c` over the imported names (§6) | true: 0 hits in `:58`-`:914`, 3 in `:915`-`:1077`. Every other imported name is used by the algorithm half |
| No test or other module names a binding-half item by module path | `git grep -n 'structured_clone::' -- crates` | true. The only module paths are `pending_tasks.rs:53` and `indexeddb/value.rs:382` (both `clone_value`, which stays) and `pending_tasks.rs:673` (moves, §1.3) |
| `engine` is not a default feature of `elidex-js` | `/usr/bin/grep -n '^default' crates/script/elidex-js/Cargo.toml` | true (`default = []`), so `check-default` does not compile either half |

### §0.3 Premise errors (reported, not acted on)

- **P-1. Row F's `register_structured_clone_global` `:922`.** `:922` is a line of that function's
  docstring (`:921`-`:924`). The fn line is `:925`. The file did not move between `658cc302` and
  `f9cf8c40` (header), so this is a mis-cite in the row and not drift. The brief's "now at `:925`" has the
  right number but the wrong cause. The re-point is L-1.
- **P-2. Row F: hunk- and line-conservation fails "a seam drawn elsewhere".** It does not fail it,
  whether the counts are compared as equal or as multisets. Mutant M2 (§2.2) also moves
  `data_clone_error` and its banner. It conserves every line, has an equal moved-out and moved-in count
  (173 = 173), and produces the same unmoved residue as the true cut. What separates the two is an
  **anchored** check: the two block identities G-2/G-3 and the exact moved count 163 (G-1). The
  correction to the row's sentence is L-1.
- **P-3. Row F's "leaving 914 and ~162 lines".** At the probe the two files are **913** and **176** lines.
  The algorithm file loses one line because `rustfmt` collapses the four-line `value` import into three
  once `NativeContext` leaves it (§1.3). The binding file is the 163 moved lines plus a 13-line header.
  One consequence is not in the row: every algorithm-half line from `:55` on shifts by exactly −1 (§1.5).
  That moves every unanchored cite of that half, including Pc's retag population and E1's `f9cf8c40`
  locators (L-2, L-3).

### §0.4 What F does not do

- It does not retag a citation, does not fix the algorithm file's module docstring, and does not narrow
  `native_structured_clone`'s visibility. All three are edits, and F is a move.
  - The docstring's first line (`:1`) still names `structuredClone(value, options?)`, which after F is
    in the other file. That line carries a drifted `§2.9`, so it is in Pc's retag population, and Pc
    rewrites it. L-2 hands the content correction to Pc in the same pass.
  - `native_structured_clone` is `pub(super)` with no consumer outside its own file. It stays
    `pub(super)` here, because a narrowing would be an edit inside the moved block. It is noted and not
    handed on.
- It does not decide or prejudge the layering question (§2.3).

---

## §1. The cut

### §1.1 The file at the base

Measured with `wc -l` and `git grep -n '^// \|^pub\|^fn\|^impl'`. A banner is three lines, and the line
given is its title.

| Lines | Section (banner) | Items |
|---|---|---|
| `:1`-`:46` | module docstring, `#![cfg(feature = "engine")]` `:46` | — |
| `:48`-`:57` | `use` block (`value` import `:52`-`:55`) | — |
| `:60` | Public entry point | `clone_value` `:75` (`pub(crate)`) |
| `:85` | Recursive worker | `clone_recursive` `:88`, `alloc_memoized_wrapper` `:196` |
| `:209` | Classification | `enum CloneKind` `:217`, `classify` `:237` |
| `:458` | Per-kind clone helpers | `clone_ordinary` `:465`, `clone_array` `:512`, `clone_regexp` `:575`, `clone_error` `:638`, `is_error_like_proto` `:697`, `alloc_wrapper` `:704`, `clone_array_buffer` `:733`, `clone_blob` `:769`, `clone_crypto_key` `:800`, `clone_typed_array` `:833`, `clone_data_view` `:874` |
| `:906` | Error helper | `data_clone_error` `:909`-`:914` |
| `:915` | blank separator | — |
| `:917` | Global registration | `impl VmInner` `:920` → `register_structured_clone_global` `:925` (`pub(in crate::vm)`); `native_structured_clone` `:944` (`pub(super)`); `validate_transfer` `:965`; `ensure_empty_transfer_list` `:998`-`:1077` (`pub(super)`) |

### §1.2 Where each half goes

- **The algorithm half stays at `vm/host/structured_clone.rs`**: `:1`-`:914`, with the one import edit of
  §1.3. That gives 913 lines.
- **The binding half moves to `crates/script/elidex-js/src/vm/host/structured_clone_binding.rs` (NEW)**,
  module `structured_clone_binding` (NEW). It holds `:915`-`:1077` verbatim (163 lines: the separator,
  the banner and the four items) under a 13-line header, which gives 176 lines.

Why the algorithm keeps the path and the binding moves:

1. **It keeps F off the algorithm half's text and consumers.** `clone_value`'s two module paths
   (`pending_tasks.rs:53`, `indexeddb/value.rs:382`) and `structured_serialize.rs:6`'s intra-doc link
   `[`super::structured_clone`]` keep resolving with no edit. NativeError PR-E1 edits `clone_recursive`,
   `classify`, `clone_ordinary`, `clone_error` and `is_error_like_proto`. All five stay in the file E1
   already names, byte-identical.
2. **It does not prejudge `#11-vm-structured-clone-layering-seam`.** Whatever home that derivation finds
   for the algorithm, its child moves one file, `structured_clone.rs`. The binding half is the part that
   `vm/host/mod.rs:11-18` already admits (it installs a global, and its argument checks are
   marshalling), so it stays in `vm/host/` under either outcome.
3. **The name.** Row F and Pc both call this half "the WebIDL binding layer". The file holds more than the
   `// Global registration` banner: it also holds the `StructuredSerializeOptions` read and the
   `sequence<object>` conversion that `postMessage` shares. So `_binding` describes it better than
   `_global`. The suffix-sibling shape follows `abort_statics.rs` and `event_source_dispatch.rs`, and
   alphabetical order puts the `mod` line between `structured_clone` and `structured_serialize`.

A directory module (`structured_clone/{mod,binding}.rs`) is rejected. It would rename the algorithm
file, which takes away reason 1 and leaves a nested path for the layering derivation to undo.

### §1.3 The edits outside the move (complete list)

| File | Edit | Lines |
|---|---|---|
| `vm/host/structured_clone.rs` | `NativeContext` leaves the `value` import. `rustfmt` then reflows `:53`-`:54` into one line | −2 / +1 |
| `vm/host/structured_clone.rs` | `:915`-`:1077` removed (moved) | −163 |
| `vm/host/structured_clone_binding.rs` (NEW) | 7 `//!` lines, a blank line, `#![cfg(feature = "engine")]`, a blank line, 3 `use` lines, a blank line; then the 163 moved lines | +176 |
| `vm/host/mod.rs` | `#[cfg(feature = "engine")]` + `pub(super) mod structured_clone_binding;` after `:330` | +2 |
| `vm/host/pending_tasks.rs:673` | `super::structured_clone::ensure_empty_transfer_list(` → `super::structured_clone_binding::ensure_empty_transfer_list(` | −1 / +1 |

The header's text (in the §1.6 recipe) contains no `§`-number and none of the words the guard pattern
matches, so it adds nothing to Pc's retag population or to G-4's pattern. No re-export is added. A
`pub use` of `ensure_empty_transfer_list` from the old path would be a shim (CLAUDE.md, *no backward
compatibility*). The one consumer's call path is edited instead.

### §1.4 Visibility

No change. Every crossing already has the visibility it needs:

- `clone_value` is `pub(crate)`.
- `register_structured_clone_global` is `pub(in crate::vm)`. It is an inherent method, so its caller at
  `vm/globals.rs:918` is unaffected by which module defines it.
- `ensure_empty_transfer_list` is `pub(super)`, which is visible to its consumer `vm/host/pending_tasks.rs`.
- `native_structured_clone` and `validate_transfer` are used only inside the new file.
- Nothing in the binding half calls a private algorithm item (§0.2).

### §1.5 Line map (base → after F)

| Base range | After F |
|---|---|
| `structured_clone.rs:1`-`:52` | same lines |
| `structured_clone.rs:53`-`:54` | `structured_clone.rs:53` (the reflowed import) |
| `structured_clone.rs:55`-`:914` | `structured_clone.rs:54`-`:913` (old − 1) |
| `structured_clone.rs:915`-`:1077` | `structured_clone_binding.rs:14`-`:176` (old − 901) |

At the probe, the binding items land at `:24` (`register_structured_clone_global`), `:43`, `:64` and
`:97`. The dense-Array read, base `:1009`, lands at `:108`, and the `iter_close`, base `:1064`, at `:163`.

### §1.6 Recipe

Run from the repository root at `f9cf8c40`, then `cargo fmt --all -- --check` (the probe ran it, and it is
a no-op).

```bash
set -euo pipefail
H=crates/script/elidex-js/src/vm/host
A=$H/structured_clone.rs
B=$H/structured_clone_binding.rs
test "$(wc -l < "$A")" -eq 1077
{
cat <<'HDR'
//! `structuredClone` global installation and its WebIDL binding layer
//! (the argument checks, the `options` dictionary read, and the
//! transfer-list conversion that `window.postMessage` shares).
//!
//! The clone algorithm itself is [`super::structured_clone`]; this module
//! only receives `structuredClone`'s arguments and hands `value` to
//! [`super::structured_clone::clone_value`].

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectKind, PropertyKey, VmError};
use super::super::VmInner;
use super::structured_clone::clone_value;

HDR
sed -n '916,1077p' "$A"
} > "$B"
python3 - "$A" <<'PY'
import sys
p = sys.argv[1]
lines = open(p, encoding="utf-8").read().split("\n")[:914]
old = ["    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,",
       "    PropertyValue, VmError,"]
assert lines[52:54] == old, lines[52:54]
lines[52:54] = ["    JsValue, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, PropertyValue, VmError,"]
open(p, "w", encoding="utf-8").write("\n".join(lines) + "\n")
PY
python3 - "$H/mod.rs" <<'PY'
import sys
p = sys.argv[1]; s = open(p, encoding="utf-8").read()
old = '#[cfg(feature = "engine")]\npub(super) mod structured_clone;\n'
assert s.count(old) == 1
s = s.replace(old, old + '#[cfg(feature = "engine")]\npub(super) mod structured_clone_binding;\n')
open(p, "w", encoding="utf-8").write(s)
PY
python3 - "$H/pending_tasks.rs" <<'PY'
import sys
p = sys.argv[1]; s = open(p, encoding="utf-8").read()
old = "    super::structured_clone::ensure_empty_transfer_list(\n"
assert s.count(old) == 1
s = s.replace(old, "    super::structured_clone_binding::ensure_empty_transfer_list(\n")
open(p, "w", encoding="utf-8").write(s)
PY
```

The blank separator `:915` is not copied by `sed -n '916,1077p'`; the header's last line stands in for
it. G-1 accounts for this, because it compares lines as a multiset.

### §1.7 Commit and PR shape

- **Commit 1, the move**: `refactor(vm): split vm/host/structured_clone.rs (pure move) — Slice F`. The
  §1.6 recipe and nothing else under `crates/`. CLAUDE.md requires the split to be a standalone PR and a
  standalone commit, and Pc's row calls it a "single commit, not bundled". This commit is that unit.
- **Commit 2, the landing obligations** of §5 that edit repository documents (`docs/plans/` only). G-1
  to G-4 are pathspec-limited to `crates/`, so they hold unchanged on the squash commit that main
  receives. Whether L-1/L-2/L-4 ride in this PR or in a follow-up docs PR is §7 Q1.

---

## §2. Acceptance, guard, and the layering trigger

### §2.1 Acceptance gates (G-1 to G-5) and the declared green guard (G-4)

`$b` = the base (`f9cf8c40`, or the PR's merge-base if main has moved; re-derive §1.5 first if
`structured_clone.rs` changed). `$h` = the PR head. `A` and `B` are the two files.

**G-1: line conservation with an exact residue.** Script, run as `bash conserve.sh $b $h`:

```bash
set -euo pipefail
b=$1; h=$2; t=$(mktemp -d)
git diff --no-color --no-renames -U0 "$b" "$h" -- crates/ > "$t/d"
grep -E '^-' "$t/d" | grep -vE '^--- (a/|/dev/null)' | cut -c2- | LC_ALL=C sort > "$t/rm"
grep -E '^\+' "$t/d" | grep -vE '^\+\+\+ (b/|/dev/null)' | cut -c2- | LC_ALL=C sort > "$t/add"
echo "removed=$(wc -l < "$t/rm") added=$(wc -l < "$t/add") hunks=$(grep -c '^@@' "$t/d")"
echo "moved (removed ∩ added, multiset)=$(LC_ALL=C comm -12 "$t/rm" "$t/add" | wc -l)"
echo "--- removed, not re-added:"; LC_ALL=C comm -23 "$t/rm" "$t/add" | sed 's/^/  |/'
echo "--- added, not moved:";      LC_ALL=C comm -13 "$t/rm" "$t/add" | sed 's/^/  |/'
rm -rf "$t"
```

The expected output, which is the probe's (`f9cf8c40` → `f16d4692`):

- `removed=166 added=180 hunks=5`
- `moved (removed ∩ added, multiset)=163`
- *removed, not re-added*, 3 lines: the two old `value`-import lines, and the old
  `super::structured_clone::ensure_empty_transfer_list(` line.
- *added, not moved*, 17 lines: the reflowed import line, the new call-path line, `mod.rs`'s two lines,
  and the binding header's 13 lines less the one blank line that pairs with the removed `:915` (two
  blank lines, seven `//!` lines, `#![cfg(feature = "engine")]` and three `use` lines).

The 5 hunks are: `mod.rs` 1, `pending_tasks.rs` 1, `structured_clone.rs` 2 (`@@ -53,2 +53 @@` and
`@@ -915,163 +913,0 @@`) and `structured_clone_binding.rs` 1. The PR body records the hunk count, which
is the row's requirement. Any line beyond these residues is an edit made while moving, and fails G-1.

**G-2: the binding block is the base block** (this is what pins the seam):
`diff <(git show $b:$A | sed -n 915,1077p) <(git show $h:$B | sed -n 14,176p)` → no output.

**G-3: the algorithm half is the base half less the import edit**:
`diff <(git show $b:$A | sed -n '1,52p;55,914p') <(git show $h:$A | sed -n '1,52p;54,913p')` → no output.
`git show $h:$A | wc -l` → 913, and `git show $h:$B | wc -l` → 176.

**G-4: the declared green guard.** The row requires zero content delta on the classification comments.
Over the pattern `P='§2\.[79]|Serializ|Deserializ'`, which matches every `§2.7.x` / `§2.9` tag and every
StructuredSerialize / StructuredDeserialize / `[Serializable]` comment in the file:
`diff <(git show $b:$A | grep -E "$P" | LC_ALL=C sort) <(git show $h:$A $h:$B | grep -E "$P" | LC_ALL=C sort)`
→ no output. That is 38 lines at the base. After F, the eight `§2.9` tags are 7 in `A` and 1 in `B` (the
`native_structured_clone` docstring, base `:933`). G-2 and G-3 already imply G-4. G-4 is stated
separately because it is the row's declared guard, and because it stays meaningful if the seam is ever
re-cut.

**G-5: behaviour.** `cargo clippy -p elidex-js --all-features --all-targets -- -D warnings` is clean.
`cargo test -p elidex-js --all-features` passes, and the PR's test list equals the base's (§4.1). Then
`mise run ci` before push.

### §2.2 The gates discriminate (mutants, run at the probe)

Each mutant is a commit on top of the probe in a second throwaway worktree.

| Mutant | What it does | G-1 | G-2 | G-3 | G-4 | Killed by |
|---|---|---|---|---|---|---|
| M1 edit-while-moving | inserts "an" into one comment inside the moved block (base `:969`) | moved 162, with one extra line in each residue | fail | pass | pass | G-1, G-2 |
| M2 seam drawn elsewhere | moves the `// Error helper` banner and `data_clone_error` into `B` as well | moved **173**; residue lists **identical** to the true cut | fail | fail | pass | G-2, G-3, G-1's exact count |
| M3 Pc's retag smuggled in | `— spec §2.9.` → `— spec §2.7.11.` in `B` | (one residue line each) | fail | pass | fail | G-4, G-1, G-2 |

M2 is P-2. It conserves lines, and its counts are equal, so the row's acceptance as written passes it. Only
the anchored checks and the exact moved count reject it.

### §2.3 `#11-vm-structured-clone-layering-seam`: the derivation is a separate artifact, and this review fires it

**Decision.** This memo does not carry the derivation's children or orderings. The derivation is its own
umbrella artifact, which this review fires. F mints nothing and re-points nothing for it.

**Evidence: the ledger row's own words** (`docs/plans/2026-07-vm-p4-slot-ledger.md:332`):

- Its Trigger is "Slice **F**'s mandatory `/elidex-plan-review` — the first forum with both halves in
  front of it". It adds: "this row is an umbrella, so the occasion fires its *derivation*, never a PR that
  would carry the work". And: "that derivation mints the terminal children, each its own PR under its own
  mandatory `/elidex-plan-review`".
- Its deliverable is "a derivation, not a doc edit and not a move to a home that already exists". Of its
  three questions, (2) must be "written at the mandate's **SSoT**, the repository's root `CLAUDE.md` §
  *Layering mandate*", and (3) asks "whether that warrant generalises" over a 128-file class (the row's own measurement, not re-run here).
- The row also says: "Distinct from `#11-vm-structured-clone-file-split` (Slice **F**) … which may not
  re-decide this".
- F's own row: "The layering question this row's size invites is not F's to decide, and is *open*".
- The same row, citing §1.0's rule: anything outside the class "goes to a §8 slot with an owner, never
  into a §5 row's acceptance".

**Why carrying it here would widen F.**

- The derivation's possible outputs include re-homing the algorithm half (a move into a new home or
  representation), a warrant written into the root `CLAUDE.md`, and a rule for the whole class the row measures. Each of those
  is a change in layering, which the brief's stop condition names.
- It is also edge-dense work under CLAUDE.md's rule: ≥3 intersecting axes (representation, admission
  warrant, class-wide generalisation, HTML §2.7.5 / §2.7.7 / §2.7.8 fidelity), and no canonical algorithm
  (the row measures both proposed homes shut). Such work needs an umbrella plan of its own, not a section
  inside a pure-move PR's plan.
- So running it here would make F's plan-review the review of a layering decision. That is the widening
  the PM has to see first. This memo neither runs it nor absorbs it.

**What "fires" means for this review.** The review records that the trigger has occurred. The review
closes with these obligations (L-5):

- The slot's Trigger cell and its live-ledger pointer row are marked *fired at Slice F's plan-review*
  (with the date and the closing commit).
- The derivation is opened as its own umbrella memo, for example
  `docs/plans/2026-11-vm-structured-clone-layering-seam.md`, under its own `/elidex-plan-review`. That
  memo mints the children and re-points the orderings the row names.

F hands the derivation its input: the two halves are separate files, and §1.2's naming is neutral to the
outcome. F's landing does not wait on the derivation.

**The consequence for NativeError PR-E1.**

- PM 374's edge reads: "E1 runs after F's plan-review and after any `#11-vm-structured-clone-layering-seam`
  child that review orders before F's consumers".
- Under this decision, F's review orders no child, because none exists yet. So no child is ordered before
  E1 by this review.
- Whether E1 should also wait for the derivation's own ordering is a sequencing question for the PM
  (§7 Q2). This memo does not answer it.
- The evidence for that question: E1 "adds no algorithm under `vm/host/`" (E1 memo, branch
  `vm-native-error-plan`, `git show vm-native-error-plan:docs/plans/2026-11-vm-native-error-e1-errordata.md | sed -n 165p`).
  A later re-home would therefore move E1's result along with the rest of the half, without
  re-deciding it.

---

## §3. Spec coverage map

F changes no behaviour, so no spec step changes branch. The table records each governing algorithm that
a moved or touched function implements, and which file holds it after F, so the review can check that
the move does not split one spec step across the seam. It does not. Each row's touch is entirely on one
side. The one crossing is HTML §2.7.11 step 1, at the single call `clone_value(ctx.vm, value)`. Section
numbers are from `.claude/tools/webref heading html 2.7` and `webref dfn html …` / `webref heading webidl 3.2.21`.
Steps are from `webref body html dom-structuredclone` / `body html structuredserializewithtransfer` /
`body webidl create-sequence-from-iterable`. §6 has the commands.

| Spec section | Step | Branch | Touch (site after F) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §2.7.11 Structured cloning API (`structuredClone(value, options)` method steps) | 1-3 | step 1 → StructuredSerializeWithTransfer(value, options["transfer"]); steps 2-3 the fused deserialize | `native_structured_clone` → `vm/host/structured_clone_binding.rs` (moved, byte-identical); its one call into the algorithm is `clone_value` | ✓ (no branch changes) | yes — `value`, `options` |
| WHATWG HTML §9.4.4 Message ports (`StructuredSerializeOptions` dictionary, `transfer` member) | dictionary conversion | `undefined`/`null` → default; Object → `Get("transfer")`; other → TypeError | `validate_transfer` → binding file (moved) | ✓ | yes |
| WHATWG HTML §2.7.8 StructuredSerializeWithTransfer ( value, transferList ) | 2 | engine restriction: any non-empty transferList → DataCloneError (unchanged) | `ensure_empty_transfer_list` → binding file (moved) | ✓ | yes |
| Web IDL §3.2.21.1 Creating a sequence from an iterable | 1-3 | `sequence<object>` conversion of `transfer`. The `iter_close` on the non-empty branch is Pc's removal site, which moves from base `:1064` to binding `:163` | `ensure_empty_transfer_list` → binding file (moved) | ✓ | yes |
| WHATWG HTML §9.3.3 Posting messages (window post message steps) | transfer argument | same conversion, through the shared helper | `vm/host/pending_tasks.rs:673` (call path only, §1.3) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal ( value, forStorage [ , memory ] ) | all | fused with StructuredDeserialize, per the docstring at `:63-64` | `clone_value` … `data_clone_error` → stays in `vm/host/structured_clone.rs` (lines ≥ `:55` shift by −1) | ✓ | yes |
| WHATWG HTML §2.7.7 StructuredDeserialize ( serialized, targetRealm [ , memory ] ) | all | output construction in the per-kind helpers | stays in `vm/host/structured_clone.rs` | ✓ | yes |

The map is breadth, not edge density: seven rows, one function each, no step touched. The single-PR
verdict is §0's "terminal by necessity".

---

## §4. Tests, mutation, size

### §4.1 Tests

There are no new tests. A pure move has no new behaviour to test. Its regression risk is a broken path
or visibility, which the compiler catches, plus an edit made while moving, which G-1 to G-4 catch.

The existing suites that run through both halves are the ones that must stay green. Test and call counts
are from `/usr/bin/grep -cE '^\s*#\[test\]'` and `/usr/bin/grep -cE 'structuredClone\(|postMessage\('`:

- `vm/tests/tests_structured_clone.rs`: 38 tests. It reaches every per-kind helper and `options.transfer`
  (the `options_transfer_*` tests, `:384` on).
- `vm/tests/tests_post_message.rs`: 32 tests. It covers the `ensure_empty_transfer_list` consumer on the
  `postMessage` path, including the WebIDL iterable cases (`:502` on).
- `tests_worker.rs`, `tests_typed_array_extras.rs`, `tests_indexeddb_value.rs` and
  `tests_indexeddb_events.rs`, which reach `clone_value` through IndexedDB.
- The `*_throws_data_clone_error` tests in `tests_realtime.rs`, `tests_screen.rs`, `tests_text_encoding.rs`
  and `tests_visual_viewport.rs`.

The acceptance is the whole `cargo test -p elidex-js --all-features` passing, plus a test list equal to the
base's: `cargo test -p elidex-js --all-features --lib -- --list | /usr/bin/grep -c ': test$'` at `$b` and at
`$h`. A move must neither add nor lose a test.

Probe result for `cargo test -p elidex-js --all-features --lib`:

- **Test list.** 6524 at `f16d4692` and 6524 at `f9cf8c40`.
- **Base.** `f9cf8c40`: 6524 passed.
- **Probe, second run.** `f16d4692`: 6524 passed.
- **Probe, first run.** `f16d4692`: 6523 passed and 1 failed:
  `respond_with_survives_gc_in_listener` (in `tests_service_worker`), with "SW reply within 5s:
  Timeout" (`vm/tests/tests_service_worker.rs:94`). That run took 176 s, against 55 s and 96 s for the
  other two, and the test passed 3 of 3 times when run alone. This is a wall-clock timeout under
  machine load. It is not a move effect: G-2/G-3 hold on that commit, so any of the moved code the
  test reaches is byte-identical to the base.
- **Implementation PR.** If this test fails in the PR, re-run it alone before reading the failure as a
  regression. Where it should be tracked is the driver's to decide, and is not F's.

### §4.2 Mutation

The mutants of §2.2 test the gates, which is what a pure move's acceptance consists of. Mutating the
moved code would test the existing suites rather than this PR, so there are none.

### §4.3 The 1000-line verdict, per resulting file

| File | After F | Verdict |
|---|---|---|
| `vm/host/structured_clone.rs` | 913 | under 1000. The remaining cohesion question (where a ~913-line WHATWG algorithm belongs) is layering, not size, and it is `#11-vm-structured-clone-layering-seam`'s (§2.3) |
| `vm/host/structured_clone_binding.rs` (NEW) | 176 | under 1000 |
| `vm/host/mod.rs` | 374 (372 + 2) | under 1000 |
| `vm/host/pending_tasks.rs` | 739 (unchanged count) | under 1000 |

---

## §5. Landing obligations

L-1 and L-2 edit family memos. Per §1.7 they land in F's PR as a separate docs commit, unless §7 Q1 rules
otherwise. None of them is done by this memo.

- **L-1. Row F** (`docs/plans/2026-07-vm-p4-slice-plan.md`, `git grep -n '^| \*\*F\*\* |'`):
  - Re-point `register_structured_clone_global` `:922` → `:925` at the base (P-1).
  - Correct "a seam drawn elsewhere fails it" to name the anchored block identities and the exact moved
    count as the separating condition (P-2, M2).
  - Replace "914 and ~162" with the measured 913 / 176 and the −1 shift (P-3).
  - Add the post-split paths: the algorithm at `vm/host/structured_clone.rs`, the binding at
    `vm/host/structured_clone_binding.rs`, citing this memo by backticked path.
- **L-2. Row Pc** (`git grep -n '^| \*\*Pc\*\* |'`), whose `Deps` names F:
  - Its `Primary module(s)` cell gains `vm/host/structured_clone_binding.rs`, because its `iter_close`
    removal site moves there. The red probe "no `iter_close` call remains at
    `vm/host/structured_clone.rs:1064`" re-points to that file (`:163` at F's landing). The anchored
    `git grep -n 'iter_close' 658cc302 -- …` evidence stays as written.
  - Its retag population now spans both files. The base count of 8 `§2.9` tags becomes 7 + 1. The
    algorithm half's unanchored line cites from `:55` on shift by −1, and Pc re-derives them at its own
    parent HEAD, as its row already requires.
  - Pc's pass also owns the algorithm file's module docstring line `:1`, which names the global that now
    lives in the binding file (§0.4).
  - The same −1 shift reaches the slot ledger's `#11-vm-webidl-sequence-early-exit-drain` row
    (`docs/plans/2026-07-vm-p4-slot-ledger.md:329`) and `#11-webidl-sequence-dense-array-fast-path`'s
    "Sites at HEAD" (`:326`, `structured_clone.rs:1009` → `structured_clone_binding.rs:108`). Both are
    unanchored, so they are re-pointed in the same commit.
  - The `658cc302`-anchored cites in `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md`
    (`:105`, `:117`, `:120`, `:160`) and the touch-time split record's blob-anchored measurement are left
    as they are, because they are anchored. The record's "re-run rather than reading forward" already
    tells a reader that `structured_clone.rs` is now 913 lines.
- **L-3. F → E1.** F changes no function E1 relies on. G-2/G-3 prove that `clone_recursive`, `classify`,
  `clone_ordinary`, `clone_error`, `is_error_like_proto` and `clone_value` stay byte-identical, at the
  same path. `native_structured_clone`, one of the three `clone_value` callers that E1 counts, moves to
  the binding file unchanged, so E1's count of three still holds. The only thing E1 sees is the −1
  shift of its `f9cf8c40` locators from `:55` on (for example the guard arm `:133` → `:132`). E1 already
  treats those lines as locators only and cites by function. E1 rebases onto F. Nothing in E1's memos is
  edited by F.
- **L-4. Ledgers.**
  - The repository slot ledger's F registration sentence (`docs/plans/2026-07-vm-p4-slot-ledger.md`
    `:178`-`:182`: "`grep -c -- '#11-vm-structured-clone-file-split' <ledger>` → **0**") is re-measured.
    It is 2 in the live ledger today, so F's landing records it as registered.
  - The live ledger (`/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/project_open-defer-slots.md`)
    is PM-owned. The driver reports to the PM: F landed (squash sha); the pointer row of
    `#11-vm-structured-clone-file-split` closes; the F → E1 edge is satisfied.
- **L-5. Layering-seam trigger** (§2.3). Recorded at this review's close, not at F's landing:
  - the slot row's Trigger cell (`docs/plans/2026-07-vm-p4-slot-ledger.md:332`) and its live-ledger
    pointer row are marked *fired*;
  - the derivation memo is opened as a separate artifact.

  The PM sees this before anything is minted.
- **L-6. The family checker and the layout guard** on every commit that edits a family memo:
  - `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md` → rc 0
  - `python3 -m unittest docs/plans/test_vm_p4_family_layout.py` → OK
  - `bash scripts/python-suites.sh` → green
- **L-7. No relative link into this memo.** L-1/L-2 cite this memo as the backticked path
  `docs/plans/2026-11-vm-p4-slice-f-structured-clone-split.md`. A relative link from a family memo to a
  `.md` outside `2026-07-vm-p4-*` turns the layout guard red (`docs/plans/test_vm_p4_family_layout.py`,
  header), and that guard's own remedy is to use a backticked path.
- **L-8. In-flight branch overlap**, measured with `git diff origin/main...<branch> --stat` at `f9cf8c40`.
  No branch touches any of F's four Rust files, so the only interaction is line-number shifts in the
  family memos that L-1/L-2 edit.
  - `vm-p4-slice-1a-plan` (`d1e9186a`, merge-base `145927c9`) edits family memos only, and touches
    neither row F nor row Pc nor the slot-ledger rows `:326` / `:329` / `:332`. It inserts five lines
    between Pc and F (its hunk `@@ -943,2 +943,7 @@`), so row F becomes `:967` once it lands, and it adds
    a row after the ledger's `:332`. Whichever lands second re-runs the `git grep -n '^| \*\*F\*\* |'`
    anchors rather than reading `:962` forward.
  - `rc2a-clone-created` (`e51fcca1`, merge-base `f9cf8c40`) touches 29 files, none of them under
    `vm/host/` among F's four (its `vm/host/` files are `custom_elements/upgrade.rs`,
    `html_template_proto.rs` and `node_methods_extras.rs`), and no family memo. No overlap.
  - `vm-native-error-plan` (`7fc0dbac`, merge-base `f9cf8c40`) edits the slot ledger at one line, `:293`
    (`#11-vm-derived-constructor-this-binding`), which is not a row L-2/L-4/L-5 touch. Its two new memos
    are read by L-3 and are not edited.

---

## §6. Re-derivation

Run from the repository root at `f9cf8c40` unless noted.

- Shape: `wc -l crates/script/elidex-js/src/vm/host/structured_clone.rs` → 1077;
  `git grep -n '^// \|^pub\|^fn\|^impl' -- crates/script/elidex-js/src/vm/host/structured_clone.rs`;
  `git grep -n '    pub(in crate::vm) fn' -- <same>` → `:925`.
- No drift since the row's baseline: `git diff --stat 658cc302 f9cf8c40 -- <same>` → empty.
- Import use per half:
  `for id in HashMap Arc 'shape::' PropertyAttrs JsValue NativeContext ObjectId ObjectKind PropertyKey PropertyStorage PropertyValue VmError VmInner BlobData; do echo "$id $(sed -n 58,914p <file> | /usr/bin/grep -c -- "$id") $(sed -n 915,1077p <file> | /usr/bin/grep -c -- "$id")"; done`
  → `NativeContext 0 3`. Every other name is nonzero in the first column.
- Cross-module consumers: `git grep -n 'structured_clone::\|ensure_empty_transfer_list\|register_structured_clone_global' -- crates/script/elidex-js/src`.
- Cite drift in the family memos: `git grep -noE 'structured_clone\.rs[^|]{0,3}:[0-9]+' -- 'docs/plans/2026-07-vm-p4-*.md'`.
- `§2.9` count per file after F: `grep -c '§2\.9' <A> <B>` → 7 / 1 at the probe.
- Spec numbers: `.claude/tools/webref heading html 2.7`; `webref dfn html structuredClone` (→ §2.7.11);
  `webref dfn html StructuredSerializeOptions` (→ §9.4.4); `webref dfn html "window post message steps"`
  (→ §9.3.3); `webref heading webidl 3.2.21` (→ §3.2.21.1). Steps from `webref body html dom-structuredclone`,
  `webref body html structuredserializewithtransfer` and `webref body webidl create-sequence-from-iterable`.
- Branch overlap: `git diff origin/main...<branch> --stat` and `git diff -U0 origin/main...<branch> -- docs/plans/2026-07-vm-p4-slice-plan.md docs/plans/2026-07-vm-p4-slot-ledger.md | /usr/bin/grep '^@@'`.
- Probe: the §1.6 recipe in `git worktree add --detach <dir> f9cf8c40`, then `cargo fmt --all -- --check`,
  `CARGO_TARGET_DIR=<scratch> cargo clippy -p elidex-js --all-features --all-targets -- -D warnings`,
  `cargo test -p elidex-js --all-features --lib`, and G-1 to G-4 against the probe commit. Mutants
  M1–M3 are single commits on top of it, each checked with the same gates.

---

## §7. Questions for the plan-review

- **Q1. Where L-1/L-2/L-4 land.** This memo's proposal is commit 2 of F's PR (§1.7). The gates are
  pathspec-limited to `crates/`, so the squash commit still measures as a pure move. The alternative is
  a follow-up docs PR, which leaves the family memos' unanchored cites stale for its duration.
- **Q2. E1 against the derivation** (§2.3). F's review orders no layering-seam child, so PM 374's edge
  places nothing before E1 from this review. Should E1 also wait for the derivation's own ordering
  output, or only for F's landing? This is the PM's call. The evidence is E1's statement that it adds no
  algorithm under `vm/host/`.
- **Q3. The name `structured_clone_binding`** against `structured_clone_global`. §1.2 gives the reason for
  `_binding`. A rename costs nothing before landing and a follow-up PR after it.

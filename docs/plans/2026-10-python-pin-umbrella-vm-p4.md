# Python pin umbrella — VM P4 coupling (companion of `2026-10-python-pin-umbrella.md`)

This file is `docs/plans/2026-10-python-pin-umbrella-vm-p4.md`, one of the umbrella's four files (body §0 *Layout*,
§6 *This umbrella's own length*); it lands with the body in #581 (PM decision 221). It holds the VM P4 lane's files
and lines that PR-1 and PR-2 edit, and **§5.3 (c), the consent subject**, for the PM to show that lane (decisions
127 (a), 205, 211, 213, 221). Section numbers are the body's (body: `docs/plans/2026-10-python-pin-umbrella.md`).
Base `ee6c7a1a`.

### §5.2 The VM P4 lane's files that PR-1 touches (for the PM to show the VM P4 lane — decisions 127 (a), 205, 211, 213)

The plan-memo tool family (`.claude/tools/plan_memo*.py`, `plan-memo-umbrella-check.py`,
`test_plan_memo_umbrella_check.py`, `plan_memo_selftest_manifest.txt`), #510's memo, #506's memos
(`docs/plans/2026-07-vm-p4-*.md`, `docs/plans/2026-10-vm-p4-memo-seam-split.md`) and its layout guard
`docs/plans/test_vm_p4_family_layout.py`, and CLAUDE.md's "Plan-memo umbrella checker" bullet are the VM P4
lane's; PR-1 edits them only with that lane's consent, obtained by the PM (decisions 117 (2), 127 (a), 205, 211,
213, 221; the one list is §5.3 (c)), and runs CLAUDE.md:55's checker after editing a family memo (§5.3 (c)).
**Derivation** at `ee6c7a1a` (the family's sources and #510's memo are unchanged since `74e15df1` but for
`:663`, §0, so V1–V13 and V15 keep their lines; `CLAUDE.md` changed, V14 is re-read at `ee6c7a1a`; #506's files are on `main` and the pathspecs below match them — draft 6's
"they match nothing until #506 merges" no longer holds; PR-1 re-runs both stages at its own base):

```sh
# stage 1 — PR-1 §4.6's construct search, extended with versions below the pin and import fallbacks.
# No \b: `git grep -E` on the measuring host (macOS, git 2.56.0) matches nothing for \b, so word edges are spelled out.
git grep -nE '3\.(9|1[0-4])([^0-9x]|$)|"3\.x"|\(3, ?9\)|(^|[^A-Za-z])(floor|legs?|matrix)([^A-Za-z]|$)|setup-python|python-version|Python 3\.[0-9]+\+|version_info|except ImportError' ee6c7a1a -- '.claude/tools/plan_memo*' '.claude/tools/plan-memo*' '.claude/tools/test_plan_memo*' 'docs/plans/2026-08-plan-memo-umbrella-checker*.md' 'docs/plans/test_vm_p4*' 'docs/plans/2026-07-vm-p4-*' 'docs/plans/2026-10-vm-p4-memo-seam-split.md'
git grep -nE '両 leg|matrix' ee6c7a1a -- CLAUDE.md
# stage 1b — the "0 hits" class (V18–V21, V23): a ledger claim tied to #506's landing, which has happened
git grep -nE '0 hits|not (yet )?in the slot SoT ledger|owed at #506' ee6c7a1a -- 'docs/plans/2026-08-plan-memo-umbrella-checker.md' 'docs/plans/2026-07-vm-p4-*' 'docs/plans/2026-10-vm-p4-memo-seam-split.md'
# stage 2 — every site coupled to a code branch stage 1 found, by the branch's own tokens
git grep -niE 'except \(OSError, RuntimeError\)|symlink[- ]loop|py(<=|≤)3\.12|3\.9[–-]3\.12' ee6c7a1a -- '.claude/tools/plan_memo*' '.claude/tools/plan-memo*' 'docs/plans/2026-08-plan-memo-umbrella-checker.md'
git grep -nE 'sre_parse|re import _parser' ee6c7a1a -- '.claude/tools/plan_memo*' '.claude/tools/plan-memo*'
# stage 1c — a call the pin deprecates (V22): module-level re.sub / re.subn / re.split with a positional
# count / maxsplit / flags. Read each hit's arguments (17 hits at ee6c7a1a; one live, sibling.py:158). A lower
# bound by symptom; the property is measured over the paths the set executes, by running it on the pin with every
# DeprecationWarning shown (R§0: sibling.py:158 is the only one at ee6c7a1a; what that run cannot see: body §11),
# which PR-1 re-runs at its base (C35).
git grep -nP '(^|[^A-Za-z0-9_.])(re|_re)\.(sub|subn|split)\(' ee6c7a1a -- '*.py'
# stage 2c — every copy of V22's line, by its own text (5: the line and the two mutation rows' find / replace)
git grep -nF 'dest, 1)' ee6c7a1a --
```

**Manifest rule** (`.claude/tools/plan_memo_selftest_manifest.py`): each manifest line carries a digest of a
different thing, by row kind — a **CONTROL** line, of the control function's whole source block, comments and
docstring included (`_source_digest`, `:196-222`); a **CASE** line, of `repr(tuple(case))` (`_case_digest`,
`:154-155`); a **MUTANT** line, of the row's `(find, replace)` pair, plus the names of the controls it targets
(`lines`, `:233-251`). So an edit inside a control's block changes its CONTROL line; an edit to the text a
mutation row finds or writes changes its MUTANT line; a deleted row removes its line. PR-1 regenerates with
`python3 .claude/tools/plan-memo-umbrella-check.py --write-manifest` (through mise's shim, i.e. on the pin) and
commits the diff. Module docstrings, helper functions and comments outside a digested block change no line.

| # | File:line (`ee6c7a1a`; V1–V13, V15 unchanged since `74e15df1`) | Text / construct | Class (PR-1 §4.6) | PR-1 edit | Manifest impact |
|---|---|---|---|---|---|
| V1 | `.claude/tools/plan-memo-umbrella-check.py:159` | "(both matrix legs)" (module docstring, WHERE THIS RUNS) | statement | "(the `tools` job's one leg, the pinned interpreter)" | none (module docstring) |
| V2 | `.claude/tools/plan-memo-umbrella-check.py:660-661` | "`reconfigure` … from Python 3.7 (below the 3.9 floor this checker advertises)" (`_utf8_streams` docstring) | statement | "(long before the pinned interpreter)"; the `getattr` reason (a replaced stream) is unchanged | none (`stream_encoding_control` exercises the function; its own block is untouched) |
| V3 | `.claude/tools/plan_memo_selftest_harness.py:25-26` | "The checker advertises "Python 3.9+, standard library only"" (module docstring) | statement | "The checker runs on the pinned interpreter (`.tool-versions`), standard library only" | none |
| V4 | `.claude/tools/plan_memo_selftest_properties.py:460-462` | "The checker advertises Python 3.9+ with only the standard library" (in `encoding_sweep_control`'s docstring) | statement | as V3 | **yes** — CONTROL "PROPERTY: no source of this checker performs text I/O without naming its encoding …" |
| V5 | `.claude/tools/plan_memo_selftest_properties.py:310-313, 325-329` | `_leading_unbounded_repeat`: `try: from re import _parser` / `except ImportError: import sre_parse` ("CPython 3.9 / 3.10"), and the docstring's "renamed in 3.11, so both names are tried" | **floor-motivated code branch** — unreachable under the pin, and it would fail if reached (R§4.1 row 3x) | remove the fallback: `from re import _parser as parser` only; the docstring keeps "unreachable parser → the control is RED", drops the two-name sentence | none (`_leading_unbounded_repeat` is a top-level helper at `:302`, outside `leading_run_scan_control`'s block at `:352`) |
| V6 | `.claude/tools/plan_memo_sibling.py:151-153, 174-183` | `_resolve`: `except (OSError, RuntimeError)`, "on Python 3.9-3.12, `RuntimeError` for a symlink loop (3.13 made that an `OSError`)" | **floor-motivated code branch** if no other `RuntimeError` can leave `Path.resolve()` on the pin — **the VM P4 lane's answer decides** (§11); both outcomes below the table | outcome (i): `except OSError`; both docstrings name `OSError` only | none in this file; coupled rows V7–V11, V15 change |
| V7 | `.claude/tools/plan_memo_selftest_mutants_pr510.py:202-205` | mutant row "R8-2 sibling: an OSError from resolve() is the unavailable-sibling miss (unguard it)", whose *find* text contains `except (OSError, RuntimeError):` | coupled to V6 | *find* text follows V6 (else the row no longer applies and the self-test is red) | **yes** — that MUTANT line (its `(find, replace)` digest) |
| V8 | `.claude/tools/plan_memo_selftest_mutants_pr510.py:304-306` | mutant row "RG2 IMP-3: RuntimeError from resolve() is guarded with OSError" | coupled to V6 — its subject no longer exists | delete the row | **yes** — that MUTANT line is removed |
| V9 | `.claude/tools/plan_memo_selftest_controls.py:178-189` | `unavailable_sibling_control`: docstring "OR, on Python 3.9-3.12, a `RuntimeError` (a symlink loop)", and the injected `RuntimeError("Symlink loop …")` arm | coupled to V6 | drop the `RuntimeError` arm and its sentence; the `OSError` arm stays | **yes** — CONTROL "an OSError from resolve() is the unavailable-sibling schema miss, never an exception" |
| V10 | `.claude/tools/plan_memo_selftest_controls.py:719-726` | `parse_runtime_error_control` docstring: "for the py<=3.12 symlink-loop case `_resolve` alone guards" | coupled to V6 (a present-tense claim about `_resolve`) | restate as history ("…which `_resolve` guarded until <PR-1>"); the control's property is unchanged | **yes** — CONTROL "a RuntimeError raised while PARSING a memo is a crash …" |
| V11 | `.claude/tools/plan_memo_population.py:18-21, 81-84` (unchanged by #569, re-read at `74e15df1`) | comments: "the py<=3.12 symlink-loop case that `plan_memo_sibling._resolve` alone guards" | coupled to V6 | restate as V10 | none (not a digested block) |
| V12 | `docs/plans/2026-08-plan-memo-umbrella-checker.md:73` | §1 interim connection, "on both matrix legs" | live rule (§5.1) | amended in place, dated marker | — |
| V13 | `docs/plans/2026-08-plan-memo-umbrella-checker.md:1116-1117` (trigger `:1123-1125` named, unchanged) | §8 timeout contract (F4) | live rule (§5.1) | amended in place, dated marker | — |
| V14 | `CLAUDE.md:54` (Workflow, "Plan-memo umbrella checker") | "`tools` job (`scripts/python-suites.sh`、matrix の両 leg)" | statement | "(pin の 1 leg)" — a plain edit of the bullet as #506 rewrote it (§5.3 O1) | — |
| V15 | `docs/plans/2026-08-plan-memo-umbrella-checker.md:434` (§3 coverage row of the sibling resolver) | "`_resolve` = `resolve()` with `OSError` or (Python 3.9–3.12 symlink loop) `RuntimeError` read as an UNAVAILABLE sibling", and the control cell "(OSError and RuntimeError injected)" | coupled to V6/V9 — a present-tense description of the code (R3 MIN; found by stage 1, missed by draft 3) | amended in place with a dated marker under outcome (i): `OSError` only, control injects `OSError` | — |
| V16 | `docs/plans/2026-10-vm-p4-memo-seam-split.md:356-357` (#506's split plan, §1.5 *Scope note*; was O5) | "The `"3.9"` / `"3.x"` matrix exists only in GitHub's `tools` job, so the 3.9 floor is measured by CI, not by `/pre-push`; the guard uses no syntax newer than 3.9." | statement in a landed memo (present tense) | amended in place, dated marker: "since <PR-1> the `tools` job has one leg, the pin (`.tool-versions`), which `mise run tools-test` / `/pre-push` also run; the sentence is the record of the split's date". The sentence before it (`:354-356`, the driver runs "under the first `python3` on `PATH`") stays true after PR-1 (Step 1 then admits only the pin) and is not edited | — |
| V17 | the same file, `:804-806` (§3 step 2's last check; was O6) | "… re-derives `timeout-minutes` in the same PR from its own run, the slower leg bounding the job): the split PR reads its own run's slower leg …" | statement in a landed memo (a procedure, done by the split PR) | amended in place, dated marker: "two legs at the split's run; one leg, the pin, since <PR-1>, whose run re-derives the timeout (§5.1)" | — |
| V18 | `docs/plans/2026-08-plan-memo-umbrella-checker.md:1097-1104` and the trigger sentence of the same entry, `:1105-1108` (the "0 hits" note of PORTFOLIO:290 (PM 188), :293 (PM 191) and :308 (the PM handoff); R8 A5-5) | the slots `#11-plan-memo-acceptance-falsifiability-check` and `#11-plan-memo-spec-field-single-home-check` "not (yet) in the slot SoT ledger … (0 hits)", "registration owed at #506's landing", "Trigger = #506's landing" | live statement in a landed memo, false since **#510's** landing (`282f04d6`, 2026-10-26), not #506's (R7 J3): each slot has one line in the slot SoT ledger's homing index *Registered in a plan memo (from MEMO rows)*, whose own words are "added to the index at #510's landing, 2026-10-26" and "added at #510's landing, 2026-10-26", with "Re-eval: 2027-01-30 (set 2026-11-01, PM 191)" (`/usr/bin/grep -n -- '<slot id>' memory/project_open-defer-slots.md` → one line each; 2026-11-01) | amended in place, dated marker: "since #510's landing (`282f04d6`, 2026-10-26) the slot SoT ledger's homing index lists this slot; the absence and the trigger this note stated did not hold — amended <PR-1's date> by PR-1" (spelled so that stage 1b no longer matches it); the ledger, not this note, holds the re-eval date | — |
| V19 | `docs/plans/2026-07-vm-p4-slot-ledger.md:58-60` (the `#11-dead-opcode-removal` adoption bullet; decision 211 D7-1) | "**Still absent from the SoT ledger** (0 hits, present only in the `m4-12-pr-d17b-*` landing memos), as is `#11-d17b-dispatch-expr-file-growth` below — so both adoptions are genuinely outstanding" | V18's class: live statement in a landed memo, false since #506 landed. Homing (2026-11-01): `/usr/bin/grep -n -- '#11-dead-opcode-removal' memory/project_open-defer-slots.md` → one line, and `… -- '#11-d17b-dispatch-expr-file-growth' …` → one line, both under the heading "VM P4 slot-ledger pointers (homed 2026-11-01 at #506 `3ec0ae5c`; PM decision 192)" (cited by slot ID and heading, not by line: the live ledger's lines move, R7 A5-4); each is an index row naming this memo as canonical, whose own rows are V20 / V21 | amended in place, dated marker: "both listed in the slot SoT ledger's pointer index since #506's landing (`3ec0ae5c`, PM decision 192), which names this memo as canonical — amended <PR-1's date> by PR-1" (not "registered": an index line is not a registration by this memo's own rule, `:278`); the ledger holds the re-eval dates | — |
| V20 | the same file, `:322` (the `#11-dead-opcode-removal` row, *Why* cell) | "**absent from the live ledger** (0 hits), so the adoption is genuinely outstanding" | as V19 | as V19, in the cell; the re-eval cell ("at registration: landing + 30 d") is a rule and is not edited | — |
| V21 | the same file, `:323` (the `#11-d17b-dispatch-expr-file-growth` row, *Why* cell) | "**absent from the live ledger** (0 hits)" | as V19 | as V20 | — |
| V22 | `.claude/tools/plan_memo_sibling.py:158` (decision 211 D7-2), and its copies in `.claude/tools/plan_memo_selftest_mutants_codex0927.py:51-52, 56-57` (the find / replace of the rows "R38 of 2026-09-27 agree" and "… run"; stage 2c) | `raw = re.split(r"[#?]", dest, 1)[0]` — `maxsplit` passed positionally | **a call the pin deprecates**: on 3.15.2, `python3.15 -I -W error -c 'import re; re.split(r"[#?]", "a#b", 1)'` → `DeprecationWarning: 'maxsplit' is passed as positional argument` (also the guard's `-W error` run, R§0); no verdict changes today (the driver runs without `-W error`). The keyword form is accepted on both measured interpreters (`… re.split(r"[#?]", "a#b?c", maxsplit=1)` → `['a', 'b?c']` on 3.15.2 and on macOS 3.9.6; 2026-11-01) | `re.split(r"[#?]", dest, maxsplit=1)[0]`, and both mutation rows' find / replace re-spelled the same way in the same commit — else `plan_memo_selftest_mutants.py:567-570` reports "substring occurs 0 times". **If the VM P4 lane prefers to fix it itself**, it does, and V22 leaves PR-1's edit set (PR-1's stage 1c then finds no live hit) | **yes** — two MUTANT lines (`plan_memo_selftest_manifest.txt:2122`, `:2126`: the digest is of `(find, replace)`) |
| V23 | `docs/plans/2026-07-vm-p4-slot-ledger.md:327` (the `#11-plan-memo-spec-field-single-home-check` row, *Trigger* cell; PM decision 213) | "The same memo states this slot's registration is owed at #506's landing with no date (its slot list, the entry for the four assertions' owner), which is the *at registration* rule this table now carries." | coupled to V18: a report of the very note V18 amends (#510's memo `:1101-1108`) — true at `ee6c7a1a`, false once V18 lands. The slot is homed: one line in the live ledger's homing index (`/usr/bin/grep -c -- '#11-plan-memo-spec-field-single-home-check' memory/project_open-defer-slots.md` → 1; 2026-11-01), and the slot is named at four lines of this memo (`git grep -n -- '#11-plan-memo-spec-field-single-home-check' ee6c7a1a -- docs/plans/2026-07-vm-p4-slot-ledger.md` → `:117`, `:278`, `:327`, `:328`; R8 A5-2, re-run 2026-11-01), of which V23 edits `:327` only | amended in V18's commit, dated marker: "#510's memo stated, until <PR-1>, that this slot's registration waited on #506's landing, with no date (…)" (spelled so that stage 1b no longer matches it); the rest of the cell is unchanged | — |

**V6's two outcomes** (R3 MIN; the VM P4 lane answers through the PM before PR-1's implementation):
- **(i) no other `RuntimeError` can leave `Path.resolve()` on the pin** — V6 removes the `RuntimeError` arm, and
  V7–V11 and V15 follow as tabled.
- **(ii) one can** (the lane names it, e.g. a `RecursionError`, which is a `RuntimeError`) — the arm is not
  floor-motivated, so V6 **keeps** `except (OSError, RuntimeError)` and re-justifies it on the pin (its docstrings
  name the pin-reachable cause, not "3.9–3.12"); V7 and V8 stay (V8's subject exists); V9 keeps its `RuntimeError`
  arm with the new reason (CONTROL line changes only if its docstring does); V10, V11 and V15 are restated to the
  new reason. **Line headroom (R8 A5-7)**: `plan_memo_selftest_controls.py` is 997 lines at `ee6c7a1a` and
  `line_bound_control` reds at 1000 (R§0), so V9's new reason replaces the "3.9-3.12" sentence within 2 added
  lines at most; if it cannot, the file's split is the VM P4 lane's prereq before PR-1 (body §6 *1000-line rule*). The class changes from "floor-motivated code branch" to "statement", and §1's "no code for an
  interpreter nothing admits" is met by the re-justification.

Stage-1 hits **not** edited, each with its class: dated measurement records naming the interpreters they ran on,
each with a live control that measures the fact on whatever interpreter runs (`plan_memo_emphasis.py:103-104,
113`, `plan_memo_tokens.py:108`, `plan_memo_selftest_harness.py:428`), and in #510's memo (`:1647`, `:1687`,
measured on 3.14.9 / 3.14.7 at named commits); history that stays true (`plan_memo_selftest_manifest.py:200-204`,
the bytecode digest's version dependence); a reason that holds under the pin (`plan_memo_sibling.py:189-196`:
`is_reserved()` is removed in 3.15 and `os.path.isreserved` is `ntpath`-only); words that are not about
interpreters (`floor` at `plan_memo_emphasis.py:298-300`, `plan_memo_selftest_growth.py:81`,
`plan_memo_selftest_records.py:641-646`, `plan_memo_selftest_mutants_codex0927.py:37`,
`plan_memo_selftest_properties.py:716`, #510's memo `:1472` ("FOUR is a floor"); "matrix" at the memo's `:1575`
and at `CLAUDE.md:30` (the *Edge-dense work* rule's edge matrix); ratios such as "×3.9" in the rounds record; and
every stage-1 hit in #506's nine `2026-07-vm-p4-*.md` memos — spec section numbers such as "§3.10.21" or
"§13.3.9.1" that match `3\.1[0-4]`, and "edge matrix"); CI/tools-owned text outside the VM P4 lane (`CLAUDE.md:100`
at `ee6c7a1a`, the `## CI` paragraph's `"3.9"` / `"3.x"` matrix — rewritten by PR-1 under §4.4, not a VM P4 edit);
and the rounds record
`docs/plans/2026-08-plan-memo-umbrella-checker-rounds.md` as a whole (a landed record).
**Stage 1b's hits, all ten.** V18 (`:1099`, `:1100`, `:1103` of #510's memo) and V19–V21 (`:58`, `:322`, `:323` of
#506's slot ledger; decision 211 D7-1, which named exactly these three) and V23 (`:327`) are edited. `:70`, `:208` of #506's memos
and #510's `:1307` are dated measurements. **`:327`** of the slot ledger is **V23** (PM decision 213).
**Found by draft 8, not PR-1's — owner and time set by PM decision 220** (`PORTFOLIO:357`: "VM P4 is also told to
fix slot-ledger :278/:327/:328 (index rows now carry Re-eval 2027-01-30, PM 191) at its next touch"): the slot
ledger's `:278` says the live ledger's homing index lists the two `#11-plan-memo-*` slots "with no Re-eval of their
own, so those two rows read *at registration*" — since 2026-11-01 each index line carries "Re-eval: 2027-01-30 (set
2026-11-01, PM 191)"; so `:278` and the *Re-eval* cells of `:327` / `:328` ("at registration: landing + 90 d") may
be stale. **Owner: the VM P4 lane, at its next touch of that memo**; not PR-1's (R8 A3-4). V23 edits `:327`'s
*Trigger* cell only: whichever of V23 and the lane's fix lands second re-reads `:327` at its base (a textual
overlap, the §8 *small checker-family PRs* rule), and the consent request names the overlap (§5.3 (c)).
Also for the VM P4 lane, outside any file: its manifest-regeneration practice ("byte-identical on
3.9/3.12/3.14", #562's commit body) becomes "regenerated on the pin".

**CI/tools-owned text that #510 wrote** (shown for completeness, not a VM P4 edit): `ci.yml`'s `tools` comment,
the timeout paragraph (`.github/workflows/ci.yml:224-235` at `ee6c7a1a`, unchanged since `74e15df1`; PR #510's sentences at `:230-235`) —
rewritten under its rule by PR-1 (the dated-measurement class).

**The landing memos are the checker's input** (R4 MIN). `symbol_attribution_control`
(`.claude/tools/plan_memo_selftest_records.py:181` at `ee6c7a1a`) checks that every written `module.symbol`
attribution names the module that defines the symbol, over a corpus (`_attribution_corpus`, `:295-307`) of the
checker's sources **plus every `docs/plans/*.md`** when the tool sits in its repository. So each memo this program
lands in `docs/plans/` — this umbrella's four files with #581 (decision 221); PR-0's, PR-1's and PR-2's plans each
with its PR —
joins that corpus, and these memos attribute family symbols (e.g. `plan_memo_sibling._resolve`, §11). A wrong or
stale attribution in them turns the VM P4 lane's self-test red in the `tools` job. Each PR runs the suites with
its memo in place (the gate does so by construction); a red from this control is fixed in the landing memo, never
in the control, and later edits to these memos stay inside the control's reach. For the VM P4 lane this is a new
coupling of its control to CI/tools-owned files, live from #581's landing: it is in the consent subject (§5.3 O8,
(c)), which is why #581's merge waits for the consent.

### §5.3 PR-1 and PR-2 × #506 (landed) — the lines they meet on `main`, one owner each (for the PM to show the VM P4 lane)

**State.** #506 merged as `3ec0ae5c` on 2026-11-01 JST (PR head `69e2a3aa`; `gh pr view 506 -R send/elidex --json
headRefOid,mergeCommit,mergedAt`; PORTFOLIO:293), **before PR-1**. So there is no merge-main and no absorption step:
each row below is a line or a coupling **on `main` at `ee6c7a1a`**, with one owner and one time. Decision 127 (b)
is moot; (a), the consent, stands — its subject is (c) below (decisions 205, 211, 213, 221). #506's diff outside `docs/**` is
`CLAUDE.md` and `.claude/skills/elidex-review/axes.md` (Axis 1 wording, `:55-56`, 2+/2−, so no line moved and the
`:139` the body cites holds); it has no `.claude/tools/**`, `.github/**`, `scripts/**` or `mise.toml` (`git diff
--stat 6aea28da 3ec0ae5c -- .claude .github scripts mise.toml` lists `axes.md` only). Draft 6's "#506's whole
diff touches no `.claude/**`" was false (R6 K1).

**(a) PR-1**

| # | Site on `main` | What PR-1 does there | Owner, when |
|---|---|---|---|
| O1 | `CLAUDE.md:54`, the "Plan-memo umbrella checker" bullet, as #506 rewrote it ("matrix の両 leg" once: `git grep -c 'matrix の両 leg' ee6c7a1a -- CLAUDE.md` → 1) | V14, a plain one-phrase edit inside the bullet; #506's added sentences hold no interpreter construct (stage 1) | PR-1, with consent |
| O2 | #510's memo | V12 (`:73`), V15 (`:434`), V13 (`:1116-1117`), V18 (`:1097-1108`), each with a dated marker; #506 changed `:663` only (1+/1−), no shared line | PR-1, with consent |
| O3 | `ci.yml` `tools` job, `timeout-minutes` | #506's guard has been in the job since `3ec0ae5c`, under the unchanged value 22 (the split re-derived nothing: `ci.yml` is unchanged, seam-split `:807-808`). PR-1's re-derive (body §5.1 *Which timeout rule*) is the first one measured with the guard, on the pin | PR-1 |
| O4 | the suite set (ten members at `ee6c7a1a`) | the guard imports `plan_memo_population` and `plan_memo_tables`, and its **import closure is twelve family modules** (blocks, emphasis, html, ids, lexer, links, memo, population, sibling, stream, tables, tokens — `plan_memo_population` imports `plan_memo_sibling._resolve`; read by `ast` over the guard and every `.claude/tools` module it reaches, at `ee6c7a1a`, R§0). PR-1 edits three sites inside it (R7 J2, correcting draft 7's "V11 only"): **V11** (comments in `plan_memo_population.py`, no behaviour), **V6** (`plan_memo_sibling._resolve`; under outcome (i) the `RuntimeError` arm goes, so a `RuntimeError` from `resolve()` then propagates — a behaviour change), **V22** (`plan_memo_sibling.py:158`, keyword `maxsplit`, the same result on both measured interpreters). So the guard's verdict is coupled to V6 and V22, and the consent covers that coupling ((c) item 5). PR-1's CI is the guard's first run on the pin: measured green alone on 3.15.2 and 3.9.6 at `ee6c7a1a` (R§0). A red from the guard in PR-1 is fixed in PR-1's edits; a defect of the guard itself goes to the VM P4 lane through the PM | PR-1, with consent (the coupling is in (c)) |
| O5 → **V16** | seam-split `:356-357` | V16 (§5.2) | PR-1, with consent (decision 205 Q1) |
| O6 → **V17** | seam-split `:804-806` | V17 (§5.2) | PR-1, with consent (decision 205 Q1) |
| O7 | the family sources and `plan_memo_selftest_manifest.txt` | V1–V11 and V22 (`plan_memo_sibling.py:158`, the two rows at `plan_memo_selftest_mutants_codex0927.py:51-52`, `:56-57`, MUTANT lines `:2122` / `:2126`), then `--write-manifest` on the pin at PR-1's base (§5.2 *Manifest rule*); #506 changed no family source | PR-1, with consent — for V22, unless the VM P4 lane fixes it itself: PR-1's stage 1c at PR-1's own base decides (a live hit → PR-1 edits it; none → V22 has left PR-1's edit set) |
| O9 | #506's slot ledger `docs/plans/2026-07-vm-p4-slot-ledger.md` (a VM-P4 family memo, in the guard's `FAMILY` glob) | V19–V21 (`:58-60`, `:322`, `:323`) and V23 (`:327`), with dated markers, cell text only (no table header, no row added or moved); then the checker run of CLAUDE.md:55 on the entry memo, exit 0 expected ((c)) | PR-1, with consent (decisions 211, 213) |
| O8 | `docs/plans/*.md` as `symbol_attribution_control`'s corpus | the corpus holds #506's ten memos and, from #581, this umbrella's four files; #581's CI and then each PR's run the control over them. **Ownership by who wrote the line, not by whose file it is** (R8 A1-2): a red on a line #581 or a later python-pin PR wrote — in a python-pin file, or in a VM P4 memo PR-1 amends (V12, V13, V15–V21, V23 and their markers) — is that PR's to fix in its own edit; a red on a line no python-pin PR wrote would already be red on `main` (the `tools` job runs on every push to `main`) and is the VM P4 lane's | #581 / PR-1 for their lines, with consent (in (c)); VM P4 for the rest |

**(b) PR-2**

| # | Site on `main` | Coupling | Owner, when |
|---|---|---|---|
| P1 | six lines launching the entry point `plan-memo-umbrella-check.py`: `CLAUDE.md:55`, `docs/plans/2026-07-vm-p4-slice-plan.md:82`, seam-split `:328`, `:510`, `:545`, `:546` | tracked launch instructions (R-b evidence) whose target is an entry point; green once PR-2's guard is in that entry point. No edit | PR-2 (the guard) |
| P2 | seam-split `:333`, `:799` — each launches the guard `docs/plans/test_vm_p4_family_layout.py` directly through `-m unittest` (described, not quoted: a quote here would itself be an X1 launch line, R7 J5) | tracked launch instructions whose target is a set member (X1): red under PR-2's X1 bound at PR-2's own base. **In PR-2's edit set** (decision 205 Q2), with the VM P4 lane's consent ((c), the one request of body §9 step 0). Each rewrite keeps its line true as a record: `:799`, an acceptance check of the landed split ("the guard green"), → the driver that runs the guard as a member, `mise run tools-test` (C6's rewrite); `:333`, a record of a mutation run made by the split's untracked `r3/guard-controls.sh`, → names that script as what ran the guard on each mutated tree, without the command (an untracked path, which R-b ignores). The VM P4 lane may give its own wording, if R-b reads it green | PR-2, with consent |

Derivation of P1–P2: `git grep -nP 'python3? +(-[A-Za-z]+ +)*(-m +unittest +)?(\S*/)?(plan-memo-umbrella-check\.py|test_vm_p4_family_layout\.py)'
ee6c7a1a -- CLAUDE.md 'docs/plans/2026-07-vm-p4-*' docs/plans/2026-10-vm-p4-memo-seam-split.md` (`-P`, not `-E`, on
the measuring host; the split's other `python3 …` lines name untracked scratch scripts or inline `-c`, which R-b
ignores). The lines are those draft 6 read at `c032379d`. The X1-targeted lines over the whole tree are three
(body §11): these two and C6's; this program's landing files add none (body §11, R7 J5).

**(c) The consent subject — the one list** (R8 A1-1 + A5-3; PM decisions 127 (a), 205, 211, 213, 221). The PM sends
it to the VM P4 lane **once**, after #581's `/external-converge` ends; #581 merges only once the lane has consented,
and PR-1's implementation starts after that (body §9 steps 0 and 3). Body §6, §8, §9 and the owner cells above
cite this list and do not restate it:
1. **V1–V23** (§5.2), with the manifest regeneration. V1–V15 are included although drafts 4–6 prepared them for
   the lane: no record of their having been sent or answered was found (`PORTFOLIO:244` records the tables as
   prepared, decision 124 "goes to the VM P4 lane after R3"; R8 A5-6). If the PM's record shows them answered, the
   request marks them so.
2. **V6's question** (§5.2 *V6's two outcomes*; body §11): may any `RuntimeError` other than the 3.9–3.12 symlink
   loop leave `Path.resolve()` on the pin? The answer picks outcome (i) or (ii).
3. **V22's option**: the lane may fix `plan_memo_sibling.py:158` itself (§5.2 V22, O7).
4. **P2** (§5.3 (b)): PR-2's rewrite of the two launch lines; the lane may give its own wording.
5. **The O4 coupling**: V6 and V22 sit in the guard's import closure, so PR-1 changes code the guard runs.
6. **PR-1's checker run** (O9): PR-1 edits a VM-P4 family memo (V19–V21, V23), so it runs `python3
   .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md` on the pin and
   expects exit 0 (CLAUDE.md:55; exit 0 at `ee6c7a1a` on 3.15.2 and 3.9.6, R§0); a 1 or 2 is fixed in PR-1's edit
   before landing (C32).
7. **O8**: `symbol_attribution_control` reads this umbrella's four files from #581 on, with the line-owner rule of
   O8's row.
8. **For information, no consent asked**: the `:327` overlap with the lane's own fix of `:278` / `:327` / `:328`
   (PM decision 220, §5.2 *Stage 1b's hits*), and #510's memo staying unsplit under PR-1's amendments (body §6
   *One stated exception*), which the lane may answer with a prereq split.

No post-#506 ratchet remains (body §9 step 8): PR-1's derivation at its base names #506's files (§5.2).


---

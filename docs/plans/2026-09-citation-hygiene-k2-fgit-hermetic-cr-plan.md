# K2 fixture git — PR #527 CR round: the capability path and the label census (plan)

**Status: approved (plan-review R1–R3 and the focused re-check of R3, 2026-10-18), revision r4** (R3's terminator fired, `CR-plan-review-R3.md`: r4
fixes its design-level items H1, H2, H4, H5, H6 and carries its impl-detail items I as required claim rows, §7; no full
R4). r3 answered R2 (`CR-plan-review-R2.md`, G1–G12); R1's F1–F26 (`CR-plan-review-R1.md`), R2's G1–G12 and §9's
amendments stay settled except where a later round replaces them: G2 replaces F19's "Q1 before CR-B2b", G3 replaces
§9's F11 claim row, R3 H4 replaces r3's caller-seeded cursor in G5's reader. Its companion
`…-cr-sweep.md` ("the sweep file"; the handoff file `CR-plan-capability-census-sweep.md` until CR-B1's move) holds the G1 ledger. Both stay in the driver's handoff
directory until the review passes; CR-B1 moves them into `docs/plans/` beside
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo") as `…-cr-plan.md` and `…-cr-sweep.md`
(H1; what moves with them, and where, is §7 "The move"). It plans clusters **B** (the capability / decline path)
and **C** (the label census and its ratchet) of the branch-wide `/code-review max 815f8d4b...14d0b35d`, as triaged in
the handoff file `R7-CR-triage.md` ("the triage"), under the driver's decisions of 2026-10-17 (`DECISIONS-R7.md` "## CR
round"; settled, not reopened) and R1's. It also assigns every item of the triage's appendix (D9, `…-cr-sweep.md` §F).

**Frame: `2f1ca601`** (branch `k2-wire-fgit-hermetic`, worktree `elidex-wt-fgit-hermetic`): the first half is in it
— CR-S1 `c3c0b74b`, CR-S2 `945a814d`, CR-S3 `598e1aa2`, CR-S5 `5f08bd6b`, CR-S8a `161ae982` — and the record rollover
`2f1ca601` (`…-impl-cr.md` §S opened, design memo §13 naming it). Every line number below is at `2f1ca601`. The old
frame `14d0b35d` (snapshot `r7cr-snap/`) is cited only to say what moved. **(M)** = measured in `git clone --local`
copies of `2f1ca601` (never in the worktree): by revision r2 under `crplan-r2/` (cited unchanged), by r3 under
`crplan-r3/`, or by this revision under `crplan-r4/`; b53 = `/opt/homebrew/bin/bash` 5.3.20, b32 = `/bin/bash` 3.2.57; BSD `/usr/bin/grep` and
`/usr/bin/sed`; each run its own process group under an alarm, at most two at once. Every (M) names its log as
`crplan-r2/logs/…`, `crplan-r3/logs/…` or `crplan-r4/logs/…` (paths relative to the handoff directory; each revision's
scripts in its `bin/`; r4 edited `crplan-r3/bin/sweep.sh` in place, backup `crplan-r4/bin/sweep.sh.r3-backup`, its output
at the frame byte-identical, `crplan-r4/logs/M-sweep-excl.txt`). These logs are measured outside the repo and stay there after the move: the figures are
stated here, the logs are not copied (`…-impl.md`:10-13, "the logs themselves are not copied"). A **targeted
trial** (`bin/trial.sh`) runs an edited copy as `_mut_trial` does: the copies sit at `<wire>.mutant.<n>.*`, so the always-on anchor check is skipped as in a real trial
(mutations.sh:559-560), and the verdict is the slot rule (mutations.sh:401-415). A **mechanism prototype** is code that is not the
slice's text (a script, or a marked edit in a clone: `crplan-r3/proto/mk.sh`; r4's standalone `crplan-r4/proto/cap-rows.sh`) and is
labelled as one. Everything
else is read from the frame.

---

## §0 Spec coverage map

**No spec surface** — this file plans changes to a shell trip-wire's self-test harness. It has the same by-design
shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this heading.

---

## §1 Scope

| cluster | findings closed | slices |
|---|---|---|
| B1 — what is declined | U3, K8, K15's instance (`_k2_line`), K9's trigger | CR-B1 |
| B2 — how membership is decided | K4, K10, K14, U8; folds H-3, AL1-alt, AL3/H-6, RU5, CL3/SI8, CL4/SI5/RU3 | CR-B2a, CR-B2c |
| D8 — pinning a probe whose 0 is legitimate | `_fifo_ok` / `_perm_ok` (K8's sibling); P-f's probe limit (R1 F18) | CR-B2b |
| C — the label census | K1, K3, C3, D3; folds SI6, D13/A3-7, AL4/H-7, CL9/RU2 | CR-C1 |
| C's files, local defects | C5, U9; folds CL6/S-7, CL7/SI7, CL10/R-5 | CR-C2 |
| B3 — the kill match (K15's class) | K15 | **done**: CR-S5 `5f08bd6b` (§4) |

**Ratified surfaces this memo changes** (each is why the memo goes to plan-review, per the CR-round terminator):
1. **D1 reverses DECISIONS-R7 S2 OPEN-3** (DECISIONS-R7.md:469-473: "OPEN-3 → Q2 via the `_ctl_need` decline path
   […] `_gopt_ok` probed in the harness beside `_fifo_ok`/`_perm_ok` […] k2 gets `_ctl_need="$_gopt_ok"`;
   `_cn_declined` decides the `⚠ NOT EXERCISED on this machine` line"). Under D1 k2 is declined nowhere.
2. **D2 replaces the `_ctl_need` design of R7 `d976f239`** (`…-impl.md` §R7.2; ctlrun.sh:108-142; the design memo's CN
   row :616): the prefix, `.declined`, the asks and the `_cn` standing check all go (§2.2).
3. **D8 (§2.3) changes the FIFO and permission decline, and P-f's machine limit, from automatic to declared.** Today a
   machine without the capability declines with a `⚠` line and stays green (design memo P-f row :609: "the treatment
   P-b and the FIFO and file-permission controls give a machine that cannot run them"; harness.sh:346-349 for P-f).
   After CR-B2b each is red until its caller declares it. The `⚠` lines and the green on a declaring machine stay.
   Whether CI's runner needs a declaration is Q1 (§11), answered on route (b).
4. **#501 R78's wording is reversed in letter** (controls.sh:642-643: "the line is decided from what the gate did
   (`_cn_declined`), not from the flag"): `_cap_lines` reads the table and the flag, through ctlrun's reader
   (`_cap_rows`, `_cap_flag_of`), never by expanding either itself. It is safe by reader 3's
   equivalence (§2.2): the gate's decision is a function of (table, flag) through the one reader, which `_cap` pins,
   and every member reached the gate, which the census pins.

---

## §2 Design B — the capability / decline path

### §2.1 B1: k2 is posed on every machine, through a grep shim (D1)

**Principle.** A capability a shim can construct is posed everywhere, not declined. Only what cannot be built is
declined: a filesystem with no FIFO, a user who reads a mode-000 file (root). How a grep treats `GREP_OPTIONS` is a
tool behaviour, and the controls file already shims tool behaviours the real machine cannot show: controls.sh:436-438
("macOS's ignores a relative `TMPDIR`, so the real tool cannot pose this question here" → `fakerelmktemp`) and
:541-543 (cfgkept: "The variable's SURVIVAL is the property under test, and that is constructible").

**The shim** (fixtures file, bare-built after `fakegrep`'s, fixtures.sh:357-358, by `_fgit_shim`, harness.sh:112-115):

```sh
mkdir -p "$CTL/goptgrep"
_fgit_shim "$CTL/goptgrep/grep" ': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1' "$_REAL_GREP"
```

A grep that answers "no match" whenever a `GREP_OPTIONS` reached it, and the real grep otherwise. It models the
property under test (did the caller's variable reach grep?), not the value's meaning to one grep, so it does not depend
on how GNU or BSD grep reads `--exclude`. That is `fakegitcfg`'s shape (controls.sh:536-540: a `git` that "answers
`ls-files` with nothing unless the caller's `GIT_CONFIG_COUNT` reached it"), so it is one way with that shim.
**Rejected:** a shim that prepends `$GREP_OPTIONS` to its arguments (BSD's documented behaviour): whether it
discriminates would then depend on GNU grep's `--exclude` on command-line files, which this machine cannot measure,
and on the stdin calls (`_match_path` wire:809, the verdict classifier wire:1180).

**The call** (controls.sh:609) becomes `_control "$CTL/k2" 1 "K2: a" "a caller's GREP_OPTIONS cannot hide a file" ""
"" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*"`. Its `path` evidence is demanded like every prefix's
(ctlrun.sh:337), its channel like every channel.

**What goes** is the sweep file's 34 B1 rows (its §2): `_gopt_ok`, its probe and comment (harness.sh:830-857, 28
lines: D2(b)'s subject, :840-843, reworded by CR-S2 `945a814d` from the old frame's "reds the run either way", and
the second resolution of grep, CL5/RU6); k2's prefix (controls.sh:609); `_k2_line` in both forms, its ask and its
print (:608, :610-614, :694; the claim moves into the static lines :688-691); the comment :600-607; record 373;
controls.sh:25-26; mutations.sh:401-408 (`_mut_trial`'s K15 example, restated without `_k2_line`/`_gopt_ok`, and
without its clause quoting the controls file's `_limit_lines` reason, which CR-B2b rewrites); corpus.md:203. **The
ledger adds** harness.sh:8-10 ("the three capability probes (FIFO, file permissions, `GREP_OPTIONS`)" → two) and
splits records.sh:367: `_gopt_ok` leaves that comment in CR-B1, so B1's grep reaches 0; the comment goes with records
372/374 in CR-B2a. Completion (`crplan-r3/bin/completion.sh`; (M) at the frame, `crplan-r3/logs/M-completion-frame.txt`):
`_gopt_ok|_k2_line|goptprobe` over the seven parts and the wire, 18 lines → 0; over the design memo, census,
landing and residuals 0 → 0; over corpus.md 2 → 1 (:321, a §6.1 history row); `three capability probes` 1 → 0;
`bin/ledger-check.py <head> B1` passes. **The wire is not edited** (1286 lines; its `unset GREP_OPTIONS`, wire:324,
is the subject).

**(M) Prototype** (`crplan-r2/logs/M-k2shim.txt`; clones `work/shim*`): the shim and the call above, record 373
deleted (222 records, floor 220 unchanged); the probe and `_k2_line` left in place. The GNU-like profile is a `grep`
first on `PATH` that unsets `GREP_OPTIONS` and execs `/usr/bin/grep` (`crplan-r2/gnulike/grep`, the triage's profile).

| case | BSD b53 | BSD b32 | GNU-like b53 | GNU-like b32 | log (`crplan-r2/logs/`) |
|---|---|---|---|---|---|
| shipped | rc 0 PASSED | rc 0 PASSED | rc 0 PASSED | rc 0 PASSED | `P-shim-ship.{bsd,gnu}.{b53,b32}.log` |
| record 100 (`unset GREP_OPTIONS` → `:`), trial | KILLED AS NAMED | same | same | same | `P-shim100.*.log` |
| record 100 + the shim's `GREP_OPTIONS` arm removed, trial | KILLED (BSD's grep honours it) | same | **SURVIVED** | **SURVIVED** | `P-shimarm100.*.log` |
| the shim's `path` mark removed, ship | `CONTROL NOT EXERCISED (<k2>): PATH prefix never reached the run` | same | — | — | `P-shimnomark.*.log` |

The survive clause depends on the profile: on BSD the real grep already honours the variable, so the arm-mutant row
is measured on the GNU-like profile, and the claim table says so.

### §2.2 B2: one capability table (D2)

**The defect class** (K4, K10, K14): membership of a capability group was written nowhere. The gate learned it from a
per-call prefix, the summary from the growth of a second log (`.declined`) between positional asks, and `_mskip` not
at all: no reader could check identity (K4), the second log needed guards and one was missing (K10), and the excuse
could not see a decline (K14). One declaration, read by every reader, removes the class.

**The table** lives in the controls file (which controls exist is that file's subject), **after `_lbd`**
(controls.sh:121-154) and before the harness is sourced (:156): it expands the group labels only once `_lbd`, "asked
before anything else runs", has asked about them, and it exists when ctlrun's guard runs at the harness's end (G7,
G3). **(M)** mechanism prototype, `crplan-r3/logs/M-proto3.txt`: the table before `_lbd` with `_fifo_lbl`'s
definition renamed → `_fifo_lbl: unbound variable` at the table's line, rc 1, unlabelled (b32 adds `$_ord_lbl`'s red
from the EXIT trap); after `_lbd` → `CONTROL FAILED (every label the parts use is defined): used but not defined, or
empty: _fifo_lbl`, rc 2, both shells.

```sh
_CAP_TABLE="group	fifo	_fifo_ok	$_fifo_lbl	this filesystem holds no FIFO	an entry git cannot store neither hangs nor hides a verdict, and a tracked path replaced by one is not opened
member	fifo	a tracked path replaced by a FIFO is not opened
member	fifo	an unstorable entry neither hangs nor hides
group	perm	_perm_ok	$_perm_lbl	this user can read a mode-000 file	an unreadable file or an unsearchable directory fails closed, and a restrictive umask decides nothing
member	perm	an unreadable file fails closed
member	perm	an unsearchable dir fails closed
member	perm	a tracked file under an unsearchable dir is not absent
member	perm	an unresolvable self-test root decides nothing
member	perm	a restrictive umask decides nothing
limit	P-f	$_pfl_lbl	this env refuses -0
fact	P-b	keyed on git's documented 2.42 boundary (CR-S3), not a probe"
```

Row kinds and TAB fields: `group` 6 (kind, name, flag variable set by the harness probe, the group's label (§2.3),
why it is absent, the claim its controls support); `member` 3 (kind, group, a `_control` label, spelled literally —
the census's literal rule); `limit` 4 (kind, a P-x ID whose machine limit is a probe's answer, its label, why); `fact`
3 (kind, a P-x ID whose limit is a decided fact, the reason). Fields are split by parameter expansion, never by
`IFS=$'\t' read`, because tab is IFS whitespace: **(M)** both shells collapse an empty field (`read:[group][zz][x][]`
for `group\tzz\t\tx`; parameter expansion gives `[group][zz][][x]`), `crplan-r2/logs/M-shell.txt`. An empty field is
a malformed row, and so is a `group` row whose flag field is not a name (G8).

**One reader (G5), which owns the iteration (R3 H4)**, in ctlrun.sh (sourced before every caller). `_cap_rows <kind>
<callback>` walks `$_CAP_TABLE` with a **function-local** cursor and function-local row fields `_cr_f[0…]`, `_cr_nf`
and `_cr_bad` (empty for a well-formed row, else the reason: a kind with the wrong field count, an empty field, or a
`group` flag field matching `''|[0-9]*|*[!A-Za-z0-9_]*`, the channel-name test of ctlrun.sh:231), by parameter
expansion, no fork, and calls the callback once per row: for `<kind>` `group`, `member`, `limit` or `fact` on each
well-formed row of that kind, for `-` on every row (`_cr_bad` set on a malformed one). The callback sees the fields
through bash's dynamic scope and stops the pass by setting `_cr_stop=1`, not by its return status (a function run as
`cb || …` runs with errexit off). Nesting is safe by construction: a pass started inside a callback shadows the
cursor and the fields with its own locals, and the outer pass resumes intact. No caller seeds a cursor or expands
`$_CAP_TABLE`; the flags are read only inside ctlrun, by `_cap_flag_of <group>` (sets `_cap_v` to the group's flag
value, or empty). A row with `_cr_bad` declares nothing to any caller. The reader is called by ctlrun's source-time
guard (G3, below) and by the five readers below; the GB2 rows pin it through the gate. **(M)** r3's caller-seeded
cursor (`_cap_next` over a global `$_cap_rest`, `crplan-r3/proto/ctlrun-reader.sh`) visits 1 of the table's 11 rows
when a lookup of the same shape (a full pass re-seeding that global) runs inside the pass; the r4 reader (`crplan-r4/proto/cap-rows.sh`, a standalone mechanism prototype,
`crplan-r4/bin/nest.sh`) visits all 11 under `set -eu`, each row's fields 0 and 1 (kind and the next field) unchanged after an inner `_cap_group_of` on a
label no row holds (the inner pass reads every row) and on the last member's label, identical on both shells
(`crplan-r4/logs/M-nest.b53.txt`, `M-nest.b32.txt`; the other fields were not compared, R3-B2a-7 does that); claim row GB2-17 pins it in the real code. (r3's wire-level
prototype, `crplan-r3/logs/M-proto3.txt` variant `ship`: PASSED, both shells, used the old shape; its other variants
measure the guard and the validation, which H4 does not change.) The readers, each placed with its owner (R1 F13):

1. **The gate** — `_cap_group_of` (ctlrun.sh, the gate's lookup). `_ctl_gate` keeps its order: registration, trust
   arm (W2), then capability, then the fixture half (`_ctl_built` was folded into `_ctl_gate` by CR-S1;
   ctlrun.sh:128-151). `_cap_group_of "$label"` sets the out-globals `_cap_g` (the label's `member` row's group, or
   empty) and `_cap_f` (that group's flag name, or empty), by exact string comparison, no fork. Not a member → run.
   **A member whose group no `group` row declares is red under the call's label, return 1, and this is resolved
   before any indirect expansion (R1 F5):** **(M)** `${!f-}` on an empty name aborts bash 5.3 ("invalid variable
   name", rc 1, also inside an `if`) and expands empty on 3.2 (`crplan-r2/logs/M-shell.txt`), so an unguarded lookup
   would split the shells. A `group` row whose flag field is not a name is `_cr_bad` and declares nothing, so its
   members fall to the same red (G8): **(M)** `${!x-}` on `a-b` or `1a` aborts b53 ("invalid variable name", rc 1)
   and is empty on b32 (`crplan-r3/logs/M-shell3.txt`); in the prototype, the validated reader gives the same run on
   both shells, and with the validation removed b53 aborts while b32 reports `missing: $a-b`
   (`crplan-r3/logs/M-proto3.txt`, variants `badflag`, `badflag-novalid`). A malformed `member` row leaves its label
   a non-member: it runs (the fail-safe direction), and `_cap_check` reds the row. Then a decline (return 2) exactly
   when `_cap_flag_of`'s value is `0` (it reads `${!_cap_f-}`; **(M)** `${!n-}` is empty when unset and the value
   when set, both shells, `crplan-r2/logs/M-shell.txt`). Any other value runs the control — the fail-safe direction — and `_cap_check` reds
   the value.
2. **The table check** — `_cap_check` (controls.sh; **a table check, not a standing check** (G11): it runs no
   `_ctl_run` row, so "the three standing checks" stays true at every site), called once as `_cap_check || ctl_ok=1`
   right after the window verdict (controls.sh:197) and before the first gated run (`_wm`, :221-229). It returns 1
   on any red; its call line is the writer (G9), and `[ "$ctl_ok" -eq 0 ] || exit 1` (controls.sh:684) precedes
   `_mut_run` and the summary, so a red line with rc 0 is impossible (GB2-13). Through `_cap_rows -` and
   `_cap_flag_of` (the table and the flags are ctlrun's to read): every row parses (`_cr_bad` empty,
   the red naming the row and the reason); group names unique; every member's group declared; member labels unique; every
   group's flag set and `0` or `1`; every ID in `$_fw_limits` has a `limit` or `fact` row (an unknown limit falls red:
   `memory/feedback_fail-safe-direction-before-list-completeness.md`); every word of `WEBREF_WIRE_LACKS` (§2.3) names
   a group or a `limit` row. Each failure is red under `$_capt_lbl`. Then the undeclared-absence check (§2.3).
3. **The summary** — `_cap_lines` (controls.sh, beside the summary lines :688-696). One line per `group` row, in table
   order (`_cap_rows group`; `<n>` by a `_cap_rows member` pass nested in its callback): flag 1 prints `<claim>`;
   flag 0 prints `⚠ NOT EXERCISED on this machine: <n> control(s) of the <g> group (<why>), so this run carries no evidence that <claim>`, `<n>` counted from the table. It replaces `_fifo_line` /
   `_perm_line` and their asks, and with them the polarity split and indent drift of CL4/SI5/RU3. It is still
   "decided from what the gate did" (controls.sh:642-643, "One site, one truth: the line is decided from what the gate
   did", #501 R78) by construction: the gate's decision is a function of (table, flag) through one lookup, which the
   standing check pins, and every member reached the gate, which the census pins (reader 4).
4. **The census** — `_mut_correspondence` (mutations.sh) adds, through `_cap_rows member`: every member label is a
   `control`-kind label of this run's registry, else red under `$_cen_lbl` ("declared capability member … is no
   `_control` this run made").
5. **`_mskip`** (CR-B2c, §2.4): asks `_cap_group_of` for the needle's group and `_cap_flag_of` for its flag, and
   `_cap_rows group` for a needle that is a group's label; it reads neither the table nor a flag itself.

**The interface (G3, replacing R1 F11's placement; R3 H4 for the reader's shape).** ctlrun.sh's WHAT IT CONSUMES
(:17-23) gains `_CAP_TABLE` (controls) and the group flags by the names the table's `group` rows give (harness); its
WHAT IT DEFINES (:24-28) gains `_cap_rows`, `_cap_group_of`, `_cap_flag_of` and the out-globals `_cap_g`, `_cap_f`,
`_cap_v`, with their outside readers (`_cap_check`, `_cap_lines`; the membership check, `_mskip`) (G12 A1), and the
reader's internal names (§7, CR-B2a's required rows). ctlrun is the only part that expands `$_CAP_TABLE` or a flag.
**The name guard sits at the first consumer**: `_CAP_TABLE`'s first reader is ctlrun's code (its guard at source
time, then `_cap_rows` from `_cap_check` after :197 and the gate from `_wm`'s run at :229), more than 400 lines
before mutations.sh is sourced (:673). Right after the `_CTLRUN` check (ctlrun.sh:31-36), a source-time guard in the
form of mutations.sh:91-100 and controls.sh:64-73 (`missing: $<name>`, exit 2) asserts `_CAP_TABLE` and then, through
`_cap_rows group`, each well-formed `group` row's flag by the row's own name (one spelling; `_cr_bad` rows are
skipped, so no indirect expansion meets a non-name, and `_cap_check` reds them); which other consumed names it asserts
is a required row of CR-B2a (§7). Every later read of the table is `_cap_rows`'s plain expansion: an unguarded
`set -u` abort cannot happen because the guard ran. **For the table, mutations.sh consumes the reader's functions
only** (`_cap_rows`, `_cap_group_of`, `_cap_flag_of`; neither the table nor a flag), so its WHAT IT CONSUMES
(:20-31) and entry guard (:91-100) name those functions and not the table (its other new consumed name is
`_CONTROLS`, §3.2 rule 1, G4). The guard's loop tests variables (`${!_n:-}`, mutations.sh:92-94), so it asserts the
three functions by `declare -F <name>`, in a loop of its own after :94 (a different test; :92 is the line CR-C1
edits for `_CONTROLS`, ledger row 205, which stays C1's alone): **(M)** under `set -eu`, `declare -F` fails for an
undefined name and for a variable, and passes for a function, both shells (`crplan-r4/logs/M-declf.txt`). The
paragraph ctlrun.sh:37-46 is reworded, not kept (G12 A1): no per-call name remains, and the guard follows it.
Claim row GB2-15, **(M)** (`crplan-r3/logs/M-proto3.txt`, the r3 reader): the table renamed in the controls file only → `missing:
$_CAP_TABLE`, rc 2, both shells; the same with the guard removed → `_CAP_TABLE: unbound variable` at the first read
after the guard (in the prototype the gate's, ctlrun.sh line 147 of the clone), rc 1, unlabelled (b32 adds
`$_ord_lbl`'s red); a `group` row naming `_fifo_okk` → `missing: $_fifo_okk`, rc 2. Which reader that first read is in
the real code is measured in CR-B2a (§7).

**The standing check `_cap`** replaces `_cn` (controls.sh:282-363). Table-driven like `_cn`, with a no-arm guard and
parse guards (the K6 class, one way with `_wm` and `_ev`). All rows run through **one** `_ctl_run "$_cap_lbl" …` call
line, in a subshell that replaces `_CAP_TABLE` and the synthetic flags `_zz_ok` / `_yy_ok` per row: the subject is the
gate's mechanism, not the real table's content. It **does not redirect `_VFY`**: its label registers at its one line
in the run's registry (what lets C pin its presence, §3.3, and removes `_cn`'s sub-registry, the reason `$_cn_lbl`
reached the census only as a text value: K1's `_cn` sub-case). **Each row's stderr goes to `$_VFY/.cap_err`** (R1
F16), as `_wm`'s, `_ev`'s and `_cn`'s go to theirs (controls.sh:229, :267, :333): a row's EXPECTED red (rows U and X)
must never reach the run's output, where under the slot rule a `CONTROL FAILED (<$_cap_lbl>)` would let any unrelated
red "kill" every `$_cap_lbl` record (K15's class). Its non-comment lines name no run primitive except its call line
(rule 2, §3.2). Rows (`M` = `$_cap_lbl` is a member of `zz`; `N` = `zz`'s members are a proper prefix and a proper
superstring of `$_cap_lbl`, never it; `Y` = `$_cap_lbl` is a member of `yy`; `U` = `$_cap_lbl` is a member of `xx`,
which no `group` row declares; `F` = `$_cap_lbl` is a member of `zz`, whose `group` row's flag field is `a-b`, then
`1a`):

| table | zz | yy | trust | channel | want | ran | pins |
|---|---|---|---|---|---|---|---|
| M | 1 | 1 | yes | — | 0 | ran | a member runs where its flag is 1 |
| M | 0 | 1 | yes | — | 2 | — | the decline arm |
| N | 0 | 0 | yes | — | 0 | ran | exact match, in both substring directions |
| Y | 0 | 1 | yes | — | 0 | ran | the member's OWN group's flag decides (K4's identity) |
| Y | 1 | 0 | yes | — | 2 | — | the same, reversed |
| U | 0 | 0 | yes | — | 1 | — | an undeclared group is red, the red naming `xx`, before any indirect expansion (F5) |
| M | 0 | 1 | no (`_fw_trusted=0`, `_fw2_said=1`) | — | 1 | — | the trust arm sits before the capability arm |
| M | 0 | 1 | yes | `X` | 1 | — | the channel is checked whatever the gate decides (K9; CR-S2's `_cn` row `0 1 - chan -`, controls.sh:355, moved here) |
| F | 0 | 0 | yes | — | 1 | — | a flag field that is not a name declares nothing: red naming `zz`, both shells (G8) |

**What goes in CR-B2a** is the sweep file's 122 B2a rows (its §2). Beyond the code (`_cn_lbl`, `_cn` :282-363, the
asks :364-387 and :568-574, :656-658, :675-678, `_fifo_line`/`_perm_line` and their prints, the seven prefixes, the
gate's need arms and their comment :108-142, `unset _ctl_need`), the statements it rewrites: ctlrun.sh WHAT IT
CONSUMES (:19), WHAT IT DEFINES (:24-28), :37-46, the registry header's `_ctl_need` sentence (:75-77), the RETURN
CONTRACT (:170-183), :222, `_control`'s comment (:351-354); controls.sh :23-26, :101-104 (the table check and the
group/limit labels join the non-`_control` producers), :219, :259, :485-491, :639-647 (R78, §1 item 4);
mutations.sh :20-31 (the reader), :481, :498-501; records.sh :156-159 (what removing `_ctl_run`'s ask reds under,
re-measured), records 372 and 374 with :364-371, record 377 **re-aimed to `$_cap_lbl`** (its subject, the gate's
answer taken as an early return before the channel validation, now dies at `_cap`'s X row) with :375-376; the design
memo's :476 (`_cap`'s untrusted row), :510 (re-measured) and the CN row (:616) → rows CAP (`$_cap_lbl`), CAPT
(`$_capt_lbl`) and one per group label. Records of past rounds (`…-impl.md`, `…-impl-cr.md`, `…-pr527*.md`,
`…-reviews.md`, corpus §6.1 rows :312, :321) keep their history. Completion (`crplan-r3/bin/completion.sh`; (M)
`crplan-r3/logs/M-completion-frame.txt`): `_ctl_need|\.declined|(^|[^A-Za-z0-9_])_cn([^A-Za-z0-9]|$)|/\.cn([^A-Za-z0-9]|$)|_fifo_line|_perm_line`
over the seven parts and the wire, 112 lines → 0 (the previous revision's grep reached 108: it missed the four
`$_VFY/.cn` lines controls.sh:285, :319, :320, :363); over the design memo, census, landing and residuals, 3 → 0;
`bin/ledger-check.py <head> B2a` passes.

**What stays:** `_control`'s `[ "$_cr" -ne 2 ] || return 0` (ctlrun.sh:361) and its measured reason (:357-360, the
ratified precedent for an unpinnable line on a capable machine), with "`_ctl_need` carried a 0" reworded to "the
table declined it" (:351-354).

### §2.3 D8: a probe's 0 is pinned by declaring the absence

**The problem.** `_fifo_ok` and `_perm_ok` (harness.sh:817-818, :826-828) are probes, and 0 is a legitimate answer on
some machines; so is P-f's machine limit, the answer of a definition test of `env` (harness.sh:343-362). The wire
alone cannot tell a defective answer from a true one on the same machine, and today a wrong 0 on a capable machine
declines its group (or skips P-f's check) with a false `⚠` and stays green — the K8 class. **(M)** The behaviour
simulations of §5.2 show today's automatic path: no FIFO, root, or both → the `⚠` lines and rc 0, on both shells
(`crplan-r2/logs/M-sim.txt`).

**Chosen: an absence the wire learns from a probe is declared by the caller; an undeclared one is red** (R1 F18: a
machine-limit answer from a PROBE is pinned only where declared — one issue, one way). `WEBREF_WIRE_LACKS` holds
group names and `limit` IDs. `_cap_check`: for each group whose flag is 0, red under the group's label unless the
variable names it; for each `$_fw_limits` line whose ID has a `limit` row, red under that row's label unless the
variable names the ID. A `fact` row needs no declaration: P-b's limit is keyed on git's documented 2.42 boundary
(CR-S3, harness.sh:283-305, :340-341), a decidable fact. The probe still decides what the gate (or the window) does;
the declaration decides only whether the answer is acceptable, so it **cannot make a capable machine skip anything**
(a stale `LACKS=fifo` there runs the FIFO controls), which is why it is safe to read from the caller's environment,
unlike the retired `WEBREF_WIRE_SELFTEST` (controls.sh:494-498).

**Labels** (each one producer, each with a record, §6): `$_fifo_lbl` = "this machine holds a FIFO, or its caller
declared that it cannot"; `$_perm_lbl` = "this user cannot read a mode-000 file, or its caller declared that it can";
`$_pfl_lbl` = "this env lists its environment NUL-separated, or its caller declared that it cannot" (named in the
`_p<x>*_lbl` convention `_mskip`'s P-x half keys on, mutations.sh:715, so its own probe-flip record is excused on a
P-f-limited declaring machine by that half: no second mechanism); `$_capt_lbl` = "the capability table is well formed
and names every absence it is asked about"; `$_cap_lbl` = "the gate declines exactly the members of a group whose
flag is 0".
- **What it pins:** a wrong probe answer is red on **every capable machine that runs the gate** — a macOS checkout, a
  non-root Linux checkout, and CI's runner if it holds both capabilities (Q1, §11: answered by the first
  `trip-wires` run on route (b), not before) — and when a runner image changes and drops a capability. The
  probe-flip records (§6 GB2b-1, -2, -4) are killed there. A runner that lacks one makes that run RED (an undeclared
  absence), never a false green; the fix is a declaration in the job's `env:` beside `runs-on`, recorded in the open
  record (G2).
- **Its cost:** a root, FIFO-less or `-0`-less contributor sets `WEBREF_WIRE_LACKS` once; the red line says so. (That a no-FIFO contributor's opt-in still reds, BS-7, is claim row R3-B2b-2, §7.)
- **Rejected:** CI declaring its runner's capabilities (pins only on CI; in `scripts/trip-wires.sh`, also `mise run
  trip-wires`'s entry, it reds every root contributor; keyed on `GITHUB_ACTIONS` it moves a `runs-on` fact off its
  job); standing rows running each probe over shimmed tools (they pin the mapping, not the observation, and `[ -r ]`
  is a builtin root bypasses).
- **Where it is read:** in the parent only, by `_cap_check` (controls.sh); the window is built from nothing and
  `--selftest` children do not source the controls (wire:1228). Mutants inherit it through `_mut_trial`'s `env -u
  WEBREF_WIRE_MUTANTS "$BASH" "$_mut_wire"` (mutations.sh:360), so a declaring machine's mutants see the same
  declaration (R1 F15: stated there, pinned by GB2b-6). Documented beside `_cap_check`, not in the wire (1286 lines)
  and not in ctlrun.sh: no ctlrun function reads it (`/usr/bin/grep -c WEBREF_WIRE_LACKS` on ctlrun.sh → 0; R1 F14).
  ctlrun.sh:37-46 is reworded anyway, for G3's guard (§2.2), not for LACKS.
- **Sites that state the automatic decline as current** (R1 F10) are the sweep file's 14 B2b rows: controls.sh:395-403
  (with §9's slot-rule rewording), harness.sh:343-349, the design memo's P-b (:605, a `fact` row now) and P-f (:609)
  rows, corpus.md:194-200 (shared with CR-B2c), landing.md :121-127 and :386 (Q1 on route (b), G2) and :183-186 (the
  launch-environment slot gains BS-3's member, G12 A5), residuals.md R4 (:111, the same member), and five #519-memo
  sites carried by residual R12 (§7). The W4 seal hazard lines (design memo :603, census.md:223-224,
  harness.sh:564) describe a hazard, not the decline, and stay (census.md:224 is a `stays` row; the other two name
  no swept identifier). Completion: the phrase greps of
  `crplan-r3/bin/completion.sh` (`reported below and not`, `printed on a green line would`, `as for the FIFO and
  file-permission controls`, `the treatment P-b and the FIFO and file-permission controls`: (M) 5 lines at the
  frame, one of them mutations.sh:407, which CR-B1 removes) → 0, and `bin/ledger-check.py <head> B2b` passes.

### §2.4 CR-B2c: the opt-in excuse reads the table (K14)

`_mskip` (mutations.sh:717-726) gains a second source beside the P-x limit lines: a record whose needle is a member of
a group whose flag is 0, or that group's label, is excused ("not exercisable on this machine: the `<g>` group is
declared absent"). Only a declaring machine reaches `_mut_run`. **The excused outcomes differ from P-x's, on
purpose.** P-x excuses a survival only (mutations.sh:738-742: a 2 "is also how a harness break … reports"). A declined
member's label never stands in a label slot on that machine (the slot rule, §4), so "killed for the wrong reason" is
that machine's expected outcome for such a record. To excuse it without excusing harness breaks, `_mut_trial` returns
**3** from its wrong-reason arm (mutations.sh:411-414) **only when the mutant exited 1** (G10). A mutant that exited 2
ended at "decided nothing" (the window's W, a missing part), which is also how a harness break reports, so that arm
still returns 2, as MATCHED NOTHING, a missing `.ctllabels`, the wrong subject and a dead negative control do; a 2 is
never excused. The `!kill` arm returns 0, 1 or 2 only, so mutgen's caller (mutgen.sh:324-326) stays exhaustive. The
caller prints the wrong-reason diagnostic when it does not excuse. A capability-excused record accepts 1 or 3; P-x
keeps 1. **A 3 that nothing excuses is red** (R1 F7, GB2c-3).

**(M) The population, today** (`crplan-r2/logs/M-k14.txt`; trials under the root simulation of §5.2): the perm-member
records 86, 98, 99, 280, 281 and 394 SURVIVE and 289 is WRONG REASON with mutant rc 1 (`_fixture_failed clean` reds
the clean controls while the umask member never runs), on both shells — K14 as it stands: the opt-in reds on a machine that declines.
After CR-B2c they are excused (1; 3 for 289). On P1 records 280 and 289 are KILLED AS NAMED (same log), so the excuse
must not fire where the group is present. The FIFO members have no record (both are among the 18 bare labels, (M)
`crplan-r2/logs/dump-head.b53/.bare`), so on a no-FIFO declaring machine the only excused record is the fifo probe
flip. **Not covered, by design:** records 201, 215, 217 and 219 (needle `$_pg_lbl`) plant a FIFO in the window; under
the no-FIFO simulation the plant fails and the window ends at W: all four WRONG REASON with mutant rc 2, both shells
(201 and 215: same log; 217 and 219: `crplan-r3/logs/M-nofifo217.txt`). Their needle is no member, and their mutant's
2 is the "decided nothing" a harness break also reports, so `_mut_trial` returns 2 and nothing excuses it: the
opt-in reds there — §6 BS-7. records.sh:277-279 ("`_control err` reports the failed build") is corrected to "where
`_perm_ok` is 1", and mutations.sh:716's commit hash goes with `_mskip`'s rewrite (CL7's other half, the changelog
:238-247, is CR-C2's).

### §2.5 B and the findings

| finding | closed by |
|---|---|
| U3 CI never asks k2 | B1: posed on every profile (M) |
| K8 `_gopt_ok` unpinned, false comment | B1: removed |
| K15's instance | B1: `_k2_line` removed (the class: CR-S5, done) |
| K9's trigger | B1: no declinable call carries a channel; the order is pinned by `_cap`'s `X` row (K9's fix is CR-S2's) |
| K4 any decline credited | B2a: no prefix; decline ⇔ table membership of the label's own group; `Y` rows |
| K10 `>>` unpinned | B2a: `.declined` removed |
| K14 `_mskip` ignores declines | B2c (M: the population above), except BS-7 (P4/P7: records whose plant needs the absent capability) |
| U8 memo CN row | B2a: the design memo's CN row → rows CAP, CAPT and one per group label |
| `_fifo_ok` / `_perm_ok` unpinned; P-f's probe limit (F18) | B2b |

---

## §3 Design C — the label census and its ratchet

### §3.1 The defect class

The census population comes from two sources, and both are wrong at the edge. Its variable half (`.vlabels`,
mutations.sh:512) is every `_*_lbl` name the parts' TEXT mentions, definitions and comments included, so a deleted or
unreachable variable-label producer stays "used" (K1). Its registry half counts only the calls that EXECUTED on this
machine, so a new unrecorded `_control` behind a machine-dependent branch is invisible (K3). And the line a registered
site reports is assumed to be the call's text line, but nothing checks it (C3). One change closes all three: **the
census population is bijected with the text.**

### §3.2 The rules (CR-C1)

1. **Variable labels count as used only where expanded.** `.vlabels` = the values of the `_*_lbl` names that have at
   least one expansion (`$name`, or `${name` followed by a non-identifier character) outside a comment, in **the
   wire or any part except the record table**. A comment is the text from a `#` that begins a word to the line's end, cut
   on every line (fix round, I-1). A definition line and an indirect `${!x}` are not uses. The
   record table is out because its here-document is quoted (records.sh:42, `<<'MUTANTS'`): a `$name` there is sed
   data, never expanded, and counting it would keep a label "used" by a record's own text (at the frame records.sh
   :361, :363, :374 spell `$_fw_lbl` and `$_mg_lbl`). **(M)** `crplan-r2/logs/M-expansions.txt`: 27 names; `_ord_lbl`
   has 0 expansions in the parts and one in the wire (wire:426, `${_ord_lbl:-…}`), so a parts-only population would
   orphan its records; with the record table excluded every name still has ≥ 1. So `_lbd` (controls.sh:121-154, under
   its comment "EVERY LABEL A PART REFERENCES IS DEFINED AND NON-EMPTY", :121; label :107 "every label the parts use
   is defined") reads the wire too, and `.vlabels` ⊆ the names `_lbd` asserted defined. `_lbd_lbl`'s text is kept:
   the check covers more than it states, and the design memo's row L (:620) and :589 say what it reads. Rule 1
   rewrites mutations.sh:512, the anchor of record 356 (`mutations:s/^  for _cen_v in [$]_lbd_names; do/…`): it is
   re-aimed on the new line, same needle. **The table's own expansion (G12 A1):** a group or limit label's only
   producer is `_cap_check`'s red line, which prints the value the reader took from the table (a `_cr_f` field, no
   `_*_lbl` name), so rule 1 counts no use there; its counted expansion is the table's `$_fifo_lbl` (G7 keeps label
   values in the table, not names). So the label stays "used" while
   the table names it even with that red line deleted, K1's shape via the table; its probe-flip record (GB2b-1/2/4)
   then SURVIVES in the opt-in run, BS-4's class (claim row GC-7).
   **The census reads `$_CONTROLS`** (rule 2's text), so mutations.sh's WHAT IT CONSUMES (:20-31) and entry guard
   (:92) gain `_CONTROLS` (G4), the drift controls.sh:32-38 records (claim row GC-8).
2. **Every label-taking call line is a registry site, and every site is such a line.** A *call line* is a non-comment
   line of `$_CONTROLS` that names `_control`, `_ctl_run`, `_ctl_gate` or `_fgit_window_verdict_exit` as a word; its
   kind is `control` when `_control` is named, else `direct`. The census requires the set of call lines (with kinds)
   to equal the set of registry sites (with kinds), and reds under `$_cen_lbl` listing each side's extras. This closes
   K3 (an unexecuted `_control` is a line with no site, on every machine), K1's direct half (an `if false && _ctl_run
   "$_rel_lbl"`, a deleted window verdict, a standing check whose rows never run) and C3 (a call registered at a line
   that is not its text). It enforces the convention the file states but never checked (controls.sh:512-513, "(no
   `if` around it)", deleted with the prefixes in CR-B2a, whose rule this becomes), and, with its "an `_lbl="…"`
   definition" reworded to an expansion (rule 1), it makes mutations.sh:180-181 true ("ADDING A LABEL MEANS ADDING A
   RECORD … the ratchet below is what makes that true"). The registry becomes
   machine-independent, so `_MUT_UNRECORDED_MAX` is exact on every profile.
   - **(M) At the frame** (`crplan-r2/logs/M-registry.txt`; clone `work/inst` = the frame plus one line dumping the
     census files): registry 90 rows, 88 sites, byte-identical between the shells (as are `.bare` 18, `.labels` 110,
     `.needles` 223, `.vlabels` 27); 91 text call lines. The three differences are text-only: controls.sh:234 and :270,
     `_wm`'s and `_ev`'s diagnostics naming `_ctl_run` ("… _ctl_run returned …") — CR-C1 rewords both to "the run
     returned …" (R1 F1) — and :333, `_cn`'s rows under its own `_VFY` (gone with CR-B2a). Continuation-line calls
     (:499, :554, :557) register at their first line, and the two-row standing checks inside a multi-line `( … )` at
     their call line (`_wm` :229, `_ev` :267, two rows each), on both shells.
   - **Trust (R1 F1).** The three standing checks open `if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1 elif ! ( …`
     (controls.sh:223, :264, :321). An untrusted run is already red: the window verdict (controls.sh:197) exits 1
     before any standing check (harness.sh:758-771). With that exit removed (the W2 record's shape, records.sh:160),
     simulated by a line **inserted** after the verdict function's last line (harness.sh:772) that redefines it to register only
     and set `_fw_trusted=0`, so no line a record anchors on is edited (G6; every record expression still changes
     exactly one line, `crplan-r3/logs/M-sim3-anchors.txt`): **(M)** (`crplan-r3/logs/M-trust3.txt`, clone
     `crplan-r3/work/untr3`) the run prints one line, W2's `CONTROL FAILED (no control runs over …)`, rc 1, with no
     anchor-check line; KILLED AS NAMED as a trial; both shells; :197 is a site, and the three call lines (:229, :267,
     :333) have none. (The previous revision edited :197 itself, record 354's anchor, and its plain run printed the
     anchor check's stale-anchor red as well.) Under CR-C1 the census then adds a `$_cen_lbl` red listing those lines: a second red on a red run, never a
     green path, and W2's records are still killed as named (the slot rule ignores other reds). The pre-read stays: it
     keeps W2 the gate's one report (ctlrun.sh:66-71).
   - **It bans, deliberately,** a primitive called from a controls-file helper defined on another line, and a call the
     text hides (`eval`, a split word), both allowed by ctlrun.sh:79-85 and mutations.sh:192-197; the controls file
     has neither. The `_rh` record (records.sh:362) is still killed: its second call line has no call text.
3. **One parse of the registry** (CL9/RU2). One `awk -F'\t'` pass yields site, kind and label, and any row with a
   field count other than three is red (a label holds no TAB). The three extractions and the mawk-untested `[^\t]`
   regexes (mutations.sh:514, :525; `codereview-max-14d0b35d/candidates.md` :51, "CI has never run this range") go.
4. **Unchanged:** a `_control` label is literal; one label, one site (two calls on one line are one site: AL2/H-2,
   declared, §6 BS-5); `_mut_census_end` (a late call the text hides); the ratchets' semantics.
5. **The window verdict keeps its `$1`** (AL4/H-7). Under rule 2 its registration pins its presence and
   reachability; without the argument `$_fw_lbl` would fall into the echo-only class rule 1 pins by text only.
6. **Comment rewrites carried by CR-C1** are the sweep file's 44 C1 rows: ctlrun.sh:72-101 (the site rule as the
   bijection; the helper clause and the "`$(…)` … (both shells)" claim of :90-92 go; the primitives the controls file
   calls directly named correctly, SI6 :87-88); mutations.sh:15-18, :20-31, :49-57, :180-181, :192-197, :477-510;
   controls.sh:15-22 (`$_CONTROLS` is read by the entry check, the site walk and now the census, not by `_lbd`:
   D13/A3-7), :75-78, :98-99 (the census counts the expanded `_*_lbl` variables); records.sh:328-352; the design
   memo's :589, rows C (:615) and L (:620), and :691; and residual R13 for the #519 memo's banner :28-30 (§7).

### §3.3 K1's `_cn` dual producer, and D3

`$_cn_lbl` has two producers, the standing check (controls.sh:333, :358, :362) and `_cn_declined` (:382), against the
file's rule "ONE LABEL PER PRODUCER" (:75). **B takes the shape away rather than splitting the label:** CR-B2a deletes
`_cn_declined` and replaces the check with `_cap` (one producer: its call line and its own red line). The new labels
each have one producer: `$_capt_lbl` (the table check), the group and limit labels (the undeclared-absence check, one
line per row) and `$_cap_lbl`. A second producer made as a direct block under an echo-only producer's variable label
(mutations.sh:491-494, design memo row C) stays declared (§6 BS-6).

**D3, a standing check's own presence is pinned.** C delivers it for all three: `_wm`, `_ev` and `_cap` (R1 F1).
Deleted: its label has no expansion and no site, so its records are orphans. Wrapped in `if false`, or its row loop
emptied: its call line has no site. Until CR-C1, `_cap`'s presence is pinned the old way only (its label is still
"used" by its definition line); that intermediate state spans three slice boundaries, CR-B2a → B2b → B2c → C1 (R1
F21). The table check `_cap_check` is not a call line (it names no run primitive), so rule 2 cannot see its call
removed; its presence is pinned by the records it kills (GB2-12, G11).

### §3.4 C5 and U9 (CR-C2)

- **C5.** `_MUT_TARGETS` (mutations.sh:235) becomes **derived**: `_MUT_PARTS` minus `records`, the one exclusion, named
  with its reason (the record table is the subject list, not a part under test — the same reason it is out of rule 1).
  That removes the second hand list (SI3). The anchor check reds a record whose first expression begins `records:`
  (`sed` reads `r ecords:…` as a file read), and mutations.sh:227-234 is corrected (`r` and `w` take a file operand,
  so an unlisted prefix starting with either is valid, not a parse failure). Latent: a claim row, a planted
  `records:s/a/b/` record → red naming the prefix, both shells.
- **U9.** records.sh:163-164 → "Each forces one of the five inputs the trust computation reads (`_fw_post_bad`,
  `_fw_diag`; `_fw_done` is the W2 record's)". No claim about `_fw_post_ok` or `_fw_unread` without a measurement.
  Guard: the line, and `sed -n 739p` of the harness (the computation), in the claim table.
- **Folded here** (the same files, D9): CL6/S-7 (the dead empty-`_lbd_files` guard, controls.sh:141-143); CL7/SI7 (the
  `_MUT_RECORDS_MIN` changelog of branch-local hashes, mutations.sh:238-247, points at corpus §6.1 instead, the one
  statement); CL10/R-5 (the part path, `${SELF%.sh}.<part>.sh`). **The population is 13 code sites** ((M)
  `crplan-r2/logs/M-partpath.txt`): ten spelled `${SELF%.sh}.` — controls.sh:115, :138, :163, :665, harness.sh:908,
  mutations.sh:103, :117, :268, :681, :697 — and three quoted-stem globs `"${SELF%.sh}".` — mutations.sh:430, :432,
  :686. All 13 become one helper defined beside `_MUT_PARTS` (R3 H6: the globs are part of the population). **The
  completion grep** is `\$\{SELF%\.sh\}\.|"\$\{SELF%\.sh\}"\.` over the parts and the wire: the second alternative
  is H6's (r3's grep, `crplan-r3/bin/completion.sh`, had only the first and could not see the globs). At the frame
  it prints 16 lines: the 13 sites, wire:1235 (the wire finds the controls file before any helper exists) and the
  comment lines mutations.sh:28 and :669 ((M) `crplan-r4/logs/M-partpath4.txt`; r3's grep printed 13 lines, the
  ten sites and the same three others, `crplan-r3/logs/M-completion-frame.txt`). After CR-C2 it prints only
  wire:1235, the helper's definition and comment lines).

---

## §4 B3 / K15: done as CR-S5 (`5f08bd6b`)

B3 (the kill counted only where the needle stands in a diagnostic's label slot, mutations.sh:401-415) did not change
B's or C's design and is committed; its full opt-in at its head measured 223 entries, 0 not killed as named, 0 not
exercisable, both shells (`…-impl-cr.md` §S.1.4). What B and C take from it:
- CR-B2c edits the arm CR-S5 rewrote (return 3 for the wrong reason, mutations.sh:411-414): written on that result.
- §2.4's argument that a declined member's needle cannot be killed as named on that machine rests on the slot rule.
  **(M)** A label printed on a green-style line plus an unrelated red (`ctl_ok=1`, no diagnostic) is WRONG REASON,
  not a kill, on both shells (`crplan-r3/logs/M-slot3.txt`, clone `crplan-r3/work/green3`: the two statements are an
  **inserted** line after controls.sh:403, so no anchored line is edited; the plain run prints no anchor-check line.
  The previous revision's `work/green` edited :404, the anchor of records 362 and 363: G6).
- controls.sh:395-401 still gives, as the reason the window names a limited postcondition by its ID only, that "a
  label printed on a green line would let any unrelated red 'kill' that record"; the measurement above, and
  mutations.sh:401-408 ("The slot closes that for every summary line at once"), say it no longer does. Per §9 the
  comment is reworded in CR-B2b to state the slot rule, in the commit that rewrites :395-397, and the old sentence
  joins B2b's 0-hit phrase greps (§2.3). B's summary prints claims, not labels, and depends on neither form.

---

## §5 The edge matrix (mapped before any code)

### §5.1 Axis intersections (R1 F17: all 45 pairs)

The two pairs this table lacks (rule 1 × the table, rule 2 × `_cap_check`) are carried as claim row R3-C1-3 (§7).

G gate · S summary · N census (bijection, one site, literal, membership) · Rf `_MUT_RECORDS_MIN` (floor) · Rx
`_MUT_UNRECORDED_MAX` (exact) · X opt-in excuse · D records · L `WEBREF_WIRE_LACKS` (groups and `limit` IDs) · W
the three standing checks (`_wm`, `_ev`, `_cap`) and, listed with them, the table check `_cap_check` (G11: not a
standing check; its presence is pinned by GB2-12) · K the k2 shim.

| pair | invariant | mechanism | guard |
|---|---|---|---|
| G×S | the summary says what the gate did | both functions of (table, flag); the lookup is pinned | GB2-1…5 |
| G×N | a declined or refused call still registers | `_ctl_reg` is the gate's first statement (ctlrun.sh:129) | (M) S-root/S-nofifo PASSED: no orphan, ratchet exact (`M-sim.txt`); P4/P6 claim rows |
| G×Rf | the gate adds or removes no record | — | independent (Rf counts records, not runs) |
| G×Rx | the bare count does not depend on what declined | registration precedes the decision | (M) as G×N: bare 18 on the simulations; claim rows P4/P6 |
| G×X | a declined member's record is excused, never killed as named | gate declines before any labelled output | GB2c-1 |
| G×D | records 372/374 go with their subject; 377 re-aimed | removal; needle `$_cap_lbl` | Rf/Rx in CR-B2a's edit; GB2-9 |
| G×L | LACKS never changes the gate's decision | the gate reads the flag only | GB2b-3 (P2 claim row) |
| G×W | `_cap` tests the lookup, not the real table | synthetic `_CAP_TABLE` per row | GB2-1…6 |
| G×K | k2 is no member | B1 removes the prefix, no table row | B1's grep |
| S×N | a member that never ran cannot be summarised as run | census membership check | GB2-7 |
| S×Rf, S×Rx | the summary adds no label and no record | prints claims from the table, not labels | independent |
| S×X | — | the summary prints after `_mut_run`; the excuse reads the table, not the lines | independent |
| S×D | no record's needle is a summary line | claims are not labels (slot rule) | (M) `crplan-r3/logs/M-slot3.txt` |
| S×L | a declined group's `⚠` line appears only on a declaring machine | an undeclared absence reds before `_mut_run` and the summary | GB2b-1/2 |
| S×W | the summary is never printed on a run whose standing check red | `[ "$ctl_ok" -eq 0 ] \|\| exit 1` (controls.sh:684) precedes it | unchanged (ship red on any W red) |
| S×K | the k2 claim prints in the static lines, and is no label | text | B1's grep |
| N×Rf | adding a guard record raises the record count, never lowers the floor in silence | floor unchanged in semantics; each slice sets it to its count | slice completion |
| N×Rx | the bijection makes `.labels` machine-independent → Rx exact on every profile | rule 2 | GC-2, P1/P4/P6 rows |
| N×X | `_mskip`'s P-x half keys on `_lbd_names`, whose population gains the wire | `_p<x>*_lbl` names unchanged (the wire defines none; `_pfl_lbl` is new) | claim row: P-x excuse unchanged |
| N×D | every new label has a record; every removed label's records go | the census itself | ship green |
| N×L | none: LACKS names groups and IDs, not labels | — | independent |
| N×W | standing checks register in the run's registry, at one line | `_cap` drops `_cn`'s `_VFY` redirect | GC-3 (D3) |
| N×K | the k2 label is a literal `control` label, recorded by 100 | unchanged | (M) `M-k2shim.txt` ship |
| Rf×Rx | a record added for a bare label moves both counters, each set by the same slice's edit | separate counters | slice completion |
| Rf×D | each slice's floor = its measured record count | `_mutants \| wc -l` (corpus.md:333) | slice completion |
| Rf×X, Rf×L, Rf×W, Rf×K | — | the floor is static over the record text | independent |
| Rx×D | bare count unchanged by B (every new label gets a record) and by C rule 1 ((M) every name ≥ 1 expansion, `M-expansions.txt`) | measured per slice | slice completion |
| Rx×X, Rx×L | the bare count is computed before `_mut_run` and does not read LACKS | static census | independent |
| Rx×W | `_cap`'s label is recorded (GB2-1…6) | as every label | ratchet exact |
| Rx×K | k2's label stays recorded (record 100) | unchanged | (M) `M-k2shim.txt` |
| X×D | P-x excuse unchanged (1 only); capability excuse 1 or 3; an unexcused 3 red | `_mut_trial` return 3 | GB2c-1…3 |
| X×L | the opt-in reaches `_mut_run` only on capable or declaring machines; mutants inherit LACKS | `_cap_check` before controls; trial env (mutations.sh:360) | P3/P5 rows; GB2b-6 |
| X×W | a standing check's record is never excused | `$_cap_lbl` and the W labels are no member | GB2c-1's P1 trial |
| X×K | record 100 is never excused (k2 is posed everywhere) | no group | (M) `M-k2shim.txt` |
| D×L | the probe-flip records die where LACKS does not name their group/ID | `_cap_check` | GB2b-1/2/4; excused on a declaring machine |
| D×W | `_cap`'s records die on every machine | synthetic table | GB2-1…6 |
| D×K | record 373 goes; record 100 kills on every profile | B1 | (M) `M-k2shim.txt` |
| L×W | LACKS is read by `_cap_check` only, and a malformed word is its red | `_cap_check` (W) is LACKS's one reader | GB2b-3 (P8 claim row) |
| L×K, W×K | — | k2 is no group and no standing check | independent |

### §5.2 Profiles

Shell (b32, b53) and grep (BSD, GNU-like) change **no expected outcome** in any row; after B1 the grep axis changes
no k2 outcome ((M) `M-k2shim.txt`). **Capability profiles are simulated by behaviour, never by editing a line a record
anchors on** (R1 F6: an edited probe line makes the probe-flip record's anchor stale, so the P6/P7 trials would test a
different file): no FIFO = a failing `mkfifo` first on `PATH` (`crplan-r2/sim/nofifo/mkfifo`; the probe,
harness.sh:818, and the window, fixtures.sh:59, resolve it by `PATH`); root = a `chmod` first on `PATH` whose mode-000
call on the harness's permission probe is a no-op (`crplan-r2/sim/root/chmod`; harness.sh:827 calls `chmod` by `PATH`,
`[ -r ]` at :828 then reads it readable). **(M)** both, as of today's automatic path: `crplan-r2/logs/M-sim.txt`. A
P-f-limited `env` is simulated the same way (an `env` first on `PATH` that refuses `-0`; harness.sh's `_FGIT_ENVBIN` is
`_fgit_resolve env`), measured in CR-B2b. Real root (every mode-000 file readable) is not reachable here; the claim
rows say "simulated".

**The rule holds for every simulation, not only the profiles (G6).** A simulation is any edit or environment that
builds a profile or an instrument (a record's trial, which applies the record's own expressions, is not one). Each is
checked by `crplan-r3/bin/anchors.sh` (the always-on anchor rule, run standalone: every record expression must still
change exactly one line): **(M)** `crplan-r3/logs/M-r2sim-anchors.txt` over the previous revision's clones —
`untr` (edited controls.sh:197, record 354's anchor) and `green` (edited :404, the anchor of records 362 and 363)
broke it and are replaced by `untr3` and `green3` (inserted lines; every expression still changes one line,
`crplan-r3/logs/M-sim3-anchors.txt`); `guard` (edited :673) and `inst` (an inserted dump line) keep it, and the PATH
shims edit no file. (The same log's other failures are by design: the record trials `r*`, `shim100`, `shimarm100`
and `v1m2` fail only on records anchored on the line the applied record edits, as inside every real trial, where
the check is skipped, mutations.sh:557-560; `v1rec` is the measured refusal of GC-4's one-expression form.)

| # | profile | LACKS | gate | `_cap_check` | summary | census / Rx | opt-in |
|---|---|---|---|---|---|---|---|
| P1 | capable (macOS; CI's runner if Q1) | unset | all run | green | every group "ran" | bijection = all lines; Rx = N | no excuse |
| P2 | capable | `fifo` (stale) | all run | green | as P1 | as P1 | the fifo probe-flip record SURVIVES unexcused (the parent's flag is 1) → red (BS-3) |
| P3 | no FIFO | unset | 2 fifo members decline | **red** `$_fifo_lbl` | not reached (exit 1) | as P1 | not reached |
| P4 | no FIFO | `fifo` | 2 decline | green | `⚠ … 2 control(s) of the fifo group …` | as P1 | fifo probe flip excused (1); records 201/215/217/219 WRONG REASON → red (BS-7) |
| P5 | root | unset | 5 perm members decline | **red** `$_perm_lbl` | not reached | as P1 | not reached |
| P6 | root | `perm` | 5 decline | green | `⚠ … 5 control(s) of the perm group …` | as P1 | 86, 98, 99, 280, 281, 394 (1), 289 (3), perm probe flip (1) excused ((M) today: `M-k14.txt`) |
| P7 | root + no FIFO | `fifo perm` | 7 decline | green | both `⚠` lines | as P1 | P4's and P6's together |
| P8 | any | `fifoo` | as the flags say | **red** `$_capt_lbl` naming `fifoo` | not reached | — | not reached |
| P9 | `env` without `-0` | unset / `P-f` | all run | **red** `$_pfl_lbl` / green | P-f's `⚠` limit line on the declaring one | as P1 | P-f records excused by the P-x half (1), as today |

**Shell-sensitive points, each a claim row on both shells.**
- `${!flag-}`, and the empty-name abort it guards against: **(M)** `M-shell.txt` (above); the gate resolves the group
  row first (F5), and the U row pins it. A flag field that is not a name: **(M)** `crplan-r3/logs/M-shell3.txt`,
  `M-proto3.txt` (§2.2, reader 1); the reader's validation, and the F row (G8).
- ctlrun's source-time guard and the table's place after `_lbd`: **(M)** `crplan-r3/logs/M-proto3.txt`, both
  shells, the named message where guarded and an unlabelled `set -u` abort where not (§2.2; G3, G7).
- The table split: parameter expansion, because IFS-tab `read` collapses empty fields on both (M).
- The reader's nesting (R3 H4): function-local cursor and fields under bash's dynamic scope, `set -eu`: **(M)**
  identical on both shells, `crplan-r4/logs/M-nest.{b53,b32}.txt` (§2.2); GB2-17 in the real code.
- The registry line of a call inside a multi-line `( … )` and of a continuation line: **(M)** the same on both at the
  frame (`M-registry.txt`); re-measured for `_cap`'s call line in CR-B2a.
- Bash 3.2's posix-mode persistence of a prefix assignment (the reason for ctlrun.sh:37-46 and :131): removed with
  `_ctl_need`. No label-taking call carries a prefix assignment: `crplan-r2/bin/prefix-grep.sh` over the controls file
  → 0 (R1 F23). **(M)** `crplan-r2/logs/M-prefixgrep.txt`: on its fixture it hits all seven must-hit lines (plain,
  indented, `_ctl_run`, `_ctl_gate` in a subshell, after `if !`, after `||`, quoted values) and none of the six others
  (`x=0; _ctl_run …`, a `$(…)`, comments); at the frame, exactly the eight `_ctl_need=` lines (controls.sh:517, :567,
  :609, :651-655). Over the other six parts and the wire it hits records.sh:374 — that log's "0" there was wrong,
  corrected in `crplan-r3/logs/M-prefixgrep3.txt`. The hit is sed data in the record table's quoted here-document,
  not a call. The grep's assignment form is `NAME=VALUE` after a space, `;`, `&`, `|`, `(` or `!`, then
  `_control`/`_ctl_run`/`_ctl_gate`. In 374 (`… || ctl_ok=1$/_ctl_need=0 _ctl_run "$_mg_lbl" …`) the word after
  `||` is `ctl_ok=1$/_ctl_need=0`: NAME `ctl_ok`, value `1$/_ctl_need=0` (`$` ends the pattern, `/` is sed's
  delimiter, no space between), followed by ` _ctl_run `; the span the grep matches is `| ctl_ok=1$/_ctl_need=0
  _ctl_run ` (`crplan-r4/logs/M-prefix374.txt`). So the `ctl_ok=1$` before the `/` is what makes the NAME= form
  match, not `_ctl_need=0` itself. In 372 and 373 the whitespace-separated word holding the replacement's
  `_ctl_need=0` is `/_ctl_need=0`, which starts with `/` and is no NAME=, and the pattern's `_ctl_need="…"` sits
  inside `controls:s/^…`, preceded by `^`; neither has the shape, hits 0 ((M) each line alone, same log). All three
  go with `_ctl_need` (373 in CR-B1, 372 and 374 in CR-B2a), so the grep over every part reaches 0 at CR-B2a. The previous revision's `^[A-Za-z_]*=.* _control ` hits one of the seven.

### §5.3 The sweep ledger (G1)

One ledger, built by the mechanism's identifiers (the 30 groups of `crplan-r3/bin/idents.tsv`: every name the slices
add, remove or change the meaning of, G1's list and the ones the slices' edits add), over the 26 files G1 names:
**(M)** `crplan-r3/logs/M-sweep-raw.txt` (raw), `crplan-r3/logs/M-sweep-summary.txt` (counts): 702 hits, 580 sites, each one row with a class and the slice that changes it
(the sweep file, §1-§3; one rule per row, `crplan-r3/bin/classify.py`). 233 rows are owned — CR-B1 34, CR-B2a 122,
CR-B2b 14, CR-B2c 17, CR-C1 44, CR-C2 4, every slice 4 (the ratchet figures); six rows have two owners — and 347
stay (`stays` 138, `hist` 199, `unrel` 10). §2.1-§2.3, §3.2 item 6 and §7 take each slice's files and completion
from it; its checks are `bin/ledger-check.py` (a negative control at the frame: each slice fails with exactly its
owned rows, (M) `crplan-r3/logs/M-ledger-check-frame.txt`) and `bin/completion.sh`. From CR-B1 on they run from the
repo (§7, "The move").

---

## §6 Guard table

The binding rule (`…-impl.md`:271): "A guard: removed → something reds (named); shipped → green". Each new guard is
listed with the mutant that reds it (KILLED) and its survive clause: the guard removed plus the mutant → SURVIVES. A
guard that cannot be pinned by a record is a claim row with a measured reason (the ctlrun.sh:357-360 precedent).

| id | guard | KILLED by (needle) | survives with | pinned | slice |
|---|---|---|---|---|---|
| GB1-1 | the shim's `GREP_OPTIONS` arm | record 100 (k2 label), every profile (M) | the arm removed, GNU-like profile (M); on BSD the real grep kills it | opt-in record 100 | B1 |
| GB1-2 | the shim's `path` mark | always-on: mark removed → `CONTROL NOT EXERCISED (<k2>): PATH prefix never reached the run`, both shells (M) | — (the shipped run reds) | always-on | B1 |
| GB2-1 | gate lookup is exact | `ctlrun:` the equality made always-true (`$_cap_lbl`, row N) | row N removed | record | B2a |
| GB2-2 | gate lookup is not a substring | the equality as `case … *"$l"*` (row N) | row N removed | record | B2a |
| GB2-3 | the decline arm | `return 2` → `:` (row M/zz=0 "ran") | that row removed | record | B2a |
| GB2-4 | the member's own group decides | flag read from the first group row (rows Y) | rows Y removed | record | B2a |
| GB2-5 | trust before capability | the two arms swapped (untrusted row) | that row removed | record | B2a |
| GB2-6 | an undeclared group is red before expansion (F5) | the arm's `return 1` → `return 0` (row U "ran", both shells) | row U removed | record | B2a |
| GB2-7 | census: every member is a registered `control` label | a `controls:` record renaming one member row (`$_cen_lbl`) | the membership check removed → green on a capable machine | record | B2a |
| GB2-8 | table structure | a member row's group misspelt in the real table (`$_capt_lbl`) | the clause removed → the gate's F5 arm reds under the member's own label: WRONG REASON, not a survival; clause and arm both removed → SURVIVES. **(M) mechanism prototype** `crplan-r2/bin/capproto.sh`, `crplan-r2/logs/M-capproto.txt`: identical on both shells; with the guard deleted outright b53 aborts and b32 runs the member | record + claim row | B2a |
| GB2-9 | the channel is checked whatever the gate decides | record 377 re-aimed (`$_cap_lbl`, row X) | row X removed | record | B2a |
| GB2-10 | `_cap` no-arm and parse guards | a row key with no arm / a malformed row | the `*)` arm removed | record (K6 shape) | B2a |
| GB2-11 | `_cap`'s rows' expected reds stay in `.cap_err` (F16) | — | — | claim row: P1 ship output holds no `CONTROL FAILED (<$_cap_lbl>)`; with the redirect removed it does | B2a |
| GB2-12 | `_cap_check`'s call is present (G11: a table check, no call line for rule 2) | the records whose kill is `_cap_check`'s red: GB2b-1, -2, -4 (and GB2-8's) | the call deleted + GB2b-1 → SURVIVES (the decline is accepted, the run green) | records; claim row naming them | B2a/B2b |
| GB2-13 | `_cap_check`'s red reaches the exit (G9) | a planted malformed row → its red line and rc 1 | the call line's `\|\| ctl_ok=1` → `\|\| :` → the red line with rc 0 | claim row, both shells | B2a |
| GB2-14 | the flag field is a name before any indirect expansion (G8) | `_cap` row F (`a-b`, `1a`): red naming `zz`, identical on both shells | the reader's validation removed → b53 aborts, b32 differs ((M) `crplan-r3/logs/M-proto3.txt`, `M-shell3.txt`) | record (row F) | B2a |
| GB2-15 | ctlrun's source-time name guard (G3; replaces §9's F11 row) | latent: the guard prints no label slot, so no record is killed by it as named (as `crplan-r2/logs/M-slot-guard.txt` showed for mutations.sh's guard) | — | claim row, (M) `crplan-r3/logs/M-proto3.txt`: table renamed in controls only → `missing: $_CAP_TABLE`, rc 2; + guard removed → `_CAP_TABLE: unbound variable` at the first read of the table after the guard, rc 1, unlabelled; flag misnamed → `missing: $_fifo_okk`; both shells | B2a |
| GB2-16 | the table is read only after `_lbd` (G7) | latent | — | claim row, (M) `crplan-r3/logs/M-proto3.txt`: a group label's definition renamed → `_lbd` names it, rc 2 (table before `_lbd`: unlabelled `set -u` abort, rc 1), both shells | B2a |
| GB2-17 | the reader's pass nests (R3 H4: ctlrun owns the iteration, cursor and fields function-local) | latent: no shipped caller depends on a nested pass being wrong | — | claim row, both shells: in the real ctlrun, an outer `_cap_rows -` pass over the shipped table with an inner `_cap_group_of` on a non-member label (reads every row) in each callback → the outer pass visits every row with its fields intact; with the cursor made global (the r3 shape) → it visits 1. (M) on the standalone prototype: `crplan-r4/logs/M-nest.{b53,b32}.txt`, 11 of 11 vs 1 | B2a |
| GB2b-1 | undeclared absence, fifo | `harness:` the probe's `_fifo_ok=1` (:818) → `=0` (`$_fifo_lbl`) on a capable machine | the absence check removed | record; excused on P4/P7 | B2b |
| GB2b-2 | undeclared absence, perm | `harness:` the probe's `_perm_ok=1` (:826) → `=0` (`$_perm_lbl`) on a non-root machine | same | record; excused on P6/P7 | B2b |
| GB2b-3 | LACKS words are groups or `limit` IDs | latent (no shipped run sets LACKS) | — | claim row P8, both shells | B2b |
| GB2b-4 | undeclared P-f limit (F18) | `harness:` P-f's definition test forced to the limit branch (`$_pfl_lbl`) on a capable machine | the limit-row check removed | record; excused on P9 by the P-x half | B2b |
| GB2b-5 | an unknown limit ID is red | latent (every ID has a row) | — | claim row: a planted `P-z — …` limit line → red `$_capt_lbl`, both shells | B2b |
| GB2b-6 | mutants see the caller's LACKS (F15) | — | — | claim row, measured by the real opt-in path, not `bin/trial.sh` (G12 A2): mutations.sh:360 edited on a copy to `env -u WEBREF_WIRE_MUTANTS -u WEBREF_WIRE_LACKS …`, then `WEBREF_WIRE_MUTANTS=1` on P4 with `LACKS=fifo` over the fifo probe-flip record → its mutant reds under `$_fifo_lbl` (shipped: excused) | B2b |
| GB2c-1 | capability excuse | latent on a capable machine | — | claim rows P4/P6: the named records excused, by `_mut_trial` on the real records under the behaviour simulations, both shells; a P1 trial of 280 / 289 → killed as named | B2c |
| GB2c-2 | 3 only for a wrong reason with mutant rc 1 (G10) | on P6, a planted record whose needle is a perm member and whose edit ends the window (mutant rc 2, e.g. `fixtures:` the `built` marker line → `exit 1`, needle `an unreadable file fails closed`) → `_mut_trial` 2, the opt-in red; a MATCHED NOTHING planted on an excused needle stays red. (On P4, records 201/215/217/219 are mutant rc 2, (M) `crplan-r2/logs/M-k14.txt`, `crplan-r3/logs/M-nofifo217.txt`: 2, red, BS-7) | the rc test removed → the planted record returns 3 and is excused: SURVIVES | claim rows | B2c |
| GB2c-3 | an unexcused 3 is red (F7) | a record on P1 whose needle is no member, killed for the wrong reason → the opt-in red ("1 not killed as named") | that branch removed → green (arm-mutant row) | claim row | B2c |
| GC-1 | `.vlabels` by expansion | `harness:` the `_pkl_lbl` NOT EXERCISED line (:525) deleted (`$_cen_lbl`: its records orphan) | `.vlabels` back to mentions | record | C1 |
| GC-2 | call lines ↔ sites | `controls:` relcwd's `if ! _ctl_run "$_rel_lbl"` (:447) → `if false && _ctl_run …`; and a one-line `if [ "$_fifo_ok" -eq 0 ]; then _control … "zz new" …; fi` (K3) — both `$_cen_lbl` | the bijection removed | records | C1 |
| GC-3 | D3: standing-check presence | `_wm`'s, `_ev`'s and `_cap`'s row loops emptied (one-line edits) | the bijection removed | records | C1 |
| GC-4 | C3 site line | **a two-expression record** (R1 F8): `[ -z "$_fw_limits" ] …` (:403) gets `; _v1="$(_control "$CTL/clean" 0 "PASSED" "green is reachable" \|\| ctl_ok=1` appended, and :404 gets `)"; ` prepended (`$_cen_lbl`) | the bijection removed | record | C1 |
| GC-5 | one registry parse | `ctlrun:` `_ctl_reg` writes a fourth field | the field-count check removed | record | C1 |
| GC-6 | `records:` refused | latent | — | claim row | C2 |
| GC-7 | rule 1 and the table (G12 A1) | the probe-flip record GB2b-1, opt-in | `_cap_check`'s absence red line deleted: rule 1 still counts the table's `$_fifo_lbl`, so always-on stays green; the record then SURVIVES in the opt-in (BS-4) | claim row | C1 |
| GC-8 | the census's `$_CONTROLS` is guarded (G4) | latent | — | claim row: `_CONTROLS` unset before mutations.sh is sourced → its guard's `missing: $_CONTROLS`, rc 2; with the guard's word removed → the census's read aborts unlabelled; both shells | C1 |

**GC-4, measured (R1 F8; `crplan-r2/logs/M-v1.txt`, `M-sed.txt`).** BSD `sed` does insert a newline from `\n`, but
the anchor check requires each expression alone to change exactly one line (mutations.sh:568-598), so the
one-expression form inserting the triage's two lines is refused ("changes 2 lines", both shells). The two-expression
form passes it (`work/v1rec`: the only red is the other record's). Applied, the inner call (text :403) registers at
:404 on both shells — one site with the real call — so the trial SURVIVES at the frame on both shells (`work/v1m2`);
under rule 2 :403 is a call line with no site, the record's kill. (The triage's two-inserted-lines shape registers
correctly on 5.3 and folds on 3.2: `work/v1m1`.) C3's guard is a record, as the triage proposed; §10 P-e is withdrawn.

**Declared blind spots** (each stated where the code is):

| id | blind spot | why it stays | slot / owner |
|---|---|---|---|
| BS-1 | a table member deleted: on a machine without the capability it runs and reds. And the window reads `_fifo_ok` itself, not the table (fixtures.sh:17, :59, :496: whether to build the FIFO fixtures) | loud, fail-safe. The window is built from nothing and its prelude forwards values, never ctlrun's functions (ctlrun.sh:15-16), so it cannot call the reader; it builds from the probe's value, which `_cap_check` judges in the parent (G12 A1) | none (stated in the table's comment) |
| BS-2 | a control declared a member when it does not need the capability: declined where the group is absent | membership is a declaration, visible as a table edit and in the summary's count, with no effect on a capable machine | **new residual R11** in residuals.md §5.2 (CR-B2a) — gap: the table cannot test a member's need; Why: the need is the control's subject, not the table's; trigger: any `member` row added or changed (its commit states the need, reviewed as a table edit); owner: the citation-hygiene program, k2 trip-wire lane; recorded 2026-10-18 (R1 F20) |
| BS-3 | a stale `WEBREF_WIRE_LACKS` on a capable machine (P2): the ship run there carries no evidence for GB2b-1/2/4, since a wrong 0 would be accepted. The opt-in run is not blind: the flag is 1 there, so the probe-flip records SURVIVE unexcused → red | the declaration is the caller's; nothing shipped sets it | residual R4's launch-environment class, `#11-trip-wire-launch-environment`, named as a member: residuals.md:111 and landing.md:183-186's amend text (CR-B2b) |
| BS-4 | an echo-only producer made unreachable while its expansion stays in the text | rule 1 counts text; reachability is what its records test | the opt-in run (X3); stated at the census |
| BS-5 | two calls under one label on one line are one site (AL2/H-2) | the line is the finest site bash reports | already declared, mutations.sh:502-510 |
| BS-5a | a `#` after whitespace inside a quoted string is cut as a comment by rule 1, so an expansion there is not counted (fix round, I-1) | the cut errs one way: a dropped real expansion leaves its label uncounted, so its records orphan: a false red, never a false green | declared at mutations.sh rule 1's comment; design memo row C |
| BS-6 | a direct block under an echo-only producer's variable label | the literal rule is `_control`'s only | already declared, mutations.sh:491-494, design memo row C |
| BS-7 | **any record whose plant needs a capability the machine lacks** may not be killed as named there, and then a declaring machine's opt-in reds: a FIFO (201, 215, 217, 219, needle `$_pg_lbl`: WRONG REASON with mutant rc 2 under the no-FIFO simulation, both shells, (M) `crplan-r2/logs/M-k14.txt`, `crplan-r3/logs/M-nofifo217.txt`); mode enforcement (189, 190: `chmod 300`/`600` on the void; 296: a `chmod 0444` parent), not measured, real root being unreachable here | their needle is no member, and their mutant's 2 is the "decided nothing" a harness break also reports, so `_mut_trial` returns 2 (G10) and nothing excuses it: the fail-safe side, an opt-in false red only | stated at `_mskip`; the opt-in at T runs on capable machines (X3). **Re-evaluation trigger:** a no-FIFO or root declaring machine becomes a supported opt-in profile. (The old BS-7, the P-x limits, is closed by F18.) |

---

## §7 Slices

**Base: `2f1ca601`** (the first half and the record rollover). **Why B/C is in #527 (R1 F4).** CLAUDE.md's
edge-dense rule (c): the base case, the terminal unit, is a *narrowly-scoped* per-PR slice that passed plan-review
under an approved umbrella, and #527 is it (umbrella row A-i-wire-fgit, `docs/plans/2026-07-citation-hygiene-umbrella.md`:23;
the design memo's draft 21 answers plan-review round 19, design memo :36-38). B and C keep it narrowly scoped: they fix
review findings in mechanisms #527 itself introduced (K1, K3: IMPORTANT defects of its own census), so its scope
(A-i-wire-fgit) does not widen; that is its review tail, not a bundle (rule (a)), and splitting would land #527 with
two known IMPORTANT defects of its own guard. The user kept B/C in #527 (2026-10-18, DECISIONS-R7 "Second half");
this plan-review reviews B/C as a design (rule (b)). Each slice runs the R7 instrument (`…-impl.md`:265-275), and
**its entry goes to the open implementation record (design memo §13), currently `…-impl-cr.md` §S** (R1 F3). "Ship"
= the wire on both shells × both grep profiles: rc 0, PASSED.

**Files and completion are derived from the sweep ledger** (G1; the sweep file's §2): a slice's files are its ledger
rows plus the additions named below, and every slice's completion includes `ledger-check.py <head> <slice>` passing,
its greps in `completion.sh` at 0 (§2.1-§2.3), and `sweep.sh` re-run at its head with every new site classified in
its record entry — from CR-B1 on, the copies in the repo ("The move", below); `crplan-r3/bin/<name>` in this memo
names the copy that measured the frame.

**The move (R3 H1, in CR-B1).** This memo and the sweep file become
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-plan.md` and
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-sweep.md`, registered in the design memo's header (:5-22)
and the umbrella (:23; :111, "All eleven" → thirteen). A citation in the repo must resolve in the repo
(`…-impl-cr.md`:13-16), so the instruments the slices' completion conditions RUN move too: `sweep.sh`, `idents.tsv`,
`ledger-check.py` and `completion.sh`. **Where:** the provenance companion keeps its scripts inside itself, as an
appendix of the files as run (`…-provenance.md`:809-821, "§E Appendix — the corpus scripts (as run)"; a reviewer
re-runs them from there, `…-provenance.md`:168-169, and superseded versions live in the history). On that precedent
the four instruments go to **§E of `…-cr-sweep.md`, "the instruments (as run)"**, an appendix CR-B1 adds in the
move (the handoff copy has none), one fenced block per file, under its file name, verbatim; a slice copies each block
to scratch under that name and runs it there (no extractor: the copy is the block). Two changes, made in the move:
- **`ledger-check.py` reads what moves, not the handoff logs** it reads today (`crplan-r3/logs/ledger.tsv`,
  `sweep-sites.tsv`): the rows from §3 of `…-cr-sweep.md` (the site column through §1's site names) and each row's
  frame text as line N of `git show 2f1ca601:<file>` (2f1ca601 is an ancestor of every slice head). **(M)**
  `crplan-r4/logs/M-h1-inputs.txt` (`crplan-r4/bin/h1-inputs.py`): §3 at r3 was byte-identical to `mkledger.py`'s
  rendering of `ledger.tsv` (580 rows; r4 then edited dispositions only, site/identifiers/class/slice unchanged, the
  same log's r4 line); the 22 site names invert to one path each; 580 of 580 frame texts equal the `git show` line.
  **The check reads §3 as committed at CR-B1** (`git show <CR-B1 commit>:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-sweep.md`),
  not the working file, so a later slice editing §3 cannot make its own check pass; a slice that must change a §3
  row says so in its record entry, with the reason.
- **`completion.sh`'s C2 part-path grep gains H6's quoted-stem alternative** (§3.4).
`sweep.sh` already carries H1's exclusion of the two moved files (`crplan-r3/bin/sweep.sh`, edited in r4; **(M)**
`crplan-r4/logs/M-sweep-excl.txt`: at the frame and on a clone with both files copied in, 26 files and output
byte-identical to the pre-edit run, both shells; the pre-edit script on that clone reads 28 files, 1385 hits against
702). The (M) logs stay in the handoff directory (the header). The sweep file grows by the four instruments (105 lines
as they stand, `wc -l`, plus fences and headings).

**The §10 split (CR-B1; a standalone commit BEFORE the move commit).** §10 mixes two kinds of bullet, and the move
sends each to one place (CLAUDE.md's touch-time split: its own commit, so the move commit carries no reorganisation).
- **(a) What moves.** Every bullet of §10 that records an error of an earlier revision or of a review round, verbatim:
  "Errors in the r1 revision", "Errors in the r2 revision", "Errors in `CR-plan-review-R1.md`", "Errors in
  `CR-plan-review-R2.md`", "Errors in `CR-plan-review-R3.md`", and any later bullet of that kind. They go to
  `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` as a new section, so the record of each
  revision's errors lives in one place. What stays in §10: P-a … P-f, and any bullet that states a convention or a
  premise the body uses (none other at r4: the body cites P-e and no moved bullet).
- **(b) References that must still resolve.** `/usr/bin/grep -n '§10'` on this memo: three hits at r4, the
  heading, "§10 P-e is withdrawn" (§6, under GC-4) and R3-B2a-3's "…: §10.". The first two stay valid because P-a …
  P-f stay. The third pointed into the "Errors in `CR-plan-review-R3.md`" bullet, which moves; the row states the
  fact itself (mutations.sh:92's loop omits `$_cen_lbl` and `$_fw_limits`, which its consumes list :20-25 names), so
  the pointer is deleted in this edit and no information goes with it. After the split `/usr/bin/grep -n '§10'`
  must show only the heading, the P-e line and this split paragraph's own mentions (the first of its (b) listing is
  the r4 count, taken before the paragraph existed).
- **(c) Sweep.** The moved text is new text in reviews.md, which the sweep does not exclude, so it adds hits for the
  identifiers it names (`_ctl_need`, `_cn`, `_fifo_line`, …). CR-B1's record entry classifies every one `hist` (a
  dated record of past rounds; stays), as the ledger's rule for reviews.md rows already does.
- **(d) Registration.** The design memo's header (:5-22), which lists reviews.md's sections, gains the new section in
  the split commit.
- **(e) Order.** Split commit first (§10 bullets out, the new reviews.md section and header line in), then the move
  commit (which also carries the §8 → sweep §F split, an exception to "no reorganisation" decided at CR-B1: see
  `…-impl-cr.md` §S.2). The split commit leaves this memo's body and §7 unchanged except for (b)'s deleted pointer.

| slice | items | files (ledger rows; frame size → budget) and additions | completion conditions (besides the three above) |
|---|---|---|---|
| CR-B1 | U3, K8, K15's instance, K9's trigger, CL5; F9's carried correction; the plan memo's move | 34 rows: controls.sh (700 → shrinks), harness.sh (915 → **887**: the probe's 28 lines out; :8-10), mutations.sh:401-408, records.sh (−1; :367), corpus.md :203; **the move** (above): this memo and the sweep file into `docs/plans/` as `…-cr-plan.md` and `…-cr-sweep.md` (the sweep file carries the same §0 no-spec-surface heading), the four instruments into the sweep file's §E, registered in the design memo's header (:5-22) and the umbrella (:23; :111, "All eleven" → thirteen); adds: fixtures.sh (692 → +2, the shim), corpus §6.1 (a "remove" row for 373), residuals.md R10 | ship; record 100 KILLED on BSD and GNU-like, both shells; arm removed + 100 → SURVIVES on GNU-like (§2.1 table); `_MUT_RECORDS_MIN` = the measured count (222 on the prototype (M)); bare count unchanged (18); the move: §E's `ledger-check.py` at the frame reproduces `crplan-r3/logs/M-ledger-check-frame.txt`'s per-slice violation counts (B1 32 … C2 221), and §E's `sweep.sh` at the CR-B1 head reads 26 files, both shells |
| CR-B2a | K4, K10, U8, H-3, AL1-alt, AL3/H-6, RU5, CL3/SI8, CL4/SI5/RU3; record 377 re-aimed; G3, G5, G7-G9, G11 | 122 rows: ctlrun.sh (379 → ≤ +60 net: the reader `_cap_rows`, the guard, `_cap_group_of`, `_cap_flag_of`), controls.sh (shrinks ≥ 40: `_cn` 282-387 out; the table after `_lbd`, `_cap`, `_cap_check`, `_cap_lines` in), records.sh (−2, +~10), mutations.sh (:20-31, :481, :498-501), design memo (:476, :510, CN → CAP/CAPT/group rows; ≤ 760 lines from 746); adds: the membership check (~+10), corpus §6.1 rows, residuals.md R11 (BS-2) | ship; GB2-1…17 as tabled, both shells; mutations.sh's function guard, latent in a wire run (every reader function is first called at controls.sh:197 or :229, before mutations.sh is sourced at :673, so a missing one aborts there): a claim row sources mutations.sh in a shell that sets the guard's variables and defines none of the three functions → the guard's refusal naming `_cap_rows`, `_cap_group_of` and `_cap_flag_of`, rc 2, both shells (the shape of the provenance companion's direct-source cell, `…-provenance.md`:671); summary on P1 says every group ran; `crplan-r2/bin/prefix-grep.sh` over all parts and the wire → 0; `_cap`'s call line registers at its line, both shells |
| CR-B2b | D8; F18 (P-f); F10; G2 | 14 rows: controls.sh :395-403 (the slot rule, §9), harness.sh :343-349 (no growth), design memo :605, :609, corpus.md :194-200, landing.md :121-127, :183-186, :386 (Q1 on route (b); BS-3's slot member), residuals.md :111 (R4's member); adds: `_cap_check`'s absence and limit checks, 3 labels, the LACKS paragraph, records.sh (+3), mutations.sh:360 (comment), residuals.md R12 | ship (P1); GB2b-1/2/4 KILLED on P1 and SURVIVE as tabled; claim rows P2, P3, P5, P8, P9, GB2b-5, GB2b-6 (simulated by behaviour), both shells |
| CR-B2c | K14; F7; G10 | 17 rows: mutations.sh (`_mut_trial` :335-346, :411; `_mskip` :708-726; the caller :736-747: ~+18), records.sh:277-279, corpus.md :194-200 | ship; claim rows P4, P6, P7: `_mut_trial` over §2.4's records and the probe-flip records → excused with the group named (P4: 201/215/217/219 still WRONG REASON, rc 2, BS-7), both shells; GB2c-2, GB2c-3 rows; a P1 trial of 280 / 289 → killed as named |
| CR-C1 | K1, K3, C3, D3; SI6, D13/A3-7, AL4/H-7, CL9/RU2; G4 | 44 rows: mutations.sh (≈ +25 net from ≈ 817, a projection: 789 at the frame + B2a's membership check ~+10 + B2c's ~+18, re-measured at C1's base; **if the measured length passes 900, stop**: the census half, `_mut_correspondence` + `_mut_census_end`, is split out first as a standalone commit), ctlrun.sh :72-101, controls.sh (:15-22, :75-78, :98-99, :136; :234 and :270 reworded), records.sh :328-356 (record 356 re-aimed), design memo :589, :615, :620, :691; adds: records.sh (+~6), corpus §6.1, residuals.md R13 | ship; the triage's K1 cases (`exp/k1rel`, `exp/k1cn`'s `_cap` analogue, `if false` on relcwd / fsmon / verdict, harness.sh:525 deleted) red on both shells; `exp/k3unrec` red on both shells, on P1 and P4; GC-1…5, -7, -8 as tabled (GC-4 SURVIVES at its base, (M)); Rx re-measured (unchanged by rule 1, (M)); bijection green at ship on P1 and P6 |
| CR-C2 | C5, U9, CL6/S-7, CL7/SI7, CL10/R-5 | 4 rows: controls.sh :138, :143, mutations.sh :235, :238-248; adds: records.sh:163-164, the part-path sites | ship; GC-6 claim row; `_MUT_TARGETS` derived (no second hand list); no branch-local hash in mutations.sh's comments (`/usr/bin/grep -c '[0-9a-f]\{8\}'` on its comment lines → 0); the part path spelled once: the 13 sites through the helper, H6's grep 16 → wire:1235, the helper's definition and comments (§3.4) |
| CR-S8b | the round's docs and record | `…-impl-cr.md` §S (the round's closing entry, after each slice's own) | each statement re-measured at the final head; then X3: the full opt-in at the new T on both shells (`…-landing.md` §9.1) |

**The carried corrections** (D5: the 1414-line #519 memo, `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`,
is provenance, not touched this round) are residual rows in residuals.md §5.2, numbered in landing order, each on
R4's precedent (the row carries the correction and names the lines it corrects), each naming no identifier its
slice's grep removes. **Each row's gap / trigger / owner / disposition (R3 H2)**, the same for all three: the #519
memo states as current what the slice made false, and D5 forbids touching it this round; **trigger** = the PR that
next touches the #519 memo; **owner** = the lane that owns that PR (L3); **disposition** = applied to the #519 memo
and removed from residuals.md by that PR — under CV2 (`…-cr-sweep.md` §F row 12) it splits the memo first, as a standalone prereq,
then applies R10, R12 and R13 in the split's follow-up commit. Recorded 2026-10-18.
- **R10 (CR-B1; R1 F9)**: the banner :33-36 describes k2's question as asked of the run's grep each run and declined
  where that grep ignores the variable; since CR-B1 it is posed on every machine through the `goptgrep` shim and
  declined nowhere.
- **R12 (CR-B2b; G1)**: :424-428, :446-449, :452, :1274 and :1391 give the FIFO and permission controls the
  machine-limit shape ("a line in the run's own summary saying so, run stays green"; "NOT EXERCISED otherwise");
  since CR-B2b an undeclared absence is red, and the summary line is `_cap_lines`'s since CR-B2a.
- **R13 (CR-C1; the ledger)**: the banner :28-30 counts "the value of every `_*_lbl` name the parts spell"; since
  CR-C1 the ratchet counts the names expanded outside a comment in the wire or a part other than the record table.

**Required claim rows carried from plan-review R3** (its I items; R3's terminator: not re-designed here, measured on
the real code by the owning slice's instrument, each row in that slice's claim table). Each row: what is measured, on
which shell(s), the expected red / green. "Both" = b53 and b32.

*CR-B2a (7 rows).*
- **R3-B2a-1 (GB2-15's unguarded arm).** The table renamed in the controls file only, plus ctlrun's guard removed →
  the run aborts at the FIRST reader after the guard in the real code (the row names which reader and line, as
  measured), `_CAP_TABLE: unbound variable`, rc 1, unlabelled; both. Guarded: `missing: $_CAP_TABLE`, rc 2; both.
- **R3-B2a-2 (GB2-13's plant).** GB2-13's planted malformed row is a `fact` row (or a `member` / `limit` row), not a
  `group` row → its `$_capt_lbl` red line, rc 1; with the call line's `|| ctl_ok=1` → `|| :`, the same red line with
  rc 0 (the survive arm); both.
- **R3-B2a-3 (ctlrun's guard scope).** The row states which consumed names ctlrun's source-time guard asserts: the
  full WHAT IT CONSUMES list, or the row states why a name is left out. (R3 gives the precedents as guarding their
  whole list; controls.sh:65 does, but mutations.sh:92 leaves out `$_cen_lbl` and `$_fw_limits`, which its
  consumes list :20-25 names.) Measured by a grep in the claim table: the guard's names
  against WHAT IT CONSUMES, the difference empty or each entry with its reason. No shell axis (text).
- **R3-B2a-4 (WHAT IT DEFINES is complete).** ctlrun's WHAT IT DEFINES lists every name the reader defines, its
  internal names included (functions, out-globals, the reader's own globals and callbacks). Checked by the harness's
  own defines check if one exists, else a grep in the claim table: every name defined in the reader's lines appears
  in WHAT IT DEFINES. Expected: no name missing. No shell axis (text).
- **R3-B2a-5 (`_cap`'s trust-open shape).** `_cap` reads `_fw_trusted` itself before its rows, in the shape of
  `_wm`'s and `_ev`'s openings (controls.sh:223, :264), so ledger rows 129, 177, 272, 559 and 563 ("the three
  standing checks … read `_fw_trusted`") stay true. Stated in the claim table and checked by a grep of `_cap`'s
  opening line against those two. No shell axis (text).
- **R3-B2a-6 (the `_cap_rows` callback contract).** A callback returns 0 (the reader stops a pass by `_cr_stop=1`,
  not by a status, §2.2). Measured on the real reader: a callback that returns non-0, run once under an errexit
  caller and once under `||` (where errexit is off), on both shells; the row states what each does to the pass
  (rows visited, exit status) and the contract's red: what turns red when a callback returns non-0, named.
- **R3-B2a-7 (GB2-17 compares every field).** GB2-17's nested pass compares every field the reader sets before and
  after the inner pass, `_cr_nf`, `_cr_bad` and every `_cr_f[i]`, not only `_cr_f[0]` and `_cr_f[1]` (all that
  H4's standalone prototype compared, `crplan-r4/bin/nest.sh`). Expected: identical before and after on every
  row, both shells; with the cursor global (the r3 shape) it visits 1 row.

*CR-B2b (4 rows).*
- **R3-B2b-1 (GB2b-6's observable).** GB2b-6 records the observed per-entry line and the `_mut_exc` count, not the
  exit code (`_mut_exc`, mutations.sh:701, :745, printed at :753): shipped (mutants inherit LACKS) → the fifo
  probe-flip entry's line says excused and `_mut_exc` counts it; with mutations.sh:360 edited to `-u
  WEBREF_WIRE_LACKS` → that entry's line says it was not excused (its mutant red under `$_fifo_lbl`) and `_mut_exc`
  does not count it (§6 GB2b-6); P4, both.
- **R3-B2b-2 (the cost statement).** Where CR-B2b states LACKS's cost (beside `_cap_check`, and §2.3 "Its cost"),
  it says that a no-FIFO contributor's opt-in still reds (BS-7). Evidence: the P4 trials of records 201, 215, 217,
  219 → WRONG REASON, mutant rc 2, `_mut_trial` 2, the opt-in red; both (as `crplan-r3/logs/M-nofifo217.txt`).
- **R3-B2b-3 (K14's row carries BS-7).** The claim table's K14 row states BS-7's property on P4, P6 and P7, with
  the root (mode-enforcement) half unmeasured (real root unreachable here). No shell axis (text, citing the P4 row).
- **R3-B2b-4 (CR-S1's `sleep` fact).** CR-S1's owed fractional-`sleep` fact (§11 Q1) is written into
  `…-landing.md`'s X9 row, as a fact for route (b). No shell axis (text).

*CR-C1 (3 rows).*
- **R3-C1-1 (GC-8's insertion point).** GC-8's `unset _CONTROLS` is inserted after the last gated call and
  before controls.sh:673 (the line that sources mutations.sh, `. "$_MUTATIONS"`), on a line no record anchors (the anchor check over the
  edited copy: every expression still changes exactly one line). Both arms on both shells: guard present →
  `missing: $_CONTROLS`, rc 2; the guard's word removed → the census's read aborts unlabelled, rc 1.
- **R3-C1-2 (Q3 in X9).** §11 Q3 (mawk's `-F'\t'` split) is written into `…-landing.md`'s X9 row, route (b). No
  shell axis (text).
- **R3-C1-3 (§5.1's two pairs).** The slice's claim table adds the two intersections §5.1 lacks, each with its
  guard: rule 1 × the table (a group or limit label counted used through the table's expansion: GC-7) and rule 2 ×
  `_cap_check` (a table check is no call line, so the bijection cannot see its call removed: GB2-12). Both, through
  those rows.

*Every slice (3 rows, each re-stated in every slice's claim table).*
- **R3-all-1 (two-owner ledger rows).** `ledger-check.py` judges a row owned by two slices by one frame text. Each
  slice gives the six two-owner rows (controls :25, :26 and records :367, B1/B2a; mutations :21, B2a/C1; mutations
  :248, every slice/C2; corpus :194, B2b/B2c) a per-owner expected text, or states the manual backstop: those rows,
  read at its head. Expected: each owner's part gone at its slice, the other's still there.
- **R3-all-2 (wrapped identifiers).** The sweep is line-based, so a multi-word identifier wrapped across a line
  break is not seen (at the frame: harness :750-751 and landing :74-75, `standing check` / `route (b)`; mutations
  :198-199 and :741-742, `wrong reason`). Each slice names this as a declared limit of the sweep and runs the
  backstop: a `perl -0777` match of the multi-word EREs with `[\s#]+` between their words over the same files, every
  hit the line sweep lacks classified in the record entry. The join must admit a comment marker: **(M)** at the
  frame `\s+` finds 1 of the 4 (landing :74) and `[\s#]+` all 4 (`crplan-r4/logs/M-wrap.txt`). No shell axis (`perl`).
- **R3-all-3 (identifiers the slices add).** The slice that adds a swept-class identifier adds it to `idents.tsv`
  (§E) in the same commit, and its re-sweep classifies the hits: `_cap`, `.cap_err`, `_cr_*`, the test flag names
  (`_zz_ok`, `_yy_ok`), `_ctl_gate`'s capability arm, the residual range "R1–R9" (residuals :7), and for C2
  mutations.sh:227-234. Expected: every new hit classified; none unclassified.

**Clerical, fixed in r4 (R3's A4 MINs):** §8 (now sweep §F) "four hold items" → three; §7 CR-C1's "≈ 817" stated as a projection
from the frame's 789; ledger row 559's disposition; §5.2's explanation of the records.sh:374 hit.

---

## §8 D9 — the triage appendix, every item assigned

Moved to `…-cr-sweep.md` §F (the 30 items, each assigned once), to keep this memo under 1000 lines.

---

## §9 ⟦DRIVER⟧

No design question is open: R1 decided D-B4 (F18, §2.3); R2 decided G1-G12. The driver's two amendments of R1
(2026-10-18) stand as R2 restated them:
- **F11's guard: a claim row, moved to ctlrun (G3).** No entry guard prints a label slot, so none can be pinned by a
  record killed as named ((M) WRONG REASON, rc 2, both shells, for mutations.sh's guard: `crplan-r2/logs/M-slot-guard.txt`;
  `crplan-r2/work/guard` edits controls.sh:673, which no record anchors on). The old row (the guard in mutations.sh)
  could not tell its arms apart: `_CAP_TABLE`'s first reader is ctlrun, long before mutations.sh is sourced. It is
  replaced by GB2-15, the guard at the first consumer, whose two arms differ (named message, rc 2 / unlabelled abort,
  rc 1), measured on both shells (§2.2).
- **The comment at controls.sh:395-401 is reworded in CR-B2b** to state the slot rule (§4), in the commit that rewrites
  :395-397, and its old sentence (`printed on a green line would`) is one of B2b's 0-hit phrase greps (§2.3, §7).

---

## §10 Premise errors and gaps in the inputs

- **P-a.** The triage appendix is headed "CONFIRMED items", but 8 of its 30 items had a finder line only (AL1, AL4,
  AL5, EF-3, RU5, CV2, A3-8, SI6's `_ctl_gate` half); each slice re-measures its own first (AL5, EF-3, A3-8 are settled,
  sweep §F). **The appendix mixes two ID spaces:** a bare `AL<n>` ("AL1-altitude", "AL4", "AL5") is
  `codereview-max-14d0b35d/candidates.md`'s; `AL<n>/H-<m>` ("AL2/H-2", "AL3/H-6", "AL4/H-7") is POOL.txt's, where
  AL1-AL5 are H-1, H-2, H-6, H-7, H-8 (its AL5 is REFUTED, unrelated to the marker AL5). §1 and sweep §F keep both spellings.
- **P-b.** "Record 100", "280", "289", "372-374", "377" are records.sh **line** numbers at `2f1ca601`, not ordinals of
  `_mutants`; they move with every records edit, so the slices cite expressions.
- **P-c.** After B1 the grep axis changes no expected outcome (M); it stays in the ship condition as a regression check.
- **P-d.** The triage's S6 condition "summary lines unchanged on a capable machine" does not hold under D2: the summary
  is derived from the table, so its text changes deliberately (§2.2, reader 3).
- **P-e (withdrawn, R1 F8).** A two-expression record can express C3's insertion (§6 GC-4, (M)).
- **P-f (restated).** The triage's "full opt-in 220/0/0" was the old frame's count; CR-S5's opt-in measured 223/0/0
  (`…-impl-cr.md` §S.1.4), and CR-B1 brings the count to 222 (prototype, (M) `M-k2shim.txt`).
- **Errors of earlier revisions and of the review rounds** are recorded in `…-reviews.md` §CR (moved there, verbatim, by CR-B1's
  split commit).

---

## §11 Facts not read here (questions and where they are answered)

- **Q1 (G2, replacing R1 F19's "before CR-B2b").** Does `ubuntu-latest` give the `trip-wires` job a FIFO-capable
  filesystem and a non-root user, so P1 holds on CI? It cannot be measured before CR-B2b: #527 targets
  `webref-cite-audit-tool` and `ci.yml` runs only on pull requests to `main` (`on: pull_request: branches: [main]`), so
  no PR run of this wire exists (`main`'s `trip-wires` job runs four other wires, (M) `crplan-r2/logs/M-ci.txt`,
  `ci-main-aedc8002-tripwires.log`: image ubuntu-24.04). It is not a precondition either: by the fail-safe direction, a
  runner that lacks a capability makes CR-B2b's run RED (an undeclared absence), never a false green. **Route (b)
  answers it** (`…-landing.md` :121-123, land-order step 7): the first `trip-wires` run on #501's CI after the squash,
  where X9 already runs; a red there is fixed by a declaration in the job's `env:` beside `runs-on` (§2.3), recorded in
  the open record. CR-B2b writes this into landing.md (:121-127, :386). A docker `ubuntu:24.04` non-root run may be
  recorded before as a PROXY, never as the runner's fact: **not run** for this revision — `docker` is installed but its
  daemon is not reachable here (`docker info` rc 1, `crplan-r3/logs/M-docker.txt`). CR-S1's owed fractional-`sleep`
  fact goes to the same route (a fact only).
- **Q3 (moved to route (b) by G2's premise).** Does mawk (CI's `awk`) split the registry with `-F'\t'` as BSD awk does?
  The same route answers it; the direction is fail-safe as well: rule 3's field-count check reds any row whose split is
  not three fields, so a mawk that splits otherwise makes the run RED. A docker run is a PROXY only, as for Q1.
- (Q2 is answered by the frame, R1 F1 (§3.2 rule 2, "Trust"); Q4 by (M) `crplan-r2/logs/M-docs.txt` (§2.2).)

# K2 fixture git — PR #527 CR2 round: the stop watches its groups as one set (plan delta 2, R29-E)

**Status: accepted (plan-review TERMINAL 2026-10-23, `…-impl-cr2-d.md` §T.K), revision r3** (r1, r2 in the handoff as
`cr2/R29-PLAN-DELTA-2.r1.md` and `.r2.md`; r2 applied plan-review round 1 and r3 applies round 2, each with
the driver's decisions for it (the handoff's `cr2/PLAN-REVIEW-R29-D2-R1.md`, "Driver decisions for r2" and, under
"# Round 2", "Driver decisions for r3"), mapped finding by finding in "r2 changes" and "r3 changes" at the end. It changes surfaces that the accepted delta
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-liveness.md` ("delta 1") ratified, so it is reviewed as
the mechanism before the slice (`feedback_plan-ratified-surface-is-a-design-change`; the driver's dispositions of the
fresh R29-D review, the handoff's `DECISIONS-R7.md`, last section). Moved into `docs/plans/` as `…-cr2-plan-liveness-2.md`
by R29-E's docs commit (§9), body as accepted, the authority for §5's lines; the notes it supersedes stay records.

⚠ **Corrective notes** (dated; one line per falsified line, added by the slice that falsifies it, under G36's rule, as the notes atop `…-cr2-plan-liveness.md`: the body below is left as accepted; the line numbers are the body's as accepted, before the first note, which shifts it by the notes' own lines):
- **:476 (§6 pins, the R29s row)**, "a latch starts only from a mark" (2026-10-24, R29-E review): a latch starts from a mark or from a later probe. With the first read's `&& kill -0 "$_kg_p" 2>/dev/null` dropped, the per-tick read (the wire's `else _kg_w[_kg_p]=1`) still latches; only an unmarked group's first-read latch is lost, so R29s is red on the batch row only, not on the set row (whose group is free at tick 1). The comment at records.sh:565 and the corpus row (`…-corpus.md` :453) say it so.
- **:228 (§1.5 (iii), the deaf row's bound)**, "its build (≤ 2 s)" (2026-10-24, R29-E review): the build is 40 × (`ps` + `sleep 0.05`), ≈ 2.37 s on the measured host (so not ≤ 2 s); the 5 s bound's conclusion stands. controls.sh:580 says "≈ 40 × (0.05 s + a `ps`)".

**Names.** As delta 1: wire = `.claude/tools/webref-generic-core-trip-wire.sh`; controls, ctlrun, records, mutations =
its parts `…trip-wire.<part>.sh`; plan = `…-cr2-plan.md`; x = `…-cr2-plan-exitpath.md`; impl-d = `…-impl-cr2-d.md`;
residuals, landing, corpus = `…-residuals.md`, `…-landing.md`, `…-corpus.md`; design memo = `…-k2-fgit-hermetic.md`;
d1 = delta 1 (its body as accepted). **Every line cite names its file. A tool line written "wire :N" (controls, records,
ctlrun, mutations alike) is at `662a7065`** (the clone `cr2/work/r29d`; read with `git -C <clone> show 662a7065:<path>`);
**"draft wire :N" is the draft's** (below). d1, plan, x, impl-d, residuals, landing, corpus and design memo lines are
their bodies as committed at `662a7065`. Provenance: `wc -l` gives wire 1386, controls 923, ctlrun 439, records 577,
mutations 623; mutations :275 `_MUT_RECORDS_MIN=296`.
**The draft**: the mechanism written into scratch copies of the three parts at `662a7065` to measure it (r3: wire 1388,
controls 954, records 596), its scripts, diffs and logs in the session scratchpad
`/private/tmp/claude-501/-Users-kazuaki-repos-send-sh-elidex/f8b04f85-202d-453f-a184-0c2a4d6809eb/scratchpad/d2-evidence/`
("ev"): `wire.new.sh`, `controls.new.sh`, `records.new.sh` and their diffs against `base/` (the three parts at
`662a7065`); `mkr2.py` wrote r2 from r1's draft (`r1/`, kept), `mkr3.py` writes r3 from r2's (`r2/`, kept, with r2's
copies of the scripts r3 extends: `rows2.sh`, `errexit2.sh`, `cites.py`, `muts2.txt`); r3's logs under `logs/r3/`, r2's
under `logs/r2/`, r1's under `logs/`. r1's ship runs used a `git clone --local` of the clone, deleted after; r2's and
r3's new rows are measured at unit level (`rows2.sh`, which runs the controls file's own row text against the draft
wire's functions) and ship again in the slice (C4, C5).
Evidence for §1, §5, §6, §7; not the slice's text. Every row this memo adds or changes is written out in §6, so the
red measurement reproduces from this file alone.

---

## §0 What changed, and what this memo reviews

The fresh review of slice R29-D (`662a7065`; 0 CRITICAL / 2 IMPORTANT / 3 MINOR) found, among others:

- **I-2**, a defect of d1's K1 (§1.1 below, measured): the stop starts every latch by one batch `kill -0` (wire :466)
  and then runs `_k2_kill_gone` **one group after another** (wire :467). A leader that answers at the batch probe (the
  usual case: a selftest child still in its TERM trap) is not read again until its own loop starts, after every earlier
  group's grace and KILLs; freed, reaped and reused meanwhile, it reads "answers, latch unset, grace over" and is KILLed.
  The wire's own comment (wire :456–:457, "so no group's latch waits out the loops before it") and R19's window
  (residuals :127; d1 §8) are false.
- **I-1**: records :551–:553 say "with that bound removed each SURVIVES" (R29i, R29j) and records :556–:557 and
  :560–:562 "with that row removed each SURVIVES" (R29k, R29l; R29m, R29p), against impl-d §T.J's measured arms (impl-d
  :248–:250: two, two and three killers). **M-1** (the deaf row's readiness is `sleep 0.2`), **M-2** (its `sleep 30`
  outlives an interrupted run), **M-3** (the reap row needs zomb's 5 s member alive, bound unstated), and two clerical
  notes.

The driver's decision for I-2: one primitive that watches a **set** of groups each tick. This memo derives it (§1),
states where `662a7065` stands (§4, with the places it departs from the decision as given), and plans slice R29-E (§9).
M-1–M-3, I-1 and the notes are §6 pins and §9 docs.

---

## §1 The mechanism, derived

### §1.1 The defect, measured

`$_K2_STOP` (wire :464–:467): TERM to each group (wire :465); `_kw[g]=1` for each leader that does not answer *now* or
is marked `:1` (wire :466); then `for p; do _k2_kill_gone "${p%:1}" "${_kw[…]-0}" 2; done` (wire :467). `_k2_kill_gone`
(wire :452–:454) holds one group's latch and returns only when that group is gone, reused or exhausted. So group *j*'s
identity is read at the batch probe and next at the start of its own loop, after groups 1…*j*−1 have each run to their
end: at most the shared grace (≤ 2 s, it ends at `SECONDS` 2) plus 21 KILL ticks each (≈ 21 × (0.05 s + a probe) ≈
1.5 s; ≈ 21 s where `sleep` is integer-only, landing :399, X9).

**The set row** (§6, a shadow of `kill`, `ps` and `sleep` around `eval "$_K2_STOP"`, as the batch row does): two groups
`_og_z`, `_og_l`, both live in the table throughout; `kill -0 _og_l` answers at tick 0, fails at tick 1 (freed, reaped),
answers from tick 2 (reused); `_og_z`'s leader answers throughout. Measured on both shells:

| text | KILLs sent to `-_og_l` |
|---|---|
| `662a7065`, row alone (ev `rows.sh`, r1; ev `rows2.sh … new`, r2: `logs/r2/rows-662.{b53,b32}.log`) | **21** (b53, b32): the batch probe saw it answering (latch 0), its loop started at tick 23 |
| `662a7065` + the set row, full ship (ev `logs/red662.{b53,b32}.log`, r1) | RC 1, `set: the stop KILLed a group whose leader it had seen free after its first tick (-46683 -46683 …)`, both shells |
| the draft (§1.2), row alone (`logs/r2/rows-draft.{b53,b32}.log`; r3's draft, `logs/r3/rows-draft.{b53,b32}.log`); full ship, r1's draft (`logs/ship.*`, `new*.*`) | **0**; PASSED, 0 CONTROL FAILED, 0 names, both shells (3 ships each, r1) |

### §1.2 One primitive over a set

**`_k2_kill_gone <grace> <g>[:1]...`** (replaces wire :448–:454; draft wire :448–:458; still in `$_K2_STOP`'s
`declare -f`). State, per group, in two sparse indexed arrays keyed by the pgid (bash 3.2 has no associative arrays;
pgids are numbers > 1, the reap's filter, wire :488): `_kg_w[g]` the latch, `_kg_k[g]` its KILL count; the pending set
is the positional parameters.

1. **First read** (once, before any table read): for each argument, `:1` → latch set; else `kill -0 g` failing → latch
   set. This is the batch probe of wire :466, moved into the primitive (§4 D2.2).
2. **Each tick**, for each pending group, in order: `_k2_live -g` false → **done**; else, if `kill -0 g` answers: with
   the latch set → **done** (reused, left alone, not reported), with it unset → go on; if it fails → **set the latch**, go
   on; then `SECONDS` < `<grace>` → stays pending (no signal); else KILL count < 21 → `kill -9 -g`, count + 1, stays
   pending; count 21 → **exhausted**: appended to `_kg_left`, leaves the set.
3. After the pass: no group pending → stop; else one tick, `sleep 0.05 || sleep 1`.
4. **Its one answer**: `_kg_left` (a global, `""` first: set under nounset), the exhausted groups. It returns 0 (draft
   wire :458, `return 0`); no caller reads its status (errexit, below).

**Latch semantics.** The latch records that the group's leader was seen not answering: its leader exited, or (macOS) is
an unreaped zombie, which fails `kill -0` and keeps its id. Either way no new process can hold that id until it is
freed, so an answer after the latch is set comes from a different process: a reused id. It is read and set at every
probe of the group, not only at its start, and stays set until the group leaves the set. The identity read stays
adjacent to the KILL it licenses (`kill -0 g` then, with no table read between, `kill -9 -g`), as d1's probe-first
order had it, now pinned (the order row, §6). Ending probes stay `_k2_live`, identity probes `kill -0` (d1 §1.2's rule;
nothing moves between them).

**Shared grace, per-group bound.** The grace is a `SECONDS` threshold, so it is one for the set and costs nothing per
group (the stop's text sets `SECONDS=0` first, J14; `_k2_end` and the fallback pass 0, so they never wait and never
reset the caller's clock). The KILL bound stays per group (21): a total bound would let one deaf group starve the rest,
and a group's exhaustion is its own fact (it is what `_k2_end` keeps and names). Because every pending group is KILLed
on the same ticks, the set's KILL phase lasts at most 21 ticks plus a last pass **in all**, not per group (§1.4).

**Tick.** One length, 0.05 s, for grace and KILL alike (d1 had 0.1 s in the grace, 0.05 s between KILLs): one sleep
site, and the grace is where a selftest leader frees (the I-2 case), so its latch granularity is the one that counts.
`sleep 0.05 || sleep 1`: where `sleep` is integer-only the tick is 1 s (landing :399, X9); the grace still ends at
`SECONDS` 2. Both lengths are pinned (r3): the tick row (§6) drives the stop's text with a fake clock (`sleep` advances
it by its argument; `SECONDS` unset first, so it is a plain variable only that clock sets) and counts the ticks to the
grace's end: 40, and 2 where `sleep` refuses a fraction; R29ab (the tick made 0.5 s) and R29ac (the whole-second
fallback dropped) are its records. The tick sets R19 (b)'s width and C7's count, so neither rests on an unpinned value.

**errexit, and why the primitive has one answer.** The wire runs `set -euo pipefail` (wire :311), and none of the
primitive's three contexts lets errexit act. (1) The stop's processes start without `SHELLOPTS`: `_k2_stop`'s (wire
:472, `env -u SHELLOPTS -u BASH_ENV "$BASH" -c`) and the watchdog's (ctlrun :371, `exec env -u SHELLOPTS -u BASH_ENV
"$BASH" -c '…'"$_K2_STOP"`). (2) `_k2_exit` turns errexit off before `_k2_reap` (wire :514, `set +e`). (3) Every other
call runs inside `_ctl_run` (`_k2_reap_add` at ctlrun :353 and :374, whose failed record runs `_k2_stop`; `_k2_end` at
ctlrun :376), and every `_ctl_run` call is an operand of `||` or of `if !` (controls :233 states the rule for `_control`;
the sites: controls :360, :404, :461, :670, :849 and ctlrun :416), or sits in the `_og` check's `elif ! ( … )` (controls
:514), where bash ignores errexit in the whole body, as it does for the check's other calls that reach the primitive:
its `_ctl_run` (controls :522), the kill row's `( _K2_REAP_D=…; _k2_reap )` (controls :528), the name row's `_k2_stop`
inside `$( … )` (controls :586; draft :604), the fallback row's (draft controls :606, also inside `$( … )`) and the deaf
row's direct `_k2_stop` (controls :598; draft :553), which is in the body bare, with no `$( … )` around it. So the KILL's own status (a group gone between probe and KILL; on macOS, EPERM on a group
that turned zombie-only, wire :442's comment; on Linux `kill(-G, 0)` succeeds on such a group, by reading, impl-d :220
D5) needs no `|| :`, as d1's KILL had none (wire :454); the callers need none; and the primitive's status would be a
second expression of exhaustion that nothing reads, so it has none: it returns 0, and `_kg_left` is the answer.
Measured (ev `errexit2.sh`, both shells; r3: `logs/r3/errexit.{b53,b32}.log`, identical): with every KILL failing
(status 1), the primitive and `_k2_stop`'s fallback (`$BASH` a missing file) run to their end and set `_kg_left` /
`_k2_left` as an operand of `||` (shape A), under `if !` (B), in the `_og` check's nesting with `$( … )` (C), and (r3)
bare in that check's body, `if false; then :; elif ! ( f; echo inner ); then …; fi` (D, the deaf row's shape), which
prints `inner`: C alone does not show D, since in non-posix bash `$( … )` by itself turns errexit off; the same call
bare under `set -e` (the negative control) exits 1 before its end.

**bash 3.2.** `local -a … =()`, sparse `a[n]=1`, `${a[n]-0}` under nounset, `${@%:1}` (as wire :475) and `$((…))` parse
and run on 3.2.57 and 5.3.20 (`bash -n`; `rows2.sh`; r1's ships). No `case` arm sits inside `$( )` (the set and order
rows' `kill` is a function defined outside its `$( )`, as `_og_bt`, controls :580–:581). The pending list is rebuilt as
a string and re-split (`set -- $_kg_q`): digits and spaces under the default IFS, which no part changes globally
(`/usr/bin/grep -nE '(^|[^_])IFS='` over all nine files at `662a7065`, the wire and its eight parts: **25 hits**, wire 5,
controls 1, correspondence 3, harness 8, mutations 4, mutgen 2, records 2, ctlrun 0, fixtures 0; every one an
assignment prefixed to `read`, 22 `IFS= read` and 3 `IFS="$(printf '\t')" read` (harness :599, mutgen :315, :339), one
inside a records expression (records :222); the complement, `/usr/bin/grep -n IFS` less those forms, is two comments,
ctlrun :64 and :263).

### §1.3 The callers, and which answer each reads

| caller | call (draft) | was (`662a7065`) |
|---|---|---|
| the stop's text | `for p; do kill -TERM -"${p%:1}"; done` then `_k2_kill_gone 2 "$@"; exit 0` (draft wire :468–:469) | wire :465–:467 (TERM, batch, per-group loop) |
| `_k2_stop`'s fallback | `\|\| _k2_kill_gone 0 "$@"` (draft wire :475) | wire :473, a `case` per group |
| `_k2_end` | `_k2_kill_gone 0 "$2:1" "$3:1"; _ctl_left="$_kg_left"`, then each group not in `_ctl_left` dropped (draft wire :511) | wire :508–:509 (`_ctl_left=""`, per-group `\|\| {…}`) |

One text, one call shape (one-issue-one-way): the three sites differ only in their arguments.

**`_k2_stop` keeps its re-read** (wire :475; the driver left it open). The stop's text runs in its own process with its
streams on `/dev/null` and no fd 9 (wire :472): it cannot hand `_kg_left` back without a new channel, and K2's read (d1
§1.4) must also cover a stop that failed or was cut (the fallback; a stop process that died). The re-read answers "not
confirmed ended", which is K2's question; a reused id reads live there and is named falsely (fail-safe, d1's K2 row).
**`_k2_end` reads `_kg_left`**: it runs in-process, and the primitive's answer is the exact one — exhausted (kept, named
in `_ctl_left`) versus reused (left alone and dropped) — where a re-read would keep a reused id's entry and name it.
Two readers because two processes, not two decisions: the exhaustion decision is made once, in the primitive.

**The fallback's two cases** (impl-d :216): (a) `env` or `$BASH` cannot be started (a failed fork or exec); (b) the
stop's process is killed by a signal, which on the exit path cannot happen without ending the run (`_k2_exit` has set
`trap '' HUP INT TERM`, an ignore the stop inherits across exec, so only a KILL stops it, and that KILL ends the run,
which then runs no fallback), so (b) is `_k2_reap_add`'s path only. The fallback row (§6) pins (a).

**K2 unchanged.** `_k2_left` is still written only by `_k2_stop` and read only by `_k2_reap_add`; the names
(`_k2_name`), the keep (wire :505) and the reap's identity-unknown names (wire :491–:492) are untouched. `_kg_left` is a
second variable with its own one meaning (the last primitive call's exhausted groups), read only by `_k2_end`.

### §1.4 Bounds, and the identity window restated (R19)

**Wall clock of a stop** with *n* groups none of which a KILL ends: the grace (until `SECONDS` 2: 1–2 s) + ≤ 21 ticks
+ a last pass, each tick 0.05 s + *n* probes. A probe of a live group (`_k2_live` with one table read, then `kill -0`)
cost ≈ 17–23 ms in one measurement session per shell on this macOS arm64 host at load average ≈ 4.4 (ev `cost.sh`,
50 calls: b32 16.9–19.8 ms, b53 22.9–23.1 ms): one host's figure, not a bound. With it, ≈ 2 + 21 × (0.05 + 0.02 *n*) s:
*n* = 1 ≈ 3.5 s, *n* = 2 ≈ 3.9 s; where `sleep` is integer-only, ≈ 2 + 21 × (1 + 0.02 *n*) ≈ 23 s **for the set**
(estimated, not measured). At `662a7065` the KILL phases were sequential: ≈ 2 + *n* × 1.5 s, and 2 + *n* × 21 s
integer-only. A group a KILL ends leaves at the first KILL (deaf, measured: 1.7–1.9 s for 1 and for 3 groups, both
shells, ev `deaf.sh`; `662a7065` 1.35–2.04 s).

**R19's window, for this mechanism.** A wrong kill needs an id freed and reused where no probe of it saw it free:
(a) **before the first read**: from the reap's judgment of the leader (or a failed record's or the watchdog's launch)
to the primitive's first read; the first read precedes every table read, so for the stop this is the stop process's
start and the TERM loop, as at `662a7065`; in `_k2_stop`'s fallback after a stop process cut by a signal (case (b),
§1.3: `_k2_reap_add`'s path), the cut stop's latches die with it and the fallback reads from scratch, so (a) there also
spans that stop's lifetime, at most its bound above; (b) **between two probes of the group**: one tick plus one pass
over the pending groups' probes (on the host above, ≈ 0.07 s for *n* = 1, ≈ 0.09 s for *n* = 2; the tick is 1 s
where `sleep` is integer-only). **Gone**: the I-2 width (earlier groups' grace and KILLs) and d1's "in the fallback (no
batch probe) also during earlier groups' loops", because every caller now starts with the first read and no group
waits for another. The reason a closer guard is not built is d1's (a handle that outlives the id; a reuse inside one gap
needs a pid wrap inside it).

### §1.5 The deaf, reap and zomb rows (M-1, M-2, M-3)

**M-1, a handshake.** The deaf child writes `$_VFY/.og_dr` after `trap '' TERM` and before `exec sleep …`; the row
waits for the file, ≤ 40 polls, `sleep 0.05 || sleep 1` apart (zomb's build loop, controls :553–:554, polls the same
way but with `sleep 0.05` alone), and reds if it never appears ("ready: no"). An ignored signal stays ignored across
`exec` (POSIX XSH, the `exec` family's description: signals other than SIGCHLD that the calling image ignores stay
ignored in the new image; cited by name, not verified verbatim: POSIX is outside `.claude/tools/webref`'s sources), so
the marker proves the group TERM-deaf for its life. A late `trap` can no longer let TERM end the child and the row pass
without a KILL. **The `|| sleep 1`**: with `sleep 0.05` alone, an integer-only `sleep` fails each poll at once, so the
40-poll bound is ≈ 0 s and a child slower to write its marker reds the row falsely; measured with a shell function that
refuses a fraction (ev `deafint.sh`, 5 runs each, both shells, `logs/r2/deafint.log`): `sleep 0.05` alone spent 2–11
of its 40 polls with no wait before the marker appeared; with `|| sleep 1`, one poll and one 1 s wait. That path is
measured with the shim, not pinned by a row: declared in §6 ("Unpinned"), with its trigger.

**M-2, `sleep 10`, not recorded.** The value separates as before: unstopped (R29c, R29i, R29j) `_k2_stop`'s `wait`
lasts ≈ 10 s (measured 9.94 s / 9.95 s, b53 / b32, ev `deaf.sh` on r1's draft with R29i applied) against the `< 5 s`
bound, and a stopped one ≈ 1.7–1.9 s; each R29c/i/j trial pays 10 s, not 30. **What it leaves**: the deaf group is
ended by its own row's `_k2_stop`, or, when that fails or the run is interrupted during the check, by its `sleep 10`
alone; the cleanup (controls :600) signals zomb's ids only. So an interrupted run leaves it ≤ 10 s, as zomb's three
fixtures ≤ 5 s: smaller than at `662a7065` (30 s), not removed: residual R23 (§8), a departure from M-2 (§4).
Recording it in the reap set (`$_K2_REAP_D/og`) was rejected: the deaf group is a fixture of the check, not a call's
group (`$_og_lbl`'s population); the run's exit-path stop would spend its grace and a KILL on it (≥ 1–2 s on every
interrupted run); and zomb's three unrecorded fixtures would then be the odd ones (one rule for the check's own
fixtures: bounded by their own sleep, unrecorded).

**M-3, remove the dependency and state the bound that remains.** (i) The reap row shadows `_k2_live` to answer, as the
mark row does (controls :584), so it no longer reads zomb's `sleep 5` member (its subject is the record-less name, wire
:491). (ii) **The deaf row moves before zomb's build** (it uses no zomb id). At `662a7065` it ran between zomb's rows
and the cleanup (controls :600), whose `kill -9 -"$_og_l"` is a real signal to a group held only while zomb's holder
lives (5 s from its launch, controls :552); the deaf row's ≈ 2 s (and its `sleep 0.2`) sat inside that 5 s. (iii) The
comment above the rows (controls :561–:569; draft controls :568–:581) states the bound: zomb's processes live 5 s from
its launch; its build (≤ 2 s) and the rows run inside it, and **on the wire unmutated** the rows' only real signals to
its ids are the name row's TERM (to the zombie group, which the stop's `_k2_live` reads gone, so no KILL) and the
cleanup's KILL. Measured on r1's draft with stamps (ev `zt.txt`, 3 ships per shell): launch to cleanup 0.50–0.57 s;
r2's four rows add ≈ 0.16–0.19 s (the rows block alone, ev `rows2.sh`: 0.25–0.29 s against r1's 0.09–0.10 s, 3 runs a
shell, `logs/r2/rows-time.log`); r3's tick row adds ≈ 0.04–0.08 s (the whole `rows2.sh` run with r2's controls against
r3's: b53 0.281–0.303 → 0.342–0.345 s, b32 0.264–0.269 → 0.338–0.343 s, 3 runs a shell, `logs/r3/rows-time.log`); C12
measures the slice.
**Under a mutant the bound can fail** (declared, §6 "Unpinned", with its trigger). The "no KILL" above is the
unmutated `_k2_live`'s answer. A mutant that makes the stop's own `_k2_live` read the zombie group live, R29a (its
zombie filter dropped) on Linux, where `kill(-G, 0)` succeeds on a group of zombies (by reading, impl-d :220 D5), makes
the name row's real stop (controls :586; draft :604) run its grace and 21 KILLs to `-$_og_z`: ≈ 3.5 s on §1.4's host
figure, ≈ 23 s where `sleep` is integer-only (estimates; nothing here runs Linux). The cleanup's `kill -9 -"$_og_l"`
then comes that much later, and past zomb's 5 s in the integer-only case, as can the stop's own late KILLs to
`-$_og_z` once zomb's holder has died and its zombie been reaped. A wrong kill there needs that id reused within those
seconds (a pid wrap), in a trial whose mutant is already red (R29a is killed by zomb's arm). On macOS `kill -0` fails on
a zombie-only group, so `_k2_live` reads it gone before any table read and R29a cannot reach this.

**The move, audited both ways** (what each side of the deaf row now holds; read against draft controls :514–:638).
*Before it*, the `_ctl_run` rows (`kill`, `norec`, `nodel`, `sig`; controls :518–:547) ran before it at `662a7065` too;
zomb's build and rows no longer do. Deaf reads `$_VFY`, its own `_og_d`, `_og_w`, `.og_dr` (removed first) and `.og_d9`
(truncated by its `9>`), and `_k2_stop` (`$BASH`, `$_K2_STOP`, `_k2_live`, `_k2_name`); `_og_t0` and `_og_dt` are
assigned before they are read. The `_ctl_run` rows leave `_og_row`, `_og_rd`, `_og_st`, `_og_p`, `_og_f`, `_og_n`,
`SELF`, `_og_t0`, `_og_dt` and the nested set `$_K2_REAP_D/og`; their `_k2_reap` runs in a subshell (controls :528), so
no `_k2_left` or `_kg_left` comes from them. None of that is read by deaf; nothing zomb left was read by it either.
*After it*, zomb's build and every zomb row now run, with the cleanup. Deaf leaves `_k2_left` (`""`, or ` $_og_d` if it
was named), `_kg_left` (only if the fallback ran in the check's shell), `_og_d`, `_og_w`, `_og_t0`, `_og_dt`, `.og_dr`,
`.og_d9`, and a live deaf group only after a red. Zomb's build resets `_og_i`, `_og_z`, `_og_l` and writes `.og_zp`
fresh; every zomb row runs in a `( … )` or `$( … )` subshell except the zombie and live arms' first calls (draft
controls :563–:564, a direct `_k2_live`, which writes only its own `local _kl_n`); `_kg_left` is set first by the primitive (and by the end
row's shadow), `_k2_left` first by `_k2_stop` (the name row) or by the keep row's shadow before `_k2_reap_add` reads it;
`.og_d9`, `_og_t0` and `_og_dt` are not read after; a leftover deaf group holds no stream (`9>&- < /dev/null > /dev/null
2>&1`) and has no zomb id. Under R29c, R29i and R29j the stop's `wait` lasts ≈ 10 s before zomb is launched, so zomb's
5 s still counts from its own launch; at `662a7065` the same delay (30 s) fell after zomb's launch and before the
cleanup, whose `kill -9 -"$_og_l"` then reached an id ≈ 25 s after its holder died (5 s from zomb's launch; the cleanup
≈ 30 s after it): a wrong kill in a trial of those mutants if a pid wrap reused the id meanwhile. The move removes it;
impl-d §T.K records it as a pre-existing defect, closed (§9).

### §1.6 What stays apart (K3, restated)

The TERM line (J13) stays before the primitive (TERM is the stop's, not `_k2_end`'s). The observer polls (`_wm_gone`,
`_og_gone`) stay their own loops (d1 K3). `_k2_end`'s watchdog KILL (wire :508) stays before its call: it makes the
watchdog's group answer to its `:1` mark as it did to `<w>` 1. Nothing else re-KILLs: `/usr/bin/grep -n 'kill -9'` on
the draft wire gives draft wire :457 (the primitive's one KILL) and :510 (`_k2_end`'s pre-wait KILL).

---

## §2 Coupled invariants

Invariants as d1 §2: **LV**, **ID**, **ST**, **TB**, **KX**, **SW**, **FS**, **RF**, **S** (9, 36 pairs). Pairs this
delta changes, with their new mechanism and check:

| pair | intersection | mechanism | check |
|---|---|---|---|
| LV×KX | a false alive / gone in the loop | per group per tick, as before; ≤ 21 KILLs a group, then `_kg_left` | always-live, three-probe rows; R29k, R29l |
| LV×SW | the stop's process needs the probe and the primitive | `declare -f _k2_ptab _k2_live _k2_kill_gone`, unchanged | R29c, R29j |
| LV×TB | table reads per unit of time | one `_k2_live` (one or two table reads) per pending group per tick; the grace's tick 0.05 s (d1's grace polled every 0.1 s), so about twice the grace's table reads | C7 (counted, stated); the tick row, R29ab |
| ID×TB | the stop's window; the fixtures' ids in time | first read before any table read; latch at every probe of every pending group; zomb's ids signalled only inside its 5 s | set, batch, latch rows; R29m, R29p, R29s, R29w; C12 (zomb's bound); residue R19 (§1.4) |
| ID×KX | KILLing a reused id | identity read adjacent to the KILL; a set latch ends the group's turn; `_k2_end`'s marks | reused, latch, set, order, waited rows; R29y, R29x |
| ID×SW | the watchdog's stop while `_ctl_run` reaps the leader | the freed id latches at the next tick (≤ one tick + one pass) | latch row; R19 |
| ID×FS | alive-by-unknown and targets | `_k2_live` before `kill -0`, as d1; a reused id is neither KILLed nor reported; an unknown identity is named, not stopped | set, reap, age rows; R29q, R29z |
| ST×KX | exhaustion may leave a stream | `_kg_left` → `_ctl_left`, entry kept (`_k2_end`); K2 names (`_k2_stop`) | end, waited rows; R29u, R29v, R29x; name, keep rows |
| TB×KX | one bound | 21 KILLs a group on shared ticks: the set's KILL phase ≤ 21 ticks in all (§1.4) | always-live row (count); C5 |
| TB×SW | one clock | the stop's `SECONDS=0`; the callers in the caller's shell pass grace 0 and never assign `SECONDS` | latch row (grace 2); tick row (the grace's 2 s in ticks), R29ab, R29ac |
| TB×FS | alive-by-unknown runs to the bound | ≈ 2 + 21 × (0.05 + 0.02 *n*) s for the set on one host, then *n* names | estimate, §1.4 |
| KX×SW | the stop cannot report | K2's re-read, kept (§1.3); the fallback runs the primitive over the set when the stop cannot start | name, fallback rows; R29n, R29aa |
| KX×S | statuses | one answer, `_kg_left`; the primitive returns 0; no `\|\| :` (errexit cannot act in any of its contexts, §1.2); `_k2_reap_add`'s `exit 2` unchanged | C14 (`errexit2.sh`), C9 |
| SW×S | the stop's own status | still `exit 0` | — |

Unchanged from d1 §2, each with its check as accepted: LV×ID, LV×ST, LV×FS, LV×RF, ST×TB, ST×SW, ST×FS, KX×FS,
SW×FS, FS×RF, RF×S, and the independent pairs d1 lists (LV×S, FS×S, ID×ST, ID×RF, TB×RF, ID×S, ST×S, TB×S, ST×RF, KX×RF,
SW×RF).

---

## §3 Spec coverage map

**No spec surface** (as d1 §3: the siblings' §0 let `preflight.py` exit 1; each row maps one of this memo's sections, the
form the preflight accepts). POSIX is cited by name (§1.5), outside `.claude/tools/webref`'s sources.

| Spec section | Step | Branch | Touch | Full enum | User-input flow |
|---|---|---|---|---|---|
| none (this memo) §1.2 | one primitive over a set | first read / gone / reused / latch set / grace / KILL / exhausted | `_k2_kill_gone` | ✓ for the probe sites `/usr/bin/grep -nE 'kill -0\|_k2_live\|kill -9'` lists in the draft wire's primitive (draft wire :454 first read; :455 `_k2_live`, `kill -0`; :457 `kill -9`) | no |
| none (this memo) §1.3 | callers | stop / fallback / `_k2_end` | `$_K2_STOP`, `_k2_stop`, `_k2_end` | ✓ for the lines `/usr/bin/grep -n '_k2_kill_gone '` lists: draft wire :448 (comment), :469, :475, :511; wire :448 (comment), :467, :473 (two calls), :509 | no |
| none (this memo) §1.5 | the check's fixtures | deaf / reap / zomb | controls `_og` rows | ✓ for zomb's ids used with a real signal (the name row's stop, the cleanup, controls :600) | no |

---

## §4 Decisions, and where `662a7065` stands

| id | decision | at `662a7065` |
|---|---|---|
| D2.1 | `_k2_kill_gone <grace> <g>[:1]...`: every pending group probed each tick (gone → done; answers and latched → done; not answering → latch; grace → wait; else KILL, ≤ 21 a group); one tick, 0.05 s (`sleep 0.05 \|\| sleep 1`); one answer, `_kg_left`; returns 0 (§1.2) | **change**: replaces wire :448–:454 (d1 K1) |
| D2.2 | the batch probe becomes the primitive's first read, for every caller | **change**: wire :466 removed; the fallback gains it |
| D2.3 | the stop's text, the fallback and `_k2_end` call it once each, with the set (§1.3) | **change**: wire :465–:467, :473, :508–:509 |
| D2.4 | `_k2_stop` keeps its post-stop re-read; `_k2_end` reads `_kg_left` (§1.3) | re-read implemented; `_k2_end` **change** |
| D2.5 | errexit: no `\|\| :` inside or at a caller; the reason stated (§1.2) | **change** (r1's draft added three `\|\| :`; none ships) |
| D2.6 | deaf: handshake (`sleep 0.05 \|\| sleep 1`), `sleep 10`, before zomb; reap row shadows `_k2_live`; the zomb bound stated (§1.5) | **change** (controls) |
| D2.7 | rows: set, order, fallback, age, end, waited, tick (new), batch and shadow rows re-described (the set and shadow rows' fake clock unsets `SECONDS`); records R29u–R29ac (nine new), eight re-anchored (§6) | **change** |
| D2.8 | I-1's comments corrected in place; the notes by new dated notes (§5, §6, §9) | **change** (comments, docs) |

**Where this memo departs from the decision as given** (the driver's text, `DECISIONS-R7.md` last section, the brief,
and the plan-review decisions, `PLAN-REVIEW-R29-D2-R1.md`):
1. **"The separate batch probe disappears"**: it disappears from the stop's text but survives as the primitive's first
   read (D2.2). Measured reason: the pure per-tick form reds the existing batch row on both shells (r1's `rows.sh`: 21
   KILLs to `_og_l`), because group *j*'s first identity read then comes after groups 1…*j*−1's `_k2_live` table reads
   (≈ 20 ms each on the host of §1.4), and an id freed at TERM and reused inside them is KILLed. The first read costs no
   line (it is the loop that parses the `:1` marks, with `&& kill -0 …` added) and is one text for all callers, so it
   adds no site; R19 (a) keeps `662a7065`'s width, except after a cut stop (§1.4).
2. **M-3**: the driver offered "state the bound, or remove the dependency"; this memo removes the reap row's dependency
   **and** moves the deaf row before zomb, because the concept sweep (rows that use zomb's ids with a real signal) found
   the cleanup's `kill -9 -"$_og_l"` (controls :600) inside zomb's 5 s only by the margin the deaf row consumed; the
   bound that remains is stated and measured, and the move is audited both ways (§1.5).
3. **M-2**: "its `sleep 30` outlives an interrupted run" is made smaller (10 s), not removed: recording the fixture in
   the reap set costs every interrupted run a stop and breaks the one rule for the check's own fixtures (§1.5). What
   remains is residual R23 (§8).
4. **Additions not in the decision**: the end row with R29u/R29v (`_k2_end`'s rewritten exhaustion text would otherwise
   be unpinned; it was unpinned at `662a7065` too); from plan-review R1, the waited row with R29x (`_k2_end`'s `:1`
   marks, A2-3, unpinned at `662a7065`), the order row with R29y (the adjacency, A3-2), the fallback row with R29aa (the
   fallback's call, rewritten here) and the age row with R29z (impl-d §T.J :255's declared gap, A5-8): every item r1
   listed unpinned is now pinned or gone. S5 is re-anchored because `_ctl_left=""` on wire :508 becomes dead and is
   removed; R29l is re-anchored to `_kg_left` because the status it pinned no longer exists (A2-1).

**Where r2 departs from or reads a plan-review decision** (each stated, none silently changed):
- A2-1 "keep `kill -9 … || :` inside only if justified": not justified (§1.2, errexit), so dropped; the KILL is as
  d1's. "Retire or re-anchor R29l": re-anchored (same subject, exhaustion unreported; killed by the always-live row).
- A5-3 "one convention for C5, consistent with DECISIONS": the convention is the mean of three, the difference of the
  unrounded means, rounded once (§T.J's other per-commit figures follow it: b53 K1 +3.4, K2 +0.2 / +1.4). Under it K1
  against L4 on b32 is **+2.2 s** (§9). `DECISIONS-R7.md` :923's "+2.3 vs L4" is the difference of the rounded means
  (30.5 − 28.2); it agrees on the baseline (L4, not base), not on the figure, so this memo writes +2.2 and says why.
- A2-4 covers the deaf handshake. Its siblings, zomb's build wait (controls :553–:554) and `_og_gone` (controls
  :515–:516), keep `sleep 0.05` alone, whose consequence landing :399 (X9) already states ("a false red at worst, never a
  false green"); this delta does not widen that to them (no decision covers them; X9 names them).

**Where r3 departs from or reads a plan-review decision** (round 2, `PLAN-REVIEW-R29-D2-R1.md` "# Round 2"; each
stated):
- **IMP-1 and MIN-2, as decided, with one consequence of M3.** R29c's ship killers are deaf and, possibly, norec (R22's
  race, rate not pinned); its survive arm removes both; records :533 says so ("KILLED as R29j below"). The survive arm
  is C4's: the unit harness does not run the deaf and norec rows (each starts real processes), so it is not measured
  here. The tick row (M3) adds a deterministic unit killer to **R29i** (its mutant leaves `_k2_kill_gone` defined in the
  shell, and the row evaluates the stop's text in that shell), so R29i's survive arm also removes the tick row; it
  does not kill R29c or R29j, whose mutants remove the function from `$_K2_STOP`'s `declare -f`, which a text evaluated
  in-process does not need (measured, `logs/r3/muts.log`). Deaf stays the anchor for all three; norec is stated as
  "may also kill".
- **M3, pinned, with a second record.** The tick row has an integer-only arm, and R29ac (the whole-second fallback
  dropped) is its record, beside R29ab (the decision's mutant): the fallback is the other half of D2.1's tick and of
  X9's integer-only consequence, and the same row kills it.
- **Not in the decisions: the set and shadow rows unset `SECONDS`.** r3's runs found the set row red on the draft once
  in a full `rows2.sh` run (b53; that run's log was overwritten by its re-run, so it is reported, not logged), then 3 of
  300 (b53) and 1 of 300 (b32) runs of the row alone (ev `setprobe.sh`,
  `logs/r3/setflake-r2rows.log`): a real second boundary between the row's `SECONDS=$_og_t` and the grace test makes
  `SECONDS` read one more, the grace ends at tick 1, where the second group's leader does not answer, and the primitive
  KILLs that group (its correct behaviour for a leaderless live group; the row asserts no KILL). With `SECONDS` unset
  first, the fake clock alone sets it: 0 of 300 on each shell (`logs/r3/setflake-r3rows.log`). The shadow rows'
  `_og_kg` gets the same line, one form for every row that steps a fake clock (its latch row was 0 of 300 before too:
  the latch is set before any grace decision); the batch and order rows set `SECONDS=5`, which a boundary only raises,
  and stay as they are.

---

## §5 Ratified surfaces this delta changes

**By vocabulary** (r1's pass, kept): `/usr/bin/grep -nE 'batch|_kw\b|<w>|sleep 0\.2|sleep 30|0\.05–0\.1|R29s|deaf|one
KILL per group|per group|21 s|_k2_kill_gone <'` at `662a7065` over d1, plan, x, residuals, the design memo, landing,
corpus and impl-d gives **88 lines (44, 9, 10, 1, 1, 1, 7, 15)**; each was read.
**By property** (a line cite into a part whose target line moves or is rewritten by R29-E). r3's population is **every
file under `docs/plans/` at `662a7065` (96) and the wire with its eight parts (9)**: ev `cites.py` (r3: mutations :275,
whose value the slice rewrites, mapped as rewritten) takes every `:N` or `:N–:M` after a part's name on its line and maps
it through `difflib` from the parts at `662a7065` to r3's draft (`logs/r3/cites-all.out`): **302 hits, in 13 docs and in
no part** (the parts' own cites, e.g. correspondence.sh:245's `records.sh:382`, name lines R29-E does not move). By the
citing file's class:
- **Current-stating** (impl-d :261's list, design memo, corpus, landing, residuals, census, umbrella, the #519 memo
  `…-Ai-wire-k2-trip-wire.md` and its `…-review-rounds.md`, plus `…-cr2-sweep.md`): **35**, each classified:
  (a) **stale present-tense, re-pointed by the docs commit, 4**: residuals :121 (R9, "the wire :474", `_k2_stop`'s
  `wait` → draft wire :476), residuals :129 (R21, "controls :557", zomb's "a live group … read gone" → draft controls
  :564), landing :223 ("the wire :474" → :476; slot text, registered verbatim per landing :190), design memo :634 (row
  OG, "controls :558–:599" → the slice's range);
  (b) **rewritten in place, the same record at the same line, the citing prose read, 9**: design memo :632 (row WM)
  `records.sh:528` (S5; "`_k2_end`'s watchdog KILL+wait dropped" still true of the new anchor) and `:536` (R29d; its
  "grace test made `false`" is false of the new anchor, made true: **edited**, the table below); design memo :634 (row
  OG) `:554`, `:558`, `:559`, `:563`, `:564`, `:568` (R29i, R29k, R29l, R29m, R29p, R29s: their mutations and killers as
  row OG describes them change, e.g. R29l "the exhaustion status made 0", R29s "the batch probe dropped", R29m's
  killers: row OG is rewritten, the table below); landing :399 `records.sh:536` (names R29d and its pin only: true; X9
  is edited for other reasons);
  (c) **dated, at its own tree (`hist`), 20**: corpus :419–:440 (19, each row at the tree it records; the docs commit
  adds rows for the re-anchors, it does not edit these) and design memo :582 (`controls.sh:738`, at base);
  (d) **at another named tree, 2**: landing :461 (`PROV@ff1322bb`:837, :839).
- **Accepted plan bodies** (a cite names the body's line as accepted, at its frame; a falsified statement gets a dated
  note above the body, §5's table): **63** — d1 28, plan 20, x 4, `…-cr-plan.md` 11 (frame `2f1ca601`).
- **Implementation records** (dated entries, each at its tree, `hist`): **130** — impl-d 106, `…-impl-cr2.md` 16,
  `…-impl-cr.md` 7, `…-impl.md` 1.
- **`…-cr-sweep.md`** (the CR round's ledger, frame `2f1ca601`, its body provenance by its own corrective note): **74**,
  `hist`.
35 + 63 + 130 + 74 = 302. The parts' comments hold no line cite into a line R29-E moves (0 of the 302).

Falsified by this delta (d1 and the notes are current-stating; corpus rows and impl-d §T.J are dated history and gain
no edit):

| site (as committed at `662a7065`) | old text | new text |
|---|---|---|
| d1 :60, :61 (§1.2 table) | "the stop's batch probe … `<w>` 0 / 1"; "K1's latch … if seen free before" | the primitive's first read (every caller); the latch read at every probe of every pending group |
| d1 :66 | the call's end `_k2_kill_gone g 1` | `_k2_kill_gone 0 g:1 w:1`, `_kg_left` → `_ctl_left` |
| d1 :105–:113, :117–:122 (K1) | `_k2_kill_gone <pgid> <w> [<grace>]`; "one batch `kill -0`", "per group"; "the fallback … no batch probe"; "1 if still live at that bound, else 0" | §1.2 and §1.3 of this memo (one answer, `_kg_left`) |
| d1 :160 (ID×TB), :168 (TB×KX), :170 (TB×FS) | "from one batch probe"; "per group, sequential; … 21 s"; "n × 21 × …" | this memo's §2 rows |
| d1 :207 (K1) | "one batch probe starts the stop's latches" | D2.1–D2.3 |
| d1 :237, :240, :242 (its §5 rows for x) | "the batch probe, `_k2_kill_gone <g> <w> 2`"; "`_k2_kill_gone "$_ke_p" 1 \|\| {…}`"; "`_k2_kill_gone <g> 1`" | `_k2_kill_gone 2 "$@"`; `_k2_kill_gone 0 "$2:1" "$3:1"; _ctl_left="$_kg_left"` |
| d1 :264–:267 (deaf) | "`exec sleep 30`", readiness unstated | handshake, `sleep 10`, before zomb (§1.5) |
| d1 :268–:270 (batch), :271–:275 (shadow rows) | "the 2nd id fails `kill -0` until the 1st group's loop reads a table"; `<w>` forms; the status `1`/`0` | "first read"; `<grace> <g>[:1]` forms; `_kg_left` (§6) |
| d1 :287, :290, :293, :296 | killers "`<w>` 1 row", "latch row", "batch row"; "Each survive arm: its killing row removed" | §6's killer table (the last already corrected by impl-d §T.J) |
| d1 :329 (C13), :338–:341 (R19) | "latches start together"; R19's window | §7 C13; §8 R19 |
| plan note :22 (G11-2c) | "pinned by R11w, R29d and the `_og` deaf row (R29i, R29j)" | a new dated note below it: … deaf row (R29c, R29i, R29j) — the clerical note |
| plan note :23 (G11-10) | "`_k2_kill_gone <g> 1`" | a new dated note below it: `_k2_kill_gone 0 <g>:1 <w>:1` |
| x notes :16, :19, :20, :21 | "TERM, the batch probe, `_k2_kill_gone <g> <w> 2`; the fallback takes `<w>` from the mark"; "`_k2_kill_gone "$_ke_p" 1 \|\| {…}`"; "`_k2_kill_gone <g> 1`"; "≤ 21 KILLs (≈ 1 s; 21 s …)" | new dated notes below them: the set call; ≤ 21 KILL ticks for the set (≈ 1.5–2 s; ≈ 21 s integer-only) |
| residuals :127 (R19) | "starts at its batch probe … in `_k2_stop`'s fallback (no batch probe) also during earlier groups' loops" | §8 |
| residuals :121 (R9), :129 (R21) | "the wire :474"; "controls :557" | the slice's lines (draft wire :476; draft controls :564) |
| residuals :130 (R22) | Why: the remedies "need their own plan delta and plan-review (`…-cr2-plan-liveness.md` §5)" | §8: the reason this delta weighed, and its pointer |
| residuals :7, :94, :96; design memo :32, :57, :652 | the residual range "R17"–"R22" | "R17"–"R23" (R23, §8; `…-cr2-sweep.md`'s `resid_range`) |
| design memo :632 (row WM) | R29d "(records.sh:536, the stop's grace test made `false`, …)" | the grace test made true (`if :`), as §6's re-anchor; the rest of the row unchanged |
| design memo :634 (row OG) | the rows list (batch; deaf after keep), "controls :558–:599"; its records' mutations and killers (R29i, R29k, R29l, R29m, R29p, R29s at :554–:568) | adds set, order, fallback, age, end, waited, tick; deaf before zomb; the slice's line range; each record's mutation and killers as §6's killer table |
| landing :223 (§9.2, the slot's text) | "the wire :474" | the slice's line (draft wire :476) |
| landing :399 (X9) | "the deaf row's `sleep 0.2`"; "`_k2_kill_gone`'s grace poll (0.1 s) and its 0.05 s between KILLs"; "at most 21, so up to 21 s" | the deaf handshake's `sleep 0.05 \|\| sleep 1` (with an integer-only `sleep`, whole-second waits until the marker appears: no false red from it); one 0.05 s tick for grace and KILL; 21 ticks for the set, up to 21 s for the set |
| **budget**: the wire's stop line 1389 (d1 §5, Q8) | 1386 at `662a7065` | **1389 unchanged**; the draft is 1388 (+2: the primitive 3 → 6 code lines, the stop's text 4 → 3; comments 11 → 11, the primitive's 4 → 5, the stop's 7 → 6): 1 line spare (r3 adds no wire line); controls ≤ 1000 (draft 954), records ≤ 1000 (draft 596) |

**How the accepted files are corrected** (d1's precedent: d1 :5, superseded notes stay records; d1 :248, the notes'
form). d1 gets G36-form dated notes at its top (one line per site above, pointing here), its body as accepted. The
R29-D notes atop plan and x that this delta falsifies (plan :22, :23; x :16, :19–:21) are **not edited**: each gains a
new dated note (R29-E) directly below it that names it and supersedes it. impl-d gets §T.K (§T.J gains no edit).

---

## §6 Pins

**Kept, as named** (both shells): R29a–R29c, R29e–R29h, R29j, R29n, R29o, R29q, R29r, R29t, R11w, R11a. Their
expressions are unchanged; R29c's comment (records :533) is rewritten (its killers, as R29j's), and R29b (`_k2_live`
always gone) gains the tick row as a unit killer (the stop then sends no KILL; `logs/r3/muts.log`), which its comment,
naming its killer and claiming no survive arm, stays true beside.

**Re-anchored in place** (records lines do not move; each expression changes exactly one draft wire line, measured by
ev `mutrun.sh`, `logs/r2/muts.log`; r3 re-ran it, `logs/r3/muts.log`):

| record | line | new anchor → mutation |
|---|---|---|
| S5 | records :528 | `local _ke_p _ke_f=0` (no `_ctl_left=""`) then the watchdog KILL+wait, dropped |
| R29d | records :534 (comment), :536 | the grace test `if [ "$SECONDS" -ge "$_kg_s" ]; then` made `if :; then` ("made true") |
| R29i | records :554 | `_k2_kill_gone 2 "$@"; exit 0'` made `:; exit 0'` |
| R29k | records :558 | `[ "${_kg_k[_kg_p]-0}" -lt 21 ]` → `-lt 1` |
| R29l | records :556–:557 (comment), :559 | the exhaustion's `{ _kg_left="$_kg_left $_kg_p"; continue; }` → `continue` (exhaustion unreported, as `|| return 0` was at wire :454) |
| R29m | records :563 | `then [ "${_kg_w[_kg_p]-0}" = 0 ] \|\| continue` → `then :` |
| R29p | records :564 | `else _kg_w[_kg_p]=1; fi;` → `else :; fi;` |
| R29s | records :565–:567 (comment), :568 | the first read's `&& kill -0 "$_kg_p" 2>/dev/null` dropped (a latch starts only from a mark) |

**Rows (controls; the slice's text may differ in wording, not in what it asserts).** Changed:
- **shadow rows** (controls :571–:578; draft controls :583–:590): `_og_kg` calls `_k2_kill_gone "$@"` and prints
  `left$_kg_left` and its KILL count; the rows pass `0 "$_og_z"`, `0 "$_og_z"`, `0 "$_og_z:1"`, `2 "$_og_z"` and expect
  `left $_og_z 21` (always live), `left 3` (three probes), `left 0` (reused), `left 0` (latch). r3: its subshell starts
  `unset SECONDS;` (§4, r3: one form for every row that steps a fake clock).
- **batch** (controls :579–:582): unchanged text, re-described "free at its first read".
- **reap** (controls :589): `_k2_live() { :; };` added (§1.5 M-3).
- **end** (new in r1, after keep): `_k2_kill_gone` shadowed to set `_kg_left=" $_og_z"` (returning 0, as the primitive).
- **deaf** (moved before zomb's build, controls :548): `command rm -f "$_VFY/.og_dr"; set -m; ( trap '' TERM; : >
  "$_VFY/.og_dr"; exec sleep 10 ) 9>&- < /dev/null > /dev/null 2>&1 & _og_d=$!; set +m`, then `_og_w=0; until [ -e
  "$_VFY/.og_dr" ] || [ "$_og_w" -ge 40 ]; do _og_w=$((_og_w + 1)); sleep 0.05 || sleep 1; done`, then as before with
  `[ -e "$_VFY/.og_dr" ]` in the condition; `.og_dr` joins the check's removal list (controls :607).
- the comment above the rows (controls :561–:569; draft controls :568–:581) rewritten for set, order, fallback, reap,
  age, end, waited and zomb's bound (§1.5), the bound stated for the wire unmutated (r3, M1).

New, as measured (draft controls :595–:597, :598–:600, :606–:608, :612–:614, :618–:620, :621–:623, :624–:629):

```sh
# set (after batch)
_og_rg() { ( unset SECONDS; set -- "$_og_z" "$_og_l"; _og_t=0; sleep() { _og_t=$((_og_t + 1)); SECONDS=$_og_t; }; ps() { echo "$_og_z $_og_z S"; echo "$_og_l $_og_l S"; }
  kill() { case "$1" in -9) [ "$2" != "-$_og_l" ] || echo "$2" ;; -0) [ "$2" != "$_og_l" ] || [ "$_og_t" -ne 1 ] ;; esac; }; eval "$_K2_STOP" ) 2>&1; }
_og_r="$(_og_rg)"; [ -z "$_og_r" ] || _og_bad="$_og_bad; set: the stop KILLed a group whose leader it had seen free after its first tick ($(echo $_og_r))"
# order (after set): any table read between a group's identity read and its KILL marks .og_ph; a KILL that finds it reds
command rm -f "$_VFY/.og_ph"; _og_or() { ( set -- "$_og_z" "$_og_l"; sleep() { SECONDS=5; }; ps() { : > "$_VFY/.og_ph"; echo "$_og_z $_og_z S"; echo "$_og_l $_og_l S"; }
  kill() { case "$1:$2" in -0:-*) ;; -0:*) command rm -f "$_VFY/.og_ph" ;; -9:*) [ ! -e "$_VFY/.og_ph" ] || echo "$2" ;; esac; }; eval "$_K2_STOP" ) 2>&1; }
_og_r="$(_og_or)"; [ -z "$_og_r" ] || _og_bad="$_og_bad; order: the stop read a table between a group's identity read and its KILL ($(echo $_og_r | cut -c1-60))"
# fallback (after name): $BASH a missing file, so env fails and _k2_stop runs the primitive itself; ids answer, groups live
_og_fb() { ( BASH="$_VFY/.og_nobash"; sleep() { :; }; wait() { :; }; _k2_live() { :; }; kill() { case "$1" in -9) echo "$2" ;; esac; }; _k2_stop "$_og_z" "$_og_l:1" 9>/dev/null ) 2>/dev/null; }
_og_r="$(_og_fb | sort | uniq -c | awk '{ print $2 ":" $1 }')"
[ "$_og_r" = "-$_og_z:21" ] || _og_bad="$_og_bad; fallback: a stop process that could not start left \"$(echo $_og_r)\", not 21 KILLs to the unmarked group and none to the marked"
# age (after reap): an entry with a record time whose leader's age cannot be read
command rm -rf "$_VFY/.og_rd"; mkdir "$_VFY/.og_rd"; echo 0 > "$_VFY/.og_rd/$_og_l"
_og_r="$( ( _K2_REAP_D="$_VFY/.og_rd"; kill() { :; }; _k2_live() { :; }; _k2_age() { return 1; }; _k2_stop() { echo "stopped $*"; }; _k2_reap 9>&1 ) 2>/dev/null )"
[ "$_og_r" = "!! could not confirm that group $_og_l ended; it may still be running" ] || _og_bad="$_og_bad; age: an entry whose leader's age cannot be read gave \"$_og_r\", not one line and no stop"
# end (after keep)
command rm -rf "$_VFY/.og_rd"; mkdir "$_VFY/.og_rd"; : > "$_VFY/.og_rd/$_og_z"; : > "$_VFY/.og_rd/$_og_l"
_og_r="$( ( _K2_REAP_D="$_VFY/.og_rd"; kill() { :; }; wait() { :; }; _k2_kill_gone() { _kg_left=" $_og_z"; }; _k2_end end "$_og_z" "$_og_l" 9>&1; echo "left$_ctl_left" ) 2>/dev/null )"
[ "$_og_r" = "left $_og_z" ] && [ -e "$_VFY/.og_rd/$_og_z" ] && [ ! -e "$_VFY/.og_rd/$_og_l" ] || _og_bad="$_og_bad; end: a group live at the bound gave \"$_og_r\" or lost its entry, or the other kept one"
# waited (after end): the real primitive; both leaders' ids answer (reused), both groups live
command rm -rf "$_VFY/.og_rd"; mkdir "$_VFY/.og_rd"; : > "$_VFY/.og_rd/$_og_z"; : > "$_VFY/.og_rd/$_og_l"
_og_r="$( ( _K2_REAP_D="$_VFY/.og_rd"; kill() { :; }; wait() { :; }; sleep() { :; }; _k2_live() { :; }; _k2_end waited "$_og_z" "$_og_l" 9>&1; echo "left$_ctl_left" ) 2>/dev/null )"
[ "$_og_r" = "left" ] && [ ! -e "$_VFY/.og_rd/$_og_z" ] && [ ! -e "$_VFY/.og_rd/$_og_l" ] || _og_bad="$_og_bad; waited: \`_k2_end\` KILLed or kept a group whose waited leader's id answered (\"$_og_r\")"
# tick (r3, after waited): the stop's grace waits 40 ticks of 0.05 s to its 2 s, 2 of 1 s where `sleep` refuses a fraction (`sleep` a fake
# clock; `SECONDS` unset, so a plain variable the fake clock sets; 99 sleeps at most).
_og_tk() { ( unset SECONDS; set -- "$_og_z"; _og_c=0; _og_n=0; _og_m=0; ps() { echo "$_og_z $_og_z S"; }; kill() { case "$1" in -9) echo "$_og_n"; exit 0 ;; esac; }
  sleep() { _og_m=$((_og_m + 1)); [ "$_og_m" -lt 99 ] || { echo spin; exit 0; }; case "$1" in *.*) [ "$_og_ti" = 0 ] || return 1; _og_s="${1#*.}00"
    _og_c=$((_og_c + ${1%%.*} * 100 + 10#${_og_s:0:2})) ;; *) _og_c=$((_og_c + $1 * 100)) ;; esac; _og_n=$((_og_n + 1)); SECONDS=$((_og_c / 100)); }; eval "$_K2_STOP" ) 2>/dev/null; }
_og_r="$(_og_ti=0 _og_tk) $(_og_ti=1 _og_tk)"; [ "$_og_r" = "40 2" ] || _og_bad="$_og_bad; tick: the stop's grace took \"$_og_r\" ticks, not 40 of 0.05 s and 2 of 1 s where \`sleep\` is integer-only"
```

No new row sends a real signal (each shadows `kill`; the fallback row's `env` fails before any stop text runs), so
zomb's ids keep the two real signals of §1.5. Measured (ev `rows2.sh`, every row between `_og_kg` and the block's `fi`
except the name row, which starts a real stop): on r3's draft, all 15 rows green, both shells
(`logs/r3/rows-draft.{b53,b32}.log`); on `662a7065` (the rows that call through the forms it shares), the set row red
(21 KILLs) and the tick row red (`"20 2"`: d1's grace polled every 0.1 s), order, fallback, age and waited green, both
shells (`logs/r3/rows-662.{b53,b32}.log`: they pin what `662a7065` already did, unpinned). The tick row alone (ev
`tickprobe.sh`, `logs/r3/survive-unit.log`): the draft `40 2`, `662a7065` `20 2`, R29ab `4 2`, R29ac `40 spin`, both
shells. The set and latch rows, 300 runs each a shell (ev `setprobe.sh`): 0 red with `SECONDS` unset
(`logs/r3/setflake-r3rows.log`), set 3 and 1 red without (`setflake-r2rows.log`; §4, r3).

**New records** (appended before `MUTANTS`, records :576 → :595; all on `$_og_lbl`, an existing label):

| record | mutation (draft wire line) | killed by (unit, both shells) |
|---|---|---|
| R29u | `_k2_end`'s keep dropped (`case "$_ctl_left " in *" $_ke_p "*) continue ;; esac` removed; :511) | end row |
| R29v | `_ctl_left="$_kg_left"` made `_ctl_left=""` (:511) | end row |
| R29w | the stop's set split, `for p; do _k2_kill_gone 2 "$p"; done` (a later group's latch waits out the earlier ones: I-2's shape; :469) | set and batch rows |
| R29x | `_k2_end`'s marks dropped, `_k2_kill_gone 0 "$2" "$3"` (:511) | waited row |
| R29y | a liveness read added between the identity read and the KILL, `fi; _k2_live -"$_kg_p" \|\| continue; if` (the plausible "re-check it is live before signalling it"; :456) | order and three-probe rows |
| R29z | the reap's `_k2_age`-false name dropped, `… \|\| continue; fi` (:494) | age row |
| R29aa | `_k2_stop`'s fallback dropped, `      \|\| :` (:475) | fallback row |
| R29ab | the tick made 0.5 s, `sleep 0.5 \|\| sleep 1` (r3, M3; :458) | tick row |
| R29ac | the tick's whole-second fallback dropped, `sleep 0.05` alone (r3; :458) | tick row (its integer-only arm) |

`_MUT_RECORDS_MIN` 296 → 305 (mutations :275); `$_og_lbl` 27 → 36 records; the ratchet's 18 unchanged. **`R29aa`**: the
26 letters of round R29 are spent, so the 27th to 29th take two (R29aa–R29ac); no tool parses record ids (`git grep -nE
'R\[0-9\]|R[0-9]+\[a-z\]'` over `.claude`, `scripts` and `docs/plans` at `662a7065` finds only the residual-range
pattern `"?R1"?–"?R[0-9]+"?`).

**Killers after R29-E** (unit measurement on the draft: each record's expression applied to the draft wire, exactly one
line changed, then every row of `rows2.sh`, both shells; ev `mutrun.sh`, `muts2.txt` labels each expression by its
record id and records line; r3: `logs/r3/muts.log`, against r2's `logs/r2/muts.log` only R29b, R29d and R29i gain the
tick row and R29ab, R29ac are new; b53 = b32 for every record; "latch" is the row `rows2.sh` reports as "probe 2").
Rows outside `rows2.sh` (deaf, zomb, name, the `_ctl_run` rows, `_wm`) are C4's, at ship; C4 measures each survive arm
and I-1's comments are written from it. r1's `muts.txt` labelled R29w's expression "R29s" and R29s's "R29s2";
`muts2.txt` replaces it. **Unit survive arms measured in r3** (`logs/r3/survive-unit.log`, both shells, `RED: none`):
R29ab, R29ac, R29i and R29b with the tick row removed; R29d with the latch, set and tick rows removed (its `_wm` killer
is C4's). R29c's arm (deaf and norec removed) has no unit part: it is C4's.

| record | unit killers (`rows2.sh`) | killers at ship (C4) | survive arm |
|---|---|---|---|
| R29c | none (its mutant drops the functions from `$_K2_STOP`'s `declare -f`; a row that evaluates the text in-process still finds them) | deaf; norec may also kill it (R22's race, rate not pinned) | deaf and norec removed (C4) |
| R29i | tick | deaf; norec may also kill it (R22's race, rate not pinned; impl-d :248) | tick, deaf and norec removed |
| R29j | none (as R29c) | deaf; norec may also kill it (as R29i) | deaf and norec removed |
| R29d | latch, set, tick | `_wm` hang `.term` (its needle) | latch, set, tick and the `_wm` hang row removed |
| R29k | always-live, three-probe, fallback | — | all three removed |
| R29l | always-live | — | it removed |
| R29m | reused, latch, batch, set, fallback, waited | — | all six removed |
| R29p | latch, set | — | both removed |
| R29s | batch | — | it removed |
| R29t | mark | — | it removed |
| R29q | reap | — | it removed |
| R29r | keep | — | it removed |
| R29n | none (the name row is outside `rows2.sh`) | name | it removed |
| R29u, R29v | end | — | it removed |
| R29w | batch, set | — | both removed |
| R29x | waited | — | it removed |
| R29y | order, three-probe | — | both removed |
| R29z | age | — | it removed |
| R29aa | fallback | — | it removed |
| R29ab, R29ac | tick | — | it removed |

**I-1, in place**: records :552–:553 (R29i/R29j: deaf, R29i the tick row too, norec "may also kill", `sleep 10`),
:556–:557 (R29k/R29l), :560–:562 (R29m/R29p), :565–:567 (R29s), :533–:534 (R29c: "as R29j below"; R29d: latch, set,
tick and `_wm` hang), each from the table above; no line moves (draft records lines 1–575 differ only at :528,
:533–:534, :536, :552–:554, :556–:565 and :568; r3 appends :591–:594 before `MUTANTS`).

Measured in r1's draft clone (one-record trials, `r3-optin.sh … default`): R29w, R29u, R29v and R29s (re-anchored)
**KILLED as named, both shells** (ev `logs/opt-{581,579,580,568}.*`); R29v's expression and R29l's anchor change in r2
(no `|| :`), and the four new records are r2's and the two r3's: each is C4's at the slice.

**Unpinned, each with its trigger** (owner of all: §8's; r3 corrects r2's "none of this delta's own surfaces", which
§9's own list contradicted). This delta's own:
- **The deaf handshake's integer-only path** (`sleep 0.05 || sleep 1` in the row's wait, §1.5 M-1): measured with a
  shell function that refuses a fraction (`logs/r2/deafint.log`), not by a row (a row about a fixture's own wait would
  test the test). Trigger: a host whose `sleep` refuses a fraction runs the check (X9's route), or an edit to that loop.
- **Zomb's 5 s bound under a mutant that reads the zombie group live** (§1.5 M-3): on the wire unmutated it holds
  (C12 measures it); under R29a on Linux the name row's stop runs grace + 21 KILLs, and the cleanup's KILL (and the
  stop's late KILLs) can come after zomb's holder died: a wrong kill only if a pid wrap reused the id within those
  seconds, in a trial already red. Not measured (no Linux here). Trigger: route (b)'s first Linux run (C11) that runs
  R29a's trial — stamp zomb's launch and the cleanup there, as C12 does — or an edit to the name row, the cleanup or
  `_k2_live`'s zombie filter.
d1's unpinned list (d1 :299–:309) is re-evaluated, since its triggers name "a K2 line" and `_k2_end`'s exhaustion text
is one: the race the re-KILL closes (the set keeps a per-group re-KILL while live, ≤ 21), Linux `kill(-pgid)`
atomicity, the snapshot race L4 closes, exhaustion on a real unkillable member (not constructible; its logic is pinned
by shadow on both sides, the always-live row and the end row) and wire :472's streams each stand for d1's reason,
unchanged.

---

## §7 Claim rows

| id | claim | measured by | expected |
|---|---|---|---|
| C1 | I-2 removed | the set row on `662a7065` and on the slice, both shells | red (21 KILLs) before; green after |
| C2 | one KILL-until-gone text | `/usr/bin/grep -n 'kill -9' <wire>`; `/usr/bin/grep -n '_k2_kill_gone ' <wire>` | the primitive's KILL and `_k2_end`'s pre-wait KILL (draft wire :457, :510); the comment and 3 calls, each with the set (draft wire :448, :469, :475, :511) |
| C3 | no identity probe outside the primitive's two and the reap's | `/usr/bin/grep -nE 'kill -0' <wire>` | wire's comment (draft wire :442), `_k2_live`'s fast path (:446), the first read (:454), the per-tick read (:455), the reap's leader (:492) |
| C4 | records | one-record trials, both shells, every record of §6 (kept, re-anchored, new); each survive arm as §6's killer table defines it (R29c, R29j: deaf and norec removed; R29i: tick, deaf, norec; R29d: latch, set, tick, `_wm` hang) | KILLED as named; each survive arm SURVIVES |
| C5 | cost, no false name | ship wall time alone, 3 alternating runs a shell, `662a7065` vs slice, the difference of the means of three rounded once; `grep -c 'could not confirm'` | ≤ +1 s; 0 names (r1's draft, loaded host: 42/38/36/37 s vs 40/38/37/37 s, b53/b32 alternating) |
| C6 | CR11-K's race probe | `cr2/bin/cr2c2-r4race.sh` (its extraction ranges hold on the draft) N ≥ 600, 2 × ncpu load | 0 survivors, both shells |
| C7 | table reads | `ps` counted per ship, before/after | stated (the grace's tick is 0.05 s, not 0.1 s) |
| C8 | counts | `_mutants \| wc -l`; anchor gate | 305 both shells; 0 not fresh |
| C9 | status unchanged | `cr2/bin/cr2c2-f2probe3.sh` on the slice's exit block | 15 of 15 keep their status, both shells |
| C10 | budgets | `wc -l` | wire ≤ 1389, controls ≤ 1000, records ≤ 1000 |
| C11 | procps | landing X9's route (b) | zomb, deaf, set, order, fallback, age, end, waited, tick rows green |
| C12 | zomb's bound (the wire unmutated) | stamps from zomb's launch to the cleanup, instrumented lane copy, 3 ships a shell | < 5 s (r1's draft: 0.50–0.57 s; r2's rows add ≈ 0.16–0.19 s, r3's tick row ≈ 0.04–0.08 s, §1.5) |
| C13 | latches start at the first read and update every tick | the batch, set and latch rows | green; R29s, R29w, R29p red |
| C14 | errexit cannot act where the primitive runs | ev `errexit2.sh` on the slice's wire: every KILL failing, the primitive and the fallback as an `\|\|` operand (A), under `if !` (B), in the `_og` nesting with `$( … )` (C) and bare in its `elif ! ( … )` body (D, the deaf row's shape); and bare under `set -e` | A–D reach their end (D prints its `inner`); the bare call exits 1 (the negative control) |
| C15 | deaf readiness | R29c, R29i, R29j trials with norec removed (and, for R29i, the tick row), 5 a shell | 5 of 5 killed by deaf each; elapsed ≈ 10 s in the mutant |
| C16 | no stale line cite | ev `cites.py` over every `docs/plans/` file and every part at the slice's docs commit, against the slice's parts | every hit in one of §5's classes, current-stating hits each re-pointed, read in place or `hist`: 0 present-tense cite stale |
| C17 | the tick's length | the tick row on the slice and on `662a7065`; R29ab, R29ac | `40 2` on the slice, red on `662a7065` (`20 2`); both records red, both shells |
| C18 | the fake-clock rows are deterministic | ev `setprobe.sh` on the slice, 300 runs a shell | 0 red for the set and latch rows |

---

## §8 Residuals touched

Owner of each: the citation-hygiene program, k2 trip-wire lane.
- **R19, restated** (residuals :127). **Gap:** a latch starts at the primitive's first read (or at a mark: the reap's
  `<g>:1`, `_k2_end`'s two) and is read at every probe of its group: an id freed and reused (a) between the reap's
  judgment of its leader (or a failed record's or the watchdog's launch) and that first read, which in `_k2_stop`'s
  fallback after a stop process cut by a signal (`_k2_reap_add`'s path) also spans the cut stop's lifetime, its latches
  dying with it, or (b) between two probes of its group (one tick plus one pass over the pending groups' probes), is
  signalled. **Consequence, Why, Trigger** as d1 §8 (re-evaluated by 2026-11-01 or at route (b)'s first run). **Origin:**
  own (`f4fb2466`; widened by `c680a6ce`; narrowed by R29-D, whose batch-then-sequential form left (c) "an earlier
  group's grace and KILLs" open, measured in §1.1; closed by R29-E).
- **R22, its Why and pointer rewritten** (residuals :130; slot `#11-trip-wire-liveness-bound`). The defect and its
  measurement are unchanged: the set form leaves the TERM line verbatim (wire :465 = draft wire :468) and does not change
  what the grace does to a member that missed it. **Why (new):** the candidate remedies change the stop's TERM, grace and
  KILL surface. R29-E's plan delta (`…-cr2-plan-liveness-2.md`), the plan delta R22 asked for on that surface, weighed
  the first and did not take it: an early KILL once a leader is free would also KILL a selftest child whose leader exits
  before its members' cleanup ends, which is what the grace is for (a member that would act on TERM, R15 (b)'s class).
  The second, a second TERM before the grace, was not weighed (it re-delivers TERM to members already in their TERM
  handlers, unmeasured) and stays a remedy for its own plan delta, under the slot. **Trigger** as stated, evaluated for
  R29-E: not fired (the stop's TERM step is the verbatim line above; no stop was measured to cost its grace outside the
  `norec` row). The stop's text before that line grows by the primitive's three code lines (parsed before TERM is sent);
  R22's rate is not re-measured here (C5's ships report the `norec` row only by its verdict).
- **R23, new.** **The `_og` check's own fixtures are not in the reap set.** Deaf (`sleep 10`) and zomb's holder, member
  and zombie (`sleep 5`, `sleep 5`, `sleep 0.2`) are ended by the check (deaf by its row's `_k2_stop`, zomb's by the
  cleanup, draft controls :631) when the check reaches its end; a run that ends through its traps during the check
  leaves them to their own sleeps, ≤ 10 s and ≤ 5 s from their launch. **Consequence:** leftovers for at most those
  seconds; they hold none of the caller's streams (`9>&- < /dev/null > /dev/null 2>&1`), send no signal, and change no
  status. **Gap:** the exit path's reap does not reach a fixture the check launched outside `_ctl_run`. **Why:** a
  fixture is not a call's group (`$_og_lbl`'s population); recording it would cost every interrupted run a stop's grace
  and KILL, and one rule holds for the check's own fixtures (§1.5 M-2); the bound is the fixture's own sleep. **Trigger:**
  a fixture whose sleep exceeds 10 s, a new unrecorded fixture in the check, or a report of a leftover `sleep` after an
  interrupted run; re-evaluated by 2026-11-01, as R19 and R22. **Origin:** own: zomb's row is the Codex R29 fix's
  (impl-d §T.H), the deaf row R29-D's (impl-d §T.J (2)), both after `e8f78896`; R29-E cuts deaf's 30 s to 10 s.
  **Recorded** in the residuals row as `Recorded YYYY-MM-DD`, the date of R29-E's docs commit, the form R19–R22 use
  (residuals :127–:130, e.g. "Recorded 2026-10-23").
- R9 and R21: their line cites re-pointed (§5); otherwise unchanged. R20 unchanged.

---

## §9 The slice

**R29-E**, after acceptance, on top of `662a7065` in `cr2/work/r29d`, by plan §11's procedure (claim and arm tables;
battery b53/b32 with `BASE_REV=662a7065`; a fresh reviewer on a frozen snapshot; re-check on an IMPORTANT; diff read;
transplant; ship ×4; `scripts/trip-wires.sh`). Code by a coder, docs by a designer.

**Commit order**, each commit green on both shells (`bash -n`, the anchor gate, `_mutants`, ship, its own records'
trials), so one failure hides no other:
1. **The fixtures (M-1–M-3)**: the controls — deaf's handshake (`sleep 0.05 || sleep 1`), `sleep 10`, its move before
   zomb, the reap row's `_k2_live`, the zomb bound in the comment (for the wire unmutated) — and two records comments,
   no record expression: records :533 (R29c: "as R29j below") and :551–:553 (R29i/R29j: deaf, `sleep 10`; norec "may
   also kill"). C12, C15.
2. **The set primitive (D2.1–D2.5, D2.7)**: the wire (primitive, the stop's text, the fallback, `_k2_end`, the two
   comments); controls (shadow rows with `unset SECONDS`, batch's wording, set (with `unset SECONDS`), order, fallback,
   age, end, waited and tick rows, the end row's shadow); records (eight re-anchors, I-1's other comments including
   :534's R29d and :552's tick for R29i, R29u–R29ac); mutations :275 → 305; C6's script re-run (fixed in place only if
   its extraction fails). C1–C4, C8, C13, C14, C17, C18.
3. **Docs** (the designer): this memo into `docs/plans/` as `…-cr2-plan-liveness-2.md` (status accepted; `preflight`
   exits 0 on it, as on d1); G36 notes atop d1, and new dated notes in plan and x below the R29-D notes they supersede
   (§5; including the plan :22 note's R29c); impl-d §T.K: the route, the commits, claim rows, the C5 correction (§T.J's
   per-commit K1 figure for b32, +2.4 s, was taken against base, 28.10 s; against L4 it is **+2.2 s**: means of three,
   L4 28.23 s (27.8, 27.6, 29.3), K1 30.47 s (30.7, 30.4, 30.3), the difference of the means rounded once,
   `c5dec/q1q3.out`), and a **"Declared, not pinned"** paragraph in §T.J's idiom (impl-d :255): d1's unpinned list as
   re-evaluated (§6), this delta's two (§6: the deaf handshake's integer-only path, measured with a shim function, not
   by a row; zomb's bound under a mutant that reads the zombie group live), each with its trigger, and the closure of
   §T.J :255's declared gap by R29z (dated, §T.J unedited); a **pre-existing defect, closed** (dated): at `662a7065`,
   in a trial of R29c, R29i or R29j, the deaf row's unstopped `sleep 30` held the check ≈ 30 s after zomb's launch, so
   the cleanup's `kill -9 -"$_og_l"` reached that pgid ≈ 25 s after zomb's holder died and freed it, a wrong kill if a
   pid wrap reused it meanwhile; closed by the deaf row's move before zomb (§1.5); residuals R19, R22 and R23 (§8), R9's
   and R21's line cites (§5), the range at residuals :7, :94, :96; corpus §6.1 (nine adds, eight re-anchors, totals
   211 / 305); design memo :632 (row WM: R29d's form, §5), :634 (row OG), :32, :57, :652 (the range) and :22 (the file
   list, in place, so its `wc -l` stays 759
   against its 760 budget, impl-d :230); umbrella :23, :111 (twenty-two → twenty-three files; which one exits 0: now
   two); `…-cr2-sweep.md` :68 (the exclusion comment names this memo) and `idents.tsv` (`_kg_left`); landing :223 (the
   slot's line cite) and :399 (X9: the handshake's waits and their integer-only consequence; one tick; 21 ticks for the
   set) and X10 (this slice's commits, `wc -l`, the cohesion re-judgment). C16 after the docs commit; C5, C7, C9–C11 in
   the battery pass after (3).

**Cohesion re-judgment** (CLAUDE.md's touch-time split; the wire > 1000 lines): the primitive is carried by
`$_K2_STOP`'s `declare -f`, built as the wire loads, so it stays beside the stop (d1 §9's reason); no new seam; not
split. Controls (954) and records (596) stay below 1000.

---

## r2 changes

Plan-review R1 (`PLAN-REVIEW-R29-D2-R1.md`: CRITICAL 0 / IMPORTANT 2 / MINOR 25 / FP 9), each finding → the sections
r2 changed:

| finding | change | §s |
|---|---|---|
| A5-IMP1 | line cites enumerated by property (`cites.py`, 35 hits, 4 stale), added to §5's population and §9's docs; C16 | §5, §7, §8, §9 |
| A5-IMP2 | R22's Why and pointer rewritten (the reason moved from §4 "Not taken"), trigger evaluated: not fired | §4, §8 |
| A2-1 | one answer, `_kg_left`; the primitive returns 0; the shadow rows read `_kg_left`; R29l re-anchored; R29v's expression | §1.2, §4, §6, §2 |
| A3-3 | every caller's `\|\| :` and the KILL's dropped, controls :233 and the three contexts cited; C14 re-measured in the production shapes with a negative control | §1.2, §2, §4, §7 |
| A2-2 | R19 (a): the fallback after a cut stop spans that stop's lifetime | §1.3, §1.4, §8 |
| A2-3 | the waited row and R29x pin `_k2_end`'s marks | §2, §4, §6 |
| A2-4 | the handshake waits `sleep 0.05 \|\| sleep 1`, measured under an integer-only shim; X9's text states the consequence | §1.5, §5, §6, §9 |
| A3-1 | M-2 is a departure and residual R23; the false controls :600 claim corrected; "same order" replaced by the two figures | §1.5, §4, §8 |
| A3-2 | the order row (R29y) and the fallback row (R29aa) constructed; no unpinned item of this delta remains; d1's list re-evaluated | §1.2, §6 |
| A3-4 | R22's slot `#11-trip-wire-liveness-bound` named | §8 |
| A3-5 | ID×TB cites C12 | §2 |
| A5-3 | C5's convention stated (mean of three, one rounding): +2.2 s; DECISIONS' +2.3 explained | §4, §7, §9 |
| A5-4 | §1.4's probe cost labelled (host, load, one session); R19's Gap carries no figure | §1.4, §8 |
| A5-5 | the R29-D notes atop plan and x stay records; new dated notes supersede them | §5, §9 |
| A5-6 | the deaf move audited both ways | §1.5 |
| A5-7 | §T.K gains "Declared, not pinned" | §9 |
| A5-8 | R29q's sibling gap pinned now: the age row and R29z | §2, §4, §6 |
| A4-1 | POSIX by name, no quote marks, SIGCHLD's exception kept | §1.5 |
| A4-2 | EPERM carries macOS and Linux's case (wire :442; impl-d :220) | §1.2 |
| A4-3 | the latch stated as what it records (macOS: an unreaped zombie leader keeps its id) | §1.2 |
| A4-4 | I-1 quotes records :553's "bound" and :557, :562's "row" | §0 |
| A4-5 | impl-d :216's two fallback cases | §1.3 |
| A4-6 | grep counts include the comment hit (wire :448; draft wire :448) | §3, §7 |
| A4-7 | every line cite names its file; "draft" marks the draft's lines | all |
| A4-8 | "759 of 760" stated as the design memo's `wc -l` against its budget | §9 |
| A4-9 | killers from a re-run with corrected labels (`muts2.txt`), unit and ship columns | §6 |

## r3 changes

Plan-review R2 (`PLAN-REVIEW-R29-D2-R1.md`, "# Round 2": mechanism 0/0/6, docs 0/1/7), the driver's decisions for r3,
each finding → the sections r3 changed. r3's draft and logs: ev `mkr3.py`, `logs/r3/` (r2's draft kept in `r2/`).

| finding | change | §s |
|---|---|---|
| IMP-1 | R29c's ship killers deaf and norec ("may also kill"), survive arm deaf + norec removed (C4: the unit harness runs neither row); records :533 says "as R29j below"; R29i gains the tick row (M3) as a unit killer, so its arm also removes it; R29c and R29j are not killed by it (measured) | §4, §6, §7 (C4, C15), §9 |
| MIN-1 | R29d's killers latch, set, tick (unit) and `_wm` hang (ship); its survive arm removes all four; records :534; unit part measured (`RED: none` with latch, set, tick removed) | §6, §7 (C4) |
| MIN-2 | norec is "may also kill (R22's race, rate not pinned)" in §6's table, the records comments and C4; deaf stays the anchor | §4, §6 |
| M1 | the zomb bound stated for the wire unmutated (controls comment and §1.5); the mutant case (R29a on Linux: grace + 21 KILLs, ≈ 3.5 s / ≈ 23 s integer-only, estimated) declared with its trigger; C12 labelled | §1.5, §6, §7, §9 |
| M2 | `662a7065`'s ≈ 25 s cleanup KILL under R29c/i/j recorded for §T.K as a pre-existing defect, closed, with the pid-wrap qualifier; §1.5's "a possible reuse in every trial" replaced | §1.5, §9 |
| M3 | pinned: the tick row (fake clock, `SECONDS` unset) counts 40 ticks to the grace's 2 s and 2 where `sleep` refuses a fraction; R29ab (0.5 s) and R29ac (fallback dropped), both red, both shells; 662a7065 red (`20 2`); C17; `_MUT_RECORDS_MIN` 305 | §1.2, §2, §4, §6, §7, §9 |
| M4 | LV×TB moved to the changed pairs (C7, the tick row) | §2 |
| M5 | C14 gains shape D, the bare `elif ! ( … )` body (the deaf row's shape), measured: `inner`, both shells | §1.2, §7 |
| M6, MIN-4 | IFS grep over all nine files: 25 hits, every one a `read` prefix (22 `IFS= read`, 3 `IFS="$(printf '\t')" read`), the complement two comments | §1.2 |
| MIN-3 | §6's "Unpinned" lists this delta's own two declared items (the handshake's integer-only path, the M1 case), each with a trigger | §6, §9 |
| MIN-5 | population every `docs/plans/` file and every part: 302 hits in four classes; all 35 current-stating hits classified; design memo :632's R29d prose found false and added to §5's table | §5, §7 (C16), §9 |
| MIN-6 | commit 1 names its records comments (no record expression) | §9 |
| MIN-7 | §1.2's errexit sites add controls :528, the fallback row, ctlrun :371 and the deaf row's bare call; §1.5's "every zomb row" qualified (:563–:564's direct `_k2_live`); R23's `Recorded YYYY-MM-DD` form | §1.2, §1.5, §8 |
| (r3's own) | the set row flaked on the draft (3/300 b53, 1/300 b32: a real second boundary ends the grace a tick early); the set and shadow rows unset `SECONDS` first: 0/300; C18 | §4, §6, §7, §9 |

# K2 fixture git — PR #527 CR2 round: a call's process groups end with it (plan delta, R29)

**Status: accepted (plan-review TERMINAL 2026-10-23, `…-impl-cr2-d.md` §T.J), revision r3** (r1, r2 in the handoff;
applies its `cr2/PLAN-REVIEW-R29-R1.md`, R1 and R2, mapped below). Moved into `docs/plans/` as `…-cr2-plan-liveness.md` by R29-D's docs commit (§9), body as accepted,
the authority for §5's lines; the G36 notes it supersedes stay records and §T.I D6's "no separate plan-review" is
withdrawn (§5's last paragraph): a change to a ratified surface is a design change, reviewed as the mechanism, not the diff.

⚠ **Corrective notes** (dated; one line per falsified line, added by the slice that falsifies it, under G36's rule, as the notes atop `…-cr2-plan.md` and `…-cr2-plan-exitpath.md`: the body below is left as accepted, because it is provenance; the line numbers are the body's as accepted, before the first note, which shifts it by the notes' own lines; the authority for each new text is `…-cr2-plan-liveness-2.md` §5, and `…-impl-cr2-d.md` §T.K records the slice):
- **:60, :61 (§1.2 table)**, "the stop's batch probe … `<w>` 0 / `<w>` 1"; "K1's latch … if seen free before: reused" (2026-10-24, R29-E): the primitive's first read, for every caller; the latch read at every probe of every pending group.
- **:66 (the call's end)**, `_k2_kill_gone g 1` (2026-10-24, R29-E): `_k2_kill_gone 0 g:1 w:1`, and `_kg_left` → `_ctl_left`.
- **:105–:113, :117–:122 (K1)**, `_k2_kill_gone <pgid> <w> [<grace>]`; "one batch `kill -0`", "per group"; "the fallback … no batch probe"; "1 if still live at that bound, else 0" (2026-10-24, R29-E): `_k2_kill_gone <grace> <g>[:1]...`, one primitive over the set, every pending group read once a tick; one answer, `_kg_left` (it returns 0) (`…-cr2-plan-liveness-2.md` §1.2, §1.3).
- **:160 (ID×TB), :168 (TB×KX), :170 (TB×FS)**, "from one batch probe"; "per group, sequential; … 21 s"; "n × 21 × …" (2026-10-24, R29-E): `…-cr2-plan-liveness-2.md` §2's rows (the latches from the first read, read every tick; ≤ 21 KILL ticks for the set).
- **:207 (K1)**, "one batch probe starts the stop's latches" (2026-10-24, R29-E): `…-cr2-plan-liveness-2.md` D2.1–D2.3 (one call over the set; each latch live from its first read).
- **:237, :240, :242 (its §5 rows for x)**, "the batch probe, `_k2_kill_gone <g> <w> 2`"; "`_k2_kill_gone "$_ke_p" 1 || {…}`"; "`_k2_kill_gone <g> 1`" (2026-10-24, R29-E): `_k2_kill_gone 2 "$@"`; `_k2_kill_gone 0 "$2:1" "$3:1"; _ctl_left="$_kg_left"` (the R29-E notes in `…-cr2-plan-exitpath.md` say so for x).
- **:264–:267 (deaf)**, "`exec sleep 30`", readiness unstated (2026-10-24, R29-E): a readiness handshake (a file written once the ignore is set, waited for by `sleep 0.05 || sleep 1`), `exec sleep 10`, the row before zomb's build (`…-cr2-plan-liveness-2.md` §1.5).
- **:268–:270 (batch), :271–:275 (shadow rows)**, "the 2nd id fails `kill -0` until the 1st group's loop reads a table"; the `<w>` forms; the status `1` / `0` (2026-10-24, R29-E): the 2nd free at its first read; the `<grace> <g>[:1]` forms; the rows read `_kg_left` (`…-cr2-plan-liveness-2.md` §6).
- **:287, :290, :293, :296**, the killers "`<w>` 1 row", "latch row", "batch row"; "Each survive arm: its killing row removed" (2026-10-24, R29-E): `…-cr2-plan-liveness-2.md` §6's killer table (the last first corrected by `…-impl-cr2-d.md` §T.J).
- **:329 (C13), :338–:341 (R19)**, "latches start together"; R19's window (2026-10-24, R29-E): the latches start at the first read and are read every tick (`…-cr2-plan-liveness-2.md` §7 C13); R19 as that memo's §8 restates it (`…-residuals.md` R19).

**Names.** wire = `.claude/tools/webref-generic-core-trip-wire.sh`; controls, ctlrun, records, mutations = its parts
`…trip-wire.<part>.sh`; plan = `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan.md`; x = its companion
`…-cr2-plan-exitpath.md`; impl-d = `…-impl-cr2-d.md`; logs = the handoff's `cr2/logs/r29-delta/`. **Every tool line
number is at `c680a6ce`**; plan and x lines are their bodies as accepted. Provenance: `wc -l` gives wire 1375,
controls 881, ctlrun 439, records 549, mutations 623; `/usr/bin/grep -n` on each symbol named; mutations :275
`_MUT_RECORDS_MIN=284`. **The draft**: the mechanism written into a clone at `c680a6ce` to measure it
(`logs/r3-draft.diff`, `r3-unit.*`, `q8-draft-unit.log`, `r3-draft-anchors.log`): evidence for §5 and §6, not the slice's text.

---

## §0 What changed, and what this memo reviews

1. **Codex R29** (`54b63ffb`, impl-d §T.H): a group of zombies (orphans of a PID 1 that reaps none) read live
   forever. Now `_k2_live <pid|-pgid>` (wire :444–:446: `kill -0`, else the table, §1.3) decides every ending probe. The startup refusal needs the table (controls :226–:230). Residual R16 discharged.
2. **TERMINAL fix** (`c680a6ce`, impl-d §T.I D3): the stop's KILL (`$_K2_STOP`, wire :454–:457) is re-sent while
   `_k2_live` holds, ≤ 20 more, 0.05 s apart; `_k2_stop`'s fallback (:460) stays one KILL.

Both were applied, then recorded. This memo derives the mechanism for `$_og_lbl`'s invariant (controls :122, "a
call's process groups end with it and hold none of its caller's streams"), states where `c680a6ce` meets it (§4),
and plans the slice (§9).

---

## §1 The mechanism, derived

### §1.1 What must hold

| id | invariant | §2 ids |
|---|---|---|
| E | **ending**: after a call (and after the run), no group it launched runs code or holds a descriptor of its caller. Residue: R9, R20, R21 | LV, ST, KX, SW |
| I | **identity**: no signal reaches a group that is not the run's. Residue: plan BS-12 (1)–(4), R19 | ID |
| F | **fail-safe**: an unreadable liveness fact reads *alive*; an unreadable identity fact reads *skip and name* (never a wrong kill). Residue: a live group read gone (R20: by two reads; R21: a partial table) | FS, RF |
| B | **bound**: every wait and every re-KILL is bounded. Residue: R9 (`_ctl_run`'s and `_k2_stop`'s `wait`) | TB |
| N | **named**: a group the run cannot confirm ended is named on its stderr (fd 9) by its end, on every path the run can see (§1.4 K2). Residue: groups read gone that were not (R20, R21), a leader no KILL ends (R9, §8), an unrecorded group (R14), a run ending outside its traps (R15) | KX×SW, KX×FS, KX×S |

### §1.2 Two questions, two probes

E asks whether a member is **active**: a zombie runs nothing and holds no fd (POSIX XSH `_exit()`, by name, not
measured), so a group of zombies has ended. Only the table shows `Z`: `kill(2)` answers for
a zombie pid (macOS, impl-d :188); for a zombie-only group macOS answers EPERM, Linux succeeds (reasoned, design memo :634). I asks whether an id is **occupied**: a zombie holds its pid (measured,
impl-d :188) and, by POSIX XBD "Process ID Reuse" (by name; reasoned: reuse cannot be forced), no id is reused while
its process or group exists; identity reasoning (`_k2_age`, J6, K1's latch) reads occupancy, `kill -0 <pid>`.

**Rule.** Ending probes use `_k2_live`; identity probes `kill -0 <pid>`. `_k2_live` true implies `kill -0` true (its
fast path), so moving an ending probe to it only turns *alive* into *gone* for zombie-only groups and adds no signal
target. An identity probe must not move: a zombie leader read absent would skip `_k2_age`.

| consumer | site (`c680a6ce`; NEW) | question | probe | alive → | gone → |
|---|---|---|---|---|---|
| the stop's carrier | wire :454 (`declare -f`) | — | + `_k2_kill_gone` | | |
| the stop's TERM grace | :456 → K1's grace | ending | `_k2_live -g` | wait (`SECONDS` < 2) | done |
| the stop's KILL | :457 → K1 | ending | `_k2_live -g` | KILL (≤ 21) | done |
| the stop's batch probe | NEW, after TERM | identity | `kill -0 g` (unmarked) | `<w>` 0 | `<w>` 1 |
| K1's latch | NEW | identity | `kill -0 g` | if seen free before: reused, 0 | latch set |
| the reap's entry | :475 | ending | `_k2_live -g` | identity test | skip (R20, R21) |
| the reap's leader | :476 | identity | `kill -0 p` | `_k2_age` | leaderless: act, passed `<g>:1` |
| the reap, identity unknown | :477, :478 | identity | record, `_k2_age` | — | NEW: named, no signal |
| K2's post-stop read | NEW | ending, no identity guard | `_k2_live -g` | named (reused id: a false name, fail-safe) | — |
| the call's end | :496 → `_k2_kill_gone g 1` | ending + latch | K1 | KILL (≤ 21) | drop |
| `_wm_gone` | controls :356 | ending | `_k2_live p` | poll ≤ 20 × 0.05 s | pass |
| the `_wm` table read | controls :362 | ending | `_k2_live -g` | red | pass |
| `_og_gone` | controls :515 | ending | `_k2_live -g` | poll ≤ 40 × 0.05 s | pass |
| zomb | controls :556–:557 | ending | shadowed `kill`/`ps`/`awk` | per case | per case |
| the refusal | controls :230 | the run's row | `_k2_ptab` + awk | run | exit 2 |
| the watchdog | ctlrun :371 | runs `$_K2_STOP` | | | |

### §1.3 The table read: fail-safe direction, snapshot, requirement, portability, cost

**Direction.** Only "rows read, no live match" is gone; a failing `ps` or `awk`, an empty table, with or without
`pipefail`, read alive (impl-d §T.H I-1). **Snapshot (reasoned, Q10).** procps walks `/proc` (reasoned); macOS `ps`
may read one sysctl snapshot (not measured). A walk can miss a member forked during it that exits before it is
listed, reading a live group gone where gone means *forget* (the call's end drops, the reap skips). The premise, a
process existing throughout a read is listed by it, is reasoned; its trigger is route (b)'s first Linux run (C11, R20).
**L4: gone needs two consecutive reads gone**, the decision text in **one place** (one awk text in a two-pass loop in
`_k2_live`), so a record editing it (R29g/R29h's kind) reaches both reads; with two copies a `s///` reaches one and
the other answers for it. A zombie-only group is stable: its second read agrees. Residue: R20.

**Requirement.** Where `kill -0` is ambiguous ending needs the table, so a `ps` that cannot list the run's own row
(its pid, its pgid, not `Z`) refuses at the controls' start, exit 2, "decided nothing" (controls :230). That proves the
row readable, not the table complete: a `ps` omitting other groups' members reads them gone, a red where zomb's live
group (controls :557) is among the missing, else an unnamed leftover. Not R17's class ("refused, never a red"): R21.

**Portability, cost** (impl-d :188, :190: macOS measured, ≈ 10 ms a read, ship 27–28 s). procps first runs at landing
X9's route (b); busybox lists the row, is refused, or is R21's case. L4's second read only on a table-gone verdict.

### §1.4 KILL until gone: one primitive

Today: two bounded re-KILL loops and a third form: the stop's last line (wire :457: KILL, then while live ≤ 20 ×
0.05 s; exhaustion silent), the call's end (`_k2_end` :495–:498: while live and the leader id free, ≤ 20 × 0.05 s;
exhaustion → `_ctl_left`, entry kept), and `_k2_stop`'s fallback (:460, one KILL).

**Why it was N** (`feedback_one-issue-one-way.md`'s end): (a) *process boundary*: the stop runs in its own process
(no fd 9, none of the caller's variables), `_k2_end` in the caller; no reason for two texts, `declare -f` already
carries the probe (wire :454). (b) *predicate*: `_k2_end` runs after its leader was waited (ctlrun :375; :494), so an
answering pid is a reused id; the stop's leader may be alive: an identity state, a parameter. (c) *TERM*: outside the
loop (J13, x row C). (d) *exhaustion*: the caller's use. (e) *bound*: equal by copying (wire :450). **Decision: fold.**

**K1.** `_k2_kill_gone <pgid> <w> [<grace>]` (beside `_k2_live`, in `$_K2_STOP`'s `declare -f`): while
`_k2_live -<pgid>`: **identity latch**, if `kill -0 <pgid>` answers and `<w>` is 1 the id is reused: return 0, no
signal; if it fails, `<w>` := 1 (the id is free; none is reused while the group exists, so a later answer is a
reuse); then while `SECONDS` < `<grace>` wait 0.1 s (`sleep 0.1 || sleep 1`); else KILL the group, at most 21 times,
0.05 s apart (`sleep 0.05 || sleep 1`); 1 if still live at that bound, else 0. Callers: the stop's text after its TERM
line and **one batch `kill -0` over all groups** starting every latch at once (`<g>:1`, a group the reap judged
leaderless at wire :476, starts at 1), then `_k2_kill_gone <g> <w> 2` per group (grace from its own `SECONDS=0`);
`_k2_end`, `<w>` 1 (leader waited); the fallback, `<w>` from the mark, no batch probe (R19; impl-d :216's reasons
show the loop adds nothing there, not that a second form is needed).
**Probe first**: no KILL to an id just read gone; `c680a6ce`'s KILL-first kept R29c's pin (impl-d :216), which the
deaf row (§6) now gives directly.

**Why the grace joins the primitive (Q6 (i)).** The latch covers one group from its start (the batch probe or the mark;
`_k2_end`'s `<w>` 1) to its last KILL, grace included: a free id seen at any probe stays seen. Split grace and KILL
loops would hand that state across (a second site of one decision). It covers neither the time before its start nor a
free-and-reused gap between two probes. The grace still sends nothing and keeps its bound (plan CR11-C). Latch over a per-iteration age read
(Q6's hint): no `ps -o etime=` per iteration (no new unreadable fact), the free gap read at poll granularity
(0.05–0.1 s) not the record's 1–2 s (plan BS-12 (2)), and the watchdog's stop holds no record time. Residue: R19.

**K2, exhaustion and identity-unknown on each path.** The stop process reports nothing; whoever holds fd 9 checks
after it. `_k2_stop` (the run's end via `_k2_reap`; a failed record via `_k2_reap_add`) reads each group with
`_k2_live` after its `wait`s; each one not confirmed gone joins `_k2_left` and gets one fd-9 line, `!! could not
confirm that group N ended; it may still be running`; status 0 (the run's: G12's form, plan :638). No "process group ":
the norec row reads the id as `${_og_st##*process group }` (controls :533), the longest match, so a K2 line after the
record's line would replace the id and red for the wrong reason (C12). `_k2_reap_add`'s failure path removes only
entries not in `_k2_left` (Q2, `_k2_end`'s policy); the run's end retries the rest. `_k2_left` means one thing: only
`_k2_stop` writes it (`""` first: set under nounset, no `${_k2_left-}`), only `_k2_reap_add` reads it; the reap's
names do not enter it. The reap's identity-unknown skips
(unreadable record :477; `_k2_age` false :478, a transient `ps` failure or a reused leader) name the entry, never
signal it (F keeps the skip); for a reused leader the name is false, fail-safe. `_k2_end` puts the group in
`_ctl_left` and keeps its entry, so K2 names it at the run's end. The watchdog's stop is cut by `_k2_end` (:494) when
the child's `wait` returns; a leader no KILL ends keeps that `wait` open (R9's slot, §8).

**K3, what stays apart.** The TERM line (J13). The observer polls `_wm_gone`/`_og_gone` (an observer sharing the loop
would share its defect; they share the probe, which zomb pins). `_k2_end`'s watchdog KILL (:494), which lets
`wait "$3"` return and so makes `<w>` 1 true.

---

## §2 Coupled invariants

Invariants: **LV** liveness (activity, §1.2, L1–L4), **ID** identity (occupancy, H3, plan BS-12, K1's latch), **ST**
no stream held (x row F), **TB** time bound, **KX** KILL until gone and its exhaustion (K1–K2), **SW** the stop's text
is self-contained (`declare -f`, fixed when the wire loads, run by the watchdog at ctlrun :371), **FS** fail-safe (F),
**RF** the refusal (controls :230), **S** status (x row S). 9 invariants, 36 pairs.

| pair | intersection | mechanism | check |
|---|---|---|---|
| LV×ID | a zombie-only group's entry is dropped; may its id be reused? | a zombie holds its pgid; identity probes stay `kill -0` | zomb; R29e |
| LV×ST | can "gone" leave a stream held? | a zombie holds no fd; a missed member → L4 | `_og` kill row (< 3 s); R29o |
| LV×TB | cost per probe | ≤ 2 reads, only after `kill -0` succeeds | C7 |
| LV×KX | a false alive / a false gone in the loop | false alive: ≤ 21 KILLs, then named; false gone: L4 | shadow rows (§6) |
| LV×SW | the stop's process needs the probe and the primitive | `declare -f _k2_ptab _k2_live _k2_kill_gone` | R29c, R29j |
| LV×FS | which read is gone | awk `exit 3` only, two reads, one text | R29f–h, R29o |
| LV×RF | the refusal proves the run's row, not completeness | a partial table reads gone | R21 (§8) |
| ID×TB | the stop's window | latch over grace and KILL from one batch probe (K1) | R29m, R29p, R29s–t; residue R19 |
| ID×KX | KILLing a reused id | latch per iteration, probe first | R29m, R29p |
| ID×SW | the watchdog's stop runs while `_ctl_run` reaps the leader | the latch reads the freed id | R29p; R19 |
| ID×FS | does alive-by-unknown add a target? | `_k2_live` only after `kill -0` succeeds; identity unknown → skip, named | plan CR11-I; R29q |
| ST×TB | a group still live at the bound | holds its streams until the run's end retries it | `_og` kill row |
| ST×KX | exhaustion may leave a stream | named (K2), entry kept | R29n, R29r |
| ST×SW | the stop process holds nothing | `9>&-`, streams `/dev/null` (wire :459) | unpinned, measured (§6) |
| ST×FS | a false gone leaves a live holder | L4; residue | R20 |
| TB×KX | one bound | ≤ 21 KILLs 0.05 s apart per group, sequential; integer-only `sleep`: 21 s (X9) | 21-KILL row; C5 |
| TB×SW | one clock | `SECONDS=0` first in the stop's text (J14); the primitive counts KILLs | — |
| TB×FS | alive-by-unknown runs every loop to its bound | estimate: ≤ 2 s grace + n × 21 × (0.05 s + 2 reads ≈ 20 ms) ≈ 1.5 s per group, then n names | 21-KILL row |
| KX×SW | the stop cannot report | K2: its caller checks | R29n |
| KX×FS | a name under an unreadable table | the line says "could not confirm", not "running" | — |
| KX×S | the new line and the status | `return 0`; `_k2_reap_add`'s `exit 2` follows regardless | C9 |
| SW×FS | a stop text missing a function reads "gone" (status 127) | not the predicate's job: pinned by records | R29c, R29j |
| SW×S | the stop's own status | ends `exit 0`; read only by the fallback's `||` | — |
| FS×RF | unknown at the start vs mid-run | at the start nothing is decided, so refusing costs nothing; mid-run the read errs alive | plan CR11-J; zomb |
| RF×S | the refusal's status | exit 2 through the exit path | plan CR11-J |

Independent: LV×S, FS×S (a probe's status is read only by a conditional); ID×ST (identity selects whom to signal;
holding is the members'); ID×RF, TB×RF (both refusals, J1's and the table's, precede the first launch); ID×S, ST×S,
TB×S (status is set by `_k2_sig`/`_k2_exit` only); ST×RF, KX×RF, SW×RF (no group exists at the refusal).

---

## §3 Spec coverage map

**No spec surface** (the siblings' §0, plan :73–:79, x :3–:6, let `preflight.py` exit 1). Each row maps one of this
memo's sections, the form the preflight accepts; POSIX is cited by name, outside `.claude/tools/webref`'s sources.

| Spec section | Step | Branch | Touch | Full enum | User-input flow |
|---|---|---|---|---|---|
| none (this memo) §1.2 | ending vs identity probe | activity / occupancy | `_k2_live`, `kill -0` | ✓ for what `/usr/bin/grep -nE 'kill -0\|_k2_live'` lists in wire, controls, ctlrun at `c680a6ce`: 18 hits (10, 8, 0), 12 code: 11 consumer rows of §1.2's table, and `_k2_live`'s definition (:445); plus :477–:478 and controls :230, read | no |
| none (this memo) §1.3 | table read | gone / alive / unreadable / partial | `_k2_live`, `_k2_ptab` | ✓ | no |
| none (this memo) §1.4 | KILL until gone | gone / exhausted / reused id | `_k2_kill_gone` (NEW), `_k2_end`, `_k2_stop` | ✓ for the KILL sites `/usr/bin/grep -n 'kill -9' <wire>` lists: :457 (two), :460, :494, :498 | no |

---

## §4 Decisions, and where `c680a6ce` stands

| id | decision | at `c680a6ce` |
|---|---|---|
| L1 | a group (a pid) has ended iff it has no member (is no process) that is not a zombie | implemented (wire :444–:446) |
| L2 | ending probes `_k2_live`; identity probes `kill -0 <pid>` (§1.2 rule) | implemented (:476, :496 keep `kill -0`) |
| L3 | only awk's `exit 3` is gone | implemented (impl-d §T.H I-1) |
| L4 | gone from the table needs two consecutive reads gone, one decision text | **change** |
| L5 | the refusal stays the run's own row; completeness is R21's | implemented; R21 new (§8) |
| K1 | one primitive `_k2_kill_gone <pgid> <w> [<grace>]`: probe first, identity latch, grace, ≤ 21 KILLs; one batch probe starts the stop's latches | **change**: replaces wire :456–:457, `_k2_end`'s loop (:495–:498), the fallback (:460) |
| K2 | `_k2_stop` names each group not confirmed gone (`_k2_left`, fd 9), status unchanged; `_k2_reap_add` keeps their entries; the reap names identity-unknown entries | **change** (new) |
| K3 | the TERM line, the observer polls and `_k2_end`'s pre-wait KILL stay apart | implemented |
| P1 | the deaf row and the shadow rows (§6) | **change** |

---

## §5 Ratified surfaces this delta changes

Re-derived mechanically (P3): `/usr/bin/grep -nE 'kill -0|kill -9|KILL|0\.05|≤ 20|up to 3 s|cannot read the
run|ps -o etime=|identity check|_k2_stop|_k2_end|TERM, 1|1376|remov|empty' <plan> <x>` gives **146 lines (plan 74, x 72)**; each was read,
and the code-text lines around the hits (x :169–:199) too. Falsified by R29 (`54b63ffb`), `c680a6ce` or this delta:

| site (as accepted) | old text | new text |
|---|---|---|
| plan :90, CR#11 row; :460 | "(TERM, 1–2 s, KILL …)" | TERM, a 1–2 s grace, then KILL until gone (`_k2_kill_gone`) |
| plan :562 | "`_k2_reap_add` and `_k2_end` (12 lines)" | the slice's `wc` of the two (draft: 5 + 6) |
| plan :555, wire budget; :704, CR-B; x :291, stop line | **1376** = base + 90 (O-2) | **1389** (Q8): the draft is +11 (code +3, comments +8, `logs/r3-draft.diff`), +3 for wording; reason: §9's cohesion re-judgment keeps the code in the wire; G22's comments stay |
| plan :625, G11-2c | "TERM, then 1–2 s (whole `SECONDS`), then KILL" | TERM, then `_k2_kill_gone` (grace, then ≤ 21 KILLs while live, latch); pinned by R11w, R29d, deaf (R29i, R29j) |
| plan :635, G11-10 | "stopped only when `kill -0 -<pgid>` fails: `_k2_end` re-KILLs … (bounded)" | … when `_k2_live -<pgid>` fails (L1, L4); `_k2_kill_gone <g> 1`; pinned: zomb, R29a–h, R29k–p (the race unpinned, §6) |
| plan :636, G11-11 | "unknown → skip; a `ps -o etime=` that cannot read the run's own pid refuses" | unknown → skip, named (R29q); either `ps` read failing on the run's row refuses (controls :230) |
| plan :649, BS-12 | (4) "… is skipped and a signal leaves that group running"; refusal: `etime` only; (1)–(4) | (4) … skipped and named; refusal: both reads; adds (5) = R19 (§8) |
| plan :698, CR11-K | "dropped only when `kill -0 -<pgid>` fails (H2)" | … when `_k2_live -<pgid>` fails; C6 |
| plan :431 | `_ctl_left`: groups it "could not confirm empty" | … could not confirm ended (L1) |
| plan :637, G11-12; :700, CR11-A | "stops the call's groups, then removes their entries"; "restores `set +m`, stops, then removes (H17)" | … stops, names and keeps the groups not confirmed gone, removes the rest (R29r) |
| plan :701, CR11-J; x :240 | the J1 text, "…the reap's identity check needs \`ps -o etime=\`…" | controls :230's text (the reap needs both `ps` reads) |
| plan :819 | "`kill -0 -<pgid>` and `ps -o etime=` are read here on macOS only" | adds the table read, refused where it cannot list the run's row |
| x :25, row R | "stopped only when `kill -0 -<pgid>` fails" | … when `_k2_live -<pgid>` fails |
| x :26 row C; :87 C×R | "then KILL"; "TERM, 1–2 s, KILL" | then `_k2_kill_gone`; one not confirmed gone named by `_k2_stop` on fd 9 |
| x :55, R×L | "the reap's own messages … must not reach the run's output" | adds: K2's line, the reap's one deliberate message, on fd 9 |
| x :169–:176 | `_K2_STOP='SECONDS=0`; a `kill -0` grace; one KILL per group; fallback `kill -9` (:175); contract: stop, wait, 0 | `declare -f` prefix of three; TERM, the batch probe, `_k2_kill_gone <g> <w> 2`; fallback `<w>` from the mark; contract adds K2's read, `_k2_left`, the names |
| x :184, :186–:188 | `kill -0 -"$_kr_p"`; unknown → `continue`; groups passed bare | `_k2_live`; unknown → named, `continue`; leaderless → `<g>:1` |
| x :193; :236 (H17) | the add arm removes every recorded entry | removes those not in `_k2_left` |
| x :195, :197–:199 | `local … _ke_i …`; the `kill -0 -"$_ke_p"` loop, ≤ 20 | `_ke_i` gone; `_k2_kill_gone "$_ke_p" 1 \|\| { _ctl_left=…; continue; }` |
| x :230–:232 | "The poll is `kill -0 -<pgid>` … (up to 3 s) … does not linger as a zombie" | `_k2_live`; grace ≤ 3 s plus ≤ 21 KILLs (≈ 1 s; 21 s under integer `sleep`); an orphan of a non-reaping PID 1 stays a zombie, read gone (L1) |
| x :194; :237, `_k2_end` | "each KILLed until empty"; "every 0.05 s, ≤ 20 times, while it has members and no leader" | `_k2_kill_gone <g> 1`: while a non-zombie member lives and the id is not reused, ≤ 21 |
| x :357, R11a | the add arm's last line as its anchor | re-anchored (§6) |

**How the accepted files are corrected.** After acceptance the slice adds a dated corrective note at the top of the
plan and of x (G36's form, as `…-cr-plan.md` :16): one line per site above, pointing here; the bodies stay as
accepted. impl-d gets the slice's entry (§T.J); §T.H's (1)–(4) and §T.I's D3 correction gain no edit (closed history),
and §T.J records that this memo supersedes them and withdraws D6.

---

## §6 Pins

**Kept, KILLED as named today (both shells, impl-d :224):** R29a–h (records :531–:547; R29c, R29d `$_wm_lbl`, the
rest `$_og_lbl`), R11w (:506), S5 (:528), R11a (:512).

**Re-anchor impact (Q4)**, measured on the draft (`logs/r3-draft-anchors.log`: each wire record's `/usr/bin/sed` on head and draft):
**6 change** 1 → 0 and are re-anchored: :506 (R11w, the TERM line's mark),
:512 (R11a), :528 (S5), :535 (R29c), :536 (R29d), :544 (R29g, its `$`-anchored tail). R29a, b, e, f, h keep text and match, and reach
both reads through L4's one text. **R29c is relabelled `$_og_lbl`**: with probe-first a stop text without its
functions sends no KILL, the `_wm` hang child still writes `.term`, and deaf reds. C4 runs them all.

**New, in the `_og` check after zomb (needle `$_og_lbl`)**, every id one the row holds (Q5, invariant I):
- **deaf**: a `set -m` job `( trap '' TERM; exec sleep 30 )` (unwaited: id held until `_k2_stop` waits it), stopped
  by the real `_k2_stop` in the row's own shell (in a `$( … )` its `wait` returns at once: how r2's draft passed R29i/
  R29j), fd 9 to a file, in `{ … } 2>/dev/null` (the job notice): **elapsed < 5 s**, gone, no line. Under R29i/R29j
  `wait` lasts the sleep. Measured (both shells, `logs/r3-unit.*`): 2 s, no notice; R29i, R29j 30 s. C3, C4 use it.
- **batch** (n = 2: `_og_z`, `_og_l`): the stop's text in a subshell, `kill`, `ps`, `sleep` shadowed; the 2nd id
  fails `kill -0` until the 1st group's loop reads a table, then answers → 0 KILLs to it (measured; batch probe
  dropped: 21). **mark**: `_k2_reap` over a leaderless entry, `_k2_stop` shadowed → `<g>:1`.
- **shadow rows** on zomb's zombie group `_og_z`, held by its non-reaping parent until zomb's cleanup (controls
  :558), `kill`, `sleep`, `_k2_live` shadowed as zomb shadows `kill` (no real signal, no cost): always live →
  `_k2_kill_gone g 0` is 1 after 21 KILLs; live for three probes → 0 after 3; `<w>` 1, `kill -0 g` answering → 0
  KILLs; **latch**: `<w>` 0, grace 2, `kill -0 g` failing at probe 2, answering at 3 → 0 KILLs, 0; **two reads**:
  `ps` gone then live → alive; **name**: the real
  `_k2_stop` on `_og_z` (its TERM reaches only that zombie group), the caller's `_k2_live` live → one line, 0;
  **reap**: `_k2_reap` over an empty entry named by zomb's live group `_og_l` (zombie leader holds the id) → one
  line, no stop; **keep**: `_k2_reap_add`, record failing, `_k2_stop` shadowed to leave one group in `_k2_left` →
  that entry stays, the other goes, status 2.

| record | mutation | killed by |
|---|---|---|
| R29i | the stop's `_k2_kill_gone` call made `:` | deaf |
| R29j | `_k2_kill_gone` dropped from `$_K2_STOP`'s `declare -f` | deaf |
| R29k | the primitive's KILL bound cut to one | 21-KILL row |
| R29l | the primitive's exhaustion status made 0 | 21-KILL row |
| R29m | the latch's reuse return dropped | `<w>` 1 row |
| R29n | `_k2_stop`'s name dropped | name row |
| R29o | `_k2_live`'s second read dropped | two-read row |
| R29p | the latch's set (`<w>` 1 on a free id) dropped | latch row |
| R29q | the reap's identity-unknown name dropped | reap row |
| R29r | `_k2_reap_add`'s keep dropped (every entry removed) | keep row |
| R29s | the stop's batch probe dropped (latches start per group) | batch row |
| R29t | the reap's `:1` mark dropped | mark row |

Each survive arm: its killing row removed. `_MUT_RECORDS_MIN` 284 → 296; the ratchet's 18 unchanged (all twelve on an
existing label).

**Unpinned, each with its trigger** (owner of all: §8's):
- The race the re-KILL closes: a separating construction forks near the per-uid limit (impl-d §T.I D3); the
  shadow rows pin the logic. Trigger: a K2 line, or an "outlived"/"left running" red.
- Linux `kill(-pgid)` atomicity against fork: a hypothesis (no Linux here). Trigger: route (b)'s first run (C11).
- The snapshot race L4 closes: not constructible (the two-read row pins the logic). Trigger: R20's.
- Exhaustion on a real unkillable member: not constructible. Trigger: a K2 line; a stuck leader is R9's slot.
- ST×SW, wire :459's `< /dev/null > /dev/null 2>&1 9>&-`: **tried** (Q7): all four removed, the ship PASSED on both
  shells (`logs/q7-streams*`), and no record anchors :459 (`/usr/bin/grep -nE 'k2stop|env -u|_k2_stop|_ks_p'`: 3 hits). Reason: the stop is outside
  `$_og_lbl`'s population (a foreground helper, not a group the call launched) and its hold is
  bounded (TB). A sig-row-derived pin (≈ 2 s) is available, not adopted: its cost buys a bounded delay outside the
  population. Trigger: an edit to :459, or a reader held after a run killed in its exit path.

---

## §7 Claim rows

| id | claim | measured by | expected |
|---|---|---|---|
| C1 | the ending/identity split | `/usr/bin/grep -nE 'kill -0' <wire and parts>`, each hit read | at `c680a6ce`: wire :442 (comment), :445, :476, :496; controls :548 (comment); records :532 (R29b). After: the reap's leader, the stop's batch probe, K1's latch, `_k2_live`'s fast path, comments, records :532, R29m/R29p |
| C2 | one KILL-until-gone text | `/usr/bin/grep -n 'kill -9' <wire>` | `_k2_kill_gone`'s one KILL and `_k2_end`'s pre-wait KILL; at `c680a6ce`: :457 (two), :460, :494, :498 |
| C3 | the stop's KILL is pinned | `nokill.diff`'s equivalent on the slice's stop call, ship, both shells | red under `$_og_lbl` (deaf's elapsed bound); at `c680a6ce` PASSED both shells, no record anchors the KILL (`logs/nokill*`) |
| C4 | records | one-record trials (`r3-optin.sh … default`), both shells | R29a–t KILLED as named (R29c under `$_og_lbl`), each survive arm SURVIVES; R11a, S5, R11w and the kept records KILLED as named |
| C5 | cost, and no false name | ship wall time alone, before/after, both shells; `grep -c 'could not confirm'` | ≤ +2 s (the deaf row's grace); 0 names |
| C6 | CR11-K's race probe | `cr2/bin/cr2c2-r4race.sh` (plan :698: the call's end from the head, not `end4`), fixed by the slice: it evals `_K2_STOP` before the stop's functions exist (empty `declare -f`, no re-KILL); they are extracted first, EXTRACT checks them (`logs/r3-r4race-extract.*`); N ≥ 600, 2 × ncpu load | 0 survivors, both shells |
| C7 | table reads | `ps` wrapped to count calls, one ship run, before/after | after ≤ before + the table-gone verdicts + K2's reads; stated |
| C8 | counts | `_mutants \| wc -l`; anchor gate (`cr2b-ftest.sh`) | 296 both shells; 0 not fresh |
| C9 | status unchanged | `cr2/bin/cr2c2-f2probe3.sh` (impl-d :69 (3)) on a copy of the slice's wire bracketed by `# R3PROTO-EXIT-BEGIN`/`-END` from wire :438 on, the new functions inside (`blk` reads only that) | all fifteen cases keep their status, both shells |
| C10 | budgets | `wc -l` | wire ≤ 1389 (§5), controls ≤ 1000, records ≤ 1000 |
| C11 | procps | landing X9's route (b) | no refusal; zomb, deaf and the shadow rows green |
| C12 | norec reads the right id when K2 fires | the norec parse over a record line followed by a K2 line (draft: `logs/q8-draft-unit.log`); norec with the group left live | the record's id; the red is "left running", not "no line named" |
| C13 | latches start together | the batch and mark rows (§6), both shells | 0 KILLs; `<g>:1`; R29s, R29t red |
| C14 | §5 covers the budget | `/usr/bin/grep -n 1376` over plan and x | 3 hits (plan :555, :704; x :291), each in §5's budget row |

---

## §8 Residuals touched

Owner of each new row: the citation-hygiene program, k2 trip-wire lane. Origin per `…-residuals.md` :96–:97.
- **R16**: discharged by `54b63ffb` (impl-d §T.H); unchanged here.
- **R19 (new; plan BS-12 (5), Q14).** **Gap:** a latch starts at the stop's batch probe (or the reap's mark):
  an id freed and reused between the reap's judgment of its leader (or a failed record's or the watchdog's launch)
  and that probe, or between two probes (0.05–0.1 s; 1 s under integer `sleep`), is signalled; in the fallback (no
  batch probe) also during earlier groups' loops. **Consequence:** a wrong kill. **Why:** a closer guard needs a handle that outlives the id
  (Linux pidfd), which the shell lacks; a reuse inside one poll needs a pid wrap inside it. **Trigger:** a wrong kill
  reported near a stop, or a shell gaining a process handle; re-evaluated by 2026-11-01 or at route (b)'s first run
  (C11), whichever first. **Origin:**
  own: `f4fb2466` (CR2-C2) made the stop with no identity guard (impl-d :216); `c680a6ce`
  widened its window by ≤ 20 more KILLs, KILL first; this delta narrows it to the above.
- **R20 (new; P1 (b)).** **Gap:** a live group read gone by two consecutive reads (a member forking and exiting
  inside each, or a failed snapshot premise, §1.3) is forgotten: `_k2_end` drops it, the reap skips it, unnamed.
  **Consequence:** an unnamed leftover, maybe holding the caller's streams; a red where a check sees it. **Why:** no
  finite number of snapshot reads proves absence against such a member; two bound it to a fork chain at scan
  cadence. **Trigger:** a leftover of a run, an "outlived"/"left running" red, or route (b)'s first run (C11).
  **Origin:** own (`54b63ffb`).
- **R21 (new; P1 (c)).** **Gap:** a `ps` listing the run's row but omitting other groups' members passes the refusal
  and reads them gone. **Consequence:** a red where zomb's live group is among the missing (controls :557), else an
  unnamed leftover. **Why:** a table's completeness cannot be proved from inside it; zomb's group is the run's one
  probe of it. **Trigger:** zomb's "a live group … read gone" red on any runner. **Origin:** own (`54b63ffb`: before
  it, no table).
- **R9's slot `#11-trip-wire-liveness-bound`, extended (P2).** Measured (`git show e8f78896:<harness>`): base's
  `_control` (:117) waited the child (:167) after the watchdog's KILL (:166), so a leader no KILL ends blocked it at
  base too (`_ctl_run` came with `fd85e29f`). Own: `_k2_stop`'s `wait "$_ks_p"` (reap and `_k2_reap_add` paths), which
  K2's read follows, so that group is never named. Extend, not a new row: the slot's property (no verdict, the run waits), trigger and owner;
  a second row would split one decision surface. The Gap (landing §9.2 :211–:215, "bounds
  the children that run through `_ctl_run`") gains "while a KILL can end the leader" in its lead (R9 :121 too) and, under "Pre-existing at `e8f78896`", "a child
  whose leader no KILL ends (uninterruptible sleep): `_ctl_run`'s `wait` (ctlrun :375) does not return", with
  `_k2_stop`'s `wait` as own; R9 names both. Registered only at ship (landing :190), so the text changes first.

---

## §9 The slice

One slice, **R29-D**, after acceptance, by plan §11's procedure (:711–:720: fresh clone; claim and arm tables;
battery b53/b32 with `BASE_REV`; a fresh reviewer on a frozen snapshot; re-check on an IMPORTANT; diff read;
transplant; ship ×4 {BSD, GNU-like} × {b53, b32}; `scripts/trip-wires.sh`). Code by a coder, docs by a designer.

**Commit order (Q9)**, each commit's pins run with it, so one failure hides no other:
(1) L4 (`_k2_live`'s loop, the two-read row, R29o, R29g re-anchored); (2) K1 (`_k2_kill_gone`, the stop's call and batch
probe, the reap's mark, the fallback, `_k2_end`, the deaf, batch, mark and shadow rows, R29i–m, R29p, R29s, R29t, the
re-anchors of R11w, S5, R29c, R29d; C6's script fix in `cr2/bin`); (3) K2 with P1 (a) and Q2 (`_k2_left`, the names,
the keep, R29n, R29q, R29r, R11a re-anchored); (4) docs. The budget is §5's (Q8): no shared STOP.

**Cohesion re-judgment (P4; CLAUDE.md's touch-time split; wire > 1000 lines).** `_k2_kill_gone` is
carried by `$_K2_STOP`'s `declare -f`, built as the wire loads, before its trap and any part, so it stays beside the
stop (x :285); the names are written on the trap's own path (`_k2_stop`, `_k2_reap`). Against `a3260410`'s section
map: no new seam; not split. This is §5's reason for the new wire stop line.

**Docs**: design memo rows WM/OG (:632, :634) and its CR2 file list (:22, in place: 759 of 760 stays)
naming `…-cr2-plan-liveness.md`; the umbrella rows :23, :111 ("All twenty-one files" → twenty-two, as CR2-D, impl-d
:49); the sweep: its glob (`…-cr2-sweep.md` :64) takes the memo and its `cr2-*` case (:72) drops it with its
siblings, so the exclusion comment (:68–:69) names it, in place; `idents.tsv` gains `_k2_kill_gone`, `_k2_left` and
the K2 line (plan :732–:734); corpus §6.1 rows R29i–t and totals; residuals R19–R21, R9's text and the range
statements (residuals :7, :94, :96; design memo :32, :57, :652); landing §9.2 (§8), X9 (route (b) runs C11), **X10** (:400:
the slice's wire commit and `wc -l`, each edited part below 1000, this touch's cohesion re-judgment appended); §5's
corrective notes; impl-d §T.J. The sweep (§E) and the cite check of moved lines run in the slice (impl-d §T.I D1).

---

## r3 changes

R2i-1 → §1.2, §1.4 K1, §2 ID×TB, §4, §5 x :169/:184, §6 batch, mark, R29s/t, §7 C13, §8 R19, §9 (2). R2i-2 → §6 deaf,
§7 C3. R2ii-1 → §7 C6, §9 (2). R2i-M1 → §1.1. R2i-M2 → §8 R9. R2i-M3 → §6 ST×SW. R2i-M4, nounset → §1.4 K2. R2i-M5 →
none. R2ii-M1 → §5, §7 C14. R2ii-M2 → §9 (3). R2ii-M3, -M4 → §8 R9. R2ii-M5 → §9. R2ii-M6 → §3. R2ii-M7 → §7 C9. R19
date → §8. The draft (wire 1386) → §5 budget, §6 re-anchor, §7 C8, C10.

## r2 changes

P1 → §1.1 (F, N, the §2-ids column), §1.4 K2 (reap names; Q2 keep), §8 R20, R21, §1.3. P2 → §8 (R9's slot, origin
measured), §9 docs. P3 → §5 (grep, 89 hits, rows added). P4 → §9 (procedure, X10, cohesion). P5 → §7 C6, C9, §2 KX×S.
Q1 → §1.4 K2 wording, §7 C12. Q2 → §1.4 K2, §5 :700/:193/:236, §6 keep row, R29r. Q3 → §1.2 table, §3. Q4 → §1.3
L4, §6 re-anchor impact, §7 C4. Q5 → §6 shadow rows. Q6 → §1.4 K1 latch and grace, §2 ID rows, §8 R19 origin.
Q7 → §6 unpinned list (:459 tried). Q8 → §5 :555 row, §7 C10, §9. Q9 → §9 commit order. Q10 → §1.3 snapshot, §8
R20. Q11 → §1.2 POSIX citations. Q12 → §0 :226–:230, §7 C1, §6 :459 provenance, §2 TB×FS estimate. Q13 → §9 docs.
Q14 → §8 R19.

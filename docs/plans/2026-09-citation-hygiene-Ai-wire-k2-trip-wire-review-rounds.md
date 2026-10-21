# Slice A-i-wire — the K2 trip-wire: external review and the design revision (§10–§11)

This file holds §10 (the external review (Codex) of #519, rounds 1–9) and §11 (the design revision
after the enumeration attestation, its review and what the implementation decided) of
`docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md` ("the #519 memo"). Both were moved here
unchanged as a touch-time split by PR #527's CR2 round (`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan.md`
§7, O-1 (A)); the #519 memo keeps a pointer at §10's place, and its banner lists the statements here that are
no longer true. §10 and §11 keep their numbers, so a bare §10.x or §11.x resolves here; every other section
reference without a file name is to `2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`.

---

## §0 Spec coverage map

**No spec surface** — this file holds the review rounds of a shell trip-wire slice. It has the same by-design
shape as the #519 memo (the umbrella's A-i-wire row): `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §10 External review (Codex) — round 1

Ten unresolved threads were already on #519 from a review of the carve commit, and this slice's
own pre-push gate had not seen them. Eight were real at the head they were written against;
**two fail-open classes and two false-positive classes survived to this head** and are fixed
here. The two the loop did not change are answered, not waved away.

| # | What | Disposition |
|---|---|---|
| **P1** | An inherited exported `WEBREF_WIRE_SELFTEST` redirects `ROOT` at an arbitrary fixture **and skips every control** — reproduced by the reviewer: exit 0 over a real tree holding a violation | **Fixed.** Self-test mode now requires a companion token the controls set; a leftover export makes the gate **refuse loudly** instead of answering about the wrong tree. Same class as the `WEBREF_WIRE_MUTANTS` bypass §9 records, on a variable that predates it. Control + record. |
| **P1** | The HEAD probe treated **any** non-zero `rev-parse --verify --quiet` as "unborn", so an operational failure silently disabled the whole HEAD pass | **Fixed.** Measured: unborn is **1**, "git cannot answer" is **128**. `>1` is now an `err` record. Control (a `git` shim failing only `--verify`) + record. |
| **P1** | *"Restrict K2 to the generic layer"* — `DESIGN.md` §Architecture assigns path heuristics to the **adapter**, and `commands/agent_brief.py` is inside K2's scope | **Answered in §2, and the memo's earlier answer withdrawn.** The two definitions of "generic core" genuinely differ; K2 is the wider one by deliberate choice. Raised as a cross-slice question with both resolutions named — not settled here. |
| **P2** | The staged-symlink blob read: the trailing-newline sentinel preserved the value but **not the status**, so a failed `cat` became a successfully-read empty target | **Fixed.** `R%d` carries both. Control (a failing `cat` shim) + record. |
| **P2** | `_match_path`'s pipeline: under `pipefail` a pre-processing failure is reported as `grep`'s 1, i.e. as "no match" | **Fixed.** No pipeline — the value is computed first. ⚠ §9's `_esc`/`_onerec` change had already removed the `tr` this could fail in, which narrowed the class without closing it. |
| **P2** | Closing punctuation counted as a path segment: `See (.claude/tools/foo/) for details` reds the gate on a **one-segment** reference outside K2 | **Fixed.** `)]}>,;` joined the running-text terminators. ⚠ The first attempt put `]` in the middle of the bracket expression, which **closes it** — the predicate silently matched nothing and the `routed` control caught it. Green-direction control + record. |
| **P2** | No leading boundary before `.claude`, so `https://example.claude/skills/team/rule.md` and an entry named `…/example.claude/skills/…` matched on a suffix | **Fixed** in both predicates. Green-direction control + record. |
| **P2** | The `cachedir` control could pass from the **index arm alone**, so a regression in the worktree arm stayed green | **Fixed.** The staged blob is clean and the violation is worktree-only — verified by disabling the worktree content arm, which the control now catches and previously did not. |
| **P1** | *"Complete the mandatory plan review before landing the wire"* | **Already true.** Written against the carve commit, before this memo existed; the memo is §4's first artifact and has had a plan-review round plus a five-axis design gate. |
| **P2** | *"Track or close the host-path classes left outside the gate"* | **Answered in §8**, which the reviewer's head predates: no slots, with the argument (a slot records work owed; a blind spot records the reach of a predicate) and the reason a re-evaluation trigger could never fire for two of the classes. §5 item 1 also adds the **fifth** class the reviewer's list did not have. |

⚠ **Two of these are the same shape as defects this slice found in itself** — a required gate
exiting 0 over something it never read, because an environment variable put it in a mode it
was never asked for. Three instances now (`WEBREF_WIRE_MUTANTS`, `WEBREF_WIRE_SELFTEST`, and
the HEAD probe's collapsed status). The general lesson is recorded where the modes are entered
rather than in a fourth ⚠ here: **a mode this wire can be put into from outside must be
unreachable or loud, never silent.**

### §10.1 Round 2 — five findings, two of them made by round 1's fixes

| # | What | Disposition |
|---|---|---|
| **P2** | An inventoried worktree path that **vanishes** before it is read matches none of `_entry`'s arms — no `ok`, no `err`, **no record at all** — and the final guard only requires the *aggregate* `SCANNED` to be non-zero, so its siblings carry the run to green. Reproduced: a forbidden untracked file removed just after `ls-files` gave exit 0 | **Fixed.** An absent tree entry that git does **not** report as tracked is now an `err`. ⚠ The membership question is asked of git rather than inferred from absence, because a *tracked* path deleted from the worktree is the legitimate case — the index pass already answered for it. Control: a `git` shim whose `--others` inventory names a path that is not there (the deterministic form of the race). |
| **P2** | `_match_path` **still** ignored `_onerec`'s status after §10's fix: it is called beneath `\|\|` in `_stored`, where `errexit` is suspended, so a failing assignment simply fell through to `grep`, which returned 1 for the empty value — the same misclassification by another route | **Fixed** with `\|\| return 4`. ⚠ **Removing the pipeline was not enough, and round 1 stopped there** — a fix aimed at one channel while the value travelled another. ⚠ **No control pins this**, and the wire says so: `_onerec` is parameter expansion now, so nothing external is left for a shim to break and the only way in is to edit the function. Kept for the shape one refactor away, on the same footing as the `-a` on `_verdict`'s arms. |
| **P2** | Round 1's punctuation terminators bought the false positive back as a **false negative**: excluded from the segment *entirely*, a real path `.claude/tools/team,inc/rule.md` stopped at the comma and read K2 zero | **Fixed.** A segment is now *"any run of path characters that does not END in punctuation"*. ⚠ The green control pins one direction and a red control the other; **neither alone could have caught this**, which is the argument for having both. |
| **P2** | The watchdog's `kill -9 "$_cpid"` reaches only the wire's own bash — a command substitution or `grep` **blocked beneath it**, precisely the FIFO hang the watchdog exists to catch, is reparented and stays blocked, so repeated local runs accumulate permanent orphans | **Fixed.** The child is started under `set -m` so it is its own process group, and the group is killed. Measured on bash 5.3 and 3.2: group kill reaps the descendant, top-PID kill does not. |
| **P2** | The green boundary fixture exercised only the **entry name**, so it pinned `$K2RE_PATH` and not the independently-changed arm in `$K2RE`; removing the boundary from the running-text predicate reintroduced the URL false positive with every control green | **Fixed.** A second fixture carries the URL as file *content*, and the mutation record that targeted only the stored-path regex now has a sibling for the running-text one. |

⚠ **Two of the five were introduced by round 1**, and both are the same mistake: a fix aimed at
the site the finding named rather than at the property. The punctuation repair traded one
direction of wrongness for the other; the pipeline repair removed the channel the status was
being lost through and left the one it was actually travelling. The mutation set caught neither
— they are not reachable by mutating a guard, they *are* the guard being wrong — which is the
clearest statement available of what that instrument does and does not buy.

### §10.2 Round 3 — the root, and why three rounds found it one site at a time

**All four findings were of one shape**: *"fresh evidence after the claimed fix"*. That is the
loop's ≥2-round recurrence trigger, so the root-check ran before any of them was patched.

**Q1 — is a canonical algorithm missing, or are these N ad-hoc paths each carrying a subset
bug?** The latter, and the algorithm has a name now:

> **A non-zero status is not a specific negative.** A command's failure may be read as a
> *particular* negative (*"there is no HEAD"*, *"this path is untracked"*) only when a
> separate, **positive** test establishes that negative. Otherwise it is an ERROR.

The reason it kept biting is that **the wrong reading is always the convenient one** — it turns
*"I could not find out"* into *"there is nothing to find"*, which is the direction that makes a
gate green.

**And the audit is written out**, so *"is there another site?"* has an answer rather than a
guess. Every status this file reads is enumerated in the wire's header with what it concludes:
**two sites inferred a specific negative** (the HEAD probe, the tracked-membership question) —
both are the R3 findings, both fixed — and every other site concludes only *error*, from a
documented contract (`grep`'s 1-vs-≥2) or from nothing at all. A new `git` call joins that list
or it is a defect.

**Q2 — own-ideal test.** The wire's own header says *"ONE CHECK, ABSOLUTE … this wire makes no
un-asserted report"* and *"One authority answers all of it, and it is git."* The walk is not the
anti-pattern; **the scattered interpretation of git's answers was** — the opposite of one
authority. That is why the disposition is the named invariant plus an audit (option A: step
back and collapse) rather than a fourth site-patch.

| # | What | Disposition |
|---|---|---|
| **P2** | `rev-parse --verify --quiet HEAD` exits **1 for a malformed branch ref too**, so R2's "only >1 is an error" still skipped the HEAD inventory and exited 0 while HEAD lookup had failed | **Fixed.** Unborn is established *positively* — `rev-list -n 1 --all` exiting 0 with empty output. Measured: truly unborn = rc 0/empty, malformed ref = **rc 128**. |
| **P2** | The membership check read `$rel` as a **pathspec**: a vanished untracked `foo[1].py` matched a tracked `foo1.py` and was reported tracked, so the entry nothing had answered for was passed over — the hole that arm was added to close | **Fixed** with `--literal-pathspecs`. Measured: rc 0 without it, rc 1 with it. ⚠ Same lesson as `${var#"$prefix"/}` one layer up: **a path is data, not a pattern** — third instance in this file. |
| **P2** | The self-test companion token is a **fixed literal in this file**, so exporting both variables — exactly what copying the two lines out of the controls produces — still redirected a normal run | **Fixed.** The companion value is the parent's **live PID**; the child's own `$PPID` must equal it, which copying cannot satisfy. Verified on bash 5.3 and 3.2, and the reviewer's exact reproduction now refuses. |
| **P2** | R2's punctuation rule over-reached onto **both** segments, so `.claude/tools/team,/rule.md` — whose comma is followed by `/` and therefore cannot be prose — went undetected | **Fixed.** The restriction belongs only on the **final** segment, because that is the only place the end of the reference is ambiguous; an intermediate segment is delimited by `/`, which settles it. |

⚠ **The punctuation predicate was wrong three times in three different directions** — too loose
(prose reddened the gate), too tight (a comma inside a segment), then too tight again (a comma
before a slash). Each repair was aimed at the example in the finding. What finally settled it
was stating the *property* — *"ambiguity exists only where the reference ends"* — and it now has
a control on each side, because **neither direction alone could have caught the other**.

⚠ **And one R2 control had to be rewritten, not just kept.** `headprobe` encoded R2's rule ("a
failed probe is an error") on a fixture that was genuinely unborn — under the corrected rule
that fixture's green is *right*, so the control could no longer distinguish anything. It has a
commit now. **A control written against a rule outlives the rule**, and a passing control is not
evidence that it still asks the question it was named for.

### §10.3 Round 4 — PAUSE, and the claim that was driving the loop

`…trip-wire.sh` has now carried a finding in **four consecutive rounds**, which is this loop's
scope-creep PAUSE. The root-check ran again, and this time it lands somewhere different from
§10.2's.

**Q1 — abstraction coverage.** §10.2's invariant (*a non-zero status is not a specific
negative*) held up: R4's HEAD finding is a violation of it that my own R3 fix introduced — I
established the negative positively, as the invariant demands, **but for the wrong subject**,
asking whether the *repository* had commits rather than whether *this HEAD* did. The invariant
is right; its first application was not. No further abstraction is missing there.

**Q2 — own-ideal test, and this is the one that moved.** The wire's header said
*"ONE CHECK, ABSOLUTE. It is closed and decidable; it is not a heuristic."* Measured against
four rounds of evidence, **that is false of half of it**:

| | subject | status |
|---|---|---|
| `$K2RE_PATH` | a **stored path** — git hands the value over whole, `/` is the only delimiter | genuinely **closed and decidable**. One finding, in R1, stable since |
| `$K2RE` | **running text** — arbitrary bytes in an unknown language | a **bounded heuristic**. Every boundary finding in R1, R2, R3 and R4 was here |

*"Does a path reference start and end here?"* cannot be decided without knowing whether the
bytes are prose, code, Markdown, a URL or a `.pyc`. **The claim was the defect, not the
regex** — it made each counter-example read as "a bug to repair", so each repair was aimed at
the example and the next round found the opposite direction:

> too loose (prose in parentheses reddened the gate) → too tight (a comma inside a segment) →
> too tight again (a comma before a slash) → wrong in kind (`@` treated as a boundary, because
> the rule was written as *"not these few path characters"* instead of *"one of these prose
> delimiters"*).

**Disposition: option A — step back and collapse**, not a fifth boundary patch. The header now
states the two halves' different status, and with it the requirement that follows: **every
boundary rule carries a control in both directions**, because each of those four defects was
invisible to a control that tested only the other way. What the wire *does* is unchanged; what
it *claims* is now true.

| # | What | Disposition |
|---|---|---|
| **P2** | `rev-list -n 1 --all` asks whether the **repository** has commits; on an orphan branch HEAD is legitimately unborn while another branch has one, so a clean fixture reported a read error | **Fixed.** `symbolic-ref -q HEAD` + `show-ref --verify` on the named ref. Measured, all three: orphan = symbolic-ref 0 / ref absent; empty repo = same; **malformed ref = symbolic-ref 128**. Green-direction control (an orphan-branch fixture). |
| **P2** | The leading boundary admitted **path characters**: `foo@.claude/skills/team/rule.md` matched on the suffix of a component | **Fixed.** The list is positive — whitespace, a control character, a quote, a backtick, an opening bracket, or `/`. ⚠ `[:cntrl:]` is load-bearing and the **binary control caught its absence in the same run that introduced it**: that fixture wraps the path in NULs. |
| **P2** | `-f` **followed an ancestor symlink**: a tracked `dir/a.py` with `dir` replaced by a link to an external directory made the gate red over bytes `git add -A` would never stage | **Fixed.** A proper ancestor being a symlink is an `err` — not a skip, because that is how entries get passed over in silence. |
| **P2** | `grep -c .` on an **empty** `.bare` prints 0 and **exits 1**, so `set -e` aborted the gate — and it fires exactly when the ratchet reaches the state it exists to permit | **Fixed** with `wc -l`. Measured: `grep -c .` rc 1 / `wc -l` rc 0 on an empty file. ⚠ No control: reaching an empty `.bare` needs every control to have a record, which is not today's state. Recorded rather than implied. |

⚠ **And two of my own fixtures were wrong in ways a passing control hid.** The `headprobe`
shim matched `--verify` *anywhere*, so it also broke the `show-ref --verify` that now decides
unbornness — the shim had silently become a control for a different arm. Narrowing it to
`rev-parse --verify` then exposed a second defect: `*" rev-parse "*" --verify "*` **can never
match**, because the first half consumes the space the second needs, so the shim matched
nothing and the control exercised an unshimmed git. **A shim shims the one call it names, and
"one call" means the verb and the flag, as one adjacent sequence.**

### §10.4 Round 5 + the design re-gate — the round that judged round 4 wrong

R4 declared *"option A — step back and collapse, not a fifth boundary patch."* Codex's R5 and
the mandated cumulative design re-gate (two axes, run because PAUSE fired and a divergent loop
never reaches TERMINAL) agree it was **both**, and that the patch half was the worst change in
this PR.

**The CRIT: R4's leading-boundary rewrite made a required gate FAIL OPEN.** Replacing the
exclusion class with a positive delimiter list closed one contrived false positive
(`foo@.claude/…`) and opened at least five false negatives. Measured, HEAD-at-R4 vs the
revision before it:

| input | R3 | R4 |
|---|---|---|
| `DEFAULT=.claude/skills/a/b.md` | HIT | **MISS** |
| `--paths=.claude/skills/a/b.md` | HIT | **MISS** |
| `k:.claude/skills/a/b.md` | HIT | **MISS** |
| `` `.claude/skills/a/b.md` `` | HIT | **MISS** |
| `**.claude/skills/a/b.md**` | HIT | **MISS** |

**What was actually missing was not a better list — it was a stated failure DIRECTION.**
Neither form can enumerate its complement; the question is where an unknown character lands:

> an **exclusion** class makes it a *boundary* → over-match → **false positive** → the gate
> reds, somebody looks, somebody fixes it.
> a **positive** list makes it *not* a boundary → under-match → **false negative** → the gate
> is green and nobody ever finds out.

In a required gate those are not symmetric. The class is an exclusion again, `@` is handled
*inside* it (beside `+`, `%`, `-`), and the reasoning is in the wire so the next round cannot
re-derive it wrong. **Six new red-direction fixtures** pin the spellings above — the direction
that had no control at all.

**And the retraction had only reached the reader, not the gate.** Three reviewers independently
found that the line CI actually prints still said `K2: 0 … -- ABSOLUTE`, over a count that is the
**union** of the closed stored-path predicate and the half R4 had just called a heuristic. It now
prints the two separately. ⚠ This is the sweep-three-faces rule: R4 swept the *statement* and
neither the *obligation* (a control in both directions — unmet for the rule R4 itself wrote) nor
the *consequence* (the verdict line).

| # | What | Disposition |
|---|---|---|
| **CRIT** | the leading boundary failed open on five spellings | **Fixed** — exclusion class, direction stated, six red-direction fixtures + a mutation record |
| **IMP** | the printed verdict still claimed ABSOLUTE over the heuristic half | **Fixed** — the two predicates report separately |
| **IMP** | the status audit named `rev-list -n1 --all` as the positive test **after R4 removed it**, and omitted `symbolic-ref`, `show-ref`, `cat-file`, `tr\|cmp`, `_ancestor_link` | **Fixed** — table re-derived at HEAD with a derivation command, and the rule is now "re-derive when a mechanism changes, do not amend around it" |
| **IMP** | `_ancestor_link` was ordered **after** `[ -L "$f" ]`, so an external leaf symlink won and the gate reported a K2 hit on bytes outside the tree | **Fixed** — the ancestor question is asked first |
| **IMP** | *"THE WHOLE LIST … THE ONE PLACE IT IS STATED"* was stale: two rounds declared a non-coverage at its own site and a third created a new undecided class | **Fixed** — items 6 and 7 added, with the rule that a ⚠ beside the code is not the list |
| **IMP** | *"Every other control here proves the wire can RED"* — false; **10 of 53** are green-direction, two added three lines above the claim | **Fixed** — replaced by a derivation, which itself had to be run before being written down (the first spelling printed the wrong awk field) |
| **IMP** | §8 said **zero slots** while the loop had added obligations it was never reopened to see | **Fixed** — three slots, each with a fireable trigger and a date; own-deferral count 3, at the cap |
| **IMP** | both files crossed **1000 lines** during the loop; §5 item 5's cohesion argument was made against a 786-line file | **Slot 1**, which is the discipline's own shape: CLAUDE.md requires the split to be *its own PR*, so doing it here is what the rule forbids |
| **IMP** | `CLAUDE.md` still justified the ungated job with *"~1s"* while the measured figure is an order of magnitude higher — two SSoT statements disagreeing | **Fixed** — the justification is the absence of a setup step, and the stale figure is retired with the reason |

⚠ **The lesson I take from this round is about the loop, not the wire.** I ran the root-check,
wrote both mandated questions, reached a correct diagnosis — *the claim is the defect* — and
then shipped a patch of exactly the kind the diagnosis forbade, in the same commit, under a
heading saying I had not. **A correct root-check does not immunise the round it is written in**,
and the thing that caught it was the mandated design re-gate firing *because the loop was still
diverging* — the one guard that does not depend on my own judgement of my own work.

### §10.5 Round 6 — one finding, and it was about a rule I had inverted

**P1: *"Land the 1000-line split before this feature."*** R5's §8 had booked the touch-time split
as a slot to be done **after** #519 lands. That inverts the rule it cites twice over: CLAUDE.md's
heading is *"1000-line debt = touch-time split (**defer しない**)"*, and *prereq* means **before**.
What the slot did was take the sentence's other half — *"split は単独 PR / 単独 commit"* — and use
it as permission to merge the oversized files first.

⚠ **Reading a rule's constraint as a licence for the thing it constrains** is the failure worth
naming. The sentence admits **単独 commit**, which is exactly the shape available here: the split
lands *in* this PR, as its own commit, before merge.

**Done in this round**, at the seam the reviewer named: `…trip-wire.mutations.sh`, separating
*"can the wire reach every verdict?"* (the controls) from *"is each control about the arm it
names?"* (the mutation set and the correspondence between the two lists).

⚠ **Two things the split had to get right, and the standing negative control caught the second:**
- the **correspondence check moves with the mutation set**, not away from it — §9's argument
  against this seam was aimed at a split that moved the table and left the check behind, and that
  version would indeed have been worse.
- the mutation runner copies the **controls** beside each mutant; it now copies this file too.
  Without that, every mutant exited 2 (*"decided nothing"*) for a reason unrelated to its
  mutation, and the harness reported it as the entry failing. **The `!survive` entry died and
  said so** — a broken harness reporting itself, in the run that broke it.

**The wire is not split**, and that is a cohesion judgement (§0's first command prints the
code/comment split; no figure is kept here): two predicates over one walk into one verdict —
CLAUDE.md's *一枚岩の cohesive unit* exemption, which is a cohesion test and not a line count.
If that is wrong the seam to propose is **predicate-vs-walk**, named in §8 so the next reviewer
argues against a position rather than a silence.

### §10.6 Round 7 — five findings; two made by R6's split, one by the edit that retired a figure

| # | What | Disposition |
|---|---|---|
| **P2** | **A caller's git configuration reached the fixtures.** `_git` deliberately preserves `GIT_CONFIG*` (#501 R97 — a checkout readable only through a caller's `safe.directory`), which is right for the **real scan** and wrong for the **fixtures**. Reproduced here and by the reviewer: `GIT_CONFIG_COUNT=1 core.excludesFile=*.py` made the fixtures' `git add` skip their own `.py` inputs → **ten** `CONTROL NOT EXERCISED` on a clean checkout | **Fixed.** `_fgit` is a subshell that unsets `GIT_CONFIG` / `GIT_CONFIG_PARAMETERS` / `GIT_CONFIG_COUNT`; re-measured **10 → 0**. The asymmetry — preserve for the repository, scrub for the fixtures — is why there are two helpers, and it is now stated at the one that scrubs |
| **P2** | **One shim was failing both `ls-files` inventories.** `" ls-files "` is in the tracked (`--stage`) call *and* the worktree (`--cached --others`) one, so removing either arm's status check left the other's error to mask it — the reviewer deleted the worktree emission and the whole run stayed green. Mutation record 17 rewrote all three `_ls_rc` checks at once, so it only ever proved *at least one* remained | **Fixed.** `_ls_rc` → `_rc_tracked` / `_rc_worktree` / `_rc_head`, so each guard is individually addressable; the shim is narrowed to `--stage`; a `--others` shim, fixture, control and record are new, and the one collapsed record becomes three |
| **P2** | The split fragment's interface omitted `$_MUTATIONS` (which `_mut_run` reads to copy itself) and **wrote the caller's `ctl_ok`** — so a rename in the controls would leave this file assigning an unused global while the caller stayed green | **Fixed.** `_mut_correspondence` **returns** a status and the caller decides; `$_MUTATIONS` joins the checked contract. ⚠ This is the cross-file drift the entry guard exists to prevent, **reintroduced by the split that added the guard** |
| **P2** | `CLAUDE.md`'s `実測は一桁秒台` — a **new figure introduced by the edit that retired a stale one**, in a paragraph that says not to record elapsed time, and already false (the reviewer measured ~40 s) | **Fixed** by removing it, not replacing it. Third time this session a figure was introduced while retiring one; the rule is *derive it or omit it* |
| **P2** | §7 criterion 3 still said the records are *"in the controls file itself"* after R6 moved them, while §4's table already named the new file — the acceptance criterion and the artifact table giving competing accounts of the seam | **Fixed.** The property the criterion is about is *ships with the thing it is about*, which the split preserves |

⚠ **And one finding of my own, from fixing the second**: excluding `lsfail` from the fixture
repository loop had been justified by *"its shim fails `ls-files` whatever the directory is"* —
which **stopped being true the moment that shim was narrowed**. Its worktree inventory then ran
for real against a non-repository and the control got `read 0 stored objects` instead of the
inventory error it names. **An exclusion justified by another mechanism's breadth expires when
that mechanism is narrowed, and nothing links the two but a note** — so the note is now at the
exclusion.

### §10.7 Round 8 — two findings; the self-test entry moved off the environment

| # | What | Disposition |
|---|---|---|
| **P2** | **Self-test mode was reachable from a parent shell.** A shell that exports `WEBREF_WIRE_SELFTEST` and `WEBREF_WIRE_SELFTEST_PPID=$$` is the parent of every wire it later launches, so the PID check passed and an ordinary run skipped every control and scanned the fixture. Reproduced here in a `git clone --local` sandbox with a violation planted in `_webref/`: the wire at `0ba1ed2c` printed `PASSED` over the fixture; the fixed wire scanned the checkout and exited 1 | **Fixed at the mechanism, not the value.** This was the third authorization this entry had carried — a literal token (R1's fix), then the parent's PID (R3's fix), now broken the same way — and the root is that the **environment is inherited by definition**, so no value in it separates "the controls started this" from "a shell with the export started this". Self-test mode is now `--selftest <root> [dir] [extra]`: arguments are not inherited and the driver passes none, and the old names are no longer read at all. The control is R8's reproduction verbatim (both names exported, the companion equal to the real parent, pointing at the clean fixture, while the argument names a violating one); a second control pins the missing-root refusal. One mutation record each |
| **P2** | The umbrella's A-i-wire row **copied** this memo's artifact list, slot verdict and blind-spot count, and all three had moved on | **Fixed by removing the copy**: the row now points at §4 and §8 instead of restating them. The same sweep found no other copy — the A-i memo's §8 passage states the blind-spot/slot argument, which still holds, and its §0 record of what A-i once carried is history |

⚠ **Neither finding came from R7's fixes**, which is the stop condition set for this round. Both
are older self-introduced defects: the PID check was R3's fix, and the row went stale when R5/R6 changed what it copied.


### §10.8 Round 9 — five findings on one head, and the loop stops here

Five threads on `0c0187b4`: three from a review Codex ran on the R8 push by itself (02:34Z), and
two from the R9 trigger. ⚠ **The first three sat unread for six hours**: the landing probe
counted only items newer than the R9 trigger, so a review that arrived before it was invisible
to it. Found by the full thread fetch, which is unscoped.

| # | What | Disposition |
|---|---|---|
| **P2** | §9 still dismissed an optimisation with *"a gate that runs in single-digit seconds"* — the figure R7 retired from `CLAUDE.md`, surviving in this memo | **Removed**, with every other figure in that table (a speed-up factor, a percentage, a duration). The ground each row gives no longer rests on a number |
| **P2** | §9 called two declined items *"the right change … recorded as a follow-up"* and *"recorded so the third copy does not have to rediscover it"* — obligations by their wording, absent from §8 | **Rejected as obligations, and now worded as such**: both rows say *not owed, so not a slot*. Nobody is committed to either; the gate is correct without them |
| **P2** | Under a **relative** `TMPDIR`, GNU `mktemp -d` returns a relative path, the trap's `/*/*` guard matched nothing, and every run left its scratch (a normal run: the whole fixture tree) behind | **Fixed by construction**: the scratch path is resolved to its physical absolute form before the trap is installed. macOS's `mktemp` ignores a relative `TMPDIR`, so the control uses a shim that answers the way GNU does and checks what the run leaves behind. Record + control |
| **P2** | *"Run the 40 controls"* here and *"41 invocations"* in the wire — both stale | **Removed**, and the same sweep found a third (*"the other thirty-nine"*) and a fourth (*"all 18 entries below"*) in the controls and mutation files |
| **P2** | A real tool's path was spliced **unquoted** into every generated shim, so a git or grep under a path with a space made the shims invalid | **Fixed at one helper**: `_shq` single-quotes every path written into shim source (every shim, the `mktemp` one included). The helper is the harness's own part, not an arm of the wire, so it gets no mutation record (the set edits the wire): it is asserted instead, by round-tripping a path holding a space, a quote, `$` and a backtick through `/bin/sh`, and the run refuses to continue if that fails. A first version added a `_control` plus a record aimed at the helper; the record could never match, because the mutation set edits the wire, and the run said so |

⚠ **The stop condition set before R8 fired.** Finding 1 is a sweep miss of R7's own fix, which
makes four consecutive rounds whose findings include one of this loop's own fixes. Per that
condition the Codex loop is **not re-triggered**. What stands in for its remaining rounds is an
enumeration by a fresh agent over the populations these findings came from — figures and
universals in prose, every shim and what it wraps, every status site, every boundary rule in
both directions, and what the fixtures inherit from the caller's environment — with anything
it finds either fixed or listed for the merge decision.

## §11 Design revision — after the enumeration attestation (rev 3, plan-review required before implementation)

### §11.0 How this section got here

The Codex loop stopped at R9 (§10.8). A fresh agent then enumerated the populations defined in §11.4
over a frozen snapshot (`d5dcad1c`) and found the CRIT and IMP items disposed of below, plus the MIN
items listed in §11.4 — nine rounds of external review had been sampling that population one member
at a time.

* **Rev 1** (`470d7fcc`) re-executed this wire under `env -i` and an allowlist. Its plan-review
  measured why that cannot work: bash consumes `BASH_ENV`, `SHELLOPTS` and `BASH_FUNC_*` at startup,
  before a script's first line (`SHELLOPTS=noexec` and an exported `exec` both stopped the re-exec); an allowlisted `GIT_CONFIG*` carries any
  git config key (`core.fsmonitor` ran a command inside this wire's own `ls-files` call); and the
  class reaches the sibling wires too. Rev 1 also proposed *declaring* D3 — the fail-safe error §10.4
  records.
* **Rev 2** (`c94e1744`) carved the whole environment class to a new umbrella slice (user decision,
  2026-09-23). Its plan-review found the premise of that carve false for D1 — `grep` reads its
  environment on every exec, so a wire can close `GREP_OPTIONS` for its own calls — and found the
  carve's ledger wrong (most carved items were this PR's own defects) and the new row at odds with
  the umbrella's forced ordering. It also measured holes in every point fix (folded into §11.2).
* **Rev 3** (this; user decision, 2026-09-23): sort the environment findings **by the direction they
  fail**, close the silent ones here, and carve only what is pre-existing and closable nowhere but
  where bash is started.

### §11.1 The environment findings, by failure direction

The wire's rule for modes reachable from outside (beside `_SELFTEST`) is *unreachable or loud,
never silent*; this section applies the same rule to inputs. It, not a list of variable names,
decides each row.

| input | fails | disposition |
|---|---|---|
| `GREP_OPTIONS` (D1) | **silent** — BSD grep honours it; an exclusion makes a named file read as "no match", so a local run goes green over a violation (CI's grep is GNU, which does not honour it; not measured here) | **closed in #519**: `unset GREP_OPTIONS` at the wire's start. Not a denylist of a guessed population: BSD grep's manual (`man grep`, ENVIRONMENT) names one variable, `GREP_OPTIONS`; locale is already pinned by `LC_ALL=C` |
| `POSIXLY_CORRECT`, `BASHOPTS=inherit_errexit` (cause of D4) | **silent** — the `_scan` subshell dies mid-walk and the verdict is taken over what it had emitted | **closed in #519 by construction** (D4, §11.2): completion is asserted, whatever killed the walk |
| `GIT_TEMPLATE_DIR` (D6) | loud — controls report NOT EXERCISED and the run is red | stays loud; the claim that `_fgit` "neutralises both config layers" is deleted |
| `GIT_TRACE*` / trace2 config (D7) | loud — red, with a misleading "walk reported errors" | stays loud; noted where the stderr rule is |
| `CDPATH` (D8) | loud — exit 1 with a `cd` error | stays loud (the exit code is the violation code, not "decided nothing" — noted, not changed) |
| a restrictive `umask` (half of D5) | loud once D5 makes the guard reachable (exit 2 with a message) | D5, §11.2 |
| `XDG_CONFIG_HOME` / `SUDO_UID` (`safe.directory`) | loud — a refusal (exit 2) | stays loud |
| `PATH` (D12) | **either direction** — the wire names its external tools and does not resolve them, so `PATH` decides which binary each name reaches, and a substituted one can answer wrongly without saying so | **bounded in practice by the controls, not by the wire**: they re-invoke this wire with the caller's `PATH` over fixtures whose answers are known, so a substituted tool that changes an answer a fixture pins reds the gate before the real scan. The residual is closable only where bash is started — slot `#11-trip-wire-launch-environment`. Not closable in the wire: absolute paths would be a second, hand-kept inventory of the same kind this file keeps retiring |
| `BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*`, `core.fsmonitor` via `GIT_CONFIG*` | whatever the person running the gate makes it do | out of this wire's reach (startup) or an environment subverting its own gate; **declared** where bash is started — slot `#11-trip-wire-launch-environment` |

**Own vs pre-existing**, per the defer policy: the rows this wire itself decides are closed here
when they fail silently and accepted under the wire's own rule when they fail loudly — not
deferred. ⚠ **Two rows are not the wire's to reach at all**, and an earlier wording of this
sentence said every row was: the startup row (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*`, read
before this file's first line runs) and the residual half of `PATH`, which the controls bound
in practice and which closes only where bash is started — and both are in the carve the next
sentence names. What **is** carved is pre-existing on `main` and not this PR's to fix: the sibling wires'
ambient `grep`, the driver's environment-entered `TRIP_WIRES_SELFTEST` (D9, from #496), and the
startup declaration, `PATH` included — one slot, `#11-trip-wire-launch-environment`, registered in the defer ledger
with an owner route, trigger and date. It does not count against this PR's own-deferral cap.

### §11.2 Point fixes in #519

| # | defect | change | control | record |
|---|---|---|---|---|
| D4 | `_verdict "$(_scan …)"` discards `_scan`'s status; a subshell killed mid-walk yields a verdict over what it emitted | `_scan` emits a terminal record after its last source **and** on its early `return` (the `no temp file` arm); `_verdict` refuses a stream without exactly one, in last position, **before** the `SCANNED -eq 0` guard | fixture: the `catfail` shim plus a tracked entry that sorts **before** the killed one (e.g. `a.py`), under an injected `POSIXLY_CORRECT=1` — pre-fix this is exit 0 PASSED, post-fix exit 2 "did not complete" (pre-fix status measured: 0) | removing the check |
| D2 | a tracked file under a directory with read but not search permission: `[ -L/-f/-e ]` all fail with EACCES, the path reads as tracked-and-gone, no record, green | "absent" needs a positive test: walk up to the **nearest existing ancestor**; it must be a searchable directory and the next component must be absent from it. A tracked path that is neither a file, a link nor provably absent is an `err`. This keeps a tracked directory deleted wholesale (a routine unstaged state) green | red: tracked file, dirty worktree, parent mode 0444, no untracked sibling (a geometry measured to discriminate; adding an untracked sibling or a violation in the index blob reds the wire without D2). Green: a tracked file whose directory was removed. Reported NOT EXERCISED where permissions are not enforced (the `$_perm_line` pattern) | removing the searchability test |
| D3 | a `]`, a quote or a backtick inside a segment ends it, so `.claude/tools/team]inc/rule.md` is missed | **widen, don't declare**: a segment **followed by `/`** is bounded only by `/` and whitespace; only the segment that **ends the match** keeps running-text terminators. `K2: 0` on the real tree is measured **at implementation**, for this class (rev 2's narrower widening measured 0; this one is wider) | red: each of `]` `"` `'` and backtick, both as the first segment and as a later intermediate one. Green: every existing green control. **New boundary this opens** — a quoted one-segment reference followed directly by a `/`-bearing token (`".claude/tools/webref","/tmp"`) now matches: it fails safe (red); it gets a control and a line in the header's list of what the heuristic half decides | one per widened character |
| D5 | `x=$(…)` under `set -e` exits before the guard that follows, at **both** sites (`SCRATCH`, `_root_p`) | `x=$(…) || x=""` at both | one per site: a restrictive umask for `SCRATCH`; a `--selftest` root that cannot be resolved for `_root_p`. `_control` has no umask channel, so the first is a block of its own (the `relcwd` pattern) | one per site |
| D10 | boundary rules with a control in one direction only | one control per missing direction, **re-derived after D3** (D3 edits the same classes) by P4, not taken from the pre-D3 list | as derived | one each |
| MIN | false current-state claims (§11.4) | **deleted by default**; corrected only where deleting would leave a reader believing something false, and never with a new count or list — a figure that matters is a command | — | — |

**The PR body** names D1's former local false green, its fix, and the carved slot, when #519's head
is next updated.

### §11.3 Coupled invariants

* **D4 terminal record × the sentinel arms** — a sentinel `err` (from `cat`/`readlink`) is a record,
  not the end; the terminal record comes only after every source or at the early return.
* **D4 refusal × the `SCANNED -eq 0` guard** — the refusal runs first, or a walk killed before its
  first record reports "read 0" instead of "did not complete".
* **D4 terminal record × the `forge` fixture** — a path cannot forge the terminal tag (`_esc`/`_onerec`
  map LF; measured); the `forge` fixture is extended to the new tag.
* **D2 positive absence × the untracked vanished-path arms** (`phantom`, `globspec` expect "gone") —
  D2 applies to tracked paths; an untracked path under a 0444 directory already reds (git warns), but
  its message still claims "gone" — corrected to not assert a negative the wire did not establish.
* **D3 widening × D10 × the `$K2RE` mutation needles** — D10's population is re-derived after D3;
  every existing needle must still apply to the widened text — measured at implementation (rev 2's
  narrower widening orphaned none).
* **New controls × the harness budget** — `ci.yml`'s `timeout-minutes` line is re-derived in the same
  PR, as that line requires.
* **New controls × the mutation ratchet** — every new control ships with its record.
* **New controls × the controls file's size** — if this PR takes it past CLAUDE.md's threshold,
  the touch-time split is its own commit first, at a seam named then. It did; §11.7 row 7 records
  the seam. Whether it is past the threshold again is `wc -l` on the file, not a figure here.
* **D5 × trap order** — the trap is installed after `SCRATCH`'s assignment and before its guard; an
  empty value is a no-op in the trap's `/*/*` case, and the guard removes the raw directory itself.

### §11.4 The instrument, recorded so it can be re-run

A fresh agent, on a frozen snapshot, enumerates **every** member of each population — one line per
member, each backed by the command that produced it, marked OK or DEFECT:

* **P1 — quantities and universals in prose**: the comments of this wire's `.sh` files — the wire,
  its controls, the control harness and the mutation set — and of
  `scripts/trip-wires.sh`; this memo's header and §0–§11 (§10 as provenance); every umbrella row and
  paragraph naming this slice; the `CLAUDE.md` and `ci.yml` paragraphs. Every figure describing current state,
  and every "every / all / only / no / none / never / the one place" with its complement measured.
  Enumerated with an unfiltered word grep; context read separately.
* **P2 — shims**: every generated script — what it intercepts, whether its pattern is verb and flag
  as one sequence, whether every embedded path goes through `_shq`, whether the wire makes that call.
* **P3 — status sites and verdict sites**: every consumed exit status — what non-zero is read as,
  whether a positive check establishes it — reconciled against the header's audit table in both
  directions; and every verdict the wire can emit, with the control that reaches it.
* **P4 — boundary rules**: every rule in `$K2RE`, `$K2RE_PATH` and the walk, with its red and its
  green control; a missing direction is proven by a surviving mutant.
* **P5 — caller environment**: every variable or ambient input reaching the scan or the fixtures,
  classified by failure direction as in §11.1.
* **P6 — mode entries**: every way into a non-default mode from outside; each unreachable from an
  ordinary driver run, or loud.

The attestation at `d5dcad1c` found these MIN items (to be deleted or corrected per §11.2's MIN row):
wire — "every boundary rule carries a control in both directions"; "never answers green over
something it did not read"; the status audit table's omissions (`cat`, the `[ -L/-f/-e ]` tests,
`_phys`/`cd`, `: >`, the discarded `$(_scan)`) and overclaims (`ls-files --error-unmatch`'s positive
test; `tr|cmp`'s NUL message on a plain failure); "the whole list … the one place"; "none of these is
a deferred obligation" vs item 7; "every line entering this tree passes review"; item 6's character
list; "two things no control pins"; "every git call goes through `_git`"; "`--opt=<path>` is how
`cli.py` spells its examples"; "two of those shapes are live"; "closing it cannot open anything";
grep stderr "fails the run"; "every other stored value avoids `$( )`"; "an unborn HEAD — every
fixture here"; "`$PIN`"; "`_scan` returns 0 unconditionally and reports every failure"; "five named
modules". Controls — "prove the wire can reach every answer" (the gap itself: slot
`#11-k2-wire-verdict-site-controls`); "`_fgit` neutralises both config layers"; "every shim writes a
real tool's path"; "one call = verb and flag"; "every other red fixture writes its path after a
space or a quote"; "each line below is a spelling live in the scanned tree"; "one name is absent …
cachedir"; "the three ways". Mutations — "`$_CONTROLS` … from the controls file"; "writes nothing
of the caller's"; "every needle must name an actual `_control` label" (a quoted-string grep); deleting
a paired or `!survive` record is silent (a code defect, not prose: the correspondence check gains the
`!survive` record and counts records per needle). Memo §0–§9 and header — the line counts and the
scanner-vs-largest-wire comparison; "carried both files past 1000"; "the list is now five"; the
superseded `check-ignore` precondition; the consumes list; "not taken" for the controls split;
"invoked as `bash "$_CONTROLS"`"; §6's retired tool enumeration; "nothing in §5–§7 is open"; "one
predicate" (at every site: `grep -n 'one predicate'`); §8 slot 2 "monotone-downward" (the slot's
trigger rests on it and is re-derived, not reworded).

**§8's cohesion judgement is re-derived, not patched**: it rested on "one predicate, one walk, one
verdict" and a line count. There are two predicates (`$K2RE`, `$K2RE_PATH`) over one walk into one
verdict; the seam a split would use is still predicate-vs-walk, and the wire stays one file on that
ground — the counts are dropped.

### §11.5 Terminator, and what happens if it is not met

plan-review of this revision → implement → P1–P6 by a fresh agent on the implementing head. **Met**
when no CRIT/IMP is found that the wire or its controls can close; a finding closable only where bash
is started goes to `#11-trip-wire-launch-environment`, and nothing else is routed away. **Not met** →
fix once, then re-run the touched populations on the fixed head. A finding that traces to this
revision's own edits (its diff touches the line the finding cites) stops the loop and is reported
rather than fixed in a second round. MIN findings from the re-run are deleted per §11.2 or listed in
the PR body. Then the merge decision, with the land order against #501, goes to the user.

### §11.6 Rev 3's review — three focused passes, and where they leave the design

Rev 3 was reviewed with a changed instrument, because both earlier plan-review rounds had been reset
by text the previous revision itself introduced: a disposal check of rev 2's findings, an audit of
every claim rev 3 newly wrote, and a prototype of §11.2 in a `git clone --local`, run on bash 5.3 and
3.2 with the mutation set. **No CRIT.** Every mechanism worked as specified; the real tree stays
`K2: 0` under the widened class; each new red control fails on the pre-fix wire. What remains is
implementation detail, most of it caught by the mutation harness itself — which is where this plan
review stops and the implementation's own gates take over.

| from | finding | disposition |
|---|---|---|
| disposal | `core.fsmonitor` (via the deliberately kept `GIT_CONFIG*`) was routed to the launch slot, but it reaches this PR's own `ls-files` calls, and a globally enabled fsmonitor changes which files git reports | **own, closed in #519**: `_git` passes `-c core.fsmonitor=false -c core.untrackedCache=false` on every call; control: an injected fsmonitor hook must not run |
| disposal, audit | §8 still counted two own deferrals | §8 lists `#11-k2-wire-verdict-site-controls` and reads **3 of 3** |
| audit | the ledger routed `POSIXLY_CORRECT`/`BASHOPTS` to the launch slot as "closable only where bash starts" — a script can undo both, and D4 closes their effect here | dropped from the slot; D4 is their owner |
| audit | "the wire's rule … a mode **or input**" — the wire's sentence covers modes | §11.1 says the rule is the wire's rule for modes, applied here to inputs |
| audit | P1 said "both umbrella rows" — more rows name this slice | P1 reads "every umbrella row and paragraph naming this slice" |
| audit | §11.0 counted `POSIXLY_CORRECT`/`BASHOPTS` among what defeats a re-exec; what was measured to defeat it was `SHELLOPTS=noexec` and an exported `exec` | worded to what was measured |
| prototype | D3 re-aims two existing records: one's sed anchor no longer matches the widened assignment; one now dies for the wrong reason (the widened first segment admits `"`, so `quotename` no longer separates the two predicates) | both re-aimed in the implementing commit; the mutation set's own "matched nothing" / "wrong reason" checks are what found them |
| prototype | D3 as worded leaves `…/a/]x/` (a trailing slash) undetermined between "intermediate" and "final" | the fail-safe reading: an alternation — a segment followed by `/`, **or** the old final-segment class — so it matches |
| prototype | D3's later-segment red controls only discriminate with the character **leading** that segment (`a/]inc/…`; `a/team]inc/…` already reds pre-fix) | that geometry |
| prototype | D3 listed controls per character but records per class | one record per control (the ratchet does not move) |
| prototype | D1 had no control and no record; deleting `unset GREP_OPTIONS` survived | control: `_ctl_env` `GREP_OPTIONS=--exclude=*` on a violating fixture (discriminates on BSD grep; on GNU grep it passes trivially — stated at the control); one record |
| prototype | the D5 umask block and mode-000 root cannot fail where permissions are not enforced (root) | both inside the `$_perm_line` gate, NOT EXERCISED otherwise |
| prototype | D2's nearest existing ancestor being a regular file (a tracked directory replaced by a file) is ENOTDIR — a positive absence — but the rule made it an `err` | a non-directory nearest ancestor is absence |
| prototype | D4's "remove the check" record contained the tag's TAB, which splits the record | the record edits the check without the TAB |
| prototype | D4's early-return terminal record has no discriminating control | accepted: both paths exit 2 and differ only in message; stated at the arm |

### §11.7 What the implementation decided where §11 did not say

Implemented over `996ec177`. The dispositions above are the specification; these are the points
it left open, answered in the code and recorded here so the attestation §11.5 calls for reads a
position rather than a silence.

| # | Left open | Decided |
|---|---|---|
| 1 | `_absent`'s remaining shapes: a nearest existing ancestor that is a dangling symlink, and a walk that runs off the top of `$ROOT` | Absence is claimed for a searchable directory, for a non-directory (ENOTDIR) and for a dangling symlink — measured: `_absent` returns absence in that last case, because a path under an unresolvable link is unreachable in the worktree and its content is still read by the index and HEAD passes. A walk that exhausts its components returns "not established", i.e. an `err`: unknown lands on red. ⚠ An earlier wording of this row said the dangling case errs, which the code does not do — the code is the one that is right here |
| 2 | how the two re-aimed records are re-aimed | `quotename`'s mutant now excludes `"` from `$K2RE_PATH`'s segments, which is the claim its label makes; the old `K2RE_PATH="$K2RE"` mutant is kept and re-needled to `stagedlink`, whose staged target holds a space — the shape that still separates the two predicates after D3. `punctslash`'s mutant now adds the closing-punctuation exclusion to the FIRST segment, which is the over-reach it was written against |
| 3 | how the fsmonitor control proves anything where a hook cannot run | It is not a `_control` (the question is whether a command RAN). The hook is first shown to run under a plain `git ls-files` over the same fixture; if it does not, the block reports **CONTROL NOT EXERCISED** and reds, rather than passing because nothing happened |
| 4 | `-c core.untrackedCache=false` has no control | Stated at the site and in the wire's list of what no control pins. It is caller state of the same kind as `core.fsmonitor`, and turning it off costs nothing this gate needs |
| 5 | what "counts records per needle" means for the correspondence check | Two checks rather than a per-needle table, which would be a hand-kept list of the kind this file keeps retiring: the `!survive` record must be present exactly once, and the record count has a floor (`_MUT_RECORDS_MIN`) that an edit must lower deliberately. ⚠ What neither sees is a deletion and an addition in the same edit; the added record still has to kill |
| 6 | D10's population after D3 | ⚠ **The answer written here was that *each* rule of `$K2RE` and `$K2RE_PATH` had been mutated against the control set, and it was false.** The enumeration was by hand, and successive attestations each declared it complete and each missed members of it — the leading `/`, the class's `A-Z` and a one-character final segment first; then, on the head that closed those, the stored first segment's minimum length (under which `.claude/skills/a/rule.md`, an entry whose own NAME is the forbidden hierarchy, reads K2 zero on a required gate) and members of the final segment's middle class, which the existing green lines could not see because they end in `)`. What now stands in place of the claim is a **mechanism**: `_mut_gen_run` in `…trip-wire.mutations.sh` **derives** the population from the wire's own two assignment lines — one mutant per bracket-expression member, one per quantifier, one per alternation branch, one per escape — splices each back over that line, and requires every one to red the control set or to be named in the **equivalence table `_mut_equivalent`, in that same file**, beside the argument why it cannot change a verdict. A generated mutant that is neither fails the run; so does an assignment it cannot read back, and so does an equivalence entry naming a mutant the generator no longer produces. It runs with the hand-written set, under the same opt-in. It found rules neither attestation had named, and the one equivalence argument that had been offered is refuted at that table. The hand records still say **which** control catches a rule; the generator says **that** no member of the population it returns is unwatched, which is the half a list cannot do. It does not return every direction: widening a branch list needs a payload the scanner would have to invent, so that one stays with the hand records |
| 7 | the controls file's size | It would have passed the threshold, so the touch-time split landed first, as its own commit: the control **harness** (how a control runs) out of the control **catalogue** (which controls exist). §4 and §8 carry it |

⚠ **What this implementation did NOT do**: the PR body (§11.2's last line) and the defer-ledger
registration of `#11-trip-wire-launch-environment` (§11.1) are not in this diff — they are
written where the PR and the ledger live.

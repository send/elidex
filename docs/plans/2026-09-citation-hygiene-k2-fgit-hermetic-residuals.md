# K2 fixture git — the residuals, the slot and a pre-existing defect

This file holds §5 (§5.1, what the window does not close; §5.2, the other residuals and the pre-existing
defect) of `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). It was moved
here unchanged as a touch-time split, before draft 19's edits; the design memo keeps a pointer at the
section. Draft 19 then extended §5.1's class-(c) sentence (D17-A), and drafts 20 and 21 extended it
again (D18-A, D19-B). The section numbers are kept, so "§5", "§5.1", "§5.2" and a residual "R1"–"R9", "R11", "R14" and "R15" resolve here (R10, R12 and R13 were discharged and removed by CR2-D, `…-cr2-plan.md` §7; R14 and R15 were added by CR2-C2, `…-cr2-plan.md` §9; residual numbers are not reused).
Every other section reference without a file name is to the design memo.

---

## §0 Spec coverage map

**No spec surface** — this file holds the residuals of a shell-harness slice. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §5 Residuals, the slot, and a pre-existing defect

### §5.1 What the window does not close: class (c)

**The unit.** A fixtures-file command can give a git process **an input not produced inside the
window** in three ways: by **removing** an allowlist entry, **overriding** one, or **adding** one.

The outside files are reachable from inside the window:
- the Homebrew system file and template are **caller-writable** (`/opt/homebrew/etc/gitconfig` is
  owned by `kazuaki admin`, and so is the templates directory);
- the caller's real home is computable with `eval "h=~$(id -un)"`.

**A command that leaves the input persisted is class (b), caught by P-g on shape (D2, E2)** when what
it persists is configuration or a git dir's shape, and by P-k when it is an object store outside the
repository; `…-census.md` §4 ("What P-g cannot see") says what lies outside all three. P-g reds, on
both shells:
- `printf '[include]…' >> .git/config`;
- `printf '[core]\n\texcludesFile=…' >> .git/config`;
- `git -C . config core.excludesFile …`;
- `git config include.path …`;
- `--separate-git-dir`, with or without a persisted key; a nested, hidden or bare git dir with a
  persisted key (draft 9, corpus §6).

On p7 the first three were **silently wrong**. The fourth was red only through S (companion §A.11). P-g
sees what is **still persisted when the build ends**. A form that persists and then reverts is class
(c).

**Class (c) is the residual, by property.** It is any fixtures-file command that gives a git an outside
input and leaves no persisted, observable state when the build ends (§0.3 gives examples). Measured on
p8: `git -C . -c include.path=<file> add -A` gives rc 0, `PASSED`, and Pdiff 50 on both shells. That is
silently wrong, and it is **declared**, not closed. A write to the verifier's own state is the same:
`xdone` (the markers written by the fixtures file) and `xredef` (`_fgit_postconditions` redefined), each
with a key persisted in `clean`, give rc 0, PASSED (§0.3; `…-pr527.md` round 14), and so does `xrace`,
a background process that outlives the build and writes the reference, and so do `mkfn`,
`shimafter` and `shimhash`, which change what the `mktemp` the postconditions run resolves to (a
fixtures-file function, a `PATH` entry after the pin, `hash -p`; §0.3 states the class by that
property, D17-A; `…-pr527-r15.md` §Q), and so does `staleshim`, an executable the fixtures write into a
caller-`PATH` directory, which changes what the **parent's** `mktemp` resolves to after the window
(D18-A extends the property to the parent), and `srcrw`, a `_mut_correspondence` the fixtures append
to the checkout's `mutations.sh` (D19-B: the property is which executable or code the verifier runs).

**D1: seed S is deleted.** Round 7 (Ax3) measured literal spellings passing S end to end with Pdiff 50
on both shells:
- `git -C . -c include.path=…`;
- the fixtures' own shim idiom, `printf '#!/bin/sh\nexec %s -c include.path=<file> "$@"\n' "$_REAL_GIT"`
  followed by `PATH=<shim dir>:$PATH git add -A`.

It also measured false reds: `commit --template=/dev/null`, `printf 'export x'` written into data, and
`git config core.autocrlf false`. Widening the regex would be the wrong repair: S's population is a
vocabulary, not the property. And keeping a second, partial surface for the same residual is decision
tax. So S, its label, its record and its census are removed. Draft 7 had adopted it, and this is
recorded as the removal of a round-7 instrument (`…-reviews.md` §D.7).

**`#11-k2-fixture-git-invocation-convention` closes (E1).** Draft 8 re-scoped it to the transient
residual. Under §0.3 that residual is class (c), so the create-time audit is applied to the slot as it
would stand:
1. **Does it name a defect this PR leaves?** No. It names a class the threat model declares out of
   scope. That is a boundary of the mechanism, not a gap in it.
2. **Is work owed?** No. Every candidate remedy is already rejected: an in-process detector is a
   vocabulary (S failed both ways), and an OS sandbox, row (iv) of §3, has no portable mechanism. Review
   of the fixtures file is the standing owner of all repository code, not a deferred task.
3. **Does it have a trigger that brings work back?** No. Draft 8's trigger, "reviewer attention on any
   change to the fixtures file", is a standing policy, not an event.
4. **Would keeping it change any decision?** No. It would only restate §0.3 in the ledger.

So it is **not a slot**. The ledger entry is removed, citing §0.3. If the threat model changes, for
example if fixtures come from untrusted contributors, that is a new decision with its own slot, not this
slot's trigger. The slot's original premise, verbatim from the ledger, "Measured 2026-09-26: the build
region contains no bare `git ` outside comments", is **moot**.

**Accounting:** 1 own deferral, `#11-trip-wire-liveness-bound` (§5.2 R9, draft 13). With
`#11-k2-fixture-git-invocation-convention` closed, the keep-set slot dissolved below and
`#11-k2-wire-exit-trap-masks-set-u-abort` closed (§5.2, `8f4ad3c8`), the ledger nets to **−2**.

`#11-k2-fgit-keepset-depends-on-git-purge-glob` **dissolves**. There is no keep-set and no helper that
calls `_git`. Its premise sentence was absent at `ff6b99a3`: the command below returns rc 1.

```sh
git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i -e exempt -e 'held back' -e purge
```

`_git` inside the window is **undefined**. A fixture calling it fails loudly: the corpus RLOUD cell gives
NE.

### §5.2 Other residuals, and the pre-existing defect

| # | residual | direction | disposition |
|---|---|---|---|
| R1 | which git executable runs, and which of the fixtures' other commands: the caller's `PATH`, verbatim, behind the pin (§0.1). A wrapper that needs other variables, or a tool that only a working-directory- or home-dependent entry provides, fails in the window (loud), and a wrapper that execs "the next `git` on `PATH`" cannot reach the pin again, because every exec-through shim drops its own directory from `PATH` before it execs (design memo §0.1, P-j; before the fix round it was an endless exec cycle, a wait; the fix round's depth counter missed the `env -i PATH=…` re-exec, review R2): it runs the caller's next `git`, and one that re-execs under `env -i` drops the window's variables and is red by their postconditions, but a `PATH` git wrapper that injects configuration only into `add` is **silent** | silent | class (a)'s **declared platform boundary** (§0.1), not closed: `PATH` → `#11-trip-wire-launch-environment`, member (3). That slot's body does not name the K2 window, and its trigger ("#519 lands") has fired; §9's ledger step amends both. Draft 10 called this "loud"; `/code-review` showed the silent case |
| R2 | filesystem-derived config written by `init`; FIFO support, permissions, raw names | either | outside P. P-d's reference `init` shares the filesystem |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any | declared blind spot. None is known at 2.55 |
| R4 | the **launch-environment class**: whatever the caller injects into the wire's own bash at startup (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*%%`, a function named `command`, a caller's `POSIXLY_CORRECT`). The window's `env -i` drops these from the child's environment, but the parent that writes the prelude has already run under them. The prelude carries only the listed data and the ten function bodies §3 lists. The loud direction, where a control's own child inherits the variable, is this row's: `POSIXLY_CORRECT=1` with the wire reds at `catfail` (`expected exit 1, got 2`), the same at base, and the #519 memo's "D4 closes their effect here" and "closed in #519 by construction" (both in its §11, moved to `…-Ai-wire-k2-trip-wire-review-rounds.md` by CR2-D: §11.6 :473 and §11.1 :340) are true of the silent direction (the scan's verdict) only. A member of this class is a caller's stale `WEBREF_WIRE_LACKS`, read from the caller's environment in the parent by `_cap_check`: on a machine that holds the capability it declares an absence the machine does not have. It cannot make a control skip (the probe's flag decides), but the shipped run there carries no evidence for the three probe-flip records, since a wrong 0 would be accepted; an unknown word in it is red under the capability table's label | any | `#11-trip-wire-launch-environment` |
| R5 | `$SCRATCH` owned by another UID | loud | outside P |
| R6 | reads through `_git` keep the caller's config, including a caller `GIT_TRACE=1`, which reds at base too (the #519 memo's D7, `…-Ai-wire-k2-trip-wire-review-rounds.md` §11.1 :342) | loud, pre-existing | `_git`'s contract |
| R7 | Windows git-bash: unmeasured | unmeasured | **declared residual.** The `trip-wires` job is ubuntu-only in CI (command below), and running `mise run ci` or the wires under Windows git-bash is not a supported surface today; nothing here claims it |
| R8 | a compiled-in reftable default (git 3.0's planned default, or a breaking-changes build): `badref` writes `.git/refs/heads/`, and P-d's `diff -r` differs between two reftable inits (random `reftable/*.ref` names, `tables.list`) — the gate would red on every PR for a non-K2 reason | — | **closed**: `GIT_DEFAULT_REF_FORMAT=files` is in the window's allowlist, pinned by P-i on every git. The `reftable` cell (a `git` that picks reftable unless the caller pins a format) is red before and green after (companion §A.14) |
| R9 | **no time bound**: a child whose read does not complete makes the run wait. By property: git reads, outside every census git dir and through a reference the census does not follow (`commondir`, `include.path`, `alternates`), something whose read does not complete (`…-census.md` §4, "The boundary": a FIFO, or `/dev/tty` under a controlling terminal); a read that completes is compared instead, by P-g's origin or by P-k. The per-link search over a huge tree is slow (`find` ends when it has walked it). The one watchdog is `_ctl_run`'s (#501 R92); `_control`, the relcwd and fsmonitor blocks and the four standing checks run through `_ctl_run`; everything else that reads fixture state is unbounded: the fixture build window, the census (its per-link `find -L`), P-k's `count-objects` and its probe, the mutation trials, and the fsmonitor block's plain `git ls-files` (a preflight at base; since `d976f239` a follow-up after `_ctl_run` returns) (`…-landing.md` §9.2 lists them) | never green: red at CI's job timeout; a local run waits | **slot `#11-trip-wire-liveness-bound`**, text in `…-landing.md` §9.2. C9 rewrites the harness's declared blind spot ("There is no time bound…") to name it. The user carved draft 12's bound out (2026-09-28) |
| R11 | **a control declared a capability-group member that does not need the capability** (the capability table, `_CAP_TABLE`, in `.claude/tools/webref-generic-core-trip-wire.controls.sh`). The gate declines every member of a group whose flag is 0, so a member that does not need the capability is declined on a machine that lacks it, and runs on every machine that has it | declined where the group is absent, never wrong where it is present | **Gap:** the table cannot test a member's need. **Why:** the need is the control's subject, not the table's; membership is a declaration, visible as a table edit and in the summary's count of the group's controls, with no effect on a machine that has the capability. **Trigger:** any `member` row added or changed (its commit states the need, reviewed as a table edit). **Owner:** the citation-hygiene program, k2 trip-wire lane. Recorded 2026-10-18 (CR-B2a; the plan memo `…-cr-plan.md` §6 BS-2, R1 F20) |
| R14 | **the instant between a launch and its own record** (`_ctl_run`, `.claude/tools/webref-generic-core-trip-wire.ctlrun.sh`: each of its two process groups is written to the run's reap set by `_k2_reap_add` right after its fork). A HUP, INT or TERM that ends the run in the child's instant leaves the child unrecorded **and with no watchdog yet**, so the run's exit path does not stop it and it runs to its own end, unbounded; in the watchdog's instant the child is recorded and stopped, and the unrecorded watchdog sleeps out its seconds (30; 5 in the `_og` rows) and then sends its stop (TERM, then KILL) to `-$_cpid`, a group that is gone unless its id was reused meanwhile | a leftover; a wrong kill only through id reuse within the watchdog's seconds (BS-12's residue states the reap's the same way); never a changed status. The frame had the same class: its watchdog's `kill -9 -$_cpid`, left running on every signal end, since the frame reaped nothing; CR2-C2 narrows it to this instant and R15 (a) | **Gap:** the reap set cannot name a group in the instant before its record. **Why:** a group's id exists only after its fork, and a recorder inside the child would be the same instant moved. **Trigger:** a run stopped by a signal in that instant (after it, a leftover selftest child, or a `sleep` of up to 30 s). **Owner:** the citation-hygiene program, k2 trip-wire lane; where the leftover runs unbounded it is R9's class and subordinate to `#11-trip-wire-liveness-bound`, whose run-wide group would bound it. Recorded 2026-10-21 (CR2-C2; the plan memo `…-cr2-plan.md` §9 BS-9) |
| R15 | **the run's groups are stopped only when the run ends through its traps with TERM deliverable.** (a) A SIGKILL of the run, or a default-fatal signal it does not trap (QUIT, ALRM, USR1, …), reaps nothing, the watchdog included: it fires its stop at `-$_cpid` after its sleep (30 s; 5 s in the `_og` rows). (b) A run whose TERM was ignored at its entry passes the ignore to its children, so the stop's TERM does nothing and its KILL ends them 1–2 s later (1.73 s on bash 5.3, 1.49 s on 3.2 at CR2-C2's head, the synthetic probe's case M), and a selftest child so stopped leaves its scratch dir (the KILL-only stop leaves 1; claim row CR11-C) | leftovers; a wrong kill only through id reuse within the watchdog's seconds ((a): the unkilled watchdog's stop reaches another group if `-$_cpid` was reused after the child ended; the frame's watchdog `kill -9 -$_cpid` had the same class, on every signal end); the status is the signal's | **Gap:** no stop reaches the groups of a run that ends outside its traps, and a TERM the run inherited ignored cannot reach them. **Why:** SIGKILL cannot be trapped; the other untrapped signals are not what terminals, CI or `kill` send to stop a run; a signal ignored at entry cannot be undone. **Trigger:** a report of a leftover selftest child, `sleep` or scratch dir after such an end. **Owner:** the citation-hygiene program, k2 trip-wire lane. (a) is pre-existing (before CR2-C2 the run reaped nothing on any end) and subordinate to `#11-trip-wire-liveness-bound` (R9), whose run-wide group is what reaches a SIGKILLed run's groups; (b) is CR2's own (the stop's TERM). The leftovers hold none of the caller's descriptors (the watchdog `exec`s, and its launch closes fd 9). Recorded 2026-10-21 (CR2-C2; `…-cr2-plan.md` §9 BS-10) |

R7's command, together with a negative case that shows it discriminates:

```sh
sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml | /usr/bin/grep runs-on   # runs-on: ubuntu-latest
sed -n '/^  check:/,/^  [a-z]/p' .github/workflows/ci.yml | /usr/bin/grep runs-on        # runs-on: ${{ matrix.os }}
```

**R9's slot, `#11-trip-wire-liveness-bound`: the text §9's ledger step writes** is in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` §9.2 (moved there unchanged, beside the
ledger step that writes it).

**Pre-existing, closed by this PR: `#11-k2-wire-exit-trap-masks-set-u-abort`.** On `/bin/bash` 3.2,
`set -euo pipefail` together with the wire's EXIT trap (since base; the ledger trigger's "wire:405" is
the trap's line at base) turned an unbound-variable abort into exit 0. The orchestrating session
reproduced it (`…/scratchpad/r4ax2/u.sh`: 3.2 rc=0, 5.3 rc=1), and review R2 measured a real false green
from it (R2-3: a renamed label, a seal failure, 3.2 rc 0). `8f4ad3c8` closes it with the **orderly-end
flag** (design memo §3): `_K2_ORDERLY=1` is set once, just before the PASSED line, and the shared trap
body `_k2_exit` (the wire's trap and the mutation run's both call it) turns status 0 without the flag
into rc 1. The closure text, with its evidence and the create-time audit, is in `…-landing.md` §9.2;
§9's ledger step removes the entry citing it.

# K2 fixture git — provenance companion

This file is the provenance for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md`, called "the
design memo" below. It holds:
- the measurements behind that memo's decisions (§A);
- what became of the replaced branch (§B);
- the premises found false (§C);
- every plan-review round's dispositions (§D).

The design memo keeps only its live decisions. Section references here are to the design memo unless a
file is named.

⚠ This file follows the design memo's rule: every figure is a measurement at a named SHA or scratch
prototype, with its command. `$S` is scratch. `/usr/bin/grep` is spelled out on purpose.

---

## §0 Spec coverage map

**No spec surface** — this file records measurements and review dispositions only. It uses the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading. The repo-wide preflight command runs over every tracked plan memo, so this file needs the
heading like any other.

## §A Measurements

### §A.1 At the two heads (moved from draft 2 §2)

**How the runs were made.** Each run was:

```sh
( cd <checkout> && env <ASSIGNMENTS> <shell> .claude/tools/webref-generic-core-trip-wire.sh ) > log 2>&1
```

NE counts `CONTROL NOT EXERCISED` and CF counts `CONTROL FAILED`, both with `/usr/bin/grep -c`. Each
checkout was a `git clone --local`, checked with `git rev-parse HEAD`. Bash 5.3.20 and 3.2.57 agree
wherever both were run.

| caller environment | `ff6b99a3` | `e8f78896` |
|---|---|---|
| `HOME` + `.gitconfig` `core.excludesFile` = a file holding `*.py` | rc 0, NE 0 | — |
| `HOME` + empty `.config/git/ignore` (control) | rc 0, NE 0 | — |
| `HOME` + `.config/git/ignore` = `*.py` | rc 1, NE 12, CF 0 | rc 1, NE 11 (3.2) |
| `XDG_CONFIG_HOME` + `git/ignore` = `*.py` | rc 1, NE 11, CF 1 | — |
| `XDG_CONFIG_HOME` + `git/attributes` = `*.py working-tree-encoding=UTF-16LE` | rc 1, NE 17, CF 5 (3.2) | rc 1, NE 17, CF 4 (3.2) |
| `XDG_CONFIG_HOME` + `git/config` `init.templateDir` | rc 0 (3.2) | — |
| `GIT_TEMPLATE_DIR` + `info/exclude` = `*.py` | rc 0 (lane SSoT) | rc 1, NE 11 (3.2) |

**Why the HOME and XDG rows differ: 12 NE against 11 NE + 1 CF.** The cause is `ff6b99a3`'s `_envlive`,
which passes `"HOME=${HOME:-}"` into its own `env -i`. See it with:

```sh
git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -n 'HOME=\${HOME'   # line 814
```

- A hostile `HOME` contaminates the probe's baseline, so `envscrub` is reported NE.
- A hostile `XDG_CONFIG_HOME` is dropped by the probe but reaches `_fgit`, so `envscrub` is reported CF.

That the lane SSoT's "12" for XDG counted both kinds together is an **inference**: its log was not
recorded.

**Content and commit channels** (the §1 table). These ran under an otherwise empty git environment, on
git 2.55.0 and Apple 2.54.0:
- baseline: `victim.py` blob `7d4290a1…`;
- `$HOME/.config/git/attributes` `working-tree-encoding=UTF-16LE`: blob `b8b71b2a…`;
- a template `hooks/pre-commit` running `git rm -q --cached victim.py`: `victim.py` is absent from the
  index and from HEAD, and `commit` still exits 0.

### §A.2 Prototype A — `_fgit` from nothing (draft 1)

This is the base with §3.3 applied. It gave rc 0, NE 0, CF 0 and `PASSED` on both shells, under each of:
- a clean caller;
- `HOME/.gitconfig` setting `core.excludesFile`;
- `HOME/.config/git/ignore`;
- `XDG_CONFIG_HOME` pointing at an ignore file;
- `XDG_CONFIG_HOME` pointing at an attributes file;
- `GIT_TEMPLATE_DIR` pointing at an exclude;
- the `GIT_CONFIG_COUNT` trio;
- `GIT_CONFIG_SYSTEM=<hostile>`;
- `GIT_DIR=<other>`.

Without `mkdir -p .git/info` in `notcommitted`, a clean caller gives rc 1 with 1 NE (`per-clone
info/exclude cannot hide an entry`).

### §A.3 Prototype B — postconditions and pins (draft 2; extended in draft 3)

This was a standalone script, run on both shells and on both gits.

**F1 planting (template content and `include.path`):**
- A template holding `config` (`core.excludesFile`, `core.hooksPath`) and `HEAD` (`ref: refs/heads/hostile`)
  leaves `find .git | sort` **equal** to a void init, but `diff -r` shows a difference.
- The copied keys report `local` with origin `file:.git/config`.
- A local `include.path` reports origin `file:<outside>`.
- A clean probe passes P-a and P-d.

**Allowlist mutants against P-b/d/e (draft 2 §6.2):**
- drop `HOME`: P-b red;
- source-time `HOME=$HOME`: P-b only;
- call-time `HOME=$HOME`: P-b red;
- drop NOSYSTEM: P-b red (`/opt/homebrew/etc/gitconfig`);
- drop ATTR_NOSYSTEM: P-b red (`/opt/homebrew/etc/gitattributes`, printed although the file is absent);
- drop TEMPLATE: P-d red;
- drop PATH: P-e red.

**Added in draft 3.**

| mutant | P-a | P-c | P-d |
|---|---|---|---|
| env config appended to `_FGIT_ENV` | red (scope `command`) | ok | ok |
| `_FGIT_VOID` at a non-empty directory | clean | red | ok |

**Reftable:** two `init --ref-format=reftable` runs differ under `diff -r` (`.git/reftable/*.ref`,
`tables.list`).

**Draft 1's roster E7 was dead.** Draft 1 used `init.templateDir` via XDG config. Under the constructed
environment it reported `NOT-LIVE`, because `GIT_TEMPLATE_DIR` outranks it.

### §A.4 Prototype D — the whole-build differential (draft 3)

**Setup.** A `git clone --local` of `e8f78896`, with four changes:
- §3.3;
- the `notcommitted` fix;
- base controls lines 83–733 extracted to a sourced fixtures file (651 lines; controls 346);
- a differential block after `ctl_ok=0`.

It ran on both shells and gave rc 0 and `PASSED`, with 80 repos compared.

**`linkname`.** Before its `ln -s "$CTL/…"` became `ln -s ok.py`, it was the one differing repo, because
it was position-dependent.

**Mutants** (applied to a copy of prototype D):

| mutant | result |
|---|---|
| `command env` | red, all 80 differ |
| call-time `HOME` | red |
| a `cachedir` step as `_git add -A` | red, `cachedir` only |
| `PATH="$CTL/fakegit:$PATH" git add -A` | red, `cachedir` only |
| `/opt/homebrew/bin/git add -A` | red, `cachedir` only |
| `env -i PATH="$PATH" git add -A` | **no** differential red |

**Errexit.** A P reader whose final command failed made the run end with no verdict. That is why every
reader ends in `true`.

**Cost of the P reader.** Four git calls per repo cost clearly more than two git calls plus file reads.
Timings were taken from in-script `Time::HiRes` stamps on a loaded machine. They are too noisy to quote,
so re-measure if needed.

**Driver timing.** `/usr/bin/time -p bash scripts/trip-wires.sh` was run three times each at base and at
prototype D. Head was slower. Both sides were far inside the 5-minute budget, and the spread overlapped
the difference. X8 repeats this on the real head.

---

## §B Fate of `ff6b99a3`'s 14 commits (moved from draft 2 §9.1)

| commit | subject | fate |
|---|---|---|
| `c8f52724` | scrub the whole `GIT_*` environment | mechanism superseded; target plumbing → C4 |
| `4ad051ea` | one target list and a resolver | → C4 |
| `c5820bd5` | comment syntax for records | → C3 |
| `a1007bfb` | keep-set grounded in config layers | superseded (premise false: a template seeds the local layer) |
| `162edf98` | prove the scrub against `env` | superseded |
| `328dc22a` | measure the disjointness | superseded |
| `c39c7989` | bound the unknown-prefix claim | → C4 |
| `f8c74c6d` | two comments | harness half superseded; mutations half → C4 |
| `6b4064fa` | `env -0` per record | superseded |
| `009e93db` | liveness before asserting | principle survives in §6.3; code superseded (it leaked `HOME`) |
| `4aced879` | restore/resolve order | → C4 |
| `2dea82ff` | retire the hostile-build count | superseded |
| `6ffe2d5d` | one spelling for pattern and target | principle survives |
| `ff6b99a3` | order-measurement note | → C4 |

**Review coverage for the code carried forward.** `ff6b99a3`'s Stage 5 reported 15 IMP / 8 MIN. The
lane SSoT does **not** list the 15 individually. What it records (`project_citation-hygiene-program.md`,
the "(2026-09-27)" block, around its lines 924–941) is the XDG hole plus three co-occurring items, all
measured:
- no control pinned the keep-set;
- the 1000-line split was not taken;
- two carve slots had false premises.

⚠ Draft 2 said "not itemised", which was too strong. All four recorded items are addressed:
- the XDG hole, by §3;
- the pins, by §6.2;
- the split, by §8;
- the slots, by §7.

The remaining, unrecorded findings cannot be recovered. C3/C4 are therefore reviewed fresh by the
rebuild's `/pre-push` Stage 4 over the whole range.

---

## §C Premises found false

**From the original brief (draft 1):**
1. "The paths `git add -A` stages" is too narrow.
2. "Nulling global config opens the channel" overstates it: base is already red.
3. The XDG "12" is 11 NE + 1 CF.
4. `_envlive`'s 5 lines call git through `env`.
5. `ff6b99a3`'s claim that the local layer "belongs to the repo `_fgit` creates" is false.
6. The `env -i` shape changed four ways (§3.2).
7. Base `_fgit` already nulled the global and system layers.

**From round 1 (draft 2):**

8. F1 was true, and refined: origin checks cannot see template-copied config.
9. F10 was true.
10. Draft 1's roster E7 was dead.
11. F7's "shadow `_git`" was unnecessary.
12. Three misattributions: §8.1's rule, the "§10.x" row (§10.6 R7), and the man-page section.

**From round 2 (draft 3):**

13. Draft 2's claim that "no in-process mechanism intercepts an absolute exec" was false. A whole-build
    output differential catches it (§A.4).
14. Draft 2's `PATH`-shim guard missed eight non-absolute spellings. That was measured by Ax3, and the
    realistic case is `PATH="$CTL/fakegit:$PATH" git`, which is the fixtures' own shim idiom. **Not
    re-measured here**: the guard is withdrawn, so no fix depends on it.
15. Draft 2 said C3 was required. It was not (§0.2).
16. Draft 2 said the parent need not be touched because the touch was "avoidable". CLAUDE.md contains no
    such rule (§8.3).
17. Draft 2 attributed a finding to "#501 R97". It was #519's Codex R7 (parent §10.6).
18. Draft 2 dropped the qualifier on `wire`. `wire:` parses as sed `w`, so `wire` is the default target
    and is never stripped.
19. Draft 2 said §3.3 was the dissolved slot's "structural closure". The slot named `_git_exec`.
20. Draft 2 said the ledger step cited a "landing SHA". That SHA is unreachable from `main` after #501's
    squash.

---

## §D Plan-review dispositions

### §D.0 Terminator for round 3

**Round 3 converges if no IMP lands on draft-3-added text and no IMP moves a mechanism.** An IMP of
either kind is a reset. If that reset is self-introduced, the next draft shrinks mechanism again rather
than adding to it. Round 2 was the first self-introduced reset: 8 of its 10 IMP landed on draft-2 text.

### §D.1 Round 1 (on draft 1 + §2.5; 0 CRIT / 13 IMP, 11 unique / 25 MIN / 14 FP) → draft 2 `0142f47a`

| F | disposition in draft 2 | fate in draft 3 |
|---|---|---|
| F1 | P-d by content, P-a by origin | kept |
| F2 | umbrella slice and memo rows | kept, pointers corrected (D4) |
| F3 | marker spelling; memo-row "by design"; ENVIRONMENT VARIABLES | kept |
| F4 | sites enumerated; parent not edited | **reversed** by D4 |
| F5 | ci.yml not edited; STOP rule | **reversed** by D1's cost clause: re-derive per the in-file rule |
| F6 | criterion change stated | kept; boundary added (D3) |
| F7 | `PATH` shim + absolute-path slot | **withdrawn** (D1) |
| F8 | ratchet widened to `_lbl` | kept; records for every label (D2) |
| F9 | separate labels for §6.3/§6.4 | superseded: one differential, one label, one record |
| F10 | source-time snapshot; measured predictions | kept |
| F11 | one parts list, three readers | kept; guard placement fixed |

### §D.2 Round 2 (on draft 2 `0142f47a`; 0 CRIT / 10 IMP / 19 MIN / 17 FP) → draft 3

| D | disposition | verifying command / evidence |
|---|---|---|
| D1 | the guard, canary, marker, absolute-path slot, `envscrub` and 12 probes are replaced by **one** whole-build differential. Three universal members, each proven live alone before the hostile build. Verdict order after `ctl_ok=0`. Errexit specified. Refs are in P. Cost re-derived by the ci.yml rule, with STOP on change | prototype D (§A.4): unmutated green; `command env`, call-time `HOME`, `_git`, `PATH=… git` and absolute path all red; `env -i git` green, booked. `git show e8f78896:.github/workflows/ci.yml \| sed -n '/^  trip-wires:/,/^  [a-z]/p'` for the rule. Driver timing, three runs per side |
| D2 | six labels, six records; `_MUT_UNRECORDED_MAX` unchanged; X11 | P-a/P-c records measured (§A.3). Existing `_lbl` records: `/usr/bin/grep -cF` → 1 each |
| D3 | §0.1 boundary: P governs git's inputs, not executable or platform (R1, R2, R5). R3 booked as `#11-k2-fgit-machine-files`, which is also R3(b). Own deferrals: **1** | design memo §0.1, §5 |
| D4 | parent touched: C0a (move §5 and §10 to a rounds companion, stub headings) + C0b (banner), under CLAUDE.md's quoted text. Umbrella pointers name files and sections exactly. The code-reference convention is stated; references are qualified in edited files | `/usr/bin/grep -n '^## '` over the parent (section map); the 17-line reference grep |
| D5 | this companion | `wc -l` of both files |
| D6 | C3's "required" claim withdrawn and re-grounded. Full clause answered, with Q1 | design memo §0.2 |

**MINs:**
- §6.4's "already live": a false ordering, fixed by §6.3's liveness order.
- The union shadowing: handled by proving each member live alone.
- Hostile files are written before the build.
- The dedicated E8 repo: moot, since there are no per-channel probes.
- The guard marker reset: moot.
- C1 states the fixtures file is sourced.
- R8 now names P-d's reftable failure.
- Refs are in P.
- The non-git commands are named as the class R4 plus §10 row 15.
- P is narrowed to what §6.3 compares.
- C6 is withdrawn.
- The Stage-5 coverage claim is corrected (§B).
- §8.1's attribution is fixed.
- The test-suite claim is removed.
- "Recipe" is narrowed to ingredients.
- The GNU sed claim is stated as unverified.
- The slot quotes are verbatim.
- The SSoT "12" is marked as inference.
- R1 is stated as this slice's own residual.
- The ledger step and the landing SHA are corrected.
- The land order gains user approvals, the #501 P2 thread, the status-cell update, and the #501
  decision.
- The sibling guard lives only in `_mut_run`.
- P-c runs last.
- `#501 R97` is corrected to `#519 R7`.
- The `wire` qualifier is restored.
- The §7 closure claim is corrected.
- The umbrella pointers are fixed.

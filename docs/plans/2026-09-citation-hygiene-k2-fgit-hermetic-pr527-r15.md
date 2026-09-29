# K2 fixture git — PR #527's review record, from plan-review round 15

This file continues `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md` ("`…-pr527.md`"),
PR #527's record, for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo").
`…-pr527.md` holds the record through plan-review round 14 and draft 16, and it is closed: at 890 lines
at `6c5750ce` (`git show 6c5750ce:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md | wc -l`)
it had no room for round 15's record or for the implementation results, so both start here. The seam is
chronological. Round 14 and earlier stay where they are, so every existing reference to "`…-pr527.md`,
round N" (N ≤ 14) still resolves, and the scripts recorded there verbatim (`cell15.sh`, `crc.sh`,
`ptycell.sh`, `pty3.py`, `pkmeas.sh`) are cited there, not copied. The references follow
`…-pr527.md`'s convention: to the design memo as of the draft the paragraph belongs to, unless a file
is named.

---

## §0 Spec coverage map

**No spec surface** — this file records review dispositions and measurements only. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §Q Plan-review round 15 onward, and the implementation results

Every result from plan-review round 15 on goes here, the planned commits included when they are
made (`…-landing.md` §9.1, step 2).

**Plan-review round 15 (focused, on draft 16, `ebc90bc1` and `6c5750ce`) → draft 17.** 0 CRIT / 2 IMP /
9 MIN, as the orchestrating session relayed them. It decided the dispositions; draft 17 implements them.
The labels D15-A…D15-C follow the design memo's collision rule (a `D…` family, no section number).

| finding | disposition |
|---|---|
| IMP-A (Ax3): the census's working files sit at fixed names in `$_FW_DIR`, which the fixtures can write. `lnblind2` (a link `$CTL/zzy` to a tree holding a git dir, with `prel` and `pgls` made links to `/dev/null`) gave rc 0, PASSED on both shells; without the two `/dev/null` links, rc 1, `K2PRE zzy:[a symlink to a tree holding a git dir]`. The memo's "a fixture-placed file there … cannot make a comparison equal" was false | **D15-A**, by construction: every file a postcondition writes and reads back is in one directory made after the build, with the references (design memo §4, "The references and the working files"; §0.3 by property). Prototype changed, cells below; a C9 record (corpus §6.1) whose survive check is "the working files back in `$_FW_DIR`"; X16 gains the cell |
| Ax2 MIN: `xrace`, a background process the fixtures leave running, writes the reference made after the build (`pgref*/a/.git/config`): rc 0, PASSED on both shells | **D15-C**: class (c), a write to the verifier's own state (D14-D's class). §4's "whose name no fixture can know" narrowed to "no fixture command that finished by the end of the build"; `xrace` listed beside `xdone`/`xredef` in §0.3 and §5.1; corpus §6.1's "`mkdir` without `-p` is equivalent" now says "within class (b)" |
| IMP-B (Ax5): #510's head is `106387e4`; its budget block keeps one figure, the job on the runner, no scaling, and the 4.3× source is gone. X8's method (three local runs, slowest × 4.3 against 150 s) is the scaling #510 retired | **D15-B**: X8 becomes a sanity record; the verdict is written from a runner run of the job (X9's, route (a) or (b)), by the citation-hygiene lane; #527's comment says the verdict is pending, with no figure; if #510 lands first its convention governs and the later lander re-measures on the runner (`…-landing.md` §9, §9.1, §11 X8/X9; design memo §3, §10 #11) |
| Ax2 MIN: "the verifier's own state" undefined; shell options fit both classes | defined by property (prelude names, files the parent or the postconditions read back, `$_FGIT_BIN`); the shell options are class (b), the state W's re-check reads (design memo §0.3) |
| Ax3 MIN: T omitted the budget side; a docs-only fix to a check's definition was not re-run | T includes the `trip-wires` job outside its comments, with a command; a docs-only fix to a check's definition re-runs that check at T in its own commit, T unmoved (`…-landing.md` §9.1) |
| Ax3 MIN: D14-A leftovers "X13 re-runs it at the final head" | "at T" in the design memo's §3 and in §5.2's ledger text |
| Ax3 MIN: "a read that fails fails that listing" over-claims | two routes, measured below: a mode-000 include fails the listing (rc 128); a nonexistent include is skipped silently and P-g reds the `include.path` line (design memo §4, "The boundary") |
| Ax4 MIN: "tool code as at `8413a4db`" for `ebc90bc1` | "except three comment-only path lines" (`git diff --stat 8413a4db ebc90bc1 -- .claude/tools scripts`: fixtures, mutations and mutgen, one line each), in `…-pr527.md` round 14 and the design memo's §0.3, which also names each boundary cell's tree |
| Ax4 MIN: the design memo's header called `…-reviews.md` "rounds 1–9 … split out unchanged" | it names §D, §S (revised since), §8 and §13 |
| Ax5 MIN: `…-pr527.md` at 890 lines with round 15 routed to it | this file, split `f218e59e` (design memo §13, `…-landing.md` §9.1 step 2) |
| §12 | round 16, a focused re-check of the draft-17 delta |

**The draft-17 prototype.** Base `35dc1153`, with round 14's `proto16.diff` applied (sha256
`34f1be51497a16f876865722fe72b6c2f03cca467cf7637f84a7980c358240ef`), then:
- `_fgit_postconditions` makes the directory first, `_pq="$(mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX")"`,
  red (`K2REF`) if that fails, and every working file is written under `$_pq`; after a clean census it
  makes `a` and `b` there by `mkdir` without `-p`, as draft 16 did;
- the two records anchored on `$_FW_DIR/env0` and `$_FW_DIR/pgcur` name `$_pq/env0` and `$_pq/pgcur`
  (the clean runs below pass the always-on anchor check).

`git -C <sandbox> diff > proto17.diff` gives 255 lines, sha256
`7d3c4048d0fb278aa2df92ca27656e9d13cc9ae7cd2e3d0d41744647b0d3e3ba`, in the orchestrating session's
scratchpad (`…/scratchpad/author17/proto17.diff`).

**The working-file set, derived by property.** On the draft-16 prototype, every path the
postconditions name under `$_FW_DIR`:

```sh
sed -n '/^_fgit_postconditions() {/,/^}/p' .claude/tools/webref-generic-core-trip-wire.harness.sh \
  | /usr/bin/grep -oE '"\$_FW_DIR/[A-Za-z0-9_.]+|\$2\.[a-z]+' | sort -u
```

gives `env0 env0.err machine_limits pgcur pgcur.z pgls pgpop pgpop.err pgpop.failed pre pre.err prel
prels` and `_pg_z`'s `$2.raw`, `$2.rec`. Each but `machine_limits` is written and then read back by
the postconditions; `machine_limits` is read by the parent (harness line 582 there), so it is return
channel and stays. No other part of the wire reads the moved names except the two records re-anchored
above (an `awk` over every `webref-generic-core-trip-wire*.sh` outside the function, for `/pre`,
`/prel`, `/prels`, `/env0`, `/pgpop`, `/pgls`, `/pgcur`, `/pgref`). On the draft-17 prototype the same
`sed | grep` gives `machine_limits`, `$2.raw` and `$2.rec`, and every `_pg_z` call there passes a `$2`
under `$_pq`.

**The cells** (2026-09-30, `cell15.sh` from `…-pr527.md` round 13 verbatim, two wires at a time,
`HOME` a scratch directory). Inserts: `clean` empty; `xrefpoison` and `xlink` as in `…-pr527.md`
rounds 14 and 13; `lnblind2` and `xrace` the reviewers', verbatim:
- `lnblind2`: `mkdir -p "$_FW_DIR/zzt/a/objects" && : > "$_FW_DIR/zzt/a/HEAD" && ln -s "$_FW_DIR/zzt" "$CTL/zzy" && ln -s /dev/null "$_FW_DIR/prel" && ln -s /dev/null "$_FW_DIR/pgls"`
- `xrace`: `xrefctl`'s insert (`…-pr527.md` round 14), then
  `( _zi=0; while [ "$_zi" -lt 6000 ]; do _zi=$((_zi+1)); for _zr in "${_FW_DIR%/*}"/pgref*/a/.git "${_FW_DIR%/*}"/pgref*/b/.git; do if [ -f "$_zr/config" ] && ! grep -q nonexistent-k2 "$_zr/config"; then git config --file "$_zr/config" core.excludesFile /nonexistent-k2 || :; fi; done; sleep 0.005; done ) > /dev/null 2>&1 &`

| cell | tree | bash 5.3 | bash 3.2 |
|---|---|---|---|
| lnblind2 | draft 16 (the control) | rc 0, PASSED, 19 s | rc 0, PASSED, 26 s |
| clean | draft 17 | rc 0, PASSED, 18 s | rc 0, PASSED, 24 s |
| lnblind2 | draft 17 | rc 1, 3 s, `K2PRE zzy:[a symlink to a tree holding a git dir]`; no control run | rc 1, 5 s, same |
| xrefpoison | draft 17 | rc 1, 6 s, P-g: every fixture repo's `core.excludesfile=/nonexistent-k2` | rc 1, 8 s, same |
| xlink | draft 17 | rc 1, 4 s, `K2PRE zzl:[a symlink to a tree holding a git dir]` | rc 1, 5 s, same |
| xrace | draft 17 | ⚠ rc 0, PASSED, 23 s; the window's bash and a `sleep` left in the group, killed by group | ⚠ rc 0, PASSED, 31 s, same |

`xrace` is class (c) as declared (D15-C). The same writes without the background process, `xrefctl`,
are red on the draft-16 prototype (`…-pr527.md` round 14), so its PASSED comes from the write to the reference.

**"A read that fails", measured** (git 2.55.0, `HOME` a scratch directory, `GIT_CONFIG_NOSYSTEM=1`):

```sh
git init -q r1 && printf '[include]\n\tpath = /nonexistent-k2\n' >> r1/.git/config
git -C r1 config --list --show-origin      # rc 0, empty stderr; last line: file:.git/config<TAB>include.path=/nonexistent-k2
git init -q r2 && printf '[core]\n\tx = 1\n' > f0 && chmod 000 f0 && printf '[include]\n\tpath = ../../f0\n' >> r2/.git/config
git -C r2 config --list --show-origin      # rc 128: fatal: unable to access '.git/../../f0': Permission denied
```

**#510 at the time of D15-B.** `gh pr view 510 --json headRefOid` gave `106387e4`; its `ci.yml`, read
with `gh api 'repos/send/elidex/contents/.github/workflows/ci.yml?ref=106387e4' --jq .content | base64
-d`, carries the budget block quoted in `…-landing.md` §9, `timeout-minutes: 10` on the `trip-wires`
job, and no `4.3` and no `K2` or `fixture` (`/usr/bin/grep -c` → 0 for each).

**Records planned by draft 17** (`…-landing.md` §9.1, corpus §6.1):
- −1 (P-j per-entry);
- +2 (W2: `post_bad`, W3), with W2's existing record re-anchored;
- +8 (P-g: `objects` link, FIFO `commondir`, scan status, `xcommon`, hard link, `xlink`, `xrefpoison`,
  `lnblind2`), with the P-f and P-g records on `env0` and `pgcur` re-anchored;
- +4 (P-k: alternates, liveness, status, stderr).

That is **+13**: 63 records of this PR, **158** in all, `_MUT_RECORDS_MIN=158`, labels 18,
`_MUT_UNRECORDED_MAX=21`. Draft 16's plan (157) is superseded. None of it is implemented yet.

**Round 15's `lnblind` insert**, cited by corpus §6.1's `lnblind2` row, verbatim from the round-15
reviewer's file (`/tmp/elidex-plan-review.7e5b6f5f-cec2-4f20-9e7a-b25f0c4752cf/r15/ax3/ins/lnblind`):
`mkdir -p "$_FW_DIR/zzt/a/objects" && : > "$_FW_DIR/zzt/a/HEAD" && ln -s "$_FW_DIR/zzt" "$CTL/zzy" && ln -s /dev/null "$_FW_DIR/prel"`.
Its one recorded run (the reviewer's `o.lnblind`, bash 5.3 on the draft-16 prototype): rc 1, 6 s,
`zzy:[a symlink to a tree holding a git dir: a/HEAD]`, the in-loop search's message.

**Plan-review round 16 (focused, on draft 17, frame `0a11ace9`) → draft 18.** 0 CRIT / 3 IMP / 12 MIN,
as the orchestrating session relayed them. It decided the IMP dispositions; for IMP-3 it and Fable's
independent verdict agreed on option 1, and the user had authorised proceeding on agreement. Labels
D16-A…D16-D (a `D…` family, no section number).

| finding | disposition |
|---|---|
| IMP-1/2 (Ax3): X9 had no place in the land order, and route (a)'s branch conflicted with the "pending" sentence | **D16-A**: route (b) is the user's decision (PR #527's description), stated in `…-landing.md` §9; route (a) removed; land order step 7 is X9 on #501: the runner run, the verdict commit on #501's branch, the threshold, a post-squash fix's owed checks, #501's fresh Codex round, and #510's conflict resolved before the re-measure |
| IMP-3 (Ax2): the parent's working files at fixed names under `$CTL`; `xbare` and `xanchor` PASSED | **D16-B**, option 1, by construction: C11 moves them into `$SCRATCH/verifyXXXXXX` made after the window; §0.3's design rule; design memo §4, "The parent's directory", with the checker; X16 gains the cells; X3 an opt-in cell; §10 #2 |
| Ax3 MIN: `mkfn`, a fixtures-file `mktemp` function | **D16-C**: construction cannot close it in the shell (below), so it is class (c), declared for every command the window runs after the fixtures file; §4's sentence narrowed |
| Ax2 MIN: `xexecarith`, fd 2 redirected before a diagnostic, PASSED | **D16-D**: a check, C12 (a canary through fd 2, read in the parent), measured below; one W record |
| Ax3 MIN: the docs-only branch had no red path | a red result is a finding; X9's row goes to X9's route; X8 cannot fail (`…-landing.md` §9.1) |
| Ax3 + Ax5 MIN: step 2's `ci.yml` target | every local-derivation passage named with its lines at `8413a4db`; no figure in `ci.yml` at all (#510 at `9df03f52` keeps none either) |
| Ax3 MIN: "half" against "nears 2x" | the line is this memo's choice, and why (`…-landing.md` §9, "The threshold") |
| Ax2 MIN: the P-g row and §4's heading still said "no fixture can reach" | narrowed to the §0.3 rule |
| Ax4+5 MINs: X14's record place; the reviews header and §S; corpus line 363 → 364; `xrace` "on the draft-16 prototype" cited to `…-pr527.md`; the unrecorded `lnblind` insert; "at 890 lines"; the "Cost" bullet | each fixed (X14 → this file; header and §S; `mutations.sh` at `8413a4db`, lines 363–364, has the `pgcur` record at 364; `xrace` cited to round 15's relayed cell; `lnblind` above; the `git show` and `wc -l` command in this file's header; "X8 records it locally; the verdict is X9's") |
| §12 | round 17, a focused re-check of the draft-18 delta |

**False premises in round 16's brief, found while implementing** (each measured below):
1. The derivation command `/usr/bin/grep -nE '(>|>>) *"\$CTL/|mkfifo "\$CTL'` "at `8413a4db`: 12 sites … after the move returns 0": it lists 14 lines at `8413a4db` (13 working-file lines and one mutation record, `mutations.sh:377`, whose `mkfifo "$CTL\/zzfifo` its unslashed `mkfifo "\$CTL` matches), and it misses the two assignment sites, `_fsm_mark="$CTL/.fsmonitor_ran"` and `_out_f="$CTL/.control_out"`. After the move it lists that record, 1. The design memo's checker adds the assignment form and the slash; it lists 16 at `8413a4db` and 1 after C11 (a fixture read, `PATH="$CTL/fakerelmktemp:$PATH"`).
2. "`command mktemp` / an absolute path resolved at source time" as a construction: a function shadows `command`, `builtin`, `unset` and a name spelled `/usr/bin/mktemp` alike.
3. `.fsmonitor_ran`, "pre-plant → NE→green": the pre-planted mark does not cause the NOT EXERCISED; it hides it (the plain-git check reads the plant as the hook's run).
4. #510's head is `9df03f52` now, not `106387e4`, and its budget block keeps no figure at all.

**Shadowing, measured** (bash 5.3 and 3.2, the same output on each):

```sh
$SH -c 'set -euo pipefail; builtin() { echo shadowed-builtin; }; unset() { echo shadowed-unset; }; command() { echo shadowed-command; }; f(){ :; }; unset -f f; builtin unset -f f; command unset -f f; declare -F | grep -c " f$"'
# shadowed-unset / shadowed-builtin / shadowed-command / 1   (f is still defined)
$SH -c '/usr/bin/mktemp() { echo shadowed-abs; }; /usr/bin/mktemp -d /tmp/xXXXXXX'   # shadowed-abs
```

A `[ /dev/fd/2 -ef "$_FW_DIR/stderr" ]` check was tried first for D16-D and does not work on macOS:
`stat -f "%d %i"` of `/dev/fd/2` gives the file's inode on the `fdesc` device (`193377003 208981939`
against the file's `16777231 208981939`), so `-ef` is false even when fd 2 is the file. Hence the canary.

**The draft-18 prototype.** Base `35dc1153`, the whole prototype as one diff (it contains
`proto17.diff`'s changes): `git -C <sandbox> diff > proto18.diff`, 532 lines, sha256
`df8e3fe22c5ed37bc1f8c9f229e236433bafa4d4e34ac2bb8b3ab746bbbb33a7` (`…/scratchpad/author17/proto18.diff`).
On top of draft 17's:
- controls, right after `_fgit_window "$_FIXTURES"`: `_VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")"`,
  checked, exit 2 with "could not create the verifier's directory … decided nothing";
- every `"$CTL/<name>"` of the moved set written as `"$_VFY/<name>"` in the four parent parts;
  `.fifoprobe` made in `$SCRATCH` (source time, before `_VFY` exists);
- the child, after the options re-check, `printf "K2-WINDOW-STDERR-CANARY\n" >&2`; the parent tests
  for it with `awk` `index()` over `stderr`, replays `stderr` with the canary taken out, and without it
  sets `_fw_why` to "the fixtures file left the window's stderr redirected, so a shell diagnostic could
  not be read" and does not count the window done.

The tree variants for X16's round-16 cells, each the draft-18 prototype plus one edit in `mutations.sh`:
- `U` (for `xbare`): `_MUT_UNRECORDED_MAX=21` → `_MUT_UNRECORDED_MAX=0`;
- `A` (for `xanchor`): the record `fixtures:s/^: > "[$]_FW_DIR\/built"$/git -C "$CTL\/clean" config --add core.bare false; …` has its anchor `built"$` changed to `builtZZSTALE"$`.

**The moved set, derived.** The checker (design memo §4) over the four parent parts: 16 lines on the
draft-16 prototype, the same set as at `8413a4db`:
`controls:187 PATH="$CTL/fakerelmktemp:$PATH"`, `controls:332 _fsm_mark="$CTL/.fsmonitor_ran"`,
`controls:333 > "$CTL/fsmhook"`, `controls:345 > "$CTL/.fsm_out"`, `controls:373 > "$CTL/.umask_out"`,
`harness:635 mkfifo "$CTL/.fifoprobe"`, `harness:692 _out_f="$CTL/.control_out"`, `mutations:507` and
`:645 > "$CTL/.mutants"`, `mutations:551 > "$CTL/.anchor"`, `mutations:588 > "$CTL/.bare"`,
`mutgen:272`, `:283 "$CTL/.genmutants"`, `mutgen:292 "$CTL/.genequiv"`, `mutgen:293`, `:297
"$CTL/.genseen"`. Every one but the first is a file the parent writes and reads back (or, for
`.fifoprobe`, makes and removes); the first is a fixture directory it reads. On the draft-18 prototype
the checker lists `controls:196 PATH="$CTL/fakerelmktemp:$PATH"` alone.

**The cells** (2026-09-30, `cell15.sh` verbatim from `…-pr527.md` round 13, two wires at a time, `HOME`
a scratch directory). Inserts verbatim: `xbare` `ln -s /dev/null "$CTL/.bare"`; `xanchor` `ln -s
/dev/null "$CTL/.anchor"`; `xexecarith` `exec 2>/dev/null` then `: $((1/0))` on the next line; `xexec`
`exec 2>/dev/null`; `xarith` `: $((1/0))`; `xnonl` `printf 'nonl' >&2`; `mkfn` and `mkctl` the round-16
reviewer's (`/tmp/elidex-plan-review.7e5b6f5f-cec2-4f20-9e7a-b25f0c4752cf/r16/ax3/ins/`): `mkctl` is
`lnblind2`'s `zzy` link plus `_k2d="${_FW_DIR%/*}/pgrefKNOWN"; mkdir "$_k2d" && ln -s /dev/null
"$_k2d/prel" && ln -s /dev/null "$_k2d/pgls"`, and `mkfn` adds `mktemp() { echo "${_FW_DIR%/*}/pgrefKNOWN"; }`.

| cell | tree | bash 5.3 | bash 3.2 |
|---|---|---|---|
| clean | draft 18 | rc 0, PASSED, 20 s | rc 0, PASSED, 26 s |
| xbare | draft 18, `U` | rc 1, 16 s, `21 labels have no mutation record, against a ratchet of 0.` | rc 1, 22 s, same |
| xanchor | draft 18, `A` | rc 1, 15 s, `mutation record "…builtZZSTALE…" no longer matches …fixtures.sh: its anchor is stale` | rc 1, 21 s, same |
| lnblind2 | draft 18 | rc 1, 3 s, `K2PRE zzy:[…]` | rc 1, 4 s, same |
| xrefpoison | draft 18 | rc 1, 5 s, P-g | rc 1, 8 s, same |
| mkctl | draft 18 | rc 1, 3 s, `K2PRE zzy:[…]` | rc 1, 4 s, same |
| mkfn | draft 18 | ⚠ rc 0, PASSED, 16 s (class (c), D16-C) | ⚠ rc 0, PASSED, 23 s |
| xexecarith | draft 18 | rc 2, 6 s, W: "the fixtures file left the window's stderr redirected, …" | rc 2, 8 s, same |
| xexec | draft 18 | rc 2, 5 s, the same W | — |
| xexec | draft 16 (no canary) | — | rc 0, PASSED, 24 s |
| xarith | draft 18 | rc 1, 6 s, W3 (`./fixtures.sh: line 694: 1/0: division by 0`) | — |
| xnonl | draft 18 | — | rc 0, PASSED, 23 s; the log holds `nonl` and no canary |
| xrace | draft 18 | ⚠ rc 0, PASSED, 21 s; `sleep` and the window's bash left in the group, killed by group | ⚠ rc 0, PASSED, 29 s, same |

On the draft-17 prototype the round-16 reviewers measured `xbare` and `xanchor` (on their `U` and `A`
variants) and `xexecarith` rc 0, PASSED on both shells, and `mkfn` rc 0, PASSED on bash 5.3, `mkctl`
rc 1 (as relayed).

**Records planned by draft 18** (`…-landing.md` §9.1, corpus §6.1): draft 17's +13, and +1 (W:
`exec 2>/dev/null`, C12). That is **+14**: 64 records of this PR, **159** in all,
`_MUT_RECORDS_MIN=159`, labels 18, `_MUT_UNRECORDED_MAX=21`. C11 adds none. Draft 17's plan (158) is
superseded. None of it is implemented yet.

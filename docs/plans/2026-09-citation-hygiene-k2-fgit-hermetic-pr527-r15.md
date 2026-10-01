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
1. The derivation command `/usr/bin/grep -nE '(>|>>) *"\$CTL/|mkfifo "\$CTL'` "at `8413a4db`: 12 sites … after the move returns 0": it lists 14 lines at `8413a4db` (13 working-file lines and one mutation record, `mutations.sh:378`, whose `mkfifo "$CTL\/zzfifo` its unslashed `mkfifo "\$CTL` matches), and it misses the two assignment sites, `_fsm_mark="$CTL/.fsmonitor_ran"` and `_out_f="$CTL/.control_out"`. After the move it lists that record, 1. The design memo's checker adds the assignment form and the slash; it lists 16 at `8413a4db` and 1 after C11 (a fixture read, `PATH="$CTL/fakerelmktemp:$PATH"`).
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

**Plan-review round 17 (focused, on draft 18, frame `f50054b1`) → draft 19.** 0 CRIT / 4 IMP / 12 MIN,
as the orchestrating session relayed them. It decided the dispositions; draft 19 implements them.
Labels D17-A…D17-D (a `D…` family, no section number).

| finding | disposition |
|---|---|
| IMP-1 (Ax3): D16-C named only functions. `shimafter` (a `PATH` entry after the pin naming a directory whose `mktemp` echoes `pgrefKNOWN`) gave rc 0, PASSED on both shells, `shimhash` (`hash -p`) rc 0, PASSED on bash 5.3; `shimprepend` PASSED on the draft-18 prototype, whose P-j lacked the first-entry clause, and was red (`first:[…/zzshim]`) with the clause added; the control `shimnone` red | **D17-A**: D16-C stated by property, anything that changes what a command name the verifier runs after the fixtures file resolves to, with examples, not a list (design memo §0.3); the prepend form is red by P-j's first-entry clause, which C6 keeps; `shimafter` and `shimhash` in §0.3's measured boundary, `…-residuals.md` §5.1 and X16; §4's `mktemp` sentence aligned. The draft-18 prototype's P-j had lost the first-entry clause (the harness at `8413a4db` has it, `harness.sh` line 257); the draft-19 prototype restores it, and C6's re-anchor record pins it (corpus §6.1, measured below) |
| Ax3 MIN (with IMP-1): "So no in-shell construction closes it" was false: `POSIXLY_CORRECT=1` with `$(exec /usr/bin/mktemp -d …)` runs the real binary despite the functions | "every in-shell construction is defeated by one fixtures-file line" (`enable -n exec` with an `exec` function, measured below), so a deliberate defeat is class (c); no reset is adopted, and why (design memo §0.3) |
| IMP-2 (Ax3): after a red or over-threshold X9, a fix that moves T left no step that re-runs X9 at the new T | **D17-B**: land order step 7.4 returns to 7.1 on a fix that moves T; step 8 is reached only when X9's run at the current T is SUCCESS and under the threshold, or, over the threshold, the user has decided at 7.3 for that T (wording aligned in draft 20, D18-C); terminator: Codex dry on the verdict commit (`…-landing.md` §9) |
| IMP-3 (Ax5): #501's branch changes `CLAUDE.md` line 94 to carry a local derivation method | **D17-C**: step 2's targets gain line 94, fixed in #501's branch by #527; not reverted to main's text, which carries "(~1s)" (`…-landing.md` §9, "CLAUDE.md's trip-wires paragraph"; false premise 1 below) |
| IMP-4 (Ax5+Ax3): X3's opt-in cell expected "red", wrong after C11 | **D17-D**: X3 states base PASSED with `0 mutant(s)` (the blind) and, after C11, PASSED with the normal non-zero count, both "predicted from the code; X3 measures it", in `…-landing.md` §11 and the design memo §4; and C11 makes an empty generated set red in `_mut_gen_run`, pinned by X3's floor cell (no record can reach it: a trial runs with `env -u WEBREF_WIRE_MUTANTS`) |
| Ax2 MIN: the entry guard and the two "WHAT IT CONSUMES" headers still name `$CTL` | C11's row replaces them with `$_VFY` (`…-landing.md` §9.1; design memo §4); the draft-19 prototype does, and its clean runs pass the guard |
| Ax2 MIN: a stderr redirect scoped to a group, `xgroup` | a class-(c) example in the design memo §0.3 and in its measured boundary |
| Ax2 MIN (optional): a cause sentence for `exec 2>&-` | not adopted: the verdict is already W alone (the window exits before completing); a cause sentence would change the message, not the verdict |
| Ax4/Ax5 MINs | the D16-A source split (the user's choice in the lane's memory of PR creation, the route in PR #527's description); "Drafts 11–17" → "11–18" (`…-landing.md` ledger text, X5; design memo §6); `pull/501/head` under "Resolvable commits"; the design memo §10 #2 refers to §0.3's rule and says the parent keeps none of its working files under `$CTL`; this file's round-16 false premise 1, `mutations.sh:377` → 378 (`git show 8413a4db:.claude/tools/webref-generic-core-trip-wire.mutations.sh | /usr/bin/grep -n 'mkfifo "\$CTL'` → 378); §9.2's lead-in names "code sourced after wire:405"; `ci.yml`'s "RE-DERIVED WHEN THE K2 WIRE LANDED" (line 167 at `8413a4db`) joins step 2's targets |
| §12 | round 18, a focused re-check limited to D17-A…D17-D |

**False premises in round 17's brief, found while implementing:**
1. "`origin/main` has since dropped" the local method: `main` never carried it. `git log --oneline -S'time -p bash scripts/trip-wires.sh' origin/main` prints nothing, and `CLAUDE.md` at `origin/main` (`f1cf5d67`) still says "(~1s)" (`/usr/bin/grep -c '(~1s)'` → 1), the figure #501 removed as false. So "make it match main's current text" would restore a figure; step 2 instead removes the local method from #501's text.
2. Not false, but wider than measured: an alias, listed among the examples, reaches only a name on the window's own lines after the fixtures file, not a function body the prelude parsed before it (measured below), so the design memo's example says so.

**Resolution routes, measured** (bash 5.3 and 3.2, the same result on each):

```sh
$SH -c 'mktemp(){ echo fn-mktemp; }; exec(){ echo fn-exec; }; unset(){ echo fn-unset; }; POSIXLY_CORRECT=1; x=$(exec /usr/bin/mktemp -d "$1/xXXXXXX"); echo "A:[$x]"' _ "$S"
# A:[<S>/x…]   (the real binary)
$SH -c 'enable -n exec; exec(){ echo fn-exec; }; POSIXLY_CORRECT=1; x=$(exec /usr/bin/mktemp -d "$1/xXXXXXX"); echo "B:[$x] rc=$?"' _ "$S"
# B:[fn-exec] rc=0
printf 'shopt -s expand_aliases\nalias mktemp="echo aliased"\n' > a.sh; printf 'f() { mktemp -d /tmp/zzXXXX; }\n' > p.sh
$SH -c '. ./p.sh
. ./a.sh
mktemp -d "$PWD/zXXXXXX"
f'
# aliased -d <PWD>/zXXXXXX   (a later top-level line)  /  /tmp/zz…   (the function parsed before)
```

The fixtures file has no instance of the routes outside comments (`8413a4db`'s file and the
prototypes' are the same here; a positive control, the harness, gives 17):

```sh
awk '!/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.fixtures.sh \
  | /usr/bin/grep -cE '^[^#]*\(\) *\{|(^|[;&| ])(alias|hash|enable|shopt|function) |(^|[^A-Za-z_])PATH='   # 0
```

**The draft-19 prototype.** Base `35dc1153`, the whole prototype as one diff: `git -C <sandbox> diff >
proto19.diff`, 565 lines, sha256 `24067f1ce6eb3ff280ad7c6a5351343cbd2a3b8a1da9245fa02e959d353a8e6b`
(`…/scratchpad/author19/proto19.diff`). On top of the draft-18 prototype's:
- P-j's first-entry clause restored as at `8413a4db`: `case "$PATH" in "$_FGIT_BIN"|"$_FGIT_BIN":*) ;; *)
  _pj="$_pj first-entry" ;; esac`;
- the P-j record re-anchored as C6 plans (`_FGIT_PATH="\/nonexistent-k2:$_FGIT_BIN:` in place of
  `_FGIT_PATH=\/usr\/bin:`);
- the mutation set's entry guard and the two "WHAT IT CONSUMES" headers name `$_VFY`. The controls
  file sets `_VFY` before it sources the mutation set (`/usr/bin/grep -n -e '_VFY="' -e '_MUTATIONS='
  .claude/tools/webref-generic-core-trip-wire.controls.sh` → lines 122 and 396), and outside the
  `MUTANTS` here-document (lines 230–387) neither `mutations.sh` nor `mutgen.sh` names `$CTL` but in
  those two headers, now `$_VFY`;
- `_mut_gen_run`: after the regex loop, a generated count of 0 prints `!! the boundary-mutant generator
  produced no mutant …` and adds one to `_mut_gen_bad`.

**The cells** (2026-09-30, `cell15.sh` verbatim from `…-pr527.md` round 13, two wires at a time, `HOME`
a scratch directory). Inserts verbatim, from the round-17 reviewer's files
(`/tmp/elidex-plan-review.7e5b6f5f-cec2-4f20-9e7a-b25f0c4752cf/r17/ax3/ins/`); each starts with `mkctl`'s
two lines (round 16's cells, above), then:
- `shimnone`: `_k2s="${_FW_DIR%/*}/zzshim"; mkdir "$_k2s" && printf '#!/bin/sh\necho "%s"\n' "$_k2d" > "$_k2s/mktemp" && chmod +x "$_k2s/mktemp"`;
- `shimafter`: `shimnone`'s line, then `PATH="${PATH%%:*}:$_k2s:${PATH#*:}"`;
- `shimhash`: `shimnone`'s line, then `hash -p "$_k2s/mktemp" mktemp`;
- `shimprepend`: `shimnone`'s line, then `PATH="$_k2s:$PATH"`.

`xgroup` is `{ : $((1/0)); } 2>/dev/null` alone. The `Jm` trees pin C6's re-anchored record: the
harness line `_FGIT_PATH="$_FGIT_BIN:$PATH"` becomes `_FGIT_PATH="/nonexistent-k2:$_FGIT_BIN:$PATH"` (the
record applied), and the record itself is pointed at the new line so the anchor check stays green.

| cell | tree | bash 5.3 | bash 3.2 |
|---|---|---|---|
| clean | draft 19 | rc 0, PASSED, 19 s | rc 0, PASSED, 25 s |
| shimnone | draft 19 | rc 1, 3 s, `K2PRE zzy:[a symlink to a tree holding a git dir]` | — |
| shimafter | draft 19 | ⚠ rc 0, PASSED, 17 s (class (c), D17-A) | ⚠ rc 0, PASSED, 23 s |
| shimhash | draft 19 | ⚠ rc 0, PASSED, 17 s (class (c)) | ⚠ rc 0, PASSED, 23 s |
| shimprepend | draft 19 | rc 1, 6 s, P-j `first-entry`; no control run | rc 1, 8 s, same |
| xgroup | draft 19 | ⚠ rc 0, PASSED, 17 s (class (c)) | ⚠ rc 0, PASSED, 23 s |
| Jm | draft 19 | rc 1, 5 s, P-j `first-entry` (the record killed) | — |
| Jm | draft 18 | rc 0, PASSED, 16 s (the record survives without the clause) | — |

**The mutation-mode cells** (X3's opt-in cells, 2026-09-30, bash 5.3, one run each). The script is
`cell15.sh` with two changes: `env WEBREF_WIRE_MUTANTS=1` before `HOME=…`, and `alarm 5400` for
`alarm 120` (the round-17 reviewer's `cellmut.sh`, whose own runs were stopped at 129 s). The plant
insert is `ln -s /dev/null "$CTL/.genmutants"`; the floor tree is the draft-19 prototype with
`_mut_gen_run`'s `>> "$_VFY/.genmutants"` in the regex loop made `>> /dev/null`.

| cell | tree | result |
|---|---|---|
| plant | `8413a4db` (base) | rc 0, PASSED, 2056 s: `145 entr(ies), 0 not killed as named`; `generated boundary set: 0 mutant(s) …, 0 neither killed nor argued equivalent` (the blind) |
| plant | draft 19 | rc 1, 2530 s: `144 entr(ies), 1 not killed as named`; `50 mutant(s) …, 0 neither killed nor argued equivalent` (the plant has no effect) |
| floor | draft 19 | rc 1, 1724 s: `144 entr(ies), 1 not killed as named`; `!! the boundary-mutant generator produced no mutant …`; `0 mutant(s) …, 1 neither killed nor argued equivalent` |

The one record not killed as named on the prototype is `MUTANT 14`, `a byte no UTF-8 locale can bracket`
(`s/^export LC_ALL=C$/export LC_ALL=C.UTF-8/`): P-h reds on it and the untrusted-build stop runs no
control, so its needle never appears. That is a C8 interaction the plan had not listed; it is open in
corpus §6.1 for round 18. (144 against base's 145: the prototype already lacks C6's deleted P-j
record.)

**Records planned by draft 19**: none added or removed. C6's re-anchor was already planned; the
empty-generated-set check has no record (above). Draft 18's totals stand: **64 records of this PR,
159 in all**, `_MUT_RECORDS_MIN=159`, labels 18, `_MUT_UNRECORDED_MAX=21`. None of it is implemented
yet.

**Draft 19's X3 record, corrected in draft 20.** Round 17's table above (IMP-4, D17-D) quotes draft 19 as
committed at `3f1a96af`: both outcomes "predicted from the code; X3 measures it". `7be0a98c` then replaced
that phrase in the design memo §4 and in `…-landing.md` §11 X3 with the measurements below, so the
quote is history, not the current text. The mutation-mode runs, one each on bash 5.3, ran as two
background chains at a time, each chain one wire at a time (so at most two wires), which is the
condition behind "29–42 min":
- chain 1: `./cell15.sh Jm_proto Jm19b b53 ins/clean`, then `./cellmut.sh floor19 floor19 b53 ins/clean`,
  then `./cellmut.sh base genmBase b53 ins/genm`;
- chain 2: `./cell15.sh Jm_p18 Jm18b b53 ins/clean`, then `./cellmut.sh proto genm19 b53 ins/genm`.

`cellmut.sh` is the round-17 reviewer's: `cell15.sh` with `env WEBREF_WIRE_MUTANTS=1` before
`HOME=…` and `alarm 5400` for `alarm 120`; `ins/genm` is `ln -s /dev/null "$CTL/.genmutants"`. The
session scratch is ephemeral, so the essential log lines are copied here verbatim:
- `genmBase` (`8413a4db`), rc 0, 2056 s:
  `  mutation set: 145 entr(ies), 0 not killed as named, 0 not exercisable on this machine` /
  `  generated boundary set: 0 mutant(s) from $K2RE and $K2RE_PATH, 0 neither killed nor argued equivalent` /
  `webref generic-core layering trip-wire PASSED`;
- `genm19` (draft-19 prototype), rc 1, 2530 s:
  `!! MUTANT 14 (a byte no UTF-8 locale can bracket) killed for the WRONG REASON (exit 1): the output` /
  `  mutation set: 144 entr(ies), 1 not killed as named, 0 not exercisable on this machine` /
  `  generated boundary set: 50 mutant(s) from $K2RE and $K2RE_PATH, 0 neither killed nor argued equivalent`;
  its first line was also `…/.claude/tools/webref-generic-core-trip-wire.harness.sh: child setpgid
  (76871 to 76871): Operation not permitted`, bash's own message. It is the known intermittent line
  that X4's normaliser already drops (`…-reviews.md` §13, the X4 and X3 rows). Round 19's analysis (as
  relayed): it comes from the fork under `_control`'s `set -m`; only the 30 s watchdog's group kill
  depends on that group, and no watchdog fired in `genm19`; it did not reproduce in 1900 forks; the
  mechanism is not established;
- `floor19` (draft-19 prototype, generator output sent to `/dev/null`), rc 1, 1724 s: the same
  `MUTANT 14` and `mutation set:` lines, then `!! the boundary-mutant generator produced no mutant: no
  rule of $K2RE or` / `  generated boundary set: 0 mutant(s) from $K2RE and $K2RE_PATH, 1 neither killed
  nor argued equivalent`.

**A discarded first run, disclosed.** The `Jm` cells of round 17 ran twice. The first trees
(`Jm_proto`, `Jm_p18`) were copied with `tar --exclude=./.git`, so they held no `.git`: `Jm18` gave
rc 2, `!! read 0 stored objects or files; this wire would report no violation for a reason that is not
'there are none'`, the wire's scan finding no repository, and `Jm19` gave rc 1 by P-j before the scan.
The cause is established from the cell directories: `cells/Jm18.b53` and `cells/Jm19.b53` have no
`.git`, `cells/Jm18b.b53` and `cells/Jm19b.b53` have one. The trees were rebuilt with `.git`, and the
table above reports those runs (`Jm18b`, `Jm19b`).

**Plan-review round 18 (focused, on draft 19, frame `805dbc46`) → draft 20.** 0 CRIT / 1 IMP / 10 MIN,
as the orchestrating session relayed them. It decided the dispositions; draft 20 implements them.
Labels D18-A…D18-C (a `D…` family, no section number).

| finding | disposition |
|---|---|
| IMP (Ax2+Ax3): a fixtures file can write an executable into a writable directory on the caller's `PATH`; it persists after the window and redirects the parent's commands. `stalectl` (a never-matching record) rc 1 "its anchor is stale" on both shells; `staleshim` (the same tree, with a `mktemp` shim in that directory answering the parent's `verifyXXXXXX` call with a prepared directory whose `.anchor` is a link to `/dev/null`) rc 0, PASSED on both shells | **D18-A**: construction cannot close it (resolving the parent's commands to absolute paths first only narrows it; the fixture can overwrite the resolved file), so it is declared class (c): D17-A's property covers the verifier's commands in the window and the parent's after it, with the filesystem route as an example; the carve-out in the design memo §0.3 (the design rule and the declared list), §4's C11 paragraph, §10 #2, `…-landing.md` §9.1's C11 row and `…-residuals.md` §5.1; the R1 intersection in one sentence; `staleshim`/`stalectl` in §0.3's measured boundary and X16 |
| open item (draft 19): the `LC_ALL=C.UTF-8` record killed for the wrong reason | **D18-B**: relabelled to P-h's label, and one record added for `rawbyte`, the scan's locale alone (`m14grep`); corpus §6.1's C8 rows. The runner scores a wrong-reason kill as not killed (`_mut_trial` returns 2 on a missing needle, `mutations.sh` lines 469–474 on the draft-18 prototype, and the caller counts it in `_mut_bad`, lines 689–690), so X3 was red, the fail-safe side |
| Ax3 MIN: the empty-set floor belongs where every run sees it | **D18-B**: `_mut_gen_floor`, always on, with its own label and a `mutgen` record (corpus §6.1, C11); the mutation-mode check of draft 19 is removed, so there is one check |
| Ax3 MIN: the no-reset reason rested on a count whose regex found 1 of 4 negative-control forms | **D18-C**: replaced by the measured direction, `gitfn` red on both shells (design memo §0.3); the count argument is dropped |
| Ax3 MIN: the step-8 gate and 7.4's path | **D18-C**: step 8 needs X9's run at the current T to be SUCCESS and under the threshold, or, over it, the user's decision at 7.3 for that T; T moving "by a fix or by a merge"; §9.1 step 1's local checks apply to a merge that moves T too. The same wording is now in `…-landing.md` 7.4, the design memo §12 and round 17's IMP-2 row above |
| Ax4+5 MINs | "unchanged" qualified for `…-residuals.md` (design memo preface and §5 pointer); the "predicted" quote and the X3 logs above; the `Jm18` first run above; step 2's complement check (`…-landing.md` §9.1 step 2) and its "changes … only" sentence naming the §Q record; the harness comment citing memo §5.2 → `…-residuals.md` §5.2 in C6's row; X3's bash-3.2 runs at T with an estimate (`…-landing.md` §11) |
| §12 | round 19, focused on draft 20's changes; 0 IMP there → implementation (C6…C12) |

**False premises in round 18's message, found while implementing:**
1. The reviewer's `ins/parentshim` does not prepare the directory it names: it writes only the shim,
   and the prepared directory with its `.anchor` link was made outside the insert. Draft 20's
   `staleshim` has the insert make both, so the fixtures file alone produces the cell.
2. "Records +1 → 65 / 160" held for the relabel and `m14grep` alone; with the floor's record (D18-B)
   the totals are 66 / 161.

**The cell script for caller-`PATH` cells**, `cellw.sh`, the round-18 reviewer's
(`/tmp/elidex-plan-review.7e5b6f5f-cec2-4f20-9e7a-b25f0c4752cf/r18/ax23/cellp.sh`, identical to its
`cellw.sh`), verbatim:

```sh
#!/bin/bash
# cellw.sh <tree> <tag> <b53|b32> <insert-file|-> [wire-sed-expr]
# cell15.sh verbatim, plus an optional sed applied to the wire copy (a record's trial).
A=${K2_CELLS:?}; tree=$1 tag=$2 sn=$3 ins=$4 wsed=${5:-}
T=$A/cells/$tag.$sn; mkdir -p "$T" && (cd "$tree" && tar cf - --exclude=./target .) | (cd "$T" && tar xf -)
if [ "$ins" != - ]; then
python3 - "$T/.claude/tools/webref-generic-core-trip-wire.fixtures.sh" "$ins" <<'P'
import sys;p,i=sys.argv[1],sys.argv[2];s=open(p).read();a=': > "$_FW_DIR/built"\n';assert s.count(a)==1
s=s.replace(a,open(i).read()+a);open(p,'w').write(s)
P
fi
if [ -n "$wsed" ]; then W="$T/.claude/tools/webref-generic-core-trip-wire.sh"; sed "$wsed" "$W" > "$W.new"; cmp -s "$W" "$W.new" && { echo "SED MATCHED NOTHING"; exit 3; }; cat "$W.new" > "$W"; rm "$W.new"; fi
if [ "$sn" = b32 ]; then SH=/bin/bash; P=$A/cpath:/bin:/usr/bin:$PATH; else SH=/opt/homebrew/bin/bash; P=$A/cpath:$PATH; fi
mkdir -p "$A/home"; s0=$(date +%s)
( cd "$T" && exec env HOME=$A/home PATH="$P" perl -e 'setpgrp; alarm 120; exec @ARGV' $SH .claude/tools/webref-generic-core-trip-wire.sh ) > "$T.log" 2>&1 & g=$!
rc=0; wait "$g" || rc=$?
echo "CELL $tag $sn rc=$rc t=$(( $(date +%s)-s0 ))s CF=$(/usr/bin/grep -c 'CONTROL FAILED' "$T.log") NE=$(/usr/bin/grep -c 'CONTROL NOT EXERCISED' "$T.log") PASSED=$(/usr/bin/grep -c 'trip-wire PASSED' "$T.log") stopped-before-controls=$(/usr/bin/grep -c K2PROTO "$T.log")"
/usr/bin/grep -e '^!!' "$T.log" | cut -c1-220 | head -3 | sed 's/^/    /'
o=$(ps -ax -o pid=,pgid=,command= | awk -v g="$g" '$2==g' | cut -c1-150)
if [ -n "$o" ]; then echo "    LEFT in group $g (killed):"; echo "$o" | sed 's/^/      /'; kill -9 -"$g" 2>/dev/null; fi
chmod -R u+rwX "$T" 2>/dev/null
```

**Inserts, verbatim.** `gitfn`: `git() { command git -c user.name=k2 -c user.email=k2@example.invalid "$@"; }`.
`staleshim` (`$S` the cell root, `K2_CELLS`; written out with it at run time):

```sh
# setup, once per cell root (`cellw.sh` puts "$K2_CELLS/cpath" first in PATH): mkdir -p "$K2_CELLS/cpath"
mkdir -p "$S/prep" && ln -sfn /dev/null "$S/prep/.anchor"
printf '#!/bin/sh\ncase "$*" in *verifyXXXXXX*) echo "%s"; exit 0;; esac\nexec /usr/bin/mktemp "$@"\n' "$S/prep" > "$S/cpath/mktemp" && chmod +x "$S/cpath/mktemp"
```

The `stale` tree is the draft-20 prototype with one record added at the end of the `MUTANTS`
here-document: `s/K2NEVERMATCHES/x/` TAB `the fixture build window completed`. Between the two
`staleshim` runs, `cpath/mktemp` and the prepared directory were removed; after each run the prepared
directory held `.control_out`, `.fsm_out`, `.mutants`, `.umask_out` and `fsmhook`, the parent's working
files (`.anchor` and `.bare` are removed by the parent at the end of their checks).

**The draft-20 prototype.** Base `35dc1153`, the whole prototype as one diff: `git -C <sandbox> diff >
proto20.diff`, 616 lines, sha256 `0c0c6aaa9f0072a92af9fc3c4e9521123bf1dcc66611ddbe8818893a9fa7ca3f`
(`…/scratchpad/author19/proto20.diff`). On top of the draft-19 prototype's:
- the `LC_ALL=C.UTF-8` record relabelled to `the fixture build window reads in the wire's locale`, and the
  record `s/_co="$(grep -aEn -- "$K2RE"/_co="$(LC_ALL=C.UTF-8 grep -aEn -- "$K2RE"/` TAB `a byte no UTF-8
  locale can bracket` added;
- `_mg_lbl="the boundary-mutant generator derives a non-empty set from the wire's regexes"` in the
  controls file; `_mut_gen_floor` in `mutgen.sh` (the generated count over `$K2RE` and `$K2RE_PATH`, red
  under `_mg_lbl` when 0 or when generation fails), called by `_mut_correspondence`; the mutation-mode
  check removed from `_mut_gen_run`; `mutgen` added to `_MUT_TARGETS`; the record
  `mutgen:s/^_mut_regex_mutants() {$/_mut_regex_mutants() { return 0/` TAB `_mg_lbl`'s text;
  `_MUT_RECORDS_MIN=146` (the prototype's own count);
- `mutgen.sh`'s header names `$_mg_lbl` and `_mut_gen_floor`.

**The cells** (2026-09-30, `cell15.sh` or `cellw.sh`, two chains at a time, `HOME` a scratch
directory). A record's kill is measured by applying its expression to the tree (`cellw.sh`'s fifth
argument for a wire target, by hand for `mutgen`), with the record itself re-pointed at the mutated text
so the always-on anchor check, which a real trial skips inside a mutant copy, stays green; the survive
check additionally removes the clause the record pins.

| cell | tree | bash 5.3 | bash 3.2 |
|---|---|---|---|
| clean | draft 20 | rc 0, PASSED, 18 s | rc 0, PASSED, 24 s |
| stalectl | `stale` | rc 1, 17 s, `mutation record "s/K2NEVERMATCHES/x/" no longer matches …: its anchor is stale` | rc 1, 21 s, same |
| staleshim | `stale` | ⚠ rc 0, PASSED, 17 s (class (c), D18-A) | ⚠ rc 0, PASSED, 24 s |
| gitfn | draft 20 | rc 1, 5 s, P-a: `command line:	user.name=k2 command line:	user.email=k2@example.invalid`; no control run | rc 1, 7 s, same |
| m14grep (kill) | draft 20 | rc 1, 15 s, the one CONTROL FAILED `a byte no UTF-8 locale can bracket`; no P-h, no stop | rc 1, 21 s, same |
| m14grep (survive) | draft 20, the `rawbyte` `_control` line prefixed by `: ` | rc 0, PASSED, 16 s | — |
| m14lc (the relabelled record) | draft 20 | rc 1, 5 s, P-h: `LC_ALL is [C], the wire's is [C.UTF-8]` | — |
| floor record (kill) | draft 20 | rc 1, 16 s, `CONTROL FAILED (the boundary-mutant generator derives a non-empty set from the wire's regexes): no mutant from $K2RE or $K2RE_PATH` | rc 1, 22 s, same |
| floor record (survive) | draft 20, the `_mut_gen_floor` call removed | rc 0, PASSED, 17 s | — |

The `m14grep` kill cells count one `trip-wire PASSED` line, which is the failed control replaying its
child's output (log line 8, indented), not the run's verdict. A first pass of the kill and survive
cells without the re-pointed record was red on the stale anchor as well; it is not reported above.

**Records planned by draft 20** (`…-landing.md` §9.1, corpus §6.1): draft 18's +14, and +2 (C8:
`m14grep`; C11: the generator floor), with one relabel (C8). That is **+16**: **66 records of this PR,
161 in all**, `_MUT_RECORDS_MIN=161`, labels 19 (the floor's added), `_MUT_UNRECORDED_MAX=21`. Draft
18's plan (159) is superseded. None of it is implemented yet.

**Plan-review round 19 (focused, on draft 20, frame `5fee0738`) → draft 21.** 0 CRIT / 1 IMP / 14 MIN,
as the orchestrating session relayed them. It decided the dispositions; draft 21 implements them.
Labels D19-A…D19-C (a `D…` family, no section number). The orchestrating session also decided that no
further prose plan-review round follows (design memo §12).

| finding | disposition |
|---|---|
| IMP (Ax2+Ax3): draft 20 regressed the empty-generator detection. `RC` (the hand set cut to `!survive`, `_MUT_RECORDS_MIN=1`, `_MUT_UNRECORDED_MAX=999`, and `_mut_gen_run`'s `>> "$_VFY/.genmutants"` made `>> /dev/null`) rc 0, PASSED with `0 mutant(s)` on both shells while the always-on floor was green (n=50); `RA` (`_mut_gen_run() { return 0`) the same | **D19-A**: two layers. The always-on floor guards the generator's output; a run-time check in `_mut_run`, right after `_mut_gen_run`, requires `_mut_gen_n` to equal `_mut_gen_floor_n` and to be non-zero. It is in the caller, not at the end of `_mut_gen_run` (false premise 1 below). No record reaches it; X3's floor cell is restored (`…-landing.md` §11). Design memo §4 rewritten as two layers. Cells below: `RC`, `RA` red on both shells; the `.genmutants -> /dev/null` shim variant red again |
| Ax2+3 MIN: §4's "reference repos … outside fixture reach by construction" | the §0.3 class-(c) exception added (they are made in a `mktemp -d` directory too) |
| Ax2+3 MIN: the verifier's source in the checkout is writable from the window (`srcrw`: rc 0, PASSED on the stale tree; `stalectl` rc 1) | **D19-B**: §0.3's property is "which executable or code the verifier runs"; `srcrw` among the examples and in the measured boundary; the filesystem route needs only a writable file the verifier runs or sources (the checkout always is one) |
| Ax2+3 MIN: the prefix comments at `mutations.sh` 136 and 426 name only `harness:` and `fixtures:` | C11's row: both name `mutgen:` (`…-landing.md` §9.1); the draft-21 prototype does |
| Ax2+3 MIN: `genm19`'s setpgid line "not investigated" | cited to `…-reviews.md` §13 and round 19's analysis (above, in the X3 record) |
| Ax4+5 MINs | **D19-C**: the design memo's preface and §5 pointer say "extended since by D17-A and D18-A"; X3's range re-derived from the two runs it cites (2056 s, 2530 s → 34–42 min) and the bash-3.2 estimate from the draft-20 clean runs (24 s / 18 s → 46–56 min); step 2's complement check adds `-e '導出'`, which also matches `再導出`; `gitfn` in X16; corpus §6.1's "every record probed" marked as relayed, with the reviewer's directory; the staleshim steps create `cpath`; C11's row plans the parent memo's banner lines 14 and 25 to name the generator; the rollover rule for this file and the design memo's next seam (design memo §13); land order 7.3 handles a red run with T unchanged; `…-landing.md` §9.1's closing sentence points to the design memo §12 |
| records | the run-time check adds none; the totals stay 66 of this PR, 161 in all, labels 19, `_MUT_UNRECORDED_MAX=21` |

**False premises in round 19's message, found while implementing:**
1. "Restore a run-time check at the end of `_mut_gen_run`": at the end of `_mut_gen_run` the check
   would not see `RA`, whose edit returns at the top of that function. The check is in `_mut_run`,
   the caller, right after the call; `RA` is red there.
2. `RA` is not "`_mut_gen_run() { return 0`" alone: the reviewer's `RA` tree also cuts the hand set to
   three records (the two draft-20 records and `!survive`) with `_MUT_RECORDS_MIN=1` and
   `_MUT_UNRECORDED_MAX=999`. Draft 21's `RA` does the same.

**The draft-21 prototype.** Base `35dc1153`, the whole prototype as one diff: `git -C <sandbox> diff >
proto21.diff`, 654 lines, sha256 `c6716ca8e666fb349e054c9a8720ca6febbaa1f7f42d8ccaa337b78bda2554ad`
(`…/scratchpad/author19/proto21.diff`). On top of the draft-20 prototype's:
- `_mut_gen_floor` sets `_mut_gen_floor_n`;
- `_mut_run`, after `_mut_gen_run`: `if [ "$_mut_gen_n" -ne "${_mut_gen_floor_n:-0}" ] || [ "$_mut_gen_n"
  -eq 0 ]`, the two-line `!!` below, and `_mut_gen_bad` + 1;
- the two prefix comments name `mutgen:`; `mutgen.sh`'s header names `_mut_gen_floor_n`.

The run-time check's failing message, verbatim (`RC`, bash 5.3):

```text
!! the generated boundary set ran 0 mutant(s), but the generator derives
   50 from the wire's regexes: this run did not test every rule.
```

**The cells** (2026-09-30; `cellm.sh` is the round-19 reviewer's, `…/r19/ax23/cellm.sh`: `cell15.sh`'s
run with no insert, and `WEBREF_WIRE_MUTANTS=1` with `alarm 900` when its fourth argument is given;
`cellwm.sh` is `cellw.sh` with `WEBREF_WIRE_MUTANTS=1` and `alarm 900`; two chains at a time, the
caller-`PATH` cells serial in one chain, `cpath/mktemp` and the prepared directory removed after each).
`RC`, `RA` and `RN` cut the hand set as above (`RN`: `!survive` only, nothing else changed). The
`genshim` insert is `staleshim`'s with `.genmutants` in place of `.anchor` in its first line; its tree
cuts the hand set to `!survive`. `srcrw`'s insert, per shell: `printf '\n_mut_correspondence() { return
0; }\n' >> "<cell copy>/.claude/tools/webref-generic-core-trip-wire.mutations.sh"`, on the `stale` tree
(the draft-21 prototype plus the never-matching record).

| cell | tree | bash 5.3 | bash 3.2 |
|---|---|---|---|
| clean | draft 21 | rc 0, PASSED, 16 s | rc 0, PASSED, 22 s |
| RC (mutation mode) | draft 21, cut | rc 1, 32 s: the message above; `0 mutant(s) …, 1 neither killed nor argued equivalent` | rc 1, 44 s, same |
| RA (mutation mode) | draft 21, cut | rc 1, 60 s: `3 entr(ies), 0 not killed as named`; the message; `1 neither …` | rc 1, 82 s, same |
| genshim (mutation mode, `.genmutants -> /dev/null` via the parent's `mktemp`) | draft 21, cut | rc 1, 33 s, the message (red in draft 19, green in draft 20; class (c) either way) | rc 1, 44 s, same |
| stalectl | `stale` | rc 1, 15 s, `… its anchor is stale` | — |
| srcrw | `stale` | ⚠ rc 0, PASSED, 16 s (class (c), D19-B) | ⚠ rc 0, PASSED, 21 s |
| gitfn | draft 21 | rc 1, 5 s, P-a `command line:` | rc 1, 7 s, same |
| RN (mutation mode, the negative control: nothing changed but the cut) | draft 21, cut | rc 0, PASSED, 748 s: `generated boundary set: 50 mutant(s) …, 0 neither killed nor argued equivalent`, no `!!` (the equality holds, 50 = 50) | — |

`RN` is the run-time check's negative control: with the generator's path intact it stays green, so
`RC`, `RA` and `genshim` are red for the cause the check names. `RN` ran with `alarm 2400` (`cellm.sh`
with that one number changed), since its 50 generated trials take longer than 900 s allows.

**The implementation (C6…C12), 2026-09-30.** No further prose plan-review round preceded it (design
memo §12). Base `fa339493` (draft 21). Each commit's per-commit checks (X1, X4b, X10, plus
`scripts/trip-wires.sh`) ran green on `/opt/homebrew/bin/bash` 5.3 and `/bin/bash` 3.2 with a scratch
`HOME`, and each new record's kill and survive were measured by the single-trial method (a
`*.mutant.<tag>.*` copy beside the wire, so the always-on anchor check inside `_mut_correspondence` is
skipped as it is for a real mutant, matching what the runner does; the anchor check itself was then
separately confirmed live at X16's `xanchor`/`stalectl` cells, run on a plain, non-`.mutant`-named copy)
before each commit:

| commit | sha | what | per-commit checks |
|---|---|---|---|
| C6 | `523ce31f` | M-PATH: `_FGIT_PATH="$_FGIT_BIN:$PATH"`, the `_fp_*` normaliser deleted; P-j loses its per-entry clause; P-j's two surviving records relabelled and one re-anchored, one deleted; `_MUT_RECORDS_MIN` 145→144 | X1/X4b/X10/trip-wires green both shells; P-j record kill+survive measured |
| C8 | `043526ee` | `_fgit_window_incomplete_exit`→`_fgit_window_verdict_exit`, the sole writer of `_fw_trusted`; `_fw_built_or_w2` reads it; W3's report moves in; W2 relabelled and re-anchored, two new records (post_bad forced, W3 forced); P-h relabel + new `rawbyte` wire record; `_MUT_RECORDS_MIN` 147 | X1/X4b/X10/trip-wires green both shells; 4 records' kill+survive measured (W2 re-anchor: kill only, see below) |
| C9 | `c9c0639d` | M-SHAPE: the two-pass census (`_pg_census`) before any postcondition runs git; the postconditions' directory `_pq` made fresh by `mktemp -d` after the build; the in-loop per-link search and the old HEAD/config two-name guard deleted (replaced by a general shape scan); 8 new records; `_MUT_RECORDS_MIN` 155 | X1/X4b/X10/trip-wires green both shells; X6 (template/include) re-checked; 8 new records' kill measured, one (the harness shape-scan-status record) also survive; a census-removed survive proxy on `xcommon`'s fixture reached the 60 s alarm (SIGTERM), the necessary-condition check the design prescribes for a FIFO-involving scenario |
| C10 | `ebbcb282` | P-k: `count-objects -v` over every `.git` P-g compares, red on `alternate:`/non-zero/stderr; a third-probe-repo liveness check, no machine-limitation arm; 4 new records; `_MUT_RECORDS_MIN` 159 | X1/X4b/X10/trip-wires green both shells; 4 records' kill+survive measured |
| C11 | `7f4259f9` | `_VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")"` right after `_fgit_window` returns; every parent working file under `$CTL` moved there; `.fifoprobe` to `$SCRATCH`; the entry guard and two headers renamed; `_mut_gen_floor` (always on) + one record; the parent memo's banner (L14/L25) and two prefix comments updated | X1/X4b/X10/trip-wires green both shells; the design memo §4 checker lists exactly one line after the move; the new record's kill+survive measured |
| C12 | `219fe5e7` | the child writes a fixed canary to fd 2 after the options re-check; the parent requires it before counting the window done, else a new `_fw_why`; one new record | X1/X4b/X10/trip-wires green both shells; the record's kill+survive measured, plus an `xexecarith`-style (redirect then division by zero) variant |
| docs | `52f3efaa` | round 19's four leftover MINs (§0.3's class-(c) wording, the `導出`/`再導出` count, both "Drafts 11–20" headings → "11–21") | n/a (docs only) |

**The two defects C8's own implementation found (both fixed before the commit landed; ⚠ the second
fix, the `return 0`, is superseded: the fix round below deleted it and moved `_fw_trusted`'s
computation into `_fgit_window`):**
1. `_fw_built_or_w2`'s guard was first written `[ "$_fw_trusted" -eq 1 ] || return 0` — the polarity of
   the old `_fw_done` guard, read wrong for the new flag (it returned "trusted, proceed" exactly when
   `_fw_trusted` was NOT 1). Caught immediately by inspection before any run; fixed to
   `[ "$_fw_trusted" -ne 1 ] || return 0`.
2. `_fgit_window_verdict_exit`'s incomplete branch, with its own `exit 2` defeated by the W2 re-anchor
   record (`harness.sh`'s `built`/`notdone` swap), fell through into the trusted computation and set
   `_fw_trusted=1` off a window that was never complete (`_fw_done` still 0) — found by the re-anchor
   record's kill measurement reading rc 0 PASSED instead of the expected W2 report. Fixed by a redundant
   `return 0` inside the same guard (a `set -e`-safe `return`, not `return 1`, since the call site in
   `controls.sh` is bare and a non-zero return there would abort the whole run instead of falling through
   to `_control`'s own gate) — see the comment beside it in `webref-generic-core-trip-wire.harness.sh`.

**The W2 re-anchored record: kill measured, no distinct survive** (superseded by the fix round below:
the record now removes both exits, and each W2 record's survive is measured against the trust computation). Its kill (rc 1, the W2 needle, `t`
below) is measured. A survive attempt that removes `_control`'s own `_fw_built_or_w2 || return 1` line
alone does **not** pass: the relative-scratch block's separate call to the same function still reports
W2 first (rc 1, same needle) — both call sites share one function, so defeating one exposes the other.
This matches the design memo §3's declared gap for this family: "removing one of the three [non-`_control`]
block gates survives the mutation set … those gates are pinned by the traced `w2rec` cell … not by a
record." The record's own claim — that the exit being gone still lets W2 fire — is what "kill" proves;
no further survive check is owed by corpus §6.1 for this row (it is a re-anchor, not a new record).

**X16, at `219fe5e7`** (C12's head, then taken for the final head; T has moved since, to `0280a67f` and
then `6ed414b2`, so X16 re-runs at T, `…-landing.md` §9.1 step 1. The rows quote the `K2PRE` tag that
`0280a67f` removed. These runs used `alarm 60`, not `cell15.sh`'s 120 s as §11 asks; every red cell
ended within 20 s, so the bound decided nothing here, but under bash 3.2 an alarm alone is no bound:
`…-landing.md` §11, "Cell runners at T") (both shells; each cell copied beside the wire under a name that
does **not** contain `.mutant.`, so the always-on anchor check runs as it does for an ordinary invocation,
except `stalectl`/`staleshim` which additionally vary the caller's `PATH`; `HOME` a scratch dir; each in
its own process group via `perl -e 'setpgrp; alarm 60; exec @ARGV'`, no leftover process found after any
run):

| cell | bash 5.3 | bash 3.2 |
|---|---|---|
| clean | rc 0, PASSED, 14 s | rc 0, PASSED, 24 s |
| R26③ `commondir` (FIFO in `clean/.git`) | rc 1, 2 s, P-g `K2PRE clean/.git:[shape: …/commondir]` | rc 1, 4 s, same |
| R26③ `HEAD` (FIFO) | rc 1, 3 s, P-g `K2PRE clean/.git:[shape: …/HEAD]` | rc 1, 3 s, same |
| R26③ `config` (FIFO) | rc 1, 2 s, P-g `K2PRE clean/.git:[shape: …/config]` | rc 1, 4 s, same |
| R26③ `config.worktree` (FIFO) | rc 1, 3 s, P-g `K2PRE clean/.git:[shape: …/config.worktree]` | rc 1, 4 s, same |
| xcommon | rc 1, 3 s, P-g `K2PRE zzr/.git:[shape: …/config]` | rc 1, 4 s, same |
| bareh | rc 1, 4 s, P-g `K2PRE zzb.git:[a git dir not named .git]` | rc 1, 4 s, same |
| outsidein | rc 1, 3 s, P-g `K2PRE zzr/.git:[shape: …/k2root]` | rc 1, 4 s, same |
| hard | rc 1, 3 s, P-g `K2PRE clean/.git:[shape: …/info/exclude]` | rc 1, 4 s, same |
| xlink | rc 1, 3 s, P-g `K2PRE zzl:[a symlink to a tree holding a git dir]` | rc 1, 4 s, same |
| xincreg | rc 1, 6 s, P-g by origin (`file:.git/../../zzi`) | rc 1, 7 s, same |
| clean (round 13) | see "clean" above | — |
| xrefpoison | rc 1, 6 s, P-g `clean:[file:.git/config core.excludesfile=…]` | rc 1, 8 s, same |
| xrefctl | rc 1, 5 s, P-g (every fixture repo) | rc 1, 7 s, same |
| lnblind2 | rc 1, 3 s, `K2PRE zzy:[a symlink to a tree holding a git dir]` | rc 1, 3 s, same |
| xbare | rc 1, 16 s, `21 labels have no mutation record, against a ratchet of 0.` | rc 1, 20 s, same |
| xanchor | rc 1, its anchor is stale (elapsed not separately timed) | rc 1, same |
| xexecarith | rc 2, 6 s, W: "the fixtures file left the window's stderr redirected, …" | rc 2, 8 s, same |
| shimprepend | rc 1, 6 s, P-j `first-entry`; no control run | rc 1, 8 s, same |
| shimafter | ⚠ rc 0, PASSED, 15 s (declared class-(c) boundary) | ⚠ rc 0, PASSED, 22 s |
| shimhash | ⚠ rc 0, PASSED, 16 s (declared class-(c) boundary) | ⚠ rc 0, PASSED, 22 s |
| stalectl | rc 1, its anchor is stale (elapsed not separately timed) | rc 1, same |
| staleshim | ⚠ rc 0, PASSED (elapsed not separately timed; declared class-(c) boundary; `cpath/mktemp` and the prepared directory held the parent's working files, `.control_out`/`.fsm_out`/`.mutants`/`.umask_out`/`fsmhook`, after the run) | ⚠ rc 0, PASSED, same |
| gitfn | rc 1, 6 s, P-a `command line:` | rc 1, 8 s, same |

All results match `…-landing.md` §11's predicted verdicts. `xcommon`/`hard`/`xlink`/`xrefpoison`/`lnblind2`
were measured on both shells during C9's own per-commit evidence at C9's head, and re-confirmed (not
re-tabulated above) at the final head on the shell/side not already shown, so each has at least one
timed row here and one from C9's PROGRESS record; none differed from the final-head re-run.

**Plan-vs-prototype disagreements resolved (report obligation, none blocking):**
- C8 had no prototype (the reviewer's directory holds only a one-line `K2PROTO` stand-in in the controls
  file); `_fgit_window_verdict_exit`'s full logic, `_fw_trusted` and the two self-found defects above are
  this implementation's own, derived from design memo §3's prose, not copied from any prototype diff.
- C10 (P-k, its liveness probe, and all four records) likewise has no prototype; implemented from design
  memo §4's P-k table row and the `…-pr527.md` round-12 P-k measurements (the C-quoted `alternates` form).
- (⚠ The P-g loop this item describes, with its name-guarded `continue`, was itself deleted in the fix
  round below: P-g iterates the census's own list.) The plan (design memo §4, C9's landing row) states "the in-loop per-link search and the two-name
  HEAD/config guard deleted"; the draft-21 prototype (`proto21.diff`) left the in-loop per-link search's
  code in place (re-pathed to `$_pq`, not removed) — a plausible artefact of the prototype's own priority
  (proving the reviewer's cells pass), not a considered design choice. This implementation follows the
  plan's stated design over the prototype's leftover code and deletes it, replacing the symlink branch
  with a `continue` guarded only by name (`.git`/`HEAD` fall through; any other is skipped), since a
  clean two-pass census already guarantees no unclassified symlink reaches that point.
- Every other C6/C8–C12 line traces to the prototype diff (for C6, C9's shape-scan replacement and
  path-moves, C11, C12) or to a design-memo passage cited in the relevant commit message; none disagreed
  with the plan.

This file is 634 lines (`wc -l`) after this entry; the rollover rule in design memo §13 applies
only when a later entry would take it past 800, which this one does not.

**The `/code-review max` fix round, 2026-10-01.** The review ran on `0280a67f` (15 findings, numbered
here as the review's output orders them). The driver decided each disposition; three code commits and
one docs commit answer them, base `0280a67f`. Every run below: `/opt/homebrew/bin/bash` 5.3 and
`/bin/bash` 3.2 (with `PATH=/bin:/usr/bin:…`), `HOME` a scratch directory, each run its own process
group (`perl -e 'setpgrp; alarm N; exec @ARGV'`) plus a `kill -9 -<pgid>` watchdog, at most two at a
time; single records applied to a copied tree, never the opt-in mutation run (X3 is the final head's).

| commit | sha | what | records (this PR's / all) |
|---|---|---|---|
| 1 | `976e1430` | records become `expr1 [TAB expr2 …] TAB needle`; the always-on anchor check applies each expression alone and needs exactly one changed line (#9), one prefix parser `_mut_parse` (SI7); a generated mutant killed only by the always-on floor `CONTROL FAILED (<generator floor label>)` is reported as killed by the wrong subject (#4), and the floor's message carries the generator's failure | 66 / 161 |
| 2 | `15d3e704` | one census (#5: P-g's own population `find`, classifier and shape scan deleted; P-g/P-k iterate pass 1's list, zero `.git` entries red); NUL-terminated census lists; one census-stderr check after both passes (SI6); P-f requires every allowlist name (D2); every repo-aimed postcondition git runs as `( cd <repo> && git --git-dir=.git … )` (#7); P-k's `grep` exit > 1 red (#11); P-g's record comparison first, line greps only for the message (EF1); records for #10; `§1, "Outside P"` cites; `K2REF` tags removed | 75 / 170 |
| 3 | `6ed414b2` | `_fw_trusted` set at the end of `_fgit_window`, the verdict function only reports, the `return 0` deleted (SI3/AL1); W3's `./prelude.sh:` record and the label `the fixture build window ran without a shell diagnostic` (#6); the pin's re-entry guard `K2_FGIT_PIN_DEPTH`, limit 8, label `the pinned git never re-enters itself` (#2); P-e's reference with `HOME=$_FGIT_VOID` (#3); `_seal_apply_probe`, a standing self-check at load (#8); `_seal` validates mode and fixture (A1-3); label regex `_[A-Za-z0-9_]*_lbl` (AL5); `_perm_ok`, an independent permission probe gating the perm block (#1); one umask gate (#12); the dead `_git` term in the controls' entry guard (#13) | 83 / 178 |
| 4 | this commit | the plan records brought to `6ed414b2` (#14), `ci.yml`'s verdict sentence (#15), the cell-runner watchdog, X5 and X13 re-anchored for T | — |

Parts at `6ed414b2` (`wc -l`): controls 427, fixtures 696, harness 856, mutations 909, mutgen 348.
`_MUT_UNRECORDED_MAX` 21. The record table is corpus §6.1's fix-round rows.

Measured (the implementers' logs, relayed; `cell15.sh`-style cells unless named):
- **#4**: the `K2RE_PATH` quant#2 `+` → `{2,}` mutant with the `nltarget` control removed was counted
  killed by the floor alone before; after, `!! MUTANT … was killed BY THE WRONG SUBJECT`; with
  `nltarget` kept it is accepted. Both shells. (Superseded by the /simplify round: the exclusion of
  `$_mg_lbl` is replaced by an allowlist of `_control` labels, `_mut_trial '!kill'`.)
- **#9**: at `0280a67f` five records changed more than one line each (the wire record hitting a
  comment and the code, three compound W2 records, the P-f `-i` record's two sites); respelling the
  wire's code line left the old anchor check green, the new one red. Each changed record: killed with
  its needle, 10/10 runs.
- **#5/#7/D1/D2/SI6/#11**: 17 new or changed records × 2 shells, all killed with their own needle;
  survive results as corpus §6.1 says. #7's scenario (a garbage `HEAD` in `clean/.git`, the scratch
  root inside a plain `git init` tree): before rc 0, PASSED on both shells; after rc 1, `git lists no
  configuration here` and `count-objects exit 128`.
- **EF1**, the whole wire, bash 5.3, alternated, ms: before 16778, 16642, 16945; after 15616, 15617,
  15871. A local sanity figure only, like X8; it decides nothing about the budget.
- **SI3**: ROg (the verdict call moved below the first control, a complete clean build) at `15d3e704`:
  rc 1, `a control was reached over a build that was not complete and trusted`; at `6ed414b2`, re-run
  for this entry on a fresh clone: rc 0, PASSED, 18 s (5.3) and 24 s (3.2). The three W2 records are
  killed with W2's label (6/6) and each survives with its clause removed from the trust computation
  (6/6).
- **#2**: wrappers that exec "the next `git` on `PATH`" (strip-own-dir, and `which -a` skip-self):
  before, no output until the bound, 4/4 (5.3 ended by the alarm, rc 142 at 40 s; 3.2 only by the
  watchdog, rc 137 at 33 s); after, rc 1 in 8–9 s with `CONTROL FAILED (the pinned git never re-enters
  itself)`, 4/4. (Superseded by review R2, below: an `env -i PATH=…` re-exec reset the counter.)
- **#3**: a `$HOME`-reading `set -u` git wrapper: before rc 1, P-e `[…] vs []`; after PASSED, both shells.
- **#8**: with `_seal_apply`'s walk deleted, before rc 0, PASSED; after, the load-time probe stops the
  run, exit 2. **A1-3**: a mode `000<TAB>x<TAB>../k2sentinel<NL>755` chmodded a sentinel outside `$CTL`
  before; after it is refused and the sentinel keeps its mode.
- **#1**: an `err` build chain that fails, and a deleted `err` seal line: before rc 0, PASSED (the perm
  block skipped as a machine limitation); after, red for the named reason, both shells.
- **#12, AL5**: no record (an absent wrong-reason line is not a needle; every shipped label fits the
  old regex), measured by hand: the umask wrong-reason line gone; a label renamed `_pk_live_lbl`
  reaches the window only with the new regex.
- **#13**: a tracer on the wire's `_git` over a full run: 1194 calls, all from the wire, none from a part.

**bash 3.2 and the alarm.** A hung wire under `/bin/bash` 3.2 ran 5+ min under the perl `alarm`, and
#2's 3.2 cells above ended only by the watchdog. A plain `perl -e 'setpgrp; alarm 2; exec @ARGV'
/bin/bash -c 'sleep 8'` ends at 2 s, rc 142, on both shells (measured for this entry), so the cause lies
in what the wire does under 3.2; it is not measured further. Every cell runner at T therefore adds a
watchdog (`…-landing.md` §11, "Cell runners at T"); X16's `219fe5e7` runs above used `alarm 60` alone.

**X5 and X13 at T.** §E.7's `gen11.py`, as run, aborts at `6ed414b2` on `m2h`'s anchor; re-anchored
by `…-landing.md` §11 "X5 at T" (extracted from that file and run on a fresh clone of `6ed414b2`):
`38 jobs`, 228 part copies, every copy clean under `bash -n`. The cells were not run. ⚠ Open: their
P-equal half needs the prototype-only P-dump hook, whose commit `7c07415c` is not reachable and whose
code is recorded nowhere (`…-landing.md` §11). `crc.sh`, patched by "X13 at T": clean under `bash -n`
on both shells, and its `w2rec` edit applies to the `6ed414b2` harness (both exits and `done`).

**`ci.yml` (#15).** The verdict sentence ("both sides far under it, so it is unchanged") was written
at `5a6f4367` (`git blame`), before C6–C12, and never measured on the code it named. It is replaced by
a pointer: no verdict there; the comment is rewritten at the final head (`…-landing.md` §9.1 step 2)
and the verdict is the runner's (design memo §3, "`ci.yml`"). Comment lines only, so T does not move.
The review's own local figures (head 17.05–18.48 s, base 12.68–12.86 s, alternated, three each) are
relayed, not re-measured, and decide nothing.

**Not done in this round, carried:** the code-comment leftovers the docs sweep found (the mutations
file's WHAT IT DEFINES omits `_mut_gen_floor`, defined in `mutgen.sh`); the P-dump hook above.

This file is 717 lines (`wc -l`) after this entry, under design memo §13's 800.

**`/code-review max` round 2, 2026-10-01.** On `92221297` (the /simplify round, whose 179 records and
reader changes are corpus §6.1's last pre-R2 row and totals): 15 findings, numbered as the review
orders them; the driver's dispositions R2-1…R2-15 follow that numbering. Code and records:
`96eea58c` (one commit, Part A); docs and comments: this commit (Part B, comment lines only in the
parts). Runs as the fix round's (both shells, own process group, perl `alarm`, `kill -9 -<pgid>`
watchdog, at most two at a time). Records: corpus §6.1, review R2 rows; 94 of this PR, 189 in all.

| # | finding (file:line at `92221297`) | fix |
|---|---|---|
| 1 | the pin's depth counter is reset by an `env -i PATH=…` re-exec: hang (harness :109) | one builder `_fgit_shim` writes the pin and the 8 fixtures shims; each drops its own dir from `PATH` before exec; counter, exit 125, `_pn_lbl` and its record gone; P-j's alias probe + strip record |
| 2 | BSD `find` skips an unsearchable dir in silence (:190) | census pass 1 also lists every dir not `u+rx`, red; record `zzm` |
| 3 | parent labels used only on red paths are never checked (controls :154) | every `_*_lbl` a part uses asserted defined before the harness loads (`_lbd_lbl`, `controls:` target); three inline labels moved to the label block |
| 4 | trust read from the absence of `post_bad` (:602) | `post_ok` written on return 0; trust needs it; "never reached a verdict" line; two records |
| 5 | P-f's presence check pinned by no record (:320) | record `export -n LC_ALL`; harness comment, design §4 P-f, corpus row corrected |
| 6 | `_seal`'s component check unpinned (:534) | record `walk/../walk/sub` |
| 7 | ratchet slack 1 (mutations :218) | `_MUT_UNRECORDED_MAX=20`, exact (fewer is red) |
| 8 | `xrefpoison` poisoned `clean` only (mutations :420) | every repo again; survives against the old fixed path |
| 9 | P-e's `!=` and P-k liveness's `-ne 1` unpinned (mutations :377) | records `GIT_EXEC_PATH=$_FGIT_VOID`, `alternates` → `/dev/null`; the `git init` record kept (else arm) |
| 10 | pre-fixtures options check and pass 1's NUL tail unpinned (mutations :342) | a 2-expression `nounset` record; an unterminated-duplicate record |
| 11 | corpus `15d3e704` row: "errexit backs the arms" is false (corpus :260) | row and PROGRESS caveat corrected: each record pins its arm |
| 12 | P-e's reference holds `HOME`, the comment and §4 say `PATH` only (:127) | harness comment and §4 row: why `HOME`, and no record |
| 13 | X4b prints harness :342 (`§0.1` on a line without the file name) | rejoined; X4b over the five parts prints nothing |
| 14 | the equivalence lookup passes the name by `awk -v`, which unescapes it (mutgen :328) | `ENVIRON` |
| 15 | stale descriptions (harness, mutations, mutgen, landing :31, design :794) | comments and docs corrected (below), except harness :595 |

Measured (Part A's logs, relayed; the driver re-verified `96eea58c` green with every record killed):
- **#1**: the `env -i` wrapper, before: no output, 5.3 rc 142 (alarm), 3.2 rc 137 (watchdog). After:
  strip-own-dir and `which -a` wrappers PASSED, the `env -i` one red with its cause in 9–10 s (it drops
  the window's variables); none hangs, both shells. Fixtures shims without the strip: a control's
  watchdog fires at 30 s (no record can separate them, the builder is shared). Strip record killed by
  P-j 2/2; alias check removed, needle absent 2/2.
- **#2**: `hide444` green before; record killed 2/2; clause removed, needle absent 2/2. `/usr/bin/find`
  is BSD (silent, exit 0, repo unlisted: measured); GNU is read from its manual, not measured.
- **#3**: rename + seal failure, before: 3.2 rc 0 (false green), 5.3 rc 1 unbound variable; after rc 2
  `used but not defined, or empty: _fws_lbl`, both shells. Record killed 2/2.
- **#4**: both records killed 2/2 (`never reached a verdict`); `post_ok` out of trust, needle absent 2/2;
  the line neutralised, needle absent 2/2 (silent exit 1).
- **#5, #6, #9, #10**: each new record killed 2/2 with its message; its clause removed, needle absent
  2/2 (#10's tail case: rc 0, PASSED).
- **#7**: one extra bare label: `21 labels … ratchet of 20`, rc 1; `MAX=21`: `only 20 … LOWER the
  ratchet`, rc 1; both shells.
- **#8**: killed 2/2 (every repo listed); against the old fixed path, needle absent 2/2.
- **#14**: `awk -v n='a\tb'` matches nothing, the `ENVIRON` form matches (BSD awk); no wire record.
- **Part B** (comment-only): the always-on wire on a `git clone --local` of `96eea58c` with this
  commit's files overlaid: 5.3.20 rc 0, PASSED, 17 s; 3.2.57 rc 0, PASSED, 23 s (the always-on record
  anchor check included). X4b over the five parts: empty.

**Carried, then closed by `3dde656e`:** harness :595 (now :615), the cause written to `$1/cause` when `cd "$1"`
fails, was unreachable (the directory it cannot enter is where it writes); below. The P-dump hook stays open.

**`3dde656e` and Stage 5 (`/elidex-review` on `3dde656e`: 0 CRIT / 1 IMP / 5 MIN, E-1…E-5), 2026-10-01.**
Code `3dde656e` and `8f4ad3c8`; docs (E-5) this commit. Runs as review R2's (both shells, own group, alarm, watchdog).
- **`3dde656e`:** the child no longer writes into the directory it failed to enter; the parent names
  that case from the directory (`the window's own directory was lost or cannot be entered`). Measured on
  5.3 and 3.2 with the window's directory mode 000 before `built` (its commit message). No record.
- **E-1 (IMP), the exit-trap slot, closed:** the orderly-end flag (design memo §3; closure text and audit
  `…-landing.md` §9.2). The wire's rc-0 ends: one, the fall-through to the PASSED line (`--selftest`'s
  green end included; no `exit 0` in the wire or a part outside the fixtures' shims and the watchdog
  subshell's TERM trap). R2-3's scenario: before 3.2 rc 0, 5.3 rc 1; `$?` captured first (rejected): 3.2
  rc 0; after 3.2 rc 1 with `CONTROL FAILED (the run ends only through a verdict)`, 5.3 rc 1. The flag's
  record killed with its needle on both shells; the two `SCRATCH=` records (`a relative scratch dir is
  removed on exit`, `a restrictive umask decides nothing`) re-run, killed on both. A fatal signal (the
  alarm) now also prints the label line; its rc is the signal's, unchanged.
- **E-2:** controls.sh names `#11-k2-wire-verdict-site-controls`. **E-4:** mutations.sh (981 lines) carries
  the rule that the edit taking it past 1000 lines first splits out the record table.
- **E-3:** a marker that exists but cannot be read is red under `$_fw_lbl` and the window untrusted (design
  memo §3). Before: rc 0 on both shells (a false green); after: rc 1 on both, for `seal_failed` and
  `fix_failed`. No record: class (c).
- **At `8f4ad3c8`:** always-on wire green, 5.3 (18 s) and 3.2 (24 s); X4b over the five parts and over the
  wire's added lines: empty; the K2P/K2REF grep: empty. Not run: the opt-in mutation run (its new trap
  line ran only in a 25 s alarm-bounded start). Records: 95 of this PR, 190 in all (corpus §6.1).
- **T is `8f4ad3c8`** (`…-landing.md` §9.1's command; this docs commit does not move it). Covered at T:
  the always-on wire on both shells, and the per-record runs above. **Still owed at T:** the whole
  final-head sequence of `…-landing.md` §9.1: step 1 (X2, X3 with the survive check and the plant and
  floor cells, X5, X6, X8, X11, X13, X14, X15, X16; X5's P-equal rows need the P-dump hook re-made and
  reviewed first, `…-landing.md` "X5 at T", which names five gaps the records leave), then steps 2–5.

This file is 797 lines (`wc -l`) after this entry, under design memo §13's 800.

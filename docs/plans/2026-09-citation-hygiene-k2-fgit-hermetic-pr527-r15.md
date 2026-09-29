# K2 fixture git — PR #527's review record, from plan-review round 15

This file continues `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md` ("`…-pr527.md`"),
PR #527's record, for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo").
`…-pr527.md` holds the record through plan-review round 14 and draft 16, and it is closed: at 890 lines
it has no room for round 15's record or for the implementation results, so both start here. The seam is
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

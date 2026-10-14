# K2 fixture git — the census, the boundary and the working files (P-g's population)

This file holds the second half of §4 ("Postconditions — run INSIDE the window") of
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"): from "P-g's population,
and the census before any git" to the end of §4 (the census and its two passes, the boundary by
property, the references and the working files, the parent's directory, the sealing after the census,
and what P-g cannot see). It was moved here unchanged as a touch-time split (S4c-0 of PR #527), before
an edit would have taken the design memo past 950 lines, as the design memo's §13 prescribed; the
design memo keeps a pointer in §4. The rest of §4 (the postcondition table P-a to P-k and what precedes
it) stays in the design memo. The section numbers are kept, so "§4" resolves in both files for its own
half. In the moved text "the table above" is the postcondition table in the design memo's §4. Every
other section reference without a file name is to the design memo, and "this memo" in the moved text
means the design memo, not this file.

---

## §0 Spec coverage map

**No spec surface** — this file holds the census of a shell-harness slice. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §4 Postconditions — run INSIDE the window (second half)

**P-g's population, and the census before any git (drafts 14–15).** The population is the git dirs
the census can **name**: a `.git` entry, or a directory holding a `HEAD` next to `objects` or
`commondir`. That is a classification by name and `HEAD`, not git's whole read set. What git reaches
through a reference is §5.2 R9's boundary (below). The census runs **in two passes, and writes its
verdict, one variable `_pg_census`, only after both.** The postconditions start with the census, and
when `_pg_census` is red, `_fgit_postconditions` reports P-g and returns before any other postcondition,
so no git runs. It is **one** census: P-g and P-k iterate pass 1's own list and act on its `.git`
entries (the fix round deleted a second population scan). It covers the git dirs under `$CTL` it names. It does not cover what git
reaches through a reference ("The boundary", below), nor the reference repos, which are outside fixture
reach by construction ("The references and the working files", below) except through §0.3's declared
class (c): they are made in a `mktemp -d` directory too, so a changed `mktemp` (`mkfn`, `shimafter`,
`staleshim`) can hand them to the fixtures:
1. **Classification and shape.** One `find` from `$CTL`, with no `-L` and no per-directory fork, lists
   every `.git` entry and every `HEAD` entry, of **any type and any letter case** (a case-insensitive
   filesystem lets git read `head` and `.GIT`, so a case variant is red). It descends everywhere:
   hidden and nested directories, and the inside of `.git` directories.
   - A `.git` that is a gitfile, a symlink or anything else but a directory is red: `[.git is not a
     directory]`.
   - A `HEAD` whose directory is not a `.git` and holds an `objects` directory **or** a `commondir`
     file is a git dir not named `.git`, and it is red. That covers a bare repo, a
     `--separate-git-dir` target, a submodule's git dir under `.git/modules/`, and a linked
     worktree's entry under `.git/worktrees/`.
   - A `.git` that is a directory has its **shape** checked: every entry under it must be a directory
     or a regular file with **one** link (`find <.git> \( \( -type f -links +1 \) -o \( ! -type f ! -type d \) \) -print`).
     A symlink of any target, a FIFO, a socket or a device is red, because git reads through a link
     and blocks opening a FIFO. So is a hard link, which shares its content with a path outside the
     git dir. A scan that fails or writes to stderr is red.
   - "Cannot search" is whatever `find` reports: any report on stderr (one check, after both passes),
     or a non-zero exit, fails the census, and so does a list that does not end in a NUL byte. A BSD `find` reports nothing for a directory it cannot enter (rc 0, no stderr, the repo beneath unlisted; measured, review R2), so the same `find` also lists every directory that is not `u+rx` (`! -perm -u=rx`), red as `[a directory the census cannot search]`; a GNU `find` reports one itself (read from its manual, not measured). That
     is one policy for every `read -d ''` reader (the census's two passes, P-f's `env -0`, `_pg_z`):
     after the loop, the remainder `read` left in its variable must be empty, else the list is
     red under the reader's own label. No command is run to ask. A list with no `.git` entry is red too, by P-g's count over it (`no fixture git dir was found`).
2. **The per-link search**, only if pass 1 found nothing red. A second `find` from `$CTL` lists every
   symlink. A link that resolves to a directory is searched through (`find -L`, any depth) whatever its name, and a `HEAD` or `.git` anywhere under it
   is red, as is a search that exits non-zero. BSD find lists a loop link without descending (rc 0,
   hiding nothing); GNU find is expected to exit 1, which is not measured here. A link *named* `HEAD`
   to a directory is red whatever it holds, because the search's start matches its own name; the
   record for that shape pins the routing to the search, and the two-levels-down record pins the
   depth.

   A link inside a git dir that pass 1 classified is already red by pass 1, so it is never searched.
   That holds for a `.git` by its shape, and for a bare dir by its name.

**If the census is red, no postcondition runs git**, on any repo: the census reports P-g, and the
build is untrusted (§3). Git reads **across** git dirs, through a `commondir`, an `alternates` chain or
an `include.path`, so both verdicts must be in before git runs anywhere.

Drafts 13 and 14 let git run before a verdict was in (round 12's `xcommon`, round 13's `xlink`); with
one verdict after both passes, both cells are red in 3–5 s on both shells (`…-pr527.md`, rounds 12–13).

**The boundary, by property (§5.2 R9).** Shape covers every git dir the census names. The census does
**not** follow `commondir`, `include.path` or `alternates`: that would emulate git's read set, the
anti-pattern this memo removed for `PATH` (§0.1: the window does not emulate the lookup). What git reads
outside a census git dir is therefore either **compared** or, if the read does not complete, **waited
on**; it is never green:
- **compared**, when the read completes. A reference that reaches configuration (`commondir`,
  `include.path`) puts what it read into `config --list --show-origin`, so P-g's origin comparison reds
  it (`xincreg`, an include naming a regular file with a key: red in 6–8 s, origin
  `file:.git/../../zzi`). A read that fails is red by one of two routes. An include git cannot open
  (a mode-000 file; `/dev/tty` with no controlling terminal) fails the listing, git exit 128, red. An
  include naming a file that does not exist is skipped silently (rc 0, nothing on stderr), but the
  listing keeps the `include.path` line itself, an entry a plain init does not write, so P-g reds it
  as an extra key (X6's include; `…-pr527-r15.md` §Q). An `alternates` naming an
  ordinary store outside leaves that listing unchanged (measured, `…-pr527.md` round 14), so P-g does
  not see it; **P-k** does (`count-objects -v` prints its `alternate:` line);
- **waited on**, when the read does not complete. The run waits, in CI until the job timeout (red).
  Measured on both shells (inserts in `…-pr527.md`, rounds 13–14). Four cells reached `cell15.sh`'s
  120 s alarm with the P-g `git … config --list` blocked in the run's group: `hdless` (a `commondir`
  naming a directory with `objects` and `refs` but no `HEAD`, whose `config` is a FIFO), `xinclude` (an
  include naming a FIFO in the fixture root), `xinclnk` (an include naming a link in the fixture root to
  a FIFO outside it) and `outroot` (a `commondir` naming a git dir outside the fixture root whose
  `config` is a FIFO). `ttyinc` (an include naming `/dev/tty`, under a pseudo-terminal that is the
  wire's controlling terminal) reached that harness's 120 s limit, and `ps` showed the `config --list`
  blocked (bash 5.3). A chained `alternates` whose store's own `alternates` is a FIFO blocks
  `count-objects -v`, P-k's read (a git-only measurement, its setup verbatim in `…-pr527.md` round 14);
- the per-link search over an otherwise clean census reads whatever a link reaches, so a link to a huge
  tree is **slow**: `find` ends when it has walked what the link reaches. Round 12 reported 455 s for
  its `outsidein` cell, whose search went through a link to `/` (`…-pr527.md`, round 12). Draft 15's
  census reds that cell by pass 1 in seconds; a link to `/` over a clean pass 1 has not been
  re-measured.

**The references and the working files, in verifier directories made after the build (drafts 16–18).** At
`8413a4db` P-a's probe repo `a` and P-d's second init `b` are made at the fixed path `$_FW_DIR/pq`,
which the fixtures can write, and `git init` over an existing repo keeps its configuration. Round 14's
`xrefpoison` made both in advance with `core.excludesFile /nonexistent-k2` and wrote the same key into
every fixture repo: rc 0, PASSED, on both shells. Every reference a postcondition compares against, and
every file it writes and reads back, by location:
- **the postconditions' directory**, made at the start of the postconditions, **after the build**, by
  `mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX"`, beside `$_FW_DIR` and never under it or `$CTL`. No fixture
  command that finished by the end of the build can know its name, unless the fixtures file changed
  what the `mktemp` the postconditions run resolves to (`mkfn`, `shimafter`, `shimhash`: class (c),
  §0.3). It holds `a`, `b`, P-g's reference
  listings and P-k's probe repo (C10), and every working file below. `a` and `b` are made by `mkdir`
  without `-p` after a clean census, so one that already exists fails the reference, red (P-g). P-a,
  P-b, P-d, P-g and P-i read `a`. A fixture command still running after the build (a background
  process) can find and write the directory; that writes the verifier's own state, class (c) (§0.3,
  `xrace`).
- **P-e's `_FGIT_WIRE_EXEC`, P-h's `_FGIT_WIRE_LC`, P-f's `_FGIT_ENV_REQ`**: values the parent takes
  before the window starts and passes as prelude assignments, not files. The fixtures file shares the
  postconditions' shell, so it could assign one, or redefine a postcondition function; that is a write
  to the verifier's own state, class (c) (§0.3). No fixture names any of them (`/usr/bin/grep -c -e _FGIT_WIRE_EXEC -e
  _FGIT_WIRE_LC -e _FGIT_ENV_REQ -e _fgit_postconditions` over the fixtures file → 0).
- **P-j's `$_FGIT_BIN/git`** is compared by path. Its content decides which executable runs, R1's; a
  fixtures-file write to it is class (c) (§0.3).
- **P-c's void** is the subject, not a reference: P-c reds any entry in it, and both reference inits
  read it as their template, so a template planted there is red through P-c (X6).

**The working files, by property (D15-A).** Draft 16 left them at fixed names in `$_FW_DIR`, and a
fixture-placed file there *can* make a comparison equal: a pass-2 link list or per-link search result
replaced by a link to `/dev/null` reads as empty, so a link to a tree holding a git dir passed
(round 15's `lnblind2`, rc 0, PASSED on both shells; red with the working files moved). So the set is
defined by property, not listed by name: **every file a postcondition writes and then reads back** is
in the postconditions' directory. Derived from the prototype by that property (`…-pr527-r15.md` §Q
gives the command), it is the census's two lists with their stderr, pass 2's per-link result, P-f's
`env -0` output, P-g's population with its stderr and failure mark and in-loop per-link result
(both deleted since, by C9 and the fix round) and its current listings. `$_FW_DIR` keeps only what both sides name:
the two sourced files, the seal manifest `seal` (fixture output by design, "sealing" below) and the
return channel the parent reads (`built`, `done`, `cause`, `post_ok`, `post_bad`, `machine_limits`,
`seal_failed`, `fix_failed`, `stderr`). A fixtures-file write to the return channel is class (c) (§0.3,
§5.1; `xdone`, D14-D).

**The parent's directory (D16-B, C11).** `$CTL` is the fixtures' tree, and after C11 the parent keeps
none of its own working files there (what a control's run does inside a fixture repo is that control's
subject, not a working file). At `8413a4db` the parent wrote its own working files at fixed names directly under
`$CTL`, and a fixture-placed link there could turn a check green: `ln -s /dev/null "$CTL/.bare"` with the
label ratchet at 0 (`xbare`), and `ln -s /dev/null "$CTL/.anchor"` with a stale record (`xanchor`), each
rc 0, PASSED on both shells (round 16's cells, as relayed); with the files moved, rc 1 with the ratchet's own message
(`…-pr527-r15.md` §Q). C11 makes one directory right after `_fgit_window` returns,
`mktemp -d "$SCRATCH/verifyXXXXXX"`, outside `$CTL` and `$_FW_DIR`, and a failure to make it ends the run
"decided nothing" (exit 2), as a failed `$CTL` does. The directory is out of the fixtures' reach by
construction **except** through the declared class (c) of §0.3: an executable the fixtures wrote into a
caller-`PATH` directory can answer that `mktemp` with a directory they prepared (`staleshim`, D18-A). The mutation set's entry guard (`mutations.sh` line 90 at
`8413a4db`) and the two "WHAT IT CONSUMES" headers (`mutations.sh` line 29, `mutgen.sh` line 12) name
`$_VFY` instead of `$CTL`: after C11 neither file reads `$CTL` outside the records' text, and the
controls file sets `_VFY` before it sources the mutation set (the draft-19 prototype, `…-pr527-r15.md`
§Q). C11
also makes **an empty generated set red** (D17-D), and **always on** (D18-B): after the move no
fixture plant can empty the set, but any other cause would print `0 mutant(s)` beside `0 neither
killed nor argued equivalent`, which X3 reads as a pass, the silent direction. It takes **two
layers** (D19-A), because the count X3 reads is not the generator's output but what `_mut_gen_run`
made of it (written to `.genmutants`, read back, split on TAB):
- **the generator's output**, a static property of `_mut_regex_mutants` over the running wire's
  `$K2RE` and `$K2RE_PATH`: by `_mut_correspondence`'s own rule (static properties go where every run
  sees them) it is checked there, always on, by `_mut_gen_floor` in `mutgen.sh`, under its own label
  (`the boundary-mutant generator derives a non-empty set from the wire's regexes`), and a record pins
  it (corpus §6.1). That needs `mutgen` as a record target beside `harness` and `fixtures`;
- **`_mut_gen_run`'s path**, checked at run time in the opt-in run: `_mut_run`, right after it calls
  `_mut_gen_run`, requires `_mut_gen_n` to equal the floor's count (`_mut_gen_floor_n`, which
  `_mut_gen_floor` sets) and to be non-zero, and otherwise prints `!! the generated boundary set ran N
  mutant(s), but the generator derives M from the wire's regexes: this run did not test every rule.`
  and counts one more in `_mut_gen_bad`. It sits in the caller, so an early `return` inside
  `_mut_gen_run` does not skip it. No record can reach it (a trial runs with `WEBREF_WIRE_MUTANTS`
  unset), so X3's floor cell pins it (`…-landing.md` §11). Draft 20 had only the first layer, and
  round 19's cells `RC` (the generator's output sent to `/dev/null` inside `_mut_gen_run`) and `RA`
  (`_mut_gen_run` returning at once) passed with `0 mutant(s)`; with both layers both are red
  (`…-pr527-r15.md` §Q). What each moved file did, at `8413a4db`:
- a plant turned a check green: `.bare` and `.anchor` (the always-on ratchet and anchor check), the
  generator's `.genmutants`, `.genequiv` and `.genseen` (opt-in mutation mode; `ln -s /dev/null
  "$CTL/.genmutants"` makes the generator read no mutant, and the run reports `0 mutant(s)` and
  passes: measured at `8413a4db`, rc 0, PASSED, `…-pr527-r15.md` §Q), and `.fsmonitor_ran`
  (the fsmonitor control's mark: a pre-planted mark stands in for the hook's run under a plain git
  call, so where the hook does not run that control's NOT EXERCISED, a red, becomes a pass);
- a plant made a check fail, red: `.mutants`, `.control_out` (the umask run is a `_control` now and writes there too);
- display only: `.fsm_out`;
- not reachable: `.fifoprobe`, made and removed at source time before the window runs. It moves to
  `$SCRATCH` so that the checker below lists no working file without an exception;
- `fsmhook`, the hook the fsmonitor control installs, moves with its mark.

The checker is static, over the six parent parts: every write, `mkfifo` or assignment of a path under
`$CTL`:

```sh
/usr/bin/grep -nE '(>|>>|[A-Za-z_]=) *"\$CTL/|mkfifo "\$CTL/' .claude/tools/webref-generic-core-trip-wire.{controls,harness,ctlrun,mutations,mutgen,records}.sh
```

With the five parts it had then (no `ctlrun`), it lists 16 lines at `8413a4db` and two at `e271cb02`; the six-part command lists the same two at `20615255`: `_seal_apply_probe`'s `: > "$CTL/real/f"`
(since `6ed414b2`), where `CTL` is the probe's own, rebound in its subshell, and the fsmonitor block's
`_fsm_root="$CTL/fsmon"` (since `2d5c393f`), a fixture the block only gates on, enters and runs over. It sees only literal
`"$CTL/…"` paths, so a path built another way is outside it. No fixture needs these names: every `ln -s`
in the fixtures file makes a link inside a fixture directory, and none names a dot-file directly under
`$CTL` (`/usr/bin/grep -nE '"\$CTL/\.'` over the fixtures file → nothing). So class (b) never reached
them; the move removes the class-(c) form of a command that finished during the build, and leaves
`xrace`'s, a process that outlives it, declared.

**Nothing is unsearchable at census time, by construction (sealing after the census).** Three
fixtures need a mode restriction: `walk/sub` and `d5root` mode 000, `d2red/sub` mode 0444 (and the file
`err/control.py` mode 000). They ask for it through `_seal <path> <mode> <fixture>`, which only appends
to a manifest in the window's directory — the path relative to `$CTL`, and refused (red, W4, and the
fixture marked failed) if it lies outside `$CTL`, holds a newline or TAB, or has a `.` or `..`
component, or if its mode is not octal or its fixture name not `[A-Za-z0-9_-]+` (refused as `(seal)`); `_seal_apply` also refuses a path with a symlink anywhere on it (`chmod` follows symlinks),
so no seal can reach outside the scratch root, and it skips a fixture whose chain already failed, so
one failure is reported once; the window applies the manifest **after** the postconditions,
and a `chmod` that fails marks that fixture failed **and is red under W4**: the controls gated on the
mode having taken effect would otherwise be skipped as a machine limitation (and they gate on an
independent probe, `_perm_ok`, never on a fixture's state). So the census
reads the whole tree, and a directory it cannot read is a restriction nobody sealed: red. Draft 9
**pruned** these directories, and a nested repo under `walk/sub`, gitlinked into `walk`'s index with an
outside `core.excludesFile`, passed with P differing (round 9, Ax3); p11 opened and restored them
(companion §A.13). Sealing replaces the declared list, the mode check and the open/restore loop.

An accidental `--separate-git-dir` is class (b): it leaves a gitfile, so it is red.

Draft 8's p8 walked `$CTL/*/` and skipped any fixture whose `.git` was not a directory. A
`--separate-git-dir` with a persisted `core.excludesFile` was **silently wrong** there (rc 0, P
differs). This author's intermediate p9 still walked only the top level, and it missed a nested repo
and a hidden one. Both were measured (corpus §6).

- **Census:** today every fixture git dir's configuration equals the reference on both shells, `find`
  reports nothing, no fixture git dir holds anything but regular files and directories (the M-SHAPE
  prototype's clean run is green on both shells, `…-pr527.md`), and the three declared directories are at their modes, so the clean and G cells
  are green (corpus §6). Legitimate fixture state (`badref`'s ref, `notcommitted`'s `info/exclude`, the
  per-call identity) is not configuration, so it stays green.
- **Cost:** measured as the census section's wall time on a clean tree (instrumented copies, companion
  §A.13): **0.76 s** on p11 against 1.45 s on p10, on both shells. Most of p11's figure is the one
  `git config --list` per fixture git dir. p10's extra time came from one `sh -c` per directory, which
  mutation mode would have paid about 115 times. X8 records the whole cost locally; the budget verdict
  is X9's (§3, "`ci.yml`").
- **What P-g cannot see:**
  - class (c) (§0.3): transient inputs, persist-then-revert, a git dir outside the fixture root;
  - **anything outside configuration and shape**, which is P-g's scope; the *content* of a regular
    file inside a git dir is not compared. Examples, not a list: hooks, `info/attributes` and
    `info/exclude` that a fixture writes, which are produced inside the window by definition and
    harmless unless configuration names them. An object store outside the repository is P-k's (table
    above). No fixture uses one today (`/usr/bin/grep -n -i -e alternate -e shared -e 'clone'` over the
    fixtures file gives three lines, 408, 666 and 667 at `8413a4db`, all comments);
  - a fixture that legitimately needs extra config. That one reds, and needs a deliberate change to
    the reference, which is the fail-safe direction.

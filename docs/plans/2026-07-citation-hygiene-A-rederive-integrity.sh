# The re-derivation harness's INTEGRITY MACHINERY — sourced FIRST by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options.
# It does resolve `$REPO_ROOT` at source time (below), because every other part
# and the dispatcher itself depend on that being settled before anything runs.
#
# What lives here is ONE COHESIVE UNIT AND BELONGS TO NO SLICE, which is the seam
# A-i §8 names: the primitive that makes a failed measurement UNREPRESENTABLE AS
# A PASS (`_measure` / `_measured`), the repo root every scan and every `git show`
# resolves against (`$REPO_ROOT`), and the check that every block on `all`'s
# roster STATES its own exit status (`selfcheck`) -- which is the same property
# one level up, and reads `_measure`'s limits as its own premise. Every part but
# `-Ai` calls `_measure` (per-part counts in the dispatcher's header, measured);
# `-common.sh` holds the blocks more than one memo cites.

# THE REPO THIS HARNESS LIVES IN, derived from THIS FILE's own path -- never from
# cwd. `_wtscan`'s roots are relative (`.claude/tools/`), so before this they
# resolved against whatever directory the caller happened to be standing in: the
# dispatcher used to `cd "$(git rev-parse --show-toplevel)"`, and that `cd`
# NO-OPS when the substitution fails. Measured, with a violation planted on the
# branch: invoked with cwd `/` the `cd` printed `fatal: not a git repository`,
# did nothing, both counts came back 0 and `couplings` printed VERDICT: GREEN;
# invoked from a SIBLING worktree it audited that worktree instead of this one.
# Both are the drift `memory/feedback_worktree-cwd-drift.md` records. Failing to
# resolve the root is now fatal rather than silent.
REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && git rev-parse --show-toplevel) || {
  printf 'FATAL: cannot resolve the repo root from %s\n' "${BASH_SOURCE[0]}" >&2
  printf '       -- run the harness from a checkout of the repo it lives in.\n' >&2
  exit 2
}

# --- THE MEASUREMENT PRIMITIVE ------------------------------------------------
# EVERY quantity this harness reports goes through here. The umbrella's `:91`
# constraint is *"Counts are commands. No slice memo carries a quantity it did not
# derive"*, so the one thing this harness must not be able to print is a count
# whose command NEVER RAN. The idiom it replaces -- `n=$(cmd | wc -l)` -- could
# not tell that apart from a real zero on EITHER axis:
#
#   STATUS  `$(...)` discards the pipeline's status and `wc -l` of nothing is
#           `0`, which is the PASS condition at every call site here. Measured,
#           in a checkout with no remote-tracking ref: `git grep … origin/main`
#           died `fatal: unable to resolve revision: origin/main`, the count read
#           `0`, and `couplings` printed `VERDICT: GREEN` and exited 0 -- it
#           certified a derivation that did not occur, which is the negation of
#           this harness's charter. `37c7eb02` fixed exactly this for `_wtscan`
#           and left the two `git grep` baselines on the adjacent lines.
#   EMPTY   `printf '%s\n' "$var" | wc -l` is **1** for an empty `$var`, because
#           printf emits the newline unconditionally. A match count that reads 1
#           when there are no matches is the same class from the other side. The
#           `if [ -n … ]` guards that used to hold this off are not needed here:
#           the count is taken from the command's own output, not from a re-print.
#
#   _measure [--nomatch <status>] <var> <cmd> [arg...]
#
# runs <cmd> ONCE, and:
#   * captures its stdout in `$_MEASURE_OUT`, so a caller that must also PRINT
#     the hits reads that instead of running the command a second time (running
#     it twice is how the pre-`37c7eb02` sites threw the status away twice, and
#     it lets the listing and the count disagree),
#   * on success sets <var> to `wc -l` of that output -- 0 for no output,
#   * on failure sets <var> to `!FAILED(rc=N)`, prints a loud diagnostic, and
#     returns 1.
#
# The sentinel is what makes "the command did not run" UNREPRESENTABLE AS A PASS
# rather than merely detected at the sites someone remembered: every gate in this
# harness is `= 0`, `!FAILED(rc=N)` is not `0` and is not a number, so no failed
# measurement can satisfy any of them -- including at a call site written later by
# someone who has not read this comment. Callers should still propagate the
# return status so the block's exit code carries it too; a caller that forgets
# still cannot print GREEN.
#
# `--nomatch <status>` is the status a SEARCH returns when it ran and matched
# nothing (`git grep` -> 1). It is REQUIRED on the baselines and forbidden
# elsewhere: without it "no matches" -- the expected, correct result -- would be
# indistinguishable from a broken ref, which is the confusion this function
# exists to end. Every OTHER status stays a failure, so git's 128 for an
# unresolvable revision and 127 for a missing interpreter are still fatal.
_MEASURE_OUT=""
_measure() {
  local __nomatch=""
  [ "${1:-}" = "--nomatch" ] && { __nomatch=$2; shift 2; }
  local __var=$1; shift
  local __raw __rc=0
  # The `\034` sentinel is not decoration: `$(...)` strips ALL trailing newlines,
  # so output ending in a blank line would count one line short. Appending a
  # non-newline byte and stripping it back makes the count byte-identical to
  # `cmd | wc -l`.
  __raw=$( { "$@"; __r=$?; printf '\034'; exit "$__r"; } ) || __rc=$?
  __raw=${__raw%$'\034'}
  if [ "$__rc" -ne 0 ] && [ "$__rc" != "$__nomatch" ]; then
    _MEASURE_OUT=""
    printf '!! MEASUREMENT FAILED (rc=%s): %s\n' "$__rc" "$*" >&2
    printf '!!   no count is reported -- a command that did not run measured nothing.\n' >&2
    printf -v "$__var" '!FAILED(rc=%s)' "$__rc"
    return 1
  fi
  _MEASURE_OUT=$__raw
  printf -v "$__var" '%s' "$(printf '%s' "$__raw" | wc -l | tr -d ' ')"
  return 0
}

# Print what the last `_measure` captured, without the extra blank line a
# `printf '%s\n'` on an already-newline-terminated capture would add.
_measured() { [ -n "$_MEASURE_OUT" ] && printf '%s' "$_MEASURE_OUT"; return 0; }

selfcheck() {  # THE HARNESS AUDITED BY THE HARNESS — every `all` block STATES its status
  # `_measure` makes a failed measurement unrepresentable as a pass AT THE CALL
  # SITES THAT USE IT, and nowhere else. That is why `5abe729e`'s sweep -- which
  # was scoped to the `git`-shaped and `subprocess.run`-shaped sites -- left
  # `ruleset`'s three `gh api` calls behind, for Codex to find as the SIXTH
  # instance of one class. Routing the sixth site fixes the site; it does not make
  # the seventh detectable.
  #
  # WHAT IS CHEAPLY DETECTABLE is not "an un-routed measurement" (that needs to
  # know which commands are measurements, which is a taste judgement no regex
  # holds) but its CONSEQUENCE, which every instance so far has shared: THE
  # BLOCK'S EXIT STATUS WAS AN ACCIDENT OF ITS LAST LINE. `ruleset` returned the
  # third `gh api`'s; `suiteset` returned an `echo`'s; `column`, `carvecolumn`,
  # `instruments`, `reloadstale` and `remedies` returned `rm -rf`'s; `bmemo`
  # returned whichever way its eleventh grep happened to fall; `suites`'s own
  # comment records the same defect as its cause (3). A block that must END IN AN
  # EXPLICIT `return` cannot have an accidental status: the author has to write
  # down what the block's verdict IS, and that is the moment the missing
  # measurement is visible. So this block enforces exactly that, over the roster
  # DERIVED FROM `all` -- not a second list, which would drift from the first.
  #
  # ⚠ WHAT IT DOES NOT CATCH, stated plainly so nobody reads more into a green:
  # a block ending in a hardcoded `return 0` while discarding a measurement
  # mid-body passes this check. It is a forcing function at the one place every
  # instance surfaced, not a proof that every quantity was derived. The proof
  # obligation still sits with `_measure` at each call site.
  #
  # Not routed through `_measure`, deliberately: `_measure` reports a COUNT and
  # CLEARS `$_MEASURE_OUT` on failure, and here the failure output -- which blocks
  # and what they end on -- is the whole answer. The python below carries its own
  # did-it-run guards instead (no parts found, roster unreadable), which is the
  # same property by the same argument.
  python3 - "$REPO_ROOT/docs/plans" <<'SELFCHECKPY'
import pathlib, re, sys

D = pathlib.Path(sys.argv[1])
DISPATCH = D / "2026-07-citation-hygiene-A-rederive.sh"
parts = sorted(D.glob("2026-07-citation-hygiene-A-rederive*.sh"))
if len(parts) < 2:
    raise SystemExit("!! found %d harness part(s) under %s; a check that read no file "
                     "reports no problem for a reason that is not 'there are none'."
                     % (len(parts), D))

m = re.search(r"^all\(\) \{ set -- (.*?)\n\s*local failed",
              DISPATCH.read_text(encoding="utf-8"), re.S | re.M)
if m is None:
    raise SystemExit("!! cannot read `all`'s roster from %s; this check would then range "
                     "over nothing and pass." % DISPATCH.name)
roster = m.group(1).replace("\\\n", " ").split()

DEF = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\(\) \{")
HEREDOC = re.compile(r"<<-?'([A-Za-z_][A-Za-z0-9_]*)'")
# `^` or after a `;`/`&&`/`||`: the last thing the block does is hand back a status.
RETURNS = re.compile(r"(?:^|[;&|]\s*)(?:return|exit)\b[^;]*;?\s*$")


def blocks(path):
    """(name, lineno, body) for column-0 definitions, with heredoc BODIES dropped
    so a python payload is never parsed as shell."""
    out, name, start, body, term = [], None, 0, [], None
    for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if term is not None:                       # inside a heredoc payload
            if line.strip() == term:
                term = None
            continue
        d = DEF.match(line)
        if d and name is None:
            if line.rstrip().endswith("}"):        # one-liner
                out.append((d.group(1), i, [line[line.index("{") + 1:].rsplit("}", 1)[0]]))
            else:
                name, start, body = d.group(1), i, []
                h = HEREDOC.search(line)
                if h:
                    term = h.group(1)
            continue
        if name is not None:
            if line == "}":
                out.append((name, start, body))
                name = None
                continue
            body.append(line)
        h = HEREDOC.search(line)
        if h:
            term = h.group(1)
    return out


def uncomment(s):
    """Drop a trailing `# ...`, quote-aware, so `return "$rc"  # why` still reads
    as a return. A `#` inside quotes -- every grep ERE in this harness has one --
    is not a comment."""
    q = None
    for i, ch in enumerate(s):
        if q is not None:
            if ch == q:
                q = None
        elif ch in "'\"":
            q = ch
        elif ch == "#" and (i == 0 or s[i - 1].isspace()):
            return s[:i]
    return s


defined, bad = {}, []
for path in parts:
    for name, lineno, body in blocks(path):
        defined[name] = (path.name, lineno)
        if name not in roster:
            continue
        last = ""
        for raw in reversed(body):
            s = raw.strip()
            if s and not s.startswith("#"):
                last = s
                break
        if not RETURNS.search(uncomment(last).rstrip()):
            bad.append((path.name, lineno, name, last[:64]))

for name in roster:
    if name not in defined:
        bad.append((DISPATCH.name, 0, name, "<dispatched by `all` but defined nowhere>"))

print(f"  {len(parts)} harness parts, {len(defined)} blocks, {len(roster)} on `all`'s roster")
for fn, lineno, name, last in sorted(bad):
    print(f"  !! {fn}:{lineno} {name}: ends on {last!r}")
if bad:
    print(f"  !! {len(bad)} block(s) whose exit status is their LAST LINE'S rather than")
    print("  !! a statement about what they measured. End each in an explicit `return`.")
    sys.exit(1)
print("  VERDICT: GREEN — every roster block states its own status")
SELFCHECKPY
  return $?
}

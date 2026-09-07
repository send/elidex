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
# The part SET is derived from the dispatcher's own `for _part in …` line and
# compared with what is on disk: a part on disk the dispatcher does not source,
# or a sourced part missing from disk, is RED. A-i §13.1's harness-part count is
# then a reading of this equality, not a number this check was asked to believe
# (`len(parts) >= 2` was all it asserted -- the block-audit of 2026-08-22).
_src = DISPATCH.read_text(encoding="utf-8")
_m = re.search(r"^for _part in ([a-zA-Z ]+); do$", _src, re.M)
if _m is None:
    raise SystemExit("!! the dispatcher has no `for _part in …; do` line; the part set cannot be derived")
_sourced = {DISPATCH} | {D / ("2026-07-citation-hygiene-A-rederive-%s.sh" % p) for p in _m.group(1).split()}
if set(parts) != _sourced:
    raise SystemExit("!! part set on disk != part set the dispatcher sources:\n   disk only: %s\n   sourced only: %s"
                     % (sorted(p.name for p in set(parts) - _sourced), sorted(p.name for p in _sourced - set(parts))))

# The roster is the `BLOCKS="…"` assignment (one list: `all` dispatches it,
# the entry point admits only it -- Codex R32); read that, not `all`'s body.
m = re.search(r'^BLOCKS="(.*?)"', DISPATCH.read_text(encoding="utf-8"), re.S | re.M)
if m is None:
    raise SystemExit("!! cannot read the `BLOCKS=` roster from %s; this check would then range "
                     "over nothing and pass." % DISPATCH.name)
roster = m.group(1).replace("\\\n", " ").split()
if not roster:
    raise SystemExit("!! the `BLOCKS=` roster is empty")

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
            bad.append((path.name, lineno, name, "ends on %r" % last[:64]))

for name in roster:
    if name not in defined:
        bad.append((DISPATCH.name, 0, name, "dispatched by `all` but defined nowhere"))

# No `python3 -c '…'` payload may exist: the shell wraps `-c` in single quotes,
# so one apostrophe inside ends the program mid-line (R15/R16), and the guard
# that read payloads "as bash does" mis-found an end and reddened sound blocks.
# The harness idiom is the quoted heredoc (`python3 - <<'PY'`), which has no
# such hazard; a `-c` payload is a defect of idiom, not a thing to parse.
# THE IDIOM, as a complement: a python payload is a QUOTED heredoc. Any `-c`
# payload (either quote, `python` or `python3`) and any UNQUOTED heredoc
# delimiter (`<<PY`, `<<EOF` -- the shell expands `$`/backticks inside) is a
# defect of idiom, not a thing to parse (second re-gate of #501: the guard had
# banned one spelling). Built from fragments so this line does not match itself.
C_PAYLOAD = re.compile("python3?" + " -c" + " [" + chr(39) + chr(34) + "]")
BARE_HEREDOC = re.compile("<<" + "-?" + "\\s*[A-Za-z_]")
for path in parts:
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if line.lstrip().startswith("#"):
            continue
        if C_PAYLOAD.search(line):
            bad.append((path.name, n, "<python -c payload>", "use the quoted-heredoc idiom"))
        if BARE_HEREDOC.search(line):
            bad.append((path.name, n, "<unquoted heredoc>", "quote the delimiter: <<'X'"))

# EVERY `rederive <name>` THE MEMOS DOCUMENT MUST BE REACHABLE. Codex R52: the
# guard admitted `$BLOCKS + $AUTHOR_LOCAL + all`, so `rederive readers <symbol>`
# -- the workflow A-i §4.2 tells an author to run, whose own function prints that
# usage string -- exited 2 as an unknown block. The population is not a word list
# of block names (the next unreachable name would not be on it) but every
# `rederive <name>` occurrence in the memos this checkout carries, split by a
# DERIVED predicate: a name DEFINED in a part on disk must be admitted by the
# dispatch guard, and a name defined in no part on disk belongs to a departed
# slice and must cite the part file that holds it, so the reader is not left to
# guess (`rederive partition`, A-i §11, names `…-A-rederive-B.sh`).
_admit = {}
for _set in ("BLOCKS", "AUTHOR_LOCAL", "PARAMETERIZED"):
    _mm = re.search(r'^%s="(.*?)"' % _set, "\n".join(q.read_text(encoding="utf-8") for q in parts), re.S | re.M)
    if _mm is None:
        raise SystemExit("!! cannot read the `%s=` set; this check would range over a short "
                         "admitted set and redden sound names." % _set)
    _admit[_set] = _mm.group(1).replace("\\\n", " ").split()
admitted = set(_admit["BLOCKS"]) | set(_admit["AUTHOR_LOCAL"]) | set(_admit["PARAMETERIZED"]) | {"all"}
_overlap = set(_admit["BLOCKS"]) & set(_admit["PARAMETERIZED"])
if _overlap:
    bad.append((DISPATCH.name, 0, "<parameterized name on `all`'s roster>",
                "`all` dispatches zero-arg: %s" % sorted(_overlap)))
PART = re.compile(r"2026-07-citation-hygiene-A-rederive[-A-Za-z]*\.sh")
_memos = sorted(D.glob("2026-07-citation-hygiene-*.md"))
if not _memos:
    raise SystemExit("!! no memo found under %s; the reachability check would range over nothing." % D)
# ENUMERATE per line, JUDGE per paragraph. The citation is prose: a reader reads
# the block of contiguous non-blank lines, and a filename that wraps onto the next
# line is no less present for it. A per-line predicate reddened this memo's own
# §15 for a sentence whose filename sat one line down -- it was testing line
# adjacency, not whether the reader is told where the block lives. Every
# occurrence is still counted; only the scope the predicate reads widens.
_cited = 0
for _memo in _memos:
    _lines = _memo.read_text(encoding="utf-8").splitlines()
    _para_of, _para_text, _start = {}, [], None
    for i, line in enumerate(_lines, 1):                    # contiguous non-blank run = one paragraph
        if line.strip():
            if _start is None:
                _start = i
            _para_of[i] = _start
        else:
            _start = None
    _para_body = {}
    for i, line in enumerate(_lines, 1):
        if i in _para_of:
            _para_body[_para_of[i]] = _para_body.get(_para_of[i], "") + line + "\n"
    for n, line in enumerate(_lines, 1):
        for _name in re.findall(r"rederive ([a-z_][a-z0-9_]*)", line):
            _cited += 1
            if _name in defined:
                if _name not in admitted:
                    bad.append((_memo.name, n, _name,
                                "defined in %s but the dispatch guard does not admit it" % defined[_name][0]))
            elif not PART.search(_para_body.get(_para_of.get(n, n), line)):
                bad.append((_memo.name, n, _name,
                            "defined in no part on disk and its paragraph names no `-A-rederive-*.sh` file"))

print(f"  {len(parts)} harness parts, {len(defined)} blocks, {len(roster)} on `all`'s roster, "
      f"{len(admitted)} admitted names, {_cited} documented `rederive` invocations over {len(_memos)} memo(s)")
for fn, lineno, name, why in sorted(bad):
    print(f"  !! {fn}:{lineno} {name}: {why}")
if bad:
    print(f"  !! {len(bad)} integrity defect(s). A block whose status is its LAST LINE'S rather")
    print("  !! than a statement about what it measured must end in an explicit `return`; a")
    print("  !! documented `rederive <name>` must be reachable or say where it lives.")
    sys.exit(1)
print("  VERDICT: GREEN — every roster block states its own status, and every documented")
print("  invocation is reachable")
SELFCHECKPY
  return $?
}

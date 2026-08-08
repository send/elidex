# The re-derivation harness's INTEGRITY MACHINERY — sourced FIRST by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options.
# It does resolve `$REPO_ROOT` at source time (below), because every other part
# and the dispatcher itself depend on that being settled before anything runs.
#
# What lives here is ONE COHESIVE UNIT AND BELONGS TO NO SLICE, which is the seam
# A-i §8 names: the primitive that makes a failed measurement UNREPRESENTABLE AS
# A PASS (`_measure` / `_measured`), the repo root every scan and every `git show`
# resolves against (`$REPO_ROOT`), and the two checks that range over the harness
# AS A WHOLE -- `selfcheck` (every block on `all`'s roster STATES its own exit
# status) and `inventory` (every block is routed to the slice that consumes it).
# Every part but `-Ai` calls `_measure` (per-part counts in the dispatcher's
# header, measured); `-common.sh` holds the blocks more than one memo cites.
#
# ⚠ `inventory` MEASURES this file's claim to be kernel and does not confirm it.
# Do not restate the answer here -- an earlier revision of this comment named A-i,
# which the umbrella-first ranking landed in the SAME commit had already made wrong.
# Run `rederive inventory` and read the rows whose `part` is `integrity`.

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

inventory() {  # THE BLOCK TABLE, DERIVED — who defines it, who declares it, where it ships
  # WHY THIS IS A BLOCK AND NOT A TABLE IN A MEMO. The design note's §2 and §3
  # are this block's output. Hand-written, the same two tables came out with 22
  # rows against a roster of 22 that were DIFFERENT SETS of 22 -- equal
  # cardinality is exactly what no count-based check catches -- and a routing
  # table read off prose sent two of A-ii's own blocks into Slice B's column.
  #
  # WHY `declare -f` AND NOT A REGEX. Bash parses bash. `declare -f` also STRIPS
  # COMMENTS, so every code signal below is a property of the code rather than of
  # the prose beside it, which is the distinction the note's Q1 turns on: a regex
  # over the raw text made `marker` a caller of `_measure` because its python
  # payload quotes the name in a comment. It is also why this block sees `all()`,
  # which closes with `; }` and which `selfcheck`'s line-oriented parser drops.
  #
  # THE MEMOS ARE AN ARGUMENT, and their absence is a FAILURE rather than an
  # empty column: "declared by no memo" must not be producible by a checkout that
  # has no memos in it. Default `$REPO_ROOT/docs/plans`; pass a sibling worktree's
  # when the harness and the memos are on different branches.
  python3 - "$REPO_ROOT/docs/plans" "${2:-$REPO_ROOT/docs/plans}" <<'INVENTORYPY'
import re, subprocess, sys
from pathlib import Path

HD, MD = Path(sys.argv[1]), Path(sys.argv[2])
DISPATCH = HD / "2026-07-citation-hygiene-A-rederive.sh"
PARTS = ["integrity", "common", "Ai", "Aii", "Aiii", "B"]
ORDER = ["umbrella", "A-i", "A-ii", "A-iii", "B", "C"]
# The umbrella ranks FIRST and is not a slice: it has LANDED, so a block it
# cites must exist from the first harness PR onward. Ranking it after A-i put
# `suites` in a column A-iii has no branch for while `:82` cites it today.
PART_SLICE = {"Ai": "A-i", "Aii": "A-ii", "Aiii": "A-iii", "B": "B"}
MEMOS = [("A-i", "Ai-spec-label-map"), ("A-ii", "Aii-gate-failure-semantics"),
         ("A-iii", "Aiii-suite-scheduler"), ("B", "B-detector-correctness"),
         ("C", "C-policy-retirement")]

src = "; ".join('. "$1/2026-07-citation-hygiene-A-rederive-%s.sh"' % p for p in PARTS)
r = subprocess.run(["bash", "--norc", "--noprofile", "-c", "set -e; %s; declare -f" % src,
                    "_", str(HD)], capture_output=True, text=True)
if r.returncode != 0:
    sys.stderr.write(r.stderr)
    raise SystemExit("!! sourcing the parts failed (rc=%d); an empty table is not a table."
                     % r.returncode)
bodies, cur = {}, None
for line in r.stdout.splitlines():
    m = re.match(r"^([A-Za-z_][A-Za-z0-9_]*) \(\) $", line)
    if m:
        cur = m.group(1); bodies[cur] = []
    elif cur:
        bodies[cur].append(line)
bodies = {k: "\n".join(v) for k, v in bodies.items()}
if len(bodies) < 2:
    raise SystemExit("!! `declare -f` yielded %d function(s); a table over nothing "
                     "reports no problem for a reason that is not 'there are none'."
                     % len(bodies))

dtext = DISPATCH.read_text(encoding="utf-8")
m = re.search(r"^all\(\) \{ set -- (.*?)\n\s*local failed", dtext, re.S | re.M)
if m is None:
    raise SystemExit("!! cannot read `all`'s roster from %s; the roster column would "
                     "then be empty for a reason that is not 'no block is on it'."
                     % DISPATCH.name)
roster = m.group(1).replace("\\\n", " ").split()
m2 = re.search(r"^(all\(\) \{.*?ALL BLOCKS EXITED 0[^\n]*)$", dtext, re.S | re.M)
bodies["all"] = m2.group(1) if m2 else " ".join(roster)
al = re.search(r'AUTHOR_LOCAL="([^"]+)"',
               (HD / "2026-07-citation-hygiene-A-rederive-common.sh").read_text()).group(1).split()

# The prose slice of a block runs from the END OF THE PREVIOUS BLOCK to its own
# last non-comment line, so the comment block that INTRODUCES a definition
# belongs to that definition rather than to the one above it -- `instruments`'
# rationale, the only site where the word "candidate" is written down, sits in
# such a header and a def-line-to-def-line slice loses it entirely.
DEF = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\(\) \{")
part, prose = {}, {}
for p in PARTS:
    lines = (HD / ("2026-07-citation-hygiene-A-rederive-%s.sh" % p)
             ).read_text(encoding="utf-8").splitlines()
    starts = [(i, DEF.match(l).group(1)) for i, l in enumerate(lines) if DEF.match(l)]
    prev = starts[0][0] if starts else 0
    for k, (i, n) in enumerate(starts):
        end = len(lines) if k + 1 == len(starts) else starts[k + 1][0]
        while end > i + 1 and (not lines[end - 1].strip()
                               or lines[end - 1].lstrip().startswith("#")):
            end -= 1
        part[n], prose[n], prev = p, lines[prev:end], end
part["all"], prose["all"] = "(disp)", bodies["all"].splitlines()
known = set(bodies)

# DECLARED BY. Heuristic over prose, so its rule is written down and what it
# drops is printed: a §15 code span naming TWO OR MORE known blocks is a
# declaration list; a span naming one is not, because §15 also spells single
# names in exclusion notices ("`lanes` and `staleclaims` are author-local") and
# in argument-bearing invocations (`readers SPEC_LABEL_REVERSE`).
declared, dropped = {}, []
for tag, fn in MEMOS:
    path = MD / ("2026-07-citation-hygiene-%s.md" % fn)
    if not path.is_file():
        raise SystemExit("!! %s is absent. 'Declared by no memo' would then be a fact "
                         "about this CHECKOUT, not about the memo -- pass the worktree "
                         "that holds them: `rederive inventory <dir>`." % path)
    sec = re.search(r"^## §15.*?(?=^## |\Z)", path.read_text(encoding="utf-8"), re.S | re.M)
    if sec is None:
        continue
    for run in re.findall(r"`([^`]+)`", sec.group(0).replace("\n", " ")):
        toks = [t for t in run.replace("*", "").split() if t in known]
        if len(toks) >= 2:
            for t in toks:
                declared.setdefault(t, set()).add(tag)
        elif len(toks) == 1:
            dropped.append((tag, run.strip()[:44], toks[0]))
    for x in re.findall(r"plus `([a-z_]+)`", sec.group(0)):
        if x in known:
            declared.setdefault(x, set()).add(tag)
utext = (MD / "2026-07-citation-hygiene-umbrella.md").read_text(encoding="utf-8")
for a, b in re.findall(r"rederive ([a-z_]+)|A-rederive\.sh ([a-z_]+)", utext):
    if (a or b) in known:
        declared.setdefault(a or b, set()).add("umbrella")


def at_command(n, body):
    """COMMAND POSITION, not "appears anywhere". `readers` prints the word
    "partition" in a diagnostic string, and `partition`'s own docstring ends a
    sentence with "all)"; bare occurrences made the first a caller of a Slice-B
    block and the second a caller of the dispatcher. A heredoc PAYLOAD is still
    shell-opaque, which is why the caller list is PRINTED in the `why` column
    rather than only consumed -- a residual false positive stays visible."""
    lead = r"(?m)(?:^\s*|[;&|(]\s*|\b(?:then|do|else|if|while|until)\s+)"
    return (re.findall(lead + re.escape(n) + r"(?![\w-])(?!\))", body)
            # `_measure [--nomatch <st>] <var> <cmd> [arg...]` takes a COMMAND as
            # its third word, so `_measure n_head _wtscan …` is a call site that
            # is not in command position. Missing it made `_wtscan` uncalled.
            + re.findall(lead + r"_measure(?:\s+--nomatch\s+\S+)?\s+\S+\s+"
                         + re.escape(n) + r"(?![\w-])", body))


CMP = re.compile(r"\[\[? [^]]*? (?:-eq|-ne|-gt|-lt|-ge|-le|=|!=) ")
CALL = {b: sorted(n for n in known if n != b and at_command(n, body))
        for b, body in bodies.items()}
rows = {}
for b in known:
    body = bodies[b]
    rows[b] = dict(part=part.get(b, "?"),
                   roster="yes" if b in roster else "author-local" if b in al else "no",
                   decl=",".join(sorted(declared.get(b, []))) or "-",
                   meas=len(at_command("_measure", body)),
                   # The needle is SPLIT so this block does not match itself: it
                   # prints the name of the signal it is looking for.
                   vrd="Y" if re.search("VERDICT" + r":\s*(GREEN|RED)", body) else "-",
                   cmp=len(CMP.findall(body)),
                   ln=len(prose.get(b, [])))

# SHIPS WITH -- four tiers, in order, each total and mechanical:
#   T0  defined in the DISPATCHER -> kernel. It is the invocation surface every
#       memo cites blocks through, it belongs to no slice, nothing calls it.
#   T1  a memo declares it -> the EARLIEST declarer in the forced order, the
#       umbrella first. A block must exist by the time its first citer lands,
#       and the order being forced is what makes that sound.
#   T2  else, defined in a slice part -> that slice.
#   T3  else (`-common.sh` / `-integrity.sh`, declared by nobody) -> the earliest
#       ship-with among its command-position CALLERS. No non-kernel caller means
#       nothing but the dispatcher needs it: kernel.
callers = {b: [c for c in known if b in CALL[c]] for b in known}
ship, why = {}, {}
for b in known:
    d = [s for s in ORDER if s in declared.get(b, ())]
    if rows[b]["part"] == "(disp)":
        ship[b], why[b] = "kernel", "T0 dispatcher"
    elif d:
        ship[b], why[b] = d[0], "T1 declared"
    elif rows[b]["part"] in PART_SLICE:
        ship[b], why[b] = PART_SLICE[rows[b]["part"]], "T2 part"
for _ in range(len(known)):
    for b in sorted(known):
        if b in ship or not all(c in ship for c in callers[b]):
            continue
        cs = [ship[c] for c in callers[b] if ship[c] in ORDER]
        ship[b] = min(cs, key=ORDER.index) if cs else "kernel"
        why[b] = "T3 " + (",".join(sorted(callers[b])) or "no caller")
for b in known:
    ship.setdefault(b, "kernel"); why.setdefault(b, "T3 caller cycle")

print("  %-13s%-10s%-13s%-22s%5s%4s%4s%5s  %-6s %s"
      % ("block", "part", "roster", "declared by", "meas", "vrd", "cmp",
         "ln", "ships", "why"))
for b in sorted(known, key=lambda x: (ORDER.index(ship[x]) if ship[x] in ORDER else -1, x)):
    q = rows[b]
    print("  %-13s%-10s%-13s%-22s%5d%4s%4d%5d  %-6s %s"
          % (b, q["part"], q["roster"], q["decl"], q["meas"], q["vrd"], q["cmp"],
             q["ln"], ship[b], why[b]))

tally = {}
for b in known:
    t = tally.setdefault(ship[b], [0, 0]); t[0] += 1; t[1] += rows[b]["ln"]
attributed = sum(rows[b]["ln"] for b in known)
files = sorted(HD.glob("2026-07-citation-hygiene-A-rederive*.sh"))
filelines = sum(len(f.read_text(encoding="utf-8").splitlines()) for f in files)
print("\n  defined=%d roster=%d declared=%d author-local=%d"
      % (len(known), len(roster), len(declared), len(al)))
print("  LINES: %d in %d files = %d attributed to a block + %d unattributed"
      % (filelines, len(files), attributed, filelines - attributed))
print("         (unattributed = each part's preamble + the dispatcher outside `all`.")
print("          A removal deletes FILES, so a share-of-the-harness figure is over %d.)"
      % filelines)
print("  ships-with (blocks, prose lines): "
      + "  ".join("%s=%d/%d" % (k, v[0], v[1]) for k, v in sorted(tally.items())))
print("  on the roster, declared by no memo: "
      + (" ".join(sorted(set(roster) - set(declared))) or "(none)"))
print("  declared but NOT on the roster    : "
      + (" ".join(sorted(set(declared) - set(roster))) or "(none)"))
print("  blocks printing a VERDICT         : "
      + (" ".join(sorted(b for b in known if rows[b]["vrd"] == "Y")) or "(none)"))

# ROUTING UNIT vs SHIPPING UNIT. Every column above routes a BLOCK; a PR adds and
# removes FILES. Where the two disagree, no file-granular action can carry out the
# routing -- `-Aiii.sh` holds a block that ships with the umbrella, and `-common.sh`
# and `-integrity.sh` hold blocks owned by a slice. This is the work list for
# reconciling them, and until it is empty "ships with X" is an assertion about a
# world in which the parts are cut differently than they are.
print("\n  -- ROUTING UNIT (block) vs SHIPPING UNIT (file): where they disagree --")
mis = [(b, rows[b]["part"], ship[b], rows[b]["ln"]) for b in sorted(known)
       if PART_SLICE.get(rows[b]["part"], "shared") != ship[b]
       and not (rows[b]["part"] not in PART_SLICE and ship[b] == "kernel")]
for b, pt, sh, ln in mis:
    home = PART_SLICE.get(pt, "no slice")
    print("   %-13s lives in %-9s (%-8s)  ships with %-8s  %4d lines" % (b, pt, home, sh, ln))
print("   %d of %d blocks / %d lines cannot be moved or removed at FILE granularity."
      % (len(mis), len(known), sum(x[3] for x in mis)))
print("\n  -- §15 code spans naming exactly ONE known block, NOT read as declarations --")
seen = set()
for tag, run, tok in dropped:
    if (tag, run) not in seen:
        seen.add((tag, run)); print("   %-6s %-12s in `%s`" % (tag, tok, run))
INVENTORYPY
  return $?
}

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

# The re-derivation harness's WHOLE-HARNESS AUDIT — three checks that range over
# the harness itself, sourced after `-integrity.sh` because each calls `_measure`
# or `$REPO_ROOT`. Not executable on its own.
#
# THE SEAM, and why it is here rather than in `-integrity.sh`: that file is the
# MEASUREMENT PRIMITIVE -- `$REPO_ROOT` and `_measure`/`_measured`, which every
# other part depends on at SOURCE time. These three are CONSUMERS of it, and each
# answers a different question about the harness as a whole:
#
#   homes      where is the block-set fact WRITTEN DOWN, and is each site guarded
#   inventory  WHOSE is each block, and does its declaration survive the tiers
#   selfcheck  does every block on `all`'s roster STATE its own exit status
#
# The split was taken when `homes` put `-integrity.sh` into the 700-800 authoring
# band (CLAUDE.md's touch-time rule: cut the seam while writing, in its own
# commit). It moves text verbatim and changes no behaviour -- verified per block,
# stdout+stderr and exit code, before and after.

homes() {  # WHERE THE BLOCK-SET FACT IS WRITTEN DOWN — every site, derived
  # WHY THIS BLOCK EXISTS. `inventory` answers "whose is each block". This one
  # answers the question five plan-review rounds kept re-opening: "where is that
  # fact written down, and did the plan name every site?" Each round, a hand-
  # written step list omitted a site, and each round the omission was found by a
  # reviewer rather than by the harness -- four to six homes at round 3, a
  # seventh at round 4, and at round 5 a parser reading the dispatcher's own text
  # whose failure moved the routing answer silently. A step list that has to be
  # complete, and is written by hand, is the same defect the memo tables had
  # before they became this harness's output. So it stops being written.
  #
  # THE DERIVATION, and its limit, stated rather than assumed:
  #   V   the vocabulary -- block names (bash parsing bash) and part stems (the
  #       same glob this census ranges over). Neither is a list kept here.
  #   R1  a CODE line naming the artifact (a harness or memo path/glob). You
  #       cannot read the harness or a memo without spelling it. Code only:
  #       a comment that merely names a file does not read it, and admitting
  #       prose here buried the real homes under every file's header.
  #   R2  a line naming TWO OR MORE members of V and not defining a block. One
  #       name is a mention; two is an enumeration of the set.
  #   R3  a literal ASSIGNMENT of list/dict/tuple/string shape. Shape, not
  #       content -- this is what catches a set written in a vocabulary the
  #       census does not share (group names, memo filenames).
  #   R4  an INDIRECT reader: a variable bound from an R1 line, and every later
  #       use of it. Without this the census sees the `read_text()` and not the
  #       three regexes over its result, which is exactly how round 5's parser
  #       hid -- it names nothing, it reads a variable.
  # ⚠ WHAT IT CANNOT SEE: a home reached through two levels of indirection, and
  # a home in a file this glob does not match. Both are reported as limits below
  # rather than left for a reviewer to discover as an absence.
  #
  # GUARDED is the column that matters. Every recurring instance has been an
  # UNGUARDED read: a hardcoded filename with no `if`, a regex with a silent
  # fallback. A home with no named failure cannot report that it stopped working.
  python3 - "$REPO_ROOT/docs/plans" <<'HOMESPY'
import re, subprocess, sys
from pathlib import Path

HD = Path(sys.argv[1])
FILES = sorted(HD.glob("2026-07-citation-hygiene-A-rederive*.sh"))
PARTFILES = [f for f in FILES if "A-rederive-" in f.name]
if len(PARTFILES) < 2:
    raise SystemExit("!! found %d harness part(s) under %s; a census over nothing "
                     "reports no homes for a reason that is not 'there are none'."
                     % (len(PARTFILES), HD))

PARTS = [f.name.split("A-rederive-")[1][:-3] for f in PARTFILES]
src = "; ".join('. "%s"' % f for f in PARTFILES)
r = subprocess.run(["bash", "--norc", "--noprofile", "-c", "set -e; %s; declare -F" % src],
                   capture_output=True, text=True)
if r.returncode != 0:
    sys.stderr.write(r.stderr)
    raise SystemExit("!! sourcing the parts failed (rc=%d); an empty vocabulary finds "
                     "no homes for the wrong reason." % r.returncode)
VOCAB = {l.split()[-1] for l in r.stdout.splitlines()} | set(PARTS)
if len(VOCAB) < 10:
    raise SystemExit("!! the vocabulary is %d token(s); too small to be the block set, "
                     "so 'no homes' would be a fact about this census." % len(VOCAB))

SELF = "2026-07-citation-hygiene"
DEF = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\(\) \{")
LIT = re.compile(r"""^\s*([A-Z][A-Z0-9_]*)\s*=\s*[\[{("']""")
BIND = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*.*?(?:read_text\(|\.glob\()")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
GUARD = re.compile(r"SystemExit|FATAL|raise |\|\|\s*(?:exit|\{)|exit [12]")

rows, bound = [], {}
for f in FILES:
    lines = f.read_text(encoding="utf-8").splitlines()
    for i, ln in enumerate(lines):
        code = not ln.lstrip().startswith("#")
        kinds = []
        if SELF in ln and code:
            kinds.append("reads the artifact")
        hits = {w for w in WORD.findall(ln) if w in VOCAB}
        d = DEF.match(ln)
        if d:
            # A definition line is not an enumeration OF ITS OWN NAME -- but
            # `all`'s definition line IS the roster, and excluding definitions
            # wholesale dropped the single most important home. Measured: the
            # first run of this census reported 83 homes and not that one.
            hits -= {d.group(1)}
        if len(hits) >= 2:
            kinds.append("enumerates %d of V" % len(hits))
        if LIT.match(ln):
            kinds.append("literal `%s`" % LIT.match(ln).group(1))
        b = BIND.match(ln)
        if b:
            bound.setdefault(f.name, {})[b.group(1)] = i + 1
        if kinds:
            guarded = any(GUARD.search(x) for x in lines[i:i + 7])
            rows.append((f.name, i + 1, "code" if code else "prose",
                         guarded, "; ".join(kinds), ln.strip()[:58]))

# R4 -- the indirect readers. A variable holding the artifact's TEXT is a home
# wherever it is used, not only where it was filled.
indirect = []
for f in FILES:
    lines = f.read_text(encoding="utf-8").splitlines()
    for name, at in sorted(bound.get(f.name, {}).items()):
        uses = [j + 1 for j, ln in enumerate(lines)
                if j + 1 > at and re.search(r"\b%s\b" % re.escape(name), ln)
                and not ln.lstrip().startswith("#")]
        if uses:
            guarded = [any(GUARD.search(x) for x in lines[u - 1:u + 6]) for u in uses]
            indirect.append((f.name, name, at, uses, guarded))

W = max(len(x[0]) for x in rows)
print("  -- HOMES OF THE BLOCK-SET FACT, derived (V = %d tokens over %d parts) --"
      % (len(VOCAB), len(PARTS)))
print("  %-*s %5s %5s %-6s %-24s %s" % (W, "file", "line", "kind", "guard", "why", "text"))
for fn, no, kind, g, why, txt in rows:
    print("  %-*s %5d %5s %-6s %-24s %s" % (W, fn, no, kind, "yes" if g else "NO", why, txt))

print("\n  -- INDIRECT (R4): a variable bound from the artifact's own text --")
for fn, name, at, uses, guarded in indirect:
    print("  %s:%d  `%s` read at %s%s" % (fn, at, name,
          ",".join(str(u) for u in uses),
          "   ⚠ %d unguarded use(s)" % guarded.count(False) if not all(guarded) else ""))

nprose = sum(1 for x in rows if x[2] == "prose")
nbad = sum(1 for x in rows if not x[3])
print("\n  HOMES: %d (%d code, %d prose) in %d files; %d with NO named failure."
      % (len(rows), len(rows) - nprose, nprose, len({x[0] for x in rows}), nbad))
print("  LIMITS (not findings -- what this census cannot see, so an absence here")
print("          is not evidence): a home reached through two levels of variable")
print("          indirection, and any home outside `%s*.sh`." % SELF)
if not rows:
    raise SystemExit("!! zero homes found. The block set is written down somewhere; "
                     "a census that found none measured nothing.")
HOMESPY
  return $?
}

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
PARTS = ["integrity", "audit", "common", "Ai", "Aii", "Aiii", "B"]
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
part, prose, defline, srcs = {}, {}, {}, {}
for p in PARTS:
    lines = (HD / ("2026-07-citation-hygiene-A-rederive-%s.sh" % p)
             ).read_text(encoding="utf-8").splitlines()
    srcs[p] = lines
    starts = [(i, DEF.match(l).group(1)) for i, l in enumerate(lines) if DEF.match(l)]
    prev = starts[0][0] if starts else 0
    for k, (i, n) in enumerate(starts):
        end = len(lines) if k + 1 == len(starts) else starts[k + 1][0]
        while end > i + 1 and (not lines[end - 1].strip()
                               or lines[end - 1].lstrip().startswith("#")):
            end -= 1
        part[n], prose[n], defline[n], prev = p, lines[prev:end], (p, i, end), end
part["all"], prose["all"] = "(disp)", bodies["all"].splitlines()
known = set(bodies)

# SHIP-WITH, DECLARED AT THE DEFINITION. A `#` comment naming the group, inside
# the block's own body (the needle is spelled once, at `_SW` below, and nowhere
# else in this block -- see (4) there). This is the authority; the four tiers
# below compute a SECOND answer and the two are cross-checked.
#
# WHY A DECLARATION AND NOT THE COMPUTATION. R4 measured two ways the computed
# answer cannot be an authority. (a) T1 routes by DECLARING MEMO, and the memos
# are prose on another branch: adding two block names to A-i's §15, with no `.sh`
# touched, moves two blocks; blanking the umbrella's three citations dissolves a
# whole group. (b) T2 is `ship = PART_SLICE[part]`, which is also the misroute
# predicate -- so a block moved into the wrong slice file is silently
# re-attributed and the check stays green.
#
# WHY THIS IS NOT THE SHAPE REJECTED FOR `kind`. The design note's §2 refuses a
# `# kind:` declaration because NO computed signal reproduces a kind assignment,
# so the declaration would be unfalsifiable -- a second thing to get wrong.
# Ship-with is the opposite case: T0-T3 compute a real answer, so a declaration
# is CHECKABLE and a disagreement is a finding rather than a preference.
#
# ⚠ THE ONE RULE THIS PATTERN PAIR EXISTS TO ENFORCE: a declaration that was
# WRITTEN and NOT READ must never come out as "undeclared". Four ways to break it
# have now been found BY PLANTING OR BY RUNNING, none by inspection:
#   (1) a `$` anchor dropped a declaration carrying a trailing comment (90e1429b);
#   (2) a ONE-LINER definition has no line whose first character is `#`, so an
#       anchored form cannot see a declaration written after its closing brace.
#       There are exactly two such definitions (`_measured`, `say`), so this is
#       not hypothetical; the alternation below admits the post-brace form;
#   (3) `\s*` after the colon CROSSES A NEWLINE, so a valueless declaration
#       captured the NEXT line's first token and reported `declares local`.
#       `[ \t]` is used everywhere a same-line gap is meant;
#   (4) THE NEEDLE MATCHED THIS BLOCK'S OWN PROSE. Admitting the post-brace form
#       made the examples in this very comment parse as `inventory`'s
#       declaration -- the `cand` column's self-ranking defect, one level down,
#       and caught by RUNNING the fix rather than reading it. So the needle is
#       BUILT FROM PIECES (as `vrd`'s is) and this comment does not spell it.
# And `claimed` counts what DECL ATTRIBUTED, not what LOOSE saw: the old form
# credited an unparseable declaration to the block, defeating the stray guard at
# exactly the sites that needed it.
_SW = "ships" "-with" ":"
DECL = re.compile(r"(?:^|\})[ \t]*#[ \t]*" + _SW + r"[ \t]*([^\s#]+)", re.M)
LOOSE = re.compile(r"#[ \t]*" + _SW, re.M)
# ONE VOCABULARY. A declared group that is not a group is a typo, and without
# this it surfaces as `declares X but the tiers compute Y` -- indistinguishable
# from a real misroute. `ORDER` is the slice sequence; kernel is not a slice.
GROUPS = set(ORDER) | {"kernel"}
decl, claimed, badname, dupe = {}, {}, [], []
for b_ in known:
    # SEARCH REGION = the block itself, definition line to close. NOT the prose
    # slice: for the FIRST definition in a part that slice excludes the file
    # preamble (measured -- a declaration above `_measure()` was invisible), and
    # for every other block it INCLUDES the preceding comment block, so a
    # declaration written above a definition would be read as its neighbour's.
    if b_ not in defline:
        continue
    pt_, i_, end_ = defline[b_]
    region = "\n".join(srcs[pt_][i_:end_])
    hits = DECL.findall(region)
    claimed[b_] = len(hits)
    if len(hits) > 1:
        dupe.append((b_, hits))
    if hits:
        decl[b_] = hits[0]
        if hits[0] not in GROUPS:
            badname.append((b_, hits[0]))

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
comp, why = {}, {}
for b in known:
    d = [s for s in ORDER if s in declared.get(b, ())]
    if rows[b]["part"] == "(disp)":
        comp[b], why[b] = "kernel", "T0 dispatcher"
    elif d:
        comp[b], why[b] = d[0], "T1 declared"
    elif rows[b]["part"] in PART_SLICE:
        comp[b], why[b] = PART_SLICE[rows[b]["part"]], "T2 part"
# WHAT T3 READS is `route`: the DECLARATION where a caller has one, the tier
# answer otherwise. Reading the tier answer unconditionally made T3 a relay for
# T1, so PROSE moved code-signal routing: measured, splitting one A-i §15 code
# span into six -- typographically null, no `.sh` touched -- turned four correct
# declarations into four DISAGREEs, and TWO of the four came through T3.
route = dict(decl)
for b, v in comp.items():
    route.setdefault(b, v)
for _ in range(len(known)):
    for b in sorted(known):
        if b in comp or not all(c in route for c in callers[b]):
            continue
        cs = [route[c] for c in callers[b] if route[c] in ORDER]
        comp[b] = min(cs, key=ORDER.index) if cs else "kernel"
        why[b] = "T3 " + (",".join(sorted(callers[b])) or "no caller")
        route.setdefault(b, comp[b])
for b in known:
    comp.setdefault(b, "kernel"); why.setdefault(b, "T3 caller cycle")

# THE DECLARATION IS THE AUTHORITY -- everywhere, not only on the disagreement
# line. `ship` is what the column, the tally and the routing-vs-shipping move
# list all use, so that list prints `DECLARED != file`, which is what a PR can
# act on. Before this, every one of those printed the COMPUTED value while the
# declaration appeared nowhere but the mismatch report: the declaration was the
# authority in the prose and the tiers were still the authority in the output.
ship = {b: decl.get(b, comp[b]) for b in known}

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

# DECLARED vs COMPUTED. The declaration is the authority and the computation is
# the check on it; neither alone is trusted. An UNDECLARED block is reported, not
# silently defaulted to its computed answer -- "nobody said" and "the tiers say"
# are different states, and collapsing them is this harness's charter inverted.
print("\n  -- SHIP-WITH: declared (`# %s <group>`) vs computed (T0-T3) --" % _SW)
# A DISAGREEMENT IS ONLY AS GOOD AS THE TIER THAT PRODUCED IT, so the two are
# separated rather than pooled:
#   T0 / T3 are CODE signals (the dispatcher; the call graph) -> BINDING.
#   T1 is a heuristic parse of PROSE on a branch under active revision. By
#      `_measure`'s own rule that is a failed measurement, not a verdict, so it
#      REPORTS. Measured: a typographically null §15 edit made it fire falsely.
#   T2 is `ship = PART_SLICE[part]`, i.e. the FILENAME -- which is exactly what
#      the routing-vs-shipping list below already checks. Binding on it would be
#      the same check twice, and it is the tautology R4 named.
bad = [(b_, decl[b_], comp[b_], why[b_]) for b_ in sorted(decl) if decl[b_] != comp[b_]]
binding = [x for x in bad if not x[3].startswith(("T1", "T2"))]
for b_, d_, c_, w_ in bad:
    print("   %s %-13s declares %-9s but the tiers compute %-9s (%s)"
          % ("!!" if (b_, d_, c_, w_) in binding else "..", b_, d_, c_, w_))
if bad and len(binding) != len(bad):
    print("   (`..` = reported, not binding: the tier that disagrees is prose (T1)")
    print("    or the filename (T2). `!!` = a code signal disagrees.)")
for b_, hits in dupe:
    print("   !! %-13s carries %d `%s` declarations; only the first is read."
          % (b_, len(hits), _SW))
for b_, g_ in badname:
    print("   !! %-13s declares `%s`, which is not a group (%s)."
          % (b_, g_, " ".join(sorted(GROUPS))))
# Every declaration string in the parts must be attributed to exactly one
# block, BY THE PARSER THAT READS IT -- `claimed` counts DECL's successes, so a
# declaration LOOSE can see and DECL cannot parse is reported here rather than
# credited to the block and lost. That is the charter: written-and-unread must
# never come out as "undeclared".
stray = sum(len(LOOSE.findall("\n".join(v))) for v in srcs.values()) - sum(claimed.values())
if stray:
    print("   !! %d `%s` string(s) no block's row carries -- written, unread."
          % (stray, _SW))
und = sorted(known - set(decl))
print("   declared=%d  agree=%d  DISAGREE=%d (binding %d)  undeclared=%d"
      % (len(decl), len(decl) - len(bad), len(bad), len(binding), len(und)))
if und:
    print("   undeclared: " + " ".join(und))
    print("   (a block with no `%s` has no owner anyone wrote down; the" % _SW)
    print("    tiers' answer for it is a guess this table does not launder.)")

# ROUTING UNIT vs SHIPPING UNIT. Every column above routes a BLOCK; a PR adds and
# removes FILES. Where the two disagree, no file-granular action can carry out the
# routing -- `-Aiii.sh` holds a block that ships with the umbrella, and `-common.sh`
# and `-integrity.sh` hold blocks owned by a slice. This is the work list for
# reconciling them, and until it is empty "ships with X" is an assertion about a
# world in which the parts are cut differently than they are.
print("\n  -- ROUTING UNIT (block) vs SHIPPING UNIT (file): where they disagree --")


def _misplaced(b):
    return (PART_SLICE.get(rows[b]["part"], "shared") != ship[b]
            and not (rows[b]["part"] not in PART_SLICE and ship[b] == "kernel"))


# TWO LISTS, because they authorise different things. A DECLARED block whose file
# disagrees is a MOVE a PR can carry out. An UNDECLARED one is the tiers guessing,
# and `declared != file` is not what is being printed for it -- merging the two
# was how a move list of tier guesses read as a work list.
mis = [(b, rows[b]["part"], ship[b], rows[b]["ln"])
       for b in sorted(decl) if _misplaced(b)]
guess = [b for b in sorted(known) if b not in decl and _misplaced(b)]
for b, pt, sh, ln in mis:
    home = PART_SLICE.get(pt, "no slice")
    print("   %-13s lives in %-9s (%-8s)  DECLARES %-8s  %4d lines" % (b, pt, home, sh, ln))
print("   MOVE LIST: %d of %d declared block(s) / %d lines -- `declared != file`."
      % (len(mis), len(decl), sum(x[3] for x in mis)))
if guess:
    print("   NO VERDICT: %d undeclared block(s) whose file differs from the TIERS' guess"
          % len(guess))
    print("               (%s)" % " ".join(guess))
    print("               -- a guess is not a move list; declare them first.")
print("\n  -- §15 code spans naming exactly ONE known block, NOT read as declarations --")
seen = set()
for tag, run, tok in dropped:
    if (tag, run) not in seen:
        seen.add((tag, run)); print("   %-6s %-12s in `%s`" % (tag, tok, run))

# THE VERDICT IS A RETURN STATUS. A disagreement that only prints is the defect
# `citations` is being fixed for, one level up. What binds: a CODE signal
# contradicting a declaration, a declaration this parser could not read, a second
# declaration in one block, and a group that is not a group. What does not: a
# prose (T1) or filename (T2) disagreement -- reported above, and deliberately
# not a gate, because a memo edit on another branch must not turn this red.
_fatal = len(binding) + bool(stray) + len(dupe) + len(badname)
if _fatal:
    raise SystemExit("!! %d binding ship-with problem(s): %d code-signal contradiction(s), "
                     "%d unread declaration(s), %d duplicate(s), %d bad group name(s)."
                     % (_fatal, len(binding), stray, len(dupe), len(badname)))
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

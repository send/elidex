# The re-derivation harness's BLOCK TABLE — `inventory`, which answers WHOSE each
# block is. Sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry
# point. Not executable on its own: it defines no dispatch and sets no shell
# options, and it reads `$REPO_ROOT` from `-integrity.sh`, which the dispatcher
# sources first.
#
# THE SEAM, and why this is not in `-audit.sh`: that file's two checks read the
# parts AS TEXT, line by line, and locate what they find at `file:line`, with
# every input in this checkout. This one SOURCES them -- `declare -f`, so every
# signal below is a property of the CODE and not of the prose beside it -- reasons
# over the call graph it gets back, and takes its authority from six memos that
# live on ANOTHER BRANCH. That is why it is the one whole-harness check that takes
# an argument (`rederive inventory <memo-dir>`), and why the tier fed by that
# prose reports rather than gates. Ownership is no single site's property.
#
# ⚠ THIS FILE IS A PART, and TWO hardcoded lists have to say so: `PARTS` below and
# the dispatcher's source loop. Missing from `PARTS`, a block leaves the audited
# set silently -- measured at this cut: `defined=` drops 35 -> 34 while `inventory`
# and `selfcheck` still exit 0 and the census reports no new finding.

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
PARTS = ["integrity", "audit", "inventory", "common", "Ai", "Aii", "Aiii", "B"]
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
def _noheredoc(lines):
    """Drop `<<'TAG'` … TAG bodies. A declaration inside one is not shell."""
    out, term = [], None
    for l in lines:
        if term is not None:
            if l.strip() == term:
                term = None
            continue
        out.append(l)
        m_ = re.search(r"<<-?'([A-Za-z_][A-Za-z0-9_]*)'", l)
        if m_:
            term = m_.group(1)
    return out


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
    # ⚠ HEREDOC PAYLOADS ARE NOT SHELL. A declaration line inside a python
    # payload is a PYTHON comment, and reading it made the payload the block's
    # authority: measured, a planted one entered the MOVE LIST as an actionable
    # move and, disagreeing only via T2, left rc=0. `selfcheck`'s parser already
    # drops payload bodies for exactly this reason; this one did not.
    # (The needle is not spelled here -- see `_SW`. Spelling it in this block's
    # own prose is how the clean tree went red twice already.)
    region = "\n".join(_noheredoc(srcs[pt_][i_:end_]))
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
        # ⚠ A BLOCK CALLED ACROSS GROUP BOUNDARIES BELONGS TO NONE OF THEM.
        # "Earliest caller wins" made the measurement primitive unrepresentable:
        # `_measure` is called from every group, so declaring it `kernel` -- the
        # layering-correct answer, and the whole reason `-integrity.sh` exists --
        # went binding-RED against T3. The umbrella's own `:89` says a slice may
        # not carry another slice's concern; a block two slices call is shared
        # infrastructure, which is what `kernel` names.
        comp[b] = ("kernel" if len(set(cs)) > 1
                   else min(cs, key=ORDER.index) if cs else "kernel")
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

# THE MEMOS' PROVENANCE MARKS, AUDITED. The 2026-08 memos mark a re-measured claim
# with `⊕`, and their own convention says every marked item carries the command in
# ITS OWN BLOCK. THREE PLAN-REVIEW ROUNDS REPORTED THE SAME CLASS -- marks standing
# alone -- and each remedy was a rewrite of the marks, which is the shape
# `memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md` names: the author
# believes the convention was followed, so only a check disagrees. Its yield when
# first run was eleven, two of them written twenty minutes earlier in the session
# that wrote the check.
#
# WHERE THIS BLOCK LIVES IS A DISPOSITION QUESTION AND THE MEMO DECIDES IT -- see
# D18. Repeating the argument here would give a placement rule a second home in the
# one file whose job is to find those, and the census says so: an earlier draft of
# this comment stated the rule inline and the census correctly filed the line as a
# `prose` placement home, moving the work list by one.
#
# THE UNIT IS THE BLOCK, NOT THE LINE, because the convention binds the ITEM: a
# bullet may attest on one line and run its command three lines down, and a fence is
# separated from the sentence introducing it by a blank line. WHAT COUNTS AS A
# COMMAND IS KEYED TO BEHAVIOUR -- the first token resolves on `PATH` -- rather than
# to a list of verb names, which would leave the next tool authoritative by default
# (`memory/feedback_enumerated-exemptions-leave-the-next-class-authoritative.md`);
# a one-token span is a NAME, since `test`, `time` and `env` all resolve, so two
# tokens are required.
#
# ⚠ FOUR BLIND SPOTS, STATED BECAUSE AN ABSENCE HERE IS NOT EVIDENCE. Two were
# declared when this landed; two more were measured by the review that read it, and
# the second of those is the load-bearing one:
#   (1) a shell BUILTIN as first token (`local n; _measure …`) does not resolve on
#       `PATH` and reads as no command.
#   (2) a command that RUNS but measures a different claim than the sentence above
#       it passes. This is the larger half of what the reviews were reporting.
#   (3) THE SAME FALSE MEASURED CLAIM WRITTEN WITHOUT THE MARK IS INVISIBLE. The
#       predicate is "does this line carry `⊕`", i.e. the population is the AUTHOR'S
#       OWN VOCABULARY rather than the property being checked
#       (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`).
#       A memo that uses a different convention contributes zero and says so below.
#   (4) a mark moved INSIDE a fence leaves the population, because fenced lines are
#       skipped so that a legend defining the mark is not read as using it -- and,
#       the other way round, a mark inside an INLINE code span stays in it, so a
#       memo quoting this block's own output back has written an attestation. That
#       is not fixable by a needle: the two are the same characters in the same
#       position, and only the sentence around them differs. Measured while writing
#       the entry that records this block.
# The population is printed for the same reason the quantity gate prints its own: a
# needle matching nothing reports clean for the wrong reason.
attest() {  # THE MEMOS' PROVENANCE MARKS — every `⊕` item carries a command in its own block
  python3 - "$REPO_ROOT/docs/plans" <<'ATTESTPY'
import re, shutil, sys
from pathlib import Path

HD = Path(sys.argv[1])
M8 = sorted(HD.glob("2026-08-citation-hygiene-harness-*.md"))
if not M8:
    raise SystemExit("!! no `2026-08-citation-hygiene-harness-*.md` under %s; a population of zero "
                     "would report clean for the reason a needle matching nothing does." % HD)

def scan(md):
    """(lineno, text, fenced) -- a ``` fence line counts as fenced on BOTH sides, so a
    legend DEFINING the mark inside a fence is not read as an item USING it."""
    fen = False
    for i, s in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
        f0 = s.lstrip().startswith("```")
        fen = fen ^ f0
        yield i, s, fen or f0

def blocks(md):
    """Blank-line-delimited blocks, with a following all-fenced block merged into the
    one above it: a fence is nobody's item on its own, and the sentence that
    introduces it is separated from it by a blank line."""
    out, cur = [], []
    for i, s, fen in scan(md):
        if not s.strip() and not fen:
            if cur:
                out.append(cur); cur = []
        else:
            cur.append((i, s, fen))
    if cur:
        out.append(cur)
    merged = []
    for b in out:
        if merged and all(f for _, _, f in b):
            merged[-1] = merged[-1] + b
        else:
            merged.append(b)
    return merged

def runnable(blk):
    for _, s, fen in blk:
        if fen and s.strip() and not s.lstrip().startswith("```"):
            return True
        for span in re.findall(r"`([^`\n]+)`", s):
            tok = span.split()
            if len(tok) > 1 and shutil.which(tok[0]):
                return True
    return False

pop, bad, per = 0, [], {}
for md in M8:
    n = 0
    for blk in blocks(md):
        att = [(i, s) for i, s, fen in blk if not fen and "⊕" in s]
        if not att:
            continue
        ok = runnable(blk)
        for i, s in att:
            for _ in range(s.count("⊕")):
                pop += 1; n += 1
                if not ok:
                    bad.append((md.name, i))
                    print("   !! %s:%d  ⊕ attests a measurement and its item carries no command: %s"
                          % (md.name, i, s.split("⊕", 1)[1].strip().replace("**", "")[:58]))
    per[md.name] = n
print("   PER MEMO: %s" % "  ".join("%s=%d" % (k.replace("2026-08-citation-hygiene-harness-", ""), v)
                                    for k, v in sorted(per.items())))
print("   POPULATION: ⊕ attestation=%d   findings=%d" % (pop, len(bad)))
if pop == 0:
    raise SystemExit("!! %d memo(s) read and not one carries the mark; the convention would then be "
                     "checked by a needle that matches nothing." % len(M8))
if bad:
    raise SystemExit("!! %d attestation(s) with no command in their own block. Do not remove the mark "
                     "to silence this -- the convention is that the command travels with the claim."
                     % len(bad))
ATTESTPY
  return $?
}

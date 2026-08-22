# The re-derivation harness's AUDIT OF ITS OWN TEXT — two checks that range over
# the part files AS TEXT and report what they find AT A SITE. Sourced after
# `-integrity.sh` because each calls `_measure` or `$REPO_ROOT`; not executable
# on its own.
#
#   homes      where is the block-set fact WRITTEN DOWN, and is each site guarded
#   selfcheck  does every block on `all`'s roster STATE its own exit status
#
# THE SEAM, and why `inventory` is no longer here: these two read the parts line
# by line and locate what they find at `file:line`, and every input they need is
# in this checkout. `inventory` SOURCES the parts (`declare -f`, which STRIPS the
# prose these two range over), reasons over the call graph, and takes its
# authority from memos on ANOTHER BRANCH -- which is why it alone takes an
# argument. `-inventory.sh` holds it and states that seam from its side.
#
# ⚠ `homes` DOES NOT MOVE: this preamble is exactly as long as the one it
# replaces, so the `-audit.sh:N` lines the 2026-08 memos cite still resolve.

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
# ⚠ R4 NEEDS A SUBJECT TEST TOO, for the same reason R3 did: bound to shape
# alone it reported `_proto`'s CALLER-SUPPLIED plan file as "the artifact's own
# text". The line must also name the artifact, which is what makes the variable
# a reader OF THIS HARNESS rather than of whatever the caller passed in.
BIND = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*.*?(?:read_text\(|\.glob\()")
HANDLE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*[^=]")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
# ⚠ THE GUARD NEEDLE MUST KNOW THIS HARNESS'S OWN IDIOM. It did not: `_measure
# <var> <cmd> || failed=1` is THE validity primitive -- the analysis note's §2
# says every shape here is a spelling of it -- and reporting those call sites as
# "no named failure" overstated the work list at exactly the idiom the program
# exists for. `budget`'s four reads all route through `_measure` and all read NO.
GUARD = re.compile(r"SystemExit|FATAL|raise |\|\|\s*(?:exit|\{|failed=|continue|rc=)"
                   r"|exit [12]|_measure\b")

# --- THE CLASS OF EACH HOME -------------------------------------------------
# WHY THE CENSUS ASSIGNS THE CLASS AND NOT THE MEMO. The disposition memo's §3
# is a rule PER CLASS, and it claimed to be "complete by construction: every row
# the census prints falls into one of these". Round 6 measured that claim false
# on 31 of 70 rows -- FIVE reviewers, independently, exactly as rounds 3-5 had
# measured the hand-written list of SITES it replaced. An enumeration written by
# hand is incomplete at whatever altitude it is written; the fix that has worked
# three times in this program is to derive it and let the machine say so.
#
# So the mapping lives here, it is TOTAL, and an unclassified row is RED. That
# does not make the mapping right -- it makes its INCOMPLETENESS a red build
# instead of a claim a reviewer has to falsify by hand.
CLASSES = {"PARTS": "partset", "ORDER": "groupvocab", "PART_SLICE": "groupvocab",
           "MEMOS": "memoset", "AUTHOR_LOCAL": "authorlocal"}


# A CALL SITE IS A VOCABULARY TOKEN IN COMMAND POSITION. The predicate this
# replaced tested only the LINE'S FIRST WORD plus a `_measure` special case, so
# the outer regex's `then `/`do `/`else ` alternatives were dead and a derivation
# called anywhere but first went unseen -- measured, the quoted crossing
# `python3 - "$(_partset)" "$(_roster)"` reached `mention` and filed a genuine
# home under the class whose rule is *nothing to do*, at rc=0.
#
# A command begins at LINE START, after `;` `&&` `||` `|`, after `(` -- which
# covers `$(` and `<(` -- after `{`, and after then/do/else/elif/if/while/until.
# Three exclusions, each one a decision measurement forced rather than a
# character copied from the old regex:
#   `${` does not open one (parameter expansion is not a command);
#   `$((` does not either (arithmetic), which is why `(` is excluded when it sits
#     beside another `(`;
#   a BACKTICK is not a command position -- a regex cannot tell a shell
#     substitution from the markdown backticks this harness writes block names
#     in, and the harness contains no legacy backtick substitution to buy.
# A bare `&` is knowingly NOT included: the rule spells `&&`, its population here
# is empty, and admitting it is the reconciliation's call, not this scan's.
#
# SINGLE-quoted spans are removed first and DOUBLE-quoted spans are not. Of the
# three readings available exactly one satisfies every observable: stripping
# double quotes too would kill the quoted crossing above, and stripping neither
# would reclassify `echo '$(_partset)'`, which is correct as `mention` today.
#
# `_measure`'s special case RETIRES into this: `_measure` is a vocabulary token
# (`VOCAB` comes from `declare -F`), so `local n; _measure a b` reaches command
# position after the `;` with no case of its own.
SQSPAN = re.compile(r"'[^']*'")
TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
KWEND = re.compile(r"\b(?:then|do|else|elif|if|while|until)$")

def _at_command(s, i):
    """Does the token starting at `s[i]` stand in command position?"""
    pre = s[:i].rstrip()
    if not pre:
        return True
    if pre[-1] in ";|" or pre.endswith("&&"):
        return True
    if pre.endswith("(") and not pre.endswith("(("):
        return True
    if pre.endswith("{") and not pre.endswith("${"):
        return True
    return bool(KWEND.search(pre)) and s[i - 1].isspace()

def hits_outside_quotes(ln):
    """Vocabulary tokens that are NOT inside a quoted span."""
    bare = re.sub(r"'[^']*'|\"[^\"]*\"", " ", ln)
    return len([w for w in WORD.findall(bare) if w in VOCAB])


def classify(fname, ln, code, kinds, span):
    if not code:
        return "prose"
    for k in kinds:
        if k.startswith("literal `"):
            return CLASSES.get(k.split("`")[1], "?")
    if span and span[0] <= LINENO[0] <= span[1]:
        return "roster"
    if re.search(r"\bfor\s+_part\s+in\b", ln):
        return "partset"
    if re.search(r"\bfor\s+m\s+in\b", ln) or "citation-hygiene-%s" in ln:
        return "memoset"
    if "reads the artifact" in " ".join(kinds):
        return "reads"
    if LITSPAN[0]:
        return LITSPAN[0]
    # ⚠ `callsite` NEEDS A SUBJECT TEST, like every other rule here. As a
    # FALLTHROUGH it swallowed a genuine home: `blocks="citations budget lanes"`
    # is a hand-written enumeration of the block set, missed by R3 (lowercase),
    # and filed under the one class whose rule is that there is nothing to do --
    # the defect this census exists to prevent, reproduced inside it. A line is a
    # call site because a vocabulary token stands in COMMAND POSITION, and
    # anything else is unclassified, which is RED.
    scan = SQSPAN.sub("''", ln)
    if any(m.group(0) in VOCAB and _at_command(scan, m.start()) for m in TOKEN.finditer(scan)):
        return "callsite"
    # A MENTION: every vocabulary hit sits inside a quoted string, so the line
    # talks ABOUT a block rather than enumerating the set. Also nothing to do --
    # and also a subject test, not a fallthrough: strip the quoted spans and if
    # any hit survives, the line is unclassified and RED.
    if hits_outside_quotes(ln) == 0:
        return "mention"
    return "?"


LINENO = [0]
LITSPAN = [None, 0]
rows, bound, handles = [], {}, {}
# `all`'s roster spans several CONTINUATION lines, and they are the same home.
rosterspan = {}
for f in FILES:
    t = f.read_text(encoding="utf-8").splitlines()
    for j, l in enumerate(t):
        if l.startswith("all() { set --"):
            e = next((k for k in range(j, len(t)) if "local failed" in t[k]), j)
            rosterspan[f.name] = (j + 1, e)
            break
for f in FILES:
    lines = f.read_text(encoding="utf-8").splitlines()
    for i, ln in enumerate(lines):
        code = not ln.lstrip().startswith("#")
        LINENO[0] = i + 1
        kinds = []
        if LITSPAN[0] and LITSPAN[1] <= 0:
            LITSPAN[0] = None
        elif LITSPAN[0]:
            LITSPAN[1] += ln.count("[") + ln.count("(") - ln.count("]") - ln.count(")")
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
        # R3 is a SHAPE rule, so it needs a subject test or it reports every
        # ALL-CAPS constant in the harness (`W`, `KEEP`, `HDR` -- literals about
        # webref paths and table headers, not about the block set). Requiring one
        # vocabulary token keeps `ORDER` (`B` is a stem), `MEMOS` (`Ai`), `PARTS`,
        # `PART_SLICE` and `AUTHOR_LOCAL`, which is the set R3 exists for.
        if LIT.match(ln) and hits:
            nm = LIT.match(ln).group(1)
            kinds.append("literal `%s`" % nm)
            # A literal may span lines; its CONTINUATIONS are the same home.
            depth = ln.count("[") + ln.count("(") - ln.count("]") - ln.count(")")
            LITSPAN[0], LITSPAN[1] = (CLASSES.get(nm, "?"), depth) if depth > 0 else (None, 0)
        # An ARTIFACT HANDLE is a variable assigned on a line that names the
        # artifact; a read THROUGH one is still a read of the artifact. ⚠ The
        # first subject test required the artifact's name on the READ line and
        # dropped `dtext = DISPATCH.read_text(…)` -- the indirect reader R4 was
        # built for. Caught by the acceptance list, not by inspection.
        h = HANDLE.match(ln)
        if h and SELF in ln:
            handles.setdefault(f.name, set()).add(h.group(1))
        b = BIND.match(ln)
        if b and (SELF in ln or any(re.search(r"\b%s\b" % re.escape(v), ln)
                                    for v in handles.get(f.name, ()))):
            bound.setdefault(f.name, {})[b.group(1)] = i + 1
        if kinds:
            # ⚠ THE WINDOW MUST BE CODE. Unfiltered, 16 of 34 "guarded" verdicts came
            # from a token inside a COMMENT -- including the comment written to
            # explain this very needle. A prose mention of `_measure` is not a guard.
            guarded = any(GUARD.search(x) for x in lines[i:i + 7]
                          if not x.lstrip().startswith("#"))
            rows.append((f.name, i + 1, "code" if code else "prose",
                         guarded, "; ".join(kinds), ln.strip()[:58],
                         classify(f.name, ln, code, kinds, rosterspan.get(f.name))))

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
print("  %-*s %5s %5s %-6s %-11s %-24s %s"
      % (W, "file", "line", "kind", "guard", "class", "why", "text"))
for fn, no, kind, g, why, txt, cl in rows:
    print("  %-*s %5d %5s %-6s %-11s %-24s %s"
          % (W, fn, no, kind, "yes" if g else "NO", cl, why, txt))

print("\n  -- INDIRECT (R4): a variable bound from the artifact's own text --")
for fn, name, at, uses, guarded in indirect:
    print("  %s:%d  `%s` read at %s%s" % (fn, at, name,
          ",".join(str(u) for u in uses),
          "   ⚠ %d unguarded use(s)" % guarded.count(False) if not all(guarded) else ""))

nprose = sum(1 for x in rows if x[2] == "prose")
nbad = sum(1 for x in rows if not x[3])
byclass = {}
for x in rows:
    byclass[x[6]] = byclass.get(x[6], 0) + 1
print("\n  BY CLASS: " + "  ".join("%s=%d" % kv for kv in sorted(byclass.items())))

# ⚠ A CLASS WITH NO RULE IS THE SAME DEFECT AS A HOME WITH NO CLASS, and the gate
# below could not see it: it reds an UNCLASSIFIABLE row, not a class the plan
# forgot to rule. Measured -- two classes reached a plan-review round with no
# rule while this block exited 0. The plan is prose, so the only thing that has
# ever held is a CHECKER OVER THE PROSE
# (`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md`): the memo's §3
# table keys ARE the census's class names, so the two sets can be compared.
PLAN = HD / "2026-08-citation-hygiene-harness-disposition.md"
unruled = []
if PLAN.is_file():
    # ⚠ SCOPED TO §3. Unscoped this was "every class has a row SOMEWHERE IN THIS
    # FILE": a rule row moved into any other table -- the fix table, the register
    # table -- still satisfied the gate, so it did not hold what it was written
    # for. `^## §3 ` with the trailing space excludes `## §3b` (the moves) and
    # `## §0.5 / §3.` (the spec map); both carry tables, neither carries a rule.
    _s3 = re.search(r"^## §3 .*?(?=^## §|\Z)", PLAN.read_text(encoding="utf-8"), re.S | re.M)
    if _s3 is None:
        raise SystemExit("!! %s has no `## §3 ` section this parser can find; 'every class "
                         "is ruled' would then be a fact about the parser." % PLAN.name)
    ruled = set(re.findall(r"^\| \*\*([a-z]+)\*\* \|", _s3.group(0), re.M))
    if not ruled:
        raise SystemExit("!! %s has no §3 rule rows this parser can read; 'every class is "
                         "ruled' would then be a fact about the parser." % PLAN.name)
    unruled = sorted(set(byclass) - ruled)
    print("  RULED BY THE PLAN: %d of %d class(es)%s"
          % (len(set(byclass)) - len(unruled), len(byclass),
             "" if not unruled else "  -- MISSING: " + " ".join(unruled)))
# ⚠ THE MEMO'S QUANTITIES, NOT ONLY ITS RULE-ROW KEYS. The gate above proves every
# class has a rule row and nothing about the DIGITS, PATH REFERENCES and REVISIONS
# the two 2026-08 memos assert. Eleven plan-review rounds reported one class over
# and over -- a stated file length, a band membership, a revision not on this
# branch, a `path:N` that no longer resolves, a `D<N>` with no target -- and TWO of
# round 11's sat in prose unchanged for up to nine drafts that ten earlier rounds
# never reported, because a reviewer reads the DIFF. Only a check over the WHOLE
# FILE reaches text nobody touched; a rule written in prose reaches none of it
# (`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md`). Same lever as
# the rule-row gate, one altitude down, at the site that already opens the memo.
# NOTHING HERE EDITS A MEMO: each check prints the file, the line, the claim and
# the MEASURED value, then appends to `_CBAD`, whose exit is taken at the very end
# beside the census's own so neither report can hide the other. The PARSER guards
# DO raise, on the rule the gate above states -- a check that could not read its
# subject must not report "no problem" -- and each prints its POPULATION, because
# a needle matching nothing reports clean for the wrong reason.
_CBAD, _CLIM, _rv = [], [], {}; _POP = dict.fromkeys(("revision", "path ref", "stated length", "band claim", "D<N> citation"), 0)
_M8 = sorted(HD.glob("2026-08-citation-hygiene-harness-*.md"))
ROOT = HD.parent.parent
# The harness's own file NAMES, from the set derived above rather than a second
# glob -- which would spell the artifact again and become a home of the fact this
# census counts.
HN = {f.name for f in PARTFILES} | {PARTFILES[0].name.split("A-rederive-")[0] + "A-rederive.sh"}
# (1) A REVISION is a backtick span whose ENTIRE content is 8 hex digits. Hex that
# is not a claim about this repository must not be reported, and the live case is
# `da39a3ee…` -- SHA-1 of the empty string, quoted inside a `shasum` EXAMPLE. The
# whole-span rule rejects it (its ellipsis is inside the backticks) without a
# needle that has to know what a transcript looks like. BOTH "no such object" and
# "resolves on some other ref" FAIL; the second is discriminated in the diagnostic
# because the repairs differ, not because it is allowed.
REV = re.compile(r"`([0-9a-f]{8})`")
# (2) A PATH REFERENCE resolves by UNIQUE SUFFIX over `git ls-files`, by path
# component OR by file-NAME suffix -- which is what makes the memos' abbreviated
# spellings resolvable (`-audit.sh:325` is a name suffix, not a path one). An
# AMBIGUOUS suffix is a finding, never a silent first match. Two states print as
# LIMITS, both NAMED rather than inferred: a bare `:N` with no path (the memos
# write several; the subject is a sentence away), and a path outside this
# repository by construction -- `memory/` is the user's memory directory, which §7
# already says a line reference cannot serve. EVERY OTHER unresolvable path is a
# FINDING; soften that and a mistyped path degrades into a limit line.
REF = re.compile(r"(?:^|[\s`(\[])(-?[A-Za-z0-9_][A-Za-z0-9_./-]*\.(?:sh|md|py|toml|rs|yml))"
                 r":(\d+)(?:-(\d+))?")
# (3) A STATED LENGTH is checked only for the HARNESS'S OWN files -- the glob this
# census ranges over. A general "**N** lines" needle misfires on every other bolded
# digit these memos carry, and a check that misfires gets switched off: a narrow
# one that runs beats a broad one that does not. Both shapes are keyed to the
# revision the prose names, so a HISTORICAL figure is measured THERE and not
# against the working tree -- reading `(**778**)` as "778 lines today" would make
# this check itself a false claim. LEN2 carries no file, so its subject is the last
# harness file named in the same PARAGRAPH.
LEN1 = re.compile(r"`([^`\s]+\.sh)` is \*\*(\d+)\*\*(?: at `([0-9a-f]{8})`)?")
LEN2 = re.compile(r"\b(?:entered|left) at `([0-9a-f]{8})` \(\*\*(\d+)\*\*\)")
SHF = re.compile(r"`([^`\s]+\.sh)`")
# (4) A BAND CLAIM is LINE-SCOPED, as a rule and not by accident: under paragraph
# scope §3's HYPOTHETICAL merged file "inside the authoring band" reads as a false
# claim about the real file its paragraph names. Band membership is the defect
# draft 11 shipped and draft 12 repaired by hand, so it stops being a habit.
BAND = re.compile(r"\b(past|in|inside|within|below|under) (?:the )?"
                  r"(?:700\s*[-–]\s*800 )?(?:authoring )?band\b", re.I)
# (5) A `D<N>` needs a `- **D<N>` bullet in the disposition's §1. Fences are
# excluded on BOTH sides: inside one a `D<N>` labels the command it introduces,
# which is where §1 writes ones no bullet repeats, so counting those would report
# a dangling reference at the very site that defines it.
DDEF, DUSE = re.compile(r"^- \*\*D(\d+)\b", re.M), re.compile(r"\bD(\d+)\b")
OUTSIDE = ("memory/", "~", "/")

def _git(*a):
    return subprocess.run(("git", "-C", str(ROOT)) + a, capture_output=True, text=True)

def _scan(md):
    """(lineno, text, fenced) over the WHOLE file; a ``` fence line is CODE."""
    fen = False
    for i, s in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
        f0 = s.lstrip().startswith("```")
        fen = fen ^ f0
        yield i, s, fen or f0

def _bad(md, i, s):
    _CBAD.append(s)
    print("   !! %s:%d  %s" % (md.name, i, s))

def _pop(k):
    _POP[k] = _POP.get(k, 0) + 1

def _rel(p):
    """Tracked paths ending in `p`, by path component or by file-name suffix."""
    return sorted({f for f in TRK if f == p or f.endswith("/" + p) or f.endswith(p)})

def _harness(p):
    h = _rel(p)
    return h[0] if len(h) == 1 and h[0].rsplit("/", 1)[-1] in HN else None

def _wcl(rel, rev=None):
    """`wc -l` of a tracked file, at a revision or in the working tree."""
    if rev is None:
        return (ROOT / rel).read_bytes().count(b"\n")
    r = _git("show", "%s:%s" % (rev, rel))
    return r.stdout.count("\n") if r.returncode == 0 else None

# ⚠ THE ARITY IS DERIVED, NOT WRITTEN. This line and the LIMIT below both spelled
# `2` -- a count of the memos that existed when they were written -- so the gate
# announced "2 memos" while ranging over three, and reported the mismatch as a
# LIMIT rather than as the literal going stale. The beta memo raised it with the
# trigger "the slice that lands the fourth memo"; the seam cut that moved §1 out
# of the disposition is that slice. There is nothing left to keep in step: the
# glob is the population, and it names what it read.
print("\n  -- THE MEMOS' OWN QUANTITIES, RUN (whole file, %d 2026-08 memo(s)) --" % len(_M8))
if not _M8:
    raise SystemExit("!! no `2026-08-citation-hygiene-harness-*.md` under %s; every quantity check "
                     "would then report clean for the reason a needle matching nothing does." % HD)
print("     %s" % " ".join(m.name.replace("2026-08-citation-hygiene-harness-", "") for m in _M8))
_ls = _git("ls-files")
if _ls.returncode != 0:
    raise SystemExit("!! `git ls-files` failed under %s; every path would then resolve to "
                     "nothing for a reason that is not 'the file is absent'." % ROOT)
TRK = _ls.stdout.split()
# ⚠ THE DEFINITIONS AND THE RULE ROWS NOW LIVE IN DIFFERENT FILES. §1 was cut out
# of the disposition as a standalone prereq when that memo passed 1000 lines, so
# `D<N>` resolves against the MEASUREMENTS memo while §3's rule rows still resolve
# against the disposition. Two paths, one for each subject; the alternative --
# globbing every memo for a definition -- would let a `D<N>` bullet written
# anywhere satisfy a citation, which is the "somewhere in this file" defect the
# §3 scoping above was landed to remove, one file up.
MEAS = HD / "2026-08-citation-hygiene-harness-measurements.md"
_s1 = re.search(r"^## §1 .*?(?=^## §|\Z)", MEAS.read_text(encoding="utf-8"),
                re.S | re.M) if MEAS.is_file() else None
_dd = set(DDEF.findall(_s1.group(0))) if _s1 else set()
if MEAS.is_file() and not _dd:
    raise SystemExit("!! %s has no `## §1 ` section, or no `- **D<N>` bullet in it, that this "
                     "parser can read; every citation would then dangle for a reason that is "
                     "not 'it has no definition'." % MEAS.name)
if not _dd:  # no memo, so no definitions -- and then a citation cannot DANGLE either
    _CLIM.append("the measurements memo is absent; no `D<N>` was resolved")
for md in _M8:
    subj = None
    for i, s, fen in _scan(md):
        for h in REV.findall(s):
            _rv.setdefault(h, []).append((md, i)); _pop("revision")
        for p, a, b in REF.findall(s):
            _pop("path ref")
            h = _rel(p)
            if len(h) != 1:
                if p.startswith(OUTSIDE):
                    _CLIM.append("%s:%d `%s:%s` is outside this repository" % (md.name, i, p, a))
                else:
                    _bad(md, i, "`%s:%s` names %s" % (p, a, "no tracked file" if not h
                         else "%d tracked files -- %s" % (len(h), " ".join(h))))
                continue
            n = _wcl(h[0])
            for k in (a, b):
                if k and not 1 <= int(k) <= n:
                    _bad(md, i, "`%s:%s` is out of range -- %s is %d line(s)" % (p, k, h[0], n))
        here = [x for x in (_harness(p) for p in SHF.findall(s)) if x]
        subj = here[-1] if here else (subj if s.strip() else None)
        for p, n, rev in LEN1.findall(s):
            rel = _harness(p)
            if rel is None:
                continue
            _pop("stated length")
            if _wcl(rel, rev or None) != int(n):
                _bad(md, i, "`%s` is stated **%s** line(s) at %s -- measured %s"
                     % (p, n, rev or "HEAD", _wcl(rel, rev or None)))
        for rev, n in LEN2.findall(s):
            _pop("stated length")
            if subj is None:
                _bad(md, i, "a **%s**-line figure at `%s` names no harness file in its "
                            "paragraph, so its subject cannot be measured" % (n, rev))
            elif _wcl(subj, rev) != int(n):
                _bad(md, i, "%s is stated **%s** line(s) at `%s` -- measured %s"
                     % (subj, n, rev, _wcl(subj, rev)))
        w = BAND.search(s.replace("*", ""))
        if w and here:
            _pop("band claim")
            v, n = w.group(1).lower(), _wcl(here[0])
            if not (n > 800 if v == "past" else n < 700 if v in ("below", "under")
                    else 700 <= n <= 800):
                _bad(md, i, "%s is called `%s` the 700-800 band -- it is %d line(s)"
                     % (here[0], v, n))
        for x in ([] if fen or not _dd else sorted(set(DUSE.findall(s)))):
            _pop("D<N> citation")
            if x not in _dd:
                _bad(md, i, "cites `D%s`, which §1 of %s does not define" % (x, PLAN.name))
for h, at in sorted(_rv.items()):
    if _git("rev-parse", "-q", "--verify", h + "^{commit}").returncode != 0:
        v = "resolves to no commit in this repository"
    elif _git("merge-base", "--is-ancestor", h, "HEAD").returncode == 0:
        continue
    else:
        # OFF-HEAD IS NOT A DEFECT. A memo on this branch legitimately pins a
        # sibling branch's head -- the slice memos live on `webref-cite-audit-tool`
        # and the note stamps its readings against them. What is a defect is a
        # revision that resolves NOWHERE, which the branch above catches. The
        # distinction stays visible as a LIMIT rather than being dropped, because
        # a pin that has been rewritten out of every ref would then read as clean.
        _CLIM.append("`%s` resolves but is off HEAD (on %s) -- a cross-branch pin, not checked further"
                     % (h, " ".join(x for x
                                    in _git("branch", "-a", "--contains", h).stdout.split()
                                    if x not in "*+")))
        continue
    for md, i in at:
        _bad(md, i, "revision `%s` %s" % (h, v))
print("   §1 defines: %s\n   POPULATION: %s   findings=%d"
      % (" ".join("D" + x for x in sorted(_dd, key=int)),
         "  ".join("%s=%d" % kv for kv in sorted(_POP.items())), len(_CBAD)))
for x in _CLIM + ["a bare `:N` with no path beside it is not reachable from here"]:
    print("   .. LIMIT: %s" % x)
print("\n  HOMES: %d (%d code, %d prose) in %d files; %d with NO named failure."
      % (len(rows), len(rows) - nprose, nprose, len({x[0] for x in rows}), nbad))
print("  LIMITS (not findings -- what this census cannot see, so an absence here")
print("          is not evidence): a home reached through two levels of variable")
print("          indirection, and any home outside `%s*.sh`." % SELF)
if not rows:
    raise SystemExit("!! zero homes found. The block set is written down somewhere; "
                     "a census that found none measured nothing.")
# AN UNCLASSIFIED HOME IS RED. A plan written per class is complete only if every
# home HAS a class, and for three rounds that property was a sentence in a memo
# rather than a check. It is now the check.
unc = [x for x in rows if x[6] == "?"]
if unc:
    print("\n  -- HOMES NO CLASS COVERS -- the per-class plan does not reach these --")
    for fn, no, kind, g, why, txt, cl in unc:
        print("   !! %s:%d  [%s]  %s" % (fn, no, why, txt))
    raise SystemExit("!! %d home(s) fall into no class. A rule per class is complete "
                     "only while this is empty." % len(unc))
if unruled:
    raise SystemExit("!! %d class(es) the census emits have no rule in %s: %s"
                     % (len(unruled), PLAN.name, " ".join(unruled)))
# A CLAIM NOBODY RAN IS RED, on the same argument as an unclassified home above.
# It exits LAST so neither report hides the other; the findings are already
# printed, so an earlier raise loses none of them.
if _CBAD:
    raise SystemExit("!! %d unexecuted claim(s) in the 2026-08 memos, listed above -- a "
                     "quantity a memo asserts and no command produces. Do not edit the "
                     "check to agree with the memo." % len(_CBAD))
HOMESPY
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

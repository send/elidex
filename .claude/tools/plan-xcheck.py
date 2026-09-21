#!/usr/bin/env python3
"""Cross-validate a plan-memo's routing tables against each other.

The concept grep is blind to decisions restated as structured data (PR labels,
checkmarks, invariant indices, cell ranges). This turns those contradictions
into failures instead of grep targets. (Round 6, Axis 3.)

Cell routing is derived from §6's **structural** grouping (its PR headers), not
from prose, and every prose cell list is checked against that per PR. Round 10
mutation-measured the previous union-based harvest: a per-PR reassignment and a
double-booking both passed it.

The numeral scanner is deliberately greedy and has no prose terminator: a numeral
absorbed out of prose surfaces as a `[CELL] … §6 does not define` error rather
than silently joining the set. A `cells …` list in a non-landing sentence is
excluded (`NON_LANDING`) — folding those in silenced the `omits` check for
exactly the harness-dependent cells, which round 14 proved by mutation.

Checks 11b/11c DO read prose. 11b reads every backticked `<path>.rs` span, with or
without a `:line` suffix and with or without a leading word (`elidex-style a/b.rs`).
11c reads every `cell(s) <list>`, where a list is one or more `<digits><letter?>`
tokens — each optionally carrying an attached `(x)` sub-reference, which is dropped
— joined by `,`, `and`, `, and`, `/`, or a dash range that is expanded. A list ends
at the first thing that is not one of those, so a numeral reachable only through
some other joiner is still unread.

What is still unchecked is *claim* integrity — a sentence whose truth
depends on a fact stated elsewhere in the file, in another crate, or in code the
memo never traced. Mutation-confirmed still-open (round 15): §3 row spec
§-numbers, M-row prose PR claims, cell M-attribution, §6 test-placement routing,
deleted slot triggers, claimed crate deps, and a cell reference misattributed to
another cell that also exists.

SCOPE — this program is specific to
`docs/plans/2026-08-line-box-decorated-inline-content.md`. The PR *namespace* is
generic (any `PR-<n><letter>` §5.3 defines, with the characterization PR derived
from §6 rather than named), but four things are this memo's content and will
mis-fire on any other memo: the prereq names check 9/13 accept (`seam-3`,
`dead-arm`, `predicate`), the `EVENT_TAGS` check 13 allows, the umbrella slot
check 8 exempts, and check 12's `SUPERSEDED` line ranges. Do not read a clean run
on a different memo as coverage.
"""
import re, sys, pathlib

# The PR namespace is derived, not fixed. The old literal `PR-1[a-z]` made a
# structurally identical plan numbered `PR-2a` undefined at check 3 and unrouted
# at checks 5-6 -- a rejection of the memo rather than of a contradiction in it
# (F5). What stays memo-specific is listed in the module docstring's SCOPE note.
PR = r"PR-\d+[a-z]"

def sect(s, n):
    m=re.search(rf"^## §{re.escape(n)}\.", s, re.M)
    if not m: return ""
    nxt=re.search(r"^## §", s[m.end():], re.M)
    return s[m.start(): m.end()+ (nxt.start() if nxt else len(s))]

_UNESCAPED_PIPE = re.compile(r"(?<!\\)\|")

def cells_of(line):
    """Columns of one markdown table row: split on UNESCAPED pipes, then unescape.

    A raw `.split("|")` turns a cell containing `\\|` into two columns and shifts
    every positional read after it. On this memo the `border-style` row parsed as
    7 columns instead of 6, so check 4 read prose where the ✓/✗ glyph is and the
    row could be neither flagged nor cleared -- it escaped the check entirely
    rather than failing it."""
    t=line.strip()
    if t.startswith("|"): t=t[1:]
    if t.endswith("|") and not t.endswith("\\|"): t=t[:-1]
    return [x.strip().replace("\\|", "|") for x in _UNESCAPED_PIPE.split(t)]

def rows(table_text):
    """Data rows of a markdown table: no separator, no header."""
    out=[]
    for l in table_text.split("\n"):
        # `|x` (no space after the pipe) is valid markdown too; requiring `| ` let a row
        # written that way escape every table check (gate #3 on rev 25, E4).
        if not l.startswith("|") or set(l) <= set("| -:"): continue
        out.append(cells_of(l))
    return out

def tokens(listing):
    """Expand a '1–4, 3b, 7 and 12d' listing into {'1','2','3','4','3b','7','12d'}."""
    got=set()
    for tok in re.findall(r"\d+[a-z]?(?:\s*[–-]\s*\d+)?", listing):
        tok=tok.replace(" ","")
        if re.fullmatch(r"\d+[–-]\d+", tok):
            a,b=re.split(r"[–-]", tok); got |= {str(i) for i in range(int(a), int(b)+1)}
        else: got.add(tok)
    return got

CELL_LIST = re.compile(r"cells? ((?:\d+[a-z]?(?:\s*[–-]\s*\d+)?(?:\s*(?:,|and)\s*)?)+)")

# A `cell(s) <list>` ANYWHERE in the file (check 11c). A token is digits + an
# optional letter + an optional ATTACHED `(x)` sub-reference; `tokens()` drops the
# sub-reference for free (it harvests digit-led runs), so `17d(b)` yields `17d`.
# The range dash is the one separator that may not cross a line: `\s*` there would
# let a wrapped list swallow the `- ` opening the next markdown bullet. The other
# separators do cross, because real listings wrap mid-list.
_CELL_TOK = r"\d+[a-z]?(?:\([a-z]\))?"
_CELL_SEP = r"(?:\s*(?:,\s*and|,|and|/)\s*|[^\S\n]*[–-][^\S\n]*)"
CELL_REF = re.compile(rf"\b(?i:cells?)\s+({_CELL_TOK}(?:{_CELL_SEP}{_CELL_TOK})*)")

# A `cell(s) <list>` that is NOT a landing claim. `flip`/`do not` mark the flip
# partition; the rest mark cells named for some other reason (unconstructible
# without X, pinned by Y). Folding those into `land` silences the `omits` check
# for them, which is how a dropped harness cell passed round 14.
NON_LANDING = re.compile(r"\s*(flip\b|do\s+not\b|(are|is)\s+unconstructible\b)", re.S)

def cell_lists(text):
    """Every 'cell(s) <list>' in `text`, split into (landing, other)."""
    land, other = set(), set()
    for m in CELL_LIST.finditer(text):
        tail = text[m.end(): m.end()+40]
        (other if NON_LANDING.match(tail) else land).update(tokens(m.group(1)))
    return land, other

def chunk_by(text, header_rx, stop_rx=None):
    """Split `text` into {header-key: body} at every line matching header_rx.

    A chunk ends at the next header OR at the next `stop_rx` line after it —
    without the stop the last chunk swallows the rest of the section, which turns
    a neighbouring bullet's cell reference into a false claim by that PR."""
    marks=[(m.start(), m.group(1)) for m in re.finditer(header_rx, text, re.M)]
    stops=[m.start() for m in re.finditer(stop_rx, text, re.M)] if stop_rx else []
    out={}
    for i,(pos,key) in enumerate(marks):
        end = marks[i+1][0] if i+1 < len(marks) else len(text)
        end = min([end] + [p for p in stops if pos < p < end])
        out[key] = text[pos:end]
    return out

def main(path):
    s=pathlib.Path(path).read_text(); fails=[]
    def bad(c,msg): fails.append(f"[{c}] {msg}")

    # `|**M3**` (no space after the pipe) is valid markdown; requiring `| ` let an
    # M-row written that way vanish from `m_rows`, and with its §2 pair row written
    # the same way the whole coupling escaped checks 1-2 (F2).
    m_rows=set(re.findall(r"^\|\s*\*\*(M\d)\*\*", s, re.M))
    s2, s3, s5, s6, s8, s10 = (sect(s,n) for n in ("2","3","5","6","8","10"))
    ledger_slots=set(re.findall(r"`(#11-[a-z0-9-]+)`", s10))

    # 0. a section this program reads but cannot find yields an EMPTY string, and
    # every check keyed on it then runs zero times and reports success. §8 was the
    # loud case: with it deleted the DoD half of checks 5-6 iterated nothing AND the
    # `elif s8` in check 7 suppressed the missing-partition error, so a memo with its
    # entire definition of done removed exited 0 (F4). An absent section is a
    # contradiction, not a pass.
    for n, body in (("2",s2), ("3",s3), ("5",s5), ("6",s6), ("8",s8), ("10",s10)):
        if not body.strip():
            bad("MISSING", f"§{n} is absent or empty — every check keyed on it ran zero times")

    # 1. §2 pair table references only existing M-rows, and each pair names a PR
    # Go through the shared row parser: the old `startswith("| ")` filter dropped
    # compact `|1 × 2 |…` rows, and its own `.split("|")` mis-columned escaped pipes.
    pairs=[c for c in rows(s2) if any("×" in x for x in c)]
    for c in pairs:
        if len(c)<4: bad("PAIR", f"row has {len(c)} cols: {c[0]}"); continue
        # a coupling may be resolved by an M-row or by deferring it to a named slot,
        # but a slot named as a routing DESTINATION must be one §10 acts on --
        # without this a fabricated `#11-…` name passes both here and the SLOT check.
        if c[2] not in m_rows:
            sl=c[2].strip("`")
            if not sl.startswith("#11-"):
                bad("PAIR", f"{c[0]} -> unknown {c[2]!r}")
            elif sl not in ledger_slots:
                bad("PAIR", f"{c[0]} -> slot {sl} has no §10 ledger row")
    # 2. every M-row appears in some pair
    for r in sorted(m_rows - {x for c in pairs for x in c}):
        bad("PAIR", f"{r} is in no coupling pair")

    # 3. PR labels used anywhere vs PRs §5.3 defines
    defined=set(re.findall(rf"\*\*({PR}) —", s5))
    used=set(re.findall(r"\*\*(PR-\w+)\*\*", s))
    for u in sorted(used-defined):
        bad("PR", f"{u} referenced but not defined in §5.3")

    # 4. §3 rows: routed-to-a-slot rows must be ✗, ✓ rows must not name a slot;
    #    and any slot a Touch cell routes to must have a §10 ledger row
    for c in rows(s3):
        if len(c)<5 or c[0].startswith("Spec section"): continue
        touch, enum = c[3], c[4]
        if "#11-" in touch and enum.startswith("✓"): bad("ENUM", f"row routed to a slot but marked ✓: {c[0][:52]}")
        if "#11-" not in touch and enum.startswith("✗") and "PR-" not in touch and not re.search(r"\bM\d\b", touch) and "§9" not in touch:
            bad("ENUM", f"row marked ✗ with no owner: {c[0][:52]}")
        for sl in re.findall(r"`(#11-[a-z0-9-]+)`", touch):
            if sl not in ledger_slots:
                bad("ROUTE", f"§3 routes to slot {sl}, which has no §10 ledger row")

    # 5-6. cell routing, PER PR. §6's headers are the structural source of truth;
    #      §8's DoD and §5.3's bullets are prose restatements of it.
    cells=set(re.findall(r"^(\d+[a-z]?)\.\s", s6, re.M))
    s6_chunks=chunk_by(s6, rf"^\*\*({PR})\b")
    s6_by_pr={k: set(re.findall(r"^(\d+[a-z]?)\.\s", v, re.M)) for k,v in s6_chunks.items()}
    routed=set().union(*s6_by_pr.values()) if s6_by_pr else set()
    for c in sorted(cells-routed, key=str):
        bad("CELL", f"cell {c} is under no §6 PR heading")
    for pr, own in s6_by_pr.items():
        for other, theirs in s6_by_pr.items():
            if other > pr:
                for c in sorted(own & theirs, key=str):
                    bad("CELL", f"cell {c} is under both {pr}'s and {other}'s §6 heading")

    for label, text, hdr, stop in (("§8 DoD", s8, rf"^\*\*({PR})\*\*", rf"^\*\*(?!{PR}\*\*)"),
                                   ("§5.3", s5, rf"^\* \*\*({PR}) —", rf"^\* (?!\*\*{PR} —)")):
        for pr, body in chunk_by(text, hdr, stop).items():
            land, _ = cell_lists(body)
            for c in sorted(land - cells, key=str):
                bad("CELL", f"{label} {pr} cites cell {c}, which §6 does not define")
            for c in sorted(land - s6_by_pr.get(pr, set()) - (land-cells), key=str):
                owner=[p for p,v in s6_by_pr.items() if c in v]
                bad("CELL", f"{label} {pr} claims cell {c}, which §6 groups under {owner[0] if owner else '(nothing)'}")
            for c in sorted(s6_by_pr.get(pr, set()) - land, key=str):
                bad("CELL", f"{label} {pr} omits cell {c}, which §6 groups under it")

    # 7. the flip set partitions the characterization PR's cells.
    # The characterization PR is read off §6 -- the one whose heading says so, else
    # the first -- not hard-coded to `PR-1a`, so a plan numbered differently is
    # still checked instead of silently comparing against an empty base set.
    char_pr=next((k for k,v in s6_chunks.items() if "characteriz" in v[:200].lower()),
                 next(iter(s6_chunks), None))
    flip=nonflip=None
    m=re.search(r"cells? ([\d,\s a-z–-]+?)\s*flip;\s*([\d,\s a-z–-]+?)\s*do not", s8)
    if m:
        flip, nonflip = tokens(m.group(1)), tokens(m.group(2))
        base=s6_by_pr.get(char_pr, set())
        for c in sorted(flip & nonflip, key=str): bad("FLIP", f"cell {c} is listed as both flipping and not")
        for c in sorted(base - flip - nonflip, key=str): bad("FLIP", f"cell {c} is in neither the flip nor the non-flip list")
        for c in sorted((flip | nonflip) - base, key=str): bad("FLIP", f"flip set names cell {c}, not a characterization cell")
    elif s8:
        bad("FLIP", "§8 states no 'cells … flip; … do not' partition")

    # 8. every slot named has a §5.3 definition and a §10 ledger row
    for sl in sorted(set(re.findall(r"`(#11-[a-z0-9-]+)`", s))):
        if sl in ("#11-line-box-decorated-inline-content",): continue
        ctx = "".join(s[m.start(): m.start()+900] for m in re.finditer(rf"`{re.escape(sl)}`", s))
        defined_here = ("Re-eval:" in ctx or "re-eval" in ctx.lower()) and "rigger" in ctx
        if defined_here and sl not in ledger_slots:
            bad("SLOT", f"{sl} is defined with Why/trigger/re-eval but has no §10 ledger row")

    # 9. own-deferral bookkeeping: §5.3's per-PR statement vs §10's own-tagged rows
    stated={}
    for m in re.finditer(rf"({PR}|seam-3 prereq|dead-arm prereq) opens? (\d+|none)", s5):
        stated[m.group(1)] = 0 if m.group(2)=="none" else int(m.group(2))
    if not stated:
        bad("COUNT", "§5.3 states no per-PR own-deferral count")
    else:
        actual={}
        for c in rows(s10):
            if len(c)<2 or "(own)" not in c[0]: continue
            key=c[1].strip("* `").replace(" PR","")
            actual[key]=actual.get(key,0)+1
        for pr,n in sorted(stated.items()):
            got=actual.get(pr,0)
            if got!=n: bad("COUNT", f"§5.3 says {pr} opens {n} own deferral(s); §10 has {got}")
            if n>3: bad("COUNT", f"{pr} opens {n} own deferrals, over the per-PR cap of 3")
        for pr in sorted(set(actual)-set(stated)):
            bad("COUNT", f"§10 tags {actual[pr]} own deferral(s) to {pr}, which §5.3 does not account for")

    # 9b. §5.3's per-PR "Owns couplings …" must reproduce §2's PR column exactly
    s2_own={}
    for c in pairs:
        if len(c)>=4 and not c[3].strip("`* ").startswith("#11-"):
            s2_own.setdefault("PR-"+c[3].strip("`* "), set()).add(c[0])
    if s2_own:
        for pr, body in chunk_by(s5, rf"^\* \*\*({PR}) —", rf"^\* (?!\*\*{PR} —)").items():
            m=re.search(r"Owns couplings? (.+?)(?:\n\n|$)", body, re.S)
            if not m: bad("OWN", f"§5.3 {pr} states no coupling ownership"); continue
            claimed={re.sub(r"\s*×\s*", " × ", t)
                     for t in re.findall(r"\d+\s*×\s*(?:\d+|\(\w+\))", m.group(1))}
            for p in sorted(claimed - s2_own.get(pr,set())): bad("OWN", f"§5.3 {pr} claims coupling {p}, §2 routes it elsewhere")
            for p in sorted(s2_own.get(pr,set()) - claimed): bad("OWN", f"§2 routes coupling {p} to {pr}, whose §5.3 bullet omits it")

    # 10. §3's PR column and §2's PR column must name a PR §5.3 defines (or a slot)
    prs = defined | {"prereq"}
    for c in pairs:
        if len(c)>=4:
            tok=c[3].strip("`* ")
            if tok and not tok.startswith("#11-") and f"PR-{tok}" not in prs and tok not in prs:
                bad("ROUTE", f"§2 pair {c[0][:28]} -> unknown PR {tok!r}")
    for l in s3.split("\n"):
        if not l.startswith("|") or set(l) <= set("| -:"): continue
        for tok in re.findall(PR, l):
            if tok not in defined: bad("ROUTE", f"§3 names undefined {tok}")

    # 11. §3's stated breadth must match the table it summarises
    data=[c for c in rows(s3) if len(c)>=4 and not c[0].startswith("Spec section")]
    m_actual=len(data)
    k_actual=len({re.sub(r" §.*", "", c[0]).strip() for c in data})
    km=re.search(r"K=(\d+) specs.*?M=(\d+) entries", s3, re.S)
    if not km: bad("COUNT", "§3 states no K=/M= breadth")
    else:
        if int(km.group(1))!=k_actual: bad("COUNT", f"§3 says K={km.group(1)}, table has {k_actual} distinct specs")
        if int(km.group(2))!=m_actual: bad("COUNT", f"§3 says M={km.group(2)}, table has {m_actual} data rows")
    # every OTHER K=/M= statement in §3 must agree too -- the split verdict restates K
    for lbl, actual in (("K", k_actual), ("M", m_actual)):
        for m in re.finditer(rf"{lbl}=(\d+)", s3):
            if int(m.group(1))!=actual:
                bad("COUNT", f"§3 restates {lbl}={m.group(1)} where the table has {actual}")

    # 11b. every `crates/...` path the memo cites must exist on disk
    root=pathlib.Path(__file__).resolve().parents[2]
    # Most of the memo's paths are crate-relative (`inline/pack/boxes.rs`), not
    # `crates/…`-prefixed -- including round 13's fabricated `builder/block/mod.rs`,
    # which the prefixed-only form could not see. Resolve a bare path by suffix
    # against the tracked file list, which is also what a reader does.
    tracked=[str(q.relative_to(root)) for q in root.glob("crates/**/*.rs")]
    # Files this program CREATES are exempt, but the `(NEW)` annotation need only
    # appear ONCE -- later mentions use the short form. Harvest the annotated set
    # first, then exempt by suffix, so a NEW file can be referenced normally while
    # a fabricated path still fails.
    new_files=set()
    for m in re.finditer(r"`([A-Za-z0-9_./{},-]+\.rs)`?[^`\n]{0,12}\(NEW", s):
        raw=m.group(1)
        b=re.search(r"\{([^}]*)\}", raw)   # `…/{stream,geometry,existence}.rs` expands
        new_files |= ({raw.replace(b.group(0), part.strip()) for part in b.group(1).split(",")}
                      if b else {raw})
    def is_new(f):
        return any(f.endswith(n) or n.endswith(f) or n.endswith("/"+f) for n in new_files)
    seen=set()
    # A span may name the crate before the path (`elidex-style resolve/…/mod.rs:260`);
    # anchoring the path to the opening backtick left those never existence-checked.
    for m in re.finditer(r"`(?:[A-Za-z0-9_-]+ )?([A-Za-z0-9_./-]+\.rs)(?::[\d-]+)?`", s):
        f=m.group(1)
        if f in seen: continue
        seen.add(f)
        if is_new(f): continue
        if f.startswith("crates/"):
            if not (root/f).is_file(): bad("PATH", f"cited file does not exist: {f}")
        elif not any(t.endswith("/"+f) for t in tracked):
            bad("PATH", f"cited path matches no file under crates/: {f}")

    # 11c. a `cell N` named ANYWHERE must be one §6 defines -- §5.3 and §8 are
    #      cross-checked per PR above, but §2/§5.1/§5.2/§7/§9/§10 were not read at all
    for m in CELL_REF.finditer(s):
        for tok in sorted(tokens(m.group(1))):
            if tok not in cells:
                bad("CELL", f"cell {tok} referenced but §6 does not define it")

    # 12. a superseded line-range must not survive beside its corrected form
    SUPERSEDED = {":264-301": ":266-303", ":386-409": ":388-411", ":411-637": ":413-639"}
    MARK = ("predates", "superseded", "pre-#497", "carried from")
    for old, new in SUPERSEDED.items():
        if new not in s: continue
        for ln in s.split("\n"):
            if old in ln and not any(m in ln for m in MARK):
                bad("RANGE", f"{old} (superseded) restated alongside {new}: {ln.strip()[:60]}")

    # 13. bookkeeping routing (rev 25, Step 4.5 root re-derivation): a §10 row tagged to
    #     a prereq PR that §8 records as LANDED must carry a ✅ discharge — otherwise the
    #     memo books work to a PR that can no longer carry it, which is exactly how the
    #     seam-3 narrowing (#508) went unnoticed by checks 1-12. Rows tagged to the
    #     umbrella's own EVENTS (approval / tooling) must use a label §8 defines, so a
    #     row cannot be routed to a PR letter whose change does not make it true.
    # A landing record is a ✅ inside a BOLD span in §8 (`**Seam-3 prereq PR — ✅ landed …**`),
    # the discharge-heading form this memo uses — not any one spelling of "landed" (gate #3
    # on rev 25 showed a reworded landing silently emptied a spelling-keyed set), and not a
    # bare ✅ in prose (§8 also *talks about* the glyph). Every such bold span must name the
    # prereq it discharges; one that names none is the positive control failing.
    # The heading is a bold span that OPENS A LINE (`^**…`); it may wrap across lines (the
    # memo hard-wraps at 100 and a heading already exceeds it), so `[^*]` spans newlines —
    # and anchoring at line start is what keeps a mid-line `**a** ✅ **b**` from reading as
    # a heading (gate #4 on rev 25).
    landed_prereqs=set()
    for m in re.finditer(r"^\*\*([^*]*✅[^*]*)\*\*", s8, re.M):
        span=m.group(1)
        if re.search(r"~~[^~]*✅[^~]*~~", span): continue
        named=re.findall(r"(?i)(seam-3|dead-arm|predicate) prereq PR", span)
        if not named:
            bad("LANDED", f"§8 landing heading names no prereq — check 13 cannot attribute it: {span.strip()[:60]}")
        landed_prereqs.update(n.lower() for n in named)
    def discharged(text):
        # a struck-through ✅ (~~✅ …~~) is a retraction, and a backticked `✅` is a mention
        return "✅" in re.sub(r"~~[^~]*~~|`[^`]*`", "", text)
    EVENT_TAGS=("approval PR", "tooling PR")
    # closed world: a PR column value is a defined PR-1x, a named prereq PR, an event
    # label §8 defines, or the memory-bookkeeping discharge. Anything else (a typo, a
    # renamed event, a PR letter §5.3 never defined) is a routing to nowhere.
    for c in rows(s10):
        if len(c)<2 or c[0].startswith("Action"): continue
        tag=c[-1].strip("* `")
        # the action text is everything but the PR column: `rows()` splits on every `|`,
        # and a backticked `grep 'a\|b'` inside a row would otherwise hide its ✅ in c[1]
        act="|".join(c[:-1])
        m=re.fullmatch(r"(seam-3|dead-arm|predicate) prereq PR", tag)
        if m and m.group(1) in landed_prereqs and not discharged(act):
            bad("LANDED", f"§10 row tagged to the landed {tag} is not discharged (no ✅): {act[:60]}")
        elif m and m.group(1) not in landed_prereqs and discharged(act):
            # the reverse direction: a discharged row for a prereq §8 does not record as
            # landed -- either the landing heading was lost (a wrap, a rename) or the row
            # claims a landing that never happened; both must be loud
            bad("LANDED", f"§10 row tagged to {tag} is ✅-discharged but §8 records no landing for it: {act[:60]}")
        elif tag=="done (memory)":
            # a memory discharge must say it was done, and WHEN: the date must sit with the
            # ✅ (within the discharge phrase), not anywhere in a row that also carries
            # re-eval dates and landing dates of other things
            if not discharged(act) or not re.search(r"✅[^|]{0,40}\b20\d\d-\d\d-\d\d\b", re.sub(r"`[^`]*`", "", act)):
                bad("DONE", f"§10 row tagged 'done (memory)' without a ✅ carrying its own date: {act[:60]}")
        elif not m and tag not in defined:
            if tag not in EVENT_TAGS:
                bad("TAG", f"§10 row routed to {tag!r}, which is no defined PR, prereq, or §8 event: {c[0][:60]}")
            elif not re.search(rf"tag(?:s|ged)\s+`{re.escape(tag)}`", s8):
                bad("EVENT", f"§10 uses event tag {tag!r} but §8 does not define it (needs 'tags `{tag}`' / 'tagged `{tag}`')")

    print(f"cross-check: {len(fails)} contradiction(s)  [cells {len(cells)} defined / "
          f"{len(routed)} routed by §6, {len(m_rows)} M-rows, {len(defined)} PRs, "
          f"§3 K={k_actual}/M={m_actual}]")
    for f in fails: print("  "+f)
    return 1 if fails else 0

sys.exit(main(sys.argv[1]))

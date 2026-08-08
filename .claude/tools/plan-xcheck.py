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
than silently joining the set. Per-PR harvesting bounds the remaining masking
case to a cell the same PR already owns, which is idempotent.

Still tabular-surface only: nothing here reads the memo's prose claims.
"""
import re, sys, pathlib

def sect(s, n):
    m=re.search(rf"^## §{re.escape(n)}\.", s, re.M)
    if not m: return ""
    nxt=re.search(r"^## §", s[m.end():], re.M)
    return s[m.start(): m.end()+ (nxt.start() if nxt else len(s))]

def rows(table_text):
    """Data rows of a markdown table: no separator, no header."""
    out=[]
    for l in table_text.split("\n"):
        if not l.startswith("| ") or set(l) <= set("| -:"): continue
        out.append([x.strip() for x in l.strip("|").split("|")])
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

def cell_lists(text):
    """Every 'cell(s) <list>' in `text`, split into (landing, flipping)."""
    land, flip = set(), set()
    for m in CELL_LIST.finditer(text):
        tail = text[m.end(): m.end()+40]
        (flip if re.match(r"\s*(flip|do not)", tail) else land).update(tokens(m.group(1)))
    return land, flip

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

    m_rows=set(re.findall(r"^\| \*\*(M\d)\*\*", s, re.M))
    s2, s3, s5, s6, s8, s10 = (sect(s,n) for n in ("2","3","5","6","8","10"))
    ledger_slots=set(re.findall(r"`(#11-[a-z0-9-]+)`", s10))

    # 1. §2 pair table references only existing M-rows, and each pair names a PR
    pairs=[l for l in s2.split("\n") if l.startswith("| ") and "×" in l]
    for l in pairs:
        c=[x.strip() for x in l.strip("|").split("|")]
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
    for r in sorted(m_rows - {c.strip() for l in pairs for c in l.strip('|').split('|')}):
        bad("PAIR", f"{r} is in no coupling pair")

    # 3. PR labels used anywhere vs PRs §5.3 defines
    defined=set(re.findall(r"\*\*(PR-1[abc]) —", s5))
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
    s6_by_pr={k: set(re.findall(r"^(\d+[a-z]?)\.\s", v, re.M))
              for k,v in chunk_by(s6, r"^\*\*(PR-1[abc])\b").items()}
    routed=set().union(*s6_by_pr.values()) if s6_by_pr else set()
    for c in sorted(cells-routed, key=str):
        bad("CELL", f"cell {c} is under no §6 PR heading")
    for pr, own in s6_by_pr.items():
        for other, theirs in s6_by_pr.items():
            if other > pr:
                for c in sorted(own & theirs, key=str):
                    bad("CELL", f"cell {c} is under both {pr}'s and {other}'s §6 heading")

    for label, text, hdr, stop in (("§8 DoD", s8, r"^\*\*(PR-1[abc])\*\*", r"^\*\*(?!PR-1[abc]\*\*)"),
                                   ("§5.3", s5, r"^\* \*\*(PR-1[abc]) —", r"^\* (?!\*\*PR-1[abc] —)")):
        for pr, body in chunk_by(text, hdr, stop).items():
            land, _ = cell_lists(body)
            for c in sorted(land - cells, key=str):
                bad("CELL", f"{label} {pr} cites cell {c}, which §6 does not define")
            for c in sorted(land - s6_by_pr.get(pr, set()) - (land-cells), key=str):
                owner=[p for p,v in s6_by_pr.items() if c in v]
                bad("CELL", f"{label} {pr} claims cell {c}, which §6 groups under {owner[0] if owner else '(nothing)'}")
            for c in sorted(s6_by_pr.get(pr, set()) - land, key=str):
                bad("CELL", f"{label} {pr} omits cell {c}, which §6 groups under it")

    # 7. the flip set partitions the characterization PR's cells
    flip=nonflip=None
    m=re.search(r"cells? ([\d,\s a-z–-]+?)\s*flip;\s*([\d,\s a-z–-]+?)\s*do not", s8)
    if m:
        flip, nonflip = tokens(m.group(1)), tokens(m.group(2))
        base=s6_by_pr.get("PR-1a", set())
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
    for m in re.finditer(r"(PR-1[abc]|seam-3 prereq|dead-arm prereq) opens? (\d+|none)", s5):
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
    for l in pairs:
        c=[x.strip() for x in l.strip("|").split("|")]
        if len(c)>=4 and not c[3].strip("`* ").startswith("#11-"):
            s2_own.setdefault("PR-"+c[3].strip("`* "), set()).add(c[0])
    if s2_own:
        for pr, body in chunk_by(s5, r"^\* \*\*(PR-1[abc]) —", r"^\* (?!\*\*PR-1[abc] —)").items():
            m=re.search(r"Owns couplings? (.+?)(?:\n\n|$)", body, re.S)
            if not m: bad("OWN", f"§5.3 {pr} states no coupling ownership"); continue
            claimed={re.sub(r"\s*×\s*", " × ", t)
                     for t in re.findall(r"\d+\s*×\s*(?:\d+|\(\w+\))", m.group(1))}
            for p in sorted(claimed - s2_own.get(pr,set())): bad("OWN", f"§5.3 {pr} claims coupling {p}, §2 routes it elsewhere")
            for p in sorted(s2_own.get(pr,set()) - claimed): bad("OWN", f"§2 routes coupling {p} to {pr}, whose §5.3 bullet omits it")

    # 10. §3's PR column and §2's PR column must name a PR §5.3 defines (or a slot)
    prs = defined | {"prereq"}
    for l in pairs:
        c=[x.strip() for x in l.strip("|").split("|")]
        if len(c)>=4:
            tok=c[3].strip("`* ")
            if tok and not tok.startswith("#11-") and f"PR-{tok}" not in prs and tok not in prs:
                bad("ROUTE", f"§2 pair {c[0][:28]} -> unknown PR {tok!r}")
    for l in s3.split("\n"):
        if not l.startswith("| ") or "---" in l: continue
        for tok in re.findall(r"PR-1[a-z]", l):
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

    # 12. a superseded line-range must not survive beside its corrected form
    SUPERSEDED = {":264-301": ":266-303", ":386-409": ":388-411", ":411-637": ":413-639"}
    MARK = ("predates", "superseded", "pre-#497", "carried from")
    for old, new in SUPERSEDED.items():
        if new not in s: continue
        for ln in s.split("\n"):
            if old in ln and not any(m in ln for m in MARK):
                bad("RANGE", f"{old} (superseded) restated alongside {new}: {ln.strip()[:60]}")

    print(f"cross-check: {len(fails)} contradiction(s)  [cells {len(cells)} defined / "
          f"{len(routed)} routed by §6, {len(m_rows)} M-rows, {len(defined)} PRs, "
          f"§3 K={k_actual}/M={m_actual}]")
    for f in fails: print("  "+f)
    return 1 if fails else 0

sys.exit(main(sys.argv[1]))

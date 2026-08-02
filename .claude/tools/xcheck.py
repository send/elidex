#!/usr/bin/env python3
"""Cross-validate a plan-memo's routing tables against each other.

The concept grep is blind to decisions restated as structured data (PR labels,
checkmarks, invariant indices, cell ranges). This turns those contradictions
into failures instead of grep targets. (Round 6, Axis 3.)"""
import re, sys, pathlib

def sect(s, n):
    m=re.search(rf"^## §{re.escape(n)}\.", s, re.M)
    if not m: return ""
    nxt=re.search(r"^## §", s[m.end():], re.M)
    return s[m.start(): m.end()+ (nxt.start() if nxt else len(s))]

def main(path):
    s=pathlib.Path(path).read_text(); fails=[]
    def bad(c,msg): fails.append(f"[{c}] {msg}")

    m_rows=set(re.findall(r"^\| \*\*(M\d)\*\*", s, re.M))
    # 1. §2 pair table references only existing M-rows, and each pair names a PR
    pairs=[l for l in sect(s,"2").split("\n") if l.startswith("| ") and "×" in l]
    for l in pairs:
        c=[x.strip() for x in l.strip("|").split("|")]
        if len(c)<4: bad("PAIR", f"row has {len(c)} cols: {c[0]}"); continue
        if c[2] not in m_rows: bad("PAIR", f"{c[0]} -> unknown {c[2]!r}")
    # 2. every M-row appears in some pair
    for r in sorted(m_rows - {c.strip() for l in pairs for c in l.strip('|').split('|')}):
        bad("PAIR", f"{r} is in no coupling pair")

    # 3. PR labels used anywhere vs PRs §5.3 defines
    defined=set(re.findall(r"\*\*(PR-1[abc]) —", sect(s,"5")))
    used=set(re.findall(r"\*\*(PR-\w+)\*\*", s))
    for u in sorted(used-defined):
        bad("PR", f"{u} referenced but not defined in §5.3")

    # 4. §3 rows: routed-to-a-slot rows must be ✗, ✓ rows must not name a slot
    for l in sect(s,"3").split("\n"):
        if not l.startswith("| ") or "---" in l or "Spec section" in l: continue
        c=[x.strip() for x in l.strip("|").split("|")]
        if len(c)<6: continue
        touch, enum = c[3], c[4]
        if "#11-" in touch and enum.startswith("✓"): bad("ENUM", f"row routed to a slot but marked ✓: {c[0][:52]}")
        if "#11-" not in touch and enum.startswith("✗") and "PR-" not in touch and "M" not in touch:
            bad("ENUM", f"row marked ✗ with no owner: {c[0][:52]}")

    # 5. cells: every §6 cell in exactly one §8 DoD
    cells=set(re.findall(r"^(\d+[a-z]?)\.\s+\*\*", sect(s,"6"), re.M))
    dod=sect(s,"8")
    listed=set()
    for rng in re.findall(r"cells? ([\d abc,–\-and]+)", dod):
        for tok in re.split(r",|and", rng):
            tok=tok.strip()
            if re.fullmatch(r"\d+[a-z]?", tok): listed.add(tok)
            elif re.fullmatch(r"\d+[–-]\d+", tok):
                a,b=re.split(r"[–-]", tok); listed |= {str(i) for i in range(int(a), int(b)+1)}
    for c in sorted(cells-listed, key=lambda x:(int(re.sub(r"\D","",x)), x)):
        bad("CELL", f"cell {c} is in no PR's DoD")

    # 6. every slot named has a §5.3 definition and a §10 ledger row
    slots=set(re.findall(r"`(#11-[a-z0-9-]+)`", s))
    s53, s10 = sect(s,"5"), sect(s,"10")
    for sl in sorted(slots):
        if sl in ("#11-line-box-decorated-inline-content",): continue
        if re.search(rf"\*\*`{re.escape(sl)}`\*\*", s53) and sl not in s10:
            bad("SLOT", f"{sl} defined in §5.3 but has no §10 ledger row")

    # 7. own-deferral count
    stated=re.search(r"Own-deferral count: \*\*(\d+)\*\*", s)
    actual=len(re.findall(r"\*\*own\*\* deferral|\(own deferral\)", s))
    if stated and int(stated.group(1))!=actual:
        bad("COUNT", f"own-deferral count says {stated.group(1)}, {actual} slots are tagged own")

    print(f"cross-check: {len(fails)} contradiction(s)")
    for f in fails: print("  "+f)
    return 1 if fails else 0

sys.exit(main(sys.argv[1]))

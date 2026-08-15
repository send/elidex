#!/usr/bin/env python3
"""§4.2's tiers and §4.4's verbs, transcribed a second time -- to be deleted.

This is a bootstrap: it is the grammar of §4.2/§4.4 transcribed from the memo's
literal text, so that the memo's own live annotations are re-executed now, while
the wire that will own this does not yet exist.

That second copy is a BOUND, and the memo records it as one in §0 and §6: the
checker "carries a second transcription of §4.2's tiers and §4.4's verbs, and
will keep carrying it until the wire exists and delegation to it is implemented
and proven by a control". Putting the transcription in its own file makes the
bound a file. When `.claude/tools/claim-provenance-trip-wire.sh` is tracked and
`check_annotations` shells out to it -- proven by a control, not by reading --
this module is deleted whole, and the deletion takes nothing else with it.

So this is not a general-purpose module and must not be made into one: it exists
to be deleted. `check_tier_transcription` (check 14, in the entry point) is what
holds this copy and the memo's copy together until then, and is the only reason
a second copy is tolerable at all.

`host` is the entry point, which binds it to itself before any of this runs:
`REPO`, `sh()`, `nr()` and `bad()` live there, and `claim-gate-plan-check.py`
has hyphens in its name and cannot be imported. `bad` is reached through `host`
on every call and never bound here: the self-test's `capture()` swaps the entry
point's `FAILS` list out for the duration of one check, and only a call-time
lookup sees the replacement.
"""
from __future__ import annotations

import re
from types import ModuleType

host: ModuleType  # bound by the entry point at import time

LAX = re.compile(r"<!--[ \t]*claim[^A-Za-z]")
STRICT = re.compile(
    r"<!--[ \t]+claim[ \t]+"
    r"(?:"
    r"(?P<cf>count|files)=(?P<n1>[0-9]+)[ \t]+(?:(?P<icase>icase)[ \t]+)?"
    r'pattern="(?P<cfpat>[^"]*)"[ \t]+in="(?P<cfin>[^"]*)"'
    r"|"
    r'lines=(?P<n2>[0-9]+)[ \t]+in="(?P<lnin>[^"]*)"'
    r")"
    r"[ \t]+-->"
)
BAD_ESCAPE = re.compile(r"\\[bBdDsSwW<>]")


def classify(lines: list[str]) -> list[tuple[int, str, dict | None]]:
    """(line-number, tier, fields) for every candidate line in one file.

    Takes no filename: §4.2's inertness rule is a line predicate identical on
    every surface, so nothing here is per-file. That is structural rather than
    incidental -- a surface argument is precisely what a second arm would need."""
    out = []
    for i, line in enumerate(lines):
        m = LAX.search(line)
        if not m:
            continue
        k = line[: m.start()].count("`")
        if k % 2 == 1:
            out.append((i + 1, "mention", None))
            continue
        sm = STRICT.search(line)
        if not sm:
            out.append((i + 1, "malformed", None))
            continue
        g = sm.groupdict()
        if g["cf"]:
            f = {"verb": g["cf"], "n": int(g["n1"]), "icase": bool(g["icase"]),
                 "pattern": g["cfpat"], "spec": g["cfin"]}
        else:
            f = {"verb": "lines", "n": int(g["n2"]), "icase": False,
                 "pattern": None, "spec": g["lnin"]}
        esc = BAD_ESCAPE.search(f["pattern"]) if f["pattern"] is not None else None
        if esc:
            out.append((i + 1, "malformed", {"why": f"pattern contains {esc.group(0)}"}))
            continue
        out.append((i + 1, "strict", f))
    return out


def execute(f: dict) -> tuple[bool, int | str]:
    _, out = host.sh(["git", "ls-files", "--", f["spec"]])
    live = [x for x in out.split("\n") if x]
    if f["verb"] == "lines":
        if len(live) != 1:
            return False, f"in= resolves to {len(live)} tracked files, not exactly 1"
        return True, host.nr(host.REPO / live[0])
    if not live:
        return False, "in= matches no tracked file"
    flag = "-nE" if f["verb"] == "count" else "-lE"
    if f["icase"]:
        flag += "i"
    _, out = host.sh(["git", "grep", flag, "-e", f["pattern"], "--", f["spec"]])
    return True, len([x for x in out.split("\n") if x])


def scan_tree() -> dict[str, list[tuple[int, str, dict | None]]]:
    _, out = host.sh(["git", "grep", "-lE", r"<!--[[:space:]]*claim[^A-Za-z]", "--",
                      "*.md", "*.rs", "*.sh", "*.toml",
                      ":(exclude).claude/tools/**"])
    res = {}
    for rel in [x for x in out.split("\n") if x]:
        res[rel] = classify((host.REPO / rel).read_text().split("\n"))
    return res


# Split from `check_annotations` so the verdict is a PURE function of an
# already-scanned tree. It was inlined behind `scan_tree()`, which reads the real
# repository, and three of its arms -- the MALFORMED report, the asserted-vs-
# actual mismatch, and §8.2's per-verb acceptance property -- were therefore
# unprobeable: firing any of them meant planting a bad annotation in a tracked
# file. All three shipped with no firing proof at all, which is the `ban_control`
# state §4.3 names, in the check that exists to execute the memo's own claims.
#
# Pure means it RETURNS its findings; the caller reports them. Reaching back for
# the host's `bad` would not be pure, and would be the one coupling that is not
# safe to have here: `capture()` swaps the entry point's FAILS list for the
# duration of one check, so a finding raised through a stale binding lands in the
# swapped-out list and the probe sees a clean run. Every ANNO negative would
# invert silently -- the failure this module's own three new probes exist to
# catch, reproduced in the wiring beneath them.
def verdict_over(
    tree: dict[str, list[tuple[int, str, dict | None]]],
) -> tuple[dict, list[tuple[str, str]]]:
    tiers = {"strict": 0, "mention": 0, "malformed": 0}
    verbs: set[str] = set()
    results = []
    found: list[tuple[str, str]] = []
    for rel, hits in sorted(tree.items()):
        for ln, tier, f in hits:
            tiers[tier] += 1
            if tier == "malformed":
                why = (f or {}).get("why", "does not match the §4.4 production")
                found.append(("ANNO", f"{rel}:{ln} classifies MALFORMED ({why})"))
            if tier != "strict" or f is None:
                continue
            verbs.add(f["verb"])
            ok, got = execute(f)
            if not ok:
                found.append(("ANNO", f"{rel}:{ln} {f['verb']}={f['n']}: {got}"))
                continue
            results.append((rel, ln, f, got))
            if got != f["n"]:
                found.append(("ANNO", f"{rel}:{ln} asserts {f['verb']}={f['n']}, "
                                      f"actual={got}  in={f['spec']}"))

    # §8.2's acceptance property, executed rather than asserted.
    for v in ("count", "files", "lines"):
        if v not in verbs:
            found.append(("ANNO", f"§8.2's property requires ≥1 strict `{v}` "
                                  f"annotation in the tracked tree; there are none"))
    return {"tiers": tiers, "results": results}, found

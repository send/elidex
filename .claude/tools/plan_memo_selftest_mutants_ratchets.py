#!/usr/bin/env python3
"""The mutation-proof rows against the RATCHETS (`plan_memo_selftest_ratchets.py`)
-- the sixth module of the ONE `MUTANTS` list, and the first carved on a
SUBJECT rather than a review round (the shape `plan_memo_selftest_cases_sibling.py`
set for the controls).

⚠ WHY A SUBJECT SEAM (the fourth attestation over `9b2e1fa9..00dfd095`).  Until
this module, no row named the ratchets at all: reverting the loop ratchet to
the keying it had replaced (a target prefix), or the kind-question ratchet to
its census-only population, left the whole proof green -- the ratchets were the
controls that decide whether a class is covered, and nothing decided whether
THEY were.  Each row here re-injects one criterion a ratchet HAD, against the
ratchet's discriminating partner, so the next reversion is a killed mutant and
not a sentence.  `_mutants_r30.py` (981 lines) could not take them without
crossing the 1000-line bound, and a subject is the seam these rows share.

`MUTANTS` is imported and appended to, exactly as the other five do; the runner
reads the one list at one import site.
"""

from plan_memo_selftest_mutants import MUTANTS, RATCHETS

SCOPE_PARTNER = ("PROPERTY: the loop ratchet credits a loop by POSITION: one row truncating one of two "
                 "loops that share a header pins that loop and no other")
KIND = "PROPERTY: the kind-phrase questions are asked at their sanctioned sites only -- \"which kind does this field DECLARE\" and \"does this cell CLAIM one, under either reading\" are two questions with one site each, and a third caller asking either one directly is the shape three rounds of findings had"
KIND_PARTNER = ("PROPERTY: the kind-question ratchet reports a caller in ANY module of the checker set, "
                "judged by (module, qualified function) -- not a caller outside a listed subset, not "
                "one whose bare name is sanctioned elsewhere")

# The ONE line of `_credit` that decides a row's credit.  Every loop-keying row
# widens it to a coarser identity, which is what each earlier version was.
_HITS = ('            hits = {k for k, node in base.items()\n'
         '                    if k in patched and _is_truncation(patched[k].iter, node.iter)}\n')

MUTANTS += [
    ("ratchets: a loop is credited by POSITION (re-inject the exact-header-LINE keying of 799349db -- "
     "one row pins every loop spelled like its target)", RATCHETS,
     _HITS,
     _HITS + '            hits = {j for j in base for k in hits\n'
             '                    if _header(text, base[j]) == _header(text, base[k])}\n',
     [SCOPE_PARTNER]),
    ("ratchets: a loop is credited by POSITION (re-inject the target-PREFIX keying 799349db replaced -- "
     "`for x in ` pins every loop over any iterable)", RATCHETS,
     _HITS,
     _HITS + '            hits = {j for j in base for k in hits\n'
             '                    if _header(text, base[j]).partition(" in ")[0]\n'
             '                    == _header(text, base[k]).partition(" in ")[0]}\n',
     [SCOPE_PARTNER]),
    ("ratchets: a loop is credited by the row's EDIT, not its find (re-inject the first "
     "version's find-text keying -- a loop the find merely contains reads as pinned)", RATCHETS,
     _HITS,
     '            hits = {k for k, node in base.items() if _header(text, node) in find}\n',
     [SCOPE_PARTNER]),
    ("ratchets: the exempt complement is guarded by its header (drop the guard -- an edit that "
     "shifts the ordinals exempts whichever loop moved into the position)", RATCHETS,
     '        elif _header(text, loops[k]) != want:',
     '        elif False:',
     [SCOPE_PARTNER]),
    ("ratchets: an exempt entry naming no loop is red (drop the check -- a stale complement "
     "exempts nothing and says so to nobody)", RATCHETS,
     '        if k not in loops:\n            bad.append(',
     '        if k not in loops:\n            continue\n            bad.append(',
     [SCOPE_PARTNER]),
    ("ratchets: the kind-question population is the WHOLE checker set (re-inject the census "
     "module alone -- the population before 799349db)", RATCHETS,
     '    return [file for _name, file in MODULES]',
     '    return ["plan_memo_population.py"]',
     [KIND, KIND_PARTNER]),
    ("ratchets: the kind-question population is DERIVED from the module set (re-inject 799349db's "
     "hand-written five-tuple -- a caller in the sibling resolver or the entry point is outside "
     "it)", RATCHETS,
     '    return [file for _name, file in MODULES]',
     '    return ["plan_memo_population.py", "plan_memo_roles.py", "plan_memo_tables.py",\n'
     '            "plan_memo_memo.py", "plan_memo_stream.py"]',
     [KIND_PARTNER]),
    ("ratchets: a sanction is keyed on (module, qualified function) (re-inject the bare-name "
     "keying -- a `_kind` in another module asks with `Population._kind`'s sanction)", RATCHETS,
     '            if (mod, caller) in sites[name]:',
     '            if caller.rsplit(".", 1)[-1] in {c.rsplit(".", 1)[-1] for _m, c in sites[name]}:',
     [KIND_PARTNER]),
    ("ratchets: a sanctioned site that makes no call is red (drop the direction -- a sanction "
     "outlives the caller it was written for)", RATCHETS,
     '            if (name, mod, caller) not in seen:',
     '            if False:',
     [KIND_PARTNER]),
]

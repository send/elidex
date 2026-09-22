#!/usr/bin/env python3
"""The mutation-proof rows against the RATCHETS (`plan_memo_selftest_ratchets.py`)
-- the sixth mutants module carved, and the first carved on a
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

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
"""

from plan_memo_selftest_mutants import RATCHETS

# This module's rows, in its OWN list: `plan_memo_selftest_mutants.mutants()` gathers every
# registry module's list in one explicit step, and none appends to another's.
MUTANTS = []

SCOPE_PARTNER = ("PROPERTY: the loop ratchet credits a loop by POSITION: one row truncating one of two "
                 "loops that share a header pins that loop and no other")
SCOPE = "PROPERTY: every loop in the census module that walks a POPULATION is pinned by a mutant that truncates THAT loop (the ratchet is derived from the code, not a list somebody extends when a reviewer names one)"
KIND = "PROPERTY: the kind-phrase questions are asked at their sanctioned sites only -- \"which kind does this field DECLARE\" and \"does this cell CLAIM one, under either reading\" are two questions with one site each, and a third caller asking either one directly is the shape three rounds of findings had"
CRITERIA_CONTROL = ("PROPERTY: every criterion the two ratchets state names an existing killing row "
                    "(the table is data in plan_memo_selftest_mutants_ratchets.CRITERIA)")
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
     '    return files(here)',
     '    return ["plan_memo_population.py"]',
     [KIND, KIND_PARTNER]),
    ("ratchets: the kind-question population is DERIVED from the module set (re-inject 799349db's "
     "hand-written five-tuple -- a caller in the sibling resolver or the entry point is outside "
     "it)", RATCHETS,
     '    return files(here)',
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

# -- the fifth attestation: every criterion the two docstrings state, each with
# a row.  The criterion -> row table is `CRITERIA` at the end of this module,
# and `plan_memo_selftest_ratchets.criteria_rows_control` checks that each row
# it names exists.
_CHECKER_14 = ('    return ["plan_memo_blocks.py", "plan_memo_emphasis.py", "plan_memo_html.py",\n'
               '            "plan_memo_ids.py", "plan_memo_lexer.py", "plan_memo_links.py",\n'
               '            "plan_memo_memo.py", "plan_memo_population.py", "plan_memo_roles.py",\n'
               '            "plan_memo_sibling.py", "plan_memo_stream.py", "plan_memo_tables.py",\n'
               '            "plan_memo_tokens.py", "plan-memo-umbrella-check.py"]')

MUTANTS += [
    ("ratchets: a truncation is recognised on the TREE (make `_is_truncation` answer True -- every "
     "loop a row's patched text still has reads as truncated)", RATCHETS,
     '    CONTAINS the original\'s spelling is not one."""\n',
     '    CONTAINS the original\'s spelling is not one."""\n    return True\n',
     [SCOPE_PARTNER]),
    ("ratchets: the truncated iterable must BE the original (drop the comparison -- a re-point to "
     "another iterable's `[:1]` pins a loop whose population it never touches)", RATCHETS,
     '    return ast.dump(v) == ast.dump(original)',
     '    return True',
     [SCOPE_PARTNER]),
    ("ratchets: `list(...)[:1]` is a truncation (drop the unwrap -- every row in that form, most "
     "of the census rows, credits nothing)", RATCHETS,
     '        v = v.args[0]\n',
     '        pass\n',
     [SCOPE_PARTNER]),
    ("ratchets: a truncating row that credits NO loop is an orphan (drop the direction -- a row "
     "whose edit truncates nothing the module has is a silent no-op)", RATCHETS,
     '        if not hits and _intends_truncation(replace):',
     '        if False:',
     [SCOPE_PARTNER]),
    ("ratchets: an unpinned loop is red (drop the report -- the buckets still add up and the "
     "verdict is clean)", RATCHETS,
     '    bad += ["%d `%s` (%s #%d) is neither pinned',
     '    _ = ["%d `%s` (%s #%d) is neither pinned',
     [SCOPE_PARTNER]),
    ("ratchets: the population is EVERY `for`, nested ones too (stop descending into a loop's "
     "body -- an inner loop leaves the population)", RATCHETS,
     '                out[(".".join(qual) or "<module>", n)] = ch\n',
     '                out[(".".join(qual) or "<module>", n)] = ch\n                continue\n',
     [SCOPE_PARTNER]),
    ("ratchets: a loop's position is WITHIN its def or class (stop separating bodies -- a method's "
     "loop takes an ordinal in the enclosing body and every later ordinal shifts)", RATCHETS,
     '                body(ch, qual + (ch.name,))\n                continue\n',
     '                pass\n',
     [SCOPE_PARTNER]),
    ("ratchets: the loop ratchet reads the WHOLE registry (read the base module's list alone -- "
     "every loop a later module's row pins reads as unpinned)", RATCHETS,
     'for name, file, find, replace, _controls in mm.mutants() if file == CENSUS]',
     'for name, file, find, replace, _controls in mm.MUTANTS if file == CENSUS]',
     [SCOPE]),
    ("ratchets: the kind-question population is read off the DISK (re-inject a hand list complete "
     "today -- a new checker module is outside it)", RATCHETS,
     '    return files(here)',
     _CHECKER_14,
     [KIND_PARTNER]),
    ("ratchets: all THREE question names are read (drop `_phrases` -- a caller asking which kind a "
     "field declares is not seen)", RATCHETS,
     '            if name not in sites:\n                continue',
     '            if name not in sites or name == "_phrases":\n                continue',
     [KIND_PARTNER]),
    ("ratchets: a call by bare NAME is a call (read attribute calls only -- `kind_disagreements(...)` "
     "leaves the population and its sanctions read as stale)", RATCHETS,
     '                name = f.attr if isinstance(f, ast.Attribute) else getattr(f, "id", None)',
     '                name = f.attr if isinstance(f, ast.Attribute) else None',
     [KIND]),
    ("ratchets: a caller's identity carries its CLASS (keep the innermost def alone -- "
     "`Population._kind` becomes `_kind`, which is what a same-named function elsewhere is)",
     RATCHETS,
     '                walk(ch, qual + (ch.name,))',
     '                walk(ch, (ch.name,))',
     [KIND]),
]

# -- the sixth attestation: the criteria `_is_truncation`, `_credit` and `_loops`
# state that had no row, and the kind population's self-test half.
MUTANTS += [
    ("ratchets: a truncation keeps ONE element (accept any constant upper bound -- `[:2]` pins "
     "a loop it does not truncate to its first element)", RATCHETS,
     "isinstance(sl.upper, ast.Constant) and sl.upper.value == 1)",
     "isinstance(sl.upper, ast.Constant))",
     [SCOPE_PARTNER]),
    ("ratchets: a truncation has NO lower bound (drop the check -- `[1:1]` is credited)", RATCHETS,
     "    if sl.lower is not None or sl.step is not None or not (",
     "    if sl.step is not None or not (",
     [SCOPE_PARTNER]),
    ("ratchets: a truncation has NO step (drop the check -- `[:1:2]` is credited)", RATCHETS,
     "    if sl.lower is not None or sl.step is not None or not (",
     "    if sl.lower is not None or not (",
     [SCOPE_PARTNER]),
    ("ratchets: only `list(...)` is unwrapped (unwrap any call -- `sorted(xs)[:1]` is credited)",
     RATCHETS,
     ' and v.func.id == "list"',
     '',
     [SCOPE_PARTNER]),
    ("ratchets: a row's find must be UNIQUE to apply (accept a repeated find -- one row pins every "
     "occurrence)", RATCHETS,
     "        if text.count(find) == 1:",
     "        if text.count(find) >= 1:",
     [SCOPE_PARTNER]),
    ("ratchets: an `async for` is a loop (read `ast.For` alone)", RATCHETS,
     "            if isinstance(ch, (ast.For, ast.AsyncFor)):",
     "            if isinstance(ch, ast.For):",
     [SCOPE_PARTNER]),
    ("ratchets: the kind population includes the SELF-TEST half (re-inject the exclusion whose "
     "stated reason measured false -- a self-test-named helper calling `_claims` is unread)",
     RATCHETS,
     "    return files(here)",
     '    return [f for f in files(here) if "selftest" not in f]',
     [KIND_PARTNER]),
    ("ratchets: every criterion of `CRITERIA` names a row that exists (rename one -- the table "
     "then points at nothing)", RATCHETS,
     "    return [c for c, row in table if not any(n.startswith(row) for n in names)]",
     "    return []",
     [CRITERIA_CONTROL]),
]

# -- the sixth-pass attestation: the kind key's projections (IMP-1 / N-2), the
# truncation and intent criteria the second fixture pins, and the bucket order.
MUTANTS += [
    ("ratchets: each question's sanction is ITS OWN (union the sites over every question -- a site "
     "sanctioned for `_claims` may ask `_phrases`)", RATCHETS,
     "            if (mod, caller) in sites[name]:",
     "            if (mod, caller) in set().union(*sites.values()):",
     [KIND_PARTNER]),
    ("ratchets: a sanction names the CALLER, not only the module (key it on the module alone)",
     RATCHETS,
     "            if (mod, caller) in sites[name]:",
     "            if mod in {m for m, _c in sites[name]}:",
     [KIND_PARTNER]),
    ("ratchets: a stale sanction is judged PER QUESTION (drop the question from the seen key -- a "
     "site seen for `_claims` hides its stale `kind_disagreements` sanction)", RATCHETS,
     "            if (name, mod, caller) not in seen:",
     "            if (mod, caller) not in {(m, c) for _q, m, c in seen}:",
     [KIND_PARTNER]),
    ("ratchets: a stale sanction is judged PER MODULE (drop the module from the seen key -- a "
     "caller of the same name elsewhere hides it)", RATCHETS,
     "            if (name, mod, caller) not in seen:",
     "            if (name, caller) not in {(q, c) for q, _m, c in seen}:",
     [KIND_PARTNER]),
    ("ratchets: a MODULE-LEVEL call is a call, named `<module>` (drop it)", RATCHETS,
     '                out.append((name, ".".join(qual) or "<module>"))',
     '                if qual:\n                    out.append((name, ".".join(qual)))',
     [KIND_PARTNER]),
    ("ratchets: only a `list` NAME is unwrapped (accept any callee -- `m.list(zs)[:1]` raises on "
     "`.id`)", RATCHETS,
     'isinstance(v, ast.Call) and isinstance(v.func, ast.Name) and v.func.id == "list"',
     'isinstance(v, ast.Call) and v.func.id == "list"',
     [SCOPE_PARTNER]),
    ("ratchets: only a ONE-argument `list(...)` is unwrapped (drop the count -- `list(zs, 0)[:1]` "
     "is credited)", RATCHETS,
     "            and len(v.args) == 1 and not v.keywords):",
     "            and not v.keywords):",
     [SCOPE_PARTNER]),
    ("ratchets: only a keyword-free `list(...)` is unwrapped (drop the check -- `list(zs, key=0)[:1]` "
     "is credited)", RATCHETS,
     "            and len(v.args) == 1 and not v.keywords):",
     "            and len(v.args) == 1):",
     [SCOPE_PARTNER]),
    ("ratchets: the upper bound must be a CONSTANT (drop the type check -- `zs[:n]` raises on "
     "`.value`)", RATCHETS,
     "isinstance(sl.upper, ast.Constant) and sl.upper.value == 1)",
     "sl.upper.value == 1)",
     [SCOPE_PARTNER]),
    ("ratchets: a truncating INTENT needs a `for` header (any `[:1]` line intends -- a non-loop "
     "line becomes an orphan)", RATCHETS,
     'return any(ln.strip().startswith("for ") and " in " in ln and "[:1]" in ln',
     'return any("[:1]" in ln',
     [SCOPE_PARTNER]),
    ("ratchets: a truncating INTENT needs ` in ` (drop it -- `for e[:1]:` becomes an orphan)",
     RATCHETS,
     'return any(ln.strip().startswith("for ") and " in " in ln and "[:1]" in ln',
     'return any(ln.strip().startswith("for ") and "[:1]" in ln',
     [SCOPE_PARTNER]),
    ("ratchets: EXEMPT is decided first (count an exempt-and-credited loop as pinned too)", RATCHETS,
     "    n_pinned = sum(1 for k in loops if k not in exempt and k in credit)",
     "    n_pinned = sum(1 for k in loops if k in credit)",
     [SCOPE_PARTNER]),
]

# THE CRITERIA TABLE: each criterion the two ratchets' docstrings state, and the
# PREFIX of the row that kills it.  DATA, checked by
# `plan_memo_selftest_ratchets.criteria_rows_control` (every prefix names an
# existing row); a criterion with no entry here is the finding.
CRITERIA = (
    ("loop: the population is every `for`, nested ones too",
     "ratchets: the population is EVERY `for`"),
    ("loop: an `async for` is a loop", "ratchets: an `async for` is a loop"),
    ("loop: a position is within its def or class", "ratchets: a loop's position is WITHIN"),
    ("loop: credit by position, not by the header line",
     "ratchets: a loop is credited by POSITION (re-inject the exact-header-LINE"),
    ("loop: credit by position, not by the target prefix",
     "ratchets: a loop is credited by POSITION (re-inject the target-PREFIX"),
    ("loop: credit by the row's edit, not its find", "ratchets: a loop is credited by the row's EDIT"),
    ("loop: a row's find must be unique", "ratchets: a row's find must be UNIQUE"),
    ("truncation: recognised on the tree", "ratchets: a truncation is recognised on the TREE"),
    ("truncation: the iterable is the original", "ratchets: the truncated iterable must BE"),
    ("truncation: keeps one element", "ratchets: a truncation keeps ONE element"),
    ("truncation: no lower bound", "ratchets: a truncation has NO lower bound"),
    ("truncation: no step", "ratchets: a truncation has NO step"),
    ("truncation: `list(...)[:1]` is one", "ratchets: `list(...)[:1]` is a truncation"),
    ("truncation: only `list` is unwrapped", "ratchets: only `list(...)` is unwrapped"),
    ("loop: an orphan truncating row is red", "ratchets: a truncating row that credits NO loop"),
    ("loop: an unpinned loop is red", "ratchets: an unpinned loop is red"),
    ("loop: the exempt complement is guarded by its header",
     "ratchets: the exempt complement is guarded"),
    ("loop: an exempt entry naming no loop is red", "ratchets: an exempt entry naming no loop"),
    ("loop: the WHOLE registry is read", "ratchets: the loop ratchet reads the WHOLE registry"),
    ("kind: the population is the whole set, not the census module",
     "ratchets: the kind-question population is the WHOLE"),
    ("kind: the population is derived, not a 5-tuple",
     "ratchets: the kind-question population is DERIVED"),
    ("kind: the population is read off the disk", "ratchets: the kind-question population is read off"),
    ("kind: the population includes the self-test half",
     "ratchets: the kind population includes the SELF-TEST"),
    ("kind: all three question names", "ratchets: all THREE question names"),
    ("kind: a bare-name call is a call", "ratchets: a call by bare NAME is a call"),
    ("kind: sanction by (module, qualified function)", "ratchets: a sanction is keyed on"),
    ("kind: a caller's identity carries its class", "ratchets: a caller's identity carries its CLASS"),
    ("kind: a sanctioned site that makes no call is red",
     "ratchets: a sanctioned site that makes no call"),
    ("criteria: every criterion names an existing row", "ratchets: every criterion of `CRITERIA`"),
    ("kind: each question's sanction is its own", "ratchets: each question's sanction is ITS OWN"),
    ("kind: a sanction names the caller", "ratchets: a sanction names the CALLER"),
    ("kind: a stale sanction is judged per question", "ratchets: a stale sanction is judged PER QUESTION"),
    ("kind: a stale sanction is judged per module", "ratchets: a stale sanction is judged PER MODULE"),
    ("kind: a module-level call is a call", "ratchets: a MODULE-LEVEL call is a call"),
    ("truncation: only a `list` name is unwrapped", "ratchets: only a `list` NAME is unwrapped"),
    ("truncation: only a one-argument `list` is unwrapped", "ratchets: only a ONE-argument"),
    ("truncation: only a keyword-free `list` is unwrapped", "ratchets: only a keyword-free"),
    ("truncation: the upper bound is a constant", "ratchets: the upper bound must be a CONSTANT"),
    ("loop: intent needs a `for` header", "ratchets: a truncating INTENT needs a `for` header"),
    ("loop: intent needs ` in `", "ratchets: a truncating INTENT needs ` in `"),
    ("loop: exempt is decided first", "ratchets: EXEMPT is decided first"),
)

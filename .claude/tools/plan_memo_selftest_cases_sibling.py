#!/usr/bin/env python3
"""The SIBLING-RESOLVER controls for `plan-memo-umbrella-check.py --self-test`.

The fourth `Case` module, and the first carved on a SUBJECT rather than on a
review round: every control here is about `plan_memo_memo.Memo.sibling_path`
-- the ONE destination -> file-on-disk mapping, and the only place this checker
decides that a link names a memo it must go and scan.  Its stages are that
function's docstring, in spec order, and each has its controls here: (a) the
WHATWG URL scheme test on the RAW component, (b) percent-decoding, (c) the one
platform-independent reading of the decoded name (anchors, DOS devices,
trailing dots and spaces, the characters Windows does not read as letters of a
name), (d) the `.md` suffix, (e) the join beside the memo.

WHY A SUBJECT AND NOT A ROUND.  The other three modules are indexed by review
round (`plan_memo_selftest_cases.py` pre-converge, `_cases_pr510.py` R1-R16,
`_cases_inline.py` R17 on), and the reason is that a round is a coherent slice
of the converge.  This family is not: the resolver was reported at R3, R4, R5,
R8, R19, R22 and R25, each round adding a stage or a clause to the SAME
function, and its controls were scattered across two of those modules with the
`n%3Achild.md` case in one and the `notes%3Achild.md` case it discriminates
against in the other.  A reader deciding whether a destination is a sibling had
to find both.  Collecting them is what makes the stage list checkable at one
sitting -- and it is what shows, for instance, that stage (a) and stage (c) now
answer the same way on every scheme-ful destination (PR #510 R25-1).

All four modules append to the SAME `CASES` list through the same `case` /
`acase` / `rcase` spellings -- one registry, one import site (the controls
module imports each for its side effect).  A control's mutant lives in
`plan_memo_selftest_mutants*.py` under its round label, keyed by the control's
NAME, so a control moving between modules moves nothing else.
"""

from plan_memo_selftest_cases import VIOLATION, build, case, rcase


# R3-2: a root-relative destination is a site URL, never a sibling on disk
rcase("NEGATIVE", "(rc) a root-relative `/guide.md` is not a sibling on disk (nothing probed): rc 0",
      build(), "See [site docs](/guide.md).", 0)


# R4-2: a percent-encoded destination names the decoded file
case("POSITIVE-NOVEL", "(link) a percent-encoded destination `slice%20sib.md` links the file "
                       "`slice sib.md`, as `<slice sib.md>` does",
     build(), "See [the walk](slice%20sib.md).", 1, files={"slice sib.md": VIOLATION + "\n"})


# R5-2: the decoded path is re-validated
rcase("NEGATIVE", "(rc) a percent-encoded ABSOLUTE destination `%2Ftmp%2Fchild.md` is rejected after "
                  "decoding (never probes `/tmp/child.md`): rc 0",
      build(), "See [x](%2Ftmp%2Fchild.md).", 0)


# R8 root: one sibling-path resolver, stages in spec order
case("POSITIVE-NOVEL", "(link) `notes%3Achild.md` has no scheme (WHATWG URL §4.4 #scheme-start-state / "
                       "#scheme-state read the input as written and `%` is in neither class; "
                       "#string-percent-decode is a later, separate operation): it is the local file "
                       "`notes:child.md`, and it is scanned",
     build(), "See [the walk](notes%3Achild.md).", 1, files={"notes:child.md": VIOLATION + "\n"})


# #3 (IMP): `sibling_path` stage (c) is ONE platform-independent rule -- the
# decoded name is read under Windows path syntax (`PureWindowsPath`, the
# superset: `/` and `\` both separate, a drive / UNC / root prefix anchors)
# on every platform, and an anchored name is rejected; stage (e) joins the
# name's parts, so `\` is a separator everywhere, never a POSIX name
# character.  WHATWG URL `#path-state` step 1 reads a special-scheme path
# (`file` is special) the same way, and calls its drive-letter quirk
# "platform-independent".  Until R19 (c) rejected a leading `/` only.
rcase("NEGATIVE", "(rc) a percent-encoded Windows drive-absolute `C%3A%5Ctemp%5Cchild.md` (`C:\\temp\\child.md`) is "
                  "rejected after decoding on every platform (stage c: a drive anchors): rc 0",
      build(), "See [x](C%3A%5Ctemp%5Cchild.md).", 0)
rcase("NEGATIVE", "(rc) a percent-encoded backslash-rooted `%5Cchild.md` (`\\child.md`) is rejected (stage c: a root "
                  "anchors): rc 0",
      build(), "See [x](%5Cchild.md).", 0)
rcase("NEGATIVE", "(rc) a percent-encoded UNC `%5C%5Cserver%5Cshare%5Cx.md` is rejected (stage c: a UNC prefix anchors): "
                  "rc 0",
      build(), "See [x](%5C%5Cserver%5Cshare%5Cx.md).", 0)
rcase("NEGATIVE", "(rc) a raw `\\\\server\\share\\x.md` destination decodes (§2.4: `\\\\` is one backslash, `\\s` is "
                  "literal) to the backslash-rooted `\\server\\share\\x.md` (commonmark.js: href "
                  "`%5Cserver%5Cshare%5Cx.md`) and is rejected: rc 0",
      build(), "See [x](\\\\server\\share\\x.md).", 0)
rcase("NEGATIVE", "(rc) drive-relative `C:child.md`: raw, it is a URL of scheme `c` (stage a); percent-encoded "
                  "`C%3Achild.md` decodes to a drive-anchored name (stage c) -- both rejected, rc 0",
      build(), "See [a](C:child.md) and [b](C%3Achild.md).", 0)
rcase("NEGATIVE", "(rc) `n%3Achild.md`: a ONE-letter name before `:` is a Windows drive letter (URL `#path-state` step "
                  "1.4.1, platform-independent) -- drive-relative, rejected, rc 0; the multi-letter `notes%3Achild.md` "
                  "of R8 stays a file name",
      build(), "See [x](n%3Achild.md).", 0)
case("POSITIVE-NOVEL", "(link) `sub%5Cchild.md`: a backslash is a path separator on every platform (WHATWG URL "
                       "`#path-state` step 1: for a special scheme -- `file` is one -- `\\` ends a segment as `/` does; "
                       "`PureWindowsPath` is that syntax) -- the file `sub/child.md` is walked",
     build(), "See [x](sub%5Cchild.md).", 1, files={"sub/child.md": VIOLATION + "\n"})


# #2 (P2) sibling: a relative destination whose COMPONENT is a Windows DOS
# device (`NUL.md`) or ends in a dot or a space has an empty
# `PureWindowsPath.anchor` and passed stage (c) until R22.  On Windows reading
# it SUCCEEDS and yields an empty stream, so the population would count a
# linked memo it never scanned and exit 0 -- §1's could-not-scan class.  The
# predicate is pure string logic (`_is_reserved_component`), so every control
# here decides the same way on POSIX; each names a file that DOES exist in the
# fixture directory, so a green control means "not read", never "not found".
DEVICE = {"NUL.md": VIOLATION + "\n", "NUL/child.md": VIOLATION + "\n", "com1.md": VIOLATION + "\n",
          "dir /child.md": VIOLATION + "\n", "prn .md": VIOLATION + "\n",
          "NULX.md": VIOLATION + "\n"}

case("NEGATIVE", "(sibling) `NUL.md` is a DOS device, not a memo beside this one: the destination is no "
                 "sibling even though a file of that name is there to read",
     build(), "See [x](NUL.md).", 0, files=DEVICE)
case("NEGATIVE", "(sibling) `NUL/child.md`: the device is read per PART of the parsed path -- a device "
                 "DIRECTORY rejects the destination too.  The device sits in a NON-FINAL component on "
                 "purpose: with it in the last one, a final-component-only reading passes this control "
                 "and the mutant survives (measured -- the probe would have had another subject)",
     build(), "See [x](NUL/child.md).", 0, files=DEVICE)
case("NEGATIVE", "(sibling) `com1.md`: the device names fold case (`ntpath._isreservedname` upper-cases "
                 "the stem), so the lower-case spelling is the same device",
     build(), "See [x](com1.md).", 0, files=DEVICE)
case("NEGATIVE", "(sibling) `dir%20/child.md`: a component ending in a SPACE is reserved too -- Windows "
                 "strips the trailing run, so that component names a different directory there than here",
     build(), "See [x](dir%20/child.md).", 0, files=DEVICE)
case("NEGATIVE", "(sibling) `prn%20.md`: the stem's trailing spaces are stripped before the device "
                 "lookup, so `prn .md` is `PRN` (Microsoft, \"Naming Files, Paths, and Namespaces\")",
     build(), "See [x](prn%20.md).", 0, files=DEVICE)
case("POSITIVE", "(sibling) `NULX.md` is an ordinary sibling and IS walked -- the discriminating half: a "
                 "device is the STEM of a component, never a prefix of one, and a guard that rejected "
                 "every name holding `NUL` would drop a memo the author linked",
     build(), "See [x](NULX.md).", 1, files=DEVICE)

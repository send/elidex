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

from plan_memo_selftest_cases import CASES, VIOLATION, build, case, rcase


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


# R8 root: one sibling-path resolver, stages in spec order.
#
# ⚠⚠ REVERSED AT PR #510 R25-1, AND THE REVERSAL IS THE CONTROL'S OWN TEXT.
# This case read "`notes%3Achild.md` ... is the local file `notes:child.md`, and
# it is scanned" and expected 1 site.  It now expects 0, and that is a DECISION,
# not the repair of a red control: R25-1 found that on Windows `notes:child.md`
# names an NTFS ALTERNATE DATA STREAM (the stream `child.md` of the file
# `notes`), which where it exists OPENS successfully and yields another file's
# content -- so the destination is read differently on different platforms,
# which is what stage (c)'s policy ("ONE platform-independent reading of the
# name") refuses.  Under that policy R8's reading was the odd one out: since
# R19 `sub\child.md` is not the POSIX file of that name, `C:x.md` and
# `n%3Ax.md` are drive-relative, and since R22 a DOS device is no file beside
# the memo.
# WHAT IT COSTS, named: a POSIX memo genuinely called `notes:child.md` is no
# longer a sibling -- dropped without a report, the standing polarity of this
# stage.
# WHAT SURVIVES OF R8: stage (a) still reads the RAW component, so
# `notes%3Achild.md` still has NO scheme; it is stage (c) that refuses it, for
# the path reason and not the URL one.  That half is no longer OBSERVABLE,
# though -- every scheme ends in a `:`, decoding never removes one, and a `:`
# is now refused at (c) -- which is why the two mutants that witnessed it are
# deleted as EQUIVALENT rather than left to survive
# (`plan_memo_selftest_mutants.py` / `_mutants_pr510.py`, each with the
# reasoning where the row stood).
case("NEGATIVE", "(link) `notes%3Achild.md` is NOT a sibling.  It has no scheme -- WHATWG URL §4.4 "
                 "#scheme-start-state / #scheme-state read the input as written and `%` is in neither "
                 "class, #string-percent-decode being a later, separate operation -- so stage (a) admits "
                 "it; stage (c) then refuses it, because the decoded `notes:child.md` is an NTFS "
                 "alternate data stream on Windows and a plain file name on POSIX, which is two readings "
                 "where that stage allows one.  ⚠ THIS EXPECTATION IS A REVERSAL of PR #510 R8, decided "
                 "at R25-1, and it costs a POSIX memo genuinely named `notes:child.md` -- dropped "
                 "without a report, as `sub\\child.md` and `NUL.md` already are",
     build(), "See [the walk](notes%3Achild.md).", 0, files={"notes:child.md": VIOLATION + "\n"})


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
                  "1.4.1, platform-independent) -- drive-relative, rejected at stage (c) by the ANCHOR, rc 0.  "
                  "⚠ Since R25-1 the multi-letter `notes%3Achild.md` is refused at the same stage by the CHARACTER "
                  "rule, so this no longer discriminates one-letter from multi-letter; what it still says is that "
                  "the drive reading is applied on every platform",
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


# ---------------------------------------------- PR #510 Codex R25-1 controls --

# The CHARACTER half of `_is_reserved_component`.  ONE case per member of
# `_RESERVED_CHARS`, because the previous reading kept the whole set out of the
# predicate on ONE argument -- "on Windows a name holding one raises `OSError`
# there, reported as an unavailable linked memo" -- and that argument is FALSE
# for `:` (an alternate data stream opens) and unverifiable from this tree for
# the other six, so it is not the discriminator anything should rest on.  The
# discriminator is stage (c)'s policy: the name must denote the same thing on
# every platform.  Enumerating only the member that was reported would leave
# the other six authoritative by default, which is the trap this PR hit at R22
# (the kind phrases), R23 (the kind gate) and R24 (§2's preprocessing).  Each
# destination is NOVEL -- no such spelling appears in the umbrella memo -- and
# each names a file that DOES exist in the fixture directory, so a green
# control means "not read", never "not found".
# ⚠ THE STEM IS TWO CHARACTERS, and that is load-bearing: a ONE-letter name
# before a `:` is a Windows DRIVE letter, so `a:notes.md` is refused by stage
# (c)'s ANCHOR clause and a probe built on it measures that clause instead of
# this one -- the R25-1 mutant SURVIVED against `a%3Anotes.md` and said so.
# The other six take the same stem so the seven differ in one character only.
RESERVED = {"ax:notes.md": VIOLATION + "\n", "ax*notes.md": VIOLATION + "\n",
            "ax?notes.md": VIOLATION + "\n", 'ax"notes.md': VIOLATION + "\n",
            "ax<notes.md": VIOLATION + "\n", "ax>notes.md": VIOLATION + "\n",
            "ax|notes.md": VIOLATION + "\n", "ax+notes.md": VIOLATION + "\n"}

# The seven control NAMES, collected as they are minted: they are composed
# here, so the mutant that must turn them red reads them from here rather than
# transcribing seven long strings a `%` away from these (`unknown control` is a
# FAIL, so a transcription would fail loudly -- but it would still be the
# grammar of a control name spelled twice).
R25_RESERVED_NAMES = []

for _ch, _enc, _why in [
        (":", "%3A", "opens an NTFS ALTERNATE DATA STREAM there -- `notes:child.md` is the stream "
                     "`child.md` of the file `notes` -- so where the stream exists reading it SUCCEEDS "
                     "and yields another file's content, the very class the DOS devices were rejected "
                     "for.  This is the member R25 reported, and the one that falsified the `OSError` "
                     "argument the other six rested on"),
        ("*", "%2A", "holds a character Microsoft's \"Naming Files, Paths, and Namespaces\" lists "
                     "among those a file name may not use"),
        ("?", "%3F", "holds a character that same list excludes"),
        ('"', "%22", "holds a character that same list excludes"),
        ("<", "%3C", "holds a character that same list excludes"),
        (">", "%3E", "holds a character that same list excludes"),
        ("|", "%7C", "holds a character that same list excludes")]:
    case("NEGATIVE", "(R25 sibling) `ax%snotes.md` is no sibling: the decoded `ax%snotes.md` %s.  ⚠ Whether "
                     "that platform RAISES is NOT the test and was not determined here (no Windows is "
                     "reachable from this tree); the test is stage (c)'s policy -- the name must denote "
                     "a file beside this memo on every platform, and this one does not"
                     % (_enc, _ch, _why),
         build(), "See [x](ax%snotes.md)." % _enc, 0, files=RESERVED)
    R25_RESERVED_NAMES.append(CASES[-1].name)
case("POSITIVE-NOVEL", "(R25 sibling) `ax%2Bnotes.md` (`ax+notes.md`) IS a sibling and IS walked -- the "
                       "discriminating half of the seven above: `+` is an ordinary name character on "
                       "every platform and outside `_RESERVED_CHARS`, so a fix that refused ASCII "
                       "punctuation wholesale would pass all seven and fail this one",
     build(), "See [x](ax%2Bnotes.md).", 1, files=RESERVED)
R25_PER_PART = ("(R25 sibling) the character rule is read per PART, as the device rule is: "
                "`ax%3Ab%2Fchild.md` (`ax:b/child.md`) puts the colon in a NON-FINAL component -- with "
                "a final-component-only reading it passes while `ax:b.md` is refused.  ⚠ The colon must "
                "sit in the DIRECTORY: `sub/ax:b.md` puts it in the last part, where that reading "
                "refuses it too and R22's per-part mutant SURVIVES (measured)")
case("NEGATIVE", R25_PER_PART,
     build(), "See [x](ax%3Ab%2Fchild.md).", 0, files={"ax:b/child.md": VIOLATION + "\n"})

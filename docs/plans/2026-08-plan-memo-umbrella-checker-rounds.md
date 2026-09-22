# PR #510 — the round-by-round ledger

The chronological record for
[`2026-08-plan-memo-umbrella-checker.md`](2026-08-plan-memo-umbrella-checker.md): every Codex review
round, every design re-gate, and the handoff each session resumes from. Carved out of that memo's §6
at PR #510 R47-3, which reported it — §6 had reached 1,790 lines by holding a stable ACCEPTANCE
CONTRACT and an append-only HISTORY in one bullet, and the two have opposite lifetimes.

▶▶▶▶ **THE NEXT SESSION STARTS HERE (2026-09-21, head `799349db` PUSHED, tree clean, NO round in
flight, and the converge loop is DELIBERATELY STOPPED).**

⚠ **THE STATE IN THIS BLOCK'S HEADER, ITS GATE LINE AND ITS MERGE PARAGRAPH IS STALE (2026-09-22) —
read this first; the block is kept as written.** Since it was written: `4e46d95f` and `00dfd095` were
pushed, and `00dfd095` is the pushed head; Codex answered `00dfd095` with a 👍 (dry,
2026-09-22T01:36Z); a fresh-agent attestation over `9b2e1fa9..00dfd095` then found **6 IMP + 5 MIN
behind that dry verdict** — the fourth time a dry round hid real defects — fixed in the LOCAL,
UNPUSHED commit that adds this note (`79309486`); a fifth attestation over that commit found 5 IMP +
3 MIN more (four of its eleven fixes had not closed their class), fixed in the local, unpushed commit
after it; and the carves' plan-review produced its sub-umbrella memo on
the stacked local branch `vm-p4-gate-population-plan` (worktree `elidex-wt-gatepop`, not pushed).
So "head `799349db` PUSHED" and "a fresh review of `799349db`" below no longer name the head: a merge
needs a fresh review of whatever head is pushed when merge is raised. Gate at the fix commit: see its
message (the ratchets now read 22 census loops = 16 pinned + 6 exempt, and 7 kind-question call
sites over the checker half of the one module population, which is derived from the disk).

**THE DECISION (user, 2026-09-21): option (B) — stop the review loop, take §8's carves to
`/elidex-plan-review`.** Do NOT `/external-converge 510`. Do NOT re-trigger Codex. The loop was not
failing; it was producing findings of a kind a loop cannot settle.

**WHY, in one line each**: R47 3 findings → R48 2 → R49 2 → R50 1 → R51 dry → R52 1. Every round's
findings were real and were fixed or carved. But the RESIDUE converged: **five §8 entries name
"Slice 2's plan-review" as owner, and four of them are the same question — what is this gate's
POPULATION.** Each of those four was reproduced, had a fix written and measured, and the fix was
REVERTED because it overturns a control this plan ratified. That is plan-review work by CLAUDE.md's
edge-dense rule, not another round.
⚠ **"REVERTED because it overturns a control this plan ratified" is true of TWO of the four, (14) and
(15)** (2026-09-22; the fourth attestation, and ⚠ the note first written here said "ONE" — the
fifth attestation corrected it, since (15)'s reason (ii) still stands in its own entry). (14)'s count
fix was reverted because it turns the R23 NEGATIVE control red, and that control is right. (15)'s arm
was reverted for two independent reasons, and (ii) — it requires overturning ratified controls —
stands; `00dfd095` corrected only (ii)'s particulars (one of the controls measures `rc`; "FOUR" is a
floor). (5)'s fix was reverted because it **broke a shipped invariant** (`id_scan_grammar_agreement_control`;
the defect is in the grammar). (12) had no fix reverted — it was never fixed here — and `00dfd095`
corrected its "overturns R42's twin" reason in §8: a keep-free kind reading fixes it without the
overturn. The conclusion does not move: all four are plan-review work — (5), (12) and (14) by the
edge-dense rule their own text invokes, (15) by its owner line (Slice 2's plan-review, decided with
(14)) — and neither half of that rests on the overturn.

**⚠ AND A SECOND REASON, which is the one worth carrying forward**: across this session a Codex DRY
verdict hid real defects **three times**. The three-agent enumeration attestation found 6 behind the
first pair of dry rounds, 3 behind R47's, and 3 behind R51's. Every one was found by a FRESH AGENT
ENUMERATING, never by my own re-reading. Two of the last three were *recurrences of the class the
commit that introduced them claimed to have closed* — a ratchet keyed by NAME inside the control
whose docstring warns against criterion-by-name, and an unswept retraction inside the commit titled
"corrections that had not been swept". **A dry round bounds what the REVIEWER found, never what is
there.**

**STATE**: PR #510, branch `vm-p4-plan-memo-checker`, worktree `elidex-wt-vmp4checker`.
**Gate at `799349db`**: 739 controls / 421 mutants 0 survived 0 crashed / trip-wires rc 0 / #506
census `--worklist` site set byte-identical over 815 rows / this memo at the four-FATAL floor /
largest `plan*.py` 983 / 22 census loops 0 unpinned / 7 kind-question call sites over 5 modules 0
unsanctioned. **All Codex threads are resolved except the five that ARE the carves.**

**▶ WHAT THE PLAN-REVIEW TAKES.** §8's entries, and the four that are one question first:
(5) the id grammar releasing a rejected core's trailing decoration; (12) the §6.2 demotion tag
recorded and never read; (14) the residue gate comparing PRESENCE where the property is kind AND
attribution; (15) the declaration site that produces no table. Each entry already carries its
reproduction, the fix that was written, and the measured reason it was reverted — the review should
start from those measurements rather than re-deriving them. (6) Slice 3 (Phase 1 offsets) is the
fifth and is a different subject.
⚠ **(6) IS NOT THE FIFTH** (2026-09-22, the fourth attestation): its owner is its own PR under the
edge-dense rule, not Slice 2's plan-review. The fifth entry that names Slice 2's plan-review as owner
is **(11)**, the unbound table that makes no kind claim; (6) is a different subject AND a different
owner.

**▶ MERGE IS NOT PROPOSED AND IS THE USER'S CALL.** The head has moved since the last dry round, so
if merge is ever raised it needs a fresh review of `799349db` first; the merge-head guard hook
enforces that mechanically.

⚠ **A session resuming this work starts HERE**, at the block headed *"THE NEXT SESSION STARTS
HERE"* below. The umbrella memo keeps the contract, the invariants, the spec coverage map, the
slices and the defer ledger; nothing in this file is a commitment, only a record of how the
commitments were met and what each round cost.

⚠ **This file is 1,766 lines and is NOT split further, deliberately.** CLAUDE.md's touch-time rule
is a COHESION judgement and not a line count, and it exempts "一枚岩の cohesive unit・巨大 generated
table・flat な case table". A chronological ledger is that shape: its only remaining seam is the
round boundary, and cutting there would scatter one continuous argument — a finding, its fix, the
measurement that refuted the fix, the re-fix — across files by date. What WOULD make it splittable
is a second subject appearing in it (a per-slice ledger for Slice 2, say); that is the trigger to
re-read this paragraph, not the line count.

⚠ **This file is in the checker's own population** — the umbrella links it, so the census walks it
and every gate that asks of the whole population asks of it too. That is deliberate: a ledger that
the checker cannot read is a ledger whose claims nothing checks.

⚠ **PR #510 Codex R12 (2026-09-07)**: **266 controls, 145 mutants / 0 / 0**, census `48 = 33 + 15`, rc 0,
36 ORDER-PROSE? rows unchanged, **717 sites, the same 717** (`--worklist` file/line/id/source set
identical to `29dc1eb7`'s), `[LEX-UNSUPPORTED?]` 5 unchanged (the memo holds no tab). The conformance
control (§3.0) is the falsifier for the LEXED rows from here on: **169 aligned / 27 excluded / 0 FAIL**.
`--self-test --mutants` **49 s → 4.2 s**: the Phase-1 linearity witness counted the lexer's
`link_label` binding (Phase 2's one call per `[l..]`, never `reference_definitions`' calls), so the
quadratic mutant RG-1 was killed by 48 s of wall clock alone; it now counts `plan_memo_blocks`'
binding and requires EXACTLY one call per line (a mis-bound counter sees fewer: red), with a runner
mutant re-binding it. One mutant deleted as EQUIVALENT (the licensing rule's stream read: since the
scanners read the stream, no fixture reaches it with raw ≠ stream at a mention boundary — reason in
`plan_memo_selftest_mutants.py`). `.github/workflows/ci.yml` `trip-wires` `timeout-minutes` 2 → 5
(211 s on Codex's runner had cancelled a green job).
⚠ **PR #510 Codex R13 (2026-09-07) — the block-grammar family closed**: `plan_memo_blocks.py` had
drawn IMP findings three rounds running, each on a §3.0 PROSE-AS-WRITTEN disposition; the root was
that the 27 conformance exclusions were the unmodelled grammar. §4.4 indented code is now a RAW
extent (the third `raw_opener` arm, the §4.4 chunk in `raw_extent`, no seed — like a fence) and §5.1
block quotes are CONTAINERS (`quote_content` + `Memo._quote`: the same `_parse` over the content,
laziness as `block_end`'s `lazy` arm); `unsupported_block`, `_QUOTE` and `container_text`'s §4.4 arm
are deleted, `Memo._phase1` is `Memo._parse(lines, linenos, lazy)`, and Phase 1 states its result
as `Memo.sequence`, which the conformance control now consumes directly (its ownership
re-derivation from dropped lines is gone). Conformance **169 / 27 / 0 → 184 / 12 / 0**; the 12 are
§5.2 list items only. Also R13: a NON-schema row's excess cells are cut at admission (GFM §4.10) and
§4.6 reads case per condition (`<![cdata[` is a paragraph). **285 controls, 163 mutants / 0 / 0**,
`--self-test --mutants` 4.3 s; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717
sites, the same 717** (`--worklist` file/line/id/source set identical to `5b2ecdd3`'s tools on the
same memo — ⚠ the brief expected the four indented lines' sites to leave the 717; they were never
in it: their ids `5` `F` `2` `4` `8` are the two declared-miss shapes), one LICENSED mention gone
(main:2959 `10a`, inside the 2955–2960 chunk), `[LEX-UNSUPPORTED?]` 5 → 4 (above). Two R13 probes
survived their mutants on the first run and were rewritten: `<PRE>` / `<DIV>` at a block start are
type-7 openers too, so the case fold was never their subject — the probes now interrupt a paragraph;
and the quote-bound mutant is a cost mutant, killed by the new linearity witness (≤ 4N
`quote_content` calls), while laziness itself has its own mutant. The harness's exclusion predicate
was narrowed from "any block-start line in a paragraph" to list-marker lines, so a quote read as
prose is a FAIL rather than a silent LEXED-FLAT re-classification.
⚠ **Design re-gate 3 (2026-09-07, 3 IMP + 7 MIN, applied on top of the `Memo` split, §7)**: IMP-1 the
§5.2 item-open bit replaces the one-block memory (`Memo._parse` `item_open` / `close`); IMP-2 ONE seed rule
for raw lines (`Memo.raw` with a reading; an indented schema row after a table is seeded, never a silent
skip); IMP-3 the false "cmark-gfm's row continuation" attribution rewritten to the measured table ends
(§2 I-A); MIN-4 the conformance exclusion is "marker line FIRST in the paragraph"; MIN-5 the block-sequence
control asserts the raw extents' CONTENT (Example 6 exactly); MIN-6 `Block quotes` §5.1 Examples 228–252
vendored, Example 237 cited for the driver's lazy stop; MIN-7 §4.6 examples 152 / 182; MIN-8 the GFM
§4.10 and CommonMark §5.1 quotes verbatim; MIN-9 a lazy HEADER row is the table's header (cmark-gfm,
measured), the delimiter arm kept; MIN-10 `setext_underline` returns the heading kind, the second spelling
deleted. **297 controls, 170 mutants / 0 / 0**, conformance **208 / 13 / 0 over 221**, census `48 = 33 +
15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the same 717** (`--worklist` file/line/id/source
set identical to `8d08b333`'s), `[LEX-UNSUPPORTED?]` **4 → 10** (§3.0), `mise run trip-wires` rc 0.
⚠ **PR #510 Codex R15 (2026-09-08) — the last Phase-1 container family closed**: §5.2 list items are
CONTAINERS exactly like block quotes (`item_marker` / `strip_columns` / `_same_list`, `Memo._list` /
`_item`, the same `_parse` and the same lazy gather; §5.3 lists with tightness stated in the sequence
entry; the §5.2 interruption rule on `starts_block`'s `para_open` bit; the §4.1 thematic-break
precedence in `item_marker`), replacing the LEXED-FLAT reading, the item-open bit, the `item` seed reading
and `container_text` — the reviewer's `- item\n\n    [child](child.md)` (Example 108's shape) was
consumed as indented code, so the link was never found and `child.md` never walked, rc 0. #2: an orphan
definition keeps its DESTINATION and the population miss is raised only where `sibling_path` (the one
resolver) maps it to a sibling on disk — `[x]: #section` / `[x]: https://…` orphans no longer promote a
later `[x]` to the exit-2 miss. #3: `ROW_NOUN` is ASCII case-insensitive in one place (`(?ai:…)`), so
`SLICE C owns it` / `ROW 9` / `UMBRELLA C` name a row. `List items` 253–300 and `Lists` 301–326 vendored
(221 → 295 examples); conformance **208 / 13 / 0 → 295 / 0 / 0** — the 13 §5.2 exclusions are gone and
NOTHING remains excluded (the GFM-table arm is empty by construction; §5.3 tightness IS checked, since the
list entry states it and the aligner refuses a `<p>` under a tight claim — ⚠ its first draft read past one
and three "loose" mutants survived, caught by the mutant run). **324 controls, 184 mutants / 0 / 0**,
`--self-test --mutants` 7.0 s; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites,
the same 717** (`--worklist` file/line/id/source set identical to `956c640b`'s tools on the same memo);
what moved: the ROLE vocabulary of 4 of them (1202 `2a` — → landing, 1757 `2ab` +landing, 2790 `0b` — →
acceptance, 3053 `1a` +owner) and one LICENSED mention back (2959 `10a`, inside the 2955–2960 chunk,
which is item `10.`'s paragraph again), both by the one §5.2 rule: an item paragraph's content no longer
carries the marker nor each continuation line's W + N columns, so the licensing window (80 characters of
the block's stream) reaches further back into the prose at those sites. `[LEX-UNSUPPORTED?]` **10 → 6**
(the four `item` lines are parsed; the six `|` shell lines remain, `indented`). `plan_memo_selftest_mutants.py`
split at the review-round seam first, as a standalone prereq commit (§7). `scripts/trip-wires.sh` rc 0.
⚠ **PR #510 Codex R16 (2026-09-08)**: #2 (IMP) §2.5 character references in a link DESTINATION are decoded by
the ONE destination normalisation (`normalize_destination`, §3 row; every destination site returns through
`link_destination`) — `[child](child&#46;md)` and `[sib]: child&#46;md` were backslash-unescaped only, so the
sibling was silently outside the population, rc 0; labels / titles / code spans are NOT decoded, each with a
control. #3 (IMP) container nesting is off the call stack: the Phase-1 passes are generator frames driven by
`Memo._run`'s explicit stack (~500 nested `>` raised `RecursionError` inside `_parse`, and the population
chokepoint — then `OSError | RuntimeError | UnicodeDecodeError` — reported the memo as "linked memo
unavailable (RecursionError)", rc 2, no census), and the chokepoint is I/O ONLY (`OSError |
UnicodeDecodeError`; `RuntimeError` is caught at `_resolve` alone, the py≤3.12 symlink loop; a parser
exception is a crash out of `check()`, crash = FAIL — control + mutant each). #1 (MIN) §7's Slice 0 / 1 / 2
bullets state the per-PR arrangement (this document IS Slice 0 + 1's per-PR plan; Slice 2 gets its own).
**341 controls, 191 mutants / 0 / 0**, `--self-test --mutants` ~9 s (8.9 / 9.6 s measured; 7.0 s at R15 — the
1,000-deep control runs `Memo` twice and `check()` once, and again under the mutant that names it);
conformance **295 / 0 / 0**; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the
same 717** (`--worklist` identical to `1ad7e523`'s tools on the same memo — the memo holds no character
reference anywhere, `grep -c '&[a-zA-Z#][a-zA-Z0-9]*;'` = 0, so no site could move), `[LEX-UNSUPPORTED?]` 6.
`scripts/trip-wires.sh` rc 0.
⚠ **PR #510 Codex R17 (2026-09-08)**: #1 (IMP) a reference DEFINITION keeps the run open, so a lazy GFM table
header right after it inside a block quote or a list item is that container's table (`Memo._parse`: ONE rule,
`cur or i == def_end`, the lazy line handed to `admit_table`; cmark-gfm MEASURED on fourteen variants, the table
in the docstring) — the reviewer's `> [a]: /u\n| # | Slice | … |\n> |---|…` in a LINKED memo dropped the schema
table, its `#11-…` slot never entered the census and, schema presence being required of the main memo only,
rc 0. The "not modelled" note is gone; the ONE divergence that stays (cmark-gfm prints that definition as a
paragraph and registers nothing; here it registers) is stated in §2, the docstring and the §3.0 table row.
Control + mutant: the runner's "a lazy schema header after a definition in a linked memo's quote is a table…"
(id declared, kind umbrella, census +1) and the "(quote) the R17 reviewer's shape…" family, the block-sequence
shapes; the mutant re-injects `cur` alone. #2 (IMP) CommonMark §6.6 raw HTML is a span of the ONE inline pass
(`_HTML_TAG`: open / closing tag with the attribute grammar — unquoted / single- / double-quoted values —
the 0.31 comment rule, processing instruction, declaration, CDATA; §4.6 condition 7 now reads the same
`OPEN_TAG` / `CLOSING_TAG`, one spelling), masked and seeded exactly as a raw HTML-block line is (the §3 row,
§3.0): `<span title="[child](absent.md)">text</span>` was read as a link — a false unavailable-memo miss, rc 2
— and ids inside attributes leaked into the scans. `Raw HTML` Examples 613–632 vendored; inline conformance
control **20 / 0 / 0** (⚠ its first draft EXCLUDED a non-paragraph read by predicate, and the
optional-whitespace mutant survived through that arm — Example 622 became a type-7 HTML block; the arm is
deleted, such a read FAILS). ⚠ The R17 brief's negative `<span title=[x](y.md)>` is a TAG (an unquoted
value admits brackets and parentheses; commonmark.js agrees) and is a NEGATIVE control saying so. Ten
R17 #2 mutants (each arm dropped or widened, the seed record and the `html` disposition dropped). #3 (MIN)
the resolver's scheme-before-decoding claim cites WHATWG URL §4.4 `#scheme-start-state` / `#scheme-state`
and §1.3 `#string-percent-decode` (webref `dfn url` / `body url`), in the docstring, the §3 row and the R8
control's name. **381 controls, 202 mutants / 0 / 0**, `--self-test --mutants` 10.7 s; conformance **295 / 0
/ 0** + **20 / 0 / 0**; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the same
717** (`--worklist` identical to `dbbeeb8e`'s tools on the same memo at `190d2adb`, save the seed note's
wording; the population holds ONE inline raw HTML span, `<this file>`, holding no id — `[LEX-UNSUPPORTED?]`
6, unchanged). `scripts/trip-wires.sh` rc 0.
⚠ **PR #510 Codex R19 (2026-09-08)**: #1 (IMP) CommonMark §6.4 — a link that closed while an IMAGE opener was
still on the bracket stack joined the population as it closed; if the image then resolved, its description is
plain text (`![alt [docs](absent.md)](image.png)` → `<img alt="alt docs">`, commonmark.js), so the inner link is
no memo link and the rc-2 miss for `absent.md` was false. ONE rule at the point the image closes (`inline_pass`,
offset containment — the entries recorded past the image's `[` are the trailing ones): a link inside a RESOLVED
image's description is DEMOTED to a masked tail (never a memo link, never prose), a nested image stays masked, a
failed reference there names no lost memo; an UNRESOLVED image is literal `![` and keeps its inner link
(`![alt [docs](child.md)][missing]` walks `child.md`). The link-in-link mirror needs no rule (the inner link
deactivates the outer `[` as it closes; `[a [b](x.md)](y.md)` is the R1-3 control). Nine R19 "(image)" /
"(link)" controls, four mutants (the immediate add re-injected; a plain drop for the demotion; the
failed-reference rule dropped; demotion re-injected on the FAILED image). #2 (MIN) diagnostics printed
`path.name`, so `a/child.md:1` and `b/child.md:1` were one `child.md:1` in `--worklist`, the findings and the
population summary: `Population.display(path)` is the ONE helper every printer reads (the worklist's `Block.file`,
the findings in `plan_memo_roles.py` / `plan_memo_memo.py` / the checker, the summary) — the path RELATIVE to
the root memo's directory, the resolved absolute path when the memo lies outside it. Runner control "diagnostics
name a memo relative to the root memo's directory…" (two same-named siblings told apart in the sites' and the
seeds' file columns; a memo one directory up named absolutely); mutant re-injects `.name`. ⚠ The R19 brief
presumed §6 figures might quote a `child.md:N` shape and that controls' expected text might carry a basename:
neither exists (0 controls adjusted). #3 (IMP) `sibling_path` stage (c) rejected a leading `/` only, so the
decoded `C%3A%5Ctemp%5Cchild.md` / `%5Cchild.md` passed and on Windows `parent / name` discarded the memo's
directory: (c) is now ONE platform-independent rule — the decoded name is read under Windows path syntax
(`PureWindowsPath`, the superset; WHATWG URL `#path-state` step 1 reads a special-scheme path the same way and
calls the drive-letter quirk "platform-independent") and must have no anchor (drive, UNC, `\` or `/` root,
drive-relative `C:`); (e) joins the name's PARTS, so **`\` is a separator on every platform, never a POSIX
name character** (`sub%5Cchild.md` walks `sub/child.md` everywhere). Seven "(rc)" / "(link)" controls
(drive-absolute, `\`-rooted, percent-encoded UNC, raw `\\server\share\x.md` → `\`-rooted by §2.4,
drive-relative raw = stage (a) and encoded = stage (c), the one-letter `n%3Achild.md`, the positive
`sub%5Cchild.md`), two mutants (the `/`-only test re-injected; the POSIX join re-injected); the R3-2 / R5-2 and
R7-2 mutants re-targeted at the new (c) line, the R3-1 image-tail mutant at the new `is_img` arm.
**398 controls, 209 mutants / 0 / 0**, `--self-test --mutants` 10.0 s; conformance **295 / 0 / 0** + **20 / 0
/ 0**; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the same 717** (`--worklist`
byte-identical to `4c15b7a7`'s tools on the same memo, path column included: the population's four memos all
sit in the root memo's directory, so the relative names ARE the former basenames); `[LEX-UNSUPPORTED?]` 6,
unchanged. `scripts/trip-wires.sh` rc 0.
⚠ **De-flake (commits `b0a7def4` + this note, after R19)**: the wall-clock ratio gates (`t(4N)/t(N) < 8`) were host-dependent in BOTH directions — once, under host load, a mutant killed only by such a gate survived (209 mutants, 1 survived; 6 clean runs followed); reproduced with 30 bursty CPU-load processes, the same gates went RED on the unmodified production code in 4 of 5 probes (`split_row` 8.4, orphan detection 9.3–10.6, `linked_files` 7.4–7.6 against the bound 8). The three controls whose mutants (`R6-2 locate`, `R8-5 sibling`, `R9 #3 row`) were killed only by a ratio now carry deterministic counting witnesses — `unresolved_references`: reads of the line-offset table ≤ N·(⌈log₂(N+1)⌉+2) (a bisect per site; 11,977/55,905 measured against 12,000/56,000) and ≥ N; `linked_files`: `PurePath.__eq__` ≤ N and `__hash__` ≥ N (a set hashes, a list compares: 0/0 eq, 2,001/8,001 hash); `split_row`: source lines executed in the module ≤ 64·(len + cells) (`sys.settrace`, 4 functions = 58 lines, 6.1/unit measured) — and the orphan-detection control keeps its exact call count and drops its ratio gate; counts are identical on CPython 3.9.6 and 3.14.6; timing is informative only. Measured: 10 consecutive `--self-test --mutants` runs under the 30-process bursty load → `209 mutant(s), 0 survived, 0 crashed` each (15.8–20.8 s wall); unloaded 3.4 s.
⚠ **PR #510 Codex R20 (2026-09-08)**: #2 (IMP) — the R14 family from the KIND side: `_APPOSITIVE` was built on
`ROW_NOUN_ID = ROW_NOUN_SEP + decorated_id(SHORT_ID)`, so a declaring field attributing the marker to a SLUG row
(`Slice `#11-zz-alpha` — **UMBRELLA, not a terminal unit**`) matched nothing: the row was read as its OWN
umbrella declaration, no `UMBRELLA-MARK` attribution finding, the pointer row inside the census, exit 0 — and the
R14 spelling sweep is blind to it (a `SHORT_ID`-only composer spells nothing twice). Fix in the grammar:
`ROW_KINDS = ("slug", "short")` / `ROW_ID` (`plan_memo_ids.py`), the ONE "a row id here" alternation — a citation
id keys a citation-table row but declares no kind, is never `Slice [C1]`, never owns, never carries the marker, so
it is not a row in this sense — composed by every row-position reader: `ROW_NOUN_ID` → `_APPOSITIVE`
(`plan_memo_tables.py`), `OWNS_TWO` (`plan_memo_roles.py`, whose local `(?:slug|short)` `_OWNER_CORE` was a second
spelling of it and is gone), and the anchored reading `_anchored` (`t.kind in ROW_KINDS`; ⚠ it read
`t.kind != "short"`, so a slug after a row noun was reported by the bare pass but never `anchored`, and the (c)
seed's prose-vs-`Deps` comparison never saw a slug named in a Slice cell). Sweep of every `SHORT_ID` /
`decorated_id(SHORT_ID)` use: routed — `ROW_NOUN_ID`, `OWNS_TWO` (2 groups), `_anchored`; kept — `_ID_RUN_TOKEN`
(all THREE kinds: `` `[C1]` `` in an id-only span is the document spelling a declared citation id), `KINDS` /
`_TOKEN` (the grammar itself), the checker's `tid.isdigit()` comment (the short kind's declared miss);
`_TRAILING_NOUN` composes NO id (it ends where the mention's token starts, whatever its kind) — the R20 brief
listed it as a composer to route, which is a false premise. Controls: "(a) the R20 reviewer's declaring field
…attributes the marker to a SLUG row…" (UMBRELLA-MARK 1), the bold-slug and short forms, "(d) `owned by
`#11-zz-alpha` and **Qx**`…", "(c-seed) `Lands after Slice `#11-zz-alpha``…" (+ its negative), the runner's
`attribution_control` now over both kinds (count unchanged, kind `pointer`), and the PROPERTY "every row-id
composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)" — 12 probes over
the grammar's tuple (a kind without a sample is red). Mutants: `ROW_ID = SHORT_ID` (kills all five at once),
`decorated_id(SHORT_ID)` re-injected at `ROW_NOUN_ID`, the short-only test re-injected in `_anchored`; the F6 (c)
mutant's re-injected `re.findall(SHORT_ID, …)` now names the grammar module explicitly (the roles import it relied on
is gone). #1 (MIN) the lexer's file-token arm required a stem of ≥1 character while `sibling_path` (d) accepted the
bare suffix, so beside a declared id `md` the prose `Read .md for details` reported `md`: ONE notion of "is a file
name" — `FILE_SUFFIX` defined in the lexer, consumed by `sibling_path` stage (d), stem unconstrained on both sides
(the token arm's run is `*`). Controls "(file) `.md` alone is a file name…" (0 sites), "(file) `notes.md` beside a
declared no-owner id `md`…" (0), "(file) bare `md` (no suffix)… IS a site" (1, the live subject), "(link) `[x](.md)`
links the sibling file named `.md`…"; mutants: the `+` re-injected in the lexer; a stem requirement re-injected on
the memo side (the two sides must agree). #3 (MIN, plan) the §3 Phase 1 → Phase 2 row's coverage cell cited its
control as proving a definition after a table "ENDS the table" while the mechanism column, the control's name and
its expectation (a one-cell ROW, width miss rc 2 — GFM Example 202) all say ROW: the cell now cites the control as
named. Every other "ends it" claim in the plan (a list item after a table, a type-7 opener after a table) matches
its control's rc-0 expectation. **409 controls, 214 mutants / 0 / 0**, `--self-test --mutants` 5.0 s (×2);
conformance **295 / 0 / 0** + **20 / 0 / 0**; census `48 = 33 + 15`, rc 0, **717 sites, the same 717**
(`--worklist` vs `ecd2bdcc`'s tools on the same memo: the file / line / id / source / role columns are identical;
two context columns shift left because an anchored mention's span now starts at the row noun — display only);
`[LEX-UNSUPPORTED?]` 6, unchanged; **`ORDER-PROSE?` 36 → 37**: the one moved row is §5's `Function`/`eval` pointer
row (line 1985, empty id cell, printed as `row None`), whose Slice cell writes `the umbrella
**`#11-vm-function-constructor-global`** (§8 — this row's own Slot cell) charters …` — the slug after the row noun
`umbrella` is now the anchored reading, so it enters the seed's prose set, and the `Deps` cell (a prose
`HostEnsureCanCompileStrings` prereq) does not carry it. The named id is the row's OWN slot (its Slot cell), not a
dependency: a seed hit of the (c) seed's declared natural-language class, invisible before only through the kind
hole this round closes. `scripts/trip-wires.sh` rc 0. ⚠ Two R20 brief premises were false: `_TRAILING_NOUN` is not
an id composer (above), and "the umbrella memo has 15 slot rows; if any declaring field attributes to a slug row
the census WILL change" — no declaring field in that memo does (48 unchanged); what moved was the (c) seed, via
`_anchored`, which the brief did not name. **#3 (MIN, follow-up commit)** — that finding printed `row None`: every
printer composed `row %r` of `Row.self_id` itself (eight sites in `plan_memo_roles.py`, one in `plan_memo_memo.py`),
so a row whose id cell is the literal blank `**—**` was named by nothing a reader could find. One naming site now:
`Row.name()` — the id's repr when the cell declares one (identical output for every keyed row), else the
declaring LOCATOR `<no id> at :LINE (TOKEN)` (the line and the declaring field's first token, decoration
stripped; line 1985 prints `row <no id> at :1985 (Function/eval)`); `Population.attributed` carries the name, not
the id. ⚠ The brief's "the ONE place rows are named in findings" did not exist — there were nine inline
`row %r` sites, and the fix is what creates the one place. Control (function-shaped, the message text is the
subject): an `extra` §5 table with a `**—**` row whose Slice cell opens `` `Function`/`eval` `` and orders after
a slug its Deps cell lacks — ORDER-PROSE? x1 on that line carrying the locator, no finding of the run spelling
`row None`; mutant `R20 #3` re-injects the unconditional `repr(self.self_id)`. 410 controls / 215 mutants 0 / 0;
census 48 / 717 / 37 / 6 unchanged.
⚠ **PR #510 Codex R21 (2026-09-08)** — the round that closed the INLINE family, and a **Step-4 PAUSE**:
`plan_memo_lexer.py` had drawn an IMP in four consecutive rounds (R17 §6.6, R19 §6.4, R20 the file token,
R21 §6.5), the shape the block phase had before R13. Option A: **§3.0b**, the spec's CLOSED list of inline
constructs (§2.4, §2.5, §6.1–§6.9) with a disposition each — LEXED / MASKED / PROSE-AS-WRITTEN — and a
control MEASURING each PROSE row's cost. #1 (IMP) §6.5 autolinks are ONE masked token tried at a `<`
before the tag grammar, so `<https://example.com/[child](absent.md)>` walks nothing (the false rc 2 is
gone); 14 `(autolink)` controls, 5 mutants. #2 (IMP) the marker short-circuit is the SEED's alone, so a
row marked in its declaring field AND again elsewhere is reported. #3 (IMP) KIND-SPELLING gates on
`pop.spellings`, never on a non-empty undetermined set. #4 (IMP) each `Schema` declares the id KINDS its
id column may key, applied in the one `bare_id`. Inline corpus 20 → **203 examples, 203 aligned, 0
excluded, 0 FAIL** (block 295 / 295 / 0). **439 controls / 226 mutants 0 / 0**; census 48 / 717 / 37 / 6
unchanged, finding-kind counts identical to R20's head.
⚠ Five R21 mutants first SURVIVED, all one class: each named a control whose SUBJECT sat outside the span
the mutation changes (an id glued to a domain label; a link placed after the would-be autolink instead of
inside it; a space after `<`, which fails at the SCHEME rather than in the tail class the mutation
widens). Each subject was moved inside the mutated span and each control now records why it is shaped so
— the `feedback_surviving-mutation-means-the-probe-has-another-subject` class, third sighting on this PR.
⚠ **PR #510 design re-gate 4 (2026-09-08) — the stream IS the rendered text**: the re-gate over
R14–R21 measured two live defects of ONE class, an id or a phrase **split by an inline construct**
where the RENDERED document reads one token: (i) with rows `9z` and `9` both declared, every one of
`Slice 9**z**` / `Slice 9<b>z</b>` / `Slice 9<!-- c -->z` / `Slice 9~~z~~` renders `Slice 9z` and was
reported on row **`9`** — a fabricated site on one row and a lost site on another, rc 0; (ii)
`**UMBRELLA, not a *terminal* unit.**` in a declaring field (or `&#44;` for the comma, or
`~~terminal~~`, or `<b>terminal</b>`, or an escaped `\,`) made the literal `MARKER` test — and the
near-miss backstop `DECLARES`, which carries the same literal — miss, so the row left the umbrella
census as an ACTIVE TERMINAL, silently, at rc 0: **the "clean exit for a could-not-scan" §1
forbids**, and the more serious half. Both are the same defect and take ONE fix, which is reading
(a) of the three the re-gate put: **the scanners consume the text a reader sees**. §2 I-A now states
it, §3.0b carries the per-construct `Renders` column that decides it, and the id scanners, the
marker, `DECLARES`, the undetermined and pointer spellings, `EMPTY_WORDS` / `is_empty`, the
acceptance and ordering vocabularies, the ownership connectives and the licensing rule all read that
ONE stream — the sweep is re-runnable and its result is an enumeration, not a sample: `grep -n` for
`MARKER` / `DECLARES` / `UNDETERMINED` / `POINTER` / `EMPTY_WORDS` / `ACCEPT_WORDS` / `ORDER_WORDS` /
`RETIRED` / `OWNS_TWO` / `ROLE_PATTERNS` / `LICENSE_BEFORE` / `LICENSE_AFTER` / `NOUN_ANCHOR` /
`_APPOSITIVE` over `plan_memo_roles.py plan_memo_tables.py plan_memo_memo.py
plan-memo-umbrella-check.py`, kept to the lines that TEST one (`search` / `match` / `finditer` /
`in` / `.sub(` — `grep -E '\.search\(|\.match\(|\.finditer\(| in |\.sub\('`), returns **18** sites, and every one of them reads `b.stream`, `m.text`,
`m.window()`, `row.field`, `stream(c.lexed)` or `_stream(row, …)`. `is_blank_id_cell` / `bare_id`
are the stated exception, since the id cell's decoration IS part of the id grammar and the two readings of that
cell agree by being the same reading. Readings (b) and (c) were rejected on §1, not on cost: (b)
"join across the invisible spans" needs the same §6.2 machinery (a `*` is invisible only when it
PAIRS — `9*z` renders verbatim), and still cannot read `&#44;` or `\,`, which are not invisible but
SUBSTITUTED; (c) "seed the class" leaves the wrong row named in the report and leaves the census
wrong at rc 0, which is the very exit §1 forbids. Where the answer IS mixed, the boundary is stated
and is not arbitrary: reading (a) covers every construct that renders no character; where a
construct renders text the checker refuses to read (§6.1 code spans, §6.5 autolinks, an image tail,
a citation id, a file name) the stream is deliberately NOT the rendered text (I-A), the two readings
disagree, and neither is this program's to pick — so that residue is (c), reported by `split_units`
as `[LEX-SPLIT?]`, and where it decides the census (the marker in a DECLARING field) it is a schema
miss, rc 2. New: `plan_memo_emphasis.py` (§6.2 + GFM strikethrough, 216 lines), `Stream` with its
offset map, `RENDERS_TEXT`, `split_units` / `_straddles`, `Population._kind_residue`,
`lex_split_seed`; `Emphasis and strong emphasis` Examples 350–481 vendored (inline corpus 203 →
**335 / 335 / 0**, block 295 / 295 / 0), the aligner claiming one `<em>` per 1-character pair and
one `<strong>` per 2-character pair, which is what caught the missing §6.4 demotion of emphasis
inside a resolved image's description (5 examples). Two second spellings of decoration are gone with
it: `is_empty`'s own `.strip("*`")` (it reads the cell's stream now) and `LICENSE_AFTER`'s
`(?:\*\*)?` prefix (measured: removing it moves no control and no site). **492 controls, 250
mutants / 0 survived / 0 crashed** (+53 controls, +24 mutants; 8 of the 24 SURVIVED their first run,
every one the R21 class — a named control whose subject sat outside the mutated span: a comment
blanked in a `Deps` cell is still an empty cell, a link tail that FOLLOWS both halves of a token
splits nothing, an unmatched `*` is unmatched under both flanking readings, and a probe of runs that
can only OPEN never enters the search `openers_bottom` guards. Each subject was moved inside the
mutated span and each mutant re-run); conformance **295 / 0 / 0** + **335 / 0 / 0**; census `48 = 33
+ 15`, rc 0, 37 ORDER-PROSE? rows unchanged, `[LEX-UNSUPPORTED?]` 6 unchanged, `[LEX-SPLIT?]` **0**
(the memo holds no straddle; the seed's controls are the fixtures), and — the one finding kind that
DID move, so this list is a delta and not a sample — `[UMBRELLA-MARK?]` **6 → 8**: main:2671 `an
**umbrella**` and main:2672 `**"Terminal under §5's criterion…` each state the kind vocabulary in a
declaring field with the word INSIDE emphasis, which is precisely the text this reading now sees;
both are gains, and rc is 0 either way because the class is a seed. Memo run **0.80 s → 1.41 s**
(the residue detector renders each block a second way, which is what the seed costs; measured twice
each, `/usr/bin/time -p`, `c6be7995`'s tools against this commit's on the same memo),
`--self-test --mutants` **3.5 s**. **Sites 717 → 714, and the three that moved are the fix working**
(`--worklist` against `c6be7995`'s tools on the same memo: file / line / id / source identical but
for these; nothing gained, 1,205 mentions unchanged, 488 → 491 licensed): main:1944 `4a` in `each of
*4a's* children`, detail:199 `1b` in `1b's **charter**` and main:2551 `A` in `row **A** is an
*umbrella* row` are each REPORTED → LICENSED, because the licensing phrase the emphasis used to
break — `'s children`, `'s charter`, `is an umbrella` — is now read as the document renders it. The
context and role columns shift on other lines (a role window of 110 characters of the stream reaches
further when the delimiters are gone): a RANKING over the reported set, never a filter on it —
measured, 8 sites gain a role vocabulary they always rendered (`(no role vocabulary)` 246 → 238).
`scripts/trip-wires.sh` rc 0.
⚠ **PR #510 Codex R22 (2026-09-08)** — three P2s, each a reading that disagreed with itself.
#1 (P2) the raw-line seed read ids with the id grammar ALONE, so a declared id inside a `.md` file
name (`slice-9z-sib.md` on an HTML-block or indented-code line) seeded `9z` where the same name in
prose seeds nothing — a seed whose contract is "what a naming scan would have read had this been
prose" reporting what no naming scan would read. The root is that "a bare `.md` file name or a
citation id, over RAW text" had no name: `Lexed.__init__` spelled it and the seed compensated with a
hand-written `t.kind != "cite"` covering only the citation half. `plan_memo_tokens.file_and_cite_spans`
is now that ONE reading, consumed by both, and the hand filter is REMOVED rather than kept beside it.
Subsumption measured, with its denominator: over the vm-p4 memo's **7** raw lines the hand filter and
the `cite` spans agree on **7 / 7**, and the `file` spans then remove ids on **1** further line —
which is main:2343, the defect. Seven lines is a sample, not a sweep, so the argument that carries the
removal is the CONSTRUCTION rather than the count: the hand filter dropped a token whose KIND was
`cite`, the spans drop a token that OVERLAPS a `cite` or `file` span, and the second is the weaker
test on the citation half except where a citation escapes its own `cite` span — `([C1]).md`, where
the balanced-paren arm swallows it, and the `file` span covers it there.
#2 (P2) `sibling_path` stage (c) took an empty `PureWindowsPath.anchor` as sufficient, so the
relative `NUL.md` / `dir/NUL.md` passed; on Windows that OPENS and yields an empty stream, so the
population counted a linked memo it never scanned and could exit 0 having dropped the sibling — §1's
forbidden clean exit again, and the same class design re-gate 4 closed. `_is_reserved_component`
applies CPython `ntpath._isreservedname`'s DEVICE reading (cited to Microsoft "Naming Files, Paths,
and Namespaces") per PART, in pure string logic so it decides identically on POSIX and is testable
there; neither `PureWindowsPath.is_reserved()` (deprecated 3.13, removed 3.15) nor
`os.path.isreserved` (3.13+, `ntpath`-only) is callable by a checker that must decide this the same
way everywhere. The CHARACTER half is deliberately NOT repeated, and the boundary is drawn on a
discriminator rather than on taste: a device opens successfully and returns empty (silent, rc 0),
whereas a reserved character raises `OSError` on Windows and is ALREADY an unavailable linked memo at
rc 2 — and rejecting `*?"<>:|` here would contradict R8's decided reading of `notes%3Achild.md` as
the local file `notes:child.md`. Cost named, not hidden: a POSIX memo genuinely called `NUL.md` is no
sibling now either, dropped without a report exactly as `/abs/x.md` and (since R19) the POSIX file
named `sub\child.md` are — this stage's standing polarity, one platform-independent reading.
#3 (P2) the undetermined-kind marker matched inside a longer word (`MANKIND UNDETERMINED`,
`KIND UNDETERMINEDNESS`, even `unkind undetermined`), which with a non-empty `Deps` cell emits
`UMBRELLA-CELL` and moves rc to 1. Fixed as a SWEEP, not as the reported spelling: there was no
spelling of "a phrase boundary" to compose, so `plan_memo_ids` now owns `BEFORE` / `AFTER` /
`bounded(phrase)` (the module that already owns `ALNUM` and is the only one the spelling sweep lets
spell a character class; `NOUN_ANCHOR` composes `BEFORE` instead of re-spelling the lookbehind), and
every phrase matcher the census reads was audited. Six are now bounded — `MARKER_RE` (a bare `in`
test at 6 call sites; `SUBUMBRELLA, not a terminal unit` matched), `UNDETERMINED`, `POINTER`, and
**three the review did not name and the sweep found**: `DECLARES` (unbounded both sides),
`LICENSE_BEFORE` (unbounded left — `grandchild of 9z` licensed) and `LICENSE_AFTER` (unbounded right
— `9z's memorandum` licensed). `LICENSE_*` is the dangerous polarity, since a match SUPPRESSES a
report, so tightening is the safe direction. The matchers left alone (`ROLE_PATTERNS`, `ORDER_WORDS`,
`ACCEPT_WORDS`, `RETIRED`, `OWNS_TWO`, `EMPTY_WORDS`, `ID_CELL_BLANKS`) each carry the reason now —
`\b` under `re.ASCII` IS a boundary, differing from `ALNUM` only on `_`. Residual named rather than
claimed away: `classify` hands the backward look a 40-character slice, so a 40-character licensing
phrase begins at index 0 and the lookbehind succeeds vacuously (measured reachable).
**517 controls, 264 mutants / 0 survived / 0 crashed**; conformance 295 / 0 / 0 + 335 / 0 / 0;
`scripts/trip-wires.sh` rc 0; census rc 0 with `51 seed(s)`, 1,205 mentions, 491 licensed, 714
REPORTED and 8 `[UMBRELLA-MARK?]` all unchanged, and the `--worklist` differing on **exactly one**
line: main:2343 `        docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md |` loses `'1a'`
and `'1b'` — both came out of the FILE NAME, which is #1 working — while the line still seeds,
because it holds a `|`.
⚠ One R22 mutant SURVIVED its first run — the R21 class, **fourth sighting on this PR**: reading
`p.parts[-1:]` instead of every part left the `dir/NUL.md` control green, because there the device IS
the final component and the probe's subject sat outside the mutated span. The control's subject moved
into the span (`NUL/child.md`, the device as a DIRECTORY) rather than the mutant being weakened, and
the control records why it is shaped so.
⚠ Every device control's destination names a file that EXISTS in the fixture directory (the shared
`DEVICE` set writes all six), so a green NEGATIVE means "not read", never "not found" — the
distinction this whole class turns on, and the one a fixture-free control could not make.
⚠ **PR #510 Codex R23 (2026-09-08)** — five findings, four real, and **one already fixed**: the
reviewer asked for a right boundary on the licensed possessive nouns (`9z's memoized` and
`9z's chartered` were licensed), which is exactly what R22 #3's SWEEP had closed a commit earlier.
Measured at that head: both are REPORTED (1 site each) while `9z's memo` / `9z's charter` stay
licensed. That is external confirmation of the sweep's premise — the matcher the review did NOT
name was the one the review would have named next.
#1 (P2) a §2.5 reference whose character is `*` or a backtick was excluded from substitution (so a
decoded `*` could not become live markup) and then left STANDING AS WRITTEN, which is a FOURTH
disposition beside drop / substitute / blank, and the only one that puts characters into the stream
the document does not render: with a no-owner row `ast` declared, `The &ast; marks it.` reported a
site `ast` that no reader can see (`The * marks it.` reports none). The re-gate-4 defect in the
fabricating direction. Disposition chosen: **BLANK** — the span renders TEXT the checker refuses to
read, which is what `RENDERS_TEXT` means; not a DROP, because `9&ast;z` renders `9*z` and dropping
would join the sides into a `9z` no reader sees. `[LEX-SPLIT?]` interaction, settled by class and
not by luck: an id can never straddle such a blank, since the reader-side fill is a `*` or a
backtick and neither is in any id token's character class — a PHRASE can, and should, so
`KIND&ast;UNDETERMINED` is rc 2.
#2 (P2) the residue gate covered the marker and nothing else, so `` KIND UNDETER`MINED` `` — which a
reader reads as `KIND UNDETERMINED` — reclassified the row as terminal, dropped its no-owner
mentions and exited **0**. ⚠ **This was a defect in re-gate 4's own fix**: I gated the phrase I was
looking at and left every other phrase that decides a row's kind authoritative by default, which is
the enumerated-exemption class this program keeps meeting. The fix is the construction, not the
addition: `KIND_PHRASES` is now the ONE enumeration, `_kind` builds its hits from it and matches
nothing directly, the gate and the seed iterate the same tuple, and a PROPERTY control reads
`_kind.__code__.co_names` (nested code objects included) and fails on any name that resolves to a
`re.Pattern` outside the tuple — so the next phrase is covered by default AND cannot slip past.
⚠ The same class in the OPPOSITE direction was found while fixing it and is not in any review:
`` KIND `x` UNDETERMINED `` classified the row *undetermined* at rc 1, because a blank stands as
spaces and `UNDETERMINED`'s `\s*` reads across it a kind the reader (`KIND x UNDETERMINED`) does
not. Gating only the reader side would have left that authoritative, so the gate asks per phrase and
keeps two conjuncts: the two readings must DISAGREE (a phrase spelled cleanly somewhere is not in
doubt), and the disagreement must come from a STRADDLE (a phrase quoted WHOLE is I-A's deliberate
disposition and stays no miss).
#3 (P2) an initial BOM was not stripped, so a linked memo whose first block is a schema table lost
that table, its rows never entered `Population.ids`, and the run exited 0 with no miss. One U+FEFF
is stripped at the single place the text becomes lines. ⚠ Honest about the citation: `webref` has no
CommonMark source and no spec prose is vendored here (only the two example corpora), so §2.1 is
cited BY SECTION NUMBER WITHOUT a machine-readable source, and the code says so; the corpora cannot
cover it either (0 of 630 examples holds a U+FEFF). What IS verified in-tree is that `utf-8` decodes
the BOM to a character — `utf-8-sig` is the codec that consumes it — which is why it landed inside
line 1. The control that carries the rule is the TWO-BOM one: one is stripped, two are not, which is
what pins "exactly one" and kills an `lstrip` mutant.
#4 (P2) nested-image demotion was quadratic against a docstring that claims linear. ⚠ The review
reported one loop; the second was measured while fixing it and had the identical shape (`pairs`, the
emphasis half: 0.004 / 0.015 / 0.057 / 0.223 s at n=250 / 500 / 1000 / 2000, also 4× per doubling).
Root: the range a close must demote was SEARCHED for by walking back over entries an inner close had
already tagged, so each descendant was tagged once per ancestor — but the range is an O(1) fact of
the stack, recorded at the `[` exactly as `delim_bottom` already was, and applied once as a union
via a difference array. Re-measured after: images 0.0013 / 0.0024 / 0.0049 s at n=1000 / 2000 / 4000
and pairs 0.0017 / 0.0033 / 0.0070 s at n=500 / 1000 / 2000 — linear in both. The work claim is a
deterministic `_count_lines` witness with a 4×-input / ≤4×-work bound, never wall-clock; the
correctness half needs no new control, since the 335 inline conformance examples were green on both
sides of the change.
**534 controls, 278 mutants / 0 survived / 0 crashed**, 0 `(unknown control)`; conformance 295 / 0 /
0 + 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; census rc 0 with 51 seeds, 1,205 mentions, 491
licensed, 714 REPORTED and 8 `[UMBRELLA-MARK?]` unchanged, the `--worklist` differing on exactly one
line and that line WORDING ONLY (the `[LEX-SPLIT?]` seed's own sentence, since the residue is
symmetric now and covers every kind phrase, so "the reader reads" and "the marker" were both false
as written). Cost: the census run 1.46 s → 1.85 s, because `split_units` now builds both renderings
of every block instead of one.
⚠ A PRE-EXISTING mutant (`R21 #1`) survived its first run here, for the R21 reason once more: the
new `_demote` range covered the demoted-link entries and overwrote the mutation, so the control's
subject had left the mutated span. The `dem_img.append` moved back before the `out`-pop loop so the
link conversion stays the ONE site saying a demoted link is not an image. Five other mutants whose
anchors this round moved failed LOUDLY (`substring occurs 0 times`), never silently — which is the
difference between an anchor that moved and the `(unknown control)` class R22 collapsed.
⚠ **Not yet a control**: the AST check that no constant is declared by both derived mutant
registries (R22) is still an ad-hoc script. Nothing in `--self-test` enforces it.
⚠ **Step-4 PAUSE assessment (2026-09-08, after R23)** — `plan_memo_memo.py` and `plan_memo_tables.py`
had each drawn a finding in THREE consecutive rounds (R22, R23, R24), which is the signal to stop
patching and ask whether the mechanism is the problem. Sorting R22–R24's findings by ORIGIN rather
than by symptom gives **three families**, each still generating: (1) *which text does this reader
read* — R23 #1, R23 #2 and R24 #4 are all incomplete application of re-gate 4's DECLARED rule, whose
weakness is that the rule is prose rather than a control; (2) *the boundary where bytes become a
document* — R22 #2 (device names), R23 #3 (BOM) and R24 #1 (literal NUL), which want §2.1's
preprocessing as a unit at one site rather than one strip per report; (3) *a matcher's context
boundary is a magic constant or absent* — R22 #3 (unbounded phrases), R24 #2 (a 70-character
backward window) and the 40-character slice in `classify` recorded above, which want the boundary
derived from the grammar. Option A (collapse) is taken for all three, each with a structural guard,
rather than four symptom fixes — the entry for that round records what each guard cannot see.
⚠ **PR #510 Codex R24 (2026-09-08) — the collapse round.** Four findings, all real, fixed as the
three families above plus one located defect, behind a prereq split. **Touch-time split first**
(`ef97295d`): `plan_memo_selftest_controls.py` had reached 961 lines and this round adds controls to
it. Seam = **the measure** — a control that states a COST and counts it with the deterministic
witnesses moves to `plan_memo_selftest_work.py`, which is now the ONLY importer of `_count_calls` /
`_count_lines` / `_CountedList`, so "is this a work control?" is an import list rather than a
judgement (`deep_nesting_control` stays: it counts nothing). Eight bodies moved byte-identically
(AST-segment checked); 961 → 671.
**Family 1** (`82552f2d`) — the last raw-text reader. `9z&#32;notes.md owns it.` renders
`9z notes.md owns it.` and so names `9z`, but `file_and_cite_spans` masked the whole source run and
the ownership claim vanished (0 sites, against 1 for the rendering-identical prose). Root: `dispose`
itself asked TWO questions of the raw source; it is two stages now, and stage 2 asks both
rendered-text questions of ONE text. ⚠ A second member, in no review, was found by enumerating the
family rather than the reports: the decoration exception's `id_only` also read raw content, so
`**&#57;z**7z` reported 0 sites where `**9z**7z` reports 1. **Structural guard**:
`render_equivalence_control` replaces each character of one prose IN TURN by its §2.5 numeric
reference and requires the census not to move — and the set of positions it must skip is read off
`inline_pass`'s own code object rather than hand-listed, so under-exclusion is RED and never
silently weaker. What it cannot see, stated: two-position disagreements, cells, shapes that prose
does not reach, raw lines.
**Family 2** (`91bfe590`) — `Memo._preprocess`, CommonMark §2's input preprocessing as ONE unit
enumerated from the section: U+0000 → U+FFFD, line endings normalised (the file opened with
`newline=""`), tabs deliberately NOT expanded, and the BOM carried as this program's own encoding
rule claiming no spec sentence. Before it, a link whose destination was `child\0.md` (a literal NUL
in the name) was rejected as a control character instead of linking the file, so that memo and its
violations left the population at rc 0. ⚠ Written WITHOUT the bracket-and-parenthesis form on
purpose: this checker reads its own plan, code spans are lexed over the whole PARAGRAPH rather than
per line, and an example spelled as a link inside a long paragraph pairs its backticks somewhere
else and becomes a LIVE link to a memo that does not exist — measured, it added a sixth FATAL to
this document before it was rewritten.
⚠ **Three of my briefing premises were wrong here and the delegate said so rather than absorbing
them.** (i) The §2.1 attribution: the subsection number is undeterminable in this tree (the corpora
carry section NAMES only and hold no example from the quoted sections), so the unit cites **§2**,
which is certain, and names each transformation by the sentence it implements — no number invented.
(ii) I claimed the NUL substitution makes `_CONTROL` a guard nothing can trip; it does not, because
stage (c) tests the PERCENT-DECODED name, so `%00`–`%1f` and `%7f` decode back to controls that the
unit never touches (verified here: `child%00.md` → not a sibling, and both `STAGE_C` mutants still
kill its control). (iii) §2's line-ending requirement was already met — by accident, through
`read_text`'s universal newlines — so it was a requirement nothing PROVED; the control that proves
it writes **bytes**, since a text write would translate `\n` and make the CRLF arm a claim about the
host instead of about the checker.
**Family 3** (`96b3cbb4`) — both context windows are `endpos`/anchor-bounded now
(`_APPOSITIVE.search(field, 0, m.start())`, `LICENSE_BEFORE.search(m.text, 0, m.start)`), with no
width constant and no block-length copy in either input; the forward side keeps its slice
deliberately, since it runs to the end and truncates nothing. ⚠ **A fourth premise of mine was
wrong**: the 70 was never what held the mention-only direction. The DASH is — `Unlike Slice 7z,
**UMBRELLA, …**` fails on the comma at any width (verified), while a 76-character slug with the dash
now attributes where it returned `None` before. **Structural guard**:
`anchored_matcher_width_control` — an ANCHORED pattern is never handed a subject a NUMBER truncated;
the predicate is "the receiver resolves to a compiled pattern" rather than a method-name or
argument-position list, and a local assigned from such a slice counts, so it fires on the
variable-hidden shape too. Cannot see: a textual anchor test, non-global receivers, taint beyond one
level inside a function, and truncation inside a helper (`Block.window` / `roles()`, which is not a
finding: those patterns carry no edge anchor and rank rather than filter).
⚠ **A deletion the mutation proof forced.** Porting `_TRAILING_NOUN` into `LICENSE_BEFORE` produced
a mutant that SURVIVED. It is unreachable: wherever a row noun precedes a declared id, the ANCHORED
pass reads that site, its span starts AT the noun and it wins the dedup — and dropping the
`_TRAILING_NOUN` substitution at `3a9f61a0` ALSO turns no control red and leaves the census
byte-identical, so it was already dead there. Deleted rather than ported, because a clause nothing
can reach reports coverage the rule does not have.
**R24-3** (`59f71999`, P3) — a §6.6 span crossing a line ending seeded its whole content against the
OPENER's line, sending a reader to the wrong place for content the ordinary naming scan deliberately
masks (so the seed is its only diagnostic). `_inline_raw` yields one entry per LINE now, each located
through `Paragraph.locate` at its own offset; the control computes the expected line FROM the
fixture, since a constant would be a claim about `HEADER`'s length.
⚠ Two "this mechanism is unreachable" comments quoted the control-registry SIZE at the moment of
measuring (540, 554) — a count that is stale the moment the next control lands, leaving a reader
unable to tell a wrong claim from an old one. Both name the MUTATION and the two commands now
(`a8178006`). ⚠ And one of them had left a *live justification for a dead case* at the rule itself
(`d420b632`): `stream()`'s drop-over-blank precedence still argued from "a link tail holding a `.md`
file token", which is precisely the overlap family 1 removed — since the tokens are read off the
rendering, no `file` span is emitted inside a dropped tail at all. The ordering stands with NO
witness; it is kept because `disp` is a max over 0 < 1 < 2 and the ordering is what makes that max
total, and the docstring now says how to look for a shape that would restore one, with the
denominator stated (four shapes tried, none overlapped — a sample, not a proof).
**555 controls, 288 mutants / 0 survived / 0 crashed**, 0 `(unknown control)`; conformance 295 / 0 /
0 + 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; and the #506 census `--worklist` **byte-identical** —
zero changed lines, which is what a round of root fixes with no census movement should look like.
⚠ **Two from the re-gate's own backlog, in no review** (`dd26b5b7`). (a) The row-kind test had TWO
spellings: the anchored pass asked `t.kind not in ROW_KINDS` — the grammar's closed set — while the
BARE pass and the residue scan asked its complement (`t.kind == "cite"` / `!= "cite"`). They agree
exactly while the kinds are {slug, short, cite}, so no behaviour rides on the collapse; what rides
on it is the NEXT kind, which added to `KINDS` and not to `ROW_KINDS` would be excluded by the
positive spelling and admitted by the complement — the two passes disagreeing about what a row id
is, with nothing comparing them. Measuring the complement rather than fixing the site I remembered
found the second one. (b) The escaped-bracket control measured SCHEMA findings containing `linked
memo not found`, a phrase NO producer emits (the chokepoint says `linked memo unavailable`).
⚠ **My first reading of this was wrong and the mutation proof is what corrected it**: I recorded it
as a control that could never go red, but a `schema` measure is scored
`got == expect and (res.rc == 2) == (got > 0)`, so the control DID go red under its mutant — with
the escape defeated the link goes live, rc becomes 2 while the count stays 0, and the second
conjunct breaks. What was actually dead is the COUNT half: all the discrimination rested on the rc
conjunct and the needle was decorative, so a later relaxation there would have left the control
silently vacuous. Both halves carry now, with a positive partner exercising the count. The property
that pointed at it — the only substring needle every one of whose cases expects 0, 1 of 31 tuple
measures — is a POINTER, not a proof of vacuity, and this control is the counter-example proving
the difference.
⚠ **E3 re-measured**, since the figure carried in the backlog (74 of 436) is not comparable to the
current registry: `523` fixture controls, of which **204 are named by no mutant**
(`{c.name for c in CASES} - {name for row in MUTANTS for name in row[4]}`). That is the honest size
of "a control that has never gone red", and it is the largest open item on this PR's own list.
⚠ **PR #510 Codex R25 (2026-09-08)** — two findings, both real, and **both landed inside limits R24
had DECLARED**. Family 1's guard says in its own docstring that it cannot see "positions holding a
character `inline_pass` branches on"; R25-2 is a backslash, exactly such a character. Family 2 wrote
its discriminator down as "a device opens successfully, whereas a reserved character raises
`OSError` and is already reported at rc 2"; R25-1 is a counter-example to that sentence. **An honest
blind spot is a map of where the next finding lands, not a disclaimer** — the lesson of the round,
and worth more than either fix (`memory/feedback_declared-blind-spots-are-where-the-next-finding-lands.md`).
#2 (P2) a §6.7 hard line break left a literal backslash in the stream: `Slice\` + newline +
`C owns it` gave **0 sites** where the soft-break and two-space spellings each give 1 (`C`), because
`_is_escape` accepts a backslash only before ASCII punctuation, so the pair recorded nothing and
`NOUN_ANCHOR` could not cross the literal backslash. Fixed as a **mark, not a substitution**: the
spec says the BREAK renders the line break and the backslash is markup, and since a §6.8 soft break
reaches the stream as the source `\n` standing as itself, the backslash now renders nothing and the
line ending stands — the two spellings agree BY THE SAME MECHANISM rather than by two constants that
coincide. `_is_hard_break` is its own predicate, because widening `_is_escape` would change the link
grammars, where a backslash before a line ending escapes nothing. The guard is extended at the level
of the PROPERTY, not the function: no single-character re-spelling can spell a line break, so
`render_equivalence_control`'s exclusion of `\` is correct and stays (`&#92;` before a line ending
really does render a literal backslash plus a soft break — a false alarm, not a missed defect), and
the new property is invariance of the verdict under re-spelling any one line break as each of
CommonMark's three: 12 re-spellings of 4 breaks, **0 disagreements after, 2 before**.
#1 (P2) an NTFS **alternate data stream** passes the sibling guard: `notes%3Achild.md` resolved to
the local file `notes:child.md`, which on Windows opens the stream SUCCESSFULLY if it exists, so the
census can scan unrelated content and still exit 0. The finding also falsifies the argument R22
wrote down — "on Windows a name holding one of `*?\"<>:|` raises `OSError`, so stage (e) reports it"
is false for an ADS path — so that sentence is gone and the predicate is drawn on the POLICY instead
of on the failure mode. `_is_reserved_component` carries the character half now, for the whole class
rather than the reported member. ⚠ **This REVERSES R8**, and the reversal is written into the
control's own text rather than only a commit message: what decided it, what survives of R8 (the
scheme test still reads the raw path, and `notes%3Achild.md` still has NO scheme — it is stage (c)
that refuses it now), and what it costs (a POSIX memo genuinely named `notes:child.md`, or holding
any of the six others, stops being a sibling and is dropped without a report — the standing polarity
`/abs/x.md`, `C:\x.md`, `sub\child.md` and `NUL.md` already carry). For the six characters other
than `:`, whether Windows raises is stated as **not determinable from this tree**, cited to
Microsoft rather than guessed.
⚠ **The largest consequence, and it SHRANK the proof**: stage (a), the URL scheme test, is now
behaviourally subsumed — every scheme ends in `:`, decoding never removes one, and a `:` in any
component is refused at (c). Two ratified mutants (`F3`, `R8-1`) became **equivalent** — no longer
killable by any control, which is a permanent FAIL if kept — and were deleted with the reasoning
recorded where they stood. Verified twice independently: 367 destinations by the author, and **370
by a differently-constructed sweep** (9 schemes × 8 bodies × 5 encodings plus relative controls,
comparing `sibling_path` as written against a variant with stage (a) removed), **0 disagreements**
both times. Stage (a) is KEPT — it asks the URL standard's own question, first and for the right
reason — and its subsumption is a fact about the current character set: if a destination is ever
found that (a) refuses and (c) admits, those rows come back with it.
**570 controls, 289 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
+ 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; census `--worklist` **byte-identical for the third
consecutive round**. Two more touch-time splits landed as standalone pure moves: the PROPERTY
controls, and the sibling-resolver controls — the first `Case` module carved on a **subject** rather
than a review round, since that resolver had been reported at R3, R4, R5, R8, R19, R22 and R25.
⚠ **This document's own checker run, settled.** Running the checker on THIS plan exits 2 with four
`no table matched schema` FATALs, and that is BY DESIGN, not a defect: `SCHEMAS` matches one
document family's exact header rows (#506's memo), so any other plan memo is a schema miss — which
is exactly why the MEMO run is not a trip-wire and only `--self-test --mutants` is (the checker's
own module docstring says so under WHERE THIS RUNS). What WAS a defect is now fixed: a fifth FATAL,
an unavailable `child.md`, came from a prose EXAMPLE whose backticks paired elsewhere in a long
paragraph, making it a live link to a memo that does not exist. Second sighting of that shape in one
day — the other was in this very ledger — so the rule is written where an author will hit it: code
spans are lexed over the PARAGRAPH, not the line, and an example spelled as a link inside a long
paragraph is a live link until proven otherwise. Re-check with
`python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-08-plan-memo-umbrella-checker.md`:
four FATALs is the floor, five means someone spelled a link.
⚠ **PR #510 Codex R26 (2026-09-08)** — five findings: four real, and **one rejected on its stated
grounds whose CONCLUSION was right for a reason the report did not give**.
#1 (P2) claimed §6.3 "permits at most 32 levels" of destination parenthesis nesting. It does not —
`spec.txt` 0.31.2 lines 7492-7494 read "(Implementations may impose limits on parentheses nesting to
avoid performance issues, but at least three levels of nesting should be supported.)", which is
PERMISSION; commonmark.js 0.31.2 imposes none and cmark 0.31.1 caps at 32, so the two reference
implementations disagree and the vendored corpus cannot arbitrate (deepest destination in all 630
examples is Example 496, depth 2). ⚠ **The cap was taken anyway, and the deciding fact is one
neither the report nor the first fix named**: a plan memo is READ ON GITHUB, which renders with
cmark-gfm — the implementation this program already uses as its GFM oracle — and measured through
it, 32 levels come back as an anchor while 33 come back as literal text. Above 32 the PUBLISHED
document holds no link, so reading one would be the checker inventing a memo its own reader never
sees, which is the class every finding on this PR has closed. Where two conforming implementations
disagree, **the one that renders the artefact wins**; the re-runnable `gh api -X POST /markdown`
command is in the code. The fact survived its stated reason being false —
[[feedback_ao-name-not-section-number-in-briefs]]'s "反証の理由と事実は別に検証".
#2 (P2) the file token and the sibling resolver disagreed about parentheses: `foo((9z)).md` matched
only the `.md` suffix, leaving the declared id `9z` exposed while `sibling_path` accepted the whole
name. `FILE_SUFFIX` is documented as the ONE spelling both readers consume, and that is the claim
this falsified. The LEXER moved, not the resolver — the resolver has no boundaries to find, its
input already delimited by the link grammar, while the token reader has nothing BUT boundaries, and
only its paren rule was ever an approximation of §6.3's "balanced pair of unescaped parentheses",
which holds at any depth and is not a regular language. ⚠ The fix reads a SECOND shape the report
did not name and which is worse: `(m.md(9z)md).md` is balanced ACROSS its groups, so it is one name
— the flat arm split it in two with the id exposed between them. The first shape leaks a
parenthesis; this one leaks an id.
#3 (P2) `inline_pass` was non-linear in **four** places and the report named one. Taking "is
`inline_pass` linear?" as the subject rather than "is this loop linear?" found the §6.6 tag grammar,
the §6.1 code closer and the bare file token beside the reported §6.3 tail; one rule covers all four
(a lookahead that cannot succeed is not attempted, one that can is bounded), 3.3-4.0× per doubling
before and 1.98-2.02× after. Two members were beyond ANY existing witness — the §6.6 scan runs
inside the C `re` engine, where neither the line-count nor the call-count witness can see it — so
two production seams exist purely to make the claim countable. No wall-clock.
#4 (P2) the self-test harness read sources with the LOCALE encoding, so
`PYTHONUTF8=0 LC_ALL=C … --self-test` died before a control ran. The report named 2 sites; the sweep
found **15** in 4 files. ⚠ And the class did not stop at call sites: with all 15 fixed the command
got further and died anyway, on a `§` in a control's NAME, because the output STREAMS carried the
same dependence — which a call-site sweep structurally cannot report, since it looks for a missing
argument and this was a missing call.
#5 (P3) was OURS: the per-file inventory above, wrong for the very tree it named. Refreshing it
would have been the third refresh; it is removed instead, for the reason recorded there.
⚠ **Why the family guards did not catch their own next members** — the second round running in which
this is the most useful output. R26-2's guard was a SHARED CONSTANT (`FILE_SUFFIX`) plus a comment
asserting both readers consume it: a shared constant guards the VALUE, not the GRAMMAR around it,
both sides genuinely read `".md"`, and the entire disagreement lived in what may precede it — the
guard could not fail. It is a sweep over a generated corpus now, which asks what the constant only
asserted. R26-3's guard was "30 nested brackets are one `inline_pass` call", which guards RE-ENTRY,
while the whole remaining class is LOOKAHEAD and never re-enters anything; the docstring had the
same shape ("no substring is re-parsed"), so prose and control were both about substrings while the
cost sat in the lookaheads.
**587 controls, 308 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
+ 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; `PYTHONUTF8=0 LC_ALL=C --self-test` rc 0; census
`--worklist` **byte-identical for the fourth consecutive round**. Three more touch-time splits
landed as standalone pure moves (`plan_memo_html.py`, `plan_memo_tokens.py`, and the R26 case /
mutant modules).
⚠ **PR #510 Codex R27 (2026-09-08) — "is it linear?" stops being a list of shapes.** Three
findings, all real. Two are non-linear scans and, counted properly, they are the **eighth and
ninth** across four rounds: R23 fixed 2, R26 fixed 4 (one reported, three found only because the
subject was "is `inline_pass` linear?" and not "is this loop linear?"), R27 brings 2 more — and one
of them is not in `inline_pass` at all (`_straddles`, on the ALWAYS-RUN seed path), which is the
evidence that the family is "this program's scans", not "that function". Every round had added a
work control for the shape that was reported, and every round the next shape had none: **a
hand-written adversarial string is a population defined by the symptom vocabulary**, the class this
program keeps failing on ([[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]], where
this is now recorded as the sixth form — the first one about a COST contract rather than a
correctness claim).
So the deliverable is `plan_memo_selftest_growth.py`, a property whose corpus is **generated from
the grammar**: every one-character literal any checker module COMPARES AGAINST (an `ast.Compare`
walk over the loaded sources, so a mutant's patched text is what is read), the emphasis delimiters,
every raw-HTML opener with its own declared closer, one document spelling per id kind (a kind added
without a spelling turns it red), and the bracket constructs — 49 atoms, each repeated and every
unordered pair interleaved, **1,225 probes**. The witness is executions PER SOURCE LINE, not their
total: a total is dominated by the linear pass and hides a two-line rescan until the input is
thousands of characters, so both of this round's defects are invisible to it at 200 characters; per
line the constants cancel, linear doubles and a rescan quadruples. Two stages (nominate at 6/12,
confirm at 96/192) because the §6.3 destination scan is bounded at 32 and has not plateaued at 6/12,
so it LOOKS quadratic there — a mutant proves the two stages are two sizes and not one.
**Verified independently of the author**: re-injecting each defect turns the property red AND names
the source line — `'![' + '[x](a.md)'` :: `plan_memo_lexer.py:824`, 4656 → 18528 (1.59× the bound);
`'![x](a.md)' + '9z '` :: `plan_memo_tables.py:785`, 18816 → 74496 (1.58×). Neither shape was written
for the defect it caught. What it cannot see is stated IN the control rather than discovered a round
later: work inside the C `re` engine (no Python line runs — which is why the §6.6 cost needs a
counted seam), anything outside a block's inline reading, a cost superlinear in nesting DEPTH rather
than in the repeated unit, and constant factors.
#1 the Appendix's "set all `[` delimiters before the opening delimiter to inactive" was WRITTEN into
every entry at every close; since every opener still on the stack precedes the opening delimiter and
none is ever reactivated, it is a COUNTER — the entry records how many links had closed when it was
pushed and is active while that figure is current. O(1), and the stack entry becomes immutable.
#3 `_straddles` summed the whole blank list per token and per phrase; `Stream.blanks` is ordered and
non-overlapping by construction, so the window is a `bisect` away. Its second control checks the
DEFINITION read one character at a time (23,040 questions over every blank layout in two adjacency
regimes) rather than the retired code, so it cannot bless a shared misreading.
#2 (P2) `ROW_NOUN` omitted `Slot`, so a slot row attributing the marker with its own schema noun
went silent: `Slice` / `Row` / `Umbrella` give `pointer` at rc 1 with `UMBRELLA-MARK`, `Slot` gave
**`umbrella` at rc 0 with no finding** — the class §1 forbids. Fixed by construction: the nouns are
the two generic ones plus the `name` of every ROW-KEYED schema, so the next schema's noun is covered
by default. The kind filter matters and its control sweeps BOTH directions, since deriving from
every schema would make `` Citation `#11-zz-alpha` — … `` attribute, a category error the positive
half cannot see. ⚠ **This moved the census for the first time in five rounds, in the right
direction**: exactly one row added (`…umbrella-review-rounds.md:149`, where `**Slot P registered**`
genuinely names row `P`), four rows printing a 5-6 character wider left context because those
mentions are now ANCHORED (the span starts at the noun; dedup is on the id position, so they are the
same sites through a wider span). 1,205 → 1,206 mentions, 714 → 715 REPORTED, licensed unchanged.
⚠ **The cost, and the stale figure it exposed.** The sweep runs once in `--self-test` and once per
mutant naming it, so `scripts/trip-wires.sh` goes 5.8 s → **26.2 / 27.2 s** locally (two runs,
`/usr/bin/time -p`). On the 4.3×-slower runner the earlier measurement used that is ~115 s against
`timeout-minutes: 5` — headroom of **2.6×, not the 10×** the workflow comment claimed. CLAUDE.md's
copy was updated with the sweep; `ci.yml`'s comment was NOT — and CLAUDE.md names that comment as
the canonical reason, so the pointer was current while the canon was stale. Both say 2.6× now, with
the instruction that the next growth re-derives the timeout instead of assuming it. Same class as
the module map three consecutive splits failed to update: **a change's blast radius includes every
site that STATES the figure, not only the site that caused it.**
**592 controls, 318 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
+ 335 / 0 / 0, unchanged by both linearity fixes as they must be; `scripts/trip-wires.sh` rc 0;
`PYTHONUTF8=0 LC_ALL=C --self-test` rc 0.
⚠ One re-anchored mutant SURVIVED and the subject was moved rather than the mutant weakened —
a shape worth naming: **converting a written FLAG into a read-time PREDICATE silently moves where a
mutant's subject lives.**
⚠ **PR #510 Codex R28 (2026-09-08)** — two findings, both real, and the round corrected how we had
been framing the whole blind-spot pattern.
#1 (P2) the memo walk drained its queue quadratically (`queue.pop(0)` shifts the whole list; and
duplicate paths accumulated). Fixed with a `deque` and `seen` consulted at SCHEDULE time.
⚠ **Our framing was half wrong, and that is the useful part.** We expected this to sit in the limit
R27's generated property declares — "work outside the block-level reading (Phase 1, the memo walk,
the population)" — i.e. a CORPUS problem, fixable by generating memo graphs as R27 generates
strings. Measured, no: the corpus extends, but the property stays **green** on the defective walk.
Over a fan of N memos each linking the same 8, the lines executed in `plan_memo_population` go
1592 / 2872 / 5432 at N = 20 / 40 / 80 — a ratio of ~1.9 against a 2.5 bound, GREEN — while the
entries the drain shifts go 14611 / 58421 / 233641, ratio exactly 4.0. Two independent reasons a
RATIO witness is blind: `pop(0)` is a C memmove, so no Python line runs and no line grows; and the
duplicate pending entries ARE Python lines but are linear in the input (N·K links is N·K of input),
so no doubling bound can flag them. **The limit that actually bit was the FIRST item on that list —
"work that runs no Python line" — not the subject-shaped one we named.**
So the corpus generator carried over and **the instrument changed**: the claim is EXACT rather than
asymptotic — *no path is taken off the queue twice* — over a corpus generated from the definition of
a memo family (every digraph on three memos, each also with one memo absent; 192 probes), of which
**77 are red against the pre-R28 walk**. Three nodes is a BOUND, not a budget (a second scheduling
needs two edges into one memo from two the root reaches); four nodes was verified green at 16,384
probes, so the stop is cost (6.53 s vs 0.054 s), not coverage. The O(1) half is a SOURCE claim, over
every source rather than the one site named, because nothing in this suite can measure a C memmove.
⇒ **The generalisation, and the correction to [[feedback_declared-blind-spots-are-where-the-next-finding-lands]]:
a declared limit is an inventory of INSTRUMENTS, not of SUBJECTS.** When the limit is the witness,
ask whether the claim can be made EXACT instead of asymptotic — an exact invariant needs no doubling
family and goes red at the minimum size.
#2 (P3) the module map omitted `plan_memo_selftest_growth.py`. ⚠ The reviewer's second clause is the
one that mattered — "include the module OR replace the hand-maintained inventory with a
non-duplicated source" — because two rounds earlier the same map was found missing three modules and
the response was to add them **and a prose warning that it drifts**. A sentence asking the author to
be careful, where a mechanism was needed; one round later it drifted again, exactly as the warning
predicted and did not prevent ([[feedback_prose-rules-cannot-fix-unexecuted-claims]]). The map is
kept — it records what each module OWNS, which `ls` cannot — and CHECKED in both directions over a
globbed population: completeness (red at the previous head) and existence (green there: the
discriminating half, and the one that catches a rename). What they cannot catch, stated: whether a
description is TRUE — a stale one, or one on the wrong module, reads like a fresh one.
⚠ **A figure of MINE that was wrong, caught while checking this round.** R27's entry above said
`scripts/trip-wires.sh` costs 26.2 / 27.2 s and reasoned "2.6× headroom". On a `git clone --local`
of the very commit that comment describes, with nothing else of this project running, it is
**11.83 / 11.83 / 11.85 s** — 2.2× less. The earlier figure was taken in a busy worktree and the
CONDITION WAS NOT STATED, which is exactly what made it unreproducible — and I introduced it in the
same commit where I corrected someone else's stale figure, for the same reason
([[feedback_convention-dependent-figures-are-argument]]). Both sites now carry the number AND its
condition, with the instruction to re-measure on a clean clone; headroom is ~5.9×, and the current
head measures 12.32 / 12.14 s the same way (R28's property costs ~0.4 s).
**596 controls, 322 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
+ 335 / 0 / 0; census `--worklist` byte-identical. Each of the four new mutants turns exactly its own
control red and leaves the other three green, verified individually.
⚠ `plan_memo_selftest_properties.py` is at **998** lines — the next addition crosses the touch-time
bound. The seam is already stated in its own docstring: controls that read SOURCE TEXT or AST and
never run the checker, versus those that run it over a GENERATED document (roughly 500/500).
⚠ **PR #510 Codex R29 (2026-09-08)** — three findings: two real (the tenth and eleventh non-linear
sites) and **one REJECTED on a false spec citation, falsified two independent ways**.
#3 asked for `hgroup` in the §4.6 type-6 tag list, citing 0.31.2. **0.31.2 contains zero occurrences
of `hgroup`** (case-insensitive, over `spec.txt`, sha256 `bfef4ddc…`); its list runs `header`, `hr`,
`html`, `iframe` and DOES contain `search`. ⚠ This read "the 0.31 change that added `search` and
removed `hgroup`" until R32, which **nothing in tree can support** — the vendored artefact is
0.31.2 only, with no prose and no prior version; measured, it also lacks `source`, which is what
the unchanged 62 actually points at. Diffed programmatically, the code's `_HTML_TAG_NAMES` matches the spec at **62 names, both
directions, no difference** (the only textual difference is `h[1-6]` for `h1`–`h6`). And the
behavioural half is false too, through the oracle that governs these memos: `text\n<hgroup>\n[x](absent.md)`
renders `<a href="absent.md">x</a>` on GitHub — a LINK — while the same input with `<header>` leaves
it literal, so `<hgroup>` starts no HTML block there either and the checker's rc 2 is right. Adding
it would be a conformance REGRESSION. ⚠ **Second false spec citation from this reviewer in four
rounds** (R26-1 claimed §6.3 caps paren nesting at 32), each costing a round of verification a gate
could answer in one command — so the list is now VENDORED as data with provenance
(`commonmark-0.31.2-html-block-tags.json`: source URL, version, section, the sha256 of the file it
was read from, and the derivation) beside the two example corpora, with a control holding the code's
alternation against it in both directions and requiring the alternation to actually stand inside the
compiled `_HTML_BLOCK`. The requested change now arrives at a red gate rather than an argument.
Provenance verified here end-to-end: the artefact's sha256 equals that of the `spec.txt` fetched
independently.
#2 (P2) the id scan backtracked across a decoration run (`"!" + "`"*n`: 0.016 / 0.063 / 0.259 s at
n = 1000 / 2000 / 4000, ×3.9 then ×4.1). ⚠ **The reviewer's remedy would not have fixed it** —
making `DECOR` atomic removes backtracking but is a constant factor, since `finditer` still restarts
the greedy run scan at every position inside the run. The anchor had to move: `_CORE` is the id
alternation alone (each kind opens with a fixed character) and decoration is walked OUTWARD from the
core, clamped by the previous token's end and by `endpos`, so the walks are disjoint. Three
invariants, because no one instrument reaches two of them: an exhaustive LANGUAGE agreement (44,376
probes against `decorated_id`'s own composition), exact COST equalities this suite can count (a run
with no id costs the same 3 source lines at 1,000 and 8,000 marks), and — the one that would have
caught the original — a SOURCE sweep, because the defect ran in the C `re` engine where every one of
the 596 controls was green on it. Predicate is `re`'s own parser: a leading unbounded repeat with a
sibling after it.
#1 (P2) `quote_content` was the marker TEST and the BUILD in one function; two of three callers
wanted only the test, so `quote_marker` is that test and the builds go 4,002 → 2,000 over 2,000
quotes. ⚠ **PARTLY DISCHARGED, and my diagnosis mis-attributed the cost** — see §8, which now
carries the residual: the Python lines are exactly linear, `gc.disable()` takes n = 64,000 from
0.681 s to 0.274 s (the collector walking N suspended frames), and what survives that is
architectural — Phase 1 hands a container's content to itself as a LIST OF STRINGS, so the
characters materialised are Σ(content length) over the nesting levels. The guarding control counts
BUILDS, never characters, because a control over the character total would assert the residual is
correct.
⚠ **A defect the round introduced and its own gate caught**: the R29-2 cost mutant made the
trip-wire ~60 s (32M traced line events). The work control now carries two ceilings — stops, not
claims, two orders above the correct scan — and catches the overflow so a red run stays red rather
than crashing.
**601 controls, 329 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
+ 335 / 0 / 0; census `--worklist` byte-identical; `PYTHONUTF8=0 LC_ALL=C` rc 0. Trip-wire re-measured
the stated way: head 13.68 / 14.36 / 14.42 s — ⚠ **and the BASE was re-measured on the same clone
minutes apart at 12.23 / 12.89 / 13.21 against the 11.8 s recorded for it**, so ~0.4 s of the
difference is the host, not this branch. Quote the pair, never the later number alone.
⚠ A prereq split landed first (the property controls part at the SUBJECT: controls that read the
checker AS WRITTEN and call nothing of it, versus controls whose question needs it RUN). ⚠ My brief
said that seam was already in the module's docstring; it was not — that docstring states the seam
BETWEEN modules, and its "what is here" paragraph named 8 of 15 controls, stale by seven since R26.
⚠ Next split candidate: `plan_memo_selftest_work.py` at 790 lines.
⚠ **PR #510 Codex R30 (2026-09-08)** — **THREE** findings, all real, and **one of them was mine**.
⚠ This entry said "two" until 2026-09-20, and the count was wrong for the reason the round
after it records: the thread fetch paged at 100 of 105, so #3 (the §6.4 image description,
fixed and controlled as **R30-3**) was disposed of unseen. The count is the record, so it is
corrected here rather than only in the round that found the omission.
#1 (P2) the `CLAUDE.md` paragraph copied the trip-wire benchmark history, the headroom calculation
and the timeout rationale **while explicitly naming the workflow comment canonical** — two live
copies of a number that moves. I wrote it ONE ROUND after correcting the same shape in the other
direction (there the pointer was refreshed and the canon left stale). The volatile half is removed:
what stands is stable policy plus a pointer, and the numbers live at the one declared home.
⚠ **The same defect was inside the SoT file itself and the finding did not reach it**: the
`timeout-minutes` line carried its OWN copy of the headroom multiple and had gone stale the moment
the block above it was re-measured. Removing that copy surfaced what no single round's figure shows
and it is now written beside the timeout: **~10× (R12) → 5.9× (R27) → 5.1× (R29) → 4.4× (R30)** —
every round's controls and mutants land in an ALWAYS-RUN job, so the headroom is a **budget, not a
constant**, and the point to act is before a runner cancels a green job (which is what the
`timeout-minutes: 2` this 5 replaced did, at 211 s).
#2 (P2) an unmatched decoration run made an id cell "blank": `**`, `*`, `` ` ``, ``` `` ```, `***`
all returned True from `is_blank_id_cell`, so a slice row keyed `**` with the marker and a non-empty
`Deps` returned **rc 0 with no findings** — in neither `ids` nor assertion (b). Correct is **rc 2**,
an unkeyed row being a schema miss. This is design re-gate 4's family at its ONE documented
exception, whose justification — "the decoration IS part of the id grammar there (`**9z**`), so the
two readings agree by being the same reading" — is sound for `**9z**` and false for `**`, where the
grammar pairs nothing and CommonMark renders the run literally. Right about WHY it exists, wrong
about HOW FAR it reaches.
⚠ **Both remedies the review offered fail as written**, which is why the shape was derived rather
than taken: "restrict to the explicitly supported blank spellings" is unavailable because the memos
really do write `**—**`, so a closed set of undecorated spellings either breaks that row or
enumerates the decorated ones unboundedly (the enumerated-exemption trap this file already names
twice); and "discard only decoration the grammar pairs" is wrong if the pairing authority is the ID
grammar, since ``` `` ``` peels to nothing under the decoration marks yet renders literally under
§6.1. So the exception was REMOVED rather than narrowed — the predicate reads what a reader sees.
⚠ **A third axis neither remedy named: WHICH rendering.** The disposed stream blanks a code span, so
`` `?` `` would come out empty and re-open the identical silent skip; only the READER's rendering
answers the question being asked. Own control, own mutant.
Every shape now agrees with a reader: `**` `*` `` ` `` ``` `` ``` `***` → rc 2; `—` `**—**` `` `—` ``
→ rc 0; `?` `` `?` `` → rc 2; `**7z**` still keys `7z` at rc 1. ⚠ **One REVERSAL no review asked
for, flagged rather than buried**: `&#8212;` as an id cell was rc 2 and is now rc 0 — correct, since
it renders `—`, but a verdict change in the opposite direction from the finding, so it carries its
own POSITIVE-NOVEL control. The agreement obligation against `bare_id` (which still reads raw, and
must, since it runs before the keep-set it declares) was MEASURED: over all 4,681 cells of length
0-4 from `{*, backtick, -, —, 9, z, space, &#8212;}` under both an empty and a populated keep-set,
**3,096 keyed readings, 0 that the new predicate also calls blank**.
**611 controls, 333 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
+ 335 / 0 / 0; census `--worklist` byte-identical — measured on the COMPLEMENT rather than the
output: 145 id cells over the population's 4 memos, **0 verdict changes**.
⚠ Three of my briefing premises were wrong and the delegate reported them: the next split candidate
is `plan_memo_selftest_mutants_inline.py` (942), not `plan_memo_selftest_work.py` (790, fifth); the
reviewer offered two shapes of which neither is a fix as stated; and the reviewer's §6.1 / §6.2
citations are sound (I had grouped this with the two false ones).
⚠ **PR #510 Codex R31 (2026-09-08/09, closed 2026-09-20)** — five findings, all real, all fixed
and all now proved. Four are R31's and the fifth is R30's third, which R30's own thread fetch
did not page far enough to see.
⚠⚠ **R31 was nearly recorded as the first DRY round, and was not.** The review came back with zero
inline threads. The three-channel scan then showed `hasNext=true` / `totalCount=105`: **the PR had
crossed 100 threads and my GraphQL query paged at 100**, so `unresolved = 0` meant "not fetched",
not "not there". Paging properly found **five live P2s** — four of R31's and **one of R30's, which
had therefore been disposed of with a third of its findings unseen**. The loop's terminator is two
dry rounds in a row, so this defect pointed straight at a false TERMINAL. Fixed in three places: the
memory note, and the GLOBAL skill `~/.claude/skills/external-converge/SKILL.md` — whose own snippet
had the pagination as a COMMENT beside a single-page call, so I followed the code and not the prose
— now pages in code and asserts `totalCount` against the number fetched (backed up first, and the
edited snippet was run: `totalCount=105 fetched=105`).
**The five, all reproduced by me before any fix, all verified fixed by me after** (`7dcbb663`, a
checkpoint commit whose message records that the controls were owed): (A) a kind phrase crossing a
resolved link inside a resolved IMAGE description — `![KIND [UNDETERMINED](x)](img.png)` renders the
alt text `KIND UNDETERMINED` but read as `KIND [UNDETERMINED`, so the row went terminal at rc 0 →
now rc 1; (B) a linked memo whose §5 schema header spells `#` as `&#35;` had its WHOLE TABLE
excluded, declarations and assertions with it, at rc 0 → now declared at rc 1 (the §1-forbidden
shape at the widest scope yet); (C) emphasis delimiters re-walked after clearing, ×3.9 → ×1.9 per
doubling; (D) the raw-line seed's token × span test, ×3.4 → ×1.7; (E) `classify` searching from
offset 0 per mention, ×3.5 → ×1.8. ⚠ **E is a regression I introduced at R24** (`96b3cbb4`):
removing the 40-character window fixed a vacuous lookbehind and replaced a bounded scan with an
unbounded one — the cost half of a correctness fix.
⚠ **Two delegates stalled on this round** (600 s no-progress, twice, on the controls). The fixes and
the `plan_memo_tables.py` → `plan_memo_stream.py` split (`e7b49ed4`) survive from them; the controls
are being written by hand.
⚠ **A pre-existing mutant SURVIVED and the generated property is what re-aimed it.** `RG4
openers_bottom` named `RG4_LINEAR`, which went green once C stopped re-walking dead delimiters. The
memo is NOT redundant — dropping it is still quadratic — but on a shape nobody had written down:
**six hand-written probes were tried and all six stayed linear**, while `R27_GROWTH`, whose corpus is
generated, reds at once and names it (a raw-HTML opener interleaved with an emphasis pair,
`plan_memo_emphasis.py:240`, 1.59× the bound). The control was moved to the one that discriminates.
A generated corpus catching the decay of a control written three rounds before it.
⚠ **And the generated corpus itself had a falsified exclusion.** Its docstring said blocked
arrangements were excluded because "the shapes that have cost this checker its linear contract are a
construct standing beside a second one over and over, not one run followed by another" — and C is
one run followed by another. Both arrangements are generated now (1,225 → **3,577 probes at the same
~3.6 s**). ⚠⚠ **But measured, that still does NOT catch C**, and the docstring says so rather than
implying otherwise: the limit is the ATOM VOCABULARY, not the arrangement — every atom is a single
character or a self-contained construct, so a delimiter atom repeated merges into one long run
(`***bbb`) instead of the N separate runs the shape needs. So C's control is a hand-written NESTED
probe added to `linear_emphasis_control`, verified GREEN on the fix (5,204 / 20,804 lines at 100 /
400 pairs, exactly 4× for 4×) and **RED on the pre-fix scan**, with its own mutant.
⚠ `_drop`'s docstring cited a `linear_emphasis_pairs_control` that **was never written** — the
delegate stalled before it. Corrected to name the control that exists.
**▶ DISCHARGED (2026-09-20, `dac4f5a7` + `fe3142e2`).** All five now carry controls and mutants.
⚠⚠ **The open question — does the generated growth property cover D and E? — was MEASURED, and the
answer is no, twice.** Each defect was put back into the checker on disk and the full self-test run:
**green both times**, the generated property included. D is unreachable from that property at all
(its corpus is a block-level text and never runs the always-run raw-content seed), and E moves none
of the four existing witnesses (both readings make one pattern application per mention and execute
the same source lines — the growth is inside the C `re` engine). So the two fixes had been shipped
with *nothing at all* watching them, which is what the untested "or the argument that the property
covers them" would have concluded the other way.
**A** (§6.4) turned out to be **three** clauses, not one, and each needed its own probe because each
is the only one its mutant moves (measured — the other two survive the other two probes): the
demoted link's `[` (the reviewer's shape, vendored Example 575), a demoted construct's TAIL, and the
image's **own `![`**, whose record is also what turns a phrase STRADDLING an image from silence into
a loud rc-2 miss. **B** (§2.5) got the positive, the raw-`#` constant, the `\#` spelling the reviewer
did not name, and a `&#36;` header that must still NOT bind — plus one mutant per clause (the
comparison, and the phase order that gives it a rendering to compare).
**E owed a third thing nobody had listed**: `plan_memo_roles.py` cites `licence_index_control` twice
as stating R31-4's reachability argument and **it was never written** — the second never-written
cited control this round (the first, `_drop`'s, was corrected one commit earlier). It exists now as
an ORACLE over `classify` itself against the whole preceding text, at all 22,574 positions of a
generated corpus, with a structural half that covers the NEXT phrase added to the tuple rather than
the ones the corpus spells. The keyword derivation had to stop raising on a phrase with no literal
prefix, so that case turns the control **red** instead of crashing it.
**E also needed a witness this suite did not have.** One pattern application per mention either way,
the same source lines, no list to count — what grows is the SPAN handed to the engine.
`_count_pattern_spans` is the fifth harness witness; the proxy delegates to the real pattern, so
every verdict under it is the production verdict.
⚠ **Touch-time split, and it is the rule's shape LATE**: `plan_memo_selftest_work.py` reached 991
lines on the controls commit, so `plan_memo_selftest_pipeline.py` carves the probes that write a
memo (991 → 581 + 486) in a commit of its own. CLAUDE.md wants the carve *before* the touching
edit; the controls went first. ⚠ Two seams, not one — growth is separated by POPULATION, work and
pipeline by PROBE — and the first draft of that docstring asserted the pipeline module is "the only
importer of `tempfile` among the work modules", which **measurement falsified** (the growth module
writes its digraph corpus to disk too). ⚠ One mutant row moved with its subject (R12-D → `PIPELINE`)
and the runner *said so* rather than degrading quietly — the R22 `STAGE_C` lesson holding.
**Gate**: 623 controls, 346 mutants / 0 survived / 0 crashed, `scripts/trip-wires.sh` rc 0, the #506
census worklist byte-identical across both commits (`cmp` clean, 815 lines, rc 0), and the split's
control NAME SET identical to the pre-split commit's. ⚠ `dac4f5a7`'s message reports 623 as **622**
— I read the figure from a run taken before `licence_index_control` was registered, my own edit
invalidating my own measurement; it cannot be amended here (the pre-commit guard refuses `--amend`)
and is corrected in `fe3142e2`'s message and here.
⚠ The disposition corrects the record: **R30 had three findings, not two**, and the two code
comments that labelled A `R31-1` now say `R30-3`.
⚠ **A correction the delegate got wrong, checked rather than accepted**: it reported the plan's
`51 seed(s)` figure as irreproducible. It is the tool's OWN summary line (`0 mechanical finding(s)
gate the exit status; 51 seed(s) and 714 reported naming site(s) do not`) — read it with `grep -a`,
since the log carries a NUL from a fixture and plain `grep` treats the file as binary and prints
nothing. That `grep` failure is real and worth knowing; the conclusion drawn from it was not.
⚠ **PR #510 Codex R32 (2026-09-20)** — **one** finding, real, plus a mid-loop design re-gate that
found more than the round did.
**R32 (P2, §6.1)**: a code span's line endings are converted to spaces BEFORE the one-space trim.
⚠ **The root is wider than the report**: the reviewer's probe was the shape where the trim
interacts, but measured against the vendored corpus the conversion was absent ALTOGETHER — the
reading was wrong on every multi-line code span, Examples 335 / 336 / 337, **all three already in a
corpus this suite runs on every self-test**. So the control is the spec's §6.1 list, not the probe
(`run_code_reading`: 19 spans over 22 examples agree; with the conversion removed 3 disagree and the
report names those examples). ⚠ That required **widening the conformance charter**, and the sentence
it widens is exactly where the finding landed: the module said the examples falsify "what Phase 1
CLAIMED — never a rendering", so the examples that settle this were present, in scope, and
deliberately not looked at, for four rounds.
**▶ The design re-gate (5 axes, fired by the Step-4 self-root-check, NOT at TERMINAL)** — the loop's
own rule says to fire it when the same finding shape recurs across ≥2 rounds, and it did:
⚠⚠ **Axis 3: my own measurement answered the wrong question.** The R31 ledger asked "does the
generated property *as written* cover D and E?", measured it honestly, and treated the answer as
settling "should a per-shape control be written?". The question that decides that is "can the
property be **parameterised** to cover them?" — and it can, measured: +3 atoms read off
`plan_memo_emphasis.DELIMS` red C; the **unchanged** corpus driven through `check()` instead of
`_read_block` reds D *and names a shape no round reported*; and E's witness has an enumerable
population of **39** module-level patterns, of which one is watched. The real blocker is COST
(~90 s for the full corpus through `check()`), and cost is nowhere written — what is written is a
reachability claim that measurement falsifies.
⚠ **Axis 3 also answered the reading-family question NO**: a call-site property is **not writable**,
with four counterexamples (`bare_id` must read raw for phase reasons, the seed reads raw on purpose,
`is_blank_id_cell` must read the reader's rendering, `Mention.text` must read the disposed stream).
Four correct answers, no local predicate. The writable surrogates are named instead.
⚠ **Axes 1/2/5: three of the suite's mechanical seam statements are FALSE** — `ast`, the harness's
module-set handles, and `build`/`run_on` (`build` has **seven** importers, worse than the re-gate
reported; I re-measured rather than take it). ⚠ The re-gate's *provenance* claim was wrong (blame
pointed at my reflow, not the clause's author) while its *fact* was right — the reason and the fact
are verified separately. ⚠ `licence_index_control`, written this round, passes a phrase with a
top-level alternation in **both** halves. ⚠ `plan_memo_selftest_pipeline.py`'s "`Memo(path)` runs
PHASE 1 ALONE" is refuted by the same delta's `Memo.__init__` (1 bind + 11 resolves, measured).
**The attribution class had no detector and five splits had run.** `symbol_attribution_control`
reads every written `module.symbol` against the AST. ⚠ **The population is the property, not the
symptom, and the numbers are the lesson**: a sweep for the symbols I knew had moved found **8**; a
dedicated re-gate agent sweeping the same class found **16**; the shape `module.symbol` found
**15**; every spelling the corpus actually uses found **28**; the control, whose corpus is every
source plus every plan memo, found **31 over 100 files**. The biggest contributor is the plan's
`mod.py::sym` coverage-map form, which no symbol-name grep reaches. ⚠ **Three of the thirty-one were
in the new control's own docstring** — it cannot tell a historical mention from a present-tense
claim, so narrative about a move now carries no locator while a pointer a reader follows must carry
a correct one. One mechanical replacement was **refused** rather than applied and fixed by hand.
**The CI timing SSoT was ~2× stale** — see §8; R30 made that block the single home and R31 then
added 12 controls and 17 mutants without re-measuring, which is the failure one-home does not
prevent (one home stops two numbers disagreeing; it does not make anybody re-measure).
⚠ And re-writing it I created the very defect its text warns about — the budget series ended up in
two places — caught before landing and collapsed back to one.
**Gate @ R32**: 625 controls / 347 mutants 0 survived 0 crashed / trip-wires rc 0 / #506 census
worklist byte-identical (`cmp` clean).
⚠ **PR #510 Codex R33 (2026-09-20)** — **three** findings, all real, all reproduced before any fix.
**Two land here; the third is Slice 2's and the plan already said so.**
⚠⚠ **PROCESS FAILURE FIRST**: R33 landed at 04:51:53Z, 13 min after the trigger, and **was not
picked up for 98 minutes** — the scheduled wakeup did not result in processing and I did not notice.
The user asked "R33 ちゃんとモニターしてますか?" and that is the only reason it was caught. A trigger
that is fired is not a round that is watched → [[feedback_every-triggered-pr-must-be-on-the-loop]].
**R33-1 (P2)**: `_APPOSITIVE` is `.search`ed and opens with `ROW_NOUN_ID`, which had **no left
boundary**, so the search could begin inside a longer word: `Subslice 9z — **UMBRELLA, …**` — and
even `xSlice 9z — …` — attributed the marker to `9z`, the containing row was read as a POINTER, and
the run **FABRICATED** an `UMBRELLA-MARK` mechanical failure. ⚠ The polarity is fabrication, not
silence: a reviewer reading the REPORT cannot catch this. `NOUN_ANCHOR` had carried the same
`BEFORE` boundary since R24; this composer did not.
**R33-2 (P2)**: the undetermined-kind phrase admitted em dash and hyphen and **not the en dash**,
while `_APPOSITIVE` and the id-cell blank set admitted all three. One rule, three spellings,
disagreeing — so `KIND – UNDETERMINED` read as terminal, assertion (b) never looked at the row's
`Deps`, and the run exited 0. Fixed as a **single home** (`plan_memo_ids.DASH`) that all three
readers compose, with `dash_spelling_sweep_control` enforcing it — the `id_spelling_sweep_control`
shape applied to the other character class these documents vary.
⚠ **Writing that control cost three corrections of my own, each caught by running it**: its
population was every source (it reported eight control NAMES — prose about a dash is not a reader
of one); its predicate was "a line with a dash and a bracket" (it reported three docstrings); and
its final form **missed the `\u2014` ESCAPE spelling**, so the mutant that re-injects exactly that
form SURVIVED until the escape was admitted. ⚠ A fourth: the widen-mutant `[—–-/]` is a reversed
RANGE and **crashed** instead of reddening, which exposed that `DASH_CLASS = "[" + DASH + "]"` is
correct only while the hyphen stays last — now guarded member-by-member in the same control.
**▶ R33-3 (P2) is REAL and is DEFERRED to Slice 2, which the plan already assigns it.** Every cell
of a schema row receives `row.self_id` and both passes discard every token equal to it *by value*,
so an umbrella row whose own Slice cell says `9z owns integration` produces no site while identical
prose outside the row is reported. Reproduced. ⚠ **But the fix is not the suppression alone**: with
it applied, the existing NEGATIVE control "a row naming itself in its own cell" goes RED, because
`9z mints its children here` is licensed by neither `LICENSE_BEFORE` (the phrase must precede) nor
`LICENSE_AFTER` (no `mints` arm) — it had been green only through the over-broad suppression. §2
**I-D** states exactly this outcome ("a row's mention of itself is classified like any other … the
existing NEGATIVE control **flips to POSITIVE** when the role is forbidden; the licensed `mints its
children` form stays NEGATIVE") and §4 row **#5** carries it as `self_id` exclusion hides a
forbidden self-attached role, I-D, priority 2, **"predicate (+ control flip)"** — and §4 #4–#6 are
**Slice 2's**, which gets its own plan-reviewed memo. Landing the suppression here without the
licensing predicate would ship a FABRICATED finding, which is the same polarity R33-1 fixes. The
change was written, measured, and reverted; Slice 2 inherits both halves.
**Gate @ R33**: 637 controls / 356 mutants 0 survived 0 crashed / trip-wires rc 0 / census worklist
byte-identical. ⚠ Four older mutant rows lost their substrings to the single-home rewrite and the
runner reported every one as "no longer applies" rather than passing them — the R22 `STAGE_C` rule
holding a fourth time.
⚠ **PR #510 Codex R34 (2026-09-20)** — **two** findings, both real, both fixed. ✅ **The new
monitoring worked**: trigger 06:43:38Z → assessed 06:57:45Z → processed immediately, against R33's
98-minute miss. The background poll is harness-tracked; `ScheduleWakeup` is a fallback only.
**R34-2 (P2)**: assertion (b) asked the emptiness of a `Deps` cell of the PROSE-SCANNING stream,
which blanks exactly the constructs a prose scanner must not read as prose — so a cell holding only
`` `b.rs` `` / a bare `.md` name / `[C1]` / an autolink read as EMPTY, an umbrella row carrying a
forbidden dependency emitted nothing, and the run exited 0. It takes the READER's rendering now —
the same distinction R30 drew for the id cell, at the OTHER of the two cells assertion (b) reads,
which makes this the reading family's fourth member. `is_empty` still decides by SHAPE, so the three
dash blanks stay blank; each masked family and each blank is its own control.
**R34-1 (P2)**: the file-name end test was "the suffix is not followed by an ASCII alphanumeric", so
a run that CONTINUES into more name still had a PREFIX masked — `9z+notes.md_tail owns it` masked
`9z+notes.md`, `sibling_path` follows no such file, and the declared id was hidden at rc 0.
⚠⚠ **The reviewer's remedy as stated would have regressed, and MEASUREMENT is what settled it.** The
file documents two incompatible rules — "the maximal run ending in FILE_SUFFIX" and "the end
boundary is ALNUM, so `x.mdの` still ends the token" — and requiring the suffix to terminate the run
bluntly kills the second. So the corpus was measured: across all **71** memos of this family, `.md`
is followed by a non-space character — and the figure is **pinned to a base sha**, because the way
it was first written could not survive its own landing. Over `docs/plans/*.md` + `CLAUDE.md` at
**`39f8f61c^`** (70 files): **86** occurrences — `)` 48 · `:` 19 · `'` 9 · `"` 4 · `;` 2 · `,` 1 ·
`.` 1 · `(` 1 · `\` 1 — and a **non-ASCII continuation occurs ZERO times**. The clause being
defended had no instances, so it is gone.
⚠⚠ **Two corrections to how this entry first stated it (R38 re-gate)**: the count was published as
**41**, a partial count of the same corpus, and it is **86**; and "zero non-ASCII" was true at that
base and is **FALSE at HEAD** — there is exactly one, `x.mdの`, and it is **this entry's own
example**. Writing the measurement down created its counterexample →
[[feedback_document-landing-invalidates-its-own-measurements]]. The conclusion is unchanged; the
way it is stated is.
⚠ **And the first fix was wrong in the other direction — two controls caught it.** Requiring the
suffix to end the run outright broke FRAGMENTS: `slice-9z-sib.md#…` stopped masking, and that is
precisely the one direction `file_token_resolver_agreement_control` forbids outright ("a string the
resolver WOULD follow to a file, which the lexer breaks into pieces and reads an id out of"). The
resolver was then asked directly and is the authority: it follows `#frag` / `?q=1` / `#` / `#a)b`
and rejects `_tail` / `.` / `)` / `の`. So a fragment or query tail is admitted, trailing punctuation
is admitted (where the two readers differ LEGITIMATELY — the resolver is handed a bounded
destination, this reader must find the boundary), and a run continuing into more name is refused.
⚠ R34-1 landed in a blind spot **declared hours earlier by this PR's own design re-gate**
(`invariants.py:476`, "the correspondence is ONE-directional"). Its docstring tolerates the resolver
refusing what the lexer tokenises — but that argument is about WHOLE-run tokens, and never
considered a masked PREFIX of a rejected run.
⚠ One older mutant became **EQUIVALENT** and was retired with its measurement: at most one end per
run can satisfy the new test, so "keep the first end per start instead of the last" edits a branch
no input reaches. Its control stays green; a row that can never go red again does not.
**Gate @ R34**: 648 controls / 360 mutants 0 survived 0 crashed / trip-wires rc 0 / census worklist
byte-identical. Three further mutant rows were retargeted where the single-home and end-test
rewrites moved their substrings, each reported by the runner rather than silently passing.
⚠ **PR #510 Codex R35 (2026-09-20)** — **one** finding, real, **and it was MINE**: R34-1's own fix
created it, one round later. Trigger 07:10:30Z → assessed 07:19:26Z → processed immediately.
**R35 (P2)**: R34-1 admitted a fragment/query tail as permission for the run to END, and went on
recording the span **at the suffix**. So `notes.md#9z owns it` masked `notes.md`, left `#9z`
standing, and the naming scan read the id out of the remainder — **the exact split
`file_token_resolver_agreement_control` forbids outright, re-created by the fix that cited the rule
against it**. ⚠ I had reasoned about this shape at R34-1, wrote down that `notes.md#9z` would leave
the id exposed, and moved on. The tail is part of the NAME; the span covers it now, less trailing
punctuation.
▶▶ **THE ROOT, and the reason this round produced a PROPERTY rather than a fourth per-shape
control.** Two consecutive rounds (R34-1, R35) landed on the same mechanism — the lexer's file-name
reading — and both landed inside the blind spot `file_token_resolver_agreement_control` DECLARES:
*"the correspondence is ONE-directional and only that direction is a claim."* Its corpus is bare
names, so a name with a TAIL was outside it entirely. `file_token_run_agreement_control` is the
other direction, stated over the tails the resolver strips: **a run the resolver FOLLOWS must leave
no declared id outside the lexer's spans.** Measured: with R34-1's defect re-injected it reds, and
with R35's defect re-injected it reds — **it would have caught both before the reviewer did**.
⚠ Writing it cost two corrections of my own. Stated as span EQUALITY ("the span is the whole run")
it was both too strong and a second implementation of the subject: a run whose tail ends in prose
punctuation (`notes.md#frag.`) is one the resolver follows while the lexer rightly stops before the
period, so equality reds on correct code — and encoding where it should stop would re-spell the very
rule under test. The claim is what the correspondence actually forbids: **no id left for the naming
scan**. And a mutant for "the tail's own trailing punctuation is not part of the span" SURVIVED and
was retired: a period is not an id, so no predicate here can see that clause — real, but cosmetic to
this suite, and a row that cannot go red is worse than none.
**Gate @ R35**: 652 controls / 361 mutants 0 survived 0 crashed / trip-wires rc 0 / census worklist
byte-identical. Three further rows were retargeted where the `_terminates_run` restructure (bool →
span end) moved their substrings.
⚠⚠ **PR #510 Codex R36 (2026-09-20) — THREE findings, all real; ONE fixed and TWO carved. Step-4
PAUSE declared.** Trigger 07:26:29Z → assessed 07:37:28Z → processed immediately.
**R36-2 (P2, FIXED, and it was mine — the THIRD round running on this reader)**: R34-1 gave the end
test a walk to the run boundary and `file_and_cite_spans` applies that test at EVERY suffix, so one
run holding N suffixes re-walked the same boundary N times — measured ×3.80 then ×4.02 per doubling.
The boundary is a fact of the text, not of the suffix asking. ⚠ **The first fix for it was ALSO
wrong and this suite's own cost control caught it**: precomputing every offset's run end is a second
full pass, linear but double the constant, and `linear_file_token_control` went red at its stated
ceiling. A single cached boundary serves every suffix of a run (they are met in increasing order),
so it is amortised O(1) with no extra pass — ×4.0 → ×2.1, 0.817 s → 0.003 s at n=4,000.
**R36-1 and R36-3 are carved, each to a home §8 now names** — see §8 for both, including the reverted
fix and the measurement that reverted it.
▶▶ **STEP-4 PAUSE, and BOTH triggers fired, not one.**
(1) *Same mechanism, three rounds*: R34-1, R35 and R36-2 are all the file-name reader's run
handling, and each round's fix produced the next round's finding.
(2) *Canary — a fix in round N breaks an invariant shipped in a prior round*: R36-3's fix turned
`id_scan_grammar_agreement_control` red. That is the literal wording of the trigger, and it fired
for the first time in this converge.
**The root-check's two written questions.** (a) *Abstraction coverage*: partly closed already —
R35's `file_token_run_agreement_control` subsumes the CORRECTNESS half of the run handling
(re-injecting R34-1's and R35's defects both red it). What has no property is the COST half: every
change to this reader needs a correctness guard AND a cost witness, and only the first is a
property, so R36-2 was caught by the reviewer rather than by the suite. (b) *Own-ideal test*: the
mechanism is NOT the anti-pattern — "ONE reading of 'a bare `.md` file name stands here'" is the
design and it is right. What violated the project's own ideal is HOW I changed it: three rounds of
edits to a reader bound by the id grammar, the sibling resolver, the disposition, a cost contract
and an agreement property — **≥3 intersecting invariant axes, which CLAUDE.md makes
plan-review-before-implementation by RULE**. I patched it inline three times instead.
**Gate @ R36**: 652 controls / **362** mutants 0 survived 0 crashed ⚠ (this line said **361** until the R38 re-gate — R35's count carried forward without re-counting, and the uncounted row is the one R36 itself added. The same shape as the `dac4f5a7` 622/623 slip, in a PR whose own recurring note is that a figure goes stale under its author's next edit) / trip-wires rc 0 / census worklist
byte-identical.
✅ **PR #510 Codex R37 + R38 (2026-09-20) — TWO DRY ROUNDS, TERMINAL.**
Both confirmed across all THREE channels with the denominator stated, which is the R31 lesson
applied: threads **115 / 115** fetched (`totalCount` matched) with **0 new** reviewer threads after
each trigger; **36** bot reviews scanned, **0** finding items on head this round; the dry verdict on
head `cd1f973c` after each trigger; `on_head: true`, `errors_since_trigger: 0`.
**Trajectory (new-real per round)**: R31 5 · R32 1 · R33 3 · R34 2 · R35 1 · R36 3 · **R37 0** ·
**R38 0**. FP across the whole loop: **0**.
**THE STEP-4 ATTESTATION, written out because that is what makes it fire:**
(1) *Is every real finding fully fixed, none deferred as "edge / nice-to-have / diminishing
returns"?* — **No, three are open, and none is an "edge" defer.** Each has a concrete home and a
ratified reason: **R33-3** → Slice 2, whose §2 I-D states the outcome *including the control flip*
and whose §4 row #5 owns it at priority 2 (landing the suppression alone ships a FABRICATED finding
— measured, an existing NEGATIVE control goes red); **R36-1** → the §8 entry whose own stated
trigger ("the next finding on this seam") it fired, same mechanism and same remedy; **R36-3** → §8,
after a fix was written, measured and REVERTED for breaking a shipped invariant, with the defect
located in the GRAMMAR and therefore plan-review-before-implementation **by rule**.
(2) *Is the stop real-gap exhaustion, not fatigue?* — **Yes**: two consecutive rounds of zero new
real findings, each with its denominator, on an unchanged head. Round count and session length were
not inputs.
⚠ **Merge is NOT proposed from here.** The TERMINAL fix-delta design re-gate (overlay Step 5.5) runs
over `bc7cb013..cd1f973c` first — this delta touches `.claude/tools/**`, which the skip table says
can never honestly show yield 0 — and merge approval is the user's in any case.
⚠⚠ **PR #510 TERMINAL design re-gate (2026-09-20, 5 axes over `bc7cb013..cd1f973c`)** — run because
the overlay makes it the pre-merge gate and this delta touches `.claude/tools/**`, which the skip
table says can never honestly show yield 0. It found **more than R36 did**, and the cluster is one
shape: *this delta built detectors for SYMBOL-level staleness and left the MODULE-DIRECTION and
SYMBOL-EXISTENCE halves open — and the delta's own new prose, its own new constant and its own plan
edits each landed in one of those two halves.*
**⚠ The worst was self-inflicted and mechanical**: the R32 attribution sweep **rewrote two
HISTORICAL locators into falsehoods** — §3's dated pointers for `Memo._blocks` and for `_glued`,
each of which names the module its symbol has SINCE LEFT, were rewritten to the module it moved
TO, so "the pre-split name" named the current one: vacuous as well as false. Both restored. ⚠ And
the R32 record said "one replacement was REFUSED rather than applied — the count==1 guard doing its
job"; **two others were applied and the guard cannot see the difference, because a count is not a
reading.** The control now reads the corpus's own dated-locator convention (`pre-… name \`X\``).
**⚠ A false seam I authored in the same commit that built the seam control**: the runner's new
import-direction paragraph said "the controls module imports the property moduleS" — it imports
`properties` only. `import_seam_control` cannot reach it: that control checks SYMBOL-level seams
("the only importer of X") and this is a MODULE-DIRECTION claim. The tree states **fourteen** of
those; the re-gate tested all fourteen and the one this PR authored was the only false one.
**⚠ The reading family's THIRD site**, which the dry-run predicted and R34-2 left behind:
`assertion_cd_seed` asked `Deps` emptiness of the prose-scanning stream, so a cell holding only a
masked construct read as EMPTY and the seed printed a finding **that contradicted itself** ("…
while its Deps cell is '\`b.rs\`'") while never asking its real question for those rows. Fixed, with
four controls and two mutants. *Fixing one site of an obligation is not fixing the obligation.*
**⚠ `_TRAILING` was narrowed against the very measurement cited for it** — twice in one edit. It
omitted `;` (which the corpus has) while claiming to be the measured set, and then the correction
dropped `"` (which the corpus also has, in raw-HTML attributes), turning a control red. The GFM
§6.9 attribution was wrong as well and uncheckable in-tree. It is local policy now, derived from
the measurement, and the measurement is pinned to `39f8f61c^`.
**⚠ My R34-1 measurement was invalidated BY MY WRITING IT DOWN**: "a non-ASCII continuation occurs
ZERO times" was true at the base and is false at HEAD — the one occurrence is *this ledger's own
example*, `x.mdの`. The published count 41 was also a partial count of the same corpus; it is 86.
Both corrected and pinned → [[feedback_document-landing-invalidates-its-own-measurements]].
**⚠ Three carves audited adversarially and all three HELD** — the re-gate executed the claims
rather than reading them: R33-3's home pre-existed at base and the control-flip claim is true;
R36-3's grammar claim is correct (`decorated_id`'s own composition gives `C` no left decoration, so
the defect is in the GRAMMAR); and R34-1's "equivalent mutant" retirement is genuine (0 differences
over 407,290 probes). **R36-1's framing held but its HOME did not** — it re-deferred an
already-fired trigger into a ledger that named it nowhere, while citing the memo that says a
boundary wants re-drawing rather than another entry. It is **Slice 3** now, with scope, owner, an
EVENT trigger and a date; R36-3's circular trigger ("do the plan-review") became Slice 2's review.
**⚠ Two more of my own stale figures**: the R36 gate line said 361 mutants (362 at that head — R35's
count carried forward), and the `ci.yml` pair was labelled "CURRENT, at R32" after four rounds added
27 controls and 14 mutants. Re-measured at R38: **no material drift**, so only the label was stale.
**⚠ A retirement reason that was FALSE when written**: the R35 "cosmetic" mutant was retired saying
"a period is not an id", while `_TRAILING` then held `*` `_` `~`, which are decoration characters.
The retirement stands; the reason is now true only because a later edit narrowed the set.
**Gate after the re-gate**: 657 controls / 364 mutants 0 survived 0 crashed / trip-wires rc 0 /
census worklist byte-identical. ⚠ Four further mutant rows were retargeted where these fixes moved
their substrings — one needed a wider anchor because the old one now matched twice.
▶▶▶▶ **THE NEXT SESSION STARTS HERE (2026-09-21 evening — the three owed decisions are TAKEN and
the round is ready to trigger).**
**STATE**: everything below (items 0–3, the Axis 5 re-run, the TERMINAL fix-delta re-gate, rounds
R42 … R42-10) was already done. THIS session took the three decisions the loop was stopped on, and
two of them turned out to have code in them.

✅ **1. HOW TO PROCEED — CONTINUE, WITHOUT `/elidex-plan-review`, UNDER A WRITTEN PER-FIX CHECK**
(user, 2026-09-21). The question was whether a surface on which three of my changes had turned a
shipped control or the census red, and on which three controls I wrote had a subject that was not
their claim, wants a plan-review before it is touched again. The answer is no — and the two
recurrences are addressed directly instead. **THE RULE, which applies to every fix from here:
before the commit, write ONE LINE EACH —**
**(a) does this control's SUBJECT match its CLAIM?** (what would make it green for a reason that
has nothing to do with the property; name the discriminating partner that rules that out), and
**(b) WHICH ARM was refuted?** (name the arm, and check the change is not narrowed to the case that
happened to be SHOWN — `memory/feedback_universal-claims-need-the-complement-measured.md`).
⚠ Both fired on their first use, which is the argument for the rule and not for the review: (a)
produced the baseline control beside the §6.6 one (a finding count of 0 is what ANY silence gives),
and (b) is what kept the §6.6 change scoped to *inside a resolved image description* rather than to
raw HTML generally — the bare-prose R17 reading is untouched and has its own negative beside it.
⚠ And the per-fix check did NOT catch everything: the R42-10 gate turned a shipped NEGATIVE red,
and only running it did. The rule narrows the class; the mutation proof is still what closes it.

✅ **2. THE DEFER CAP — OPTION (d), ACCEPTED WITH A WRITTEN RATIONALE** (user, 2026-09-21). §8's cap
block now states why (a) fold, (b) narrow and (c) split are each unavailable in this PR, entry by
entry, and gives the count as an ENUMERATION rather than as a figure. **Ten own / one pre-existing.**
⚠ Two entries left the list and neither is a tally edit — the test being that under (d) removing
them buys nothing: the spec-prose entry is SETTLED (below), and the standalone quote-half entry was
a DUPLICATE of Slice 3, whose own text says the two halves are one carve and which was written at
R38 to replace it. The R38 re-slice had been applied to the prose and not to the list.

✅ **3. THE TWO SPEC-PROSE READINGS — SETTLED BY EXECUTION, and they went opposite ways.** `brew
install cmark` and `npm install commonmark@0.31.2` both succeeded here, at the vendored corpus's
own version, so the "no session could execute a reference implementation" premise is retired.
· **§6.3 vs the Appendix (`[x <http://a>](absent.md)`) — NOT A DEFECT, the Appendix wins.** Both
implementations render `<a href="absent.md">x <a href="http://a">http://a</a></a>`: the outer link
IS a link and the autolink nests inside it. The checker implements the Appendix verbatim and was
right; the prose reading loses. The link-in-link control (`[x [y](b)](a)` → `[x <a href="b">y</a>](a)`)
reproduces beside it, so the deactivation rule itself is unaffected.
· **§6.6 raw HTML inside a resolved image description — A REAL DEFECT, and FIXED.**
`![UMBRELLA, not a <span>terminal unit](img.png)` gives
`alt="UMBRELLA, not a &lt;span&gt;terminal unit"`. The span contributes its own SOURCE TEXT to the
alt where in ordinary prose it renders nothing — an `html_inline` node's plain string content is
its source.
⚠ **THE FIRST STATEMENT OF THE REASON HERE WAS WRONG AND THE FACT WAS NOT.** It read *"cmark
ESCAPING the angle brackets is what decides it"*, which reads as a cross-implementation property
and is not one: commonmark.js 0.31.2 emits `alt="UMBRELLA, not a <span>terminal unit"` — the same
characters, UNESCAPED. Escaping is cmark's serializer choice about an attribute value. What BOTH
implementations agree on, and what the fix rests on, is that the span's characters are IN the alt
rather than dropped. The reason and the fact had to be verified separately
(`memory/feedback_ao-name-not-section-number-in-briefs.md` 追補 6). Masked as markup the span
JOINED its two sides and the run exited **1** on an ownership claim nobody made: the one
FABRICATED finding on this surface, which is exactly why it was never fixed on the prose alone.
⚠ The fix is the same mechanism as the R42-5 code-span and autolink demotions — `dem_html` beside
`dem_code` / `dem_auto`, one range applied once, so the R23 linear contract holds by construction —
plus the seed following the reading (a demoted span hides nothing, the rule an autolink already
had). ⚠ **No line-ending substitution, and that is MEASURED**: §6.1 normalises a code span's
endings to spaces (`` ![a `b\nc` d](i.png) `` → `a b c d`) and §6.6 does not
(`![a <span\nx>b](i.png)` keeps it). ⚠ **The population was enumerated wrongly on the first pass** —
a `grep` for `lx.html` that EXCLUDED the self-test files, so the conformance control and a mutant
row were missed and 21 spec examples crashed; the whole-population grep found them in one line
(`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`).
· **§6.4 (is an image's alt the document's text?)** stays the FP R42-10 measured it to be.

✅ **4. THE LINKED-MEMO UNBOUND TABLE — MEASURED, AND THE CENSUS-CLAIM HALF IS FIXED.** §8 carried
two unmeasured candidates and both were measured on the real population, which is 141 plan memos /
511 tables / **507 unbound** across this worktree and `elidex-wt-vmp4plan`.
· *kind PHRASE in any cell* — **0** findings over that corpus, while the phrases themselves occur
**77** times in it, so the zero is a silence and not an empty population; and it fires on the
reproduced defect. **Shipped**, reading the RENDERED cell as the census reads a declaring field.
· *header NEAR-MISS* — 0 over the corpus too, and REFUSED for a reason no corpus count shows: it
fires on a renamed header whose table **declares nothing**. A two-fixture pair discriminates them,
and it is a NEGATIVE control now with a mutant that re-injects the predicate.
· *the id shape* stays refused (**151** tables over the same corpus), also pinned by a control and
a mutant.
⚠⚠ **AND THE GATE IMMEDIATELY FOUND ONE INSIDE THIS SUITE'S OWN FIXTURES.** R31-1's `$`-header
NEGATIVE is a linked memo whose table binds to nothing **while carrying an umbrella row** — the
identical class, reached independently, with a different header. Its site-count measure also
asserts `rc != 2`, and that implicit half had been blessing the silence for eleven rounds. ⚠ The
fixture and the SUBJECT are untouched and only the measure moved (to the gate that names the
unbound claim, which a bound table would not produce): replacing the marker to quiet the gate would
have taken the discrimination with it, because without an umbrella there is no no-owner row and a
loosened header match produces no site either
(`memory/feedback_control-rewritten-to-bless-the-defect.md`).
**What remains in §8** is the residual only: an unbound table making NO kind claim.
**GATE AFTER ALL FOUR**: **693 controls / 392 mutants 0 survived 0 crashed** / trip-wires rc 0 /
#506 census `--worklist` byte-identical (815 lines, rc 0) / this memo at the four-FATAL floor.
⚠ Per the Axis 5 rule, the self-test is re-run AFTER this memo edit, never only before it.
⚠⚠ **R43 AND R44 WERE BOTH DRY — AND THE TERMINAL ATTESTATION FOUND SIX THINGS THE TWO DRY ROUNDS
DID NOT.** The reviewer returned `Didn't find any major issues` on `fc0cc556` twice, each with its
denominator measured (threads 132 of totalCount 132 over two pages, 0 new since the trigger; 47
bot reviews, 0 P-badge bodies; the one comment since the trigger being the verdict itself). By the
loop's own rule that is TERMINAL. **It was not**, and the reason is the rule
`memory/feedback_attestation-by-enumeration-not-assertion.md` states: two dry rounds are not an
attestation. Three fresh adversarial agents were given the enumeration — every finding of the
whole loop with one re-runnable command each, a CONCEPT sweep for siblings of this session's
fixes, and a falsification pass over this session's own written claims.
· 🔴 **A SIBLING OF MY OWN FIX, LEFT BEHIND IN THE SAME FUNCTION.** `inline_claim` — the falsifier
every spec-example control rests on — got its `.html` demoted filter from the §6.6 fix and kept
counting `len(lx.code)` and `len(lx.autolinks)` whole, which R42-5a had demoted one round earlier.
`![a `b` c](img.png)` reported *"the html emits 0 `<code>`, Phase 2 claims 1 code span(s)"*
against cmark's own `alt="a b c"`: a FABRICATED falsifier failure, in the direction that also
CANCELS a real one through the `left` conservation count. The corpus cannot reach it, and ⚠ the figure first
written here was wrong in a way worth keeping: *"22 of 335 inline examples render an `<img>`"* is
the IMAGES SECTION count, a different predicate — **23** render one (19 Images, 3 Links, 1
Emphasis). The SAFETY conclusion is unchanged and is measured by over-approximation so no regex
can drop a row: zero of the 23 carry a backtick anywhere, and the two carrying a `<` are Example
580 (`![foo](<url>)`, in the DESTINATION) and Example 475 (raw HTML with no description). So the
sentence the §6.6 fix wrote for `.html` — "the filter removes nothing this corpus measures" — was
true of the siblings too, and hid them. Fixed at all four sites, with a control that runs each
§3.0b family through `inline_claim` against the spec's own html plus a BARE twin and a
deliberately-wrong body, and four mutants. ⚠ **The module had never been mutable**: it was not in
the mutant module set at all, so no row could name the falsifier in either direction. It is
`CONFORMANCE` now, and `patched_module` grew the one generalisation that needed — a self-test LEAF
owns no `registry()` and is reached by the owning module's call-time import, so it is installed
under its real name for the row.
· 🔴 **§6.1 STEP ONE SHIPPED UNPINNED.** Both R42-6 mutants patch the TRIM; replacing the
line-ending substitution with a no-op left all 695 controls green while a real naming site
vanished (`![Slot #11-zz-alph`<NL>a ` owns it](img.png)`: one site becomes none, the slug split in
silence). The fix's own commit called this "half of §6.1" and the unwatched half was that same
half. Pinned, with the space-padded twin as the arm it must not move.
· 🔴 **TWO POPULATION-SCOPE LOOPS WERE CORRECT AND UNWATCHED.** `_unkeyed` and the table-miss loop
both scope to the whole population; `self.memos[:1]` — the identical edit five other loops already
have a mutant for — survived on both. Two fixtures, measured to discriminate SEPARATELY (scoping
one leaves the other's miss reported), and two mutants. `memory/feedback_derived-populations-
shrink-in-silence.md`.
· 🔴 **THREE CLAIMS I WROTE THIS SESSION WERE FALSE.** (a) *"cmark ESCAPING the angle brackets is
what decides it"* — commonmark.js 0.31.2 emits the same characters UNESCAPED; escaping is cmark's
serializer, not a CommonMark property. The FACT survives and the REASON did not, and they had to
be checked separately. (b) The id-shape figure **151** is the count for *every body row*, while
the mutant re-injecting that predicate was written for *any row including the header* (333 / more)
— the number and the thing it justified were different predicates. Both say every body row now.
(c) *"the phrases occur 77 times"* — **76**, and the memo edit in the very commit asserting it is
what removed the 77th. ⚠ **And it moved AGAIN, to 78** — because that "removal" took the number
out of the memo and left it standing in `Population._unbound_claims`' docstring, so the same
figure was re-falsified a round later by edits that had nothing to do with it. It is gone from
BOTH now. This document is in its own corpus, so
the figure moves whenever this paragraph is edited, which is the failure
`memory/feedback_document-landing-invalidates-its-own-measurements.md` names and which was cited
three lines below the number.
· 🔴 **A REVIEW FINDING WITH NO DISPOSITION ANYWHERE** — the three-hyphen delimiter ask. Settled
the way this PR settled its other two spec questions, by EXECUTION: `cmark-gfm 0.29.0.gfm.13`
makes a `<table>` from `|-|`, `|:-|`, `|-:|` and `|:-:|` alike, so the checker is right and the
finding is an FP. ⚠ `is_separator` had zero self-test references and no fixture used a short
delimiter, so the suite could not have told the readings apart. Four controls and a mutant now.
· **§8 HYGIENE, from the same audit**: *"exactly nine attributions, all in §3's coverage map"* was
false in both halves — nine PAIRS over fourteen SITES, and **three of those sites are the sentence
making the claim** (`plan_memo_ids._TOKEN` occurs in this memo at that line and nowhere else); the
hand-written-table entry's **59 / 38** are WITHDRAWN, since the predicate as worded yields 97 / 84
and no variant reaches them; the wire entry's "six rows name it" is withdrawn for having no stated
command; two entries carrying neither a date nor a trigger-only declaration now say which they
are; and the `self_id` deferral — real, reproduced, homed in §4 row #5 whose table has no trigger
column — is listed in §8 as **(10)**, pre-existing with its grounding (`aee896dd`, the ratified
plan, against the branch's first commit `5e9439b4`), so §8 is a complete index of this PR's
deferrals. The enumeration is **twelve entries, ten own and two pre-existing**; the CAP is
unchanged, which is what made writing (10) down cheap and leaving it out dishonest.
**GATE AFTER R45**: **701 controls / 400 mutants 0 survived 0 crashed** / trip-wires rc 0 / #506
census `--worklist` byte-identical / this memo at the four-FATAL floor / largest `plan*.py` 983.
⚠ **What the round says about the loop**: the reviewer went dry twice on a head carrying a
fabricated falsifier, an unpinned spec step, two unwatched scope loops and three false claims of
mine. A dry round bounds what the REVIEWER found, never what is there
— which is exactly why the attestation is an enumeration and not a sentence.
**▶ NEXT: the head has MOVED, so the two dry rounds are spent. Re-trigger `/external-converge 510`
on the new head, arm a harness-tracked background poll in the same turn with a negative control
fired at the judgement first (`memory/feedback_a-monitor-needs-a-negative-control.md`), and do NOT
read R43/R44 as covering this delta. Merge is NOT to be proposed.**

**▶ WHAT THE 2026-09-20 / 2026-09-21 HANDOFFS SAID, kept because their items are the record of
what was done:**
0. ✅ **DISCHARGED (2026-09-20, the next session's first act).** The split is taken, recorded in
   §7's Slice 0 with the seam it actually used (the handoff's was refined — it would have split
   `dash_spelling_sweep_control` from the sibling its own docstring names), and the header's
   invariant holds again: `plan_memo_selftest_properties.py` 1,110 → 751 + `plan_memo_selftest_
   records.py` 420, largest `plan*.py` now 974. Control NAME SET identical, census `--worklist`
   byte-identical, 657 / 364 0-0, trip-wires rc 0. **The original CRIT, kept for the record:**
   🔴 **THE PLAN'S OWN INVARIANT IS FALSE AT THE MERGE CANDIDATE, and this PR broke it.**
   The header states, with the command that decides it, that *"every `.claude/tools/plan*.py` is
   under the 1000-line touch-time bound"*. Measured: `plan_memo_selftest_properties.py` was **738**
   at `bc7cb013` and is **1110** now — I added 372 lines of property controls across this session
   and took no touch-time split, which CLAUDE.md requires AT TOUCH TIME and which §7 records seven
   times for this PR. Three more files are in the 900s (`_mutants_inline.py` 974, `plan_memo_lexer.py`
   961, `_mutants_r26.py` 938). ⚠ Those four figures are pinned to `eb1bfefd`, the head that
   measured them, and they have moved twice since — which is why no line count is transcribed into
   this document any more: `line_bound_control` measures the whole set every run and prints the
   largest file, so the number a reader would use to pick the next split target is taken NOW
   (`memory/feedback_document-landing-invalidates-its-own-measurements.md`).
   ⚠ **The seam is already identified and is a real cohesion seam, not a line count**: everything
   this session added to that file sweeps the tree for CROSS-FILE CONSISTENCY — `symbol_attribution_control`,
   `import_seam_control`, `dash_spelling_sweep_control`, with `_ATTRIB_SPELLINGS`, `_IMPORT_SEAMS`,
   `_DATED_LOCATOR`, `_defining_module`, `_attribution_corpus`, `_prose_of` — against the original
   population, which sweeps the checker AS WRITTEN (its source text, AST, code objects, docstrings).
   Carve the first group out, keep `_swept_sources` as the shared population, re-point the `PROPERTIES`
   mutant rows, and confirm the control NAME SET is unchanged.
   ⚠ **It was not done there, and that was a deliberate stop, not an oversight**: that session ran
   out of context, and a split botched at the end is worse than a split recorded as owed. It was
   item 0 because it is the one thing that should not reach a merge.
1. ⚠ **Axis 5 of the re-gate never finished** (project-context: stale measured figures, blind-spot
   classification, defer/slot hygiene, touch-time line counts). The other four axes found **four**
   stale figures of mine, so treat this as owed, not optional. Re-run it alone over
   `bc7cb013..e82d3324` — do NOT re-run the other four.
2. ⚠ **TERMINAL is STALE: the head moved.** R37/R38 were dry on `cd1f973c`; `e82d3324` has never
   been reviewed. Trigger `@codex review` and get the dry round back before any merge talk. The
   merge-head guard hook enforces this mechanically.
3. Then, and only then, surface the merge proposal — **merge approval is the user's**, and the
   three open carves below must be named in it.
**▶ THE THREE OPEN FINDINGS — all real, all reproduced, none an "edge" defer:**
· **R33-3** (thread `PRRT_kwDORYj7cc6kHEkI`, deliberately left OPEN) → **Slice 2**. §2 I-D names the
outcome *including the control flip*; §4 row #5 owns it. Verified at the re-gate: the home
pre-existed at base and the flip claim is true — the suppression alone turns an existing NEGATIVE
control red, i.e. it would ship a FABRICATED finding.
· **R36-1** → **Slice 3** (§8), which this session created because the first filing re-deferred an
already-fired trigger into a home no ledger named. Scope, owner, EVENT trigger and re-eval date are
all there now.
· **R36-3** → **Slice 2's plan-review** (§8). The defect is in the GRAMMAR (`decorated_id` itself
gives `C` no left decoration in `prefix**C**`, verified by executing it), which makes it edge-dense
and plan-review-first BY RULE.
✅ **AXIS 5 RE-RUN ALONE over `bc7cb013..94124588`, 2026-09-20 — 1 CRIT / 6 IMP / 4 MIN / 3 FP, all
processed.** The CRIT was the split commit's OWN: the §7 record spelled a dead attribution bare in
this memo (the table's pre-split name `plan_memo_selftest_properties._IMPORT_SEAMS`), `_attribution_corpus` reads
`<root>/docs/plans/*.md`, and the always-run wire went rc 1 on the commit whose record asserts rc 0.
Root cause was ORDER, not spelling — the gate was run, the memo was edited afterwards, and a memo
edit is a CORPUS edit. **The rule this PR now runs under: re-run the self-test AFTER the memo edit,
never only before it.** Discharged, with the three FPs recorded as FP (the two ship-time slot
registrations, the 717-vs-715 site figures, the 1000-line invariant itself).
What each finding became: the **"39 module-level `re.Pattern` globals"** denominator is REMOVED (it
reproduces under no enumeration — 34 / 41 / 47 / 60 — and the carve needs the population's
definition and its command, not a number); the enumerated-table **blind-spot class** now has a §8
entry with scope / owner / EVENT trigger / re-eval, and is measured rather than predicted (the
split had to widen `_IMPORT_SEAMS`, landing inside the territory declared undetected); the
`stale-claim-detector` lane's second wire is now an explicit §1 merge obligation on `REQUIRED_WIRES`
(append, never `--ours`) and the `ci.yml` budget block states that its figures cover THIS wire alone
— ⚠ Axis 5 refuted the premise that the collision is a BUDGET problem: 26 s + 6 s against a
10-minute timeout is a merge hazard on the inventory, not a cost one; the **three parallel
plan-memo checkers** now cross-reference, with the boundary written as three questions (this =
the row-kind census, `claim-gate-plan-check.py` = is a number still true, `plan-xcheck.py` = do two
layout memos agree) and a trigger for revisiting the collapse; the NUL byte is fixed at the
CHANNEL rather than in the one control whose name legitimately carries one — `printable()` escapes
every C0 character and DEL, with a control over all 33 and two mutants (drop the arm / escape
everything). ⚠ Still OPEN and routed to the user: the **defer cap** (10 own against ≤3, classified
in §8 without merging or deleting an entry to move the number).

✅ **THEN THE TERMINAL FIX-DELTA DESIGN RE-GATE RAN, and it was not clean** (2026-09-20, axes 3 / 4
/ 5 over `cd1f973c..7121476e`; axes 1 and 2 argued to yield 0 by an EMPTY POPULATION — `git diff
--name-only cd1f973c..HEAD -- crates/` is 0 files, so the Layering and ECS axes have no subject,
which is a different argument from "it is only docs"
(`memory/feedback_terminal-gate-not-optional-on-doc-only-delta.md`)). **1 CRIT / 9 IMP / 6 MIN /
14 FP.** Codex had gone dry twice on `7121476e` by then; every one of these came from the design
re-gate, not from the external reviewer.
· **CRIT — the NUL fix's control had the WRONG SUBJECT, and two axes reached it by different
routes.** `printable_output_control` iterates all 33 control characters through `printable()`, and
both mutants edit `printable()`'s own expression — so all three prove the FUNCTION. Axis 5 proved
the gap by EXECUTING it: strip every `printable(` call site from the runners, leave the function
intact, and the suite stays green while a raw NUL returns to the log. And the docstring's claim
("it lives at the one place every line goes through") was false when written: two print channels,
fourteen emit sites, three wrapped. Fixed at the CHANNEL — every emit site wrapped with no
exemption for "this one only formats numbers", plus `report_channel_control`, whose population is
the EMIT SITES read off the AST, plus two mutants that unwrap one. Verified by reproducing Axis 5's
attack byte for byte: **rc 1** now, where it was rc 0 and green.
· **The 1000-line invariant is a CONTROL now, not a sentence** — `line_bound_control`, six lines
over the population `_swept_sources()` already globbed. Both axes asked for it, and the reason is
the entry it replaces: the §8 touch-time pre-commitment asserted "the touch this PR gave
`_mutants_r26.py` was a re-point of two rows, not growth" **in the commit that appended two mutant
rows to it** (`b8324d06`, +22 lines) — its own trigger had already fired when it was written. So
the split is taken (`_mutants_r30.py`, the review-round seam for the fifth time; the ONE name
crossing the boundary was measured and moved, leaving zero) and the entry is GONE. Its mutant grows
the lexer's docstring by 50 line endings, 961 → 1,011, because the subject must be a real line
count and not the control's own threshold.
· **Axis 4** — I re-introduced a citation this tree had WITHDRAWN: `plan_memo_memo.py` records that
the subsection number for CommonMark's insecure-character rule is not determinable here, and I
wrote "the §2.1 control". Three pre-existing sibling sites carried it too; the population was
classified mechanically (NUL rule vs line endings) and only the three NUL ones moved to bare §2,
the four line-ending ones being correct. Two memo citations repaired (the blank-line definition is
§2.1, not §4.9; `` <a b="c"d> `` is this suite's fixture and Example 622 is
`` <a href='bar'title=title> ``, read off the vendored corpus), and the GFM §6.9 premise is marked
UNVERIFIED because the same paragraph says nothing in this tree can check a GFM citation.
· **The "~29 s" FP verdict is RETRACTED** — see §8. Three clean-clone measurements at nearby heads
gave 24.9 / 26.1 / 28.5 for one command, so the absolute is not reproducible to better than ~20%
here and the derivation rests on the ratio.
· **Two §8 entries were mis-scoped and both are corrected**: the hand-written-table class was
scoped by the SYMPTOM VOCABULARY (it selected the tables that had already declared themselves);
and the GFM row-splitter entry's `(pre-existing)` grounding was measurably false
(`git grep -c split_row origin/main` = 0; `main` carries one splitter and this PR introduces
another), so it is own. The cap is 10 own either way, by two corrections that cancelled.
· **The three-checker cross-reference is 1 of 3**, not done, and the reciprocal is owed on two
other lanes with a trigger and a date.

⚠ **R42 — and the round that carried it read as SILENCE for 27 minutes.** After the re-gate's fixes
landed, Codex went dry once on `0fd691fe`, and the next round returned **two P2s as INLINE THREADS**
(threads 116 → 118). The poll's terminator read the reviews+comments resolver only — 2 of the 3
channels the skill names — so a threads-only round showed as `round_items=1 findings=0 dry=false`,
which is indistinguishable from silence, and it printed "trigger likely dropped". ⚠ **Second time
in one session that the MONITOR, not the reviewer, lost the round** (the first was a `jq` error that
fabricated the same verdict). The poll now terminates on a new-unresolved-thread delta as well, and
a THIRD control was added for exactly this shape: a resolver returning the 2-channel line is
reported as `PROBE BROKEN`, not as a quiet round (all three controls executed).
· **R42-2 is FIXED**: a blank id cell whose declaring field carries the umbrella marker is a
contradiction, not a deliberate non-row — keyed by nothing it is absent from `ids`, assertion (a)
sees the marker, assertion (b) skips `self_id is None`, and the `Deps` edge goes unasserted at rc 0.
⚠ Two things the fix had to get right and got wrong first: the field is read from the disposed cell
**at that moment** (`row.field` is written by a LATER pass, so the first version was vacuous — its
own mutant now pins that), and the predicate is the **MARKER alone**, not every kind phrase (written
over all three it flagged the #506 memo's `Function`/`eval` row at `:1985`, the legitimate empty-id
pointer row `declaring_rows` names — a NEGATIVE control now holds that line). The `KIND_PHRASES`
read is factored into ONE site (`Population._phrases`) rather than spelled twice, and the R23 mutant
that guarded the old site follows it.
⚠ **And three existing NEGATIVE controls were blessing the defect as scaffolding**: `_idcell` put
the marker and a `Deps` edge on every R30 id-cell fixture, so the three deliberate-blank cases were
asserting a second thing nobody chose. Their own subject (is `` `—` `` read as a blank?) is
untouched — they keep it with `marked=False` — and the contradiction has controls of its own. That
is the opposite of `memory/feedback_control-rewritten-to-bless-the-defect.md`: the control was
green over a real defect, and it is the DEFECT that turned red.
· **R42-1 is CARVED, not fixed** (§8): a code span inside a resolved image's description keeps its
backticks, so the marker is blanked and the row reads terminal. Five intersecting axes — the §6.1
reader, the `marks` recorder, the image-close branch, `code_mask`'s two exceptions, and the `_demote`
cost contract R23 measured — make it plan-review-first BY RULE.

⚠ **R42-3, the round after, and the fixed poll caught it in 13 minutes where the broken one took
45 and was wrong.** Two P2s again, both in threads.
· **An UNKNOWN CLI OPTION was discarded in silence — FIXED.** `main` took the path list as "every
argv entry that does not start with `--`", so `--worklis`, ONE CHARACTER off `--worklist`, ran the
ordinary report and exited **0**; a caller that asked for the worklist got the other format with
nothing saying so. ⚠ **That caller is on this PR**: the census attestation every round of this
converge has rested on is a byte comparison of `--worklist` output. The fix is a CLOSED set
(`OPTIONS`) whose COMPLEMENT is refused, never a deny-list of known-bad spellings, with a control
that checks the set against the usage lines in both directions and runs `main` over a real fixture,
and two mutants — one re-injecting the discard, one widening the set until the complement is empty
(the direction the rc probes alone cannot see).
· **The DEFER-CAP breach was raised as a finding, independently.** It is not a new defect: it is
the decision §8 already routes to the user, and the reviewer reached the same reading of the policy
("pause to fold, narrow, split, or obtain explicit acceptance; landing unchanged bypasses that
decision"). The thread stays OPEN, because the resolution is not the author's to make — and it is
the one item on this PR that no amount of further review can close.

⚠⚠ **R42-4 FIRED THE ≥2-ROUND SELF-ROOT-CHECK, and the root was in the two fixes of the round
before.** Both P2s were direct consequences of R42-3: an unknown option was refused but a KNOWN one
in the wrong MODE still ran the wrong operation at rc 0 (`--mutants memo.md`), and the escape that
now guarded the self-test's report left the CHECKER's own report — the only channel that prints
MEMO-CONTROLLED text — untouched, so a memo carrying an ESC put a terminal-clear sequence straight
into the default output.
**The root, written out**: both fixes had taken the population the REVIEWER named instead of
deriving it. `_REPORT_MODULES` was a hand-written two-element tuple naming the two self-test
runners — **the enumerated-table class, inside the control written to close an enumerated-population
defect** — and it left out the entry point's 32 print sites; `OPTIONS` was the option SET where the
contract is per MODE. That is the plan's own ideal inverted: a population is DERIVED, not listed.
So this round does not patch the two sites: the population is derived from the ASTs (a module that
calls `print`), `printable` MOVES to the entry point — the report boundary it guards, which the
self-test borrows off the loaded module — and the CLI contract becomes `MODES` (per mode: the flags
it accepts and the positional count it takes).
⚠ **And the proxy is no longer the only witness.** `report_bytes_control` asks the property
DIRECTLY: it runs the program over a memo carrying all 30 plantable C0 characters and DEL in a
reported naming context, in both report modes, and looks at the bytes. The AST sweep could never
have seen this class — its first population was wrong, and a sweep is only as complete as its
reading of the source.
⚠ **Three things the fix got wrong first, each caught by a measurement rather than by review**:
(i) the escape ate the worklist's TABS, because a total escape does not know a separator from
content — caught by the census byte-comparison, exactly the consumer that format exists for. The
fields are escaped and the tabs are the caller's now, and the control asserts the COUNT (five per
row) rather than exempting U+0009, since exempting it would have let a memo-planted tab through.
(ii) A mutant that re-narrowed the population SURVIVED and was **withdrawn rather than weakened**:
with every site wrapped, narrowing the population changes nothing observable, so the row asserted
nothing. (iii) The separate unknown-option guard's mutant survived the moment the mode check landed
— the mode check refuses `--worklis` on its own — so the guard was a strictly weaker second
spelling and **the guard is gone**. Measured, not argued.

⚠⚠⚠ **R42-5 — THE LOOP WAS PAUSED ON A FALSE PREMISE, AND THE PAUSE IS LIFTED.** Two more P2s:
an autolink inside a resolved image description contributes **nothing at all** (0 reported sites
where emphasis, link, nested image and code span all give 1), and `[x <http://a>](absent.md)` exits
2 where §6.3's prose makes the outer syntax literal. Both reproduced before being believed.
**The disposition written here first was to PAUSE and carve a "generator layer", and it rested on
two claims that are FALSE, both of which a second opinion measured:**
· *"three consecutive rounds where the shape hopped between construct families."* **Two.** The
construct-family findings landed at 11:51Z (code span) and 13:38Z (autolink + opener); the two
rounds between them were the CLI and report-channel findings, and "link" was R30-3, rounds earlier.
· *"Round N+1 would have found family five."* **There is no family five.** §3.0b is headed *"the
spec's CLOSED list (the bound IS this table)"* — this document's own bound. §6.2 / §6.3 / §6.4 were
demoted at R30-3, §2.4 / §2.5 substitute, §6.6 renders nothing, §6.7 / §6.8 are line endings, §6.9
is text. **§6.1 and §6.5 were the only two rows left**, and both were in hand.
So the "generating layer" was a **bounded enumeration with exactly two holes**, not a layer that
keeps producing — and the remedy for a bounded enumeration is to close it, which is what the
policy's *容認しない pattern* says outright for work this size.
⚠ **Two further errors in the same disposition**: the carve was **mis-homed** (Slice 2 is the PROSE
half, I-D/I-E, reviewed against a different reference; §6.4 is lexical and belongs to Slice 1, which
was plan-reviewed for exactly this scope — invoking *Edge-dense work* to defer a defect INSIDE the
slice that owns it inverts that rule's base case); and R42-5b was **bundled with the wrong
mechanism** (it lives in the `closed` counter at link close, not the image-close branch).
**✅ FIXED instead, and the fix is the one rule the entry asked for**: the bracket stack carries the
`code` and `auto` bottoms, the image-close branch retags the entries above them `"demoted"` by index
range — the same linear shape as `dem_img`, so the R23 cost contract holds by construction — and the
disposition reads the tag: a demoted code span contributes its content with its backtick runs as
marks, a demoted autolink its URI with the angle brackets as marks. ⚠ The id-only test runs FIRST,
so the decoration exception survives into alt text exactly as `dispose` already keeps the `**` one.
**Measured**: `lx.code` and `lx.autolinks` have ONE consumer each, which is what makes this ≈45 lines
and not a program; the §3.0b cross-product is now a control per row INSIDE a resolved image against
the bare-prose baseline, plus the decorated-id equality in all four positions, plus three mutants
(drop each demotion, and blank a demoted span instead of routing it through `id_only`).
**677 controls / 377 mutants 0 survived 0 crashed**, census `--worklist` byte-identical.
⚠ **What the PAUSE reflex got right** is that it stopped the corner-by-corner patching and asked
which layer generates the findings. The answer was "a closed table with two empty rows" — and the
cost of answering it wrongly was one commit, caught because the judgement was put to a second
reader before it was acted on rather than after.

⚠ **R42-6 — the fold's own follow-on, and the loop caught it in 13 minutes.** §6.1's TRIM is part
of the CONTENT §6.4 reads, and the demotion did not apply it: a padded span only started reaching
that branch once it stopped being masked, so `` ![Slot #11-zz-alph` a ` owns it](img.png) `` left
the padding standing and the slug split into `#11-zz-alph` + `a` — declared nowhere, no site, rc 0.
Fixed where the demotion is: the trimmed spaces JOIN the delimiter marks, which is the same
mechanism saying the same thing — what the reader does not see does not separate. The spec's own
"but does not consist entirely of space characters" arm is a separate control.
⚠⚠ **BOTH of its mutants survived their first fixtures, for two DIFFERENT reasons, and each is the
same lesson**: the padded control measured the site COUNT, and the wrong reading still reports one
site — `9`, which the template also declares — so "one site" was true either way and the claim is
about WHICH id (the fixture now splits an id whose halves are declared nowhere, making the count
the verdict); and the all-space control carried a SPACE after the closing backtick, so that space
did the separating and the claim was never tested (the span is the only separator now).
A probe whose subject is not the claim is green for a reason that has nothing to do with the
property (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`) — twice in
one fix, both found by running the mutants rather than by reading the controls.

⚠⚠⚠ **R42-7 — and the sharpest thing this PR has produced: a fix of mine SILENTLY SHRANK A
CONTROL'S POPULATION, and nothing went red.** Two more P2s, both follow-ons from the fold, both
fixed; but the third defect was found only because a mutant that had been green for forty rounds
started surviving.
· **The demotion was QUADRATIC.** Written as a retag loop per image close, every enclosing close
re-walked every descendant: 0.09 / 0.32 / 1.26 s over 9 / 18 / 36 KB. ⚠ That is the contract
`_demote` exists for, it is the same shape `_demote`'s docstring records being fixed at R23 for
nested images, and the commit that wrote the loops asserted it "holds by construction". It did not.
Ranges, applied once, like `dem_img`: ~1.9× per doubling now, measured.
· **§6.1 was half-implemented**, again: the trim was applied to the RAW content, so a span whose
padding IS a line ending had neither step fire. That is the identical half-implementation `_inner`
records having had at R32 — three lines below the code that repeated it.
· ⚠⚠ **AND THE REAL FINDING: `render_equivalence_control` derives its EXCLUDED set from
`inline_pass.__code__.co_consts`**, on the reading that a single-character constant of that
function is a character it BRANCHES on. The §6.1 step-one substitution put the payload `" "` in
that function — so the SPACE joined the excluded set, **fifteen positions of the §2.5 re-spelling
sweep stopped being swept**, and the mutant guarding the rendered file/cite reading survived.
Nothing was red; the control simply asked less. The step-one pass is a function of its own now, so
the derivation keeps meaning what it says. **A population derived from an implementation detail is
a population the next edit can shrink in silence** — and the only reason this was caught is that the
mutation proof runs every row every time.
⚠ The split that followed was forced by the line-bound control I had just added, firing on me:
`plan_memo_lexer.py` hit 1,008. §6.3's grammar is carved to `plan_memo_links.py` (714 + 340), the
seam MEASURED rather than chosen — an AST pass reported the group references nothing outside itself
while the rest of the lexer references six names in it. ⚠ The split then had to move the work
WITNESS too: `linear_inline_tail_control` watched the lexer alone, and its mutant survived because
the scan it re-introduces now runs in the other file. `_count_lines` takes a module SET.
⚠⚠ **R42-8 — the blind spot I DECLARED, landing one round later, and a control that needed
controls of its own.** Two P2s.
· **§6.6 raw HTML in an alt** → §8, NOT fixed: it is a FABRICATED finding (rc 1 where the alt may
hold no marker), the corpus has **22** Images examples and **zero** with any `<` in a description,
and it corrects this document's own claim that the closed list had two holes. It had three — the
`RENDERS_TEXT` column the claim rested on is the FLOW disposition, and §6.4 asks a different
question of the same kind.
· **The emit-site predicate could not see an `IfExp`.** `print(<%-expr> if cond else <str>)` in the
population summary — memo-controlled through `pop.display` — was unescaped, and the control's
docstring had declared an f-string and a concatenation as its blind spots ONE ROUND EARLIER. Two
named shapes, a third one live
(`memory/feedback_declared-blind-spots-are-where-the-next-finding-lands.md`, measured again).
**So the predicate is INVERTED to the complement**: compliant = a bare string LITERAL, or escaped,
or the one structural form where the escape is inside — `sep.join(printable(x) for x in ...)`,
which is how a machine-readable format keeps its separator as LAYOUT while its fields are content.
Everything else is red. The population went **19 → 41 sites**, and closing it turned up two more
unescaped lines (the conformance reports) the old predicate had never looked at.
⚠ **Two mutants, and the second needed something new.** Unwrapping the `IfExp` is red; a mutant
that WIDENS the predicate is invisible to both the sweep and the ratchet, because `sites` counts
every `print` either way and a looser predicate just empties the finding list. The predicate
therefore has **its own controls**: six hand-built nodes, three that must be accepted and three
that must be refused, checked before the sweep runs — `empty_registry_control`'s idea applied to a
predicate, and the shape `memory/feedback_derived-populations-shrink-in-silence.md`, written
earlier THIS session, asks for.
⚠ And the complement first wrapped TOO MUCH — it caught the worklist's `"\t".join(...)` and the
conformance reports' newlines, both LAYOUT. That is where the structural third arm came from, and
it is stated as structure rather than as an exemption.

⚠ **R42-9 — I narrowed PAST THE EVIDENCE, and that is its own class.** One P2: a blank id cell whose
declaring field spells `KIND UNDETERMINED` was accepted as a deliberate non-row, so with a `Deps`
cell it exited **0** with no gate reporting it. The R42 fix had narrowed that predicate from all
three kind phrases to the MARKER alone — because a real memo row refuted the POINTER arm — and took
the UNDETERMINED arm with it, **though nothing had refuted that one**. §5 puts an undetermined row
IN the naming population with the same no-owner obligation an umbrella has, so a blank id
contradicts it for the same reason. Widened back by exactly one, with a mutant in EACH direction.
**Narrowing to the case that was SHOWN, rather than to the complement of what was REFUTED**, is the
same shape this checker keeps finding in the documents it reads — and it is the third time in this
PR that a correction overshot (`memory/feedback_universal-claims-need-the-complement-measured.md`).
⚠ And the NEGATIVE control that should have caught the overshoot had the WRONG SUBJECT for a third
time: its fixture read "Owned by **9z**, which carries the marker", which matches NO phrase in
`KIND_PHRASES` — so it asserted "a blank row with no kind phrase is silent", true and irrelevant.
The pointer arm's mutant surviving is what exposed it. The fixture spells the phrase verbatim now
(`is a pointer rather than a slice`).

⚠ **R42-10 — the reported finding is an FP, and measuring WHY found a better one.** One P2: a
schema header spelled `![#](i.png)` should match the schema, because §6.4 makes the description the
image's text alternative.
**Rejected, with the measurement**: the vendored corpus holds **22** Images examples and in **every
one** the description's text appears ONLY inside the `alt` attribute, never as document text. The
checker's model — an image puts a picture in the flow, not the letters of `alt` — is the
corpus-supported reading, and R30-3's controls hold it. ⚠ **Two attempts to "fix" it each turned one
of those controls RED** (returning the description read the wrong text entirely — the span handed to
`_inner` is the TAIL; returning blanks stopped a phrase from straddling and silenced a reported
near-miss). A control going red under a fix is the FIX's subject being wrong.
⚠⚠ **But the round's underlying observation was real, and wider than its framing**: a LINKED memo
whose table binds to no schema declares nothing and the run exits **0** — reproduced with a header
reading `No.` instead of `#`, carrying an umbrella row with a nonempty `Deps`. Nothing to do with
images. **The fix for THAT was written and measured and withdrawn**: "unbound table whose first
column tokenises as row ids" is exactly right on four fixtures and reports three legitimate
documentation tables in the real #506 population (`obj`, `R1`…`R7` in a landing record). Carved in
§8 with the candidates that remain, because an over-firing gate on the population this tool exists
to scan is worse than the silence it replaces.
⚠ Three times in this one round a change of mine turned a shipped control or the census red. That
is the gate working, and it is also the measure of how much of this surface I do not hold.
⚠ **An off-by-one INSIDE the sentence correcting an off-by-one**: the R38 note said R33–R36 added
"14 mutants (625/347 → 652/362)"; 362 − 347 = **15**. Fixed.
**▶ ALSO CARVED**: `symbol_attribution_control`'s **existence half** (§8) — nine dead §3 pointers
are named there, and one of them (`fenced_spans`) is a legitimately-planned site, which is why the
half needs §3's `(NEW)` / `✗ (absent)` conventions read first.
**▶ MONITORING**: `ScheduleWakeup` alone loses rounds (R33 sat 98 min). Use a harness-tracked
background poll — `/tmp/wait_r<N>.sh` shape, `HEAD` + `SINCE` filled in, `run_in_background: true`.
⚠ Its exit code is always 0; **read the log**.
**Gate @ `e82d3324`**: 657 controls / 364 mutants 0 survived 0 crashed / trip-wires rc 0 / #506
census worklist byte-identical.

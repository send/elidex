#!/usr/bin/env python3
"""Re-executable mutation proof for `plan-memo-umbrella-check.py --self-test --mutants`.

A control that has never gone red proves nothing.  `MUTANTS` names, for each
lexing clause and each gating stage, ONE edit to the source that removes it,
and the control that must turn red when the edit is applied.  The runner
patches the source TEXT, exec's a fresh module set from it (plan §2 I-F), and
re-runs the named control(s) against the patched checker:

  * the substring must occur EXACTLY ONCE in its file -- a substring that no
    longer applies (the code moved) is a FAIL, never "survived";
  * every named control must fail under the mutant; a mutant all of whose
    controls stay green has SURVIVED, and that is a FAIL too;
  * a control that CRASHES under the mutant proves nothing about the clause
    (an exception is not the control going red) and is a FAIL.

Row shape: (name, file, find, replace, [control names]).
"""

LEXER, TABLES, ROLES, CHECK = (
    "plan_memo_lexer.py", "plan_memo_tables.py", "plan_memo_roles.py",
    "plan-memo-umbrella-check.py")

MUTANTS = [
    # -- CommonMark §4.5 fenced code blocks
    ("fence: opener needs >=3 fence characters", LEXER,
     '(`{3,}|~{3,})', '(`{4,}|~{4,})',
     # the tilde control: three literal backtick lines would pair as a code
     # span and mask the site anyway, so only the tilde form can go red
     ["(fence) a tilde fence masks too"]),
    ("fence: opener indent <=3 spaces", LEXER,
     '_FENCE_OPEN = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")',
     '_FENCE_OPEN = re.compile(r"^ *(`{3,}|~{3,})(.*)$")',
     ["(fence) four spaces of indent is not a fence"]),
    ("fence: backtick info string may not hold a backtick", LEXER,
     '(m.group(1)[0] == "`" and "`" in m.group(2))', 'False',
     ["(fence) a backtick fence whose info string holds a backtick is not a fence"]),
    ("fence: closer uses the opener's character", LEXER,
     're.escape(ch)', '"[`~]"',
     ["(fence) a closer of the OTHER character does not close"]),
    ("fence: closer at least as long as the opener", LEXER,
     '"{%d,}" % k', '"{3,}"',
     ["(fence) a shorter closer does not close"]),
    ("fence: closer followed only by spaces/tabs", LEXER,
     'r"[ \\t]*$"', 'r".*$"',
     ["(fence) a closer followed by text does not close"]),
    ("fence: fenced lines are not paragraph lines (A x B)", TABLES,
     'if i in self.fenced or i in self.table_lines or is_blank(line):',
     'if i in self.table_lines or is_blank(line):',
     ["(fence) a link inside a fence is not a link (A x B)"]),
    # -- CommonMark §6.1 code spans
    ("span: opener and closer are backtick strings of EQUAL length", LEXER,
     'while n and j < len(runs) and runs[j][1] - runs[j][0] != n:\n            j += 1', 'pass',
     ["(span) backtick strings pair by EQUAL length"]),
    ("span: an unmatched backtick string is literal", LEXER,
     '        else:\n            i += 1\n    return out\n\n\ndef blank_spans',
     '        else:\n            out.append((a0, len(s)))\n            i += 1\n    return out\n\n\ndef blank_spans',
     ["(span) an unmatched backtick string is literal, not a mask to end of line"]),
    ("span: lexed over the paragraph, not the line", TABLES,
     '            if one_line_block(line):\n                flush()', '            flush()',
     ["(span) a code span may cross a line ending"]),
    ("span: a list item starts a block", LEXER,
     '_LIST_ITEM.match(line) or ', '',
     ["(span) a paragraph ends at a list item: a backtick open in one item and closed in the next is literal"]),
    ("span: a `>` line starts a block", LEXER,
     ' or _QUOTE.match(line))', ')',
     ["(span) a paragraph ends at a `>` line"]),
    ("span: an ATX heading is a block", LEXER,
     '_ATX.match(line) or _THEMATIC', '_THEMATIC',
     ["(span) a paragraph ends at an ATX heading"]),
    ("A x E: kind markers are read from the MASKED declaring field", TABLES,
     'row.field = prose(row.cells[row.schema.decl].lexed)',
     'row.field = row.cells[row.schema.decl].text',
     ["(span) a quoted kind marker is not a declaration (A x E)"]),
    # -- GFM §4.10 rows and tables
    ("row: an unescaped `|` splits even inside backticks (Example 200)", LEXER,
     '        elif c == "|":\n            bounds.append((start, i))',
     '        elif c == "|" and line[start:i].count("`") % 2 == 0:\n            bounds.append((start, i))',
     ["(row) an unescaped `|` inside backticks SPLITS the row: each half is prose with a literal "
      "backtick, so the id on each side is a mention"]),
    ("row: `\\|` becomes `|` (the backslash is consumed)", LEXER,
     '            breaks.append(i)\n            i += 2', '            i += 2',
     ["(row) `\\|` is `|` in the cell, so `9z \\| 7z` is an id-only run"]),
    ("row: the leading pipe is optional", LEXER,
     'if stripped.startswith("|") and bounds:', 'if bounds:',
     ["(row) a table without leading and trailing pipes declares its rows"]),
    ("row: the trailing pipe is optional", LEXER,
     'if stripped.endswith("|") and not stripped.endswith("\\\\|") and bounds:', 'if bounds:',
     ["a table with and without edge pipes reads the same"]),
    ("row: offsets map through each cell's raw segments", LEXER,
     '        return raw_start + (i - off)', '        return i',
     ["a site after an escaped pipe is reported at its raw column"]),
    ("table: delimiter cell = >=1 hyphen with optional colons", LEXER,
     '_DELIM_CELL = re.compile(r":?-+:?")', '_DELIM_CELL = re.compile(r"[:\\- ]*")',
     ["(table) a delimiter cell is >=1 hyphen with optional colons; `:` alone is not"]),
    ("table: header and delimiter must have equal width", TABLES,
     '        if len(header) != width:\n            i += 1\n            continue',
     '        if False:\n            i += 1\n            continue',
     ["(rc) a slice header over a one-cell delimiter row is not a table, so its wide body row "
      "is not a width miss"]),
    ("table: a schema body row of the wrong width is exit 2", TABLES,
     'if schema is not None and len(body) != width:', 'if False:',
     ["(rc) a schema body row whose width differs from its header is rc 2"]),
    # -- CommonMark §6.3 links / §4.7 definitions
    ("link: bare destination balances parentheses", LEXER,
     '        if c == "(":\n            depth += 1', '        if c == "(":\n            break',
     ["(link) bare destination with a balanced parenthesis pair"]),
    ("link: backslash escapes ASCII punctuation (destination)", LEXER,
     'return s[j] == "\\\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT', 'return False',
     ["(link) bare destination with an escaped parenthesis"]),
    ("link: backslash escapes ASCII punctuation (`<dest>`)", LEXER,
     'return s[j] == "\\\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT', 'return False',
     ["(link) `<dest>` may contain an escaped `>`"]),
    ("link: backslash escapes ASCII punctuation (title)", LEXER,
     'return s[j] == "\\\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT', 'return False',
     ['(link) a `"…"` title may contain an escaped `"`']),
    ("link: only ASCII space ends a bare destination, not Unicode whitespace", LEXER,
     'if c == " " or ord(c) <= 31 or ord(c) == 127:', 'if c.isspace() or ord(c) <= 31:',
     ["(link) a non-ASCII space is destination content (only ASCII space and controls end it)"]),
    ("link: 0x7F is an ASCII control character", LEXER,
     'if c == " " or ord(c) <= 31 or ord(c) == 127:', 'if c == " " or ord(c) <= 31:',
     ["(link) an ASCII control character (0x7F) ends a bare destination"]),
    ("link: `<dest>` holds no unescaped `<`", LEXER,
     'elif s[j] in "<>\\n":', 'elif s[j] in ">\\n":',
     ["(link) `<dest>` may not contain an unescaped `<`"]),
    ("link: a `(…)` title holds no unescaped `(`", LEXER,
     '        elif opener == "(" and s[j] == "(":\n            return None',
     '        elif False:\n            return None',
     ["(link) a `(…)` title may not contain an unescaped `(`"]),
    ("link: label matching collapses internal whitespace", LEXER,
     'return " ".join(label.split()).casefold()', 'return label.strip().casefold()',
     ["(link) label matching collapses internal whitespace"]),
    ("link: label matching is a case FOLD", LEXER,
     'return " ".join(label.split()).casefold()', 'return " ".join(label.split()).lower()',
     ["(link) label matching is a Unicode case FOLD, not lower()"]),
    ("link: `[text]` followed by a link label is not a shortcut", LEXER,
     '                # a link label follows, so `[text]` is not a shortcut either\n'
     '                unresolved.append((i, raw))\n'
     '                i += 1\n                continue',
     '                pass',
     ["(link) `[label][undefined]` is neither a full reference nor a shortcut (§6.3: a shortcut "
      "is not followed by a link label)"]),
    ("link: collapsed reference resolves the text as label", LEXER,
     'dest = defs.get(normalize_label(text)) if not inner else None', 'dest = None',
     ["(link) collapsed reference `[label][]`"]),
    ("link: bracket text nests (full reference with inner brackets)", LEXER,
     '            if depth > 1:\n                inner = True', '            if depth > 1:\n                return None',
     ["(link) full reference whose text holds nested brackets; the label is the link's tail, not "
      "prose, and so is the definition"]),
    ("link: links are taken from the masked stream (A x B)", LEXER,
     'masked = blank_spans(text, self.code)', 'masked = text',
     ["(rc) a code-quoted link to an absent file is not a link: rc 0"]),
    ("def: the first definition of a label wins", TABLES,
     'self.defs.setdefault(normalize_label(raw), dest)', 'self.defs[normalize_label(raw)] = dest',
     ["(def) the FIRST definition of a label wins"]),
    ("def: up to one line ending before the destination", LEXER,
     '        k = _skip_ws(s, k + 1)\n        dest, k = link_destination(s, k)',
     '        k = _skip_ws(s, k + 1, newlines=0)\n        dest, k = link_destination(s, k)',
     ["(def) one line ending is allowed before the destination"]),
    ("def: nothing but whitespace after the destination/title", LEXER,
     '    if s[k] == "\\n":\n        return k + 1\n    return None',
     '    if s[k] == "\\n":\n        return k + 1\n    return k',
     ["(def) text after the destination is not a definition"]),
    ("def: a definition cannot interrupt a paragraph", LEXER,
     'self.definitions, self.defs_end = ([], 0) if cell else reference_definitions(masked)',
     'self.definitions, self.defs_end = [d for ln in masked.split("\\n") for d in reference_definitions(ln)[0]], 0',
     ["(def) a definition cannot interrupt a paragraph"]),
    # -- I-F one population, one pipeline
    ("population: every memo's rows are declared (census)", TABLES,
     '        for memo in self.memos:\n            self._declare(memo)', '        self._declare(self.main)',
     ["(population) an umbrella declared in a linked memo is in the census"]),
    ("population: every memo's ids are in the keep-set", TABLES,
     '        return set(self.ids)',
     '        return {rid for rid, r in self.ids.items() if r.memo is self.main}',
     ["(population) a terminal id declared in a linked memo is in the keep-set, so `Tq / 9z` is "
      "an id-only run, not code"]),
    ("population: every memo's rows are asserted", TABLES,
     'return [r for memo in self.memos for r in memo.schema_rows(name)]',
     'return list(self.main.schema_rows(name))',
     ["(b) a sibling umbrella's Deps edge is asserted"]),
    ("population: the link walk is transitive", TABLES,
     'queue.extend(memo.linked_files())',
     'queue.extend(memo.linked_files() if len(self.memos) == 1 else [])',
     ["(population) the population is transitive: a memo linked from a linked memo is scanned"]),
    ("gate: an absent linked memo is a schema miss", TABLES,
     'self.misses.append((p.name, 0, "linked memo not found -- its population is unscanned"))',
     'pass',
     ["(rc) a linked memo that is not on disk is rc 2, never clean"]),
    ("gate: an unmatched schema is a schema miss", TABLES,
     '                if s.name not in matched:', '                if False:',
     ["(rc) a schema with no matching table is rc 2"]),
    ("gate: a duplicate declaration is a schema miss", TABLES,
     '                if rid in self.ids:', '                if False:',
     ["(rc) the same id declared in two memos is rc 2"]),
    ("gate: KIND-SPELLING is a mechanical finding", CHECK,
     '        if len(pop.spellings) > 1:', '        if False:',
     ["(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1"]),
    ("gate: a mechanical finding is exit 1", CHECK,
     'rc = 1 if mechanical else 0', 'rc = 0',
     ["(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1"]),
    # -- /code-review high: one mutant per fix
    ("F1 id cell: the id is followed by a non-id character, not the cell end", TABLES,
     '+ r"(?![0-9A-Za-z-])")', '+ r"$")',
     ["(id) an id cell with trailing prose declares the id at its start",
      "(id) a backticked slug with trailing prose declares the slug"]),
    ("F1 id cell: a cell not starting with an id declares nothing (no fallback to the cell)", TABLES,
     'return g.group("id") if g else None', 'return g.group("id") if g else cell_text.strip()',
     ["(id) a cell that does not start with an id declares nothing"]),
    ("F1 id cell: a non-id id cell is reported", TABLES,
     'self.undeclared.append((memo.path.name, row.lineno, s.name, row.id_cell()))', 'pass',
     ["(id) a non-empty id cell that is not an id is reported as a note"]),
    ("F2 population: links in CELLS join the population", TABLES,
     '        for lx in self.lexed():\n            for _, _, dest in lx.links:',
     '        for lx in (p.lexed for p in self.paragraphs):\n            for _, _, dest in lx.links:',
     ["(rc) a link to an absent memo inside a table CELL is rc 2",
      "(population) a violation in a sibling linked ONLY from a cell is reported"]),
    ("F3 population: a destination with a scheme or `//` is not a sibling", TABLES,
     'if _SCHEME.match(dest) or dest.startswith("//"):', 'if False:',
     ["(rc) an absolute URL ending in `.md` is not a sibling on disk: rc 0",
      "(rc) a protocol-relative `//host/x.md` is not a sibling on disk: rc 0"]),
    ("F4 attribution: the FIRST marker occurrence decides", TABLES,
     '    m = re.search(re.escape(MARKER), field)',
     '    m = list(re.finditer(re.escape(MARKER), field))[-1]',
     ["(a) a self-declaring field that later says a sibling 'is not it' stays self-declaring"]),
    ("F5 kind: the undetermined spelling is collected beside the marker", TABLES,
     '        if m:\n            self.spellings.add(m.group(0))',
     '        if m and MARKER not in row.field:\n            self.spellings.add(m.group(0))',
     ["(rc) a row carrying the marker AND one undetermined spelling, beside another row's other "
      "spelling, is KIND-SPELLING rc 1"]),
    ("F6 (c): the Deps cell's ids are the population's mentions, not a raw tokenisation", ROLES,
     'cell_ids = in_deps.get((row.memo.path.name, row.lineno), set())',
     'cell_ids = set(re.findall(SHORT_ID, deps))',
     ["(c-seed) a Deps cell naming a FILE whose name holds the id does not carry the id",
      "(c-seed) a Deps cell `xxxxC` does not carry the id `C`"]),
    ("F7 check: an empty no-owner census is clean, not a schema miss", CHECK,
     '    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))',
     '    if not umb:\n        return _result(findings, notes, 2, [], pop)\n'
     '    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))',
     ["(rc) a memo whose every row is terminal is rc 0, not a schema miss",
      "(rc) an all-terminal memo reports a zero census"]),
    ("F8 empty cell: `n/a` is empty", TABLES,
     'EMPTY_CELL = frozenset({"", "\\u2014", "-", "n/a"})', 'EMPTY_CELL = frozenset({"", "\\u2014", "-"})',
     ["(c-seed) a Deps cell `n/a` is empty, so ordering prose is reported"]),
    ("F8 empty cell: `-` is empty", TABLES,
     'EMPTY_CELL = frozenset({"", "\\u2014", "-", "n/a"})', 'EMPTY_CELL = frozenset({"", "\\u2014", "n/a"})',
     ["(id) an id cell `-` is empty: not declared, not reported"]),
    ("F9 span: an escaped backtick opens no span", LEXER,
     '        if _escaped(s, a0):\n            a0 += 1', '        if False:\n            a0 += 1',
     ["(span) a backtick behind a backslash is literal and opens no span"]),
    ("F10 def: a cell parses no reference definition", LEXER,
     '([], 0) if cell else reference_definitions(masked)', 'reference_definitions(masked)',
     ["(def) a cell shaped like a definition is inline content and is scanned"]),
    ("F12 link: the link tail masks a slug", TABLES,
     '    out += [(a, b, "link") for a, b, _ in lx.links]', '    pass',
     ["(link) a `#11-` slug in a link DESTINATION is not a naming site"]),
    ("F13 link: an unanswered full reference is reported", LEXER,
     '                unresolved.append((i, raw))', '                pass',
     ["(link) a full reference no definition answers is reported as unresolved"]),
    ("F13 link: a shortcut with an orphan definition is reported", LEXER,
     '            unresolved.append((i, text))\n        i += 1', '            pass\n        i += 1',
     ["(link) a shortcut whose only definition sits mid-paragraph is reported as unresolved"]),
    ("C7 kind: the population's pointer kind excludes the row from the acceptance seed", ROLES,
     '        if row.kind != "terminal":\n            continue',
     '        if row.kind in ("umbrella", "undetermined"):\n            continue',
     ["(accept-vocab seed) a POINTER row is excluded from the population",
      "(accept-vocab seed) a row whose marker is ATTRIBUTED to another row is a pointer and owes "
      "no acceptance condition"]),
    ("C8 disposition: a kept slug inside a code span is visible", TABLES,
     '            if m.group(0) in keep:', '            if False:',
     ["(span) a kept slug inside a command-line code span is a naming site"]),
]


def run(reg):
    """Apply each mutant to a fresh module set and re-run its controls.
    Returns the list of FAIL strings (empty = every mutant was killed)."""
    import plan_memo_umbrella_selftest as st

    fails = []
    print()
    print("mutants (each must turn its control red):")
    for name, file, find, replace, controls in MUTANTS:
        src = (st.HERE / file).read_text()
        n = src.count(find)
        if n != 1:
            fails.append("MUTANT %r: substring occurs %d times in %s (must be exactly 1) -- the "
                         "mutant no longer applies and proves nothing" % (name, n, file))
            print("  FAIL [MUTANT] %s (substring x%d)" % (name, n))
            continue
        unknown = [c for c in controls if c not in reg]
        if unknown:
            fails.append("MUTANT %r names unknown control(s) %s" % (name, unknown))
            print("  FAIL [MUTANT] %s (unknown control)" % name)
            continue
        M = st.load({file: src.replace(find, replace)})
        survived, crash = [], None
        try:
            for c in controls:
                try:
                    ok = reg[c][1](M)[0]
                except Exception as e:
                    # an exception is not the control going red: the clause
                    # under test was never exercised, so this proves nothing
                    crash = "%s: %s" % (type(e).__name__, str(e)[:60])
                    break
                if ok:
                    survived.append(c)
        finally:
            st.unload()
        if crash:
            fails.append("MUTANT %r crash: %s" % (name, crash))
        elif survived:
            fails.append("MUTANT %r SURVIVED: control(s) stayed green: %s" % (name, survived))
        print("  %-4s [MUTANT] %s%s" % ("FAIL" if (survived or crash) else "ok", name,
                                        " (crash: %s)" % crash if crash else ""))
    print("%d mutant(s), %d survived, %d crashed."
          % (len(MUTANTS), sum(1 for f in fails if "SURVIVED" in f),
             sum(1 for f in fails if " crash: " in f)))
    return fails

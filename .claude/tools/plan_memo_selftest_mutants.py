#!/usr/bin/env python3
"""Re-executable mutation proof for `plan-memo-umbrella-check.py --self-test --mutants`.

A control that has never gone red proves nothing.  `MUTANTS` names, for each
lexing clause and each gating stage, ONE edit to the source that defeats it,
and the control that must turn red when the edit is applied.  Most rows
REMOVE a clause; a few (F7, the shape rule of `is_empty`) RE-INJECT the
defect the clause was written against, because the clause is a default (an
empty census is clean; a cell with no alphanumeric is empty) and there is no
line to delete -- the defect has to be put back to show the control sees it.
The runner patches the source TEXT, exec's a fresh module set from it (plan
§2 I-F), and re-runs the named control(s) against the patched checker:

  * the substring must occur EXACTLY ONCE in its file -- a substring that no
    longer applies (the code moved) is a FAIL, never "survived";
  * every named control must fail under the mutant; a mutant all of whose
    controls stay green has SURVIVED, and that is a FAIL too;
  * a control that CRASHES under the mutant proves nothing about the clause
    (an exception is not the control going red) and is a FAIL.

Row shape: (name, file, find, replace, [control names]).  A row whose file is
the RUNNER itself (`plan_memo_umbrella_selftest.py`) patches the runner, not
the checker set, and takes its controls from the patched runner's registry.
"""

LEXER, TABLES, ROLES, CHECK, SELFTEST = (
    "plan_memo_lexer.py", "plan_memo_tables.py", "plan_memo_roles.py",
    "plan-memo-umbrella-check.py", "plan_memo_umbrella_selftest.py")

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
     'row.field = stream(row.cells[row.schema.decl].lexed)',
     'row.field = row.cells[row.schema.decl].text',
     ["(span) a quoted kind marker is not a declaration (A x E)"]),
    # -- GFM §4.10 rows and tables
    ("row: an unescaped `|` splits even inside backticks (Example 200)", LEXER,
     '        elif c == "|":\n            bounds.append((start, i))',
     '        elif c == "|" and line[start:i].count("`") % 2 == 0:\n            bounds.append((start, i))',
     ["(row) an unescaped `|` inside backticks SPLITS the row: each half is prose with a literal "
      "backtick, so the id on each side is a mention"]),
    ("row: `\\|` becomes `|` (the backslash is consumed)", LEXER,
     '                breaks.append(j - 1)\n                i = j + 1', '                i = j + 1',
     ["(row) `\\|` is `|` in the cell, so `9z \\| 7z` is an id-only run"]),
    ("row: the leading pipe is optional", LEXER,
     'if stripped.startswith("|") and bounds:', 'if bounds:',
     ["(row) a table without leading and trailing pipes declares its rows"]),
    ("row: the trailing pipe is optional", LEXER,
     'if stripped.endswith("|") and bounds and not _escaped(stripped, len(stripped) - 1):', 'if bounds:',
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
    ("table: a schema body row of the wrong width is exit 2 (the width miss gates)", TABLES,
     '                for lineno, msg in t.misses:\n'
     '                    self.misses.append((memo.path.name, lineno, msg))',
     '                for lineno, msg in t.misses:\n'
     '                    pass',
     ["(rc) a schema body row whose width differs from its header is rc 2"]),
    # -- CommonMark §6.3 links / §4.7 definitions
    ("link: bare destination balances parentheses", LEXER,
     '        if c == "(":\n            depth += 1', '        if c == "(":\n            break',
     ["(link) bare destination with a balanced parenthesis pair"]),
    ("link: backslash escapes ASCII punctuation (destination, `<dest>`, title)", LEXER,
     'return s[j] == "\\\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT', 'return False',
     ["(link) bare destination with an escaped parenthesis",
      "(link) `<dest>` may contain an escaped `>`",
      '(link) a `"…"` title may contain an escaped `"`']),
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
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     'return label.strip(" \\t\\r\\n").casefold()',
     ["(link) label matching collapses internal whitespace"]),
    ("link: label matching is a case FOLD", LEXER,
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).lower()',
     ["(link) label matching is a Unicode case FOLD, not lower()"]),
    ("link: label matching folds spaces, tabs, line endings -- not Unicode whitespace", LEXER,
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     'return " ".join(label.split()).casefold()',
     ["(link) label matching collapses spaces, tabs and line endings only: a non-breaking "
      "space is not a space, so the reference is unanswered"]),
    ("link: label content = a character that is not a space, tab, or line ending", LEXER,
     'return bool(raw.strip(" \\t\\r\\n"))', 'return bool(raw.strip())',
     ["(link) a label holding only a non-breaking space is a label (§6.3: at least one "
      "character that is not a space, tab, or line ending)"]),
    ("link: `[text]` followed by a link label is not a shortcut", LEXER,
     '                # a link label follows, so `[text]` is not a shortcut either\n'
     '                unresolved.append((i, raw))\n'
     '                i += 1\n                continue',
     '                pass',
     ["(link) `[label][undefined]` is neither a full reference nor a shortcut (§6.3 Example 571: "
      "a shortcut is not followed by a link label) -- the unanswered label is a schema miss, not "
      "a link to the sibling"]),
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
     ["(def) text after the destination is not a definition, so the reference is unanswered: "
      "a schema miss"]),
    ("def: a definition cannot interrupt a paragraph", LEXER,
     'self.definitions, self.defs_end = ([], 0) if cell else reference_definitions(masked)',
     'self.definitions, self.defs_end = [d for ln in masked.split("\\n") for d in reference_definitions(ln)[0]], 0',
     ["(def) a definition cannot interrupt a paragraph: the reference is unanswered, and "
      "reported ONCE (`[text][label]` re-scans `[label]`)"]),
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
     ["(id) a cell that does not start with an id declares nothing: the row is unkeyed (its "
      "Deps edge would go unasserted), so the run is a schema miss"]),
    ("#2 gate: an unkeyed schema row is a schema miss (not a note, not a silent drop)", TABLES,
     '                    if not is_blank_id_cell(row.id_cell()):\n'
     '                        self.misses.append(',
     '                    if False:\n'
     '                        self.misses.append(',
     ["(id) a cell that does not start with an id declares nothing: the row is unkeyed (its "
      "Deps edge would go unasserted), so the run is a schema miss"]),
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
     'cell_ids = in_deps.get(_row_key(row), set())',
     'cell_ids = set(re.findall(SHORT_ID, deps))',
     ["(c-seed) a Deps cell naming a FILE whose name holds the id does not carry the id",
      "(c-seed) a Deps cell `xxxxC` does not carry the id `C`"]),
    ("F7 check: an empty no-owner census is clean, not a schema miss", CHECK,
     '    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))',
     '    if not umb:\n        return _result(findings, notes, 2, [], pop)\n'
     '    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))',
     ["(rc) a memo whose every row is terminal is rc 0, not a schema miss",
      "(rc) an all-terminal memo reports a zero census"]),
    ("F8 / #4 empty cell: `n/a` is a lexical exception (case-insensitive)", TABLES,
     'EMPTY_WORDS = frozenset({"n/a", "none"})', 'EMPTY_WORDS = frozenset({"none"})',
     ["(c-seed) a Deps cell `n/a` is empty, so ordering prose is reported",
      "(c-seed) a Deps cell `N/A` is empty (the lexical exception is case-insensitive)"]),
    ("#4 empty cell: emptiness is decided by SHAPE (no alphanumeric), re-injecting the old list", TABLES,
     'return not any(ch.isalnum() for ch in bare) or bare.casefold() in EMPTY_WORDS',
     'return bare in {"", "\\u2014", "-"} or bare.casefold() in EMPTY_WORDS',
     ["(c-seed) a Deps cell `–` (en dash) is empty by shape: no alphanumeric",
      "(c-seed) a Deps cell `--` is empty by shape"]),
    ("4.5 id cell: blanks are LITERAL, not the shape rule (re-inject `is_empty`)", TABLES,
     '                    if not is_blank_id_cell(row.id_cell()):',
     '                    if not is_empty(row.id_cell()):',
     ["(id) an id cell `?` is not a blank: unkeyed, rc 2",
      "(id) an id cell `…` is not a blank: unkeyed, rc 2 (the shape rule would skip it)",
      "(id) an id cell `**?**` is not a blank: decoration does not blank it, rc 2"]),
    ("4.5 id cell: `—` is a literal blank", TABLES,
     'ID_CELL_BLANKS = frozenset({"", "\\u2014", "-", "\\u2013"})',
     'ID_CELL_BLANKS = frozenset({"", "-", "\\u2013"})',
     ["(id) an id cell `—` is a literal blank: a deliberate non-row, rc 0"]),
    ("4.5 link: a citation-grammar label is exempt in every reference form", TABLES,
     'exempt = _CITE_LABEL.fullmatch(key) is not None or _is_shortcut(lx, off)',
     'exempt = _is_shortcut(lx, off)',
     ["(link) adjacent citations `[C19][C20]` are not a full reference: rc 0",
      "(link) a collapsed-shaped citation `[C19][]` is not a reference: rc 0"]),
    # -- PR #510 Codex R1
    ("R1-1 row: only an ODD backslash run escapes `|` (even run = literal backslash + pipe)", LEXER,
     '            if j < n and line[j] == "|" and (j - i) % 2:',
     '            if j < n and line[j] == "|":',
     ["(row) `a\\\\|b` holds an UNESCAPED pipe (§2.4: `\\\\` is a literal backslash): 5 cells "
      "under a 4-cell header is a width miss, rc 2"]),
    ("R1-1 row: the trailing-pipe check uses the same parity", LEXER,
     'if stripped.endswith("|") and bounds and not _escaped(stripped, len(stripped) - 1):',
     'if stripped.endswith("|") and bounds and not stripped.endswith("\\\\|"):',
     ["(row) a trailing `\\\\|` is a literal backslash then the trailing pipe: rc 0"]),
    ("R1-2 runner: the emptiness guard fires on zero controls / zero mutants", SELFTEST,
     '    if n_controls == 0:\n        out.append(', '    if False:\n        out.append(',
     ["an empty control or mutant registry is a FAIL, never green"]),
    ("R1-3 link: a completed link inside the bracket text makes the outer brackets text", LEXER,
     '        if inner and links(text, defs)[0]:', '        if False:',
     ["(link) nested inline links: the INNER link is the link, the outer tail is text -- "
      "`child.md` joins the population, absent `parent.md` is not linked",
      "(link) a reference link nested in inline brackets: the inner reference is the link, the "
      "outer tail is text"]),
    ("R1-4 def: an orphan candidate is parsed with its continuation line, not per line", LEXER,
     '            defs, _ = reference_definitions(s[off:])', '            defs, _ = reference_definitions(s[off:end])',
     ["(def) a would-be MULTILINE definition that interrupts a paragraph is an orphan: the "
      "shortcut naming it is a schema miss, not an exempt citation-style shortcut"]),
    ("#4 empty cell: a word outside the lexical exceptions is NOT empty", TABLES,
     'EMPTY_WORDS = frozenset({"n/a", "none"})', 'EMPTY_WORDS = frozenset({"n/a", "none", "nil"})',
     ["(b) a Deps cell `nil` -- a word outside the lexical exceptions -- is NOT empty: the "
      "stated polarity is a reported edge (false rc 1), never a silent skip"]),
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
     ["(link) a full reference no definition answers is a schema miss"]),
    ("F13 link: a shortcut with an orphan definition is reported", LEXER,
     '            unresolved.append((i, text))\n        i += 1', '            pass\n        i += 1',
     ["(link) a shortcut whose only definition sits mid-paragraph is a schema miss"]),
    ("#1 gate: an unresolved reference is a schema miss (rc 2), not a note", TABLES,
     '            for lineno, label in memo.unresolved_references():\n'
     '                self.misses.append(',
     '            for lineno, label in ():\n'
     '                self.misses.append(',
     ["(rc) a full reference no definition answers is rc 2, never clean",
      "(link) a full reference no definition answers is a schema miss"]),
    ("C7 kind: the population's pointer kind excludes the row from the acceptance seed", ROLES,
     '        if row.kind != "terminal":\n            continue',
     '        if row.kind in ("umbrella", "undetermined"):\n            continue',
     ["(accept-vocab seed) a POINTER row is excluded from the population",
      "(accept-vocab seed) a row whose marker is ATTRIBUTED to another row is a pointer and owes "
      "no acceptance condition"]),
    ("C8 disposition: a kept slug inside a code span is visible", TABLES,
     '            if m.group(0) in keep:', '            if False:',
     ["(span) a kept slug inside a command-line code span is a naming site"]),
    # -- /elidex-review Stage 6
    ("#3 bare id: bounded by the complement of the id-continuation class (re-inject a list)", CHECK,
     '_ID_CONTINUES = re.compile(r"[0-9A-Za-z]")', '_ID_CONTINUES = re.compile(r"[^\\s,;/()\\[\\]*`.:]")',
     ["(bare) an id before `?` is bounded", "(bare) an id before `!` is bounded",
      "(bare) an id inside ASCII double quotes is bounded",
      "(bare) an id inside curly double quotes is bounded"]),
    ("#3 bare id: a hyphen bounds a short id (re-inject it into the class)", CHECK,
     '_ID_CONTINUES = re.compile(r"[0-9A-Za-z]")', '_ID_CONTINUES = re.compile(r"[0-9A-Za-z-]")',
     ["(bare) a hyphen bounds a short id: `after 9z-7z` names 9z"]),
    ("#3 file token: a bare `.md` file name is masked before the bare scan", LEXER,
     '|(?P<file>[\\w./-]+\\.md\\b)', '',
     ["(bare) a bare `.md` file name holding an id is a file token, not a site"]),
    ("#3 bare id: a dotted number is one token", CHECK,
     '    return text[i] == "." and 0 <= j < len(text) and text[j].isalnum()', '    return False',
     ["(bare) a dotted number is one token: `§6.9z` names no row"]),
    ("#3 bare id: a decorated side is bounded by its decoration", CHECK,
     '        if not tok.group("r") and _glued(text, e, +1):', '        if _glued(text, e, +1):',
     ["(bare) the decoration closes the token even against an id character: `**9z**7z`"]),
    ("#8 stream: the (c) seed reads the Slice cell's disposed stream", ROLES,
     '        body = _stream(row, "Slice")\n        empty = is_empty(deps)',
     '        body = row.col("Slice").text\n        empty = is_empty(deps)',
     ["(c-seed) ordering vocabulary inside a code span is code, not prose"]),
    ("#8 stream: the acceptance seed and RETIRED read the disposed stream", ROLES,
     '        body = _stream(row, "Slice")\n        # the population decided',
     '        body = row.col("Slice").text\n        # the population decided',
     ["(accept-vocab seed) a retirement word inside a code span does not retire the row"]),
    ("#8 stream: the (d) seed reads the disposed stream", ROLES,
     'for m in OWNS_TWO.finditer(_stream(row, "Slice")):', 'for m in OWNS_TWO.finditer(row.col("Slice").text):',
     ["(d) an ownership clause inside a code span is code, not a two-owner claim"]),
    ("#8 stream: the licensing rule reads the disposed stream", CHECK,
     '        return self.block.stream\n\n    @property\n    def key',
     '        return self.block.text\n\n    @property\n    def key',
     ["(licence) a licensing phrase inside a code span licenses nothing"]),
    ("#8 stream: every span of the mask is blanked, not only code", TABLES,
     'return blank_spans(lx.text, [(a, b) for a, b, _ in lx.mask])',
     'return blank_spans(lx.text, [(a, b) for a, b, k in lx.mask if k == "code"])',
     ["(stream) ordering vocabulary in a link TITLE is the link's tail, not prose"]),
    ("#11 identity: per-memo maps are keyed on the resolved path, not the basename", TABLES,
     '        return str(self.path)', '        return self.path.name',
     ["(c-seed) a sibling of the SAME basename in another directory, whose Deps cell at the "
      "same line names the party, does not discharge the main memo's row"]),
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
        patched = src.replace(find, replace)
        if file == SELFTEST:
            # a mutant against the runner: the checker set is unpatched, the
            # controls come from the PATCHED runner's registry
            M, table = st.load(), st.patched_runner(patched).registry()
        else:
            M, table = st.load({file: patched}), reg
        survived, crash = [], None
        try:
            for c in controls:
                try:
                    ok = table[c][1](M)[0]
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

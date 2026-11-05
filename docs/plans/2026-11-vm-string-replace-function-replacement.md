# `#11-vm-string-replace-function-replacement` — functional replacement and GetSubstitution for `String.prototype.replace` / `replaceAll` (umbrella split + per-PR plan-memo for S1)

**Slot**: `#11-vm-string-replace-function-replacement` (registered in the live ledger 2026-11-05, PM 414;
`memory/project_open-defer-slots.md:257`). **Owner**: VM P4. **Home as registered**: slice-plan umbrella
**9b** (`String` prototype surface). **Baseline**: `efe8d05b`. Every `file:line` below is at `efe8d05b`
unless it names another revision; §6 re-derives each one. Paths are relative to
`crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`.

**The defect, stated as a defect.** Neither `String.prototype.replace` nor `String.prototype.replaceAll`
has a functionalReplace branch, and neither has GetSubstitution. Both natives `ToString` the
replacement unconditionally, so a function replacement is never called — its `ToString` text is
spliced in, `function () { [native code] }` for a user function — and a template replacement is
spliced in verbatim, so `$&`, `` $` ``, `$'`, `$$`, `$n` and `$<name>` are never expanded. Both are
silent in release and debug builds. `replaceAll` additionally never treats a RegExp as a RegExp: it
`ToString`s it and searches for the source text (`'abab'.replaceAll(/b/g, 'X')` → `abab`), and a
non-global RegExp does not throw.

**This memo does two things.** §2 measures the slot as registered and finds it an **umbrella** (six
intersecting axes), splits it, and routes the RegExp half to the rows that own it (§5). The remainder
of the memo is the per-PR plan for the one terminal child this slot keeps, **S1**: the
non-RegExp path of both methods plus the shared replacement chokepoint and GetSubstitution, written
once in the shape the RegExp-side algorithm will consume.

## §0. Scope, premises, decisions

### §0.1 Scope

**The slot (umbrella), by measurement (§2):** every replacement that `replace` / `replaceAll` perform,
string and RegExp pattern alike.

**S1 (this PR, terminal) — in scope:**

- **§22.1.3.19** `String.prototype.replace` steps 4-15 (the path taken when `searchValue` is not an
  Object, or is an Object with no `%Symbol.replace%` method — today: any value that is not
  `ObjectKind::RegExp`): `native_string_replace`'s string branch, `vm/natives_string.rs:530-545`.
- **§22.1.3.20** `String.prototype.replaceAll` steps 4-17, same condition:
  `native_string_replace_all`, `vm/natives_string_ext.rs:206-252`, whole body except that its step 3
  stays absent (below).
- **§22.1.3.19.1** GetSubstitution, once, with its full spec signature (`captures`, `namedCaptures`),
  in a new module `vm/string_replace.rs` (NEW), together with the per-match replacement chokepoint
  both methods — and, later, `%RegExp.prototype%[%Symbol.replace%]` — go through (§1.4).

**S1 — not in scope** (each one homed in §5):

- The RegExp branch of `native_string_replace` (`vm/natives_string.rs:477-528`). It keeps today's
  behaviour, function `ToString` included, until the `%Symbol.replace%` child of umbrella 8b replaces
  it (§0.4 OD-1). S1 does **not** add a functional branch to it: that would be a second
  implementation of §22.2.6.11's loop beside the one 8b's child must ship.
- `replaceAll` step 3 (IsRegExp, the `flags`/`"g"` TypeError, `GetMethod(searchValue,
  %Symbol.replace%)`). It needs `%Symbol.match%` and `%Symbol.replace%` ids that do not exist (8d), and
  writing it on `ObjectKind::RegExp` branding would add the special-casing umbrella 8b exists to remove
  (9b's row says so for this member by name).
- The RegExpBuiltinExec `lastIndex` overrun hang (§5 SD-1) and the other RegExp-side defects §1.3
  records.

### §0.2 Premises re-measured

| Premise (brief / live ledger) | At `efe8d05b` | Command |
|---|---|---|
| `replace` with a function replacement never calls it | **holds** (F01-F22: every function cell red, §1.3) | probe P1 (§6) |
| the replacement is `ToString`'d, "so it returns the function's **source text**" | **wrong in its consequence**: it returns `function () { [native code] }` — `Function.prototype.toString` does not return source text for user functions, which is the separate slot `#11-vm-function-tostring-source-text` (`memory/project_open-defer-slots.md:259`) | probe P1 F01 |
| `replaceAll` "probably the same" | **holds, and worse**: function never called (R01, R03, R07, R09, R11), and the RegExp pattern is `ToString`'d (R04-R06, R08) | probe P1 |
| GetSubstitution with a string replacement is a usable **control** | **wrong**: there is no GetSubstitution in the crate (`git grep -n -i 'substitution' efe8d05b -- crates/script/elidex-js/src` → no output); G01-G06, G08, G10, G11 red. Only templates that GetSubstitution leaves literal pass (G07, G09, G12, S19, S20) | `git grep` above; probe P1 G* |
| Home = 9b (String prototype surface) | **holds for the non-RegExp half only**; the RegExp-pattern half is §22.2.6.11, which slice-plan row 8b assigns to its `%Symbol.replace%` child (§5) | `docs/plans/2026-07-vm-p4-slice-plan.md:979`, `:983` |
| a RegExp `@@replace` path may exist | **does not**: `register_regexp_prototype` installs `test` / `exec` / `toString` only (`vm/globals_primitives.rs:194-201`); `WellKnownSymbols` allocates 7 symbols, no `replace` (`vm/well_known.rs:1578-1584`); `typeof Symbol.replace` → `undefined` (O01) | `sed -n 194,201p crates/script/elidex-js/src/vm/globals_primitives.rs` |
| Priority high, silently wrong | holds; and the most common in-the-wild form, `s.replace(/re/g, fn)`, is in the RegExp half S1 does not fix (OD-1) | — |

### §0.3 Premise errors (in the brief and the ledger)

1. "returns the function's source text" — it returns the native-code placeholder (§0.2 row 2).
2. "GetSubstitution with a string replacement, as a control" — GetSubstitution does not exist; the
   template cells are red, not a control.
3. "the RegExp `@@replace` path if one exists" — none exists; the RegExp handling is open-coded on
   `ObjectKind::RegExp` inside `native_string_replace` (`vm/natives_string.rs:477-487`) and is absent
   from `replaceAll`.
4. Implicit: "the slot is one unit". By measurement it is an umbrella (§2).

### §0.4 Open decisions — homed, not chosen here

- **OD-1 — when the RegExp half lands, and whether anything interim ships.** The RegExp half is
  §22.2.6.11 and its canonical home is the `%Symbol.replace%` child of umbrella 8b. That child consumes
  8d (the id), 8c (`groups`, §22.2.6.11 step 15.j) and 8a (`flags`, step 7) — 8b's row places those
  edges — and in practice SD-1 (§5), because every global zero-width cell hangs today. The options:
  **(A)** no interim; the RegExp half waits for 8b's child, which consumes S1's chokepoint unchanged.
  **(B)** rewrite the open-coded branch's body now onto S1's chokepoint. That ships a second
  implementation of the §22.2.6.11 loop for 8b's child to delete, and ships `groups` silently absent
  (8c). It is the strangler state "One issue, one way" forbids. **(C)** = (A), with the PM minting 8b's
  `%Symbol.replace%` child and ordering its prerequisites (8d → 8c, SD-1 → that child) right after S1.
  The design lens picks (A) or (C) over (B). Choosing between (A) and (C) is a cross-row ordering
  decision, so it is the **PM's**. S1 is the same PR under every option.
- **OD-2 — the file-wide `§21.1.3` → `§22.1.3` citation retag in `vm/natives_string.rs`.** Slice-plan
  row 9b places it on "whichever child lands first with `vm/natives_string.rs` in its touch set" and
  then requires 9b's derivation to rule on that device (row K precedent). S1 touches that file but is
  not a child 9b's derivation minted, so S1 does **not** take the retag. It writes every citation it
  adds or rewrites correctly (§22.1.3.19 / §22.1.3.20 / §22.1.3.19.1). The ruling stays with 9b's
  derivation.

## §1. Design

### §1.1 Implementation sites (census)

| Site | What it does today | Spec it stands in for |
|---|---|---|
| `vm/natives_string.rs:468` `native_string_replace` | `coerce_this_string` (`:473`), then an `ObjectKind::RegExp` brand test (`:477-487`) | §22.1.3.19 |
| `vm/natives_string.rs:488-489` | RegExp branch: `ToString(replaceValue)` unconditionally | §22.2.6.11 steps 5-6 (no functionalReplace) |
| `vm/natives_string.rs:495` | `set_regexp_last_index(ctx, re_id, 0)` **unconditionally** — for non-global too (O05) | §22.2.6.11 step 9 (global only) |
| `vm/natives_string.rs:496-509` | global loop: exec and splice per match, interleaved; zero-length advance `m.end() + 1` (`:505`), not AdvanceStringIndex | §22.2.6.11 steps 10-17 (collect first, then replace) |
| `vm/natives_string.rs:510-520` | non-global: one exec, splice | same |
| `vm/natives_string.rs:522-524` | comment cites `§21.2.5.10` (ES5-era number; the algorithm is §22.2.6.11) | — (OD-2 class) |
| `vm/natives_string.rs:530-545` | string pattern: `ToString` search, `ToString` replacement (`:532`), `find_u16`, splice | §22.1.3.19 steps 4-15 (**S1**) |
| `vm/natives_string_ext.rs:206` `native_string_replace_all` | `ToString` both arguments (`:214-215`), empty-search arm (`:219-233`), scan loop (`:234-247`) | §22.1.3.20 steps 4-17 (**S1**); step 3 absent |
| `vm/natives_regexp.rs:22` `run_regexp` | builtin exec; clamps `lastIndex` to the length (`:46`) instead of failing when it exceeds it | §22.2.7.2 RegExpBuiltinExec step 13.a (SD-1) |
| `vm/natives_regexp.rs:93` `native_regexp_exec` | result array with `index` / `input`, no `groups` (O09) | §22.2.7.2 steps 30-34 (8c) |
| `vm/globals_primitives.rs:62`, `:66` | install `replaceAll`, `replace` | — |
| `vm/globals_primitives.rs:194-201` | `RegExp.prototype` = `test`, `exec`, `toString`; no `[%Symbol.replace%]` | §22.2.6.11 (8b) |
| `vm/well_known.rs:1578-1584` | 7 well-known symbols, no `replace` / `match` | Table 1 (8d) |
| GetSubstitution | **absent** | §22.1.3.19.1 (**S1**) |

Reused, unchanged: `NativeContext::call_function` (`vm/native_context.rs:85`, Call),
`NativeContext::to_string_val` (`:71`, ToString), `ObjectKind::is_callable` (`vm/object_kind.rs:1586`,
IsCallable on an Object), `crate::wtf16::find_u16` (`crates/script/elidex-js/src/wtf16.rs:160`,
StringIndexOf from 0).

### §1.2 The spec, step by step (functionalReplace branch marked)

Numbers from `.claude/tools/webref aoid ecma262 <AO>` / `heading ecma262 <n> --exact`; steps from
`.claude/tools/webref body ecma262 <anchor>` (§6).

**§22.1.3.19 `String.prototype.replace ( searchValue, replaceValue )`.** 1-2 RequireObjectCoercible(this).
3 if `searchValue` is an Object: `GetMethod(searchValue, %Symbol.replace%)`; if not undefined, return
`Call(replacer, searchValue, « thisValue, replaceValue »)` — **the RegExp half enters here**. 4
`string = ToString(this)`. 5 `searchString = ToString(searchValue)`. **6 `functionalReplace =
IsCallable(replaceValue)`. 7 if false, `replaceValue = ToString(replaceValue)`** — so a callable
replacement is never `ToString`'d, and the template is `ToString`'d after `searchString`. 8-10
`position = StringIndexOf(string, searchString, 0)`; not-found → return `string` (no call, no
`ToString` of anything further). 11-12 preceding / following. **13 functionalReplace: `replacement =
ToString(Call(replaceValue, undefined, « searchString, 𝔽(position), string »))`** — `this` is
undefined, three arguments, the result `ToString`'d and never template-expanded. 14 else
`GetSubstitution(searchString, string, position, « », undefined, replaceValue)` (`!`: cannot throw
with `namedCaptures` undefined). 15 concatenate.

**§22.1.3.20 `String.prototype.replaceAll ( searchValue, replaceValue )`.** 1-2 as above. 3 if
`searchValue` is an Object: `IsRegExp` → if true, `Get(flags)`, RequireObjectCoercible, no `"g"` →
TypeError; then `GetMethod(searchValue, %Symbol.replace%)` → Call if present. 4-5 `ToString` this,
search. **6-7 functionalReplace as in replace.** 8-9 `advanceBy = max(1, searchLength)`. 10-12 **all
match positions are collected first** (`StringIndexOf(string, searchString, position + advanceBy)`).
13-15 for each position: preserved slice; **15.b functionalReplace: `ToString(Call(replaceValue,
undefined, « searchString, 𝔽(matchPosition), string »))`** — call *i* and its `ToString` both precede
call *i+1* (S17); 15.c else `GetSubstitution(searchString, string, matchPosition, « », undefined,
replaceValue)`. 16-17 tail.

**§22.2.6.11 `RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue )`** (8b's child; S1 only
fixes the interface it consumes). 3 `ToString(string)`. **5-6 functionalReplace as above.** 7 `flags =
ToString(Get(regexp, "flags"))`. 9 global → `Set(lastIndex, 0)`. 10-12 **collect every RegExpExec
result first**; on an empty match, `AdvanceStringIndex(string, lastIndex, fullUnicode)`. 15 for each
result: captures (each `ToString`'d unless undefined), `namedCaptures = Get(result, "groups")`; **15.k
functionalReplace: `replacerArgs = « matched » + captures + « 𝔽(position), string »`, plus
`namedCaptures` if not undefined; `ToString(Call(replaceValue, undefined, replacerArgs))`**; 15.l else
`namedCaptures = ToObject(namedCaptures)` if not undefined, then `GetSubstitution(...)` (`?`). 15.m a
result whose position moved backwards is skipped.

**§22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures,
replacementTemplate )** — a left-to-right scan of the template: `$$` → `$`; `` $` `` → `string[0,
position)`; `$&` → matched; `$'` → `string[min(position + len(matched), len))`; `$` + 1-2 digits →
two-digit index if ≤ capture count, else one digit then a literal digit; `1 ≤ index ≤ m` →
capture (undefined → empty), otherwise the `ref` text literally (so `$0`, `$00` and every `$n` with
no captures are literal); `$<`: if no `>` or `namedCaptures` undefined → literal `$<`, else
`ToString(Get(namedCaptures, groupName))` (undefined → empty) — **the one throwing arm**; any other
code unit is literal. No functionalReplace branch: the function arm never reaches it.

### §1.3 Measured matrix (actual at `efe8d05b` vs spec)

Probe P1 (§6): 88 cases, each run in a fresh `Vm` through a throwaway test, strict mode. The **Spec**
column is the value the §1.2 steps produce, and it is the only expectation any cell asserts.
**HANG** = no result within 20 s (process killed).
[evidence] The Spec values were first obtained by running the same case text under Node 26.13 and then checked by hand against the §1.2 steps, row by row. The Node run is provenance only, not an acceptance input; a review that finds a row where the steps disagree corrects the row to the steps.

| Id | Case (abridged) | Spec | Actual |
|---|---|---|---|
| F01 | `'a'.replace('a', fn→'z')` | `z` | `function () { [native code] }` |
| F02 | `'abc'.replace('b', fn args)` | `a[b\|1\|abc]c` | `afunction () {…}c` |
| F03 / R07 | `typeof this` in replacer (replace / replaceAll) | `undefined` | not called (`unset`) |
| F04 / F05 / F06 | return `42` / `{toString→'T'}` / `undefined` | `42` / `T` / `undefined` | function text |
| F07 / F15 / R02 | no match → calls | `0:abc` | `0:abc` (green) |
| F08 / R03 | empty search, positions | `[0]ab` / `[0]a[1]b[2]` | function text spliced |
| F09 / F11 / F12 | RegExp: args, captures, unmatched capture | `a[b\|1\|abc]c` / `x[12\|1\|2\|1\|x12y]y` / `[a\|undefined\|0\|ab]b` | function text |
| F10 | `/b/g` order, positions | `1,3:aXaX` | not called |
| F13 / F14 | named groups → `groups` argument / plain groups → no `groups` argument | `xobject,1,2,6y` / `xlen5,stringy` | function text |
| F16 | RegExp: `this` | `undefined` | not called |
| F17 / R09 / S10 | replacer throws (string; replaceAll, throw on call 2) | `RangeError:boom` / `2:RangeError` | `nothrow` |
| F18 | `/a/g`, throw on call 2 | `2:RangeError:boom` | `nothrow` |
| F19 / S18 | re-entrant `replace` inside replacer (RegExp outer / string outer) | `axByc` | function text |
| F20 | replacer resets `re.lastIndex = 0` on a `/g` regexp | `2:aXaX` (collect first) | not called |
| F21 | replacer runs `replace` with the same `/g` regexp | `accacc:0` | function text |
| F22 | `string` argument per call | `aXbXc,aXbXc:a-b-c` | not called |
| R01 / R11 | replaceAll args per match / `'aaaa'.replaceAll('aa', fn p)` | `a[b\|1\|abab]a[b\|3\|abab]` / `[0][2]` | function text |
| R04 / R05 | `'abab'.replaceAll(/b/g, 'X' / fn)` | `aXaX` / `a[1]a[3]` | `abab` (searches `"/b/g"`) |
| R06 | `'abab'.replaceAll(/b/, 'X')` | TypeError | `abab` |
| R08 | replaceAll `/(?<d>\d)/g` + groups | `x<1>y<2>` | `x1y2` |
| R10 | `'abab'.replaceAll('b', '[$&]')` | `a[b]a[b]` | `a[$&]a[$&]` |
| G01-G04 | `'abc'.replace('b', '[$&]' / '[$`]' / "[$']" / '[$$]')` | `a[b]c` / `a[a]c` / `a[c]c` / `a[$]c` | template verbatim |
| G05 / G06 | `$2$1` / `$<b>$<a>` (RegExp) | `x21y` | template verbatim |
| G07 / G12 / S19 / S20 | `$1` (string pattern) / lone `$` / `$<x>` / `$01$10$00` (no captures) | literal | literal (green) |
| G08 / G11 | `/(a)/` + `$10` / `/(b)/` + `$0\|$01\|$9` | `a0b` / `a$0\|b\|$9c` | `$10b` / verbatim |
| G09 | `/b/` + `$<x>`, no named groups | `a$<x>c` | green |
| G10 | `/b/g` + `[$&]` | `a[b]a[b]` | verbatim |
| S01 / S02 / S15 | replacer = `class {}` / `String.prototype.toUpperCase` / `Math.max` | TypeError / TypeError / `NaN` | function text |
| S03 | replacer = `fn.bind('B')` returning `this` | `B` | `function bound () {…}` |
| S04 | replacer returns `'$&'` | `$&b` (no expansion) | function text |
| S05 / S16 | replacer with own `toString` | `z`; `toString` **not** called (`:z`) | `T`; called (`t:T`) |
| S07 / S11 / S14 | `typeof position` / `arguments.length` / `string` arg with object `this` | `number` / `3` / `string:ab` | function text |
| S08 / S09 | `'ab'.replaceAll('', '[$`]')` / `'abab'.replaceAll('b', "[$`\|$']")` | `[]a[a]b[ab]` / `a[a\|ab]a[aba\|]` | verbatim |
| S12 | nested template `replace` inside replacer | `aa[X]aa` | function text |
| S17 | replaceAll: call *i* and its `ToString` before call *i+1* | `c1,t1,c2,t2:12` | not called |
| O02 / S06 / S13 | `ToString` order when not callable (search, then replacement) | `sr` / `sr` / `r` | green |
| O11 | callable replacement with object search: `ToString(search)` only | `sf:z` | `s:function () {…}` |
| O12 | `replace.call(12312, '2', fn)` | `1string12312312` | `1function () {…}312` |
| O01 / O08 | `typeof Symbol.replace` / user `[Symbol.replace]` object | `symbol` / `custom:abc:z` | `undefined` / `abc` |
| O03 / O04 / O06 | `replace('a')`; sticky miss; `/a/g` `lastIndex` after | `undefined`; `aab:0`; `bb:0` | green |
| O05 | `/a/` with `lastIndex = 5`, after `replace` | `5` | `0` |
| O09 | `/(?<a>x)/.exec('x').groups` | `object:x` | `undefined` |
| O07 / O10 / H01-H06 | any global regexp with a zero-width match at the end: `'ab'.replace(/(?:)/g, …)`, `/x*/g`, `'a'.replace(/$/g, '!')`, `'ab'.match(/(?:)/g)`, `/(?:)/gu` on a surrogate pair | `-a-b-`, `-a-b-`, `a!`, `3`, `4` | **HANG** |
| X01 | `/$/g`, `lastIndex = 5`, `exec('a')` | `null:0` | `match@1:1` (SD-1's root) |
| X02 | non-global `exec` reads `lastIndex` (`valueOf` side effect) | `v` | `` (not read) |

Of the 88 cases, 72 are red and 16 green. The green ones are the no-call cases (F07, F15, R02), the
literal templates (G07, G09, G12, S19, S20), the `ToString`-order cases with no callable replacement
(O02, S06, S13), O03 / O04 / O06, and the two class-call controls C01 / C02 (`(class {})()` and
`[1].map(class {})` throw TypeError). No green cell calls a replacer or expands a `$` form.

### §1.4 One canonical form

The three algorithms in §1.2 make the same per-match decision: replacement = functionalReplace ?
`ToString(Call(f, undefined, args))` : `GetSubstitution(...)`. The canonical shape is one chokepoint
used by all three, not a branch copied into each native.

- `Replacer` (NEW), in `vm/string_replace.rs` (NEW): `enum Replacer { Function(ObjectId),
  Template(StringId) }`. `resolve_replacer` (NEW) is §22.1.3.19 steps 6-7 / §22.1.3.20 steps 6-7 /
  §22.2.6.11 steps 5-6: an Object whose `kind.is_callable()` → `Function`; anything else →
  `Template(ToString(v))`. It is called **after** `ToString(searchValue)` in the two String methods (O02
  / S06 / S13 order) and never `ToString`s a callable (O11 / S16).
- `MatchRecord` (NEW): `{ matched: StringId, position: usize, captures: Vec<Option<StringId>>,
  named_captures: Option<ObjectId> }`. S1 always builds `captures: vec![]` and `named_captures:
  None`. 8b's `%Symbol.replace%` child builds the full record from each RegExpExec result.
- `replacement_for_match` (NEW)`(ctx, &Replacer, &MatchRecord, string: StringId) ->
  Result<Vec<u16>, VmError>`. The `Function` arm builds « matched, …captures (undefined for `None`),
  𝔽(position), string [, namedCaptures] » and calls `ctx.call_function(f, JsValue::Undefined, &args)`.
  Its `?` propagates a throw (F17 / S10) and its result goes through `to_string_val` exactly once,
  with no template expansion (S04). The `Template` arm calls `get_substitution`.
- `get_substitution` (NEW): §22.1.3.19.1 in full, over WTF-16 slices, returning `Result` because the
  `$<` arm is a `Get`. Its arms with captures or `namedCaptures` are unreachable from JS until 8b's
  child supplies a record that carries them. They are pinned from Rust (T-SUB-UNIT) under the
  slice-plan §5 prerequisite rule (the same licence row 8d uses), so 8b's child consumes the helper
  without reopening it. The digit arm **is** JS-reachable in S1 with an empty capture list (`$0`,
  `$01`, `$10`, `$00` → literal, S20).
- `native_string_replace`'s string branch (`vm/natives_string.rs:530-545`) becomes §22.1.3.19 steps
  4-15 over these. `native_string_replace_all` becomes §22.1.3.20 steps 4-17. It **collects the
  positions first** (`advanceBy = max(1, searchLength)`), which subsumes today's special empty-search
  arm (`vm/natives_string_ext.rs:219-233`), and then makes one `replacement_for_match` per position in
  order, so the S17 interleaving holds.

**GC.** Strings are pool-permanent (`vm/gc/mod.rs:90`), so `StringId`s held across a replacer call are
safe. The replacer `ObjectId` is an argument of the native call and is therefore reachable from the
caller's frame for the whole native. S1 holds no other object across a call. `named_captures` is the
one object a `MatchRecord` holds across calls; it is always `None` in S1. Rooting it across the
replacer calls of later records is an obligation of 8b's child (§5).

**Re-entrancy.** The two String natives hold no VM-side state across the call, only Rust locals (the
position list, the accumulated `Vec<u16>`), so a nested `replace` / `replaceAll` is independent (S12,
S18). The RegExp form of re-entrancy (F20, F21: a shared `lastIndex`) is what "collect every result
first" in §22.2.6.11 steps 10-12 protects. It is 8b's child's obligation and cannot arise on the
string path.

**Layering.** These are ECMAScript built-ins, the VM core's own surface; no DOM / engine-independent
crate is involved. `vm/string_replace.rs` sits beside the natives and does not live in `host/`.

## §2. Edge density, by measurement

Criterion (slice-plan §5's head): an axis counts only on the evidence of a failure that occurs while
the others pass. The axes below are counted over the slot as registered; §1.3 gives the base evidence
and §4.3 the mutants.

| Axis | Invariant | Evidence that it fails alone |
|---|---|---|
| **A1** replacer call protocol | IsCallable selects the branch before any `ToString(replaceValue)`; `Call(f, undefined, « … »)`; result `ToString`'d once, not template-expanded; abrupt completion propagates | O11 / S16 red while the not-callable order cells O02 / S06 / S13 are green; M2-M5, M11 each fail only A1 cells |
| **A2** template substitution (GetSubstitution) | each `$` form per §22.1.3.19.1 | G01-G04 red with no function involved; G07 / G12 / S19 / S20 green (the literal arms work by accident); M8-M10 |
| **A3** string-pattern match enumeration | positions collected first, `advanceBy = max(1, len)`, `position` is the argument and the `` $` `` / `$'` anchor | R11 / S09 depend on the position set while A1 / A2 are fixed; M6, M7, M12 |
| **A4** RegExp result collection | RegExpExec loop; `lastIndex` reset only if global; AdvanceStringIndex on empty match; every result collected before any replacer call | O05 red with a plain template; H01 / H06 hang with a plain template; F20 / F21 |
| **A5** named captures | `groups` from RegExpBuiltinExec reaches the replacer and `$<name>` | O09 red on `exec` alone; F13, G06, R08 |
| **A6** protocol dispatch | `GetMethod(%Symbol.replace%)`; `replaceAll`'s IsRegExp + `"g"` TypeError | O01 / O08 red with no replacement at all; R04-R06 |

**Six measured intersecting axes over the slot**, which is well past the threshold of three. There is
also no single canonical algorithm for the slot: it spans three spec algorithms owned by two umbrella
rows. So under CLAUDE.md "Edge-dense work" the slot is an **umbrella**. It is split:

- **S1 (this memo)** — A1 + A2 + A3 over the non-RegExp path, plus the chokepoint (§1.4).
- **A4 + A5 + A6 and the RegExp form of A1 / A2** → the `%Symbol.replace%` child of umbrella 8b (the
  §22.2.6.11 algorithm), the String-side `GetMethod` child of 8b (step 3 of both methods), 8c
  (`groups`), 8d (ids), and the `replaceAll` child of 9b (its step 3). Each edge is placed in §5. The
  A4 hang is a separate defect with its own owner (§5 SD-1).

### §2.1 Coupled invariants of S1 (Pre-condition #3)

S1 still spans three axes, so its coupled invariants and their pairwise intersections are listed:

- **I1 (A1)** — a callable `replaceValue` is called once per match, in order, with `this` undefined
  and « matched, position, string », its result `ToString`'d exactly once; a non-callable one is
  `ToString`'d exactly once, after the search string.
- **I2 (A2)** — a template is expanded per §22.1.3.19.1, against the empty capture list and undefined
  `namedCaptures` on this path.
- **I3 (A3)** — the position list is exactly the non-overlapping StringIndexOf sequence with
  `advanceBy = max(1, searchLength)`, computed before any replacement.

Intersections:

- **I1 × I2** — the selection point: `resolve_replacer` decides the arm once, before the loop, so a
  function result is never template-expanded (S04, M4), and a callable is never `ToString`'d (S16, M5).
- **I1 × I3** — position *i*'s call and its `ToString` both precede call *i+1* (S17, M14), and a throw
  at call *k* stops the loop with no call *k+1* (S10, M11). The `position` argument is I3's value
  (R01, R11, M3).
- **I2 × I3** — `` $` `` and `$'` read `position` and `position + len(matched)` from I3's list
  (S08, S09, M9), including the empty-search boundary positions 0 and `len` (S08).

### §2.2 Terminality of S1

S1 is **terminal**. Its first ground is CLAUDE.md "Edge-dense work" (c): it is a narrowly scoped
per-PR slice under the approved VM P4 umbrella (#506), homed at slice-plan row 9b, and it is
plan-reviewed before implementation (this memo goes to `/elidex-plan-review`). Its second ground is
the shared chokepoint. A1 and A2 are the two arms of one decision that `resolve_replacer` /
`replacement_for_match` make, and A3 feeds both arms the same position. A split along any axis would
land the chokepoint with one arm stubbed, so a second PR would rewrite the same loop in the same two
natives — the decision tax "One issue, one way" exists to remove. S1 carries **no** RegExp-side
invariant: it calls no RegExpExec, reads no `lastIndex` and constructs no `groups`.

**Re-slice trigger**: the post-implementation mutant run (§4.3) finds a mutant that fails alone and
breaks an invariant none of I1-I3 owns.

## §3. Spec coverage map

Section numbers from `.claude/tools/webref aoid ecma262 <AO>` (GetSubstitution, StringIndexOf,
IsCallable, Call, IsRegExp, RegExpExec, RegExpBuiltinExec, AdvanceStringIndex) and `.claude/tools/webref
heading ecma262 <n> --exact` (the three method headings); step text from `.claude/tools/webref body
ecma262 <anchor>`. Skeleton from `.claude/tools/webref coverage-map` (§6). Rows marked **→ owner**
are not S1's touch. They are listed so the full-enum column covers every branch of the algorithms
S1 shares a chokepoint with.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 3 | Object with `%Symbol.replace%` → Call(replacer) | **→ 8b String-side `GetMethod` child** (today: `ObjectKind::RegExp` brand, `vm/natives_string.rs:477-487`, unchanged) | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 4-5 | ToString(this), ToString(searchValue), in that order | `vm/natives_string.rs:473`, `:531` | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 6-7 | IsCallable true → no ToString; false → ToString(replaceValue) | `resolve_replacer` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 9-10 | not-found → return string, no call | string branch, `find_u16` | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 13 | functionalReplace: Call(replaceValue, undefined, « searchString, position, string »), ToString | `replacement_for_match` (NEW) `Function` arm | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 14 | template: GetSubstitution with « » and undefined | `replacement_for_match` (NEW) `Template` arm | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 3.a-3.b | IsRegExp true, flags without `"g"` → TypeError | **→ 9b `replaceAll` child** (consumes 8d's `%Symbol.match%`) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 3.c-3.d | `%Symbol.replace%` method → Call | **→ 9b `replaceAll` child**, via 8b's String-side `GetMethod` child | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 4-7 | ToString order; functionalReplace selection | `vm/natives_string_ext.rs:206` → `resolve_replacer` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 9-12 | advanceBy = max(1, len); all positions collected first (empty search included) | `native_string_replace_all` position list (replaces `:219-247`) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 15.b | functionalReplace per position, in order | `replacement_for_match` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 15.c | template per position | `replacement_for_match` (NEW) → `get_substitution` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 16-17 | tail after last match | `native_string_replace_all` | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.b-5.e | `$$`, `` $` ``, `$&`, `$'` (tail clamped to length) | `get_substitution` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.f | `$n` / `$nn`: two-digit fallback, in-range capture (undefined → empty), out-of-range → literal | `get_substitution` (NEW); in-range arm Rust-pinned | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.g | `$<`: no `>` or namedCaptures undefined → literal; else ToString(Get(namedCaptures, name)), undefined → empty, throw propagates | `get_substitution` (NEW); Get arm Rust-pinned | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.h | any other code unit literal | `get_substitution` (NEW) | ✓ | yes |
| ECMA-262 §6.1.4.1 StringIndexOf ( string, searchValue, fromIndex ) | 1-4 | empty search → fromIndex (≤ len); not-found | `crate::wtf16::find_u16` over `&s[from..]` | ✓ | yes |
| ECMA-262 §7.2.3 IsCallable ( arg ) | 1-3 | non-Object → false; `[[Call]]` → true (class constructors included) | `ObjectKind::is_callable` (`vm/object_kind.rs:1586`) | ✓ | yes |
| ECMA-262 §7.3.13 Call ( func, thisValue [ , argList ] ) | 1-3 | callable → `[[Call]]`; a class constructor's `[[Call]]` throws TypeError (S01) | `NativeContext::call_function` (`vm/native_context.rs:85`) | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 5-6, 15.k-15.l | functionalReplace / GetSubstitution with captures + namedCaptures | **→ 8b `%Symbol.replace%` child**, consuming `replacement_for_match` (NEW) unchanged | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 7-12 | flags read; global `lastIndex` reset; collect all results; empty-match advance | **→ 8b `%Symbol.replace%` child** (8a `flags`) | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 15.j | `namedCaptures = Get(result, "groups")` | **→ 8b `%Symbol.replace%` child** (8c) | ✓ | yes |
| ECMA-262 §7.2.6 IsRegExp ( arg ) | 1-4 | `%Symbol.match%` read before the `[[RegExpMatcher]]` test | **→ 9b `replaceAll` child** (8d id) | ✓ | yes |
| ECMA-262 §22.2.7.1 RegExpExec ( regexp, string ) | 1-3 | user `exec` vs builtin | **→ 8b `%Symbol.replace%` child** | ✓ | yes |
| ECMA-262 §22.2.7.2 RegExpBuiltinExec ( regexp, string ) | 2, 13.a | `lastIndex` read unconditionally; `lastIndex > length` → fail (and reset if g/y) | **→ SD-1 slot** (`vm/natives_regexp.rs:46`) | ✓ | yes |
| ECMA-262 §22.2.7.3 AdvanceStringIndex ( string, index, unicode ) | 1-5 | unicode advance past a surrogate pair | **→ 8b `%Symbol.replace%` child** (today `vm/natives_string.rs:505`) | ✓ | yes |

**Breadth**: K=1 (ecma262), M=27 rows, of which 18 are S1 touches. The `SPLIT-RECOMMENDED` verdict
preflight may print counts the → owner rows, and that split is the one §2 already made. The S1 single-PR
verdict is §2.2's.

### §3.1 User-input audit

Every value S1 reads is script-controlled: the receiver, the search value, the replacement (callable
or template), and whatever the replacer returns. After S1, two places run user code. The replacer runs
inside `replacement_for_match` with `this` undefined and no VM state held across the call. A
non-callable replacement and the search value run user `toString` through `to_string_val`, once each,
in spec order. Pre-existing adjacent exposure that S1 neither adds nor removes: the RegExp branch at
`vm/natives_string.rs:477-528` reaches the SD-1 hang from any script with a global zero-width regexp.
S1 reaches no RegExp code.

## §4. Tests

### §4.1 Cells (S1)

New file `crates/script/elidex-js/src/vm/tests/tests_string_replace.rs` (NEW), registered in
`vm/tests/mod.rs` (204 lines at `efe8d05b`). `tests_string_complement.rs` (931 lines) holds three
`replaceAll` cells (`:196-207`) and is not grown. Those three stay green, and
`tests_string.rs:71-75` / `tests_m4_10_2.rs:292-297` (plain replacements) stay green. Each cell runs
in a fresh `Vm` through the `eval_string` helper (`vm/tests/helpers.rs`). Base values are probe P1's
(§1.3).

| Cell | Asserts (spec value) | Base | Mutant it kills |
|---|---|---|---|
| T-FN-CALL | F01 `z`; F04 `42`; F05 `T`; F06 `undefined`; S05 `z` | red | M1 |
| T-FN-ARGS | F02 `a[b\|1\|abc]c`; S11 `a3c`; S07 `anumber`; S14 `astring:ab`; O12 `1string12312312`; R01 `a[b\|1\|abab]a[b\|3\|abab]` | red | M1, M3, M12 |
| T-FN-THIS | F03 / R07 `undefined`; S03 `B`; S02 TypeError; S01 TypeError; S15 `NaN` | red | M2 |
| T-FN-NOCALL | F07 / R02 `0:abc` | green (stays) | M13 |
| T-FN-NOSUBST | S04 `$&b` | red | M4 |
| T-SEL | O11 `sf:z`; S16 `:z` (callable never `ToString`'d); O02 / S06 `sr`, S13 `r` (not-callable order) | O11 / S16 red, the rest green | M5 |
| T-POS | R11 `[0][2]`; F08 `[0]ab`; R03 `[0]a[1]b[2]` | red | M6, M7 |
| T-ORDER | S17 `c1,t1,c2,t2:12` | red | M14 |
| T-ABRUPT | F17 / R09 `RangeError:boom`; S10 `2:RangeError` | red | M11, M14 |
| T-REENTRY | S12 `aa[X]aa`; S18 `axByc` | red | M1 |
| T-SUB | G01 `a[b]c`; G02 `a[a]c`; G03 `a[c]c`; G04 `a[$]c`; R10 `a[b]a[b]`; S08 `[]a[a]b[ab]`; S09 `a[a\|ab]a[aba\|]` | red | M8, M9 |
| T-SUB-LIT | G07 `a$1c`; G12 `a$c`; S19 `a$<x>c`; S20 `a$01$10$00c` | green (stays) | M10b |
| T-SUB-UNIT | Rust, calling `get_substitution` (NEW) directly: captures `[Some("b")]` → `$1` = `b`, `$01` = `b`, `$10` = `b0`, `$2` literal, `$0` literal; `[None]` → `$1` empty; `namedCaptures` an object `{n: 'N'}` → `$<n>` = `N`, `$<m>` empty, `$<n` literal; a getter that throws → `Err`; `matched` longer than the tail → `$'` empty | (NEW helper) | M10, M10b, M15 |

### §4.2 Known-deviation cells (out of S1's scope)

These are written into the same file now, with the **spec** value asserted, each `#[ignore = "known
deviation: <owner>"]`. The owner's acceptance is to remove the `ignore`. None of them asserts today's
wrong value: pinning that would bless the defect.

| Cells | Owner |
|---|---|
| F09, F10, F11, F12, F14, F15 (green today; keep as the no-call control), F16, F18, F19, F20, F21, F22, G05, G08, G10, G11 | the `%Symbol.replace%` child of umbrella 8b (§22.2.6.11) |
| F13, G06, R08 | that same child, after 8c (`groups`) |
| O05 | that same child (step 9: global only) |
| O01, O08 | 8d (ids) + 8b's String-side `GetMethod` child |
| R04, R05, R06 | the `replaceAll` child of umbrella 9b (step 3), after 8d and 8b's `GetMethod` child |
| O07, O10, H01-H06, X01 | SD-1. ⚠ These **hang** today, so they must stay `#[ignore]` until SD-1 lands, and `cargo test -- --ignored` hangs on them until then. They go in under SD-1's slot rather than here if the plan-review prefers no hanging ignored cell in tree |
| X02 | umbrella 8's §22.2.7.2 child (step 2) |

### §4.3 Mutants

These run on the final head only, sequentially (at most 2 parallel builds). Each is recorded as
killed-by-cell. A mutant no cell kills is a missing cell, added before the PR leaves review.

- **M1** `resolve_replacer` always returns `Template` (today's behaviour).
- **M2** the replacer is called with `this` = the receiver string.
- **M3** the `position` argument is dropped (« matched, string »).
- **M4** a function result is passed through `get_substitution`.
- **M5** `ToString(replaceValue)` runs before the IsCallable test, and its value is discarded for a
  callable.
- **M6** replaceAll's `advanceBy` = 1 (overlapping positions: R11 → `[0][1][2]`-shaped).
- **M7** replaceAll's `advanceBy` = `searchLength` (empty search does not advance). This one is killed
  by a hang, so it is run under `timeout`.
- **M8** `$$` is not collapsed.
- **M9** `$'` anchors at `position` instead of `position + len(matched)`.
- **M10** the two-digit fallback of step 5.f.vi is removed (killed only by T-SUB-UNIT `$10`).
- **M10b** an out-of-range `$n` expands to the empty string instead of the literal.
- **M11** an abrupt replacer completion is swallowed (the original match text is kept).
- **M12** replaceAll calls the replacer for the first position only.
- **M13** the replacer is called once even when nothing matches.
- **M14** replaceAll calls every replacer first and `ToString`s the results afterwards.
- **M15** `get_substitution`'s `$<` arm maps a throwing `Get` to the empty string.

## §5. Relation to rows 8b, 8d, 9b; hand-offs; sibling defects

- **9b** (`docs/plans/2026-07-vm-p4-slice-plan.md:983`, `#11-vm-builtin-prototype-static-sweep`). S1
  discharges the non-RegExp halves of the §22.1.3.19 and §22.1.3.20 members. When 9b's derivation
  runs, it counts those halves as landed and mints the `replaceAll` child for step 3 only. That child
  carries the 8d edge and the child-to-child edge onto 8b's String-side `GetMethod` child, both of
  which 9b's row already places. The OD-2 retag ruling stays 9b's.
- **8b** (`:979`, UMBRELLA). Its `%Symbol.replace%` child is the canonical home of the RegExp half
  (§22.2.6.11). It **consumes S1's `Replacer` / `MatchRecord` / `replacement_for_match` /
  `get_substitution` unchanged**, and builds the full `MatchRecord` (captures `ToString`'d unless
  undefined, `namedCaptures` via `ToObject` on the template arm only, per step 15.l.i). Obligations
  S1 hands it: collect every result before any replacer call (F20, F21); `Set(lastIndex, 0)` only when
  global (O05); AdvanceStringIndex on an empty match (`vm/natives_string.rs:505` today); the
  step 15.m backwards-position skip; rooting `named_captures` across the replacer calls of later
  records (§1.4 GC); deleting the open-coded branch at `vm/natives_string.rs:477-528` and its
  `§21.2.5.10` comment. Its String-side `GetMethod` child replaces the `ObjectKind::RegExp` brand test
  at `:477-487` with step 3's `GetMethod`. S1 adds **no** new brand site.
- **8d** (`:978`, terminal). No S1 dependency: S1 reads no `%Symbol.*%` id, because every S1 path is
  one where step 3 finds no replacer. Its ids gate 8b's children and 9b's `replaceAll` child, not S1.
- **8c** (`:980`). `groups` (O09) gates the `namedCaptures` arm reaching JS; until then that arm is
  Rust-pinned (T-SUB-UNIT).
- **SD-1 — RegExpBuiltinExec `lastIndex` overrun → infinite loop** (new; not this slot's; **needs a
  slot**). `run_regexp` clamps `lastIndex` to the subject length (`vm/natives_regexp.rs:46`)
  where §22.2.7.2 step 13.a fails the match (resetting `lastIndex` to 0 when global or sticky). With
  the `m.end() + 1` advance at `vm/natives_string.rs:505` / `:604`, every global regexp with a
  zero-width match at the end of the subject loops forever in `replace` and `match`: H01
  `'ab'.replace(/(?:)/g, '-')`, H05 `/x*/g`, H06 `'a'.replace(/$/g, '!')`, H04 `'ab'.match(/(?:)/g)`.
  User `exec` loops are affected too (X01: `/$/g` with `lastIndex = 5` matches at 1 instead of
  returning null). This is a script-reachable hang of the renderer's main thread. Its home is umbrella
  8's §22.2.7 clause (RegExpBuiltinExec). It is a prerequisite of 8b's `%Symbol.replace%` child's
  global cells. Recommended priority: above this slot, because it is loud only as a hang, and the trim
  idiom `s.replace(/\s*$/g, '')` reaches it. Not probed separately: whether the `u`-flag form also
  mis-advances once the clamp is fixed (H02 hangs on the clamp first).
- **SD-2 — non-global `replace` resets `lastIndex`** (O05, `vm/natives_string.rs:495`) → 8b's
  `%Symbol.replace%` child (listed above). **SD-3 — non-global `exec` never reads `lastIndex`** (X02;
  §22.2.7.2 step 2 reads it unconditionally) → umbrella 8's §22.2.7.2 child.
- **Already slotted, observed here**: `#11-vm-function-tostring-source-text` (the placeholder text
  in §0.2); `split` / `search` `ToString` the RegExp (X03, 8b's row evidence).
- **Not observed as a defect**: a class constructor called from a native throws TypeError (probe C01
  / C02), so S01's TypeError needs nothing beyond `call_function`.

## §6. Re-derivation

Probe P1 is a throwaway test module, never committed. It was registered temporarily in
`vm/tests/mod.rs` through `#[path]`, then reverted with `git checkout`. It reads tab-separated
`id<TAB>body` lines from `$PROBE_CASES`, wraps each body as `(function(){ try { var __r = (function(){
<body> })(); return typeof __r + ':' + String(__r); } catch (e) { return 'THROW:' + e.name + ':' +
e.message; } })()`, evaluates each in a fresh `Vm`, and prints `PROBE<TAB>id<TAB>result`.
[evidence] The §1.3 cross-check ran the same bodies as `new Function('"use strict";' + body)()` under Node.
The hanging cases ran one per
process under `timeout 20` against the built test binary. Case text and output were kept in the
session scratchpad.

```sh
git grep -n 'fn native_string_replace\|fn native_string_replace_all' efe8d05b -- crates/script/elidex-js/src
sed -n 468,546p crates/script/elidex-js/src/vm/natives_string.rs
sed -n 206,251p crates/script/elidex-js/src/vm/natives_string_ext.rs
sed -n 40,50p crates/script/elidex-js/src/vm/natives_regexp.rs
sed -n 194,201p crates/script/elidex-js/src/vm/globals_primitives.rs
sed -n 1578,1584p crates/script/elidex-js/src/vm/well_known.rs
git grep -n -i 'substitution' efe8d05b -- crates/script/elidex-js/src      # no output
.claude/tools/webref heading ecma262 22.1.3.19 --exact
.claude/tools/webref heading ecma262 22.1.3.20 --exact
.claude/tools/webref heading ecma262 22.2.6.11 --exact
.claude/tools/webref aoid ecma262 GetSubstitution   # likewise StringIndexOf IsCallable Call IsRegExp RegExpExec RegExpBuiltinExec AdvanceStringIndex
.claude/tools/webref body ecma262 sec-string.prototype.replace     # also sec-string.prototype.replaceall, sec-regexp.prototype-%symbol.replace%, sec-getsubstitution, sec-regexpbuiltinexec
.claude/tools/webref coverage-map ecma262 22.1.3.19 ecma262 22.1.3.20 ecma262 GetSubstitution ecma262 22.2.6.11 ecma262 StringIndexOf ecma262 IsCallable ecma262 Call ecma262 IsRegExp ecma262 RegExpExec ecma262 AdvanceStringIndex
```

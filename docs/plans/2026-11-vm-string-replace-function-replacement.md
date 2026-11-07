# `#11-vm-string-replace-function-replacement` — functional replacement and GetSubstitution for `String.prototype.replace` / `replaceAll` (umbrella split + per-PR plan-memo for S1)

**Slot**: `#11-vm-string-replace-function-replacement` (registered in the live ledger 2026-11-05, PM 414; the
live entry of that id in `memory/project_open-defer-slots.md`; re-scoped as an umbrella by PM 434; stays open
after S1 lands, PM 462, §7). **Owner**: VM P4. **Home as registered**: slice-plan umbrella **9b** (`String`
prototype surface); PM 434 keeps the non-RegExp half at 9b and homes the RegExp half at 8b, in the child
**`#11-vm-regexp-symbol-replace`** (id assigned by PM 462). **Order (PM 434)**: SD-1
(`#11-vm-regexp-builtin-exec-lastindex-overrun`, **landed and closed**: #605 `b86671a5`) → **S1** → 8d → 8c →
`#11-vm-regexp-symbol-replace` (§0.4 OD-1). **Baseline**: `47b6c934` (origin/main, merged into this branch;
it contains `b86671a5`). Every `file:line` below is at `47b6c934` unless it names another revision; the
probe's Actual column (§1.3) is at `efe8d05b`, the revision it ran on. §6 re-derives every anchor at S1's
implementation base. Paths are relative to `crates/script/elidex-js/src/` unless they start with `crates/`
or `docs/`.

**The defect, stated as a defect.** Neither `String.prototype.replace` nor `String.prototype.replaceAll`
has a functionalReplace branch, and neither has GetSubstitution. Both natives `ToString` the
replacement unconditionally, so a function replacement is never called — its `ToString` text is
spliced in, `function () { [native code] }` for a user function — and a template replacement is
spliced in verbatim, so `$&`, `` $` ``, `$'`, `$$`, `$n` and `$<name>` are never expanded. Both are
silent in release and debug builds. `replaceAll` additionally never treats a RegExp as a RegExp: it
`ToString`s it and searches for that text (`'abab'.replaceAll(/b/g, 'X')` → `abab`), and a
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
  `ObjectKind::RegExp`): `native_string_replace`'s string branch, `vm/natives_string.rs:528-543`.
- **§22.1.3.20** `String.prototype.replaceAll` steps 4-17, same condition:
  `native_string_replace_all`, `vm/natives_string_ext.rs:206-252`, whole body except that its step 3
  stays absent (below, with its in-code marker, §1.4).
- **§22.1.3.19.1** GetSubstitution, once, with its full spec signature (`captures`, `namedCaptures`),
  in a new module `vm/string_replace.rs` (NEW), together with the per-match replacement chokepoint
  both methods — and, later, `%RegExp.prototype%[%Symbol.replace%]` — go through (§1.4).

**S1 — not in scope** (each one homed in §5):

- The RegExp branch of `native_string_replace` (`vm/natives_string.rs:475-526`). It keeps today's
  behaviour, function `ToString` included, until `#11-vm-regexp-symbol-replace` replaces it (§0.4 OD-1,
  ratified as (C)). S1 does **not** add a functional branch to it, nor move it onto S1's chokepoint: PM 434
  rejected that interim (option (B)), because it would be a second implementation of §22.2.6.11's loop
  beside the one that child must ship. S1 adds only an in-code marker comment there (§1.4).
- `replaceAll` step 3 (IsRegExp, the `flags`/`"g"` TypeError, `GetMethod(searchValue,
  %Symbol.replace%)`). It needs `%Symbol.match%` and `%Symbol.replace%` ids that do not exist (8d), and
  writing it on `ObjectKind::RegExp` branding would add the special-casing umbrella 8b exists to remove
  (9b's row says so for this member by name). Owner: 9b's `replaceAll` child,
  `#11-vm-builtin-prototype-static-sweep` item 1 (PM 471).
- The RegExp-side defects §1.3 records (homed in §8). SD-1, the RegExpBuiltinExec `lastIndex` overrun
  hang, is not among them: it landed as #605 (`b86671a5`) and is closed, and S1's base contains it.

### §0.2 Premises re-measured

| Premise (brief / live ledger) | Measured | Command |
|---|---|---|
| `replace` with a function replacement never calls it | **holds** at `efe8d05b` (F01-F22: every function cell red, §1.3); unchanged at `47b6c934` (#605 does not touch the string branch, §6) | probe P1 (§6) |
| the replacement is `ToString`'d, "so it returns the function's **source text**" | **wrong in its consequence**: it returns `function () { [native code] }` — `Function.prototype.toString` does not return source text for user functions, which is the separate slot `#11-vm-function-tostring-source-text` | probe P1 F01 |
| `replaceAll` "likely the same (not probed)" | **probed, and worse**: function never called (R01, R03, R07, R09, R11), and the RegExp pattern is `ToString`'d (R04-R06, R08) | probe P1 |
| GetSubstitution with a string replacement is a usable **control** | **wrong**: there is no GetSubstitution in the crate (the command → no output, rc 1); G01-G06, G08, G10, G11 red. Only templates that GetSubstitution leaves literal pass (G07, G09, G12, S19, S20) | `git grep -n -i -e getsubstitution -e get_substitution 47b6c934 -- crates/script/elidex-js/src`; probe P1 G* |
| Home = 9b (String prototype surface) | **holds for the non-RegExp half only**; the RegExp-pattern half is §22.2.6.11, the `%Symbol.replace%` member of umbrella 8b, minted as `#11-vm-regexp-symbol-replace` (PM 434 / PM 462) | `docs/plans/2026-07-vm-p4-slice-plan.md:979`, `:983` |
| a RegExp `@@replace` path may exist | **does not**: `register_regexp_prototype` installs `test` / `exec` / `toString` only (`vm/globals_primitives.rs:194-201`); `WellKnownSymbols` allocates 7 symbols, no `replace` (`vm/well_known.rs:1578-1584`); `typeof Symbol.replace` → `undefined` (O01) | `sed -n 194,201p crates/script/elidex-js/src/vm/globals_primitives.rs` |
| Priority high, silently wrong | holds; and the most common in-the-wild form, `s.replace(/re/g, fn)`, is in the RegExp half S1 does not fix; it stays broken until `#11-vm-regexp-symbol-replace` lands (OD-1 = (C), PM 434) | — |

### §0.3 Premise errors (in the brief and the ledger)

1. "returns the function's source text" — it returns the native-code placeholder (§0.2 row 2).
2. "`replaceAll` is likely the same (not probed)" — it was probed, and it is worse (§0.2 row 3).
3. "GetSubstitution with a string replacement, as a control" — GetSubstitution does not exist; the
   template cells are red, not a control.
4. "the RegExp `@@replace` path if one exists" — none exists; the RegExp handling is open-coded on
   `ObjectKind::RegExp` inside `native_string_replace` (`vm/natives_string.rs:475-484`) and is absent
   from `replaceAll`.
5. Implicit: "the slot is one unit". By measurement it is an umbrella (§2).

The live entry still carries errors 1 and 2; correcting it is a landing item (§7 L-4).

### §0.4 Decisions — ratified

- **OD-1 = (C) (ratified, PM 434; child id PM 462) — the RegExp half lands in
  `#11-vm-regexp-symbol-replace`, the `%Symbol.replace%` child of umbrella 8b, with no interim.** The
  RegExp half is §22.2.6.11. That child consumes 8d (the id), 8c (`groups`, §22.2.6.11 step 15.j) and 8a
  (`flags`, step 7) — 8b's derivation places those edges, as row 8b records (`docs/plans/2026-07-vm-p4-slice-plan.md:979`) — and consumes S1's
  chokepoint unchanged (§1.4, I4). **The order is SD-1 (landed, #605) → S1 → 8d → 8c →
  `#11-vm-regexp-symbol-replace`** (PM 434). **No interim ships**: option (B), rewriting the open-coded
  RegExp branch onto S1's chokepoint now, is rejected (PM 434). It would ship a second implementation of
  the §22.2.6.11 loop for that child to delete, with `groups` silently absent (8c) — the strangler state
  "One issue, one way" forbids. Consequence, stated as a defect: until that child lands,
  `s.replace(/re/g, fn)` keeps splicing the function's `ToString` text, and RegExp-pattern templates stay
  unexpanded. S1's own shape is unaffected by the ruling. The umbrella slot stays open until that child
  lands (PM 462; §7).
- **OD-2 (ratified, PM 434) — the file-wide `§21.1.3` → `§22.1.3` citation retag in
  `vm/natives_string.rs` stays with 9b's derivation.** Slice-plan row 9b places it on "whichever child
  lands first with `vm/natives_string.rs` in its touch set" and then requires 9b's derivation to rule on
  that device (row K precedent). S1 touches that file but is not a child 9b's derivation minted, so S1
  does **not** take the retag. It writes every citation it adds or rewrites correctly (§22.1.3.19 /
  §22.1.3.20 / §22.1.3.19.1). **Open input to 9b's derivation, not decided here (PM 471):** the retag
  population is wider than row 9b names. At `47b6c934`, `vm/natives_string.rs` has 7 `§21.1.3` cites (10
  `§21.` cites in all; #605 moved these from 8 and 14), and `vm/natives_string_ext.rs` — also in S1's
  touch set and not named by row 9b — has 10 `§21.1.x` cites (`:9 :33 :42 :97 :116 :135 :178 :254 :272
  :289`). Command: `git grep -c '§21\.' <rev> -- crates/script/elidex-js/src/vm/natives_string.rs
  crates/script/elidex-js/src/vm/natives_string_ext.rs`.

## §1. Design

### §1.1 Code-site census

| Site | What it does today | Spec it stands in for |
|---|---|---|
| `vm/natives_string.rs:466` `native_string_replace` | `coerce_this_string` (`:471`), then an `ObjectKind::RegExp` brand test (`:475-484`) | §22.1.3.19 |
| `vm/natives_string.rs:485-487` | RegExp branch: `ToString(replaceValue)` unconditionally | §22.2.6.11 steps 5-6 (no functionalReplace) |
| `vm/natives_string.rs:494-508` | global: `lastIndex` reset (`:495`, inside the global branch since #605), then exec and splice per match, interleaved; zero-length advance `m.end() + 1` (`:504`), not AdvanceStringIndex | §22.2.6.11 steps 9-17 (collect first, then replace) |
| `vm/natives_string.rs:509-520` | non-global: one exec, splice | same |
| `vm/natives_string.rs:528-543` | string pattern: `ToString` search (`:529`), `ToString` replacement (`:530`), `find_u16`, splice | §22.1.3.19 steps 4-15 (**S1**) |
| `vm/natives_string_ext.rs:206` `native_string_replace_all` | `ToString` both arguments (`:214-215`), empty-search arm (`:219-234`), scan loop (`:235-249`) | §22.1.3.20 steps 4-17 (**S1**); step 3 absent |
| `vm/natives_string.rs:602` | `match`'s global loop: zero-length advance `m.end() + 1`, not AdvanceStringIndex | §22.2.6.8 step 10.e (`#11-vm-regexp-symbol-match`) |
| `vm/natives_regexp.rs:22` `run_regexp` | builtin exec; since #605 it fails and resets when `lastIndex > length` for g / y (`:48-58`, step 13.a). Its comment `:39-47` records the remaining deviation: step 2's `? ToLength` is not performed for non-g / non-y (X02) | §22.2.7.2 RegExpBuiltinExec step 2 (`#11-vm-regexp-builtin-exec-last-index-read`) |
| `vm/natives_regexp.rs:104` `native_regexp_exec` | result array with `index` / `input`, no `groups` (O09) | §22.2.7.2 steps 30-34 (8c) |
| `vm/globals_primitives.rs:62`, `:66` | install `replaceAll`, `replace` | — |
| `vm/globals_primitives.rs:194-201` | `RegExp.prototype` = `test`, `exec`, `toString`; no `[%Symbol.replace%]` | §22.2.6.11 (`#11-vm-regexp-symbol-replace`) |
| `vm/well_known.rs:1578-1584` | 7 well-known symbols, no `replace` / `match` | Table 1 (8d) |
| GetSubstitution | **absent** | §22.1.3.19.1 (**S1**) |

Reused, unchanged: `call_function` on `NativeContext` (`vm/native_context.rs:85`, Call; it enters
`call` on `VmInner`, `vm/interpreter.rs:76`, which wraps `with_call_mode`, `:160`), `to_string_val` on
`NativeContext` (`vm/native_context.rs:71`, ToString), `is_callable` on `ObjectKind`
(`vm/object_kind.rs:1586`, IsCallable on an Object), `to_object` (`vm/coerce.rs:697`, ToObject),
`find_u16` (`crates/script/elidex-js/src/wtf16.rs:160`, a substring search from offset 0, returning
`Some(0)` for an empty needle).

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
TypeError (3.b.iii); then `GetMethod(searchValue, %Symbol.replace%)` → Call if present. 4-5 `ToString`
this, search. **6-7 functionalReplace as in replace.** 8-9 `advanceBy = max(1, searchLength)`. 10-12 **all
match positions are collected first** (`StringIndexOf(string, searchString, position + advanceBy)`).
13-15 for each position: preserved slice; **15.b functionalReplace: `ToString(Call(replaceValue,
undefined, « searchString, 𝔽(matchPosition), string »))`** — call *i* and its `ToString` both precede
call *i+1* (S17); 15.c else `GetSubstitution(searchString, string, matchPosition, « », undefined,
replaceValue)`. 16-17 tail.

**§6.1.4.1 StringIndexOf ( string, searchValue, fromIndex )** — step 2: an empty `searchValue` with
`fromIndex ≤ length` returns `fromIndex`; otherwise steps 4-5 scan `fromIndex ≤ i ≤ length −
searchLength` and return not-found when that range is empty. **`fromIndex > length` is a legal input**:
`replaceAll` with an empty search reaches it at step 12.b (`position + advanceBy` = `length + 1` after the
last boundary position), and the answer is not-found by step 5, for any search.

**§22.2.6.11 `RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue )`**
(`#11-vm-regexp-symbol-replace`; S1 only fixes the interface it consumes). 3 `ToString(string)`. **5-6
functionalReplace as above.** 7 `flags = ToString(Get(regexp, "flags"))`. 9 global → `Set(lastIndex, 0)`.
10-12 **collect every RegExpExec result first**; on an empty match, `AdvanceStringIndex(string, lastIndex,
fullUnicode)`. 15 for each result, **inside the loop and so after the replacer calls of earlier results**:
captures (each `ToString`'d unless undefined), **15.j `namedCaptures = ? Get(result, "groups")` — any
language value**; **15.k functionalReplace: `replacerArgs = « matched » + captures + « 𝔽(position),
string »`, plus `namedCaptures` itself if not undefined (no ToObject); `ToString(Call(replaceValue,
undefined, replacerArgs))`**; **15.l else `namedCaptures = ? ToObject(namedCaptures)` if not undefined**,
then `GetSubstitution(...)` (`?`). 15.m a result whose position moved backwards is skipped.

**§22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures,
replacementTemplate )** — `namedCaptures` is "an Object or undefined" in its signature, and step 5.g.iii.3
asserts it is an Object. A left-to-right scan of the template: `$$` → `$`; `` $` `` → `string[0,
position)`; `$&` → matched; `$'` → `string[min(position + len(matched), len))`; `$` + 1-2 digits →
two-digit index if ≤ capture count, else one digit then a literal digit; `1 ≤ index ≤ m` →
capture (undefined → empty), otherwise the `ref` text literally (so `$0`, `$00` and every `$n` with
no captures are literal); `$<`: if no `>` or `namedCaptures` undefined → literal `$<`, else
`ToString(Get(namedCaptures, groupName))` (undefined → empty) — **the one throwing arm**; any other
code unit is literal. No functionalReplace branch: the function arm never reaches it.

### §1.3 Measured matrix (actual at `efe8d05b` vs spec)

Probe P1 (§6): 88 cases, each run in a fresh `Vm` through a throwaway test, strict mode, at `efe8d05b`.
**Spec** is the value the §1.2 steps produce, and it is the only expectation any cell asserts. It is
derived from the steps, not read off an engine: the derivations that are not one-step readings are
recorded below the table, and plan-review Axis 4 re-derived over 40 of the §4.1 cells from the steps
with no disagreement.
[evidence] A run of the same case text under Node 26.13 agreed with every Spec value; it is a cross-check only, never an acceptance input, and where it and the steps disagree the steps win.
[history] The first draft of this column was read off that Node run and checked by hand; the column is now grounded on the steps as stated above.
**HANG** = no result within 20 s (process killed).

| Id | Case (abridged) | Spec | Actual (`efe8d05b`) |
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
| O07 | `'😀'.replace(/(?:)/gu, '-').length` | `4` | **HANG** |
| O10 | `'ab'.replace(/(?:)/g, (m, p) => '[' + p + ']')` | `[0]a[1]b[2]` | **HANG** |
| H01 | `"ab".replace(/(?:)/g, "-")` | `-a-b-` | **HANG** |
| H02 | `"😀".replace(/(?:)/gu, "-").length` | `4` | **HANG** |
| H03 | `"😀".match(/(?:)/gu).length` | `2` | **HANG** |
| H04 | `"ab".match(/(?:)/g).length` | `3` | **HANG** |
| H05 | `"ab".replace(/x*/g, "-")` | `-a-b-` | **HANG** |
| H06 | `"a".replace(/$/g, "!")` | `a!` | **HANG** |
| X01 | `/$/g`, `lastIndex = 5`, `exec('a')` | `null:0` | `match@1:1` (SD-1's root) |
| X02 | non-global `exec` reads `lastIndex` (`valueOf` side effect) | `v` | `` (not read) |

Of the 88 cases, 72 are red and 16 green at `efe8d05b`. The green ones are the no-call cases (F07, F15, R02), the
literal templates (G07, G09, G12, S19, S20), the `ToString`-order cases with no callable replacement
(O02, S06, S13), O03 / O04 / O06, and the two class-call controls C01 / C02 (`(class {})()` and
`[1].map(class {})` throw TypeError). No green cell calls a replacer or expands a `$` form.

**Derivations of the HANG-row Spec values** (§22.2.6.11 for `replace`, §22.2.6.8 for `match`; RegExpBuiltinExec
step 13.a fails when `lastIndex > length`, and AdvanceStringIndex is §22.2.7.3). Subject `"ab"` has length 2;
`"😀"` (`😀`) has length 2 and is one code point of two code units.

- **H01** `"ab".replace(/(?:)/g, "-")`: global, so step 9 sets `lastIndex` 0. Step 12 collects empty matches at
  0, 1 and 2 (`lastIndex` 2 = length is allowed; each empty match advances it by AdvanceStringIndex step 2,
  `index + 1`); at 3, RegExpBuiltinExec step 13.a fails. Step 15 inserts `-` at 0, 1, 2 → **`-a-b-`**.
- **H05** `"ab".replace(/x*/g, "-")`: `x*` matches empty at 0, 1, 2 exactly as H01 → **`-a-b-`**.
- **O10** `'ab'.replace(/(?:)/g, (m, p) => '[' + p + ']')`: the positions are H01's, and step 15.k passes
  each as `p` → **`[0]a[1]b[2]`**.
- **H06** `"a".replace(/$/g, "!")`: empty match at 1 (= length); the advance gives 2 > length, so the next exec
  fails → one insertion at 1 → **`a!`**.
- **H04** `"ab".match(/(?:)/g).length`: §22.2.6.8 step 10 pushes the empty matches at 0, 1, 2; 3 > length →
  null, so step 10.b.ii returns the array → **3**.
- **O07 / H02** `'😀'.replace(/(?:)/gu, '-').length`: `fullUnicode` is true (step 12.c.iii.2.b). Empty match at
  0; AdvanceStringIndex(S, 0, true): `index + 1` = 1 < 2, so step 6 returns 0 + the code point's
  `[[CodeUnitCount]]` 2 → 2. Empty match at 2 (= length); step 4 returns 3. At 3 the exec fails. Two
  insertions, at 0 and 2 → `-😀-`, length 2 + 2 = **4**.
- **H03** `"😀".match(/(?:)/gu).length`: §22.2.6.8 with `fullUnicode` true (step 6): match at 0,
  advance to 2 (as above); match at 2, advance to 3; at 3 null → the array has **2** elements. (It is not 4:
  `match` counts matches, `replace` counts code units of the result.)

**The HANG rows at S1's base.** The Actual column is `efe8d05b`. SD-1 landed (#605, `b86671a5`) and S1's base
contains it, so none of these cells can hang there. That changes no S1 cell: every §4.1 cell uses a string
pattern. #605 changed both `vm/natives_regexp.rs` (step 13.a) and `vm/natives_string.rs` (a
`Result`-returning `lastIndex` writer, the global-only reset at `:495`, and `search`'s SameValue guards),
and none of those edits is on the string branch (`:528-543`) or in `replaceAll`. What it changes is where the
cells belong (§4.2):

- H04, H06 and X01 are pinned by #605's own cells in `vm/tests/tests_regexp_last_index_overrun.rs`
  (`'a'.replace(/$/g, '!')`, `'ab'.match(/(?:)/g)`, `/$/g` with `lastIndex = 5`), with the trim idiom. S1
  does not duplicate them.
- O05 (non-global `replace` resets `lastIndex`, SD-2) was folded into SD-1 (PM 454) and is fixed and pinned
  there (`non_global_replace_frozen_regexp_does_not_throw`, `non_global_sticky_replace_uses_current_last_index`).
  S1 does not write it.
- H01 and H05 are **not** in #605's cells (`git grep -n 'replace(/(?:)/g\|x\*/g' -- crates/script/elidex-js/src/vm/tests/tests_regexp_last_index_overrun.rs`
  → no output). Tracing the RegExp branch at `47b6c934` (`:494-508`: empty match, `m.end() + 1` advance,
  step 13.a failing at 3) gives `-a-b-` for both, so they are expected green at S1's base; the implementer's
  first run decides (§4.2).
- O10 is a function replacement on the RegExp path. With the hang gone it is still red (no functionalReplace
  on that branch), so it is `#11-vm-regexp-symbol-replace`'s.
- O07, H02, H03 (`u` flag over a surrogate pair) were never measured past the clamp. Whether the `m.end() + 1`
  advance (`vm/natives_string.rs:504` / `:602`) gives the spec value once the regex engine is asked to search
  from the middle of a surrogate pair is unknown. It is the AdvanceStringIndex obligation (§22.2.7.3): the replace
  cells (O07, H02) are `#11-vm-regexp-symbol-replace`'s (PM 454), and H03 (`match`) is
  `#11-vm-regexp-symbol-match`'s (PM 471).

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
  named_captures: JsValue }`. `named_captures` is the **raw** §22.2.6.11 step 15.j value: `Undefined` when
  there is none, otherwise whatever `Get(result, "groups")` returned — an object, but also `null` or a
  primitive, since a user `exec` builds `result`. S1 always builds `captures: vec![]` and
  `named_captures: JsValue::Undefined`. `#11-vm-regexp-symbol-replace` builds the full record from each
  RegExpExec result, inside its step-15 loop (I4).
- `replacement_for_match` (NEW)`(ctx, &Replacer, &MatchRecord, string: StringId) ->
  Result<Vec<u16>, VmError>`. The `Function` arm builds « matched, …captures (undefined for `None`),
  𝔽(position), string », appends `named_captures` **as is** when it is not `Undefined` (step 15.k.ii; no
  ToObject), and calls `ctx.call_function(f, JsValue::Undefined, &args)`. Its `?` propagates a throw
  (F17 / S10) and its result goes through `to_string_val` exactly once, with no template expansion (S04).
  The `Template` arm first converts `named_captures`: `Undefined` → `None`; anything else →
  `Some(ToObject(v)?)` (step 15.l.i; `null` → TypeError, a primitive → its wrapper), and only then calls
  `get_substitution`. ToObject appears **only** in this arm.
- `get_substitution` (NEW)`(ctx, matched, string, position, captures: &[Option<StringId>],
  named_captures: Option<ObjectId>, template: StringId) -> Result<Vec<u16>, VmError>`: §22.1.3.19.1 in
  full, over WTF-16 slices. Its `named_captures` parameter is `Option<ObjectId>` because the AO's signature
  and its step 5.g.iii.3 assert make it "an Object or undefined"; the conversion from the raw value is the
  caller's, in the `Template` arm above. It returns `Result` because the `$<` arm is a `Get`.
- `string_index_of` (NEW)`(s: &[u16], search: &[u16], from: usize) -> Option<usize>`: §6.1.4.1 in full.
  Step 2: empty `search` and `from ≤ len` → `Some(from)`. **`from > len` → `None`**, checked before any
  slice is taken (steps 4-5: the scan range is empty; `&s[from..]` would panic for `from = len + 1`).
  Otherwise `find_u16(&s[from..], search).map(|i| from + i)`. Both String methods use it; nothing calls
  `find_u16` with an offset directly. Pinned by S08, R03 and T-POS-END (§4.1), and by mutant M16.
- `native_string_replace`'s string branch (`vm/natives_string.rs:528-543`) becomes §22.1.3.19 steps
  4-15 over these. `native_string_replace_all` becomes §22.1.3.20 steps 4-17. It **collects the
  positions first** (`advanceBy = max(1, searchLength)`, `string_index_of(.., position + advanceBy)`), which
  subsumes today's special empty-search arm (`vm/natives_string_ext.rs:219-234`), and then makes one
  `replacement_for_match` per position in order, so the S17 interleaving holds.

**Arms with no JS path in S1 (S1's own design choice).** `get_substitution` is written once, in full, rather
than as the subset S1's path reaches, so that `#11-vm-regexp-symbol-replace` consumes it without reopening
it ("One issue, one way"). Its capture arms and its `$<` Get arm, and the `named_captures` handling in both
arms of `replacement_for_match`, have no JS-reachable input until that child supplies records that carry
them; S1 pins them from Rust instead (T-SUB-UNIT, T-I4-UNIT, §4.1). This is not licensed by the slice-plan's
"Prerequisite row" rule — S1 is not a prerequisite row — and is stated here as S1's own choice. The digit
arm **is** JS-reachable in S1 with an empty capture list (`$0`, `$01`, `$10`, `$00` → literal, S20).

**In-code markers S1 places** (each names its owner slot, so a `git grep` for the id finds the site):

- In `native_string_replace_all`, where step 3 would sit, before the `ToString` of `searchValue`:
  `// §22.1.3.20 step 3 (IsRegExp, the "flags" / "g" TypeError, GetMethod(searchValue, %Symbol.replace%))`
  `// is absent: a RegExp searchValue is ToString'd here and searched as text. Known deviation:`
  `// #11-vm-builtin-prototype-static-sweep item 1 (the replaceAll child of umbrella 9b).`
- In `native_string_replace`'s RegExp branch, at its `ToString(replaceValue)` (`:485-487` today):
  `// Known deviation: #11-vm-regexp-symbol-replace. This branch stands in for §22.2.6.11 without`
  `// functionalReplace (steps 5-6) or GetSubstitution (step 15.l): a callable replaceValue is ToString'd`
  `// and a template is spliced verbatim. It is not moved onto vm/string_replace.rs (PM 434, OD-1 = (C)).`

**GC.** The VM's invariant is that GC does not run inside a native call: both native dispatch sites clear
`gc_enabled` for the call and restore it after (`vm/interpreter.rs:360-381`; `vm/dispatch.rs:857-864`), and
a re-entrant `call_function` goes through `with_call_mode`, which saves and restores the flag and never sets
it (`git grep -n 'gc_enabled = true' <rev> -- crates/script/elidex-js/src/vm ':!*tests*'` → only
`vm/init.rs:855`, the post-init enable). So nothing S1 holds across a replacer call can be collected under the
current VM: the replacer `ObjectId` (whether it lives in the caller's frame or in a native's owned argument
vector), and the `StringId`s, which are pool-permanent anyway (`vm/gc/mod.rs:90`). S1 holds no other object
across a call; its `named_captures` is always `Undefined`. **The rooting obligation for later records is
`#11-vm-regexp-symbol-replace`'s**: it holds the step-10 `results` list (objects) across every replacer call,
and each record's `named_captures` across its own. It discharges that either under the invariant above, stated
in its code as `natives_array_hof.rs:141-149` states it for `reduce`, or with that comment's rooted-slot
convention (`push_stack_scope`), which future-proofs for GC during native→JS callbacks. Neither choice changes
`MatchRecord`'s shape: a `JsValue` can be pinned in a stack slot as it is.

**Re-entrancy.** The two String natives hold no VM-side state across the call, only Rust locals (the
position list, the accumulated `Vec<u16>`), so a nested `replace` / `replaceAll` is independent (S12,
S18). The RegExp form of re-entrancy (F20, F21: a shared `lastIndex`) is what "collect every result
first" in §22.2.6.11 steps 10-12 protects. It is `#11-vm-regexp-symbol-replace`'s obligation and cannot
arise on the string path.

**Layering.** These are ECMAScript built-ins, the VM core's own surface; no DOM / engine-independent
crate is involved. `vm/string_replace.rs` sits beside the natives and does not live in `host/`.

**ECS-native**: N/A — pure ECMAScript string built-ins: no per-entity state, no ECS component read or
write, no side-store; all state is Rust locals and pool-permanent `StringId`s.

## §2. Edge density, by measurement

Criterion (slice-plan §5's head): an axis counts only on the evidence of a failure that occurs while
the others pass. The axes below are counted over the slot as registered; §1.3 gives the base evidence
(at `efe8d05b`) and §4.3 the mutants.

| Axis | Invariant | Evidence that it fails alone |
|---|---|---|
| **A1** replacer call protocol | IsCallable selects the branch before any `ToString(replaceValue)`; `Call(f, undefined, « … »)`; result `ToString`'d once, not template-expanded; abrupt completion propagates | O11 / S16 red while the not-callable order cells O02 / S06 / S13 are green; M2-M5, M11 each fail only A1 cells |
| **A2** template substitution (GetSubstitution) | each `$` form per §22.1.3.19.1 | G01-G04 red with no function involved; G07 / G12 / S19 / S20 green (the literal arms work by accident); M8-M10 |
| **A3** string-pattern match enumeration | positions collected first, `advanceBy = max(1, len)`, StringIndexOf over its whole domain, `position` is the argument and the `` $` `` / `$'` anchor | R11 / S09 depend on the position set while A1 / A2 are fixed; M6, M7, M12, M16 |
| **A4** RegExp result collection | RegExpExec loop; `lastIndex` reset only if global; AdvanceStringIndex on empty match; every result collected before any replacer call | at `efe8d05b`: O05 red with a plain template, H01 / H06 hang with a plain template (#605 has since fixed the hang and O05); F20 / F21 |
| **A5** named captures | `groups` from RegExpBuiltinExec reaches the replacer and `$<name>` | O09 red on `exec` alone; F13, G06, R08 |
| **A6** protocol dispatch | `GetMethod(%Symbol.replace%)`; `replaceAll`'s IsRegExp + `"g"` TypeError | O01 / O08 red with no replacement at all; R04-R06 |

**Six measured intersecting axes over the slot**, which is well past the threshold of three. There is
also no single canonical algorithm for the slot: it spans three spec algorithms owned by two umbrella
rows. So under CLAUDE.md "Edge-dense work" the slot is an **umbrella**. It is split:

- **S1 (this memo)** — A1 + A2 + A3 over the non-RegExp path, plus the chokepoint (§1.4).
- **A4 + A5 + A6 and the RegExp form of A1 / A2** → `#11-vm-regexp-symbol-replace` (the §22.2.6.11
  algorithm), the String-side `GetMethod` child of 8b (step 3 in each method; un-minted, 8b's derivation),
  8c (`groups`), 8d (ids), and 9b's `replaceAll` child, `#11-vm-builtin-prototype-static-sweep` item 1
  (its step 3). Each edge is placed in §5. The A4 hang was a separate defect with its own slot,
  `#11-vm-regexp-builtin-exec-lastindex-overrun` (SD-1), landed and closed by #605.

### §2.1 Coupled invariants of S1 (Pre-condition #3)

S1 spans three axes, and its chokepoint fixes an interface a fourth party consumes, so its coupled
invariants and their pairwise intersections are listed:

- **I1 (A1)** — a callable `replaceValue` is called once per match, in order, with `this` undefined
  and « matched, position, string », its result `ToString`'d exactly once; a non-callable one is
  `ToString`'d exactly once, after the search string.
- **I2 (A2)** — a template is expanded per §22.1.3.19.1, against the empty capture list and undefined
  `namedCaptures` on this path.
- **I3 (A3)** — the position list is exactly the non-overlapping StringIndexOf sequence with
  `advanceBy = max(1, searchLength)`, computed before any replacement, with StringIndexOf defined for
  `fromIndex > length` (not-found).
- **I4 (the consumer interface)** — `MatchRecord.named_captures` carries the raw §22.2.6.11 step 15.j
  value (`JsValue`; `Undefined` = none). The `Function` arm passes it unconverted when it is not
  `Undefined` (step 15.k.ii); the `Template` arm alone applies ToObject (step 15.l.i), so `null` throws
  TypeError only on the template path; `get_substitution` receives an Object or nothing.

Intersections:

- **I1 × I2** — the selection point: `resolve_replacer` decides the arm once, before the loop, so a
  function result is never template-expanded (S04, M4), and a callable is never `ToString`'d (S16, M5).
- **I1 × I3** — position *i*'s call and its `ToString` both precede call *i+1* (S17, M14), and a throw
  at call *k* stops the loop with no call *k+1* (S10, M11). The `position` argument is I3's value
  (R01, R11, M3).
- **I2 × I3** — `` $` `` and `$'` read `position` and `position + len(matched)` from I3's list
  (S08, S09, M9), including the empty-search boundary positions 0 and `len` (S08, T-POS-END).
- **I1 × I4** — the argument list: `named_captures` is appended after `string` iff it is not `Undefined`,
  and appended raw. On S1's path it is always `Undefined`, so the replacer sees exactly three arguments
  (S11 `3`); the appended and the raw-`null` forms are pinned from Rust (T-I4-UNIT; M17, M18).
- **I2 × I4** — the conversion boundary: ToObject runs in the `Template` arm, once per record, before
  GetSubstitution and never inside it; `Undefined` reaches GetSubstitution as "undefined", so `$<x>` stays
  literal (S19); `null` makes the arm throw before any substitution (T-I4-UNIT; M17).
- **I3 × I4** — empty on S1's path: the string-pattern position list never yields a `named_captures`
  value. On the RegExp path the corresponding intersection is the child's (below).
- **I4 × A4 (the child's result collection)** — step 15.j's `Get(result, "groups")` (and step 15.i's
  capture reads) run inside the step-15 loop, after the replacer calls of earlier results, not during the
  step 10-12 collection. So `#11-vm-regexp-symbol-replace` builds each `MatchRecord` at its own iteration,
  and the objects held across replacer calls are the `results` list and the current record's
  `named_captures` (§1.4 GC).
- **I4 × A5 (8c's `groups`)** — for the builtin exec, 8c makes `groups` an object or `undefined`
  (RegExpBuiltinExec steps 30-34), and both reach `MatchRecord` unchanged; a user `exec` can return any
  value there, which is why the field is `JsValue` and not `Option<ObjectId>`.

### §2.2 Terminality of S1

The criterion is slice-plan §5's head (CLAUDE.md "Edge-dense work"): a unit that bundles ≥3 intersecting
invariant axes, or touches a subsystem with no canonical algorithm, is not one PR; the base case (c) is a
narrowly scoped per-PR slice under an approved umbrella that has passed plan-review. Measured on S1's shared
chokepoint:

- **First disjunct: met, at the threshold.** S1 bundles three axes that each fail alone (§2: A1, A2, A3) and
  intersect pairwise at the chokepoint (§2.1). S1's terminality therefore does **not** follow from an axis
  count, and this memo does not claim it does.
- **Second disjunct: not met.** Every branch S1 touches has a governing AO, cited through `webref` in §3.
- **Base case (c).** S1 is the per-PR child that this memo's §2 split mints from the umbrella slot
  `#11-vm-string-replace-function-replacement` (PM 434). It sits under the approved VM P4 umbrella (#506)
  and its §5 head, and this memo is its plan-review input. It is not a child 9b's derivation minted (that
  derivation has not run; §5 records how it will count S1). Its touch set is one new module and two native
  bodies, plus two marker comments. Whether that is "narrowly scoped" is the plan-review's ruling; S1
  asks for it here rather than asserting it.
- **The split S1 does not take.** A2 (`get_substitution`, a pure function of its inputs) could land alone,
  with S1's template cells, as an **S1a** (`get_substitution` + `string_index_of` + both natives' template
  paths and `replaceAll`'s position list: A2 + A3), followed by an **S1b** (the `Function` variant of `Replacer` (NEW) and the
  `Function` arm: A1). Neither half would stub an arm the other rewrites. S1 is proposed as one PR because
  the three intersections that involve A1 (I1 × I2, I1 × I3, I1 × I4) are asserted on the selection point
  and the per-position loop both PRs would build, so S1b would re-touch the loop S1a just landed in both
  natives. That is a cost argument, not a structural impossibility. If the plan-review does not ratify S1
  under (c), S1a / S1b is the split.

S1 carries **no** RegExp-side invariant: it calls no RegExpExec, reads no `lastIndex` and constructs no
`groups`.

**Re-slice trigger**: the post-implementation mutant run (§4.3) finds a mutant that fails alone and
breaks an invariant none of I1-I4 owns.

## §3. Spec coverage map

Section numbers from `.claude/tools/webref aoid ecma262 <AO>` (GetSubstitution, StringIndexOf,
IsCallable, Call, ToObject, RequireObjectCoercible, IsRegExp, RegExpExec, RegExpBuiltinExec, AdvanceStringIndex)
and `.claude/tools/webref heading ecma262 <n> --exact` (the four method headings and §10.2.1); step text
from `.claude/tools/webref body ecma262 <anchor>`. Skeleton from `.claude/tools/webref coverage-map` (§6).
Rows marked **→ owner** are not S1's touch. They are listed so the full-enum column covers every branch of
the algorithms S1 shares a chokepoint with.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 1-2 | RequireObjectCoercible(this): null / undefined → TypeError | `coerce_this_string` (`vm/natives_string.rs:116`), unchanged | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 3 | Object with `%Symbol.replace%` → Call(replacer) | **→ 8b's String-side `GetMethod` child** (today: `ObjectKind::RegExp` brand, `vm/natives_string.rs:475-484`, unchanged) | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 4-5 | ToString(this), ToString(searchValue), in that order | `vm/natives_string.rs:471`, `:529` | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 6-7 | IsCallable true → no ToString; false → ToString(replaceValue) | `resolve_replacer` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 9-10 | not-found → return string, no call | string branch, `string_index_of` (NEW) from 0 | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 13 | functionalReplace: Call(replaceValue, undefined, « searchString, position, string »), ToString | `replacement_for_match` (NEW) `Function` arm | ✓ | yes |
| ECMA-262 §22.1.3.19 String.prototype.replace ( searchValue, replaceValue ) | 14 | template: GetSubstitution with « » and undefined | `replacement_for_match` (NEW) `Template` arm | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 1-2 | RequireObjectCoercible(this): null / undefined → TypeError | `coerce_this_string`, unchanged (`vm/natives_string_ext.rs:211`) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 3.a-3.b | IsRegExp true, flags without `"g"` → TypeError (3.b.iii) | **→ `#11-vm-builtin-prototype-static-sweep` item 1** (consumes 8d's `%Symbol.match%`); in-code marker (§1.4) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 3.c-3.d | `%Symbol.replace%` method → Call | **→ `#11-vm-builtin-prototype-static-sweep` item 1**, via 8b's String-side `GetMethod` child | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 4-7 | ToString order; functionalReplace selection | `vm/natives_string_ext.rs:206` → `resolve_replacer` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 9-12 | advanceBy = max(1, len); all positions collected first (empty search included, ending at fromIndex = len + 1) | `native_string_replace_all` position list over `string_index_of` (NEW) (replaces `:219-249`) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 15.b | functionalReplace per position, in order | `replacement_for_match` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 15.c | template per position | `replacement_for_match` (NEW) → `get_substitution` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue ) | 16-17 | tail after last match | `native_string_replace_all` | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.b-5.e | `$$`, `` $` ``, `$&`, `$'` (tail clamped to length) | `get_substitution` (NEW) | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.f | `$n` / `$nn`: two-digit fallback, in-range capture (undefined → empty), out-of-range → literal | `get_substitution` (NEW); in-range arm Rust-pinned | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.g | `$<`: no `>` or namedCaptures undefined → literal; else ToString(Get(namedCaptures, name)), undefined → empty, throw propagates | `get_substitution` (NEW); Get arm Rust-pinned | ✓ | yes |
| ECMA-262 §22.1.3.19.1 GetSubstitution ( matched, string, position, captures, namedCaptures, replacementTemplate ) | 5.h | any other code unit literal | `get_substitution` (NEW) | ✓ | yes |
| ECMA-262 §6.1.4.1 StringIndexOf ( string, searchValue, fromIndex ) | 1-5 | empty search, fromIndex ≤ len → fromIndex; scan; not-found, including fromIndex > len | `string_index_of` (NEW), over `crate::wtf16::find_u16` only when fromIndex ≤ len | ✓ | yes |
| ECMA-262 §7.2.3 IsCallable ( arg ) | 1-3 | non-Object → false; `[[Call]]` → true (class constructors included) | `is_callable` on `ObjectKind` (`vm/object_kind.rs:1586`) | ✓ | yes |
| ECMA-262 §7.3.13 Call ( func, thisValue [ , argList ] ) | 1-3 | callable → `func.[[Call]]` | `call_function` on `NativeContext` (`vm/native_context.rs:85`), unchanged | ✓ | yes |
| ECMA-262 §10.2.1 [[Call]] ( thisArg, argList ) | 4 | `[[IsClassConstructor]]` true → TypeError (S01) | `call_function`, unchanged (probe C01 / C02) | ✓ | yes |
| ECMA-262 §7.1.20 ToObject ( arg ) | — | null → TypeError; primitive → wrapper; Object → itself (via §22.2.6.11 step 15.l.i) | `replacement_for_match` (NEW) `Template` arm → `to_object` (`vm/coerce.rs:697`); Rust-pinned (T-I4-UNIT) | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 5-6, 15.k-15.l | functionalReplace / GetSubstitution with captures + namedCaptures | **→ `#11-vm-regexp-symbol-replace`**, consuming `replacement_for_match` (NEW) unchanged | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 7-12 | flags read; global `lastIndex` reset; collect all results; empty-match advance | **→ `#11-vm-regexp-symbol-replace`** (8a `flags`) | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 15.i-15.j | captures `ToString`'d unless undefined; `namedCaptures = Get(result, "groups")`, per result, inside the loop | **→ `#11-vm-regexp-symbol-replace`** (8c) | ✓ | yes |
| ECMA-262 §22.2.6.11 RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue ) | 15.m | a result whose position moved backwards is skipped | **→ `#11-vm-regexp-symbol-replace`** | ✓ | yes |
| ECMA-262 §22.2.6.8 RegExp.prototype [ %Symbol.match% ] ( string ) | 6, 10.e | global loop; empty match → AdvanceStringIndex with fullUnicode | **→ `#11-vm-regexp-symbol-match`** (today `vm/natives_string.rs:602`) | ✓ | yes |
| ECMA-262 §7.2.6 IsRegExp ( arg ) | 1-5 | `%Symbol.match%` read before the `[[RegExpMatcher]]` test; otherwise false | **→ `#11-vm-builtin-prototype-static-sweep` item 1** (8d id) | ✓ | yes |
| ECMA-262 §22.2.7.1 RegExpExec ( regexp, string ) | 1-4 | user `exec` vs builtin (step 4) | **→ `#11-vm-regexp-symbol-replace`** | ✓ | yes |
| ECMA-262 §22.2.7.2 RegExpBuiltinExec ( regexp, string ) | 2 | `lastIndex` read with `? ToLength` unconditionally (X02) | **→ `#11-vm-regexp-builtin-exec-last-index-read`** (comment at `vm/natives_regexp.rs:39-47`) | ✓ | yes |
| ECMA-262 §22.2.7.2 RegExpBuiltinExec ( regexp, string ) | 13.a | `lastIndex > length` → fail (and reset if g/y) | landed: #605 `b86671a5` (`vm/natives_regexp.rs:48-58`), closed | ✓ | yes |
| ECMA-262 §22.2.7.3 AdvanceStringIndex ( string, index, unicode ) | 1-6 | unicode advance past a surrogate pair (step 6) | **→ `#11-vm-regexp-symbol-replace`** (replace side, today `vm/natives_string.rs:504`) and **→ `#11-vm-regexp-symbol-match`** (match side, `:602`) | ✓ | yes |

**Breadth**: K=1 (ecma262), M=34 rows, of which 21 are S1 touches (the rows without **→ owner** and
without "landed"). Count re-derived from the table at this revision; re-derive on any row edit. Preflight's
`SPLIT-DEFAULT` verdict (M ≥ 30) counts the 13 non-S1 rows too; the split they stand for is the one §2
already made (each is homed in §5). The single-PR justification for S1's 21 rows is §2.2, which also names
the split (S1a / S1b) the plan-review may choose instead.

### §3.1 User-input audit

Every value S1 reads is script-controlled: the receiver, the search value, the replacement (callable
or template), and whatever the replacer returns. After S1, two places run user code. The replacer runs
inside `replacement_for_match` with `this` undefined and no VM state held across the call. A
non-callable replacement and the search value run user `toString` through `to_string_val`, once each,
in spec order. The SD-1 hang that the RegExp branch reached at `efe8d05b` is gone at S1's base (#605). S1
reaches no RegExp code.

**Relation to the call-depth guard program (`#11-vm-call-depth-guard`; plan on the unpushed branch
`vm-stack-safety-plan`).** S1 adds a native→JS re-entry: the replacer call from `replacement_for_match`. A
replacer that calls `replace` again recurses without bound, and every level enters the VM through
`call_function` → `call` on `VmInner` → `with_call_mode` (`vm/interpreter.rs:76`, `:160`), which is the guard's
choke point CP-1. So S1 creates no recursion path outside the guard's coverage, and it adds no stack-probe
site of its own. Until the guard lands, unbounded replacer recursion has the same exposure as any other
native→JS recursion today. The guard's PR-1 also edits `vm/natives_string.rs`, at the regex compile sites in
`native_string_match` and `native_string_search`, not in `native_string_replace`. The two PRs therefore touch
the same file at different functions; whichever lands second rebases and re-derives its `file:line` anchors
(§6). **The merge order is the PM's to set**; S1 does not depend on it in either direction.

## §4. Tests

### §4.1 Cells (S1)

New file `crates/script/elidex-js/src/vm/tests/tests_string_replace.rs` (NEW), registered in
`vm/tests/mod.rs` (206 lines at `47b6c934`). `tests_string_complement.rs` (931 lines) holds three
`replaceAll` cells (`:196-207`) and is not grown. Those three stay green, and
`tests_string.rs:71-75` / `tests_m4_10_2.rs:292-297` (plain replacements) stay green. Each cell runs
in a fresh `Vm` through the `eval_string` helper (`vm/tests/helpers.rs:50`). Base values are probe P1's
(§1.3). One `#[test]` per case id, so a count of `#[ignore]` attributes is a count of cells (§7 L-3).

| Cell | Asserts (spec value) | Base | Mutant it kills |
|---|---|---|---|
| T-FN-CALL | F01 `z`; F04 `42`; F05 `T`; F06 `undefined`; S05 `z` | red | M1 |
| T-FN-ARGS | F02 `a[b\|1\|abc]c`; S11 `a3c`; S07 `anumber`; S14 `astring:ab`; O12 `1string12312312`; R01 `a[b\|1\|abab]a[b\|3\|abab]` | red | M1, M3, M12 |
| T-FN-THIS | F03 / R07 `undefined`; S03 `B`; S02 TypeError; S01 TypeError; S15 `NaN` | red | M2 |
| T-FN-NOCALL | F07 / R02 `0:abc` | green (stays) | M13 |
| T-FN-NOSUBST | S04 `$&b` | red | M4 |
| T-SEL | O11 `sf:z`; S16 `:z` (callable never `ToString`'d); O02 / S06 `sr`, S13 `r` (not-callable order) | O11 / S16 red, the rest green | M5 |
| T-POS | R11 `[0][2]`; F08 `[0]ab`; R03 `[0]a[1]b[2]` | red | M6, M7, M16 |
| T-POS-END | `''.replaceAll('', '-')` → `-` (position 0, then StringIndexOf from 1 > length 0 → not-found); `'ab'.replaceAll('', '-')` → `-a-b-` (from 3 > 2) | green today, both (the second is `tests_string_complement.rs:206-207`); kept as the end-of-domain guard | M7, M16 |
| T-ORDER | S17 `c1,t1,c2,t2:12` | red | M14 |
| T-ABRUPT | F17 / R09 `RangeError:boom`; S10 `2:RangeError` | red | M11, M14 |
| T-REENTRY | S12 `aa[X]aa`; S18 `axByc` | red | M1 |
| T-SUB | G01 `a[b]c`; G02 `a[a]c`; G03 `a[c]c`; G04 `a[$]c`; R10 `a[b]a[b]`; S08 `[]a[a]b[ab]`; S09 `a[a\|ab]a[aba\|]` | red | M8, M9, M16 |
| T-SUB-LIT | G07 `a$1c`; G12 `a$c`; S19 `a$<x>c`; S20 `a$01$10$00c` | green (stays) | M10b |
| T-SUB-UNIT | Rust, calling `get_substitution` (NEW) directly: captures `[Some("b")]` → `$1` = `b`, `$01` = `b`, `$10` = `b0`, `$2` literal, `$0` literal; `[None]` → `$1` empty; `namedCaptures` an object `{n: 'N'}` → `$<n>` = `N`, `$<m>` empty, `$<n` literal; a getter that throws → `Err`; `matched` longer than the tail → `$'` empty | (NEW helper) | M10, M10b, M15 |
| T-I4-UNIT | Rust, calling `replacement_for_match` (NEW) through a `NativeContext` over a test `Vm` (precedent `tests_coerce.rs:480`): `Template` arm, template `[$<length>]`, `named_captures` = the string `'str'` → `[3]` (ToObject wraps it); `named_captures` = `null` → `Err` TypeError; `Function` arm, replacer `function () { return arguments.length + ':' + String(arguments[arguments.length - 1]) }`, `named_captures` = `null` → `4:null` (passed raw, no throw); `named_captures` = `Undefined` → `3:<string>` | (NEW helper) | M17, M18 |

### §4.2 Known-deviation cells (out of S1's scope)

These are written into the same file now, with the **spec** value asserted. A cell that is red at S1's base
is `#[ignore = "known deviation: <owner>"]`, and its owner's acceptance is to remove the `ignore`. A cell that
is green at S1's base is asserted un-ignored (its owner then has nothing left to remove for it). None of them
asserts today's wrong value: pinning that would bless the defect. Owner strings are slot ids, so `git grep
'known deviation: <id>'` lists each owner's cells.

| Cells | Owner string (`known deviation: …`) | Base |
|---|---|---|
| F09, F10, F11, F12, F14, F16, F18, F19, F20, F21, F22, G05, G08, G10, G11, O10 | `#11-vm-regexp-symbol-replace` | red |
| F13, G06, R08 | `#11-vm-regexp-symbol-replace` (after 8c, `groups`) | red |
| O07, H02 | `#11-vm-regexp-symbol-replace` (AdvanceStringIndex, replace side; PM 454) | unknown (§1.3); measured by the implementer's first run |
| H01, H05 | `#11-vm-regexp-symbol-replace` (zero-width global replace) | expected green (traced, §1.3); measured by the implementer's first run |
| F15 | none: asserted un-ignored, the RegExp-path no-call control | green |
| H03 | `#11-vm-regexp-symbol-match` (AdvanceStringIndex, match side; PM 471) | unknown (§1.3); measured by the implementer's first run |
| O01 | `#11-vm-regexp-constructor-and-flags (8d)` | red |
| O08 | `#11-vm-regexp-constructor-and-flags (8b String-side GetMethod child)` — un-minted; 8b's derivation mints it and re-points this string | red |
| R04, R05 | `#11-vm-builtin-prototype-static-sweep item 1` (after 8d, 8b's `GetMethod` child and `#11-vm-regexp-symbol-replace`) | red |
| R06 | `#11-vm-builtin-prototype-static-sweep item 1` (PM 471) | red |
| X02 | `#11-vm-regexp-builtin-exec-last-index-read` (PM 471) | red |

**Not here.** H04, H06 and X01 are pinned by #605's cells, and O05 by #605's non-global replace cells (§1.3). S1
does not duplicate them.

**No hanging cell.** S1's implementation base contains `b86671a5` (checked with `git merge-base --is-ancestor
b86671a5 HEAD` before the first S1 commit), so no cell in this file can hang under `cargo test -- --ignored`.
What remains open is only the base value of O07 / H02 / H03 / H01 / H05: the implementer runs them once at S1's
base and applies the rule above.

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
- **M16** `string_index_of` drops its `from > len` check (slices `&s[from..]` unconditionally): a panic on the
  empty-search end position.
- **M17** the `Template` arm passes a non-Object `named_captures` as `None` instead of applying ToObject.
- **M18** the `Function` arm applies ToObject to `named_captures` before appending it.

## §5. Relation to rows 8b, 8d, 9b; ordering; hand-offs; sibling defects

**Order (PM 434, ratified OD-1 = (C)):** **SD-1 (landed, #605 `b86671a5`) → S1 → 8d → 8c →
`#11-vm-regexp-symbol-replace`**. S1's base contains `b86671a5` (§4.2). 8d and 8c precede the child because
it consumes their ids and `groups`. No step in this order ships an interim RegExp-side replacement loop
(option (B), rejected).

Each hand-off below names the §4.2 cells it must un-ignore. **Un-ignoring them is that owner's acceptance**,
and a cell the owner's PR cannot un-ignore is an open obligation of that owner, not a closed one.

- **9b** (`docs/plans/2026-07-vm-p4-slice-plan.md:983`, `#11-vm-builtin-prototype-static-sweep`). S1
  discharges the non-RegExp halves of the §22.1.3.19 and §22.1.3.20 members. When 9b's derivation
  runs, it counts those halves as landed. The `replaceAll` step 3 work is item 1 of that slot (PM 471;
  `docs/plans/2026-07-vm-p4-slot-ledger.md:303`). That child carries the 8d edge and the child-to-child edge
  onto 8b's String-side `GetMethod` child; row 9b records both as edges 9b's derivation must place (they are
  outputs of the derivation, not of the row). The OD-2 retag ruling stays 9b's (ratified, PM 434), with the
  `natives_string_ext.rs` population as an open input (§0.4). **Un-ignores**: R06; R04 and R05 once
  `#11-vm-regexp-symbol-replace` has landed (both dispatch to it). **Removes** the `replaceAll` step-3
  marker comment (§1.4).
- **8b** (`:979`, UMBRELLA). Its `%Symbol.replace%` child, **`#11-vm-regexp-symbol-replace`** (PM 434;
  id PM 462), is the home of the RegExp half (§22.2.6.11). It comes after S1, 8d and 8c in the PM 434
  order. It **consumes S1's `Replacer` / `MatchRecord` / `replacement_for_match` / `get_substitution`
  unchanged** (I4: `named_captures` is already the raw `JsValue`, and the ToObject is already in the
  `Template` arm), and builds each `MatchRecord` inside its step-15 loop (captures `ToString`'d unless
  undefined; `named_captures` = `Get(result, "groups")` as is). Obligations S1 hands it: collect every
  result before any replacer call (F20, F21); AdvanceStringIndex on an empty match, replace side
  (`vm/natives_string.rs:504` today; PM 454); the step 15.m backwards-position skip; rooting the `results`
  list and each record's `named_captures` across replacer calls (§1.4 GC); deleting the open-coded branch at
  `vm/natives_string.rs:475-526` and the marker comment S1 places in it. **Un-ignores**: F09, F10, F11, F12,
  F14, F16, F18, F19, F20, F21, F22, G05, G08, G10, G11, O10, F13, G06, R08, and whichever of O07, H02, H01,
  H05 are ignored at S1's base; F15 stays green. Its String-side `GetMethod` child (un-minted; 8b's
  derivation) replaces the `ObjectKind::RegExp` brand test at `:475-484` with step 3's `GetMethod`.
  **Un-ignores**: O08 (with 8d). S1 adds **no** new brand site.
- **8d** (`:978`, terminal; slot `#11-vm-regexp-constructor-and-flags (8d)`). No S1 dependency: S1 reads no
  `%Symbol.*%` id, because every S1 path is one where step 3 finds no replacer. Its ids gate 8b's children
  and 9b's `replaceAll` child, not S1. **Un-ignores**: O01.
- **8c** (`:980`). `groups` (O09) gates the `namedCaptures` arm reaching JS; until then that arm is
  Rust-pinned (T-SUB-UNIT, T-I4-UNIT). Its cells F13 / G06 / R08 are un-ignored by
  `#11-vm-regexp-symbol-replace`, which consumes it.
- **Umbrella 8** (`:976`). Two children minted by PM 471, both owner VM P4 and neither a facet of
  `#11-vm-regexp-constructor-and-flags`:
  - **`#11-vm-regexp-builtin-exec-last-index-read`** (SD-3): RegExpBuiltinExec (§22.2.7.2) step 2's
    `? ToLength(? Get(R, "lastIndex"))` is not performed for a regexp that is neither global nor sticky, so a
    `valueOf` on `lastIndex` never runs (X02). The deviation comment is `vm/natives_regexp.rs:39-47`.
    **Un-ignores**: X02.
  - **`#11-vm-regexp-symbol-match`**: RegExp.prototype[%Symbol.match%] (§22.2.6.8) is absent as a method;
    `String.prototype.match`'s open-coded global loop advances an empty match by `m.end() + 1`
    (`vm/natives_string.rs:602`) where step 10.e.ii uses AdvanceStringIndex with `fullUnicode` (§22.2.7.3
    step 6). **Un-ignores**: H03 (if ignored at S1's base).
- **SD-1** (`#11-vm-regexp-builtin-exec-lastindex-overrun`) — landed and closed: #605 `b86671a5`. Its cells are
  in `vm/tests/tests_regexp_last_index_overrun.rs`; it also discharged SD-2 / O05 (PM 454).
- **Already slotted, observed here**: `#11-vm-function-tostring-source-text` (the placeholder text
  in §0.2); `split` / `search` `ToString` the RegExp (X03, 8b's row evidence).
- **Not observed as a defect**: a class constructor called from a native throws TypeError (probe C01
  / C02), so S01's TypeError needs nothing beyond `call_function` (§10.2.1 [[Call]] step 4).

## §6. Re-derivation

Anchors are re-derived at **S1's implementation base** (the origin/main commit S1's implementation branch is
cut from, which must contain `b86671a5`; at the time of writing `47b6c934`), not at `efe8d05b`. The probe's
Actual column alone is at `efe8d05b`.

Probe P1 is a throwaway test module, never committed. It was registered temporarily in
`vm/tests/mod.rs` through `#[path]`, then reverted with `git checkout`. It reads tab-separated
`id<TAB>body` lines from `$PROBE_CASES`, wraps each body as `(function(){ try { var __r = (function(){
<body> })(); return typeof __r + ':' + String(__r); } catch (e) { return 'THROW:' + e.name + ':' +
e.message; } })()`, evaluates each in a fresh `Vm`, and prints `PROBE<TAB>id<TAB>result`.
The hanging cases ran one per process under `timeout 20` against the built test binary. Case text and output
were kept in the session scratchpad.
[evidence] The Node cross-check of §1.3 ran the same bodies as `new Function('"use strict";' + body)()` under Node; it is evidence only (§1.3).

```sh
B=<S1's base>   # 47b6c934 at the time of writing
git merge-base --is-ancestor b86671a5 $B && echo contains-SD-1
git grep -n 'fn native_string_replace\|fn native_string_replace_all' $B -- crates/script/elidex-js/src
git show $B:crates/script/elidex-js/src/vm/natives_string.rs | sed -n 466,544p
git show $B:crates/script/elidex-js/src/vm/natives_string.rs | sed -n 600,604p
git show $B:crates/script/elidex-js/src/vm/natives_string_ext.rs | sed -n 205,252p
git show $B:crates/script/elidex-js/src/vm/natives_regexp.rs | sed -n 22,58p
git show $B:crates/script/elidex-js/src/vm/globals_primitives.rs | sed -n 194,201p
git show $B:crates/script/elidex-js/src/vm/well_known.rs | sed -n 1578,1584p
git show $B:crates/script/elidex-js/src/vm/interpreter.rs | sed -n 360,381p
git grep -n 'gc_enabled = true' $B -- crates/script/elidex-js/src/vm ':!*tests*'          # vm/init.rs:855 only
git grep -n -i -e getsubstitution -e get_substitution $B -- crates/script/elidex-js/src   # no output, rc 1
git grep -n 'replace(/(?:)/g\|x\*/g' $B -- crates/script/elidex-js/src/vm/tests/tests_regexp_last_index_overrun.rs  # no output (H01 / H05 not there)
git grep -c '§21\.' $B -- crates/script/elidex-js/src/vm/natives_string.rs crates/script/elidex-js/src/vm/natives_string_ext.rs   # 10 / 10
.claude/tools/webref heading ecma262 22.1.3.19 --exact
.claude/tools/webref heading ecma262 22.1.3.20 --exact
.claude/tools/webref heading ecma262 22.2.6.11 --exact
.claude/tools/webref heading ecma262 22.2.6.8 --exact
.claude/tools/webref heading ecma262 10.2.1 --exact
.claude/tools/webref aoid ecma262 GetSubstitution   # likewise StringIndexOf IsCallable Call ToObject RequireObjectCoercible IsRegExp RegExpExec RegExpBuiltinExec AdvanceStringIndex
.claude/tools/webref body ecma262 sec-string.prototype.replace     # also sec-string.prototype.replaceall, sec-regexp.prototype-%symbol.replace%, sec-regexp.prototype-%symbol.match%, sec-getsubstitution, sec-stringindexof, sec-regexpbuiltinexec, sec-advancestringindex
.claude/tools/webref coverage-map ecma262 22.1.3.19 ecma262 22.1.3.20 ecma262 GetSubstitution ecma262 22.2.6.11 ecma262 StringIndexOf ecma262 IsCallable ecma262 Call ecma262 IsRegExp ecma262 RegExpExec ecma262 AdvanceStringIndex
```

## §7. Landing obligations

What S1's implementation PR (or, where marked, the PM / PMO at its landing) must do. This memo records them;
it edits neither the ledger nor the slice-plan (PM 471).

- **L-1. Repo ledger rows, in S1's PR (PM 471 item 3).** In one edit, with the precedents
  `docs/plans/2026-11-vm-isconstructor-check.md` (its §5 landing items) and
  `docs/plans/2026-11-vm-native-error-program.md` (its "Landing obligations" bullet) as the form:
  - `docs/plans/2026-07-vm-p4-slot-ledger.md` §8 table (`| Slot | Why deferred | Trigger | Re-eval |`): a row for
    `#11-vm-string-replace-function-replacement`, declaring `UMBRELLA, not a terminal unit` in its `Why deferred`
    cell (the field §8 rows declare the kind in), with its children S1 (landed, this PR) and
    `#11-vm-regexp-symbol-replace`, and its retire condition: that child lands (PM 462); and a row for
    `#11-vm-regexp-symbol-replace` (terminal, owner VM P4, acceptance = un-ignoring its §4.2 cells).
  - `docs/plans/2026-07-vm-p4-slice-plan.md` row **8b** (`:979`): its Slot cell gains a pointer to
    `#11-vm-regexp-symbol-replace` as the `%Symbol.replace%` child.
  - Row **9b** (`:983`): a note that S1 landed the non-RegExp halves of §22.1.3.19 / §22.1.3.20, that the
    `replaceAll` step-3 member is `#11-vm-builtin-prototype-static-sweep` item 1, and the OD-2 open input (§0.4).
  - The figures those files derive from their own rows are re-derived in the same edit, not carried: the
    slot-row count (`grep -cE '^[|] .#11-' docs/plans/2026-07-vm-p4-slot-ledger.md`, 53 at `47b6c934` before the
    two rows), the umbrella-marker program's §8 half, and the registration and carve-marker passages the
    isconstructor precedent names. Then `python3 .claude/tools/plan-memo-umbrella-check.py
    docs/plans/2026-07-vm-p4-es-language-completeness.md` (rc 0) and `bash scripts/python-suites.sh` (which runs
    `docs/plans/test_vm_p4_family_layout.py`).
- **L-2. The new slot ids' homes (PM 471).** `#11-vm-regexp-builtin-exec-last-index-read` and
  `#11-vm-regexp-symbol-match` are children of umbrella 8 (slice-plan `:976`), owner VM P4. At S1's landing
  (their live entries' re-eval point) the PM / PMO checks that each live entry names its §4.2 cell (X02; H03)
  as its acceptance, and that `#11-vm-regexp-builtin-exec-last-index-read` names the in-code comment at
  `vm/natives_regexp.rs:39-47` (which says "umbrella 8's RegExpBuiltinExec child") as its site. S1 does not
  edit that comment (`vm/natives_regexp.rs` is outside S1's touch set).
- **L-3. Re-count the ignored cells at landing, from the tree.** `git grep -c 'known deviation:
  #11-vm-regexp-symbol-replace' -- crates/script/elidex-js/src/vm/tests/tests_string_replace.rs`, and the same
  for each owner string in §4.2. The live entry of `#11-vm-regexp-symbol-replace` ("~23 cells", "S1 plan §5") is
  updated to that figure and to §4.2. No hand count is carried here: it depends on the base values of O07, H02,
  H01 and H05.
- **L-4. Live ledger, for the PM / PMO.** The live entry of `#11-vm-string-replace-function-replacement` gains a
  pointer to S1 (landed: PR and sha) and stays open (PM 462); its premise text is corrected: "returns the
  function's source text" → the native-code placeholder, and "`replaceAll` is likely the same (not probed)" →
  probed, worse (§0.3 items 1-2). The live entry of `#11-vm-regexp-constructor-and-flags`, which lists the
  `replaceAll` non-global TypeError among what it subsumes, is re-pointed to
  `#11-vm-builtin-prototype-static-sweep` item 1 (PM 471 item 2).

## §8. Out of scope, and slots

Homes ruled by PM 434, PM 454, PM 462 and PM 471. Pointers only; the defect text is in §5 and §1.3.

- **SD-1** `#11-vm-regexp-builtin-exec-lastindex-overrun` — **landed and closed** (#605 `b86671a5`).
- **SD-2** — non-global `replace` resetting `lastIndex` (O05): folded into SD-1 (PM 454) and discharged by #605.
- **SD-3** — non-global `exec` never reads `lastIndex` (X02) → **`#11-vm-regexp-builtin-exec-last-index-read`**
  (PM 471; umbrella 8).
- **`%Symbol.match%` and its AdvanceStringIndex (H03)** → **`#11-vm-regexp-symbol-match`** (PM 471; umbrella 8).
- **The RegExp half** (A4, A5, A6 and the RegExp form of A1 / A2) → **`#11-vm-regexp-symbol-replace`**, after
  8d and 8c (OD-1 = (C), PM 434; id PM 462); `replaceAll` step 3 → `#11-vm-builtin-prototype-static-sweep`
  item 1 (PM 471); step 3 of `replace` → 8b's String-side `GetMethod` child (§5).
- **The umbrella slot** `#11-vm-string-replace-function-replacement` stays open after S1 lands, points to S1 and
  `#11-vm-regexp-symbol-replace`, and retires when that child lands (PM 462).
- **The `§21.1.3` → `§22.1.3` retag** of `vm/natives_string.rs` → 9b's derivation (OD-2, ratified, PM 434);
  the `vm/natives_string_ext.rs` cites are an open input to it (§0.4).
- **Already slotted, observed here**: `#11-vm-function-tostring-source-text` (§0.2).

# N2a `IBP-sandbox-flagset` — the sandboxing flag set in restriction form (per-PR plan-memo)

Nested PR **N2a** of node `IBP-sandbox` (`docs/plans/2026-09-ibp-sandbox.md`, approved by #532, `6270e219`; "the node
memo"). The node memo's approval scope (`:6-14`) binds: its **decided** lines (D1, J7, J14, §4's *Home/kind* line for the
flag set, §5.1's N2a row, §5.2's edges n3 and n6a, §1's "N2a narrow" verdict, §3's *Branch* column) are inherited as
decisions; its **input** lines (§4's mechanism prose, §5.3 population (e), §5.4 N2a's named risks, input file
`docs/plans/2026-09-ibp-sandbox-input.md` §B (e), §D, §E C10) are re-derived here.

**Rev 1** (2026-10-24) — first draft, before `/elidex-plan-review`.

**Revision 2** (2026-10-24) — plan-review R1. The polarity flip is made **safe by construction**: `SandboxingFlagSet` is a
newtype over a module-private `bitflags` type, so it has no public `empty()`, `all()`, flag constant or `Option` form;
its only constructors are `unsandboxed()` and `all_represented()` (both `const fn`) and `parse_sandboxing_directive`
(D-N1). Every HEAD spelling of a flag value then fails to compile at the type swap, and the migration population is
**(p) := the compiler's error list** (§5.1), mapped by a total rule over HEAD's *value class* (D-N6, §4.2); populations
(e), (e+), (e!) stay as premise cross-checks, not the gate. G5 is withdrawn, G6 and G7 are rewritten, G3b and G8b
are new; C10d is folded into C10, C10f and C10g are new (§6, §9). Also: (e!)'s count corrected to 8 in `sandbox.rs` / 17
outside (B4); §1 counts two intersecting pairs; B7, B9, D-N2, D-N3, D-N4, §3, §3.1, §5, §7, §8 corrected per the R1
minors; J14's `empty()` is read as the empty set's value (§0.3 R4).

**Revision 3** (2026-10-24) — plan-review R2 re-check; PM-approved, narrow. (1) **G11 and the §5.1 partition rule
((p) ∪ (d)) are dropped**: (p) stays the compiler's error list, recorded per pass with each site's HEAD value class →
new spelling, with no accounting rule; G10 stays, with C10e / C10g named as ADDED cells. (2) **The operator surface is
by construction**: the new type implements no `BitOr` / `BitOrAssign` / `BitAnd` / `BitXor` / `Not` / `Sub` (or `Add`),
so a HEAD `a | b` of allow bits fails to compile and joins (p); the J7 union is a named method,
`union_restrictions`; cell C10h checks the absence (grep gate — the workspace has no `trybuild`). (3) the
`sandbox.rs` test module sees the private `Flags`, so its treatment is stated (D-N6). (4) "no `Option`" is restated as a
property of parameter / field / return types and G3 / G3b are widened. (5) §8 hands N5 the `unsandboxed()` reading of
J14's DoD grep. (6) G8a no longer bans "maximum restriction". Rev 3 adds no other gate, count or partition.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`d306af5a`** coordinate (origin/main at the branch cut). The node memo's frame is `f1cf5d67`;
`git diff --stat f1cf5d67 d306af5a -- crates/core/elidex-plugin crates/script crates/shell/elidex-shell/src/content/iframe
crates/shell/elidex-shell/src/pipeline.rs crates/shell/elidex-shell/src/content/link_nav.rs
crates/shell/elidex-shell/src/content_iframe_security_tests.rs crates/shell/elidex-shell/src/content_window_open_tests.rs`
is empty (verified 2026-10-24 via that command), so every node-memo coordinate in N2a's files holds unchanged. Spec text
is `.claude/tools/webref body` output of 2026-10-24; every `§X.Y = name` pair is `webref heading` / `dfn` output. `grep`
in commands is `git grep`; `-P` wherever `\b` appears (this git's `-E` has no `\b` and returns a silent 0).

### §0.2 Premises re-measured

| # | Claim | Command | Result |
|---|---|---|---|
| B1 | The flag type, its parser and its predicates live in one module | `sed -n 1,181p crates/core/elidex-plugin/src/sandbox.rs` | holds: `IframeSandboxFlags` (`:25`, seven allow bits, `u16`), `parse_sandbox_attribute` (`:58`), six predicates over `Option<IframeSandboxFlags>` (`:105`–`:181`); re-export `crates/core/elidex-plugin/src/lib.rs:66` |
| B2 | Population (e) = 125 occurrences in 16 files (input §B) | `git grep -c IframeSandboxFlags -- crates` | **holds**, verified 2026-10-24 (125 / 16, per-file table §5.1) — but the count is by the type's **name**; the property that matters is "every site whose text encodes the old polarity", which is wider (B3, B4, B13) |
| B3 | (re-derived) value-flow sites that encode the old polarity without naming the type | §5.1's command (e+) | **45 lines, 9 files** (verified 2026-10-24), one of them production-only and outside (e)'s 16: `crates/script/elidex-js/src/vm/host/window_dialogs.rs:214` |
| B4 | (re-derived) **silent-flip** sites: `empty()`/`all()` of the type, which type-check under both polarities and mean the opposite after a rename | `git grep -n -P '(IframeSandboxFlags\|\bF)::(empty\|all)\(\)' -- crates` | verified 2026-10-24: 25 occurrences in 10 files; **8 in `sandbox.rs`** itself, **17 in 9 files outside it, all in tests** (§5.1 (e!)); 0 in production outside `sandbox.rs` |
| B5 | The `top_navigation_allowed` implication (`allow-top-navigation` ⇒ by-user-activation) lives in the predicate | `sed -n 143,170p crates/core/elidex-plugin/src/sandbox.rs` | holds (`:166-169`); the spec puts it in the **parse** (*parse a sandboxing directive* step 3, the with-user-activation arm "unless … either … or …") |
| B6 | `apply_sandbox_origin` open-codes a flag test, against the module's "every capability check delegates here" contract (`sandbox.rs:9-14`) | `sed -n 541,553p crates/shell/elidex-shell/src/content/iframe/load.rs` | holds: `:545-546` `if let Some(flags) … !flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN)` |
| B7 | Readers that default on a **missing `HostData`** (unbound) through the flag *value* | `git grep -n -E 'and_then\(.*sandbox_flags' -- crates` | two: `crates/script/elidex-js/src/engine.rs:666`, `crates/script/elidex-js/src/vm/host/window_dialogs.rs:214`; the verdict-defaulting sites do not name the value: `is_none_or(\|hd\| hd.*_allowed())` (`engine.rs:674`, `:682`; `window_dialogs.rs:52`) and `.is_some_and(\|hd\| !hd.scripts_allowed())` (`vm/host/event_handler_attrs.rs:553`) — the same default (unbound → not refused) |
| B8 | No struct holding the flags derives `Default` (so the set needs no `Default`) | `grep -n 'derive' crates/shell/elidex-shell/src/pipeline.rs`; `sed -n 855,870p crates/script/elidex-js/src/vm/host_data/mod.rs` | holds: `PreEvalFrameState`/`PreEvalFrameInputs` derive nothing; `HostData::new` sets `sandbox_flags: None` explicitly (`:863`) |
| B9 | The modules' docstrings and comments name the old contract and deleted code | G8a's and G8b's commands (§9) at `d306af5a` over §5.1's 21 touched files; plus G1's command for comments naming the type | **15 lines** (verified 2026-10-24 via those commands; rev 3's G8a drops the `maximum restriction` term, which five still-correct or type-bound comment lines use — `sandbox.rs:23`, `:92`, `:327`, `tests_window_open_modals.rs:246`, `navigation.rs:376`; `:23` and `:92` name the deleted type and are G1's): `sandbox.rs:2`, `:3`, `:11`, `:12`, `:94`, `:156`; `host_data/mod.rs:188`, `:1091`, `:1110`, `:1111`; `tests_window_open_modals.rs:132`; `elidex-script-session/src/engine.rs:517`; `content/iframe/load.rs:400`; `pipeline.rs:55`, `:83`. G1 adds the comment lines naming `IframeSandboxFlags` (e.g. `sandbox.rs:23`, `:92`; `host_data/mod.rs:190`, `:1093`; `elidex-script-session/src/engine.rs:514`). **Filter**: sandbox-domain hits only — `boa` is searched in `sandbox.rs` alone (G8b), because the touched files' other `boa` hits are S1/S5 cutover history unrelated to the flag contract (e.g. `pipeline.rs:219`, `navigation.rs:57`), and G8a's terms run over the touched files only, which excludes the unrelated `Some(empty)` hits elsewhere (`vm/host/fetch/dispatch.rs:656`, `vm/host/request_response/request_ctor.rs:341`, `vm/host/url/setters.rs:74`) |
| B10 | No HEAD feature reads one of the seven flags D1 leaves unrepresented | `git grep -n -i -E 'autofocus\|"domain"\|document_domain\|autoplay\|requestPointerLock\|pointer_lock' -- 'crates/**/*.rs'` less tests | holds: the only hits are the focus-delegate walk (`elidex-dom-api/src/focus/delegate.rs`), which is not *flush autofocus candidates* (slot `#11-flush-autofocus-candidates-page-load`, `delegate.rs:163`); no `document.domain` setter, Pointer Lock, downloads, orientation lock, presentation |
| B11 | Files N2a touches and the 1000-line rule | `git show d306af5a:<file> \| wc -l` on §5.1's touched files (verified 2026-10-24) | one file >1000: `crates/script/elidex-js/src/vm/host_data/mod.rs` (2023); watch: `crates/shell/elidex-shell/src/pipeline.rs` (973) |
| B12 | The iframe's flag set is read from the attribute at load | `sed -n 399,407p crates/shell/elidex-shell/src/content/iframe/load.rs` | `parse_sandbox` (`:401`): absent → `None`, present → parse; spec (§4.8.5): post-connection steps step 2 parses, attribute set/changed re-parses, removal empties, and "these flags only take effect when the content navigable … is navigated" — on the **in-process** load paths a read at navigation is observationally the maintained set. The OOP `Navigate` rebuild (`content/iframe/thread.rs:227`) instead re-reads the retiring document's stored set (B14) |
| B13 | (re-derived) old-polarity sites that **no** vocabulary population lists: positional `None` arguments | `sed -n 155,291p crates/script/elidex-script-session/src/scripting.rs` | **11** bare `None` flag arguments in `scripting.rs`'s tests (`:160`, `:168`, `:179`, `:185`, `:196`, `:221`, `:229`, `:254`, `:267`, `:274`, `:288`), none in (e), (e+) or (e!) — the motivating case for (p) (NR1) |
| B14 | Callers of the retyped API that compile unchanged (the value only flows) | `git grep -n 'sandbox_flags()' -- crates`; `sed -n 602,619p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | four files outside (e) ∪ (e+): `vm/host/event_handler_attrs.rs:602`, `:611`, `:618`; `content/iframe/thread.rs:227` (OOP `Navigate` rebuild — no production sender, `thread.rs:233-235`); `content/link_nav.rs:57`; `tests_engine_s1d.rs:97` (`let _ = engine.sandbox_flags();`) |

### §0.3 Premise errors (reported, not edited — the A114 idiom)

None in a **decided** line. D1, J7, J14, §4's flag-set *Home/kind*, §5.1's N2a row, n3, n6a and §3's N2a *Branch* cell
("nine represented flags … implied grants at parse; unrepresented → `#11-sandbox-flag-set-sources`") are each consistent
with HEAD and the spec as re-measured (§0.2, §3).

Findings against **input** lines (re-derived here; they change no decision, so they are recorded as this PR's named
risks, §5.4 of the node memo's idiom):

- **R1** input §B (e)'s command counts by the type's name; its rewrite target "`None` → `empty()`" covers one of the two
  polarity mappings. The other — HEAD's `Some(IframeSandboxFlags::empty())`, *maximum restriction* — would become the new
  type's `empty()`, *unsandboxed*, under a rename, and still type-check (B4); and bare `None` arguments are named by no
  vocabulary at all (B13). Re-derived: the population is defined by the **compiler** (§5.1 (p)), made total by D-N1
  (no spelling of HEAD's values compiles against the new type) and mapped by HEAD value class (D-N6, §4.2); (e), (e+),
  (e!) are kept as premise cross-checks.
- **R2** input §D assigns the top-navigation predicate's restatement to N2b; N2a cannot compile without re-typing it, and
  the spec moves its implication into the parse (B5). Re-derived: N2a restates its body over the restriction set with
  its signature kept (D-N3); N2b still owns its absorption into §7.4.2.4 (D-a).
- **R3** J14 names the interim **stores** (`HostData`, `PreEvalFrameState`) and their "not a sandboxed iframe" value; it
  does not name the two **unbound-default** reads (B7), whose HEAD value is also `None`. They are not a store; §5.1 of
  the node memo already keeps them HEAD's ("before N4 an unbound gate reads HostData as at HEAD", node memo `:534-535`).
  Re-derived: D-N5.
- **R4** (reading, not a conflict) J14 (node memo `:393`) and the node memo's open-window list (`:534`) spell the
  interim "not sandboxed" value `empty()`. The decided content is the **value** — the spec's empty sandboxing flag set
  ("its active sandboxing flag set must be empty", §7.1.5) — and the stores that hold it; N2a holds exactly that value in
  exactly those stores, spelled `SandboxingFlagSet::unsandboxed()`, because D-N1 exposes no method named `empty()` (an
  `empty()` that means *unsandboxed* on the new type and *maximum restriction* on HEAD's is the silent-flip vector, R1).
  Consequence for an input line: N5's "J14's DoD (no interim `empty()` survives)" (node memo `:620`) greps
  `unsandboxed()` at the deleted stores.

### §0.4 Decisions (this PR)

Inherited (node memo, decided): **D1** — `SandboxingFlagSet` replaces `IframeSandboxFlags`, restriction form, the
sandboxed navigation browsing context flag represented; **J7** — union in restriction form from N2a; **J14** — N2a to
N5 the interim stores hold the empty set for HEAD's "not a sandboxed iframe" (spelled `unsandboxed()`, R4); Home
`elidex-plugin::sandbox`. **D7** (E26 gates the node's first nested landing) is met at the base: the E26 umbrella memo's
approval is `d306af5a` (#541), the branch cut.

- **D-N1 — the represented set, safe by construction.** `SandboxingFlagSet` (NEW) is a **newtype** —
  `pub struct SandboxingFlagSet(Flags)` — over a **module-private** `bitflags` type `Flags` over `u32` (the spec's list
  has sixteen flags and grows; `u32` leaves room for `#11-sandbox-flag-set-sources` without a re-type) with exactly the
  nine flags §3's decided *Branch* cell names, each constant named after its spec flag: `SANDBOXED_NAVIGATION`,
  `SANDBOXED_AUXILIARY_NAVIGATION`, `SANDBOXED_TOP_LEVEL_NAVIGATION_WITHOUT_USER_ACTIVATION`,
  `SANDBOXED_TOP_LEVEL_NAVIGATION_WITH_USER_ACTIVATION`, `SANDBOXED_ORIGIN`, `SANDBOXED_FORMS`, `SANDBOXED_SCRIPTS`,
  `SANDBOX_PROPAGATES_TO_AUXILIARY_BROWSING_CONTEXTS`, `SANDBOXED_MODALS` (all NEW, all on the private `Flags`).
  Public surface, exhaustively: `SandboxingFlagSet::unsandboxed()` (NEW, `const fn`, `Self(Flags::empty())` — the
  spec's empty set, HEAD's `None`); `SandboxingFlagSet::all_represented()` (NEW, `const fn`, `Self(Flags::all())` —
  "every flag this representation holds", the parse of an empty directive (C10 asserts the equality) and the value F10r
  returns from N4/N5; the name carries that the seven unrepresented flags are absent from it);
  `parse_sandboxing_directive` (D-N2); `union_restrictions(self, other: Self) -> Self` (NEW — the union, J7; the
  polarity is in the name: the result is restricted at least as much as either operand, which is the reverse of what
  `a | b` meant on HEAD's allow-form bits; it is `pub` because J7 decides the union from N2a and its callers, N7 and N8
  (n6a), are outside the module — a private helper would be dead code in N2a's production build); derives `Clone,
  Copy, Debug, PartialEq, Eq, Hash` (the set-valued test asserts, D-N6). There is **no** public `empty()`, `all()`,
  `contains`, `from_bits*`, flag constant, `Default` or operator, and **no `Option<SandboxingFlagSet>` in any
  parameter, field or return type** (G3, G6, C10h): the type implements **none** of `BitOr`, `BitOrAssign`, `BitAnd`,
  `BitAndAssign`, `BitXor`, `BitXorAssign`, `Not`, `Sub`, `SubAssign`, `Add`, `AddAssign` — any set operator whose
  meaning flips between allow-form and restriction-form is absent by construction, because an operator that compiles
  against both forms is the silent-flip vector (R1) in operator position. Every HEAD spelling of a flag value or
  combination — `F::empty()`, `F::all()`, an `ALLOW_*` constant, a bare bit binding, `a | b`, `None`, `Some(..)`,
  `Option<F>` — fails to compile against the new type, which is what makes (p) the complete population (§5.1). **No `Default`**: a defaulted set would be an implicit *unsandboxed*;
  every construction names its value (fail-safe direction; B8 shows nothing needs it). A hand-built restriction set that
  omits a flag would *grant* it; with the constants private, no code outside the module can build one. The seven
  unrepresented flags (pointer lock, automatic features, `document.domain`, orientation lock, presentation, downloads,
  custom protocols navigation) are `#11-sandbox-flag-set-sources`' (D1 restatement, node memo §8); the type's docstring
  lists them and the slot (B10: no HEAD reader).
- **D-N2 — one parser, the spec's algorithm.** `parse_sandboxing_directive(input: &str) -> SandboxingFlagSet` (NEW)
  replaces `parse_sandbox_attribute` and implements HTML §7.1.5 *parse a sandboxing directive* steps 1–3: split on ASCII
  whitespace; start empty; **add** each represented flag unless its clearing token is present. So the implied grant
  (the with-user-activation flag is cleared by `allow-top-navigation-by-user-activation` **or** `allow-top-navigation`)
  is in the parse, the navigation flag is added unconditionally, and `allow-popups-to-escape-sandbox` clears the
  propagate flag (a token HEAD ignores; its only reader is N8's, so recognising it changes no HEAD verdict). Tokens match
  ASCII case-insensitively, as at HEAD: §4.8.5 defines the `sandbox` attribute's value as tokens "that are ASCII
  case-insensitive", so step 3's "tokens contains" is matched accordingly for the attribute; how the CSP `sandbox`
  directive's tokens compare is N6's to decide when it routes CSP through this parser. The parse is additive from empty,
  so an unrecognised or mis-cased token **clears nothing** — the flag step 3 adds stays set; the unknown falls on the
  restricted side by construction. The one parser serves the attribute now and the CSP `sandbox` directive at N6 (n6a);
  the name is the spec's, not the attribute's.
- **D-N3 — predicates over the set, one read API.** Every predicate takes `SandboxingFlagSet` by value (no `Option`):
  `scripts_allowed`, `forms_allowed`, `popups_allowed`, `modals_allowed`, `scripting_enabled` (restated, input §D),
  `top_navigation_allowed(flags, has_transient_activation)` (restated with its signature: activation → the
  with-user-activation flag, else the without flag — §7.4.2.4 steps 3.2/3.3 literally; N2b absorbs it), and
  **`same_origin_allowed`** (NEW) — `!flags.0.contains(Flags::SANDBOXED_ORIGIN)`, the *determine the origin* step 1
  test that `apply_sandbox_origin` open-codes today (B6). Outside `elidex-plugin/src/sandbox.rs` no code can name a flag
  constant or test a flag (D-N1, by construction; G4 is the textual backstop); every reader goes through a predicate,
  every constructor through `parse_sandboxing_directive`, `unsandboxed()` or `all_represented()`. This keeps S5-4 §4.1's
  ratified predicate home and its module contract (`sandbox.rs:9-14`), now with no exception. The S5-4 §4.1 *Shape*
  block's contract comment ("`None` = unsandboxed (all allowed); `Some(empty)` = max restriction",
  `docs/plans/2026-07-s5-4-sandbox-enforcement.md:525`) is superseded together with that section's rejection
  (`:538-542`) by D1's recorded amendment (node memo `:152-155`); N2a records nothing new there.
- **D-N4 — the interim stores (J14), retyped in place.** `HostData.sandbox_flags` (`host_data/mod.rs:200`, initializer
  `:863`), `PreEvalFrameState.sandbox_flags` (`pipeline.rs:56`), `PreEvalFrameInputs.sandbox_flags` (`pipeline.rs:84`),
  the `ScriptEngine` pair `set_sandbox_flags` / `sandbox_flags` (`elidex-script-session/src/engine.rs:515`, `:519`; impl
  `elidex-js/src/engine.rs:655`, `:661`), the session functions' parameters (`scripting_disabled_for_platform_object`,
  `elidex-script-session/src/scripting.rs:87`; `window_open_disposition`, `elidex-script-session/src/navigation.rs:272`)
  and the `iframe/load.rs` parameters (`:89`, `:383`, `:421`, `:452`, `:542`) take `SandboxingFlagSet`; `None` →
  `unsandboxed()`. `parse_sandbox` (`load.rs:401`) returns `unsandboxed()` for an absent attribute (§4.8.5: removal
  empties the iframe sandboxing flag set). Names (`sandbox_flags`, `parse_sandbox`) are kept: N5 deletes the stores and
  N7 owns the iframe set's computation, so renaming here is churn they undo.
- **D-N5 — the unbound defaults stay HEAD's.** The two value reads on a missing `HostData` (B7) map it to
  `unsandboxed()` — HEAD's `None`, permissive — through `map_or(SandboxingFlagSet::unsandboxed(), …)`; each carries a
  one-line comment naming J2 and N4. N2a makes no gate fail closed: that is J2's, from N4 (node memo §2), and D1 is
  "re-keyed in place" (node memo §5.1 *Closed*). The two sites are N4's population, handed over by edge n3 (§8); G7 names
  them and C10g pins their value.
- **D-N6 — the mapping rule, by HEAD value class (total).** Every site of (p) is rewritten by the class of the **value**
  its HEAD text denotes, not by its spelling (§4.2 is the full table):
  - `None` (literal, positional, field initializer or `const`) → `SandboxingFlagSet::unsandboxed()`;
  - `Some(IframeSandboxFlags::empty())` (any alias) → `SandboxingFlagSet::all_represented()`;
  - `Some(<allow bits>)` → `parse_sandboxing_directive("<the same tokens>")` — HEAD's bit `ALLOW_X` is the token
    `allow-x`, and `IframeSandboxFlags::all()` is all seven of HEAD's tokens;
  - HEAD `a | b` of allow bits (`sandbox.rs:340-341` `F::ALLOW_TOP_NAVIGATION | by_ua`) →
    `parse_sandboxing_directive("<tokens of a> <tokens of b>")`: it is the union of *allows*, i.e. the parse of both
    tokens, never `union_restrictions` (the two unions are duals); a HEAD **bit-value binding** (`sandbox.rs:322`,
    `navigation.rs:368` `let by_ua = F::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION`) becomes a binding of the token string,
    and each `Some(by_ua)` use becomes `parse_sandboxing_directive(by_ua)`. These are all the HEAD sites of the class
    (verified 2026-10-24: `git grep -n -P 'ALLOW_[A-Z_]+\s*\||\|\s*(F::|IframeSandboxFlags::)?ALLOW_' -- crates` → the two
    lines `sandbox.rs:340`, `:341`; `git grep -n -P '=\s*(F|IframeSandboxFlags)::ALLOW_[A-Z_]+;' -- crates` → the two
    bindings);
  - a type position `Option<IframeSandboxFlags>` / `Option<F>` → `SandboxingFlagSet`; an import of
    `IframeSandboxFlags` (with or without `as F`) → `SandboxingFlagSet`, unaliased (G3b);
  - a HEAD **reader** of the option (`is_none_or`, `if let Some`, `and_then`) → §4.2's reader rows;
  - **any other HEAD expression → stop and record** in the (p) record (§5.1) before choosing a spelling.
  `const` fixtures use the two `const fn`s (`navigation.rs:304` `const UNSANDBOXED` → `unsandboxed()`, `:305`
  `const MAX_RESTRICTION` → `all_represented()`; names kept so their use sites compile unchanged). Production builds a
  present attribute's set with `parse_sandboxing_directive`, and C10 pins `parse_sandboxing_directive("") ==
  all_represented()`, so every fixture value is one production produces (user CLAUDE.md "fixture は実物の形を反映させる").
  **The `sandbox.rs` test module** (`mod tests`, `:184` to the end of the file; `use super::*` makes the private
  `Flags` and its constants visible there, so the compiler guarantee of D-N1 does not hold inside it) constructs values
  **only** through the public constructors — `unsandboxed()`, `all_represented()`, `parse_sandboxing_directive` — and
  reads them only through the predicates or `==`; a flag without a predicate (navigation, propagate) is observed by
  equality between two parses, not through `Flags`. Its two halves are treated differently. The **predicate tests**
  (`:284`–`:394`) are **migrated** by the value-class rule: each assert keeps its predicate and its polarity, only the
  value expression inside it is rewritten (inline value expressions are why G10's textual check is for the other
  migrated files). The **token-parser tests** (`:186`–`:282`) assert through `contains` / `is_empty` on the old type,
  which the new type does not expose, so they cannot migrate; they are **replaced** by C10 / C10b, which carry every
  scenario they hold (empty string, single / multiple / unrecognised tokens, case folding, all seven tokens, the
  `allow-top-navigation-by-user-activation` token and its implication).
  The assertion lines of the other migrated test files do not change except the set-valued equality asserts, which change only
  their expected value (`tests_engine_s1b.rs:166`, `:172`, `:196`; `content_iframe_security_tests.rs:119`, `:232`,
  `:247`, `:263`, `:283`).

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| Representation | restriction form; nine flags on a module-private type; public constructors `unsandboxed()`, `all_represented()`, `parse_sandboxing_directive` only; union by the named `union_restrictions`; no `empty()`, `all()`, constant, operator impl, `Option` form or `Default` |
| Parse | *parse a sandboxing directive* steps 1–3, additive from empty; implied grant at parse |
| Readers | every verdict through a predicate in `elidex-plugin::sandbox`; every HEAD verdict unchanged |
| Interim stores (J14) | `None` → `unsandboxed()`; unbound defaults HEAD's (D-N5); population (p) = the compiler's, mapped by value class |

**Edge density: not edge-dense — single PR** (*verdict*). The node memo decided "N2a narrow — one type re-keyed in
place, with the sandboxed navigation flag added" (§1) and split it from N2b so the polarity flip is reviewed alone. Re-
measured: the PR has one canonical algorithm (*parse a sandboxing directive*), no new reader, no new store, and no
runtime change; of the invariant pairs in §2 two intersect (K1 × K3, K1 × K4), both through K1 and neither touching a
third axis — below CLAUDE.md's ≥3-intersecting-axes trigger. It is an approved narrowly-scoped slice under an approved
node, the rule's base case (c); the plan-review is still mandatory (rule (b): the node is edge-dense). Breadth: K = 1
spec, M = 9 rows (§3), "ok → single PR scope" (`webref coverage-map`).

## §2. Coupled invariants

- **K1 Behaviour equivalence** — for every token string `T` over HEAD's seven tokens and for the absent attribute, each
  predicate's verdict after N2a equals HEAD's (HEAD's verdict = presence of the clearing token; absent = permissive).
  Includes the unbound defaults (D-N5) and `apply_sandbox_origin`.
- **K2 Restriction-form union (J7)** — `a.union_restrictions(b)` restricts at least as much as `a` and as `b`: for every
  single-flag predicate `p`, `p(a.union_restrictions(b)) == p(a) && p(b)`; `unsandboxed()` is the identity and `all_represented()` absorbs. (No N2a
  reader; N7's embedder union and N8's popup copy rely on it, n6a.)
- **K3 Interim mapping (J14)** — every site of (p) is rewritten by HEAD value class (D-N6): `None` → `unsandboxed()`,
  `Some(empty)` → `all_represented()`, `Some(A)` → `parse(tokens(A))`, HEAD `a | b` → `parse` of both token lists; no operator impl and no
  `Option<SandboxingFlagSet>` in a parameter, field or return type exists (D-N1, by construction; C10h, G3).
- **K4 Parse totality** — `parse_sandboxing_directive("") == all_represented()`, so every represented constant has a
  parse arm (a constant added without one fails the cell).

Pairs: **K1 × K3** — the mapping is what makes K1 hold at the stores and fixtures: `None` → `unsandboxed()` is
permissive for every predicate (HEAD's `is_none_or`; C10f), `Some(empty)` → `all_represented()` refuses every predicate,
and `Some(A)` → `parse(tokens(A))` is the same verdict per predicate because HEAD's bit `A ∋ ALLOW_X` and the new arm
"add `SANDBOXED_X` unless `allow-x`" read the same token (the top-navigation pair: HEAD's disjunction
`TOP_NAV ∨ (act ∧ BY_UA)` equals "activation ? ¬with-UA : ¬without-UA" over the parsed set, because the with-UA arm is
cleared by either token). The construction (D-N1) is what makes K3 range over **every** site: a site the rule does not
reach does not compile. **K1 × K2** — none: no N2a reader takes a union (and HEAD's `a | b`, a union of allows, is mapped by `parse`, never by `union_restrictions`, D-N6). **K1 × K4** — K4's empty directive is HEAD's
`Some(empty())`: every predicate false, so K1's max-restriction row is K4's value and the `Some(empty)` →
`all_represented()` class is sound. **K2 × K3**, **K2 × K4**, **K3 × K4** — none beyond the shared type.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.1.5 Sandboxing | *sandboxing flag set* (the flag list) | nine represented (navigation; auxiliary navigation; top-level navigation without / with user activation; origin; forms; scripts; propagates to auxiliary browsing contexts; modals); seven unrepresented → `#11-sandbox-flag-set-sources` (D1) | `crates/core/elidex-plugin/src/sandbox.rs` `SandboxingFlagSet` (NEW) | ✗ | no |
| WHATWG HTML §7.1.5 Sandboxing | *parse a sandboxing directive* steps 1–3 | step 1 ASCII-whitespace split; step 2 empty; step 3 the nine represented arms incl. the unconditional navigation flag and the two-token with-user-activation arm (implied grant at parse); unrecognised tokens clear nothing; the seven unrepresented arms → slot | `parse_sandboxing_directive` (NEW) | ✗ | yes |
| WHATWG HTML §4.8.5 The iframe element | `sandbox` attribute (ASCII case-insensitive tokens); post-connection steps step 2; set / changed / removed | absent → `unsandboxed()`; present → parse; read when the iframe loads (flags take effect only at navigation, §4.8.5) | `crates/shell/elidex-shell/src/content/iframe/load.rs:401` `parse_sandbox` | ✓ | yes |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | settings clause ("does not have its sandboxed scripts browsing context flag set") | `scripting_enabled` over the set; the composition is N4's | `sandbox.rs` `scripting_enabled`; callers `crates/script/elidex-script-session/src/scripting.rs:89`, `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:611` | ✓ | no |
| WHATWG HTML §8.9.1 Simple dialogs | *cannot show simple dialogs* step 1 | sandboxed modals flag | `modals_allowed`; `crates/script/elidex-js/src/vm/host_data/mod.rs:1139` | ✓ | no |
| WHATWG HTML §7.3.1.7 Navigable target names | *the rules for choosing a navigable* step 8 (the sandboxed-auxiliary-navigation option) | aux-nav flag → popup blocked; step 8's create-a-new-top-level-traversable option, its substeps 8.2 (navigation flag) and 9 (propagate flag) → N8 / `#11-browsing-context-model-window-open-postmessage` | `popups_allowed`; `crates/script/elidex-script-session/src/navigation.rs:286`, `:293`; `crates/shell/elidex-shell/src/content/link_nav.rs:46` | ✗ | yes |
| WHATWG HTML §7.4.2.4 Preventing navigation | *allowed by sandboxing to navigate* steps 3.2, 3.3 | the two top-level flags × activation; steps 1, 2, 3.1, 3.4, 4, 5, 6 → N2b (D-a) | `top_navigation_allowed`; `navigation.rs:279`; `link_nav.rs:56` | ✗ | yes |
| WHATWG HTML §4.10.22.3 Form submission algorithm | step 4 | sandboxed forms flag; which document's set → N5 | `forms_allowed`; `crates/shell/elidex-shell/src/content/form_input.rs:129` via `crates/script/elidex-js/src/engine.rs:669` | ✗ | yes |
| WHATWG HTML §7.3.2.1 Creating browsing contexts | *determine the origin* step 1 | sandboxed origin flag → opaque; steps 2–5 → N6 | `same_origin_allowed` (NEW); `load.rs:545` `apply_sandbox_origin` | ✗ | yes |

Numbers and titles: `webref heading html 7.1.5` (Sandboxing), `4.8.5` (The iframe element), `8.1.3.4`, `8.9.1`,
`7.3.1.7`, `7.4.2.4`, `4.10.22.3`, `7.3.2.1`; `webref dfn html 'parse a sandboxing directive'` → §7.1.5
`#parse-a-sandboxing-directive`; `'determine the origin'` → §7.3.2.1 `#determining-the-origin`; `'cannot show simple
dialogs'` → §8.9.1; `'The rules for choosing a navigable'` → §7.3.1.7. Step texts: `webref body html` on
`parse-a-sandboxing-directive`, `attr-iframe-sandbox`, `the-iframe-element`, `allowed-to-navigate`,
`cannot-show-simple-dialogs`, `determining-the-origin`, `form-submission-algorithm`, `the-rules-for-choosing-a-navigable`,
`enabling-and-disabling-scripting` (2026-10-24). Breadth: K = 1, M = 9 rows.

### §3.1 User-input touch audit

The one author-controlled input is the `sandbox` attribute string (`IframeData.sandbox`,
`crates/core/elidex-ecs/src/components.rs:704`, `:745`). Its path: `parse_sandbox` → `parse_sandboxing_directive` →
stored set → predicates. Cases: ASCII whitespace (TAB LF FF CR SPACE — `str::split_ascii_whitespace`, which excludes
U+000B like Infra) ; mixed case (ASCII-folded, as at HEAD); non-ASCII look-alikes (`ALLOW-SCRİPTS`) are not folded, so
the flag stays set; duplicates idempotent; unknown tokens clear nothing, and the five tokens that clear only
unrepresented flags (`allow-downloads`, `allow-orientation-lock`, `allow-pointer-lock`, `allow-presentation`,
`allow-top-navigation-to-custom-protocols`) clear nothing represented; empty / whitespace-only value →
`all_represented()`. Every mismatch leaves a restriction set (D-N2). No new input surface; CSP `sandbox` reaches the
parser at N6.

## §4. Mechanism per fact

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| sandboxing flag set (§7.1.5) | `SandboxingFlagSet` (NEW), a newtype over a module-private `bitflags` type over `u32`, `crates/core/elidex-plugin/src/sandbox.rs`; re-exported at `elidex_plugin::SandboxingFlagSet` (NEW) (`lib.rs:66` line replaced) | decided (D1, node §4); newtype, `u32`, private constants, no `Default`: D-N1 |
| union of restriction sets (J7) | `SandboxingFlagSet::union_restrictions` (NEW); no operator impl (C10h) | D-N1; node memo J7 |
| the empty set (unsandboxed) | `SandboxingFlagSet::unsandboxed()` (NEW, `const fn`) | D-N1; J14's value (R4) |
| every represented restriction | `SandboxingFlagSet::all_represented()` (NEW, `const fn`) | D-N1 |
| parse a sandboxing directive | `parse_sandboxing_directive` (NEW), same module; re-exported beside the type | D-N2 |
| capability verdicts | `sandbox::{scripts_allowed, forms_allowed, popups_allowed, modals_allowed, top_navigation_allowed, scripting_enabled, same_origin_allowed}` — `same_origin_allowed` NEW | D-N3 |
| iframe sandboxing flag set | `parse_sandbox` (`load.rs:401`), shell marshalling over `IframeData` | D-N4 (N7 owns its later form) |
| interim stores | `HostData.sandbox_flags`, `PreEvalFrameState.sandbox_flags`, `PreEvalFrameInputs.sandbox_flags`, `ScriptEngine::{set_sandbox_flags, sandbox_flags}` | D-N4 (J14; deleted by N5) |

### §4.2 Rewrite mapping (HEAD → N2a)

Value classes (constructors; D-N6 — total over HEAD values, anything else stops and is recorded):

| HEAD value class | N2a expression | Why equal (K1) |
|---|---|---|
| `None` — literal, positional argument, field initializer (`host_data/mod.rs:863`), `const` (`navigation.rs:304`), `parse_sandbox`'s absent arm | `SandboxingFlagSet::unsandboxed()` | every predicate permissive on both (C10f) |
| `Some(IframeSandboxFlags::empty())` / `Some(F::empty())`, incl. `const` (`navigation.rs:305`) | `SandboxingFlagSet::all_represented()` | every predicate false on both (K1 × K4; C10) |
| `Some(ALLOW_X \| ALLOW_Y …)`; HEAD `a \| b` of allow bits (`sandbox.rs:340-341`) | `parse_sandboxing_directive("allow-x allow-y …")` — `<tokens of a> <tokens of b>` | per-predicate token presence (§2 K1 × K3; C10b); never `union_restrictions` (dual union) |
| bit-value binding `let by_ua = F::ALLOW_…` (`sandbox.rs:322`, `navigation.rs:368`) | a token-string binding; `Some(by_ua)` → `parse_sandboxing_directive(by_ua)` | as the row above |
| `Some(IframeSandboxFlags::all())` (`sandbox.rs:390`) | `parse_sandboxing_directive` of HEAD's seven tokens | every predicate true on both |
| `parse_sandbox_attribute(s)` (`load.rs:405`) | `parse_sandboxing_directive(s)` | C10b |

Type and import positions:

| HEAD | N2a |
|---|---|
| `Option<IframeSandboxFlags>` / `Option<F>` (signature, field, local, tuple element) | `SandboxingFlagSet` |
| `use …IframeSandboxFlags` / `use …IframeSandboxFlags as F` (`sandbox.rs:321`, `navigation.rs:299`, `content_window_open_tests.rs:400`) | `use …SandboxingFlagSet`, no alias (G3b) |

Readers (HEAD expressions over the option; pre-recorded here, so they are not "stop and record" at implementation):

| HEAD reader | N2a expression | Why equal (K1) |
|---|---|---|
| `flags.is_none_or(\|f\| f.contains(ALLOW_X))` | `!flags.0.contains(Flags::SANDBOXED_X)` (inside `sandbox.rs` only) | — |
| `if let Some(f) = flags { !f.contains(ALLOW_SAME_ORIGIN) }` (`load.rs:545-546`) | `!sandbox::same_origin_allowed(flags)` | absent → `unsandboxed()` → allowed, as HEAD's `None` arm |
| `iframe_data.sandbox.as_deref().map(parse_sandbox_attribute)` (`load.rs:402-405`) | `….map_or(SandboxingFlagSet::unsandboxed(), parse_sandboxing_directive)` | absent → the `None` class |
| `host.and_then(…sandbox_flags)` (B7: `elidex-js/src/engine.rs:666`, `vm/host/window_dialogs.rs:214`) | `host.map_or(SandboxingFlagSet::unsandboxed(), …)` | D-N5; C10g |

### §4.3 ECS-native and Layering checks

No per-entity state is added, moved or read. `HostData.sandbox_flags` remains a per-VM side-store holding a per-Document
fact — the CLAUDE.md side-store→component rule's case — and its component form is F9 at N5 (decided, J1); N2a changes
only its value type (J14). OO → ECS: none. Layering: `vm/host/window_dialogs.rs` and `vm/host/event_handler_attrs.rs`
stay marshalling (read the store, call `elidex_plugin::sandbox`); the shell (`load.rs`, `link_nav.rs`) calls the same
predicates; no algorithm enters `vm/host/`. The one open-coded test (B6) moves into the predicate home.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `SandboxingFlagSet` (NEW) | HTML §7.1.5 *sandboxing flag set* (`#sandboxing-flag-set`); each constant its flag's anchor (e.g. `#sandboxed-navigation-browsing-context-flag`); the unrepresented seven → `#11-sandbox-flag-set-sources` |
| `union_restrictions` (NEW) | node memo J7 (elidex's own; no spec algorithm in N2a takes a union — N7 / N8 are the callers) |
| `unsandboxed` (NEW) | HTML §7.1.5 the empty sandboxing flag set ("When the Document is created, its active sandboxing flag set must be empty"); J14's interim value |
| `all_represented` (NEW) | HTML §7.1.5 *parse a sandboxing directive* step 3 over an empty token list; F10r's fail-closed value is elidex's own |
| `parse_sandboxing_directive` (NEW) | HTML §7.1.5 *parse a sandboxing directive* steps 1–3; §4.8.5 `#attr-iframe-sandbox` (ASCII case-insensitive tokens) |
| `scripts_allowed`, `scripting_enabled` | §7.1.5 sandboxed scripts flag; §8.1.3.4 settings clause |
| `forms_allowed` | §7.1.5 sandboxed forms flag; §4.10.22.3 step 4 |
| `popups_allowed` | §7.1.5 sandboxed auxiliary navigation flag; §7.3.1.7 *the rules for choosing a navigable* step 8 |
| `modals_allowed` | §7.1.5 sandboxed modals flag; §8.9.1 *cannot show simple dialogs* step 1 |
| `top_navigation_allowed` | §7.4.2.4 *allowed by sandboxing to navigate* steps 3.2, 3.3 |
| `same_origin_allowed` (NEW) | §7.1.5 sandboxed origin flag; §7.3.2.1 *determine the origin* step 1 |
| `parse_sandbox` | §4.8.5 post-connection steps step 2; attribute set / changed / removed |

## §5. Populations and named risks

### §5.1 Populations (full enumeration)

**(p) — the migration population: the compiler's error list at the type swap** (the gate; D-N1 makes it complete). The
*swap* is the `sandbox.rs` / `lib.rs` change alone: `IframeSandboxFlags` and `parse_sandbox_attribute` deleted,
`SandboxingFlagSet` / `parse_sandboxing_directive` / the predicates over the new type in place, and no other file
touched. Then, crate by crate in dependency order (`elidex-plugin`, `elidex-script-session`, `elidex-js`,
`elidex-shell`), run `cargo check -p <crate> --all-targets --all-features --message-format short` and record every
error's `file:line` before fixing any site. rustc suppresses errors behind a failed import (a `use … as F` that no
longer resolves silences every `F::…` use beneath it) and cargo does not check a crate whose dependency fails, so the
collection is **iterated**: fix exactly the recorded sites (by D-N6), re-run, record the new errors, until the workspace
checks clean. **(p) := the union of the recorded sites over all passes.** It is not counted here: a count derived
before the swap would be the vocabulary measurement R1 rejects. The implementation PR description records, per pass,
the `cargo check` command and its error sites, and per site the HEAD value class (D-N6) → the new spelling, plus every
"stop and record" case with its resolution; no gate counts it.

The vocabulary populations below are kept as **premise cross-checks** of (p), not as the gate: every line of
(e) ∪ (e+) ∪ (e!) outside `sandbox.rs` that is code (not a comment) should be in (p); a grep line missing from (p) is a site
the compiler did not see (e.g. a `cfg` that `--all-features` on the check host does not enable) and is recorded and
mapped by hand.

**(e) — type-name sites** (input §B, re-measured): `git grep -c IframeSandboxFlags -- crates` → **125 occurrences, 16
files** (verified 2026-10-24 via that command): `elidex-plugin/src/sandbox.rs` 65, `elidex-plugin/src/lib.rs` 1;
`elidex-js/src/engine.rs` 2, `tests_call_listener.rs` 1, `tests_engine_s1a.rs` 2, `tests_engine_s1b.rs` 5,
`vm/host_data/mod.rs` 5, `vm/tests/tests_event_handler_attrs.rs` 1, `vm/tests/tests_window_open_modals.rs` 16;
`elidex-script-session/src/engine.rs` 3, `navigation.rs` 3, `scripting.rs` 3; `elidex-shell/src/content/iframe/load.rs`
8, `content_iframe_security_tests.rs` 6, `content_window_open_tests.rs` 2, `pipeline.rs` 2.

**(e+) — polarity-encoding sites that do not name the type**: `git grep -n -P
'ALLOW_[A-Z_]+|Option<F>|UNSANDBOXED|MAX_RESTRICTION|vm_with_flags\(None|sandbox_flags\(\),? *None|sandbox_flags:
None|and_then\(.*sandbox_flags|let Some\(flags\) = sandbox_flags|parse_sandbox_attribute' -- crates | grep -v
IframeSandboxFlags | grep -v '^crates/core/elidex-plugin/src/sandbox.rs'` → **45 lines, 9 files** (verified 2026-10-24
via that command): `elidex-js/src/engine.rs:666`; `tests_engine_s1b.rs:166`, `:196`; `vm/host_data/mod.rs:863`;
`vm/host/window_dialogs.rs:214`; `vm/tests/tests_window_open_modals.rs:121`, `:173`, `:284`, `:296`, `:314`, `:333`,
`:372`, `:396`, `:426`, `:449`, `:474`; `elidex-script-session/src/navigation.rs` 20 lines in its test module
(`:304`–`:439`); `content_iframe_security_tests.rs:232`, `:538`, `:580`, `:618`, `:648`;
`content_window_open_tests.rs:390`, `:404`; `content/iframe/load.rs:405`, `:545`. Union with (e): **17 files**
(`git grep -l` of both commands, `sort -u | wc -l`).

**(e!) — silent-flip subclass** (B4): `git grep -n -P '(IframeSandboxFlags|\bF)::(empty|all)\(\)' -- crates` → 25 in
10 files; **8 in `sandbox.rs`** (`:59`, `:81`, `:92`, `:297`, `:328`, `:329`, `:388`, `:390`); outside it **17 in 9
files, all tests** (verified 2026-10-24 via that command): `tests_call_listener.rs:212`; `tests_engine_s1a.rs:63`;
`tests_engine_s1b.rs:171`, `:172`; `vm/tests/tests_event_handler_attrs.rs:551`; `vm/tests/tests_window_open_modals.rs:136`,
`:153`, `:194`, `:212`, `:247`, `:258`, `:381`; `elidex-script-session/src/navigation.rs:305`; `scripting.rs:136`;
`content_iframe_security_tests.rs:284`, `:468`; `content_window_open_tests.rs:410`; plus `navigation.rs`'s
`MAX_RESTRICTION` uses (in (e+)). Each is `Some(empty)` or `all()` at HEAD and maps by D-N6's value class; under D-N1
none can compile unchanged, which is the point.

**Comment and docstring sites** (B9): the 15 lines G8a/G8b find at the base plus G1's comment hits; each is rewritten
with the new contract.

**Touched files** (21; docstring-only `elidex-js/src/vm/host/storage.rs:79-80` excluded — it pairs `set_sandbox_flags`
with `set_origin` and encodes no polarity): the 17 above, plus four whose sites only carry the value (B14) and are
expected to compile unchanged — `elidex-js/src/vm/host/event_handler_attrs.rs` (`:602`, `:611`, `:618`),
`elidex-shell/src/content/iframe/thread.rs` (`:227`), `elidex-shell/src/content/link_nav.rs` (`:57`),
`elidex-js/src/tests_engine_s1d.rs` (`:97`) — listed so the reviewer checks they compile unchanged and that the value
reaching them is the producer's (§4.2).

### §5.2 Named risks

- **NR1 silent flip** — a rename-first migration turns every max-restriction fixture into an unsandboxed one, and the
  refusal tests go red or, where a test asserts "allowed", stay green for the wrong reason. The motivating example is
  outside every vocabulary population: `scripting.rs`'s eleven positional `None` arguments (B13, `:160` … `:288`), where
  mapping them to `all_represented()` would keep the suppression asserts at `:181`, `:192`, `:225`, `:250` green
  through the settings clause instead of the node-document clause they test. Mitigation: D-N1 (no spelling of HEAD's values compiles, so no site is migrated by accident), (p) as the
  population, D-N6's value-class rule, the (p) record (§5.1), and C10b's equivalence oracle written over **token
  strings**, not over constants.
- **NR2 unbound defaults** — D-N5's two sites keep HEAD's permissive read; N4 must delete them with the route (J2). G7
  names them and C10g pins their value, so N4's plan inherits a named population.
- **NR3 `host_data/mod.rs` (2023 lines)** — touch-time rule applied: the touched concern (field `:187-200`, initializer
  `:863`, accessors `:1090-1141`) is re-typed in place, net LoC ≈ 0, and is deleted whole at N5; the file's reduction is
  the user's option-A placement in `#11-host-data-full-decomposition` (2026-07-11; slot re-eval 2026-10-31,
  `project_open-defer-slots.md`). The cited precedent (the parent memo `docs/plans/2026-09-inline-box-predicate.md:1483`)
  is phrased for a **deleting** touch ("is deleted whole … so no seam along the touch remains to split first"); the node
  memo's decided-scope §4 flag-set line (`:456-458`, "N2a's light re-type of `vm/host_data/mod.rs` (net LoC ≈ 0, deleted
  at N5, option A)") extends it to N2a's re-type. No prereq split. No other touched file exceeds 1000 lines (B11).
- **NR4 `pipeline.rs` (973)** — two field types and two doc lines; net 0; G9.
- **NR5 newly recognised token** — `allow-popups-to-escape-sandbox` clears the propagate flag, which nothing reads until
  N8: no verdict changes (C10b covers HEAD's seven tokens; C10 the eighth).
- **NR6 docstrings** — the comment and docstring sites (§5.1) are rewritten with the new contract; `sandbox.rs`'s boa citations are deleted (the
  boa engine no longer exists in `crates/script`).
- **NR7 OOP `Navigate` rebuild (latent)** — `content/iframe/thread.rs:227` fills `PreEvalFrameInputs.sandbox_flags` from
  `pipeline.runtime.sandbox_flags()` — the retiring document's stored set, through the getter whose impl carries D-N5's
  unbound arm — and `pipeline.rs:279` writes it into the new store. The parent's G12 (`inline-box-predicate.md:1478`)
  says this path needs the **frame's** flags. Latent: the path has no production sender (`thread.rs:233-235`). N2a only
  carries the type through (B14); the record goes to N4 (the unbound gate) and N5 (F9 and the cross-process set) — §7, §8.

## §6. Test and coverage plan

Declared surface: no WPT subset (no runtime change; the node's `html/browsers/sandboxing/` subset is exercised by later
PRs). Oracles: `elidex-plugin` unit tables; the existing VM and shell integration suites, migrated per D-N6 with
assertions unchanged.

| # | Cell | Where |
|---|---|---|
| C10 | parse truth table (node cell): for every subset of the eight recognised tokens (256), the parsed set equals the spec list's arms evaluated on that subset (a table of `(flag, clearing tokens)` transcribed from step 3); plus literal rows — **`SandboxingFlagSet::all_represented() == parse_sandboxing_directive("")`** (K4) and whitespace-only → `all_represented()`; navigation flag always set; `allow-top-navigation` alone clears both top-level flags; case folding; non-ASCII look-alike does not clear; unknown tokens and the five unrepresented-only tokens clear nothing; TAB/LF/FF/CR separators | `sandbox.rs` tests |
| C10b (NEW) | HEAD equivalence (K1): for every subset `T` of HEAD's seven tokens and `activation ∈ {false, true}`, each predicate on `parse(T)` equals HEAD's token-presence verdict (`allow-x ∈ T`; top navigation: `allow-top-navigation ∈ T ∨ (activation ∧ allow-top-navigation-by-user-activation ∈ T)`) | `sandbox.rs` tests |
| C10c (NEW) | union (K2, J7): for every pair of parsed subsets, `p(a.union_restrictions(b)) == p(a) && p(b)` for each single-flag predicate, and `top_navigation_allowed(a.union_restrictions(b), act)` implies it on `a` and on `b`; `unsandboxed()` identity, `all_represented()` absorbing | `sandbox.rs` tests |
| C10d | withdrawn in rev 2 — folded into C10's K4 row (number not reused) | — |
| C10e (NEW) | origin: `apply_sandbox_origin` returns opaque iff `allow-same-origin` is absent from a present attribute or the frame is credentialless; absent attribute keeps the input origin | `content_iframe_security_tests.rs` (existing order-proof arms re-asserted) |
| C10f (NEW) | `unsandboxed()` allows every predicate: `scripts_allowed`, `forms_allowed`, `popups_allowed`, `modals_allowed`, `scripting_enabled`, `same_origin_allowed` true, and `top_navigation_allowed(unsandboxed(), act)` true for both `act` (HEAD's `None` row) | `sandbox.rs` tests |
| C10g (NEW) | an unbound read yields `unsandboxed()` as HEAD's `None`: `window.open('/x', '_blank')` on a `Vm` with no `HostData` installed (`Vm::new()` without `install_host_data`; `vm/init.rs:250` `host_data: None`) takes the unsandboxed disposition; the cell is written and run at the base first, so its expected result is HEAD's. The engine getter's unbound arm (`elidex-js/src/engine.rs:661-667`) is unreachable through `ElidexJsEngine`'s constructors (both install `HostData`, `engine.rs:44`, `:100`) and is covered by D-N6's `None` class and G7 alone. If the `window.open` native cannot run without `HostData` (Q-N1), the implementer stops and reports rather than substituting another cell | `vm/tests/tests_window_open_modals.rs` |
| C10h (NEW) | no operator surface (D-N1): a grep gate — **`trybuild` is not a workspace dev-dependency** (`/usr/bin/grep -rn trybuild Cargo.toml crates/*/*/Cargo.toml` → no match, verified 2026-10-24), so no compile-fail harness is added: `git grep -n -E 'impl[^{]*(BitOr\|BitAnd\|BitXor\|Not\|Sub\|Add)(Assign)?[^{]*for[^{]*SandboxingFlagSet' -- crates/core/elidex-plugin` → 0. It is blind to a macro-generated impl, which the module cannot contain: the only macro is `bitflags!` on the private `Flags`, and the newtype derives only `Clone, Copy, Debug, PartialEq, Eq, Hash` | CI step / reviewer command (no test file) |
| — | migrated suites green with assertions unchanged (the `sandbox.rs` predicate tests migrate, its token-parser tests are replaced by C10 / C10b: D-N6): `tests_window_open_modals.rs`, `tests_engine_s1a.rs`, `tests_engine_s1b.rs`, `tests_call_listener.rs`, `tests_event_handler_attrs.rs`, `scripting.rs` and `navigation.rs` in-file tests, `content_iframe_security_tests.rs`, `content_window_open_tests.rs` | per crate |

C10b is the mutation guard for the predicates' flip: inverting one predicate's polarity or dropping the two-token arm
fails it. A site mis-mapped between `unsandboxed()` and `all_represented()` is not C10b's subject; it is caught by the
(p) record's per-site value class (§5.1) and, where the site's test asserts a verdict, by that unchanged assertion (G10). C10e and C10g
are **added** cells: their new lines in `content_iframe_security_tests.rs` and `tests_window_open_modals.rs` are
additions to the migrated files, not G10 violations.
Commands: `cargo test -p elidex-plugin --all-features`, `cargo test -p elidex-script-session --all-features`, `cargo test
-p elidex-js --all-features`, `cargo test -p elidex-shell --all-features`; `mise run ci` before push.

**Questions for the scout**: **Q-N1** — does the `window.open` native (`vm/host/window_dialogs.rs`) run, and enqueue an
observable navigation intent, on a `Vm` with no `HostData` installed (C10g's precondition)?

## §7. Out of scope and slots

| Gap | Why not here | Trigger / owner | Re-eval |
|---|---|---|---|
| the seven unrepresented §7.1.5 flags | no reader at HEAD (B10); D1 | `#11-sandbox-flag-set-sources` (registered at the node's approval) | 2026-11-01 (the parent's) |
| unbound gates fail open | J2 is N4's | N4, edge n3 (NR2) | N4's plan |
| the OOP `Navigate` rebuild reads the retiring document's set, not the frame's (`thread.rs:227`; parent G12, `inline-box-predicate.md:1478`) | latent, no production sender (`thread.rs:233-235`); N2a carries the type only (NR7) | input to N4 (the unbound getter it reads through) and N5 (F9 and the cross-process set); edge n3 | N4's / N5's plan |
| §7.4.2.4 steps 1–6 by relation; the navigation flag's reader | D-a | N2b, edge n6a | — |
| the propagate flag's reader; the popup set | F18 | N8, n6a | — |
| restriction-set union as a creation input | J4 | N7, n6a | — |
| CSP `sandbox` through the parser (incl. its token comparison, D-N2) | J8 | N6, n6a | — |
| the store flip (F9, F10r, `HostData` copy deleted) | J1 | N5 | — |
| `<iframe>.sandbox` as a `DOMTokenList` (`[SameObject, PutForwards=value, Reflect] readonly attribute DOMTokenList sandbox`, `webref idl html HTMLIFrameElement`; HEAD reflects a string, `vm/host/html_iframe_proto.rs:263`) | IDL surface, not the flag set | existing `#11-classlist-dataset-extras` (roadmap's `<iframe>.sandbox` sub-task). Risk: both facets the slot names in `project_open-defer-slots.md` are closed (relList.supports #392, dataset-extras #461) and its roadmap re-eval date (`m4-12-platform-gap-roadmap.md:1202`, 2026-06-17) has passed, so this remaining facet's home depends on that slot's hygiene — reported to the PM, not re-homed here | that slot's |

Own deferrals: **0** (node memo §5.2). No slot is registered or appended by N2a.

## §8. Edges and hand-offs

- **n3** (N2a → N4): F10's flags argument is `SandboxingFlagSet`; N4 receives D-N5's two unbound-default sites (G7's
  list) as population to delete with the route, and NR7's `thread.rs:227` record (it reads through the getter's unbound
  arm). N5 receives NR7 too (F9, the frame's set across the process boundary). **N5 also receives R4's consequence**:
  its J14 DoD grep (node memo `:620`, "no interim `empty()` survives") reads `unsandboxed()` at the deleted stores — no
  `empty()` exists on the new type to grep for (D-N1).
- **n6a** (N2a → N2b, N6, N7, N8): N2b reads `SANDBOXED_NAVIGATION` and absorbs `top_navigation_allowed`; N6 calls
  `parse_sandboxing_directive` for the CSP directive; N7 unions with `union_restrictions` (K2; there is no operator, D-N1); N8 reads the propagate flag. Each of
  those PRs adds its reader through a predicate in `elidex-plugin::sandbox` (the constants are module-private, D-N1) or
  argues the exception in its own plan (G4 is N2a's landing fact, not a node invariant).
- Records: none ship with N2a (node memo §8 records table has no N2a row; D1's amendments shipped with the approval PR).

## §9. Definition of done

**Prerequisites**: D7 met at the base (`d306af5a`, #541). The node memo's §9 live lanes (`domform-submittable-category`,
`domform-slice1`) are parked (MEMORY *Active state*: L3 DOM/form stopped since 07-27) and disjoint from N2a's files;
open PR #531 (html5ever) is needed only before N9; E1/E2 are needed only before N4 and N5.

Mechanical greps (each over `crates`, at the PR head):

- **G1** `git grep -n -E 'IframeSandboxFlags|parse_sandbox_attribute' -- crates` → 0.
- **G2** `git grep -n -E 'ALLOW_(SCRIPTS|SAME_ORIGIN|FORMS|POPUPS|TOP_NAVIGATION|MODALS)' -- crates` → 0.
- **G3** no `Option<SandboxingFlagSet>` in any parameter, field or return type, under any path spelling:
  `git grep -n -P 'Option<\s*(::)?((elidex_plugin|crate|super|self)::)*(sandbox::)?(SandboxingFlagSet|F)\s*>' -- crates` →
  0, and `git grep -n -P 'Option<\s*Self\s*>' -- crates/core/elidex-plugin/src/sandbox.rs` → 0 (`Self` is the flag type
  only inside that module's `impl SandboxingFlagSet`; elsewhere `Option<Self>` is another type). The `F` arm catches a
  surviving alias; at the base (with `IframeSandboxFlags` in place of the first arm) its `F` hits are only the alias sites
  `sandbox.rs:323`, `navigation.rs:304`, `:305` (re-verified 2026-10-24 with the widened path prefix).
- **G3b** `git grep -n -P 'SandboxingFlagSet\s+as\s|\btype\s+\w+\s*=\s*[^;]*SandboxingFlagSet' -- crates` → 0 (no
  `use … as` or `type` alias of the new type, so G3's regex cannot be bypassed by renaming it).
- **G4** `git grep -n -E 'SANDBOXED_|SANDBOX_PROPAGATES' -- crates ':!crates/core/elidex-plugin/src/sandbox.rs'` → 0
  (textual backstop; the constants are private, D-N1).
- **G5** — withdrawn in rev 2 (`all_represented()` is a fixture constructor under D-N6's mapping rule).
- **G6** `git grep -n 'SandboxingFlagSet::empty\|IframeSandboxFlags' -- crates` → 0.
- **G7** the value reads on a missing `HostData` at the PR head are exactly two, each yielding `unsandboxed()`:
  the `sandbox_flags` impl of the `ScriptEngine` trait (`elidex-js/src/engine.rs`, `:661-667` at the base) and the `window.open` read
  (`vm/host/window_dialogs.rs`, `:214` at the base). Reviewer check: the production callers of `HostData::sandbox_flags`
  (`git grep -n -P '\bsandbox_flags\(\)|HostData::sandbox_flags' -- crates/script/elidex-js/src
  ':!crates/script/elidex-js/src/tests_*' ':!crates/script/elidex-js/src/vm/tests'`; four lines at the base — the two
  above, `vm/host/event_handler_attrs.rs:602`, which reads a host already resolved by its `let Some(host) … else`
  (`:597-601`), and the doc comment `engine.rs:134`) are enumerated and each optional-host caller is one of the two;
  C10g pins the value.
- **G8a** over §5.1's 21 touched files, `git grep -n -i -P 'allow-token|positive allow|Some\(empty|<NONE>|,
  if sandboxed|, if it is\s*$' -- <files>` with `<NONE>` = `` `?None`? (for (a )?top-level|= no
  `?sandbox|if no `?sandbox) `` → 0 (13 lines at the base, verified 2026-10-24; the pattern is one string — the line breaks above are
  layout). The phrase "maximum restriction" is not a term: it is still correct under the new contract (`all_represented()`
  is the maximum restriction), and the rev 2 term made five such comment lines, including still-correct ones
  (`navigation.rs:376`), spurious hits. The gate targets the old contract's phrasing only (`Some(empty)` as the
  maximum, "`None` for top-level / unsandboxed", "positive allow", "allow-token"; boa / `iframe_bridge` in G8b).
- **G8b** `git grep -n -i 'boa\|iframe_bridge' -- crates/core/elidex-plugin/src/sandbox.rs` → 0 (2 at the base).
- **G9** `wc -l crates/shell/elidex-shell/src/pipeline.rs` < 1000.
- **G10** the diff of each migrated test file changes no `assert` line other than D-N6's set-valued ones, and adds
  none other than C10e's and C10g's (the ADDED cells, §6) (reviewer check: `git diff origin/main -- <file> | grep -E
  '^[-+].*assert'`). The `sandbox.rs` test module is outside this textual check (D-N6: inline value expressions; its
  predicate asserts keep predicate and polarity).

Behavioural: C10, C10b, C10c, C10e, C10f, C10g present and green, C10h's command returns 0; the four `cargo test -p` commands and `mise run ci`
pass. Preflight: `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-ibp-sandbox-flagset.md`
exits 0.

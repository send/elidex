# N2a `IBP-sandbox-flagset` — the sandboxing flag set in restriction form (per-PR plan-memo)

Nested PR **N2a** of node `IBP-sandbox` (`docs/plans/2026-09-ibp-sandbox.md`, approved by #532, `6270e219`; "the node
memo"). The node memo's approval scope (`:6-14`) binds: its **decided** lines (D1, J7, J14, §4's *Home/kind* line for the
flag set, §5.1's N2a row, §5.2's edges n3 and n6a, §1's "N2a narrow" verdict, §3's *Branch* column) are inherited as
decisions; its **input** lines (§4's mechanism prose, §5.3 population (e), §5.4 N2a's named risks, input file
`docs/plans/2026-09-ibp-sandbox-input.md` §B (e), §D, §E C10) are re-derived here.

**Rev 1** (2026-10-24) — first draft, before `/elidex-plan-review`.

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
| B2 | Population (e) = 125 occurrences in 16 files (input §B) | `git grep -c IframeSandboxFlags -- crates` | **holds**, verified 2026-10-24 (125 / 16, per-file table §5.1) — but the count is by the type's **name**; the property that matters is "every site whose text encodes the old polarity", which is wider (B3, B4) |
| B3 | (re-derived) value-flow sites that encode the old polarity without naming the type | §5.1's command (e+) | **45 lines, 9 files** (verified 2026-10-24), one of them production-only and outside (e)'s 16: `crates/script/elidex-js/src/vm/host/window_dialogs.rs:214` |
| B4 | (re-derived) **silent-flip** sites: `empty()`/`all()` of the type, which type-check under both polarities and mean the opposite after a rename | `git grep -n -P '(IframeSandboxFlags\|\bF)::(empty\|all)\(\)' -- crates` | verified 2026-10-24: 25 occurrences in 10 files; 6 in `sandbox.rs` itself, **19 in 9 files outside it, all in tests** (§5.1 (e!)); 0 in production outside `sandbox.rs` |
| B5 | The `top_navigation_allowed` implication (`allow-top-navigation` ⇒ by-user-activation) lives in the predicate | `sed -n 143,170p crates/core/elidex-plugin/src/sandbox.rs` | holds (`:166-169`); the spec puts it in the **parse** (*parse a sandboxing directive* step 3, the with-user-activation arm "unless … either … or …") |
| B6 | `apply_sandbox_origin` open-codes a flag test, against the module's "every capability check delegates here" contract (`sandbox.rs:9-14`) | `sed -n 541,553p crates/shell/elidex-shell/src/content/iframe/load.rs` | holds: `:545-546` `if let Some(flags) … !flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN)` |
| B7 | Readers that default on a **missing `HostData`** (unbound) through the flag *value* | `git grep -n -E 'and_then\(.*sandbox_flags' -- crates` | two: `crates/script/elidex-js/src/engine.rs:666`, `crates/script/elidex-js/src/vm/host/window_dialogs.rs:214`; the `is_none_or(\|hd\| hd.*_allowed())` sites (`engine.rs:138`, `:674`, `:682`; `window_dialogs.rs:52`; `event_handler_attrs.rs:553`) default on the *verdict* and do not name the value |
| B8 | No struct holding the flags derives `Default` (so the set needs no `Default`) | `grep -n 'derive' crates/shell/elidex-shell/src/pipeline.rs`; `sed -n 855,870p crates/script/elidex-js/src/vm/host_data/mod.rs` | holds: `PreEvalFrameState`/`PreEvalFrameInputs` derive nothing; `HostData::new` sets `sandbox_flags: None` explicitly (`:863`) |
| B9 | The modules' docstrings name the old contract and deleted code | `git grep -n -i -E 'allow-token\|positive allow\|Some\(empty\)\|maximum restriction' -- crates`; `grep -n 'iframe_bridge\|boa' crates/core/elidex-plugin/src/sandbox.rs` | `sandbox.rs:2-3`, `:23`, `:91-96`, `:156`; `tests_window_open_modals.rs:132`; `pipeline.rs:55`, `:83`; `elidex-script-session/src/engine.rs:514-519`; `host_data/mod.rs:187-199`, `:1089-1116`; `sandbox.rs:11-13` cites the deleted boa `iframe_bridge.rs` / `globals/window/mod.rs` |
| B10 | No HEAD feature reads one of the seven flags D1 leaves unrepresented | `git grep -n -i -E 'autofocus\|"domain"\|document_domain\|autoplay\|requestPointerLock\|pointer_lock' -- 'crates/**/*.rs'` less tests | holds: the only hits are the focus-delegate walk (`elidex-dom-api/src/focus/delegate.rs`), which is not *flush autofocus candidates* (slot `#11-flush-autofocus-candidates-page-load`, `delegate.rs:163`); no `document.domain` setter, Pointer Lock, downloads, orientation lock, presentation |
| B11 | Files N2a touches and the 1000-line rule | `wc -l` on §5.1's touched files (verified 2026-10-24) | one file >1000: `crates/script/elidex-js/src/vm/host_data/mod.rs` (2023); watch: `crates/shell/elidex-shell/src/pipeline.rs` (973) |
| B12 | The iframe's flag set is read from the attribute at load | `sed -n 399,407p crates/shell/elidex-shell/src/content/iframe/load.rs` | `parse_sandbox` (`:401`): absent → `None`, present → parse; spec (§4.8.5): post-connection steps step 2 parses, attribute set/changed re-parses, removal empties, and "these flags only take effect when the content navigable … is navigated" — a read at navigation is observationally the maintained set |

### §0.3 Premise errors (reported, not edited — the A114 idiom)

None in a **decided** line. D1, J7, J14, §4's flag-set *Home/kind*, §5.1's N2a row, n3, n6a and §3's N2a *Branch* cell
("nine represented flags … implied grants at parse; unrepresented → `#11-sandbox-flag-set-sources`") are each consistent
with HEAD and the spec as re-measured (§0.2, §3).

Findings against **input** lines (re-derived here; they change no decision, so they are recorded as this PR's named
risks, §5.4 of the node memo's idiom):

- **R1** input §B (e)'s command counts by the type's name; its rewrite target "`None` → `empty()`" covers one of the two
  polarity mappings. The other — HEAD's `Some(IframeSandboxFlags::empty())`, *maximum restriction* — becomes the new
  type's `empty()`, *unsandboxed*, under a rename, and still type-checks (B4). Re-derived as populations (e), (e+), (e!)
  with the mapping table §4.2.
- **R2** input §D assigns the top-navigation predicate's restatement to N2b; N2a cannot compile without re-typing it, and
  the spec moves its implication into the parse (B5). Re-derived: N2a restates its body over the restriction set with
  its signature kept (D-N3); N2b still owns its absorption into §7.4.2.4 (D-a).
- **R3** J14 names the interim **stores** (`HostData`, `PreEvalFrameState`) and their "not a sandboxed iframe" value; it
  does not name the two **unbound-default** reads (B7), whose HEAD value is also `None`. They are not a store; §5.1 of
  the node memo already keeps them HEAD's ("before N4 an unbound gate reads HostData as at HEAD"). Re-derived: D-N5.

### §0.4 Decisions (this PR)

Inherited (node memo, decided): **D1** — `SandboxingFlagSet` replaces `IframeSandboxFlags`, restriction form, the
sandboxed navigation browsing context flag represented; **J7** — union in restriction form from N2a; **J14** — N2a to
N5 the interim stores hold `empty()` for HEAD's "not a sandboxed iframe"; Home `elidex-plugin::sandbox`.

- **D-N1 — the represented set.** `SandboxingFlagSet` (NEW) is a `bitflags` set over `u32` (the spec's list has
  sixteen flags and grows; `u32` leaves room for `#11-sandbox-flag-set-sources` without a re-type) with exactly the nine
  flags §3's decided *Branch* cell names, each constant named after its spec flag: `SANDBOXED_NAVIGATION`,
  `SANDBOXED_AUXILIARY_NAVIGATION`, `SANDBOXED_TOP_LEVEL_NAVIGATION_WITHOUT_USER_ACTIVATION`,
  `SANDBOXED_TOP_LEVEL_NAVIGATION_WITH_USER_ACTIVATION`, `SANDBOXED_ORIGIN`, `SANDBOXED_FORMS`, `SANDBOXED_SCRIPTS`,
  `SANDBOX_PROPAGATES_TO_AUXILIARY_BROWSING_CONTEXTS`, `SANDBOXED_MODALS` (all NEW). `empty()` is the spec's empty
  flag set (unsandboxed). `SandboxingFlagSet::all_represented()` (NEW, `const fn`, `= Self::all()`) names "every flag
  this representation holds" — the value F10r returns from N4/N5 and the parse of an empty directive; the name carries
  that the seven unrepresented flags are absent from it. **No `Default`**: a defaulted set would be `empty()`, an implicit
  *unsandboxed*; every construction names its value (fail-safe direction; B8 shows nothing needs it). The seven
  unrepresented flags (pointer lock, automatic features, `document.domain`, orientation lock, presentation, downloads,
  custom protocols navigation) are `#11-sandbox-flag-set-sources`' (D1 restatement, node memo §8); the type's docstring
  lists them and the slot (B10: no HEAD reader).
- **D-N2 — one parser, the spec's algorithm.** `parse_sandboxing_directive(input: &str) -> SandboxingFlagSet` (NEW)
  replaces `parse_sandbox_attribute` and implements HTML §7.1.5 *parse a sandboxing directive* steps 1–3: split on ASCII
  whitespace; start empty; **add** each represented flag unless its clearing token is present. So the implied grant
  (the with-user-activation flag is cleared by `allow-top-navigation-by-user-activation` **or** `allow-top-navigation`)
  is in the parse, the navigation flag is added unconditionally, and `allow-popups-to-escape-sandbox` clears the
  propagate flag (a token HEAD ignores; its only reader is N8's, so recognising it changes no HEAD verdict). Tokens match
  ASCII case-insensitively (§4.8.5: the value's tokens "are ASCII case-insensitive"), as at HEAD. The parse is additive
  from empty, so an unrecognised or mis-cased token **adds a restriction it fails to clear** — the unknown falls on the
  restricted side by construction. The one parser serves the attribute now and the CSP `sandbox` directive at N6 (n6a);
  the name is the spec's, not the attribute's.
- **D-N3 — predicates over the set, one read API.** Every predicate takes `SandboxingFlagSet` by value (no `Option`):
  `scripts_allowed`, `forms_allowed`, `popups_allowed`, `modals_allowed`, `scripting_enabled` (restated, input §D),
  `top_navigation_allowed(flags, has_transient_activation)` (restated with its signature: activation → the
  with-user-activation flag, else the without flag — §7.4.2.4 steps 3.2/3.3 literally; N2b absorbs it), and
  **`same_origin_allowed`** (NEW) — `!flags.contains(SANDBOXED_ORIGIN)`, the *determine the origin* step 1 test that
  `apply_sandbox_origin` open-codes today (B6). Outside `elidex-plugin/src/sandbox.rs` no code names a flag constant or
  calls `contains` on the set; every reader goes through a predicate, every constructor through
  `parse_sandboxing_directive`, `empty()` or `all_represented()` (DoD G4, G5). This keeps S5-4 §4.1's ratified
  predicate home and its module contract (`sandbox.rs:9-14`), now with no exception.
- **D-N4 — the interim stores (J14), retyped in place.** `HostData.sandbox_flags` (`host_data/mod.rs:200`, initializer
  `:863`), `PreEvalFrameState.sandbox_flags` (`pipeline.rs:56`), `PreEvalFrameInputs.sandbox_flags` (`pipeline.rs:84`),
  the `ScriptEngine` pair `set_sandbox_flags` / `sandbox_flags` (`elidex-script-session/src/engine.rs:515`, `:519`; impl
  `elidex-js/src/engine.rs:655`, `:661`) and the `iframe/load.rs` parameters (`:89`, `:383`, `:421`, `:452`, `:542`)
  take `SandboxingFlagSet`; `None` → `empty()`. `parse_sandbox` (`load.rs:401`) returns `empty()` for an absent
  attribute (§4.8.5: removal empties the iframe sandboxing flag set). Names (`sandbox_flags`, `parse_sandbox`) are kept:
  N5 deletes the stores and N7 owns the iframe set's computation, so renaming here is churn they undo.
- **D-N5 — the unbound defaults stay HEAD's.** The two value reads on a missing `HostData` (B7) map it to `empty()` —
  HEAD's `None`, permissive — through `map_or(SandboxingFlagSet::empty(), …)`; each carries a one-line comment naming
  J2 and N4. N2a makes no gate fail closed: that is J2's, from N4 (node memo §2), and D1 is "re-keyed in place" (node
  memo §5.1 *Closed*). The two sites are N4's population, handed over by edge n3 (§8).
- **D-N6 — fixtures construct sets the way production does.** Every test site builds its set with
  `parse_sandboxing_directive("<tokens>")` (`""` for HEAD's `Some(empty())`), or `SandboxingFlagSet::empty()` for HEAD's
  `None`; no fixture names a flag constant. A hand-built restriction set that omits a flag *grants* it, so the parse is
  the only fixture constructor that cannot under-restrict (user CLAUDE.md "fixture は実物の形を反映させる"). The assertion
  lines of the migrated tests do not change except the three `sandbox_flags()` equality asserts whose expected value is a
  set (`tests_engine_s1b.rs:166`, `:172`, `:196`; `content_iframe_security_tests.rs:119`, `:232`, `:247`, `:263`, `:283`
  compare against a set too, and change only their right-hand side).

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| Representation | restriction form; nine flags; `empty()` = unsandboxed, `all_represented()` = every represented restriction; no `Default` |
| Parse | *parse a sandboxing directive* steps 1–3, additive from empty; implied grant at parse |
| Readers | every verdict through a predicate in `elidex-plugin::sandbox`; every HEAD verdict unchanged |
| Interim stores (J14) | `None` → `empty()`; unbound defaults HEAD's (D-N5) |

**Edge density: not edge-dense — single PR** (*verdict*). The node memo decided "N2a narrow — one type re-keyed in
place, with the sandboxed navigation flag added" (§1) and split it from N2b so the polarity flip is reviewed alone. Re-
measured: the PR has one canonical algorithm (*parse a sandboxing directive*), no new reader, no new store, and no
runtime change; of the invariant pairs in §2 one intersects (behaviour equivalence × interim mapping) — below CLAUDE.md's
≥3-intersecting-axes trigger. It is an approved narrowly-scoped slice under an approved node, the rule's base case (c);
the plan-review is still mandatory (rule (b): the node is edge-dense). Breadth: K = 1 spec, M = 9 rows (§3), "ok →
single PR scope" (`webref coverage-map`).

## §2. Coupled invariants

- **K1 Behaviour equivalence** — for every token string `T` over HEAD's seven tokens and for the absent attribute, each
  predicate's verdict after N2a equals HEAD's (HEAD's verdict = presence of the clearing token; absent = permissive).
  Includes the unbound defaults (D-N5) and `apply_sandbox_origin`.
- **K2 Restriction-form union (J7)** — `a | b` restricts at least as much as `a` and as `b`: for every single-flag
  predicate `p`, `p(a | b) == p(a) && p(b)`; `empty()` is the identity and `all_represented()` absorbs. (No N2a reader;
  N7's embedder union and N8's popup copy rely on it, n6a.)
- **K3 Interim mapping (J14)** — `None` → `empty()` at every store and parameter; HEAD's `Some(A)` → `parse(tokens(A))`
  (§4.2); no `Option` of the set survives (G3).
- **K4 Parse totality** — `parse_sandboxing_directive("") == all_represented()`, so every represented constant has a
  parse arm (a constant added without one fails the cell).

Pairs: **K1 × K3** — the mapping is what makes K1 hold at the stores: `None` → `empty()` is permissive for every
predicate (HEAD's `is_none_or`), and `Some(A)` → `parse(tokens(A))` is the same verdict per predicate because HEAD's bit
`A ∋ ALLOW_X` and the new arm "add `SANDBOXED_X` unless `allow-x`" read the same token (the top-navigation pair: HEAD's
disjunction `TOP_NAV ∨ (act ∧ BY_UA)` equals "activation ? ¬with-UA : ¬without-UA" over the parsed set, because the
with-UA arm is cleared by either token). **K1 × K2** — none: no N2a reader takes a union. **K1 × K4** — K4's empty
directive is HEAD's `Some(empty())`: every predicate false, so K1's max-restriction row is K4's value. **K2 × K3**,
**K2 × K4**, **K3 × K4** — none beyond the shared type.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.1.5 Sandboxing | *sandboxing flag set* (the flag list) | nine represented (navigation; auxiliary navigation; top-level navigation without / with user activation; origin; forms; scripts; propagates to auxiliary browsing contexts; modals); seven unrepresented → `#11-sandbox-flag-set-sources` (D1) | `crates/core/elidex-plugin/src/sandbox.rs` `SandboxingFlagSet` (NEW) | ✗ | no |
| WHATWG HTML §7.1.5 Sandboxing | *parse a sandboxing directive* steps 1–3 | step 1 ASCII-whitespace split; step 2 empty; step 3 the nine represented arms incl. the unconditional navigation flag and the two-token with-user-activation arm (implied grant at parse); unrecognised tokens add nothing; the seven unrepresented arms → slot | `parse_sandboxing_directive` (NEW) | ✗ | yes |
| WHATWG HTML §4.8.5 The iframe element | `sandbox` attribute (ASCII case-insensitive tokens); post-connection steps step 2; set / changed / removed | absent → `empty()`; present → parse; read when the iframe loads (flags take effect only at navigation, §4.8.5) | `crates/shell/elidex-shell/src/content/iframe/load.rs:401` `parse_sandbox` | ✓ | yes |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | settings clause ("does not have its sandboxed scripts browsing context flag set") | `scripting_enabled` over the set; the composition is N4's | `sandbox.rs` `scripting_enabled`; callers `crates/script/elidex-script-session/src/scripting.rs:89`, `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:611` | ✓ | no |
| WHATWG HTML §8.9.1 Simple dialogs | *cannot show simple dialogs* step 1 | sandboxed modals flag | `modals_allowed`; `crates/script/elidex-js/src/vm/host_data/mod.rs:1139` | ✓ | no |
| WHATWG HTML §7.3.1.7 Navigable target names | *the rules for choosing a navigable* step 8 (the sandboxed-auxiliary-navigation option) | aux-nav flag → popup blocked; substeps 8.2 (navigation flag) and 9 (propagate flag) → N8 / `#11-browsing-context-model-window-open-postmessage` | `popups_allowed`; `crates/script/elidex-script-session/src/navigation.rs:286`, `:293`; `crates/shell/elidex-shell/src/content/link_nav.rs:46` | ✗ | yes |
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
the flag stays set; duplicates idempotent; unknown tokens and the seven tokens of unrepresented flags clear nothing they
should not (`allow-downloads` etc. clear an unrepresented flag, i.e. nothing); empty / whitespace-only value →
`all_represented()`. Every mismatch leaves a restriction set (D-N2). No new input surface; CSP `sandbox` reaches the
parser at N6.

## §4. Mechanism per fact

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| sandboxing flag set (§7.1.5) | `SandboxingFlagSet` (NEW), `bitflags` over `u32`, `crates/core/elidex-plugin/src/sandbox.rs`; re-exported at `elidex_plugin::SandboxingFlagSet` (`lib.rs:66` line replaced) | decided (D1, node §4); `u32`, constants, no `Default`: D-N1 |
| every represented restriction | `SandboxingFlagSet::all_represented()` (NEW) | D-N1 |
| parse a sandboxing directive | `parse_sandboxing_directive` (NEW), same module; re-exported beside the type | D-N2 |
| capability verdicts | `sandbox::{scripts_allowed, forms_allowed, popups_allowed, modals_allowed, top_navigation_allowed, scripting_enabled, same_origin_allowed}` — `same_origin_allowed` NEW | D-N3 |
| iframe sandboxing flag set | `parse_sandbox` (`load.rs:401`), shell marshalling over `IframeData` | D-N4 (N7 owns its later form) |
| interim stores | `HostData.sandbox_flags`, `PreEvalFrameState.sandbox_flags`, `PreEvalFrameInputs.sandbox_flags`, `ScriptEngine::{set_sandbox_flags, sandbox_flags}` | D-N4 (J14; deleted by N5) |

### §4.2 Rewrite mapping (HEAD → N2a)

| HEAD expression | N2a expression | Why equal (K1) |
|---|---|---|
| `None` (unsandboxed) | `SandboxingFlagSet::empty()` | every predicate permissive on both |
| `Some(IframeSandboxFlags::empty())` | `parse_sandboxing_directive("")` (fixtures) / `all_represented()` (`sandbox.rs` tests only) | every predicate false on both |
| `Some(ALLOW_X \| ALLOW_Y …)` | `parse_sandboxing_directive("allow-x allow-y …")` | per-predicate token presence (§2 K1 × K3) |
| `Some(IframeSandboxFlags::all())` (`sandbox.rs:390`) | `parse_sandboxing_directive` of all eight recognised tokens | every predicate true on both |
| `flags.is_none_or(\|f\| f.contains(ALLOW_X))` | `!flags.contains(SANDBOXED_X)` (inside `sandbox.rs` only) | — |
| `if let Some(f) = flags { !f.contains(ALLOW_SAME_ORIGIN) }` (`load.rs:545-546`) | `!sandbox::same_origin_allowed(flags)` | absent → `empty()` → allowed, as HEAD's `None` arm |
| `host.and_then(sandbox_flags)` (B7) | `host.map_or(SandboxingFlagSet::empty(), …)` | D-N5 |

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
`content_window_open_tests.rs:390`, `:404`; `content/iframe/load.rs:405`, `:545`. Union with (e): **17 files**.

**(e!) — silent-flip subclass** (B4): `git grep -n -P '(IframeSandboxFlags|\bF)::(empty|all)\(\)' -- crates` → 25 in
10 files; outside `sandbox.rs` **19 in 9 files, all tests** (verified 2026-10-24 via that command):
`tests_call_listener.rs:212`; `tests_engine_s1a.rs:63`; `tests_engine_s1b.rs:171`, `:172`;
`vm/tests/tests_event_handler_attrs.rs:551`; `vm/tests/tests_window_open_modals.rs:136`, `:153`, `:194`, `:212`, `:247`,
`:258`, `:381`; `elidex-script-session/src/navigation.rs:305`; `scripting.rs:136`; `content_iframe_security_tests.rs:284`,
`:468`; `content_window_open_tests.rs:410`; plus `navigation.rs`'s `MAX_RESTRICTION` uses (in (e+)). Each becomes
`parse_sandboxing_directive("")` (D-N6); none may become `SandboxingFlagSet::empty()` (G6).

**Touched files** (union, 18 with docstring-only `elidex-js/src/vm/host/storage.rs:79-80` excluded as not
polarity-bearing): the 17 above plus `elidex-js/src/vm/host/event_handler_attrs.rs` (`:602`, `:611` — type flows, no
textual change expected; listed so the reviewer checks it compiles unchanged).

### §5.2 Named risks

- **NR1 silent flip** — (e!): a rename-first migration turns every max-restriction fixture into an unsandboxed one and
  the refusal tests go red or, where a test asserts "allowed", stay green for the wrong reason. Mitigation: D-N6, G6,
  and C10b's equivalence oracle written over **token strings**, not over constants.
- **NR2 unbound defaults** — D-N5's two sites keep HEAD's permissive read; N4 must delete them with the route (J2). G7
  pins that exactly these two exist at N2a's landing, so N4's plan inherits a measured population.
- **NR3 `host_data/mod.rs` (2023 lines)** — touch-time rule applied: the touched concern (field `:187-200`, initializer
  `:863`, accessors `:1088-1141`) is re-typed in place, net LoC ≈ 0, and is deleted whole at N5; the file's reduction is
  the user's option-A placement in `#11-host-data-full-decomposition` (2026-07-11; the parent memo
  `docs/plans/2026-09-inline-box-predicate.md:1483`). No prereq split. No other touched file exceeds 1000 lines (B11).
- **NR4 `pipeline.rs` (973)** — two field types and two doc lines; net 0; G9.
- **NR5 newly recognised token** — `allow-popups-to-escape-sandbox` clears the propagate flag, which nothing reads until
  N8: no verdict changes (C10b covers HEAD's seven tokens; C10 the eighth).
- **NR6 docstrings** — B9's sites are rewritten with the new contract; `sandbox.rs`'s boa citations are deleted (the
  boa engine no longer exists in `crates/script`).

## §6. Test and coverage plan

Declared surface: no WPT subset (no runtime change; the node's `html/browsers/sandboxing/` subset is exercised by later
PRs). Oracles: `elidex-plugin` unit tables; the existing VM and shell integration suites, migrated per D-N6 with
assertions unchanged.

| # | Cell | Where |
|---|---|---|
| C10 | parse truth table (node cell): for every subset of the eight recognised tokens (256), the parsed set equals the spec list's arms evaluated on that subset (a table of `(flag, clearing tokens)` transcribed from step 3); plus literal rows — `""` and whitespace-only → `all_represented()`; navigation flag always set; `allow-top-navigation` alone clears both top-level flags; case folding; non-ASCII look-alike does not clear; unknown and unrepresented-flag tokens clear nothing; TAB/LF/FF/CR separators | `sandbox.rs` tests |
| C10b (NEW) | HEAD equivalence (K1): for every subset `T` of HEAD's seven tokens and `activation ∈ {false, true}`, each predicate on `parse(T)` equals HEAD's token-presence verdict (`allow-x ∈ T`; top navigation: `allow-top-navigation ∈ T ∨ (activation ∧ allow-top-navigation-by-user-activation ∈ T)`); every predicate true on `empty()` | `sandbox.rs` tests |
| C10c (NEW) | union (K2, J7): for every pair of parsed subsets, `p(a \| b) == p(a) && p(b)` for each single-flag predicate, and `top_navigation_allowed(a \| b, act)` implies it on `a` and on `b`; `empty()` identity, `all_represented()` absorbing | `sandbox.rs` tests |
| C10d (NEW) | totality (K4): `parse_sandboxing_directive("") == SandboxingFlagSet::all_represented()` | `sandbox.rs` tests |
| C10e (NEW) | origin: `apply_sandbox_origin` returns opaque iff `allow-same-origin` is absent from a present attribute or the frame is credentialless; absent attribute keeps the input origin | `content_iframe_security_tests.rs` (existing order-proof arms re-asserted) |
| — | migrated suites green with assertions unchanged: `tests_window_open_modals.rs`, `tests_engine_s1a.rs`, `tests_engine_s1b.rs`, `tests_call_listener.rs`, `tests_event_handler_attrs.rs`, `scripting.rs` and `navigation.rs` in-file tests, `content_iframe_security_tests.rs`, `content_window_open_tests.rs` | per crate |

C10b is the mutation guard for the flip: inverting one predicate's polarity, dropping the two-token arm, or mapping
`None` to `all_represented()` each fails it. Commands: `cargo test -p elidex-plugin --all-features`, `cargo test -p
elidex-script-session --all-features`, `cargo test -p elidex-js --all-features`, `cargo test -p elidex-shell
--all-features`; `mise run ci` before push.

## §7. Out of scope and slots

| Gap | Why not here | Trigger / owner | Re-eval |
|---|---|---|---|
| the seven unrepresented §7.1.5 flags | no reader at HEAD (B10); D1 | `#11-sandbox-flag-set-sources` (registered at the node's approval) | 2026-11-01 (the parent's) |
| unbound gates fail open | J2 is N4's | N4, edge n3 (NR2) | N4's plan |
| §7.4.2.4 steps 1–6 by relation; the navigation flag's reader | D-a | N2b, edge n6a | — |
| the propagate flag's reader; the popup set | F18 | N8, n6a | — |
| restriction-set union as a creation input | J4 | N7, n6a | — |
| CSP `sandbox` through the parser | J8 | N6, n6a | — |
| the store flip (F9, F10r, `HostData` copy deleted) | J1 | N5 | — |
| `<iframe>.sandbox` as a `DOMTokenList` (`[SameObject, PutForwards=value, Reflect] readonly attribute DOMTokenList sandbox`, `webref idl html HTMLIFrameElement`; HEAD reflects a string, `vm/host/html_iframe_proto.rs:263`) | IDL surface, not the flag set | existing `#11-classlist-dataset-extras` (roadmap's `<iframe>.sandbox` sub-task) | its own |

Own deferrals: **0** (node memo §5.2). No slot is registered or appended by N2a.

## §8. Edges and hand-offs

- **n3** (N2a → N4): F10's flags argument is `SandboxingFlagSet`; N4 receives D-N5's two unbound-default sites (G7's
  list) as population to delete with the route.
- **n6a** (N2a → N2b, N6, N7, N8): N2b reads `SANDBOXED_NAVIGATION` and absorbs `top_navigation_allowed`; N6 calls
  `parse_sandboxing_directive` for the CSP directive; N7 unions with `|` (K2); N8 reads the propagate flag. Each of
  those PRs adds its reader through a predicate in `elidex-plugin::sandbox` or argues the exception in its own plan
  (G4 is N2a's landing fact, not a node invariant).
- Records: none ship with N2a (node memo §8 records table has no N2a row; D1's amendments shipped with the approval PR).

## §9. Definition of done

Mechanical greps (each over `crates`, at the PR head):

- **G1** `git grep -n -E 'IframeSandboxFlags|parse_sandbox_attribute' -- crates` → 0.
- **G2** `git grep -n -E 'ALLOW_(SCRIPTS|SAME_ORIGIN|FORMS|POPUPS|TOP_NAVIGATION|MODALS)' -- crates` → 0.
- **G3** `git grep -n -P 'Option<\s*(elidex_plugin::)?(sandbox::)?SandboxingFlagSet\s*>' -- crates` → 0.
- **G4** `git grep -n -E 'SANDBOXED_|SANDBOX_PROPAGATES' -- crates ':!crates/core/elidex-plugin/src/sandbox.rs'` → 0.
- **G5** `git grep -n -E 'SandboxingFlagSet::all\(\)|all_represented\(\)' -- crates ':!crates/core/elidex-plugin/src/sandbox.rs'` → 0 (N4/N5 introduce F10r's use).
- **G6** `git grep -n 'SandboxingFlagSet::empty()' -- crates ':!crates/core/elidex-plugin/src/sandbox.rs'` — every hit
  is one of: the four production sites (`host_data/mod.rs` initializer, `elidex-js/src/engine.rs` `sandbox_flags`,
  `vm/host/window_dialogs.rs` `window.open` read, `content/iframe/load.rs` `parse_sandbox` absent arm), or a test
  fixture standing for HEAD's `None` (a line in (e+)'s `None` list or (e)'s `None` sites); no line of (e!) appears.
- **G7** `git grep -n -E 'map_or\(SandboxingFlagSet::empty\(\)' -- crates` → exactly two lines,
  `elidex-js/src/engine.rs` and `vm/host/window_dialogs.rs` (D-N5).
- **G8** `git grep -n -i -E 'allow-token|positive allow|iframe_bridge' -- crates/core/elidex-plugin crates/script
  crates/shell/elidex-shell/src/pipeline.rs` → 0.
- **G9** `wc -l crates/shell/elidex-shell/src/pipeline.rs` < 1000.
- **G10** the diff of each migrated test file changes no `assert` line other than D-N6's set-valued ones (reviewer
  check: `git diff origin/main -- <file> | grep -E '^[-+].*assert'`).

Behavioural: C10, C10b–C10e present and green; the four `cargo test -p` commands and `mise run ci` pass.
Preflight: `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-ibp-sandbox-flagset.md` exits 0.

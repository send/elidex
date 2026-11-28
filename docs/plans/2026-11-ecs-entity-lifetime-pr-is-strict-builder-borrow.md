# ECS entity lifetime, PR (is): the strict tree builder borrows the caller's dom — per-PR plan

Status: **plan (pre-implementation)**; `/elidex-plan-review` before code. A per-PR slice of the umbrella
[2026-11-ecs-entity-lifetime-umbrella.md](2026-11-ecs-entity-lifetime-umbrella.md) (its §5 row **is**, added by
PM 708; the slice id is PM 714 (3)'s), so it is the base case of CLAUDE.md's *Edge-dense* rule: a narrowly scoped
slice under the umbrella, not a program of its own. It lands **before** PR (ig)
([2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md](2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md)),
whose strict take window then opens on the caller's dom (§8 hand-off); the program's order is PR (is) → PR (ig) →
[the tree-model program, `docs/plans/2026-11-tree-model-shadow-root-is-a-root-umbrella.md`] → PR (io) → PR (ii) (PM 730; the umbrella §5 order predates it). Owner: L3. Base: `93ba0257`; every code anchor is at that commit (verified
2026-11-13 at `cee74989`, re-anchored 2026-11-28: every `git grep` below re-run at `93ba0257`, same output apart from the line shifts now written in; an inline "verified 2026-11-13" marks the first measurement, at `cee74989`, and every such measurement reproduced at `93ba0257`; §3's coverage-map and §9's webref lookups were re-checked in the 2026-11-28 plan-review). Rulings: PM 730 (the tree-model program between PR (ig) and PR (io)); PM 1197 (the closing slot's Re-eval, 2026-12-05); PM 708 (this slice, its order before the dispatcher-take guard, structure only, and the slot it
closes); PM 714 (the slice ids; the slot `#11-unwind-leaves-detached-build-subtree`, §8). Driver decisions approved by
the PM with PM 714: the append-panic fixture is this PR's, and PR (ig) uses it (§6); in the strict window this PR's
U-1 pins that the caller's tree is intact and PR (ig)'s U-5 pins that the dispatcher is back in the caller's dom (§6).
Before PM 714 the slice was named "i-s" and PR (ig) was half of a PR (i).

## §0. Charter

PR (is) removes the strict fragment parse's ownership shuffle. `TreeBuilder::build_fragment`
(`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:143-160`) moves the caller's `EcsDom` out of its place
into a parse-local owner (`std::mem::replace(dom, EcsDom::new())`, `:156`), runs the build on it through
`run_fragment` (`:162-244`), and moves it back (`:158`). After this PR the tree builder **borrows** the dom it builds
into, for both the fragment parse and the document parse, and no `EcsDom` ever leaves its owner's place during a
parse.

**The defect it closes** (`#11-strict-fragment-parse-unwind-loses-document`, opened by PM 708). A panic during the
strict fragment build unwinds through `run_fragment`, which drops the tree builder and, with it, the caller's dom
that the builder owns. `build_fragment`'s move-back (`:158`) never runs, so the caller's `*dom` is left holding the
placeholder `EcsDom::new()`: the whole document is gone, the taken dispatcher box is dropped with `run_fragment`'s
local (`:180`), and the slot of the placeholder is `Absent`. This contradicts:
- `build_fragment`'s own rustdoc contract (`:140-142`): "On a parse error the partial subtree is torn down and `dom`
  is left pristine". An unwind is not a parse error, but the contract's premise, that `dom` is the caller's dom
  throughout, is exactly what the shuffle breaks;
- CLAUDE.md *Concurrency by ownership and phases*: the renderer main thread is the ECS DOM's sole owner and writer.
  A parse is a phase that writes into the document; the shuffle instead transfers the document to a parse-local
  owner, so the document's lifetime briefly follows the parse's.

**Reachability.** Only an embedder that catches the unwind can observe it. The workspace profiles set no
`panic = "abort"` (`git grep -n -P '^\[profile|panic' 93ba0257 -- Cargo.toml` → `[profile.dev]` `:294`,
`[profile.release]` `:297`, and the clippy key `missing_panics_doc` `:323`; no `panic =` key, so unwinding is the
default). No production code catches a panic and continues: `git grep -n -P 'catch_unwind' 93ba0257 -- crates ':!*tests*'`
(verified 2026-11-13) gives 22 lines; of them, `crates/net/elidex-net/src/broker/cancel.rs:93`,
`crates/net/elidex-net/src/pool.rs:510` and `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs:363`,
`:397`, `:432` are inside `#[cfg(test)]` modules (the last `#[cfg(test)]` before them is at `cancel.rs:68`,
`pool.rs:408`, `upgrade.rs:361`); the only non-test call is the VM's dispatch boundary
`crates/script/elidex-js/src/vm/interpreter.rs:197`, which always re-raises (`resume_unwind`, `:256`) after
truncating its stacks (its `use` is `:169`); every other hit is a comment. So in production the panic propagates out of the process's
outermost frame, and the lost document is never observed. The defect is a contract and ownership defect, not a
user-visible one.

**Why now, and why before PR (ig).** PR (ig) puts all four dispatcher take windows under one Drop guard
(`DispatcherTake`, PR (ig) plan §4.1) that borrows the place it was opened on. On the strict window that place is the
caller's `*dom`, and at base the build runs on a value moved out of it, so without this PR the guard's plan would have
to state that on unwind it restores the dispatcher into the placeholder (PR (ig) plan §2 TAKE × SHUF). With the builder borrowing, the
guard and the builder both borrow the caller's dom, and the restore lands in the document it was taken from.

**Behaviour.** Structure only. On every non-panicking path (document parse, fragment parse, the strict parse-error
rollback, the strict → tolerant fallback in `crates/dom/elidex-html-parser/src/lib.rs:229`) the behaviour is the
base's, pinned by the existing cells of §6. The one change is on the unwind path of the strict fragment build: the
caller's dom keeps its tree (U-1). Two things on that path stay as at base and are not this PR's:
- the dispatcher: when one was installed, the taken box is still dropped while unwinding and the slot is left `Taken`
  (now in the caller's dom, where at base the placeholder's slot was `Absent`); with none installed the take takes
  nothing and the slot stays `Absent`. `#11-dispatcher-take-window-unwind-guard`, closed by
  PR (ig). U-1 asserts nothing about the slot; that is PR (ig)'s U-5 (§6);
- the throwaway: the throwaway Document, its `<html>` root and every node built under them before the panic stay
  live in the caller's dom, unreachable from its tree. This is the tolerant window's behaviour at base
  (`parse_html_fragment`, `crates/dom/elidex-html-parser/src/lib.rs:143-159`, builds in the caller's dom with no
  shuffle), so the strict window now matches it; the defect is `#11-unwind-leaves-detached-build-subtree` (PM 714;
  §8).

## §1. Scope

**In** (lines at `93ba0257`; none of the touched files is over 1000 lines, so CLAUDE.md's touch-time split does not
apply; sizes measured with `wc -l` at the base, and the largest touched file, `crates/core/elidex-ecs/src/dom/mod.rs`,
957, gains about six lines here).
- `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs` (478): `TreeBuilder` (`:67-93`) gains a lifetime and
  its `dom` field becomes `&'d mut EcsDom`; the field's doc (`:71`); the struct doc (`:65-66`, "owns … the DOM under
  construction"); `TreeBuilder::new` (`:96-112`) takes the dom and the parse state; `TreeBuilder::build_document`
  (NEW) is the one document-parse body, called by `build` (`:118-120`) and `build_with_declarative_shadow`
  (`:124-130`); `build_fragment` (`:132-160`) absorbs `run_fragment`'s body; `run_fragment` (`:162-244`), `run`
  (`:347-352`) and `into_result` (`:456-468`) are deleted; the doc of `drive` (`:358-361`) names `build_document`.
- The other `impl TreeBuilder` headers, which become `impl TreeBuilder<'_>` and nothing else (§4.2):
  `crates/dom/elidex-html-parser-strict/src/tree_builder/implied_end.rs:16` (55),
  `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:17` (398),
  `crates/dom/elidex-html-parser-strict/src/tree_builder/reset_insertion_mode.rs:6` (113),
  `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:68` (223),
  `crates/dom/elidex-html-parser-strict/src/tree_builder/text_only.rs:11` (61).
- The append-panic fixture (§6), introduced and owned here and used by PR (ig)'s unwind cells (§8 hand-off), fixture-gated as `adopt_walks`
  is (`dom/mod.rs:113-114`): the field `append_panic_countdown` (NEW) on `EcsDom` and its initialiser in
  `EcsDom::new` (`crates/core/elidex-ecs/src/dom/mod.rs:159-172`); `EcsDom::arm_append_panic_fixture` (NEW) and the
  check at the top of `EcsDom::append_child`, both in `crates/core/elidex-ecs/src/dom/tree/mutation.rs` (723;
  `append_child` `:29`); the `fixtures` feature's comment in `crates/core/elidex-ecs/Cargo.toml` (26; `:21-22`, which
  names only the ownerless creators and is already stale for `adopt_walks`).
- Cells: U-1 and U-2 in `crates/dom/elidex-html-parser-strict/src/tree_builder/tests_fragment.rs` (741; about
  +90 lines, so about 830 after this PR; PR (ig)'s strict unwind cell, U-5, is a separate cell in the same file
  whose subject is the dispatcher, §8).

**Out.**
- The crate's public API: `parse_strict` (`crates/dom/elidex-html-parser-strict/src/lib.rs:66`) and
  `parse_fragment_strict` (`:93`) keep their signatures and docs; `TreeBuilder` is `pub(crate)`.
- The 56 mode-handler signatures (`fn …(tb: &mut TreeBuilder, …)`; §4.2) and every `self.dom` / `tb.dom` use: their
  text is unchanged.
- The dispatcher take and restore: they keep the base's form (`begin_detached_fragment`'s returned box, restored by
  hand on both return paths), moved with `run_fragment`'s body into `build_fragment`. The guard is PR (ig)'s.
- The tolerant backend (`parse_html_fragment`), `parse_into_inert_document`, and `parse_fragment_progressive`: no
  change; the last is a regression pin (§6).
- The throwaway left in the caller's dom on unwind (§0; `#11-unwind-leaves-detached-build-subtree`, §8).

## §2. Coupled invariants

Elements: **OWN** (the dom's owner keeps it in its place for the whole parse; the builder borrows), **UNW** (unwinding
out of the build), **RB** (the strict parse-error rollback leaves the caller's dom pristine, `build_fragment`'s
contract, `:140-142`), **FB** (the strict → tolerant fallback over the same dom, `elidex-html-parser/src/lib.rs:229`
→ `:245`), **DOC** (the document parse: a fresh dom whose owner is the parse entry, returned in `ParseResult`),
**ONE** (one `TreeBuilder` type and one constructor for both parses), **TAKE** (the dispatcher's take and restore
around the fragment build; PR (ig)'s guard comes after), **CACHE** (the caller's `document_root` cache, preserved
because the throwaway is created cache-free), **FIX** (the append-panic fixture).

| Pair | Intersection | Pinned by |
|---|---|---|
| OWN × UNW | unwinding drops the builder, which holds a borrow; the caller's place still holds its own dom, its tree untouched by the unwind (the nodes already built are under the throwaway, which is not linked to the caller's tree) | U-1; M-1 |
| OWN × RB | the rollback (`:224-234`: the stack-only declarative-shadow templates, then `despawn_subtree(document)`) runs on the caller's dom through the borrow, as it ran on the moved dom; the entity count returns to its value before the parse | `tests_fragment.rs:214`, `:492`; C1's U5 (`crates/dom/elidex-html-parser/src/tests_template_owner.rs:126`); M-4 |
| OWN × FB | `build_fragment`'s borrow ends when it returns `Err`, so `parse_fragment_progressive` hands the same `dom` to the tolerant parse; at base the move-back did the same at `:158`. The borrow checker enforces the order | `crates/dom/elidex-html-parser/src/tests.rs:752` |
| OWN × DOC | the document parse's owner is `build_document`'s local `dom`; the builder borrows it for `drive`, and once the borrow ends the local is moved into `ParseResult`. On a parse error the local is dropped with `build_document`'s frame, as the base dropped the builder that owned it | the document-parse cells (§6) |
| ONE × OWN | one representation of the dom in the builder (`&'d mut EcsDom`) for both parses; the parses differ only in who owns the dom, never in the builder's type. The two struct literals (`:103`, `:200`) become one, in `TreeBuilder::new` | structural (§8 DoD) |
| ONE × handlers | the lifetime reaches the six `impl` headers and the struct, not the handlers: a handler's `&mut TreeBuilder` names the type with an elided lifetime, which the workspace lints allow, and no handler returns a reference, so no elision becomes ambiguous (§4.2) | structural (compile); `mise run ci` (clippy) |
| TAKE × OWN | `begin_detached_fragment` takes the box from the caller's dom and the restore writes it back into the same dom on both return paths, before `build_fragment` returns: the base's restore point, so the caller's placement fires events as before | U-2; `crates/script/elidex-script-session/src/mutation/html_fragment.rs:668`; M-3 |
| TAKE × UNW | unchanged by this PR: when a dispatcher was installed, the box is a local of the unwinding frame and is dropped; the slot is left `Taken`, now in the caller's dom. PR (ig) closes it with a guard that borrows the same dom the builder borrows (§8) | not pinned here (PR (ig) U-5) |
| TAKE × RB | the strict rollback's `despawn_subtree` nests a take inside the fragment window (it finds `Taken`, takes nothing, restores nothing), as at base | `tests_fragment.rs:214`; C1's U5 |
| CACHE × OWN | the throwaway is created with `create_document_node`, not `create_document_root`, in the caller's dom, so the caller's `document_root` survives every return path, as at base; after an unwind it now survives too | `tests_fragment.rs:465`; U-1 |
| FIX × UNW | the fixture panics inside `append_child` before it mutates anything, so the unwind starts inside the build with the throwaway in a consistent state (U-1; M-2); it disarms itself when it fires, so the cell's later appends do not panic (U-1's closing append; M-5) | U-1; M-2, M-5 |
| FIX × TAKE | the fixture is in `append_child`, below the dispatcher: it fires whether or not the slot is `Taken`, so the same fixture serves the three parse windows' unwind cells of PR (ig) (its teardown window's cell panics in its own closure) | PR (ig) U-5, U-6, U-8 (they arm this PR's fixture) |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* step 1 (the temporary Document) | built in the caller's dom | `begin_detached_fragment`, now on the borrowed dom (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *initialize an HTML fragment parser* steps 12–14 (root created, appended to the temporary Document, sole entry of the stack of open elements) | — | `begin_detached_fragment`; `state.open_elements.push(root)` | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *initialize an HTML fragment parser* step 15 (root's insertion target redirection map entry), with *HTML fragment parsing algorithm* step 2 (the fragment is created given the target's node document) | nodes created under root take the context's node document | `FragmentParseRoot` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *initialize an HTML fragment parser* steps 7 and 10 (allow declarative shadow roots; parser scripting disabled) | — | `TreeBuilder::new`'s `allow_declarative_shadow`; `state.scripting` | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *initialize an HTML fragment parser* step 11 (the tokenization stage's state) | HTML-namespace context only | `set_fragment_tokenizer_state` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *initialize an HTML fragment parser* step 16 (a template context pushes "in template") | HTML-namespace `template` only | `state.template_modes` (unchanged) | ✓ | yes |
| WHATWG HTML §13.2.4.1 The insertion mode | *reset the insertion mode appropriately* step 3, the fragment case (called from *initialize an HTML fragment parser* step 18) | the fragment case substitutes the context element | `reset_insertion_mode_appropriately` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *initialize an HTML fragment parser* step 19 (the form element pointer) | stops at a shadow root; HTML-namespace `form` only | `set_fragment_form_pointer` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* step 5 (run the parser) | — | `drive` on the borrowed dom | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* steps 6–7 (adopt, the identity here; return the fragment) | root's children come back detached | `take_fragment_children` → `finish_detached_fragment` (unchanged) | ✓ | yes |
| WHATWG HTML §13.2.2 Parse errors | the first parse error | strict aborts; the rollback is elidex's, not a spec step, and leaves the caller's dom pristine | the `Err` arm of `build_fragment` (moved from `run_fragment`) | ✓ | yes |
| WHATWG HTML §13.2.6 Tree construction | the document parse | builds into the dom `build_document` owns | `TreeBuilder::build_document` (NEW) | ✓ | yes (document markup) |

**Breadth**: K=1 spec (HTML), M=12 table rows over 4 sections (the coverage-map command counts sections:
`.claude/tools/webref coverage-map html 13.4 html 13.2.4.1 html 13.2.2 html 13.2.6` → "spec=1 (html), step=4 entries",
verdict "ok → single PR scope", verified 2026-11-13; the 12 rows are steps within those 4 sections). The PR implements no spec step: every row is a step whose
outcome the change of who owns the dom must preserve, and each is preserved because the step's code is moved, not
edited (§5). The unwind path is not a spec branch, and neither is the dispatcher suppression ("not a spec step",
`tree_builder/mod.rs:171-179`).

### §3.1 User-input touch audit

The strict fragment build is reached with script-supplied markup through `parse_fragment_progressive`, the only
non-test caller of `parse_fragment_strict` (`git grep -n -P '\bparse_fragment_strict\(' 93ba0257 -- crates ':!*tests*' ':!*test_*'`
→ the definition `crates/dom/elidex-html-parser-strict/src/lib.rs:93` and the call
`crates/dom/elidex-html-parser/src/lib.rs:229`, verified 2026-11-13). Its callers are the `innerHTML` /
`setHTMLUnsafe`, `outerHTML` and `insertAdjacentHTML` appliers in
`crates/script/elidex-script-session/src/mutation/html_fragment.rs` (lines 93/174/209/218/235/245, verified
2026-11-13 with `git grep -n -P '\bparse_fragment_progressive\(' 93ba0257 -- crates ':!*tests*'`; the seventh hit,
`crates/dom/elidex-dom-api/src/range/mutation.rs:649`, is a doc comment), and `DOMParser.parseFromString` reaches the
same applier through `parse_into_inert_document` (PR (ig) plan §3.1 lists the script entry points). The document parse
is reached with document markup through `crates/dom/elidex-html-parser/src/lib.rs:66-67` (`parse_strict`) and
`:187` (`parse_progressive_str`).

What script can observe of the change: nothing. On every returning path the dom, the returned nodes and the
dispatcher's restore point are the base's. A panic is not script-catchable (the VM boundary re-raises, §0), so the
unwind path's change is visible only to an embedder that catches the panic.

Adjacent pre-existing exposure: the throwaway left live after an unwind (§0; `#11-unwind-leaves-detached-build-subtree`,
§8), now in the strict window as it already was in the tolerant one; and the slot left `Taken` (PR (ig)).

## §4. Design

### §4.1 The representation: `TreeBuilder<'d>` borrows its dom

```rust
// crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs

/// The strict tree builder: owns the tokenizer and the parse state, borrows the DOM it builds into, and drives
/// WHATWG HTML §13.2.6 tree construction. The dom's owner keeps it in its place for the whole parse: the caller for
/// §13.4 fragment parsing, `build_document` for document parsing.
pub(crate) struct TreeBuilder<'d> {
    tokenizer: Tokenizer,
    /// The DOM under construction, borrowed from its owner for the parse.
    dom: &'d mut EcsDom,
    document: Entity,
    state: ParseState,
    pending_text: String,
    pending_table_text: String,
    allow_declarative_shadow: bool,
}

impl<'d> TreeBuilder<'d> {
    /// The one constructor, for both parses. `allow_declarative_shadow` is WHATWG HTML §13.2.4.5 Other parsing
    /// state flags (the base's sentence on `new`, kept).
    fn new(
        html: &str,
        dom: &'d mut EcsDom,
        document: Entity,
        state: ParseState,
        allow_declarative_shadow: bool,
    ) -> Self { /* the one struct literal */ }

    /// Document parse, WHATWG HTML §13.2.6 Tree construction (the body `build` and `build_with_declarative_shadow`
    /// share), into a fresh dom this function owns.
    fn build_document(html: &str, allow_declarative_shadow: bool) -> Result<ParseResult, StrictParseError> {
        let mut dom = EcsDom::new();
        let document = dom.create_document_root();
        TreeBuilder::new(html, &mut dom, document, ParseState::new(), allow_declarative_shadow).drive()?;
        Ok(ParseResult { dom, document, errors: Vec::new(), encoding: None, tier: ParseTier::Clean })
    }

    /// §13.4 fragment parse: `run_fragment`'s body, on the caller's dom.
    pub(crate) fn build_fragment(
        html: &str,
        dom: &mut EcsDom,
        context: Entity,
        opts: ParseFragmentOptions,
    ) -> Result<Vec<Entity>, StrictParseError> {
        let (document, root, saved_dispatcher) = dom.begin_detached_fragment(context);
        let mut state = ParseState::new();
        // `:188-199` unchanged: scripting, the stack, the fragment context, the template mode (reads `dom`).
        let mut tb = TreeBuilder::new(html, dom, document, state, opts.allow_declarative_shadow);
        // `:209-237` unchanged: the three §13.4 set-ups, `drive`, `Ok` = `take_fragment_children`, `Err` = rollback.
        // `:238-242` unchanged: restore the dispatcher on `tb.dom`.
        result
    }
}
```

`build`'s `TreeBuilder::new(html, true).run()` becomes `Self::build_document(html, true)`, and
`build_with_declarative_shadow`'s likewise; their signatures, and the 11 test calls of them, are unchanged
(`tree_builder/tests.rs` lines 140/351/410/454/491/521, `tests_foreign.rs` lines 22/35/269/325,
`tests_html5lib_tree.rs` line 179; verified 2026-11-13 with
`git grep -n -P 'TreeBuilder::(build|build_with_declarative_shadow)\(' 93ba0257 -- crates/dom/elidex-html-parser-strict/src/tree_builder`).
`run` and `into_result` existed to move the owned dom out of a consumed builder, and `run_fragment` to return the
moved dom beside the result; with a borrowing builder none of the three has a job, so they are deleted, not kept as
wrappers. The rustdoc of `build_fragment` keeps its contract (`:132-142`) and gains two sentences, which name only
what exists at this PR's head: "The builder borrows `dom`; it never leaves the caller's place, so a panic during the
build leaves the caller's tree in `dom`. What an unwind leaves of the build itself is not torn down: the throwaway
Document and the nodes built under it stay (`#11-unwind-leaves-detached-build-subtree`), and the taken dispatcher is
not put back (`#11-dispatcher-take-window-unwind-guard`)." (PR (ig) rewrites the second clause when it closes that
slot.) The comment
`:149-155` (the shuffle's rationale) is deleted. The comment `:171-179` loses "then the dispatcher is restored" only
if the restore moves; here it does not.

**Why this is the ideal, not a workaround.**
1. **Ownership.** A parse is a phase that writes into a document; it is not an owner of the document. With the
   borrow, the document has one owner for its whole life (CLAUDE.md *Concurrency by ownership and phases*), and the
   builder's type says so: `TreeBuilder<'d>` cannot outlive the dom it builds into.
2. **Unwind safety is structural.** Rust drops a borrow on unwind, never the borrowed value; no guard, no
   `catch_unwind`, no restore code is needed for the document to survive. The defect is not repaired, it becomes
   unwritable.
3. **One representation** (CLAUDE.md *One issue, one way*). Both parses build through the same type with the same
   field type and one constructor; they differ only in who owns the dom, which is the caller's business, not the
   builder's. At base the two paths also share one type, but the fragment path needs a shuffle to fit the owned
   representation; the borrow removes the shuffle without adding a second form.
4. **It composes with PR (ig).** PR (ig)'s `DispatcherTake` borrows the place it is opened on; after this PR the
   strict window's guard and the builder borrow the same dom (the builder reborrows through the guard's `dom()`), so
   the restore needs no reasoning about a placeholder (§8 hand-off).
5. **Cost.** The comment at `:149-155` rejected "threading a lifetime through every mode handler". Measured, the
   lifetime does not reach the handlers (§4.2): six `impl` headers, the struct and the constructor change; the 41
   `self.dom` uses and 15 `tb.dom` uses keep their text (auto-deref through `&mut`). A placeholder `EcsDom::new()`
   is no longer built and dropped per fragment parse.

**Rejected alternatives.**
- **R-1: keep the shuffle and restore the dom on unwind** (a Drop guard on the place, or `catch_unwind` in
  `build_fragment` followed by `resume_unwind`). The moved dom is owned by the tree builder inside `run_fragment`'s
  frame, and unwinding drops it before any guard in `build_fragment` runs; a guard can put it back only if the builder
  holds the place, which is the borrow. `catch_unwind` would need `AssertUnwindSafe` over the whole build and would be
  a reactive repair of a transfer that need not exist (CLAUDE.md *設計優先*).
- **R-2: an owned-or-borrowed enum** (`enum DomSlot<'d> { Owned(EcsDom), Borrowed(&'d mut EcsDom) }`, with a `Deref`
  or a branch at each use). It still threads `'d`, keeps two ownership forms inside the builder, and makes every
  reader of the field ask which one it has: two ways for one job.
- **R-3: pass `&mut EcsDom` to every handler and method as a separate argument.** It changes the 56 handler
  signatures and their call sites, plus every method in the six `impl` blocks that reads `self.dom`, and splits the
  parser's state (which the spec defines on one parser object) from the document it builds, for no gain over a borrowed
  field.
- **R-4: a generic builder** (`TreeBuilder<D: BorrowMut<EcsDom>>`). Two monomorphised builder types, a bound on every
  `impl` and on every handler (which would become generic): the "two builder types" that *One issue, one way* rules
  out.
- **R-5: owned for documents, borrowed for fragments, as two types or two constructors with different field types.**
  Two representations by construction.

### §4.2 What the lifetime touches (measured)

- The struct and the six `impl TreeBuilder {` headers: `git grep -n -P '^impl(<[^>]*>)? TreeBuilder\b' 93ba0257 -- crates/dom/elidex-html-parser-strict/src`
  → 6 lines (verified 2026-11-13): `implied_end.rs:16`, `insert.rs:17`, `mod.rs:95`, `reset_insertion_mode.rs:6`,
  `shadow.rs:68`, `text_only.rs:11`. An `impl` header must name the lifetime (`impl TreeBuilder<'_>`; without it,
  E0726 "implicit elided lifetime not allowed here"; reproduced with rustc 1.99.1 on a two-type probe, 2026-11-13).
- The handlers: 56 parameter lines of the form `tb: &mut TreeBuilder` / `&TreeBuilder`
  (`git grep -n -P '\w+: &(mut )?TreeBuilder\b' 93ba0257 -- crates/dom/elidex-html-parser-strict/src ':!*tests*' | wc -l`
  → 56, verified 2026-11-13) keep their text. A path type with an elided lifetime is accepted in a fn signature
  (the same probe), and the lint that would flag it, `elided_lifetimes_in_paths`, is allow-by-default and not enabled
  by the workspace (`[workspace.lints.rust]`, `Cargo.toml:301-302`, sets only `unsafe_code`). All 56 lines are in
  `tree_builder/modes/`, and no handler returns a reference:
  `git grep -h -P -o '\)\s*->\s*[^{]+' 93ba0257 -- crates/dom/elidex-html-parser-strict/src/tree_builder/modes | sort | uniq -c`
  → 4 return types, `Result<Flow, StrictParseError>` (38), `Flow` (6), `Result<(), StrictParseError>` (6) and `bool`
  (5), verified 2026-11-13. So the extra input lifetime makes no output elision ambiguous.
- The field's uses: `git grep -c -F 'self.dom' 93ba0257 -- crates/dom/elidex-html-parser-strict/src` → 41
  (`implied_end.rs` 2, `insert.rs` 24, `mod.rs` 9, `reset_insertion_mode.rs` 1, `shadow.rs` 5), and
  `git grep -c -F 'tb.dom' …` → 15 (`mod.rs` 4, `modes/foreign.rs` 10, `modes/in_body.rs` 1), verified 2026-11-13.
  Method calls auto-deref through `&mut EcsDom` unchanged. The three `&self.dom` arguments (`insert.rs:380`, `:388`,
  `:396`, to `parse_state::has_in_scope` / `has_entity_in_scope`, which take `&EcsDom`) coerce from `&&mut EcsDom`;
  if clippy prefers `&*self.dom` there, that is the only spelling change. The moves of the field (`mod.rs:243`,
  `:462`) go with `run_fragment` and `into_result`.
- No other code names the type in a type position: the non-test `TreeBuilder` hits outside the handlers' parameters
  are the six headers, the struct, the two literals (`mod.rs:103`, `:200`), the `use` lines of the mode files, two
  doc links and the two calls in `lib.rs:67`, `:99`.

### §4.3 ECS-native shape

- **The parse is a phase over the one World, not an owner of it.** The `EcsDom` (the World and its slot state) has
  one owner for its whole life, the caller's place; the tree builder borrows it for the parse (CLAUDE.md *Concurrency
  by ownership and phases*: a single writer, with the writer's window made explicit by the borrow's scope). OO idiom
  → ECS: a parser object that owns "its" document and hands it back → a system run over the World by `&mut`, with no
  transfer of the World. The base's shuffle (`std::mem::replace(dom, EcsDom::new())`) was the OO form; it put a
  second, placeholder World in the caller's place for the parse's duration.
- **Entities built by the parse are World entities from creation.** The throwaway Document, its root and every node
  are spawned in the caller's World (as the tolerant parse already does); nothing is built in a side World and
  merged. What an unwind leaves of them is a lifetime question of the ECS (`#11-unwind-leaves-detached-build-subtree`,
  umbrella §8), not of the parser.
- **No side-store is added.** The fixture is a fixture-gated field on `EcsDom` (a countdown, World-level test state,
  like `adopt_walks`), not a registry keyed by entity.

## §5. Touch list (per function)

| Function | Change |
|---|---|
| `TreeBuilder` (`tree_builder/mod.rs:67`) | gains `'d`; `dom: &'d mut EcsDom`; struct and field docs (§4.1) |
| `TreeBuilder::new` (`:100`) | takes `dom`, `document` and `state`; the one struct literal |
| `TreeBuilder::build_document` (NEW) | the document parse (§4.1); owns the fresh dom |
| `TreeBuilder::build` (`:118`), `TreeBuilder::build_with_declarative_shadow` (`:125`) | call `build_document`; signatures unchanged |
| `TreeBuilder::build_fragment` (`:143`) | the shuffle (`:149-158`) goes; `run_fragment`'s body (`:171-243`) moves in, on the borrowed dom; rustdoc gains the unwind sentence |
| `TreeBuilder::run_fragment` (`:165`), `TreeBuilder::run` (`:349`), `TreeBuilder::into_result` (`:460`) | deleted |
| `TreeBuilder::drive` (`:362`) | doc `:358-361` names `build_document` instead of `Self::run` |
| the five other `impl TreeBuilder` headers (§4.2) | `impl TreeBuilder<'_>` |
| `EcsDom` (`crates/core/elidex-ecs/src/dom/mod.rs:67`), `EcsDom::new` (`:159`) | the fixture field `append_panic_countdown` (NEW) and its initialiser, fixture-gated |
| `EcsDom::arm_append_panic_fixture` (NEW), `EcsDom::append_child` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:29`) | the fixture (§6), fixture-gated |
| `crates/core/elidex-ecs/Cargo.toml` `fixtures` comment (`:21-22`) | names the two fixtures beside the ownerless creators (the walk counter and the append-panic countdown) |

## §6. Cells

**The append-panic fixture (NEW, `#[cfg(any(test, feature = "fixtures"))]`), introduced and owned here, used by PR (ig).** A
panic has to be raised inside the build from outside it, and nothing outside the build runs inside it: the mutation
dispatcher is taken for the window, so a panicking dispatcher never fires (that suppression is what
`tests_fragment.rs:705` pins), and no valid or invalid markup should panic the builder. `EcsDom` gains a countdown,
`append_panic_countdown: u32` (NEW; 0 = disarmed), and `EcsDom::arm_append_panic_fixture(&mut self, nth: u32)`
(NEW), beside `append_child`; at the top of `append_child`, before any check or mutation, an armed countdown is
decremented and, when it reaches 0, panics. The `nth` `append_child` after arming therefore panics with nothing
mutated, and the fixture is disarmed once it has fired. PR (ig)'s unwind cells arm the same fixture (PR (ig) plan §6,
which refers here); defining it here once gives the program one injection point. The strict crate
already enables `fixtures` from `[dev-dependencies]` (`crates/dom/elidex-html-parser-strict/Cargo.toml:22`). Arming
count: `begin_detached_fragment` appends once (`crates/core/elidex-ecs/src/dom/tree/teardown.rs:313`), so `nth = 2`
panics at the build's first append (`tree_builder/insert.rs:45`), after the take and the throwaway exist.

Rejected injection points: a strict-crate-local test hook (for example in `TreeBuilder::dispatch`) would serve this
cell only, and PR (ig) would still need an `elidex-ecs` one for the tolerant and inert-document windows: two
fixtures for one purpose. A panicking dispatcher cannot fire inside the window (above).

- **U-1** `fragment_build_unwind_keeps_the_callers_document` (NEW; the unwind cell): a caller document
  (`create_document_root` → `doc`), `body` under it, the context `ctx` under `body`, and an existing child `pre` under
  `ctx`; record `doc`'s and `ctx`'s child lists. `arm_append_panic_fixture(2)`; `catch_unwind(AssertUnwindSafe(||
  parse_fragment_strict("<p></p>", ctx, &mut dom, ParseFragmentOptions::default())))` returns `Err`. Afterwards:
  `dom.contains(doc)`, `dom.document_root() == Some(doc)`, `dom.children(doc)` and `dom.children(ctx)` equal the
  recorded lists, `dom.children(body) == [ctx]`, `dom.get_parent(ctx) == Some(body)`. At base every one of these
  fails: the place holds a fresh `EcsDom::new()`. Last, an `append_child` of a new element under `ctx` returns `true`
  without panicking: the fixture disarmed itself when it fired. The cell asserts nothing about the dispatcher slot (U-1 installs no
  dispatcher, so the take takes nothing and the slot stays `Absent`; with one installed it is left `Taken` until PR (ig), and that is the subject of PR (ig)'s U-5, a separate cell, §8) and nothing about the entity count (the throwaway stays live, §0).
- **U-2** `fragment_build_restores_dispatcher_on_return` (NEW): `ctx` under a caller document, a counting dispatcher
  installed (an `Arc<AtomicUsize>` incremented per dispatch, as in `tests_fragment.rs:705`). Two runs, one per return
  path: (a) `parse_fragment_strict("<p>a</p>", …)` returns `Ok`; (b) `parse_fragment_strict("<div><span></div>", …)`
  returns `Err` (the misnested end tag of `tests_fragment.rs:214`). After each, the counter is still 0 (nothing fired
  inside the window), and an `append_child` of a new element under `ctx` increments it: the box is back in the
  caller's dom. Base passes it; it pins that the restore, moved with `run_fragment`'s body, still runs on both paths.

**Unchanged and still holding** (the regression pins of §2; no expectation changes):
- the fragment parse: the whole of `tests_fragment.rs`, in particular `:174` (detached return), `:214` (rollback, no
  leak, pristine context), `:465` (the caller's `document_root` survives), `:492` (rollback despawns a declarative
  shadow root), `:705` (the build fires nothing);
- the rollback with a template and the inert template document: C1's U5,
  `crates/dom/elidex-html-parser/src/tests_template_owner.rs:126`, and U4 `:94`;
- the fallback: `crates/dom/elidex-html-parser/src/tests.rs:752` (strict first, then tolerant over the same dom),
  `:787`, `:423`;
- the placement after the parse fires `Insert`: `crates/script/elidex-script-session/src/mutation/html_fragment.rs:668`;
- the document parse: the 11 `TreeBuilder::build` / `build_with_declarative_shadow` calls of §4.1, the
  html5lib tree corpus (`tests_html5lib_tree.rs:179`), the crate's own `parse_strict` tests
  (`crates/dom/elidex-html-parser-strict/src/lib.rs`, the module at `:102`), and
  `crates/dom/elidex-html-parser/src/tests_node_document.rs:71`.

## §7. Mutants (run on the final head)

| Id | Mutation | Killed by |
|---|---|---|
| M-1 | `build_fragment` builds into an owned temporary moved out of `*dom` and moves it back afterwards (the base shuffle, through `TreeBuilder::new(html, &mut owned, …)`) | U-1 only: every return-path cell passes, which shows U-1 is the unwind pin |
| M-2 | `arm_append_panic_fixture` arms nothing | U-1 (`catch_unwind` returns `Ok`): the positive control that the cell's panic is raised inside the build |
| M-3 | the dispatcher restore is dropped on the `Ok` path; separately, on the `Err` path (one run each) | U-2 (a) / (b); the `Ok` run also `html_fragment.rs:668` |
| M-4 | the `Err` arm skips `despawn_subtree(document)` | `tests_fragment.rs:214`; C1's U5 |
| M-5 | on firing, the countdown is set to 1 instead of being left at the disarmed 0, so the next `append_child` panics too | U-1 (its closing append panics); PR (ig)'s unwind cells, which append afterwards, too |

## §8. DoD and landing

**DoD.**
- U-1 and U-2 green; the §6 regression pins green; M-1 to M-5 each killed on the final head.
- `cargo test -p elidex-html-parser-strict --all-features`, then `-p elidex-ecs`, `-p elidex-html-parser`,
  `-p elidex-script-session` and `-p elidex-form` (the DOMParser path nests the strict build), each
  `--all-features`; then `mise run ci`.
- Structural, the representation: `git grep -n -P 'mem::(replace|swap|take)\(' -- crates/dom/elidex-html-parser-strict/src/tree_builder ':!*tests*'`
  → only the two text-buffer takes (`insert.rs:189`, `modes/in_table_text.rs:18`). At `93ba0257`: 3 lines, those two
  and `mod.rs:156` (the shuffle). Across the workspace no other code moves an `EcsDom` out of its place
  (`git grep -n -P 'mem::(replace|take)\([^;]*EcsDom|EcsDom::new\(\)\s*\)' 93ba0257 -- crates` at `93ba0257` → 4
  lines: `mod.rs:156` and three `Box::new(EcsDom::new())` constructions in benches and a test).
- Structural, one builder form: `git grep -n -P '(^|=)\s*TreeBuilder \{' -- crates/dom/elidex-html-parser-strict/src`
  → 1 line, in `TreeBuilder::new`; at `93ba0257`, 2 (`mod.rs:103`, `:200`). `git grep -n -P 'fn (run_fragment|run|into_result)\b' -- crates/dom/elidex-html-parser-strict/src`
  → no line (exit 1); at `93ba0257`, 3 (`mod.rs:165`, `:349`, `:460`). The field's type is `&'d mut EcsDom` (read at
  the diff).
- The fixture: both items are `#[cfg(any(test, feature = "fixtures"))]`, and `fixtures` stays enabled only from
  `[dev-dependencies]` (the comment at `crates/core/elidex-ecs/Cargo.toml:21-22` says so; read at the diff).

**Landing.**
- `#11-strict-fragment-parse-unwind-loses-document` (opened by PM 708) **closes** at this PR's landing (sha): the
  builder borrows the caller's dom, so an unwind out of the strict build leaves the caller's tree in place (U-1). pmo
  writes the ledger.
- **Ledger text this PR makes false** (pmo, at landing). The Note under `#11-strict-fragment-parse-unwind-loses-document`
  and the matching PM 709 parenthetical in `#11-dispatcher-take-window-unwind-guard` say the strict window's slot is
  dropped with the builder rather than left `Taken`. After this PR the strict window is like the other three: when a
  dispatcher was installed, an unwind leaves the slot `Taken`, in the caller's dom (§0). pmo amends both passages, and
  the Note's instruction to keep that fact on this slot's side when PR (ig) closes the guard slot is withdrawn.
- **Hand-off to PR (ig)** (the points its plan depends on; the PR (ig) plan states each). PR (ig)'s strict window
  design depends on this PR:
  - its strict window holds `DispatcherTake` in `build_fragment` and the builder borrows through the guard
    (`let (mut window, document, root) = dom.begin_detached_fragment(context); … TreeBuilder::new(html, window.dom(), …)`);
    there is no `run_fragment`, no shuffle and no placeholder. Its §1 "Out" item on the shuffle, its §2 TAKE × SHUF
    row, its §4.1 strict snippet and "Why it is unwind-safe" item 4, and its §5 row for
    `build_fragment` / `run_fragment` re-anchor to this PR's head;
  - the two strict unwind cells divide the subject: this PR's U-1 pins that the caller's tree is intact, and
    PR (ig)'s U-5 (a separate cell in `tests_fragment.rs`) pins that the dispatcher is back in the caller's dom
    (`Installed`, the counter counts), without re-asserting the tree;
  - the append-panic fixture is this PR's: PR (ig)'s §1 and §6 refer to it and introduce none of its items
    (`dom/mod.rs`'s field, `mutation.rs`'s check and arming method, `Cargo.toml`'s comment);
  - the umbrella §8 slot row's strict window anchors (`tree_builder/mod.rs:180` → `:241`) move into
    `build_fragment` at this PR's head.
- **`#11-unwind-leaves-detached-build-subtree`** (opened by PM 714; owner L3; trigger as the ledger: a PR after PR (ii), or a PR adding an unwind boundary around parsing; its row is the
  umbrella §8's; this PR does not change it). An unwind out of a fragment window leaves the throwaway Document, its
  `<html>` root and every node built under them live in the caller's dom, unreachable from its tree. At base this
  holds for the tolerant window (`parse_html_fragment`) and, through it, for the inert-document build's nested
  parses; the strict window's case was hidden by the lost document and is now the same. Neither PR (ig) (its guard
  restores only the dispatcher; teardown stays caller-owned, PR (ig) plan §4.1) nor PR (ii) (it reclaims owned units,
  and the throwaway is owned by nothing) covers it.

## §9. Citation table

Looked up with `.claude/tools/webref` (`heading` and `dfn` for the anchors, `body html parsing-html-fragments` for the
step numbers), 2026-11-13.

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [S1] | WHATWG HTML §13.4 Parsing HTML fragments — *HTML fragment parsing algorithm* steps 1, 2, 5, 6, 7 | `html#parsing-html-fragments`, `html#html-fragment-parsing-algorithm` | §3 |
| [S2] | WHATWG HTML §13.4 — *initialize an HTML fragment parser* steps 7, 10, 11, 12–14, 15, 16, 18, 19 | `html#initialize-an-html-fragment-parser` | §3 |
| [S3] | WHATWG HTML §13.2.4.1 The insertion mode — *reset the insertion mode appropriately* | `html#the-insertion-mode`, `html#reset-the-insertion-mode-appropriately` | §3 |
| [S4] | WHATWG HTML §13.2.4.5 Other parsing state flags — *insertion target redirection map* | `html#insertion-target-redirection-map` | §3 (step 15 row) |
| [S5] | WHATWG HTML §13.2.2 Parse errors | `html#parse-errors` | §3 |
| [S6] | WHATWG HTML §13.2.6 Tree construction | `html#tree-construction` | §3 |
| [S7] | WHATWG HTML §13.4 — *fragment case* | `html#fragment-case` | §3 (the reset row) |

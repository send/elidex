# Fragment top-level `<template shadowrootmode>` stays a plain template (per-PR plan-memo)

Slot: `#11-strict-fragment-declarative-shadow-on-context` (ledger row "SCOPE INVERTED 2026-10-24"). Base: `origin/main`
`e44ffce6`. Owner lane: L3 DOM/form.

**Rev 1** (2026-10-31) — first draft, before `/elidex-plan-review`. **Status: blocked on a user decision (§0.1).** This
memo plans the spec-current option; the alternative is recorded in §8 with what it keeps and what it overturns.

## §0. Frame, premises, decisions

### §0.1 The decision: this reverses a ratified reading

PR #337 (squash `cb4ab7e7`, 2026-06-13) ratified **DSD-on-context**: `el.setHTMLUnsafe('<template
shadowrootmode=open>…')` attaches a shadow root to `el`, the fragment-parse context. This memo plans the opposite outcome.
That is a reversal of a ratified reading, so the user decides; the memo does not.

| Side | Text it rests on | Outcome |
|---|---|---|
| #337 (2026-06) | Spec text of the time: the declarative-shadow branch ran on the **adjusted current node** being "not the topmost element", with the host being the adjusted current node. §13.2.4.2 defines the adjusted current node as "the parser's fragment context element if that element is non-null and the stack of open elements has only one element in it (fragment case)". For a top-level template the stack is `[synthetic html root]`, so the adjusted current node is the context, which is not topmost, so the shadow attaches to the context. The #337 plan draft first said "top-level → plain template"; implementation inverted it after the VM `element_get_html_*` round-trip tests failed (memory `html-fragment-unify-slice2b-plan.md` §2.4, :86-92; also :12, :36, :155). | Shadow on context; the context is mutated (the one documented exception to the read-only-context contract); strict declines and falls back to tolerant. |
| Current WHATWG HTML §13.2.6.4.4 (`webref body html parsing-main-inhead`, read 2026-10-31) | "A start tag whose tag name is "template"", step 6: "Let declarativeShadowHostElement be the current node." Note: "This is not the adjusted current node, so that in the fragment case a top-level template element never attaches a shadow root to the parser's fragment context element." Step 7: hostIsRoot is true if declarativeShadowHostElement is the topmost element in the stack of open elements. Step 13 "Keep" → 1.1: if allow declarative shadow roots is false, **or hostIsRoot is true**, or the insertion target redirection map has declarativeShadowHostElement → insert the template as a regular element and return. | Plain `<template>` among the returned nodes; the context is never mutated. |

The spec note names this exact case, so the outcome is not spec-silent and no browser measurement is needed to choose
it.

### §0.2 Premises re-measured (at `e44ffce6`)

- Tolerant: `crates/dom/elidex-html-parser/src/convert.rs:94` `convert_fragment_top_level` calls
  `try_attach_declarative_shadow(child, context, …)` (:105-106). Rationale comment at `crates/dom/elidex-html-parser/src/lib.rs:143-160`.
- Strict: `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:101-105` declines with
  `unsupported_fragment_construct("fragment-context-declarative-shadow-unsupported")` when
  `is_fragment() && open_elements.len() == 1`; otherwise it attaches only when `open_elements.len() > 1` (:111). The
  attach uses `current_node()` as host (`shadow.rs:152`), so for nested templates strict already implements step 6.
- html5ever 0.39.0 (`Cargo.lock`) is not on the path: elidex rebuilds declarative shadows after the parse from `RcDom`
  (`convert.rs`), so the tolerant defect is elidex's own.
- Whole-document tolerant parsing never attaches declarative shadows (`convert_document` passes
  `ParseFragmentOptions::default()`, `convert.rs:16-25`); that is the separate slot
  `#11-tolerant-document-declarative-shadow-roots`, not this one.

### §0.3 Premise errors in the brief (reported, not edited)

1. "html5ever's own tree builder already follows the current spec": the **outcome** matches (its predicate is
   `open_elems.len() > 1`, `html5ever-0.39.0/src/tree_builder/mod.rs:1500-1505`), but its comment argues from the old
   adjusted-current-node wording, so the match is a coincidence of the stack-depth test. It does not matter here (§0.2).
2. The ledger row's "check whether #531 (html5ever 0.40.1) changes it": `main` still locks 0.39.0, and the html5ever
   version cannot change the tolerant outcome because elidex does not use html5ever's declarative-shadow hook.
3. The brief's list of pinned tests is incomplete. §5 adds: three "flag off" tests that stop discriminating, two "host
   already has a shadow" tests whose path changes, the `elidex-dom-api` round-trip doc claim, the strict
   `unsupported_fragment_construct` doc, and the stale step numbers in `parse_state.rs` / `mod.rs`.

## §1. Shape and verdict

Single PR, about 4 crates touched (`elidex-html-parser`, `elidex-html-parser-strict`, `elidex-script-session`,
`elidex-js` tests; `elidex-dom-api` doc only). **Not edge-dense**: one spec step (step 6 / 7 / 13.1.1 of one algorithm),
a canonical algorithm exists, and the change removes a branch instead of adding one. It still goes through
`/elidex-plan-review` because it changes a ratified surface (memory `feedback_plan-ratified-surface-is-a-design-change`).

## §2. Invariants

- **I1 (context read-only, unconditional)**: no fragment parse mutates `context`, in either backend. This removes the
  "one documented exception" #337 recorded. `parse_html_fragment`'s doc (`lib.rs:39-40`) and
  `parse_fragment_progressive`'s doc ("with `context` itself unmodified") already claim this; they become true.
- **I2 (hostIsRoot)**: a `<template shadowrootmode>` processed while the current node is the topmost element of the stack
  becomes a plain template (with its content fragment), in both backends.
- **I3 (strict-native)**: strict never declines because of a declarative-shadow template. The fragment decline path
  remains for the non-HTML-namespace context only.
- **I4 (nested unchanged)**: a template whose current node is a non-topmost element still attaches to that element (or
  becomes plain when the attach fails), as today.

I1–I4 do not intersect beyond "I2 implies I1", so no pair matrix is needed.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | template start tag, step 6 (declarativeShadowHostElement = current node) | top-level (stack = `[root]`) / nested | strict `shadow.rs` `template_start_tag`; tolerant `convert.rs` `convert_fragment_top_level` / `convert_children` | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | step 7 (hostIsRoot) | true / false | strict: `open_elements.len() > 1`; tolerant: top-level loop never attempts an attach | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | step 13 "Keep" → 1.1 | allow false / hostIsRoot / redirection-map hit (sanitizer: not implemented, no parser sanitizer configuration exists) | strict `allow_declarative_shadow` gate; tolerant `opts.allow_declarative_shadow` gate | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | step 13 "Keep" → 1.8 (host already a shadow host) and 1.9.2 (attach throws) | plain template | `EcsDom::attach_shadow_with_init` failure path (unchanged) | ✓ | yes |
| WHATWG HTML §13.2.4.2 The stack of open elements | adjusted current node | no longer consulted by the template start tag | comments only | ✓ | no |
| WHATWG HTML §13.4 Parsing HTML fragments | root `html` element; stack starts as `[root]` | — | `EcsDom::begin_detached_fragment` (unchanged) | ✓ | no |
| WHATWG DOM §4.9 Interface Element | attach a shadow root, steps 1–2 (valid shadow host name) | `html` not a valid shadow host name | §4.3 document case only (unchanged) | ✓ | no |
| WHATWG HTML §8.5.2 HTML parsing and insertion methods | `setHTMLUnsafe` | Element / ShadowRoot | `crates/script/elidex-js/src/vm/host/dom_inner_html.rs:408`, `:463` (unchanged; tests only) | ✓ | yes |
| WHATWG HTML §8.5.3 HTML serialization methods | `getHTML` round-trip claim | — | `crates/dom/elidex-dom-api/src/element/tree.rs:768-774` doc | ✓ | no |

Numbers and titles: `webref heading html 13.2.6.4.4`; `webref coverage-map html 13.2.6.4.4 html 13.4 html 13.2.4.2 dom
4.9 html 8.5.2`; `webref dfn html getHTML` → §8.5.3; `webref dfn html 'HTML fragment serialization algorithm'` → §13.3;
`webref dfn dom 'valid shadow host name'` → §4.9. Step texts: `webref body html parsing-main-inhead`, `body html
adjusted-current-node`, `body dom concept-attach-a-shadow-root`, `body dom valid-shadow-host-name`, `body html
dom-element-sethtmlunsafe` (2026-10-31). Breadth: K = 2, M = 9 rows.

## §4. Mechanism

### §4.1 Strict

In `template_start_tag` (`shadow.rs`), delete the `is_fragment() && len == 1` decline. The gate becomes "allow, valid
`shadowrootmode`, and `open_elements.len() > 1`" (hostIsRoot ⇔ the stack has one element, since the current node is the
bottom and the topmost is the first). Everything else falls to the ordinary-template arm, which already attaches the
content fragment. Result: `parse_fragment_strict` returns `Ok` with the plain template; no tolerant fallback.
`unsupported_fragment_construct` stays (foreign context); its doc (`tree_builder/mod.rs:480-487`) drops the
DSD-on-context clause.

### §4.2 Tolerant

`convert_fragment_top_level` loses the `context` parameter and its `try_attach_declarative_shadow` call: the top-level
loop converts each child with `convert_node` and appends it to `root`. This loop is the tolerant encoding of hostIsRoot;
its doc says so (step 6/7 with the spec note), replacing the adjusted-current-node text. The call in `lib.rs:162`
drops `context`. `convert_children` is unchanged (nested hosts, I4); its doc drops the "routes the host to the context"
sentence.

Not chosen: delete `convert_fragment_top_level` and call `convert_children(root)`, relying on the attach to the synthetic
`html` root failing (DOM §4.9: `html` is not a valid shadow host name). The outcome is the same, but hostIsRoot would then
be encoded by a host-name list that may grow; the spec checks hostIsRoot before the attach, and so does this plan.

`owner_doc = dom.owner_document(context)` stays: light top-level templates' content fragments are parentless and still
need the context document (Codex PR380 R2). The comment (`lib.rs:152-159`) drops the DSD-on-context clause and keeps
the light-node reason.

### §4.3 Document case (hostIsRoot outside fragments)

Nothing to change. Strict document parsing uses the same `len > 1` gate, so a template whose current node is the
topmost `html` is plain. Tolerant document parsing never attaches (slot `#11-tolerant-document-declarative-shadow-roots`);
when that slot enables it, `convert_children` would try `html` as host, the attach fails at DOM §4.9 step 2 (`html` is
not a valid shadow host name), and §13.2.6.4.4 step 13 → 1.9.2 inserts the template as a regular element, the same
insert as the hostIsRoot branch. The memo hands this observation to that slot (§7); it adds no code here.

## §5. Populations

### §5.1 Tests (enumerated by concept: every test that runs a fragment parse with declarative shadows enabled or that is about the flag)

| Test | File | New assertion |
|---|---|---|
| `fragment_top_level_declarative_shadow_attaches_to_context` | `crates/dom/elidex-html-parser/src/tests.rs:284` | Rename `fragment_top_level_declarative_shadow_is_plain_template`: `added` = one `template`; its content fragment holds the `<p>`; `get_shadow_root(context)` is `None`; `children(context)` empty. |
| `fragment_nested_declarative_shadow_attaches_to_inner_host` | `tests.rs:325` | Behaviour unchanged; comment cites step 6/7 by name, not "step 10". |
| (NEW) `fragment_progressive_top_level_declarative_shadow_strict_native` | `tests.rs` | `parse_fragment_strict` returns `Ok` (no fallback), same tree as the tolerant test. |
| `top_level_declarative_shadow_on_context_is_rejected` | `crates/dom/elidex-html-parser-strict/src/tree_builder/tests_fragment.rs:600` | Rename `top_level_declarative_shadow_is_plain_template`: `Ok`, one `template` root with a content fragment holding `<p>`, `ctx` has no shadow root. |
| `declarative_shadow_opt_off_keeps_template_element` | `tests_fragment.rs:267` | Wrap in `<section>` so the flag is what is tested (top-level is plain either way now). |
| `set_inner_html_default_opts_leaves_template_shadowroot_as_plain_element` | `crates/script/elidex-script-session/src/mutation/html_fragment.rs:309` | Nested `<div><template …>`: the `<div>` has no shadow, the template is its child. |
| `set_html_unsafe_dsd_on_context_preserves_shadow_and_excludes_it_from_removed` (#337 R2 guard) | `html_fragment.rs:335` | Keep the guard on a **pre-attached** shadow: host has `attach_shadow` + light children; top-level template → shadow unchanged, not in `removed_nodes`, `removed_nodes == [old1, old2]`, `added_nodes` = the template. Rename `…_pre_existing_shadow_excluded_from_removed`. |
| `set_inner_html_allow_declarative_shadow_attaches_open_shadow_root` | `html_fragment.rs:392` | Split: top-level → host has no shadow, the template is a light child whose content holds `<p>`; nested `<div><template …>` → the `<div>` gets an open shadow holding `<p>`. |
| `…_attaches_closed_shadow_root` | `html_fragment.rs:435` | Nested; the `<div>` gets a closed shadow. |
| `set_inner_html_invalid_shadowrootmode_leaves_template_as_element` | `html_fragment.rs:458` | Nested, so the invalid value is what is tested. |
| `set_inner_html_declarative_shadow_already_attached_falls_back_silently` | `html_fragment.rs:481` | Keep top-level (pre-existing shadow preserved, template a light child); comment says hostIsRoot. Add a nested case for step 13 → 1.8: `<div><template shadowrootmode=open><p>a</p></template><template shadowrootmode=open><p>b</p></template></div>` → the `<div>`'s shadow holds `a`, the second template is a light child. |
| `set_inner_html_case_insensitive_shadowrootmode` | `html_fragment.rs:509` | Nested. |
| `element_set_html_unsafe_parses_template_shadowrootmode_as_shadow_root` | `crates/script/elidex-js/src/vm/tests/tests_inner_html_mixin.rs:150` | Rename `…_top_level_template_stays_plain`: `host.shadowRoot === null`, `host.firstElementChild.tagName === 'TEMPLATE'`, its `content.firstElementChild.tagName === 'P'`. Add a nested twin that asserts the shadow. |
| `element_set_html_unsafe_closed_mode_hides_shadow_root_from_getter` | `:165` | Nested; assert the template was consumed (`inner.firstChild === null`) and `inner.shadowRoot === null`. |
| `element_set_html_unsafe_on_host_with_existing_shadow_does_not_reattach` | `:176` | Keep; comment says the template is plain by hostIsRoot. |
| `element_set_html_unsafe_template_shadowroot_attrs_parse_as_booleans` | `:193` | Nested. |
| `element_get_html_round_trip_with_set_html_unsafe` | `:641` | Round-trip through a parent: `wrap.setHTMLUnsafe('<div>' + html + '</div>')`, then `wrap.firstChild.shadowRoot` holds `P`. Add an assertion that `dst.setHTMLUnsafe(html)` leaves `dst.shadowRoot === null`. |
| `element_get_html_manual_slot_assignment_round_trip` | `:683` | Same parent wrapping. |
| `element_get_html_emits_shadowrootslotassignment_for_manual_mode` | `:661` | Comment only (`setHTMLUnsafe(getHTML(…))` wording → parent-wrapped form). |
| `element_inner_html_setter_does_not_attach_declarative_shadow` | `:702` | Nested, so the flag is what is tested. |

Unchanged, checked: strict `tests_fragment.rs:282`, `:449` and strict `tree_builder/tests.rs:405`, `:450`, `:482` (nested or document);
`elidex-html-parser` `tests.rs:552` (nested); VM `:611` (nested inside a ShadowRoot).

### §5.2 Comments and docs to rewrite

`convert.rs:46-58`, `:81-93`, `:103-104`; `lib.rs:143-159`; strict `shadow.rs:1-2`, `:83-112`, `:116`, `:143-165`;
`tree_builder/mod.rs:99`, `:480-487`; `tree_builder/parse_state.rs:37`, `:273`, `:325`; `tests_fragment.rs:596-597`;
`html_fragment.rs` test comments (:338, :393-397, :436, :510); `tests_inner_html_mixin.rs:644`, `:664-665`;
`crates/dom/elidex-dom-api/src/element/tree.rs:768-774` (the "`setHTMLUnsafe(host.getHTML(opts))` round-trip" claim
becomes false: the serialization round-trips when placed under a parent; the stale "§13.5" in this block becomes §13.3).
Step citations for this algorithm are written by name (declarativeShadowHostElement, hostIsRoot, the "Keep" branch),
not by number: the sanitizer steps renumbered them once already. Other stale `§4.4.6` / `§13.5` getHTML cites
(`tree.rs:743`, `:758`, `:818`, `dom_inner_html.rs:2`, `:156`, `element_proto.rs:139`) belong to the existing
HTML-renumber sweep and are not edited here.

## §6. Test and coverage plan

`cargo test -p elidex-html-parser -p elidex-html-parser-strict -p elidex-script-session -p elidex-js --all-features`,
then `mise run ci`. Negative control: before the source change, the renamed parser tests fail (red), and after it they
pass. Supported-surface coverage: the §5.1 unit/integration tests; no WPT subset is claimed.

## §7. Out of scope and slots

- **No new slot.** The document-case observation (§4.3) is appended to `#11-tolerant-document-declarative-shadow-roots`
  as a note, not a slot.
- Tolerant "host = `RcDom` parent" versus the spec's "host = current node" (adoption agency / foster parenting can
  differ) is a property of the post-parse rebuild in general, not of this step; not addressed.
- `crates/dom/elidex-dom-api/src/element/tree.rs` has 1076 lines. CLAUDE.md's touch-time rule applies to any touch,
  including this doc-only edit: decide before implementation whether it has a real seam (if yes, a standalone prereq
  split PR first; if no, record why). Open question Q2.

## §8. Alternative: keep DSD-on-context

Keeps: the #337 reading and its tests; `setHTMLUnsafe(getHTML())` on the same element round-trips. Overturns: the
current spec text (step 6 and its note), I1 (the context stays mutable in one case), and strict stays non-native (it must
keep declining, since attaching to the context would make its rollback impossible). It would also need the slot's
backstop text rewritten as a recorded spec deviation with a re-check date.

## §9. Definition of done and slot closure

- §4.1 / §4.2 landed; every §5.1 row has its new assertion; every §5.2 site rewritten.
- `git grep -n -E "DSD-on-context|attaches_to_context|fragment-context-declarative-shadow|strict-fragment-declarative-shadow-on-context" -- crates`
  returns nothing.
- Ledger closure text: "~~`#11-strict-fragment-declarative-shadow-on-context`~~ ✅ CLOSED #NNN `<sha>` — fragment
  top-level `<template shadowrootmode>` is a plain template in both backends (WHATWG HTML §13.2.6.4.4 template start
  tag: host = current node, hostIsRoot → regular template); strict handles it natively (decline removed); #337's
  DSD-on-context reading reversed by user decision <date>; memory `html-fragment-unify-slice2b-plan` §2.4 superseded."
- PM: mark `html-fragment-unify-slice2b-plan.md` §2.4 superseded, and add the §4.3 note to
  `#11-tolerant-document-declarative-shadow-roots`.

## Open questions

- **Q1 (user)**: reverse #337's DSD-on-context (this plan) or keep it (§8)?
- **Q2 (driver)**: does `elidex-dom-api/src/element/tree.rs` (1076 lines) have a real seam for a prereq split?
- Resolved while drafting: `EcsDom::begin_detached_fragment` (`crates/core/elidex-ecs/src/dom/tree/teardown.rs:286`)
  creates the root as `html`, as §4.2's "not chosen" note assumes. Incidental, not in this PR's scope: the same function
  appends the root inside `debug_assert!(self.append_child(document, root), …)` (`teardown.rs:296-299`), so in release
  builds the root is never appended to the throwaway document. Reported to the driver for routing.

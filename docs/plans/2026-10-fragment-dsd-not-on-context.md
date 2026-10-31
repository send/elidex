# Fragment top-level `<template shadowrootmode>` stays a plain template (per-PR plan-memo)

Slot: `#11-strict-fragment-declarative-shadow-on-context` (ledger row "SCOPE INVERTED 2026-10-24"). Base: `origin/main`
`e44ffce6`. Owner lane: L3 DOM/form.

**Rev 1** (2026-10-31) — first draft. **Status: PR 1 blocked on Q1 (§0.1); PR 0b is not.** This memo plans the
spec-current option; the alternative is in §8.

**Rev 2** (2026-10-31) — driver decisions: `tree.rs` split as a prereq (now PR 0a, §1.1); the `begin_detached_fragment`
release-build defect is fixed (now PR 0b, §1.2).

**Rev 3** (2026-10-31) — plan-review R1 (0 CRIT / 5 IMP / 14 MIN): F1 both-sided ratification record (§0.1); F2 the
leak, not the owner document (§1.2); F3 `#11-shadow-declarative-reuse` measured and re-scoped (§7); F4
`#11-tolerant-document-declarative-shadow-roots` disposition (§7); F5 the tolerant host divergence measured as reachable
(§7). M1–M14 applied in place; citations are by algorithm name, not step number.

## §0. Frame, premises, decisions

### §0.1 The decision: this reverses a ratified reading

PR #337 (squash `cb4ab7e7`, 2026-06-13) shipped **DSD-on-context**: `el.setHTMLUnsafe('<template
shadowrootmode=open>…')` attaches a shadow root to `el`, the fragment-parse context. This memo plans the opposite
outcome. The PM routed the reversal to the user (PM 87); the memo does not decide it.

| Side | Text it rests on | Outcome |
|---|---|---|
| #337 implementation (2026-06) | Spec text of the time: the declarative branch keyed on the **adjusted current node** being "not the topmost element", and used it as the host. §13.2.4.2 defines the adjusted current node as "the parser's fragment context element if that element is non-null and the stack of open elements has only one element in it (fragment case)". A top-level template sees stack `[root]`, so the host was the context. | Shadow on context; context mutated (the one exception to the read-only-context contract); strict declines and falls back to tolerant. |
| Current WHATWG HTML §13.2.6.4.4 (`webref body html parsing-main-inhead`, 2026-10-31) | Template start tag: "Let declarativeShadowHostElement be the current node", with the note "This is not the adjusted current node, so that in the fragment case a top-level template element never attaches a shadow root to the parser's fragment context element." hostIsRoot is true when that element is the topmost element of the stack. In the "Keep" branch, hostIsRoot (or allow declarative shadow roots false, or a redirection-map entry for the host) inserts the template as a regular element and returns. | Plain `<template>` among the returned nodes; the context is never mutated. |

Precedents on each side (memory, read 2026-10-31):

- **For a plain template**: the #337 plan's own §7 Risk 1 (`html-fragment-unify-slice2b-plan.md:162`) recorded the
  plain template as the intended *correctness fix* and as the recorded intent of `project_parser-strict-tolerant-deployment`
  §NEXT; its §E test 3 (`:170`) rewrote the old attach test to a plain template. The PM's 2026-10-24 ledger entry
  (`project_open-defer-slots.md:697`) calls the current behaviour "a correctness defect in BOTH backends".
- **For DSD-on-context**: the #337 implementation inverted the plan when the VM `element_get_html_*` round-trip tests
  failed under `mise run ci` (`html-fragment-unify-slice2b-plan.md` §2.4, `:86-92`; `:12`, `:36`, `:155`). The landings
  ledger (`m4-12-landings-ledger.md:109`) and `project_parser-strict-tolerant-deployment.md:20-21` record that inversion
  and call the plan's reading a "spec misread"; `html-side-defer-sweep-2026-06-14.md:35` restates it.
- **No record of an explicit user approval of DSD-on-context was found** (grep over the memory directory for the
  reading with "user / 承認 / approv / ratif"). The inversion came from a test failure during implementation, under the
  then spec text.

The spec note names this exact case, so the outcome is not spec-silent; no browser measurement is needed to choose it.

### §0.2 Premises re-measured (at `e44ffce6`)

- Tolerant: `crates/dom/elidex-html-parser/src/convert.rs:94` `convert_fragment_top_level` calls
  `try_attach_declarative_shadow(child, context, …)` (:105-106). Rationale comment `crates/dom/elidex-html-parser/src/lib.rs:143-160`.
- Strict: `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:101-105` declines with
  `unsupported_fragment_construct` when `is_fragment() && open_elements.len() == 1`; otherwise it attaches only when
  `open_elements.len() > 1` (:111), using `current_node()` as host (`shadow.rs:152`).
- html5ever is not on the path. html5ever 0.39.0 calls `TreeSink::attach_declarative_shadow`; `markup5ever_rcdom`
  0.39.0 does not override it, and the trait default in markup5ever 0.39.0 returns `false`
  (`markup5ever-0.39.0/interface/tree_builder.rs:279-285`), so html5ever inserts every template into the `RcDom` as a
  regular element and elidex rebuilds declarative shadows afterwards (`convert.rs`). The tolerant defect is elidex's.
  #555 (html5ever 0.40.1, open) does not change this.
- Whole-document tolerant parsing never attaches declarative shadows (`convert_document`, `convert.rs:16-25`, passes
  `ParseFragmentOptions::default()`): slot `#11-tolerant-document-declarative-shadow-roots` (§7).

### §0.3 Premise errors in the brief (reported, not edited)

1. "html5ever already follows the current spec": the outcome matches (`open_elems.len() > 1`,
   `html5ever-0.39.0/src/tree_builder/mod.rs:1500-1505`), but its comment argues from the old wording; and it is unused.
2. The ledger's "check #531 (html5ever 0.40.1)": `main` locks 0.39.0; the bump is #555, irrelevant per §0.2.
3. The pinned-test list was incomplete; §5 enumerates by concept.

## §1. Shape and verdict

Three PRs. **PR 0b** (§1.2, `elidex-ecs` only) is independent of Q1 and goes now. **PR 0a** (§1.1, pure move) and
**PR 1** (this plan: `elidex-html-parser`, `elidex-html-parser-strict`, `elidex-script-session`, `elidex-js` tests,
`elidex-dom-api` doc) wait for Q1. **Not edge-dense**: one branch of one algorithm (the template start tag's host and
hostIsRoot), a canonical algorithm exists, and the change removes a branch. It goes through `/elidex-plan-review`
because it changes a ratified surface (memory `feedback_plan-ratified-surface-is-a-design-change`).

### §1.1 PR 0a — split `elidex-dom-api/src/element/tree.rs` (prereq, pure move; only if Q1 picks this plan)

`crates/dom/elidex-dom-api/src/element/tree.rs` has 1076 lines and PR 1 edits its doc (§5.2), so CLAUDE.md's
touch-time rule applies. Seam: `:560-1003` (the `innerHTML` section through `serialize_node`) versus the tree-mutation
handlers (`:1-558`) and attribute-name validation (`:1004-1033`).

- **Boundary**: move `SetInnerHtml`, `InsertAdjacentHtml`, `GetInnerHtml`, `RAW_TEXT_ELEMENTS`, `VOID_ELEMENTS`,
  `SerializeOptions` and every `serialize_*` / `emit_*shadow_root*` / `shadow_root_should_emit` /
  `serialization_children_root` / `is_safe_attr_name` item into
  `crates/dom/elidex-dom-api/src/element/markup.rs` (NEW) (about 445 lines; `tree.rs` keeps about 630).
- **Why the setters go with the getters**: the three handlers are the HTML §8.5 "DOM parsing and serialization APIs"
  surface. The setters share no helper with `tree.rs` (they only record `Mutation::SetInnerHtml` /
  `Mutation::InsertAdjacentHtml`; the parse runs in `elidex-script-session`'s `mutation/html_fragment.rs`, whose module
  doc names the same family) and do not call `resolve_adjacent_site` / `hierarchy_error`. Splitting serialization alone
  would put the innerHTML getter and setter in different files.
- **Mechanics**: `element/mod.rs` adds `mod markup;` and re-exports from it (public paths through `element::*`
  unchanged); `tests_tree.rs:651` `super::tree::serialize_inner_html` → `super::markup::serialize_inner_html` (NEW).
  The only text addition is the new file's `//!` header, citing HTML §8.5.2 HTML parsing and insertion methods, §8.5.3
  HTML serialization methods, §8.5.4 The innerHTML property, §8.5.6 The insertAdjacentHTML() method and §13.3
  Serializing HTML fragments (`webref heading html 8.5.2` … `8.5.6`, `13.3`). Review = `git diff --color-moved`.

### §1.2 PR 0b — `begin_detached_fragment` appends root only in debug builds (standalone, now)

`crates/core/elidex-ecs/src/dom/tree/teardown.rs:296-299` runs `self.append_child(document, root)` inside
`debug_assert!`. `debug_assert!` does not evaluate its arguments without `debug_assertions`, so in release builds the
synthetic root is never appended to the throwaway document, against HTML §13.4 ("Append root to document", right after
root is created). Consequence: on a strict parse error the rollback `despawn_subtree(document)`
(`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:229`) does not reach root, so **root and the partial
subtree leak** in release builds. (The success path is unaffected: `finish_detached_fragment` destroys root and
document explicitly.)

- Fix: `let ok = self.append_child(document, root); debug_assert!(ok, …);` (the `convert.rs:76` idiom).
- Sweep: every `debug_assert*!` in `crates/` whose argument calls a mutating method
  (`git grep -n -A2 "debug_assert" -- crates`, filtered for `append_child|insert_|remove_|attach_|set_*(|push(|despawn|destroy_`)
  — this is the only one.
- Tests: `begin_detached_fragment_root_is_child_of_document` (NEW, `elidex-ecs`) asserts `get_parent(root) ==
  Some(document)`. The dev profile cannot observe the defect (the append runs inside the assertion), and CI runs only
  the dev profile. Release-profile control, manual and one-off, recorded in the PR body: on the unfixed code,
  `cargo test --release -p elidex-html-parser-strict no_leak` and the parent test fail (the existing
  `world().len()` pristine-after-rollback assertions in `parse_error_rolls_back_with_no_leak_and_pristine_context`
  and `rollback_despawns_shadow_root_with_no_leak`, `tests_fragment.rs:188`, `:449`, observe the leak); after
  the fix both pass. The fix stands on the language rule, not on the tests.
- No defer slot. The `clippy::debug_assert_with_mut_call` (NEW) idea (nursery lint) is tooling: handed to the PM as a
  CI/tools-lane suggestion, not a platform slot.

## §2. Invariants

- **I1 (context read-only, unconditional)**: no fragment parse mutates `context`, in either backend. `parse_html_fragment`'s
  doc (`lib.rs:39-40`) and `parse_fragment_progressive`'s doc already claim it; they become true.
- **I2 (hostIsRoot)**: a `<template shadowrootmode>` whose current node is the topmost stack element becomes a plain
  template with its content fragment, in both backends.
- **I3 (strict-native)**: strict never declines because of a declarative template; the fragment decline remains for a
  non-HTML-namespace context only.
- **I4 (nested unchanged)**: a template whose current node is not topmost attaches to it, or is plain when the attach
  fails, as today (tolerant caveat in §7).

I1–I4 intersect only as "I2 implies I1", so no pair matrix.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | template start tag: declarativeShadowHostElement = current node | top-level (stack `[root]`) / nested | strict `shadow.rs` `template_start_tag`; tolerant `convert.rs` `convert_fragment_top_level` / `convert_children` | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | template start tag: hostIsRoot | true / false | strict `open_elements.len() > 1`; tolerant top-level loop never attempts an attach | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | "Keep" branch, regular-template condition | allow false / hostIsRoot / redirection-map entry for the host. §13.4 sets the map entry for root on every fragment parse, so a top-level template meets two conditions; elidex has no redirection map (no sanitizer) and encodes both by hostIsRoot | strict / tolerant `allow_declarative_shadow` gates + hostIsRoot | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | "Keep" branch: host already a shadow host; attach throws | plain template | `EcsDom::attach_shadow_with_init` error path (unchanged) | ✓ | yes |
| WHATWG HTML §13.2.4.2 The stack of open elements | adjusted current node | no longer consulted by the template start tag | comments only | ✓ | no |
| WHATWG HTML §13.4 Parsing HTML fragments | root `html` element; "Append root to document"; stack `[root]`; redirection map entry for root | — | `EcsDom::begin_detached_fragment` (changed in PR 0b) | ✓ | no |
| WHATWG DOM §4.9 Interface Element | attach a shadow root: valid shadow host name; shadow-host reuse | `html` invalid; reuse not implemented (§7) | `attach_shadow_with_init` (unchanged) | ✓ | no |
| WHATWG HTML §8.5.2 HTML parsing and insertion methods | `setHTMLUnsafe` | Element / ShadowRoot | `crates/script/elidex-js/src/vm/host/dom_inner_html.rs:408`, `:463` (tests only) | ✓ | yes |
| WHATWG HTML §8.5.3 HTML serialization methods | `getHTML` round-trip claim | — | `crates/dom/elidex-dom-api/src/element/markup.rs` (NEW) (PR 0a; today `tree.rs:768-774`) doc | ✓ | no |

Numbers and titles: `webref heading html 13.2.6.4.4`, `8.5.2`, `8.5.4`, `8.5.6`, `13.3`; `webref coverage-map html
13.2.6.4.4 html 13.4 html 13.2.4.2 dom 4.9 html 8.5.2`; `webref dfn html getHTML` → §8.5.3; `webref dfn html 'allow
declarative shadow roots'` → §13.2.4.5; `webref dfn dom 'valid shadow host name'` → §4.9. Prose: `webref body html
parsing-main-inhead`, `html-fragment-parsing-algorithm`, `adjusted-current-node`, `dom-element-sethtmlunsafe`; `body dom
concept-attach-a-shadow-root`, `valid-shadow-host-name`, `dom-element-attachshadow` (2026-10-31). Breadth: K = 2, M = 9.

## §4. Mechanism (PR 1)

### §4.1 Strict

In `template_start_tag` (`shadow.rs`), delete the `is_fragment() && len == 1` decline. The gate becomes "allow, valid
`shadowrootmode`, and `open_elements.len() > 1`" (hostIsRoot ⇔ one stack element, since the current node is the
bottom and the topmost is the first). Everything else takes the ordinary-template arm, which attaches the content
fragment. `parse_fragment_strict` returns `Ok`; no fallback. `unsupported_fragment_construct` stays (foreign context);
its doc (`tree_builder/mod.rs:480-487`) drops the DSD-on-context clause.

### §4.2 Tolerant

`convert_fragment_top_level` loses `context` and its `try_attach_declarative_shadow` call: the top-level loop converts
each child with `convert_node` and appends it to `root`. The loop is the tolerant encoding of hostIsRoot; its doc
quotes the spec note. `lib.rs:162` drops `context`. `convert_children` is unchanged; its doc drops the context-routing
sentence.

Not chosen: call `convert_children(root)` and rely on the attach to `html` failing (DOM §4.9). Same outcome, but
hostIsRoot would be encoded by a host-name list that may grow; the spec checks hostIsRoot before any attach.

`owner_doc = dom.owner_document(context)` stays: light top-level templates' content fragments are parentless and still
need the context document (Codex PR380 R2). The comment (`lib.rs:152-159`) keeps that reason and drops the
DSD-on-context clause.

### §4.3 Document case (hostIsRoot outside fragments)

Nothing to change. Strict document parsing uses the same `len > 1` gate. Tolerant document parsing never attaches
(§7); if it is enabled later, `convert_children` would try `html` as host, the attach fails on the valid shadow host
name check, and the "Keep" branch's attach-throws catch inserts the template as a regular element — the same insert as
the hostIsRoot branch.

### §4.4 ECS-native check

No component, system, query or registry is added or removed. The change deletes one branch per backend; the shadow
attach remains `EcsDom::attach_shadow_with_init` (the `ShadowHost` / `ShadowRoot` components). `template_content_targets`
is per-parse builder state, not per-entity state.

## §5. Populations

### §5.1 Tests (by concept: every fragment parse with declarative shadows on, or about the flag)

| Test | File | New assertion |
|---|---|---|
| `fragment_top_level_declarative_shadow_attaches_to_context` | `crates/dom/elidex-html-parser/src/tests.rs:284` | Rename `…_is_plain_template`: `added` = one `template` whose content holds `<p>`; **discriminator** `get_shadow_root(context)` is `None`. `children(context)` empty is kept but non-discriminating (true before and after). |
| `fragment_nested_declarative_shadow_attaches_to_inner_host` | `tests.rs:325` | Unchanged behaviour; comment by name. |
| (NEW) `fragment_progressive_top_level_declarative_shadow_strict_native` | `tests.rs` | `parse_fragment_strict` returns `Ok`, same tree as the tolerant test. |
| (NEW) `fragment_dsd_parity_template_in_template` | `tests.rs` | `<template><template shadowrootmode=open><p>x</p></template></template>`: strict and tolerant give the same tree (outer plain; inner plain inside outer content, host `template` is invalid). |
| (NEW) `fragment_dsd_parity_nested_host_in_top_level_template` | `tests.rs` | `<template><div><template shadowrootmode=open><p>x</p></template></div></template>`: both backends give the `<div>` inside the outer content an open shadow holding `<p>`. |
| `top_level_declarative_shadow_on_context_is_rejected` | `crates/dom/elidex-html-parser-strict/src/tree_builder/tests_fragment.rs:600` | Rename `top_level_declarative_shadow_is_plain_template`: `Ok`, one `template` root with content `<p>`; discriminator: `ctx` has no shadow root. |
| `declarative_shadow_opt_off_keeps_template_element` | `tests_fragment.rs:267` | Wrap in `<section>` so the flag is what is tested. |
| `set_inner_html_default_opts_leaves_template_shadowroot_as_plain_element` | `crates/script/elidex-script-session/src/mutation/html_fragment.rs:309` | Nested `<div><template …>`: no shadow on the `<div>`. |
| `set_html_unsafe_dsd_on_context_preserves_shadow_and_excludes_it_from_removed` (#337 R2 guard) | `html_fragment.rs:335` | Pre-attached shadow + light children; top-level template → shadow unchanged, not in `removed_nodes`, `removed_nodes == [old1, old2]`, `added_nodes` = the template. Rename `…_pre_existing_shadow_excluded_from_removed`. |
| `set_inner_html_allow_declarative_shadow_attaches_open_shadow_root` | `html_fragment.rs:392` | Split: top-level → no shadow on host, template is a light child with content `<p>`; nested → the `<div>` gets an open shadow. |
| `…_attaches_closed_shadow_root` | `html_fragment.rs:435` | Nested; closed shadow on the `<div>`. |
| `set_inner_html_invalid_shadowrootmode_leaves_template_as_element` | `html_fragment.rs:458` | Nested. |
| `set_inner_html_declarative_shadow_already_attached_falls_back_silently` | `html_fragment.rs:481` | Keep top-level (comment: hostIsRoot). Add nested two-template case for the shadow-host condition: `<div>` shadow holds `a`, second template is a light child. |
| `set_inner_html_case_insensitive_shadowrootmode` | `html_fragment.rs:509` | Nested. |
| `element_set_html_unsafe_parses_template_shadowrootmode_as_shadow_root` | `crates/script/elidex-js/src/vm/tests/tests_inner_html_mixin.rs:150` | Rename `…_top_level_template_stays_plain`: `host.shadowRoot === null`, first child `TEMPLATE` whose `content.firstElementChild` is `P`. Add a nested twin asserting the shadow. |
| `element_set_html_unsafe_closed_mode_hides_shadow_root_from_getter` | `:165` | Nested; template consumed (`inner.firstChild === null`) and `inner.shadowRoot === null`. |
| `element_set_html_unsafe_on_host_with_existing_shadow_does_not_reattach` | `:176` | Keep; comment: hostIsRoot. |
| `element_set_html_unsafe_template_shadowroot_attrs_parse_as_booleans` | `:193` | Nested. |
| `element_get_html_round_trip_with_set_html_unsafe` | `:641` | `wrap.setHTMLUnsafe('<div>' + html + '</div>')` → `wrap.firstChild.shadowRoot` holds `P`; plus `dst.setHTMLUnsafe(html)` leaves `dst.shadowRoot === null`. |
| `element_get_html_manual_slot_assignment_round_trip` | `:683` | Same wrapping. |
| `element_get_html_emits_shadowrootslotassignment_for_manual_mode` | `:661` | Comment only. |
| `element_inner_html_setter_does_not_attach_declarative_shadow` | `:702` | Nested. |

Unchanged, checked: strict `tests_fragment.rs:282`, `:449`, strict `tree_builder/tests.rs:405`, `:450`, `:482`;
`elidex-html-parser` `tests.rs:552`; VM `:611` (all nested or document).

### §5.2 Comments and docs to rewrite

Audit method (2026-10-31, `e44ffce6`): P1 `git grep -n -E "13\.2\.6\.4\.4 steps? [0-9]" -- crates` = 12 hits, all
in scope; P2 `git grep -n -E "13\.4 step 6|carries step 6" -- crates` = 3, all in scope; P3 `git grep -n -P
"[Ss]teps? (4-5|6-7|6-9|9|10)(\.\d+)*(/\d+(\.\d+)*)?(?!\d)"` over strict `shadow.rs`, `tests_fragment.rs`,
`tree_builder/mod.rs` = 20, of which 14 are in scope and 6 are §13.4 tokenizer-state cites (still correct, or the
sweep's); plus two line-wrapped cites found by reading (`mod.rs:87-88`, `shadow.rs:69-70`) and the DSD-on-context
concept grep of §9.

Sites: `convert.rs:46-58`, `:81-93`, `:103-104`; `lib.rs:143-159`; strict `shadow.rs:1-2`, `:43`, `:69-70`, `:83-112`,
`:116`, `:143-165`; `tree_builder/mod.rs:87-91` (the flag is HTML §13.2.4.5 Other parsing state flags), `:99`,
`:480-487`; strict `lib.rs:76`; `tree_builder/parse_state.rs:37`, `:273`, `:325`; `tests_fragment.rs:264`, `:596-607`;
`html_fragment.rs` test comments (`:338`, `:393-397`, `:436`, `:510`); `tests_inner_html_mixin.rs:644`, `:664-665`;
`crates/dom/elidex-dom-api/src/element/markup.rs` (NEW) (PR 0a; today `tree.rs:768-774`): the
"`setHTMLUnsafe(host.getHTML(opts))` round-trip" claim becomes "round-trips when placed under a parent", and the stale
"§13.5" at today's `tree.rs:755` and `:768` becomes §13.3 Serializing HTML fragments. Prescribed wording names the
condition "hostIsRoot (the fragment root also has a redirection-map entry)". Cites are by name
(declarativeShadowHostElement, hostIsRoot, the "Keep" branch). Left to the HTML-renumber sweep: `§4.4.6` getHTML cites
(today `tree.rs:743`, `:758`, `:818` → `markup.rs` after PR 0a; `dom_inner_html.rs:2`, `:156`; `element_proto.rs:139`)
and §13.4 step numbers outside the flag (`tests_fragment.rs:6`, strict `lib.rs:76`'s other numbers).

## §6. Test and coverage plan

`cargo test -p elidex-html-parser -p elidex-html-parser-strict -p elidex-script-session -p elidex-js --all-features`,
then `mise run ci`. Negative control: the renamed and new parser tests fail before §4.1/§4.2 and pass after (the two
parity tests pass before too only if both backends already agree; record which). PR 0b's controls are in §1.2. No WPT
subset is claimed.

## §7. Out of scope and slots (no new slot created here)

- **`#11-shadow-declarative-reuse`** (triggered: PR 1 touches declarative attach in both tiers). Measured: the parser
  half is spec-correct — the "Keep" branch's shadow-host condition yields a plain template, and both tiers do that on
  `AlreadyAttached` (strict's ordinary-template fallback; tolerant `try_attach_declarative_shadow` returns false). The
  slot text's "strict skips the template silently" is wrong. The DOM half is **not implemented**: DOM §4.9 attach a
  shadow root, "If element is a shadow host", reuses a *declarative* shadow root of the same mode (remove its children,
  set declarative to false, return), and `attachShadow` then returns it. elidex's `ShadowRoot` component has no
  `declarative` field and the template start tag never sets it; `attach_shadow_with_init` returns `AlreadyAttached`
  unconditionally (`crates/core/elidex-ecs/src/dom/shadow.rs:184-186`), which the VM maps to an exception. Proposed
  re-scope for the PM: "**Defect**: `host.attachShadow({mode})` on a host whose shadow root came from a declarative
  template throws, where DOM §4.9 empties and returns that shadow root. Missing: a `declarative` flag on `ShadowRoot`, set by both parser tiers and cleared on reuse. **Trigger**: a PR
  touching `attach_shadow_with_init` or `attachShadow`. **Re-eval** 2026-12-31." Not fixed in PR 1: a different
  algorithm (DOM attach), a new component field, a VM surface.
- **`#11-tolerant-document-declarative-shadow-roots`** (triggered). Disposition: extend with cause — PR 1 does not touch
  the document parse path, and enabling DSD for tolerant document parsing is the strict/tolerant auto-dispatch decision.
  Notes for the slot: (a) §4.3's observation; (b) its "Unchecked" item, partly answered: innerHTML leaves the flag off,
  pinned by `element_inner_html_setter_does_not_attach_declarative_shadow`; DOMParser and `parseHTMLUnsafe` remain
  unchecked; (c) suggestion to the PM: narrow the trigger "a PR that touches declarative shadow roots in either tier"
  to the document path, since every fragment DSD PR fires it.
- **Tolerant host = `RcDom` parent, not the current node: reachable (measured).** Throwaway test (not committed), body
  context, declarative shadows on, input `<b><div><template shadowrootmode="open"><span>x</span></template></b>`:
  tolerant returns `b`, `div > b > template` (plain; no shadow anywhere); strict declines (misnested end tag). Per
  spec the current node at the template start tag is the `<div>`, so the `<div>` gets a shadow holding `<span>` and the
  template is never inserted. Cause: with `RcDom` the template is inserted into the `<div>`; `</b>` then runs the
  adoption agency algorithm, which moves the `<div>`'s children into a new `<b>`; elidex's post-parse rebuild then
  tries `<b>` (not a valid shadow host). Proposed slot text for the PM:
  "`#11-tolerant-dsd-host-from-tree-builder` — **Defect**: the tolerant backend picks the declarative-shadow host from
  the finished `RcDom` parent instead of the current node at the template start tag, so markup that the adoption
  agency algorithm reparents loses its shadow root (repro above). Fix direction: drive declarative attach from the tree
  builder (a `TreeSink` that implements `attach_declarative_shadow`) instead of rebuilding after the parse. **Owner**:
  parser. **Trigger**: enabling tolerant document DSD, or a PR that changes `convert.rs`'s declarative path. **Re-eval**
  2026-12-31." Not fixed in PR 1: it replaces the tolerant conversion design, not one branch.

## §8. Alternative: keep DSD-on-context

Keeps: the #337 behaviour and its tests; `setHTMLUnsafe(getHTML())` on the same element round-trips. Also keeps a
defect: the DSD-on-context shadow content is built while the mutation dispatcher is taken, so it produces no
MutationRecord and no custom element reactions. Overturns: the current spec text and its note, I1, and strict stays
non-native (it must keep declining; attaching to the context would make its rollback impossible). The slot would need
rewriting as a recorded spec deviation with a re-check date.

## §9. Definition of done and slot closure

- PR 0b merged (independent). PR 0a merged before PR 1. §4.1 / §4.2 landed; every §5.1 row asserted; every §5.2 site
  rewritten.
- These return nothing: `git grep -n -E "DSD-on-context|attaches_to_context|fragment-context-declarative-shadow|strict-fragment-declarative-shadow-on-context" -- crates`;
  `git grep -n -E "13\.2\.6\.4\.4 steps? [0-9]|13\.4 step 6|carries step 6" -- crates`; `git grep -n -P
  "[Ss]teps? (4-5|6-7|6-9|9|10\.\d+)" -- crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs`.
- Ledger closure text: "~~`#11-strict-fragment-declarative-shadow-on-context`~~ ✅ CLOSED #NNN `<sha>` — fragment
  top-level `<template shadowrootmode>` is a plain template in both backends (WHATWG HTML §13.2.6.4.4 template start
  tag: host = current node; hostIsRoot → regular template); strict handles it natively; #337's DSD-on-context
  reversed by user decision <date>."
- PM memory sweep: mark superseded `html-fragment-unify-slice2b-plan.md` §2.4, `project_parser-strict-tolerant-deployment.md:18`,
  `:20`, `html-side-defer-sweep-2026-06-14.md:35`; re-scope `#11-shadow-declarative-reuse` and add the notes to
  `#11-tolerant-document-declarative-shadow-roots` (§7); decide registration of `#11-tolerant-dsd-host-from-tree-builder`;
  tell the HTML-renumber sweep its `tree.rs` paths move to `markup.rs` after PR 0a; the clippy lint suggestion (§1.2)
  goes to the CI/tools lane.

## Open questions

- **Q1 (user, via PM)**: reverse #337's DSD-on-context (this plan) or keep it (§8)? PR 0b does not wait for it.
- Resolved: Rev 2's Q2 (PR 0b is independent); Rev 2's Q3 (handed off, §1.2); `begin_detached_fragment`'s root is `html`.

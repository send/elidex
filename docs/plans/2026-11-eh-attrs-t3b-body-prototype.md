# T3-b `EH-attrs-at-creation-T3-b` — `HTMLBodyElement` is an HTML `body`'s interface (per-PR plan-memo)

*Provenance: split from the T3-ab memo (`docs/plans/2026-11-eh-attrs-t3-applicability-and-body-prototype.md`, rev 1.1;
deleted by the same plan PR) by PM decision 767, following plan-review R3 wave 1's Axis 3 split verdict (finding W1-1;
PM 227). Its other half is the T3-a memo (`docs/plans/2026-11-eh-attrs-t3a-applicability.md`).*

Nested PR **T3-b** ("`body`'s interface") of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, "the umbrella"; its in-place edits for this PR are carried by
the T3 memo's *Rev 8.<t>* entry, since the four memos land in one plan PR). It is a prerequisite of **T3**
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, "the T3 memo", rev 6.3). It carries one decision, with its id
kept from the T3 memo rev 5: **D-T11** (the VM gives `HTMLBodyElement.prototype` to an HTML `body` only). The ids of
premises, rows and cells (H-A*, P-A*, B*, FN-*, T-*) are kept from the T3-ab memo, so an id names the same thing in
the T3 memo, in the T3-a memo and here; ids this memo does not carry (H-A1 to H-A3, H-A6 to H-A9, B1 to B3, B5 to B9,
the AP rows, T-21, the T-A and U cells) are the T3-a memo's; H-B* are new here. Rev 1.1 (2026-11-18).

**Review numbering.** This memo's own plan-review rounds are numbered from **R1** (its first full five-axis review).
"T3's R1" and "T3's R2" are the reviews of the T3 memo rev 4.8.2 and rev 5; "R3 wave 1" is the review of the T3-ab
memo rev 1.1 (with the T3-pre memo rev 2.1) whose findings this memo folds (W1-*; listed in the Changelog).
**This memo's R1 is "R3 wave 2" in the PM's records** (PM decision 767 (3): wave 2 reviews T3 rev 6 and the two new
memos with five axes); its findings are folded in rev 1.1 (Changelog).

**Owner: Layout lane (L2)**, plan and implementation (PM decisions 741 (3), 767). No surface of another lane is
changed.

**Placement and order (PM decisions 741 (3), 767; measured in §8.1).** Row **T3-b** of the umbrella's §1.2. One edge:
**x16** T3-b → T3 (umbrella §1.3). **No edge with T3-a, L3's PR (is), PR (ig), PR (io), L3's tree-model program or
T3-pre**: T3-b edits two functions of `vm/host/` that none of them edits, and reads no creation event, dispatcher
window, walker or tree link (§8.1); it cites no L3 plan at a snapshot, and the program's re-pin of L3's unratified
plans is the umbrella §8.5. Its prerequisites are all on main, so its implementation branch can be cut from
main today, and it lands in parallel with every unit between C1 and T3. The user's order (PM decision 153: N1 → C0 →
C1 → T3 → slice b) is kept: T3-b is a part of T3's scope landing earlier, between C1 and T3.

**Base.** main when T3-b's implementation branch is cut. Every code coordinate below is a `cee74989` coordinate (#626,
the T3 memo's frame) and is re-run at the base (§9 item 1). main is past `cee74989` (`653d7c24`, #636, on 2026-11-18); of the
files this memo cites, one differs there, `crates/script/elidex-js/src/vm/host_data/mod.rs` (3 lines deleted; not a
cited line), and none moved (H-A10).

**Scope.**
- **D-T11.** `HTMLBodyElement.prototype` and its receiver check require the HTML namespace as well as the local name
  `body`, through one statement of that fact read by both (`is_html_body_element`, NEW, in `vm/host/`). This closes T3
  rev 5's residual R-D1.
- Cell T-19 (id kept from the T3 memo rev 5), rewritten (§6.1; R3 wave 1 W1-2).

**Not in scope.** Which names are event handler content attributes of an element (D-T8, the T3-a memo); target
determination, every facet (T3); the HTML-namespace test of step 1's class in `is_body_or_frameset` (T3 memo D-T5);
the interfaces of foreign elements other than `body` (§8.2 item 1); `document.createElementNS`
(`#11-vm-create-element-ns-missing`, L3); the `HTMLFrameSetElement` interface (`#11-htmlframesetelement-interface`,
compat, PM decision 130; registered at this PR's landing, the first that cites it, §8.3 item 2); a global
`HTMLBodyElement` interface object (the VM has none, nor any per-tag HTML interface object but `HTMLElement`, H-B2;
not this program's, §10 Q-B1).

**Changelog.** *Clerical* (2026-11-18; no rev bump): clerical: pointer to `#11-html-element-type-tests-without-namespace` (PM 780).
*Clerical before plan PR, PM 778/779* (2026-11-18; no rev bump): Placement points to the umbrella §8.5.
*Clerical, with T3 memo rev 6.4* (2026-11-18; no revision bump, no design change): Q-A2 points at
§1.3 for K and M instead of restating them (F2), and §8.3 item 2's registration text leaves T3's effect to T3's
append, the T3 memo §8.3 item 9 (F7).
*Rev 1.1* (2026-11-18; this memo's R1 = R3 wave 2: CRIT 0 / IMP 0 / MIN 13; its Axes 1 and 2 ruled the
placement of `is_html_body_element` acceptable, the wave-2a tally). No decision changes. **Spec (Axis 4):** an SVG-namespace `body`
is "not an `HTMLBodyElement`", and its interface is DOM §4.5's default `Element`, not `SVGElement` (SVG2 §4.4.1 states
`SVGElement` for elements of the SVG language; rev 1 said `SVGElement`); a MathML `body` is an unknown MathML element
with `MathMLElement` (MathML Core §2.1, §2.2.3); no outcome changes (H-B1, D-T11, §2.1, §3). The `TypeError` of a
wrong receiver cites Web IDL §3.7.6 Attributes (§2.1, §3, §4.4). HTML §3.2.2 step 4 matches the local name exactly,
and the VM's tag test is ASCII-case-insensitive: the deviation is noted with its home (§4.4, §8.3 item 1).
**Ground (Axis 2):** D-T11's *Placement* (2) separates what PM decision 133 decided (the scope of unification with
`document.body`) from what the T3 memo's D-T5 derives from it (no `EcsDom` symbol). **Sibling sites (Axes 1, 2, 3):**
the seven tag-only `body` tests in four crates are listed with their homes; the two that had none
(`elidex-style`'s `find_body_child`, `elidex-dom-api`'s `offsetParent` fallback) join §8.2 item 1's defect text,
which also says that `is_html_body_element`'s body migrates under the general fix; the slot-less "later convergence"
wording is removed (D-T11, §7, §8.2 item 1). **Records (Axis 5):** `#11-htmlframesetelement-interface` is registered
at this PR's landing, the first that cites it (§8.3 item 2; the text moved from the T3 memo §8.3 item 9, restated at
`cee74989` with a Re-eval date); §8.2 item 1's "no ledger row" is re-measured and names the PM 768 siblings; Q-B1
names its likely home and H-B2 is widened (no per-tag HTML interface object exists but `HTMLElement`); §8.1's
*Result* and Q-A3 point to the T3-pre memo DoD item 7 and §8.3 item 3 and no longer allow a landing after io; the
review numbering maps R1 to R3 wave 2. **Checks:** DoD item 4's and H-A11's count of direct `EcsDom` calls also
covers the rebinding form, with positive controls; §1.3 is written as three axes (trigger (a), base case (c)).
*Rev 1* (2026-11-18; PM decision 767): new memo, the D-T11 half of the T3-ab memo rev 1.1, carried
across unchanged except for the folds below. The T3-ab memo's own history (rev 1, 2026-11-15, PM decision 741 (3);
rev 1.1, PM decision 756) is in that file's last revision (`0f8113eb`).
Folded from R3 wave 1: **W1-2** (cell T-19 no longer reads a global `HTMLBodyElement`, which does not exist: H-B2,
P-B1; the prototype is taken from an HTML `body`; both oracle rows re-derived; §2.1 FN-3 and §6.3 follow); **W1-4**
(the interface of `body` in the HTML namespace is HTML §3.2.2's *element interface* step 4 with HTML §4.3.1's
`HTMLBodyElement`; DOM §4.5 gives only the default: H-B1, §0.4, §3, §4.4); MINs: the HTML-`body` fact has one
statement, and its placement is decided with its ground (D-T11, *Placement*); §7 and §8.2 item 1 give one deferral
reason, with part (i)'s stated as what it is; SVG2 §4.4.1 Interface SVGElement is cited by number; §1.3's K and M are
`webref coverage-map` output; this memo's review numbering is stated (above). Also: the reviewer's note on
`SVGSVGElement includes WindowEventHandlers` is answered in D-T11 (no change).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`cee74989`** coordinate, measured with `git grep -n -E '<pattern>' cee74989 -- <paths>`,
`git show cee74989:<path> | sed -n <range>p` or `/usr/bin/grep -n` (never the default ugrep; never `\b` under `git
grep -E`, which matches nothing). Paths are abbreviated after first use: `elidex-js/…` =
`crates/script/elidex-js/src/…`, `ecs/…` = `crates/core/elidex-ecs/src/…`. Spec text is `.claude/tools/webref body` /
`idl` output of 2026-11-18; every `§X.Y = title` pair is `webref heading` / `dfn` output (§3's footnote). Symbols
written as `Type` + method (`HostData::dom_shared`, `HostData::tag_matches_ascii_case`, `EcsDom::is_html_namespace`,
`EcsDom::is_base_element`, `EcsDom::create_element_ns`) name existing methods inside `impl` blocks, which preflight's
literal symbol grep does not find (its soft warnings).

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `cee74989` |
|---|---|---|---|
| H-B1 | (new; R3 wave 1 W1-4) Which interface an element has | `.claude/tools/webref body dom concept-element-interface`; `webref body html elements-in-the-dom`; `webref body html the-body-element`; `webref element html body`; `webref body SVG2 InterfaceSVGElement`; `webref idl mathml-core MathMLElement`; `webref body mathml-core mathml-elements-and-attributes`; `webref body mathml-core dom-and-javascript`; `webref body webidl dfn-attribute-getter` | DOM §4.5 Interface Document: "element interface for any name and namespace is Element, unless stated otherwise" (the default only). HTML §3.2.2 Elements in the DOM: "The element interface for an element with name *name* in the HTML namespace is determined as follows", whose step 4 is "Otherwise, if this specification defines an interface appropriate for the element type corresponding to the local name *name*, then return that interface". HTML §4.3.1 The body element: "DOM interface: `interface HTMLBodyElement : HTMLElement`", "`HTMLBodyElement includes WindowEventHandlers`" (`webref element html body` → `HTMLBodyElement`). So `HTMLBodyElement` is stated for the local name `body` **in the HTML namespace**, by HTML §3.2.2 step 4 with §4.3.1; for any other namespace HTML states nothing and DOM's default stands unless that namespace's spec states otherwise. **An SVG-namespace `body` (rev 1.1):** SVG2 §4.4.1 Interface SVGElement says "All of the SVG DOM interfaces that correspond directly to elements in the SVG language (such as the SVGPathElement interface for the ‘path’ element) derive from the SVGElement interface": it states `SVGElement` for elements of the SVG language, and `body` is not one. No SVG2 text read here states an interface for an unknown element in the SVG namespace (`webref dfn SVG2 'unknown element'` → no dfn), so DOM's default `Element` stands. What this memo needs is the negative, which holds on either reading: it is **not an `HTMLBodyElement`** (`interface SVGElement : Element`, `SVGElement includes GlobalEventHandlers`: no `WindowEventHandlers`, no `HTMLBodyElement` member; `Element` has neither mixin). **A MathML-namespace `body`:** MathML Core §2.1 Elements and attributes: "The term MathML element refers to any element in the MathML namespace … Any MathML element that is not listed below is called an Unknown MathML element"; §2.2.3 DOM and JavaScript: "All the nodes representing MathML elements in the DOM must implement, and expose to scripts, the following MathMLElement interface, unless otherwise specified", `interface MathMLElement : Element { }`, `MathMLElement includes GlobalEventHandlers`. So it has `MathMLElement`. **The `TypeError`:** Web IDL §3.7.6 Attributes, the *attribute getter* steps: "If jsValue does not implement target, then: … Otherwise, throw a TypeError" (step 1.1.2.3; the *attribute setter* has the same test, its `validThis`) |
| H-B2 | (new; R3 wave 1 W1-2) The VM has no global `HTMLBodyElement` | `git grep -n '"HTMLBodyElement"' cee74989 -- crates/script/elidex-js/src`; control `git grep -n '"HTMLElement"' cee74989 -- crates/script/elidex-js/src`; `git show cee74989:crates/script/elidex-js/src/vm/host/html_passive_protos.rs \| sed -n 44p` and `sed -n 57,66p`; `git show cee74989:crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs \| sed -n 467,480p` | one hit, the brand-error string of `require_body_receiver` (`elidex-js/vm/host/event_handler_attrs.rs:423`); the control finds `HTMLElement`'s constructor (`elidex-js/vm/host/custom_elements/html_element.rs:69`). `html_body_prototype` is allocated as a brand-only prototype object (`html_passive_protos.rs:44`), with the body pair installed on it (`:63-66`); no constructor and no global binding. Existing tests reach it as `Object.getPrototypeOf(document.createElement('body'))` (`tests_event_handler_attrs.rs:473`). Same at `27ca7d1e`. **Wider (rev 1.1; R1 Axis 5):** at `cee74989` no per-tag HTML interface object exists except `HTMLElement`: `git grep -n -E '"HTML[A-Za-z]+Element"' cee74989 -- crates/script/elidex-js/src ':!*tests*'` lists brand-error strings of `require_receiver` calls and name atoms only, and the one constructor installed as a global is `HTMLElement`'s (`custom_elements/html_element.rs:95`, through the atom `html_element_global`). The 13 constructor-global atoms of the form-control family (`html_label_global` … `validity_state_global`, `crates/script/elidex-js/src/vm/well_known.rs:995-1011`) are defined and unused: `for a in html_label_global html_optgroup_global html_legend_global html_option_global html_fieldset_global html_form_global html_button_global html_textarea_global html_select_global html_input_global html_form_controls_collection_global html_options_collection_global validity_state_global; do git grep -n -w $a cee74989 -- crates \| wc -l; done` → 1 each (the definition; control: `git grep -n -w html_element_global cee74989 -- crates \| wc -l` → 2) |
| H-A4 | (T3 rev 5's H-T18) Foreign elements take HTML prototypes | `git show cee74989:crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs \| sed -n 124,198p`; `git show cee74989:crates/script/elidex-js/src/vm/host/elements.rs \| sed -n 291,296p`; `git grep -n -e 'require_body_receiver(' -e 'tag_matches_ascii_case(entity, "body")' cee74989 -- crates/script/elidex-js/src` | `prototype_kind_for` (`entity_kind.rs:150`) returns `PrototypeKind::Element` for every `TagType` entity; `HostData::tag_matches_ascii_case` (`:124-133`) reads no namespace. The `body` arm of `tag_specific_t2b_prototype` (`elidex-js/vm/host/elements.rs:294-296`) and `require_body_receiver` (`elidex-js/vm/host/event_handler_attrs.rs:418-432`, the test at `:427`; called by `native_body_weh_get` `:445` and `native_body_weh_set` `:468`) are its two `body` readers. So an SVG or MathML `body` gets `HTMLBodyElement.prototype` and passes its brand check |
| H-A5 | The namespace read that exists, and the predicates beside it | `git show cee74989:crates/core/elidex-ecs/src/dom/classify.rs \| sed -n 26,46p` and `sed -n 67,138p`; `git show cee74989:crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs \| sed -n 230,246p`; `git grep -n 'dom_shared()' cee74989 -- crates/script/elidex-js/src/vm/host`; `git grep -n 'is_html_namespace(' cee74989 -- crates ':!*tests*'` | `EcsDom::is_html_namespace` (`ecs/dom/classify.rs:136`: an element whose namespace is `Html`; its doc: "The canonical HTML-namespace gate for predicates that the HTML spec restricts to HTML elements"). `EcsDom::is_base_element` (`:39-46`) is a tag test `&&` that gate; its readers are in other crates (the base-URL consumer in `elidex-dom-api`). The other callers compose the gate with their own test at the site (`crates/dom/elidex-dom-api/src/class_list.rs:285`, `focus/predicate.rs:55` and five more there, `crates/dom/elidex-form-core/src/input.rs:795`). `HostData::dom_shared` (`bind.rs:242`; it asserts a bound host) is the existing `&HostData → &EcsDom` path, with 10 callers in `vm/host/` (custom elements, slot and table prototypes) |
| H-A6, H-A7 | How a foreign element comes to exist; the HTML parser cannot create a foreign `body` | the T3-a memo §0.2, rows H-A6 and H-A7 (same commands) | no `createElementNS` in the VM; a foreign element is created by the HTML parser's foreign content or by a clone of one, and a `body` start tag breaks out of foreign content (HTML §13.2.6.5). **No script can create an SVG or MathML `body` at the base** |
| H-A10 | The cited files on main | `git diff --stat cee74989 origin/main -- crates/script/elidex-js/src/vm/host/elements.rs crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-js/src/vm/host/html_passive_protos.rs crates/script/elidex-js/src/vm/host_data crates/core/elidex-ecs/src/dom/classify.rs crates/script/elidex-js/src/vm/test_helpers.rs` | one file, `vm/host_data/mod.rs`, 3 deletions, at `653d7c24` (#636) and at `origin/main` `1b176ea9` (verified 2026-11-18; rev 1: at `44a2d8cb`) |
| H-A11 | The direct `EcsDom` method calls in the two VM files, in three forms (rev 1.1: (B) and (C) added, R1 Axis 1) | per file `f`, with `X` = `git show cee74989:$f`: (A) chained, `X \| /usr/bin/grep -c -E 'dom_shared\(\)\.\|\.dom\(\)\.'`; (B) a binding of the dom to a name, `X \| /usr/bin/grep -c -E '=[[:space:]]*[^;=]*\.(dom\|dom_shared)\(\)[[:space:]]*;'`; (C) a method call on a name `dom`, `X \| /usr/bin/grep -c -E '(^\|[^_[:alnum:].])dom\.[a-z_]+\('` | `event_handler_attrs.rs`: 0, 0, 0 (it passes `host.dom()` as an argument to two session functions, `:542`, `:610`, and the closure parameter `dom` at `:542` is passed on, not called on; it calls no `EcsDom` method itself). `elements.rs`: 0, 0, 0. Positive controls of the same expressions on known hits: (B) on `crates/script/elidex-js/src/vm/host/html_table_proto.rs` → 1 (`:143`, `let dom = ctx.host().dom_shared();`; 82 such lines in `vm/host/`); (C) on `crates/script/elidex-js/src/vm/host/custom_elements/lookup.rs` → at least 1 (`:159`). Same at `653d7c24` (the two files are unchanged, H-A10) |
| H-B3 | T3-b's planned code reads no parent and no child link | `for f in crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-js/src/vm/host/elements.rs; do git show cee74989:$f \| /usr/bin/grep -c -E 'get_parent\|children_iter\|first_child\|last_child\|next_sibling\|prev_sibling\|traverse_descendants\|TreeRelation'; done` | 0 and 0; D-T11 reads `TagType` and `Namespace` of the one entity it is given |

### §0.3 Premise errors (reported)

- **P-B1 (the T3-ab memo rev 1.1's cell T-19, row FN-3 and two §6.3 rows; R3 wave 1 W1-2).** They read a global
  `HTMLBodyElement` (`svgBody instanceof HTMLBodyElement`, `HTMLBodyElement.prototype`). No such binding exists
  (H-B2), so the script threw a ReferenceError on the base and at the head alike, neither oracle row could be
  produced, and the two D-T11 mutants were unguarded. T-19 now takes the prototype object from an HTML `body` (§6.1).
- **P-B2 (the T3-ab memo rev 1.1 §3, §4.4, D-T11; R3 wave 1 W1-4).** "`HTMLBodyElement` is the interface of `body` in
  the HTML namespace only" was credited to DOM §4.5, which states only the default (`Element`). The statement is
  HTML §3.2.2's *element interface* step 4 with §4.3.1 (H-B1).
- **P-A2 (T3 rev 5's cell T-19).** Its script called `document.createElementNS`, which the VM does not have, and no
  other script path creates a foreign `body` (H-A6, H-A7). T-19 takes its elements from the harness (§6.1).
- **P-A5 (T3 rev 5's residual R-D1, reach).** R-D1 is about a foreign `body`, which no script can create. It is a
  defect of fixture-built elements today and becomes reachable when `createElementNS` is wired
  (`#11-vm-create-element-ns-missing`).

### §0.4 Decisions (this PR)

- **D-T11 — `HTMLBodyElement` is chosen for an HTML `body` only (PM decisions 153, 741 (3); R-D1).** The spec states
  `HTMLBodyElement` for the local name `body` in the HTML namespace (HTML §3.2.2 step 4 with §4.3.1). A `body` in
  another namespace is not an `HTMLBodyElement`: a MathML one is an unknown MathML element with `MathMLElement`
  (MathML Core §2.1, §2.2.3), and for an SVG one SVG2 states no interface (§4.4.1 states `SVGElement` for elements
  of the SVG language), so DOM §4.5's default `Element` stands (H-B1; until rev 1.1: "`SVGElement`"). The `body` arm of `tag_specific_t2b_prototype` (`elements.rs:294`) and
  `require_body_receiver` (`event_handler_attrs.rs:427`) both ask one function for that fact (§4.2):
  `is_html_body_element(host, entity)` (NEW) = the local name is `body` and `EcsDom::is_html_namespace` holds (a
  component read for interface selection and for a brand check: marshalling). No `HostData` method is added and no
  `host_data/` file is edited. A foreign `body` then takes `HTMLElement.prototype`, the fallback every foreign
  element without an HTML namesake already takes (H-A4). Every member it loses is one the spec does not give it:
  `HTMLBodyElement`'s own attributes and its 18 WindowEventHandlers accessors are on none of `Element`, `SVGElement`
  and `MathMLElement` (H-B1). The fallback `HTMLElement.prototype` still gives it more than the spec does (the HTML
  members, and for an SVG `body` the GlobalEventHandlers accessors, which `MathMLElement` includes and `Element`
  does not): that is the stand-in every foreign element has today, §8.2 item 1's defect, and T3-b does not widen it.
  So the change is toward the spec at every observation (row FN-3; cell T-19).
  *`SVGSVGElement includes WindowEventHandlers`* (R3 wave 1, Axis 4 note; SVG2 §5.13.2 Interface SVGSVGElement): no
  change. That mixin is on the interface of the `svg` element; an SVG-namespace element whose local name is `body`
  is not an `SVGSVGElement`, and neither `Element` nor `SVGElement` includes it. The `svg` element's own accessors are part of §8.2 item 1's defect.
  *Placement of the HTML-`body` fact (R3 wave 1 MIN, Axes 1 and 2; the lens: one issue, one way; Layering).* The T3-ab
  memo wrote the pair (tag `body`, HTML namespace) inline at both VM sites and paired them by a DoD grep. Decided:
  **one function, in `vm/host/`, read by both sites; no `EcsDom` predicate.**
  - *One function, not two inline tests.* The prototype choice and the brand check must agree for every entity
    (B4×B10: an entity has `HTMLBodyElement.prototype` iff it passes `require_body_receiver`). With one statement
    they agree by construction; with two they agree as long as a grep is run. The `&&` order is also load-bearing
    (the tag test is false for an unbound host, and `dom_shared` asserts a bound one), and is then written once.
  - *Not an `EcsDom::is_body_element` beside `EcsDom::is_base_element`.* (1) The fact's readers after T3-b are two,
    both in `vm/host/`; `is_base_element` is on `EcsDom` because its readers are in other crates (H-A5). (2) The
    third site R3 counted, the T3-a predicate's HTML arm, asks a different question (`body` *or* `frameset`, the
    class of the attribute change steps' step 1; `frameset` has no interface in the VM), inside a `match` arm that
    has already fixed the namespace. Its class test `is_body_or_frameset` is namespaced by T3's D-T5. Two grounds
    are in play there, and they are not the same one (rev 1.1; R1 Axis 2). *PM decision 133 (the scope of
    unification):* step 1's class test is unified with `document.body`'s predicate
    (`first_body_or_frameset_child`) only if T3's step 3 or 4 uses "the body element"; neither does, so
    `document.body`'s test stays out of T3. *D-T5's derived consequence (the T3 memo's, not the PM's):* with no
    second reader to share it with, the namespace clause is added to the session-private `is_body_or_frameset`, and
    no `EcsDom` symbol is added. An `EcsDom` predicate added here would be a second form beside that one, read by
    neither: the new-seam-plus-legacy state one issue, one way forbids. The convergence of every "HTML `body`" test
    on one form is the general fix's (§8.2 item 1), not T3-b's. (3) The canonical statement of the *namespace*
    fact already exists and is the one used: `EcsDom::is_html_namespace`, which its doc names as the gate and which
    every caller composes with its own tag test at the site (H-A5). (4) The general fix (§8.2 item 1) namespaces
    every tag arm; its form will be a tag-parameterised (local name, HTML namespace) test where
    `HostData::tag_matches_ascii_case` lives, and a per-tag `EcsDom` predicate would be one more form for it to
    migrate. `is_html_body_element` is itself a per-tag form, in `vm/host/`; under the general fix its body becomes
    a call of that tag-parameterised test (or the function is replaced by it), which §8.2 item 1's text states.
  - *The sibling sites (rev 1.1; R1 Axes 1, 2, 3).* Seven sites in four crates test a DOM entity for the tag `body`
    without its namespace at `cee74989` (`git grep -n -E '"body"' cee74989 -- 'crates/**/src/**' ':!*tests*'`, the
    hits that test an entity's `TagType`; the parsers' token matches and `elidex-dom-compat`'s presentational-hint
    tag lists are not entity class tests). Each has a home: `elements.rs:294` and `event_handler_attrs.rs:427`
    (`elidex-js`) are D-T11's; the session's `is_body_or_frameset` (`mod.rs:231`) and `is_body_element` (`:261`)
    are T3's (D-T5; the second is deleted, D-T1); `first_body_or_frameset_child`
    (`crates/dom/elidex-dom-api/src/char_data/document_props.rs:171`) is the T3 memo §8.1 item 3's defect text; and
    two were recorded nowhere: `find_body_child` (`crates/css/elidex-style/src/walk.rs:592-596`, the `body` whose
    overflow propagates to the viewport; `EcsDom::has_tag`, an exact compare) and the `offsetParent` fallback
    (`crates/dom/elidex-dom-api/src/element/layout_query.rs:374`, `tag.0 == "body" || tag.0 == "html"`, also exact).
    They ask different questions (which element propagates overflow; which ancestor ends the `offsetParent` walk),
    so they are not readers of `is_html_body_element`; they are siblings of the same defect class and join §8.2
    item 1's text.
  - Not changed by this decision: the T3 memo's D-T5.
  *Why not every arm (the general fix):* for a tag that is also an SVG or MathML element name (`a`, `script`, `style`,
  `title`, …) the fallback `HTMLElement.prototype` would remove members the foreign interface has (e.g. `type` on
  `SVGScriptElement`), which the HTML interface supplies today: worse than the base. Those arms need the SVG and
  MathML interface families first. The arms without a foreign namesake (`div`, `iframe`, …) need nothing new and
  would move toward the spec; they are not done here because they are not R-D (`body` is in this program for its
  WindowEventHandlers accessors, which act on the Window), and doing them means partitioning every prototype arm and
  every per-interface receiver check of the VM by "is this also a foreign element name", an engine-wide change with
  its own enumeration and review. §8.2 item 1 is the defect text, with both parts.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| B4 `body`'s interface | prototype choice and receiver check, one fact: local name `body` in the HTML namespace (D-T11) |

Coupled from outside, on the base and not changed: B10 the wrapper's prototype is chosen once, at first allocation
(C1-pre D-3); B2 the element's `Namespace` and local name, immutable per element. (B1, B3 and B5 to B9 are the T3-a
memo's axes; T3-b does not meet them.)

### §1.1 Outcome changes at T3-b (toward the spec)

Harness-built elements only, until `#11-vm-create-element-ns-missing` is wired (P-A5): an SVG or MathML `body` does
not get `HTMLBodyElement.prototype` (base: it does, with the 18 WindowEventHandlers accessors acting on the Window),
and the body pair's accessors called on it throw (base: they act on the Window). Row FN-3; cell T-19. No
script-reachable outcome changes: every `body` a script can create is an HTML `body` (H-A7).

### §1.2 Is T3-b no worse than its base, and is T3 no worse landing after it?

Criterion: the umbrella's K-0 with T3-b's base as "HEAD". T3-b changes one row, FN-3, to the spec. **Rows worse than
the base: 0.** T3-b and T3-a are unordered: FN-3 reads no T3-a outcome. On a foreign `body` × a WindowEventHandlers
name, T3-b decides the IDL side and T3-a (a MathML `body`) or T3 (an SVG `body`'s target, T3 memo FN-2) the content
attribute; with T3-b alone the IDL side is on the spec and the content attribute base-equal, which is not worse than
both off it, and no script can build the element.

The other direction (T3 on a base that has T3-b): the row T3-b closes is equal on T3's base and after T3 (T3 memo
§2.1, marked ‡). The two can land in this order only: T3 before T3-b would leave R-D1 open at T3's landing, which PM
decision 153's strict reading does not allow, and T3's namespaced step 1 without D-T11 would put row FN-3 on neither
the base nor the spec (an SVG `body`'s body pair would write the element; T3 memo §1.3, seam S1).

### §1.3 Edge density

*Verdict:* plan-review is mandatory (PM decisions 741 (3), 767: this memo's R1, five axes), whichever way the count
falls. Three axes meet: one own (B4) and two external (B2, B10), so the rule's trigger (a) (three or more
intersecting invariant axes) can fire; they meet in a single intersection (B4×B2×B10, §2). No canonical algorithm is
missing: the interface of an element is a function of (namespace, local name) the specs state (H-B1).

Breadth (all §3 rows): `.claude/tools/webref coverage-map html 3.2.2 html 4.3.1 dom 4.5 SVG2 4.4.1 mathml-core 2.2.3
webidl 3.7.6` → "**Breadth (requested)**: spec=5 (SVG2, dom, html, mathml-core, webidl), step=6 entries", "**Split
decision**: K≥4 or M≥20 → 分割推奨 (per-PR scope を再評価)" (output of 2026-11-18; K = 5, M = 6; rev 1: K = 4, M = 5,
before the Web IDL row). Re-evaluated: K counts the specs that state the answer for each namespace (HTML for the
positive case; SVG2 and MathML Core for what a foreign `body` has instead; DOM for the default) and the one that
states the wrong receiver's exception (Web IDL), not mechanisms. The change is one function with two readers and one cell; there is no
seam inside it to split along (a prototype choice without its receiver check, or the reverse, breaks B4×B10).

**The split that produced this memo (PM decision 767).** The T3-ab memo held D-T8 and D-T11 in one slice; R3 wave 1's
Axis 3 ruled them two (W1-1). T3-b is the D-T11 half.

T3-b is a narrowly-scoped per-PR slice under the approved EH-attrs umbrella (§1.2 row T3-b): the recursion that
trigger (a) starts ends at the rule's base case (c).
External couplings: C1-pre (landed); T3 (x16, after). Decisions: PM 153, 741 (3), 767.

## §2. Coupled invariants

- **B4×B10** — the namespace and the local name never change after creation, and the prototype is chosen once: an
  entity has `HTMLBodyElement.prototype` iff it passes `require_body_receiver`, because both ask
  `is_html_body_element`.
- **B4×B2** — `is_html_body_element` reads the namespace through `EcsDom::is_html_namespace`, the gate the T3-a
  predicate's `namespace_of` and T3's D-T5 read the same component through. They are different questions (which
  interface; which names are content attributes; which class step 1 tests) and stay separate tests of one component.
- **B4 and the name classes** — after D-T11 a foreign `body` has no WindowEventHandlers accessor; its
  GlobalEventHandlers accessors are `HTMLElement.prototype`'s normal pair, as for every other foreign element.

### §2.1 Enumeration (Base / T3-b / spec)

Verdict as in the T3 memo §2.1. *Reach*: **S** a script can build it; **F** harness-built only (H-A7). "The body
prototype" = the object `Object.getPrototypeOf` returns for an HTML `body` (`HTMLBodyElement.prototype` in the spec;
the VM has the object and no global name for it, H-B2).

| Row | Element | Observation | Reach | Base | T3-b | Spec | Verdict | Cells |
|---|---|---|---|---|---|---|---|---|
| FN-3 | SVG or MathML `body` | its prototype is the body prototype | F | yes | no | no (not an `HTMLBodyElement`: `Element` for an SVG `body`, `MathMLElement` for a MathML one, H-B1; the VM's stand-in is `HTMLElement.prototype`, §8.2 item 1) | spec (R-D1) | T-19 (value 1) |
| FN-3 | SVG or MathML `body` | `el.on<T>` for a WindowEventHandlers name, get and set | F | the body pair: reads and writes the Window | an ordinary property | an ordinary property (no such attribute on `Element`, `SVGElement` or `MathMLElement`) | spec (R-D1) | T-19 (values 2 to 4) |
| FN-3 | SVG or MathML `body` | the body pair's accessor called on it | F | succeeds, on the Window | `TypeError` | `TypeError` (the receiver does not implement `HTMLBodyElement`: Web IDL §3.7.6 Attributes, the *attribute getter* and *attribute setter* steps) | spec (R-D1) | T-19 (value 5) |
| FN-3h | HTML `body` | all three | S | the body prototype; the body pair; succeeds | the same | the same | both | the existing F-24 and the slice-a body cells, kept green; T-19's `BP` |
| FN-3n | HTML element other than `body` | the body pair's accessor called on it | S | `TypeError` | `TypeError` | `TypeError` | both | the existing F-24 |

## §3. Spec coverage map

Skeleton: `.claude/tools/webref coverage-map html 3.2.2 html 4.3.1 dom 4.5 SVG2 4.4.1 mathml-core 2.2.3` (§1.3).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §3.2.2 Elements in the DOM | *element interface* for a name in the HTML namespace, step 4 | the HTML namespace and local name `body` → the interface HTML defines for it; steps 1 to 3 and 5 to 7 not touched | `is_html_body_element` (NEW) in `crates/script/elidex-js/src/vm/host/elements.rs`, read at `:294` | ✗ | yes |
| WHATWG HTML §4.3.1 The body element | DOM interface: `HTMLBodyElement : HTMLElement`, `includes WindowEventHandlers` | the receiver of the body pair's accessors is an `HTMLBodyElement` | `require_body_receiver` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:418` | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | *element interface* ("Element, unless stated otherwise") | a namespace for which HTML states nothing → not `HTMLBodyElement` | the false side of `is_html_body_element` | ✗ | yes |
| SVG2 §4.4.1 Interface SVGElement | `SVGElement` is the base of the interfaces "that correspond directly to elements in the SVG language"; `SVGElement : Element`, `includes GlobalEventHandlers` | `body` is not an element of the SVG language, so SVG2 states no interface for an SVG-namespace `body` (DOM's default); on either reading it has no WindowEventHandlers IDL attribute and is not an `HTMLBodyElement` | the false side of `is_html_body_element` (the arm at `crates/script/elidex-js/src/vm/host/elements.rs:294` not taken) | ✗ | yes |
| MathML Core §2.2.3 DOM and JavaScript | every MathML element (any element in the MathML namespace, MathML Core §2.1) implements `MathMLElement : Element`, `includes GlobalEventHandlers` | a MathML `body` is a `MathMLElement`: no WindowEventHandlers IDL attribute, not an `HTMLBodyElement` | the same | ✗ | yes |
| Web IDL §3.7.6 Attributes | *attribute getter* step 1.1.2.3 and the *attribute setter*'s `validThis` test | the receiver does not implement the interface → `TypeError` | `require_body_receiver` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:418` (the brand check both of the body pair's natives call) | ✗ | yes |

Numbers and titles (2026-11-18): `webref heading html 3.2.2` (Elements in the DOM, `#elements-in-the-dom`), `4.3.1`
(The body element, `#the-body-element`), `13.2.6.5` (The rules for parsing tokens in foreign content); `webref heading
dom 4.5` (Interface Document); `webref dfn dom 'element interface'` → §4.5 `#concept-element-interface`; `webref
heading SVG2 4.4.1` (Interface SVGElement, `#InterfaceSVGElement`); `webref body SVG2 InterfaceSVGSVGElement` (its
heading: 5.13.2. Interface SVGSVGElement); `webref heading mathml-core 2.2.3` (DOM and JavaScript), `2.1` (Elements
and attributes, `#mathml-elements-and-attributes`); `webref dfn mathml-core 'Unknown MathML element'` → §2.1
`#dfn-unknown-mathml-element`; `webref heading webidl 3.7.6` (Attributes, `#js-attributes`); `webref heading html
16.3.2` (Frames, `#frames`) and `16.3` (Requirements for implementations), for §8.3 item 2. Texts: `webref body
html elements-in-the-dom` (the seven steps), `webref body html the-body-element` (the IDL block), `webref element html
body`, `webref body dom concept-element-interface`, `webref body SVG2 InterfaceSVGElement` (the IDL and its `includes`
lines), `webref idl mathml-core MathMLElement`, `webref body mathml-core mathml-elements-and-attributes` and
`dom-and-javascript` (the two quoted sentences), `webref body webidl dfn-attribute-getter` and `dfn-attribute-setter`,
`webref idl html WindowEventHandlers` (18 attributes), `webref idl html HTMLFrameSetElement`; `webref heading
css-overflow-3 3.1.4` (Overflow Viewport Propagation, `#overflow-propagation`), for §8.2 item 1's sibling site. "Full enum?" is
✗ on every row: T3-b decides one local name of HTML §3.2.2's step 4 and no other step (§8.2 item 1 has the rest).

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: any element whose wrapper is allocated (the prototype choice), and
any receiver passed to the body pair's accessors with `.call`. At the base no author input changes outcome: every
`body` a script can create is an HTML `body`. Once `createElementNS` exists, a foreign `body` loses an HTML
interface. The change removes a path by which a non-HTML element reads and writes the Window's handlers; it adds
none, and no security boundary is touched.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| "this entity is an HTML `body`" (HTML §3.2.2 step 4 for `body`) | `is_html_body_element` (NEW), `pub(super)` free fn in `elidex-js/vm/host/elements.rs`, beside the prototype selection; a pure read of the entity's `TagType` and `Namespace` components | D-T11 |
| its two readers | the `body` arm of `tag_specific_t2b_prototype` (same file); `require_body_receiver` (`elidex-js/vm/host/event_handler_attrs.rs`) | D-T11 |

No component, store, trait, event variant, crate edge, `pub` or `pub(crate)` item is added, and nothing is deleted.

### §4.2 Shape

```rust
// elidex-js/vm/host/elements.rs
/// (NEW) D-T11. The one statement of "an HTML `body`" for the VM's interface choice and its brand check.
/// The tag test comes first: it is false for an unbound host, and `dom_shared` asserts a bound one.
pub(super) fn is_html_body_element(host: &HostData, entity: elidex_ecs::Entity) -> bool {
    host.tag_matches_ascii_case(entity, "body") && host.dom_shared().is_html_namespace(entity)
}

// tag_specific_t2b_prototype, the `body` arm
if is_html_body_element(host, entity) { return self.html_body_prototype; }

// elidex-js/vm/host/event_handler_attrs.rs, require_body_receiver
if !super::elements::is_html_body_element(ctx.host(), entity) { /* Illegal invocation, unchanged */ }
```

T3-b touches none of the functions the umbrella's §4.1 fixes the shape of (`process_handler_content_attributes`,
`handle_attribute_change`): those are the T3-a memo's and T3's.

Re-runnable after T3-b: `git grep -n 'is_html_body_element(' -- crates ':!*tests*'` → the definition and two calls;
`git grep -n -e 'tag_matches_ascii_case(entity, "body")' -- crates/script/elidex-js/src` → one hit, inside the
definition.

### §4.3 ECS-native and Layering checks

"The subclass prototype is picked by tag name" (a class registry keyed by tag in object-oriented engines) → the same
pick as a query over two components of the entity, the tag and the namespace. The fact is a function of immutable
components, so it is not cached (nothing to invalidate; the side-store rule has no state to home). Layering:
`vm/host/` gains one component read through the existing `HostData::dom_shared` path, used for prototype selection
and for a brand check, both of the kinds the Layering mandate names for `vm/host/`; no algorithm is added there. The
one `is_html_namespace` call is the only direct `EcsDom` method call T3-b adds, in the chained form of §4.2 (H-A11's
three counts: 0, 0, 0 in both files on the base; after, 0, 0, 0 in `event_handler_attrs.rs` and 1, 0, 0 in
`elements.rs`; DoD item 4).

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `is_html_body_element` (NEW) | HTML §3.2.2 Elements in the DOM, *element interface* step 4 (a name in the HTML namespace); HTML §4.3.1 The body element (`HTMLBodyElement`); DOM §4.5 Interface Document, *element interface* (the default for a namespace HTML does not state); the foreign-interface defect (§8.2 item 1). **Deviation, stated in the docstring:** step 4 matches the local name exactly, and `HostData::tag_matches_ascii_case` compares ASCII-case-insensitively, so an HTML-namespace element whose local name is `BODY` would pass. No production path creates one at the base; it becomes creatable with `createElementNS`, and is recorded on that slot (§8.3 item 1) |
| the `body` arm | a pointer to `is_html_body_element` |
| `require_body_receiver` | its existing doc's "non-`<body>` wrapper" reads "a wrapper that is not an HTML `body` (`is_html_body_element`)"; HTML §4.3.1 (`HTMLBodyElement includes WindowEventHandlers`); Web IDL §3.7.6 Attributes (a receiver that does not implement the interface → `TypeError`) |

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The entities with local name `body` and a namespace other than HTML: at the base, harness-built only (H-A7). Every
other entity's outcome is unchanged (rows FN-3h, FN-3n).

### §5.2 Existing tests whose expectation changes

None: no existing test builds a foreign `body` (`git grep -n -e 'create_element_ns(' cee74989 -- crates | /usr/bin/grep
-i body` → 0; control: the same grep without the `body` filter lists the parser and test callers of H-A6). The full
run at the base confirms it (§9 item 1).

### §5.3 Touched files

Sizes at `origin/main` `1b176ea9` (`git show origin/main:<path> | wc -l`, re-verified 2026-11-18 after #636; equal to `cee74989`'s).

- `crates/script/elidex-js/src/vm/host/elements.rs` (455) — `is_html_body_element`; the `body` arm.
- `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` (683) — `require_body_receiver`.
- tests: `elidex-js/vm/tests/tests_body_interface.rs` (NEW; T-19), `crates/script/elidex-js/src/vm/test_helpers.rs`
  (326; the helper that builds the foreign `body` elements, §6.1, if T3-a has not added it).
- docs: none beyond this plan PR.

No file is over 1000 lines, so the touch-time split check does not fire. Not touched: `crates/core/elidex-ecs` (D-T11,
*Placement*), `crates/script/elidex-script-session` (the T3-a memo's and T3's), `elidex-js/vm/host_data/`,
`elidex-js/vm/host/html_passive_protos.rs` (the body pair's install is unchanged; only who receives the prototype
changes).

### §5.4 Named risks

- **R-A4 — D-T11's reach is harness-only until `createElementNS` lands.** Its cell (T-19) is real code through the
  real wrapper path, but its elements come from the harness. When `#11-vm-create-element-ns-missing` is wired, T-19's
  script form (`document.createElementNS(…, 'body')`) becomes writable; §8.3 item 1 records that for the slot.
- **R-B1 — merge contact.** None with T3 in production code (the T3 memo's *Not touched* lists `elements.rs`; T3
  edits other functions of `event_handler_attrs.rs`); with T3-a the one shared file is the test helper file (§6.1).

## §6. Test plan

Red-first: the **red** cell is committed failing on T3-b's base before the fix commit.

### §6.1 VM integration (`tests_body_interface.rs`, NEW)

The cell uses `with_page` / `run` (`elidex-js/vm/test_helpers.rs:168`, `:193`; `fn` counts its calls in `fnRan`) and
the harness-built globals `svgBody` and `mathBody`: an SVG `body` and a MathML `body` created before the script with
`EcsDom::create_element_ns`, owned by the bound document, their wrappers made by the production
`create_element_wrapper`, so D-T11's arm is the code under test. **The helper's shape is stated once, in the T3-a
memo §6.1; it is written by whichever of T3-a and T3-b lands first, and the other finds it on its base** (test
support only; not an edge, since either PR can write it).

The body prototype is taken from an HTML `body` (H-B2: there is no global `HTMLBodyElement`):

| Cell | Kind | Script | Base | T3-b (= spec) |
|---|---|---|---|---|
| T-19 | **red (R-D1; FN-3; D-T11)** | `var BP = Object.getPrototypeOf(document.createElement('body')); var D = Object.getOwnPropertyDescriptor(BP,'onhashchange'); r.push(Object.getPrototypeOf(svgBody) === BP); svgBody.onhashchange = fn; svgBody.dispatchEvent(new Event('hashchange')); r.push(fnRan === 0, window.onhashchange === null); mathBody.onhashchange = fn; r.push(window.onhashchange === null); try { D.get.call(svgBody); r.push(false) } catch (e) { r.push(e instanceof TypeError) }` | `true,true,false,false,false` | `false,true,true,true,true` |

Derivation, value by value. `BP` is the body prototype and `D` its own `onhashchange` accessor on both sides
(`html_passive_protos.rs:63-66` installs the body pair on it; an HTML `body` is unchanged, row FN-3h).
1. *Base:* the tag-only arm gives `svgBody` the body prototype → `true`. *T3-b:* `HTMLElement.prototype` → `false`.
2. `fnRan === 0` after a `hashchange` dispatched at the detached `svgBody`. *Base:* the write went through the body
   pair to the Window, so nothing on `svgBody` runs → `true`. *T3-b:* `onhashchange` is not an accessor on
   `HTMLElement.prototype` (it installs the `Global` and `DocumentElement` scopes), so the write made an ordinary
   property, which is not a handler → `true`. Equal on both sides: it pins that the head's property is inert.
3. `window.onhashchange === null`. *Base:* the Window holds `fn` → `false`. *T3-b:* untouched → `true`.
4. The same after `mathBody.onhashchange = fn`. *Base:* `false`. *T3-b:* `true`.
5. `D.get.call(svgBody)`. *Base:* `require_body_receiver` passes a tag-`body` receiver → no throw → `false`.
   *T3-b:* "Illegal invocation" `TypeError` → `true`.

### §6.2 Unit

None: the fact is two component reads behind one function whose two readers are VM code; T-19 runs both through the
production wrapper path, and the existing F-24 (`body_weh_accessor_rejects_non_body_receiver`,
`elidex-js/vm/tests/tests_event_handler_attrs.rs:467`) pins the non-`body` HTML receiver.

### §6.3 Mutation guards

| Mutant | Killed by |
|---|---|
| `is_html_body_element` tag-only (the base) | T-19 (values 1, 3, 4, 5) |
| the `body` arm bypasses the function with a tag-only test (the receiver check still namespaced) | T-19: value 1 is `true`, and the write `svgBody.onhashchange = fn` then throws from the body pair's setter, so the cell fails at the script |
| `require_body_receiver` bypasses the function with a tag-only test (the arm still namespaced) | T-19 (value 5: the accessor called on a foreign `body` succeeds) |
| the function namespace-only (any HTML element passes) | the existing F-24 (the setter called on a `div` no longer throws), kept green |
| the function with `\|\|` for `&&` | the existing F-24 |
| the namespace test inverted (a foreign `body` passes, an HTML one does not) | every existing HTML-`body` cell (an HTML `body` loses the body pair: F-24's `Object.getOwnPropertyDescriptor(…).set` reads a property of `undefined`), and T-19 the same way at `D.get` |

## §7. Out of scope and slots

| Item | Why not T3-b | Home |
|---|---|---|
| which names are an element's event handler content attributes | the session's predicate | the T3-a memo D-T8 |
| target determination; the namespace in `is_body_or_frameset` | T3 (PM decisions 25, 129, 133) | the T3 memo D-T1 to D-T7 |
| foreign elements' HTML interfaces other than `body`'s | not R-D. Arms whose local name is also a foreign element name need the SVG / MathML interface families first; the others need nothing new but are an engine-wide change of every prototype arm and receiver check, with its own enumeration (D-T11, *Why not every arm*) | proposed defect text (§8.2 item 1; PM to route) |
| an `EcsDom` predicate for "HTML `body`"; one form for every "HTML `body`" test (the seven sites of D-T11, *The sibling sites*) | D-T11, *Placement*: the general fix's form is tag-parameterised, and a per-tag predicate now would be one more form to migrate | §8.2 item 1 (the general fix; its text names the sites). Not changed here: T3 memo D-T5, PM decision 133 |
| `document.createElementNS` | not this program's | `#11-vm-create-element-ns-missing` (L3; §8.3 item 1) |
| a global `HTMLBodyElement` interface object | not R-D; the VM exposes no per-tag HTML interface object but `HTMLElement` (H-B2) | none opened here; likely home `#11-navigator-interface-object-branding` (the VM-wide interface-object program) with its C1-pre append; the routing is the PM's (§10 Q-B1) |
| the `HTMLFrameSetElement` interface | compat (PM decision 130); not R-D | `#11-htmlframesetelement-interface`, registered at this PR's landing (§8.3 item 2) |

Own deferrals: **0**. Residuals: **0**.

## §8. Edges, hand-offs, records

### §8.1 Placement (measured) and edges

**What T3-b needs on its base.** `tag_specific_t2b_prototype` and `require_body_receiver` (older; slice a's body
pair, landed by #563), `EcsDom::is_html_namespace` and `HostData::dom_shared` (older). All are on main at `cee74989`
and at `653d7c24` (H-A10).

**What T3-b does not need.**

| Unit | Does T3-b depend on it? | Why | Contact |
|---|---|---|---|
| T3-a | no (and T3-a does not depend on T3-b) | no shared production code; FN-3 reads no T3-a outcome (§1.2) | the test helper of §6.1, written by whichever lands first |
| L3 PR (is), PR (ig), PR (io) | no | no dispatcher window, no walker | none |
| T3-pre | no | no creation event | none |
| L3's tree-model program | no | no parent or child link read (H-B3) | none |
| T3 | T3 depends on T3-b (x16) | PM decision 153's R-D1 is closed before T3; T3's no-worse analysis is made on a base that has D-T11 | none in code (T3 does not edit the two sites) |

**Result.** T3-b can be cut from main now and depends on no unlanded unit; it is unordered against T3-a, is, ig,
T3-pre and the tree-model program. Two things bound its landing: it lands before T3's implementation branch is cut
(x16), and **before io lands**, the condition T3-a and T3-b carry for T3-pre's no-op window (the T3-pre memo DoD
item 7; PM decisions 756, 767 (1)). What happens when that condition is at risk is the T3-pre memo §8.3 item 3's
ledger text, and is not restated here. (Until rev 1.1: "before, between or after any of … io", against DoD item 7.)

**The no-op window of T3-pre's `FragmentParse` arms.** T3-b is not in that window's sequence; the condition it and
T3-a carry for the window is stated once, in the T3-pre memo DoD item 7 (PM decisions 756, 767), and its escalation
once, in the T3-pre memo §8.3 item 3, and not here.

**Edge (umbrella §1.3):** **x16** T3-b → T3, decision precondition (PM decisions 153, 741 (3), 767).

### §8.2 Slot texts (for the PM; defects only)

1. **Foreign elements take HTML interfaces** (pre-existing, engine-wide; recorded on `#11-html-element-type-tests-without-namespace` (PM 780, owner L3), which carries the full census; T3-b fixes only the `body` arm and `require_body_receiver`; until PM 780 the ledger had no row for it:
   re-measured 2026-11-18, `/usr/bin/grep -n -E 'SVGElement|MathMLElement|foreign element|foreign-element'
   project_open-defer-slots.md` → 0 hits, control `/usr/bin/grep -c 'attribute-namespace-model'` on the same file →
   2; the nearest rows are its namespace siblings registered by PM decision 768,
   `#11-attach-shadow-non-html-namespace` (a tag-only shadow-host test) and `#11-attribute-namespace-model`, which
   are other defects of the same class, a tag read without its namespace; candidate home: the master roadmap's
   `M4-13:#svg` row).
   Text: "The VM chooses an element wrapper's prototype by tag name alone: `prototype_kind_for` returns
   `PrototypeKind::Element` for every `TagType` entity
   (`crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:164-165` at `cee74989`), and the per-tag
   selection (`crates/script/elidex-js/src/vm/host/elements.rs:216-`) uses `tag_matches_ascii_case`
   (`entity_kind.rs:124-133`), which reads no namespace, as do the per-interface receiver checks that call it (e.g.
   `html_style_proto.rs:90`). So an SVG or MathML element whose local name is an HTML tag (`a`, `script`, `style`,
   `title`, `div`, …) gets that HTML interface's prototype, and every other foreign element gets
   `HTMLElement.prototype`; no `SVGElement` / `MathMLElement` interface is exposed, nor `SVGSVGElement` with its
   WindowEventHandlers attributes (HTML §3.2.2 Elements in the DOM states HTML interfaces for the HTML namespace
   only; WHATWG DOM §4.5 *element interface*; SVG2 §4.4.1, §5.13.2; MathML Core §2.2.3). Since T3-b (#<PR>,
   `<squash>`) the `body` arm and `HTMLBodyElement`'s receiver check also require the HTML namespace
   (`is_html_body_element`; T3-b memo D-T11), so a foreign `body` takes `HTMLElement.prototype`. The rest is in two
   parts: (i) arms whose local name is not an SVG or MathML element name can require the HTML namespace with no new
   interface (toward the spec at every member); nothing blocks this part, and it was left out of T3-b only because
   it changes every such arm and receiver check and needs the partition of local names into (i) and (ii) decided
   first; (ii) arms whose local name is also a foreign element name (`a`, `script`, `style`, `title`, …) need the
   foreign interfaces first, since `HTMLElement.prototype` lacks members the foreign interface has (e.g.
   `SVGScriptElement`'s `type`). The fix's form is one tag-parameterised (local name, HTML namespace) test where
   `tag_matches_ascii_case` lives; `is_html_body_element`'s body then becomes a call of it (or the function is
   replaced by it), so no per-tag form remains. Sibling sites of the same class outside the VM, to converge with it
   or be decided beside it: the tag-only `body` tests of `elidex-style` (`find_body_child`,
   `crates/css/elidex-style/src/walk.rs:592-596` at `cee74989`: the `body` whose overflow propagates to the viewport,
   CSS Overflow 3 §3.1.4 Overflow Viewport Propagation) and of `elidex-dom-api`'s `offsetParent` fallback
   (`crates/dom/elidex-dom-api/src/element/layout_query.rs:374`), both exact-match and without a namespace test;
   `document.body`'s (`first_body_or_frameset_child`) has its own text (T3 memo §8.1 item 3), and is a site of `#11-html-element-type-tests-without-namespace` (PM 780). **Trigger:** an SVG / MathML DOM interface PR, the `M4-13:#svg` program's plan, or a
   WPT `svg/` / `mathml/` subset. **Re-eval:** 2026-12-31. ⚠ No remedy reviewed."

### §8.3 Records for existing slots (for the PM, exact texts; at T3-b's landing)

1. `#11-vm-create-element-ns-missing` — append: "Since T3-b (#<PR>, `<squash>`) one outcome for a foreign `body` is
   pinned on harness-built elements only, because no script can create one (the HTML parser's foreign content breaks
   out at a `body` start tag, HTML §13.2.6.5): an SVG or MathML `body` does not get `HTMLBodyElement.prototype`
   (T3-b memo cell T-19). When `createElementNS` is wired that cell takes its script form. The same wiring makes a
   non-lowercase HTML-namespace local name creatable (`createElementNS(html, 'BODY')`), which the
   ASCII-case-insensitive tag predicates (`tag_matches_ascii_case`, the session's `is_body_or_frameset`,
   `EcsDom::is_base_element`) would then treat as the lowercase element (T3 memo §7)."
   **Trigger, old → new (PM decision 756): Trigger unchanged** (the one statement of the reason; the T3-a memo §8.3
   item 3 points here). Old: the row has no *Trigger* field; its fields end "**Owner**: L3。**Re-eval-date**:
   2026-12-15。" (`project_open-defer-slots.md`, the row `#11-vm-create-element-ns-missing`, read 2026-11-18). New:
   the same. Because the wait runs from the slot to these outcomes, not the other way: the outcomes R-D1 / R-D2 (and
   the T3 memo's FN rows on a foreign `body`) are on the spec at T3-a's, T3-b's and T3's landings on harness-built
   elements, and no script can build a foreign `body` before the wiring (H-A7), so nothing becomes wrong or
   reachable that would call for starting the wiring. What waits on it is the cells' script form, an obligation of
   the wiring PR, which the appended sentences state. A Trigger tied to T3-a, T3-b or T3 would schedule L3's work on
   an event that changes no reachable outcome.
2. `#11-htmlframesetelement-interface` — **register** (PM decision 130, compat layer; the ledger has no row:
   `/usr/bin/grep -c 'htmlframesetelement-interface' project_open-defer-slots.md` → 0 on 2026-11-18). Registered
   here because this memo is the first of the plan PR's landings to cite the id (*Not in scope*), and a cited id
   needs a row (driver's decision in R1; until T3 memo rev 6.2 the registration was T3's landing, T3 memo §8.3 item
   9, with rev 3's text at a `74e15df1` coordinate and no Re-eval date). Text, restated at `cee74989`:
   "pre-existing. elidex has no `HTMLFrameSetElement` interface (HTML §16.3.2 Frames: `interface HTMLFrameSetElement
   : HTMLElement` with reflected `cols` / `rows`, and `HTMLFrameSetElement includes WindowEventHandlers`). A
   `frameset` element's wrapper takes `HTMLElement.prototype`: the per-tag prototype selection has no `frameset` arm
   (`tag_specific_html_prototype` and `tag_specific_t2b_prototype`,
   `crates/script/elidex-js/src/vm/host/elements.rs:216-376` at `cee74989`; the fallback at `:131-133`). So
   `frameset.cols` / `.rows` and its 18 WindowEventHandlers IDL attributes read `undefined`. A `frameset`'s handler
   content attributes do not depend on the interface (the session reads the tag): today they are written to the
   element; T3's landing appends its effect (T3 memo §8.3 item 9). **Layer:** compat: `frameset` is an
   obsolete feature (HTML §16.3 Requirements for implementations), so the interface belongs to a compat plugin, not
   core (CLAUDE.md core/compat/deprecated). **Owner:** none; decided when the compat plugin work starts (PM decision
   130). **Trigger:** the compat plugin's first plan, or a compatibility-survey hit on a frameset page. **Re-eval:**
   2026-12-31. ⚠ No remedy reviewed."

### §8.4 Hand-offs

- **T3** (x16): T3 relies on D-T11 being on its base (its rows FN-3 ‡, its seam S1) and edits neither site. T3's
  `svgBody` cells (T-9, T-9b) reuse the harness helper of §6.1. T3 cites `#11-htmlframesetelement-interface` and
  appends one sentence to it at its landing (T3 memo §8.3 item 9); the registration is §8.3 item 2's.
- **L3's `#11-vm-create-element-ns-missing`**: §8.3 item 1.
- **T3-a**: none in production code; the test helper of §6.1.

## §9. Definition of done

1. T3-b's branch is cut from main (no prerequisite beyond what is on main, §8.1); every `cee74989` coordinate in
   §0.2 is re-measured there by its command; the full run of `elidex-js` and `elidex-shell` on the red commit shows
   no existing test changed (§5.2), and any that is changed is added to the PR description with its row of §2.1.
2. `/elidex-plan-review` passes on this memo (this memo's R1, five axes; PM decisions 741 (3), 767).
3. Red-first: T-19 committed failing with the base row of §6.1 (not with a script error); the fix makes it green.
4. Structural greps:
   - `git grep -n 'is_html_body_element(' -- crates ':!*tests*'` → the definition and two calls (§4.2);
   - `git grep -n -e 'tag_matches_ascii_case(entity, "body")' -- crates/script/elidex-js/src` → one hit, in the
     definition;
   - the direct `EcsDom` method calls in `vm/host/` (T3's R2 Axis 1; rev 1.1, R1 Axis 1: the rebinding form too),
     H-A11's three counts on the two files at the head: (A) chained, `/usr/bin/grep -c -E
     'dom_shared\(\)\.|\.dom\(\)\.' $f`; (B) a binding, `/usr/bin/grep -c -E
     '=[[:space:]]*[^;=]*\.(dom|dom_shared)\(\)[[:space:]]*;' $f`; (C) a call on a name `dom`, `/usr/bin/grep -c -E
     '(^|[^_[:alnum:].])dom\.[a-z_]+\(' $f` → `event_handler_attrs.rs` 0, 0, 0 and `elements.rs` 1, 0, 0 (base: 0, 0,
     0 in both, H-A11), the 1 being the `is_html_namespace` read in §4.2's chained form. A (B) hit means a dom was
     bound to a name: every use of that name is then read at the diff and counted, whatever the name. Positive
     controls, run with the counts: (B) on `crates/script/elidex-js/src/vm/host/html_table_proto.rs` → at least 1
     (`:143` at `cee74989`); (C) on `crates/script/elidex-js/src/vm/host/custom_elements/lookup.rs` → at least 1
     (`:159`);
   - `git diff --stat <base>..HEAD -- crates/core crates/script/elidex-script-session` → empty.
5. `cargo test -p elidex-js -p elidex-shell --all-features` green; `mise run ci` before push.
6. §8.2 item 1 and §8.3 items 1 and 2 handed to the PM in the PR description.
7. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-eh-attrs-t3b-body-prototype.md` exit 0.

## §10. Open questions

**Decided:** the carve (PM decision 741 (3)) and its split into T3-a and T3-b (PM decision 767); D-T11 (PM decision
153's R-D, T3 rev 2 to rev 5); the spec ground (H-B1).

**Within the carve, the designer's (not new scope; flagged for the plan-review):** one function for the two readers,
in `vm/host/`, and no `EcsDom` predicate (D-T11, *Placement*); T-19 takes the prototype from an HTML `body` (P-B1);
the foreign `body` cells are harness-built (P-A2).

**For the PM / the driver:**
- **Q-A2 (a further split) — answered (PM decision 767): two memos.** Preflight still reports SPLIT-RECOMMENDED for
  this memo (its K and M: §1.3); §1.3 states why no seam remains.
- **Q-A3 (the no-op window) — answered (PM decisions 756, 767): not an escalation**; the condition (T3-a and T3-b
  land before io lands) is the T3-pre memo DoD item 7's, and what is raised when it is at risk is the T3-pre memo
  §8.3 item 3's (§8.1).
- **Q-B1 (for the PM to route; not blocking).** The absence of a global `HTMLBodyElement` is one case of a wider
  fact (H-B2, widened in rev 1.1): no per-tag HTML interface object exists except `HTMLElement`, and 13
  constructor-global atoms are defined and unused. The likely home is the ledger row
  `#11-navigator-interface-object-branding` ("one VM-wide interface-object program"; carved by #398), whose append
  from C1-pre's landing (#616, 2026-11-10) already widens its trigger to DOM-node interface objects (`Node`,
  `Element`, `Document`; "`x instanceof Document` throws `ReferenceError`"); the per-tag HTML interface objects
  would be one more append there. This memo opens no slot and does not depend on the answer; whether to append to
  that row or open another is the PM's.
- **The placement of `is_html_body_element`** (a `pub(super)` function in `vm/host/elements.rs`) was ruled
  acceptable by R1's Axes 1 and 2 (the wave-2a tally, which attributes the ruling to PM decision 770; the PORTFOLIO's
  entry for 770 does not state it, reported to the driver).

No browser measurement is needed: every outcome in §2.1 is spec-defined (H-B1).

# C0 `EH-attrs-at-creation-C0` — every insertion adopts the node into the parent's node document (program memo)

Nested **program C0** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 by
#570; "the umbrella"). Since rev 3 C0 is a program of three PRs, each with its own per-PR plan memo and its own
`/elidex-plan-review` (CLAUDE.md *Edge-dense work* (a)–(c); PM decision 244):

| PR | per-PR plan memo | content (decisions of §0.4) |
|---|---|---|
| **C0a** `EH-attrs-at-creation-C0a` | `docs/plans/2026-11-eh-attrs-c0a-replace-record-order.md` (NEW) | D-12, after its prerequisite split D-14 (a) |
| **C0b** `EH-attrs-at-creation-C0b` | `docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` (NEW) | D-1, D-2 (no `Adopt` event), D-3 (a)(b), D-4, D-5, D-7, D-8, D-10, D-11, D-15 (iii), D-17, D-18, D-20, after its prerequisites C0b-pre1 = D-14 (c) and the CI/tools default-features check PR #585 (D-17 (iii), PM 255; merged `115a667c`) |
| **C0c** `EH-attrs-at-creation-C0c` | `docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md` (NEW) | D-3 (c), D-6, D-13, D-19, after its prerequisite splits D-14 (b), (d), (e) |

This memo holds what the three share: the frame and premises (§0), the decisions with their owning PR (§0.4), the
program's shape and order (§1), the coupled-invariant matrix (§2), the spec map of the whole (§3), the creation
census and the sibling tables (§4), the populations (§5), out-of-scope rows and slots (§7), program edges, hand-offs and
records (§8), the program's definition of done (§9) and the R1 disposition (§10). A per-PR memo restates the parts of
its scope that its own review needs and cites the rest here by ID.

The program closes slot `#11-cross-document-adopt-on-insert` (carved by S5-4a #444; closed at C0c's landing, when its
last facet — `adoptNode` — lands) and slot `#11-custom-elements-adopted-callback` (D-17 #227; closed by C0c), whose
trigger is "cross-document adoption infra landing" (H-12); C0b closes the non-template, non-Range-planned facets of
slot `#11-ownerless-node-creation` (owner Layout C0 / C1). Siblings cited by ID: IBP-sandbox N1
(`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570, implementation landed by #580
`27b50dfe`; "the N1 memo"); its node memo `docs/plans/2026-09-ibp-sandbox.md` ("the node memo"; amended in place on
this branch, D-15) and its input file `docs/plans/2026-09-ibp-sandbox-input.md` ("the node input"); the IBP parent memo
`docs/plans/2026-09-inline-box-predicate.md` ("the parent"); C1 (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`,
**rev 3.1 `b5e47f4b`** on branch `layout-eh-c1-template-owner-plan`, unpushed; "the C1 memo"); T3
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 3 on branch `layout-eh-target-determination-plan` at
`14ff3dc2`; "the T3 memo"); L3's Range program (`docs/plans/2026-11-range-contents-program.md`, **Rev 6.1 `d5bff269`**,
PR #583 at head **`a164ffe0`**, branch `range-full-impl-plan`, whose local tip `9a74cdf9` adds two A6 wording commits on
2026-11-03; "the L3 memo"; it splits RC-2 into **RC-2a** = the `CloneCreated` event and its consumers, the `cloneNode`
migration, D14 and the #466 reversal, C1's prerequisite (PM 249 Q-Z4), and **RC-2b** = the contents algorithm; its
Appendix A site IDs A1–A12 are cited as that memo's; L3 coordinates below are `a164ffe0`'s). Unlanded IDs are cited as
those memos' IDs.

**Placement (evidence).** PM decision 143 (2026-10-31; ledger `project_open-defer-slots.md`, slot
`#11-cross-document-adopt-on-insert`, sub-bullet "Owner assigned 2026-10-31"): "Layout lane, as C0 of the EH-attrs
program. C0 is a standalone PR before C1 … Edge-dense, so plan-review is mandatory. L3's P1 umbrella treats C0 as
landing first (the insertion-steps boundary goes in C0's plan §2)". The same sub-bullet still records the order "N1
implementation → T3 → C0 → C1 → slice b"; PM decision 153 (2026-10-31, user decision) superseded it:
**N1 implementation → C0 → C1 → T3 → slice b** (record R-8 corrects the ledger line). PM decision 223 O-2
(2026-11-01): C0 owns lifting the new-node owner-document entry into `EcsDom`. PM decision 244 (2026-11-03): after
plan-review R1 (verdict SPLIT) C0 is the program of three PRs above; "in C0" in PM decisions 158 / 160 (a) reads "in the
C0 program". So at C0a's base N1's implementation has landed and C1, T3 have not: HEAD's `resolve_handler_target`
(`crates/script/elidex-script-session/src/event_handler_processing/mod.rs`) is the target rule, and T3's
`determine_event_handler_target` does not exist (§8 H-T).

**Scope (the program).** One fact — the node document, stored as `AssociatedDocument` — set at creation by one entry,
kept true by one mechanism (DOM *adopt*) run at the one tree-linking chokepoint, read by one function, announced by one
event, and reachable from script through `document.adoptNode`; plus the *replace* record order. Per PR: §1.

**Not in scope.** Template contents' owner document and the template adopting steps' target `f(document)` (C1); the
Range program's planned sites A6–A12 except as §4.5 assigns them; the Selection API's document-membership check (§7);
`Node.isConnected`'s bound-document compare (§7, D-16); custom-element registry steps 3.2 / 3.3.2 (§7); `importNode`
(§7); `replaceChild(X, X)`'s records (§7); `moveBefore` (not implemented); the unification of the VM's Attr wrapper
with dom-api Attr entities (`#11-dom-api-attr-entity-model-unification`, §7).

**Changelog.** *Rev 3.6* (2026-11-03): C0c's plan-review (`76af676a`; dispositions in the C0c memo §10) and PM 306 /
311. Edge c0h (#583 → C0c) and the order line; §9 item 6 gains C0c's points; D-3 (c) / D-6: a segment under a `None`
pair is `None`, `Adopt` iff the root's pair; D-14 (b), §4.7: `adoptNode` in `document.rs`; c0e's file overlap; R-2's
iframe side; H-C1 (vi); §7's null-registry row points to the slot; K-0 names C0c's recorded regression. *Rev 3.5*
(2026-11-03): C0b-pre2 dropped (PM 300). *Rev 3.4.1* (2026-11-03): mise task and CLAUDE.md edits removed (PM 297). *Rev 3.4* (2026-11-03): C0b's plan-review of the re-cut (`0aaa80d9`; dispositions in the C0b memo §10.1)
and PM 290. D-17 (iii) (b): the boundary script is its own step; c0g: the clone.rs stamps by landing order; R-7
(parse cost) deleted; R-6 (c), (g) carried by C0b, R-8 by C0a; ledger cites by slot ID. *Rev 3.3* (2026-11-03): C0b's plan-review (`15e99049`; dispositions in the C0b memo §10) and PM 283.
D-21 is C0b-pre2's (NEW per-PR memo), before C0b: header table, D-21, §1, §3, edge c0-pre2, R-3, §9. The CI/tools PR
is #585 (edge c0-pre-ci). D-17 (iii), H-24: the boundary check reads build edges too. §4.4: the census population by
property, raw spawns included. *Rev 3.2* (2026-11-03): C0a's plan-review (`4c79d9c8`; 0 CRIT / 3 IMP / 15 MIN; dispositions in the
C0a memo §10) and PM 266 / 271 / 273. D-12: `capture_move_source`, the per-line renumber table (C0a). D-9 / R-3 /
H-T: C0a also writes the order line's immediate consequences in the umbrella (PM 271, amending PM 223 O-1; table
E1–E13 in R-3), R-B stated as text, "nothing else" narrowed. D-17 (iii), A8, H-24: the guard claims only that
`fixtures` is enabled from `[dev-dependencies]` only, read from `cargo tree` (PM 266); the generic guard is handed to
`#11-test-only-feature-boundary-guard`; the CI/tools step is the default-features Clippy step (PM 263). D-21: the
replace order (cache entry before the set chokepoint), the reverse pointer at the owner's despawn, two more reads.
P-16 / P-19 / H-L3: no relay to L3 (PM 273); §9 item 6 compares the wording with #583 instead. *Rev 3.1* (2026-11-03): plan-review R2 on rev 3 (`9809a8ee`; 0 CRIT / 5 IMP / 27 MIN) and PM 255 / 258.
D-21 (NEW, C0b): the element↔Attr relation is written only in `elidex-ecs` (R2 Ax2 IMP-1). D-17 (iii) split per PM 255
(the CI/tools PR owns the default-features step; C0b owns `scripts/check-fixture-boundary.sh`) and its guard defined by
property over every test-only feature (PM 258). D-18: `AssociatedDocument` crate-private. D-14 (c) is a prereq (PM
258). D-9 / R-3 carry PM 227 O-1's order-line rewrite. D-15 (iii) moves to C0b (PM 244 Q4). The CE constructor's owner
decision moves into `elidex-custom-elements` (D-11). §11 O-3 / O-4 decided. R2 dispositions: §10.1. *Rev 3* (2026-11-03): re-sliced per plan-review R1 (0 CRIT / 13 IMP / 19 MIN raw, verdict SPLIT) and PM
decision 244. This memo becomes the program memo; three per-PR memos (NEW) carry C0a / C0b / C0c. Decided (were open):
Q-2 → D-20 (per-node entry with the converted parent); Q-3 → D-17 (ownerless creation is test-only by construction;
the census is a default-features `cargo check` CI step that an unmigrated site cannot pass); Q-4 → D-18 (one writer module, one reader); the consumer walk → D-19
(one walker in spec order). Redesigned: D-10's substitution is now the §13.4 insertion target redirection keyed on the
fragment-parse **root** (`FragmentParseRoot`, NEW, replaces rev 2's `FragmentParseDocument` on the throwaway), read by
the entry and by the link-time adopt target, so no fragment-build node ever holds the throwaway document and D-8's
early re-home is deleted (§0.4 D-8, D-10; R1 Ax2 IMP-2, Ax4 IMP-1). T19 moved to unit level (U1 Attr variant, U16);
H-17 corrected. New cell T23 (Range `extractContents` / `cloneContents` `ownerDocument`, both landing orders). Node
memo amendment cite corrected (P-14), node input swept (D-15), parent F10 amendment line added (D-15). Every R1 finding
and its disposition: §10. *Rev 2.1* (2026-11-03, `d262ca8c`): re-framed onto `27b50dfe`. *Rev 2* (2026-11-01,
`42ca6a2c`): rebased onto `593aa314`; node memo amended (D-15, PM 157); `adoptNode` in scope (PM 158); umbrella row
rule (PM 159 / 223 O-1); replace record order and the session split (PM 160); `isConnected` → slot (PM 161); the
new-node entry in `EcsDom` (PM 223 O-2). *Rev 1* (2026-10-31, `5c4aedf2`): first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`27b50dfe`** coordinate (`origin/main` when rev 2.1 was framed; this branch's crates are
byte-identical to it: `git diff --quiet 27b50dfe HEAD -- crates && echo same` → `same`, run 2026-11-03). `origin/main`
has since moved (`d7d34035` at the 2026-11-03 fetch); each per-PR memo re-measures its coordinates at its own base
(§9 item 1 of each). Code references come from `/usr/bin/grep -n`, `git grep -n` and `sed -n` over `27b50dfe`, run
2026-11-03 for every coordinate this revision added or changed and carried from rev 2.1 otherwise (rev 2.1's frame
check: the crates are identical). Every § number ↔ title pair and algorithm step comes from `.claude/tools/webref`
(`heading`, `dfn`, `body`), run 2026-11-03; the commands are in §3 and in each per-PR memo's §3.

### §0.2 Premises re-measured

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-1 | No insertion adopts | `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → definition `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251`, prod caller `finish_detached_fragment` (`:337`), test `crates/core/elidex-ecs/src/dom/tests/destroy.rs:324`. `append_child` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`) runs `detach_with_hook` (`:40`) → `link_node` (`:43`) → `fire_after_insert` (`:47`) and never writes `AssociatedDocument` |
| H-2 | Tree linking has one chokepoint | `/usr/bin/grep -rn 'link_node(' crates --include='*.rs'` → definition `mutation.rs:210` (`pub(super)`) and three calls `:43`, `:119`, `:197`; `/usr/bin/grep -rn 'rel.parent = ' crates --include='*.rs'` → `mutation.rs:218` (link) and `crates/core/elidex-ecs/src/dom/tree/mod.rs:85` (clear). `attach_shadow_with_init` links the shadow root through `append_child(host, sr)` (`crates/core/elidex-ecs/src/dom/shadow.rs:203`) |
| H-3 | `owner_document` is stamp-first with two tree-position fallbacks | `EcsDom::owner_document` `crates/core/elidex-ecs/src/dom/mod.rs:817`; the getter adds a third: `owner_document(this).or_else(\|\| dom.document_root())` (`crates/dom/elidex-dom-api/src/node_methods/core.rs:279`) |
| H-4 | Which nodes carry a stamp at HEAD | stamped: `create_*_with_owner` callers, deep clones (`clone_subtree` `crates/core/elidex-ecs/src/dom/tree_clone.rs:186`), strict foreign elements and strict template contents (`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271`, used at `insert.rs:108`, `shadow.rs:124`), tolerant template contents (`crates/dom/elidex-html-parser/src/convert.rs:284`), every node a fragment parse returns (`teardown.rs:336-337`). **Unstamped**: every other parser creation (strict `insert.rs:61`, `:133`, `:141`, `:155`, `:210`, `shadow.rs:153`; tolerant `convert.rs:251`, `:305`, `:309`, `:316`), shadow roots (`shadow.rs:189`), shallow clones (`clone_node_shallow(src)` `tree_clone.rs:251` takes no document; `crates/dom/elidex-dom-api/src/node_methods/clone.rs:295` "The shallow cloner never stamps"), Attr entities (`create_attribute` `mod.rs:634`), doctypes (`create_document_type` `mod.rs:616`), the dom-api / engine sites of §4.4 |
| H-5 | N1's proxy (clause (b)), on N1's landed body | `scripting_disabled_for_platform_object` `crates/script/elidex-script-session/src/scripting.rs:97`; Document arm `:107`; node arm `:115-121` (composed tree root if a Document, else `owner_document`); docstring slot paragraph `:49-87` |
| H-6 | "Document → itself" idioms | `owner_doc` `crates/dom/elidex-dom-api/src/element/document_base.rs:173-178` (callers `:313`, `:365`, `:395`); `owner_document(x).unwrap_or(x)` `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`, `crates/dom/elidex-dom-api/src/char_data/document_props.rs:75` |
| H-7 | The session record layer models adopt step 2 | `capture_move_source` / `source_removal_record` / `move_record_list` (`crates/script/elidex-script-session/src/mutation/mod.rs:304-384`) |
| H-8 | Replace-with-fragment bypasses `EcsDom::replace_child` | `apply_replace_child` (`mutation/mod.rs:672`) fragment arm: `remove_child(parent, old_child)` (`:713`), then `expand_fragment` (`:719-722`); the fragment is never re-homed |
| H-9 | `MutationEvent` has no adoption variant | `pub enum MutationEvent<'a>` `crates/core/elidex-ecs/src/dom/mutation_event.rs:107`, seven variants; one exhaustive production `match` (`crates/dom/elidex-dom-api/src/range/live.rs:113-163`), one exhaustive test `match` (`crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:68-148`); others `_ => {}` |
| H-10 | Fire order inside the primitives | `append_child`: `is_connected` (`:39`), `detach_with_hook` (`:40`, def `:442`), `link_node` (`:43`), `rev_version` (`:44`), `fire_after_insert` (`:47`). `replace_child` (`:138`): `detach_with_hook(new_child)` (`:164`), `detach(old_child)` (`:186`), `fire_after_remove` (`:189`), `link_node` (`:197`), `fire_after_insert` (`:201`) |
| H-11 | Fragment builds run with the dispatcher taken; the root is a child of the throwaway | `begin_detached_fragment` (`teardown.rs:293`) takes the dispatcher, creates the throwaway (`:301`) and the root (`:302`, ownerless) and appends it (`:304`); the link is what the strict rollback `despawn_subtree(document)` reaches the build through (`teardown.rs:280-289`). Callers: strict `tree_builder/mod.rs:180`, tolerant `crates/dom/elidex-html-parser/src/lib.rs:143` |
| H-12 | `adoptedCallback` has a reserved slot and no fire path | `CustomElementReaction::Adopted` (`crates/dom/elidex-custom-elements/src/reaction.rs:21-25`); flush no-op (`crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133-138`); consumer matches `Insert` / `Remove` / `AttributeChange` (`crates/dom/elidex-custom-elements/src/consumer.rs:75-96`) |
| H-13 | `EcsDom::is_connected` counts any Document root | `crates/core/elidex-ecs/src/dom/tree/walkers.rs:143-158` |
| H-14 | HEAD's target rule ignores the node document | `resolve_handler_target` `event_handler_processing/mod.rs:249-255` |
| H-15 | `adoptNode` / `importNode` do not exist | `/usr/bin/grep -rn 'adoptNode\|importNode\|adopt_node' crates --include='*.rs'` → no hit |
| H-16 | The walk's order | `for_each_uncapped_shadow_inclusive` (`walkers.rs:99`): node, its shadow tree, its light children; with `TemplateContentsWalk::Include` a template's contents **after the template's whole subtree** (pushed first, popped last: `:112-118`). `Include` callers: `despawn_subtree` (`teardown.rs:197`) and `adopt_subtree` (`:253`), both order-insensitive; `Exclude`: the event-handler bind pass (`event_handler_processing/mod.rs:178`) — `git grep -n 'TemplateContentsWalk::' -- 'crates/*.rs'` |
| H-17 | Attr nodes: two models, only one reachable from script | dom-api: `create_attribute` (`mod.rs:634`) spawns `AttrData` + `NodeKind::Attribute`, no stamp; callers `CreateAttribute` (`crates/dom/elidex-dom-api/src/char_data/attr.rs:35`) and `GetAttributeNode` (`attr.rs:90`, cached in `AttrEntityCache`, `crates/core/elidex-ecs/src/components.rs:675`). **VM: neither is reached** — the VM has no `createAttribute` (`git grep -n createAttribute -- crates/script/elidex-js/src` → test comments only), its `getAttributeNode` is the VM-native `native_element_get_attribute_node` (`crates/script/elidex-js/src/vm/host/element_attrs.rs:479`) returning a wrapper with no entity, and `Attr.prototype`'s parent is `Object.prototype` (`crates/script/elidex-js/src/vm/host/attr_proto.rs:134-146`), so `attr.ownerDocument` is `undefined`. Rev 2's T19 (VM cell) was unreachable (§10) |
| H-18 | DOMParser documents are never destroyed | `parse_into_inert_document` (`crates/dom/elidex-form/src/inert_document.rs:110`) has no destroy path |
| H-19 | The fragment substitution lives in the strict tier only | strict `node_document_for_new_nodes` (`pub(super)`, `tree_builder/mod.rs:271`): fragment → `owner_document(context)`, else the parse document; tolerant threads `owner_doc = dom.owner_document(context)` (`lib.rs:158`) into template contents only (`convert.rs:284-286`) |
| H-20 | Touched files over 1000 lines | `wc -l` at `27b50dfe`: `crates/script/elidex-script-session/src/mutation/mod.rs` 994, `crates/script/elidex-js/src/vm/host/document.rs` 1197, `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` 994 (C0b, D-14 (c)), `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` 992 (C0b, D-18), `crates/dom/elidex-dom-api/src/range/live.rs` 1021 (C0c, D-14 (d)), `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` 1143 (C0c, D-14 (e)), `crates/dom/elidex-dom-api/src/selection.rs` 1036 (not touched) |
| H-21 | The `AssociatedDocument` write and read surface | writers: `set_associated_document` (`pub`, `mod.rs:789`) — prod callers `teardown.rs:257`, `tree_clone.rs:221`, `:230`, `:417`, `node_methods/clone.rs:283`, `:284`, `:299`; the creators' `insert_one` (`mod.rs:341-358` and kin). Readers: `get_associated_document` (`pub`, `mod.rs:777`; outside `mod.rs` only test reads `scripting.rs:201`, `:234`), `owner_document`. The component's field is `pub` (`components.rs:595`); no production code outside `elidex-ecs` names the type: `git grep -n 'set_associated_document(\|get_associated_document(\|AssociatedDocument(' -- 'crates/*.rs'` |
| H-22 | The four ownerless creators and their reach | `create_element` (`mod.rs:331`), `create_text` (`:492`), `create_document_fragment` (`:519`), `create_comment` (`:593`), all `pub`, each `…_with_owner(…, None)`. Raw call lines: `git grep -n -E '\.create_(element\|text\|comment\|document_fragment)\(' -- 'crates/*.rs' \| wc -l` → 2577; outside test-path files (`… \| /usr/bin/grep -v -E '/tests/\|tests\.rs\|/tests_\|test_helpers\|test_util\|/benches/' \| wc -l`) → 431, of which most are inline `#[cfg(test)]` modules (§4.4) |
| H-23 | The element↔Attr relation has four partial writers in dom-api | `GetAttributeNode` creates, sets `owner_element` and registers in `AttrEntityCache` (`char_data/attr.rs:90`, `:97`, `:101-110`); `SetAttributeNode` (`:117-183`) sets `owner_element` only (`:180`) — no stamp, no cache entry, the replaced Attr keeps its element; `RemoveAttributeNode` nulls `owner_element` (`:233`) and keeps the cache entry; `RemoveAttribute` evicts the entry (`element/props.rs:148`) and keeps the Attr's `owner_element`. DOM §4.9 *append an attribute* 1–3, *replace an attribute* 2–5, *remove an attribute* 2–3 (`webref body dom concept-element-attributes-append`, `-replace`, `-remove`, run 2026-11-03) set list, element and node document together. Unit-observable only (H-17) |
| H-24 | `fixtures` (NEW, C0b) can be checked from cargo's own feature graph: a dev-only feature does not appear on normal edges | `test-hooks` (`crates/net/elidex-net/Cargo.toml:39`, "Not enabled in production builds") is enabled only by `elidex-js`'s `[dev-dependencies]` (`crates/script/elidex-js/Cargo.toml:197`, `:202`): `cargo tree --workspace -e normal,features -i elidex-net \| /usr/bin/grep -c 'feature "test-hooks"'` → 0, with `-e normal,dev,features` → 1 (run 2026-11-03), so the `-e normal,features` tree is the population and a check over it is not vacuous; `cargo tree --workspace -e normal,features -i elidex-ecs` lists only `elidex-ecs feature "default"` at `27b50dfe` (run 2026-11-03), and so does `-e normal,build,features` (the build edges, which a product build also activates; the one `[build-dependencies]` table, `crates/dom/elidex-html-parser-strict/Cargo.toml:17`, names external crates), under which `test-hooks` counts 0, and 1 with `dev` added (run 2026-11-03). The only target-specific dependency tables (`crates/core/elidex-sandbox/Cargo.toml:14`, `:18`, `:21`) name external crates only, so no target-specific edge can enable `fixtures` and the host-target tree is complete for it. `parallel` (`elidex-style`) is enabled by no member (`cargo tree --workspace -e normal,features -i elidex-style \| /usr/bin/grep -c 'feature "parallel"'` → 0); its code (`git grep -n 'cfg(feature = "parallel")' -- crates` → 4) creates no node |
| H-25 | The strict tier leaves *in frameset* in the fragment case | `crates/dom/elidex-html-parser-strict/src/tree_builder/modes/in_frameset.rs:31-33` switches to *after frameset* without HTML §13.2.6.4.18 (`#parsing-main-inframeset`)'s "If the parser's fragment context element is null and" guard; *after after frameset* then inserts a comment into `tb.document` (`modes/after_after_frameset.rs:15`), which in the fragment case is the throwaway |

### §0.3 Premise errors (reported)

P-1 to P-13 are rev 2's (brief inputs and code comments; their text is unchanged and is kept in `42ca6a2c`). Those
that bind a per-PR memo are restated there: P-9 (replace's comment numbering) in C0a; P-2, P-6, P-7, P-8, P-10, P-11,
P-12, P-13 in C0b; P-4 in C0c. New in rev 3:
- **P-14 (the node memo amendment, rev 2's text).** It said the node clause is "Replaced … by C0's stored node document,
  `EcsDom::node_document` (D8), in HEAD's order (`elidex-script-session/src/scripting.rs:83-114`): a Document target →
  itself; a node → its `AssociatedDocument`". No HEAD code does that: at the node memo's frame `f1cf5d67` those lines are
  N1's composed-root proxy (`git show f1cf5d67:crates/script/elidex-script-session/src/scripting.rs | sed -n 83,114p`:
  a connected node → its composed tree root if a Document, else `owner_document`). Restated on this branch (D-15).
- **P-15 (L3 memo, rev 2's P-3).** Rev 2 reported "`CloneCreated` does not occur in the L3 memo" at Rev 3 `7ac367a3`.
  At Rev 5 `b6e500e7` it does (its C2: `MutationEvent::CloneCreated { root, scope }` (NEW)). P-3 is withdrawn; edge c0e
  names it (§8).
- **P-16 (L3 memo Rev 5 `:183-184`).** "C0 keeps this line's code (C0 memo §5.1 P-3 lists `core.rs:279` under 'values
  change, code does not')": stale since rev 2 — C0b deletes the getter's `document_root()` fallback (D-1 (c)). Since PM
  273 #583 states its points as decided substance, so this needs no relay (§9 item 6 checks the wording).
- **P-17 (C1 memo rev 3.1, its D-C4 last bullet, A2×A6 row, §3 §13.4 row).** "linked nodes are re-homed through the
  throwaway and back as C0's overwrite note states" and "`finish_detached_fragment`, whose walk (D-C5) maps contents":
  after D-8 / D-10 (rev 3) no fragment-build node is re-homed through the throwaway and the finishing walk is deleted;
  the component is `FragmentParseRoot` on the root, not `FragmentParseDocument` on the throwaway. C1's value claim
  (contents get `f(context's document)`, never `f(throwaway)`) holds more strongly. Told to C1 (H-C1).
- **P-18 (R1 Ax5, ledger `:686-687`).** Rev 2's §7 and DoD 11 called `#11-vm-isconnected-bound-document` and
  `#11-replace-child-self-records` "proposed"; both are registered (PM 227, ledger `:686`, `:687`).
- **P-19 (L3 memo at `a164ffe0`, rev 3's C0 facts not yet folded; PM 258).** (a) `EcsDom::adopt` attributed to C0c
  (`:92`, `:291`, `:1939`, `:2090`, `:2106`): it is C0b's (D-2); `MutationEvent::Adopt` is C0c's, so the adopt-reset's
  hook and trigger stay C0c. (b) the entry `-> Option<Entity>` (`:296`, `:1674`): it returns `NewNodeOwner` (D-10, D-17).
  (c) P1 switch "once C0 lands … it means the C0 program" (`:1932-1934`): the boundary is C0b → P1 (edge c0d), with C0c's
  `Adopt` in the same window (§2.0). (d) §7 (`:1841-1861`) has no `mutation_hook.rs` row although RC-2a adds an arm to
  its exhaustive match, and its `live.rs` RC-0a row lacks "dropped if C0c's prereq did it" (D-14 (d), (e)); L3
  `55d35a16` has since put the `mutation_hook.rs` split in RC-0a. Since PM 273 #583 states these points as
  decided substance with PM IDs, and the C0 program PR that lands after #583 checks the wording (§9 item 6); no relay
  to L3 is needed.

### §0.4 Decisions

Each decision names its PR. Full text here; each per-PR memo restates the parts its review needs (mechanism detail,
cells) and cites this text.

- **D-1 (C0b) — the node document is the stored fact, read by one function.** `EcsDom::node_document` (NEW) `(&self, node:
  Entity) -> Option<Entity>`, in `crates/core/elidex-ecs/src/dom/mod.rs`: a `NodeKind::Document` → `Some(node)`
  (DOM §4.4: "The node document of a document is that document itself"); any other node → its `AssociatedDocument` if
  that is a live Document, else `None`. (a) `owner_document` stays the `ownerDocument` getter's contract (`None` for a
  Document, else `node_document`); **both tree-position fallbacks of H-3 are deleted**. (b) Its production callers
  become exactly two: the getter (`node_methods/core.rs:279`) and `same_document` (`selection.rs:618`, not touched,
  §7). (c) The getter's `.or_else(|| dom.document_root())` is deleted. *Side-store rule:* already a component
  (`AssociatedDocument`, `components.rs:595`). *Idiom table:* OO engines' per-node document pointer → the component,
  written by creation (D-10, D-17) and by the one adopt function (D-2), read by `node_document`.
- **D-2 (C0b; the event is C0c's) — `EcsDom::adopt(&mut self, node: Entity, document: Entity)` (NEW) is DOM §4.5
  *adopt*.** Step 1 oldDocument = `node_document(node)`; step 2 if `node` has a parent, remove it through the
  primitives' removal path (`detach_with_hook`, firing `MutationEvent::Remove`); step 3, if `document` ≠ oldDocument,
  the walk of D-3. A `None` oldDocument re-homes **silently** (no spec state corresponds to a node without a node
  document). Home: `crates/core/elidex-ecs/src/dom/tree/mutation.rs`. Callers: the three primitives (D-4), the
  session's fragment arm (D-5), and from C0c `apply_adopt_node` (D-13). C0c adds one `MutationEvent::Adopt` after the
  walk (D-6); C0b ships no event variant (no consumer-less event, PM 244).
- **D-3 — the adopt walk.** `adopt_subtree` moves to `mutation.rs` and keeps its walk (`for_each_uncapped_shadow_inclusive(root,
  TemplateContentsWalk::Include, …)`). **(a) (C0b)** sets every visited node's `AssociatedDocument` to `document`
  (step 3.1; for template contents, HTML §4.12.3's adopting steps with the identity target under C2; C1 replaces that
  one edge with `f(document)`, C1 memo D-C5); **(b) (C0b)** re-homes each visited element's materialized Attr entities —
  its `AttrEntityCache` entries (step 3.3.1, H-17, H-23); **(c) (C0c)** records a contents segment `{ fragment, documents:
  Option<(old, new)> }` per contents fragment reached, `None` when `old` is `None` or equals `new`, or when the template's
  own pair is `None` (C0c memo D-3 (c)).
- **D-4 (C0b) — the primitives adopt at the spec's position.** Each of `append_child`, `insert_before`, `replace_child`
  computes `target = node_document_for_new_nodes(parent)` — the parent's node document, through the one redirection
  of D-10 — and in place of its bare `detach_with_hook(child, parent)` runs D-2: the removal, then the walk if `target`
  is `Some` and differs, then `link_node` and `fire_after_insert` as today. If `target` is `None` the child keeps its
  stamp. For `replace_child` the adopt of `new_child` precedes `old_child`'s removal (*replace* step 6 before 7, H-10).
- **D-5 (C0b) — replace with a DocumentFragment adopts the fragment** (*replace* step 6): the session's fragment arm
  (H-8) calls `EcsDom::adopt(fragment, d)`, `d = node_document(parent)`, before `remove_child(parent, old_child)`, and
  carries the arm's "step 6" comment. Pre-insert of a fragment adopts only the children (*insert* 7.1).
- **D-6 (C0c) — `MutationEvent::Adopt` (NEW) carries each adopted node's (old, new) pair.** `Adopt { node: Entity,
  documents: Option<(Entity, Entity)>, contents: &'a [AdoptedContents] }`, `AdoptedContents { fragment: Entity,
  documents: Option<(Entity, Entity)> }` (NEW); one event per adopt call, after the walk and before the root is linked,
  iff the root's pair is `Some` (adopt step 3's guard). Consumer: the custom-element consumer's `Adopt` arm walks the subtree with the spec-ordered
  `Include` walk (D-19) and enqueues `CustomElementReaction::Adopted` for each `CEState::Custom` element whose pair is
  `Some` (adopt 3.3.3); the VM flush invokes `adoptedCallback(old, new)`. The exhaustive `match` in `range/live.rs` and
  the test `match` in `mutation_hook.rs` gain explicit no-op arms. *Batch equivalence* and the idiom table: C0c memo.
- **D-7 (C0b) — every node-document derivation reads `node_document`.** (a) N1's clause (b) proxy becomes
  `dom.node_document(target).is_some_and(|d| d != bound_document)`; (b) `owner_doc` deleted; (c) the two
  `unwrap_or(x)` idioms → `node_document(x)`; (d) `focus/delegate.rs:85`, `tree_clone.rs:208`,
  `node_methods/clone.rs:242`, `teardown.rs:336` read `node_document` (or are deleted, D-8); (e) `selection.rs` not
  changed.
- **D-8 (C0b) — the fragment parse returns nodes already in the context's document; no finishing re-home.**
  `finish_detached_fragment(root, document)` snapshots root's children, destroys root and the throwaway, and returns the
  children; its `adopt_subtree(root, doc)` call (`teardown.rs:336-337`) and its `context` parameter are deleted. Every
  node the build linked under root or below already carries the redirect document of D-10 (A5), so HTML §13.4 *HTML
  fragment parsing algorithm* (`#html-fragment-parsing-algorithm`) step 22 ("Adopt fragment into target's node
  document") is the identity for elidex as it is in the spec when no sanitizer configuration is given (that algorithm's
  step 14 creates the fragment in target's node document).
- **D-9 (program) — the umbrella's §1.2 row: one row for the program.** C0a's PR (the first to land) adds **one** row
  naming C0a / C0b / C0c with the order and the three PR names, and a changelog entry; C0b's and C0c's PRs each update
  only their own clause of that row; an edge row into C1 / T3 / L3 is added by the dependent PR, except x9 (C0 → T3),
  which C0a adds under PM 271. **C0a's PR also
  rewrites the §1.2 order line** to PM 153's order with C0 as the program (PM 227 O-1: "the first C0 PR rewrites the
  order row to PM 153's text"), and writes that rewrite's immediate consequences elsewhere in the umbrella — the sentences
  the new order line or an already-made decision (PM 123 / 153 / 227 / 244) contradicts, including the edges into C0
  (x10 N1 → C0) and out of it into T3 (x9 C0 → T3) — under **PM 271, amending PM 223 O-1** (R-3's table E1–E13; no T3
  design content). Revision-number rule (unchanged from rev 2): the next number after `main`'s current one, assigned
  at rebase. Exact texts: §8 R-3.
- **D-10 (C0b) — one new-node owner entry, with the §13.4 redirection as its only special case.**
  `EcsDom::node_document_for_new_nodes(&self, source: Entity) -> NewNodeOwner` (NEW) (`NewNodeOwner`: D-17): the node
  document a node created on behalf of `source`, **or inserted under `source`**, receives — `node_document(source)`,
  except when `source` carries `FragmentParseRoot { redirect_document: Option<Entity> }` (NEW component, on the §13.4
  fragment-parse root only), when it is `redirect_document`.
  - *Why the root, on the current §13.4* (`webref body html html-fragment-parsing-algorithm`, run 2026-11-03): step 4
    creates the parser's document (the throwaway); step 11 creates `root` given that document and step 12 appends it;
    step 14 creates `fragment` "given document if configuration is non-null; otherwise target's node document"; step
    15 sets the parser's **insertion target redirection map[root] = (fragment, null)**; step 22 adopts fragment into
    target's node document; step 23 returns it. In §13.2.6.1 (`webref body html creating-and-inserting-nodes`) the
    *adjusted insertion location* returns the map's value for root, so every insertion into root lands in the fragment;
    *insert a character* and *insert a comment* create their node "given targetParent's node document" of that
    adjusted location (the fragment's: the context's document); *create an element for the token* step 3 takes
    "intendedParent's node document", and *insert a foreign element* step 1 passes "the target parent of the
    appropriate place" — **not** adjusted — so a top-level element is created in the throwaway and adopted into the
    fragment's document when step 2 inserts it (DOM *insert* 7.1), before any child of it is created; every deeper
    element's intendedParent is already in the context's document.
  - *The elidex encoding.* elidex keeps root as the build container (the bottom of the stack of open elements and the
    list it returns, D-8) and has no separate fragment object, so root **stands for the redirect target**: the
    component on root carries the fragment's node document, and the one entry answers it both for a node created on
    behalf of root (text, comments; top-level elements) and for a node linked under root (D-4's target). Result: every
    node of a build that is root or below root, except root, holds the context's document from its creation; no link
    adopts into the throwaway. *Scope:* the one build path that creates outside root is the strict tier's frameset
    fragment deviation (H-25): its comment is created in and appended to the throwaway itself, is never returned, and is
    orphaned (not despawned) when `destroy_entity(document)` (`teardown.rs:341`) destroys the throwaway; the spec never reaches that mode in the fragment case (proposed `#11-strict-frameset-fragment-mode`,
    R-6 (g)).
    The one difference from the spec's object graph — a top-level element (and a top-level template's contents) never
    holds the throwaway (resp. its template contents owner document) — is unobservable: in the fragment case *create an
    element for the token* sets willExecuteScript false (step 13: "definition is non-null and the parser's fragment
    context element is null"), *insert an element at the adjusted insertion location* pushes and runs no element queue
    (steps 5 and 7, "If the parser's fragment context element is null"), and elidex builds with the dispatcher taken
    (H-11). Under C1 this is what makes `f` never computed for a throwaway (C1 memo D-C4, A7, U4): a template created
    on behalf of root or of any build node gets the context's document, and no link ever walks a template into the
    throwaway.
  - *Lifetime.* `begin_detached_fragment(&mut self, context: Entity)` creates the throwaway, creates root with
    `node_document_for_new_nodes(throwaway)` (= the throwaway, step 11), appends it (step 12; the strict rollback's reach,
    H-11), then inserts `FragmentParseRoot { redirect_document: node_document(context) }` on root (step 15; for a
    shadow-root target its host is `context`, and after D-11 the shadow root's node document is the host's, so
    step 14's "target's node document" equals it). The component dies with root (both teardowns despawn it).
    *Side-store rule:* a per-entity fact about root, `Send + Sync` → a component. *Rejected:* rev 2's
    `FragmentParseDocument` on the throwaway (it substitutes at creation only, so a link under root re-homed the build
    to the throwaway and back, and a tolerant bottom-up link walked whole subtrees, template contents included, into
    it — R1 Ax2 IMP-2); a parameter on every creation (dead at every non-parser caller); an `EcsDom` field.
  - *C1's hook point.* The entry has no template branch: inside contents `source` is the contents fragment or a
    descendant, whose stamp `attach_template_contents` wrote; every caller of `attach_template_contents(template,
    owner)` passes `node_document_for_new_nodes(template)` (the clone post-pass the pair's document); C1 D-C3 applies
    `f` inside the helper.
- **D-11 (C0b) — every production node-creation site takes its owner from the entry.** The census is a population,
  defined by property and enumerated by command (§4.4), not a fixed list: every production call of an `elidex-ecs`
  node creator. Each passes `node_document_for_new_nodes(source)` with `source` = the node the spec creates it "given …
  node document" of (the C0b memo §4.4's `source` column for the `27b50dfe` snapshot), or, for a clone, the operation's document
  (`clone_node_shallow` gains a `document` parameter, DOM *clone a single node* "given document"; P-2). Shadow roots
  are stamped from the host (*attach a shadow root* step 5); Attr entities and doctypes gain an owner. The custom-element constructor's owner decision lives in
  `elidex_custom_elements::spawn_custom_element_entity(dom, …, document: Option<Entity>) -> Option<Entity>` (the entry
  over the current global's associated Document); the VM passes the marshalled `document_entity_opt()`, and a `None`
  document (VM not bound: before bind and after unbind, `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:477-483`) spawns nothing — the VM throws a `TypeError` —
  because an ownerless `NewNodeOwner` cannot be built outside `elidex-ecs` (D-17) and a node without a node document
  has no spec state (D-2). Any production
  creation site present at C0b's base — including L3 RC-2b's contents-algorithm sites if RC-2b lands first (§4.5 A6) —
  is in the population and is migrated by C0b; the argumentless creators no longer exist for production code (D-17), so
  a site left unmigrated does not compile.
- **D-12 (C0a) — the *replace* mutation records follow the current step order (PM 160 (a)).** The non-fragment arm
  captures the source-removal record's siblings **without** stepping over `old_child` (adopt step 2 runs at replace
  step 6, before step 7 removes `old_child`), through `capture_move_source(dom, new_child)` (`mutation/mod.rs:304`, the
  move-source builder append and insert already use); every replace step comment in `apply_replace_child`, the dom-api
  `ReplaceChild` handler and the replace tests is renumbered to the current 1–11 (validity: "replace step 1 (ensure
  pre-insert validity step 2 / 3)", `mutation/mod.rs:691`, `:700`; per-line table: C0a memo D-12), except the fragment
  arm's step 6, which arrives with its code in C0b (D-5). Records: count and order unchanged; only the source record's
  siblings change (T17, T17b, U14, U14c). This reverses B1.2a's step-over (#384 `c3bdb6f9`; C0a memo P-20).
- **D-13 (C0c) — `document.adoptNode(node)` (PM 158; `[CEReactions]`).** DOM §4.5 steps 1 Document →
  `NotSupportedError`, 2 ShadowRoot → `HierarchyRequestError`, 3 adopt node into this, 4 return node (no exclusion for a
  host-linked fragment: template contents are adoptable). VM native (marshalling) → dom-api `AdoptNode` (NEW) → session
  `apply_adopt_node(dom, node, document) -> Vec<MutationRecord>` (NEW) = the step-2 source-removal record
  (`source_removal_record`, H-7) then `EcsDom::adopt`; the dom-api handler pushes the records. Reactions drain where every
  `[CEReactions]` method's do today (`VmInner::flush_ce_reactions`, `flush.rs:39`, at the script-execution and event
  checkpoints `crates/script/elidex-js/src/vm/interpreter.rs:54`, `crates/script/elidex-js/src/engine.rs:165`), not at
  method return — the engine-wide gap is L3's proposed `#11-ce-reactions-method-scope-drain` (L3 memo §9; PM 224).
  `importNode` stays out (§7).
- **D-14 — prerequisite pure-move splits (CLAUDE.md touch-time split).** (a) **before C0a**:
  `crates/script/elidex-script-session/src/mutation/mod.rs` (994; C0a, C0b and C0c edit it) — the child-list family
  (`:296-895`) → `mutation/child_list.rs` (NEW), `pub` items re-exported. (b) **before C0c**:
  `crates/script/elidex-js/src/vm/host/document.rs` (1197) — the node factories (`:278-421`) →
  `vm/host/document_factories.rs` (NEW) and the accessors (`:423-938`) →
  `vm/host/document_accessors.rs` (NEW). (c) **before C0b** (PM 258): `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs`
  (994; C0b adds `create_table_element` and edits eight sites) — its *DomApiHandler bindings* section (from the section
  rule at `:624` to the end, before `mod tests;`) → `table_mutation/handlers.rs` (NEW); the algorithms stay in `mod.rs`
  (≈ 625 lines). (d) **before C0c, unless L3 RC-0a has done it**:
  `crates/dom/elidex-dom-api/src/range/live.rs` (1021; C0c adds an `Adopt` arm) — its inline test module (from `:498`)
  moves to a sibling file; this is the move L3 RC-0a plans (L3 memo §7, `live.rs` row), so whichever of RC-0a and C0c's
  prereq lands first does it and the other drops it (L3's row does not say so yet, P-19 (d)). (e) **before C0c, unless
  L3 RC-0a has done it**: `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` (1143; C0c and RC-2a each add an arm
  to its exhaustive match) — the specialised-hook scenarios (from the section rule at `:973`, "after_remove descendants
  snapshot", to the end) move to a sibling test file; whichever of L3 RC-0a and C0c's prereq lands first carries the
  split (L3 #583 `55d35a16` moved it into RC-0a's scope, so RC-2a starts from the split file; P-19 (d)). Each verified as a
  pure move (`git diff --stat`, `cargo test -p <crate>` green).
- **D-15 ((i), (ii), (iv) on this branch; (iii) in C0b; all made true by C0b) — the IBP documents name
  `node_document`.** (i) The node memo's `effective_node_document` → `node_document`, in place, with the dated amendment
  paragraph (PM 157), and its cite restated (P-14). (ii) The node input's five rows (`:12`, `:67`, `:68`, `:102`,
  `:155`) → `node_document`, with a dated note. (iii) The parent's F10 cell (`docs/plans/2026-09-inline-box-predicate.md:1180`,
  "the effective-node-document rule (`elidex-script-session/src/scripting.rs:96-111`) promoted unchanged") is not
  rewritten; one amendment line follows its table, carried by **C0b's PR** (PM 244 Q4: the PR that makes it true), text
  in the C0b memo §8: a rule **replacement** (the composed-root proxy → the stored node document), not a rename. (iv)
  The N1 memo's n3 hand-off ("promotes the effective-node-document rule into `EcsDom::effective_node_document` (N4
  surface)", `docs/plans/2026-10-ibp-sandbox-handler-gates.md:961-962`) is superseded; the N1 memo is landed and not
  edited — the supersession is stated in the node memo's amendment paragraph, which is what N4 reads (the precedent:
  the node memo's own 2026-10-25 erratum and PM 17 (i) paragraphs). (i), (ii) and (iv) land with the program memo (PM
  157: the node memo is amended in place inside C0's plan); they mark `node_document` NEW (planned), and the amendment paragraph names C0b as the PR it lands in.
- **D-16 (program) — `isConnected` is not covered** (PM 161): slot `#11-vm-isconnected-bound-document` (registered,
  PM 227, owner L3).
- **D-17 (C0b; Q-3 decided) — ownerless creation is test-only by construction.** (i) The production creators take
  `owner: NewNodeOwner` (NEW; `pub struct NewNodeOwner(Option<Entity>)`, field private, no public constructor):
  `create_element_with_owner`, `create_element_ns`, `create_text_with_owner`, `create_comment_with_owner`,
  `create_document_fragment_with_owner`, `create_document_type`, `create_attribute`, `attach_template_contents`. A
  `NewNodeOwner` is obtained only from `node_document_for_new_nodes` (pub) or inside `elidex-ecs` (`pub(crate)`: the
  cloner, the parse root). So production code cannot ask for an ownerless node; it can only name a source, and a node is
  ownerless only when that source has no node document. (ii) The four argumentless creators (H-22) stay on `EcsDom` for
  fixtures but are compiled only under `#[cfg(any(test, feature = "fixtures"))]` (NEW `elidex-ecs` feature). Each crate
  that depends on `elidex-ecs` and calls them in its tests enables the feature **in `[dev-dependencies]` only**
  (`elidex-ecs = { workspace = true, features = ["fixtures"] }`; the population and its command: C0b memo D-17 (3)); no
  `.rs` file changes for it. (iii) **The CI-run census**
  (PM 244 item 5), split by PM 255 (user decision γ) into two PRs: **(a) the CI/tools default-features check PR**
  (before C0b, owned by the CI/tools lane; it has value alone) adds the step "Clippy (default features, no dev-only
  feature)" — `cargo clippy` over the workspace with **default features** and `-D warnings` (PM 263), so lib and bin
  targets, where dev-dependencies and their features are not active — to CI's `check` job and to `mise run ci`, and
  writes the exception into CLAUDE.md's "cargo tasks gate on `--all-features`" line (*Commands*): `--all-features`,
  which every other job uses (`.github/workflows/ci.yml:86`, `:89`, `:108`), turns every feature on for every target
  and so cannot see a test-only boundary. (A default-features `cargo check --workspace`, the build that step compiles,
  was green at `27b50dfe`, run 2026-11-03, exit 0; the step's own verdict is the CI/tools PR's.) A production call of
  an argumentless creator (a site C0b missed, or RC-2b's `create_document_fragment()` rebased onto C0b) fails this
  build: it does not compile. It depends on no list of operations. **(b) C0b** adds `scripts/check-fixture-boundary.sh`
  (NEW) and runs it as its own step directly after that step, and adds no `mise` task and writes no CLAUDE.md line (C0b memo D-17 (4); PM 255, 297). Its claim (PM 266): *`fixtures` is enabled from `[dev-dependencies]` only*. It names `fixtures` and reads
  the population from cargo, not from the manifests' text: it fails if `cargo tree --workspace -e normal,build,features
  -i elidex-ecs` lists `elidex-ecs feature "fixtures"` (H-24: that tree has no dev edges, and a dev edge does show there
  under `-e normal,build,dev,features`, so the check is not vacuous; the host-target tree is complete for
  `fixtures`, H-24). The generic guard: R-9. **Scope of the guarantee (A8):** the default-features build of the workspace (every
  product feature some member enables is in it) with `fixtures` off its normal edges; code compiled only under a
  product feature that no member enables is outside it — at `27b50dfe` only `elidex-style/parallel`, whose 4 `cfg`
  sites create no node (H-24). *Rejected:* migrating the ~2,577
  fixture call lines onto the entry (a fixture needs no document, so the lines whose outcome reads no node document gain
  nothing; the tests whose assertions do read an ownerless node's node document pin a state production cannot reach after
  C0b and are found and rewritten by C0b's pre-implementation run, C0b memo §5.2 / §9); a dev-dependency-only fixture crate
  with an extension trait (production cannot see a dev-dependency in any build, but every one of 296 test files (`git grep -l -E '\.create_(element|text|comment|document_fragment)\(' 27b50dfe -- 'crates/*.rs' | wc -l`, run 2026-11-03) gains an
  import — 12 of them over 1000 lines, each a touch-time split — and the crate still needs a pub, feature-gated door
  into `elidex-ecs`, which `--all-features` exposes, so it needs the same kind of CI guard); a grep ratchet over
  non-test code (rev 2's P-8: inline `#[cfg(test)]` modules make the population fuzzy).
- **D-18 (C0b; Q-4 decided) — one writer module, one reader.** `adopt_subtree` becomes a private helper of `adopt`
  (its one caller after D-8); `set_associated_document` and `get_associated_document` become `pub(crate)`;
  the type `AssociatedDocument` becomes `pub(crate)` and leaves `elidex-ecs`'s re-export (`lib.rs:30`), so outside the
  crate no `insert_one`, `get::<&mut _>`, copy or `remove_one::<AssociatedDocument>` names it — closed by construction,
  with no ratchet to keep (outside `elidex-ecs` the type appears only in comments and two intra-doc links,
  `git grep -n AssociatedDocument -- 'crates/*.rs' ':!crates/core/elidex-ecs'`, which C0b rewrites; `cargo doc` with
  `-D warnings` catches a missed link). The dom-api pass-4 hand stamps (`node_methods/clone.rs:282-285`, `:294-301`) are dead at C0b's head (the shadow
  root is stamped by `attach_shadow_with_init` from its host, the cloned host and each shallow clone by the shallow
  cloner) and are removed per edge c0g (§8). Writers after C0b, all inside `elidex-ecs`: the creators (D-17), the adopt walk
  (D-3), the cloner (`tree_clone.rs:221`, `:230`, `:417`). Readers outside `elidex-ecs`: `node_document` and `owner_document` only. No
  residual: the earlier `world_mut().remove_one::<…>` hole closes with the type's visibility.
- **D-19 (C0c) — one walker, in the spec's order.** `TemplateContentsWalk::Include` visits a template's contents
  **immediately after the template node**, before its light children (adopt step 3.4 runs inside the per-descendant
  loop), instead of after its subtree (H-16). Its two other callers are order-insensitive (H-16), so the change is
  unobservable to them; the `Adopt` consumer uses the same walk. *Rejected:* a second walker or an order variant (two
  walks of one tree-order concept, one-issue-one-way).
- **D-20 (C0b; Q-2 decided) — the tolerant converter calls the entry per node.** `convert_node` gains the converted
  parent (`convert_node(handle, parent, dom, opts)`; call sites `convert.rs:74` with `parent`, `:103` with `root`), and
  each creation passes `node_document_for_new_nodes(parent)`; the threaded `owner_document` parameter of
  `convert_children` / `convert_fragment_top_level` / `try_attach_declarative_shadow` / `convert_node` and
  `lib.rs:158`'s `owner_doc` are deleted. "Intended parent" (§13.2.6.1) = the converted parent for an RcDom conversion
  (html5ever has already placed each node). Order stays bottom-up: with D-10 every created node already holds the
  context's (or the parsed document's) node document, so each bottom-up link is a no-op compare and the final link
  under root is one too.

## §1. Program shape

**Axes (§2): 9** — A1 the stored fact and its reader, A2 the insertion position, A3 the walk, A4 the adoption
notification, A5 fragment and DOMParser builds, A6 node-document consumers, A7 the EH program's contract, A8 node
creation, A9 mutation records. ≥ 3 intersecting axes: edge-dense (PM 143); each PR's plan-review is mandatory (CLAUDE.md
*Edge-dense work* (b)); each per-PR slice, once its plan-review passes under this program, is a terminal unit (base case
(c)).

**The cut (PM 244, from R1 Ax3 + Ax5).**
- *C0a = D-12 alone.* The record layer already sees the primitive detach `new_child` before `old_child` (H-10); the fix
  is the session's sibling capture, independent of adoption (A9 meets the core only at A2×A9).
- *C0b = the node-document fact, unsplittable* (the base-case judgement C0b's plan-review makes). The candidate cuts
  and why each fails: (1) **creation first** (stamps at creation, reader collapsed, no adoption): every moved node keeps
  its creation document — T3's moved parser node reads the old document where HEAD's position proxy reads the new one,
  worse than HEAD (K-0). (2) **adoption first** (stamps maintained at insertion, fallbacks and proxy kept): K-0 holds
  (unstamped nodes still read HEAD's fallbacks), but for that PR's lifetime the node document has two derivations — the
  stored fact for stamped nodes and the position proxy for the rest — which one-issue-one-way forbids, and the next PR
  would still have to land creation and the reader collapse together (cut (1)'s coupling). (3) **reader collapse
  last**: the same two-derivation period as (2). So creation, maintenance, the reader collapse and the consumers' migration land together; the 23 pairs are the creation, maintenance and
  reads of one fact.
- *C0c = notification + `adoptNode`.* Without D-6, `adoptedCallback` is never called — HEAD's behaviour (H-12) — so K-0
  holds at C0b's head and no consumer-less event ships (the `Adopt` variant arrives with its consumer). The notification
  reads C0b's fact one-way (§2.2 A1×A4). *Why one PR, not notification → `adoptNode` (a design call; the coupling is
  ordering, not mechanism):* `adoptNode` before the notification would ship a new author-visible `[CEReactions]` method
  without its callback (T15); after it, a separate PR is possible but carries no mechanism of its own — one session
  function over C0b's `adopt` plus marshalling (D-13) — and one pair of its own, A1×A9 (T16); its pair with the
  notification, A4×A9, is pinned by T15, which needs both halves. PM 158
  put `adoptNode` in the program as the adopt algorithm's own entry. A fourth per-PR memo and review would buy no
  smaller decision surface.

**Order.** N1 (landed, `27b50dfe`) → **C0a** → **C0b** → **C0c** → C1 → T3 → slice b. C0a may land any time before
C0b; C0b after C0b-pre1 = D-14 (c) and the CI/tools default-features check PR #585 (edges c0-pre-c,
c0-pre-ci); C0c after #583 (edge c0h, PM 311) and any time after C0b
and before C1. RC-2a is C1's prerequisite (PM 249 Q-Z4; L3 orders RC-2a before RC-1), in either order with C0b (edge
c0g) and C0c (edge c0e). Downstream needs: C1's template-contents value (D-C3, D-C4) needs C0b; C1's
per-segment `adoptedCallback` pairs (D-C5's notification half, T25) need C0c; T3 needs C0b (`node_document`); L3 RC-2 /
P1 need C0b (the entry, §2.0's boundary). C1's own edge text is fixed by C1's driver (H-C1).

C0b-pre2 (D-21) was dropped (PM 300): it would deepen the dormant dom-api Attr model that #433 rejected; the `setAttributeNode` gap → `#11-dom-api-attr-entity-model-unification` (PM 300).

**K-0 lens (umbrella K-0: no step worse than HEAD).** C0a: records only, each change toward the spec. C0b: every change
of the C0b memo §5.4 moves an outcome from HEAD's to the spec's; `adoptedCallback` stays uncalled as at HEAD. C0c: adds
the callback and `adoptNode` where HEAD has none, with one recorded, bounded regression (C0c memo §1, §8; PM 311).

### §1.1 Browser measurement

None is needed for the program's decisions (every outcome in §6 of the per-PR memos is defined by DOM §4.2.3 / §4.4 /
§4.5 / §4.9 / §4.11 / §5.5 and HTML §4.2.3 / §4.9.1 / §4.9.5 / §4.9.8 / §4.12.3 / §4.13.6 / §13.2.6.1 / §13.4). The
one spec-vs-browser question nearby (`replaceChild(X, X)`) is a registered slot whose measurement procedure is in the C0a memo §7 (program §7 row).

## §2. Coupled invariants (program matrix)

### §2.0 Where adopt sits inside *insert* (the boundary for L3's P1)

DOM §4.2.3 *insert*: **7.1 "Adopt node into parent's node document"**; 7.2 / 7.3 link; 7.4–7.6 slots; 7.7 insertion
steps and `connectedCallback`; 8 the record; 9 children changed; 10–12 post-connection. *Adopt* (§4.5): 2 remove from
the old parent; 3 per shadow-including inclusive descendant: 3.1 node document, 3.3.1 attributes, 3.3.3
`adoptedCallback`, 3.4 adopting steps. *Replace*: 6 adopt node before 7 remove child (suppressed) and 9 insert.

| elidex step (after C0c) | spec step | event / record | PR |
|---|---|---|---|
| `is_connected` capture | input to 7.7 | — | — |
| removal from the old parent (`detach_with_hook`) | adopt 2 | `Remove` (old parent); session source record (H-7) | — |
| re-home (`adopt` walk): stamps, attrs | adopt 3.1, 3.3.1, 3.4 (C2 identity) | — | C0b |
| contents segments; `MutationEvent::Adopt` | adopt 3.3.3, 3.4 hook | iff the root's pair is `Some` | C0c |
| `replace_child` only: `old_child` removal | replace 7 | `Remove` (old child) | — |
| `link_node`, `rev_version`, `fire_after_insert` | insert 7.2–7.7, 8, 9, 12 | `Insert` | — |

**The contract P1 inherits** (from C0b): adoption (stamps written) is complete before `link_node`, so before every
insertion-steps, children-changed, post-connection and `connectedCallback` consumer and the destination record, and it
follows the old parent's removal. From C0c, `Adopt` is dispatched in the same window. P1 may split `fire_after_insert`
into hook phases; it must not move the adopt after any of them. The adopting-steps hook is `MutationEvent::Adopt` (one
per adopt call, per-node pairs), not a per-descendant hook.

### §2.1 Invariant per axis

| axis | invariant | PR |
|---|---|---|
| A1 fact | **I-U:** for every node n with a parent or host p **whose `node_document_for_new_nodes(p)` is `Some`**, `node_document(n)` equals it — the parent's node document, except under the fragment-parse root (D-10), which exists only during a build; template contents are not children (their fragment is stamped by `attach_template_contents` and by the walk's contents edge). Under an ownerless p the child keeps its stamp (D-4, U3): reachable only from fixtures and below a node whose source had no node document (A5×A8). Each element's materialized Attrs share its node document. `node_document` reads only the stamp; no production reader derives a node document from tree position or `document_root()` | C0b |
| A2 position | At every link the child's adoption completes after its old-parent removal and before `link_node`; in `replace_child` before `old_child`'s removal; a replaced-in fragment is adopted itself; a pre-inserted fragment keeps its node document | C0b |
| A3 walk | Adoption re-homes the root's shadow-including inclusive descendants, each element's cached Attr entities and each reached template's contents (C2: same document), uncapped; from C0c in spec order (D-19), recording each contents fragment's pair | C0b, C0c |
| A4 notification | `Adopt` fires once per adopt call iff the root's pair is a (live, different) pair, before that root's `Insert`; every custom element whose pair is `Some` gets exactly one `Adopted` with its own pair, in spec order, before its `Connected` | C0c |
| A5 builds | No node of a fragment build other than its root ever holds the throwaway document; the build's returned nodes carry the redirect document from creation; `FragmentParseRoot` exists only during a build; DOMParser builds stamp their document | C0b |
| A6 consumers | Every production node-document read goes through `node_document`, or `owner_document` at exactly two sites; no position-based or "Document → itself" derivation remains | C0b |
| A7 EH contract | N1's gate and T3's step 3 read `node_document`; slice a's Insert arm sees the adopted document; C1 edits only `attach_template_contents` and the walk's contents edge (and, from C0c, each segment's `new`); K-0 holds at each PR's head | C0b, C0c |
| A8 creation | Every production creation takes a `NewNodeOwner` from the entry (or a clone's document); no lib or bin target can call an argumentless creator — scope: the workspace's default-features build, with `fixtures` off its normal edges (D-17 (iii); code under a product feature no member enables, at `27b50dfe` only `elidex-style/parallel`, is outside it and creates no node); a node is ownerless only if its source has no node document | C0b |
| A9 records | Replace's source-removal siblings are captured with `child` in place; `adoptNode` of a parented node queues exactly the *remove* record; adoption itself queues none | C0a, C0c |

### §2.2 All pairs (36), with intersection and owning PR

| pair | intersect? | intersection | PR |
|---|---|---|---|
| A1×A2 | yes | I-U is established by D-4 at every link (H-2); a primitive that linked before adopting would let an `Insert` consumer read the old stamp (U4) | C0b |
| A1×A3 | yes | I-U across the host edge needs a shadow-including walk; the contents edge's stamp comes from the same walk (U1, T4) | C0b |
| A1×A4 | yes | Each pair's `old` is a stamp read before the walk, `new` the stamp written; the consumer reads no tree position | C0c |
| A1×A5 | yes | Build nodes are created and linked in the redirect document (D-10); the throwaway holds only root; the strict rollback despawns everything | C0b |
| A1×A6 | yes | Deleting the fallbacks is sound only if every script-reachable node is stamped: I-U for linked nodes, D-11 / D-17 for creations (T20–T23, U8, U12) | C0b |
| A1×A7 | yes | N1's node clause becomes `node_document(target) != bound`; equal to the proxy's answer for a connected node, the last adoption's for a detached one (T8–T10) | C0b |
| A1×A8 | yes | Creation is the stamp's first writer; adoption its only later writer (D-18 by visibility) | C0b |
| A1×A9 | yes | `adoptNode`'s record is the removal's; the stamp change is no record | C0c |
| A2×A3 | yes | The walk runs between removal and link, on a detached subtree | C0b |
| A2×A4 | yes | `Adopt` precedes `Insert`; in `replace_child`, `Adopt(new_child)` precedes `Remove(old_child)` (T7 variant) | C0c |
| A2×A5 | yes | During a build each link's target is the redirect document, equal to the child's creation stamp: a compare, no walk (D-10, U9) | C0b |
| A2×A6 | yes | The base-URL maintainer's `Insert` arm reads `node_document(node)` after the re-home (T12) | C0b |
| A2×A7 | yes | Slice a's T-a2 runs inside `Insert`, after D-4's adoption (T8) | C0b |
| A2×A8 | yes | A node created under `source = p` and linked under `p` is a no-op adopt; under an ownerless parent it keeps its stamp (U3) | C0b |
| A2×A9 | yes | The record layer captures the source record before calling the primitive (H-7); D-12 changes which siblings it reads, not when | C0a |
| A3×A4 | yes | The consumer visits the walk's node set in spec order (D-19), each node with its segment's pair (T18, U13) | C0c |
| A3×A5 | yes | No build link walks a subtree into another document (D-10), so the contents edge never meets the throwaway | C0b |
| A3×A6 | yes | Every consumer reads the node's own stamp (attrs included); none walks up | C0b |
| A3×A7 | yes | A handler on a shadow-tree element of a moved host reads the new document through its own stamp; C1 replaces only the contents target | C0b |
| A3×A8 | yes | Attr entities are stamped at creation and re-homed by the walk while attached (U1 Attr variant, U16) | C0b |
| A3×A9 | no | The walk queues no record | — |
| A4×A5 | yes | Builds run with the dispatcher taken and adopt nothing (D-10), so no `adoptedCallback` comes from a build (U9) | C0c |
| A4×A6 | no | `Adopt` has one consumer; node-document consumers read the stamp | — |
| A4×A7 | yes | `adoptedCallback` runs at the next reaction checkpoint, after the insertion completes; slice a's processing is inside `Insert` | C0c |
| A4×A8 | yes | After C0b a script-reachable custom element always has a node document, so the silent re-home cannot swallow an `adoptedCallback` | C0c |
| A4×A9 | yes | `adoptNode` fires `Adopt` like an insertion's adoption; removal record, then `Adopted` (T15, T16) | C0c |
| A5×A6 | yes | A DOMParser document's nodes read it through their stamp; after a move the receiving document (T1) | C0b |
| A5×A7 | yes | N1's x4 hand-off (raw values at DOMParser-node creation read null) is unchanged | C0b |
| A5×A8 | yes | The redirect is the entry's only special case and exists only while `FragmentParseRoot` exists; a documentless context yields ownerless creations, as HEAD's orphan result | C0b |
| A5×A9 | yes | `parse_into_inert_document` drops its records; unchanged | C0b |
| A6×A7 | yes | N1's clause (b) and the node memo's D8 rule are one function (D-15); T3 reads `node_document` | C0b |
| A6×A8 | yes | The getter's fallback is deleted on the strength of D-17's compile-time guarantee | C0b |
| A6×A9 | no | Node-document consumers read no records | — |
| A7×A8 | yes | Slice b processes raw values at creation; the creation stamp is what N1's gate reads there | C0b |
| A7×A9 | no | The EH contract reads no record content | — |
| A8×A9 | yes | Range creation sites (A1–A4, or RC-2b's A6–A9) take their owner from the entry; their records are the Range code's (§4.5) | C0b |

Count: 36 pairs; 32 intersect, 4 do not. By PR: C0a 1, C0b 23, C0c 8. Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| A[0-9]×A[0-9]/' <memo> | awk -F'|' '{print $3, $5}' | sort | uniq -c`.

## §3. Spec coverage map

The program's union (each per-PR memo carries its own map with its own lookups). Lookups (2026-11-03): `heading dom
4.2.2 / 4.2.3 / 4.2.6 / 4.3.2 / 4.4 / 4.5 / 4.8 / 4.9 / 4.11 / 5.5`; `heading html 2.4.3 / 4.2.3 / 4.9.1 / 4.9.5 /
4.9.8 / 4.12.3 / 4.13.6 / 8.1.8.1 / 8.5.1 / 13.2.6.1 / 13.4`; `body dom concept-node-insert`, `concept-node-adopt`,
`concept-node-replace`, `dom-document-adoptnode`, `concept-range-extract`, `concept-range-clone`, `concept-text-split`;
`body html html-fragment-parsing-algorithm`, `creating-and-inserting-nodes`, `create-a-table-element`,
`dom-tbody-insertrow`, `dom-tr-insertcell`, `set-the-frozen-base-url`, `getting-the-current-value-of-the-event-handler`;
`idl dom Document` (`[CEReactions] Node adoptNode(Node node)`); `body dom concept-element-attributes-append`, `-replace`,
`-remove`, `-set`; `heading html 13.2.6.4.18`, `body html parsing-main-inframeset`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* 7.1; *replace* 1–11; *pre-insert*, *replace all* | adopt before link; replace's step order and record siblings | `crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`, `:95`, `:138`; `crates/script/elidex-script-session/src/mutation/mod.rs:672` (C0a D-12, C0b D-4 / D-5) | ✓ | yes |
| WHATWG DOM §4.2.6 Mixin ParentNode | *convert nodes into a node* | text and fragment given this's node document | `crates/dom/elidex-dom-api/src/child_node/mod.rs:302` (C0b D-11) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 1–3.4; `adoptNode` 1–4 | oldDocument, remove, per-descendant steps; adoptNode's errors | `EcsDom::adopt` (NEW, C0b); `MutationEvent::Adopt` (NEW, C0c); dom-api `AdoptNode` (NEW, C0c) | ✗ (3.2, 3.3.2 §7) | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument`; *clone a single node*; *string replace all* 2 | the stored fact; the getter; clones and texts given document | `crates/core/elidex-ecs/src/dom/mod.rs:817`; `crates/dom/elidex-dom-api/src/node_methods/core.rs:279`; `crates/core/elidex-ecs/src/dom/tree_clone.rs:251`; `crates/dom/elidex-dom-api/src/node_methods/text_content.rs:119-120` (C0b) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *attach a shadow root* 5; `insertAdjacentText` 1 | given element's node document | `crates/core/elidex-ecs/src/dom/shadow.rs:171`; `crates/dom/elidex-dom-api/src/element/tree.rs:431-432` | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | *split a Text node* 5 | new node given node's node document | `crates/dom/elidex-dom-api/src/char_data/split_text.rs:148` (C0b) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | *extract* 1, 4.1, 16.1, 19.1; *clone the contents* 1 | fragment given range's start node's node document; clones | `crates/dom/elidex-dom-api/src/range/mutation.rs:266` (C0b; RC-2b's sites per §4.5) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including tree order* | the walk's order | `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (C0c D-19) | ✓ | no |
| WHATWG DOM §4.3.2 Queuing a mutation record | records of adopt's removal and of replace | siblings per current order | `mutation/mod.rs:341` (C0a, C0c) | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | *connected* | shadow-including root is a document | `walkers.rs:143` (unchanged; D-16) | ✗ (VM getter) | no |
| WHATWG HTML §4.12.3 The template element | *adopting steps*; *template contents* | contents into the appropriate owner (C2: the template's) | walk's contents edge (C0b); `attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557` | ✗ (C1) | yes |
| WHATWG HTML §4.13.6 Custom element reactions | *enqueue a custom element callback reaction* | `adoptedCallback` « oldDocument, document » | consumer arm + flush (C0c D-6) | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for the token* 3, 13; *insert a foreign element* 1–2; *insert a character* 5; *insert a comment* 4; *adjusted insertion location* 3–5 | intendedParent's / targetParent's node document; the redirection | strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`; tolerant `crates/dom/elidex-html-parser/src/convert.rs:237` (C0b D-10, D-11, D-20) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* (`#html-fragment-parsing-algorithm`) steps 4, 11–15, 22–23 | throwaway, root, fragment in target's node document, redirection map[root], adopt fragment | `crates/core/elidex-ecs/src/dom/tree/teardown.rs:293`, `:330` (C0b D-8, D-10) | ✓ | yes |
| WHATWG HTML §13.2.6.4.18 The "in frameset" insertion mode | end tag `frameset`: "If the parser's fragment context element is null and" | strict tier omits the guard (H-25); scopes D-10's claim | `crates/dom/elidex-html-parser-strict/src/tree_builder/modes/in_frameset.rs:31-33` (not changed; R-6 (g)) | ✗ (slot) | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | *parse HTML from a string* | the new document owns its nodes | `crates/dom/elidex-form/src/inert_document.rs:120` (C0b) | ✓ | yes |
| WHATWG HTML §4.9.1 The table element | *create a table element*; `createTHead` and kin; `insertRow` 2–3 | element given tableElement's node document | `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs:116` (C0b) | ✓ | yes |
| WHATWG HTML §4.9.5 The tbody element | `insertRow` 2 | tr given this element's node document | `table_mutation/mod.rs:448` (C0b) | ✓ | yes |
| WHATWG HTML §4.9.8 The tr element | `insertCell` 2 | td given this tr element's node document | `table_mutation/mod.rs:512` (C0b) | ✓ | yes |
| WHATWG HTML §4.2.3 The base element | *set the frozen base URL* 1 | document = element's node document | `crates/dom/elidex-dom-api/src/element/document_base.rs:173` (C0b D-7 (b)) | ✓ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *document base URL* | per node document | `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50` (C0b D-7 (c)) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* (`#getting-the-current-value-of-the-event-handler`) 3.1–3.2 | element's node document | `crates/script/elidex-script-session/src/scripting.rs:97` (C0b D-7 (a)) | ✓ | yes |

### §3.1 User-input touch audit

C0 parses nothing new and stores no string; `Adopt` carries entities only. What changes is which Document an author node
is attributed to at creation and after a move, and four reads with it: `ownerDocument`, URL resolution through the
document base URL, N1's compile gate, and the custom-element reaction arguments. Two new script entry points (C0c):
`adoptedCallback` (the existing reaction flush, error routing as `connectedCallback`) and `document.adoptNode`.

## §4. Shared mechanism records

### §4.1 Home / kind

`node_document`, `node_document_for_new_nodes`, `NewNodeOwner`, `adopt`, `MutationEvent::Adopt`, `AdoptedContents` and
`FragmentParseRoot` live in **`elidex-ecs`**; the argumentless fixture creators stay there behind the `fixtures` feature (D-17).
The session's fragment arm and `apply_adopt_node`, the custom-element consumer and the dom-api `AdoptNode` are
engine-independent; so is the CE constructor's owner decision (`elidex-custom-elements`, D-11). The VM changes are
marshalling only (§4.7): in C0b the CE constructor passes `document_entity_opt()` and maps `None` to a `TypeError`, the
`ownerDocument` getter's docstring (`vm/host/node_methods_extras.rs:36-40`) and the cells file
`vm/tests/tests_insertion_adoption.rs` (NEW); in C0c the flush arm's callback invocation and `adoptNode`'s native.

§4.2 (reconcile HEAD → head), §4.8 (citations) and §4.9 (comment sites) are per PR (in each per-PR memo).

### §4.4 Creation census (D-11)

**Population (definition, by property).** Every production spawn of an entity whose `NodeKind` is a DOM node kind
(`Element` … `DocumentFragment`, nodeType 1–11), in any crate, outside test targets — through an `elidex-ecs` node
creator (the creators of D-17 (i), the argumentless creators of H-22, `create_document_node` / `create_document_root`
(a Document is its own; no owner decision), `clone_node_shallow`, `clone_subtree`) or through a raw `World` spawn.
**Command (a), creator calls** (re-run at C0b's base and at its head, C0b DoD):
`git grep -n -E '\.(create_(element|element_with_owner|element_ns|text|text_with_owner|comment|comment_with_owner|document_fragment|document_fragment_with_owner|document_type|attribute)|attach_template_contents|clone_node_shallow|clone_subtree)\(' -- 'crates/*.rs' | /usr/bin/grep -v -E '/tests/|tests\.rs|/tests_|test_helpers|test_util|/benches/'`;
the hits it lists at or after their file's first `#[cfg(test)]` line are inline test modules; **the hits before it are
the production sites** (by property; at `27b50dfe`: `… | while IFS=: read f n _; do c=$(/usr/bin/grep -n -m1
'^[[:space:]]*#\[cfg(test)\]' "$f" | cut -d: -f1); if [ -z "$c" ] || [ "$n" -lt "$c" ]; then echo "$f:$n"; fi; done |
wc -l` → 60: the six `dom/mod.rs` lines where creators delegate to one another, and the sites of the snapshot below,
some rows of which cite the owner-read line one above the creation call). At C0b's head the command is informational: the
guarantee is D-17's (an unmigrated argumentless call does not compile; an owner-taking call cannot be ownerless except
through the entry's `None`). **Command (b), raw spawns**: `git grep -n -P '\.(spawn|spawn_at|spawn_batch|reserve_entity|reserve_entities)\('
-- 'crates/*.rs'` with (a)'s two test filters → 15 production hits at `27b50dfe` (run 2026-11-03); the members are those
whose bundle carries a DOM `NodeKind`, read at the hit (the builder at `tree_clone.rs:379` adds the source's kind): the
seven creator bodies in `dom/mod.rs` (`:348`, `:411`, `:503`, `:529`, `:604`, `:622`, `:635`), `shadow.rs:189` (the
shadow root) and `tree_clone.rs:379`; the other six spawn `Window`, `Worker` or `OffscreenCanvas` entities or threads.
**Snapshot at `27b50dfe`** (not the population): the C0b memo §4.4, with each member's `source`; C0b re-derives it at
its base.

### §4.5 L3's Range sites (L3 memo Appendix A at Rev 5 `b6e500e7`)

| id | site | after C0b | owner, by landing order |
|---|---|---|---|
| A1 | `range/mutation.rs:266` extract fragment | `node_document_for_new_nodes(start node)` (*extract* 1) | **C0b** if present at its base; else replaced by A6 |
| A2–A4 | `:303`, `:354`, `:392` Text clones | entry(the cloned Text) (*extract* 4.1, 16.1, 19.1 "a clone of": default document = its node document) | **C0b** if present; else replaced by A7 |
| A5 | `:505` → `split_text.rs:148-149` | entry(text) | **C0b** |
| A6 | RC-2b contents fragment (extract and clone, step 1) | entry(start node) | **RC-2b owns the call in both orders** (PM 244 / L3): with C0b in its base it calls the entry (the argumentless creator does not exist, D-17); landing first, it keeps today's call (A1's) and C0b's D-11 migrates that existing site. **C0b owns the entry, the `ownerDocument` cell (T23, both orders) and the census** |
| A7, A8 | RC-2b CharacterData / partially contained shallow clones | stamped by the shallow cloner with the operation's document | **C0b** (the cloner), either order |
| A9 | RC-2b contained child deep clone | `node_document(src)` | **C0b** (the reader), either order |
| A10 | template clone | unchanged | C1 (D-C6) |
| A11, A12 | RC-4 `body` context element, result fragment | entry(start node) | L3 RC-4 |

### §4.6 What the C0 program provides to C1 (C1 memo rev 3.1 §4.7)

| C1 needs | program answer | PR |
|---|---|---|
| one reader | `node_document` (D-1); `owner_document` = the getter's wrapper | C0b |
| every insertion adopts, before `Insert` | D-4, D-5 | C0b |
| a contents fragment's node document is the parent document for nodes placed into it | `node_document(fragment)` (U1 contents variant) | C0b |
| the one adopt walk to edit | D-3: the walk's contents edge (C0b); the segments (C0c) | C0b, C0c |
| one adoption notification | D-6: per-segment pairs, spec order (D-19); C1 changes no consumer (T25 pins values) | C0c |
| one new-node entry, every creation site on it | D-10, D-11, D-17, D-20; hook = `attach_template_contents`'s second argument (`NewNodeOwner`) | C0b |
| no `f(throwaway)` | D-10: no build node holds the throwaway; no build link walks | C0b |

### §4.7 Layering: VM site → engine-independent API (C0b and C0c; R1 Ax1 MIN-1, R2 Ax1 MIN-1/2)

| VM site (marshalling only) | engine-independent API it calls | engine call | PR |
|---|---|---|---|
| `document.adoptNode` native (NEW, `vm/host/document.rs`, beside the method tables, after D-14 (b); C0c memo H-c6) | `invoke_dom_api("adoptNode")` → dom-api `AdoptNode` (NEW) | session `apply_adopt_node` (NEW) → `EcsDom::adopt` | C0c |
| CE flush `Adopted` arm (`flush.rs:133-138`) | `invoke_callback(entity, adopted_callback, [old, new])` (the existing helper the other arms use, `flush.rs:131`) | — (two Document wrappers from the reaction's entities) | C0c |
| CE constructor (`custom_elements/html_element.rs:231`) | `elidex_custom_elements::spawn_custom_element_entity(dom, qualified_name, definition_name, document_entity_opt())` → `Option<Entity>`; `None` → `TypeError` | the owner decision inside the helper: `node_document_for_new_nodes(document)` (D-11) | C0b |
| `ownerDocument` getter (`vm/host/node_methods_extras.rs:41`; docstring `:36-40` rewritten) | dom-api `ownerDocument.get` (`node_methods/core.rs:279`) | `owner_document` | C0b |

## §5. Program populations and risks

- **P-1 linking sites** (H-2): definition + 3 calls; `rel.parent = ` → 2. Unchanged by the program.
- **P-3 node-document readers**: `/usr/bin/grep -rn 'owner_document(' crates --include='*.rs' | /usr/bin/grep -v
  '/tests/\|tests\.rs\|/tests_\|test_helpers\|///' | /usr/bin/grep -v 'fn owner_document\|validate_owner_document'` →
  21 lines at `27b50dfe` (list: C0b memo §5.1); at C0b's head `core.rs:279` and `selection.rs:618` only.
- **P-6 `MutationEvent` consumers**: `/usr/bin/grep -rln 'MutationEvent::Insert' crates --include='*.rs' |
  /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → 11 files; re-run by C0c and by RC-2a, whichever lands second (c0e).
- **Risks** (detail in the per-PR memos): R-1 `ownerDocument` after a move; R-2 a removed parse node keeps its document;
  R-3 N1's gate follows the adopted document; R-4 base URL; R-5 `adoptedCallback` / `adoptNode` reach author code
  (C0c); R-6 replace's records (C0a) and fragment (C0b); R-8 the getter fallback deleted
  (C0b, guarded by D-17); R-9 P1 collision (§2.0); R-10 the `MutationEvent` census with L3 RC-2a (c0e).

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| template contents' owner document | C1 (`#11-template-contents-owner-document`) | under C2 the contents' node document is the template's |
| adopt steps 3.2, 3.3.2 (registries), scoped half | `#11-shadow-scoped-custom-element-registry` | a shadow root's / element's scoped registry is not re-pointed at adoption |
| adopt step 3.3.2, non-scoped half | L3's proposed `#11-document-null-registry-adopt-reset` (hooks C0c's `MutationEvent::Adopt`; L3 Rev 6.1 H8, PM 249); C0c's row | the slot's (L3 #583 §9): elidex has no null-registry document and no adopt-time registry reset (C0c's regression: C0c memo §8) |
| `importNode` | **proposed** `#11-import-node` (R-6 (b)); why deferred: PM 158 put `adoptNode` in the program and kept `importNode` out | absent (H-15): `document.importNode` is a TypeError |
| `replaceChild(X, X)` | `#11-replace-child-self-records` (**registered**, PM 227); measurement procedure: C0a memo §7 | a record-free no-op; *replace* gives two records; Chromium unmeasured |
| strict frameset fragment mode switch | **proposed** `#11-strict-frameset-fragment-mode` (R-6 (g)) | in the fragment case the strict tier leaves *in frameset* for *after frameset* (H-25), so a later comment lands in the throwaway and is orphaned; the tolerant tier is not affected (html5ever's tree builder) — unverified, measured by the slot |
| Selection API document-membership | **proposed** `#11-selection-document-membership` (R-6 (c)) | `same_document` (`selection.rs:614-619`) accepts a detached node whose node document is the selection's; step text unverified (webref cannot render the anchors) |
| VM `isConnected` | `#11-vm-isconnected-bound-document` (**registered**, PM 227, owner L3) | compares the composed root with the bound document |
| HTML `img` adopting steps | **proposed** `#11-img-adopting-steps-relevant-mutations` (R-6 (f)) | an `img` adopted into another document does not re-run *update the image data* (HTML §4.8.4.3.2 Reacting to DOM mutations, *relevant mutations*: "The element's adopting steps are run"); `/usr/bin/grep -rln 'relevant mutation\|update_image_data' crates` → no hit |
| VM Attr wrapper vs dom-api Attr entity | `#11-dom-api-attr-entity-model-unification` (registered) | the VM's Attr has no entity and no `ownerDocument` (H-17), so adopt 3.3.1 is observable only at unit level until that slot lands |

Own deferrals: **4 proposed slots** for pre-existing defects the program touches but does not fix ((b), (c), (f),
(g)); each has defect, owner, trigger and date in R-6, registered at landing (PM 251). No planned implementation is
deferred.

## §8. Edges, hand-offs and records

**Edges (program).**

| # | edge | kind | note |
|---|---|---|---|
| c0-pre-a | D-14 (a) → C0a | code predecessor | pure move |
| c0-pre-b | D-14 (b), (d), (e) → C0c | code predecessor | pure moves; (d) and (e) shared with L3 RC-0a (whoever lands first; (e) in RC-0a since L3 `55d35a16`, P-19 (d)) |
| c0-pre-c | D-14 (c) → C0b | code predecessor | pure move (PM 258) |
| c0-pre-ci | CI/tools default-features check PR #585 → C0b | code predecessor (CI); landed (`115a667c`) | PM 255 / 263: #585 owns the step "Clippy (default features, no dev-only feature)", its `mise` task `check-default` and the CLAUDE.md exception; C0b adds `scripts/check-fixture-boundary.sh` as its own step after that step (D-17 (iii)) |
| c0a | N1 (#580, landed) → C0b | code + semantic | clause (b) on N1's landed body; T8–T10 need N1's gate |
| c0-ab | C0a → C0b | code predecessor (soft) | C0a may land any time before C0b; both edit `apply_replace_child` (D-12 / D-5) |
| c0-bc | C0b → C0c | code + semantic | `adopt`, the walk, the stamps the pairs read |
| c0b | C0b → C1; C0c → C1 | semantic + code | C1's contents value (C0b); its per-segment pairs (C0c); C1's edge rows are C1's |
| c0c | C0b → T3 | data flow | T3 reads `node_document` |
| c0d | C0b → L3 P1 | boundary | §2.0 |
| c0e | C0c ↔ L3 RC-2a | census collision | RC-2a's `CloneCreated` and C0c's `Adopt` add variants to the same enum, arms to the same consumer and exhaustive matches; whichever lands second re-runs P-6 and adds its arm to every exhaustive match (`range/live.rs`, `mutation_hook.rs`). Both also edit `tree/walkers.rs` and the `dom/mod.rs` / `lib.rs` re-exports (C0c memo §5) |
| c0h | #583 → C0c | ordering (PM 311) | C0c lands after #583 (the slots C0c points to are registered at #583's landing; C0c checks §9 item 6's C0c points) |
| c0f | C0b ↔ L3 RC-2b | creation entry | §4.5 A6: RC-2b calls the entry if C0b is in its base; C0b migrates RC-2b's sites if RC-2b lands first (compile-enforced either way, D-17) |
| c0g | C0b ↔ L3 RC-2a | file overlap `node_methods/clone.rs` | the pass-4 hand stamps `:282-285` (cloned host + replicated shadow root) and `:294-301` (the `!deep` shadow-child stamp; `git show 27b50dfe:crates/dom/elidex-dom-api/src/node_methods/clone.rs | sed -n '282,285p;294,301p'`) call `set_associated_document`, `pub(crate)` from C0b (D-18). C0b first: C0b removes both, and RC-2a writes no hand stamp. RC-2a first: D14 (RC-2a) removes `:294-301`, C0b removes `:282-285`. C0b's re-measure scope includes RC-2's files (`node_methods/clone.rs`, `node_methods/tests/clone.rs`, `elidex-custom-elements/src/consumer.rs`, `dom/mutation_event.rs`, `dom/mod.rs`, `tree_clone.rs`, `range/contents.rs`) |

**Hand-offs.**
- **H-C1 to C1** (via its driver): rev 3's changes for C1's text: (i) the program is three PRs; C1's contents value
  needs C0b, its T25 pairs need C0c; (ii) `FragmentParseDocument` → `FragmentParseRoot` on the root (D-10), so its
  D-C4 last bullet, A2×A6 row and §3 §13.4 row ("re-homed through the throwaway and back", "the finishing walk") are
  stale (P-17), the value claim stands; (iii) `attach_template_contents`'s second argument is a `NewNodeOwner` (D-17);
  (iv) Q-2 is decided per-node (D-20), so its §4.3 constraint is met by construction; (v) frame `d262ca8c` → this rev;
  (vi) D-19 (C0c) changes `walkers.rs` (the `Include` order), so C1's "unchanged" for it (C1 memo `:441`, `:647` at
  `b5e47f4b`) is stale.
- **H-T to T3** (rev 4, after C1): step 3 calls `EcsDom::node_document`; R-B closed by C0b (cell T13). **New:** the
  umbrella's §1.2 C0 row is now the program row (three PRs, R-3), added by C0a together with the order line; T3's
  umbrella edits on `layout-eh-target-determination-plan` must drop, when rebasing onto C0a, their C0 row and
  order-line edits **and C0a's consistency edits under PM 271** (R-3's table E1–E13: the header, the T3 and slice b
  rows, x6, x9, x10, the closing paths, §1.6, G3, §8.1, §8.2), keep their own edge x8 (C1 → T3), leave the C1 row and
  x7 (C0 → C1) to C1's PR (each row is added by its own PR, PM 159 / 223 O-1 as amended by PM 271), and renumber their
  rev headings (8.4, 8.5 on that branch) after C0a, C0b and C0c take the next numbers (C0a plan-review Ax5 MIN-2).
- **H-P1 to L3's P1 umbrella** (via the PM): §2.0's contract, split by PR.
- **H-L3 to L3** (via the PM): work handed to L3 only (PM 273: #583 states the shared points as decided substance with
  PM IDs; C0 edits need no notification, and the C0 program PR that lands after #583 checks the wording, §9 item 6):
  (1) A6 (§4.5): RC-2b owns the call in both orders; C0b owns the entry, T23 and the census; either way a missed site
  does not compile (D-17); (2) the edges c0e (RC-2a × C0c), c0f (RC-2b × C0b), c0g (RC-2a × C0b, `clone.rs`) and
  D-14 (d) / (e) bind L3's PRs in either landing order; (3) the slots `#11-selection-document-membership` (proposed),
  `#11-vm-isconnected-bound-document`, `#11-import-node` (proposed, owner L3) and `#11-strict-frameset-fragment-mode`
  (proposed, owner L3 as the fragment-parse lane) are L3's.
- **H-N4 to N4**: D-15 — `node_document`, adds no function; the N1 memo's n3 hand-off is superseded.

**Records (for the PM; exact texts).**
- **R-1 ledger `#11-cross-document-adopt-on-insert` — at C0c's landing: close**: "Closed by the C0 program (C0a #<PR>
  `<squash>`, C0b #<PR> `<squash>`, C0c #<PR> `<squash>`): every insertion adopts (DOM §4.2.3 *insert* 7.1, *replace* 6)
  through `EcsDom::adopt` at the three linking primitives; `document.adoptNode` added; `AssociatedDocument` is the node
  document, set at creation by `EcsDom::node_document_for_new_nodes` and read only by `EcsDom::node_document`; the
  composed-tree-root proxy, `owner_document`'s fallbacks and the getter's `document_root()` fallback are deleted.
  `selection.rs` is not a node-document reader (`#11-selection-document-membership`)." At C0b's landing append:
  "C0b (#<PR>): insertion adoption and the node-document fact landed; `adoptNode` and `adoptedCallback` follow in C0c."
- **R-2 ledger / roadmap `#11-custom-elements-adopted-callback` — at C0c's landing: close**: "Closed by C0c (#<PR>,
  `<squash>`): `MutationEvent::Adopt` → `CustomElementReaction::Adopted` → `adoptedCallback(oldDocument, document)`,
  each node with its own pair, in spec order. The iframe side (`contentDocument`) stays with
  `#11-windowproxy-browsing-context`."
- **R-3 umbrella §1.2, in place (D-9).** Added by C0a's PR, revision number per D-9's rule, row text:
  "| **C0** `EH-attrs-at-creation-C0` (owner: Layout lane, this program; PM decisions 143, 244), a program of three PRs
  in order **C0a** `EH-attrs-at-creation-C0a` (*replace* record order) → **C0b** `EH-attrs-at-creation-C0b` (the node
  document as the stored fact: creation entry, insertion adoption, one reader) → **C0c** `EH-attrs-at-creation-C0c`
  (`adoptedCallback` and `document.adoptNode`); program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` |
  slot `#11-cross-document-adopt-on-insert` (DOM §4.2.3 *insert* step 7.1, §4.5 *adopt*); C0b ends the stale node
  document after a cross-document move (a node moved into another document keeps its old node document, which T3's
  step 3 would read; edge x9). C0a: landed (#<PR>). C0b: planned. C0c: planned. | IBP-sandbox N1's implementation landed; C0a after D-14 (a);
  C0b after C0a, D-14 (c), the CI/tools default-features check PR (#585); C0c after C0b and D-14 (b) |". **Order line**
  (PM 227 O-1; also by C0a's PR): the line "**Order (PM decisions 21, 23, 25; §L):** approval PR → slice a → {
  IBP-sandbox N1, T3 } → C1's handling decided → slice b. N1 and T3 are independent of each other. The PM decides C1's
  handling once slice a and N1 have landed (§8.1)." (umbrella on `main` `d7d34035`, §1.2) is replaced by:
  "**Order (PM decisions 21, 23, 25, 123, 153, 244; §L):** approval PR → slice a → IBP-sandbox N1 → **C0** (program: C0a →
  C0b → C0c) → C1 → T3 → slice b (PM 153, user decision: N1 implementation → C0 → C1 → T3 → slice b; PM 244: C0 is a
  program of three PRs)." **Consistency edits under PM 271, amending PM 223 O-1** (also by C0a's
  PR; each before → after; `<n>` is the revision number of D-9's rule):

  | # | umbrella site (`27b50dfe`) | before | after |
  |---|---|---|---|
  | E1 | header `:5-6` | "it also owns **T3**, slice b's independent prerequisite PR: slot `#11-event-handler-target-determination` was reassigned to this program by PM decision 25 (§1.2, §8.1)." | "it also owns **T3**, slice b's prerequisite PR: slot `#11-event-handler-target-determination` was reassigned to this program by PM decision 25 (§1.2, §8.1). From rev 8.<n> it also owns **C0** (`#11-cross-document-adopt-on-insert`, PM decision 143; a program of three PRs, PM decision 244) and **C1** (`#11-template-contents-owner-document`, branch A, PM decision 123), and T3 lands after them (PM decision 153; until rev 8.<n> T3 was "slice b's independent prerequisite PR")." |
  | E2 | §1.2 T3 row, prerequisites `:424` | "slice a landed" | "C1 landed (PM decision 153; slice a, N1's implementation and C0 precede it in the order line; until rev 8.<n>: "slice a landed")" |
  | E3 | §1.2 slice b row, prerequisites `:425` | "T3, N1, C1's handling decided" | "T3, N1, C0 and C1 landed (PM decisions 123, 153; until rev 8.<n>: "T3, N1, C1's handling decided")" |
  | E4 | §1.3 x6 `:439`, kind and reason | "scheduling precondition (PM decision 25) \| T3 is started once slice a lands. No code or semantic dependency is claimed." | "scheduling precondition (PM decisions 25, 153) \| T3 is started after C1 lands (PM decision 153; until rev 8.<n>: "once slice a lands"), so x6 holds transitively (slice a → N1 → C0 → C1 → T3). No code or semantic dependency is claimed." (the rest of the cell unchanged) |
  | E5 | §1.3, new row after x6 | — | "\| x9 \| C0 → T3 (rev 8.<n>) \| semantic precondition (PM decision 153) \| without insertion adoption a node moved into another document keeps its old node document, which T3's step 3 reads; C0b makes the node document a stored fact, kept true at every insertion and read by `EcsDom::node_document` \|" |
  | E6 | §1.3, new row after x9 | — | "\| x10 \| N1 → C0 (rev 8.<n>) \| code predecessor (PM decision 153) \| C0b rewrites N1's compile-gate clause (b) to read `EcsDom::node_document` (C0 program memo D-7 (a)), so N1's implementation (#580) lands first \|" |
  | E7 | §1.3 `:441` | "Acyclic. The order satisfies x4 transitively." | "Acyclic. The order satisfies x4 transitively. x7 (C0 → C1) and x8 (C1 → T3) are added by the PR at each edge's head (C0 program memo D-9)." |
  | E8 | §1.4 closing path `:467-468` | "slice a lands → T3 (this program's PR, slot `#11-event-handler-target-determination`; N1 lands independently) → C1's handling decided (PM) → slice b → the PM closes the slot above (§1.3 x6, x3–x5, x2)." | "slice a lands → N1 → C0 → C1 → T3 (this program's PR, slot `#11-event-handler-target-determination`) → slice b → the PM closes the slot above (§1.3 x6, x3–x5, x2, x9, x10; rev 8.<n>, PM decision 153; until rev 8.<n>: "slice a lands → T3 (…; N1 lands independently) → C1's handling decided (PM) → slice b")." |
  | E9 | §1.6 `:504-505`, after "(§8.1)." | — | "Rev 8.<n>: decided, branch A (PM decision 123); C1 is this program's nested PR, after C0 and before T3 (§1.2)." |
  | E10 | §7 G3 `:820-821` | "It is T3, slice b's independent prerequisite PR, owned by this program (PM decision 25; §1.2)." | "It is T3, slice b's prerequisite PR, owned by this program (PM decision 25; §1.2), after C1 (PM decision 153)." |
  | E11 | §8.1 `:842` | "slice b's independent prerequisite PR (x3), started once slice a lands (x6)," | "slice b's prerequisite PR (x3), started after C1 lands (x6; PM decision 153)," |
  | E12 | §8.2 closing path and trigger `:898-899` | "**Closing path:** slice a lands → T3 (this program) → C1's handling decided → slice b. **Trigger:** T3 and N1 landed, and C1's handling decided (the ledger" | "**Closing path:** slice a lands → N1 → C0 → C1 → T3 (this program) → slice b (PM decision 153). **Trigger:** N1, C0, C1 and T3 landed (the ledger" |
  | E13 | §8.2 confidence `:901-902` | "The order is fixed (PM decisions 21, 23, 25) and T3 has an owner, but T3 starts only after slice a lands, and slice b also waits on C1's handling (owner TBD)." | "The order is fixed (PM decisions 21, 23, 25, 153) and T3 has an owner; T3 starts after C1 lands, and C1's handling is decided (branch A, PM decision 123)." |

  E1–E7 and E9 are the items PM 271 lists; E8 and E10–E13 are the other umbrella sentences that state the replaced order
  (found by `/usr/bin/grep -n -i 'independent prerequisite\|once slice a\|lands independently\|handling decided' <umbrella>`
  at `27b50dfe`: hits `:5`, `:59`, `:425`, `:427-428`, `:438`, `:439`, `:468`, `:821`, `:842`, `:899`), edited under the same
  rule. Not edited: `:59` (rev 6's changelog entry, history); x5 `:438` (its condition, "C1's handling decided", is met by
  E9; its branch-A clause stays true). No design content of T3 is brought in (T3's facets, D-T*, rev 8.4 / 8.5 scope
  stay on T3's branch).

  Changelog: "*Rev 8.<n>* (<date>; carried by C0a's PR): §1.2 gains the C0 program row and its order line follows PM
  153; the header, the T3 and slice b rows, x6, the closing paths, §1.6, G3, §8.1 and §8.2 follow the new order and PM
  123; edges x9 and x10 added (PM decisions 123, 153, 159, 223 O-1 as amended by 271, 227 O-1, 244, 271)." C0b's PR
  replaces "C0b: planned." with "C0b: landed (#<PR>)." and C0c's PR "C0c: planned." with "C0c: landed (#<PR>)." (each
  with a changelog line). The program's umbrella edits are the row, the order line and these clause updates, plus the
  PM 271 consistency edits listed here; nothing else.
- **R-4 node memo and node input (D-15 (i), (ii), (iv); applied on this branch); parent (D-15 (iii); C0b's PR).** See
  the node memo's "Amendment 2026-11-01" paragraph (restated 2026-11-03), the node input's "Amendment 2026-11-03" note,
  and for the parent's amendment line the C0b memo §8 (exact text).
- **R-5 N1 memo**: none in place (landed); its n3 hand-off's supersession is in R-4.
- **R-6 slot texts (defects only).** (a) `#11-ownerless-node-creation` — append at C0b's landing: "C0b (#<PR>): the
  `EcsDom` entry `node_document_for_new_nodes` exists; every production creator takes a `NewNodeOwner` from it, and the
  argumentless creators are test-only by construction (`elidex-ecs`'s `fixtures` feature, enabled only from
  `[dev-dependencies]`; checked by `scripts/check-fixture-boundary.sh` in CI). Remaining facets: template contents (C1); L3 RC-4's planned sites (they call
  the entry)." Also correct the slot's text, which still says "C1 puts both parser tiers on it": per PM
  223 O-2 / 227 C0 (C0b) migrates every production site including both parser tiers; C1 changes only
  `attach_template_contents`. (b) **proposed** `#11-import-node` — defect: "`document.importNode(node, options)` is
  absent (TypeError); DOM §4.5 `[CEReactions, NewObject] importNode` clones node into this." Owner: L3 (DOM node
  surface). Trigger: the C0 program landed (the adopt and clone-with-document machinery it needs exist). Re-eval:
  2026-12-15. Why deferred: PM 158 (`PORTFOLIO-2026-10-26.md:260`) — "include
  document.adoptNode (exclude importNode)". (c) **proposed** `#11-selection-document-membership` — defect: "Selection methods accept a detached node
  whose node document is the selection's document (`selection.rs:614-619`); the Selection API's check is unverified
  against its step text (webref cannot render the anchors)." Owner: L3 (RC-1's file). Trigger: webref renders the
  Selection API anchors, or L3's next `selection.rs` touch. Re-eval: 2026-12-15. (d) `#11-vm-isconnected-bound-document`
  — registered (PM 227); no text change. (e) `#11-replace-child-self-records` — registered (PM 227); its re-eval "after
  C0 lands" reads "after C0a lands" (the record mechanism is C0a's); backstop 2026-12-15. (f) **proposed**
  `#11-img-adopting-steps-relevant-mutations` — defect as §7's row. Owner: Layout (image request state, IBP's F4–F6
  family). Trigger: C0c landed (the `Adopt` event is the hook). Re-eval: 2026-12-15. (g) **proposed**
  `#11-strict-frameset-fragment-mode` — defect: "In the fragment case the strict tree builder switches from *in
  frameset* to *after frameset* on `</frameset>` (`in_frameset.rs:31-33`), omitting HTML §13.2.6.4.18's 'fragment
  context element is null' guard, so a later comment is inserted into the parser's throwaway document
  (`after_after_frameset.rs:15`) and orphaned when it is destroyed." Owner: L3 (the fragment-parse lane of #574 / #576
  / #577; PM to confirm — the strict tier has no lane of its own). Trigger: the next strict tree-builder touch, or a
  frameset fragment WPT failure. Re-eval: 2026-12-15.
- **R-7 roadmap**: none beyond R-2.
- **R-8 ledger order line (slot `#11-cross-document-adopt-on-insert`, PM 143 sub-bullet; at C0a's landing)**: "Order: N1 implementation → T3 → C0 → C1 → slice b"
  is superseded by PM 153 ("N1 implementation → C0 → C1 → T3 → slice b") and, for C0's inside, by PM 244 (C0a → C0b →
  C0c). Append: "*Corrected 2026-11-03*: order per PM 153 / 244 — N1 → C0a → C0b → C0c → C1 → T3 → slice b."
- **R-9 (PM 266).** The generic guard is handed to `#11-test-only-feature-boundary-guard` (CI/tools, registered PM 266).

## §9. Program definition of done

1. C0a, C0b, C0c each landed with its own per-PR plan-review passed and its own DoD met (their §9).
2. D-14 (a) landed before C0a; (c) and the CI/tools default-features check PR #585 before C0b; (b), (d), (e) before C0c (or
   (d) / (e) by L3 first).
3. The umbrella row, the order line and the PM 271 consistency edits (R-3) added by C0a; the row updated by C0b and
   C0c.
4. The PM has applied R-1, R-2, R-6 (a), R-8 and registered R-6 (b), (c), (f), (g) at the landings (PM 251; each is in
   the "For the PM" line of the PR that carries it: R-6 (e), R-8 C0a; R-1's append, R-6 (a), (c), (g) C0b; R-1, R-2, R-6 (b), (f) C0c).
5. `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo, the per-PR memos (C0a, C0b, C0c) and the node memo exits 0
   (the node input is an input file with no §3 and is not a preflight target) at each landing (run; do not trust a stored verdict).
6. **The C0 program PR that lands after #583** (PM 273): before landing, compare this memo's text against #583's text
   as it stands on main on these points, and fix any wording mismatch on the C0 side:
   - the A6 owner;
   - `EcsDom::adopt` = C0b;
   - `NewNodeOwner`;
   - C0b → P1, with C0c's Adopt in the same window;
   - the mutation_hook.rs / live.rs split, L3 side = RC-0a;
   - the clone.rs pass-4 stamps (H9: C0b first → C0b removes :282-285 and :294-301; RC-2a first → D14 :294-301, C0b :282-285).

   C0c's points, checked by C0c's PR (it lands after #583, edge c0h):
   - H6's census rule (whichever of C0c and RC-2a lands second re-runs the census and adds the arms);
   - H8's null-registry row ("the row is C0c's because of that hook");
   - #583 §7's `dom/mod.rs` row (`:1888` on `1d57b387`, "C0c does not edit it") and its restatements, against C0c's
     re-export (C0c memo §5);
   - the `#11-ce-reactions-method-scope-drain` slot text, including PM 306's line (a pending upgrade loses
     `adoptedCallback`), and `#11-document-null-registry-adopt-reset`'s, including PM 311's (C0c memo §8).
7. No dangling reference to the generic feature guard (PM 266): `git grep -n 'FEATURE_CLASS\|test-only feature' --
   docs/plans/` hits only this item and changelog / review-disposition lines that record earlier revisions (run at each
   landing; at rev 3.2: §10.2 of this memo lists the hits).

## §10. Review dispositions

**R1** (rev 2.1 `d262ca8c`): the per-finding table is rev 3's (`git show 9809a8ee:docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md
| sed -n '/^## §10/,/^## §11/p'`), unchanged. Added at rev 3.1: **PM 227 O-1** ("the first C0 PR rewrites the order row
to PM 153's text"), dropped by rev 2's withdrawal of the order rewrite and by rev 3's R-3 "nothing else" (R2 Ax5
IMP-2) → C0a's PR rewrites the umbrella §1.2 order line (D-9, R-3, exact text there).

### §10.1 R2 (rev 3 `9809a8ee`; 0 CRIT / 5 IMP / 27 MIN; PM 258)

| finding | disposition | where |
|---|---|---|
| Ax5 IMP-1, Ax3 MIN-2 (PM 255 not reflected) | split: CI/tools PR owns step + task + CLAUDE.md; C0b the script; edge c0-pre-ci; O-3 decided | D-17 (iii), §8, §11; C0b D-17, §8, §9 |
| Ax5 IMP-2 (PM 227 O-1) | order line in R-3; §10 | D-9, R-3; C0a |
| Ax5 IMP-3, MIN-2, MIN-8 (L3 relay) | P-19; H-L3 (2)–(4); header coordinates | P-19, H-L3, header |
| Ax5 IMP-4 (Chromium procedure lost) | restored as a procedure in C0a §7; §7 row points there | §7; C0a §7 |
| Ax3 MIN-1 (test-only features by property) | guard over every test-only feature, classified in the script with a complement check; A8 scoped | D-17 (iii), H-24, A8 |
| Ax3 MIN-3, Ax2 MIN-2 (D-18 residual) | the type is crate-private: no residual | D-18 |
| Ax3 MIN-4 (table_mutation) | D-14 (c) prereq, handlers → `handlers.rs` | D-14, §8 |
| Ax3 MIN-5 (D-14 (d)(e) not on L3's side) | P-19 (d), H-L3 (2), D-14 cites L3 §7 | D-14, H-L3 |
| Ax3 MIN-6, MIN-7 (C0b / C0c verdicts) | true reasons stated; C0c kept whole, reason stated | §1 |
| Ax3 MIN-8 (fixture tests of unreachable states) | Rejected reworded; C0b check | D-17; C0b §9 |
| Ax3 MIN-9 (`#11-import-node` why) | PM 158 | §7, R-6 (b) |
| Ax3 MIN-10, Ax2 MIN-4 (census caveat) | stated by property (60 lines); four rows added; line pairs | §4.4 |
| Ax2 MIN-1 (strict frameset) | D-10 claim scoped; slot proposed | D-10, H-25, §3, §7, R-6 (g) |
| Ax2 MIN-3 (I-U vs ownerless parent) | stated | §2.1 A1 |
| Ax2 P2 (does `cargo tree -e normal` see dev features?) | measured: no (H-24) | H-24 |
| Ax1 MIN-1, MIN-2 (CE owner; VM rows' PR) | decision in `spawn_custom_element_entity`, `None` → no element (VM `TypeError`); PR column, §4.1 restated | D-11, §4.1, §4.7; C0b |
| Ax4 MIN-1, MIN-3 (bare §13.4 steps; §3 Touch) | algorithm named; `tree.rs:431-432`, `text_content.rs:119-120` | D-8, §3; C0b §3, §4.8 |
| Ax4 MIN-2 (node memo "§13.4 step 10") | real: → step 8 (the scripting read) | node memo `:211` |
| Ax4 / Ax5 MIN-3 (D-15 (iv) quote; F10 placement and wording) | quote fixed; F10 line moved to C0b as a rule replacement; node memo heading names the plan PR | D-15, R-4; C0b §8; node memo |
| Ax5 MIN-1 (c0g / C0b DoD 9) | C0b DoD 9 aligned with PM 249 | C0b §9 |
| Ax5 MIN-4, FP (slots at landing) | O-4 decided (PM 251) | §7, §9, §11 |
| Ax5 MIN-5, MIN-7 (T3 branch, H-T; RC-2a vs C1) | branch name, H-T restated; RC-2a is C1's prerequisite | header, H-T, §1 Order |
| Ax5 MIN-6 (A6 naming) | RC-2b owns the call; C0b entry, T23, census | §4.5, H-L3 |

### §10.2 Rev 3.2 (C0a plan-review at `4c79d9c8`; PM 263 / 266 / 271 / 273; D-21 re-check)

| item | disposition | where |
|---|---|---|
| C0a plan-review (0 CRIT / 3 IMP / 15 MIN) | every finding fixed; the per-finding table is the C0a memo §10 | C0a memo; D-12 |
| PM 271 (amends PM 223 O-1) | C0a writes the order line's immediate consequences in the umbrella (E1–E13); "nothing else" narrowed; T3 drops them at rebase and renumbers its revs | D-9, R-3, H-T; C0a §8, §9 |
| PM 266 (D-17 narrowed) | the guard claims `fixtures` only, read from `cargo tree`; the classification table, its `awk` population and the generic claim removed; the host-target concern does not apply to `fixtures` (H-24); the generic guard is R-9 | D-17 (iii), A8, H-24, R-9; C0b D-17 (4), U17, §6.3, §9 |
| PM 263 | the CI/tools step is "Clippy (default features, no dev-only feature)" | D-17 (iii), c0-pre-ci; C0b D-17 (4), §6.4, §9 |
| PM 273 | no relay to L3; §9 item 6 compares the wording with #583 at the C0 program PR that lands after it; H-L3 keeps the work items | P-16, P-19, H-L3, §9; per-PR DoDs |

Dangling-reference check (§9 item 7), run 2026-11-03 at rev 3.2: `git grep -n 'FEATURE_CLASS\|test-only feature' --
docs/plans/` → 5 hits, each intended: this memo's rev 3.1 changelog (`*Rev 3.1*` entry), §9 item 7 itself, this paragraph, §10.1's R2 row
"Ax3 MIN-1" (the rev 3.1 disposition, superseded by PM 266 above), and the C0b memo's rev 3.1 changelog. The rev 3
changelog's "default-features `cargo check` CI step" (Q-3) is likewise history, superseded by PM 263.

### §10.3 Rev 3.3 (C0b plan-review at `15e99049`; PM 283)

| item | disposition | where |
|---|---|---|
| C0b plan-review (Ax2, Ax3, Ax4) | every finding fixed; the per-finding table is the C0b memo §10 | C0b memo |
| Ax3 MIN (c0-pre-ci without an ID; build edges) | #585; `-e normal,build,features` | §8, D-17 (iii), H-24 |
| Ax2 MIN-3 (census by name) | population by property, command (b) for raw spawns | §4.4 |

### §10.4 Rev 3.6 (C0c plan-review at `76af676a`; PM 306 / 311)

| item | disposition | where |
|---|---|---|
| C0c plan-review (0 CRIT / 8 IMP / 22 MIN) | every finding fixed; the per-finding table is the C0c memo §10 | C0c memo; D-3 (c), D-6, D-14 (b), §1, §2.0, §4.7, §7, §8, §9 |

## §11. Open questions (for the PM)

None. O-3 (default-features CI check) is decided by PM 255 (user decision γ; D-17 (iii), edge c0-pre-ci); O-4 (slot
proposals) by PM 251 (registered at landing; §9 item 4).

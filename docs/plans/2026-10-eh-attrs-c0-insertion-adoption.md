# C0 `EH-attrs-at-creation-C0` — every insertion adopts the node into the parent's node document (program memo)

Nested **program C0** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 by
#570; "the umbrella"). Since rev 3 C0 is a program of three PRs, each with its own per-PR plan memo and its own
`/elidex-plan-review` (CLAUDE.md *Edge-dense work* (a)–(c); PM decision 244):

| PR | per-PR plan memo | content (decisions of §0.4) |
|---|---|---|
| **C0a** `EH-attrs-at-creation-C0a` | `docs/plans/2026-11-eh-attrs-c0a-replace-record-order.md` (NEW) | D-12, after its prerequisite split D-14 (a) |
| **C0b** `EH-attrs-at-creation-C0b` | `docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` (NEW) | D-1, D-2 (no `Adopt` event), D-3 (a)(b), D-4, D-5, D-7, D-8, D-10, D-11, D-15, D-17, D-18, D-20; D-14 (c) if triggered |
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
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 3 on branch `layout-eh-attrs-t3-plan` at `14ff3dc2`;
"the T3 memo"); L3's Range program (`docs/plans/2026-11-range-contents-program.md`, **Rev 5 `b6e500e7`** on branch
`range-full-impl-plan`, unpushed; "the L3 memo"; Rev 6 is announced to split RC-2 into **RC-2a** = the `CloneCreated`
event and its consumers, the `cloneNode` migration, D14 and the #466 reversal, landing before C1, and **RC-2b** = the
contents algorithm; its Appendix A site IDs A1–A12 are cited as that memo's). Unlanded IDs are cited as those memos' IDs.

**Placement (evidence).** PM decision 143 (2026-10-31; ledger `project_open-defer-slots.md`, slot
`#11-cross-document-adopt-on-insert`, sub-bullet "Owner assigned 2026-10-31"): "Layout lane, as C0 of the EH-attrs
program. C0 is a standalone PR before C1 … Edge-dense, so plan-review is mandatory. L3's P1 umbrella treats C0 as
landing first (the insertion-steps boundary goes in C0's plan §2)". The same sub-bullet still records the order "N1
implementation → T3 → C0 → C1 → slice b" (ledger `:132`); PM decision 153 (2026-10-31, user decision) superseded it:
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

**Changelog.** *Rev 3* (2026-11-03): re-sliced per plan-review R1 (0 CRIT / 13 IMP / 19 MIN raw, verdict SPLIT) and PM
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
| H-20 | Touched files over 1000 lines | `wc -l` at `27b50dfe`: `crates/script/elidex-script-session/src/mutation/mod.rs` 994, `crates/script/elidex-js/src/vm/host/document.rs` 1197, `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` 994, `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` 992 (C0b, D-18), `crates/dom/elidex-dom-api/src/range/live.rs` 1021 (C0c, D-14 (d)), `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` 1143 (C0c, D-14 (e)), `crates/dom/elidex-dom-api/src/selection.rs` 1036 (not touched) |
| H-21 | The `AssociatedDocument` write and read surface | writers: `set_associated_document` (`pub`, `mod.rs:789`) — prod callers `teardown.rs:257`, `tree_clone.rs:221`, `:230`, `:417`, `node_methods/clone.rs:283`, `:284`, `:299`; the creators' `insert_one` (`mod.rs:341-358` and kin). Readers: `get_associated_document` (`pub`, `mod.rs:777`; outside `mod.rs` only test reads `scripting.rs:201`, `:234`), `owner_document`. The component's field is `pub` (`components.rs:595`); no production code outside `elidex-ecs` names the type: `git grep -n 'set_associated_document(\|get_associated_document(\|AssociatedDocument(' -- 'crates/*.rs'` |
| H-22 | The four ownerless creators and their reach | `create_element` (`mod.rs:331`), `create_text` (`:492`), `create_document_fragment` (`:519`), `create_comment` (`:593`), all `pub`, each `…_with_owner(…, None)`. Raw call lines: `git grep -n -E '\.create_(element\|text\|comment\|document_fragment)\(' -- 'crates/*.rs' \| wc -l` → 2577; outside test-path files (`… \| /usr/bin/grep -v -E '/tests/\|tests\.rs\|/tests_\|test_helpers\|test_util\|/benches/' \| wc -l`) → 431, of which most are inline `#[cfg(test)]` modules (§4.4) |

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
  change, code does not')": stale since rev 2 — C0b deletes the getter's `document_root()` fallback (D-1 (c)). Told to
  L3 (H-L3).
- **P-17 (C1 memo rev 3.1, its D-C4 last bullet, A2×A6 row, §3 §13.4 row).** "linked nodes are re-homed through the
  throwaway and back as C0's overwrite note states" and "`finish_detached_fragment`, whose walk (D-C5) maps contents":
  after D-8 / D-10 (rev 3) no fragment-build node is re-homed through the throwaway and the finishing walk is deleted;
  the component is `FragmentParseRoot` on the root, not `FragmentParseDocument` on the throwaway. C1's value claim
  (contents get `f(context's document)`, never `f(throwaway)`) holds more strongly. Told to C1 (H-C1).
- **P-18 (R1 Ax5, ledger `:686-687`).** Rev 2's §7 and DoD 11 called `#11-vm-isconnected-bound-document` and
  `#11-replace-child-self-records` "proposed"; both are registered (PM 227, ledger `:686`, `:687`).

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
  one edge with `f(document)`, C1 memo D-C5); **(b) (C0b)** re-homes each visited element's cached Attr entities
  (`AttrEntityCache`) (step 3.3.1, H-17); **(c) (C0c)** records a contents segment `{ fragment, documents:
  Option<(old, new)> }` per contents fragment reached, `None` when `old` is `None` or equals `new`.
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
  iff some pair is `Some`. Consumer: the custom-element consumer's `Adopt` arm walks the subtree with the spec-ordered
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
  node the build linked under root or below already carries the redirect document of D-10 (A5), so HTML §13.4 step 22
  ("Adopt fragment into target's node document") is the identity for elidex as it is in the spec when no sanitizer
  configuration is given (step 14 creates the fragment in target's node document).
- **D-9 (program) — the umbrella's §1.2 row: one row for the program.** C0a's PR (the first to land) adds **one** row
  naming C0a / C0b / C0c with the order and the three PR names, and a changelog entry; C0b's and C0c's PRs each update
  only their own clause of that row; an edge row into C1 / T3 / L3 is added by the dependent PR. Revision-number rule
  (unchanged from rev 2): the next number after `main`'s current one, assigned at rebase. Exact texts: §8 R-3.
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
    node of a build except root holds the context's document from its creation; no link adopts into the throwaway.
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
  node document" of (§4.4's `source` column for the `27b50dfe` snapshot), or, for a clone, the operation's document
  (`clone_node_shallow` gains a `document` parameter, DOM *clone a single node* "given document"; P-2). Shadow roots
  are stamped from the host (*attach a shadow root* step 5); Attr entities and doctypes gain an owner. Any production
  creation site present at C0b's base — including L3 RC-2b's contents-algorithm sites if RC-2b lands first (§4.5 A6) —
  is in the population and is migrated by C0b; the argumentless creators no longer exist for production code (D-17), so
  a site left unmigrated does not compile.
- **D-12 (C0a) — the *replace* mutation records follow the current step order (PM 160 (a)).** The non-fragment arm
  captures the source-removal record's siblings **without** stepping over `old_child` (adopt step 2 runs at replace
  step 6, before step 7 removes `old_child`); every replace step comment in `apply_replace_child` is renumbered to the
  current 1–11 (step 1 = "ensure pre-insert validity", `mutation/mod.rs:691`, `:700`), except the fragment arm's step 6,
  which arrives with its code in C0b (D-5). Records: count and order unchanged; only the source record's siblings
  change (T17).
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
  `vm/host/document_factories.rs` (NEW; `adoptNode` joins it in C0c) and the accessors (`:423-938`) →
  `vm/host/document_accessors.rs` (NEW). (c) **with C0b, if triggered**: `table_mutation/mod.rs` (994) → handlers
  split if C0b's head reaches 1000. (d) **before C0c, unless L3 RC-0a has done it**:
  `crates/dom/elidex-dom-api/src/range/live.rs` (1021; C0c adds an `Adopt` arm) — its inline test module (from `:498`)
  moves to a sibling file; this is the move L3 RC-0a plans (L3 memo Appendix A note), so whichever of RC-0a and C0c's
  prereq lands first does it and the other drops it. (e) **before C0c, unless L3 RC-2a has done it**:
  `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` (1143; C0c and RC-2a each add an arm to its exhaustive match)
  — the specialised-hook scenarios (from the section rule at `:973`, "after_remove descendants snapshot", to the end)
  move to a sibling test file; whichever of RC-2a and C0c lands first carries the split (edge c0e). Each verified as a
  pure move (`git diff --stat`, `cargo test -p <crate>` green).
- **D-15 (this branch; made true by C0b) — the IBP documents name `node_document`.** (i) The node memo's
  `effective_node_document` → `node_document`, in place, with the dated amendment paragraph (PM 157), and its cite
  restated (P-14). (ii) The node input's five rows (`:12`, `:67`, `:68`, `:102`, `:155`) → `node_document`, with a dated
  note. (iii) The parent's F10 cell (`docs/plans/2026-09-inline-box-predicate.md:1180`, "the effective-node-document rule
  … promoted unchanged") is not rewritten; one amendment line follows its table (PM 244 Q4). (iv) The N1 memo's n3
  hand-off ("promotes the effective-node-document rule into `EcsDom::effective_node_document` (planned)",
  `docs/plans/2026-10-ibp-sandbox-handler-gates.md:961-962`) is superseded; the N1 memo is landed and not edited — the
  supersession is stated in the node memo's amendment paragraph, which is what N4 reads (the precedent: the node memo's
  own 2026-10-25 erratum and PM 17 (i) paragraphs). All four land in the PR that carries the node-memo amendment; that
  is this branch, merged with the program memo; C0b is the PR that makes their content true (C0b memo §8).
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
  whose tests call them enables the feature **in `[dev-dependencies]` only** (`elidex-ecs = { workspace = true, features =
  ["fixtures"] }`; 26 crates, `git grep -l -E '\.create_(element|text|comment|document_fragment)\(' -- 'crates/*.rs' |
  cut -d/ -f1-3 | sort -u | wc -l`, verified 2026-11-03); no `.rs` file changes for it. (iii) **The CI-run census**
  (PM 244 item 5) is `scripts/check-fixture-boundary.sh` (NEW), run by a new `mise run check-fixture-boundary` task
  (part of `mise run ci`) and by a step of CI's `check` job: (a) `cargo tree --workspace -e normal,features -i
  elidex-ecs` must show no `fixtures` feature (no normal dependency edge enables it); (b) `cargo check --workspace` with
  **default features** — lib and bin targets, where dev-dependencies and their features are not active — must pass. A
  production call of an argumentless creator (a site C0b missed, or RC-2b's `create_document_fragment()` rebased onto
  C0b) therefore fails (b): it does not compile. It depends on no list of operations. The step is a deliberate
  exception to "cargo tasks gate on `--all-features`" (CLAUDE.md *Commands*): `--all-features`, which every other job
  uses (`.github/workflows/ci.yml:86`, `:89`, `:108`), turns `fixtures` on for every target and so cannot see the
  boundary; C0b's PR records the exception in CLAUDE.md's CI section. *Rejected:* migrating the ~2,577 fixture call lines
  onto the entry (a fixture needs no document; churn without a property gained); a dev-dependency-only fixture crate
  with an extension trait (production cannot see a dev-dependency in any build, but every one of 296 test files (verified 2026-11-03, the command above without `cut`/`sort`) gains an
  import — 12 of them over 1000 lines, each a touch-time split — and the crate still needs a pub, feature-gated door
  into `elidex-ecs`, which `--all-features` exposes, so it needs the same kind of CI guard); a grep ratchet over
  non-test code (rev 2's P-8: inline `#[cfg(test)]` modules make the population fuzzy).
- **D-18 (C0b; Q-4 decided) — one writer module, one reader.** `adopt_subtree` becomes a private helper of `adopt`
  (its one caller after D-8); `set_associated_document` and `get_associated_document` become `pub(crate)`;
  `AssociatedDocument`'s field becomes `pub(crate)`. The dom-api pass-4 hand stamps (`node_methods/clone.rs:282-285`, `:294-301`) are dead at C0b's head (the shadow
  root is stamped by `attach_shadow_with_init` from its host, the cloned host and each shallow clone by the shallow
  cloner) and are removed per edge c0g (§8). Writers after C0b, all inside `elidex-ecs`: the creators (D-17), the adopt walk
  (D-3), the cloner (`tree_clone.rs:221`, `:230`, `:417`). Readers outside `elidex-ecs`: `node_document` and
  `owner_document` only. Residual not closed by visibility: `world_mut()` is `pub`, so `world_mut().remove_one::<AssociatedDocument>`
  compiles; `git grep -n 'AssociatedDocument>' -- 'crates/*.rs' ':!crates/core/elidex-ecs'` → 0 production hits at
  `27b50dfe`, re-run in C0b's DoD.
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
- *C0b = the node-document fact, unsplittable.* Creation stamps without adoption make every moved parser node keep its
  old document once the fallbacks go (T3 worse than HEAD, K-0); adoption without creation stamps leaves the unstamped
  detached population (H-4) reading the getter's fallback or null; keeping the fallbacks until a later PR leaves a
  position proxy beside the fact (one-issue-one-way). So creation, maintenance, the reader collapse and the consumers'
  migration land together.
- *C0c = notification + `adoptNode`.* Without D-6, `adoptedCallback` is never called — HEAD's behaviour (H-12) — so K-0
  holds at C0b's head and no consumer-less event ships (the `Adopt` variant arrives with its consumer). The notification
  reads C0b's fact one-way (§2.2 A1×A4). `adoptNode`'s callback cell (T15) needs D-6, so they travel together.

**Order.** N1 (landed, `27b50dfe`) → **C0a** → **C0b** → **C0c** → C1 → T3 → slice b. C0a may land any time before
C0b; C0c any time after C0b and before C1. L3 orders RC-2a before RC-1 (L3 Rev 6.1), so RC-2a can land before C1 and
in either order with C0b (edge c0g) and C0c (edge c0e). Downstream needs: C1's template-contents value (D-C3, D-C4) needs C0b; C1's
per-segment `adoptedCallback` pairs (D-C5's notification half, T25) need C0c; T3 needs C0b (`node_document`); L3 RC-2 /
P1 need C0b (the entry, §2.0's boundary). C1's own edge text is fixed by C1's driver (H-C1).

**K-0 lens (umbrella K-0: no step worse than HEAD).** C0a: records only, each change toward the spec. C0b: every change
of the C0b memo §5.4 moves an outcome from HEAD's to the spec's; `adoptedCallback` stays uncalled as at HEAD. C0c: adds
the callback and `adoptNode` where HEAD has none.

### §1.1 Browser measurement

None is needed for the program's decisions (every outcome in §6 of the per-PR memos is defined by DOM §4.2.3 / §4.4 /
§4.5 / §4.9 / §4.11 / §5.5 and HTML §4.2.3 / §4.9.1 / §4.9.5 / §4.9.8 / §4.12.3 / §4.13.6 / §13.2.6.1 / §13.4). The
one spec-vs-browser question nearby (`replaceChild(X, X)`) is a registered slot with its own measurement step (§7).

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
| contents segments; `MutationEvent::Adopt` | adopt 3.3.3, 3.4 hook | iff some pair is `Some` | C0c |
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
| A1 fact | **I-U:** for every node n with a parent or host p, `node_document(n) == node_document_for_new_nodes(p)` — the parent's node document, except under the fragment-parse root (D-10), which exists only during a build; template contents are not children (their fragment is stamped by `attach_template_contents` and by the walk's contents edge). `node_document` reads only the stamp; no production reader derives a node document from tree position or `document_root()` | C0b |
| A2 position | At every link the child's adoption completes after its old-parent removal and before `link_node`; in `replace_child` before `old_child`'s removal; a replaced-in fragment is adopted itself; a pre-inserted fragment keeps its node document | C0b |
| A3 walk | Adoption re-homes the root's shadow-including inclusive descendants, each element's cached Attr entities and each reached template's contents (C2: same document), uncapped; from C0c in spec order (D-19), recording each contents fragment's pair | C0b, C0c |
| A4 notification | `Adopt` fires once per adopt call iff some pair is a (live, different) pair, before that root's `Insert`; every custom element whose pair is `Some` gets exactly one `Adopted` with its own pair, in spec order, before its `Connected` | C0c |
| A5 builds | No node of a fragment build other than its root ever holds the throwaway document; the build's returned nodes carry the redirect document from creation; `FragmentParseRoot` exists only during a build; DOMParser builds stamp their document | C0b |
| A6 consumers | Every production node-document read goes through `node_document`, or `owner_document` at exactly two sites; no position-based or "Document → itself" derivation remains | C0b |
| A7 EH contract | N1's gate and T3's step 3 read `node_document`; slice a's Insert arm sees the adopted document; C1 edits only `attach_template_contents` and the walk's contents edge (and, from C0c, each segment's `new`); K-0 holds at each PR's head | C0b, C0c |
| A8 creation | Every production creation takes a `NewNodeOwner` from the entry (or a clone's document); no lib or bin target can call an argumentless creator (it does not compile without the dev-only `fixtures` feature; CI checks that build, D-17); a node is ownerless only if its source has no node document | C0b |
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
| A3×A8 | yes | Attr entities are stamped at creation and re-homed by the walk (U1 Attr variant, U16) | C0b |
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
`idl dom Document` (`[CEReactions] Node adoptNode(Node node)`).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* 7.1; *replace* 1–11; *pre-insert*, *replace all* | adopt before link; replace's step order and record siblings | `crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`, `:95`, `:138`; `crates/script/elidex-script-session/src/mutation/mod.rs:672` (C0a D-12, C0b D-4 / D-5) | ✓ | yes |
| WHATWG DOM §4.2.6 Mixin ParentNode | *convert nodes into a node* | text and fragment given this's node document | `crates/dom/elidex-dom-api/src/child_node/mod.rs:302` (C0b D-11) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 1–3.4; `adoptNode` 1–4 | oldDocument, remove, per-descendant steps; adoptNode's errors | `EcsDom::adopt` (NEW, C0b); `MutationEvent::Adopt` (NEW, C0c); dom-api `AdoptNode` (NEW, C0c) | ✗ (3.2, 3.3.2 §7) | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument`; *clone a single node*; *string replace all* | the stored fact; the getter; clones given document | `crates/core/elidex-ecs/src/dom/mod.rs:817`; `crates/dom/elidex-dom-api/src/node_methods/core.rs:279`; `crates/core/elidex-ecs/src/dom/tree_clone.rs:251` (C0b) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *attach a shadow root* 5; `insertAdjacentText` 1 | given element's node document | `crates/core/elidex-ecs/src/dom/shadow.rs:171` (C0b) | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | *split a Text node* 5 | new node given node's node document | `crates/dom/elidex-dom-api/src/char_data/split_text.rs:148` (C0b) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | *extract* 1, 4.1, 16.1, 19.1; *clone the contents* 1 | fragment given range's start node's node document; clones | `crates/dom/elidex-dom-api/src/range/mutation.rs:266` (C0b; RC-2b's sites per §4.5) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including tree order* | the walk's order | `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (C0c D-19) | ✓ | no |
| WHATWG DOM §4.3.2 Queuing a mutation record | records of adopt's removal and of replace | siblings per current order | `mutation/mod.rs:341` (C0a, C0c) | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | *connected* | shadow-including root is a document | `walkers.rs:143` (unchanged; D-16) | ✗ (VM getter) | no |
| WHATWG HTML §4.12.3 The template element | *adopting steps*; *template contents* | contents into the appropriate owner (C2: the template's) | walk's contents edge (C0b); `attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557` | ✗ (C1) | yes |
| WHATWG HTML §4.13.6 Custom element reactions | *enqueue a custom element callback reaction* | `adoptedCallback` « oldDocument, document » | consumer arm + flush (C0c D-6) | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for the token* 3, 13; *insert a foreign element* 1–2; *insert a character* 5; *insert a comment* 4; *adjusted insertion location* 3–5 | intendedParent's / targetParent's node document; the redirection | strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`; tolerant `crates/dom/elidex-html-parser/src/convert.rs:237` (C0b D-10, D-11, D-20) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | steps 4, 11–15, 22–23 | throwaway, root, fragment in target's node document, redirection map[root], adopt fragment | `crates/core/elidex-ecs/src/dom/tree/teardown.rs:293`, `:330` (C0b D-8, D-10) | ✓ | yes |
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
engine-independent. The VM changes (C0c) are the flush arm's callback invocation and `adoptNode`'s native (§4.7).

§4.2 (reconcile HEAD → head), §4.3 (cost), §4.8 (citations) and §4.9 (comment sites) are per PR (in each per-PR memo).

### §4.4 Creation census (D-11)

**Population (definition).** Every production call of an `elidex-ecs` node creator — the creators of D-17 (i), the
argumentless creators of H-22, `create_document_node` / `create_document_root` (a Document is its own; no owner
decision), `clone_node_shallow`, `clone_subtree` — in any crate, outside test targets. **Command** (re-run at C0b's base
and at its head, C0b DoD):
`git grep -n -E '\.(create_(element|element_with_owner|element_ns|text|text_with_owner|comment|comment_with_owner|document_fragment|document_fragment_with_owner|document_type|attribute)|attach_template_contents|clone_node_shallow|clone_subtree)\(' -- 'crates/*.rs' | /usr/bin/grep -v -E '/tests/|tests\.rs|/tests_|test_helpers|test_util|/benches/'`;
the inline `#[cfg(test)]` modules it still lists are test targets (at `27b50dfe` every listed hit not in the snapshot below sits in one: `elidex-a11y` `names.rs` (from `:139`), `tree.rs` (from `:301`), `elidex-dom-api` `traversal.rs` (from `:1224`), `tree_nav.rs` (from `:492`), `test_util.rs` (a `#[cfg(test)]` module, `lib.rs:15-16`), `elidex-layout-block` `paint_order.rs` (from `:187`), `elidex-script-session` `mutation/html_fragment.rs` (from `:277`), `session.rs` (from `:190`)). At C0b's head the command is informational: the
guarantee is D-17's (an unmigrated argumentless call does not compile; an owner-taking call cannot be ownerless except
through the entry's `None`). **Snapshot at `27b50dfe`** (not the population; the per-PR memo re-derives it at C0b's
base):

| site (`27b50dfe`) | creates | owner today | C0b `source` |
|---|---|---|---|
| strict `insert.rs:61`, `:111`, `:133`, `:141`, `:210` | elements, foreign elements, comments, text | `None` / strict entry (`:108`) | intended / target parent (§13.2.6.1) |
| strict `insert.rs:155` | doctype | none | the parse document |
| strict `shadow.rs:124`, `:153` | contents fragment; DSD fallback template | strict entry; `None` | the template; intended parent |
| tolerant `convert.rs:251`, `:305`, `:309`, `:316`, `:284` | elements, text, comments, doctype; contents | `None`; threaded | the converted parent (D-20); the template |
| `teardown.rs:302` | fragment root `<html>` | `None` | the throwaway (D-10) |
| `shadow.rs:189` | shadow root | none | the host |
| `char_data/attr.rs:35`, `:90` | Attr | none | `this`; the element |
| `child_node/mod.rs:302`, `:328`, `:364` | `append('s')` texts; convert fragment | `None` | `this` |
| `element/tree.rs:431`; `node_methods/text_content.rs:119` | `insertAdjacentText` / `textContent` text | `owner_document(this)` | `this` |
| `char_data/split_text.rs:148` | split tail | `owner_document(entity)` | the split node |
| `element/select.rs:326-329`; `cssom_sheet.rs:213-214` | options; `<style>` text | `owner_document(..)` | the select; the sheet's element |
| `table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359` | thead, tfoot, caption, tbody, tr, tbody (table-created) | `None` | the table, via `create_table_element` (NEW, §4.9.1) |
| `table_mutation/mod.rs:448`; `:512` | section's tr; row's td | `None` | the section (§4.9.5 `insertRow` 2); the row (§4.9.8 `insertCell` 2) |
| `document.rs:235`, `:291`; `char_data/document_props.rs:337`, `:353`, `:377`, `:399` | createElement and kin | `Some(this)` | `this` (value unchanged) |
| `elidex-custom-elements/src/entity_spawn.rs:57` | CE element | caller's owner: the VM constructor passes `ctx.host().document_entity_opt()` (`crates/script/elidex-js/src/vm/host/custom_elements/html_element.rs:231`) | the caller's document (the constructor: the current global's associated Document) |
| `elidex-form/src/inert_document.rs:120` | DOMParser `<html>` | `Some(doc)` | `doc` |
| `elidex-form/src/submit.rs:526`; `elidex-style/src/pseudo.rs:63`; `elidex-layout-table/src/grid.rs:422`, `:467`; `elidex-layout/src/layout/anonymous_table.rs:159` | reset text; pseudo text; anonymous table wrappers | `None` | the element they are created for |
| `range/mutation.rs:266`, `:303`, `:354`, `:392` (A1–A4) | extract fragment, Text clones | `None` | §4.5 |
| `tree_clone.rs:251` (`clone_node_shallow`) | shallow clone | none | the operation's document (new parameter) |

### §4.5 L3's Range sites (L3 memo Appendix A at Rev 5 `b6e500e7`)

| id | site | after C0b | owner, by landing order |
|---|---|---|---|
| A1 | `range/mutation.rs:266` extract fragment | `node_document_for_new_nodes(start node)` (*extract* 1) | **C0b** if present at its base; else replaced by A6 |
| A2–A4 | `:303`, `:354`, `:392` Text clones | entry(the cloned Text) (*extract* 4.1, 16.1, 19.1 "a clone of": default document = its node document) | **C0b** if present; else replaced by A7 |
| A5 | `:505` → `split_text.rs:148-149` | entry(text) | **C0b** |
| A6 | RC-2b contents fragment (extract and clone, step 1) | entry(start node) | **RC-2b** if C0b is in RC-2b's base (the argumentless creator does not exist, D-17); **C0b** if RC-2b lands first (an existing production site of D-11's population; it does not compile until migrated) |
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

### §4.7 Layering: VM site → engine-independent API (C0c; R1 Ax1 MIN-1)

| VM site (marshalling only) | engine-independent API it calls | engine call |
|---|---|---|
| `document.adoptNode` native (NEW, `vm/host/document_factories.rs` after D-14 (b)) | `invoke_dom_api("adoptNode")` → dom-api `AdoptNode` (NEW) | session `apply_adopt_node` (NEW) → `EcsDom::adopt` |
| CE flush `Adopted` arm (`flush.rs:133-138`) | `invoke_callback(entity, adopted_callback, [old, new])` (the existing helper the other arms use, `flush.rs:131`) | — (two Document wrappers from the reaction's entities) |
| CE constructor (`custom_elements/html_element.rs:231`) | `elidex_custom_elements::spawn_custom_element_entity(…, owner)` | C0b: `owner` = `node_document_for_new_nodes(current global's associated Document)` |
| `ownerDocument` getter (`vm/host/node_methods_extras.rs:41`) | dom-api `ownerDocument.get` (`node_methods/core.rs:279`) | `owner_document` (C0b) |

## §5. Program populations and risks

- **P-1 linking sites** (H-2): definition + 3 calls; `rel.parent = ` → 2. Unchanged by the program.
- **P-3 node-document readers**: `/usr/bin/grep -rn 'owner_document(' crates --include='*.rs' | /usr/bin/grep -v
  '/tests/\|tests\.rs\|/tests_\|test_helpers\|///' | /usr/bin/grep -v 'fn owner_document\|validate_owner_document'` →
  21 lines at `27b50dfe` (list: C0b memo §5.1); at C0b's head `core.rs:279` and `selection.rs:618` only.
- **P-6 `MutationEvent` consumers**: `/usr/bin/grep -rln 'MutationEvent::Insert' crates --include='*.rs' |
  /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → 11 files; re-run by C0c and by RC-2a, whichever lands second (c0e).
- **Risks** (detail in the per-PR memos): R-1 `ownerDocument` after a move; R-2 a removed parse node keeps its document;
  R-3 N1's gate follows the adopted document; R-4 base URL; R-5 `adoptedCallback` / `adoptNode` reach author code
  (C0c); R-6 replace's records (C0a) and fragment (C0b); R-7 parse cost (C0b §4.3); R-8 the getter fallback deleted
  (C0b, guarded by D-17); R-9 P1 collision (§2.0); R-10 the `MutationEvent` census with L3 RC-2a (c0e).

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| template contents' owner document | C1 (`#11-template-contents-owner-document`) | under C2 the contents' node document is the template's |
| adopt steps 3.2, 3.3.2 (registries), scoped half | `#11-shadow-scoped-custom-element-registry` | a shadow root's / element's scoped registry is not re-pointed at adoption |
| adopt step 3.3.2, non-scoped half | L3's proposed `#11-document-null-registry-adopt-reset` (hooks C0c's `MutationEvent::Adopt`; L3 Rev 6.1 H8, PM 249); C0c's row | an element from a null-registry document adopted into a registry-bearing one keeps the null registry |
| `importNode` | **proposed** `#11-import-node` (R-6 (b)) | absent (H-15): `document.importNode` is a TypeError |
| `replaceChild(X, X)` | `#11-replace-child-self-records` (**registered**, PM 227) | a record-free no-op; *replace* gives two records; Chromium unmeasured |
| Selection API document-membership | **proposed** `#11-selection-document-membership` (R-6 (c)) | `same_document` (`selection.rs:614-619`) accepts a detached node whose node document is the selection's; step text unverified (webref cannot render the anchors) |
| VM `isConnected` | `#11-vm-isconnected-bound-document` (**registered**, PM 227, owner L3) | compares the composed root with the bound document |
| HTML `img` adopting steps | **proposed** `#11-img-adopting-steps-relevant-mutations` (R-6 (f)) | an `img` adopted into another document does not re-run *update the image data* (HTML §4.8.4.3.2 Reacting to DOM mutations, *relevant mutations*: "The element's adopting steps are run"); `/usr/bin/grep -rln 'relevant mutation\|update_image_data' crates` → no hit |
| VM Attr wrapper vs dom-api Attr entity | `#11-dom-api-attr-entity-model-unification` (registered, ledger `:721`) | the VM's Attr has no entity and no `ownerDocument` (H-17), so adopt 3.3.1 is observable only at unit level until that slot lands |

Own deferrals: **3 proposed slots** for pre-existing defects the program touches but does not fix ((b), (c), (f));
each has defect, owner, trigger and date in R-6. No planned implementation is deferred.

## §8. Edges, hand-offs and records

**Edges (program).**

| # | edge | kind | note |
|---|---|---|---|
| c0-pre-a | D-14 (a) → C0a | code predecessor | pure move |
| c0-pre-b | D-14 (b), (d), (e) → C0c | code predecessor | pure moves; (d) / (e) shared with L3 RC-0a / RC-2a (whoever lands first) |
| c0a | N1 (#580, landed) → C0b | code + semantic | clause (b) on N1's landed body; T8–T10 need N1's gate |
| c0-ab | C0a → C0b | code predecessor (soft) | C0a may land any time before C0b; both edit `apply_replace_child` (D-12 / D-5) |
| c0-bc | C0b → C0c | code + semantic | `adopt`, the walk, the stamps the pairs read |
| c0b | C0b → C1; C0c → C1 | semantic + code | C1's contents value (C0b); its per-segment pairs (C0c); C1's edge rows are C1's |
| c0c | C0b → T3 | data flow | T3 reads `node_document` |
| c0d | C0b → L3 P1 | boundary | §2.0 |
| c0e | C0c ↔ L3 RC-2a | census collision | RC-2a's `CloneCreated` and C0c's `Adopt` add variants to the same enum, arms to the same consumer and exhaustive matches; whichever lands second re-runs P-6 and adds its arm to every exhaustive match (`range/live.rs`, `mutation_hook.rs`) |
| c0f | C0b ↔ L3 RC-2b | creation entry | §4.5 A6: RC-2b calls the entry if C0b is in its base; C0b migrates RC-2b's sites if RC-2b lands first (compile-enforced either way, D-17) |
| c0g | C0b ↔ L3 RC-2a | file overlap `node_methods/clone.rs` | the pass-4 hand stamps in `node_methods/clone.rs` are `:282-285` (cloned host + replicated shadow root) and `:294-301` (the `!deep` shadow-child stamp; verified at `27b50dfe`, `git show 27b50dfe:crates/dom/elidex-dom-api/src/node_methods/clone.rs | sed -n '282,285p;294,301p'`). D14 (in RC-2a) removes `:294-301` in either order. Whichever of C0b and RC-2a lands second removes the stamps that are dead at its head; the other notes this in its DoD. If C0b is in RC-2a's base, RC-2a writes no hand stamp. If RC-2a lands first, only `:282-285` is left for C0b to remove (PM 247 / 249, L3 Rev 6.1 H9). C0b's re-measure scope includes RC-2's files (`node_methods/clone.rs`, `node_methods/tests/clone.rs`, `elidex-custom-elements/src/consumer.rs`, `dom/mutation_event.rs`, `dom/mod.rs`, `tree_clone.rs`, the RC-2b contents module) |

**Hand-offs.**
- **H-C1 to C1** (via its driver): rev 3's changes for C1's text: (i) the program is three PRs; C1's contents value
  needs C0b, its T25 pairs need C0c; (ii) `FragmentParseDocument` → `FragmentParseRoot` on the root (D-10), so its
  D-C4 last bullet, A2×A6 row and §3 §13.4 row ("re-homed through the throwaway and back", "the finishing walk") are
  stale (P-17), the value claim stands; (iii) `attach_template_contents`'s second argument is a `NewNodeOwner` (D-17);
  (iv) Q-2 is decided per-node (D-20), so its §4.3 constraint is met by construction; (v) frame `d262ca8c` → this rev.
- **H-T to T3** (rev 4, after C1): unchanged from rev 2 — step 3 calls `EcsDom::node_document`; R-B closed by C0b
  (cell T13).
- **H-P1 to L3's P1 umbrella** (via the PM): §2.0's contract, split by PR.
- **H-L3 to L3** (via the PM): (1) A6 per order (§4.5): if C0b is in RC-2b's base, RC-2b's contents fragment calls
  `node_document_for_new_nodes(start node)` for extract **and** cloneContents; if RC-2b lands first, C0b migrates it;
  either way a missed site does not compile (D-17), and C0b's T23 pins the observable for both orders; (2) edges c0e
  (RC-2a × C0c), c0f (RC-2b × C0b), c0g (RC-2a × C0b, `clone.rs`); (3) P-16 (L3 Rev 5 `:183-184` stale); (4) the
  slots `#11-selection-document-membership` (proposed) and `#11-vm-isconnected-bound-document` are L3's.
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
  each node with its own pair, in spec order."
- **R-3 umbrella §1.2, in place (D-9).** Added by C0a's PR, revision number per D-9's rule, row text:
  "| **C0** `EH-attrs-at-creation-C0` (owner: Layout lane, this program; PM decisions 143, 244), a program of three PRs
  in order **C0a** `EH-attrs-at-creation-C0a` (*replace* record order) → **C0b** `EH-attrs-at-creation-C0b` (the node
  document as the stored fact: creation entry, insertion adoption, one reader) → **C0c** `EH-attrs-at-creation-C0c`
  (`adoptedCallback` and `document.adoptNode`); program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` |
  slot `#11-cross-document-adopt-on-insert` (DOM §4.2.3 *insert* step 7.1, §4.5 *adopt*); closes T3's residual R-B
  (C0b). C0a: landed (#<PR>). C0b: planned. C0c: planned. | IBP-sandbox N1's implementation landed; C0a after D-14 (a);
  C0b after C0a; C0c after C0b and D-14 (b) |"; changelog: "*Rev 8.<n>* (<date>; carried by C0a's PR): §1.2 gains the C0
  program row (PM decisions 159, 223 O-1, 244)." C0b's PR replaces "C0b: planned." with "C0b: landed (#<PR>)." and
  C0c's PR "C0c: planned." with "C0c: landed (#<PR>)." (each with a changelog line); nothing else.
- **R-4 node memo, node input and parent (D-15; applied on this branch).** See the node memo's "Amendment 2026-11-01"
  paragraph (restated 2026-11-03), the node input's "Amendment 2026-11-03" note and the parent's amendment line after
  its §1 table.
- **R-5 N1 memo**: none in place (landed); its n3 hand-off's supersession is in R-4.
- **R-6 slot texts (defects only).** (a) `#11-ownerless-node-creation` — append at C0b's landing: "C0b (#<PR>): the
  `EcsDom` entry `node_document_for_new_nodes` exists; every production creator takes a `NewNodeOwner` from it, and the
  argumentless creators are test-only by construction (`elidex-ecs`'s `fixtures` feature, enabled only from
  `[dev-dependencies]`; checked by `scripts/check-fixture-boundary.sh` in CI). Remaining facets: template contents (C1); L3 RC-4's planned sites (they call
  the entry)." Also correct the slot's text (ledger `:683`), which still says "C1 puts both parser tiers on it": per PM
  223 O-2 / 227 C0 (C0b) migrates every production site including both parser tiers; C1 changes only
  `attach_template_contents`. (b) **proposed** `#11-import-node` — defect: "`document.importNode(node, options)` is
  absent (TypeError); DOM §4.5 `[CEReactions, NewObject] importNode` clones node into this." Owner: L3 (DOM node
  surface). Trigger: the C0 program landed (the adopt and clone-with-document machinery it needs exist). Re-eval:
  2026-12-15. (c) **proposed** `#11-selection-document-membership` — defect: "Selection methods accept a detached node
  whose node document is the selection's document (`selection.rs:614-619`); the Selection API's check is unverified
  against its step text (webref cannot render the anchors)." Owner: L3 (RC-1's file). Trigger: webref renders the
  Selection API anchors, or L3's next `selection.rs` touch. Re-eval: 2026-12-15. (d) `#11-vm-isconnected-bound-document`
  — registered (PM 227); no text change. (e) `#11-replace-child-self-records` — registered (PM 227); its re-eval "after
  C0 lands" reads "after C0a lands" (the record mechanism is C0a's); backstop 2026-12-15. (f) **proposed**
  `#11-img-adopting-steps-relevant-mutations` — defect as §7's row. Owner: Layout (image request state, IBP's F4–F6
  family). Trigger: C0c landed (the `Adopt` event is the hook). Re-eval: 2026-12-15.
- **R-7 roadmap**: none beyond R-2.
- **R-8 ledger order line (PM 143 sub-bullet, ledger `:132`)**: "Order: N1 implementation → T3 → C0 → C1 → slice b"
  is superseded by PM 153 ("N1 implementation → C0 → C1 → T3 → slice b") and, for C0's inside, by PM 244 (C0a → C0b →
  C0c). Append: "*Corrected 2026-11-03*: order per PM 153 / 244 — N1 → C0a → C0b → C0c → C1 → T3 → slice b."

## §9. Program definition of done

1. C0a, C0b, C0c each landed with its own per-PR plan-review passed and its own DoD met (their §9).
2. D-14 (a) landed before C0a; (b), (d), (e) before C0c (or (d) / (e) by L3 first); (c) recorded as triggered or not by C0b.
3. The umbrella row (R-3) added by C0a and updated by C0b and C0c.
4. The PM has applied R-1, R-2, R-6 (a), R-8 and decided R-6 (b), (c), (f).
5. `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo, the three per-PR memos and the node memo exits 0
   (the node input is an input file with no §3 and is not a preflight target) at each landing (run; do not trust a stored verdict).

## §10. R1 disposition (plan-review R1 on rev 2.1 `d262ca8c`; summary and per-axis files of 2026-11-03)

| finding | disposition | where |
|---|---|---|
| Ax3 + Ax5 SPLIT | applied (PM 244): program + C0a / C0b / C0c | §1, header |
| Ax3 IMP-1 (base case (c) not met) | each per-PR slice is now narrow and reviewed alone; the umbrella row names them (D-9) | §1, R-3 |
| Ax2 IMP-1 / Ax3 IMP-2 (Q-2) | decided per-node; `convert_node` gains the parent | D-20 |
| Ax2 IMP-2 / Ax4 IMP-1 (tolerant bottom-up × link-time adopt; current §13.4) | D-10 rewritten on the current §13.4 and re-keyed on the root (the redirection map); D-8's re-home deleted; §4.3 restated in C0b | D-8, D-10; C0b §4.3 |
| Ax2 IMP-3 (T19 unreachable) | VM cell withdrawn; U1 Attr variant + U16 (dom-api); H-17 corrected; the slot is not a predecessor (the unit cells pin D-3 (b) and D-11; the VM observable belongs to `#11-dom-api-attr-entity-model-unification`) | H-17, C0b §6 |
| Ax3 IMP-3 / Ax5 IMP-2 (Q-3, P-8 ratchet) | test-only by construction (feature + default-features CI check); T23 for Range | D-17, C0b T23 |
| Ax5 IMP-1 (A6 owner) / IMP-3 (clone.rs edge, re-measure scope) | §4.5 per order; edges c0f, c0g; RC-2a vs RC-2b named | §4.5, §8 |
| Ax5 IMP-4 (node memo cite) | restated | P-14, D-15 |
| Ax5 IMP-5 (node input, N1 hand-off) | swept; supersession in the node memo | D-15 |
| Ax5 IMP-6 (parent F10) | one amendment line (PM 244 Q4) | D-15 (iii) |
| Ax1 MIN-1 (mapping table; CE constructor owner) | added | §4.7, §4.4 |
| Ax1 MIN-2 (walker order) | one walker in spec order | D-19 |
| Ax2 MIN / Ax3 MIN-1, MIN-2 (visibility) | decided | D-18 |
| Ax3 MIN-3, MIN-4 (slots, `img`) | owner / trigger / date; `img` slot proposed; "Own deferrals" restated | §7, R-6 |
| Ax4 MIN (9) | D-8 wording (step 13), anchors + names, `create_table_element` only for table-created, §4.9.5 / §4.9.8 steps, extract 4.1 / 16.1 / 19.1, replace step 1, `split_text.rs` cites, citation rows, §3 rows (§4.2.3 base, §5.5 Range), `[CEReactions]` | D-10, D-12, D-13, §3, §4.4, §4.5; C0b §4.8, §4.9; C0c §4.8 |
| Ax5 MIN (C1 frame, ledger order, registered slots, ledger `:683`) | applied | header, R-6, R-8, P-18 |
| dry-run GAP-cand 1 (T16 recorder) | the session record layer: `apply_adopt_node` returns the records, the dom-api handler pushes them | D-13 |
| dry-run GAP-cand 2 (T18 host-linked fragment) | adoptable; DOM `adoptNode` has no exclusion (`webref body dom dom-document-adoptnode`) | D-13 |

## §11. Open questions (for the PM)

- **O-3 A default-features CI check.** D-17 adds one `cargo check --workspace` (default features) to CI's `check` job
  and to `mise run ci` (via `scripts/check-fixture-boundary.sh`), a deliberate exception to the all-features rule. Cost:
  one more check build per CI run. Recommended: accept (it is the only build in which the fixture boundary is visible).
  Before C0b's implementation the step is run once on the base: if the default-feature build is red today, that is a
  pre-existing defect reported to the PM, not absorbed.
- **O-4 Slot proposals R-6 (b), (c), (f)**: register, with the owners proposed.

# C0b `EH-attrs-at-creation-C0b` — the node document is the stored fact: one creation entry, insertion adoption, one reader (per-PR plan memo)

Per-PR slice **C0b** of the C0 program (program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`, rev 3.1;
"the program memo"), under the umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`. It carries program
decisions **D-1, D-2 (without the `Adopt` event), D-3 (a)(b), D-4, D-5, D-7, D-8, D-10, D-11, D-15 (iii), D-17, D-18,
D-20, D-21**, after two prerequisites: D-14 (c) (the `table_mutation/mod.rs` pure-move split) and the CI/tools
default-features check PR (PM 255). Program IDs (H-*, P-*, D-*, A*, R-*, edges c0*) are cited as the program memo's;
this memo's own IDs carry a `b` where they could collide (H-b*, U16, U17, U19, U20, T23). Siblings: the program memo's
header (C1 rev 3.1 `b5e47f4b`, L3 Rev 6.1 / PR #583 `a164ffe0`, T3 rev 3, N1 landed).

**Placement.** PM 143 / 153 (C0 before C1, order N1 → C0 → C1 → T3 → slice b); PM 223 O-2 (the entry in `EcsDom`);
PM 244 (C0b = the node-document fact, unsplittable; `adoptedCallback` stays uncalled as on HEAD; the `Adopt` variant
does not exist yet); PM 255 (user decision γ: the default-features check is a CI/tools PR before C0b; C0b adds the
fixture-boundary script); PM 258 (the guard by property; the Attr relation in C0b; D-14 (c) a prereq). Program order:
C0a → **C0b** → C0c (C0a may land any time before C0b).

**Scope.** One fact — the node document, stored as `AssociatedDocument`:
1. **Read** by one function, `EcsDom::node_document` (NEW); `owner_document` is the getter's wrapper; every
   position-based or "Document → itself" derivation and both fallbacks are deleted (D-1, D-7).
2. **Set at creation** by one entry, `EcsDom::node_document_for_new_nodes(source) -> NewNodeOwner` (NEW), whose only
   special case is the HTML §13.4 insertion target redirection on the fragment-parse root (`FragmentParseRoot`, NEW;
   D-10); every production creator takes a `NewNodeOwner` (NEW), and ownerless creation is test-only by construction
   (D-11, D-17, D-20).
3. **Kept true** by DOM *adopt* (`EcsDom::adopt`, NEW: steps 1, 2, 3.1, 3.3.1, 3.4 under C2) at the three linking
   primitives and the session's replace-with-fragment arm (D-2, D-3 (a)(b), D-4, D-5); the fragment parse needs no
   finishing re-home (D-8).
4. **Written and read only inside `elidex-ecs`** (D-18), the element↔Attr relation included (D-21).
5. The IBP documents' `effective_node_document` becomes this PR's `node_document` (D-15: the node memo and node
   input land with the program memo; the parent's F10 amendment line lands in this PR, §8).

**Not in scope.** `MutationEvent::Adopt` (NEW: C0c), the contents segments, the custom-element `Adopt` arm, `adoptedCallback`, the
spec-order walk and `document.adoptNode` (C0c: D-3 (c), D-6, D-13, D-19); the *replace* record order (C0a); template
contents' owner `f` (C1); program §7.

**Changelog.** *Rev 3.1* (2026-11-03, with the program memo's rev 3.1; plan-review R2 of the program, PM 255 / 258):
D-21 (NEW: the Attr relation; U19); D-17 (4) split per PM 255 and the guard by property over every test-only feature;
D-18's type crate-private; D-11's CE owner decision in `elidex-custom-elements` (U20); D-14 (c) a prereq; the parent's
F10 line carried here (§8); §1's verdict reasons; DoD 9 aligned with PM 249; the `27b50dfe` census snapshot moved here
(§4.4). *Rev 1* (2026-11-03): first draft, carved from the C0 memo rev 2.1 (`d262ca8c`) per plan-review R1's
SPLIT and PM 244, with R1's fixes (program §10): D-10 re-keyed on the root, D-8's re-home deleted, D-17 / D-18 / D-20
decided, T19 → U1 Attr variant + U16, T23 added, U4 restated.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame and re-measure scope

Coordinates are `27b50dfe` (program §0.1). C0b's base is `origin/main` with C0a landed (soft), and possibly L3's
RC-0a, RC-2a and / or RC-2b (program edges c0f, c0g). At the base (§9 item 1) every coordinate below is re-measured,
and the scope of the re-measure includes RC-2's files whether or not they have landed: `crates/dom/elidex-dom-api/src/node_methods/clone.rs`,
`crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` (or its RC-0a split), `crates/dom/elidex-custom-elements/src/consumer.rs`,
`crates/core/elidex-ecs/src/dom/mutation_event.rs`, `crates/core/elidex-ecs/src/dom/mod.rs`,
`crates/core/elidex-ecs/src/dom/tree_clone.rs`, and RC-2b's contents-algorithm module if present. The creation census
is re-derived by its command (§4.4), not carried.

### §0.2 Premises

The program's H-1 to H-6, H-11, H-13, H-16 to H-25 bind this PR (program §0.2). Added here:

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-b1 | The tolerant converter creates bottom-up and links after the recursion | `convert_node` (`crates/dom/elidex-html-parser/src/convert.rs:237`) creates the element (`:251`), converts its children under it (`:289`; template contents `:284-286`), and returns; the caller links it (`convert_children` `:74-75`, `convert_fragment_top_level` `:103-104`). The document entry is `convert.rs:19` |
| H-b2 | The strict builder inserts each element at creation, before its children | `insert_element_named` (`crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`) and the foreign path (`:108-111`) create and insert; strict template contents are attached after the template's insertion (`shadow.rs:124`, "post-insert" in `attach_template_contents`'s docstring, `crates/core/elidex-ecs/src/dom/mod.rs:553`) |
| H-b3 | The clone paths | `clone_subtree` (`tree_clone.rs:186`) takes a document override (`:208`) and stamps (`:221`, `:230`, `:417`); `clone_node_shallow(src)` (`:251`) takes none; the dom-api shadow pass hand-stamps the replicated shadow root and the cloned host (`node_methods/clone.rs:283-284`) and each shallow shadow child (`:294-301`, the `!deep` arm) |
| H-b4 | CI compiles every target with all features, so a dev-only feature is on for lib targets there | `.github/workflows/ci.yml:86` `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `:89` nextest `--all-features`; `:108` doc `--all-features` |

### §0.3 Premise errors (program's, restated for this PR's review)

- **P-2.** `clone_node_with_shadow_honor` stamps the replicated shadow root **and** the cloned host (`clone.rs:283-284`)
  and each shallow shadow child (`:299`), not only the shadow root. D-11 / D-18 subsume all three.
- **P-6.** An unstamped shadow root reads `None` (`find_tree_root` stops at the shadow root); the getter's
  `document_root()` fallback gives the page document. T4 appends into a second DOMParser document.
- **P-7.** The base-URL maintainer's Insert arm routes the recompute to `owner_doc(x)` = the page (x's stamp), whose tree
  does not contain `x`, so `dp`'s base is never recomputed (T12).
- **P-8.** `expand_fragment`'s "step 7: per-node adopt + insert" (`mutation/mod.rs:441`) and `fire_after_remove`'s
  "DOM `insert` adopts the node" (`tree/mutation.rs:330-339`) describe an adoption HEAD does not do; C0b makes them
  true.
- **P-10.** `owner_document`'s docstring calls the unstamped population "legacy" (`mod.rs:812-815`); it is most parser
  output, every shadow root and every shallow clone (H-4).
- **P-11.** `split_text.rs` cites "§4.10" (it is §4.11 Interface Text) and off-by-one steps; full list §4.9.
- **P-12 / P-13.** VM `isConnected` compares with the bound document (D-16, §7); `same_document` (`selection.rs:614-619`)
  is not touched (§7).
- **P-17.** The C1 memo's text on the re-home through the throwaway is stale after D-8 / D-10 (program H-C1).

### §0.4 Decisions (this PR; full statements in the program memo)

- **D-1 — `node_document`, one reader.** As program D-1. `owner_document`'s body becomes `None` for a Document, else
  `node_document`; its docstring is rewritten (P-10). The getter (`node_methods/core.rs:279`) becomes
  `owner_document(this)` mapped to a wrapper or null. P-3's population at C0b's head: `core.rs:279` and `selection.rs:618`.
- **D-2 — `EcsDom::adopt` (NEW), no event.** Steps 1, 2 (`detach_with_hook`, firing `Remove`), 3 (the walk if
  `document` ≠ oldDocument). `pub` (callers outside `elidex-ecs`: the session's fragment arm; C0c's `apply_adopt_node`).
  The silent re-home of a `None` oldDocument is the documented direction rule.
- **D-3 (a)(b) — the walk.** `adopt_subtree`'s body moves into `mutation.rs` as a private helper of `adopt` (D-18) and
  keeps `TemplateContentsWalk::Include`; it stamps every visited node (contents included: the template adopting steps
  under C2 adopt contents into the same document) and the entities of each visited element's `AttrEntityCache`
  (step 3.3.1). Nothing is recorded per segment (C0c).
- **D-4 — the primitives.** `target = node_document_for_new_nodes(parent).document()`; removal, then (if `target` is
  `Some` and differs from the child's node document) the walk, then `link_node`, `rev_version`, `fire_after_insert`.
  In `replace_child`: `new_child`'s removal and walk, then `old_child`'s removal, then the link (H-10's order kept).
- **D-5 — replace with a fragment.** The session's fragment arm (in `mutation/child_list.rs` after D-14 (a)) calls
  `dom.adopt(fragment, d)` with `d = node_document(parent)` (if `Some`) before `remove_child(parent, old_child)`, with
  the comment "*replace* step 6: adopt node (the fragment itself) into parent's node document" on C0a's numbering.
- **D-7 — derivations.** (a) clause (b) at `scripting.rs:106-124` → one arm,
  `dom.node_document(target).is_some_and(|d| d != bound_document)`; docstring slot paragraph `:49-87` rewritten (the
  slot closes at C0c; the paragraph states the fact, not the slot). (b) `owner_doc` (`document_base.rs:173-178`)
  deleted, callers `:313`, `:365`, `:395` read `node_document` (HTML §4.2.3 *set the frozen base URL* step 1, "Let
  document be element's node document"). (c) `href_accessor.rs:50`, `document_props.rs:75` → `node_document(x)`.
  (d) `focus/delegate.rs:85`, `tree_clone.rs:208`, `node_methods/clone.rs:242` → `node_document`; `teardown.rs:336` is
  deleted with D-8.
- **D-8 — no finishing re-home.** `finish_detached_fragment(root, document)` (its `context` parameter and `:336-337`
  deleted): snapshot `child_list_uncapped(root)`, destroy root, destroy the throwaway, return. Both parser tiers' calls
  drop the `context` argument (strict `tree_builder/mod.rs`, tolerant `lib.rs`).
- **D-10 — the entry and the redirection.** As program D-10 (rationale on the current §13.4 there). Implementation:
  `pub fn node_document_for_new_nodes(&self, source: Entity) -> NewNodeOwner` in `dom/mod.rs`:
  `if let Ok(r) = world.get::<&FragmentParseRoot>(source) { return NewNodeOwner(r.redirect_document) }` else
  `NewNodeOwner(self.node_document(source))`. `FragmentParseRoot { redirect_document: Option<Entity> }` (NEW) in
  `components.rs`. `begin_detached_fragment(&mut self, context: Entity)`: throwaway; `root =
  create_element_with_owner("html", …, node_document_for_new_nodes(throwaway))` (= the throwaway, §13.4 step 11);
  `append_child(throwaway, root)` (step 12; a compare: same document); insert `FragmentParseRoot { redirect_document:
  node_document(context) }` on root (step 15). The strict `TreeBuilder`'s method `node_document_for_new_nodes`
  (`tree_builder/mod.rs:271`) is deleted; its callers (`insert.rs:108`, `shadow.rs:124`) and every other strict
  creation pass `self.dom.node_document_for_new_nodes(intended_parent)` (§13.2.6.1: for elements the target parent of
  the appropriate place; for text and comments the adjusted insertion location's parent; for a doctype the parse
  document).
- **D-11 — the census migrated.** Every member of §4.4's population takes its owner from the entry (or the clone's
  document). `clone_node_shallow(&mut self, src, document: NewNodeOwner)` (DOM *clone a single node* "given document");
  `attach_shadow_with_init` spawns the shadow root with `node_document_for_new_nodes(host)` (*attach a shadow root*
  step 5); `create_attribute(name, owner)` (callers `CreateAttribute` `char_data/attr.rs:35` → `this`;
  `GetAttributeNode` `attr.rs:90` → the element, through D-21's `attr_node`); `create_document_type(…, owner)`; the table helpers through
  `create_table_element(dom, table, local_name)` (NEW, HTML §4.9.1 *create a table element*) for the six
  table-created sites (`table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359`), and the entry with the
  section (`:448`, §4.9.5 `insertRow` step 2) and the row (`:512`, §4.9.8 `insertCell` step 2) directly (their
  coordinates in `table_mutation/mod.rs` after D-14 (c), which moves only the handlers). **The CE constructor:**
  `elidex_custom_elements::spawn_custom_element_entity(dom, qualified_name, definition_name, document: Option<Entity>)
  -> Option<Entity>` (`entity_spawn.rs:51`) makes the owner decision — `Some(d)` → `create_element_with_owner(…,
  dom.node_document_for_new_nodes(d))`; `None` → spawns nothing and returns `None`. The VM
  (`custom_elements/html_element.rs:231`) passes `ctx.host().document_entity_opt()` (the current global's associated
  Document; `None` before bind and after unbind, `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:477-483`)
  and maps `None` to a `TypeError` (marshalling only). *Why not an ownerless element:* `NewNodeOwner` has no public
  constructor (D-17), and an unbound VM is an elidex-only state with no spec counterpart (the constructor's "current
  global object's associated Document" always exists in the spec), so refusing creation is the fail-closed reading. The
  two existing tests that spawn with `None` (`elidex-custom-elements/src/tests.rs:261`, `:283`) pass a fixture Document.
- **D-15 — the IBP documents.** The node memo amendment, node input note and N1 n3 supersession landed with the
  program memo; **this PR carries the parent's F10 amendment line** (PM 244 Q4: the PR that makes it true; exact text
  §8 R-b1) and makes all of them true (`EcsDom::node_document` exists). C0b's DoD re-runs `python3
  .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-09-ibp-sandbox.md` (the node input has no §3 and is
  not a preflight target).
- **D-17 — ownerless creation is test-only by construction (the CI-run census).** As program D-17. Concretely:
  (1) `NewNodeOwner` (NEW, `dom/mod.rs`): `pub struct NewNodeOwner(Option<Entity>)`, private field, `Copy`, no public
  constructor; `pub fn document(self) -> Option<Entity>` (C1's `f` reads it in `attach_template_contents`); `pub(crate)`
  constructors for the cloner, the parse root and the fixture creators. (2) The creators of program D-17 (i) take
  `owner: NewNodeOwner`; `validate_owner_document` keeps its liveness check. (3) `create_element`, `create_text`,
  `create_comment`, `create_document_fragment` (`mod.rs:331`, `:492`, `:519`, `:593`) gain
  `#[cfg(any(test, feature = "fixtures"))]` and their docstrings say "test fixture: an ownerless node, which production
  code cannot create"; `crates/core/elidex-ecs/Cargo.toml` gains `[features] fixtures = []`. Every crate whose tests call
  them (26 crates at `27b50dfe`, verified 2026-11-03: `git grep -l -E '\.create_(element|text|comment|document_fragment)\(' -- 'crates/*.rs'
  | cut -d/ -f1-3 | sort -u | wc -l`) enables the feature in `[dev-dependencies]` only; no `.rs` file changes for it.
  (4) **The guard, split by PM 255.** The default-features `cargo check --workspace` step in CI's `check` job, its
  `mise run ci` task and the CLAUDE.md exception to the all-features rule (H-b4) are the **CI/tools default-features
  check PR**'s, landed before C0b (edge c0-pre-ci; green at `27b50dfe`, run 2026-11-03). C0b adds
  `scripts/check-fixture-boundary.sh` (NEW) and appends its invocation to that step and that task; C0b writes no
  CLAUDE.md line and adds no step or task. The script's property (PM 258): *every feature that exposes test-only API is
  enabled only from `[dev-dependencies]`*. It holds `FEATURE_CLASS`, a table classifying every `[features]` key of every
  tracked `Cargo.toml` (`default` excluded) as `test-only` or `product` — at C0b: `test-only` `elidex-ecs/fixtures`,
  `elidex-net/test-hooks`; `product` `elidex-style/parallel`, `elidex-js/compat-webapi`, `elidex-js/engine`,
  `elidex-script-session/web-storage`, `elidex-storage-core/web-storage` (program H-24) — and fails (i) if a key is
  missing from the table (H-24's `awk` enumeration; an unknown falls on the guarded side), (ii) if a table row names
  no existing key (a stale row), (iii) if `cargo tree --workspace -e normal,features -i <crate>` lists `<crate> feature
  "<f>"` for a `test-only` `<f>` (H-24: the `-e normal` filter drops dev edges, so the assertion is not always red).
  *Why a table in the script:* one place, beside the guard that reads it; fail-closed by (i); readable with POSIX
  `awk` / `grep` (the `check` job declares no language runtime, CLAUDE.md *CI*). *Rejected:* a per-crate
  `[package.metadata]` marker (fails open: an unmarked test-only feature escapes; `cargo metadata` needs a JSON
  parser); a naming convention (binds existing names, `test-hooks` / `fixtures`, and fails open the same way). The
  guarantee, scoped (program A8): in the workspace's default-features build — CI's step and every product build — an
  argumentless creator does not exist, so an unmigrated production call does not compile, and no normal edge turns on
  a test-only feature; production code has no way to construct an ownerless `NewNodeOwner`. Outside it: code under a
  product feature no member enables (`elidex-style/parallel`, creating no node, H-24).
- **D-18 — visibility.** `adopt_subtree` private (in `mutation.rs`); `set_associated_document` / `get_associated_document`
  `pub(crate)`; the type `AssociatedDocument` `pub(crate)` and out of the re-export (`lib.rs:30`), so no code outside
  `elidex-ecs` can name it — `insert_one`, `get::<&mut _>`, a copy or `world_mut().remove_one::<_>` of it do not
  compile there (R2 Ax2 MIN-2, Ax3 MIN-3: closed by construction, no ratchet); its two outside intra-doc links
  (`entity_spawn.rs:45`, `node_methods/core.rs:247`) are rewritten, and `cargo doc -D warnings` catches a missed one. The dom-api pass-4 hand stamps `node_methods/clone.rs:282-285`
  and `:294-301` are dead at C0b's head (H-b3: the shadow root takes its host's document at attach, the host and each
  shallow clone the shallow cloner's); which of them C0b removes depends on RC-2a (edge c0g). The tests that call them (`node_methods/tests/clone.rs:873-973`, seven calls; scripting
  tests `scripting.rs:201`, `:234`) create their nodes with owners or read `node_document`.
- **D-20 — tolerant per-node entry.** `convert_node(handle, parent, dom, opts)`; `convert_children(rc_handle, parent,
  dom, opts)`, `convert_fragment_top_level(rc_handle, root, dom, opts)`, `try_attach_declarative_shadow(child, parent,
  dom, opts)` lose `owner_document`; each creation passes `dom.node_document_for_new_nodes(parent)`; a template's
  contents are attached with `node_document_for_new_nodes(entity)` and converted with `parent = fragment`; DSD content
  with `parent = shadow_root`. `lib.rs:158`'s `owner_doc` is deleted. Bottom-up order is kept (§4.3 shows why it costs
  nothing).
- **D-21 (C0b; R2 Ax2 IMP-1, PM 258) — an element's Attr entities are one relation, written only in `elidex-ecs`.**
  The spec's element *attribute list* and each attribute's *element* are one relation; elidex encodes it as the
  element-side `AttrEntityCache` (name → materialized Attr entity) and the Attr-side `AttrData.owner_element`, today
  written piecemeal by four dom-api handlers (H-23). **SoT:** the cache is the element's materialized attribute list
  (the adopt walk's set, D-3 (b)); `owner_element` its back-pointer — two encodings of one relation written together by
  one module, as `link_node` writes both sides of `TreeRelation` (H-2). **Writers** (in
  `crates/core/elidex-ecs/src/dom/attribute.rs`): `attr_node(element, name)` (get or materialize; the Attr created with
  `node_document_for_new_nodes(element)`; `GetAttributeNode`'s cache logic moves here); `set_attr_node(element, attr)`
  (DOM *set an attribute* 2–8: the in-use check, then *append an attribute* 1–3 or *replace an attribute* 2–5 — cache
  entry, element, node document = the element's, the old Attr's element null — with the value and *handle attribute
  changes* through the existing `set_attribute` chokepoint); and the `remove_attribute` chokepoint (`attribute.rs:291`)
  evicts a materialized Attr and nulls its element (*remove an attribute* 2–3), as `sync_cached_attr_value` already
  keeps its value in the set chokepoint. `AttrEntityCache` (type and field) and `AttrData.owner_element` become `pub(crate)` —
  `AttrEntityCache` leaves the re-export (`lib.rs:30`), its only outside users being the handlers this decision empties
  (`git grep -n AttrEntityCache -- 'crates/*.rs' ':!crates/core/elidex-ecs'` → `char_data/attr.rs`, `element/props.rs`,
  a VM comment); `attr_owner_element(attr)` (NEW, pub) replaces the dom-api reads (`attr.rs:317`, `:359`); `SetAttributeNode`, `RemoveAttributeNode`, `RemoveAttribute`
  (`props.rs:148`) and `GetAttributeNode` keep only marshalling and error mapping. *Rejected:* the SoT as a query over
  `AttrData.owner_element` (a world scan per adopted element, and the cache stays for identity: still two encodings,
  one of them then unmaintained); stamping inside the dom-api handlers (four partial writers of one relation: H-23's
  shape). Unit-observable only (H-17): cell U19 reads `node_document(attr)` after the element's adopt.

## §1. Shape and verdict

**Axes (program §2): 8 of 9** — A1, A2, A3, A5, A6, A7, A8, and A9 only through A8×A9 (Range creation sites). Edge-dense;
plan-review mandatory; a terminal unit once it passes (base case (c)). **Unsplittable** (program §1, the cuts
enumerated there): creation first (stamps at creation, reader collapsed, no adoption) makes a moved node keep its
creation document — T3 worse than HEAD (K-0); adoption first with the fallbacks kept is K-0-safe but leaves, for that
PR's lifetime, two node-document derivations (the stored fact and the position proxy), which one-issue-one-way
forbids, and its successor would carry cut (1)'s coupling anyway; the reader collapse last has the same two-derivation
period. D-21 belongs here because the walk's Attr edge (D-3 (b)) is correct only over a faithful attribute list.
Preflight's breadth warning (§3) is answered by the same argument: the rows are the creation, maintenance and reads of
one fact.

**K-0 lens.** Every observable change (§5.4) moves an outcome from HEAD's to the spec's; two are more restrictive than
HEAD and the spec's (R-3: a bound node moved into a DOMParser subtree stops compiling raw handlers; R-1). The
`adoptedCallback` is uncalled, as on HEAD (H-12); no `MutationEvent` variant is added.

## §2. Coupled invariants (program §2.2 pairs with PR = C0b)

The 23 pairs marked C0b in program §2.2, with what this PR's cells pin:

| pair | pinned by |
|---|---|
| A1×A2 | U4 (stamp readable inside the `Insert` consumer; `Remove` → `Insert` order), T1, T2 |
| A1×A3 | U1 (shadow, contents, Attr variants), T4, T5 |
| A1×A5 | U9, U11 (no build node holds the throwaway), U8 |
| A1×A6 | T20–T23, U8, U12, D-17's compile guarantee |
| A1×A7 | T8–T10 |
| A1×A8 | U2, U3, U7, U20; D-18 by visibility |
| A2×A3, A2×A8 | U1, U2, U3 |
| A2×A5, A3×A5 | U9 (a recording dispatcher installed around a fragment parse sees no walk: every link a compare), U11 |
| A2×A6 | T12 |
| A2×A7 | T8 |
| A3×A6, A3×A7 | T4 (handler variant), U1 |
| A3×A8 | U1 Attr variant, U16, U19 (D-21) |
| A5×A6, A5×A7, A5×A8, A5×A9 | T1, U11 (documentless context), existing DOMParser cells |
| A6×A7 | T8–T10; D-15 |
| A6×A8 | U15, T20–T23 |
| A7×A8 | (slice b's; the creation stamp is what N1's gate reads — T10) |
| A8×A9 | T23 (Range) |

## §3. Spec coverage map

Lookups (2026-11-03): `heading dom 4.2.3 / 4.2.6 / 4.4 / 4.5 / 4.9 / 4.11 / 5.5`; `heading html 2.4.3 / 4.2.3 /
4.9.1 / 4.9.5 / 4.9.8 / 4.12.3 / 8.1.8.1 / 8.5.1 / 13.2.6.1 / 13.4`; `body dom concept-node-insert` (7.1),
`concept-node-adopt` (1–3), `concept-node-replace` (6), `concept-range-extract` (1, 4.1, 16.1, 19.1),
`concept-range-clone` (1), `concept-text-split` (5); `body html html-fragment-parsing-algorithm` (4, 11–15, 22–23),
`concept-element-attributes-set` (2–8), `-append` (1–3), `-replace` (2–5), `-remove` (2–3);
`creating-and-inserting-nodes` (*create an element for the token* 3, 13; *insert a foreign element* 1–2; *insert a
character* 5; *insert a comment* 4; *adjusted insertion location* 3–5), `create-a-table-element`, `dom-tbody-insertrow`
(2), `dom-tr-insertcell` (2), `set-the-frozen-base-url` (1), `getting-the-current-value-of-the-event-handler` (3.1).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* 7.1; *replace* 6 | adopt into parent's node document before the link; replace adopts node (a fragment included) | `append_child` `crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`, `insert_before` `:95`, `replace_child` `:138`; session fragment arm `crates/script/elidex-script-session/src/mutation/mod.rs:713` | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 1, 2, 3, 3.1, 3.3.1, 3.4 | oldDocument; remove; per descendant: node document, attributes, adopting steps (C2 identity) | `EcsDom::adopt` (NEW); `adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` (moves, private) | ✗ (3.2, 3.3.2: program §7; 3.3.3: C0c) | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument`; *clone a single node* (given document); *string replace all* 2 | the stored fact; null for a Document; clones and texts given a document | `crates/core/elidex-ecs/src/dom/mod.rs:817`; `crates/dom/elidex-dom-api/src/node_methods/core.rs:279`; `crates/core/elidex-ecs/src/dom/tree_clone.rs:251`; `crates/dom/elidex-dom-api/src/node_methods/text_content.rs:119` | ✓ | yes |
| WHATWG DOM §4.2.6 Mixin ParentNode | *convert nodes into a node* 1, 3 | texts and fragment given this's node document | `crates/dom/elidex-dom-api/src/child_node/mod.rs:302` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *attach a shadow root* 5; `insertAdjacentText` 1 | given element's node document | `crates/core/elidex-ecs/src/dom/shadow.rs:171`; `crates/dom/elidex-dom-api/src/element/tree.rs:431-432` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *set an attribute* 2–8; *append an attribute* 1–3; *replace an attribute* 2–5; *remove an attribute* 2–3 | list, element and node document written together (D-21) | `crates/core/elidex-ecs/src/dom/attribute.rs:291` (`remove_attribute`), `attr_node` / `set_attr_node` (NEW); handlers `crates/dom/elidex-dom-api/src/char_data/attr.rs:42`, `:118`, `:188`, `crates/dom/elidex-dom-api/src/element/props.rs:148` | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | *split a Text node* 5 | new node given node's node document | `crates/dom/elidex-dom-api/src/char_data/split_text.rs:148` | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | *extract* 1, 4.1, 16.1, 19.1; *clone the contents* 1 | fragment given range's start node's node document; clones given their source's | `crates/dom/elidex-dom-api/src/range/mutation.rs:266` (and RC-2b's sites if present, program §4.5) | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for the token* 3; *insert a foreign element* 1–2; *insert a character* 5; *insert a comment* 4; *adjusted insertion location* 3–5 | intendedParent's / targetParent's node document; insertions into root redirected | strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`, `:111`, `:133`, `:141`, `:155`, `:210`, `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:124`, `:153`; tolerant `crates/dom/elidex-html-parser/src/convert.rs:237` | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* (`#html-fragment-parsing-algorithm`) steps 4, 11, 12, 14, 15, 22, 23 | throwaway; root in it; fragment in target's node document; map[root] → fragment; adopt fragment (identity) | `begin_detached_fragment` `teardown.rs:293`, `finish_detached_fragment` `teardown.rs:330`; `FragmentParseRoot` (NEW) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *template contents* establishment; *adopting steps* 1–2 | contents created in (C2) the template's node document; adopted with it | `attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557` (second argument `NewNodeOwner`); the walk's contents edge | ✗ (C1's `f`) | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | *parse HTML from a string* | the new document owns its nodes | `crates/dom/elidex-form/src/inert_document.rs:120` | ✓ | yes |
| WHATWG HTML §4.9.1 The table element | *create a table element*; `createTHead` / `createTFoot` / `createCaption` / `createTBody`; `insertRow` 2, 3.1 | given tableElement's node document | `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359` → `create_table_element` (NEW) | ✓ | yes |
| WHATWG HTML §4.9.5 The tbody element | `insertRow` 2 | tr given this element's node document | `table_mutation/mod.rs:448` | ✓ | yes |
| WHATWG HTML §4.9.8 The tr element | `insertCell` 2 | td given this tr element's node document | `table_mutation/mod.rs:512` | ✓ | yes |
| WHATWG HTML §4.2.3 The base element | *set the frozen base URL* 1 | element's node document | `crates/dom/elidex-dom-api/src/element/document_base.rs:173` (callers `:313`, `:365`, `:395`) | ✓ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *document base URL* | per node document | `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`; `crates/dom/elidex-dom-api/src/char_data/document_props.rs:75` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* 3.1 (`#getting-the-current-value-of-the-event-handler`) | "document be element's node document" | clause (b) `crates/script/elidex-script-session/src/scripting.rs:97` | ✓ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-built trees. C0b parses nothing new and stores no string. What changes
is which Document an author node is attributed to at creation and after a move, and three reads: `ownerDocument`, URL
resolution through the document base URL, N1's compile gate. No new script entry point.

## §4. Mechanism

### §4.1 Home / kind

`node_document`, `node_document_for_new_nodes`, `NewNodeOwner`, `adopt`, `FragmentParseRoot` (NEW) in `elidex-ecs`;
the argumentless fixture creators behind `elidex-ecs`'s `fixtures` feature (NEW). One component is added (`FragmentParseRoot`, transient, on one
entity per build); no event variant; no side-store.

### §4.2 Reconcile — HEAD → C0b

| site | HEAD | C0b |
|---|---|---|
| `owner_document` `mod.rs:817-856` | stamp, else tree-root Document, else fragment root's stamp | `None` for a Document, else `node_document` |
| getter `core.rs:279` | `owner_document(this).or_else(document_root)` | `owner_document(this)` |
| creators `mod.rs:331-647` | four argumentless shims + `Option<Entity>` owners | `NewNodeOwner` owners; shims behind `#[cfg(any(test, feature = "fixtures"))]` |
| `append_child` / `insert_before` `mutation.rs:40`, `:114` | `detach_with_hook` then link | removal, walk if the target differs, link |
| `replace_child` `mutation.rs:164-197` | detach `new_child`, detach `old_child`, link | removal + walk of `new_child`, `old_child`'s removal, link |
| `adopt_subtree` `teardown.rs:251-259` (`pub`) | walk + stamp | private in `mutation.rs`; + attrs |
| `begin_detached_fragment` `teardown.rs:293` | no context; ownerless root | `context`; root in the throwaway; `FragmentParseRoot` on root |
| `finish_detached_fragment` `teardown.rs:330-343` | re-home walk, detach, despawn | detach, despawn |
| strict entry `tree_builder/mod.rs:271` | `pub(super)` method | deleted; `EcsDom` entry with the intended parent |
| tolerant `lib.rs:158`, `convert.rs` | `owner_doc` threaded to contents only | entry per node with the converted parent (D-20) |
| session fragment replace `mutation/mod.rs:713` | `remove_child(old_child)` first | `adopt(fragment, d)`, then `remove_child(old_child)` |
| clause (b) `scripting.rs:106-124` | Document arm + composed-root proxy | one arm over `node_document` |
| `document_base.rs:173-178`; `href_accessor.rs:50`; `document_props.rs:75` | local derivations | `node_document` |
| `set_associated_document` / `get_associated_document` `mod.rs:789`, `:777`; `AssociatedDocument` `components.rs:595`, re-export `lib.rs:30` | `pub` | `pub(crate)`; not re-exported |
| element↔Attr relation (`char_data/attr.rs:90-110`, `:180`, `:233`; `element/props.rs:148`) | four partial writers in dom-api (H-23) | `attr_node`, `set_attr_node`, `remove_attribute` in `elidex-ecs` (D-21); handlers marshal |
| `spawn_custom_element_entity` `entity_spawn.rs:51` | `owner: Option<Entity>` from the VM | `document: Option<Entity>` → owner decision inside; `None` → `None` (D-11) |
| dom-api pass-4 hand stamps `clone.rs:282-285`, `:294-301` | stamp the shadow root, host, shallow shadow child | dead; removed per c0g |
| creation sites (§4.4) | ownerless or `owner_document(x)` | `node_document_for_new_nodes(source)` |

### §4.3 Cost (R1 Ax2 IMP-2: rev 2's claim restated)

Every parse — document or fragment, either tier — creates each node with `node_document_for_new_nodes(p)` for the `p`
it is then linked under (strict: the intended parent of §13.2.6.1, H-b2; tolerant: the converted parent, D-20; under
the fragment root: the redirect document, D-10). Each link's adopt therefore finds `target ==` the child's stamp and
does no walk: one component read for the target, one for the child. Moves inside a build (strict foster parenting, the
adoption agency's reparenting) stay inside one document: compares. The fragment parse's finishing walk (HEAD: one
uncapped walk over every returned node) is deleted (D-8). The entry per creation is one `FragmentParseRoot` lookup and
one `node_document` read; the stamp joins the spawn bundle where the creator allows it (`create_*_with_owner` today
spawns then `insert_one`s; the PR may fold the component into the spawn tuple). Measured in the PR (§9 item 7): `mise
run bench` CSS / style / layout parse benches before and after; a regression beyond noise goes to the PM with the
profile, the fallbacks are not restored.

### §4.4 Creation census (D-11)

The population is defined in program §4.4 (every production call of an `elidex-ecs` node creator), enumerated by the
command there, run at C0b's base (§9 item 1) and at its head (§9 item 3). The program's table is the **`27b50dfe`
snapshot**; the base's run replaces it in the PR description, with each member's `source`. Members not in the snapshot
(RC-2b's contents-algorithm sites if RC-2b landed first; anything else landed since) are migrated by the same rule:
`source` = the node whose node document the spec creates it given (*clone the contents* / *extract* step 1: the
range's start node; a clone: the operation's document). The guarantee does not rest on the run: D-17 makes an
unmigrated argumentless call fail to compile.

**Snapshot at `27b50dfe`** (moved from the program memo at rev 3.1; not the population; the base's run replaces it):

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
| `element/tree.rs:431-432`; `node_methods/text_content.rs:119-120` | `insertAdjacentText` / `textContent` text | `owner_document(this)` | `this` |
| `char_data/split_text.rs:148-149` | split tail | `owner_document(entity)` | the split node |
| `element/select.rs:326-329`; `cssom_sheet.rs:213-214` | options; `<style>` text | `owner_document(..)` | the select; the sheet's element |
| `table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359` | thead, tfoot, caption, tbody, tr, tbody (table-created) | `None` | the table, via `create_table_element` (NEW, §4.9.1) |
| `table_mutation/mod.rs:448`; `:512` | section's tr; row's td | `None` | the section (§4.9.5 `insertRow` 2); the row (§4.9.8 `insertCell` 2) |
| `document.rs:235`, `:264`, `:291`; `char_data/document_props.rs:337`, `:353`, `:377`, `:399` | createElement and kin; a created template's contents (`:264`) | `Some(this)` | `this` (value unchanged); `:264` the template |
| `node_methods/clone.rs:321`, `:323`, `:422` | deep clone, shallow clone, a cloned template's contents | `doc_override`; none; `clone_doc` | the operation's document (*clone a single node* "given document") |
| `elidex-custom-elements/src/entity_spawn.rs:57` | CE element | caller's owner: the VM constructor passes `ctx.host().document_entity_opt()` (`crates/script/elidex-js/src/vm/host/custom_elements/html_element.rs:231`) | entry(the document argument) inside `spawn_custom_element_entity`; `None` → no element (D-11) |
| `elidex-form/src/inert_document.rs:120` | DOMParser `<html>` | `Some(doc)` | `doc` |
| `elidex-form/src/submit.rs:526`; `elidex-style/src/pseudo.rs:63`; `elidex-layout-table/src/grid.rs:422`, `:467`; `elidex-layout/src/layout/anonymous_table.rs:159` | reset text; pseudo text; anonymous table wrappers | `None` | the element they are created for |
| `range/mutation.rs:266`, `:303`, `:354`, `:392` (A1–A4) | extract fragment, Text clones | `None` | §4.5 |
| `tree_clone.rs:251` (`clone_node_shallow`) | shallow clone | none | the operation's document (new parameter) |

### §4.5 Range sites

Program §4.5. C0b's share: A1–A5 if present at the base; A6 if RC-2b landed first (an existing production site of the
population); A7–A9 through the cloner and the reader in either order.

### §4.7 ECS-native and Layering checks

One fact (the component), one creation entry, one later writer (the adopt walk), one reader (`node_document`). The
redirection is a component on the entity it describes. No side-store, no observer registry. Writes and reads are
confined to `elidex-ecs` by visibility (D-18), and so is the element↔Attr relation (D-21: one relation, two encodings
written by one module, the `TreeRelation` idiom). No algorithm in `vm/host/`: the CE constructor passes the marshalled
document to `spawn_custom_element_entity`, which makes the owner decision (program §4.7).

### §4.8 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `node_document` (NEW) | DOM §4.4 *node document* (`#concept-node-document`) |
| `owner_document` (doc rewrite) | DOM §4.4 `ownerDocument` getter (`#dom-node-ownerdocument`) |
| `node_document_for_new_nodes` (NEW home) | DOM §4.4 *node document* ("set upon creation"); HTML §13.2.6.1 *create an element for the token* step 3, *insert a character* step 5, *insert a comment* step 4, *adjusted insertion location* steps 3–5; HTML §13.4 steps 14–15 (the redirection) |
| `NewNodeOwner` (NEW) | DOM §4.4 *node document*; the program's D-17 (why ownerless is test-only) |
| `attr_node` / `set_attr_node` (NEW); `remove_attribute` (doc) | DOM §4.9 *get an attribute by name*, *set an attribute* steps 2–8, *append an attribute* 1–3, *replace an attribute* 2–5 (`#concept-element-attributes-set`, `-append`, `-replace`); *remove an attribute* 2–3 (`#concept-element-attributes-remove`) |
| `spawn_custom_element_entity` (doc) | DOM §4.4 *node document* (set upon creation); the `None` arm is elidex-only (D-11) |
| `FragmentParseRoot` (NEW) | HTML §13.4 *HTML fragment parsing algorithm* step 15 (*insertion target redirection map*[root] = (fragment, null)), step 14 (fragment in target's node document) |
| `adopt` (NEW) | DOM §4.5 *adopt* steps 1–3 (3.2, 3.3.2 not modelled; 3.3.3 C0c), named |
| `append_child` / `insert_before` / `replace_child` (doc) | DOM §4.2.3 *insert* step 7.1; *replace* step 6 |
| session fragment arm (doc) | DOM §4.2.3 *replace* step 6 |
| `begin_detached_fragment` (doc rewrite) | HTML §13.4 steps 4, 11, 12, 15 |
| `finish_detached_fragment` (doc rewrite) | HTML §13.4 steps 22–23 (identity: step 14 created the fragment in target's node document) |
| `attach_template_contents` (doc) | HTML §4.12.3 *template contents* (C2: the template's node document) |
| `create_attribute` (doc) | DOM §4.5 `createAttribute` ("creating an attribute given this"; `#dom-document-createattribute`) |
| `create_document_type` (doc) | DOM §4.4 *node document* (set upon creation) |
| `attach_shadow_with_init` (doc) | DOM §4.9 *attach a shadow root* step 5 ("given element's node document") |
| `clone_node_shallow` (signature + doc) | DOM §4.4 *clone a single node* ("given document") |
| `create_table_element` (NEW) | HTML §4.9.1 *create a table element* (`#create-a-table-element`) |
| section `insertRow` / row `insertCell` sites | HTML §4.9.5 `insertRow` step 2 (`#dom-tbody-insertrow`); HTML §4.9.8 `insertCell` step 2 (`#dom-tr-insertcell`) |
| strict tree-builder creation sites | HTML §13.2.6.1 (the named algorithm and step at each site) |
| tolerant `convert_node` (doc) | HTML §13.2.6.1 *create an element for the token* step 3 (intendedParent = the converted parent for an RcDom conversion) |
| inert document `<html>` | HTML §8.5.1 *parse HTML from a string* |
| `split_text_at_offset` (doc fix) | DOM §4.11 *split a Text node* step 5 (P-11) |
| clause (b) docstring (rewrite) | HTML §8.1.8.1 *get the current value of the event handler* step 3.1–3.2; DOM §4.4 *node document* |
| `BaseUrlMaintainer` callers | HTML §4.2.3 *set the frozen base URL* step 1 |
| the four fixture creators (doc) | none (test fixtures; the docstring states that production cannot create an ownerless node, program D-17) |

### §4.9 Comment sites made true or false by C0b

`/usr/bin/grep -rn 'cross-document-adopt-on-insert' crates` → `scripting.rs:65`, `:83` (rewritten, D-7 (a)) and N1's
cell comments `crates/script/elidex-js/src/vm/tests/tests_event_handler_scripting_gates.rs:149-153` (C31) and `:192-195`
(C8b (iii)) ("elidex's insertion does not adopt": false after C0b; outcomes unchanged); P-8's two comments (true);
`owner_document`'s docstring `mod.rs:802-816` (P-10); `get_associated_document`'s "layers tree-root fallback"
(`mod.rs:772-775`); the shims' "New callers … should prefer the `_with_owner` variant" (`mod.rs:325-330`);
`attach_template_contents`'s caller list and "tolerant: the conversion's threaded document" (`mod.rs:541-556`);
`adopt_subtree`'s docstring (`teardown.rs:245-250`); `begin_detached_fragment`'s parent-link paragraph
(`teardown.rs:276-289`: reason (1) is false after D-10, reason (2) — the strict rollback's reach — stays);
`finish_detached_fragment`'s re-home and "orphan-detached" paragraphs (`teardown.rs:316-322`); `clone.rs:276-281`,
`:290-299` ("spawns the replicated shadow root without a node document", "The shallow cloner never stamps");
`walkers.rs:23` (`adopt_subtree` "so HTML §13.4 fragment adopt moves the contents") and `:94`; `lib.rs:150-159`
(tolerant's `owner_doc` comment); the getter's fallback comment (`core.rs:245-278`); `document_props.rs:59`
("falls back to the tree-root walk"); `split_text.rs`: §4.10 → §4.11 at `:1`, `:27`, `:34`, `:71`, `:153`, `:194`,
`:215`, `:362`, and the step offsets against `webref body dom concept-text-split` (`:27` and `:153` "step 7.2" → steps
7.4–7.5, the parent-offset updates; `:141` "Step 4: allocate" → step 5; `:362` "step 8" → step 7.2; `:194` "7.2/7.3"
correct), `:140-147` ("legacy entities relying on the tree-root Document fallback").

## §5. Populations, tests that change, touched files, risks

### §5.1 Populations (re-runnable; program §5 for P-1, P-3, P-6)

- **P-2 adopt callers**: `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → at `27b50dfe` def + 1 prod
  (`teardown.rs:337`) + 1 test; at C0b's head the private helper's def + `adopt`'s call only. `/usr/bin/grep -rnF
  '.adopt(' crates --include='*.rs'` → the primitives' shared path, the session fragment arm, tests.
- **P-3 readers** at `27b50dfe` (21 lines): `teardown.rs:336`, `tree_clone.rs:208`, `scripting.rs:120`,
  `cssom_sheet.rs:213`, `href_accessor.rs:50`, `element/tree.rs:431`, `select.rs:326`, `document_base.rs:177`,
  `document_props.rs:75`, `split_text.rs:148`, `focus/delegate.rs:85`, `node_methods/core.rs:279`, `text_content.rs:119`,
  `node_methods/clone.rs:242`, `selection.rs:618`, `elidex-html-parser/src/lib.rs:158`,
  `elidex-html-parser-strict/src/tree_builder/mod.rs:273` (+ the test fn name `range/live.rs:900`, the VM getter name
  `vm/host/node_methods_extras.rs:41`, comments `document_base.rs:357`, `node_methods/clone.rs:418`). At C0b's head:
  `core.rs:279`, `selection.rs:618`.
- **P-4 position proxies**: `/usr/bin/grep -rn 'find_tree_root_composed' crates --include='*.rs' | /usr/bin/grep -v
  '/tests/\|tests\.rs\|/tests_'` → `scripting.rs:115` (deleted), `node_proto.rs:451`, `:455` (D-16), def
  `walkers.rs:342`; `/usr/bin/grep -rn 'document_root())' crates/dom/elidex-dom-api/src/node_methods/core.rs` → 1 → 0.
- **P-5 idioms**: `/usr/bin/grep -rn 'owner_document(.*)\.unwrap_or(' crates --include='*.rs'` → 2 → 0;
  `/usr/bin/grep -rn 'fn owner_doc\b' crates --include='*.rs'` → 1 → 0.
- **P-7 Document-destroying paths**: `finish_detached_fragment` (`teardown.rs:341`), the strict error path
  (`despawn_subtree(document)`, `tree_builder/mod.rs:234`); DOMParser documents are never destroyed (H-18).
- **P-8 creation census**: program §4.4's command (population, not list); D-17's compile guarantee.
- **P-9 write surface**: `git grep -n 'set_associated_document(\|AssociatedDocument' -- 'crates/*.rs' ':!crates/core/elidex-ecs'`
  → at C0b's head: comments only (the type cannot be named outside `elidex-ecs`, D-18; a code use is a compile error).
- **P-10 Attr relation writers** (D-21): `git grep -n -E 'owner_element *=|AttrEntityCache' -- 'crates/*.rs'
  ':!crates/core/elidex-ecs'` → 23 lines at `27b50dfe`: `char_data/attr.rs` 9 (writes `:97`, `:180`, `:233`; cache `:3`, `:61`, `:77`, `:101-107`), `element/props.rs` 3,
  dom-api tests writing `owner_element` directly (`char_data/tests/attr.rs` 6, `node_methods/tests/contains_compare.rs`
  3: rewritten over `set_attr_node`, §5.2), a VM comment (`element_attrs.rs:374`) and a property-name table entry
  (`vm/well_known.rs:417`, not a write); at C0b's head the last two only.

### §5.2 Existing tests whose expectation changes

Known at `27b50dfe`: `crates/core/elidex-ecs/src/dom/tests/associated_document.rs` — `:31`, `:64`, `:172` assert the
deleted fallbacks (rewritten to the stamp contract); `crates/core/elidex-ecs/src/dom/tests/destroy.rs:310`
(`adopt_subtree_rehomes_template_content_fragment`, calls the now-private helper → rewritten over `adopt`); the
`destroy.rs` fragment-teardown cells that read `finish_detached_fragment`'s re-home; `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs`
(9 `owner_document(` calls; the seven `set_associated_document` calls `:873-973`; `:902`'s
`clone_node_shallow_shadow_children_carry_owner_document` pins the hand stamp); `crates/dom/elidex-html-parser/src/tests.rs`
(6; `:618` "Both backends must leave it unstamped") and `crates/dom/elidex-html-parser-strict/src/tree_builder/tests_fragment.rs`
(2, `:351`); the `scripting.rs` inline tests (`:192`, `:226`, `:269` comments; `:201`, `:234` raw reads); the N1 cells
C8, C8b–C8e, C8g, C8h, C30, C31 in `tests_event_handler_scripting_gates.rs`; the base-URL maintainer tests in
`document_base.rs`; `crates/script/elidex-js/src/vm/tests/tests_associated_document.rs`; `elidex-custom-elements/src/tests.rs:259-290`
(spawn with `None`, D-11); `crates/dom/elidex-dom-api/src/char_data/tests/attr.rs` (D-21). The complete list is a
pre-implementation step (§9 item 2), which also lists every fixture test whose assertion reads the node document of an
ownerless (fixture-created) node — a state production cannot reach after C0b (program D-17 *Rejected*) — and rewrites
each over an owned node or deletes it as testing nothing reachable.

### §5.3 Touched files

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | `node_document`, entry, `NewNodeOwner` (NEW); creators; visibility; shims out | net ≈ +10; < 1000; re-measure (RC-2 may add `fire_clone_created`) |
| `crates/core/elidex-ecs/src/components.rs` | 822 | `FragmentParseRoot` (NEW); `AssociatedDocument`, `AttrEntityCache` and `AttrData.owner_element` visibility | < 1000 |
| `crates/core/elidex-ecs/src/dom/attribute.rs`; `crates/core/elidex-ecs/src/lib.rs` | 331; — | `attr_node`, `set_attr_node`, `attr_owner_element` (NEW), `remove_attribute` evicts (D-21); re-exports narrowed | ≈ 420; < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` | 467 | `adopt` (NEW), the walk (moved in), primitives | ≈ 560; < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | walk out; `begin` / `finish` | < 1000 |
| `crates/core/elidex-ecs/src/dom/shadow.rs`, `tree_clone.rs` | 549, 425 | shadow-root stamp; shallow clone document; reader | < 1000 |
| `crates/core/elidex-ecs/Cargo.toml` | — | `[features] fixtures` | — |
| 26 crates' `Cargo.toml` (`[dev-dependencies]`) | — | `features = ["fixtures"]` (D-17) | mechanical; no `.rs` file touched for it |
| `scripts/check-fixture-boundary.sh` (NEW); `mise.toml` and `.github/workflows/ci.yml` (the CI/tools PR's default-features task and step gain the script's invocation) | — | the boundary check (D-17 (4)); C0b adds no task, step or CLAUDE.md line (PM 255) | new |
| `crates/script/elidex-script-session/src/mutation/child_list.rs` (NEW: D-14 (a), before C0a) | — | D-5 | re-measure |
| `crates/script/elidex-script-session/src/scripting.rs` | 279 | clause (b) + docstring | < 1000 |
| `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` | 994 → ≈ 625 after D-14 (c) (handlers to `table_mutation/handlers.rs`, NEW, a prereq PR) | `create_table_element` (NEW), eight sites (`git grep -n 'create_element(' -- crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` → 8) | split first (PM 258) |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 992 | D-18 test rewrites | if RC-0a has not split it and the edit reaches 1000, a standalone split first |
| `crates/dom/elidex-dom-api/src/` — `element/document_base.rs` 555, `href_accessor.rs` 567, `char_data/document_props.rs` 403, `char_data/split_text.rs` 447, `char_data/attr.rs` 386, `child_node/mod.rs` 459, `element/tree.rs` 633, `element/select.rs` 358, `node_methods/text_content.rs` 181, `node_methods/core.rs` 346, `node_methods/clone.rs` 458, `cssom_sheet.rs` 873, `focus/delegate.rs` 253, `range/mutation.rs` 640, `document.rs` 665 | as listed | D-7, D-11, D-18 | all < 1000 |
| parsers — strict `tree_builder/mod.rs` 493, `insert.rs` 380, `shadow.rs` 218; tolerant `lib.rs` 258, `convert.rs` 678 | as listed | D-10, D-11, D-20 | < 1000 |
| `crates/dom/elidex-custom-elements/src/entity_spawn.rs` 68, `tests.rs`; `crates/script/elidex-js/src/vm/host/custom_elements/html_element.rs` | — | owner decision in the helper; VM marshals `None` → `TypeError` (D-11) | < 1000 |
| `crates/dom/elidex-dom-api/src/char_data/tests/attr.rs` 450, `element/props.rs` 153, `node_methods/tests/contains_compare.rs` | as listed | D-21 handlers; tests over `set_attr_node`; U19 | re-measure `tests/attr.rs` (≥ 1000 → split first) |
| `crates/dom/elidex-form/src/inert_document.rs`, `submit.rs` 546; `crates/css/elidex-style/src/pseudo.rs` 81; `crates/layout/elidex-layout-table/src/grid.rs` 513; `crates/layout/elidex-layout/src/layout/anonymous_table.rs` 170 | as listed | owner argument | < 1000 |
| `crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs` (NEW), `crates/script/elidex-js/src/vm/tests/tests_insertion_adoption.rs` (NEW) | — | cells | new |

### §5.4 Named risks (the observable change each is)

R-1 `ownerDocument` after a cross-document move → the receiving document. R-2 a removed parse node keeps its last
document (T3). R-3 N1's gate follows the adopted document (more restrictive for a page node moved into a DOMParser
subtree; the spec's). R-4 a moved `<base>` updates its new document (T12). R-6 a replaced-in fragment's
`ownerDocument` becomes the parent's (T11). R-7 parse cost (§4.3). R-8 the getter's fallback deleted → a creation site
missed by D-11 would read null; it cannot compile (D-17). R-9 P1 (program §2.0).

## §6. Test plan

Columns: *base* = C0b's base (= `27b50dfe` for these outcomes unless a sibling landed: then re-recorded red-first);
*C0b = spec* (under C2 for template rows). **Red-first**: every red cell is run on the base with its control green, and
the failure recorded in the PR. (×2) = both parser tiers. `dp` = `new DOMParser().parseFromString('<div id=d><span
id=s></span></div>', 'text/html')`; `dp2` a second one.

### §6.1 VM integration (`tests_insertion_adoption.rs`, NEW)

| cell | kind | script | base | C0b = spec |
|---|---|---|---|---|
| T1 | red | `d = dp.getElementById('d'); s = d.firstChild; document.body.append(d); [d.ownerDocument === document, s.ownerDocument === document]` | `[false, false]` | `[true, true]` |
| T2 | red | `x = document.createElement('div'); dp.body.append(x); x.ownerDocument === dp` | `false` | `true` |
| T3 | red (×2) | initial HTML `<p id=p></p>`: `dp.body.append(p); a = p.ownerDocument === dp; p.remove(); [a, p.ownerDocument === dp]` | `[true, false]` | `[true, true]` |
| T4 | red | `h = dp.createElement('div'); sr = h.attachShadow({mode:'open'}); sr.innerHTML = '<b></b>'; dp2.body.append(h); [sr.ownerDocument === dp2, sr.firstChild.ownerDocument === dp2]` | `[false, false]` (P-6) | `[true, true]` |
| T5 | red (C2) | `dp3 = new DOMParser().parseFromString('<template id=t><b></b></template>','text/html'); t = dp3.getElementById('t'); document.body.append(t); [t.content.ownerDocument === document, t.content.firstChild.ownerDocument === document]` | `[false, false]` | `[true, true]` (C1 turns both into the inert document) |
| T8 | red (needs N1) | `d2 = dp4.getElementById('q')` from `'<div id=q onclick="window.hit=1"></div>'`; `x = document.createElement('div'); x.append(d2); typeof d2.onclick` | `'object'` | `'function'` |
| T9 | red (needs N1) | `document.body.append(d2); d2.remove(); typeof d2.onclick` | `'object'` | `'function'` |
| T10 | red (needs N1) | `y = document.createElement('div'); y.setAttribute('onclick','window.hit=1'); h = dp.createElement('div'); h.append(y); typeof y.onclick` | `'function'` | `'object'` |
| T11 | red | `f = dp.createDocumentFragment(); f.append(dp.createElement('i')); c = document.createElement('u'); document.body.append(c); document.body.replaceChild(f, c); f.ownerDocument === document`; variant `document.body.append(f2)` → `f2.ownerDocument === dp` | `[false, true]` | `[true, true]` |
| T12 | red | `x = document.createElement('base'); x.setAttribute('href','https://other.test/'); dp.head.append(x); a = dp.createElement('a'); a.setAttribute('href','y'); dp.body.append(a); a.href` | not `'https://other.test/y'` (P-7; exact value recorded red-first) | `'https://other.test/y'` |
| T13 | guard (R-B, PM 153) | `p = document.createElement('div'); dd = new DOMParser().parseFromString('<body></body>','text/html'); b = dd.body; p.append(b); b.ownerDocument === document`; `b2 = document.createElement('body'); dd.documentElement.append(b2); b2.remove(); b2.ownerDocument === dd` | `false`, `false` | `true`, `true` (T3 memo's XW-9 / XW-10 / EW-15 / EW-16 facts) |
| T19 | withdrawn (rev 3) | the VM has no Attr entity and no `ownerDocument` on Attr (H-17) | — | moved to U1 (Attr variant) and U16 |
| T20 | red | `dp.body.cloneNode(false).ownerDocument === dp` | `false` | `true` |
| T21 | red | `t = dp.createElement('table'); [t.createTHead(), t.insertRow(), t.createCaption(), t.tBodies[0].insertRow(), t.rows[0].insertCell()].map(n => n.ownerDocument === dp)` | all `false` | all `true` (§4.9.1; §4.9.5; §4.9.8) |
| T22 | red | `x = dp.createElement('div'); x.append('t', dp.createElement('i')); x.insertAdjacentText('beforeend','u'); [x.firstChild.ownerDocument === dp, x.lastChild.ownerDocument === dp]` | `[false, false]` | `[true, true]` |
| T23 | red (Range; both landing orders vs L3 RC-2b) | `q = new DOMParser().parseFromString('<p id=p>ab<i>c</i></p>','text/html'); p = q.getElementById('p'); r = document.createRange(); r.setStart(p.firstChild, 1); r.setEnd(p, 2); f = r.extractContents(); [f.ownerDocument === q, f.firstChild.ownerDocument === q, f.lastChild.ownerDocument === q]`; **cloneContents leg** (present iff RC-2b is in C0b's base): `r2 = document.createRange(); r2.selectNodeContents(q.getElementById('p')); g = r2.cloneContents(); [g.ownerDocument === q, g.firstChild.ownerDocument === q]` | extract: `[false, false, false]` (ownerless fragment and Text clone → getter `document_root()`); clone leg: as RC-2b left it | all `true` (*extract* 1, 16.1; *clone the contents* 1). If RC-2b lands after C0b, RC-2b adds the clone leg to this cell (H-L3) |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs`, NEW, unless stated)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `append_child(D_parent, n)` with `n` stamped `X`, a child, a shadow root with content, a template with contents; variants `insert_before`, `replace_child`, an append into a contents fragment stamped `D`; **Attr variant**: `n` with an `AttrEntityCache` entry whose entity was created by `create_attribute(name, owner X)` | every visited node, the contents and the cached Attr entity stamped `D` (adopt 3.1, 3.3.1, 3.4) |
| U2 | red | ownerless `n` (a fixture creator) appended under `D` | stamped `D` (silent re-home) |
| U3 | red | `n` stamped `D` appended under an ownerless parent | keeps `D` |
| U4 | red | a recording `MutationDispatcher` over `append_child` of an `X`-stamped parented node, and over `replace_child`; the recorder's `Insert` arm reads `node_document(node)` | events `Remove(old parent)`, `Insert`; replace: `Remove(new_child's old parent)`, `Remove(old_child)`, `Insert`; the `Insert` arm reads `D` (the stamp is written before `Insert`). (C0c adds `Adopt`'s position) |
| U5 | guard | same-document move | stamps unchanged; no walk (a counting walk hook in the test build) |
| U6 | guard | adoption past the depth and breadth caps (`destroy.rs:381`'s shape) | every node re-homed |
| U7 | red | `node_document(D) == Some(D)`; `owner_document(D) == None`; a node whose document was destroyed → `None`; an unstamped node in a Document's tree → `None` (no fallback) | as stated |
| U8 | guard (×2) | parse a document with templates, a declarative shadow root, foreign content and a doctype; every node except the Document carries `D` **at creation**, before any later append | all stamped |
| U9 | guard (×2) | fragment parse into a context in `D` with a recording dispatcher installed around it and a walk counter | no event reaches the dispatcher; the walk counter stays 0 (every build link is a compare, A2×A5); returned nodes stamped `D`; `FragmentParseRoot` absent afterwards (success and strict rollback) |
| U10 | red | `EcsDom::adopt(fragment, D)` on a parentless fragment with children | fragment and children stamped `D` (C0c adds the event count) |
| U11 | red | `node_document_for_new_nodes`: a Document → itself; a node → its stamp; the fragment root → the context's document; the throwaway → the throwaway; a node created under root during a build → the context's document, **never the throwaway** (both tiers, including a top-level template's contents and a template nested in a top-level element); documentless context → `None` | as stated |
| U12 | guard (dom-api) | a DOMParser document's tree after `CreateAttribute`, `cloneNode(false)`, the table helpers, `append('s')`, `splitText`, `textContent`, Range extract (and cloneContents if present) — every created node stamped. A value check, **not** the census (D-17 is) | all stamped |
| U15 | red (dom-api) | `ownerDocument.get` on an element whose document was destroyed (stamp dangling), with a bound document present | `JsValue::Null` (base: the bound document) |
| U16 | red (dom-api, H-17) | `CreateAttribute` on Document `X` → the Attr entity's `node_document == X`; `GetAttributeNode` on an element of `X` → the cached entity stamped `X`; append the element under `D` → the cached entity stamped `D` | as stated (the dom-api Attr path is not reached from the VM; `#11-dom-api-attr-entity-model-unification`) |
| U17 | control (D-17) | in a scratch clone: (a) add `dom.create_element("x", Default::default())` to a non-test fn of `elidex-dom-api` → the CI/tools default-features `cargo check` step fails (while `cargo clippy --all-features` compiles it: why the step exists); (b) add `features = ["fixtures"]` to `elidex-dom-api`'s normal `elidex-ecs` dependency → `bash scripts/check-fixture-boundary.sh` fails at (iii); (c) add `features = ["test-hooks"]` to `elidex-js`'s normal `elidex-net` dependency (`Cargo.toml:84`) → fails at (iii); (d) add an unclassified `[features] probe = []` to any crate → fails at (i); (e) a `FEATURE_CLASS` row naming a missing key → fails at (ii); (f) revert → green | recorded in the PR |
| U19 | red (dom-api, D-21) | Attr `a` from `CreateAttribute` on Document `Y` (`node_document(a) == Y`); `SetAttributeNode(a)` on element `e` of Document `X` → `node_document(a) == X` (*append* 3) and `GetAttributeNode(e, name)` returns `a` (same entity, *append* 1); `append_child` of `e` under a node of `D` → `node_document(a) == D`; `SetAttributeNode(b)` with the same name → `attr_owner_element(a) == None` (*replace* 5); adopt `e` into `X` → `a` keeps `D`, `b` is `X`; `RemoveAttributeNode(b)`, then adopt `e` into `D` → `b` keeps `X`; `GetAttributeNode` then `RemoveAttribute(name)` → the removed Attr keeps its document across the next adopt. Observation: `EcsDom::node_document(attr)` and `attr_owner_element(attr)` read in the test (the VM never reaches these handlers, H-17) | base: `a` unstamped from creation (`create_attribute` takes no owner, H-4) and after `SetAttributeNode`, never re-homed, and `GetAttributeNode` returns a new entity; C0b: as stated |
| U20 | red (`elidex-custom-elements` unit, D-11) | `spawn_custom_element_entity(dom, "x-e", "x-e", Some(doc))` → `node_document == doc`; with `None` → `None` and no entity spawned (entity count unchanged) | base: `None` spawns an ownerless element |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| no adoption in the primitives | T1, T2, U1 |
| adopt after `link_node` | U4 |
| light-tree-only walk | T4, U1 |
| attrs not re-homed | U1 Attr variant, U16 |
| fallbacks kept | U7; getter fallback kept: U15 |
| a creation site left ownerless | compile (D-17, U17 (a)); T20–T23, U8, U12 |
| entry without the redirection / redirection on the throwaway instead of root | U11, U9 (walk counter) |
| replace-with-fragment does not adopt the fragment / pre-insert adopts it | T11, U10 |
| clause (b) keeps the proxy | T9, T10 |
| a normal dependency enables `fixtures` / `test-hooks`; an unclassified feature; a stale class row | U17 (b)–(e) |
| `SetAttributeNode` does not stamp or register; a removed Attr stays in the walk's set | U19 |
| the CE helper creates an ownerless element for `None` | U20 |

### §6.4 Kept green

`destroy.rs` (fragment teardown), `associated_document.rs` (rewritten), the #380 template suite, #577's fragment
cells, slice a's E1–E8 and U8, N1's cells not listed in §5.2. Commands: `cargo test --all-features` for `-p
elidex-ecs`, `-p elidex-dom-api`, `-p elidex-custom-elements`, `-p elidex-html-parser`, `-p
elidex-html-parser-strict`, `-p elidex-form`, `-p elidex-script-session`, `-p elidex-js`, `-p elidex-style`, `-p
elidex-layout-table`, `-p elidex-layout`; then `mise run ci` (includes the default-features step with the
fixture-boundary script).

## §7. Out of scope

Program §7. In particular the dom-api Attr path is pinned at unit level only (U16) until
`#11-dom-api-attr-entity-model-unification` gives the VM an Attr entity; that slot is not a predecessor, because U1's
Attr variant and U16 pin both of C0b's Attr obligations (D-3 (b) and D-11) on the code path that has them.

## §8. Edges and records

| # | edge | kind | note |
|---|---|---|---|
| c0a | N1 (#580) → C0b | code + semantic | clause (b) on N1's landed body; T8–T10 |
| c0-ab | C0a → C0b | code (soft) | D-5's step-6 comment on C0a's numbering |
| c0-pre-c | D-14 (c) → C0b | code predecessor | `table_mutation/mod.rs` handlers → `table_mutation/handlers.rs` (NEW), a pure move (PM 258) |
| c0-pre-ci | CI/tools default-features check PR → C0b | code predecessor (CI) | the step, its `mise` task and the CLAUDE.md exception are the CI/tools lane's (PM 255); C0b appends `scripts/check-fixture-boundary.sh` to them |
| c0-bc | C0b → C0c | code + semantic | `adopt`, the walk, the stamps |
| c0b | C0b → C1 | semantic + code | the entry, `NewNodeOwner` hook, the walk's contents edge; C1's edge rows are C1's |
| c0c | C0b → T3 | data flow | `node_document` |
| c0d | C0b → L3 P1 | boundary | program §2.0 |
| c0f | C0b ↔ L3 RC-2b | creation entry | if C0b is in RC-2b's base, RC-2b's contents fragment calls `node_document_for_new_nodes(start node)` for extract **and** cloneContents (the argumentless creator does not exist); if RC-2b lands first, C0b's D-11 migrates RC-2b's sites as existing production sites of the population (they do not compile until migrated). T23 pins the observable in both orders |
| c0g | C0b ↔ L3 RC-2a | file overlap `node_methods/clone.rs` | the pass-4 hand stamps in `node_methods/clone.rs` are `:282-285` (cloned host + replicated shadow root) and `:294-301` (the `!deep` shadow-child stamp; verified at `27b50dfe`, `git show 27b50dfe:crates/dom/elidex-dom-api/src/node_methods/clone.rs | sed -n '282,285p;294,301p'`). D14 (in RC-2a) removes `:294-301` in either order. Whichever of C0b and RC-2a lands second removes the stamps that are dead at its head; the other notes this in its DoD. If C0b is in RC-2a's base, RC-2a writes no hand stamp. If RC-2a lands first, only `:282-285` is left for C0b to remove (PM 247 / 249, L3 Rev 6.1 H9) |

**Records carried by C0b's PR.** The umbrella row's "C0b: planned." → "C0b: landed (#<PR>)." with a changelog line
(program R-3). **R-b1, the parent's F10 amendment line** (D-15 (iii), PM 244 Q4), added after the parent's §1 table
(`docs/plans/2026-09-inline-box-predicate.md`, after the F19 row), its cell `:1180` not rewritten:
"*Amendment (F10 cell, `:1180`; <date>, EH-attrs C0b #<PR>; PM decisions 157, 244 Q4):* the node clause's interim
representation, 'the effective-node-document rule (`elidex-script-session/src/scripting.rs:96-111`) promoted
unchanged', is **replaced**, not renamed: the composed-tree-root proxy is deleted, and the node clause reads the stored
node document, `EcsDom::node_document` (C0b), which `AssociatedDocument` holds from the node's creation and DOM *adopt*
re-homes at every insertion; unresolvable → the node clause is false, as before (fail-open unchanged). Per the node
memo's 2026-11-01 amendment (`docs/plans/2026-09-ibp-sandbox.md`)." (Applied in C0b's PR, so it never names a function
that does not exist on `main`.) **For the PM:** program R-1's C0b append; R-6 (a) (append and the `:683` correction).

## §9. Definition of done

1. Coordinates re-measured at C0b's base over §0.1's scope (RC-2's files included); populations P-2 to P-9 re-run and
   recorded in the PR; the census command (program §4.4) run at the base, its result recorded with each member's
   `source`.
2. Pre-implementation run: the fallbacks deleted alone on the base, §6.4's commands, every red test recorded with its
   reason.
3. At C0b's head: the census command re-run and recorded; P-3 = `core.rs:279`, `selection.rs:618`; P-4 first command =
   the definition and `node_proto.rs` only, second = 0; P-5 = 0 and 0; P-9, P-10 as stated; the four argumentless creators
   absent from `EcsDom` (`/usr/bin/grep -n 'pub fn create_element(&mut self, tag' crates/core/elidex-ecs/src/dom/mod.rs`
   → 0).
4. D-1, D-2, D-3 (a)(b), D-4, D-5, D-7, D-8, D-10, D-11, D-17, D-18, D-20, D-21 implemented with §4.8's citations; the strict
   tree builder's method is gone; P-1 unchanged.
5. §6.1 and §6.2 cells present; every red cell red on the base with its control green; U17 recorded.
6. §5.2 rewrites done; §4.9's comment sites rewritten.
7. §6.4 green; `mise run ci` green (incl. the default-features step with the fixture-boundary script) and the CI
   `check` job's default-features step green; parse cost measured (§4.3). The §5.2 list of fixture tests that assert
   an unreachable ownerless-node state is recorded with each one's rewrite or deletion.
8. `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo, the program memo,
   and `docs/plans/2026-09-ibp-sandbox.md` exits 0 (run).
9. The umbrella row's C0b clause updated (program R-3); the parent's F10 line (R-b1) added. Edge c0g (PM 249): D14
   (RC-2a) removes `:294-301` in either order, and whichever of C0b and RC-2a lands second removes the stamps dead at
   its head. If RC-2a is in C0b's base, C0b removes `:282-285`. If not, C0b removes neither and this DoD notes that
   RC-2a, landing second, removes both (D14 `:294-301`, and `:282-285`, dead at its head) and writes no hand stamp.
10. **T3's residual R-B is closed by C0b** (PM 153): cell T13 green at C0b's head; H-T hands T3 the re-derived verdicts
    of XW-9, XW-10, EW-15, EW-16 and pin T-18.

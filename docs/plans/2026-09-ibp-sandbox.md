# IBP-sandbox — the Document's sandboxing flag set, stamped at creation (node plan-memo)

Node `IBP-sandbox` of the inline-box predicate prereq program (`docs/plans/2026-09-inline-box-predicate.md`, approved by
#526 at rev 73, `f1cf5d67`; "the parent"). The parent's decisions bind here (its approval scope, `:3-9`).

**Approval scope (the #526 idiom, parent `:3-9`).** This memo **decides**: the nested-PR set, their order and edges
(§5.1–§5.2) and the parent edges they bind; each fact's home and representation kind (§4's *Home/kind* lines); the
node-level invariants J1–J14 and J17 with their landing qualifiers (§2); the D-items, amendments and user decisions
(§0.4); the spec-semantics decisions (which algorithm, which steps, which outcome — §0.4, §3's *Branch* column); the
per-PR verdicts marked *decision* in §1; and the slots and records (§8). §4's mechanisms, §5.3's populations, §5.4's
named risks, §6's tables, §7's cells, §3's *Touch* column, the verdicts marked *input* in §1 and the whole of
`docs/plans/2026-09-ibp-sandbox-input.md` are **input to each nested PR's plan-review**, which re-derives them. A review
finding against that input changes this memo only when it changes a decision; otherwise it is recorded as a named risk in
the owning nested PR's input lines (§5.4).

**Approval vehicle.** This memo and its input file land through a **docs-only approval PR**, the #526 vehicle. The user's
merge approval of that PR ratifies the approval-scope paragraph, the D-items and every amendment in §0.4. Every
amendment record of a ratified surface (the parent memo, the S5-4 plan) ships **with the approval PR** (§8 records table);
records that describe landed code or memory state ship with the nested PR whose landing makes them true. No umbrella
ledger row (the A-row idiom) is drafted: no amendment here touches a surface the umbrella
(`docs/plans/2026-08-line-box-decorated-inline-content.md`) ratified — the amended surfaces are the parent program memo and
the S5-4 plan, neither of which keeps an A-row ledger; their records live in this memo under the A114 idiom ("recorded,
not edited").

**Revision 9** (2026-10-01) folds the Step 4.5 focused re-check on rev 8 (0 CRIT / 1 IMP / 4 MIN). *IMP*: rev 8's "the shell's
submission entries are connected by construction" was false at `f1cf5d67` — both entries dispatch script-observable events
(`mousedown`/`mouseup`/`click`; `keydown`) before `handle_form_submit`, so a listener can detach the form, and HEAD submits it;
and §4.10.22.3 checks *cannot navigate* three times (steps 1, 5.9 after the `submit` event, 9 after the `formdata` event).
The claim is withdrawn; the three checks are a pre-existing gap (A96) recorded on `#11-form-navigation` under its own
trigger; the fail-closed F10r read is unchanged. Also: F10r's `None` is the unresolvable node-derived document only (an
unbound VM never reaches F10r, J2); the F10r input-domain widening is listed as a parent amendment; the policy-container
grep uses `-P`; the windowproxy row quotes the ledger's pre-split Date as past. Its focused re-check (0 IMP / 3 MIN) then
folded: D8's two detach windows stated exactly (before activation: resolves to a document carrying F9 → submits, else refuses;
during `submit`: submits); the F10r amendment names the non-Window domain; cells C33–C35; and a second F10r forms reader the sweep had
missed — the VM's `form.requestSubmit()` fires `invalid`/`submit`/`formdata` with no forms-flag read at HEAD (A96), closed
by N5 with the read at step 4's position, before validation.

*Rev 8* (2026-10-01) folds the Step 4.5 focused re-check on rev 7 (0 CRIT / 2 IMP / 7 MIN unique). *K1* (IMP): rev 7's
form-submission arm asserted a §4.10.22.3 step 1 (*cannot navigate*) that neither HEAD nor N5 implements and left the
`None` direction unstated; the claim is withdrawn, and an F10r reader whose node-derived document is unresolvable reads
`all_represented()` inside F10r's one read path (fail closed, the parent's F10r "flag absence can never fail open"); the
shell's submission entries are connected by construction today (withdrawn in rev 9: false); step 1 is appended to `#11-form-navigation` (D8, J13,
§3, §4 F10r, §5.1, §5.4 N5, §8, C32). *L1* (IMP): the windowproxy overdue row notes the ledger Date it shared with
`#11-auxiliary-browsing-context-opener` has been split, and decides windowproxy only. Also: `bind_worker`'s guard is one
non-panicking guard (K2); F10's signature types (K3); `ScriptingSubject` collapses `WorkerGlobal`/`VmObject` into
`OtherPlatformObject` (K4); `#11-policy-container-members` → at approval (K5); the worker-blob roadmap edit in the
records row (L2); §11's memory sweep scoped to the resume block (L3) and its `N2` pattern case-sensitive (L4).

*Rev 7* (2026-10-01) folded `/elidex-plan-review` round 6 on rev 6 (0 CRIT / 2 IMP / 20 MIN unique). *Root
finding* (security): rev 6 wrote the unresolvable node document's fail-open at **gate** scope ("the gate fails open"),
which split the derivation (gate) from the composition (F10): on `None` there was neither a target nor a settings
document, so a caller-side branch either refused (J2, breaking C28) or skipped F10 and with it the settings clause — a
detached owner-`None` element with a raw handler in a no-`allow-scripts` realm compiling from N4. The root fix is the
parent's own "one composition": `EcsDom::scripting_disabled_for` owns the derivation and the §8.1.3.4 clause order, and
takes the spec's object forms as subject variants (`ScriptingSubject`, renamed from `PlatformObjectRef` because eval's
settings-object form is not a platform object); an unresolvable node document makes **only the node clause** false, and no
caller has a `None` branch (D8, §4 F10, J2, J13, §5.1, C28b). Also folded: `bind_worker`'s kind guard is N4's addition,
not HEAD's; D-a's unknown-depth rule scoped to `_top`/`_parent`; D5, eval and `window.open` cite their exact object
forms; form submission's §4.6.5 *cannot navigate* precedes its flag read; the registration-timing rule's arms made
exclusive (`#11-csp-violation-reporting`, `#11-nested-iframe-loading` → at approval; `#11-sandbox-flag-set-sources` keeps
the parent's date); `#11-worker-blob-script` and `#11-windowproxy-browsing-context` dispositions; ledger cited by slot
name; records for N2b and `#11-oop-iframe-navigate-completeness`; evidence coordinates; input rows. *Rev 6* named the one
derivation, `EcsDom::effective_node_document` (NEW), the parent's interim, and split rev 5's N2 into N2a/N2b. *Rev 5* withdrew
rev 4's D-b/D-c `node-document` PR, J15, J16 and D-c's marker, which had rewritten the parent's F10 cell (`:1180`) and
widened its gate column (`:1362`) without a user decision.

Review rounds 1–6 and the Step 4.5 re-check on rev 7 ran on session-local snapshots of revs 1–7; they are not a source. The tables (creation entries,
migration populations, per-site Layering, docstring citations, cells) live in the sibling input file
`docs/plans/2026-09-ibp-sandbox-input.md`; named risks stay here in §5.4.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`f1cf5d67`** coordinate (`git diff --stat e2d62b9e f1cf5d67 -- crates` is empty). Spec text is
`.claude/tools/webref body` output (2026-09-30 / 2026-10-01); every `§X.Y = name` pair is `webref heading`/`dfn` output.
`grep` = `/usr/bin/grep`. Memory paths are under `~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/`.

### §0.2 Premises re-measured

| # | Claim | Command | Result |
|---|---|---|---|
| A1 | Gate sites (Appendix A) | `git grep -n 'sandbox_flags\|forms_allowed()\|popups_allowed()\|modals_allowed()\|scripts_allowed()' -- crates` less tests | hold; incomplete: `pipeline.rs:279` install and carriers `pipeline.rs:56`, `:84` |
| A2 | 15-line deletion list | `git grep -n 'fn set_sandbox_flags\|fn sandbox_flags\|fn forms_allowed\|fn popups_allowed\|fn modals_allowed\|fn scripts_allowed' -- crates/script crates/shell` | holds |
| A3–A13 | parse entries; creation paths; F11 callers; G2/G3; wrapper callers; `host_data/mod.rs`; 1000-line watch; parent §7's arm | per rev 3 | hold, except `parse_progressive_str` at `lib.rs:194` and strict `result.rs:150` (a `#[test]`) |
| A14 | F11: the tolerant backend "ignores" scripting | `sed -n 108,117p crates/dom/elidex-html-parser/src/lib.rs`; html5ever 0.39 `src/tree_builder/rules.rs:225`, `:990` | the cause is elidex's `ParseOpts::default()` (`lib.rs:49`, `:110`); open PR #531 bumps html5ever to 0.40.1 |
| A16 | get-the-current-value step 3.2 tests the flag only (S5-4 §2.7-C1) | `webref body html event-handler-attributes` | **false**: "If scripting is disabled for document, then return null" |
| A17 | processing-algorithm step 1 "If scripting is disabled for eventTarget" | as rev 3 (processing algorithm; WebIDL invoke / call-operation; *prepare to run script*; HostEnqueuePromiseJob) | **false**: no invoke-time scripting check exists **for event handlers** |
| A18 | timers | `webref body html timer-initialisation-steps`; `sed -n 330,355p crates/script/elidex-js/src/engine.rs` | §8.7 step 9.5 checks scripting; HEAD relies on transitivity |
| A19 | `document_root()` as the settings document | `sed -n 440,454p crates/core/elidex-ecs/src/dom/mod.rs` | a World-singleton cache; breaks under multi-root fixtures and B1 |
| A20 | `vm::test_helpers` is test-only | `sed -n 82,84p crates/script/elidex-js/src/vm/mod.rs` | false: `pub` under `engine` |
| A21 | production `ScriptEngine::eval` callers | `git grep -n -E '\.eval\(' -- crates/shell crates/script/elidex-script-session` less tests | two, both bound |
| A22 | `PipelineResult.credentialless` | `git grep -n 'pub credentialless' -- crates/shell/elidex-shell/src` | `elidex-shell/src/lib.rs:231` |
| A23 | node creation sets the node document | `sed -n 365,400p crates/core/elidex-ecs/src/dom/mod.rs` | **false**: the `owner = None` shims (`dom/mod.rs:372`, `:548`, `:575`, `:649`) and their parser callers (`convert.rs:258`, `:309`, `:313`; strict `tree_builder/insert.rs:61`, `:141`, `:149`, `:218`) omit it; `owner_document` falls back to a tree-root walk. **Routed to its ratified owner** (§8), not built here |
| A24 | §7.4.2.4 is implemented | `webref body html allowed-to-navigate`; `sed -n 144,172p crates/core/elidex-plugin/src/sandbox.rs` | **partial**: only 3.2/3.3, applied even when source is target |
| A25 | (new) *window open steps* step 5 location | `sed -n 245,290p crates/script/elidex-script-session/src/navigation.rs` | in `window_open_disposition` (`:251-256` doc, `:285` `target.is_empty()`), not in the native |
| A26 | (new) blob URL store | `sed -n 110,140p crates/api/elidex-api-workers/src/validate.rs` | none: `blob:` rejected (`:114-116`, `:133-138`), slot `#11-worker-blob-script` (`m4-12-platform-gap-roadmap.md:1483`) |
| A27 | (new) the source's depth (D-a's input) | `sed -n 279,281p crates/shell/elidex-shell/src/pipeline.rs` | HEAD installs `iframe_depth` per VM pre-eval (`:281`); depth 0 = top-level |

### §0.3 Premise errors (reported, not edited — the A114 idiom)

- **P1** (rev 1's own, withdrawn at rev 2): `Tab` is the browser-side navigable record (`app/tab.rs:31-43`).
- **P2** parent F11 row: A14. **P3** Appendix A: strict `result.rs:150`. **P4** parent F10r gate list: A16. **P6**
  Appendix A A1: A1.
- **P5** parent F18 writer set: the quote is §7.1.5; the second writer is §7.1.3.2 steps 13–14 after a group swap (none in
  elidex).
- **P7** S5-4's processing-algorithm gate: A17. **P8** parent F10 "settings document = `document_root`": A19.
- **P9** `m4-12-landings-ledger.md` has no S5-4a entry.
- **P10** (rev 4's, **withdrawn at rev 5**): rev 4 read A23 as a defect of the node clause to be fixed in this node; the
  parent's F10 cell already decides that clause's interim and final representation, so A23 is a record for that owner
  (§8), not a premise error of the parent.

### §0.4 Decisions, amendments, user decisions

**U1 — F18's home: RESOLVED (a′)** (coordinator 2026-10-01; independent Fable verdict). Parent §0.6 item 15 and I5 stand
unamended. The `Tab` (`app/tab.rs:31-43`) holds `TraversableCreation { popup_sandboxing_flag_set, .. }` (NEW) as the
authoritative store and single writer; the value is computed content-side at §7.3.1.7; `TraversableCreation` is a
**required** parameter of every content-thread spawn function, so a future respawn (design 05 §5.2.5; 09 §9.4 under the
default `SiteIsolation`) supplies it by type. The spec holder is the top-level browsing context; a group swap (P5) is the
future second writer. Shell-held on I5's interim ground.

**D1 — restriction form** (N2a): `SandboxingFlagSet` replaces `IframeSandboxFlags`. It also represents the *sandboxed
navigation browsing context flag*, because D-a's steps 4.2 and 5 read it (the parent's F9 rule: every flag a program source
sets that a program cell reads). Amendments (recorded, not edited) of the parent's F9 cell inner type, the parent's F10r cell text "`Some(IframeSandboxFlags::empty())`" and S5-4 §4.1's rejection
(`2026-07-s5-4-sandbox-enforcement.md:538-542`); support: the parent's rev 42.

**D2 — handler gates follow the current spec** (N1): compile gate = "scripting is disabled for document" (A16); the
invoke-time gate for event handlers is removed (A17). History: S5-4a #444 `24c9b0d4`; the slot's S1a origin and the user's
design-review request (`boa-vm-cutover-s1-plan.md:379`, `:489-497`). Lens attestation as rev 3. Whether S5-4's 2026-07-02
cite was stale or the spec changed is undetermined (webref has no spec history). S5-4 surfaces amended (recorded, not
edited): `:69`, `:129-134`, `:321`, `:461` (§3.5, `:452-466`), `:553`, `:697-699`, `:726-729`, `:1047-1050`, `:1123`,
§2.7-C1, §5.1.2, §6 E1.

**D3** F11's repair locus is elidex's `ParseOpts`; the fragment derivation stays at the parent's Home,
`elidex-script-session`.

**D4** no `Fragment` parser mode (its producer is a stub under `#11-range-full-impl`); recorded as a change to the
parent's §4 F11 obligation and §3 §13.2.4.5 row (input per `:3-9`). N9 re-points `range/mutation.rs:632`.

**D5** the script-fetch gate (§4.12.1.1 step 18, "If scripting is disabled for *el*") is folded into N6 although
pre-existing and A96: ideal over pragmatic. Subject `Node(el)` (D8); for a parser-inserted script the realm resolution is
the document being created — its F9, stamped from the creation input (F9's writer, N5), is the flag set the settings clause
reads; no VM exists at that point.

**D6** `#11-sandbox-flag-set-sources` widened (amendment of the parent's §6 row): CSP3 §6.3.2.1 sandbox Initialization
(workers); the §7.1.3.2 popup writer; trigger + "a worker CSP delivery or a group swap lands". §7.4.2.4 step 4.1 (the one
permitted sandboxed navigator) is **not** this slot's: its reader and writer have one owner, `#11-browsing-context-model-window-open-postmessage`
facet (1), which already holds the fact; this slot's row cross-references it. Steps 4.2 and 5 are implemented (D1, D-a).

**D7 — E26** gates the node's **first** nested landing (parent item 11's literal over the node paragraph's general rule
`:1381-1382`); the approval PR itself is a docs artefact and is not gated by E26.

**D8 / D-i — settings identity and the node document: one composition owns both.** The parent's F10 cell is **one
composition**, `EcsDom::scripting_disabled_for` (NEW); it owns the node-document derivation **and** HTML §8.1.3.4's clause
order, so no caller derives a node document for F10, evaluates a clause, or has a `None` branch. Callers only marshal: they
pass the realm resolution (today the bound realm's associated Document, J12; in the loader, D5, the document being
created) and a **subject** — the spec's object form at the reading step, `ScriptingSubject` (NEW):
- `Node(el)` — "scripting is disabled for *el*" (the script-fetch gate, §4.12.1.1 step 18, D5; later F4/F16/F17 readers):
  settings clause over **el's relevant realm's** associated Document (HTML §8.1.3.3.4; §7.2.2); node clause over
  `effective_node_document(el)`.
- `NodeDocumentOf(el)` — "scripting is disabled for *el's node document*" (the compile gate, §8.1.8.1 *get the current
  value of the event handler* 3.1–3.2, for an element target; the fragment derivation, §13.4 step 10, contextDocument =
  context's node document, step 5): document = `effective_node_document(el)`; settings clause over **that document's**
  relevant realm's associated Document; node clause over that document.
- `Window(w)` (the compile gate for a Window target, whose *document* is the Window's associated Document; the Window
  clause), `OtherPlatformObject` (any other platform object — a worker global included: the settings clause alone,
  whose worker discriminator is read from the settings document, so no worker variant exists), and `Settings` — eval,
  §8.1.4.4 *check if we can run script* step 2, "scripting is disabled for *settings*": the settings clause alone over
  the bound settings document. Five variants: the spec's three object shapes (Node, Window, other platform object) with
  the node shape's two readings, plus the settings-object form.

**Unresolvable node document** (the parent's interim; its fail-open is the **node clause's** only, parent F10/F10r): for
`Node(el)` the node clause is false and the settings clause is evaluated as for any element; for `NodeDocumentOf(el)` the
node clause is false and the settings clause is evaluated over **el's** relevant realm's associated Document (today the
bound document). The settings clause is evaluated first, as at HEAD (`elidex-script-session/src/scripting.rs:89-91`), so
a sandboxed realm refuses whatever the node document (C28b). This is not J2's case: an unbound VM has no realm and the
route refuses before calling F10 (J2); an unresolvable node document has a bound realm.

`Node(el)` and `NodeDocumentOf(el)` are **not** interchangeable under B1: a node's realm is fixed at creation, while
adoption changes only its node document, so after an adopt into another realm's document the two variants read different
settings documents. Today they agree (every object reachable from a VM's `EcsDom` was created in its one realm).

`EcsDom::effective_node_document(node)` (NEW) — landed by N4 in `dom/document_policy.rs` — **is** the parent's interim rule
(the node clause, below) and the one derivation — never `owner_document` alone: the stored owner is not re-homed on
cross-document insert (that is `#11-cross-document-adopt-on-insert`, a record only, §8). It is called inside the
composition and by the one F10r reader family that names a node document (two sites, both N5: `form_input.rs:129`,
`vm/host/html_form_proto.rs:499`):
- **Form submission** (§4.10.22.3 steps 3–4): the *form document* = `effective_node_document(form)`, its flags read
  through F10r.

**F10r over a node-derived document — unresolvable fails closed** (inside the parent's F10r cell: "flag absence can
never fail open"). An F10r reader whose document is derived from a node and does not resolve
(`effective_node_document(..) == None`) reads `all_represented()` — every flag set, so its gate refuses. The mapping lives
in F10r's one read path (§4 F10r), so no reader branches on `None`. This is the only F10r reader family that can see
`None`: today two members, both reading the forms flag over the form document — the shell's submission
(`form_input.rs:129`) and the VM's `form.requestSubmit()` (`vm/host/html_form_proto.rs:499`), which at HEAD fires `invalid`,
`submit` and `formdata` (`:563`, `:571`) with **no** forms-flag read at all: a pre-existing fail-open of §4.10.22.3 step 4 (A96)
that **N5 closes** by reading the forms flag through F10r on the form document at step 4's position — after the submitter
checks and **before** constraint validation (`:546`; step 5.4 fires `invalid`) and the `submit` dispatch (C33). `form.submit()`
(`:469`) fires no event and does not navigate (`#11-form-navigation`), so its step 4 read lands with that slot. (The sweep
covers the per-reader lists above and below; F10's own settings clause never passes `None`, D8 above.) It is not the node clause's fail-open, which the parent confines to F10's node clause.
§4.10.22.3's three *cannot navigate* checks (HTML §4.6.5) — step 1, step 5.9 (re-run after the `submit` event) and
step 9 (re-run after the `formdata` event; `webref body html form-submission-algorithm`) — are **not** implemented here,
at HEAD or by any nested PR: they are a pre-existing gap (A96) appended to the existing slot `#11-form-navigation`
(roadmap `m4-12-platform-gap-roadmap.md`, its row; D-29 #209; §8) under that slot's own trigger. The gap is reachable
today: both shell entries dispatch script-observable events before the activation behaviour (`content/event_handlers.rs`
at `f1cf5d67`: `dispatch_event` of `mousedown`/`mouseup`/`click`, then `:155 handle_form_submit`; `keydown`, then
`handle_key_text` → `:580 handle_form_submit`), and `handle_form_submit` dispatches `submit` (`form_input.rs:137-139`)
without re-checking. Two windows follow, and this node changes only which document the forms flag is read from, so every
outcome is HEAD's or stricter: (i) a detach **before** activation (a `mousedown`/`mouseup`/`click`/`keydown` listener) —
HEAD submits; from N5 the forms flag is read from `effective_node_document(form)`, so a form that still resolves to the
live document submits as at HEAD, while one that no longer resolves (an owner-`None` parser-created form) or resolves to
an F9-less throwaway document (a DOMParser form, never re-homed, `#11-cross-document-adopt-on-insert`) refuses (C34);
(ii) a detach **during** the `submit` event (the step 5.9 window) — N5's read precedes the dispatch, so the form submits
at HEAD and after N5 alike (C35, the A96 record).

The other F10r readers hold a Document directly, derive no node document and never see `None`:
- **Simple dialogs** (§8.9.1 step 1): the Window's associated Document (the relevant realm's).
- **`window.open`**: the flags are §7.3.1.7 *the rules for choosing a navigable* step 3's — currentNavigable's active
  document's active sandboxing flag set, where currentNavigable is sourceDocument's node navigable (§7.2.2.1 *window open
  steps* step 13) and sourceDocument the entry global object's associated Document (step 2).

At **N1** the compile gate reaches the same rule through HEAD's composition
(`elidex_script_session::scripting_disabled_for_platform_object`, given the element: settings clause first, then the
effective node document; HEAD's single realm makes it identical); N4 moves the rule into the composition and passes
`NodeDocumentOf(element)`, so no window opens at N1. No reader keys on `document_root()` or calls `owner_document` for
this purpose. Amendment (recorded, not edited) of the parent's F10 cell's "`document_root`" wording.

**Node clause — the parent's ratified interim (not a new decision).** Parent F10 cell (`:1180`): "interim representation
the effective-node-document rule … promoted unchanged (it fails **open** when the node's document cannot be resolved …);
final representation the enriched `#11-domparser-full-document-parse-fidelity` marker". Promoted unchanged into `EcsDom`
as `effective_node_document` (D8), HEAD's order (`elidex-script-session/src/scripting.rs:83-114`): a Document target →
itself; a node → its composed tree root if that is a Document; otherwise `owner_document`; the result compared against the
subject's settings document (D8); unresolvable → the **node clause** is false, after the settings clause has been
evaluated (D8; never a gate-scope skip). *Window clause* (the parent's cell): the same fact, read for the Window's
associated Document. `OtherPlatformObject` and `Settings` → settings clause only. F9 is a creation fact and **nothing reads F9's
presence as the node clause's fact** (a browsing context being null).

**D-a — §7.4.2.4 *allowed by sandboxing to navigate*** (A24), owner **N2b**. Spec steps 1–6: 1 source is target → true;
2 source is an ancestor of target → true; 3 target is an ancestor of source → 3.1 target not top-level → true, 3.2/3.3 the
two flags × transient activation, 3.4 true; 4 target is a top-level traversable → 4.1 one permitted sandboxed navigator,
4.2 the sandboxed navigation flag, 4.3 true; 5 the sandboxed navigation flag → false; 6 true. Implemented: every step but
4.1 — 4.2 and 5 read the sandboxed navigation flag, which N2a represents (D1). Step 4.1 is owned end to end by
`#11-browsing-context-model-window-open-postmessage` facet (1) (D6); until it lands the source is never the permitted
navigator, so 4.2 decides (restrictive; §5.1). The relation type distinguishes 4 from 5: `NavigableRelation =
SourceIsTarget | SourceIsAncestor | TargetIsAncestor { target_is_top_level } | Other { target_is_top_level }` (NEW). It is
derived **once**, by an engine-independent function next to `window_open_disposition` in
`elidex-script-session::navigation`; callers marshal its inputs and do not derive it. Its source-navigable input is the
**source's depth** (`_top` needs depth > 0; `_parent`'s target is top-level iff the depth is 1, and step 3.1 returns true
for a deeper source), with an interim home: HEAD's per-VM `iframe_depth` (installed pre-eval, `pipeline.rs:281`; depth 0 =
top-level), part of the B1 cluster (`#11-browsing-context-state-ecs-components`). The depth matters **only** for the
`_top`/`_parent` keyword family: there an **unknown** depth (no `HostData`; `iframe_depth()` answers 0 at
`engine.rs:685-691`) derives the **restrictive** relation, `TargetIsAncestor { target_is_top_level: true }` (3.2/3.3
apply), never `SourceIsTarget`, and the decision is made inside the derivation function, not at a marshalling site (the
unknown arm needs a representation — input, §5.4 N2b). `_self`, `""` and a named child hit (`SourceIsTarget`,
`SourceIsAncestor`) do not depend on depth, and the rule is never applied to a non-ancestor target, where
`TargetIsAncestor` would loosen `Other`. N2b reads N2a's set (edge **n6a**) and precedes the
first sandboxed top-level sources (N6 CSP, N8 popup) through edge **n6b**. Outcome: a sandboxed top-level document's
`_top`/`_parent` is allowed regardless of activation.

**D-d — no post-hoc writer.** Documents are created only through the one production writer (fixtures included, with an
explicit `DocumentCreationInput`); `bind_vm` does not stamp; no `test-hooks` feature.

**D-e — *window open steps* step 5** stays in `elidex-script-session::navigation`, where HEAD has it (A25): a `window.open`
entry maps `""` → `_blank` and then calls the shared disposition with a resolved target; the native only marshals. Links
pass *get an element's target* (§4.2.3) unchanged, `""` = `_self` (*the rules for choosing a navigable* step 4). Behaviour
change: a link's named-target miss becomes a new-tab promotion per step 8, gated by the auxiliary-navigation flag.

**D-h — policy containers** (N6). *Determine navigation params policy container* (§7.1.7) whole: step 1 (history-stored
container) as a new `HistoryEntry` field (`elidex-navigation/src/navigation.rs:18`) written when a navigation to a URL that
requires storing the container commits; 2 `about:srcdoc`; 3 local URL + initiator; 4 response container; 5 new. *Create a
policy container from a fetch response*: steps 2–7; **step 1 (blob URL) is A96** — elidex has no blob URL store (A26) —
routed to `#11-worker-blob-script` with the added trigger "a blob URL store lands".

**`WorkerGlobalRoot` — the parent's ratified writer.** `Vm::bind_worker` (`elidex-js/src/vm/vm_api.rs:382`) stamps the
marker on the document root it binds, as the parent's F10 cell (`:1180`) ratifies: the one seam every worker and
service-worker VM binds through (`vm/worker_thread.rs`, `vm/sw_thread.rs`, and the test helper). HEAD's `bind_worker`
(`vm_api.rs:382-411`) has **no** global-kind check; **N4 adds the guard** inside the parent's writer — one guard,
non-panicking: `bind_worker` stamps only when the VM's `GlobalScopeKind` is a worker kind (`Vm::new_worker`,
`Vm::new_service_worker`); on a Window-kind VM it binds without stamping. A marker on a Window realm's document would turn
off the sandbox condition (fail open); a missing marker reads as Window, so the sandbox condition applies (the parent's
premise, fail-safe) — the guard keeps J5's one discriminator on the fail-safe side. Worker roots stay `create_document_root()`. D-d concerns F9, not this global-kind stamp. (Rev 5
moved the writer to a new worker-root creation function without a defect in the parent's choice; withdrawn.)

## §1. Node shape and verdicts

| Axis | Own invariant | Intersects |
|---|---|---|
| Security gates (VM, shell) | fail closed on absence; Window/worker; snapshot at call; §7.4.2.4 relation | store, popup IPC |
| Document creation | stamp at creation (I7); one creation input | parser, CSP, origin, popup |
| Parser | mode == F10's settings clause; §13.4 step 10 | creation, fragment callers |
| VM / shell | one store; one route on the relevant realm's document | gates |
| Navigable state + popup IPC | `Tab` writer; required spawn input | creation |
| CSP plumbing | one parser; CSP-derived flags; F19 clone rules | creation, origin, framing |

**Node: edge-dense — divided** (*decision*; CLAUDE.md *Edge-dense work* (a)). Per nested PR — each plan-review applies the
rule to its own PR; the parent's `:1377-1380` exception covers only IBP-layout and is not an exit here:
- *Decisions* (bind later plan-reviews): **N1** narrow — the two handler gates over HEAD's store and composition, on two
  axes: the invoke-time gate's removal, and the compile gate's node-document derivation (the parent's interim through HEAD's
  composition given the element, D8 — never the stored owner); **N2a** narrow — one type re-keyed in place, with the
  sandboxed navigation flag added (D1); **N2b** narrow — the §7.4.2.4 function and its one derivation site (D-a), split
  from N2a so the polarity flip (population (e), input §B) and the new algorithm with three caller families are reviewed
  apart; **N3** narrow — one parser and one check; **N7** narrow —
  one branch of one pure function and its one caller; **N9** narrow — one mode value into two backends and one fragment
  entry.
- *Input* (each PR's own plan-review decides, recursing if edge-dense; a user exception only if a single indivisible
  invariant is itself edge-dense — possible, not pre-decided): **N4** `f10-route` bundles J2 (unbound), J5, J12, J13 and
  the interim sources — may be edge-dense; **N5** `stamp` (creation × store × gates × IPC `iframe/thread.rs:227` × parser
  entries) — may be edge-dense; **N6** `csp-sandbox` (the origin derivation cannot leave it; frame-ancestors-over-F19 and
  D5 can); **N8** `popup` (IPC × spawn API × disposition × creation) — may be edge-dense.

## §2. Coupled invariants (decisions, with landing qualifiers)

- **J1 One store** — from **N5**, F9 is the only representation; from N2a to N5 the store is HEAD's `HostData` /
  `PreEvalFrameState` (J14).
- **J2 Fail closed on absence** — unbound VM (no realm, so no settings document) → the route refuses before F10, from
  **N4**; no F9 on the settings document → `all_represented()`, from **N5**. An **unresolvable node document** is not this
  case: the realm is bound, F10 evaluates the settings clause and only the node clause is false (D8).
- **J3 Stamp at creation (I7)** — from **N5**: F9 (and F19 from N6) written by the one writer as the root is spawned.
- **J4 One creation input** — origin from the final set from **N6**; final set complete (CSP N6, embedder N7, popup N8)
  from **N8**; parser mode from the same input from **N9**.
- **J5 Global-kind discrimination** — from **N4**: `WorkerGlobalRoot`, stamped by `Vm::bind_worker` (the parent's seam) on
  a worker-kind VM only (N4's guard), is the only non-Window test. No production worker-realm reader exists after N1; C7's direct oracle pins it.
- **J6 Snapshot at call** — HEAD for the aux-nav verdict; the popup set from **N8**.
- **J7 Union in restriction form** — from **N2a**.
- **J8 One CSP parser** — from **N3** (frame-ancestors); F19 and the CSP-derived flags from **N6**.
- **J9 Fragment mode follows contextDocument** — from **N9**.
- **J10 F18: one store, one writer (`Tab`)** — from **N8**.
- **J11 F19 clone rules** — from **N6** (D-h).
- **J12 One VM read route** — from **N4**: bound → (the relevant realm's document, `HostData::dom()`); unbound → refuse.
  At N4 the route's flag source is `HostData` (mapped per J14); from N5, F9 via F10r.
- **J13 Settings identity and node document** — from **N4**: callers pass the realm resolution and the spec's subject form
  (D8); the one composition derives every node document F10 reads through `EcsDom::effective_node_document`, derives the
  settings document per subject, and evaluates the settings clause first; no caller derives or branches on `None`. The
  one F10r reader family naming a node document (form submission: `form_input.rs:129` and
  `html_form_proto.rs:499`, both from N5) derives it with the same function; unresolvable → F10r
  reads `all_represented()` (fail closed, D8).
  From N1 to N4 the compile gate applies the same rule through HEAD's composition (identical in HEAD's single realm).
- **J14 Interims to N5** — N2a to N5: interim stores hold `empty()` for HEAD's "not a sandboxed iframe"; N4 to N5: F10's
  flags argument, which the route fills from `HostData` (§4 F10). N5 deletes both.
- **J17 §7.4.2.4 by relation** — from **N2b**: the relation is derived once and read first; for `_top`/`_parent` an
  unknown source depth derives the restrictive relation (D-a). (J15 and J16 were withdrawn at
  rev 5; numbers not reused.)

Pairs: J1×J2 (from N5, no fallback to "unsandboxed"); J2×J5 (worker documents skip the sandbox condition); J2×J12
(unbound natives refuse); J3×J4 (mode from the input, not read back); J4×J6 (the popup set enters the same
`determine_origin`); J4×J8 (CSP-derived flags join before the parse); J4×J11 (a clone's CSP-derived flags are not
recomputed); J9×J13 (the fragment derivation passes `NodeDocumentOf(context)`: contextDocument is derived inside F10, the
settings document is contextDocument's relevant realm's associated Document; a DOMParser contextDocument has no browsing
context, so the node clause disables; an unresolvable contextDocument leaves the mode to the settings clause over the
context's realm); J13×B1 (B1 changes only the realm → document resolution, where `Node` and `NodeDocumentOf` diverge); J17×J6
(the relation is part of the call-time snapshot); node clause × J13 (the parent's interim compares against the subject's
settings document; unresolvable → the node clause alone is false).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.1.5 Sandboxing | *parse a sandboxing directive* | nine represented flags (incl. the sandboxed navigation flag, D1); implied grants at parse; unrepresented → `#11-sandbox-flag-set-sources` | N2a | ✗ | yes |
| WHATWG HTML §7.1.5 Sandboxing | *active sandboxing flag set* | F9 written once at creation | N5 | ✓ | no |
| WHATWG HTML §7.1.5 Sandboxing | *determine the creation sandboxing flags* | embedder; null (popup) | N7, N8 | ✓ | yes |
| WHATWG HTML §7.1.5 Sandboxing | *CSP-derived sandboxing flags* | last enforced `sandbox` | N6 | ✓ | yes |
| WHATWG HTML §7.1.5 Sandboxing | *popup sandboxing flag set* | writer 1; writer 2 (§7.1.3.2 13–14) → slot | N8 | ✗ | yes |
| WHATWG HTML §7.3.1.7 Navigable target names | *the rules for choosing a navigable* steps 3, 4, 8 (substeps 7, 8.1, 9) | substep 8.2 → `#11-browsing-context-model-window-open-postmessage` facet (1) | N8 | ✗ | yes |
| WHATWG HTML §7.2.2.1 Opening and closing windows | *window open steps* step 5 | `""` → `_blank` in the session's `window.open` entry (D-e) | N8 | ✓ | yes |
| WHATWG HTML §4.2.3 The base element | *get an element's target* | links pass it unchanged; `""` = `_self` | N8 | ✓ | yes |
| WHATWG HTML §4.6.5 Following hyperlinks | *get an element's noopener* | `_blank` without `rel=opener` | N8 | ✓ | yes |
| WHATWG HTML §7.4.2.4 Preventing navigation | *allowed by sandboxing to navigate* steps 1–6 | 4.1 → `#11-browsing-context-model-window-open-postmessage` facet (1), restrictive until then (D-a) | N2b | ✗ | yes |
| WHATWG HTML §7.3.2.1 Creating browsing contexts | *create a new browsing context and document* steps 6, 15, 19.2; *determine the origin* 1–5 | popup initial `about:blank` → slot | N5, N6, N7 | ✗ | no |
| WHATWG HTML §7.4.5 Populating a session history entry | *create navigation params by fetching* 21.9–21.11; *… from a srcdoc resource* 3, 6 | fetched; `srcdoc` | N6 | ✓ | yes |
| WHATWG HTML §7.5.1 Shared document creation infrastructure | *create and initialize a Document object* | F9, F19 at spawn | N5, N6 | ✓ | no |
| WHATWG HTML §7.5.7 Loading a document for inline content that doesn't have a DOM | error page | load-error / framing-blocked fallbacks | N6 | ✓ | no |
| WHATWG HTML §7.1.7 Policy containers | *create a policy container from a fetch response* 1–7; *determine navigation params policy container* 1–5 | step 1 of the former (blob) → `#11-worker-blob-script`; non-CSP members → slot | N6 | ✗ | yes |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | settings / node / Window clauses; the object forms (settings, *el*, *el's node document*, Window) | one composition owns derivation and clause order; node clause = the parent's interim (unresolvable → node clause false), one `effective_node_document` (D8) | N4, N5 | ✗ | yes |
| WHATWG HTML §8.1.4.4 Calling scripts | *check if we can run script* step 2 | eval gate (`Settings`, D8) | N4 | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | getting the current value 3.1–3.2; processing algorithm 1–3 | no invoke-time scripting check; step 3 → slot (§8) | N1 | ✗ | yes |
| WHATWG HTML §8.7 Timers | *timer initialization steps* step 9.5 | satisfied transitively; breaking conditions → slot (§8) | none | ✗ | no |
| WHATWG HTML §8.9.1 Simple dialogs | *cannot show simple dialogs* step 1 | modals flag | N4, N5 | ✓ | yes |
| WHATWG HTML §4.10.22.3 Form submission algorithm | steps 3–4; steps 1, 5.9, 9 (*cannot navigate*, §4.6.5) | forms flag over the form document, unresolvable → `all_represented()` (fail closed, D8); steps 1, 5.9, 9 → `#11-form-navigation` (§8, A96) | N4, N5 | ✗ | yes |
| WHATWG HTML §4.12.1.1 Processing model | *prepare the script element* step 18 | script fetch gate (`Node(el)`, D5) | N6 | ✓ | yes |
| WHATWG HTML §13.2.4.5 Other parsing state flags | scripting mode | Fragment omitted (D4) | N9 | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | step 10 | `template.content` omitted | N9 | ✗ | yes |
| CSP3 §2.2.1 Parse a serialized CSP | all steps | | N3 | ✓ | yes |
| CSP3 §2.2.2 Parse response’s Content Security Policies | steps 1–4 | empty policies not appended | N3 | ✓ | yes |
| CSP3 §6.3.2 sandbox | header delivery | §6.3.2.1 sandbox Initialization (workers) → slot | N6 | ✗ | yes |
| CSP3 §6.4.2.1 frame-ancestors Navigation Response Check | steps 1–7 | `elidex-plugin` function; reporting → slot | N3, N6 | ✗ | yes |
| WHATWG HTML §4.2.5.3 Pragma directives | CSP `<meta>` | `#11-document-base-url`; a `sandbox` directive there is ignored (CSP3 §6.3.2: "ignored entirely when delivered … within a meta element") | none | ✗ | yes |

**Breadth**: K=2 (HTML, CSP3), M=29 → split recommended; the split is §5. CSP3 rows verified with `webref heading CSP3 …`
(preflight has no `CSP3` label).

### §3.1 User-input touch audit

The `sandbox` attribute; CSP headers (new parse surface, N3); `window.open` target/URL; link `target`/`rel`; `<noscript>`
(N9). New sources only add restrictions; absence fails closed; D-a widens one outcome (a sandboxed top-level document may
navigate itself via `_top` — source is target, no escape); D-e adds new-tab promotion for link named misses, gated by the
auxiliary-navigation flag. The node clause's unresolvable arm is HEAD's (the parent's interim): it makes only the node
clause false; the settings clause is always evaluated (D8).

## §4. Facts → mechanisms (Home/kind lines decided; the rest input)

- **Flag set** (N2a) — *Home/kind*: `SandboxingFlagSet`, `elidex-plugin::sandbox`, restriction form, incl. the sandboxed
  navigation flag (D1). Input: represented flags; `parse_sandboxing_directive`; predicates; the N2a interim (J14); N2a's
  light re-type of `vm/host_data/mod.rs` (net LoC ≈ 0, deleted at N5, option A).
- **§7.4.2.4** (N2b) — no new *Home/kind* line beyond `NavigableRelation` (below) and D-a. Input: where the predicate over
  (`SandboxingFlagSet`, `NavigableRelation`, activation) lives, given the crate direction (`elidex-script-session` depends
  on `elidex-plugin`).
- **F9** (N5) — *Home/kind*: `ActiveSandboxingFlagSet(SandboxingFlagSet)`, a component in `elidex-ecs/src/components.rs`, on
  the Window document root, written once by `EcsDom::create_window_document_root(input)` (NEW) — the only writer —
  in `elidex-ecs/src/dom/document_policy.rs` (NEW). A creation fact; never read as the node clause's fact. `DocumentCreationInput`
  (NEW, `elidex-ecs`, `Clone`) is a required argument of every document-parse entry. Input: input file §A, §B.
- **F10** (N4 composition; N5 re-source) — *Home/kind* (**N5's form**): `EcsDom::scripting_disabled_for(realm_document:
  Entity, subject: ScriptingSubject)` (NEW) in `dom/document_policy.rs` — the parent's **one composition**, owning the
  node-document derivation and the clause order (D8). `realm_document` is non-`Option`: an unbound VM is refused by the
  route before the call (J2). It is the **realm → document resolution** — the one contract B1 replaces (today the bound
  realm's associated Document; under B1 the relevant realm's, §8 B1 append (5)). `ScriptingSubject = Node(Entity) |
  NodeDocumentOf(Entity) | Window(Entity) | OtherPlatformObject | Settings` (NEW), payloads non-`Option`, one variant per
  spec object form (D8). In F10's interface the only `Option` is `effective_node_document`'s return, consumed inside the composition (F10r's
  `Option` is its own, below). Order: settings clause first — the subject's settings document derived (for `NodeDocumentOf` from
  `effective_node_document`, falling back to the element's realm on `None`), then `WorkerGlobalRoot` on it → the sandbox
  condition does not apply, else `!scripting_enabled(flags)`; then the node or Window clause: the parent's interim (§0.4),
  `None` → false. **At N4** the function takes one more argument, the flags, which the route fills from `HostData` (J14
  mapping) — an N4–N5 interim listed in J14 and deleted by N5 (`elidex-ecs` cannot see `HostData`, and reading F10r there
  before N5 would answer `all_represented()` everywhere); the host never evaluates a clause itself. **From N5** the flags
  come from F9 via F10r; the node clause stays the interim until `#11-domparser-full-document-parse-fidelity`'s marker
  lands. Call direction: the host route `crates/script/elidex-js/src/vm/host/settings_route.rs` (NEW) of J12 marshals and
  calls the session boundary `elidex_script_session::scripting_disabled_for_platform_object` (Script↔ECS), which forwards
  to `EcsDom::scripting_disabled_for`. Readers: eval (`Settings`), compile (`NodeDocumentOf` / `Window`), D5 script-fetch
  (`Node`, N6), fragment derivation (`NodeDocumentOf`, N9); later F4/F16/F17 (`Node`). Invoke-time gate deleted (N1).
  *Timers*: no reader wired (§8 slot).
- **Node document** (N4; N1 through HEAD's composition) — *Home/kind*: `EcsDom::effective_node_document(node)` (NEW) in
  `dom/document_policy.rs`, the parent's interim rule (§0.4, D8); the one derivation, called inside F10's composition and by
  form submission's F10r read (D8).
- **F10r** (N4 over `HostData`; N5 over F9) — *Home/kind*: `EcsDom::active_sandboxing_flags(document)` (NEW) → F9 or
  `all_represented()`; it takes the reader's document as resolved (`Option<Entity>`; readers holding a Document pass
  `Some`), so an unresolvable node-derived document (`None`) also reads
  `all_represented()` inside this one read path (D8, fail closed). `None` means only that: an unbound VM is refused by the
  route before any read (J2) and never reaches F10r, so the two absences do not share a value; at N4 the route passes the `HostData` value instead.
- **`WorkerGlobalRoot`** (N4) — *Home/kind*: marker on the worker realm's document root, written by `Vm::bind_worker` (the
  parent's F10 cell), only on a worker-kind VM (the guard N4 adds, §0.4).
- **F11** (N9) — *Home/kind*: `ParserScriptingMode` (`Normal | Disabled | Inert`) in `elidex-html-parser-strict::result`;
  document mode from the creation input; fragment derivation in `elidex-script-session/src/mutation/html_fragment.rs`.
- **F18** (N8) — *Home/kind*: `TraversableCreation` on the `Tab`; the value from `sandbox::popup_sandboxing_flag_set`
  (NEW), computed inside the shared disposition (D-e).
- **`determine_creation_sandboxing_flags`** (N7/N8) — *Home/kind*: `elidex-plugin::sandbox`, a pure function over
  `Embedder { iframe_set, embedder_document_set } | Popup(set)`.
- **`NavigableRelation`** (N2b) — *Home/kind*: `elidex-script-session::navigation`; one derivation function over the
  source's depth (unknown → restrictive, for `_top`/`_parent` only; D-a).
- **F19** (N3 parse, N6 stamp) — *Home/kind*: `DocumentCspList(CspList)` in `components.rs`; `CspList` and
  `csp::frame_ancestors_check` (NEW) in `elidex-plugin::csp`; writers per J11 and D-h.
- **Origin** (N6) — *Home/kind*: `determine_origin` in `elidex-plugin::origin`; one install path in
  `run_scripts_and_finalize`. N6 also deletes the `credentialless` TODO (`pipeline.rs:282-286`) as that block collapses.

## §5. Nested PRs

### §5.1 List (decided)

| Nested PR | Scope |
|---|---|
| **N1 `IBP-sandbox-handler-gates`** | D2; the compile gate's node document through the parent's interim (D8) |
| **N2a `IBP-sandbox-flagset`** | D1 (incl. the sandboxed navigation flag); the J14 interim stores |
| **N2b `IBP-sandbox-nav-relation`** | D-a: `NavigableRelation`, its one derivation, the predicate (§7.4.2.4) and its callers |
| **N3 `IBP-sandbox-csp-list`** | `elidex-plugin::csp` parse + `frame_ancestors_check`; `check_framing_allowed` marshals |
| **N4 `IBP-sandbox-f10-route`** | F10 composition in `EcsDom`, owning derivation and clause order over `ScriptingSubject` (node clause = the parent's interim, `effective_node_document`); `WorkerGlobalRoot` stamped by `bind_worker`, with the worker-kind guard it adds; the VM route (unbound → refuse); every gate routed over the `HostData` store |
| **N5 `IBP-sandbox-stamp`** | the store flip: F9, F10r over F9, every creation entry threaded, `HostData`'s copy deleted |
| **N6 `IBP-sandbox-csp-sandbox`** | CSP-derived flags; F19 (D-h); `determine_origin`; frame-ancestors over F19; D5 |
| **N7 `IBP-sandbox-embedder-union`** | `determine_creation_sandboxing_flags`' embedder branch |
| **N8 `IBP-sandbox-popup`** | F18 (U1); required spawn parameter; D-e |
| **N9 `IBP-sandbox-parser-mode`** | F11; `range/mutation.rs:632` re-pointed |

**Accepted interim windows** — the population is **every window a decided item opens**: J1–J14 and J17, U1, the D-items,
`bind_worker`'s kind guard, the node clause and the amendments (§0.4). Each window equals HEAD or is more restrictive than it; none is a restriction escape.
Listed (open):
- J1/J2/J3 (store, absence, stamp): N2a–N5 the store is `HostData` with `empty()` for "not sandboxed" (J14); before N4 an
  unbound gate reads HostData as at HEAD.
- J4 (one complete creation input): a CSP-sandboxed document is unsandboxed until N6; a nested frame lacks the embedder
  union until N7; a popup lacks the popup set until N8; the origin ignores CSP until N6.
- J4/J9 with C20 (parser mode == F10's settings clause): false **from HEAD until N9** — HEAD already parses sandboxed
  documents' `<noscript>` in Normal mode; N5 only makes the invariant expressible (the creation input exists).
- J6/J10 (popup set): absent until N8.
- J8/J11 (F19, clones): absent until N6; frame-ancestors reads headers through N3's parser until then.
- J14's second interim (F10's flags argument): N4 to N5; it carries HEAD's `HostData` value.
- D5 (script-fetch gate): absent until N6 — external scripts of sandboxed documents are fetched, not run.
- Forms flag in `form.requestSubmit()`: absent until N5 — HEAD fires `invalid`/`submit`/`formdata` in a document without
  `allow-forms` (A96); equals HEAD until N5 closes it (D8, C33).
- D-a step 4.1: from N2b until `#11-browsing-context-model-window-open-postmessage` facet (1) lands, a sandboxed source is
  never the one permitted sandboxed navigator — **more restrictive** than the spec and HEAD; unreachable in production
  today (named hits resolve only child navigables, `link_nav.rs:76-85`; `window.open` drains only on the top level).
- Node clause: the parent's interim until the `#11-domparser-full-document-parse-fidelity` marker lands — the parent's
  accepted window (`:1180`), restated, not opened here. Its extent is the **node clause only**: an unresolvable node
  document makes that clause false, while the settings clause is evaluated for every subject (D8), and an F10r reader
  whose node-derived document is unresolvable reads `all_represented()` (fail closed, D8). **No gate-scope window exists**: no reader skips F10
  or the settings clause on `None`.

Closed (no window): J5, J12, J13 (land with their first reader, N4; `WorkerGlobalRoot` is not load-bearing before N5);
J7 (N2a), J17 (N2b) with their readers; **D2 × D8** — the compile gate's node document — closed at N1 itself (HEAD's
composition given the element, D8), so the reverse case (a `createElement` node with a raw handler moved into a DOMParser
document) stays suppressed and the mirror case (a DOMParser node appended into the live document) runs at every landing;
and a detached owner-`None` element in a sandboxed realm stays suppressed at every landing (settings clause first, C28b);
D1 (re-keyed in place); U1, D3, D4, D6, D7, D-d (no runtime change); D8's subject variants and per-reader settings
documents (equal today; they diverge only under B1) and `bind_worker`'s kind guard (only worker VMs reach it today); D-e (spec
behaviour change, gated by the auxiliary-navigation flag); D-h (its blob arm is HEAD's absence, A96); the parent and S5-4
amendments (records of the decisions above). D2's handler compiled while live and then moved into a DOMParser document
runs from N1 (C8b) — the spec's outcome (A17), not a window.

### §5.2 Edges (decided; the parent's one rule)

| # | Edge | Kind |
|---|---|---|
| n1 | IBP-split-ecs → N4 | code predecessor (E1: N4 is the first to edit `dom/mod.rs`, adding `dom/document_policy.rs`) |
| n2 | IBP-split-parser → N5 | code predecessor (E2: N5 is the first to edit `elidex-html-parser/src/lib.rs` — its parse entries take the input; measured: no N1–N4 scope touches that file) |
| n3 | N1 → N4; N2a → N4 | data flow (N4 routes N1's gate set and promotes its node-document rule; F10 takes `SandboxingFlagSet`) |
| n4 | N4 → N5 | data flow (N5 re-sources N4's route and F10) |
| n5 | N3 → N6 | data flow (`CspList`) |
| n6a | N2a → N2b, N6, N7, N8 | data flow (the restriction-form set D-a reads; union-safe set; propagate flag) |
| n6b | N2b → N6, N8 | data flow (the §7.4.2.4 relation precedes the first sandboxed top-level sources) |
| n7 | N5 → N6, N7, N8, N9 | data flow (creation path, F9, F10r) |
| n8 | N6 → N7 → N8 | data flow (`determine_origin`, the F19 clone; the creation-flags function) |

Acyclic. **Parent edges**: E1 → N4; E2 → N5; **E3/E17** follow **N8**; **E29** from **N6**; **E26** before the node's first
nested landing (D7).

**Own deferrals per nested PR** (cap 3): N9 — 1, the `content-visibility` pin, conditional on the cell firing at its base
(N6 or N8 instead, if one lands after N9 and fires the witness family first; no double pin, so the node total stays the
parent's **1**). Every other nested PR — 0: the slots it registers or appends to (§8) are pre-existing class (A96).

### §5.3 Migration populations (input)

Input file §B defines populations (a)–(h) by property with counts at `f1cf5d67`; owners: (a) N1; (b) N5; (c) N5; (d) N5;
(e) N2a; (f) N8; (g) N5; (h) N4. Population (i) (node creation without an owner) was withdrawn with D-b.

### §5.4 Named risks per nested PR (input lines; reviewer evidence kept)

- **N1**: `tests_event_handler_attrs.rs:567`, `:691`, `:719` flip (→ C8b); `:597` re-grounded; `tests_call_listener.rs:188`
  survives. The reverse and mirror cases (C30, C31) are built in production form, through the gate, not on `EcsDom`
  directly. HEAD's host wrapper `scripting_disabled_for_platform_object` (`event_handler_attrs.rs:593-620`) per input §C.
- **N2a**: population (e) (input §B's command and count; the polarity flip, J14 mapping); `host_data/mod.rs` touch per §4.
- **N2b**: the relation's callers — the VM native, the link path and app mode's drain (`app/drain_host/host.rs:91`, G4-7);
  input §E C17 must use D-a's field name `target_is_top_level` (G4-7); the depth input's unknown arm (D-a) needs a cell.
  The unknown arm is unrepresentable on HEAD's surface: `ScriptEngine::iframe_depth() -> usize`
  (`elidex-script-session/src/engine.rs:547`) and `HostData::iframe_depth` (`host_data/mod.rs:1172`) answer 0 when
  `HostData` is absent (`elidex-js/src/engine.rs:685-691`), indistinguishable from a top-level source; N2b needs
  `Option<usize>` (or an equivalent) so the restrictive decision is made inside `elidex-script-session::navigation`, not in
  a marshal site — callers `content/iframe/lifecycle.rs:390`, `iframe/thread.rs:229`, and the postMessage routing family
  (`tests_engine_s6a.rs:455`). The predicate over (`SandboxingFlagSet`, `NavigableRelation`, activation) cannot stay in
  `elidex-plugin` over `NavigableRelation` (crate direction, §4). At N2b the link path (`link_nav.rs:46`, `:57`) passes its
  target as HEAD's string and reads HEAD's `runtime.sandbox_flags()`; the resolved-target form is N8's (input §C).
- **N3**: HEAD's `elidex_plugin::parse_frame_ancestors`, `FrameAncestorsPolicy` and `is_framing_allowed` (`origin.rs:190`,
  `:202`, `:249`; caller `iframe/load.rs:509-511`) are absorbed and deleted (J8, input §C). The enforce-only filtering is CSP3 §4.2.5 step 2.1.5 ("If policy's disposition is "enforce", then set result to
  "Blocked""), the caller of §6.4.2.1 — the docstring cites both; `check_framing_allowed` passes the ancestor origin chain.
- **N4**: population (h) is a **two-step** migration (G4-5): at N4 the only writer is `create_document_root()` plus
  `set_sandbox_flags` (the store is still `HostData`), so N4 binds those fixtures to such roots; N5 migrates them again under
  (b)/(d); (d)'s command misses `engine.bind(ctx)` binds, which some N4-rewritten `tests_engine_s1*` fixtures use. Includes
  `tests_webapi_gate.rs:73-99` and `scripting.rs`'s in-source tests; the `*test*` filter misses in-file `#[cfg(test)]`.
  `WorkerGlobalRoot` has no production reader (J5); the worker test helper (`tests_worker.rs:36-43`, `:79-93`) binds
  through `bind_worker`, so it gets the marker with no fixture change; C7's unmarked arm is a direct `EcsDom` oracle;
  `bind_worker`'s new kind guard needs a cell (a Window VM never stamps). C28 and C28b pin the unresolvable arm in both
  realm polarities through the compile gate.
- **N5**: J14's DoD (no interim `empty()` survives) needs a grep over the deleted stores; (b)'s OOP order-proof tests take the
  input; (c) per-file co-occurrence; (d) misses `engine.bind(ctx)`/`with_bound`; mapping residuals per input §C. N5 deletes
  the `HostData` surface the link path reads (`popups_allowed`, `sandbox_flags`), so it re-points `link_nav.rs:46`, `:57`
  to F10r on `pipeline.document` until N8. The form gate (`form_input.rs:129`) moves after the form lookup (`:132`),
  because its document is the form's (`effective_node_document(form)`, D8); an unresolvable form document reads
  `all_represented()` (C32). N5 adds the forms-flag read the VM's `form.requestSubmit()` lacks at HEAD
  (`vm/host/html_form_proto.rs:499`), at step 4's position — before the validation block (`:546`, which fires `invalid`) and
  the `submit` dispatch (`:563`) — through F10r on the form document (D8, C33). N5 implements no *cannot navigate* check (§4.10.22.3 steps 1, 5.9, 9 are `#11-form-navigation`'s, A96).
- **N6**: `HistoryEntry`'s new field and its write point (D-h); error-page fallbacks per §7.5.7; the depth-limit fallback
  has no spec counterpart.
- **N7**: computing the iframe set and the embedder's set is shell marshalling (`build_load_context`,
  `iframe/lifecycle.rs:381-406`); only top-level embedders exist in production.
- **N8**: (f) misses the `ContentState`/`Tab` test constructors; the spawn function signatures change.
- **N9**: A14 re-measured at the cut (PR #531); `dom_inner_html.rs:419-421`, `:474-476` field removal.
- **Node-level**: none open. (Round 4's M3, D8's argument wording, is closed by D8's subject variants.)

## §6. ECS-native and Layering checks (seed; input)

**OO → ECS** (decided rows): `Document.activeSandboxingFlagSet` → `ActiveSandboxingFlagSet` (written once at spawn; a
creation fact, never read as the node clause's fact); "node document's browsing context is null" → the parent's interim rule (unresolvable → the node clause alone false), final
form the enriched `#11-domparser-full-document-parse-fidelity` marker; `Document.policyContainer.cspList` →
`DocumentCspList`; Window/WorkerGlobalScope dispatch → `WorkerGlobalRoot` stamped by `Vm::bind_worker` + one `get`; "an element's node
document" → `EcsDom::effective_node_document` (one function, called inside F10's one composition, D8); the spec's object
forms ("disabled for *el*" / "for *el's node document*" / Window / settings) → `ScriptingSubject` variants (D8); per-VM
`HostData.sandbox_flags` → deleted; `BrowsingContext.popupSandboxingFlagSet` → `TraversableCreation` on the `Tab` + a
required spawn input (I5 interim).

Layering: `vm/host/` marshals (route, F10r reads); the composition lives in `elidex-ecs` behind the script-session
boundary; the shell reads F10r on `pipeline.document`; parsers take the input; `elidex-navigation` calls `elidex-plugin`
pure functions. Per-site table: input file §C. Docstring citation seed: input file §D.

## §7. Test and coverage plan (input)

Declared surface as rev 3. The cells, each with its owning nested PR, are input file §E (rev 5: C25 and C26 withdrawn with
D-b/D-c). Rev 6: C28 (the node clause's unresolvable arm, unsandboxed settings document) and C29 (a DOMParser node appended
into the live document) are restated **through the compile gate**; C30 (reverse case) and C31 (mirror case) pin D8's
derivation from N1 in production form. Rev 7: C28b, C28's sandboxed twin (a no-`allow-scripts` realm: the getter is null
at N1 and every later landing — the settings clause is evaluated before the unresolvable node clause). Rev 8: C32, the
F10r unresolvable-document oracle (direct `EcsDom`: `None` → `all_represented()`, D8).

## §8. Out of scope, dispositions, records

**Registration timing** (one rule for every slot this node registers or first lists; the arms are exclusive, decided by
one test — does the gap's subject exist at HEAD?): a gap that exists at HEAD (A96) is registered **at approval** (re-eval =
the approval PR's merge date + 1 month); a gap created by code a nested PR adds is registered **at that landing** (re-eval =
the landing date + 1 month). No fixed calendar date is pre-written. **Exempt**: `#11-sandbox-flag-set-sources`, whose
registration and re-eval date (2026-11-01) the parent ratified (§6 row); this node keeps the parent's date.

**Slots** (each new slot recorded as an **addition to the parent's §6**, recorded not edited):
- `#11-sandbox-flag-set-sources` (the parent's registration; D1 restatement + D6 widening; cross-references
  `#11-browsing-context-model-window-open-postmessage` facet (1) for §7.4.2.4 step 4.1): **at approval**, re-eval
  2026-11-01 as the parent's §6 row ratifies (exempt from the timing rule's date).
- `#11-csp-violation-reporting` (NEW): A96; trigger: a CSP reporting WPT subset or the Reporting API
  (`m4-12-platform-gap-roadmap.md:465`, `:1128`); **at approval** — the gap exists at HEAD: HEAD parses and enforces
  `frame-ancestors` (`origin.rs:202`, `:249`) and sends no violation report.
- `#11-policy-container-members` (NEW): the embedder-policy and integrity-policy members; A96; trigger: a COEP or
  Integrity-Policy consumer (cross-reference `phase4-plan.md:139`, `:419-421`); **at approval** — the gap exists at HEAD:
  HEAD handles neither COEP nor Integrity-Policy (`git grep -n -i -P 'embedder.?policy|cross-origin-embedder|integrity-policy|\bcoep\b'
  f1cf5d67 -- crates`: 0 hits; `-P`, because this git's `-E` does not support `\b` and returns a silent 0). The ledger's `#11-referrer-policy` merge-candidate reference (overdue table) resolves
  from approval.
- `#11-nested-iframe-loading` (NEW): A96; obligation: pass the intermediate document as embedder; trigger: a
  nested-browsing-context WPT subset declared supported; **at approval** — the gap exists at HEAD: only top-level
  embedders load in production (§5.4 N7).

**Existing slots routed to** (append rows in the records table):
- `#11-domparser-full-document-parse-fidelity` — the node clause's ratified final owner: restate its marker facet with its
  **writer obligation** — a document whose browsing context becomes null at HTML §7.5.10 *destroy a document* step 8 (from
  *destroy a document and its descendants* and from *unload a document* step 20, §7.5.9) carries the marker, alongside the
  DOMParser documents the slot already targets; cross-reference B1 append (1). Add a **node-document-totality** facet: "a
  node's node document is always set (DOM §4.4; a Document's node document is itself, §4.5)". Evidence (record only; no
  totality work enters this node): the `owner = None` shims (`dom/mod.rs:372`, `:548`, `:575`, `:649`) and their parser
  callers (A23); `create_document_type` (`dom/mod.rs:672`) and `create_attribute` (`:690`), which spawn without an owner;
  `attach_shadow_with_init` (`dom/shadow.rs:189`, a `ShadowRoot` with no `AssociatedDocument`); the variable-owner
  `_with_owner(.., owner)` callers (`git grep -n -E '_with_owner\(' -- crates` less definitions, tests and literal
  `Some(..)`/`None`, node-creating callees only — `Range::new_with_owner` is not one: `dom/mod.rs:418`, `:627`; `elidex-custom-elements/src/entity_spawn.rs:57`;
  `elidex-dom-api/src/char_data/split_text.rs:149`, `cssom_sheet.rs:214`, `element/select.rs:327`, `:329`,
  `element/tree.rs:431`, `node_methods/text_content.rs:120`). The facet also covers the document entity outliving its
  nodes: `owner_document` guards a despawned owner (`dom/mod.rs:976-984`, `:993-998`, incl. template contents and the
  fragment-parse throwaway document that `finish_detached_fragment` (`dom/tree/teardown.rs:353`) despawns with
  `destroy_entity(document)`, `:364`); whether
  `adopt_subtree` re-homes template contents is unverified (round-4 M4). Cross-reference
  `#11-cross-document-adopt-on-insert` (insert does not re-home `AssociatedDocument`; D8 routes every gate around it).
  *Trigger* for the facet: the marker's landing, or a reader of the node document other than D8's function. N9 restates
  the `<noscript>` fragment facet it discharges.
- `#11-browsing-context-state-ecs-components` (B1) — appends: (1) under B1 the node clause becomes reachable for a document
  that loses its browsing context (HTML §7.5.10 *destroy a document* step 8, from both *destroy a document and its
  descendants* and *unload a document* step 20, §7.5.9); the marker's writer obligation is
  `#11-domparser-full-document-parse-fidelity`'s (above); (2) processing-algorithm step 3 (not fully active) is
  unreachable only until then; (3) the timers' step 9.5 transitivity breaks if scripting becomes runtime-mutable
  (user-disabled scripting, S5-4's `scripting_enabled` note) or cross-realm (`contentWindow.setTimeout`,
  `#11-windowproxy-browsing-context`); (4) the interim home of the source's depth (`iframe_depth`, D-a); (5) D8's
  per-reader settings documents — the relevant realm's, the entry global's, the form document — and
  `effective_node_document` diverge once a World holds several realms.
- `#11-worker-blob-script` (roadmap `m4-12-platform-gap-roadmap.md`, `#11-worker-blob-script` row) — registered in the
  open-slot ledger 2026-10-01 **ahead of approval** (records row "applied ahead of approval"), with the append of the
  policy container's blob arm (D-h) and the trigger "a blob URL store lands" (its old trigger had expired with no
  re-eval); re-eval per the registration-timing rule (approval merge + 1 month).
- `#11-browsing-context-model-window-open-postmessage` — iframe-originated opens (facet (4)); substep 8.2 (facet (1)), which
  now owns the one permitted sandboxed navigator end to end — its writer and §7.4.2.4 step 4.1's reader (D-a, D6); append
  the popup initial `about:blank` F19 clone and the auxiliary browsing-context object.
- `#11-oop-iframe-navigate-completeness` — enriched at N5: `BrowserToIframe::Navigate` carries the document's creation
  sandboxing flags from N5 (C9; `iframe/thread.rs:227` reads them), so a production sender must compute them through
  `determine_creation_sandboxing_flags` (N7).
- `#11-form-navigation` (roadmap `m4-12-platform-gap-roadmap.md`, its row; D-29 #209) — append at approval (the gap
  exists at HEAD, A96): HTML §4.10.22.3's *cannot navigate* checks (§4.6.5) — steps 1, 5.9 and 9 — are this slot's
  obligation under its existing trigger, together with `form.submit()`'s step 4 forms-flag read (it fires no event and
  does not navigate today). Reachable today through listener detach (D8's two windows): a form detached before
  activation submits from N5 only if its document still resolves to a document carrying F9, else refuses (C34); a form
  detached during `submit` submits at HEAD and after N5 alike (C35).
- `#11-document-base-url` (`<meta>` CSP); `#11-range-full-impl` (Fragment mode); `#11-template-contents-owner-document`;
  `#11-transient-activation-tracking`; `#11-windowproxy-browsing-context` (B1 append (3)'s cross-realm timers; overdue
  table).

**Overdue re-evals — 5-way disposition applied now** (`feedback_defer_lifecycle_policy.md:64-75`, `:94-97`):

| Slot | Due | Disposition | Cause / owner |
|---|---|---|---|
| `#11-referrer-policy` | 2026-09-30 (`project_open-defer-slots.md`, its entry) | extend-with-cause → 2026-11-01 | the policy container lands at N6; N6's plan-memo re-evaluates whether the referrer-policy member merges into it (merge-with-related candidate with `#11-policy-container-members`) |
| `#11-transient-activation-tracking` | 2026-09-30 (S5-4 §8-D2; `project_s5-4-sandbox-kickoff.md`, its D2 carve) | extend-with-cause → 2026-11-01 | D-a keeps activation a parameter; no new demand; N2b's plan-memo re-evaluates |
| `#11-domparser-full-document-parse-fidelity` | 2026-08-31 (`project_open-defer-slots.md`, its entry; the parent `:1695`) | extend-with-cause → 2026-11-01 | the parent assigns its refresh to IBP-sandbox (`:1695`); the facet enrichment (marker writer obligation, node-document totality) lands at N5 |
| `#11-range-full-impl` | 2026-09-01 (`m4-12-pre-pr-a2-sweep-result.md:140`; roadmap `:1251`) | extend-with-cause → 2026-11-01 | the Fragment parser mode is its only new consumer (D4); N9's plan-memo re-evaluates; still a PR7 prerequisite |
| `#11-windowproxy-browsing-context` | 2026-06-27 (`project_open-defer-slots.md`, its entry was "**Date**: 2026-06-27" before the 2026-10-01 split, the ledger's re-eval field — siblings carry future Dates and the parent read the domparser slot's Date as a passed re-eval, `:1695`; here it equals the registration day) | extend-with-cause → 2026-11-01 | its trigger's world_id half is superseded by #434's agent-scoped `EcsDom`; the remaining dependency is B1 (`#11-browsing-context-state-ecs-components`) and the S5 flip; this node only records its cross-realm timer case (B1 append (3)) and adds no demand. The ledger's Date was one, shared with `#11-auxiliary-browsing-context-opener`; it has been split (2026-10-01). This memo decides `#11-windowproxy-browsing-context` only: the opener slot's 2026-11-01 extension is a ledger-only disposition (home `#11-browsing-context-model-window-open-postmessage`) that no records row here ratifies |

**Records**:

| Record | Ships with |
|---|---|
| Amendments of the parent: F9 cell inner type, F10r cell text (D1); F10r cell's input domain — a node-derived document that may be unresolvable or a non-Window document without F9, each read as `all_represented()` (D8; the rule "flag absence can never fail open" unchanged, the domain widened beyond "a Window settings document"); F10 cell's `document_root` wording (D8); §4 F11 obligation and §3 §13.2.4.5 row (D4); §6 rows: D6 widening, the three new slots; the E26 reading (D7). Amendments of S5-4: §4.1 (D1); the D2 list; §4.3.3's top-navigation decision (D-a). Withdrawn-item record: rev 4's D-b/D-c/J15/J16/N4 `node-document` and rev 5's worker-root-creation writer of `WorkerGlobalRoot` (never ratified; listed so no later plan cites them); rev 5's N2 is split into N2a/N2b (the name `N2` is not reused). Slots registered at approval: `#11-sandbox-flag-set-sources` (re-eval 2026-11-01, the parent's); `#11-csp-violation-reporting`; `#11-policy-container-members`; `#11-nested-iframe-loading` | **the approval PR** |
| Memory: append to the roadmap's `#11-form-navigation` row (text = §8's `#11-form-navigation` bullet) — §4.10.22.3 steps 1, 5.9 and 9 (*cannot navigate*, §4.6.5) and `form.submit()`'s step 4 forms-flag read are that slot's obligation under its existing trigger (A96); reachable today through listener detach — a form detached before activation submits from N5 only if its document resolves to one carrying F9, else refuses (C34); one detached during `submit` submits at HEAD and after N5 (C35) | **the approval PR** |
| Memory dispositions applied 2026-10-01 ahead of approval (the overdue table above; the open-slot ledger's `#11-transient-activation-tracking` backstop; `#11-worker-blob-script` registered in the open-slot ledger with its D-h append, citing this memo's §8, and its roadmap row's (`m4-12-platform-gap-roadmap.md`, `#11-worker-blob-script`) expired trigger replaced by "a blob URL store lands"): each cites this memo and is ratified by the approval PR's merge | **the approval PR** |
| Memory: `project_open-defer-slots.md` — the `#11-scripting-disabled-eventhandler-processing-step1` closure ground and the `#11-cross-document-adopt-on-insert` entry restated (its event-dispatch gate deleted); `project_s5-4-sandbox-kickoff.md:35` | N1 |
| Append to `#11-browsing-context-model-window-open-postmessage` facet (1): §7.4.2.4 step 4.1's reader obligation (the one permitted sandboxed navigator, read by N2b's predicate; restrictive until the facet lands, D-a) | N2b |
| The parent's §6 *Enrich `#11-cross-document-adopt-on-insert`* action (its proxy promoted into F10); B1 record (relevant realm → document); `#11-browsing-context-state-ecs-components` appends (1)–(5) | N4 |
| B1 + SoT: parent item 4; S5-4 items 8, 9 (first half); enrich `#11-domparser-full-document-parse-fidelity` (node-document totality + the marker's writer obligation restated; its date was refreshed at approval); enrich `#11-oop-iframe-navigate-completeness` (`Navigate` carries the creation sandboxing flags; a production sender computes them through `determine_creation_sandboxing_flags`) | N5 |
| S5-4 item 15 CSP half; B1: F19 precedes B1 | N6 |
| B1: the embedder union | N7 |
| S5-4 item 15 popup half; F18 on the `Tab` in `#11-browsing-context-state-ecs-components`' SoT; append to `#11-browsing-context-model-window-open-postmessage` | N8 |
| S5-4 item 9 second half (E5); `#11-domparser-full-document-parse-fidelity`'s `<noscript>` facet restated; the `content-visibility` pin (unless an earlier firer recorded it) | N9 |

## §9. Prerequisites

- **E1 / E2**: the pure-move splits of `crates/core/elidex-ecs/src/dom/mod.rs` (1075) and
  `crates/dom/elidex-html-parser/src/lib.rs` (1017); before N4 and N5.
- **E26** (landing): the `content-visibility` umbrella memo's approval precedes the node's first nested landing (D7).
- **Live lanes**: local `domform-submittable-category` (touches `content/form_input.rs:289`, disjoint from `:129`; also
  `elidex-ecs/src/components.rs`, F9/F19's Home, and `elidex-js/src/vm/mod.rs`) and `domform-slice1`; open PR #531
  (html5ever 0.40.1) before N9.

## §10. Questions for the scout

- **Q1** Production paths reaching `window_dialogs.rs:52`/`:214` while unbound (eval: answered, A21).
- **Q2** Does elidex process `<script>` inserted by DOM mutation (Inert's "already started")?
- **Q3** Which backends run the html5lib tree suite; where is `#script-off` filtered?
- **Q5** Where is `IframeLoadContext` built; is the embedder's document in scope?
- **Q6** Branches touching N4/N5's files at their cuts.
- **Q7** Production top-level document creation sites (F18's readers), with a command.
- **Q8** Consumers of `LoadedDocument` / `load_document` outside `elidex-shell`.

## §11. Definition of done (node)

- Every nested PR in §5.1 has landed in an order consistent with §5.2, each with its own reviewed plan-memo.
- The decisions of §0.4 and §2 hold at the node's last landing; each nested plan-memo states its DoD greps.
- The §8 records have landed with the approval PR or their nested PRs.
- **Checkers** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-09-ibp-sandbox.md`; `python3 .claude/tools/plan-sweep.py --pattern
  'node-document|DocumentBrowsingContext|J15|J16|liveness|marker.s removal' --pattern 'E26' --pattern
  'create_worker_document_root|worker-root|(?-i:\bN2\b(?![ab]))|fails open|gate fails open|already asserts'
  docs/plans/2026-09-ibp-sandbox.md` (`plan-sweep.py` compiles every pattern case-insensitively, so the `N2` pattern
  carries a case-sensitive group and does not hit the §5.2 edge label `n2`) — a residue sweep, read **by surface, not by
  count**: legitimate hits remain (the effective-node-document rule and the composition's node-document derivation — §0.4
  D8 and the node clause, §1 N1, §4 F10, §5.2 n3; the node-document-totality facet; the withdrawal notes in §0.4
  `WorkerGlobalRoot` and §2 J17, and the withdrawn-item records; the revision history; and this bullet's own
  commands; `E26`'s hits are its live decisions — D7, §5.2's parent edges, §8's records, §9). The same patterns are run over the input file and over the memory surfaces the
  records rows cite, under §0.1's memory path: `/usr/bin/grep -n -E
  'create_worker_document_root|worker-root|\bN2([^ab]|$)'` on `project_open-defer-slots.md` and
  `project_s5-4-sandbox-kickoff.md`, and on the lane SSoT `project_line-box-decorated-inline-content.md`'s **resume block
  only** — the first `### ` heading containing `NEXT SESSION STARTS HERE` and not `(superseded)`, up to the next `### `
  heading (`awk '/^### .*NEXT SESSION STARTS HERE/ && !/superseded/ {f=1; next} f && /^### / {f=0} f'`), with split-history
  wording removed first (`sed -E 's/N2 ?(→|->) ?N2[ab]//g'`), as the memo sweep exempts its revision history: no memory
  surface may name the withdrawn `N2` or the rev 5 writer. `plan-xcheck.py` is scoped to the umbrella memo and does not
  apply.

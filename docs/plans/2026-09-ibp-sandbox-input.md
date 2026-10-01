# IBP-sandbox node — nested-PR input (not decisions)

Companion to the node plan-memo `docs/plans/2026-09-ibp-sandbox.md` (rev 8). Under that memo's **approval scope**, this
file is **input to each nested PR's plan-review**: it decides nothing. Every row names its owning nested PR (N1, N2a, N2b,
N3–N9, the node memo's §5.1 numbering), and that PR's plan-review re-derives it; a finding against a row changes the node memo only
when it changes a decision there. Named risks per nested PR stay in the node memo's §5.4; this file holds the tables they
refer to.

Coordinates are `f1cf5d67`; `grep` = `/usr/bin/grep`. The tables were first measured for the node memo's rev 3 and are
renumbered and updated here to rev 8's decisions: no post-hoc writer (D-d), §7.4.2.4 by relation over the source's depth
(D-a; unknown depth restrictive for `_top`/`_parent` only), resolved link targets with step 5 in the session (D-e), the
node clause as the parent's ratified interim with one `EcsDom::effective_node_document` called inside F10's one
composition over `ScriptingSubject` (D8; an unresolvable node-derived document reads `all_represented()` through F10r),
`WorkerGlobalRoot` stamped by `Vm::bind_worker` (the parent's writer) with the non-panicking worker-kind guard N4 adds, and rev 5's N2 split into N2a/N2b. Rev 4's `node-document` nested PR, its population and its marker were withdrawn at rev 5.

Nested PRs: N1 `handler-gates` · N2a `flagset` · N2b `nav-relation` · N3 `csp-list` · N4 `f10-route` · N5 `stamp` ·
N6 `csp-sandbox` · N7 `embedder-union` · N8 `popup` · N9 `parser-mode`.

## §A. Creation entries and their true sources (owner N5)

At N5 every Window document-creation entry takes a `DocumentCreationInput` and passes it to the one writer,
`EcsDom::create_window_document_root` (NEW), with HEAD's sources (later sources: N6 CSP, N7 embedder union, N8 popup):

| Entry | Input at N5 |
|---|---|
| `load_document` (`loader.rs:181`) — callers `content/mod.rs:651`, `content/navigation.rs:216`, `app/navigation.rs:295`, `iframe/load.rs:122`, `pipeline.rs:947` | top-level: empty; URL iframe: the parsed `sandbox` attribute; OOP `Navigate`: the message's creation flags |
| `build_pipeline_interactive_shared` via `build_iframe_pipeline` (`load.rs:463-480`) — `srcdoc`, `about:blank`, no-src, every `blank_entry` | the parsed `sandbox` attribute |
| `build_pipeline_interactive{,_with_network}` (`pipeline.rs:563`, `:644`), blank tab, app mode | empty |
| WPT harness (`harness.rs:38`) | empty |
| parse entries (`parse_html`, `parse_strict`, `parse_progressive_str`, `parse_progressive`) → root spawn (`convert.rs:18`, `tree_builder/mod.rs:102`) | passed through |

Not applicable: worker / service-worker roots (`create_document_root()`, no F9; from N4 `Vm::bind_worker` stamps
`WorkerGlobalRoot` on them), DOMParser's `create_document_node`,
the fragment scratch document. Fixtures (D-d) create their documents through the same writer with an explicit
`DocumentCreationInput::unsandboxed()` or the sandboxed set they assert; the shell gains a builder taking the input for
its tests.

## §B. Migration populations (by property, counted at `f1cf5d67`)

| Pop. | Property | Command | Count | Owner | Rewrite target |
|---|---|---|---|---|---|
| (a) | invoke-time suppression assertions | `git grep -l parseFromString -- 'crates/script/elidex-js/src/*test*' \| xargs grep -l -E 'dispatchEvent\|\.on[a-z]+ *=\|onclick='` ∪ `git grep -l set_sandbox_flags -- 'crates/**/*test*' \| xargs grep -l -E 'dispatchEvent\|\.on[a-z]+ *=\|on[a-z]+=\|call_listener'` | 2 files (`vm/tests/tests_event_handler_attrs.rs`, `tests_call_listener.rs`) | N1 | `:567`, `:691`, `:719` → C8b (the handler runs); `:597`'s oracle re-grounded on C8; `tests_call_listener.rs:188` survives via the compile gate |
| (b) | sandbox installed through the API N5 deletes, or a `sandbox_flags:` literal | `git grep -n -E 'set_sandbox_flags\|\.sandbox_flags\(\)\|sandbox_flags:' -- 'crates/**/*test*'` | 25 lines, 8 files | N5 | VM sites → documents created by `create_window_document_root(input)`; `tests_engine_s1b.rs:166-196` deleted with the trait surface; `content_window_open_tests.rs:110` → the shell builder taking the input; `content_iframe_security_tests.rs:119/232/247/263/283` → assert F9 on `pipeline.document`; `:468`/`:502` move into the input of `synth_cross_origin_loaded` (`:396-407`); `:538`/`:580` drop the field; `:618`/`:648` rename. The order-proof oracle becomes stamp at parse |
| (c) | one `EcsDom` holding ≥2 document roots | `for f in $(git grep -l 'create_document_root()' -- crates); do awk -v F=$f '/EcsDom::new\(\)/{n=0} /create_document_root\(\)/{n++; if(n==2) print F":"NR}' $f; done` | 15 hits (1 a doc comment) | N5 | no rewrite (settings identity is the relevant realm's document, J13); C24 pins it. Named risk: the command is per-file co-occurrence, not per-`EcsDom` |
| (d) | VM-binding fixtures over an unstamped root | `git grep -l -E 'vm\.bind\(\|\.vm\(\)\.bind\(\|bind_vm\(\|\.bind\(&raw\|\.bind\(std::ptr' -- 'crates/script/elidex-js/src/*test*' \| xargs grep -l 'create_document_root()'` | 97 files (521 `bind_vm(` sites; 20 direct binds in 6 files) | N5 | move document creation from `create_document_root()` to the writer with `unsandboxed()` (D-d; `bind_vm` does not stamp). Named risk: the command misses `engine.bind(ctx)` / `with_bound` binds |
| (e) | `IframeSandboxFlags` users | `git grep -c IframeSandboxFlags -- crates` | 125 occurrences, 16 files | N2a | `SandboxingFlagSet` (`None` → `empty()`, J14) |
| (f) | content-thread spawn callers | `git grep -n 'spawn_content_thread\(_url\|_blank\)\?(' -- crates/shell` less definitions | 9 (4 production: `content_messages.rs:383`, `threaded.rs:672`, `viewport.rs:274`, `:290`; 5 tests) | N8 | pass a `TraversableCreation`. Named risk: misses the `ContentState`/`Tab` test constructors |
| (g) | document-parse entry callers | `git grep -n -E '(parse_html\|parse_strict\|parse_progressive_str\|parse_progressive)\(' -- crates \| grep -v -E 'pub fn \|^[^:]+:[0-9]+:\s*//'` | 57 | N5 | pass a `DocumentCreationInput` |
| (h) | evaluates or calls a gated native while unbound | `git grep -l fresh_unbound -- 'crates/script/elidex-js/src/*test*'` ∪ `tests_window_open_modals.rs` ∪ `tests_webapi_gate.rs` (`:73-99`) ∪ `scripting.rs`'s in-source tests (`:150-175`) | 7 + 3 files (the `*test*` filter misses in-file `#[cfg(test)]` modules) | N4 | two-step: at N4 bind a `create_document_root()` root (+ `set_sandbox_flags` where sandboxed behaviour is asserted); at N5 move to the writer under (b)/(d); C4b keeps one unbound refusal |

**Must not break**: S5-4b's order-proof suite (`content_iframe_security_tests.rs`) and `content_window_open_tests.rs`,
migrated in N5 via (b).

## §C. Per-site Layering table

| Site | Today | After | Owner | Engine-independent API |
|---|---|---|---|---|
| `engine.rs:174` eval gate | `HostData` `scripts_allowed` | route → F10 with `ScriptingSubject::Settings` (§8.1.4.4 step 2: the settings clause alone over the bound settings document); `ScriptingSubject` keeps `Window` and `OtherPlatformObject` (settings clause only; a worker global included) distinct | N4 | host route → `scripting_disabled_for_platform_object` (session) → `EcsDom::scripting_disabled_for` |
| `event_handler_attrs.rs:550-556` compile gate | `HostData` `scripts_allowed` | HEAD's composition given the **element** (N1: settings clause first, then the parent's interim derives its node document, D8); from N4 the route passes `NodeDocumentOf(element)` (a Window target: `Window(w)`) and F10 derives the document, its settings document and the clause order; the gate has no `None` branch | N1, N4 | same |
| `event_handler_attrs.rs:593-620` host wrapper `scripting_disabled_for_platform_object` | wraps the session composition over `HostData` | N1 deletes it if its last callers are the three invoke gates (the compile gate then calls the session function directly); otherwise N4 folds it into the route. No second unbound fail-open path survives N4 | N1, N4 | — |
| `engine.rs:207-217`, `dispatch_target.rs:203-213`, `natives_promise.rs:533-542` | invoke wrapper | deleted | N1 | — |
| `window_dialogs.rs:52`, `:214` | `HostData` flags | route → F10r; unbound → `all_represented()`; `:214` marshals into the session's `window.open` entry (step 5 there, D-e), which derives the §7.4.2.4 relation once | N2b, N4, N5, N8 | `EcsDom::active_sandboxing_flags`, `sandbox::*`, `window_open_disposition` |
| `vm/host/settings_route.rs` (NEW file) | — | the VM route + wrappers | N4 | — (marshalling) |
| `vm_api.rs:382` `bind_worker` (callers `vm/worker_thread.rs:193`, `vm/sw_thread.rs:209`, `tests_worker.rs:36-43`) | binds; no global-kind check (`:382-411`) | also stamps `WorkerGlobalRoot` on the root it binds (the parent's writer), guarded, non-panicking: only on a worker-kind VM, a Window-kind VM binds unstamped (N4 adds the guard); the roots stay `create_document_root()` | N4 | — |
| `scripting.rs` (`elidex-script-session`) | composition | forwards to `EcsDom::scripting_disabled_for` | N4 | — (boundary) |
| `html_fragment.rs`, `inert_document.rs:143`, `dom_inner_html.rs:403`, `:419-421`, `:458`, `:474-476` | pass `scripting_disabled` | one fragment entry derives the mode through F10 with `NodeDocumentOf(context)` (§13.4 steps 5, 10); an unresolvable contextDocument leaves the mode to the settings clause over the context's realm; the field is dropped | N9 | `elidex-script-session` fragment entry |
| `form_input.rs:129` | `runtime.forms_allowed()` | moved after the form lookup (`:132`), because its document is the form's: F10r on the form document, `effective_node_document(form)` (D8; equals `pipeline.document` while the form stays in the document; a listener on the shell's pre-activation events can detach it, D8); unresolvable → `all_represented()` inside F10r (C32). No *cannot navigate* check: §4.10.22.3 steps 1, 5.9, 9 are `#11-form-navigation`'s (A96) | N5 | `EcsDom::active_sandboxing_flags`, `sandbox::forms_allowed` |
| `vm/host/html_form_proto.rs:499` (`form.requestSubmit()`) | no forms-flag read (fires `invalid` via validation `:552`, `submit` `:563`, `formdata` `:571`) | reads the forms flag through F10r on the form document (`effective_node_document(form)`, unresolvable → `all_represented()`) at §4.10.22.3 step 4's position: after the submitter checks, before the validation block (`:546`, which fires `invalid`) and the `submit` dispatch (D8, C33; A96 closed). `form.submit()` (`:469`) fires no event and does not navigate: its step 4 read is `#11-form-navigation`'s | N5 | `EcsDom::active_sandboxing_flags`, `sandbox::forms_allowed` |
| `link_nav.rs:46`, `:57` | `runtime.popups_allowed()` / `sandbox_flags()` | N2b: HEAD's target string and `runtime.sandbox_flags()`, plus the relation; N5: flags from F10r on `pipeline.document` (N5 deletes the `HostData` surface); N8: `window_open_disposition(resolved target, F10r, relation, activation = true)`; `""` = `_self`; named miss → new tab (D-e) | N2b, N5, N8 | `window_open_disposition` |
| `content_messages.rs:372-392` | `Vec<Url>` → spawn → `create_tab` | build `TraversableCreation`, spawn with it, store on `Tab` | N8 | — (browser record) |
| `navigation.rs:617-624` named MISS | `aux_nav_allowed` | same decision + carried set | N8 | — |
| `ipc.rs:532` | `OpenNewTab(url)` | `OpenNewTab { url, popup_sandboxing_flag_set }` | N8 | — |
| `app/tab.rs` `Tab` | navigable record | + `TraversableCreation` | N8 | — |
| `content/mod.rs:504`, `:531`, `:556` spawn functions | spawn | `TraversableCreation` required | N8 | — |
| `app/threaded.rs:644`/`:672`, `viewport.rs:274`/`:290` | spawn | spawn with an empty `TraversableCreation` | N8 | — |
| `loader.rs:181` `load_document` and its five callers | `load_document(url, h, req)` | + creation input (N5), from `determine_creation_sandboxing_flags` (N7, N8) | N5, N7, N8 | `elidex-plugin` pure functions |
| `iframe/load.rs` | `parse_sandbox`, `apply_sandbox_origin`, framing header scan (`:509-511`) | creation input from `IframeLoadContext`; framing via `csp::frame_ancestors_check` | N3, N6, N7 | `determine_creation_sandboxing_flags`, `determine_origin`, `frame_ancestors_check` |
| `elidex-plugin/src/origin.rs:190` `FrameAncestorsPolicy`, `:202` `parse_frame_ancestors`, `:249` `is_framing_allowed` (re-exported `lib.rs:60-62`) | HEAD's frame-ancestors parser and check | absorbed into `CspList` / `csp::frame_ancestors_check` and deleted (J8: one CSP parser) | N3 | `elidex-plugin::csp` |
| `pipeline.rs:256-281` | origin fork + flag install | one origin install, no flag install | N5, N6 | `determine_origin` (via `LoadedDocument`) |
| `pipeline.rs:563`, `:644` | parse without input | parse with empty input | N5 | — |
| `pipeline.rs:390-421` `build_paged_pipeline` | dead code | deleted | N5 | — |
| `harness.rs:38` | `parse_html(html)` | `parse_html(html, unsandboxed())` | N5 | — |
| `loader.rs:256-278` | unconditional script fetch | gated per §4.12.1.1 step 18 through F10 with `Node(el)` and the document being created as realm resolution (D5): the settings clause reads its F9, stamped from the creation input | N6 | `EcsDom::scripting_disabled_for` |
| `iframe/thread.rs:227` | `runtime.sandbox_flags()` | the `Navigate` message's creation flags | N5 | — |

## §D. Docstring citation table (seed for each nested memo)

| Item (NEW unless noted) | Cites | Owner |
|---|---|---|
| `SandboxingFlagSet`, `parse_sandboxing_directive` | HTML §7.1.5 *parse a sandboxing directive* | N2a |
| `scripts_allowed`, `forms_allowed`, `popups_allowed`, `modals_allowed`, `scripting_enabled` (restated) | HTML §7.1.5; §8.1.3.4 | N2a |
| the top-navigation predicate (restated), `NavigableRelation` (NEW) and its derivation | HTML §7.4.2.4 *allowed by sandboxing to navigate* steps 1–6 | N2b |
| the `window.open` entry | HTML §7.2.2.1 *window open steps* step 5 | N8 |
| `determine_creation_sandboxing_flags` | HTML §7.1.5 *determine the creation sandboxing flags* | N7 |
| `popup_sandboxing_flag_set`, `PopupSandboxingFlagSet`, `TraversableCreation` | HTML §7.3.1.7 *the rules for choosing a navigable* step 8, substep 9; §7.1.5 *popup sandboxing flag set*; §7.1.3.2 *obtain a browsing context to use for a navigation response* steps 13–14 | N8 |
| `ActiveSandboxingFlagSet` | HTML §7.1.5 *active sandboxing flag set*; §7.5.1 *create and initialize a Document object* | N5 |
| `DocumentCspList`, `CspList`, `CspList::csp_derived_sandboxing_flags` (NEW) | HTML §7.1.7 policy container CSP list; §7.1.5 *CSP-derived sandboxing flags*; CSP3 §2.2 | N3, N6 |
| `parse_serialized_csp`, `parse_response_csps`, `csp::frame_ancestors_check` (NEW) | CSP3 §2.2.1; §2.2.2; §6.4.2.1 steps 1–7; §4.2.5 step 2.1.5 (enforce-only) | N3 |
| `determine_origin` | HTML §7.3.2.1 *determine the origin* | N6 |
| `DocumentCreationInput`, `EcsDom::create_window_document_root` (NEW) | HTML §7.4.2.1 navigation params; §7.5.1; §7.3.2.1 *create a new browsing context and document* steps 6, 15 | N5 |
| `WorkerGlobalRoot` (NEW), its write in `Vm::bind_worker` and the worker-kind guard | HTML §8.1.3.4 settings clause; §10.2.1.1 | N4 |
| `EcsDom::effective_node_document` (NEW) | HTML §8.1.8.1 *get the current value of the event handler* step 3.1; §13.4 step 5; §4.10.22.3 step 3; DOM §4.4 *node document*; its unresolvable arm (node clause false, inside F10) is the parent's interim | N4 (N1 via HEAD's composition) |
| `EcsDom::scripting_disabled_for` (NEW), `ScriptingSubject` (NEW), `EcsDom::active_sandboxing_flags` (NEW), the VM route | HTML §8.1.3.4 (its object forms; clause order: settings clause first); §8.1.3.3.4 *relevant settings object*; §7.2.2 *associated Document*; fail-closed defaults are elidex's own | N4, N5 |
| `ParserScriptingMode`, `ParserScriptingMode::for_document` (NEW), the fragment derivation | HTML §13.2.4.5; §13.4 step 10 | N9 |
| compile gate | HTML §8.1.8.1 *getting the current value of the event handler* steps 3.1–3.2 | N1 |
| eval / forms / script-fetch gates | HTML §8.1.4.4 *check if we can run script* step 2 (`Settings`); §4.10.22.3 *form submission algorithm* steps 3–4 (steps 1, 5.9, 9, §4.6.5 *cannot navigate*, are `#11-form-navigation`'s); §4.12.1.1 *prepare the script element* step 18 (`Node(el)`) | N4, N5, N6 |
| `window.open` flag read | HTML §7.3.1.7 *the rules for choosing a navigable* step 3; §7.2.2.1 *window open steps* steps 2, 13 | N4, N5, N8 |

## §E. Cells (C1–C32, C28b)

Declared surface: WPT `html/browsers/sandboxing/`, `content-security-policy/sandbox/`,
`content-security-policy/frame-ancestors/`, `html/semantics/scripting-1/the-noscript-element/`, html5lib `#script-off`,
as engine-independent equivalents. Oracles: `elidex-plugin` truth tables, `elidex-ecs` units, parser crates,
`cargo test -p elidex-js --all-features`, `cargo test -p elidex-shell`. († = Appendix A's *Rev 4 cell ideas*.)

| # | Cell | Owner |
|---|---|---|
| C1† | a top-level root carries F9 when the parse entry returns, before `resolve_with_mode` | N5 |
| C2† | sandboxed iframe (URL, `srcdoc`, blank arms): no script; F10 true on its `<canvas>`; `<noscript>` Disabled (N9) | N5 / N9 |
| C3† | DOMParser document: F10 true on its nodes (node clause: their effective document is not the settings document); its parse Disabled | N5 / N9 |
| C4† | a document created without the writer (`create_document_root()`) and bound: eval refused, compile gate null, dialogs refused, `window.open` blocked, forms/`_blank` refused | N5 |
| C4b | HostData installed, nothing bound: `alert` refused, `window.open` blocked | N4 |
| C5† | `<canvas>` in a declarative shadow root resolves its document through the composed root | N4 |
| C6† | every gate refuses in a sandboxed frame as at HEAD, fixtures created through the writer | N5 |
| C7 | on a `bind_worker`-bound DOM the root carries `WorkerGlobalRoot` and `EcsDom::scripting_disabled_for(worker_document, ScriptingSubject::OtherPlatformObject)` (a worker settings document) is false with `all_represented()` flags; the unmarked arm is a **direct `EcsDom` oracle** on a root never passed to `bind_worker`: true; a Window-kind VM through `bind_worker` → no marker (the non-panicking guard) | N4 |
| C8 | compile gate on an element of a DOMParser document: the getter is null | N1 |
| C8b | an IDL-set handler, and a handler compiled while live, on a node moved into a DOMParser document run (rewrites `tests_event_handler_attrs.rs:567/:691/:719`) | N1 |
| C9 | OOP `Navigate` builds from the message's creation flags | N5 |
| C10 | parse truth table (tokens, implied grants, propagate flag, the sandboxed navigation flag, empty = `all_represented()`) | N2a |
| C11 | CSP parse: comma-joined header, report-only, duplicate directive, empty policy not appended; `frame_ancestors_check` steps 1–7 incl. step 1 local URL | N3 |
| C12 | `Content-Security-Policy: sandbox allow-scripts` top level: scripts run, origin `"null"`, forms refused | N6 |
| C13 | `Content-Security-Policy: sandbox`: no script runs or is fetched | N6 |
| C13b | `<iframe sandbox src=…>` with an external `<script src>`: not fetched | N6 |
| C14 | report-only `sandbox` no effect; two enforced `sandbox` policies: the last wins | N6 |
| C14b | a traversal back to a local-URL document restores its history-stored policy container (D-h, step 1) | N6 |
| C14c | a `pushState` (and a fragment) entry of a document whose URL requires storing the policy container, then a cross-document navigation, then a traversal back to that same-document entry: the document's history policy container is restored, not a default (D-h, shared document state) | N6 |
| C15 | `srcdoc` child of a CSP-sandboxed top level: F19 clone, F9 from the union, scripts off | N7 (lands second; N6 → N7) |
| C16 | CSP-sandboxed (`sandbox allow-scripts`) top level embeds an iframe with no `sandbox`: scripts on, origin opaque, forms refused | N7 |
| C17 | CSP-sandboxed top level (`sandbox allow-scripts`): `_top` / `_parent` by link click and by `window.open` are allowed with and without activation (§7.4.2.4 step 1, source is target); 3.2/3.3 pinned at unit level with `TargetIsAncestor { target_is_top_level: true }` × activation; depth 2 `_parent` → `TargetIsAncestor { target_is_top_level: false }` (3.1); unknown depth → the restrictive relation; 4.2/5 with the sandboxed navigation flag; 4.1 never permitted (interim) | N2b |
| C18 | CSP-sandboxed top level (`sandbox allow-scripts allow-popups`) calls `window.open(u, '_blank')`: the new tab's documents carry the set, incl. after a later navigation; with `allow-popups-to-escape-sandbox`: empty (iframe-originated opens strand — facet (4) of `#11-browsing-context-model-window-open-postmessage`) | N8 |
| C19 | link `_blank` and named-MISS promotion carry the set computed at the call | N8 |
| C19b | the popup's first document (spawn parameter at `content/mod.rs:651`) and a later `handle_navigate` document carry the same set; the `Tab` holds it | N8 |
| C20 | for every Window creation entry, parser mode == F10's settings clause on the created root | N9 |
| C21 | `innerHTML` `<noscript><p id=x>` in a sandboxed `srcdoc` document: `#x` an element on strict and forced-tolerant; unsandboxed: raw text | N9 |
| C22 | html5lib `#script-off` enabled (`tests_html5lib_tree.rs:19`) | N9 |
| C23 | the `content-visibility` pin (noscript witness at (10px, 40px) inside `hidden="until-found"`; elidex hits the `span`, conformant the `div`; flip = the c-v PR wiring skipped contents into hit testing), recorded by its first firer | N9 by default |
| C24 | two roots in one `EcsDom` (`tests_match_media.rs:850-857`): bound to `doc_a`, gates read `doc_a`'s F9 | N5 |
| C27 | a link whose named target misses opens a new tab when the auxiliary-navigation flag is clear, and does nothing when set; `<a target="">` navigates `_self` | N8 |
| C27b | `window.name = 'foo'` then `<a target="foo">` (and `window.open(url, 'foo')`): navigates the current context, no new tab, and no block in a sandbox without `allow-popups` (D-e) | N8 |
| C28 | the node clause's unresolvable arm (parent's interim), **through the compile gate**, with an **unsandboxed** settings document: a parser-created element (owner `None`, A23) detached by script, with a raw handler attribute, resolves no node document and its handler compiles | N4 |
| C28b | C28's sandboxed twin: the same detached owner-`None` element with a raw handler, in a realm whose settings document has no `allow-scripts` — the getter is null and the handler never runs, at N1 (HEAD's composition, settings clause first) and every later landing | N1 |
| C29 | HEAD's proxy case **through the compile gate**: a DOMParser-created node appended into the live document resolves to the live document (composed tree root); its raw handler compiles and runs | N4 |
| C30 | reverse case, production form: `b = document.createElement('button'); b.setAttribute('onclick', …); dp.body.append(b); b.dispatchEvent(new Event('click'))` with `dp` a DOMParser document — the getter is null and the handler does not run, at every landing from N1 | N1 |
| C31 | mirror case, production form: a DOMParser node appended into the live document, then `setAttribute('onclick', …)` and `click()` — the handler runs, at every landing from N1 (C29's oracle at N1) | N1 |
| C32 | F10r's unresolvable-document arm, a **direct `EcsDom` oracle**: `active_sandboxing_flags(None)` (a node-derived document that does not resolve, e.g. `effective_node_document` of a detached owner-`None` form) → `all_represented()`, so the forms flag refuses (fail closed, D8) | N5 |
| C33 | `form.requestSubmit()` in a document whose flag set lacks `allow-forms`: no `invalid`, `submit` or `formdata` event fires (a `required` empty control included), from N5 (HEAD fires them, A96) | N5 |
| C34 | a `click` listener removes a parser-created (owner-`None`) form before activation: no navigation, from N5 (HEAD submits); a `createElement` form detached the same way still submits (resolves to the live document) | N5 |
| C35 | a `submit` listener detaches the form: it still submits, at HEAD and after N5 (§4.10.22.3 step 5.9 is `#11-form-navigation`'s, A96) | N5 |

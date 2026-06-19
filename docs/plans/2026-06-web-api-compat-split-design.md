# Web API Core/Compat Split — Audit, Inventory & Boundary Design (Program A / A0)

Plan date: 2026-06-20 JST
Status: **DESIGN / DOC ONLY — no `.rs` change in this PR.**
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → Program A, PR0 (A0).
Follows up: audit F1 (sync storage on core surface) + F2 (`document.cookie` on core Document)
in `docs/audits/2026-06-elidex-philosophy-implementation-audit.md`.
Audience: Claude / maintainers (and Codex via `## Review guidelines` of the umbrella).

> This is the design gate for the F1/F2 remediation. Per the umbrella and
> `CLAUDE.md` ("Edge-dense work = multi-PR program + 実装前 plan-review 必須"),
> F1 and F2 may **not** be fixed in a single PR; each constituent PR (A1/A2/A3)
> must pass `/elidex-plan-review` before implementation. This doc decides the
> *boundary and the mechanism* so those PRs have an anchor that is not the stale
> audit prose. **It deliberately implements nothing.**
>
> Every file:line below was **re-grepped against `main` at HEAD `2f4a9d5a`
> (2026-06-20)** — not transcribed from the audit (2026-06-19) or the umbrella
> snapshot. Deltas vs. the umbrella's §2.7 framing are called out in §2; the most
> important is that the `WebApiSpecLevel`/`DomSpecLevel` **enums already exist** —
> what is missing is the *carrier + enforcement + mode selector*, which is a
> sharper and more actionable gap than "no gate exists at all."

---

## 0. TL;DR (decisions this doc commits to)

1. **Inventory + classification (§1).** Of the entire core-VM
   `Window`/`Document`/`navigator` surface, exactly **three** API families are
   **Legacy** (compat-destined): the **Web Storage** family
   (`localStorage`/`sessionStorage` **+ `StorageEvent`**, Codex P2),
   `document.cookie` (**+ the coupled `navigator.cookieEnabled`**, Codex P2), and
   the **live-collection** surface (`getElementsByClassName` confirmed by design
   §12.1.2; `getElementsByTagName`/`getElementsByName` **+ `forms`/`images`/`links`
   — all confirmed live, not snapshot — + the `Element`-side getters** need an
   explicit design call, routed to B0). Everything else is **Modern**. **No
   Deprecated API is implemented** (verified absent: `document.write`/`writeln`/
   `open`/`close`/`all`/`execCommand`, `XMLHttpRequest`, `alert`/`confirm`/
   `prompt`, `attachEvent`). The `navigator` UA-compat constant fields
   (`appName`/`appVersion`/`product`/…) are **frozen-for-compat constants**, not
   removable Legacy — a distinct third bucket (§1.4).

2. **Mechanism (§3).** The recommendation is a **two-part gate**: a per-API
   `WebApiSpecLevel`/`DomSpecLevel` classification *attached at the VM
   registration site*, enforced by a **single runtime `SpecLevelPolicy`** resolved
   once at VM/session construction from an embedder-supplied **mode**
   (`browser-compat` / `browser-core` / `app`), **plus** a compile-time
   `feature = "compat-webapi"` that lets `app` builds refuse to even *link* the
   compat shims. The gate is applied at **every install seam** — `install_*`
   tables **and** direct `register_*_global()` installers (Codex P2) — so the same
   gate governs storage, `StorageEvent`, cookie, live collections, and any future
   legacy top-level global (e.g. `XMLHttpRequest`) — not a storage-only special
   case (**One issue, one way**). The classification *vocabulary* already exists
   in `elidex-plugin`; A1 builds the carrier + enforcement + mode selector.

3. **Compat placement + async core (§4).** The VM-native JS glue (`vm/host/
   storage.rs`, the cookie natives) **stays in `elidex-js`** gated by the feature +
   policy — it imports private VM types, so relocating it into a crate `elidex-js`
   depends on would be a **dependency cycle** (Codex P2); the backends already live
   in `elidex-storage-core` / `elidex-net`, so whether a *separate*
   `-storage-compat`/`-cookies-compat` crate is even needed is an A1 plan-review
   question (§4.1). The async **core equivalents** (`elidex.storage`, CookieStore)
   are **not** a precondition for the `BrowserCompat` split (A2/A3 ship without
   them) but **are** the precondition for ever selecting `BrowserCore`/`App`
   storage (Codex P2 — a *core* session is contracted to have `elidex.storage`,
   §14.4.3) — recorded as defer slot **`#11-async-core-storage-cookiestore`**
   (§4.2), **not** built here.

4. **PR list (§5).** A1 (gate/mode mechanism) → A2 (storage behind compat) ∥ A3
   (cookie behind compat). Adjusted B/C/D/E acceptance criteria carried from the
   umbrella with the sharpened mechanism. Plus two cheap clerical sweeps:
   stale "stub" comments (F2) **and** stale spec-section citations in the same
   files (newly found here — §1.5).

---

## §A. Spec coverage map

> Plan-review schema section (annex token `§A` chosen so it does not collide
> with this doc's numeric "§3 = Design Decision" cross-references). For a
> **doc-only** A0 the "Touch" column names the *registration site classified* +
> the *child PR that will act on it*, not a dispatch site this PR edits (this PR
> edits no `.rs`). "Full enum?" = is the classification **closed** for that
> surface. All citations webref-verified (§7).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §12.2 The API | classify Web Storage surface | Legacy (sync→compat) | `window.rs:438` storage install → **A2** | ✓ (localStorage+sessionStorage) | yes (`setItem` value) |
| WHATWG HTML §12.2.1 The Storage interface | classify `Storage` iface | Legacy | `storage.rs:237` `register_storage_global` → **A2** | ✓ | yes |
| WHATWG HTML §12.2.2 The sessionStorage getter | classify `sessionStorage` | Legacy | `window.rs:524` / getter `:541` → **A2** | yes | yes |
| WHATWG HTML §12.2.3 The localStorage getter | classify `localStorage` | Legacy | `window.rs:523` / getter `:530` → **A2** | yes | yes |
| WHATWG HTML §12.2.4 The StorageEvent interface | classify `StorageEvent` global (Codex P2) | Legacy | `globals.rs:656` `register_globals` → **A2** | ✓ | no (ctor) |
| WHATWG HTML §3.1.4 Resource metadata management | classify `document.cookie` | Legacy | `document.rs:1108` (getter `:600` / setter `:646`) → **A3** | ✓ (cookie attr) | yes (setter value) |
| WHATWG HTML §3.1.7 DOM tree accessors | classify `getElementsByName` (live `NodeList`) | **Legacy? (open, §1.3)** | `document.rs:1029` → **B0** | ⚠ open | yes (name arg) |
| WHATWG DOM §4.5 Interface Document | classify `getElementsByTagName`/`getElementsByClassName` + `forms`/`images`/`links` (all live, Codex P2) | **Legacy? (open, §1.3)** | `document.rs:1022`/`:1026`/`:722`/`:740`/`:912` → **B0** (+ §12.1.2 amend) | ⚠ open (broader live-collection sweep → §1.3) | yes (tag/class arg) |

**Breadth**: K=2 specs (html, dom), M=8 entries → single-PR scope. This is the
design doc; the *implementation* children A1/A2/A3/B0 each re-run their own
coverage map at plan time. The full `LiveCollection`-allocation-site sweep (incl.
`Element.prototype` + `table.rows` + `form.elements` + `select.options`, §1.3) is
B0's, not enumerated row-by-row here.

### §A.1 Surface-completeness note

The **Modern** surface (the ~95% of §1 not tabled above) is not spec-enumerated
here because classification is the identity map "installed ⇒ Modern unless
§14.4.2 / §12.1.2 demotes it"; the closed question is *which APIs demote*, and
that set is the eight rows above (storage ×4 + `StorageEvent` collapse to one Web
Storage family; cookie; live-collections — the `⚠ open` rows, whose *full* site
sweep is B0's, §1.3). No Deprecated API is installed (verified absent,
§1.1–§1.2), so the Deprecated branch is empty by construction.

---

## 1. Core-VM Web API Surface Inventory + Classification

Method: `install_methods` / `install_ro_accessors` / `install_rw_accessors`
registration tables read directly. `(M)` = Modern, `(L)` = Legacy (compat-destined),
`(F)` = frozen-for-compat constant (spec-mandated; stays in every mode),
`(stub)` = installed but not yet backed (Modern target, orthogonal to core/compat).

Classification basis: design `14-script-engines-webapi.md` §14.4 (Web API
core/compat), `12-dom-cssom.md` §12.1.2 (DOM core/compat), ADR #14/#16/#17.

### 1.1 `Window` (`crates/script/elidex-js/src/vm/host/window.rs`, `register_window_prototype` :411)

| API | Site | Class | Notes |
|---|---|---|---|
| `scrollTo` / `scroll` / `scrollBy` | `WINDOW_METHODS` :469 (:470/:475/:476) | M | CSSOM-View. |
| `postMessage` | :478 | M | |
| `getComputedStyle` | :482 | M | CSSOM. |
| `getSelection` | :489 | M | Selection API. |
| `innerWidth`/`innerHeight`/`scrollX`/`scrollY`/`pageXOffset`/`pageYOffset`/`devicePixelRatio` | `WINDOW_RO_ACCESSORS` :501 (:502–:508) | M | `pageXOffset`/`pageYOffset` are spec-mandated aliases of `scrollX`/`scrollY`, **not** Legacy (HTML keeps them). |
| `self`/`parent`/`top`/`frames`/`frameElement`/`opener`/`length`/`closed` | :509–:516 | M (stub) | Browsing-context accessors; **stubs today** (see :498 comment). Modern target; their stub-ness is F4-adjacent, not a core/compat question. |
| `name` | `WINDOW_RW_ACCESSORS` :520 | M | Only writable Window attr; backed by `VmInner::window_name`. |
| **`localStorage`** | `WINDOW_STORAGE_ACCESSORS` :522 (:523), getter :530, install :438 | **L** | Sync storage → compat (§14.4.2, ADR #16). |
| **`sessionStorage`** | :524, getter :541, install :438 | **L** | Sync storage → compat. |
| event-handler IDL attrs (`onload`, …) | install :445 (Global+Window scopes) | M | GlobalEventHandlers + WindowEventHandlers mixins. |
| **`StorageEvent`** (global/ctor) | `globals.rs:656` `register_globals` (unconditional); slot `#11-storage-web` | **L** | **Added (Codex P2):** HTML §12.2.4 — part of the Web Storage API surface; must be gated **with** `Storage` (else a `BrowserCore`/`App` session observes `typeof StorageEvent === 'function'` while Web Storage is absent). In-code citation `§11.4.2` (`mod.rs:581`, `object_kind.rs:855`) is **stale** → §12.2.4; fold into §1.5 sweep. Installed via a **direct global**, not a table (see §3.2 F4). |

Not present (verified absent): `alert`/`confirm`/`prompt`, `XMLHttpRequest`.
`fetch` is a **bare global**, not a `Window` method (`fetch/mod.rs:91`,
`register_fetch_global`) — Modern, out of this inventory's table by shape.

### 1.2 `Document` (`crates/script/elidex-js/src/vm/host/document.rs`, registration :983)

| API | Site | Class | Notes |
|---|---|---|---|
| `getElementById` | `DOCUMENT_METHODS` :1017 (:1018) | M | ID component fast-path. |
| `querySelector` / `querySelectorAll` | :1019/:1020 | M | Static `NodeList`. Primary query API. |
| `getElementsByTagName` | :1022 | **L?** | Returns **live** `HTMLCollection`. §12.1.2 only explicitly demotes `getElementsByClassName`; this sibling needs a design call (§1.3). |
| **`getElementsByClassName`** | :1026 | **L** | Live `HTMLCollection` — design §12.1.2 marks it ✗core/✓compat. |
| `getElementsByName` | :1029 | **L?** | Live collection sibling; same open call as `getElementsByTagName`. |
| `createElement` / `createTextNode` / `createComment` / `createDocumentFragment` | :1030–:1036 | M | |
| `hasFocus` | :1043 | M | Focus-management reader (§6.6.6). |
| `getSelection` | :1047 | M | Mirrors Window binding (same singleton). |
| traversal factories (`createTreeWalker`/`createNodeIterator`/`createRange`) | `document_traversal::FACTORIES` install :987 | M | |
| `documentElement`/`head`/`body` | `DOCUMENT_RO_ACCESSORS` :1050 | M | |
| `firstElementChild`/`lastElementChild`/`children`/`childElementCount` | :1056–:1068 | M | ParentNode mixin. |
| `URL`/`documentURI`/`baseURI`/`compatMode`/`defaultView`/`doctype` | :1069–:1075 | M | |
| `readyState` | :1072 | M (stub) | Returns `"complete"` (no real lifecycle yet). |
| `referrer` | :1078 | M (stub) | |
| `forms`/`images`/`links` | :1079–:1081, getters :722/:740/:912 | **L?** | **Correction (Codex P2):** these allocate **live** `LiveCollection` (`document.rs:730`/`:748`/`:920`), **not** snapshot arrays. The in-code comment "snapshot arrays" (`document.rs:591`) is itself stale → fold into the §1.5 clerical sweep. Live-collection family → open call (§1.3), routed to B0. |
| `activeElement`/`hidden`/`visibilityState` | :1084–:1089 | M | |
| `styleSheets` | :1093 | M | CSSOM §6.8. |
| `title` | `DOCUMENT_RW_ACCESSORS` :1101 (:1102) | M | |
| **`cookie`** | :1108, getter :600, setter :646 | **L** | Sync string cookie API → compat (§14.4.2, ADR #16). |

Not present (verified absent): `document.write`/`writeln`/`open`/`close`/`all`,
`execCommand`. The `document.all` exclusion (design §12.1.2, Phase-0 survey 0%)
holds.

### 1.3 Open classification call: the live-collection family

Design §12.1.2 lists **only** `getElementsByClassName` as Legacy/compat. But
`getElementsByTagName`/`getElementsByClassName` return live `HTMLCollection`s
(DOM LS §4.5 Interface `Document`), and `getElementsByName` returns a live
`NodeList` (**HTML §3.1.7 DOM tree accessors**, not DOM — a split spec-home that
B0 must account for when deciding (α)/(β)). All three are "live collection"
shaped. Two consistent positions:

- **(α) Demote all three** as the "live collection" Legacy family (matches the
  §12.1.2 *rationale* — "live collections" — even though the table names one).
- **(β) Demote only `getElementsByClassName`** (literal §12.1.2) and treat the
  other two as Modern-retained DOM methods.

**Recommendation: (α), pending a design-doc amendment.** The §12.1.2 rationale is
"ライブコレクション" (live collection), which is exactly what all three are; the
*One issue, one way* lens says the gate should treat the family uniformly rather
than carve one member. **But** this is a DOM-API (Program B / §12 territory)
classification, not storage/cookie — so A0 **surfaces** it and routes the
decision to **B0** (the mutation/DOM audit) + a §12.1.2 amendment, rather than
folding it into A1's storage/cookie scope. A1's *mechanism* must nonetheless be
able to express it (see §3.4).

**Scope correction (Codex P2): the live-collection surface is broader than the
three `Document` getters.** This inventory is scoped to `Window`/`Document`/
`navigator` (the audit's F1/F2 surface), but `LiveCollection` is allocated at
many **more** sites that B0 must enumerate before demoting the family — otherwise
`BrowserCore` still exposes live collections after the "family" is demoted:
`Document.forms`/`images`/`links` (§1.2, corrected); `Element.prototype`
`getElementsByTagName`/`getElementsByClassName` (`element_proto.rs`);
`form.elements` (`html_form_proto.rs`); `table.rows`/`tBodies` +
`tableSection.rows` (`html_table_proto.rs` / `html_table_section_proto.rs`);
`select.options` (`html_options_collection.rs`); `map.areas`
(`html_map_proto.rs`); `node.childNodes`. **B0's acceptance criterion must be
"every `alloc_collection(LiveCollection::new(...))` site is classified,"** not
just the three named getters. A0 records the surface; B0 owns the full sweep.

### 1.4 `navigator` (`crates/script/elidex-js/src/vm/host/navigator.rs`, `register_navigator_global` :32)

| API | Site | Class | Notes |
|---|---|---|---|
| `userAgent` | :49 | M | Real UA string. |
| `appName`/`appVersion`/`product`/`productSub`/`vendor`/`vendorSub` | :50–:55 | **(F)** | **Frozen-for-compat constants.** HTML mandates these hard-coded historical values (`"Netscape"`, `"Gecko"`, `"20030107"`, …) in *every* modern browser. They are neither removable Legacy nor a clean-core violation — they are spec-required compat constants. **Keep as-is**, document the (F) bucket. |
| `platform`/`language` | :56/:57 | M (F-adjacent) | `platform` is OS-derived; HTML treats it as a near-frozen hint. |
| `onLine` | :77 | M | |
| `cookieEnabled` | :78 | M **(A3-coupled)** | **Correction (Codex P2):** hard-coded `false` today. But A3 makes `document.cookie` work in `BrowserCompat` (a real `CookieJar` is installed in the shell/content path), so leaving `cookieEnabled=false` is **inconsistent** — pages that gate cookie use on `navigator.cookieEnabled` would skip cookies while the compat API works. **A3 must flip/policy-gate `cookieEnabled` together with `document.cookie`** (not just fix its stale comment). Added to A3's AC (§5). |
| `javaEnabled` | :79 | M (F) | Spec-frozen no-op returning `false` (HTML keeps `navigator.javaEnabled()` as a hard `false`). Harmless constant. |
| `hardwareConcurrency` | :92 | M | |
| `languages` (Array) | :107 | M | |
| `serviceWorker` (conditional) | :120 (block :115) | M | SW §3.4. |

No methods; `clipboard`/`storage`/`permissions`/`mediaDevices` not present.

**`navigator` takeaway:** nothing on `navigator` is *Legacy-destined-for-compat*.
The UA-compat fields are the **(F)** bucket — record it so a future "spec-faithful
navigator" review does not mistake them for a core/compat violation. (The audit
did not flag navigator; this is a pre-emptive clarification.)

### 1.5 Newly-found clerical drift (fold into the F2 sweep)

Several stale-comment classes, all reviewer-misleading, all in F1/F2 files:

- **Stale "stub" comments** (already in audit F2): `document.rs:1098-1100`
  ("`cookie` is currently a stub whose setter silently drops writes") and
  `navigator.rs:72-75` (same "silently drop" claim) — contradicted by the **real**
  setter at `document.rs:646` (forwards to `CookieJar::set_cookie_from_script`,
  `:682`).
- **Stale "snapshot arrays" comment** (newly found, Codex P2): `document.rs:591`
  ("forms / images / links (snapshot arrays)") — contradicted by the **real**
  getters that allocate `LiveCollection` (`document.rs:730`/`:748`/`:920`).
- **Stale spec-section citations** (newly found here): `window.rs:434` cites
  storage as "WHATWG HTML §11.2"; the cookie code cites "WHATWG §6.5.2"; the
  `StorageEvent` code cites "WHATWG HTML §11.4.2" (`mod.rs:581`,
  `object_kind.rs:855`). All drifted. Verified-correct anchors (webref, §7):
  storage = **HTML §12.2** ("The API"), Storage interface §12.2.1,
  `sessionStorage` getter §12.2.2, `localStorage` getter §12.2.3, **`StorageEvent`
  = §12.2.4**; `document.cookie` = **HTML §3.1.4** ("Resource metadata
  management", `#dom-document-cookie`).

Both classes are pure Axis-3/Axis-4 docstring corrections, separable from the
migration. **Recommendation:** fold the citation fix into the same clerical micro-PR
as the stale-stub fix (or into A2/A3 if not landed first) — a single
comment-only sweep over `window.rs`/`document.rs`/`navigator.rs`.

---

## 2. Mechanism State — what exists vs. what is missing (sharpens umbrella §2.7)

The umbrella §2.7 said "there is **no** `WebApiSpecLevel`/`DomSpecLevel` runtime
gate **in the VM**." Re-grep refines this: the **classification vocabulary
exists**; the **carrier, enforcement, and mode selector do not**. The precise
layering:

| Layer | Exists? | Evidence |
|---|---|---|
| **Classification enums** `WebApiSpecLevel {Modern,Legacy,Deprecated}` / `DomSpecLevel {Living,Legacy,Deprecated}` | **yes** | `crates/core/elidex-plugin/src/spec_level.rs:68` / `:25` (`#[non_exhaustive]`, `Default`). |
| **DOM carrier trait** `DomApiHandler::spec_level()` (default `Living`) | **yes** | `crates/script/elidex-script-session/src/dom_api.rs` + `macros.rs:21`; dispatched via `PluginRegistry<dyn DomApiHandler>` through `invoke_dom_api` (`dom_bridge.rs:475`). |
| **Web-API carrier trait** (a `WebApiHandler` carrying `WebApiSpecLevel`) | **no** | `WebApiSpecLevel` is carried **only** by `NetworkMiddleware::spec_level()` (`traits.rs:271`). Storage/cookie/navigator have **no** handler trait — they are static VM tables (`install_methods`/`install_ro_accessors`). |
| **Enforcement** (registry/installer excludes a level) | **no** | `PluginRegistry::resolve` (`registry.rs:32`) is a pure name→handler map; it never reads `spec_level`. Nothing prunes `Legacy`/`Deprecated`. The static `install_*` tables never consult any level at all. |
| **Mode selector** (`elidex-app` / `elidex-browser`, a `Mode` enum, a compat flag) | **no** | Repo-wide grep negative. The only `cfg` near the VM surface is the whole-module `#![cfg(feature = "engine")]`. |
| **Compat crates** `elidex-api-storage-compat` / `-cookies-compat` / `-xhr` | **no** | `crates/api/` = `canvas`/`crypto`/`fetch`/`observers`/`sw`/`workers`/`ws`/`cache-api` only. |
| **Async core equivalents** (`elidex.storage` JS global / `cookieStore` / CookieStore impl) | **no** | No JS-visible `elidex.storage`/`cookieStore` (grep negative). Design §14.4.3's `AsyncStorage` trait is **unimplemented** (no `AsyncStorage` in `elidex-storage-core`). |
| **Backends the compat shims need** | **yes** | `WebStorageManager` (sync `local_get`/`local_set`, `web_storage.rs:202`) + `SessionStorageState` (`:501`); `CookieJar` (`cookies_for_script` `:349` / `set_cookie_from_script` `:400`, `cookie_jar.rs:79`). |

**Structural root (the real F1/F2 blocker):** legacy Web APIs are installed
through **static VM registration tables that bypass the spec-level vocabulary
entirely**. There is no seam at which a mode could say "do not install Legacy."
So F1/F2 are not "move X" — they are "**introduce the carrier + enforcement +
mode selector first, then re-route the static tables through it**." That
construction is A1; A2/A3 are then genuinely a re-route.

**Second binding (boa engine, S5-cohort — light-touch):** the legacy boa engine
carries a **parallel** storage/cookie surface — `elidex-js-boa/src/globals/storage.rs`
(`localStorage`/`sessionStorage`), `globals/document/mod.rs:711`/`:731`
(`document.cookie`), and even a `globals/cookie_store.rs` (CookieStore). It is
**delete-destined** (S5 / D-26 PR7 boa removal) per the boa-findings-light-touch
policy, so A2/A3 do **not** separately gate it — it disappears with boa. Recorded
here only so A2/A3 plan-memos treat the elidex-js VM path as the **one surviving**
binding rather than assuming it is the only path that exists today.

---

## 3. Design Decision — the gate / mode mechanism

### 3.1 Requirements (derived from the philosophy lenses)

1. **Whole-engine consistency (Axis 1b):** the *same* core/compat/deprecated
   pattern HTML/CSS/ES already use must govern Web/DOM APIs — not a storage-only
   bolt-on.
2. **One issue, one way:** a single gate covering storage **and** cookie **and**
   live collections **and** any future legacy API — no "new seam + N legacy
   tables" coexistence.
3. **Plugin-first:** legacy Web APIs should resolve through the same
   static-enum-dispatch + spec-level model as `DomApiHandler`/`NetworkMiddleware`,
   not a VM-local `if mode == compat` branch.
4. **Ideal over pragmatic:** design for the `elidex-app` ↔ `elidex-browser`
   dual-mode (ADR #9/#16) even though `elidex-app` does not exist yet — the
   mechanism must already express "app mode excludes this."
5. **No half-built strangler:** if a mode switch is introduced, it must be the
   single mode authority, not one of two.

### 3.2 The recommendation: classification-at-registration + one runtime policy + a compile-time hard-exclude

Two cooperating parts, with a clear division of labor:

**(a) Per-API classification attached at *every* registration seam.** Extend the
VM's registration so every installed Web/DOM API carries a
`WebApiSpecLevel`/`DomSpecLevel` (reusing the existing enums), defaulting to
`Modern`/`Living` (mechanical, near-noise for the ~95% Modern surface). This is
the **vocabulary made load-bearing** rather than decorative. There are **three**
seams the gate must cover (Codex P2 — the table-only form is incomplete):

  1. **`install_methods`/`install_ro_accessors`/`install_rw_accessors` table
     entries** — the `Window`/`Document` method/accessor surface (incl. the
     `WINDOW_STORAGE_ACCESSORS` storage pair and the static `DOCUMENT_METHODS`
     live-collection getters).
  2. **Direct `register_*_global()` installers** — many top-level Web APIs are
     installed by a flat sequence of `register_*_global()` calls in
     `vm/globals.rs` `register_globals` (e.g. `register_storage_global:483`,
     `StorageEvent` `:656`, `register_websocket_global`, `register_event_source_global`,
     crypto, and any **future `XMLHttpRequest`-shaped global**). A table-only gate
     would leave these outside the policy — contradicting "one gate for any future
     legacy API." The policy must apply to `register_*_global` too (e.g. each takes
     / is wrapped by a level, and the installer no-ops it when excluded).
  3. **Live-collection getters that allocate directly** — see §3.4: these are in
     the static `DOCUMENT_METHODS` table **and** allocate `LiveCollection` in
     `vm/host/document.rs`, *not* through the `DomApiHandler` registry, so they are
     gated at seam (1), not via registry pruning.

The gate is therefore "**a level at every install seam + one policy consulted by
every installer**," not "a level on one table."

**(b) One runtime `SpecLevelPolicy`, resolved once at VM/session construction.**
The embedder (shell vs. the future app host) supplies a **mode**; the VM
translates it into a policy that the installer consults:

```
enum WebApiMode { BrowserCompat, BrowserCore, App }   // embedder-supplied
// BrowserCompat → install Modern + Legacy (current behavior)
// BrowserCore   → install Modern only  (⚠ not selectable for a real session until
//                 the async core lands — a core session needs elidex.storage, §4.2)
// App           → install Modern only; Legacy compile-excluded (⚠ same async-core
//                 precondition for storage, §4.2)
```

The installer skips entries whose level the policy excludes. This is the single
mode authority (requirement 5); it generalizes to **every** legacy API in §1
(requirement 2); and it routes through the registration seam rather than scattered
branches (requirement 3).

**(c) A compile-time `feature = "compat-webapi"` for the hard `app` build.** A
runtime policy still *links* the compat shim code. The `elidex-app` build (ADR
§14.4.3: "コンパイル時除外") must be able to ship **without the sync-storage shim in
the binary at all**. So the compat shims live behind a cargo feature; an `app`
profile builds with it off. Runtime mode handles the browser's
core-vs-compat toggle; the feature handles the app's *absence* guarantee. They are
not redundant — they answer different questions ("is it reachable now?" vs. "is it
in the binary?").

**Where each piece lives:**

- The **enums** stay in `elidex-plugin` (already correct home).
- The **`WebApiMode`/`SpecLevelPolicy`** type lives in `elidex-plugin` (alongside
  the enums) so every layer can name it; the **policy value** is threaded into the
  VM at `bind_session`/VM construction from the embedder (shell supplies
  `BrowserCompat` today — zero behavior change).
- The **classification-at-registration** change lives in the VM host registration
  plumbing (`elidex-js`), since that is where `install_*` lives.
- The **compat shims** live in new `elidex-api-storage-compat` / `-cookies-compat`
  crates (§4), behind `feature = "compat-webapi"`.

### 3.3 Alternatives considered (and why rejected)

- **Compile-time feature only.** Rejected: a browser that ships compat still needs
  a *runtime* core-vs-compat distinction (a page in "core mode" shouldn't see
  `localStorage` even though the shim is linked). A pure `cfg` cannot express
  per-session mode. (Keep `cfg` only for the app's *absence* guarantee.)
- **Runtime policy only.** Rejected: cannot give `elidex-app` the "not in the
  binary" guarantee ADR §14.4.3 promises; the shim code would always link.
- **A `WebApiHandler` trait mirroring `DomApiHandler`, with full registry
  dispatch for storage/cookie.** Tempting for symmetry, but storage/cookie are
  *thin host bindings over a backend*, not algorithmic DOM methods; forcing them
  through a `JsValue`-marshalling registry adds a layer without removing one. The
  **lighter** "level on the registration entry + policy at install" achieves the
  gate without a new dispatch trait. **Flagged for plan-review (§8 Q1):** confirm
  the level-on-registration form is preferred over a full `WebApiHandler` trait.
- **Storage-only conditional (`if jar_bound { … }`-style).** Rejected outright —
  the exact "new seam + N legacy" decision-surface anti-pattern (One issue, one
  way).

### 3.4 The mechanism must also be able to express the live-collection family

Because §1.3 may demote the live-collection getters to `Legacy`, the gate must
reach them too. **Correction (Codex P2):** the original draft routed this through
the `DomApiHandler` registry — that is **wrong** for these methods.
`getElementsByTagName`/`getElementsByClassName`/`getElementsByName` are installed
from the **static `DOCUMENT_METHODS` table** and allocate `LiveCollection`
**directly** in `vm/host/document.rs` (`:211`/`:240`/`:263`), *not* through
`invoke_dom_api`/`DomApiHandler`. So pruning the `DomApiHandler` registry would
**not** gate them — in `BrowserCore` the JS properties would still install and
bypass the policy. Their gate is therefore at **seam (1)** (the static table,
§3.2a), exactly like the storage accessors — which reinforces the "gate at every
registration seam" thesis rather than weakening it.

`DomApiHandler::spec_level()` enforcement (pruning `Legacy` handlers at
resolve/install) is still worth doing for the methods that *are* genuinely
bridge-dispatched (`setAttribute`/`getAttribute` go through `invoke_dom_api`), so
A1's enforcement is written against **both** the install seams (1)+(2) **and** the
`DomApiHandler` registry — but the live-collection getters belong to the
*table-seam* half, not the registry half. The actual *demotion* + the full
`LiveCollection`-site sweep stay a B0/§12.1.2 decision (§1.3 scope correction);
A1 only makes the gate capable of it.

---

## 4. Compat Placement + Async Core Equivalents — ownership

### 4.1 Sync compat shims (this program builds these)

Per design §14.4.2:

**Correction (Codex P2) — what actually moves vs. what stays.** The original draft
said the compat crates "house the `localStorage`/`sessionStorage` thin VM binding."
That is **not buildable**: the VM-native glue (`vm/host/storage.rs`, the cookie
getter/setter) imports **private `elidex-js` VM types** (`NativeContext`,
`ObjectId`, `ObjectKind`, `VmInner`), and `elidex-js` is the crate that would
*depend on* the new compat crates — so relocating the glue creates a **dependency
cycle** (or forces exposing VM internals). Therefore:

| Layer | Where it lives | Gated by |
|---|---|---|
| Backend (quota / persistence / origin registry / cookie store) | **already exists** — `elidex-storage-core` (`WebStorageManager`/`SessionStorageState`), `elidex-net` (`CookieJar`) | n/a |
| Optional compat shim/integration (if any beyond the backend) | new `elidex-api-storage-compat` / `-cookies-compat` | `feature = "compat-webapi"` |
| **VM-native JS glue** (`install_*` / `register_*_global` / `NativeContext`-using natives) | **stays in `elidex-js`** | `feature = "compat-webapi"` + runtime `SpecLevelPolicy` |

So "move behind compat" = **gate the native glue in place** (feature + policy),
**not** relocate it. The new compat crate(s) hold only what can sit *below*
`elidex-js` without a cycle — and since the backends already live in
`elidex-storage-core`/`elidex-net`, A1 plan-review (§8 Q1) should confirm whether
a *separate* `-storage-compat`/`-cookies-compat` crate is even needed or whether
the design §14.4.2 "compat shim" role is already filled by the existing backend
crate + the gated in-`elidex-js` glue. The layering mandate (`storage.rs:1` "VM
thin binding"; algorithm in `elidex-storage-core`) is preserved either way.

### 4.2 Async core equivalents (this program does **not** build these)

| Core API | Spec | Status today | Owner |
|---|---|---|---|
| `elidex.storage` (async KV) | design §14.4.3 `AsyncStorage` trait | **unimplemented** (no JS global, no trait impl) | **separate future PR** (precondition for `app` mode) |
| `cookieStore` / CookieStore | Cookie Store API Standard (`cookiestore`) | **unimplemented** | **separate future PR** (`elidex-api-cookies`, design §14.4.1 P1) |

**Defer slot (so this is ledger-tracked, not prose-tracked — avoids the exact
F4 "untracked narrative" anti-pattern the umbrella criticizes):** register
**`#11-async-core-storage-cookiestore`** at A1 landing.
- *Why deferred:* the async core is a larger build (a new `AsyncStorage`-backed
  `elidex.storage` JS global + a CookieStore implementation in `elidex-api-cookies`),
  orthogonal to moving the *existing* sync shims behind the gate (A2/A3). Building
  it is not a precondition for the **browser-mode** compat split.
- *Re-evaluation trigger:* the moment an `elidex-app` mode (or any `App`-mode
  storage) is introduced — at which point sync storage is compile-excluded
  (§14.4.3) and the async core becomes the *only* storage API, so it MUST exist.
- *Re-evaluation date:* revisit when the `world_id` / S5-boa-removal program opens
  the dual-mode work (MEMORY.md Active state), or sooner if app-mode storage is
  scheduled.

**The ordering decision (important) — corrected (Codex P2).** §14.4.3's mode
table says **both** `elidex-browser（コア）` **and** `elidex-app` have sync storage
`利用不可` but async `elidex.storage` `利用可能` — i.e. a *core* session is
contracted to have `elidex.storage`, **not no storage**. So hiding `localStorage`
in `BrowserCore` **without** the async core would make `BrowserCore` violate the
mode contract (a core session with zero storage API), even though it would *pass*
A1's gate tests. The async core is therefore a precondition for **`BrowserCore`
too**, not only `App`. Resolution:

- `elidex-app` and a selectable `BrowserCore` storage mode **do not exist yet**, so
  nothing is stranded today. A2/A3 target the **`BrowserCompat`** mode only: keep
  `localStorage`/`cookie` reachable via the compat boundary; A2/A3 introduce the
  *classification + gate plumbing* and the `compat-webapi` feature, but **do not
  flip any production session to `BrowserCore`/`App`**. The gate's
  `BrowserCore`/`App` *exclusion* is exercised only by unit tests (a marked-`Legacy`
  test API), never by a real session, until the async core lands.
- The async core (`elidex.storage`/CookieStore, slot
  `#11-async-core-storage-cookiestore`) is the **hard precondition for selecting
  `BrowserCore` *or* `App` storage** in production. A1's mode-enum docs must state:
  *"`BrowserCore`/`App` must not be selected for a real session until
  `#11-async-core-storage-cookiestore` lands — else the session has no storage
  API, violating design §14.4.3."* A0 does **not** schedule the async core into
  Program A; it pins the dependency so the gate can't silently ship a
  contract-violating mode.

### 4.3 Side-store → component lens (pre-answer for Axis 2)

The compat backends are **shared cross-cutting state**, not per-entity:
`WebStorageManager` is `Arc`-shared and origin-keyed; `CookieJar` is
session/browsing-context-level. They are exception **(b)** ("shared cross-cutting
state — cookie jar / NetworkHandle 等") in the CLAUDE.md side-store→component rule
— correctly **not** ECS components. No new per-entity `Send+Sync` side-store is
introduced by the compat split. (Recorded so plan-review Axis 2 is pre-cleared.)

---

## 5. Subsequent PR List (the A0 deliverable)

Legend: **PR-R** = `/elidex-plan-review` required before implementation.
AC = gate conditions.

### Program A (this design's children)

| PR | Purpose | Main files / crates | Depends | Plan-review | Acceptance criteria |
|---|---|---|---|---|---|
| **A1** | Build the **gate**: (a) level at **every install seam** — `install_*` tables **and** direct `register_*_global()` installers (§3.2a, Codex P2); (b) `WebApiMode`/`SpecLevelPolicy` in `elidex-plugin`, threaded into VM construction; (c) enforcement that prunes excluded levels at every installer — covering the static tables, the global installers, **and** the `DomApiHandler` registry; (d) `feature = "compat-webapi"`. **No API moves yet; no behavior change** (shell supplies `BrowserCompat`). | `elidex-plugin` (policy type), `elidex-js` VM host registration plumbing incl. `vm/globals.rs`, `elidex-script-session` (registry enforcement) | A0 | **PR-R** | Policy classifies+conditionally-installs by level; `localStorage`/`cookie` still installed under `BrowserCompat`; unit tests show `BrowserCore`/`App` excludes a marked-`Legacy` API installed via **both** a table entry **and** a `register_*_global`; gate proven against a `DomApiHandler` too; `compat-webapi` compiles on/off. **`BrowserCore`/`App` exercised only by tests, never a real session (§4.2 async-core precondition).** |
| **A2** | Gate `Storage` (`localStorage`/`sessionStorage`) **+ `StorageEvent`** (Codex P2) behind the policy + `compat-webapi` feature, **in place** in `elidex-js` (native glue does not relocate — §4.1 dependency-cycle); classify them `Legacy`. | `vm/host/storage.rs`, `window.rs`, `vm/globals.rs` (StorageEvent), opt. new `elidex-api-storage-compat` (§4.1 — confirm need) | A1 | **PR-R** | Sync storage **and** `StorageEvent` reachable only under `BrowserCompat`; both absent together in `BrowserCore`/`App` (no `typeof StorageEvent==='function'` while storage absent); opaque-origin slot `#11-storage-opaque-origin-securityerror` re-evaluated; `BrowserCompat` byte-identical; tests green. **Sequence after JS-side media Slice 2b** (window.rs collision — §6). |
| **A3** | Gate `document.cookie` **+ `navigator.cookieEnabled`** (Codex P2) behind the policy + feature, in place in `elidex-js`; classify cookie `Legacy`; **flip/policy-gate `cookieEnabled`** to match cookie reachability. Fold the F2 clerical fix (stale stub + stale citation, §1.5). | `vm/host/document.rs`, `vm/host/navigator.rs`, opt. new `elidex-api-cookies-compat` (§4.1 — confirm need) | A1 (∥ A2) | **PR-R** | `document.cookie` reachable only under `BrowserCompat`; `navigator.cookieEnabled` reports `true` exactly when `document.cookie` is reachable **and** a `CookieJar` is bound (no `cookieEnabled=false` while the compat API works); stale comments + citations removed; tests green. |

`elidex.storage` / CookieStore async core = **out of Program A** (§4.2, slot
`#11-async-core-storage-cookiestore`), the **`BrowserCore`/`App` storage
precondition** (not just `App`) for a later PR.

### Adjusted B/C/D/E (carried from umbrella, refined)

- **B0/B1/B2 (F3)** — unchanged scope; **but** B0 now also owns the
  live-collection classification call (§1.3) + the §12.1.2 amendment, **with AC =
  "every `alloc_collection(LiveCollection::new(...))` site is classified"** (incl.
  `forms`/`images`/`links`, `Element.prototype` getters, `table.rows`,
  `form.elements`, `select.options` — Codex P2), not just the named `Document`
  getters. B1's enforcement reuses **A1's gate** for any `Legacy` DOM method
  (don't build a second gate). Cross-reference A1 ⇄ B1.
- **C0 (F4)** — unchanged (decide remove-vs-slot for iframe `contentDocument`/
  `contentWindow`; the (M)(stub) Window browsing-context accessors §1.1 are the
  same family — note the linkage).
- **D0 (F5)** — unchanged (plugin-metadata tag dispatch investigation).
- **E0 (F6)** — **sharpened:** E0's "defer until a mode mechanism lands"
  recommendation now has a concrete target — **A1's `WebApiMode`**. E0 should
  recommend the shell pipeline take its compat-vs-core choice from the *same* mode
  authority A1 introduces, not a separate style-only flag. Cross-reference E0 ⇄ A1.
- **F2 clerical micro-PR** — now covers **two** comment classes (stub + citation,
  §1.5); land any time `document.rs` is quiet, or fold into A3.

### 5.1 Dependency graph

```
A0 (this doc) ──► A1 ──► A2  (storage; after JS-side Slice 2b, window.rs)
                   ├──► A3  (cookie; folds F2 clerical)
                   └──(gate reused)──► B1   (F3 enforcement)
B0 ──► B1 ──► B2          (B0 also owns §1.3 live-collection call + §12.1.2 amend)
C0   (independent, cheap)
D0   (independent, investigate)
E0   (recommend: consume A1's WebApiMode)
F2 clerical (stub + citation) — independent micro-PR, or folded into A3
```

---

## 6. Collision / Coordination (re-confirmed at HEAD `2f4a9d5a`)

- **`window.rs` (HIGH): A2 vs JS-side media Slice 2b.** Both edit
  `crates/script/elidex-js/src/vm/host/window.rs` (Slice 2b adds `matchMedia`; A2
  moves the storage accessors). **Do not open A2 while Slice 2b is open** → let
  Slice 2b land, rebase, then A2. **A0/A1 are doc / non-window.rs** (A1's host
  changes are in registration plumbing, not the storage table specifically — but
  A1 still touches `elidex-js`; confirm at open-time whether Slice 2b is in
  flight).
- **`document.rs` (LOW): A3 + F2 clerical vs JS-side.** Not on the Slice 2b path;
  confirm at open-time.
- **iframe (LOW): C0 vs HTML-side focus A2b.** A2b touches iframe *focus*, not
  `contentDocument`/`contentWindow`; C0 is comment/slot only.
- **Worktree isolation:** every code-touching child PR builds in a dedicated
  worktree off `origin/main`. This A0 PR is doc-only on `docs/plans/`.

---

## 7. Citation Appendix (webref-verified, `.claude/tools/webref`)

| Concept | §number → title | Anchor |
|---|---|---|
| Web storage chapter | HTML §12.2 — *The API* | `#storage` |
| `Storage` interface | HTML §12.2.1 — *The Storage interface* | `#the-storage-interface` |
| `sessionStorage` getter | HTML §12.2.2 — *The sessionStorage getter* | `#dom-sessionstorage` |
| `localStorage` getter | HTML §12.2.3 — *The localStorage getter* | `#dom-localstorage` |
| `StorageEvent` interface | HTML §12.2.4 — *The StorageEvent interface* | `#the-storageevent-interface` |
| `document.cookie` attribute | HTML §3.1.4 — *Resource metadata management* | `#dom-document-cookie` |
| `getElementsByClassName` (live) | DOM LS §4.5 — *Interface Document* | `#dom-document-getelementsbyclassname` |
| CookieStore (core equivalent) | Cookie Store API Standard | shortname `cookiestore` |

Opaque-origin `SecurityError` for storage: the getter algorithms HTML §12.2.2 /
§12.2.3 throw `SecurityError` for opaque origins (consistent with
`storage.rs:82`'s deviation doc + slot `#11-storage-opaque-origin-securityerror`).

> Stale in-code citations to fix (§1.5): `window.rs:434` "§11.2" → §12.2;
> cookie "§6.5.2" → §3.1.4; `StorageEvent` "§11.4.2" (`mod.rs:581`,
> `object_kind.rs:855`) → §12.2.4. Plus the stale `document.rs:591` "snapshot
> arrays" comment (forms/images/links are live). (Code comments, not this doc —
> folded into the F2 sweep.)

---

## 8. Open Questions for `/elidex-plan-review`

1. **Mechanism form (§3.2/§3.3):** is "spec-level on the registration entry +
   `SpecLevelPolicy` at install" preferred over a full `WebApiHandler` dispatch
   trait mirroring `DomApiHandler`? (A0 recommends the lighter form for thin host
   bindings; confirm it is not under-modelling the plugin-first lens.)
2. **Mode home (§3.2):** does `WebApiMode`/`SpecLevelPolicy` belong in
   `elidex-plugin` next to the enums, or in a VM/session-construction crate? Is
   threading it through `bind_session`/VM construction the right seam?
3. **Compile-time + runtime duo (§3.2c):** is having *both* `feature =
   "compat-webapi"` and a runtime `WebApiMode` the right division (binary-absence
   vs. per-session reachability), or does it create two overlapping authorities
   (strangler risk)?
4. **Live-collection demotion (§1.3):** (α) demote all three live-collection
   getters vs. (β) only `getElementsByClassName`. A0 recommends (α) + a §12.1.2
   amendment, routed to B0 — confirm the routing and that A1's gate must be able
   to express it.
5. **Async-core ordering (§4.2):** is it acceptable that A2/A3 ship the
   browser-mode compat split **before** `elidex.storage`/CookieStore exist, with
   `App`-mode storage documented as blocked on a later PR — or must the async core
   land first?
6. **Navigator (F) bucket (§1.4):** confirm the frozen UA-compat constants are
   correctly treated as "keep, document as (F)" rather than Legacy-destined.
7. **Re-check discipline:** this doc is a 2026-06-20 snapshot — A1/A2/A3
   plan-memos must re-grep §1/§2 file:lines and re-confirm Slice 2b's branch state
   before implementation (Axis 5).

# A3 — `document.cookie` Demotion (plan-memo)

Plan date: 2026-06-21 JST
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → Program A,
PR A3. Parent design (SSoT): A0 = `docs/plans/2026-06-web-api-compat-split-design.md`
(§5 row **A3**, §1.2/§1.4). Builds on A1 (gate mechanism, #376) and A2 (Web Storage
demotion, #390, `aa49ce51`) — A3 is the **∥-to-A2 cookie sibling**: same one-source
flip + `compat-webapi` glue-gate pattern, **minus** A2's realm/variant/crate-feature
facets, **plus** a `navigator.cookieEnabled` value-derivation.

> **Edge-dense → `/elidex-plan-review` REQUIRED** (A0 §5 marks A3 **PR-R**). Base-case
> slice under the approved A0 umbrella. Every file:line re-grepped at HEAD `aa49ce51`
> (post-A2). Citations webref-verified (§8).

---

## §0. Premise-correction (vs A0 snapshot; at HEAD `aa49ce51`)

1. **A1 pre-wired the cookie install seam.** `document.rs:1017`
   `if self.installs(document_cookie_spec_level()) { install_rw_accessors(doc_wrapper,
   DOCUMENT_COOKIE_RW_ACCESSOR) }`. `document_cookie_spec_level()` =
   `event_handler_consumer.rs:253` → returns `Modern`. ⟹ A3's exclusion is a
   **one-source flip** Modern→Legacy. (Mirrors A2's `web_storage_spec_level` flip.)

2. **`document.cookie` has NO realm-scope facet** (unlike A2's `Storage`/`StorageEvent`
   globals). It is a **string RW accessor on `Document`** (`DOCUMENT_COOKIE_RW_ACCESSOR`,
   `document.rs:1176`), and `Document` exists only in the Window realm — the accessor
   installs only on the Document wrapper (`document.rs:1017`, Window-only by nature).
   No `[Exposed=Window]` worker over-exposure. **No `ObjectKind` variant** either
   (cookie is a string, not a wrapper object like `Storage`).

3. **CookieJar stays always-compiled — NO crate feature** (unlike A2's
   `elidex-storage-core/web-storage`). `HostData::cookie_jar`
   (`host_data.rs:173`, `Option<Arc<elidex_net::CookieJar>>`) is the VM handle to
   **elidex-net's HTTP-cookie infrastructure** — used for HTTP cookie handling in
   *every* mode, and (after this PR) read by `navigator.cookieEnabled`. It is a
   **shared cross-cutting exception** (CLAUDE.md side-store (b), documented at
   `host_data.rs:181`). So A3 gates only the **`document.cookie` JS glue**, never the
   CookieJar. (Within elidex-js, `cookie_jar()` is read only by the two cookie natives
   today — `document.rs:629/677` — but `cookieEnabled` will also read it, keeping it
   live in all modes.)

4. **`navigator.cookieEnabled` is a hard-coded `false` stub** (`navigator.rs:78`,
   `("cookieEnabled", false)` static `Data` property). A0 R5-2 / §1.4: A3 derives its
   *value* from cookie **handling** (a CookieJar bound), per HTML §8.10.1.5 — **NOT**
   `document.cookie` reachability. Since the jar binds *after* navigator install
   (`install_cookie_jar` at/after bind), a static install-time value can't reflect it
   → `cookieEnabled` must become a **getter** reading `cookie_jar().is_some()` at access
   time (the first accessor on `navigator`; siblings are data props). Stays Modern,
   installed in **all** modes.

5. **Out of A3 scope (defer / other PR):** the cookie-file stale comments
   (`document.rs:592` "snapshot arrays", the cookie "stub/silently-drop" comments,
   the stale `§6.5.2` citation at `document.rs:595/640`) belong to the **independent
   F2 clerical micro-PR** (A0 §1.5) — A3 must **not** fold them in (avoids the per-owner
   ownership error A0 R3-3 flagged). Cookie opaque-origin `SecurityError` (HTML §3.1.4
   requires it for sandboxed/opaque origins; the getter/setter currently silently
   `""`/no-op) → defer slot `#11-cookie-opaque-origin-securityerror` (A0 A3 row).

---

## §A. Spec coverage map (preflight hard-gate)

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §3.1.4 *Resource metadata management* | classify `document.cookie` `Legacy` | Modern→Legacy | `event_handler_consumer.rs:253` `document_cookie_spec_level()` (single source) | ✓ | yes (setter value → `CookieJar::set_cookie_from_script`) |
| HTML §3.1.4 (getter) | gate `document.cookie` getter | Legacy | `document.rs:1176` `DOCUMENT_COOKIE_RW_ACCESSOR` + `native_document_get_cookie` (`:601`) | yes | no |
| HTML §3.1.4 (setter) | gate `document.cookie` setter | Legacy | `native_document_set_cookie` (`:647`) | yes | yes |
| HTML §8.10.1.5 *Cookies* (NavigatorCookies) | derive `cookieEnabled` value | Modern (value-derive) | `navigator.rs:78` static `false` → getter `cookie_jar().is_some()` | n/a | no |

**Breadth**: K=1 spec (html), M=4 entries (verified 2026-06-21 — table rows) →
single-PR scope (base-case slice under the approved A0 umbrella).
**User-input audit**: A3 introduces **no new untrusted-input path** — the setter value
flow into `CookieJar::set_cookie_from_script` is unchanged; A3 only *gates* the
existing glue.

### §A.1 Surface-completeness
The cookie family is closed: `document.cookie` (getter+setter) is the only Legacy
member. `navigator.cookieEnabled` is **Modern** (kept, value-derived — not demoted).
The full spec-faithful `navigator` surface (other NavigatorID/Plugins fields) is the
separate `#11-navigator-spec-faithful-surface` slot — A3 touches only `cookieEnabled`.

---

## §1. Verified anchors (re-grep at impl-open)

- Single source: `event_handler_consumer.rs:253` `document_cookie_spec_level()` →
  `Modern` (test asserts at `:504`). Re-exported `lib.rs`.
- Install seam: `document.rs:1017` (gated by source) + `DOCUMENT_COOKIE_RW_ACCESSOR`
  `document.rs:1176`.
- Glue: `native_document_get_cookie` `document.rs:601` / `native_document_set_cookie`
  `document.rs:647`, both via `hd.cookie_jar()` (`:629/:677`).
- `HostData::cookie_jar` field `host_data.rs:173`, `install_cookie_jar` `:1007`,
  accessor `cookie_jar()` (used by both natives; will be read by `cookieEnabled`).
- `navigator.cookieEnabled` static `false` `navigator.rs:78` (in the `bool_fields`
  list `:76-89`, installed via `define_shaped_property` Data).
- Gate predicate `VmInner::installs` `globals.rs` (A1). `compat-webapi` feature +
  hard-ceiling (A1/A2, `init.rs`).

---

## §2. Decisions this memo commits to

1. **Exclusion = one-source flip.** `document_cookie_spec_level()` Modern→Legacy.
   Under `BrowserCompat` `document.cookie` still installs (byte-identical);
   `BrowserCore`/`App` + `compat-webapi`-off drop it.

2. **`compat-webapi` glue-gate (App absence).** cfg-gate the elidex-js `document.cookie`
   glue under `feature = "compat-webapi"`: `native_document_get_cookie` /
   `native_document_set_cookie` / `DOCUMENT_COOKIE_RW_ACCESSOR` / the `:1017-1018`
   install block. **No crate feature** — `elidex-net` + `CookieJar` stay (HTTP cookies
   + `cookieEnabled` need them in all modes). The `cookie_jar` HostData field +
   `cookie_jar()` + `install_cookie_jar` **stay always-compiled** (read by the
   always-present `cookieEnabled` getter). (Contrast A2, where the storage backend was
   Web-Storage-only → got its own `web-storage` feature.)

3. **`navigator.cookieEnabled` value-derivation.** Convert the static `false` data
   property to a **getter** returning `host.cookie_jar().is_some()` (the UA "handles
   cookies" iff a jar is bound, HTML §8.10.1.5) — independent of `document.cookie`
   exposure (so `BrowserCore`/`App` with HTTP cookies report `true` while
   `document.cookie` is hidden). Stays Modern, installed in all modes. Replace the now
   stale `navigator.rs:72-75` "deliberately false / writes silently dropped" comment
   (this *is* an A3-owned comment on the line A3 changes, not the F2 cookie-stub sweep).
   **Comment precision (plan-review F-A):** the new comment must attribute `true` to
   cookie **handling** (a `CookieJar` bound → HTTP cookies processed, §8.10.1.5) — it
   must **NOT** imply the `document.cookie` JS *write path* succeeds in `BrowserCore`/
   `App` (there the accessor is gated off entirely; only the HTTP/jar path persists).
   I.e. `cookieEnabled == true` ⇏ `document.cookie` is reachable.

4. **No realm/variant/crate-feature facet** (§0.2/§0.3) — A3 is materially narrower
   than A2 on the gating axis.

5. **ECS-native check (Axis 2).** No new component / per-entity side-store / OO
   pattern. `CookieJar` is `Arc`-shared session/browsing-context state — CLAUDE.md
   side-store exception (b), already on `HostData` (not a component); A3 adds nothing
   per-entity. The `cookieEnabled` getter reads `cookie_jar()` (shared state), not an
   ECS component. (A0 §4.3 pre-clears the cookie backend.)

---

## §3. File-level change plan

1. `event_handler_consumer.rs:253` — `document_cookie_spec_level()` Modern→Legacy;
   docstring update; flip the `:504` assertion (Legacy).
2. `document.rs` — `compat-webapi`-gate the two cookie natives + `DOCUMENT_COOKIE_RW_ACCESSOR`
   + the install block (`:1017`). The `document_cookie_spec_level` import (`:65`) → gate
   if it becomes unused when off (it's read only by the gated install block).
3. `navigator.rs` — `cookieEnabled`: remove from the static `bool_fields` list; install
   a getter (accessor) reading `host.cookie_jar().is_some()`. Update the `:72-75`
   comment. (First navigator accessor — confirm the accessor-install path exists or add
   a minimal one; **§7 Q2**.)
4. `host_data.rs` — `cookie_jar` field/methods **unchanged** (stay always-compiled).
5. No `Cargo.toml` change (no crate feature; `elidex-net` stays under `engine`).

---

## §4. Testing / Acceptance criteria

1. **One-source flip**: `BrowserCore`/`App` → `document.cookie` absent
   (`'cookie' in document` false / accessor undefined); `BrowserCompat` → present
   (parity). `compat-webapi`-off → absent in all modes (hard ceiling).
2. **`cookieEnabled` value-derived**: with a CookieJar bound → `true` in **all** modes
   (incl. `BrowserCore`/`App` where `document.cookie` is hidden); with no jar
   (cookie-averse) → `false`. Independent of `document.cookie` exposure.
3. **Both Cargo profiles compile**: `engine`+`compat-webapi` (cookie glue present) and
   `engine` alone (cookie glue absent, `CookieJar`/`cookieEnabled` intact). `clippy -D
   warnings` clean both ways.
4. `BrowserCompat` byte-identical for `document.cookie` (the only behavior change is
   `cookieEnabled` false→derived, an intended spec fix).
5. `cargo fmt` + `mise run ci` green; scoped `-p elidex-js` / `-p elidex-script-session`.

---

## §5. Collision / sequencing

- `document.rs` / `navigator.rs` — not on any active branch's hot path (A2 merged;
  media Slices touch window.rs/media; B-program touches mutation/tree). Confirm at
  open-time. `event_handler_consumer.rs` source flip is a 1-line change (low collision).
- A3 is `∥`-eligible with media/B work; no dependency on a pending Slice.
- Worktree `webapi-compat-a3-cookie` off `origin/main` (`aa49ce51`).

---

## §6. Defer slots registered by this PR

- **`#11-cookie-opaque-origin-securityerror`** (A0 A3 row, R5-8). *Why:* the cookie
  getter/setter silently return `""` / no-op for cookie-averse / opaque-origin cases,
  but HTML §3.1.4 requires a `SecurityError` for sandboxed/opaque origins. *How:* throw
  `SecurityError` from the getter/setter on an opaque/sandboxed origin. *Trigger:* when
  sandboxed-iframe opaque-origin plumbing lands (same dependency as
  `#11-storage-opaque-origin-securityerror`). *Date:* with the sandbox-origin work.
  (Already named in A0; register in `project_open-defer-slots` at landing.)

---

## §7. Open questions for `/elidex-plan-review`

1. **`cookieEnabled` getter mechanism** — converting the static `false` data property
   to an accessor is the first accessor on `navigator` (siblings are `define_shaped_property`
   Data). Confirm the cleanest install (a native getter via the existing accessor-install
   helper used elsewhere, e.g. the Window/Document RO-accessor path) vs. a navigator-local
   accessor. Is value-derivation in-scope for A3, or should A3 only flip the cookie
   source and leave `cookieEnabled` to `#11-navigator-spec-faithful-surface`? (A0 R5-2
   says A3 derives the value — recommendation: do it, it's the spec-correct pair to the
   demotion and small.)
2. **`cookieEnabled` semantics** — `cookie_jar().is_some()` as the "UA handles cookies"
   signal: correct per §8.10.1.5, or should it also consider a cookie *policy* (e.g.
   third-party-blocked)? Recommendation: `is_some()` now (policy is future); confirm.
3. **Comment ownership** — A3 rewrites the `navigator.rs:72-75` stale comment (the line
   it changes), but leaves the `document.rs` cookie-stub / `§6.5.2` / "snapshot arrays"
   comments to the F2 micro-PR (A0 §1.5). Confirm the boundary (A3 touches only its own
   changed line's comment).
4. **Re-grep discipline** — all §1 anchors re-grepped at `aa49ce51`; confirm none drift
   before impl.

---

## §8. Citation appendix (webref-verified)

| Concept | §number → title | Anchor |
|---|---|---|
| `document.cookie` attribute | HTML §3.1.4 — *Resource metadata management* | `#dom-document-cookie` |
| `navigator.cookieEnabled` (NavigatorCookies) | HTML §8.10.1.5 — *Cookies* | `#dom-navigator-cookieenabled` |

> The stale in-code cookie citation (`document.rs` "§6.5.2") is owned by the **F2
> clerical micro-PR** (A0 §1.5), **not** A3 — A3 must not "correct" it (per the A2
> precedent / A0 R3-3 ownership boundary).

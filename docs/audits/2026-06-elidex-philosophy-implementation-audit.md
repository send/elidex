# Elidex Philosophy Implementation Audit

Date: 2026-06-20
Scope: high-signal implementation audit, not exhaustive
Reader: Claude / maintainers

## Summary

This note records implementation areas that appear to drift from the elidex
design philosophy captured in `CLAUDE.md` and `docs/design/ja/*`. It is not a
complete repository-wide audit. Treat it as a handoff memo for follow-up
planning: each finding should be re-checked against current code before
implementation, especially if related work has landed since this date.

The strongest signals are:

- synchronous legacy Web APIs are exposed from the current core VM surface;
- several DOM write paths appear to bypass the intended `ScriptSession`
  mutation boundary;
- an iframe cross-context API is published as a parity stub;
- some plugin-first surfaces are still represented as VM-local hard-coded
  dispatch;
- the shell defaults to the compat style resolver without an obvious core/app
  mode switch at the call site.

## Philosophy References

- `CLAUDE.md`:
  - `Whole-engine core/compat/deprecated consistency`
  - `Plugin-first extensibility`
  - `ScriptSession as the sole Script<->ECS boundary`
  - `Ideal over pragmatic`
  - `Layering mandate`
- `docs/design/ja/12-dom-cssom.md`:
  - DOM APIs are built on `ScriptSession`;
  - DOM writes are recorded in the session mutation buffer;
  - `DomApiHandler` carries `DomSpecLevel` and makes methods individually
    toggleable.
- `docs/design/ja/14-script-engines-webapi.md`:
  - `localStorage` / `sessionStorage` are compat-only;
  - `document.cookie` is compat-only;
  - async `elidex.storage` and CookieStore are the core equivalents.
- `docs/design/ja/28-adr.md`:
  - ADR #14: DOM API architecture uses `DomApiHandler` with
    Living/Legacy/Deprecated levels.
  - ADR #16: sync Web APIs are compat; async equivalents are core.
  - ADR #17: `ScriptSession` is the unified Script<->ECS boundary.
- `.claude/skills/elidex-review/axes.md`:
  - Axis 1: layering mandate and core/compat split;
  - Axis 2: ECS-native lens;
  - Axis 3: pragmatic shortcut / stub markers.

## Findings

### F1. Sync legacy storage APIs are exposed from the core VM surface

Severity: IMP

Evidence:

- `crates/script/elidex-js/src/vm/host/window.rs:522` registers
  `localStorage` and `sessionStorage` accessors on `Window`.
- `crates/script/elidex-js/src/vm/host/window.rs:527` implements the
  `window.localStorage` getter as a direct VM host binding.
- `crates/script/elidex-js/src/vm/host/storage.rs:1` defines the `Storage`
  interface as a VM host binding.
- `crates/script/elidex-js/src/vm/host/storage.rs:82` documents a known
  opaque-origin spec deviation: opaque origins are partitioned into a
  per-VM sentinel bucket instead of throwing `SecurityError`.

Why this conflicts:

`docs/design/ja/14-script-engines-webapi.md` classifies
`localStorage / sessionStorage` as compat-only, with async `elidex.storage` as
the core equivalent. ADR #16 repeats the same boundary: sync APIs block the
main thread and require blocking IPC in a multi-process architecture.

Recommended next investigation:

- Decide whether current `Storage` should move behind a compat Web API feature
  or plugin boundary.
- Check whether any elidex-app/core entry point can currently observe
  `window.localStorage`.
- Re-evaluate the opaque-origin sentinel fallback when the compat boundary is
  defined; it may be acceptable only as a browser-compat shim, not as core
  behavior.

### F2. `document.cookie` is implemented on the core Document host binding

Severity: IMP

Evidence:

- `crates/script/elidex-js/src/vm/host/document.rs:594` implements the
  `document.cookie` getter.
- `crates/script/elidex-js/src/vm/host/document.rs:639` implements the
  `document.cookie = value` setter.
- `crates/script/elidex-js/src/vm/host/document.rs:1098` still says `cookie`
  is a stub whose setter drops writes, but the implementation is real.

Why this conflicts:

`docs/design/ja/14-script-engines-webapi.md` classifies `document.cookie` as
compat-only and names CookieStore API as the core equivalent. This also touches
`CLAUDE.md`'s whole-engine core/compat rule: sync legacy APIs should not be
mixed into the clean core surface.

Recommended next investigation:

- Decide whether `document.cookie` should be moved behind a cookies-compat
  feature/plugin boundary.
- Confirm that CookieStore remains the intended core API for browser and app
  modes.
- Fix the stale doc comment regardless of the larger migration; it currently
  misleads reviewers about observable behavior.

### F3. Some DOM write paths appear to bypass the `ScriptSession` mutation buffer

Severity: IMP

Evidence:

- `crates/script/elidex-js/src/vm/host/element_attrs.rs:112` implements
  `attr_set` as a direct `EcsDom::set_attribute` call.
- `crates/script/elidex-js/src/vm/host/element_attrs.rs:235` removes
  attributes via the VM helper path, not via a visible session mutation.
- Many reflected HTML IDL setters call `ctx.host().dom().set_attribute(...)`
  directly, for example:
  - `crates/script/elidex-js/src/vm/host/html_input_proto.rs:460`
  - `crates/script/elidex-js/src/vm/host/html_button_proto.rs:183`
  - `crates/script/elidex-js/src/vm/host/html_element_proto.rs:430`
  - `crates/script/elidex-js/src/vm/host/html_select_proto.rs:254`

Counter-signal:

The main `Node` tree mutation methods look better aligned: for example
`crates/script/elidex-js/src/vm/host/node_proto.rs:692` routes
`appendChild` through `dom_bridge::invoke_dom_api`.

Why this conflicts:

`docs/design/ja/12-dom-cssom.md` says DOM writes go through the
`ScriptSession` mutation buffer and flush path. ADR #17 frames
`ScriptSession` as the single mechanism for identity mapping, buffered
mutation, MutationObserver records, GC coordination, and live query management.
`CLAUDE.md` now repeats this as `ScriptSession as the sole Script<->ECS
boundary`.

Recommended next investigation:

- Audit all direct `ctx.host().dom().set_attribute`, `remove_attribute`,
  `append_child`, `insert_before`, and `remove_child` calls in
  `crates/script/elidex-js/src/vm/host/`.
- For each direct write, determine whether it is only a marshalling helper
  exception or an algorithmic DOM mutation that should route through
  `elidex-dom-api` / `elidex-script-session::DomApiHandler`.
- Confirm that MutationObserver, custom element reactions, style invalidation,
  form-state reconciliation, and live collection invalidation all observe these
  direct reflected-attribute writes. If they do, document the exception. If they
  do not, migrate to a single mutation path.

### F4. iframe `contentDocument` / `contentWindow` are exposed as parity stubs

Severity: IMP

Evidence:

- `crates/script/elidex-js/src/vm/host/html_iframe_proto.rs:31` describes
  `contentDocument` and `contentWindow` as "Parity null stubs".
- `crates/script/elidex-js/src/vm/host/html_iframe_proto.rs:115` installs
  those accessors.
- `crates/script/elidex-js/src/vm/host/html_iframe_proto.rs:299` and
  `crates/script/elidex-js/src/vm/host/html_iframe_proto.rs:315` always return
  `null`.

Why this conflicts:

The comments explicitly preserve legacy boa parity. That conflicts with
`CLAUDE.md`'s `Ideal over pragmatic` rule unless the stub is backed by a
tracked defer slot with a clear spec gap, trigger, and owner. The behavior is
also observably wrong for same-origin iframes, even though it happens to match
the cross-origin null case.

Recommended next investigation:

- Decide whether these accessors should be absent until same-origin
  browsing-context proxies exist, or whether they should stay with an explicit
  defer slot.
- If retained, record the exact missing model: sub-frame browsing-context
  entity, associated Document/Window proxy identity, same-origin access checks,
  and cross-VM proxy semantics.
- Add targeted tests that distinguish same-origin, cross-origin, sandboxed, and
  detached iframe cases when the real implementation starts.

### F5. HTML tag-to-prototype routing is hard-coded in the VM

Severity: MIN / IMP

Evidence:

- `crates/script/elidex-js/src/vm/host/elements.rs:186` starts a VM-local
  `tag_specific_html_prototype` chain.
- `crates/script/elidex-js/src/vm/host/elements.rs:194` and following lines
  dispatch by repeated `tag_matches_ascii_case` calls.
- `crates/script/elidex-js/src/vm/host/elements.rs:249` and
  `crates/script/elidex-js/src/vm/host/elements.rs:352` continue the same
  pattern across T2b/T2c/T2d groups.

Counter-signal:

The code references defer slots and explains the performance tradeoff. The
current finding is therefore less urgent than F1-F4.

Why this conflicts:

`CLAUDE.md`'s plugin-first rule says HTML tag handling should converge on the
same static/dynamic plugin mental model as other extension points. The current
shape puts the tag surface inside VM host code and makes future tag additions
look like more VM branches, not new handlers behind a shared trait/spec-level
surface.

Recommended next investigation:

- Decide whether prototype selection should be generated from, or delegated to,
  an HTML element registry/spec-level table rather than hand-maintained in
  `VmInner`.
- Keep VM responsibilities to wrapper identity, prototype object storage, brand
  checks, and JS value marshalling.
- Avoid a purely mechanical O(1) lookup refactor if it preserves the same
  VM-owned tag-surface problem.

### F6. The shell pipeline defaults to compat style resolution

Severity: MIN

Evidence:

- `crates/shell/elidex-shell/src/pipeline.rs:62` performs initial style
  resolution "with compat layer".
- `crates/shell/elidex-shell/src/pipeline.rs:95` re-resolves styles after
  script mutations "with compat layer".
- `crates/shell/elidex-shell/src/lib.rs:261` defines `resolve_with_compat`
  using legacy UA styles plus presentational hints.

Counter-signal:

The compat logic itself is well separated in `elidex-dom-compat`, and the shell
may intentionally be browser-compat by default. This is not the same as mixing
compat algorithms into `elidex-style` core.

Why this may conflict:

`CLAUDE.md` and ADR #16 draw a mode boundary between core/app and browser
compat surfaces. If app/core shell paths reuse this pipeline, compat behavior
may be observable where it should be opt-in or mode-selected.

Recommended next investigation:

- Identify every caller of this pipeline and classify it as browser core,
  browser compat, app, or test.
- If both core/app and compat callers share this function, add an explicit
  style-compat policy parameter rather than hard-wiring `resolve_with_compat`.
- Keep the current `elidex-dom-compat` crate boundary; the question is default
  mode selection, not the compat implementation itself.

## Non-Findings / Already Aligned Areas

- `with` appears to be rejected in strict/core execution rather than
  implemented as a legacy semantic.
- `eval` comments indicate sloppy direct-eval caller-scope injection is treated
  as compat-plugin territory.
- The main `Node` mutation methods use `dom_bridge::invoke_dom_api`, which is
  closer to the intended engine-independent dispatch than direct ECS mutation.
- CSS compat logic is mostly isolated in `elidex-dom-compat`; the concern is
  shell defaulting, not core contamination inside `elidex-style`.

## Suggested Follow-Up Work Items

1. Web API compat split audit:
   - enumerate `Window`, `Document`, and navigator APIs exposed by core VM;
   - classify each as Modern / Legacy / Deprecated;
   - check whether compile-time feature gates or plugin boundaries match the
     classification.
2. ScriptSession mutation boundary audit:
   - enumerate every direct ECS write in `vm/host`;
   - map each write to MutationObserver, custom element reactions, style
     invalidation, form-state reconciliation, and live query effects;
   - converge on one canonical mutation path.
3. iframe browsing-context plan:
   - write an implementation plan for same-origin and cross-origin
     `contentWindow` / `contentDocument`;
   - decide whether current null stubs should be removed, gated, or registered
     as a formal defer slot before the real model lands.
4. HTML element registry/prototype dispatch plan:
   - check whether tag-to-prototype routing can derive from plugin metadata;
   - keep built-in hot paths static while preserving dynamic extension for
     custom or policy-gated HTML surfaces.
5. Shell mode audit:
   - confirm whether `elidex-app` or core shell paths can observe legacy UA
     stylesheet / presentational hints;
   - add an explicit mode/policy if needed.

## Notes For Claude

- Do not treat the recommended actions above as final fixes. They are audit
  leads. Re-run local searches and inspect current code before writing plans or
  patches.
- Prefer a plan-review before implementing F1, F2, or F3. Each spans multiple
  invariants and can easily become an edge-dense PR.
- If a finding is intentionally accepted as a phase constraint, record the
  defer slot with: why deferred, re-evaluation trigger, and re-evaluation date.

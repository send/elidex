//! Live DOM collections — `HTMLCollection` + `NodeList` (WHATWG DOM
//! §4.2.10 / §4.2.10.1).
//!
//! Both interfaces are implemented as payload-free `ObjectKind`
//! variants ([`ObjectKind::HtmlCollection`](super::super::value::ObjectKind::HtmlCollection)
//! / [`ObjectKind::NodeList`](super::super::value::ObjectKind::NodeList))
//! with their filter state held in
//! [`VmInner::live_collection_states`](super::super::VmInner::live_collection_states).
//! They share this single state table because every kind is
//! resolved by the same per-access ECS traversal — the only
//! distinction at read time is which variant of
//! [`LiveCollectionKind`] drives the filter predicate and (for
//! HTMLCollection only) whether `namedItem` is exposed.
//!
//! ## Liveness
//!
//! Per spec, `HTMLCollection` is always live and `NodeList` is
//! live *except* when returned by `querySelectorAll` (§4.2.6).
//! That single static case is represented by
//! [`LiveCollectionKind::Snapshot`], which carries a pre-captured
//! `Vec<Entity>`; every other kind exposes the *observable*
//! semantics that mutations made after the collection is obtained
//! are visible on subsequent `length` / `item(i)` / indexed
//! accesses.
//!
//! Each non-snapshot wrapper carries a [`LiveCollectionCache`]
//! validated against [`EcsDom::inclusive_descendants_version`] of
//! the collection's root: cache hit on an unchanged subtree skips
//! the descendant walk, cache miss on a bumped version re-walks
//! and refreshes.  Snapshot variants bypass the cache entirely
//! (their entity list is frozen at construction).  This is a pure
//! performance optimisation — the cache version check is driven
//! by [`EcsDom::rev_version`], which is invoked from every tree-
//! and attribute-mutation site so the cache stays observably
//! equivalent to a per-read re-walk.
//!
//! ## GC contract
//!
//! The prototypes are rooted via the `proto_roots` array (gc.rs).
//! `LiveCollectionKind` and `LiveCollectionCache` together store
//! only `Entity`, `StringId`, `Vec<StringId>`, `Vec<Entity>`, and
//! `Cell<Option<u64>>` (the cached subtree version, `None` until
//! the first miss-path populates it) — **no `ObjectId`
//! references** — so the trace step has nothing to fan out.  The
//! sweep tail prunes `live_collection_states` entries whose key
//! `ObjectId` was collected, same pattern as
//! `headers_states` / `blob_data`.
//!
//! ## Brand check
//!
//! Methods on both prototypes route through
//! [`require_collection_receiver`] which extracts the backing
//! filter kind and issues "Illegal invocation" TypeError for
//! non-collection receivers (so `HTMLCollection.prototype.item.call({})`
//! throws, matching WebIDL brand semantics).

#![cfg(feature = "engine")]

use std::cell::{Cell, RefCell};

use elidex_ecs::{EcsDom, Entity, NodeKind};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError, ARRAY_ITER_KIND_VALUES,
};
use super::super::{NativeFn, StringId, VmInner};

// -------------------------------------------------------------------------
// Filter discriminator
// -------------------------------------------------------------------------

/// The filter backing an `HTMLCollection` / `NodeList` wrapper.
///
/// HTMLCollection variants filter to **Element** nodes only
/// (WHATWG §4.2.10); NodeList variants include Text and Comment
/// nodes as well.  The `Snapshot` variant alone is static — every
/// other kind re-traverses on read.
pub(crate) enum LiveCollectionKind {
    // HTMLCollection filters (Element-only).
    ByTag {
        root: Entity,
        tag: StringId,
        all: bool,
    },
    ByClass {
        root: Entity,
        class_names: Vec<StringId>,
    },
    Children {
        parent: Entity,
    },
    Forms {
        doc: Entity,
    },
    Images {
        doc: Entity,
    },
    Links {
        doc: Entity,
    },
    // NodeList filters.
    ChildNodes {
        parent: Entity,
    },
    Snapshot {
        entities: Vec<Entity>,
    },
    ByName {
        doc: Entity,
        name: StringId,
    },
    /// `form.elements` / `fieldset.elements` —
    /// HTMLFormControlsCollection backing.  Filters listed elements
    /// (button / fieldset / input / object / output / select /
    /// textarea):
    ///
    /// - When `scope` is a `<form>`: walks the form's tree root
    ///   and matches every listed element whose form-association
    ///   resolves to `scope` (HTML §4.10.18.4 step 1).  This
    ///   observes cross-tree associates via the `form="<id>"`
    ///   content attribute.
    /// - When `scope` is a `<fieldset>`: walks `scope`'s
    ///   descendants only (HTML §4.10.7).
    ///
    /// The owning prototype (HTMLFormControlsCollection in both
    /// cases) is fixed at install time; only the resolver's walk
    /// shape branches.
    FormControls {
        /// The form OR fieldset that owns this collection.
        scope: Entity,
    },
    /// `select.options` — HTMLOptionsCollection backing.  Filters
    /// `<option>` elements within `select`'s descendants per HTML
    /// §2.7.4 (includes options nested inside `<optgroup>`).
    /// Mutable per HTML §2.7.4.2 — the
    /// `length` setter / `add()` / `remove()` mutate the parent
    /// `<select>` directly; the variant itself stays immutable but
    /// downstream re-walks observe the new tree state.  Unlike
    /// [`Self::FormControls`], the resolver is descendant-tag-only
    /// (no attribute reads) so caching against
    /// [`EcsDom::inclusive_descendants_version`] of the `<select>`
    /// is safe: every tree mutation under the select bumps that
    /// counter.
    Options {
        /// The `<select>` element whose descendant `<option>`s the
        /// collection enumerates.
        select: Entity,
    },
}

impl LiveCollectionKind {
    /// `true` if this variant backs an `HTMLCollection`; `false`
    /// for `NodeList` variants.  Drives prototype selection at
    /// wrapper construction and `namedItem` exposure at method
    /// install time.
    fn is_html_collection(&self) -> bool {
        matches!(
            self,
            Self::ByTag { .. }
                | Self::ByClass { .. }
                | Self::Children { .. }
                | Self::Forms { .. }
                | Self::Images { .. }
                | Self::Links { .. }
                | Self::FormControls { .. }
                | Self::Options { .. }
        )
    }

    /// Root entity whose `inclusive_descendants_version` drives the
    /// cache validity check.  `None` for [`Self::Snapshot`] —
    /// snapshot entity lists are frozen at construction and bypass
    /// the cache entirely.
    fn cache_root(&self) -> Option<Entity> {
        match self {
            Self::ByTag { root, .. } | Self::ByClass { root, .. } => Some(*root),
            Self::Children { parent } | Self::ChildNodes { parent } => Some(*parent),
            Self::Forms { doc }
            | Self::Images { doc }
            | Self::Links { doc }
            | Self::ByName { doc, .. } => Some(*doc),
            Self::FormControls { scope } => Some(*scope),
            Self::Options { select } => Some(*select),
            Self::Snapshot { .. } => None,
        }
    }

    /// `true` when the variant's resolved entity list may be cached
    /// against [`Self::cache_root`]'s
    /// [`EcsDom::inclusive_descendants_version`]; `false` when the
    /// list is invalidated by mutations not visible through that
    /// version counter.
    ///
    /// HTMLFormControlsCollection (per plan review finding #1):
    /// listed-element membership for a form depends on the `form`
    /// attribute on cross-tree associates AND on the `disabled` /
    /// `name` attributes of every form-control descendant.  The
    /// `set_attribute` path bumps `rev_version` only on the
    /// mutated entity's ancestor chain, NOT on a sibling form/scope
    /// that consumes the value through its filter — so caching
    /// would silently return stale entity lists across an attribute
    /// edit on any descendant control.  Opt out for correctness;
    /// typical forms are <50 elements and the per-access walk cost
    /// is bounded.
    fn is_cacheable(&self) -> bool {
        // Only `FormControls` opts out — its filter consults the
        // `form` attribute on cross-tree associates, whose
        // mutations don't bump the form's `inclusive_descendants_version`.
        // `Options` filters by tag only (`<option>` descendants of
        // `<select>`), so the standard descendants-version cache
        // tracks every mutation that affects membership.
        !matches!(self, Self::FormControls { .. })
    }
}

/// Per-wrapper entity-list cache, validated against
/// [`EcsDom::inclusive_descendants_version`] of the kind's
/// [`cache_root`](LiveCollectionKind::cache_root).
///
/// Stored alongside [`LiveCollectionKind`] in
/// [`VmInner::live_collection_states`](super::super::VmInner::live_collection_states)
/// so cache state survives across `length` / `item(i)` / iter
/// reads on the same wrapper without `&mut self` access through
/// the resolve path.  `Cell` + `RefCell` give the interior
/// mutability needed for that — the cache is owned during the
/// `remove → resolve → insert` dance in
/// [`resolve_receiver_entities`] / [`try_indexed_get`], but the
/// cache fields are written via shared-ref methods so the dance
/// is purely a borrow-aliasing accommodation, not a logical
/// requirement.
/// `cached_version` is `Option<u64>` rather than `u64` so the
/// default `None` state can never collide with a real
/// [`EcsDom::inclusive_descendants_version`] value of `0` (a
/// freshly spawned entity that has not yet been mutated through
/// any `rev_version`-bumping site).  Without the explicit
/// "uninitialized" marker the very first read on such an entity
/// would false-hit `cache.cached_version.get() == 0` and return
/// the empty `cached_entities` even when the descendant walk
/// would have yielded a non-empty list — load-bearing safety
/// margin against any future change to the rev_version
/// invariants under which a node could end up with descendants
/// before its own version has been bumped.
#[derive(Default)]
pub(crate) struct LiveCollectionCache {
    cached_version: Cell<Option<u64>>,
    cached_entities: RefCell<Vec<Entity>>,
}

// -------------------------------------------------------------------------
// Entity resolution (per-access re-traversal)
// -------------------------------------------------------------------------

/// Re-resolve the entity list backing `kind`.  Every live variant
/// walks the ECS at this point; `Snapshot` clones the cached vec.
///
/// Takes resolved UTF-8 needles (tag / class-name / name-attr)
/// pre-materialised by the caller — avoids threading the
/// `StringPool` through here alongside the `&EcsDom` borrow, which
/// would otherwise fight the `NativeContext` split-field pattern.
///
/// Returning a `Vec` (not an iterator) is deliberate — callers
/// typically need the length up front, and the re-allocation
/// cost is dominated by the traversal anyway.
fn resolve_entities_with_needles(
    dom: &EcsDom,
    kind: &LiveCollectionKind,
    resolved_needles: &ResolvedNeedles,
) -> Vec<Entity> {
    match kind {
        LiveCollectionKind::ByTag { root, all, .. } => {
            let tag_str = resolved_needles.primary.as_deref().unwrap_or("");
            let mut out = Vec::new();
            dom.traverse_descendants(*root, |e| {
                if e == *root {
                    return true; // skip root itself (Element.getElementsByTagName semantics)
                }
                let matches = *all
                    || dom.with_tag_name(e, |t| t.is_some_and(|s| s.eq_ignore_ascii_case(tag_str)));
                if matches && dom.node_kind_inferred(e) == Some(NodeKind::Element) {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::ByClass { root, .. } => {
            if resolved_needles.class_names.is_empty() {
                return Vec::new();
            }
            let mut out = Vec::new();
            dom.traverse_descendants(*root, |e| {
                if e == *root {
                    return true;
                }
                if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
                    return true;
                }
                let has_all = dom.with_attribute(e, "class", |class_attr| {
                    class_attr.is_some_and(|c| {
                        resolved_needles
                            .class_names
                            .iter()
                            .all(|n| c.split_whitespace().any(|tok| tok == n))
                    })
                });
                if has_all {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::Children { parent } => dom
            .children_iter(*parent)
            .filter(|&e| dom.node_kind_inferred(e) == Some(NodeKind::Element))
            .collect(),
        LiveCollectionKind::Forms { doc } => collect_descendants_with_tag(dom, *doc, "form"),
        LiveCollectionKind::Images { doc } => collect_descendants_with_tag(dom, *doc, "img"),
        LiveCollectionKind::Links { doc } => {
            let mut out = Vec::new();
            dom.traverse_descendants(*doc, |e| {
                if e == *doc {
                    return true;
                }
                let tag_ok = dom.with_tag_name(e, |t| {
                    t.is_some_and(|s| s.eq_ignore_ascii_case("a") || s.eq_ignore_ascii_case("area"))
                });
                if tag_ok && dom.has_attribute(e, "href") {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::ChildNodes { parent } => dom.children_iter(*parent).collect(),
        LiveCollectionKind::Snapshot { entities } => entities.clone(),
        LiveCollectionKind::ByName { doc, .. } => {
            let needle = resolved_needles.primary.as_deref().unwrap_or("");
            let mut out = Vec::new();
            dom.traverse_descendants(*doc, |e| {
                // Skip the doc root itself — `getElementsByName` is
                // a descendant-only query — and restrict to Element
                // nodes per WHATWG HTML §3.1.5 step 1 ("list of
                // elements with the given name").  Non-Element
                // nodes that happen to carry a `name` attribute
                // (possible via direct `EcsDom::set_attribute` on
                // any entity) must not leak into the result.
                if e == *doc {
                    return true;
                }
                if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
                    return true;
                }
                if dom.with_attribute(e, "name", |v| v == Some(needle)) {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::FormControls { scope } => {
            let scope_is_form = dom.has_tag(*scope, "form");
            let mut out = Vec::new();
            // For `<form>` scope, walk the form's tree root so that
            // cross-tree associates via `form="<id>"` are observed
            // (HTML §4.10.18.4 step 1).  For `<fieldset>` scope,
            // walk descendants only (HTML §4.10.7).
            //
            // `find_tree_root` returns the physical tree root —
            // doc for attached forms, the topmost detached ancestor
            // otherwise — which matches the spec's "form element's
            // root" wording without `owner_document`'s
            // associated-document indirection (which would point at
            // the bound doc even for a detached subtree).
            let walk_root = if scope_is_form {
                dom.find_tree_root(*scope)
            } else {
                *scope
            };
            dom.traverse_descendants(walk_root, |e| {
                if e == walk_root {
                    return true;
                }
                if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
                    return true;
                }
                let is_listed = dom.with_tag_name(e, |t| match t {
                    Some(s) => is_listed_form_element_tag(s),
                    None => false,
                });
                if !is_listed {
                    return true;
                }
                if scope_is_form {
                    if super::form_assoc::resolve_form_owner_dom(dom, e) == Some(*scope) {
                        out.push(e);
                    }
                } else {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::Options { select } => {
            // HTML §2.7.4 — `select.options` enumerates every
            // descendant `<option>` of the `<select>`.  Spec wording
            // restricts to direct + `<optgroup>`-nested options;
            // since the elidex DOM tree only allows option as a
            // child of select / optgroup at parser time, a full
            // descendant walk filtered by tag is equivalent.
            let mut out = Vec::new();
            dom.traverse_descendants(*select, |e| {
                if e == *select {
                    return true;
                }
                if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
                    return true;
                }
                if dom.with_tag_name(e, |t| t.is_some_and(|s| s.eq_ignore_ascii_case("option"))) {
                    out.push(e);
                }
                true
            });
            out
        }
    }
}

/// HTML §4.10.2 listed elements that participate in form-controls
/// collections.  `<input type=image>` (image button) is technically
/// excluded from `form.elements` per HTML §4.10.18.4 step 1.6 but
/// included in `fieldset.elements`; we approximate here by including
/// every `<input>` and refining at a future spec-tightening pass
/// (slot #11-form-image-button-exclusion).
pub(super) fn is_listed_form_element_tag(tag: &str) -> bool {
    const LISTED_ELEMENTS: [&str; 7] = [
        "button", "fieldset", "input", "object", "output", "select", "textarea",
    ];
    LISTED_ELEMENTS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(tag))
}

/// Pre-materialised UTF-8 needles for entity resolution — filled
/// by [`resolve_needles`] from the filter's `StringId`s before
/// the EcsDom borrow is taken.
struct ResolvedNeedles {
    primary: Option<String>,
    class_names: Vec<String>,
}

/// Extract the UTF-8 strings a kind's filter consults so the
/// actual traversal can run with a disjoint `&EcsDom` borrow.
fn resolve_needles(vm: &VmInner, kind: &LiveCollectionKind) -> ResolvedNeedles {
    match kind {
        LiveCollectionKind::ByTag { tag, .. } => ResolvedNeedles {
            primary: Some(vm.strings.get_utf8(*tag)),
            class_names: Vec::new(),
        },
        LiveCollectionKind::ByClass { class_names, .. } => ResolvedNeedles {
            primary: None,
            class_names: class_names
                .iter()
                .map(|sid| vm.strings.get_utf8(*sid))
                .collect(),
        },
        LiveCollectionKind::ByName { name, .. } => ResolvedNeedles {
            primary: Some(vm.strings.get_utf8(*name)),
            class_names: Vec::new(),
        },
        _ => ResolvedNeedles {
            primary: None,
            class_names: Vec::new(),
        },
    }
}

/// Public resolver — returns the cached entity list when the
/// subtree's
/// [`inclusive_descendants_version`](EcsDom::inclusive_descendants_version)
/// is unchanged, otherwise materialises needles from the VM's
/// string pool and runs the ECS traversal.
///
/// Hot-path semantics + drift-safety: the post-walk refresh runs
/// through [`cache_store`], which is the single store-side helper
/// shared with [`try_indexed_get`].  The version-check halves
/// still differ between the two call sites because they have
/// different borrow shapes — `NativeContext` / `host_if_bound`
/// here vs an explicit `&EcsDom` arg in `try_indexed_get` — so
/// the read sides are mirrored manually rather than abstracted.
/// `host` is dropped between the cache-hit phase and the miss
/// phase (re-borrowed below) so the shared `&VmInner` for needle
/// materialisation does not alias the `&EcsDom` borrow used for
/// the version check.
pub(super) fn resolve_entities_for(
    ctx: &mut NativeContext<'_>,
    kind: &LiveCollectionKind,
    cache: &LiveCollectionCache,
) -> Vec<Entity> {
    // Non-cacheable kinds (FormControls today) skip both the
    // version check and the post-walk store — the cache fields are
    // simply unused for these variants.  See
    // [`LiveCollectionKind::is_cacheable`] for the rationale.
    let cacheable = kind.is_cacheable();

    // Phase 1 — cache-hit check (no needle allocation).  The
    // `host` borrow is scoped to this block; Phase 2 re-borrows.
    let cur_version = if cacheable {
        if let Some(root) = kind.cache_root() {
            // Post-unbind access to a retained collection wrapper must
            // not panic; return an empty list so `.length` reads 0,
            // `.item(i)` returns null, and `@@iterator` yields no
            // elements.  `HostData::dom()` asserts `is_bound()`.
            let Some(host) = ctx.host_if_bound() else {
                return Vec::new();
            };
            let cur = host.dom().inclusive_descendants_version(root);
            if cache.cached_version.get() == Some(cur) {
                return cache.cached_entities.borrow().clone();
            }
            Some(cur)
        } else {
            None
        }
    } else {
        None
    };

    // Phase 2 — cache miss (or Snapshot, or non-cacheable).
    // Needles only get materialised here, off the hot path.
    let needles = resolve_needles(ctx.vm, kind);
    let Some(host) = ctx.host_if_bound() else {
        return Vec::new();
    };
    let fresh = resolve_entities_with_needles(host.dom(), kind, &needles);
    if cacheable {
        cache_store(cache, cur_version, &fresh);
    }
    fresh
}

/// Refresh the cache after a successful descendant walk.  Skips
/// the store when `cur_version == None` (Snapshot variants — see
/// [`LiveCollectionKind::cache_root`]).
///
/// Reuses the existing `cached_entities` `Vec` capacity via
/// `clear()` + `extend_from_slice` rather than re-allocating
/// (`fresh.to_vec()`); on mutation-heavy workloads the cache buffer
/// quickly stabilises at the result-set's high-water mark, after
/// which subsequent miss-path refreshes become allocation-free.
fn cache_store(cache: &LiveCollectionCache, cur_version: Option<u64>, fresh: &[Entity]) {
    if let Some(cur) = cur_version {
        cache.cached_version.set(Some(cur));
        let mut buf = cache.cached_entities.borrow_mut();
        buf.clear();
        buf.extend_from_slice(fresh);
    }
}

/// Helper — collect every descendant Element with a given tag.
fn collect_descendants_with_tag(dom: &EcsDom, root: Entity, tag: &str) -> Vec<Entity> {
    let mut out = Vec::new();
    dom.traverse_descendants(root, |e| {
        if e == root {
            return true;
        }
        if dom.with_tag_name(e, |t| t.is_some_and(|s| s.eq_ignore_ascii_case(tag))) {
            out.push(e);
        }
        true
    });
    out
}

// -------------------------------------------------------------------------
// Named item tag allowlist (HTMLCollection only)
// -------------------------------------------------------------------------

/// Tags whose `name` attribute contributes to
/// `HTMLCollection.namedItem` lookup per WHATWG §4.2.10.2.1
/// step 2.2.  `id` matching applies to every Element regardless of
/// tag; the `name` fallback is restricted to this list so
/// `<div name="foo">` does NOT surface via `namedItem("foo")`.
///
/// Compared via `eq_ignore_ascii_case` against static literals — no
/// owned lowercase copy is allocated per call (namedItem resolution
/// invokes this in its inner loop on every indexed access).
pub(super) fn tag_allows_name_lookup(tag: &str) -> bool {
    const NAMED_ITEM_TAGS: [&str; 15] = [
        "a", "area", "button", "form", "fieldset", "iframe", "img", "input", "object", "output",
        "select", "textarea", "map", "meta", "embed",
    ];
    NAMED_ITEM_TAGS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(tag))
}

// -------------------------------------------------------------------------
// Prototype registration
// -------------------------------------------------------------------------

impl VmInner {
    /// Allocate `HTMLCollection.prototype` and install `length` /
    /// `item` / `namedItem` / `[Symbol.iterator]`.  Shared methods
    /// (`length` / `item` / `@@iterator`) use HTMLCollection-tagged
    /// wrappers so brand-check failures surface `"HTMLCollection"`
    /// in the error message rather than the shared-native default.
    pub(in crate::vm) fn register_html_collection_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.html_collection_prototype = Some(proto_id);

        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_hc_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.item,
            native_hc_item,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.named_item,
            native_collection_named_item,
            shape::PropertyAttrs::METHOD,
        );
        self.install_symbol_iterator(proto_id, native_hc_iterator);
    }

    /// Allocate `HTMLOptionsCollection.prototype` chained to
    /// [`Self::html_collection_prototype`] (HTML §2.7.4).  Adds the
    /// **mutable** surface (`length` setter / `add()` / `remove()`)
    /// on top of the inherited HTMLCollection methods.  Member set
    /// per HTML §2.7.4.2:
    ///
    /// - `length` accessor — getter inherited from HTMLCollection,
    ///   setter installed here that grows / shrinks the parent
    ///   `<select>`'s `<option>` children.
    /// - `add(option, before?)` — inserts an `<option>` /
    ///   `<optgroup>` before another item or appends if `before` is
    ///   `null` (default).
    /// - `remove(index)` — removes the option at `index`.  Spec
    ///   defers to `ChildNode.remove` semantics on the option.
    /// - `selectedIndex` accessor — read-only here (the IDL is
    ///   actually RW; the setter goes on HTMLSelectElement.prototype
    ///   instead and proxies through, matching the spec's "alias"
    ///   wording).
    pub(in crate::vm) fn register_html_options_collection_prototype(&mut self) {
        let parent = self.html_collection_prototype.expect(
            "register_html_options_collection_prototype called before \
             register_html_collection_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_options_collection_prototype = Some(proto_id);

        // Override `length` with an HTMLOptionsCollection-specific
        // getter/setter pair.  Getter behaviour matches the
        // inherited HTMLCollection getter; the override exists
        // primarily so the **setter** path lands on the correct
        // (HTMLOptionsCollection-tagged) brand check.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_options_length_get,
            Some(native_options_length_set),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.add_method,
            native_options_add,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.remove_method,
            native_options_remove,
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate `HTMLFormControlsCollection.prototype` chained to
    /// [`Self::html_collection_prototype`].  The chain inherits
    /// `length` / `item` / `[Symbol.iterator]` from the parent and
    /// installs a more permissive `namedItem(name)` here that
    /// dispatches against form-controls semantics (per HTML
    /// §4.10.18.4): walks the collection looking for a matching `id`
    /// or `name` attribute on any listed element (no tag allowlist
    /// — every listed element is allowed).  Multiple matches all
    /// being radio inputs returning a `RadioNodeList` (HTML
    /// §4.10.18.5) is deferred — see plan §F-1 / slot
    /// #11-tags-radionodelist.
    pub(in crate::vm) fn register_html_form_controls_collection_prototype(&mut self) {
        let parent = self.html_collection_prototype.expect(
            "register_html_form_controls_collection_prototype called before \
             register_html_collection_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_form_controls_collection_prototype = Some(proto_id);

        // Override `namedItem` with the FCC-specific lookup that
        // does NOT restrict the `name` match to a tag allowlist.
        self.install_native_method(
            proto_id,
            self.well_known.named_item,
            native_form_controls_named_item,
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate `NodeList.prototype` and install `length` / `item` /
    /// `forEach` / `[Symbol.iterator]`.  Shared methods use
    /// NodeList-tagged wrappers so brand-check error messages
    /// say `"NodeList"` when reached via `NodeList.prototype.*`.
    pub(in crate::vm) fn register_node_list_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.node_list_prototype = Some(proto_id);

        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_nl_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.item,
            native_nl_item,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.for_each,
            native_node_list_for_each,
            shape::PropertyAttrs::METHOD,
        );
        self.install_symbol_iterator(proto_id, native_nl_iterator);
    }

    /// Install `[Symbol.iterator]` pointing at the per-interface
    /// iterator wrapper so brand-check errors reflect the prototype
    /// the user reached through.
    fn install_symbol_iterator(&mut self, proto_id: ObjectId, iter_fn: NativeFn) {
        let fn_id = self.create_native_function("[Symbol.iterator]", iter_fn);
        let sym_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_key,
            PropertyValue::Data(JsValue::Object(fn_id)),
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate a new collection wrapper backed by `kind`.  The
    /// returned `ObjectId` carries `ObjectKind::HtmlCollection` or
    /// `ObjectKind::NodeList` (chosen by the kind's discriminator)
    /// and points at the matching prototype.
    ///
    /// `FormControls` variants get the
    /// `HTMLFormControlsCollection.prototype` (which itself chains
    /// to `HTMLCollection.prototype`) so `coll instanceof
    /// HTMLCollection === true` still holds and the FCC's
    /// `namedItem` override surfaces.
    pub(crate) fn alloc_collection(&mut self, kind: LiveCollectionKind) -> ObjectId {
        let (object_kind, proto) = if matches!(kind, LiveCollectionKind::FormControls { .. }) {
            (
                ObjectKind::HtmlCollection,
                self.html_form_controls_collection_prototype.expect(
                    "alloc_collection FormControls before \
                     register_html_form_controls_collection_prototype",
                ),
            )
        } else if matches!(kind, LiveCollectionKind::Options { .. }) {
            (
                ObjectKind::HtmlCollection,
                self.html_options_collection_prototype.expect(
                    "alloc_collection Options before \
                     register_html_options_collection_prototype",
                ),
            )
        } else if kind.is_html_collection() {
            (
                ObjectKind::HtmlCollection,
                self.html_collection_prototype
                    .expect("alloc_collection before register_html_collection_prototype"),
            )
        } else {
            (
                ObjectKind::NodeList,
                self.node_list_prototype
                    .expect("alloc_collection before register_node_list_prototype"),
            )
        };
        let id = self.alloc_object(Object {
            kind: object_kind,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.live_collection_states
            .insert(id, (kind, LiveCollectionCache::default()));
        id
    }

    /// Identity-preserving allocator for `select.options`
    /// HTMLOptionsCollection wrappers — same control returns the
    /// same collection ObjectId across repeated `.options` reads
    /// (matches browser semantics:
    /// `select.options === select.options` is `true`).  Cached on
    /// [`Self::options_collection_wrappers`]; reachability gated by
    /// the owning select wrapper through `gc/roots.rs` step (e4).
    pub(crate) fn cached_or_alloc_options_collection(&mut self, select: Entity) -> ObjectId {
        if let Some(&id) = self.options_collection_wrappers.get(&select) {
            return id;
        }
        let id = self.alloc_collection(LiveCollectionKind::Options { select });
        self.options_collection_wrappers.insert(select, id);
        id
    }
}

// -------------------------------------------------------------------------
// Native methods
// -------------------------------------------------------------------------

/// Brand-check helper — recover the `ObjectId` + whether the
/// receiver is an HTMLCollection (vs NodeList), enforcing per-method
/// interface contracts.
///
/// `interface` is the name of the prototype the method was installed
/// on (`"HTMLCollection"` or `"NodeList"`).  It drives both the
/// error message AND the accepted receiver kinds: HTMLCollection's
/// prototype accepts HTMLCollection receivers, NodeList's prototype
/// accepts NodeList receivers.  Shared methods (`length` / `item` /
/// `@@iterator`) are installed on BOTH prototypes via per-interface
/// wrapper natives so the error message reflects the prototype the
/// caller reached through — e.g.
/// `NodeList.prototype.length.call({})` reports
/// `"Failed to execute 'length' on 'NodeList'"`.
///
/// Returns a TypeError for non-collection receivers or mismatched
/// collection kinds so `.call({})` and cross-interface
/// `.call(otherKind)` both throw "Illegal invocation" (matches
/// browser behaviour).
fn require_collection_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    interface: &'static str,
) -> Result<(ObjectId, bool), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )));
    };
    let kind_is_html = matches!(ctx.vm.get_object(id).kind, ObjectKind::HtmlCollection);
    let kind_is_node_list = matches!(ctx.vm.get_object(id).kind, ObjectKind::NodeList);
    // `interface == "NodeList"` accepts only NodeList receivers;
    // every other interface value (including the two wrapper-less
    // HTMLCollection-only natives below) accepts only HTMLCollection.
    let ok = if interface == "NodeList" {
        kind_is_node_list
    } else {
        kind_is_html
    };
    if !ok {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )));
    }
    Ok((id, kind_is_html))
}

/// Resolve the backing entity list for a live collection receiver.
///
/// The temporary remove / re-insert dance is deliberate: the
/// traversal needs a shared borrow of `ctx.vm.strings` (via
/// `resolve_needles`) **and** a shared borrow of `ctx.host().dom()`
/// on the same `NativeContext`, but Rust's split-field rules deny
/// two aliasing paths through `ctx.vm` in one expression.  Taking
/// the kind out of the map temporarily breaks that aliasing into
/// two sequential borrows.
pub(super) fn resolve_receiver_entities(ctx: &mut NativeContext<'_>, id: ObjectId) -> Vec<Entity> {
    let Some((kind, cache)) = ctx.vm.live_collection_states.remove(&id) else {
        return Vec::new();
    };
    let entities = resolve_entities_for(ctx, &kind, &cache);
    ctx.vm.live_collection_states.insert(id, (kind, cache));
    entities
}

// Per-interface `length` getter wrappers — shared body, per-
// interface brand-failure message.
fn native_hc_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_length_get_impl(ctx, this, "HTMLCollection")
}

fn native_nl_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_length_get_impl(ctx, this, "NodeList")
}

fn collection_length_get_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    interface: &'static str,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "length", interface)?;
    let entities = resolve_receiver_entities(ctx, id);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(entities.len() as f64))
}

// Per-interface `item` wrappers.
fn native_hc_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_item_impl(ctx, this, args, "HTMLCollection")
}

fn native_nl_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_item_impl(ctx, this, args, "NodeList")
}

fn collection_item_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    interface: &'static str,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "item", interface)?;
    let index = match args.first() {
        Some(JsValue::Number(n)) if n.is_finite() => {
            let trunc = n.trunc();
            if trunc < 0.0 {
                return Ok(JsValue::Null);
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = trunc as usize;
            idx
        }
        Some(v) => {
            // WebIDL `unsigned long` — coerce; negatives / NaN map
            // to out-of-range (null).
            let n = super::super::coerce::to_int32(ctx.vm, *v)?;
            if n < 0 {
                return Ok(JsValue::Null);
            }
            #[allow(clippy::cast_sign_loss)]
            let idx = n as usize;
            idx
        }
        None => return Ok(JsValue::Null),
    };
    let entities = resolve_receiver_entities(ctx, id);
    Ok(match entities.get(index) {
        Some(&e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
        None => JsValue::Null,
    })
}

/// Shared `namedItem` walk: id match wins over name match (WHATWG
/// §4.2.10.2.1).  The `name_tag_allowlist` argument decides whether
/// the `name` attribute fallback is restricted to a specific set of
/// tags (HTMLCollection's allowlist via [`tag_allows_name_lookup`])
/// or accepts every element (HTMLFormControlsCollection per HTML
/// §4.10.18.4 — every listed element supports `name`).  Multi-match
/// radio-group RadioNodeList is deferred per slot
/// #11-tags-radionodelist.
fn named_item_lookup(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    interface: &'static str,
    name_tag_allowlist: bool,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "namedItem", interface)?;
    let Some(arg) = args.first().copied() else {
        return Ok(JsValue::Null);
    };
    let key_sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let key = ctx.vm.strings.get_utf8(key_sid);
    if key.is_empty() {
        return Ok(JsValue::Null);
    }
    let entities = resolve_receiver_entities(ctx, id);
    let mut name_hit: Option<Entity> = None;
    for &e in &entities {
        let dom = ctx.host().dom();
        if dom.with_attribute(e, "id", |v| v == Some(key.as_str())) {
            return Ok(JsValue::Object(ctx.vm.create_element_wrapper(e)));
        }
        if name_hit.is_none() {
            let name_tag_ok = if name_tag_allowlist {
                dom.with_tag_name(e, |t| t.is_some_and(tag_allows_name_lookup))
            } else {
                true
            };
            if name_tag_ok && dom.with_attribute(e, "name", |v| v == Some(key.as_str())) {
                name_hit = Some(e);
            }
        }
    }
    Ok(match name_hit {
        Some(e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
        None => JsValue::Null,
    })
}

fn native_collection_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    named_item_lookup(ctx, this, args, "HTMLCollection", true)
}

/// `HTMLFormControlsCollection.prototype.namedItem(name)` per HTML
/// §4.10.18.4 — same id-then-name walk as the parent class, but
/// without the tag allowlist (every listed form element exposes
/// `name`).
fn native_form_controls_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    named_item_lookup(ctx, this, args, "HTMLFormControlsCollection", false)
}

// Per-interface `@@iterator` wrappers.
fn native_hc_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_iterator_impl(ctx, this, args, "HTMLCollection")
}

fn native_nl_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_iterator_impl(ctx, this, args, "NodeList")
}

fn collection_iterator_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
    interface: &'static str,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "@@iterator", interface)?;
    let entities = resolve_receiver_entities(ctx, id);
    let values: Vec<JsValue> = entities
        .into_iter()
        .map(|e| JsValue::Object(ctx.vm.create_element_wrapper(e)))
        .collect();
    // Wrap the snapshot Array in an `ArrayIterator` so standard
    // `for ... of` consumption walks the snapshot.  The
    // `ARRAY_ITER_KIND_VALUES` discriminant selects the values-
    // yielding iterator (vs keys / entries).  Note: this iterator
    // reflects the *snapshot* at `@@iterator` time, not live DOM
    // changes that occur during iteration — consistent with
    // WHATWG §4.2.10 where the collection is live on access but
    // iteration uses IteratorProtocol over a captured Array.
    let array_id = ctx.vm.create_array_object(values);
    let proto = ctx.vm.array_iterator_prototype;
    let iter_obj = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(super::super::value::ArrayIterState {
            array_id,
            index: 0,
            kind: ARRAY_ITER_KIND_VALUES,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

fn native_node_list_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // NodeList-only method — brand-check against NodeList.
    let (id, _) = require_collection_receiver(ctx, this, "forEach", "NodeList")?;
    let callback = args.first().copied().unwrap_or(JsValue::Undefined);
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let callable = match callback {
        JsValue::Object(oid) => ctx.vm.get_object(oid).kind.is_callable(),
        _ => false,
    };
    if !callable {
        return Err(VmError::type_error(
            "Failed to execute 'forEach' on 'NodeList': callback is not callable".to_string(),
        ));
    }
    let entities = resolve_receiver_entities(ctx, id);
    for (idx, entity) in entities.into_iter().enumerate() {
        let value = JsValue::Object(ctx.vm.create_element_wrapper(entity));
        #[allow(clippy::cast_precision_loss)]
        let index_val = JsValue::Number(idx as f64);
        ctx.vm
            .call_value(callback, this_arg, &[value, index_val, this])?;
    }
    Ok(JsValue::Undefined)
}

// -------------------------------------------------------------------------
// Indexed property access (called from `ops_property::get_element`)
// -------------------------------------------------------------------------

/// Try to resolve `collection[index]` for HTMLCollection / NodeList
/// receivers.  Returns `Some(entity)` when the key is a valid
/// numeric index (or a numeric-string for HTMLCollection's legacy
/// named property fallback).  Returns `None` when the caller
/// should fall through to the regular property / prototype lookup
/// path (so non-index lookups like `coll.length` still see the
/// prototype accessor).
///
/// Returns the backing `Entity` rather than a wrapped `JsValue`
/// so the caller can drop the `&EcsDom` borrow before invoking
/// `create_element_wrapper`.  The wrapper path mutably borrows
/// `VmInner::host_data::wrapper_cache`, which aliases the shared
/// reborrow chain used to obtain this `&EcsDom` from `HostData`;
/// splitting the phases eliminates the Stacked-Borrows violation.
pub(crate) fn try_indexed_get(
    vm: &mut VmInner,
    dom: &EcsDom,
    id: ObjectId,
    key: JsValue,
) -> Option<Entity> {
    let is_html_collection = matches!(vm.get_object(id).kind, ObjectKind::HtmlCollection);
    // Remove / re-insert mirrors `resolve_receiver_entities` so we
    // can take concurrent borrows of `vm.strings` (for the needle
    // lookup) and `dom` (for the traversal) without tripping the
    // aliasing rules.
    let (kind, cache) = vm.live_collection_states.remove(&id)?;

    // Resolve `entities` to a slice borrow.  Three branches:
    //
    // 1. `Snapshot` — slice directly into the stored `Vec`, no
    //    clone (`querySelectorAll(...)[i]` is now O(1) through
    //    here, not O(N) via the previous `entities.clone()`).
    // 2. Cache hit — clone the cached `Vec` once.  Cloning rather
    //    than re-borrowing keeps the `RefCell` borrow short so
    //    the wrapper can be re-inserted into
    //    `live_collection_states` below; the resulting `Vec` is
    //    owned by `entities_storage` for the rest of the function.
    // 3. Cache miss — materialise needles, walk descendants,
    //    refresh the cache, and own the freshly walked `Vec`.
    //
    // `entities_storage` only carries the owned `Vec` for cases 2
    // and 3; `entities: &[Entity]` is the unified slice the
    // index / named-property logic reads from.
    let entities_storage: Vec<Entity>;
    let entities: &[Entity] = if let LiveCollectionKind::Snapshot { entities } = &kind {
        entities.as_slice()
    } else if !kind.is_cacheable() {
        // Non-cacheable kinds (FormControls today) walk fresh on
        // every access; cache fields go unused.  See
        // [`LiveCollectionKind::is_cacheable`] for the rationale.
        let needles = resolve_needles(vm, &kind);
        entities_storage = resolve_entities_with_needles(dom, &kind, &needles);
        entities_storage.as_slice()
    } else {
        // Every non-Snapshot, cacheable variant has `cache_root() == Some`
        // (see [`LiveCollectionKind::cache_root`]).  Materialising
        // `cur` outside the inner branches keeps it shared between
        // the hit / miss arms.
        let cur = dom.inclusive_descendants_version(
            kind.cache_root()
                .expect("non-Snapshot cacheable variants always have cache_root() == Some"),
        );
        if cache.cached_version.get() == Some(cur) {
            entities_storage = cache.cached_entities.borrow().clone();
        } else {
            let needles = resolve_needles(vm, &kind);
            let fresh = resolve_entities_with_needles(dom, &kind, &needles);
            cache_store(&cache, Some(cur), &fresh);
            entities_storage = fresh;
        }
        entities_storage.as_slice()
    };

    let result = match key {
        JsValue::Number(n) if n.is_finite() => {
            let trunc = n.trunc();
            if (trunc - n).abs() > f64::EPSILON || trunc < 0.0 {
                None
            } else {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let idx = trunc as usize;
                entities.get(idx).copied()
            }
        }
        JsValue::String(sid) => {
            // Canonical array-index parse (ES §6.1.7 "array index" /
            // §7.1.21 CanonicalNumericIndexString): rejects
            // non-canonical forms like "01" / "+1" / "1.0" so
            // `coll['01']` does NOT alias `coll[1]`.  Reuses the
            // shared `parse_array_index_u32` helper that already
            // enforces the leading-zero + length constraints.
            let key_units = vm.strings.get(sid);
            if let Some(idx_u32) = super::super::coerce_format::parse_array_index_u32(key_units) {
                let idx = idx_u32 as usize;
                entities.get(idx).copied()
            } else if !is_html_collection {
                // Non-canonical-index string on a NodeList → no
                // named-property path (WHATWG; see HTMLCollection-
                // only `namedItem` below).
                None
            } else {
                // HTMLCollection legacy named property access
                // (WHATWG HTML §4.2.10): `id` attribute first, then
                // `name` restricted to the tag allowlist.
                let key_str = vm.strings.get_utf8(sid);
                let mut name_hit: Option<Entity> = None;
                let mut id_hit: Option<Entity> = None;
                for &e in entities {
                    if dom.with_attribute(e, "id", |v| v == Some(key_str.as_str())) {
                        id_hit = Some(e);
                        break;
                    }
                    if name_hit.is_none()
                        && dom.with_tag_name(e, |t| t.is_some_and(tag_allows_name_lookup))
                        && dom.with_attribute(e, "name", |v| v == Some(key_str.as_str()))
                    {
                        name_hit = Some(e);
                    }
                }
                id_hit.or(name_hit)
            }
        }
        _ => None,
    };

    vm.live_collection_states.insert(id, (kind, cache));
    result
}

// -------------------------------------------------------------------------
// HTMLOptionsCollection mutable surface — `length` setter, `add()`,
// `remove()` (HTML §2.7.4.2)
// -------------------------------------------------------------------------

/// Recover the parent `<select>` entity from an HTMLOptionsCollection
/// receiver.  Errors with TypeError "Illegal invocation" if `this`
/// is not an HTMLOptionsCollection.
fn require_options_collection_select(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Option<Entity>, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOptionsCollection': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::HtmlCollection) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOptionsCollection': Illegal invocation"
        )));
    }
    match ctx.vm.live_collection_states.get(&id) {
        Some((LiveCollectionKind::Options { select }, _)) => Ok(Some(*select)),
        Some(_) | None => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOptionsCollection': Illegal invocation"
        ))),
    }
}

fn native_options_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Brand-check the receiver as HTMLOptionsCollection (the
    // inherited `length` getter would also work but its brand-check
    // surfaces "HTMLCollection" in error messages — a direct
    // HTMLOptionsCollection brand keeps the trail consistent).
    let _ = require_options_collection_select(ctx, this, "length")?;
    let JsValue::Object(id) = this else {
        return Ok(JsValue::Number(0.0));
    };
    let entities = resolve_receiver_entities(ctx, id);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(entities.len() as f64))
}

fn native_options_length_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select) = require_options_collection_select(ctx, this, "length")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_len = super::super::coerce::to_uint32(ctx.vm, val)?;

    // Snapshot the current options.
    let JsValue::Object(id) = this else {
        return Ok(JsValue::Undefined);
    };
    let current = resolve_receiver_entities(ctx, id);
    let current_len = u32::try_from(current.len()).unwrap_or(u32::MAX);

    if new_len < current_len {
        // Truncate — remove options past `new_len`.  Walk in
        // reverse so each removal does not shift subsequent indices.
        let dom = ctx.host().dom();
        for &option in current.iter().skip(new_len as usize).rev() {
            if let Some(parent) = dom.get_parent(option) {
                let _ = dom.remove_child(parent, option);
            }
        }
    } else if new_len > current_len {
        // Extend — append empty `<option>` elements as direct
        // children of the `<select>`.
        let dom = ctx.host().dom();
        let to_add = new_len - current_len;
        for _ in 0..to_add {
            let opt = dom.create_element("option", elidex_ecs::Attributes::default());
            let _ = dom.append_child(select, opt);
        }
    }
    Ok(JsValue::Undefined)
}

/// `add(element, before?)` — HTML §2.7.4.2.  Inserts `element`
/// (HTMLOptionElement or HTMLOptGroupElement) into the parent
/// `<select>`.  `before` may be:
/// - missing / `null` / `undefined` — append at end.
/// - a non-negative integer — insert before the option at that index.
///   Out-of-range indices append at end (per spec step 2.4 — `before`
///   that resolves to `null` falls back to append).
/// - an HTMLOptionElement — must be a descendant of the same
///   select; `NotFoundError` otherwise.
///
/// Throws `HierarchyRequestError` if `element` is an ancestor of the
/// select (per spec step 1).
pub(super) fn native_options_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select) = require_options_collection_select(ctx, this, "add")? else {
        return Ok(JsValue::Undefined);
    };
    // First arg — element to insert.  Must be HTMLOptionElement or
    // HTMLOptGroupElement (Element wrapper).
    let element_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(element_id) = element_val else {
        return Err(VmError::type_error(
            "Failed to execute 'add' on 'HTMLOptionsCollection': \
             argument 1 is not an HTMLOptionElement or HTMLOptGroupElement"
                .to_string(),
        ));
    };
    let element_entity = match ctx.vm.get_object(element_id).kind {
        ObjectKind::HostObject { entity_bits } => match Entity::from_bits(entity_bits) {
            Some(e) => e,
            None => {
                return Err(VmError::type_error(
                    "Failed to execute 'add' on 'HTMLOptionsCollection': \
                     argument 1 is not a DOM Element"
                        .to_string(),
                ))
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'add' on 'HTMLOptionsCollection': \
                 argument 1 is not an Element"
                    .to_string(),
            ));
        }
    };
    {
        let dom = ctx.host().dom();
        let is_ok = dom.with_tag_name(element_entity, |t| {
            t.is_some_and(|s| {
                s.eq_ignore_ascii_case("option") || s.eq_ignore_ascii_case("optgroup")
            })
        });
        if !is_ok {
            return Err(VmError::type_error(
                "Failed to execute 'add' on 'HTMLOptionsCollection': \
                 argument 1 is not an HTMLOptionElement or HTMLOptGroupElement"
                    .to_string(),
            ));
        }
        // HierarchyRequestError if `element` is an ancestor of select
        // (would create a cycle on insert).
        if dom.is_ancestor_or_self(element_entity, select) {
            return Err(super::dom_exception::hierarchy_request_error(
                ctx.vm.well_known.dom_exc_hierarchy_request_error,
                "HTMLOptionsCollection",
                "add",
                "the new element is a parent of the receiver",
            ));
        }
    }

    // Resolve `before` (second arg) — missing / null / undefined →
    // append; integer → index into options; element → must be a
    // descendant of select (NotFoundError otherwise).
    let before_arg = args.get(1).copied();
    let before_entity: Option<Entity> = match before_arg {
        None | Some(JsValue::Undefined | JsValue::Null) => None,
        Some(JsValue::Number(n)) if n.is_finite() => {
            let trunc = n.trunc();
            if trunc < 0.0 {
                None
            } else {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let idx = trunc as usize;
                let JsValue::Object(coll_id) = this else {
                    return Ok(JsValue::Undefined);
                };
                let entities = resolve_receiver_entities(ctx, coll_id);
                entities.get(idx).copied()
            }
        }
        Some(JsValue::Object(before_id)) => match ctx.vm.get_object(before_id).kind {
            ObjectKind::HostObject { entity_bits } => match Entity::from_bits(entity_bits) {
                Some(be) => {
                    let dom = ctx.host().dom();
                    // Strict-descendant check: include-self via
                    // `is_ancestor_or_self`, exclude the trivial
                    // self case explicitly.
                    if be == select || !dom.is_ancestor_or_self(select, be) {
                        return Err(VmError::dom_exception(
                            ctx.vm.well_known.dom_exc_not_found_error,
                            "Failed to execute 'add' on 'HTMLOptionsCollection': \
                             reference element is not a descendant of the select",
                        ));
                    }
                    Some(be)
                }
                None => None,
            },
            _ => None,
        },
        Some(_) => {
            // Non-integer numbers / booleans / strings — coerce per
            // WebIDL `unsigned long`.  Coerce explicitly here.
            let n = super::super::coerce::to_uint32(ctx.vm, before_arg.unwrap())?;
            let JsValue::Object(coll_id) = this else {
                return Ok(JsValue::Undefined);
            };
            let entities = resolve_receiver_entities(ctx, coll_id);
            entities.get(n as usize).copied()
        }
    };

    // Perform the insertion.  Detach `element` from its current
    // parent first (insert_before within the same parent re-parents
    // by spec).
    let dom = ctx.host().dom();
    if let Some(cur_parent) = dom.get_parent(element_entity) {
        let _ = dom.remove_child(cur_parent, element_entity);
    }
    match before_entity {
        Some(b) => {
            // Insert before `b` within b's parent (which must be a
            // descendant chain of select).
            if let Some(b_parent) = dom.get_parent(b) {
                let _ = dom.insert_before(b_parent, element_entity, b);
            } else {
                let _ = dom.append_child(select, element_entity);
            }
        }
        None => {
            let _ = dom.append_child(select, element_entity);
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_options_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_options_collection_select(ctx, this, "remove")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    // Spec defers to ChildNode.remove for the option at `index`.
    // Out-of-range / non-finite index → no-op.
    let n = match val {
        JsValue::Number(n) if n.is_finite() => n.trunc(),
        _ => super::super::coerce::to_int32(ctx.vm, val)? as f64,
    };
    if n < 0.0 {
        return Ok(JsValue::Undefined);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = n as usize;
    let JsValue::Object(id) = this else {
        return Ok(JsValue::Undefined);
    };
    let entities = resolve_receiver_entities(ctx, id);
    if let Some(&option) = entities.get(idx) {
        let dom = ctx.host().dom();
        if let Some(parent) = dom.get_parent(option) {
            let _ = dom.remove_child(parent, option);
        }
    }
    Ok(JsValue::Undefined)
}

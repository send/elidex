//! DOM component types stored on ECS entities.

use std::sync::Arc;

use hecs::Entity;
use indexmap::IndexMap;

use elidex_plugin::{Point, Size, Vector};

/// Generate string-keyed map accessor methods for a struct wrapping an `IndexMap<String, String>`.
macro_rules! impl_string_map {
    ($type:ty, $field:ident, $key_label:literal) => {
        impl $type {
            #[doc = concat!("Get a ", $key_label, " value by name.")]
            pub fn get(&self, name: &str) -> Option<&str> {
                self.$field.get(name).map(String::as_str)
            }

            #[doc = concat!("Set a ", $key_label, " value. Returns the previous value if present.")]
            pub fn set(
                &mut self,
                name: impl Into<String>,
                value: impl Into<String>,
            ) -> Option<String> {
                self.$field.insert(name.into(), value.into())
            }

            #[doc = concat!("Remove a ", $key_label, " by name. Returns the removed value if present.")]
            pub fn remove(&mut self, name: &str) -> Option<String> {
                self.$field.shift_remove(name)
            }

            #[doc = concat!("Returns `true` if the ", $key_label, " exists.")]
            pub fn contains(&self, name: &str) -> bool {
                self.$field.contains_key(name)
            }

            #[doc = concat!("Iterate over all ", $key_label, " name-value pairs.")]
            pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
                self.$field.iter().map(|(k, v)| (k.as_str(), v.as_str()))
            }
        }
    };
}

/// The HTML tag name of an element (e.g., "div", "span", "a").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagType(pub String);

/// The namespace of an element, stored as a sparse ECS component.
///
/// HTML is the overwhelming default, so the component is attached **only**
/// to foreign (SVG / MathML) elements: an element with no `Namespace`
/// component is in the [`Html`](Namespace::Html) namespace by construction.
/// Read namespace via [`EcsDom::namespace_of`](crate::EcsDom::namespace_of)
/// (which resolves absence to `Html`), and create foreign elements via
/// [`EcsDom::create_element_ns`](crate::EcsDom::create_element_ns).
///
/// The three namespace URI constants are defined in the WHATWG Infra
/// Standard §8 "Namespaces".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Namespace {
    /// The HTML namespace, `http://www.w3.org/1999/xhtml`.
    #[default]
    Html,
    /// The SVG namespace, `http://www.w3.org/2000/svg`.
    Svg,
    /// The MathML namespace, `http://www.w3.org/1998/Math/MathML`.
    MathMl,
}

impl Namespace {
    /// The namespace URI string (WHATWG Infra Standard §8 "Namespaces").
    #[must_use]
    pub const fn uri(self) -> &'static str {
        match self {
            Namespace::Html => "http://www.w3.org/1999/xhtml",
            Namespace::Svg => "http://www.w3.org/2000/svg",
            Namespace::MathMl => "http://www.w3.org/1998/Math/MathML",
        }
    }
}

/// Key-value attribute map for an element.
///
/// Uses `IndexMap` to preserve insertion order, matching the WHATWG DOM spec
/// requirement that `getAttributeNames()` returns attributes in insertion order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attributes {
    map: IndexMap<String, String>,
}

impl_string_map!(Attributes, map, "attribute");

impl Attributes {
    /// Returns the number of attributes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if there are no attributes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns attribute names in insertion order.
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.map.keys().map(String::as_str).collect()
    }
}

// ---------------------------------------------------------------------------
// Base URL state (HTML §2.4.3 + §4.2.3)
// ---------------------------------------------------------------------------

/// Frozen base URL per WHATWG HTML §4.2.3 — set on each `<base>`
/// element at mutation time.
///
/// Absent (component not attached) when the element has no `href`
/// attribute. When the `href` attribute IS present, the component is
/// ALWAYS attached — per HTML §4.2.3 step 3 "if any of the following
/// are true" disjunction, the fallback case still SETS the frozen URL
/// to fallback (the only absent case is "no href attribute").
#[derive(Debug, Clone)]
pub struct BaseFrozenUrl(pub url::Url);

/// Derived document base URL — first `<base>`'s frozen URL OR the
/// fallback URL when no qualifying `<base>` exists.
///
/// Always present on Document entities (eager populate at
/// [`crate::EcsDom::create_document_root`]). Maintained by
/// `elidex_dom_api::BaseUrlMaintainer`.
#[derive(Debug, Clone)]
pub struct DocumentBaseUrl(pub url::Url);

/// Tree structure relationships linking entities into a DOM tree.
///
/// Fields are `pub(crate)` to ensure tree mutations go through [`EcsDom`]
/// methods, which enforce invariants (no cycles, consistent sibling links).
///
/// **Warning:** `Clone` is derived for internal snapshotting only. Inserting a
/// cloned `TreeRelation` as a component on a different entity will break tree
/// invariants. Always use [`EcsDom`] mutation methods instead.
///
/// [`EcsDom`]: crate::EcsDom
#[derive(Debug, Clone, Default)]
pub struct TreeRelation {
    pub(crate) parent: Option<Entity>,
    pub(crate) first_child: Option<Entity>,
    pub(crate) last_child: Option<Entity>,
    pub(crate) next_sibling: Option<Entity>,
    pub(crate) prev_sibling: Option<Entity>,
    /// Monotonically increasing version counter for live collection cache invalidation.
    /// Bumped on any mutation (child add/remove, attribute change) that affects
    /// the subtree rooted at this entity. Propagated to ancestors via `rev_version()`.
    pub(crate) inclusive_descendants_version: u64,
}

/// Text content for text nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextContent(pub String);

/// Persisted collapsed + positioned inline runs for one anonymous inline
/// formatting context (CSS 2 §9.2.1.1), keyed on the run-start entity.
///
/// Produced once by layout (`elidex-layout-block`'s `LinePacker`), consumed by
/// render's display-list builder — the single source of inline-text geometry
/// (One-issue-one-way: render no longer re-collects / re-collapses / re-measures
/// / re-positions the DOM text). Stored on the first top-level child of the
/// inline run (`run[0]`), the same entity both passes derive as the run start.
///
/// Lives in `elidex-ecs` (not `elidex-plugin`, where `LayoutBox` lives) because
/// `InlineFlowRun` references the style-owning `Entity` and `elidex-plugin` does
/// not depend on `elidex-ecs`. The referenced entities are same-`EcsDom` DOM
/// entities (not per-VM identity handles), so intra-world references are sound.
///
/// Coordinates are stored along the **inline** and **block** axes, but layout
/// applies the writing-mode projection (the same `is_vertical` rule as
/// `static_positions` / inline `LayoutBox`es) at persist, so each scalar already
/// holds the **absolute physical coordinate for its axis**: for horizontal,
/// `inline_start` = physical x and `block_start` = physical y; for vertical,
/// `inline_start` = physical y and `block_start` = physical x. Render therefore
/// reads them without a coordinate transform, selecting the right field per writing
/// mode (no vertical-rl block-axis reversal — matching the box convention).
///
/// **Fragmentation (slice 4 / I).** A run that spans fragmentainers (paged-media
/// pages; columns, once I-multicol lands) carries one [`InlineFragment`] per
/// fragmentainer it spans — the inline portion of the project's fragment tree.
/// A 1-entity → N-fragment relation fits a component, unlike the N:M
/// entity↔layer relation that forces the layer tree into a standalone structure
/// (`docs/design/en/15-rendering-pipeline.md` §15.4.1 "Layer Tree as Independent
/// Structure"). A non-fragmented run has a single fragment at generation `0`. The
/// entity→fragments relation is 1:N (still one component on the run-start entity).
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFlow {
    /// One entry per fragmentainer this IFC-run spans (a single entry for the
    /// non-fragmented common case). Render paints the fragment(s) whose
    /// [`generation`](InlineFragment::generation) matches the page being walked
    /// (`expected_generation`), all of them off the paged path.
    pub fragments: Vec<InlineFragment>,
}

impl InlineFlow {
    /// A flow with a single [`InlineFragment`] — the common case: one
    /// fragmentainer, or one paged page's per-page slice (paged writes the Vec at
    /// length-1-per-page, replacing it each page's full re-layout). `generation`
    /// is the fragmentainer discriminator (paged page number; `0` otherwise).
    #[must_use]
    pub fn single(generation: u32, lines: Vec<InlineFlowLine>) -> Self {
        Self {
            fragments: vec![InlineFragment { generation, lines }],
        }
    }
}

/// One fragmentainer's worth of an [`InlineFlow`] — the per-(IFC-run,
/// fragmentainer) collapsed + positioned line set. The inline node of the
/// project's fragment tree (the 1:N entity→fragment relation that, unlike the
/// N:M layer tree of `docs/design/en/15-rendering-pipeline.md` §15.4.1, fits an
/// ECS component).
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFragment {
    /// Fragmentainer discriminator, consumed exactly like render's per-page
    /// `expected_generation` gate: the paged-media page number, or `0` for
    /// non-paged content (a non-fragmented run, or — once I-multicol lands —
    /// multicol columns, which coexist on one surface at absolute coords). Off the
    /// paged path staleness is reconciled by layout explicitly removing the
    /// [`InlineFlow`] component, not by generation comparison.
    pub generation: u32,
    /// This fragment's line boxes in block order, continuation-rebased so the
    /// first kept line sits at the fragmentainer's block-start (absolute,
    /// already-projected coords — render reads them without a transform).
    pub lines: Vec<InlineFlowLine>,
}

/// One positioned line box within an [`InlineFragment`].
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFlowLine {
    /// Absolute block-axis offset of this line box's block-start edge — physical y
    /// (line top) for horizontal, physical x (column block-start edge) for vertical.
    pub block_start: f32,
    /// Line box block size (CSS 2 §10.8 line height calculations). Horizontal render
    /// places each run's baseline at `block_start + ascent` (the leading-naive
    /// legacy behaviour) and does not yet read this (a later slice distributes
    /// half-leading, CSS 2 §10.8.1). Vertical render **does** consume it: the glyph
    /// column center is `block_start + block_size / 2`.
    pub block_size: f32,
    /// Logical-order paintable members on this line ([`InlineFlowRun::Text`] runs
    /// and [`InlineFlowRun::AtomicBox`] inline-level boxes, interleaved in order).
    pub runs: Vec<InlineFlowRun>,
    /// `text-align: justify` within-run extra advance: render's `place_glyphs` adds
    /// this once per within-run `is_word_separator` cluster (CSS Text 3 §6.4). `0.0`
    /// for every non-justified line (the common case). Layout bakes the *between-run*
    /// expansion into each run's `inline_start` separately, so the two are disjoint;
    /// the layout/render split rationale lives on the producer (`pack::bake_justify`).
    pub justify_word_spacing: f32,
}

/// One paintable member of an [`InlineFlowLine`], stored in logical order.
///
/// A [`Text`](InlineFlowRun::Text) run is shaped and emitted at its `inline_start`;
/// an [`AtomicBox`](InlineFlowRun::AtomicBox) is painted by `walk()`-ing the entity
/// at its own (absolute) `LayoutBox`. The members are *stored* in logical order, but
/// render's current consumer (`emit_inline_flow`) does not yet paint them in a single
/// interleaved pass: it emits all `Text` runs first, then `walk()`s the collected
/// `AtomicBox` entities after the `InlineFlow` read borrow drops (`walk()` needs
/// `&mut` access). In the common case the members occupy disjoint positions, so the
/// order is not visually observable; where they DO overlap (e.g. negative margins
/// pulling an atomic box over adjacent text), the text-before-atomic order changes
/// which paints on top. A future bidi pass (slice 4) that needs strict visual order
/// would replace this with a single interleaved walk.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineFlowRun {
    /// A contiguous same-style collapsed text run.
    Text {
        /// Element/pseudo entity whose `ComputedStyle` paints this run (render
        /// re-reads colour / font / decoration / transform / opacity / spacing
        /// from it — layout owns geometry, render owns paint-time style).
        entity: Entity,
        /// Collapsed text (CSS Text 3 §4.1.1 Phase I), this line, this style-run.
        text: String,
        /// Absolute inline-axis start, `text-align` already applied — physical x
        /// for horizontal, physical y (pen top) for vertical.
        inline_start: f32,
    },
    /// A static (non-positioned) atomic inline-level box (CSS Display 3 §A
    /// `#atomic-inline`: `inline-block`/`-flex`/`-grid`/`-table` — an inline-level
    /// box that establishes its own formatting context and cannot split across
    /// lines). Render paints it by `walk()`-ing the entity at its **own** absolute
    /// `LayoutBox`, which layout repositions to `inline_start` (this line's
    /// `block_start`) at persist — so render reads the box, not this field. The
    /// box is the single source of the rendered rect (size + padding/border/margin
    /// live only there); `inline_start` records the text-align-baked inline
    /// position layout used to place the box (parallel to [`Text`](Self::Text)).
    AtomicBox {
        /// The atomic inline-level element whose `LayoutBox` holds its geometry.
        entity: Entity,
        /// Absolute inline-axis start, `text-align` already applied — the position
        /// layout repositioned the atomic's `LayoutBox` to (render paints via the
        /// box, so it does not re-read this).
        inline_start: f32,
    },
}

impl InlineFlowRun {
    /// The member's style/paint entity, common to both variants.
    #[must_use]
    pub fn entity(&self) -> Entity {
        match self {
            Self::Text { entity, .. } | Self::AtomicBox { entity, .. } => *entity,
        }
    }

    /// The member's absolute inline-axis start, common to both variants.
    #[must_use]
    pub fn inline_start(&self) -> f32 {
        match self {
            Self::Text { inline_start, .. } | Self::AtomicBox { inline_start, .. } => *inline_start,
        }
    }

    /// Mutable access to the run's inline-axis start, common to both variants
    /// (used by layout to fold IFC-local → absolute and bake the `text-align`
    /// offset uniformly across text and atomic members).
    pub fn inline_start_mut(&mut self) -> &mut f32 {
        match self {
            Self::Text { inline_start, .. } | Self::AtomicBox { inline_start, .. } => inline_start,
        }
    }

    /// The collapsed text of a [`Text`](Self::Text) member, or `None` for an
    /// [`AtomicBox`](Self::AtomicBox) (which carries no text — its content is
    /// painted by walking the box).
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } => Some(text),
            Self::AtomicBox { .. } => None,
        }
    }
}

/// Transient per-column IFC-line carrier for a multicol **mid-break** inline
/// formatting context (Z-1b, Option D — `memory/terminal-z-z1b-consume-delta.md`).
///
/// Written by `layout_inline_context_fragmented` on the **IFC container** entity
/// (the multicol direct child that breaks mid-column) for the
/// `frag_is_column && !column_is_whole` case, carrying — **per run-start group**
/// (the same `group_key` the converged [`InlineFlow`] persist uses) — this
/// column's folded [`InlineFlowLine`]s at column-0 base coords. Multicol fill
/// **drains** it (get + remove) into the column's fragment snapshot, and
/// `position_column_fragments` folds each column's lines (offset to the column's
/// inline position) into the run-start's `InlineFlow::single` — the sink the
/// existing `emit_inline_flow` consumes.
///
/// **Never read by render** — it lives only between the IFC layout (write) and the
/// multicol fill (drain) within one layout pass (transport, not state), so it is a
/// component (per-entity, `Send + Sync`, not a per-VM identity handle — the
/// side-store→component rule), not a side-store. A stray write that is never
/// drained is benign (render never reads it); the IFC reconciles it insert-or-
/// remove each pass, mirroring `clear_inline_flows`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnFlowSlice {
    /// Per-run-start folded lines for this column (Z-1b Option D). The sink
    /// `position_column_fragments` builds each run-start's `InlineFlow` from.
    pub flow_groups: Vec<(Entity, Vec<InlineFlowLine>)>,
    /// Mid-break atomics to reposition at the multicol seam (terminal-Z C-2), as
    /// `(entity, inline_abs, block_abs, unoffset_origin)` — the on-line target (IFC-
    /// absolute physical coords at **column-0 base**) plus the reposition delta basis
    /// (the un-offset margin-box origin `layout_atomic_items` returned, which differs
    /// from the box origin under an asymmetric writing mode, so it cannot be
    /// reconstructed at the seam). `position_column_fragments` adds the column's
    /// inline offset to `inline_abs` and moves each atomic's `LayoutBox` there,
    /// preserving any baked relative offset (basis = un-offset origin).
    ///
    /// Holds BOTH **static** atomics (also `AtomicBox` flow members in `flow_groups`,
    /// but their box is repositioned via this record so the seam needs no second walk
    /// of the runs) and **relpos/sticky** atomics (NOT flow members — render Layer 6
    /// paints the positioned box, so a member would double-paint). One uniform record
    /// for both: the seam moves every atomic the same way, regardless of kind.
    pub atomic_repositions: Vec<(Entity, f32, f32, Point)>,
}

/// Inline style declarations on an element.
///
/// Properties are stored in an `IndexMap` to preserve insertion order
/// (matching CSSOM `style.cssText` serialization order) while enforcing
/// uniqueness (last declaration wins, matching CSS cascade behavior).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineStyle {
    properties: IndexMap<String, String>,
}

impl_string_map!(InlineStyle, properties, "style property");

impl InlineStyle {
    /// Returns the number of properties.
    #[must_use]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Returns `true` if there are no properties.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Serialize all properties to a CSS text string.
    #[must_use]
    pub fn css_text(&self) -> String {
        self.properties
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Get the property name at the given index (insertion order).
    #[must_use]
    pub fn property_at(&self, index: usize) -> Option<&str> {
        self.properties.keys().nth(index).map(String::as_str)
    }
}

/// Marker component for pseudo-element entities (`::before`, `::after`).
///
/// Pseudo-element entities are generated during style resolution and
/// inserted as children of the originating element. They carry a
/// `ComputedStyle` and `TextContent` but are not real DOM elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PseudoElementMarker;

/// Resolved list-item marker text (CSS Lists 3 §4.6 implicit `list-item`
/// counter, §4.7 `counter()` formatting) for a `display: list-item` element.
///
/// Written by the pre-layout generated-content pass (`elidex-style`) as the
/// single source of marker text (One-issue-one-way: render no longer runs the
/// counter machine to derive it), read by render's marker paint. The pass
/// **reconciles** this component every resolve — inserting it on a visible-type
/// list-item, removing it otherwise — so a stale value never survives an element
/// that stops being a list-item (the same explicit-clear discipline as
/// [`InlineFlow`]; element entities persist across re-resolves, unlike pseudo
/// entities which are regenerated). Render owns the `visibility` paint guard, so
/// this is written independently of the element's visibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItemMarker(pub String);

/// Dynamic element state flags for CSS pseudo-class matching.
///
/// Tracks whether an element is hovered, focused, active, or a link.
/// Used by the selector engine to match `:hover`, `:focus`, `:active`,
/// `:link`, `:visited`, and form-related pseudo-classes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ElementState(pub u16);

impl ElementState {
    pub const HOVER: u16 = 0x0001;
    pub const FOCUS: u16 = 0x0002;
    pub const ACTIVE: u16 = 0x0004;
    pub const LINK: u16 = 0x0008;
    pub const VISITED: u16 = 0x0010;
    pub const DISABLED: u16 = 0x0020;
    pub const CHECKED: u16 = 0x0040;
    pub const REQUIRED: u16 = 0x0080;
    pub const VALID: u16 = 0x0100;
    pub const INVALID: u16 = 0x0200;
    pub const READ_ONLY: u16 = 0x0400;
    pub const INDETERMINATE: u16 = 0x0800;

    /// Returns `true` if the given flag is set.
    #[must_use]
    pub fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// Set the given flag.
    pub fn insert(&mut self, flag: u16) {
        self.0 |= flag;
    }

    /// Clear the given flag.
    pub fn remove(&mut self, flag: u16) {
        self.0 &= !flag;
    }

    /// Set or clear the given flag based on `value`.
    pub fn set(&mut self, flag: u16, value: bool) {
        if value {
            self.insert(flag);
        } else {
            self.remove(flag);
        }
    }

    /// Returns `true` if no flags are set.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Shadow root mode (WHATWG DOM §4.8).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowRootMode {
    /// Shadow root is accessible via `element.shadowRoot`.
    Open,
    /// Shadow root is not accessible via `element.shadowRoot`.
    Closed,
}

/// Slot assignment mode for shadow roots (WHATWG DOM §4.8).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SlotAssignmentMode {
    /// Slots are assigned by matching `<slot name>` attributes (default).
    #[default]
    Named,
    /// Slots are assigned manually via `slot.assign()`.
    Manual,
}

/// Marker: this entity is a shadow root.
///
/// A shadow root is a document fragment attached to a host element.
/// It provides style encapsulation and DOM isolation (WHATWG DOM §4.8).
#[derive(Clone, Copy, Debug)]
pub struct ShadowRoot {
    /// Open or closed mode.
    pub mode: ShadowRootMode,
    /// The host element that owns this shadow root.
    pub host: Entity,
    /// Whether focus is delegated to the first focusable element in the shadow tree.
    pub delegates_focus: bool,
    /// How slots are assigned to light DOM children.
    pub slot_assignment: SlotAssignmentMode,
    /// Whether the shadow root is cloned by `Node.cloneNode` (WHATWG DOM §4.8).
    /// Behavior propagation through cloneNode is deferred to slot
    /// `#11-shadow-clone-serialize-propagation`; the field stores the
    /// init-time flag for feature-detection round-trip.
    pub clonable: bool,
    /// Whether the shadow root is serialized by HTML fragment serialization
    /// algorithms when `serializableShadowRoots=true` (WHATWG HTML §2.7.3).
    /// Behavior propagation is deferred to slot
    /// `#11-shadow-clone-serialize-propagation`; the field stores the
    /// init-time flag for feature-detection round-trip.
    pub serializable: bool,
}

/// Marker: this element hosts a shadow root.
///
/// Attached to elements that have had `attachShadow()` called on them.
#[derive(Clone, Copy, Debug)]
pub struct ShadowHost {
    /// The shadow root entity.
    pub shadow_root: Entity,
}

/// Slot assignment for distributed nodes.
///
/// Attached to `<slot>` entities in the shadow tree.
/// Contains the list of light DOM nodes distributed to this slot.
#[derive(Debug, Default)]
pub struct SlotAssignment {
    /// Light DOM nodes assigned to this slot, in order.
    pub assigned_nodes: Vec<Entity>,
}

/// Marker attached to light DOM nodes that have been distributed to a slot.
///
/// Added by `distribute_slots()` for O(1) slotted-element checks in
/// selector matching and event retargeting.
#[derive(Clone, Copy, Debug)]
pub struct SlottedMarker;

/// Marker for `<template>` elements (inert — not rendered/styled).
///
/// Template content is not part of the rendered document. Elements
/// with this marker are excluded from style resolution and rendering.
#[derive(Clone, Copy, Debug)]
pub struct TemplateContent;

/// Marker set on `<dialog>` entities by `dialog.showModal()`, cleared
/// by `dialog.close()` (HTML §4.11.4, slot `#11-tags-T2d-interactive`).
///
/// Render-side top-layer / focus-management consumption is deferred to
/// slot `#11-dialog-top-layer` (Phase 4 shell pairing).
#[derive(Clone, Copy, Debug)]
pub struct IsModalDialog;

/// Per-`<dialog>` return-value state (HTML §4.11.4).  Defaults to the
/// empty string; updated by `dialog.returnValue` setter or by the
/// optional argument to `dialog.close(returnValue?)`.  Slot
/// `#11-tags-T2d-interactive`.
#[derive(Clone, Debug, Default)]
pub struct DialogReturnValue(pub String);

/// Per-`<output>` default-value state (HTML §4.10.13).  Stored separately
/// from the rendered text so that form reset can restore the displayed
/// content from the default snapshot.  Slot `#11-tags-T2d-interactive`.
#[derive(Clone, Debug, Default)]
pub struct OutputDefaultValue(pub String);

/// Per-`<output>` value-mode override (HTML §4.10.13).  `None` means
/// the element is in default mode and reads/writes go through
/// `textContent`; `Some(_)` means the `value` IDL setter has been
/// called and the explicit override is the source of truth.  Cleared
/// by form reset.  Slot `#11-tags-T2d-interactive`.
#[derive(Clone, Debug, Default)]
pub struct OutputValueOverride(pub Option<String>);

/// Decoded image pixel data for `<img>` elements.
///
/// Stored as a component on image entities after the image has been
/// fetched and decoded. Pixel data is RGBA8 format (4 bytes per pixel).
#[derive(Debug, Clone)]
pub struct ImageData {
    /// RGBA8 pixel data (width × height × 4 bytes).
    ///
    /// Wrapped in `Arc` so that the display list can share the data
    /// without cloning the entire pixel buffer every frame.
    pub pixels: Arc<Vec<u8>>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// External CSS loaded for a `<link rel="stylesheet">` element
/// (HTML §4.6.7 — a `<link>` whose resource is successfully fetched and
/// parsable has an *associated CSS style sheet*; that association is what
/// `document.styleSheets` enumerates, CSSOM §6.8).
///
/// Attached by the resource loader after a successful fetch. The element
/// is the source of truth for its loaded sheet: the CSSOM stylesheet
/// walker, the per-entity stylesheet cache, and `link.sheet` all read it.
/// Absent when the link is not a stylesheet, has no href, or the fetch
/// failed — so component presence == "associated CSS style sheet exists".
#[derive(Clone, Debug)]
pub struct LinkStylesheet {
    /// Raw CSS source text as fetched (parsed lazily by the CSSOM cache,
    /// mirroring how `<style>` text content is parsed lazily).
    pub source: String,
    /// Resolved absolute URL of the linked sheet (CSSOM §6.2
    /// `StyleSheet.href`).
    pub href: String,
    /// Monotonic version, bumped on each write (loader attach +
    /// CSSOM `insertRule`/`deleteRule` flush). The per-entity stylesheet
    /// cache uses it as the O(1) divergence key, since a void `<link>`
    /// has no `inclusive_descendants_version` child-mutation signal.
    pub version: u64,
}

/// Decoded background image layers for CSS `background-image`.
///
/// Each entry corresponds to a background layer. `None` entries indicate
/// layers that are not URL-based (e.g. gradients, or `none`).
#[derive(Debug, Clone)]
pub struct BackgroundImages {
    /// Per-layer decoded image data. `None` = gradient or none.
    pub layers: Vec<Option<Arc<ImageData>>>,
}

/// The kind of DOM node (WHATWG DOM §4.4 Node.nodeType).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeKind {
    /// Element node (nodeType = 1).
    Element,
    /// Attribute node (nodeType = 2).
    Attribute,
    /// Text node (nodeType = 3).
    Text,
    /// CDATA section node (nodeType = 4).
    CdataSection,
    /// Processing instruction node (nodeType = 7).
    ProcessingInstruction,
    /// Comment node (nodeType = 8).
    Comment,
    /// Document node (nodeType = 9).
    Document,
    /// Document type node (nodeType = 10).
    DocumentType,
    /// Document fragment node (nodeType = 11).
    DocumentFragment,
    /// Window object (WHATWG HTML §7.2).
    ///
    /// Not a Node per WHATWG DOM — has no `nodeType` — but tracked as a
    /// `NodeKind` so that the scripting layer can distinguish the global
    /// `window` entity from DOM tree entities with a uniform component query.
    /// Window entities carry only this component (no `TreeRelation`,
    /// `Attributes`, etc.) and never participate in tree traversal.
    Window,
    /// Dedicated worker global scope (WHATWG HTML §10.2.1.1
    /// `WorkerGlobalScope`).
    ///
    /// Like [`NodeKind::Window`], this is not a Node and has no `nodeType`.
    /// It marks two distinct (non-tree) entities the scripting layer attaches
    /// `EventListeners` to: on a **worker** `Vm`, the single worker-global-scope
    /// entity (the realm's Window analog); and on the **main** `Vm`, one entity
    /// per main-side `Worker` object (the parent's handle, brand-keyed by a
    /// `WorkerRef` component). So a worker VM has exactly one, but a main VM may
    /// have many. Such entities carry no `TreeRelation` and never participate in
    /// tree traversal.
    Worker,
    /// `OffscreenCanvas` object (WHATWG HTML §4.12.5.3 "The OffscreenCanvas
    /// interface").
    ///
    /// Like [`NodeKind::Window`] and [`NodeKind::Worker`], this is not a Node
    /// and has no `nodeType`. It marks an entity that hosts a detached 2D
    /// rendering target: one entity per `new OffscreenCanvas(w, h)` call OR per
    /// `HTMLCanvasElement.transferControlToOffscreen()` invocation. Such
    /// entities carry no `TreeRelation`, never participate in tree traversal,
    /// and (in v1, main-thread-only scope) live in the main `EcsDom`. The
    /// scripting layer brand-checks via `(NodeKind::OffscreenCanvas + HostObject
    /// over the entity)`, mirror of the `NodeKind::Worker` brand pattern.
    OffscreenCanvas,
}

impl NodeKind {
    /// Returns the WHATWG `Node.nodeType` numeric value.
    ///
    /// Returns `0` for [`NodeKind::Window`], which has no `nodeType`
    /// (Window is not a Node per WHATWG). `from_node_type(0)` is therefore
    /// `None`, i.e. Window is deliberately excluded from the round-trip.
    #[must_use]
    pub fn node_type(self) -> u32 {
        match self {
            Self::Element => 1,
            Self::Attribute => 2,
            Self::Text => 3,
            Self::CdataSection => 4,
            Self::ProcessingInstruction => 7,
            Self::Comment => 8,
            Self::Document => 9,
            Self::DocumentType => 10,
            Self::DocumentFragment => 11,
            Self::Window | Self::Worker | Self::OffscreenCanvas => 0,
        }
    }

    /// Whether this kind is a Node per WHATWG DOM (has a `nodeType`).
    ///
    /// `false` for [`NodeKind::Window`], [`NodeKind::Worker`], and
    /// [`NodeKind::OffscreenCanvas`] — all three are EventTargets but **not**
    /// Nodes (`nodeType == 0`). Node-argument coercion (`appendChild` /
    /// `insertBefore` / `ChildNode` / `ParentNode` etc.) must reject non-Node
    /// kinds so a `window` / `Worker` / `OffscreenCanvas` object cannot be
    /// inserted into the DOM tree.
    #[must_use]
    pub fn is_node(self) -> bool {
        self.node_type() != 0
    }

    /// Create a `NodeKind` from a WHATWG `Node.nodeType` numeric value.
    #[must_use]
    pub fn from_node_type(node_type: u32) -> Option<Self> {
        match node_type {
            1 => Some(Self::Element),
            2 => Some(Self::Attribute),
            3 => Some(Self::Text),
            4 => Some(Self::CdataSection),
            7 => Some(Self::ProcessingInstruction),
            8 => Some(Self::Comment),
            9 => Some(Self::Document),
            10 => Some(Self::DocumentType),
            11 => Some(Self::DocumentFragment),
            _ => None,
        }
    }
}

/// Data for a comment node (`<!-- ... -->`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentData(pub String);

/// Data for a document type node (`<!DOCTYPE ...>`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocTypeData {
    /// The `name` part of the doctype (e.g. `"html"`).
    pub name: String,
    /// The `publicId` part of the doctype.
    pub public_id: String,
    /// The `systemId` part of the doctype.
    pub system_id: String,
}

/// WHATWG DOM §4.4 node document pointer — the `Document` entity a
/// node was created in.
///
/// Component-based storage mirrors WHATWG's per-node "node document"
/// slot: it is set by `document.createElement` / `createTextNode` /
/// `createComment` / `createDocumentFragment` (and propagated through
/// `cloneNode`) so that queries like `Node.prototype.ownerDocument`
/// can report the *creating* document even while the node is still
/// detached.  The tree-root walk used before this component was
/// introduced returned the *bound* global document, which is wrong
/// for nodes created by a secondary Document (e.g. `doc.cloneNode`).
///
/// Absence of the component on a given entity is still valid: the
/// caller is expected to fall back to [`crate::EcsDom::find_tree_root`],
/// which matches the pre-component behaviour exactly and keeps legacy
/// fixtures (html5ever-produced trees, layout-only tests) working
/// without migration.
///
/// See [`crate::EcsDom::owner_document`] for the fallback-aware accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssociatedDocument(pub Entity);

/// Scroll state for elements with `overflow: scroll | auto | hidden`.
///
/// Tracks the current scroll position and content/client dimensions
/// for scroll containers (CSS Overflow L3 §3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScrollState {
    /// Current scroll offset in CSS pixels (displacement from origin).
    pub scroll_offset: Vector,
    /// Total scrollable content size.
    pub scroll_size: Size,
    /// Visible client area size (padding box minus scrollbar).
    pub client_size: Size,
}

impl ScrollState {
    /// Create a new `ScrollState` with the given dimensions.
    #[must_use]
    pub fn new(
        scroll_width: f32,
        scroll_height: f32,
        client_width: f32,
        client_height: f32,
    ) -> Self {
        Self {
            scroll_offset: Vector::<f32>::ZERO,
            scroll_size: Size {
                width: scroll_width,
                height: scroll_height,
            },
            client_size: Size {
                width: client_width,
                height: client_height,
            },
        }
    }

    /// Maximum horizontal scroll offset (clamped to 0).
    #[must_use]
    pub fn max_scroll_x(&self) -> f32 {
        (self.scroll_size.width - self.client_size.width).max(0.0)
    }

    /// Maximum vertical scroll offset (clamped to 0).
    #[must_use]
    pub fn max_scroll_y(&self) -> f32 {
        (self.scroll_size.height - self.client_size.height).max(0.0)
    }

    /// Clamp scroll offsets to valid range.
    pub fn clamp_scroll(&mut self) {
        self.scroll_offset.x = self.scroll_offset.x.clamp(0.0, self.max_scroll_x());
        self.scroll_offset.y = self.scroll_offset.y.clamp(0.0, self.max_scroll_y());
    }
}

/// Marker for anonymous table objects (CSS 2.1 §17.2.1).
///
/// Re-layout reuses existing entities with this marker to prevent
/// duplicate anonymous box generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnonymousTableMarker;

/// Data for an Attr node (WHATWG DOM §4.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrData {
    /// The attribute's local name.
    pub local_name: String,
    /// The attribute's value.
    pub value: String,
    /// The element that owns this attribute, if any.
    pub owner_element: Option<Entity>,
}

/// Cache mapping attribute names to their `Attr` entity representations.
///
/// Attached to element entities so that `getAttributeNode("x")` returns the
/// same `Attr` entity on repeated calls (WHATWG DOM identity semantics).
#[derive(Debug, Clone, Default)]
pub struct AttrEntityCache {
    /// Maps lowercase attribute name to the `Attr` entity.
    pub entries: std::collections::HashMap<String, Entity>,
}

/// Loading attribute for `<iframe>` and `<img>` elements (WHATWG HTML §4.8.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoadingAttribute {
    /// Load immediately (default).
    #[default]
    Eager,
    /// Defer loading until near the viewport (lazy loading).
    Lazy,
}

/// Data for an `<iframe>` element (WHATWG HTML §4.8.5).
///
/// Stored as an ECS component on iframe entities. Used by layout for
/// intrinsic sizing (replaced element model) and by the shell for
/// iframe lifecycle management.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct IframeData {
    /// URL to load (`src` attribute).
    pub src: Option<String>,
    /// Inline HTML content (`srcdoc` attribute).
    pub srcdoc: Option<String>,
    /// Raw sandbox attribute value.
    pub sandbox: Option<String>,
    /// Iframe width in CSS pixels (HTML attribute, default 300).
    pub width: u32,
    /// Iframe height in CSS pixels (HTML attribute, default 150).
    pub height: u32,
    /// Frame name for targeting (`name` attribute).
    pub name: Option<String>,
    /// Loading strategy.
    pub loading: LoadingAttribute,
    /// Whether fullscreen is allowed (`allowfullscreen` attribute).
    pub allow_fullscreen: bool,
    /// Referrer policy (`referrerpolicy` attribute).
    pub referrer_policy: Option<String>,
    /// Permissions policy (`allow` attribute).
    pub allow: Option<String>,
    /// Whether credentials are suppressed (`credentialless` attribute).
    pub credentialless: bool,
}

impl IframeData {
    /// Create `IframeData` from HTML attributes.
    ///
    /// Centralizes the attribute-to-field mapping used by both the HTML parser
    /// and JS `setAttribute` handling.
    #[must_use]
    pub fn from_attributes(attrs: &Attributes) -> Self {
        Self {
            src: attrs.get("src").map(String::from),
            srcdoc: attrs.get("srcdoc").map(String::from),
            sandbox: attrs.get("sandbox").map(String::from),
            width: attrs
                .get("width")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            height: attrs
                .get("height")
                .and_then(|v| v.parse().ok())
                .unwrap_or(150),
            name: attrs.get("name").map(String::from),
            loading: if attrs
                .get("loading")
                .is_some_and(|v| v.eq_ignore_ascii_case("lazy"))
            {
                LoadingAttribute::Lazy
            } else {
                LoadingAttribute::Eager
            },
            allow_fullscreen: attrs.contains("allowfullscreen"),
            referrer_policy: attrs.get("referrerpolicy").map(String::from),
            allow: attrs.get("allow").map(String::from),
            credentialless: attrs.contains("credentialless"),
        }
    }
}

impl Default for IframeData {
    fn default() -> Self {
        Self {
            src: None,
            srcdoc: None,
            sandbox: None,
            width: 300,
            height: 150,
            name: None,
            loading: LoadingAttribute::Eager,
            allow_fullscreen: false,
            referrer_policy: None,
            allow: None,
            credentialless: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_state_new_and_defaults() {
        let s = ScrollState::new(500.0, 1000.0, 300.0, 400.0);
        assert_eq!(s.scroll_offset, Vector::<f32>::ZERO);
        assert_eq!(s.scroll_size.width, 500.0);
        assert_eq!(s.scroll_size.height, 1000.0);
        assert_eq!(s.client_size.width, 300.0);
        assert_eq!(s.client_size.height, 400.0);

        let d = ScrollState::default();
        assert_eq!(d.scroll_offset, Vector::<f32>::ZERO);
        assert_eq!(d.scroll_size.width, 0.0);
    }

    #[test]
    fn scroll_state_clamp() {
        let mut s = ScrollState::new(500.0, 1000.0, 300.0, 400.0);
        s.scroll_offset = Vector::new(999.0, -10.0);
        s.clamp_scroll();
        assert!((s.scroll_offset.x - 200.0).abs() < f32::EPSILON);
        assert_eq!(s.scroll_offset.y, 0.0);
    }

    #[test]
    fn scroll_state_max_scroll_zero_content() {
        let s = ScrollState::new(100.0, 50.0, 200.0, 100.0);
        assert_eq!(s.max_scroll_x(), 0.0);
        assert_eq!(s.max_scroll_y(), 0.0);
    }
}

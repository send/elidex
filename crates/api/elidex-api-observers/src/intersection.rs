//! `IntersectionObserver` API (Intersection Observer §3).
//!
//! Observes visibility changes of elements relative to a root element or viewport.
//!
//! ECS-native model: each observation lives as an `IntersectionObservedBy`
//! component on the observed target entity (carrying the per-observer
//! last-reported ratio for threshold-crossing detection), so a despawned entity
//! drops its observations automatically. The registry holds the id counter plus
//! the per-observer `IntersectionObserverInit` configuration.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::Rect;

/// Geometry source for `gather_observations`: returns the current bounding rect
/// for an entity. Takes `&EcsDom` so it can read layout without aliasing the
/// `&mut EcsDom` write-back borrow.
type RectProvider<'a> = dyn Fn(&EcsDom, Entity) -> Option<Rect> + 'a;

/// A unique identifier for an intersection observer registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntersectionObserverId(u64);

impl IntersectionObserverId {
    /// Create an ID from a raw u64 value.
    #[must_use]
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw u64 value.
    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Options for creating an `IntersectionObserver`.
#[derive(Debug, Clone, Default)]
pub struct IntersectionObserverInit {
    /// The root element (None = viewport).
    pub root: Option<Entity>,
    /// Root margin (CSS margin syntax, e.g. "10px 20px").
    pub root_margin: String,
    /// Thresholds at which to trigger (0.0 to 1.0).
    pub threshold: Vec<f64>,
}

/// An intersection observation entry (Intersection Observer §3.4).
#[derive(Debug, Clone)]
pub struct IntersectionObserverEntry {
    /// The observed element.
    pub target: Entity,
    /// The intersection ratio (0.0 to 1.0).
    pub intersection_ratio: f64,
    /// Whether the target is intersecting.
    pub is_intersecting: bool,
    /// `boundingClientRect`: the target's bounding rect (zero-size for a
    /// box-less / unrendered target).
    pub bounding_client_rect: Rect,
    /// `intersectionRect`: target ∩ root-with-margin (zero when disjoint).
    pub intersection_rect: Rect,
    /// `rootBounds`: the root intersection rect with `rootMargin` applied.
    /// `None` only for the cross-origin implicit-root case (deferred slot
    /// `#11-intersection-observer-cross-origin-rootbounds`); same-origin
    /// always reports the rect.
    pub root_bounds: Option<Rect>,
}

/// A single observation on a node (one per observer watching this entity).
#[derive(Debug, Clone)]
struct IntersectionObservation {
    observer: IntersectionObserverId,
    /// Last reported intersection ratio (for threshold-crossing detection).
    last_ratio: Option<f64>,
}

/// Per-node component listing the intersection observers watching this entity.
/// Dropped automatically when the entity is despawned.
#[derive(Debug, Default)]
struct IntersectionObservedBy(Vec<IntersectionObservation>);

/// Internal per-observer config — pairs the user-supplied
/// [`IntersectionObserverInit`] with its pre-parsed `rootMargin`
/// shorthand.  `rootMargin` is parsed once at `register` time so
/// `gather_observations` (a per-frame hot path) does not re-split +
/// re-parse the string for every observed target.
#[derive(Debug)]
struct RegisteredObserver {
    init: IntersectionObserverInit,
    parsed_root_margin: [MarginComponent; 4],
}

/// Registry for active intersection observers.
#[derive(Debug, Default)]
pub struct IntersectionObserverRegistry {
    next_id: u64,
    observers: std::collections::HashMap<IntersectionObserverId, RegisteredObserver>,
}

impl IntersectionObserverRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new intersection observer, returning its ID.
    ///
    /// Parses `init.root_margin` at register-time (W3C Intersection
    /// Observer §3.1 ctor step — `SyntaxError` if the shorthand is
    /// not a valid `<length-percentage>{1,4}`); the parsed
    /// `[MarginComponent; 4]` is cached on the registered observer so
    /// `gather_observations` does not re-parse on every per-target
    /// frame tick.
    ///
    /// # Errors
    ///
    /// Returns `Err(RootMarginParseError)` if `init.root_margin`
    /// contains an unrecognised unit (`em` / `vh` / etc.), a
    /// non-finite numeric, or more than 4 whitespace-split components.
    /// The host wraps this in an interface-scoped `SyntaxError`
    /// thrown from the JS constructor.
    pub fn register(
        &mut self,
        mut init: IntersectionObserverInit,
    ) -> Result<IntersectionObserverId, RootMarginParseError> {
        let parsed_root_margin = parse_root_margin(&init.root_margin)?;
        // Canonicalise threshold here (spec §3.1: "If options.threshold
        // is not present, set it to [0]") so `gather_observations`'s
        // hot path can use the slice unconditionally — the host-side
        // constructor and any crate-only caller (test harness, future
        // bindings) both get the same default.
        if init.threshold.is_empty() {
            init.threshold = vec![0.0];
        }
        let id = IntersectionObserverId(self.next_id);
        self.next_id += 1;
        self.observers.insert(
            id,
            RegisteredObserver {
                init,
                parsed_root_margin,
            },
        );
        Ok(id)
    }

    /// Start observing a target (Intersection Observer §2.2 `observe(target)`).
    /// Re-observing the same target is a no-op.
    pub fn observe(&mut self, dom: &mut EcsDom, id: IntersectionObserverId, target: Entity) {
        // Ignore observe for an id with no init config (e.g. unregistered):
        // a stale `IntersectionObservedBy` would be scanned each gather but
        // never delivered (gather skips missing init). Restores the
        // pre-refactor registry-lookup guard.
        if !self.observers.contains_key(&id) {
            return;
        }
        if let Ok(mut comp) = dom.world_mut().get::<&mut IntersectionObservedBy>(target) {
            if !comp.0.iter().any(|o| o.observer == id) {
                comp.0.push(IntersectionObservation {
                    observer: id,
                    last_ratio: None,
                });
            }
            return;
        }
        let _ = dom.world_mut().insert_one(
            target,
            IntersectionObservedBy(vec![IntersectionObservation {
                observer: id,
                last_ratio: None,
            }]),
        );
    }

    /// Stop observing a specific target (Intersection Observer §2.2 `unobserve(target)`).
    pub fn unobserve(&mut self, dom: &mut EcsDom, id: IntersectionObserverId, target: Entity) {
        let mut now_empty = false;
        if let Ok(mut comp) = dom.world_mut().get::<&mut IntersectionObservedBy>(target) {
            comp.0.retain(|o| o.observer != id);
            now_empty = comp.0.is_empty();
        }
        if now_empty {
            let _ = dom.world_mut().remove_one::<IntersectionObservedBy>(target);
        }
    }

    /// Stop observing all targets for this observer (Intersection Observer §2.2 `disconnect()`).
    pub fn disconnect(&mut self, dom: &mut EcsDom, id: IntersectionObserverId) {
        let mut emptied: Vec<Entity> = Vec::new();
        for (entity, comp) in &mut dom
            .world_mut()
            .query::<(Entity, &mut IntersectionObservedBy)>()
        {
            comp.0.retain(|o| o.observer != id);
            if comp.0.is_empty() {
                emptied.push(entity);
            }
        }
        for entity in emptied {
            let _ = dom.world_mut().remove_one::<IntersectionObservedBy>(entity);
        }
    }

    /// Remove the observer entirely (drops its registrations and config).
    pub fn unregister(&mut self, dom: &mut EcsDom, id: IntersectionObserverId) {
        self.disconnect(dom, id);
        self.observers.remove(&id);
    }

    /// Gather intersection observations by computing intersection ratios
    /// (Intersection Observer §3.2.8 "Run the Update Intersection Observations
    /// Steps").
    ///
    /// `rect_fn` provides the current bounding rect for an entity, taking
    /// `&EcsDom` so it can read layout without aliasing the `&mut EcsDom`
    /// write-back borrow. `viewport` is the root viewport rect. Returns
    /// `(observer_id, entries)` pairs for observers with threshold crossings.
    pub fn gather_observations(
        &mut self,
        dom: &mut EcsDom,
        rect_fn: &RectProvider<'_>,
        viewport: Rect,
    ) -> Vec<(IntersectionObserverId, Vec<IntersectionObserverEntry>)> {
        // BTreeMap keyed by the monotonic observer id → iteration (and the
        // returned Vec) is in registration order by construction.
        let mut grouped: std::collections::BTreeMap<
            IntersectionObserverId,
            Vec<IntersectionObserverEntry>,
        > = std::collections::BTreeMap::new();
        let mut changes: Vec<(Entity, IntersectionObserverId, f64)> = Vec::new();

        // Memoised per-observer (post-rootMargin) root rect.  The
        // root rect + parsed margin vary by observer, not by target,
        // so walking N targets × M observers would otherwise call
        // `apply_root_margin` (and `rect_fn` for the explicit-root
        // case) N×M times per frame.  Lazy-fill so a registry holding
        // M observers with only K actually observing anything pays
        // K × resolve cost, not M.
        let mut resolved_roots: std::collections::HashMap<IntersectionObserverId, Rect> =
            std::collections::HashMap::new();

        // Phase A: read observations + compute ratios (shared borrows only —
        // the `&IntersectionObservedBy` query and `rect_fn`'s layout reads
        // touch disjoint components, so they coexist).
        //
        // A box-less target (`rect_fn` → None: display:none / pre-layout) is
        // NOT skipped: per Intersection Observer §3.1 (observe) the first
        // "update intersection observations" run must still deliver an initial
        // entry (ratio 0 / isIntersecting false / zero rects), so we treat the
        // missing box as a zero-area target and run the same crossing logic.
        for (entity, comp) in &mut dom.world().query::<(Entity, &IntersectionObservedBy)>() {
            let maybe_rect = rect_fn(&*dom, entity);
            for obs in &comp.0 {
                let Some(reg) = self.observers.get(&obs.observer) else {
                    continue;
                };
                let root_rect = *resolved_roots.entry(obs.observer).or_insert_with(|| {
                    apply_root_margin(
                        reg.init
                            .root
                            .and_then(|r| rect_fn(&*dom, r))
                            .unwrap_or(viewport),
                        &reg.parsed_root_margin,
                    )
                });
                // `threshold` is canonicalised to `[0.0]` at register time
                // (constructor — see `parse_intersection_observer_init`),
                // so the per-target slice access here is unconditional.
                let thresholds = reg.init.threshold.as_slice();
                // Compute the overlap rect ONCE and derive both the
                // ratio and the spec-mandated `intersectionRect` from
                // it — previously the intersection was computed twice
                // (once for the ratio, once for the entry's
                // `intersectionRect`).  IO §3.7: `isIntersecting` is
                // driven by whether intersection is non-null (edge-
                // adjacent / zero-area overlaps still count), NOT by
                // `ratio > 0`; and a zero-area target reports
                // ratio = 1 when intersecting.
                let (ratio, is_intersecting, bounding_client_rect, intersection_rect) =
                    match maybe_rect {
                        Some(target_rect) => {
                            let inter_opt = target_rect.intersection_inclusive(&root_rect);
                            let is_intersecting = inter_opt.is_some();
                            let inter = inter_opt.unwrap_or_default();
                            let ratio = ratio_from_overlap(
                                target_rect.size.area_f64(),
                                &inter,
                                is_intersecting,
                            );
                            (ratio, is_intersecting, target_rect, inter)
                        }
                        None => (0.0, false, Rect::default(), Rect::default()),
                    };
                let last = obs.last_ratio.unwrap_or(-1.0);
                if crossed_threshold(last, ratio, thresholds) {
                    changes.push((entity, obs.observer, ratio));
                    grouped
                        .entry(obs.observer)
                        .or_default()
                        .push(IntersectionObserverEntry {
                            target: entity,
                            intersection_ratio: ratio,
                            is_intersecting,
                            bounding_client_rect,
                            intersection_rect,
                            root_bounds: Some(root_rect),
                        });
                }
            }
        }

        // Phase B: write back last ratios (exclusive borrow).
        for (entity, observer, ratio) in changes {
            if let Ok(mut comp) = dom.world_mut().get::<&mut IntersectionObservedBy>(entity) {
                if let Some(obs) = comp.0.iter_mut().find(|o| o.observer == observer) {
                    obs.last_ratio = Some(ratio);
                }
            }
        }

        grouped.into_iter().collect()
    }
}

/// Compute the intersection ratio from a pre-resolved target area and
/// overlap rect.  Single canonical formula shared by
/// `gather_observations` (production hot path) and the
/// `#[cfg(test)]` wrapper [`compute_intersection_ratio`], so unit
/// tests and production can never diverge.
///
/// IO §3.7: when `target_area == 0` the area-ratio is undefined (0/0);
/// the spec collapses it to `1.0` if the target is intersecting (a
/// zero-area point inside the root is fully visible) and `0.0`
/// otherwise.
fn ratio_from_overlap(target_area: f64, overlap: &Rect, is_intersecting: bool) -> f64 {
    if target_area <= 0.0 {
        return if is_intersecting { 1.0 } else { 0.0 };
    }
    (overlap.size.area_f64() / target_area).clamp(0.0, 1.0)
}

/// Convenience wrapper for unit tests that exercise the ratio formula
/// directly without the surrounding gather machinery.  Delegates to
/// the shared [`ratio_from_overlap`] used by production.
#[cfg(test)]
fn compute_intersection_ratio(target: Rect, root: Rect) -> f64 {
    let inter_opt = target.intersection_inclusive(&root);
    let is_intersecting = inter_opt.is_some();
    let overlap = inter_opt.unwrap_or_default();
    ratio_from_overlap(target.size.area_f64(), &overlap, is_intersecting)
}

/// Check if the ratio transition crosses any threshold.
fn crossed_threshold(old: f64, new: f64, thresholds: &[f64]) -> bool {
    for &t in thresholds {
        let old_above = old >= t;
        let new_above = new >= t;
        if old_above != new_above {
            return true;
        }
    }
    false
}

/// One `rootMargin` component: an absolute px length or a percentage resolved
/// against the root dimension (Intersection Observer §3.1 `rootMargin`).
#[derive(Debug, Clone, Copy)]
enum MarginComponent {
    Px(f32),
    Pct(f32),
}

impl MarginComponent {
    /// Resolve to px against `basis` (root width for left/right, root height
    /// for top/bottom).
    fn resolve(self, basis: f32) -> f32 {
        match self {
            Self::Px(v) => v,
            Self::Pct(p) => basis * p / 100.0,
        }
    }
}

/// `rootMargin` parse failure (W3C Intersection Observer §3.1 ctor
/// step — "If options.rootMargin is given but is not a valid string
/// representing a `<length-percentage> [<length-percentage>]{0,3}`,
/// throw a SyntaxError").  Carries the offending token so the host
/// can wrap it in an interface-scoped error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootMarginParseError {
    /// The offending whitespace-split token (`em`-suffixed value,
    /// numeric `NaN`, unrecognised unit, etc).  Empty string if the
    /// shorthand had too many components.
    pub token: String,
}

impl std::fmt::Display for RootMarginParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.token.is_empty() {
            write!(f, "rootMargin shorthand has too many components (>4)")
        } else {
            write!(
                f,
                "rootMargin token '{}' is not a valid <length-percentage>",
                self.token
            )
        }
    }
}

impl std::error::Error for RootMarginParseError {}

/// Parse a CSS-margin-shorthand `rootMargin` string into `[top, right,
/// bottom, left]` components (Intersection Observer §3.1 — strict
/// `px` / `%` only; 1/2/3/4 value shorthand).  Anything else
/// (`em` / `vh` / garbage / NaN) is a SyntaxError per spec.
fn parse_root_margin(raw: &str) -> Result<[MarginComponent; 4], RootMarginParseError> {
    let parse_one = |tok: &str| -> Result<MarginComponent, RootMarginParseError> {
        let invalid = || RootMarginParseError {
            token: tok.to_owned(),
        };
        let parse_finite = |s: &str| -> Result<f32, RootMarginParseError> {
            let n: f32 = s.trim().parse().map_err(|_| invalid())?;
            // Reject NaN / ±Infinity per spec — a margin must be a
            // finite `<length-percentage>`.
            if n.is_finite() {
                Ok(n)
            } else {
                Err(invalid())
            }
        };
        if let Some(num) = tok.strip_suffix('%') {
            Ok(MarginComponent::Pct(parse_finite(num)?))
        } else if let Some(num) = tok.strip_suffix("px") {
            Ok(MarginComponent::Px(parse_finite(num)?))
        } else {
            // The empty token is the empty-string default for an
            // omitted rootMargin — already filtered by
            // `split_whitespace`; any non-empty token without
            // `px` / `%` is an unrecognised unit.
            Err(invalid())
        }
    };
    let toks: Vec<MarginComponent> = raw
        .split_whitespace()
        .map(parse_one)
        .collect::<Result<_, _>>()?;
    // CSS shorthand directions (t/r/b/l = top/right/bottom/left) shadow
    // the function param `raw` plus the per-arm helper bindings; clippy
    // tallies them as five single-char names across the match, but the
    // direction names are the conventional CSS spelling.
    #[allow(clippy::many_single_char_names)]
    let arr = match toks.as_slice() {
        [] => [MarginComponent::Px(0.0); 4],
        [all] => [*all; 4],
        [v, h] => [*v, *h, *v, *h],
        [t, h, b] => [*t, *h, *b, *h],
        [t, r, b, l] => [*t, *r, *b, *l],
        // 5+ tokens is a syntax error (CSS margin shorthand caps at 4).
        _ => {
            return Err(RootMarginParseError {
                token: String::new(),
            });
        }
    };
    Ok(arr)
}

/// Expand `root` outward by the resolved `rootMargin` (top/right/bottom/left)
/// per W3C Intersection Observer §3.1 `rootMargin` (offset applied to the
/// root intersection rect before the target-vs-root overlap is computed).
fn apply_root_margin(root: Rect, margin: &[MarginComponent; 4]) -> Rect {
    let top = margin[0].resolve(root.size.height);
    let right = margin[1].resolve(root.size.width);
    let bottom = margin[2].resolve(root.size.height);
    let left = margin[3].resolve(root.size.width);
    Rect::new(
        root.origin.x - left,
        root.origin.y - top,
        root.size.width + left + right,
        root.size.height + top + bottom,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn intersection_ratio_fully_visible() {
        let ratio = compute_intersection_ratio(
            Rect::new(10.0, 10.0, 100.0, 100.0), // target
            Rect::new(0.0, 0.0, 1024.0, 768.0),  // viewport
        );
        assert!((ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn intersection_ratio_half_visible() {
        let ratio = compute_intersection_ratio(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(50.0, 0.0, 1000.0, 1000.0), // viewport starts at x=50
        );
        assert!((ratio - 0.5).abs() < 0.001);
    }

    #[test]
    fn intersection_ratio_not_visible() {
        let ratio = compute_intersection_ratio(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(200.0, 200.0, 100.0, 100.0),
        );
        assert!((ratio - 0.0).abs() < 0.001);
    }

    #[test]
    fn edge_adjacent_target_reports_is_intersecting() {
        // IO §3.7: target sharing an edge with root has a degenerate
        // (zero-width) intersection; spec says `isIntersecting = true`
        // even though `intersectionRatio = 0`.
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(IntersectionObserverInit::default()).unwrap();
        reg.observe(&mut dom, id, el);

        // Target right edge at x=100; root left edge at x=100 → 0-width overlap.
        let target = Rect::new(0.0, 0.0, 100.0, 50.0);
        let root = Rect::new(100.0, 0.0, 100.0, 50.0);
        let observations = reg.gather_observations(&mut dom, &|_, _| Some(target), root);

        assert_eq!(observations.len(), 1);
        let entry = &observations[0].1[0];
        assert!(
            entry.is_intersecting,
            "edge-adjacent target must report isIntersecting=true"
        );
        assert!(
            (entry.intersection_ratio - 0.0).abs() < 0.001,
            "edge-adjacent target ratio = 0 (zero-area overlap / positive-area target)"
        );
    }

    #[test]
    fn zero_area_target_inside_root_reports_ratio_one() {
        // IO §3.7: a zero-area target (e.g. an empty scroll sentinel) that
        // sits inside the root reports ratio = 1.0, not 0/0 = NaN/0.
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(IntersectionObserverInit::default()).unwrap();
        reg.observe(&mut dom, id, el);

        let target = Rect::new(50.0, 25.0, 0.0, 0.0); // zero-area point inside root
        let root = Rect::new(0.0, 0.0, 100.0, 50.0);
        let observations = reg.gather_observations(&mut dom, &|_, _| Some(target), root);

        assert_eq!(observations.len(), 1);
        let entry = &observations[0].1[0];
        assert!(
            entry.is_intersecting,
            "zero-area target inside root intersects"
        );
        assert!(
            (entry.intersection_ratio - 1.0).abs() < 0.001,
            "zero-area target inside root: ratio = 1.0 per spec"
        );
    }

    #[test]
    fn zero_area_target_outside_root_reports_ratio_zero() {
        // IO §3.7 complement: a zero-area target outside the root is
        // not intersecting and reports ratio = 0.
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(IntersectionObserverInit::default()).unwrap();
        reg.observe(&mut dom, id, el);

        let target = Rect::new(200.0, 200.0, 0.0, 0.0); // zero-area point outside root
        let root = Rect::new(0.0, 0.0, 100.0, 50.0);
        let observations = reg.gather_observations(&mut dom, &|_, _| Some(target), root);

        // `crossed_threshold` treats the initial `last_ratio = -1.0`
        // → `new = 0.0` step as crossing the default `[0]` threshold,
        // so an initial non-intersecting entry IS emitted (the spec's
        // first-broadcast step).  What we pin is that any such entry
        // reports `isIntersecting = false` — gather never spuriously
        // upgrades a disjoint zero-area target to intersecting.
        assert!(
            observations
                .iter()
                .all(|(_, entries)| entries.iter().all(|e| !e.is_intersecting)),
            "zero-area target outside root must not report intersecting"
        );
    }

    #[test]
    fn threshold_crossing_detection() {
        assert!(crossed_threshold(-1.0, 0.5, &[0.0]));
        assert!(!crossed_threshold(0.5, 0.6, &[0.0]));
        assert!(crossed_threshold(0.4, 0.6, &[0.5]));
        assert!(crossed_threshold(0.6, 0.4, &[0.5]));
    }

    #[test]
    fn gather_initial_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit {
            threshold: vec![0.0],
            ..Default::default()
        };
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init).expect("test init has valid rootMargin");
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        // Element fully inside viewport.
        let observations = reg.gather_observations(
            &mut dom,
            &|_, e| {
                if e == el {
                    Some(Rect::new(10.0, 10.0, 100.0, 100.0))
                } else {
                    None
                }
            },
            viewport,
        );
        // Initial ratio -1.0 → 1.0 crosses threshold 0.0.
        assert_eq!(observations.len(), 1);
        assert!(observations[0].1[0].is_intersecting);
    }

    #[test]
    fn no_crossing_no_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit {
            threshold: vec![0.5],
            ..Default::default()
        };
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init).expect("test init has valid rootMargin");
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);

        // First: fully visible → ratio 1.0, crosses 0.5.
        reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );

        // Still fully visible — same ratio, no crossing.
        let observations = reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(20.0, 20.0, 100.0, 100.0)),
            viewport,
        );
        assert!(observations.is_empty());
    }

    #[test]
    fn despawn_clears_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit::default();
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init).expect("test init has valid rootMargin");
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );

        let _ = dom.destroy_entity(el);
        let observations = reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );
        assert!(observations.is_empty());
    }

    #[test]
    fn observe_unregistered_id_is_noop() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = IntersectionObserverRegistry::new();
        // `ghost` has no init config (never register()'d) — observe must not
        // attach a component that gather would scan but never deliver.
        let ghost = IntersectionObserverId::from_raw(999);
        reg.observe(&mut dom, ghost, el);
        assert!(
            dom.world().get::<&IntersectionObservedBy>(el).is_err(),
            "observe on an unregistered id must not attach a component"
        );
    }

    #[test]
    fn box_less_target_delivers_initial_observation() {
        // display:none / pre-layout target (rect_fn → None) must still get the
        // mandated initial observation: ratio 0, isIntersecting false, zero
        // rects — and only once (no re-delivery while it stays box-less).
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = IntersectionObserverRegistry::new();
        let id = reg
            .register(IntersectionObserverInit {
                threshold: vec![0.0],
                ..Default::default()
            })
            .expect("test init has valid rootMargin");
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        let first = reg.gather_observations(&mut dom, &|_, _| None, viewport);
        assert_eq!(first.len(), 1, "box-less target must deliver once");
        let entry = &first[0].1[0];
        assert!(!entry.is_intersecting);
        assert_eq!(entry.intersection_ratio, 0.0);
        assert_eq!(entry.bounding_client_rect, Rect::default());
        assert_eq!(entry.intersection_rect, Rect::default());

        // Still box-less → no second delivery (last_ratio now 0).
        let second = reg.gather_observations(&mut dom, &|_, _| None, viewport);
        assert!(second.is_empty(), "no re-delivery while still box-less");
    }

    #[test]
    fn root_margin_expands_root_bounds() {
        // A target just outside the viewport becomes intersecting once a
        // positive rootMargin expands the root bounds to reach it.
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = IntersectionObserverRegistry::new();
        let id = reg
            .register(IntersectionObserverInit {
                root_margin: "100px".to_string(),
                threshold: vec![0.0],
                ..Default::default()
            })
            .expect("test init has valid rootMargin");
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1000.0, 1000.0);
        // Target at y=1050 (50px below viewport bottom) — only reachable with
        // the 100px bottom margin.
        let obs = reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 1050.0, 100.0, 100.0)),
            viewport,
        );
        assert_eq!(obs.len(), 1);
        let entry = &obs[0].1[0];
        assert!(entry.is_intersecting, "rootMargin should pull target in");
        let rb = entry.root_bounds.expect("same-origin reports rootBounds");
        assert_eq!(rb.origin.y, -100.0, "top edge expanded by 100px");
        assert_eq!(rb.size.height, 1200.0, "height grown by top+bottom margin");
    }

    #[test]
    fn parse_root_margin_shorthand() {
        // 2-value: vertical | horizontal.
        let m = parse_root_margin("10px 20%").expect("valid shorthand");
        let root = Rect::new(0.0, 0.0, 200.0, 100.0);
        let expanded = apply_root_margin(root, &m);
        // top/bottom = 10px, left/right = 20% of width(200) = 40px.
        assert_eq!(expanded.origin.x, -40.0);
        assert_eq!(expanded.origin.y, -10.0);
        assert_eq!(expanded.size.width, 200.0 + 80.0);
        assert_eq!(expanded.size.height, 100.0 + 20.0);
    }

    #[test]
    fn parse_root_margin_rejects_invalid_units() {
        // W3C Intersection Observer §3.1 — `rootMargin` must be a
        // valid `<length-percentage>{1,4}`.  `em` / `vh` / bare
        // numbers / NaN / oversize shorthand are SyntaxError.
        for bad in [
            "10em",                 // unsupported unit
            "10vh 5px",             // mixed: second token ok, first fails
            "10",                   // bare number, no unit
            "NaNpx",                // non-finite
            "10px 5%  3px 2px 1px", // 5-component shorthand
        ] {
            assert!(
                parse_root_margin(bad).is_err(),
                "expected SyntaxError for rootMargin '{bad}'"
            );
        }
        // Valid examples still parse.
        for good in ["", "10px", "10px 20%", "0% 0% 0% 0%", "-10px"] {
            assert!(
                parse_root_margin(good).is_ok(),
                "expected '{good}' to parse"
            );
        }
    }

    #[test]
    fn register_rejects_invalid_root_margin() {
        let mut reg = IntersectionObserverRegistry::new();
        let err = reg
            .register(IntersectionObserverInit {
                root_margin: "10em".to_string(),
                threshold: vec![0.0],
                ..Default::default()
            })
            .expect_err("invalid unit must surface as a SyntaxError");
        assert!(err.to_string().contains("10em"));
    }

    #[test]
    fn gather_observations_is_id_sorted() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = IntersectionObserverRegistry::new();
        for _ in 0..4 {
            let id = reg
                .register(IntersectionObserverInit {
                    threshold: vec![0.0],
                    ..Default::default()
                })
                .expect("test init has valid rootMargin");
            reg.observe(&mut dom, id, el);
        }

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        let result = reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );
        let got: Vec<u64> = result.iter().map(|(id, _)| id.raw()).collect();
        let mut sorted = got.clone();
        sorted.sort_unstable();
        assert_eq!(got.len(), 4);
        assert_eq!(got, sorted, "gather must deliver in id-sorted order");
    }
}

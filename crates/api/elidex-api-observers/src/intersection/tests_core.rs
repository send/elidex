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
    // IO §3.2.7: target sharing an edge with root has a degenerate
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
    // IO §3.2.7: a zero-area target (e.g. an empty scroll sentinel) that
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
    // IO §3.2.7 complement: a zero-area target outside the root is
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
fn apply_root_margin_clamps_oversized_negative_margin_to_empty() {
    // IO §3.6 (legacy `rootMargin`): negative margins shrink the
    // root rect; a shrink larger than the dimension must collapse
    // to 0, not produce a negative-sized rect (which would corrupt
    // downstream `Rect::intersection_inclusive`).
    let root = Rect::new(0.0, 0.0, 50.0, 50.0);
    let neg = MarginComponent::Px(-100.0);
    let margin = [neg, neg, neg, neg]; // top/right/bottom/left = -100px
    let out = apply_root_margin(root, &margin);
    assert_eq!(
        out.size.width, 0.0,
        "oversized negative rootMargin must clamp width to 0"
    );
    assert_eq!(
        out.size.height, 0.0,
        "oversized negative rootMargin must clamp height to 0"
    );
}

#[test]
fn clear_root_entities_scrubs_explicit_roots_only() {
    // Vm::unbind hook: retained `IntersectionObserverInit.root: Entity`
    // would alias an unrelated entity in the rebound DOM (worlds share
    // index space).  `clear_root_entities` reverts those to `None`
    // (implicit viewport) without touching the rest of the config.
    let mut dom = EcsDom::new();
    let explicit_root = elem(&mut dom, "div");

    let mut reg = IntersectionObserverRegistry::new();
    let explicit_id = reg
        .register(IntersectionObserverInit {
            root: Some(explicit_root),
            threshold: vec![0.5],
            ..Default::default()
        })
        .unwrap();
    let implicit_id = reg
        .register(IntersectionObserverInit {
            root: None,
            threshold: vec![0.5],
            ..Default::default()
        })
        .unwrap();

    reg.clear_root_entities();

    assert_eq!(
        reg.observers.get(&explicit_id).unwrap().init.root,
        None,
        "explicit root scrubbed to None"
    );
    assert_eq!(
        reg.observers.get(&implicit_id).unwrap().init.root,
        None,
        "implicit-None preserved"
    );
    // Threshold (non-Entity config) is left untouched.
    assert_eq!(
        reg.observers.get(&explicit_id).unwrap().init.threshold,
        vec![0.5]
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
    // W3C Intersection Observer §2.2 — `rootMargin` must be a
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

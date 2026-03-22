//! Layout query handlers: getBoundingClientRect, offset*, client*, scroll*.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, JsValue, LayoutBox, Position};
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

// ---------------------------------------------------------------------------
// getBoundingClientRect
// ---------------------------------------------------------------------------

/// `element.getBoundingClientRect()` — returns a `DOMRect` with viewport-relative coordinates.
pub struct GetBoundingClientRect;

impl DomApiHandler for GetBoundingClientRect {
    fn method_name(&self) -> &str {
        "getBoundingClientRect"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let bb = get_border_box(dom, this);
        // Subtract accumulated scroll offsets from ancestor scroll containers
        // to convert document coordinates to viewport-relative coordinates
        // (CSSOM View §5 getBoundingClientRect).
        let (scroll_x, scroll_y) = accumulated_scroll_offset(dom, this);
        Ok(dom_rect_value(bb.0 - scroll_x, bb.1 - scroll_y, bb.2, bb.3))
    }
}

// ---------------------------------------------------------------------------
// offset* properties
// ---------------------------------------------------------------------------

/// `element.offsetWidth` — border box width.
pub struct GetOffsetWidth;
impl_layout_handler!(GetOffsetWidth, "offsetWidth.get", |dom, entity| {
    let bb = get_border_box(dom, entity);
    JsValue::Number(f64::from(bb.2))
});

/// `element.offsetHeight` — border box height.
pub struct GetOffsetHeight;
impl_layout_handler!(GetOffsetHeight, "offsetHeight.get", |dom, entity| {
    let bb = get_border_box(dom, entity);
    JsValue::Number(f64::from(bb.3))
});

/// `element.offsetTop` — distance from top of offsetParent's content.
pub struct GetOffsetTop;
impl_layout_handler!(GetOffsetTop, "offsetTop.get", |dom, entity| {
    let (_, offset_y) = offset_from_parent(dom, entity);
    JsValue::Number(f64::from(offset_y))
});

/// `element.offsetLeft` — distance from left of offsetParent's content.
pub struct GetOffsetLeft;
impl_layout_handler!(GetOffsetLeft, "offsetLeft.get", |dom, entity| {
    let (offset_x, _) = offset_from_parent(dom, entity);
    JsValue::Number(f64::from(offset_x))
});

/// `element.offsetParent` — nearest positioned ancestor, or null.
pub struct GetOffsetParent;

impl DomApiHandler for GetOffsetParent {
    fn method_name(&self) -> &str {
        "offsetParent.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        match find_offset_parent(dom, this) {
            Some(parent) => Ok(JsValue::ObjectRef(parent.to_bits().get())),
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// client* properties
// ---------------------------------------------------------------------------

/// `element.clientWidth` — content width + padding (no border, no scrollbar).
pub struct GetClientWidth;
impl_layout_handler!(GetClientWidth, "clientWidth.get", |dom, entity| {
    let pb = get_padding_box(dom, entity);
    JsValue::Number(f64::from(pb.2))
});

/// `element.clientHeight` — content height + padding (no border, no scrollbar).
pub struct GetClientHeight;
impl_layout_handler!(GetClientHeight, "clientHeight.get", |dom, entity| {
    let pb = get_padding_box(dom, entity);
    JsValue::Number(f64::from(pb.3))
});

/// `element.clientTop` — top border width.
pub struct GetClientTop;
impl_layout_handler!(GetClientTop, "clientTop.get", |dom, entity| {
    let border_top = dom
        .world()
        .get::<&LayoutBox>(entity)
        .map_or(0.0, |lb| lb.border.top);
    JsValue::Number(f64::from(border_top))
});

/// `element.clientLeft` — left border width.
pub struct GetClientLeft;
impl_layout_handler!(GetClientLeft, "clientLeft.get", |dom, entity| {
    let border_left = dom
        .world()
        .get::<&LayoutBox>(entity)
        .map_or(0.0, |lb| lb.border.left);
    JsValue::Number(f64::from(border_left))
});

// ---------------------------------------------------------------------------
// scroll* properties
// ---------------------------------------------------------------------------

/// `element.scrollWidth` — total scrollable width (content + overflow).
pub struct GetScrollWidth;
impl_layout_handler!(GetScrollWidth, "scrollWidth.get", |dom, entity| {
    // Simplified: return clientWidth (no overflow tracking yet).
    let pb = get_padding_box(dom, entity);
    JsValue::Number(f64::from(pb.2))
});

/// `element.scrollHeight` — total scrollable height (content + overflow).
pub struct GetScrollHeight;
impl_layout_handler!(GetScrollHeight, "scrollHeight.get", |dom, entity| {
    let pb = get_padding_box(dom, entity);
    JsValue::Number(f64::from(pb.3))
});

/// `element.scrollTop` — vertical scroll position.
pub struct GetScrollTop;
impl_layout_handler!(GetScrollTop, "scrollTop.get", |dom, entity| {
    let offset = dom
        .world()
        .get::<&elidex_ecs::ScrollState>(entity)
        .map_or(0.0, |s| s.scroll_offset.y);
    JsValue::Number(f64::from(offset))
});

/// `element.scrollLeft` — horizontal scroll position.
pub struct GetScrollLeft;
impl_layout_handler!(GetScrollLeft, "scrollLeft.get", |dom, entity| {
    let offset = dom
        .world()
        .get::<&elidex_ecs::ScrollState>(entity)
        .map_or(0.0, |s| s.scroll_offset.x);
    JsValue::Number(f64::from(offset))
});

// ---------------------------------------------------------------------------
// getClientRects
// ---------------------------------------------------------------------------

/// `element.getClientRects()` — returns a list of `DOMRect` values.
///
/// For block elements, returns a single rect (same as `getBoundingClientRect`).
/// For inline elements, a simplified single rect is returned (per-line rects
/// require line box information not yet available).
pub struct GetClientRects;

impl DomApiHandler for GetClientRects {
    fn method_name(&self) -> &str {
        "getClientRects"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let bb = get_border_box(dom, this);
        let (scroll_x, scroll_y) = accumulated_scroll_offset(dom, this);
        // Return a single DOMRect as a newline-separated list of "x,y,w,h" entries.
        // The JS bridge splits on newlines to build the DOMRectList.
        Ok(dom_rect_value(bb.0 - scroll_x, bb.1 - scroll_y, bb.2, bb.3))
    }
}

// ---------------------------------------------------------------------------
// scrollIntoView
// ---------------------------------------------------------------------------

/// `element.scrollIntoView()` — scroll the nearest scrollable ancestor so the
/// element is visible.
///
/// Simplified: finds the nearest ancestor with `ScrollState` and adjusts its
/// scroll offset so that the element's border box top-left is at the scroll
/// container's top-left. Does nothing if no scrollable ancestor exists.
pub struct ScrollIntoView;

impl DomApiHandler for ScrollIntoView {
    fn method_name(&self) -> &str {
        "scrollIntoView"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let bb = get_border_box(dom, this);
        let elem_x = bb.0;
        let elem_y = bb.1;

        // Walk up ancestors to find the nearest scroll container.
        let mut current = dom.get_parent(this);
        let mut depth = 0;
        while let Some(ancestor) = current {
            if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
                break;
            }
            if dom
                .world()
                .get::<&elidex_ecs::ScrollState>(ancestor)
                .is_ok()
            {
                // Found a scroll container. Compute the container's border box
                // origin and set scroll offset so the element is visible.
                let container_bb = get_border_box(dom, ancestor);
                let target_x = elem_x - container_bb.0;
                let target_y = elem_y - container_bb.1;

                if let Ok(mut scroll) = dom
                    .world_mut()
                    .get::<&mut elidex_ecs::ScrollState>(ancestor)
                {
                    scroll.scroll_offset.x = target_x.max(0.0);
                    scroll.scroll_offset.y = target_y.max(0.0);
                }
                break;
            }
            current = dom.get_parent(ancestor);
            depth += 1;
        }

        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Macro for simple layout property handlers that return a `JsValue`.
macro_rules! impl_layout_handler {
    ($name:ident, $method:expr, |$dom:ident, $entity:ident| $body:expr) => {
        impl DomApiHandler for $name {
            fn method_name(&self) -> &str {
                $method
            }

            fn invoke(
                &self,
                this: Entity,
                _args: &[JsValue],
                _session: &mut SessionCore,
                $dom: &mut EcsDom,
            ) -> Result<JsValue, DomApiError> {
                let $entity = this;
                Ok($body)
            }
        }
    };
}
// Make macro usable above its definition (Rust allows this in the same file).
use impl_layout_handler;

/// Get border box as (x, y, width, height).
fn get_border_box(dom: &EcsDom, entity: Entity) -> (f32, f32, f32, f32) {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map_or((0.0, 0.0, 0.0, 0.0), |lb| {
            let bb = lb.border_box();
            (bb.origin.x, bb.origin.y, bb.size.width, bb.size.height)
        })
}

/// Get padding box as (x, y, width, height).
fn get_padding_box(dom: &EcsDom, entity: Entity) -> (f32, f32, f32, f32) {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map_or((0.0, 0.0, 0.0, 0.0), |lb| {
            let pb = lb.padding_box();
            (pb.origin.x, pb.origin.y, pb.size.width, pb.size.height)
        })
}

/// Find the nearest positioned ancestor (offsetParent) per CSSOM View §6.
///
/// Returns `None` for the `<body>` element or if the element is fixed/hidden.
fn find_offset_parent(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    let mut current = dom.get_parent(entity)?;
    let mut depth = 0;
    loop {
        if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
            return None;
        }
        // Check if this ancestor is positioned (not static).
        if let Ok(style) = dom.world().get::<&ComputedStyle>(current) {
            if style.position != Position::Static {
                return Some(current);
            }
        }
        // Check if this is the body/html element (fallback offsetParent).
        if let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(current) {
            if tag.0 == "body" || tag.0 == "html" {
                return Some(current);
            }
        }
        current = dom.get_parent(current)?;
        depth += 1;
    }
}

/// Compute offset from the offsetParent's border box origin.
fn offset_from_parent(dom: &EcsDom, entity: Entity) -> (f32, f32) {
    let (ex, ey, _, _) = get_border_box(dom, entity);
    let parent = find_offset_parent(dom, entity);
    let (px, py) = parent.map_or((0.0, 0.0), |p| {
        let (x, y, _, _) = get_border_box(dom, p);
        (x, y)
    });
    (ex - px, ey - py)
}

/// Compute accumulated scroll offset from all ancestor scroll containers.
///
/// Walks up the ancestor chain and sums `ScrollState.scroll_offset` from each
/// ancestor that has one. This converts document-absolute coordinates to
/// viewport-relative coordinates for `getBoundingClientRect`.
fn accumulated_scroll_offset(dom: &EcsDom, entity: Entity) -> (f32, f32) {
    let mut sx = 0.0_f32;
    let mut sy = 0.0_f32;
    let mut current = dom.get_parent(entity);
    let mut depth = 0;
    while let Some(ancestor) = current {
        if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
            break;
        }
        if let Ok(scroll) = dom.world().get::<&elidex_ecs::ScrollState>(ancestor) {
            sx += scroll.scroll_offset.x;
            sy += scroll.scroll_offset.y;
        }
        current = dom.get_parent(ancestor);
        depth += 1;
    }
    (sx, sy)
}

/// Create a `JsValue` representing a `DOMRect`.
///
/// Returns coordinates as a comma-separated string `"x,y,width,height"`.
/// The JS bridge parses this and constructs a proper `DOMRect` object with
/// all 8 properties (`x`, `y`, `width`, `height`, `top`, `right`, `bottom`, `left`).
fn dom_rect_value(x: f32, y: f32, w: f32, h: f32) -> JsValue {
    JsValue::String(format!("{x},{y},{w},{h}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    #[test]
    fn border_box_from_layout_box() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(root, div);

        let lb = LayoutBox {
            content: elidex_plugin::Rect::new(10.0, 20.0, 100.0, 50.0),
            padding: elidex_plugin::EdgeSizes::new(5.0, 5.0, 5.0, 5.0),
            border: elidex_plugin::EdgeSizes::new(2.0, 2.0, 2.0, 2.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(div, lb);

        let (x, y, w, h) = get_border_box(&dom, div);
        // border_box = content expanded by padding, then by border
        // content: (10, 20, 100, 50)
        // padding_box: (5, 15, 110, 60)
        // border_box: (3, 13, 114, 64)
        assert!((x - 3.0).abs() < f32::EPSILON);
        assert!((y - 13.0).abs() < f32::EPSILON);
        assert!((w - 114.0).abs() < f32::EPSILON);
        assert!((h - 64.0).abs() < f32::EPSILON);
    }

    #[test]
    fn offset_parent_finds_positioned_ancestor() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        let _ = dom.append_child(root, body);
        let positioned = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(body, positioned);
        let child = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(positioned, child);

        // Make positioned div have position: relative.
        let _ = dom.world_mut().insert_one(
            positioned,
            ComputedStyle {
                position: Position::Relative,
                ..Default::default()
            },
        );

        let op = find_offset_parent(&dom, child);
        assert_eq!(op, Some(positioned));
    }

    #[test]
    fn offset_parent_falls_back_to_body() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        let _ = dom.append_child(root, body);
        let child = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(body, child);

        // No positioned ancestor — should fall back to body.
        let op = find_offset_parent(&dom, child);
        assert_eq!(op, Some(body));
    }

    #[test]
    fn client_top_returns_border_width() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(root, div);

        let lb = LayoutBox {
            border: elidex_plugin::EdgeSizes::new(3.0, 2.0, 1.0, 4.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(div, lb);

        let mut session = SessionCore::new();
        let result = GetClientTop
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(3.0));
    }

    #[test]
    fn dom_rect_value_format() {
        let rect = dom_rect_value(10.0, 20.0, 100.0, 50.0);
        if let JsValue::String(s) = rect {
            // Comma-separated format: "x,y,width,height"
            let parts: Vec<&str> = s.split(',').collect();
            assert_eq!(parts.len(), 4);
            assert!((parts[0].parse::<f32>().unwrap() - 10.0).abs() < f32::EPSILON);
            assert!((parts[1].parse::<f32>().unwrap() - 20.0).abs() < f32::EPSILON);
            assert!((parts[2].parse::<f32>().unwrap() - 100.0).abs() < f32::EPSILON);
            assert!((parts[3].parse::<f32>().unwrap() - 50.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn get_bounding_client_rect_subtracts_scroll() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let parent = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(root, parent);
        let child = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(parent, child);

        // Give child a layout box at document position (100, 200).
        let lb = LayoutBox {
            content: elidex_plugin::Rect::new(100.0, 200.0, 50.0, 30.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(child, lb);

        // Add a ScrollState to the parent with offset (10, 20).
        let scroll = elidex_ecs::ScrollState {
            scroll_offset: elidex_plugin::Vector::new(10.0, 20.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(parent, scroll);

        let mut session = SessionCore::new();
        let result = GetBoundingClientRect
            .invoke(child, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::String(s) = result {
            let parts: Vec<f32> = s.split(',').map(|p| p.parse().unwrap()).collect();
            // x = 100 - 10 = 90, y = 200 - 20 = 180
            assert!((parts[0] - 90.0).abs() < f32::EPSILON);
            assert!((parts[1] - 180.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn get_client_rects_returns_single_rect() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(root, div);

        let lb = LayoutBox {
            content: elidex_plugin::Rect::new(10.0, 20.0, 100.0, 50.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(div, lb);

        let mut session = SessionCore::new();
        let result = GetClientRects
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        // Should return same format as getBoundingClientRect.
        if let JsValue::String(s) = result {
            let parts: Vec<f32> = s.split(',').map(|p| p.parse().unwrap()).collect();
            assert_eq!(parts.len(), 4);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn scroll_into_view_adjusts_scroll_state() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let scroller = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(root, scroller);
        let child = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(scroller, child);

        // Scroller at (0, 0), child at (50, 200).
        let _ = dom.world_mut().insert_one(
            scroller,
            LayoutBox {
                content: elidex_plugin::Rect::new(0.0, 0.0, 400.0, 300.0),
                ..Default::default()
            },
        );
        let _ = dom.world_mut().insert_one(
            child,
            LayoutBox {
                content: elidex_plugin::Rect::new(50.0, 200.0, 80.0, 20.0),
                ..Default::default()
            },
        );
        let _ = dom.world_mut().insert_one(
            scroller,
            elidex_ecs::ScrollState {
                scroll_offset: elidex_plugin::Vector::new(0.0, 0.0),
                ..Default::default()
            },
        );

        let mut session = SessionCore::new();
        ScrollIntoView
            .invoke(child, &[], &mut session, &mut dom)
            .unwrap();

        let scroll = dom
            .world()
            .get::<&elidex_ecs::ScrollState>(scroller)
            .unwrap();
        // Child at (50, 200), scroller at (0, 0), so scroll should be (50, 200).
        assert!((scroll.scroll_offset.x - 50.0).abs() < f32::EPSILON);
        assert!((scroll.scroll_offset.y - 200.0).abs() < f32::EPSILON);
    }
}

//! Layout types for the box model and layout algorithms.

mod boxes;
mod rect;
mod vectors;

pub use boxes::{EdgeSizes, LayoutBox, LayoutContext, LayoutResult};
pub use rect::{CssSize, Rect, Size};
pub use vectors::{Point, Vector};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_arithmetic() {
        let a = Vector::new(1.0, 2.0);
        let b = Vector::new(3.0, 4.0);
        assert_eq!(a + b, Vector::new(4.0, 6.0));
        assert_eq!(b - a, Vector::new(2.0, 2.0));
        assert_eq!(-a, Vector::new(-1.0, -2.0));
        assert_eq!(a * 3.0, Vector::new(3.0, 6.0));

        let mut c = a;
        c += b;
        assert_eq!(c, Vector::new(4.0, 6.0));

        // Component-wise mul/div
        assert_eq!(a * b, Vector::new(3.0, 8.0));
        assert_eq!(b / a, Vector::new(3.0, 2.0));

        assert_eq!(Vector::<f32>::ZERO, Vector::new(0.0, 0.0));
        assert_eq!(Vector::default(), Vector::<f32>::ZERO);
    }

    #[test]
    fn point_vector_arithmetic() {
        let p = Point::new(10.0_f32, 20.0);
        let v = Vector::new(3.0_f32, 4.0);

        // Point + Vector = Point
        assert_eq!(p + v, Point::new(13.0, 24.0));
        // Point - Vector = Point
        assert_eq!(p - v, Point::new(7.0, 16.0));
        // Point - Point = Vector
        assert_eq!(Point::new(5.0, 8.0) - p, Vector::new(-5.0, -12.0));

        let mut q = p;
        q += v;
        assert_eq!(q, Point::new(13.0, 24.0));
        q -= v;
        assert_eq!(q, Point::new(10.0, 20.0));

        // to_vector
        assert_eq!(p.to_vector(), Vector::new(10.0_f32, 20.0));

        assert_eq!(Point::ZERO, Point::new(0.0_f32, 0.0));
        assert_eq!(Point::default(), Point::ZERO);
    }

    #[test]
    fn is_finite() {
        assert!(Point::new(1.0_f32, 2.0).is_finite());
        assert!(!Point::new(f32::NAN, 0.0).is_finite());
        assert!(!Point::new(0.0_f32, f32::INFINITY).is_finite());
        assert!(Vector::new(1.0_f32, 2.0).is_finite());
        assert!(!Vector::new(f32::NAN, 0.0).is_finite());
    }

    #[test]
    fn vector_to_point() {
        let v = Vector::new(5.0_f32, 10.0);
        assert_eq!(v.to_point(), Point::new(5.0, 10.0));
    }

    #[test]
    fn point_f64_conversion() {
        let p = Point::new(10.0_f32, 20.0);
        let p64 = p.to_f64();
        assert_eq!(p64, Point::new(10.0_f64, 20.0));
        let back = p64.to_f32();
        assert_eq!(back, p);
    }

    #[test]
    fn size_scale_from() {
        let target = Size::new(200.0, 100.0);
        let source = Size::new(400.0, 200.0);
        let scale = target.scale_from(source);
        assert!((scale.x - 0.5).abs() < f64::EPSILON);
        assert!((scale.y - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn rect_point_at_pct() {
        let r = Rect::new(10.0, 20.0, 200.0, 100.0);
        let center = r.point_at_pct(Point::new(50.0, 50.0));
        assert_eq!(center, Point::new(110.0, 70.0));
        let top_left = r.point_at_pct(Point::ZERO);
        assert_eq!(top_left, r.origin);
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn rect_intersection() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 100.0, 100.0);
        let inter = a.intersection(&b).unwrap();
        assert_eq!(inter, Rect::new(50.0, 50.0, 50.0, 50.0));

        // No overlap
        let c = Rect::new(200.0, 200.0, 10.0, 10.0);
        assert!(a.intersection(&c).is_none());

        // Edge-touching (zero area)
        let d = Rect::new(100.0, 0.0, 50.0, 50.0);
        assert!(a.intersection(&d).is_none());

        // Full containment
        let e = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(a.intersection(&e), Some(e));
    }

    #[test]
    fn rect_inset() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.inset(5.0), Rect::new(15.0, 25.0, 90.0, 40.0));
        // Over-inset clamps to zero size
        assert_eq!(r.inset(60.0), Rect::new(70.0, 80.0, 0.0, 0.0));
    }

    #[test]
    fn size_new_and_zero() {
        assert_eq!(
            Size::new(10.0, 20.0),
            Size {
                width: 10.0,
                height: 20.0
            }
        );
        assert_eq!(Size::ZERO, Size::default());
    }

    #[test]
    fn rect_from_origin_size() {
        let r = Rect::from_origin_size(Point::new(5.0, 10.0), Size::new(100.0, 50.0));
        assert_eq!(r, Rect::new(5.0, 10.0, 100.0, 50.0));
    }

    #[test]
    fn rect_edges_and_center() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
        assert_eq!(r.max_point(), Point::new(110.0, 70.0));
        assert_eq!(r.center(), Point::new(60.0, 45.0));
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        // Inside
        assert!(r.contains(Point::new(50.0, 40.0)));
        // Top-left corner (inclusive)
        assert!(r.contains(Point::new(10.0, 20.0)));
        // Bottom-right (exclusive)
        assert!(!r.contains(Point::new(110.0, 70.0)));
        // Just inside bottom-right
        assert!(r.contains(Point::new(109.99, 69.99)));
        // Outside
        assert!(!r.contains(Point::new(5.0, 40.0)));
        assert!(!r.contains(Point::new(50.0, 80.0)));
    }

    #[test]
    fn rect_default() {
        let r = Rect::default();
        assert_eq!(r.origin.x, 0.0);
        assert_eq!(r.origin.y, 0.0);
        assert_eq!(r.size.width, 0.0);
        assert_eq!(r.size.height, 0.0);
    }

    #[test]
    fn size_default() {
        let s = Size::default();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn edge_sizes_default() {
        let e = EdgeSizes::<f32>::default();
        assert_eq!(e.top, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.bottom, 0.0);
        assert_eq!(e.left, 0.0);
    }

    #[test]
    fn layout_box_padding_box() {
        let b = LayoutBox {
            content: Rect::new(20.0, 20.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            ..Default::default()
        };
        let pb = b.padding_box();
        assert_eq!(pb.origin.x, 10.0);
        assert_eq!(pb.origin.y, 10.0);
        assert_eq!(pb.size.width, 120.0);
        assert_eq!(pb.size.height, 70.0);
    }

    #[test]
    fn layout_box_border_box() {
        let b = LayoutBox {
            content: Rect::new(25.0, 25.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            ..Default::default()
        };
        let bb = b.border_box();
        assert_eq!(bb.origin.x, 10.0);
        assert_eq!(bb.origin.y, 10.0);
        assert_eq!(bb.size.width, 130.0);
        assert_eq!(bb.size.height, 80.0);
    }

    #[test]
    fn layout_box_margin_box() {
        let b = LayoutBox {
            content: Rect::new(30.0, 30.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            margin: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            first_baseline: None,
            layout_generation: 0,
        };
        let mb = b.margin_box();
        assert_eq!(mb.origin.x, 10.0); // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.origin.y, 10.0); // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.size.width, 140.0);
        assert_eq!(mb.size.height, 90.0);
    }

    #[test]
    fn layout_box_default_all_zero() {
        let b = LayoutBox::default();
        let mb = b.margin_box();
        assert_eq!(mb, Rect::default());
    }

    #[test]
    fn layout_box_asymmetric_edges() {
        let b = LayoutBox {
            content: Rect::new(20.0, 10.0, 200.0, 100.0),
            padding: EdgeSizes {
                top: 5.0,
                right: 15.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            },
            margin: EdgeSizes {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            first_baseline: None,
            layout_generation: 0,
        };
        let bb = b.border_box();
        assert_eq!(bb.origin.x, 6.0); // 20 - 10 (pad.left) - 4 (border.left)
        assert_eq!(bb.origin.y, 4.0); // 10 - 5 (pad.top) - 1 (border.top)
        assert_eq!(bb.size.width, 231.0); // 200 + 10 + 15 + 4 + 2
        assert_eq!(bb.size.height, 119.0); // 100 + 5 + 10 + 1 + 3
    }

    #[test]
    fn layout_context_default() {
        let ctx = LayoutContext::default();
        assert_eq!(ctx.viewport, Size::default());
        assert_eq!(ctx.containing_block, Size::default());
    }

    #[test]
    fn layout_result_default() {
        let r = LayoutResult::default();
        assert_eq!(r.bounds, Rect::default());
        assert_eq!(r.margin, EdgeSizes::default());
    }

    #[test]
    fn vector_scalar_div() {
        let v = Vector::new(10.0_f32, 20.0);
        assert_eq!(v / 2.0, Vector::new(5.0, 10.0));

        let v64 = Vector::new(9.0_f64, 12.0);
        assert_eq!(v64 / 3.0, Vector::new(3.0, 4.0));
    }

    #[test]
    fn vector_sub_assign() {
        let mut v = Vector::new(10.0_f32, 20.0);
        v -= Vector::new(3.0, 5.0);
        assert_eq!(v, Vector::new(7.0, 15.0));
    }

    #[test]
    fn point_f32_to_tuple() {
        let p = Point::new(1.5_f32, 2.5);
        assert_eq!(p.to_tuple(), (1.5, 2.5));
    }

    #[test]
    fn vector_f64_zero() {
        assert_eq!(Vector::<f64>::ZERO, Vector::new(0.0_f64, 0.0));
        assert_eq!(Vector::<f64>::default(), Vector::<f64>::ZERO);
    }

    #[test]
    fn vector_f64_to_point() {
        let v = Vector::new(3.0_f64, 4.0);
        assert_eq!(v.to_point(), Point::new(3.0_f64, 4.0));
    }

    #[test]
    fn vector_f64_to_f32() {
        let v = Vector::new(1.5_f64, 2.5);
        assert_eq!(v.to_f32(), Vector::new(1.5_f32, 2.5));
    }

    #[test]
    fn size_scalar_mul() {
        let s = Size::new(10.0, 20.0);
        assert_eq!(s * 2.0, Size::new(20.0, 40.0));
    }

    #[test]
    fn size_scalar_div() {
        let s = Size::new(10.0, 20.0);
        assert_eq!(s / 2.0, Size::new(5.0, 10.0));
    }
}

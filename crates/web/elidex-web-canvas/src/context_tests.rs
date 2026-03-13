use super::*;

#[test]
fn new_canvas() {
    let ctx = Canvas2dContext::new(100, 50).unwrap();
    assert_eq!(ctx.width(), 100);
    assert_eq!(ctx.height(), 50);
    // All pixels should be transparent (premultiplied zeros).
    assert!(ctx.pixels().iter().all(|&b| b == 0));
}

#[test]
fn zero_size_returns_none() {
    assert!(Canvas2dContext::new(0, 0).is_none());
    assert!(Canvas2dContext::new(100, 0).is_none());
    assert!(Canvas2dContext::new(0, 100).is_none());
}

#[test]
fn fill_rect_draws_pixels() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("red");
    ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
    // Check center pixel is red (premultiplied with a=255).
    let offset = (5 * 10 + 5) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[0], 255); // R
    assert_eq!(px[1], 0); // G
    assert_eq!(px[2], 0); // B
    assert_eq!(px[3], 255); // A
}

#[test]
fn clear_rect_clears_pixels() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("blue");
    ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
    ctx.clear_rect(2.0, 2.0, 6.0, 6.0);
    // Center should be cleared.
    let offset = (5 * 10 + 5) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[3], 0); // Alpha should be 0
}

#[test]
fn save_restore_state() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("red");
    assert_eq!(ctx.fill_style(), CssColor::RED);
    ctx.save();
    ctx.set_fill_style("blue");
    assert_eq!(ctx.fill_style(), CssColor::BLUE);
    ctx.restore();
    assert_eq!(ctx.fill_style(), CssColor::RED);
}

#[test]
fn restore_empty_stack_is_noop() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("green");
    ctx.restore(); // Should not panic.
    assert_eq!(ctx.fill_style(), CssColor::GREEN);
}

#[test]
fn line_width_validation() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    assert_eq!(ctx.line_width(), 1.0);
    ctx.set_line_width(3.0);
    assert_eq!(ctx.line_width(), 3.0);
    ctx.set_line_width(0.0); // Invalid.
    assert_eq!(ctx.line_width(), 3.0);
    ctx.set_line_width(-1.0); // Invalid.
    assert_eq!(ctx.line_width(), 3.0);
    ctx.set_line_width(f32::INFINITY); // Invalid.
    assert_eq!(ctx.line_width(), 3.0);
    ctx.set_line_width(f32::NAN); // Invalid.
    assert_eq!(ctx.line_width(), 3.0);
}

#[test]
fn global_alpha_validation() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    assert_eq!(ctx.global_alpha(), 1.0);
    ctx.set_global_alpha(0.5);
    assert_eq!(ctx.global_alpha(), 0.5);
    ctx.set_global_alpha(-0.1); // Invalid.
    assert_eq!(ctx.global_alpha(), 0.5);
    ctx.set_global_alpha(1.5); // Invalid.
    assert_eq!(ctx.global_alpha(), 0.5);
}

#[test]
fn invalid_fill_style_unchanged() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("red");
    ctx.set_fill_style("notacolor");
    assert_eq!(ctx.fill_style(), CssColor::RED);
}

#[test]
fn path_fill() {
    let mut ctx = Canvas2dContext::new(20, 20).unwrap();
    ctx.set_fill_style("#00ff00");
    ctx.begin_path();
    ctx.rect(2.0, 2.0, 16.0, 16.0);
    ctx.fill();
    // Check center pixel.
    let offset = (10 * 20 + 10) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[1], 255); // Green channel.
}

#[test]
fn fill_preserves_path() {
    let mut ctx = Canvas2dContext::new(20, 20).unwrap();
    ctx.set_fill_style("#ff0000");
    ctx.begin_path();
    ctx.rect(2.0, 2.0, 16.0, 16.0);
    ctx.fill(); // Should not consume the path.
    ctx.set_fill_style("#00ff00");
    ctx.fill(); // Same path, green overwrites red.
                // Center pixel should be green (second fill overwrites first).
    let offset = (10 * 20 + 10) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[0], 0); // No red.
    assert_eq!(px[1], 255); // Green.
}

#[test]
fn get_put_image_data_roundtrip() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("rgb(100, 150, 200)");
    ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
    let data = ctx.get_image_data(0, 0, 10, 10);
    assert_eq!(data.len(), 10 * 10 * 4);
    // Verify the center pixel is approximately our fill color.
    let offset = (5 * 10 + 5) * 4;
    assert_eq!(data[offset], 100);
    assert_eq!(data[offset + 1], 150);
    assert_eq!(data[offset + 2], 200);
    assert_eq!(data[offset + 3], 255);
}

#[test]
fn create_image_data_is_transparent() {
    let data = Canvas2dContext::create_image_data(5, 5);
    assert_eq!(data.len(), 5 * 5 * 4);
    assert!(data.iter().all(|&b| b == 0));
}

#[test]
fn to_rgba8_straight_conversion() {
    let mut ctx = Canvas2dContext::new(4, 4).unwrap();
    ctx.set_fill_style("red");
    ctx.fill_rect(0.0, 0.0, 4.0, 4.0);
    let straight = ctx.to_rgba8_straight();
    // Check center pixel.
    let offset = (2 * 4 + 2) * 4;
    assert_eq!(straight[offset], 255);
    assert_eq!(straight[offset + 1], 0);
    assert_eq!(straight[offset + 2], 0);
    assert_eq!(straight[offset + 3], 255);
}

#[test]
fn transform_translate() {
    let mut ctx = Canvas2dContext::new(20, 20).unwrap();
    ctx.set_fill_style("white");
    ctx.translate(5.0, 5.0);
    ctx.fill_rect(0.0, 0.0, 5.0, 5.0);
    // Pixel at (7, 7) should be white (translated).
    let offset = (7 * 20 + 7) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[0], 255);
    assert_eq!(px[3], 255);
    // Pixel at (0, 0) should be transparent.
    assert_eq!(ctx.pixels()[3], 0);
}

#[test]
fn measure_text_returns_positive() {
    let ctx = Canvas2dContext::new(10, 10).unwrap();
    let w = ctx.measure_text("hello");
    assert!(w > 0.0);
}

#[test]
fn stroke_rect_draws() {
    let mut ctx = Canvas2dContext::new(20, 20).unwrap();
    ctx.set_stroke_style("red");
    ctx.set_line_width(2.0);
    ctx.stroke_rect(2.0, 2.0, 16.0, 16.0);
    // Top edge should have red pixels.
    let offset = (2 * 20 + 10) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert!(px[0] > 0); // Some red.
    assert!(px[3] > 0); // Visible.
}

#[test]
fn fill_rect_negative_dimensions() {
    let mut ctx = Canvas2dContext::new(20, 20).unwrap();
    ctx.set_fill_style("red");
    // Negative width/height should be normalized.
    ctx.fill_rect(10.0, 10.0, -10.0, -10.0);
    // Should fill from (0,0) to (10,10).
    let offset = (5 * 20 + 5) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[0], 255);
    assert_eq!(px[3], 255);
}

#[test]
fn move_to_line_to_non_finite_is_noop() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.set_fill_style("red");
    // Non-finite moveTo should be ignored (no subpath started).
    ctx.move_to(f32::NAN, 0.0);
    ctx.line_to(10.0, 10.0);
    ctx.fill();
    // lineTo with NaN in first position creates a move_to (path was empty),
    // but the non-finite check prevents it, so path is still empty → no fill.
    assert!(
        ctx.pixels().iter().all(|&b| b == 0),
        "NaN moveTo should be a no-op"
    );

    ctx.begin_path();
    ctx.move_to(0.0, 0.0);
    ctx.line_to(f32::INFINITY, 5.0);
    ctx.line_to(10.0, 10.0);
    ctx.fill();
    // The Infinity lineTo is skipped, so the path is just a single point → no fill.
    assert!(
        ctx.pixels().iter().all(|&b| b == 0),
        "Infinity lineTo should be a no-op"
    );
}

#[test]
fn fill_rect_nan_is_noop() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.fill_rect(f32::NAN, 0.0, 5.0, 5.0);
    // All pixels should remain transparent.
    assert!(ctx.pixels().iter().all(|&b| b == 0));
}

#[test]
fn transform_nan_is_noop() {
    let mut ctx = Canvas2dContext::new(10, 10).unwrap();
    ctx.translate(f32::NAN, 0.0);
    ctx.rotate(f32::INFINITY);
    ctx.scale(f32::NEG_INFINITY, 1.0);
    // Transform should still be identity.
    ctx.set_fill_style("red");
    ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
    let offset = (5 * 10 + 5) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert_eq!(px[0], 255);
}

#[test]
fn stroke_rect_zero_width_draws_line() {
    let mut ctx = Canvas2dContext::new(20, 20).unwrap();
    ctx.set_stroke_style("red");
    ctx.set_line_width(2.0);
    // Zero width should draw a vertical line.
    ctx.stroke_rect(10.0, 2.0, 0.0, 16.0);
    let offset = (10 * 20 + 10) * 4;
    let px = &ctx.pixels()[offset..offset + 4];
    assert!(px[0] > 0); // Some red from the line stroke.
    assert!(px[3] > 0);
}

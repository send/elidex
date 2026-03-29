use super::*;

// ---------------------------------------------------------------------------
// DOMPoint / DOMMatrix (CSSWG Geometry §5-6)
// ---------------------------------------------------------------------------

#[test]
fn dom_point_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = new DOMPoint(1, 2, 3, 4);
        console.log(p.x === 1 && p.y === 2 && p.z === 3 && p.w === 4);
    ",
    );
}

#[test]
fn dom_point_defaults() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = new DOMPoint();
        console.log(p.x === 0 && p.y === 0 && p.z === 0 && p.w === 1);
    ",
    );
}

#[test]
fn dom_rect_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var r = new DOMRect(10, 20, 100, 50);
        console.log(r.x === 10 && r.y === 20 && r.width === 100 && r.height === 50 &&
            r.top === 20 && r.left === 10 && r.right === 110 && r.bottom === 70);
    ",
    );
}

#[test]
fn dom_matrix_identity() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        console.log(m.is2D && m.isIdentity && m.a === 1 && m.d === 1 && m.e === 0 && m.f === 0);
    ",
    );
}

// ---------------------------------------------------------------------------
// visualViewport
// ---------------------------------------------------------------------------

#[test]
fn visual_viewport_exists() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof visualViewport === 'object' && visualViewport.scale === 1)",
    );
}

// ---------------------------------------------------------------------------
// DOMPoint.fromPoint / DOMPointReadOnly.fromPoint
// ---------------------------------------------------------------------------

#[test]
fn dom_point_from_point() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = DOMPoint.fromPoint({ x: 10, y: 20, z: 30, w: 2 });
        console.log(p.x === 10 && p.y === 20 && p.z === 30 && p.w === 2);
    ",
    );
}

#[test]
fn dom_point_readonly_from_point() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = DOMPointReadOnly.fromPoint({ x: 5, y: 15 });
        console.log(p.x === 5 && p.y === 15 && p.z === 0 && p.w === 1);
    ",
    );
}

// ---------------------------------------------------------------------------
// DOMMatrix transformation methods
// ---------------------------------------------------------------------------

#[test]
fn dom_matrix_translate_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        var r = m.translateSelf(10, 20);
        console.log(r === m && m.e === 10 && m.f === 20);
    ",
    );
}

#[test]
fn dom_matrix_scale_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        m.scaleSelf(2, 3);
        console.log(m.a === 2 && m.d === 3);
    ",
    );
}

#[test]
fn dom_matrix_rotate_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        m.rotateSelf(90);
        // After 90 degree rotation: a ~= 0, b ~= 1, c ~= -1, d ~= 0
        console.log(Math.abs(m.a) < 0.001 && Math.abs(m.b - 1) < 0.001 &&
                    Math.abs(m.c + 1) < 0.001 && Math.abs(m.d) < 0.001);
    ",
    );
}

#[test]
fn dom_matrix_multiply_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m1 = new DOMMatrix();
        m1.scaleSelf(2, 2);
        var m2 = new DOMMatrix();
        m2.translateSelf(5, 10);
        m1.multiplySelf(m2);
        // scale(2,2) * translate(5,10) = a=2, d=2, e=10, f=20
        console.log(m1.a === 2 && m1.d === 2 && m1.e === 10 && m1.f === 20);
    ",
    );
}

#[test]
fn dom_matrix_invert_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        m.scaleSelf(2, 4);
        m.invertSelf();
        console.log(Math.abs(m.a - 0.5) < 0.001 && Math.abs(m.d - 0.25) < 0.001);
    ",
    );
}

#[test]
fn dom_matrix_translate_immutable() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        var m2 = m.translate(10, 20);
        console.log(m.e === 0 && m2.e === 10 && m2.f === 20);
    ",
    );
}

#[test]
fn dom_matrix_scale_immutable() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        var m2 = m.scale(3, 5);
        console.log(m.a === 1 && m2.a === 3 && m2.d === 5);
    ",
    );
}

#[test]
fn dom_matrix_readonly_no_mutation_methods() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrixReadOnly();
        console.log(typeof m.translateSelf === 'undefined' && typeof m.scaleSelf === 'undefined');
    ",
    );
}

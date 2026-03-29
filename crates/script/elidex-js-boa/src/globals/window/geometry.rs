//! DOM Geometry types: `DOMPoint`, `DOMPointReadOnly`, `DOMRect`, `DOMMatrix`,
//! `DOMMatrixReadOnly`, and `visualViewport`.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Helper: build a DOMPoint-like JS object (shared by `DOMPoint` and `DOMPointReadOnly`).
#[allow(clippy::unnecessary_wraps)]
fn build_dom_point(
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    mutable: bool,
    ctx: &mut Context,
) -> JsResult<boa_engine::JsObject> {
    let mut init = ObjectInitializer::new(ctx);
    let attr = if mutable {
        Attribute::WRITABLE | Attribute::CONFIGURABLE
    } else {
        Attribute::READONLY | Attribute::CONFIGURABLE
    };
    init.property(js_string!("x"), JsValue::from(x), attr);
    init.property(js_string!("y"), JsValue::from(y), attr);
    init.property(js_string!("z"), JsValue::from(z), attr);
    init.property(js_string!("w"), JsValue::from(w), attr);

    // toJSON()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("DOMPoint: this is not an object")
            })?;
            let vx = obj.get(js_string!("x"), ctx)?;
            let vy = obj.get(js_string!("y"), ctx)?;
            let vz = obj.get(js_string!("z"), ctx)?;
            let vw = obj.get(js_string!("w"), ctx)?;
            let mut json_init = ObjectInitializer::new(ctx);
            json_init.property(js_string!("x"), vx, Attribute::all());
            json_init.property(js_string!("y"), vy, Attribute::all());
            json_init.property(js_string!("z"), vz, Attribute::all());
            json_init.property(js_string!("w"), vw, Attribute::all());
            Ok(JsValue::from(json_init.build()))
        }),
        js_string!("toJSON"),
        0,
    );

    Ok(init.build())
}

/// Extract point coordinates from args (x?, y?, z?, w?) with defaults.
fn extract_point_args(args: &[JsValue]) -> (f64, f64, f64, f64) {
    let x = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
    let y = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
    let z = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
    let w = args.get(3).and_then(JsValue::as_number).unwrap_or(1.0);
    (x, y, z, w)
}

/// Extract point from an object dict (for `fromPoint` static methods).
fn extract_point_dict(val: &JsValue, ctx: &mut Context) -> JsResult<(f64, f64, f64, f64)> {
    if let Some(obj) = val.as_object() {
        let x = dict_number(&obj, "x", 0.0, ctx)?;
        let y = dict_number(&obj, "y", 0.0, ctx)?;
        let z = dict_number(&obj, "z", 0.0, ctx)?;
        let w = dict_number(&obj, "w", 1.0, ctx)?;
        Ok((x, y, z, w))
    } else {
        Ok((0.0, 0.0, 0.0, 1.0))
    }
}

/// Read a numeric property from a JS object, returning `default` when the
/// property is `undefined` or `null` (boa's `to_number` converts `undefined`
/// to `NaN`, so we must check explicitly).
fn dict_number(
    obj: &boa_engine::JsObject,
    key: &str,
    default: f64,
    ctx: &mut Context,
) -> JsResult<f64> {
    let v = obj.get(js_string!(key), ctx)?;
    if v.is_undefined() || v.is_null() {
        Ok(default)
    } else {
        Ok(v.to_number(ctx).unwrap_or(default))
    }
}

/// Register `DOMPoint`, `DOMPointReadOnly`, `DOMMatrix`, `DOMMatrixReadOnly`, `DOMRect`.
#[allow(clippy::too_many_lines)]
pub(super) fn register_dom_geometry(ctx: &mut Context) {
    // DOMPointReadOnly constructor.
    ctx.register_global_callable(
        js_string!("DOMPointReadOnly"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let (x, y, z, w) = extract_point_args(args);
            Ok(JsValue::from(build_dom_point(x, y, z, w, false, ctx)?))
        }),
    )
    .expect("failed to register DOMPointReadOnly");

    // DOMPointReadOnly.fromPoint static method.
    {
        let global = ctx.global_object();
        let ctor = global
            .get(js_string!("DOMPointReadOnly"), ctx)
            .expect("DOMPointReadOnly must exist");
        if let Some(ctor_obj) = ctor.as_object() {
            let from_point = NativeFunction::from_copy_closure(|_this, args, ctx| {
                let dict = args.first().cloned().unwrap_or(JsValue::undefined());
                let (x, y, z, w) = extract_point_dict(&dict, ctx)?;
                Ok(JsValue::from(build_dom_point(x, y, z, w, false, ctx)?))
            });
            let desc = boa_engine::property::PropertyDescriptorBuilder::new()
                .value(from_point.to_js_function(ctx.realm()))
                .writable(true)
                .enumerable(false)
                .configurable(true)
                .build();
            ctor_obj
                .define_property_or_throw(js_string!("fromPoint"), desc, ctx)
                .expect("failed to set DOMPointReadOnly.fromPoint");
        }
    }

    // DOMPoint constructor.
    ctx.register_global_callable(
        js_string!("DOMPoint"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let (x, y, z, w) = extract_point_args(args);
            Ok(JsValue::from(build_dom_point(x, y, z, w, true, ctx)?))
        }),
    )
    .expect("failed to register DOMPoint");

    // DOMPoint.fromPoint static method.
    {
        let global = ctx.global_object();
        let ctor = global
            .get(js_string!("DOMPoint"), ctx)
            .expect("DOMPoint must exist");
        if let Some(ctor_obj) = ctor.as_object() {
            let from_point = NativeFunction::from_copy_closure(|_this, args, ctx| {
                let dict = args.first().cloned().unwrap_or(JsValue::undefined());
                let (x, y, z, w) = extract_point_dict(&dict, ctx)?;
                Ok(JsValue::from(build_dom_point(x, y, z, w, true, ctx)?))
            });
            let desc = boa_engine::property::PropertyDescriptorBuilder::new()
                .value(from_point.to_js_function(ctx.realm()))
                .writable(true)
                .enumerable(false)
                .configurable(true)
                .build();
            ctor_obj
                .define_property_or_throw(js_string!("fromPoint"), desc, ctx)
                .expect("failed to set DOMPoint.fromPoint");
        }
    }

    // DOMRect constructor.
    ctx.register_global_callable(
        js_string!("DOMRect"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let rx = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
            let ry = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
            let rw = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
            let rh = args.get(3).and_then(JsValue::as_number).unwrap_or(0.0);
            let mut init = ObjectInitializer::new(ctx);
            let attr = Attribute::WRITABLE | Attribute::CONFIGURABLE;
            init.property(js_string!("x"), JsValue::from(rx), attr);
            init.property(js_string!("y"), JsValue::from(ry), attr);
            init.property(js_string!("width"), JsValue::from(rw), attr);
            init.property(js_string!("height"), JsValue::from(rh), attr);
            // Derived read-only properties.
            init.property(
                js_string!("top"),
                JsValue::from(ry.min(ry + rh)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!("right"),
                JsValue::from(rx.max(rx + rw)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!("bottom"),
                JsValue::from(ry.max(ry + rh)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!("left"),
                JsValue::from(rx.min(rx + rw)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.function(
                NativeFunction::from_copy_closure(|this, _args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMRect: this is not an object")
                    })?;
                    let vals: Vec<(String, JsValue)> = [
                        "x", "y", "width", "height", "top", "right", "bottom", "left",
                    ]
                    .iter()
                    .map(|key| {
                        let v = obj.get(js_string!(*key), ctx).unwrap_or(JsValue::from(0.0));
                        ((*key).to_string(), v)
                    })
                    .collect();
                    let mut json_init = ObjectInitializer::new(ctx);
                    for (key, v) in vals {
                        json_init.property(js_string!(key.as_str()), v, Attribute::all());
                    }
                    Ok(JsValue::from(json_init.build()))
                }),
                js_string!("toJSON"),
                0,
            );
            Ok(JsValue::from(init.build()))
        }),
    )
    .expect("failed to register DOMRect");

    // DOMMatrix / DOMMatrixReadOnly — 4×4 identity matrix by default.
    register_dom_matrix(ctx, "DOMMatrixReadOnly", false);
    register_dom_matrix(ctx, "DOMMatrix", true);
}

/// Register a `DOMMatrix` or `DOMMatrixReadOnly` constructor.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn register_dom_matrix(ctx: &mut Context, name: &str, mutable: bool) {
    let constructor = NativeFunction::from_copy_closure(move |_this, _args, ctx| {
        // Default: 4×4 identity matrix.
        let attr = if mutable {
            Attribute::WRITABLE | Attribute::CONFIGURABLE
        } else {
            Attribute::READONLY | Attribute::CONFIGURABLE
        };

        let mut init = ObjectInitializer::new(ctx);

        // 2D aliases (CSS transform): a=m11, b=m12, c=m21, d=m22, e=m41, f=m42.
        let identity = [
            ("a", 1.0),
            ("b", 0.0),
            ("c", 0.0),
            ("d", 1.0),
            ("e", 0.0),
            ("f", 0.0),
        ];
        for (key, val) in &identity {
            init.property(js_string!(*key), JsValue::from(*val), attr);
        }

        // Full 4×4 matrix elements.
        let m4x4 = [
            ("m11", 1.0),
            ("m12", 0.0),
            ("m13", 0.0),
            ("m14", 0.0),
            ("m21", 0.0),
            ("m22", 1.0),
            ("m23", 0.0),
            ("m24", 0.0),
            ("m31", 0.0),
            ("m32", 0.0),
            ("m33", 1.0),
            ("m34", 0.0),
            ("m41", 0.0),
            ("m42", 0.0),
            ("m43", 0.0),
            ("m44", 1.0),
        ];
        for (key, val) in &m4x4 {
            init.property(js_string!(*key), JsValue::from(*val), attr);
        }

        init.property(
            js_string!("is2D"),
            JsValue::from(true),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );
        init.property(
            js_string!("isIdentity"),
            JsValue::from(true),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );

        // transformPoint(point) — applies matrix transform to a point.
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let dict = args.first().cloned().unwrap_or(JsValue::undefined());
                let (px, py, pz, pw) = extract_point_dict(&dict, ctx)?;
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let e = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let f = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
                // 2D affine: x' = a*px + c*py + e, y' = b*px + d*py + f
                let rx = a * px + c * py + e * pw;
                let ry = b * px + d * py + f * pw;
                Ok(JsValue::from(build_dom_point(rx, ry, pz, pw, true, ctx)?))
            }),
            js_string!("transformPoint"),
            0,
        );

        if mutable {
            // --- Mutation methods (return `this` for chaining) ---

            // translateSelf(tx, ty, tz?) — post-multiply by translation matrix.
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let tx = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
                    let ty = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
                    let tz = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
                    let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                    let m43 = obj
                        .get(js_string!("m43"), ctx)?
                        .to_number(ctx)
                        .unwrap_or(0.0);
                    let ne = a * tx + c * ty + e;
                    let nf = b * tx + d * ty + f;
                    obj.set(js_string!("e"), JsValue::from(ne), false, ctx)?;
                    obj.set(js_string!("m41"), JsValue::from(ne), false, ctx)?;
                    obj.set(js_string!("f"), JsValue::from(nf), false, ctx)?;
                    obj.set(js_string!("m42"), JsValue::from(nf), false, ctx)?;
                    obj.set(js_string!("m43"), JsValue::from(m43 + tz), false, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("translateSelf"),
                2,
            );

            // scaleSelf(scaleX, scaleY?, scaleZ?)
            // Post-multiply by a scale matrix: a' = a*sx, b' = b*sx, c' = c*sy, d' = d*sy.
            // e and f are unchanged by pure scale.
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let sx = args.first().and_then(JsValue::as_number).unwrap_or(1.0);
                    let sy = args.get(1).and_then(JsValue::as_number).unwrap_or(sx);
                    let sz = args.get(2).and_then(JsValue::as_number).unwrap_or(1.0);
                    let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                    let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                    let m33 = obj
                        .get(js_string!("m33"), ctx)?
                        .to_number(ctx)
                        .unwrap_or(1.0);
                    obj.set(js_string!("a"), JsValue::from(a * sx), false, ctx)?;
                    obj.set(js_string!("m11"), JsValue::from(a * sx), false, ctx)?;
                    obj.set(js_string!("b"), JsValue::from(b * sx), false, ctx)?;
                    obj.set(js_string!("m12"), JsValue::from(b * sx), false, ctx)?;
                    obj.set(js_string!("c"), JsValue::from(c * sy), false, ctx)?;
                    obj.set(js_string!("m21"), JsValue::from(c * sy), false, ctx)?;
                    obj.set(js_string!("d"), JsValue::from(d * sy), false, ctx)?;
                    obj.set(js_string!("m22"), JsValue::from(d * sy), false, ctx)?;
                    obj.set(js_string!("m33"), JsValue::from(m33 * sz), false, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("scaleSelf"),
                1,
            );

            // rotateSelf(rotX, rotY?, rotZ?)
            // For 2D: when only one arg is given, it's the Z rotation angle in degrees.
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let angle_deg = if args.len() <= 1 {
                        args.first().and_then(JsValue::as_number).unwrap_or(0.0)
                    } else {
                        args.get(2).and_then(JsValue::as_number).unwrap_or(0.0)
                    };
                    let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                    let (na, nb, nc, nd) = rotate_2d(a, b, c, d, e, f, angle_deg);
                    write_matrix_to_obj(&obj, na, nb, nc, nd, e, f, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("rotateSelf"),
                1,
            );

            // multiplySelf(other) — post-multiply this by other (2D).
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let other = args.first().and_then(JsValue::as_object).ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("multiplySelf: argument must be a DOMMatrix")
                    })?;
                    let (a1, b1, c1, d1, e1, f1) = read_matrix_components(&obj, ctx)?;
                    let (a2, b2, c2, d2, e2, f2) = read_matrix_components(&other, ctx)?;
                    let (na, nb, nc, nd, ne, nf) =
                        multiply_2d(a1, b1, c1, d1, e1, f1, a2, b2, c2, d2, e2, f2);
                    write_matrix_to_obj(&obj, na, nb, nc, nd, ne, nf, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("multiplySelf"),
                1,
            );

            // invertSelf() — invert the 2D matrix in-place.
            init.function(
                NativeFunction::from_copy_closure(|this, _args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                    if let Some((na, nb, nc, nd, ne, nf)) = invert_2d(a, b, c, d, e, f) {
                        write_matrix_to_obj(&obj, na, nb, nc, nd, ne, nf, ctx)?;
                    } else {
                        // Singular matrix — set all to NaN per spec.
                        write_matrix_to_obj(
                            &obj,
                            f64::NAN,
                            f64::NAN,
                            f64::NAN,
                            f64::NAN,
                            f64::NAN,
                            f64::NAN,
                            ctx,
                        )?;
                    }
                    Ok(this.clone())
                }),
                js_string!("invertSelf"),
                0,
            );
            // setMatrixValue(transformList) — parse "matrix(a,b,c,d,e,f)" or "none".
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let input = args
                        .first()
                        .map(|v| v.to_string(ctx))
                        .transpose()?
                        .map_or(String::new(), |s| s.to_std_string_escaped());
                    let trimmed = input.trim();
                    if trimmed == "none" || trimmed.is_empty() {
                        for key in ["a", "m11"] {
                            obj.set(js_string!(key), JsValue::from(1.0), false, ctx)?;
                        }
                        for key in ["b", "m12", "c", "m21", "e", "m41", "f", "m42"] {
                            obj.set(js_string!(key), JsValue::from(0.0), false, ctx)?;
                        }
                        for key in ["d", "m22"] {
                            obj.set(js_string!(key), JsValue::from(1.0), false, ctx)?;
                        }
                    } else if let Some(inner) =
                        trimmed.strip_prefix("matrix(").and_then(|s| s.strip_suffix(')'))
                    {
                        let parts: Vec<f64> = inner
                            .split(',')
                            .filter_map(|s| s.trim().parse::<f64>().ok())
                            .collect();
                        if parts.len() == 6 {
                            let keys = [
                                ("a", "m11"),
                                ("b", "m12"),
                                ("c", "m21"),
                                ("d", "m22"),
                                ("e", "m41"),
                                ("f", "m42"),
                            ];
                            for (i, (k1, k2)) in keys.iter().enumerate() {
                                obj.set(js_string!(*k1), JsValue::from(parts[i]), false, ctx)?;
                                obj.set(js_string!(*k2), JsValue::from(parts[i]), false, ctx)?;
                            }
                        } else {
                            return Err(JsNativeError::syntax()
                                .with_message("setMatrixValue: invalid matrix() format")
                                .into());
                        }
                    } else {
                        return Err(JsNativeError::syntax()
                            .with_message(
                                "setMatrixValue: unsupported transform (only matrix(a,b,c,d,e,f) and none)",
                            )
                            .into());
                    }
                    Ok(this.clone())
                }),
                js_string!("setMatrixValue"),
                1,
            );
        }

        // --- Immutable methods (return new DOMMatrix) ---

        // multiply(other) — return new DOMMatrix = this * other (2D).
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let other = args.first().and_then(JsValue::as_object).ok_or_else(|| {
                    JsNativeError::typ().with_message("multiply: argument must be a DOMMatrix")
                })?;
                let (a1, b1, c1, d1, e1, f1) = read_matrix_components(&obj, ctx)?;
                let (a2, b2, c2, d2, e2, f2) = read_matrix_components(&other, ctx)?;
                let (na, nb, nc, nd, ne, nf) =
                    multiply_2d(a1, b1, c1, d1, e1, f1, a2, b2, c2, d2, e2, f2);
                Ok(JsValue::from(build_dom_matrix_obj(
                    na, nb, nc, nd, ne, nf, ctx,
                )?))
            }),
            js_string!("multiply"),
            1,
        );

        // inverse() — return new inverted DOMMatrix (2D).
        init.function(
            NativeFunction::from_copy_closure(|this, _args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                let (na, nb, nc, nd, ne, nf) = invert_2d(a, b, c, d, e, f).unwrap_or((
                    f64::NAN,
                    f64::NAN,
                    f64::NAN,
                    f64::NAN,
                    f64::NAN,
                    f64::NAN,
                ));
                Ok(JsValue::from(build_dom_matrix_obj(
                    na, nb, nc, nd, ne, nf, ctx,
                )?))
            }),
            js_string!("inverse"),
            0,
        );

        // rotate(rotX, rotY?, rotZ?) — return new rotated DOMMatrix (2D Z-rotation).
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let angle_deg = if args.len() <= 1 {
                    args.first().and_then(JsValue::as_number).unwrap_or(0.0)
                } else {
                    args.get(2).and_then(JsValue::as_number).unwrap_or(0.0)
                };
                let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                let (na, nb, nc, nd) = rotate_2d(a, b, c, d, e, f, angle_deg);
                Ok(JsValue::from(build_dom_matrix_obj(
                    na, nb, nc, nd, e, f, ctx,
                )?))
            }),
            js_string!("rotate"),
            1,
        );

        // translate(tx, ty, tz?) — returns a new DOMMatrix with translation post-multiplied.
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let tx = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
                let ty = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
                let _tz = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
                let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                let ne = a * tx + c * ty + e;
                let nf = b * tx + d * ty + f;
                let result = build_dom_matrix_obj(a, b, c, d, ne, nf, ctx)?;
                Ok(JsValue::from(result))
            }),
            js_string!("translate"),
            2,
        );

        // scale(scaleX, scaleY?, scaleZ?) — returns a new scaled DOMMatrix.
        // Post-multiply by a scale matrix: a' = a*sx, b' = b*sx, c' = c*sy, d' = d*sy.
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let sx = args.first().and_then(JsValue::as_number).unwrap_or(1.0);
                let sy = args.get(1).and_then(JsValue::as_number).unwrap_or(sx);
                let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let e = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let f = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let result = build_dom_matrix_obj(a * sx, b * sx, c * sy, d * sy, e, f, ctx)?;
                Ok(JsValue::from(result))
            }),
            js_string!("scale"),
            1,
        );

        Ok(JsValue::from(init.build()))
    });

    ctx.register_global_callable(js_string!(name), 0, constructor)
        .expect("failed to register DOMMatrix");
}

// --- Pure 2D matrix math helpers ---

/// 2D matrix multiply: `[a1 c1 e1; b1 d1 f1; 0 0 1] * [a2 c2 e2; b2 d2 f2; 0 0 1]`.
#[allow(clippy::too_many_arguments)]
fn multiply_2d(
    a1: f64,
    b1: f64,
    c1: f64,
    d1: f64,
    e1: f64,
    f1: f64,
    a2: f64,
    b2: f64,
    c2: f64,
    d2: f64,
    e2: f64,
    f2: f64,
) -> (f64, f64, f64, f64, f64, f64) {
    (
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * e2 + c1 * f2 + e1,
        b1 * e2 + d1 * f2 + f1,
    )
}

/// Invert a 2D matrix. Returns `None` if the matrix is singular.
#[allow(clippy::many_single_char_names)]
fn invert_2d(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
) -> Option<(f64, f64, f64, f64, f64, f64)> {
    let det = a * d - b * c;
    if det.abs() < f64::EPSILON {
        return None;
    }
    let inv = 1.0 / det;
    Some((
        d * inv,
        -b * inv,
        -c * inv,
        a * inv,
        (c * f - d * e) * inv,
        (b * e - a * f) * inv,
    ))
}

/// Rotate a 2D matrix by `angle_deg` degrees around Z.
fn rotate_2d(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    _e: f64,
    _f: f64,
    angle_deg: f64,
) -> (f64, f64, f64, f64) {
    let angle = angle_deg * std::f64::consts::PI / 180.0;
    let cos = angle.cos();
    let sin = angle.sin();
    (
        a * cos + c * sin,
        b * cos + d * sin,
        a * -sin + c * cos,
        b * -sin + d * cos,
    )
}

/// Read the 6 2D matrix components (a-f) from a JS object.
fn read_matrix_components(
    obj: &boa_engine::JsObject,
    ctx: &mut Context,
) -> JsResult<(f64, f64, f64, f64, f64, f64)> {
    Ok((
        obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0),
        obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0),
        obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0),
        obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0),
        obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0),
        obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0),
    ))
}

/// Write 2D matrix components (a-f + m aliases) to a mutable `DOMMatrix` JS object.
#[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
fn write_matrix_to_obj(
    obj: &boa_engine::JsObject,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    ctx: &mut Context,
) -> JsResult<()> {
    obj.set(js_string!("a"), JsValue::from(a), false, ctx)?;
    obj.set(js_string!("m11"), JsValue::from(a), false, ctx)?;
    obj.set(js_string!("b"), JsValue::from(b), false, ctx)?;
    obj.set(js_string!("m12"), JsValue::from(b), false, ctx)?;
    obj.set(js_string!("c"), JsValue::from(c), false, ctx)?;
    obj.set(js_string!("m21"), JsValue::from(c), false, ctx)?;
    obj.set(js_string!("d"), JsValue::from(d), false, ctx)?;
    obj.set(js_string!("m22"), JsValue::from(d), false, ctx)?;
    obj.set(js_string!("e"), JsValue::from(e), false, ctx)?;
    obj.set(js_string!("m41"), JsValue::from(e), false, ctx)?;
    obj.set(js_string!("f"), JsValue::from(f), false, ctx)?;
    obj.set(js_string!("m42"), JsValue::from(f), false, ctx)?;
    Ok(())
}

#[allow(
    clippy::unnecessary_wraps,
    clippy::many_single_char_names,
    clippy::too_many_arguments
)]
fn build_dom_matrix_obj(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    ctx: &mut Context,
) -> JsResult<boa_engine::JsObject> {
    let attr = Attribute::WRITABLE | Attribute::CONFIGURABLE;
    let mut init = ObjectInitializer::new(ctx);
    init.property(js_string!("a"), JsValue::from(a), attr);
    init.property(js_string!("b"), JsValue::from(b), attr);
    init.property(js_string!("c"), JsValue::from(c), attr);
    init.property(js_string!("d"), JsValue::from(d), attr);
    init.property(js_string!("e"), JsValue::from(e), attr);
    init.property(js_string!("f"), JsValue::from(f), attr);
    init.property(js_string!("m11"), JsValue::from(a), attr);
    init.property(js_string!("m12"), JsValue::from(b), attr);
    init.property(js_string!("m13"), JsValue::from(0.0), attr);
    init.property(js_string!("m14"), JsValue::from(0.0), attr);
    init.property(js_string!("m21"), JsValue::from(c), attr);
    init.property(js_string!("m22"), JsValue::from(d), attr);
    init.property(js_string!("m23"), JsValue::from(0.0), attr);
    init.property(js_string!("m24"), JsValue::from(0.0), attr);
    init.property(js_string!("m31"), JsValue::from(0.0), attr);
    init.property(js_string!("m32"), JsValue::from(0.0), attr);
    init.property(js_string!("m33"), JsValue::from(1.0), attr);
    init.property(js_string!("m34"), JsValue::from(0.0), attr);
    init.property(js_string!("m41"), JsValue::from(e), attr);
    init.property(js_string!("m42"), JsValue::from(f), attr);
    init.property(js_string!("m43"), JsValue::from(0.0), attr);
    init.property(js_string!("m44"), JsValue::from(1.0), attr);
    let is_identity = (a - 1.0).abs() < f64::EPSILON
        && b.abs() < f64::EPSILON
        && c.abs() < f64::EPSILON
        && (d - 1.0).abs() < f64::EPSILON
        && e.abs() < f64::EPSILON
        && f.abs() < f64::EPSILON;
    init.property(
        js_string!("is2D"),
        JsValue::from(true),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    init.property(
        js_string!("isIdentity"),
        JsValue::from(is_identity),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    // transformPoint on the result matrix.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let dict = args.first().cloned().unwrap_or(JsValue::undefined());
            let (px, py, pz, pw) = extract_point_dict(&dict, ctx)?;
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("DOMMatrix: this is not an object")
            })?;
            let ma = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
            let mb = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let mc = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let md = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
            let me = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let mf = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let rx = ma * px + mc * py + me * pw;
            let ry = mb * px + md * py + mf * pw;
            Ok(JsValue::from(build_dom_point(rx, ry, pz, pw, true, ctx)?))
        }),
        js_string!("transformPoint"),
        0,
    );
    Ok(init.build())
}

/// Register `window.visualViewport` object.
#[allow(clippy::similar_names)]
pub(super) fn register_visual_viewport(ctx: &mut Context, bridge: &HostBridge) {
    use boa_engine::property::PropertyDescriptorBuilder;

    let global = ctx.global_object();

    let b = bridge.clone();
    let realm = ctx.realm().clone();

    // Build the visualViewport object with dynamic getters.
    let mut init = ObjectInitializer::new(ctx);

    // width — same as innerWidth.
    let b_w = b.clone();
    let w_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(f64::from(bridge.viewport_width())))
        },
        b_w,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("width"),
        Some(w_getter.clone()),
        None,
        Attribute::CONFIGURABLE,
    );

    // height — same as innerHeight.
    let b_h = b.clone();
    let h_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(f64::from(bridge.viewport_height())))
        },
        b_h,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("height"),
        Some(h_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // offsetLeft, offsetTop — offset of visual viewport from layout viewport.
    init.property(
        js_string!("offsetLeft"),
        JsValue::from(0.0),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    init.property(
        js_string!("offsetTop"),
        JsValue::from(0.0),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    // pageLeft, pageTop — offset relative to page origin.
    let b_pl = b.clone();
    let pl_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.scroll_x()))),
        b_pl,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("pageLeft"),
        Some(pl_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    let b_pt = b.clone();
    let pt_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.scroll_y()))),
        b_pt,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("pageTop"),
        Some(pt_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // scale — pinch-zoom scale factor (1.0 = no zoom).
    init.property(
        js_string!("scale"),
        JsValue::from(1.0),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    // addEventListener / removeEventListener stubs.
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("addEventListener"),
        2,
    );
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("removeEventListener"),
        2,
    );

    let vv = init.build();

    let desc = PropertyDescriptorBuilder::new()
        .value(vv)
        .writable(false)
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("visualViewport"), desc, ctx)
        .expect("failed to register visualViewport");
}

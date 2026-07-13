//! ECMA-262 §21.4 Date objects — constructor, statics, and prototype methods.
//!
//! The `[[DateValue]]` internal slot lives directly in
//! [`ObjectKind::Date`](super::value::ObjectKind::Date) as an `f64` time value
//! (milliseconds since the Unix epoch, or `NaN` for an Invalid Date). See
//! [`algorithms`] for the §21.4.1 time-value math and the **UTC-baseline**
//! rationale (`getTimezoneOffset` is always `+0`; local components equal UTC).

mod algorithms;
mod format;
mod parse;

use algorithms::{
    date_from_time, day, hour_from_time, make_date, make_day, make_time, min_from_time,
    month_from_time, ms_from_time, sec_from_time, time_clip, time_within_day, week_day,
    year_from_time,
};

use super::shape::PropertyAttrs;
use super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, PropertyValue, VmError,
};
use super::{NativeFn, VmInner};

/// Current Unix epoch milliseconds — the system-clock time value returned by
/// `Date.now()` / `new Date()`, and the single source every "current time"
/// call site shares (One issue, one way). Relocated here from `host/file.rs`
/// (where `File.lastModified` first needed it) so `Date` — the canonical
/// owner of wall-clock time — holds it.
///
/// `SystemTime::duration_since(UNIX_EPOCH)` fails only when the system clock
/// predates 1970 (extraordinarily unusual); fall back to `0` in that case.
/// Returns an integer `f64` (truncated to whole milliseconds), matching the
/// `Date.now()` browser observable + the WebIDL `long long` integer-truncation
/// rule applied to user-supplied timestamps.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn now_epoch_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// The `[[DateValue]]` of `this`, or a `TypeError`. Every `Date.prototype`
/// method begins with this brand check — `RequireInternalSlot(this,
/// [[DateValue]])` (e.g. §21.4.4.10 `getTime` step 1); the standalone
/// `thisTimeValue` AO was removed from the spec.
fn this_time_value(ctx: &NativeContext<'_>, this: JsValue) -> Result<f64, VmError> {
    if let JsValue::Object(id) = this {
        if let ObjectKind::Date(t) = ctx.get_object(id).kind {
            return Ok(t);
        }
    }
    Err(VmError::type_error(
        "Date.prototype method called on incompatible receiver",
    ))
}

/// The receiver's `ObjectId` **and** current `[[DateValue]]`, or a `TypeError`
/// — the combined brand-check-and-slot-read the `set*` methods need (they write
/// the slot back via the id and derive new fields from `t`). A single
/// `get_object` lookup, with no unreachable NaN fallback.
fn date_this(ctx: &NativeContext<'_>, this: JsValue) -> Result<(ObjectId, f64), VmError> {
    if let JsValue::Object(id) = this {
        if let ObjectKind::Date(t) = ctx.get_object(id).kind {
            return Ok((id, t));
        }
    }
    Err(VmError::type_error(
        "Date.prototype method called on incompatible receiver",
    ))
}

/// `ToNumber(args[i])`, treating a missing argument as `undefined` (→ `NaN`).
fn arg_num(ctx: &mut NativeContext<'_>, args: &[JsValue], i: usize) -> Result<f64, VmError> {
    ctx.to_number(args.get(i).copied().unwrap_or(JsValue::Undefined))
}

/// `ToNumber(args[i])` if present, else the (already-computed) `default`.
fn opt_num(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    i: usize,
    default: f64,
) -> Result<f64, VmError> {
    match args.get(i) {
        Some(&v) => ctx.to_number(v),
        None => Ok(default),
    }
}

/// §21.4.1.30 MakeFullYear — the two-digit-year rule (`0..=99` → `1900 + y`),
/// shared by the component constructor (§21.4.2.1) and `Date.UTC` (§21.4.3.4).
fn make_full_year(y: f64) -> f64 {
    if y.is_finite() {
        let yi = y.trunc();
        if (0.0..=99.0).contains(&yi) {
            return 1900.0 + yi;
        }
    }
    y
}

/// Store `tv` on a known-Date receiver and return it as the method result.
fn store(ctx: &mut NativeContext<'_>, id: ObjectId, tv: f64) -> Result<JsValue, VmError> {
    ctx.vm.promote_to_date(id, tv);
    Ok(JsValue::Number(tv))
}

// ---------------------------------------------------------------------------
// Constructor + statics (§21.4.2 / §21.4.3)
// ---------------------------------------------------------------------------

/// §21.4.2.1 `Date(...)` / `new Date(...)`.
pub(crate) fn native_date_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Mirrors `native_number_constructor`'s positive `is_construct()` shape —
    // the construct/call split stays a single `if`/`else` here rather than an
    // early `!is_construct()` guard (native-ctor-guard trip-wire discipline).
    if ctx.is_construct() {
        let tv = match args {
            [] => time_clip(now_epoch_ms()),
            [v] => date_from_single(ctx, *v)?,
            _ => date_from_components(ctx, args)?,
        };
        match this {
            JsValue::Object(instance_id) => {
                ctx.vm.promote_to_date(instance_id, tv);
                Ok(JsValue::Object(instance_id))
            }
            // Defensive: `do_new` always passes an Object receiver.
            _ => Ok(JsValue::Object(ctx.vm.create_date(tv))),
        }
    } else {
        // Called as a plain function (no `new`): return the current time as a
        // String; the arguments are ignored (§21.4.2.1, called-as-function).
        let now = time_clip(now_epoch_ms());
        let sid = ctx.intern(&format::to_string(now));
        Ok(JsValue::String(sid))
    }
}

/// §21.4.2.1 — the single-argument form.
fn date_from_single(ctx: &mut NativeContext<'_>, v: JsValue) -> Result<f64, VmError> {
    // If the argument is itself a Date, copy its time value verbatim.
    if let JsValue::Object(id) = v {
        if let ObjectKind::Date(t) = ctx.get_object(id).kind {
            return Ok(t);
        }
    }
    let prim = ctx.vm.to_primitive(v, "default")?;
    if let JsValue::String(sid) = prim {
        let s = ctx.get_utf8(sid);
        Ok(parse::parse(&s))
    } else {
        let n = ctx.to_number(prim)?;
        Ok(time_clip(n))
    }
}

/// §21.4.2.1 — the multi-argument `(year, month[, date, hours, …])` form.
#[allow(clippy::many_single_char_names)] // spec field names: y, m, d, h, s
fn date_from_components(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<f64, VmError> {
    let y = ctx.to_number(args[0])?;
    let m = ctx.to_number(args[1])?;
    let d = opt_num(ctx, args, 2, 1.0)?;
    let h = opt_num(ctx, args, 3, 0.0)?;
    let mi = opt_num(ctx, args, 4, 0.0)?;
    let s = opt_num(ctx, args, 5, 0.0)?;
    let ms = opt_num(ctx, args, 6, 0.0)?;
    let yr = make_full_year(y);
    // Components are local time; `UTC(finalDate)` is the identity here.
    let final_date = make_date(make_day(yr, m, d), make_time(h, mi, s, ms));
    Ok(time_clip(final_date))
}

/// §21.4.3.1 `Date.now()`.
fn native_date_now(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(time_clip(now_epoch_ms())))
}

/// §21.4.3.2 `Date.parse(string)`.
fn native_date_parse(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = ctx.to_string_val(arg)?;
    let s = ctx.get_utf8(sid);
    Ok(JsValue::Number(parse::parse(&s)))
}

/// §21.4.3.4 `Date.UTC(year, month, …)`. Unlike the constructor, `month`
/// defaults to `+0` and the components are already UTC (no `UTC()` wrap).
#[allow(clippy::many_single_char_names)] // spec field names: y, m, d, h, s
fn native_date_utc(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let y = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let m = opt_num(ctx, args, 1, 0.0)?;
    let d = opt_num(ctx, args, 2, 1.0)?;
    let h = opt_num(ctx, args, 3, 0.0)?;
    let mi = opt_num(ctx, args, 4, 0.0)?;
    let s = opt_num(ctx, args, 5, 0.0)?;
    let ms = opt_num(ctx, args, 6, 0.0)?;
    let yr = make_full_year(y);
    let tv = make_date(make_day(yr, m, d), make_time(h, mi, s, ms));
    Ok(JsValue::Number(time_clip(tv)))
}

// ---------------------------------------------------------------------------
// Prototype getters (§21.4.4). UTC-baseline: local and UTC share one impl.
// ---------------------------------------------------------------------------

/// A component getter: `NaN` receiver → `NaN`, else `f(t)`.
fn component(
    ctx: &NativeContext<'_>,
    this: JsValue,
    f: fn(f64) -> f64,
) -> Result<JsValue, VmError> {
    let t = this_time_value(ctx, this)?;
    Ok(JsValue::Number(if t.is_nan() { f64::NAN } else { f(t) }))
}

/// §21.4.4.10 `getTime` / §21.4.4.44 `valueOf` — the raw time value.
fn native_date_get_time(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(this_time_value(ctx, this)?))
}

macro_rules! date_getter {
    ($name:ident, $f:expr) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            component(ctx, this, $f)
        }
    };
}

// Per-getter §21.4.4.x citations (plain comments — a doc comment before a
// macro invocation would not attach to the generated fn). Each UTC alias
// (`getUTC*`) shares the base impl under the UTC-baseline.
// §21.4.4.4 getFullYear
date_getter!(native_date_get_full_year, year_from_time);
// §21.4.4.8 getMonth
date_getter!(native_date_get_month, month_from_time);
// §21.4.4.2 getDate
date_getter!(native_date_get_date, date_from_time);
// §21.4.4.3 getDay
date_getter!(native_date_get_day, week_day);
// §21.4.4.5 getHours
date_getter!(native_date_get_hours, hour_from_time);
// §21.4.4.7 getMinutes
date_getter!(native_date_get_minutes, min_from_time);
// §21.4.4.9 getSeconds
date_getter!(native_date_get_seconds, sec_from_time);
// §21.4.4.6 getMilliseconds
date_getter!(native_date_get_milliseconds, ms_from_time);

// §21.4.4.11 getTimezoneOffset — always +0 under the UTC-baseline (finite
// receiver → zero offset; NaN stays NaN, both via `component`).
date_getter!(native_date_get_timezone_offset, |_| 0.0);

// ---------------------------------------------------------------------------
// Prototype setters (§21.4.4). UTC-baseline: local and UTC share one impl.
// ---------------------------------------------------------------------------

/// §21.4.4.27 `setTime(time)`.
fn native_date_set_time(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, _t) = date_this(ctx, this)?;
    let v = arg_num(ctx, args, 0)?;
    store(ctx, id, time_clip(v))
}

/// §21.4.4.23 `setMilliseconds(ms)`.
fn native_date_set_milliseconds(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t) = date_this(ctx, this)?;
    let ms = arg_num(ctx, args, 0)?;
    let time = make_time(hour_from_time(t), min_from_time(t), sec_from_time(t), ms);
    store(ctx, id, time_clip(make_date(day(t), time)))
}

/// §21.4.4.26 `setSeconds(sec, ms?)`.
fn native_date_set_seconds(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t) = date_this(ctx, this)?;
    let sec = arg_num(ctx, args, 0)?;
    let ms = opt_num(ctx, args, 1, ms_from_time(t))?;
    let time = make_time(hour_from_time(t), min_from_time(t), sec, ms);
    store(ctx, id, time_clip(make_date(day(t), time)))
}

/// §21.4.4.24 `setMinutes(min, sec?, ms?)`.
fn native_date_set_minutes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t) = date_this(ctx, this)?;
    let min = arg_num(ctx, args, 0)?;
    let sec = opt_num(ctx, args, 1, sec_from_time(t))?;
    let ms = opt_num(ctx, args, 2, ms_from_time(t))?;
    let time = make_time(hour_from_time(t), min, sec, ms);
    store(ctx, id, time_clip(make_date(day(t), time)))
}

/// §21.4.4.22 `setHours(hour, min?, sec?, ms?)`.
fn native_date_set_hours(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t) = date_this(ctx, this)?;
    let hour = arg_num(ctx, args, 0)?;
    let min = opt_num(ctx, args, 1, min_from_time(t))?;
    let sec = opt_num(ctx, args, 2, sec_from_time(t))?;
    let ms = opt_num(ctx, args, 3, ms_from_time(t))?;
    let time = make_time(hour, min, sec, ms);
    store(ctx, id, time_clip(make_date(day(t), time)))
}

/// §21.4.4.20 `setDate(date)`.
fn native_date_set_date(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t) = date_this(ctx, this)?;
    let dt = arg_num(ctx, args, 0)?;
    let new_day = make_day(year_from_time(t), month_from_time(t), dt);
    store(ctx, id, time_clip(make_date(new_day, time_within_day(t))))
}

/// §21.4.4.25 `setMonth(month, date?)`.
fn native_date_set_month(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t) = date_this(ctx, this)?;
    let mo = arg_num(ctx, args, 0)?;
    let dt = opt_num(ctx, args, 1, date_from_time(t))?;
    let new_day = make_day(year_from_time(t), mo, dt);
    store(ctx, id, time_clip(make_date(new_day, time_within_day(t))))
}

/// §21.4.4.21 `setFullYear(year, month?, date?)` — the one setter that can
/// revive an Invalid Date (a `NaN` receiver is treated as `+0`).
fn native_date_set_full_year(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, t0) = date_this(ctx, this)?;
    let t = if t0.is_nan() { 0.0 } else { t0 };
    let y = arg_num(ctx, args, 0)?;
    let mo = opt_num(ctx, args, 1, month_from_time(t))?;
    let dt = opt_num(ctx, args, 2, date_from_time(t))?;
    let new_day = make_day(y, mo, dt);
    store(ctx, id, time_clip(make_date(new_day, time_within_day(t))))
}

// ---------------------------------------------------------------------------
// Prototype string conversions (§21.4.4) + Symbol.toPrimitive
// ---------------------------------------------------------------------------

/// §21.4.4.36 `toISOString` — throws `RangeError` on a non-finite time value.
fn native_date_to_iso_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let t = this_time_value(ctx, this)?;
    if !t.is_finite() {
        return Err(VmError::range_error("Invalid time value"));
    }
    let sid = ctx.intern(&format::iso_string(t));
    Ok(JsValue::String(sid))
}

/// §21.4.4.37 `toJSON` — `null` for a non-finite time value, else the ISO
/// string.
fn native_date_to_json(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let t = this_time_value(ctx, this)?;
    if !t.is_finite() {
        return Ok(JsValue::Null);
    }
    let sid = ctx.intern(&format::iso_string(t));
    Ok(JsValue::String(sid))
}

macro_rules! date_stringifier {
    ($name:ident, $f:path) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let t = this_time_value(ctx, this)?;
            let sid = ctx.intern(&$f(t));
            Ok(JsValue::String(sid))
        }
    };
}

date_stringifier!(native_date_to_string, format::to_string);
date_stringifier!(native_date_to_date_string, format::to_date_string);
date_stringifier!(native_date_to_time_string, format::to_time_string);
date_stringifier!(native_date_to_utc_string, format::utc_string);

/// §21.4.4.45 `Date.prototype[Symbol.toPrimitive](hint)`. Our default
/// `toString`/`valueOf` return primitives directly, so `OrdinaryToPrimitive`
/// short-circuits: `"number"` → the time value, `"string"`/`"default"` → the
/// `toString` rendering.
fn native_date_to_primitive(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !matches!(this, JsValue::Object(_)) {
        return Err(VmError::type_error(
            "Date.prototype[Symbol.toPrimitive] called on non-object",
        ));
    }
    let hint = match args.first() {
        Some(JsValue::String(sid)) => ctx.get_utf8(*sid),
        _ => String::new(),
    };
    match hint.as_str() {
        "number" => Ok(JsValue::Number(this_time_value(ctx, this)?)),
        "string" | "default" => {
            let t = this_time_value(ctx, this)?;
            let sid = ctx.intern(&format::to_string(t));
            Ok(JsValue::String(sid))
        }
        _ => Err(VmError::type_error(
            "Date.prototype[Symbol.toPrimitive] called with invalid hint",
        )),
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `Date`, `Date.prototype`, and the statics (§21.4). Mirrors
    /// [`register_number_prototype`](Self::register_number_prototype).
    pub(super) fn register_date_prototype(&mut self) {
        let proto_methods: &[(&str, NativeFn)] = &[
            ("getTime", native_date_get_time),
            ("valueOf", native_date_get_time),
            ("getFullYear", native_date_get_full_year),
            ("getUTCFullYear", native_date_get_full_year),
            ("getMonth", native_date_get_month),
            ("getUTCMonth", native_date_get_month),
            ("getDate", native_date_get_date),
            ("getUTCDate", native_date_get_date),
            ("getDay", native_date_get_day),
            ("getUTCDay", native_date_get_day),
            ("getHours", native_date_get_hours),
            ("getUTCHours", native_date_get_hours),
            ("getMinutes", native_date_get_minutes),
            ("getUTCMinutes", native_date_get_minutes),
            ("getSeconds", native_date_get_seconds),
            ("getUTCSeconds", native_date_get_seconds),
            ("getMilliseconds", native_date_get_milliseconds),
            ("getUTCMilliseconds", native_date_get_milliseconds),
            ("getTimezoneOffset", native_date_get_timezone_offset),
            ("setTime", native_date_set_time),
            ("setMilliseconds", native_date_set_milliseconds),
            ("setUTCMilliseconds", native_date_set_milliseconds),
            ("setSeconds", native_date_set_seconds),
            ("setUTCSeconds", native_date_set_seconds),
            ("setMinutes", native_date_set_minutes),
            ("setUTCMinutes", native_date_set_minutes),
            ("setHours", native_date_set_hours),
            ("setUTCHours", native_date_set_hours),
            ("setDate", native_date_set_date),
            ("setUTCDate", native_date_set_date),
            ("setMonth", native_date_set_month),
            ("setUTCMonth", native_date_set_month),
            ("setFullYear", native_date_set_full_year),
            ("setUTCFullYear", native_date_set_full_year),
            ("toISOString", native_date_to_iso_string),
            ("toJSON", native_date_to_json),
            ("toString", native_date_to_string),
            ("toDateString", native_date_to_date_string),
            ("toTimeString", native_date_to_time_string),
            ("toUTCString", native_date_to_utc_string),
        ];
        let proto_id = self.create_object_with_methods(proto_methods);
        self.date_prototype = Some(proto_id);

        // `Date.prototype[Symbol.toPrimitive]` — `{ ¬W, ¬E, C }` (§21.4.4.45).
        let to_prim_fn =
            self.create_native_function("[Symbol.toPrimitive]", native_date_to_primitive);
        let to_prim_key = PropertyKey::Symbol(self.well_known_symbols.to_primitive);
        self.define_shaped_property(
            proto_id,
            to_prim_key,
            PropertyValue::Data(JsValue::Object(to_prim_fn)),
            PropertyAttrs {
                writable: false,
                enumerable: false,
                configurable: true,
                is_accessor: false,
            },
        );

        let ctor_id = self.create_constructable_function("Date", native_date_constructor);
        self.wire_constructor_global("Date", ctor_id, proto_id);

        let statics: &[(&str, NativeFn)] = &[
            ("now", native_date_now),
            ("parse", native_date_parse),
            ("UTC", native_date_utc),
        ];
        for &(name, func) in statics {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                ctor_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }
}

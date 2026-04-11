//! ES spec formatting and key enumeration utilities.
//!
//! - `write_number_es`: ES §7.1.12.1 Number::toString
//! - `collect_own_keys_es_order`: ES §9.1.11.1 OrdinaryOwnPropertyKeys

use super::value::{ObjectId, PropertyKey, StringId};
use super::VmInner;

// ---------------------------------------------------------------------------
// ES key enumeration order (§9.1.11.1 OrdinaryOwnPropertyKeys)
// ---------------------------------------------------------------------------

/// Collect own enumerable string keys in ES spec order (§9.1.11.1):
/// array-index keys in ascending numeric order, then other string keys
/// in insertion order.
pub(crate) fn collect_own_keys_es_order(vm: &VmInner, obj_id: ObjectId) -> Vec<StringId> {
    let obj = vm.get_object(obj_id);
    let mut index_keys: Vec<(u32, StringId)> = Vec::new();
    let mut other_keys: Vec<StringId> = Vec::new();

    for (k, attrs) in obj.storage.iter_keys(&vm.shapes) {
        if !attrs.enumerable {
            continue;
        }
        let sid = match k {
            PropertyKey::String(s) => s,
            PropertyKey::Symbol(_) => continue,
        };
        match parse_array_index_u32(vm.strings.get(sid)) {
            Some(idx) => index_keys.push((idx, sid)),
            None => other_keys.push(sid),
        }
    }

    index_keys.sort_by_key(|(idx, _)| *idx);

    let mut keys = Vec::with_capacity(index_keys.len() + other_keys.len());
    keys.extend(index_keys.into_iter().map(|(_, sid)| sid));
    keys.extend(other_keys);
    keys
}

/// Parse a WTF-16 string as an ES array index (0..2^32-2). Returns `None` for
/// non-index strings, leading-zero forms like "01", and out-of-range values.
pub(crate) fn parse_array_index_u32(units: &[u16]) -> Option<u32> {
    if units.is_empty() || units.len() > 10 {
        return None;
    }
    if units.len() > 1 && units[0] == u16::from(b'0') {
        return None;
    }
    let mut val: u64 = 0;
    for &u in units {
        let d = u.wrapping_sub(u16::from(b'0'));
        if d > 9 {
            return None;
        }
        val = val * 10 + u64::from(d);
        if val > u64::from(u32::MAX) - 1 {
            return None;
        }
    }
    Some(val as u32)
}

// ---------------------------------------------------------------------------
// ES Number::toString (§7.1.12.1)
// ---------------------------------------------------------------------------

/// Write a finite positive or negative f64 to a `String` buffer in ES
/// Number::toString format (§7.1.12.1).
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
pub(crate) fn write_number_es(n: f64, output: &mut String) {
    debug_assert!(n.is_finite());
    if n == 0.0 {
        output.push('0');
        return;
    }
    if n < 0.0 {
        output.push('-');
        write_number_es(-n, output);
        return;
    }

    let (sig, exp) = extract_sig_exp(n);
    let k = sig.len() as i32;

    if k <= exp && exp <= 21 {
        output.push_str(&sig);
        for _ in 0..(exp - k) {
            output.push('0');
        }
    } else if 0 < exp && exp < k {
        let e = exp as usize;
        output.push_str(&sig[..e]);
        output.push('.');
        output.push_str(&sig[e..]);
    } else if exp <= 0 {
        let leading_zeros = (-exp) as usize;
        if leading_zeros < 6 {
            output.push_str("0.");
            for _ in 0..leading_zeros {
                output.push('0');
            }
            output.push_str(&sig);
        } else {
            write_es_exponent(&sig, exp, output);
        }
    } else {
        write_es_exponent(&sig, exp, output);
    }
}

/// Format significant digits + exponent in ES exponent notation.
fn write_es_exponent(sig: &str, exp: i32, output: &mut String) {
    use std::fmt::Write;
    if sig.len() == 1 {
        output.push_str(sig);
    } else {
        output.push(sig.as_bytes()[0] as char);
        output.push('.');
        output.push_str(&sig[1..]);
    }
    let e = exp - 1;
    if e >= 0 {
        let _ = write!(output, "e+{e}");
    } else {
        let _ = write!(output, "e{e}");
    }
}

/// Extract the significant digits and decimal exponent from Rust's f64 Display.
#[allow(clippy::cast_possible_wrap)]
fn extract_sig_exp(n: f64) -> (String, i32) {
    let s = format!("{n}");
    if let Some(e_pos) = s.find('e') {
        let mantissa = &s[..e_pos];
        let exp_str = &s[e_pos + 1..];
        let rust_exp: i32 = exp_str.parse().unwrap_or(0);
        let sig: String = mantissa.replace('.', "");
        let sig = sig.trim_end_matches('0');
        let dot_offset = mantissa.find('.').map_or(mantissa.len(), |p| p);
        let exp = rust_exp + dot_offset as i32;
        (sig.to_string(), exp)
    } else if let Some(dot_pos) = s.find('.') {
        let sig: String = s.replace('.', "");
        let sig = sig.trim_start_matches('0');
        let sig_trimmed = sig.trim_end_matches('0');
        let before_dot = &s[..dot_pos];
        if before_dot == "0" {
            let frac = &s[dot_pos + 1..];
            let leading_zeros = frac.bytes().take_while(|&b| b == b'0').count();
            let exp = -(leading_zeros as i32);
            (sig_trimmed.to_string(), exp)
        } else {
            let exp = before_dot.len() as i32;
            (sig_trimmed.to_string(), exp)
        }
    } else {
        let sig = s.trim_end_matches('0');
        let exp = s.len() as i32;
        (sig.to_string(), exp)
    }
}

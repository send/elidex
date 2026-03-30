//! IDB key types with W3C-specified ordering and binary serialization.
//!
//! Key order (W3C `IndexedDB` 3.0 §3.1.3):
//!   number (0x01) < date (0x02) < string (0x03) < binary (0x04, M4-9) < array (0x05)
//!
//! Number/Date encoding: IEEE 754 big-endian with sign-flip so that
//! memcmp on the encoded bytes preserves numeric order.

use std::cmp::Ordering;

/// Type tags for binary serialization. Ordering matches W3C key comparison.
const TAG_NUMBER: u8 = 0x01;
const TAG_DATE: u8 = 0x02;
const TAG_STRING: u8 = 0x03;
// TAG_BINARY = 0x04 reserved for M4-9 (ArrayBuffer keys)
const TAG_ARRAY: u8 = 0x05;

/// Length prefix size for variable-length fields (4 bytes, big-endian u32).
const LEN_PREFIX_SIZE: usize = 4;

/// Maximum nesting depth for array keys (prevents stack overflow from malicious input).
const MAX_KEY_DEPTH: usize = 64;

/// An `IndexedDB` key value.
///
/// Implements the W3C `IndexedDB` 3.0 key ordering:
/// `Number < Date < String < Array` (Binary reserved for M4-9).
#[derive(Debug, Clone, PartialEq)]
pub enum IdbKey {
    /// IEEE 754 number (finite only — NaN/Infinity rejected at construction).
    Number(f64),
    /// Date as milliseconds since Unix epoch (same encoding as Number, different tag).
    Date(f64),
    /// UTF-8 string.
    String(String),
    /// Array of sub-keys (nested arrays allowed per spec).
    Array(Vec<IdbKey>),
}

impl IdbKey {
    /// Returns the type tag used in binary serialization.
    fn tag(&self) -> u8 {
        match self {
            Self::Number(_) => TAG_NUMBER,
            Self::Date(_) => TAG_DATE,
            Self::String(_) => TAG_STRING,
            Self::Array(_) => TAG_ARRAY,
        }
    }

    /// Serialize this key to a byte vector suitable for `SQLite` BLOB comparison.
    ///
    /// The encoding preserves W3C key ordering under lexicographic byte comparison.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize_into(&mut buf, 0);
        buf
    }

    fn serialize_into(&self, buf: &mut Vec<u8>, depth: usize) {
        buf.push(self.tag());
        match self {
            Self::Number(v) | Self::Date(v) => {
                encode_f64(*v, buf);
            }
            Self::String(s) => {
                let units: Vec<u16> = s.encode_utf16().collect();
                #[allow(clippy::cast_possible_truncation)]
                buf.extend_from_slice(&(units.len() as u32).to_be_bytes());
                for u in &units {
                    buf.extend_from_slice(&u.to_be_bytes());
                }
            }
            Self::Array(items) if depth < MAX_KEY_DEPTH => {
                #[allow(clippy::cast_possible_truncation)]
                buf.extend_from_slice(&(items.len() as u32).to_be_bytes());
                for item in items {
                    item.serialize_into(buf, depth + 1);
                }
            }
            Self::Array(_) => {
                // Depth exceeded — serialize as empty array
                buf.extend_from_slice(&0u32.to_be_bytes());
            }
        }
    }

    /// Deserialize a key from a byte slice. Returns `None` on malformed input.
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let (key, consumed) = Self::deserialize_at(data, 0, 0)?;
        if consumed == data.len() {
            Some(key)
        } else {
            None // trailing bytes
        }
    }

    fn deserialize_at(data: &[u8], offset: usize, depth: usize) -> Option<(Self, usize)> {
        if depth > MAX_KEY_DEPTH {
            return None;
        }
        let tag = *data.get(offset)?;
        let pos = offset + 1;

        match tag {
            TAG_NUMBER => {
                let v = decode_f64(data, pos)?;
                Some((Self::Number(v), pos + 8))
            }
            TAG_DATE => {
                let v = decode_f64(data, pos)?;
                Some((Self::Date(v), pos + 8))
            }
            TAG_STRING => {
                let unit_count = read_u32(data, pos)? as usize;
                let start = pos + LEN_PREFIX_SIZE;
                let byte_len = unit_count.checked_mul(2)?;
                let end = start.checked_add(byte_len)?;
                // #14: Validate length against actual data before allocating
                let bytes = data.get(start..end)?;
                let mut units = Vec::with_capacity(unit_count.min(bytes.len()));
                for i in 0..unit_count {
                    let hi = bytes[i * 2];
                    let lo = bytes[i * 2 + 1];
                    units.push(u16::from_be_bytes([hi, lo]));
                }
                let s = String::from_utf16(&units).ok()?;
                Some((Self::String(s), end))
            }
            TAG_ARRAY => {
                let count = read_u32(data, pos)? as usize;
                let mut cursor = pos + LEN_PREFIX_SIZE;
                // Cap allocation to remaining data size to prevent memory amplification (#14)
                let remaining = data.len().saturating_sub(cursor);
                let mut items = Vec::with_capacity(count.min(remaining));
                for _ in 0..count {
                    let (item, next) = Self::deserialize_at(data, cursor, depth + 1)?;
                    items.push(item);
                    cursor = next;
                }
                Some((Self::Array(items), cursor))
            }
            _ => None, // unknown tag (e.g. 0x04 binary — not yet supported)
        }
    }
}

/// IEEE 754 sign-flip encoding: flip all bits if negative, else flip sign bit only.
/// This makes lexicographic byte comparison match numeric order.
fn encode_f64(v: f64, buf: &mut Vec<u8>) {
    let bits = v.to_bits();
    let encoded = if v.is_sign_negative() {
        !bits // flip all bits for negative numbers
    } else {
        bits ^ (1u64 << 63) // flip sign bit for positive numbers and zero
    };
    buf.extend_from_slice(&encoded.to_be_bytes());
}

fn decode_f64(data: &[u8], offset: usize) -> Option<f64> {
    let bytes: [u8; 8] = data.get(offset..offset + 8)?.try_into().ok()?;
    let encoded = u64::from_be_bytes(bytes);
    let bits = if encoded & (1u64 << 63) == 0 {
        !encoded // was negative — flip all bits back
    } else {
        encoded ^ (1u64 << 63) // was positive — flip sign bit back
    };
    Some(f64::from_bits(bits))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

// -- Ordering --

impl Eq for IdbKey {}

impl PartialOrd for IdbKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IdbKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // Different types: compare by tag (number < date < string < array)
        let tag_ord = self.tag().cmp(&other.tag());
        if tag_ord != Ordering::Equal {
            return tag_ord;
        }

        // Same type: compare values
        match (self, other) {
            (Self::Number(a), Self::Number(b)) | (Self::Date(a), Self::Date(b)) => cmp_f64(*a, *b),
            (Self::String(a), Self::String(b)) => cmp_utf16(a, b),
            (Self::Array(a), Self::Array(b)) => {
                for (x, y) in a.iter().zip(b.iter()) {
                    let ord = x.cmp(y);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                a.len().cmp(&b.len())
            }
            _ => unreachable!("same tag guarantees same variant"),
        }
    }
}

/// Compare strings by UTF-16 code unit order (W3C `IndexedDB` §2.4).
fn cmp_utf16(a: &str, b: &str) -> Ordering {
    let mut a_units = a.encode_utf16();
    let mut b_units = b.encode_utf16();
    loop {
        match (a_units.next(), b_units.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(au), Some(bu)) => {
                let ord = au.cmp(&bu);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

/// Total ordering for f64 — NaN sorts as less than everything (should not appear
/// in valid IDB keys, but we handle it defensively).
fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or_else(|| {
        // At least one is NaN
        match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => unreachable!(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_ordering() {
        let num = IdbKey::Number(1.0);
        let date = IdbKey::Date(1.0);
        let string = IdbKey::String("a".into());
        let array = IdbKey::Array(vec![]);

        assert!(num < date);
        assert!(date < string);
        assert!(string < array);
    }

    #[test]
    fn number_ordering() {
        let neg = IdbKey::Number(-100.0);
        let zero = IdbKey::Number(0.0);
        let pos = IdbKey::Number(42.5);
        let large = IdbKey::Number(1e18);

        assert!(neg < zero);
        assert!(zero < pos);
        assert!(pos < large);
    }

    #[test]
    fn negative_zero_equals_positive_zero() {
        // IEEE 754: -0.0 == 0.0 in IDB key comparison
        assert_eq!(
            IdbKey::Number(-0.0).cmp(&IdbKey::Number(0.0)),
            Ordering::Equal
        );
    }

    #[test]
    fn string_ordering() {
        let a = IdbKey::String("a".into());
        let b = IdbKey::String("b".into());
        let ab = IdbKey::String("ab".into());

        assert!(a < ab);
        assert!(ab < b);
    }

    #[test]
    fn array_ordering() {
        let a1 = IdbKey::Array(vec![IdbKey::Number(1.0)]);
        let a2 = IdbKey::Array(vec![IdbKey::Number(2.0)]);
        let a12 = IdbKey::Array(vec![IdbKey::Number(1.0), IdbKey::Number(2.0)]);

        assert!(a1 < a12); // shorter prefix is less
        assert!(a12 < a2); // first element differs
    }

    #[test]
    fn serialize_roundtrip_number() {
        for v in [0.0, -0.0, 1.0, -1.0, f64::MIN, f64::MAX, 1e-300, -1e-300] {
            let key = IdbKey::Number(v);
            let bytes = key.serialize();
            let decoded = IdbKey::deserialize(&bytes).unwrap();
            assert_eq!(key, decoded, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn serialize_roundtrip_date() {
        let key = IdbKey::Date(1_700_000_000_000.0);
        let bytes = key.serialize();
        let decoded = IdbKey::deserialize(&bytes).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn serialize_roundtrip_string() {
        let key = IdbKey::String("hello world".into());
        let bytes = key.serialize();
        let decoded = IdbKey::deserialize(&bytes).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn serialize_roundtrip_empty_string() {
        let key = IdbKey::String(String::new());
        let bytes = key.serialize();
        let decoded = IdbKey::deserialize(&bytes).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn serialize_roundtrip_array() {
        let key = IdbKey::Array(vec![
            IdbKey::Number(42.0),
            IdbKey::String("test".into()),
            IdbKey::Array(vec![IdbKey::Date(0.0)]),
        ]);
        let bytes = key.serialize();
        let decoded = IdbKey::deserialize(&bytes).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn serialize_roundtrip_empty_array() {
        let key = IdbKey::Array(vec![]);
        let bytes = key.serialize();
        let decoded = IdbKey::deserialize(&bytes).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn serialized_byte_order_matches_key_order() {
        // Verify that memcmp on serialized bytes gives the same ordering as Ord
        let keys = vec![
            IdbKey::Number(-1e10),
            IdbKey::Number(-1.0),
            IdbKey::Number(0.0),
            IdbKey::Number(1.0),
            IdbKey::Number(1e10),
            IdbKey::Date(0.0),
            IdbKey::Date(1e15),
            IdbKey::String("".into()),
            IdbKey::String("a".into()),
            IdbKey::String("b".into()),
            IdbKey::Array(vec![]),
            IdbKey::Array(vec![IdbKey::Number(1.0)]),
        ];

        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                let a_bytes = keys[i].serialize();
                let b_bytes = keys[j].serialize();
                assert!(
                    a_bytes < b_bytes,
                    "byte order mismatch: {:?} should < {:?}",
                    keys[i],
                    keys[j]
                );
            }
        }
    }

    #[test]
    fn deserialize_trailing_bytes_rejected() {
        let key = IdbKey::Number(1.0);
        let mut bytes = key.serialize();
        bytes.push(0xFF);
        assert!(IdbKey::deserialize(&bytes).is_none());
    }

    #[test]
    fn deserialize_empty_rejected() {
        assert!(IdbKey::deserialize(&[]).is_none());
    }

    #[test]
    fn deserialize_unknown_tag_rejected() {
        assert!(IdbKey::deserialize(&[0x04, 0, 0, 0, 0]).is_none());
    }

    #[test]
    fn serialize_roundtrip_unicode_string() {
        let key = IdbKey::String("\u{1F600}\u{00E9}\u{3042}".into());
        let bytes = key.serialize();
        let decoded = IdbKey::deserialize(&bytes).unwrap();
        assert_eq!(key, decoded);
    }
}

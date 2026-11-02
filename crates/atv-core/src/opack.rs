//! OPACK binary serialization, ported from `fake_atv.py`
//! (`opack_pack`/`_opack_pack` and `opack_unpack`/`_opack_unpack`).
//!
//! Notes on fidelity:
//! * `Value::Int` holds a u64. `pack` sizes integers the way Apple's CoreUtils does
//!   (signed int8/16/32/64), unlike `fake_atv.opack_pack`, for 0x80-0xFF, 0x8000-0xFFFF
//!   and 0x8000_0000-0xFFFF_FFFF. `unpack` reads sized integers unsigned, like the
//!   reference, and never yields negatives.
//! * `Float` unpacked from a 0x35 marker becomes a `Double`, like Python's
//!   single `float` type; packing a `Double` always emits 0x36.
//! * The object-reference table on unpack uses strict, type-aware equality
//!   (Python's `in` would consider `1 == 1.0`; real Companion traffic never
//!   relies on that quirk).

use std::fmt;

/// An OPACK value. Dicts preserve insertion order, like Python dicts.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Uuid([u8; 16]),
    /// Absolute time (0x06), decoded as a raw little-endian u64 like the
    /// Python reference does.
    Time(u64),
    Int(u64),
    Double(f64),
    Str(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Dict(Vec<(Value, Value)>),
}

impl Value {
    pub fn as_dict(&self) -> Option<&Vec<(Value, Value)>> {
        match self {
            Value::Dict(entries) => Some(entries),
            _ => None,
        }
    }

    /// Look up `key` in a dict value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_dict()?.iter().find_map(|(k, v)| match k {
            Value::Str(s) if s == key => Some(v),
            _ => None,
        })
    }

    pub fn as_int(&self) -> Option<u64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Double(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub fn str(s: &str) -> Value {
        Value::Str(s.to_string())
    }

    pub fn int(i: u64) -> Value {
        Value::Int(i)
    }

    pub fn dict(entries: Vec<(Value, Value)>) -> Value {
        Value::Dict(entries)
    }

    pub fn kv(key: &str, value: Value) -> (Value, Value) {
        (Value::str(key), value)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "None"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Uuid(u) => write!(f, "uuid:{}", hexish(u)),
            Value::Time(t) => write!(f, "time:{t}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Double(d) => write!(f, "{d}"),
            Value::Str(s) => write!(f, "{s:?}"),
            Value::Bytes(b) => write!(f, "<bytes:{}>", b.len()),
            Value::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Dict(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

fn hexish(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpackError(pub String);

impl fmt::Display for OpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OPACK error: {}", self.0)
    }
}

impl std::error::Error for OpackError {}

/// Pack a value with OPACK. Sized integers follow Apple's CoreUtils (see the module notes);
/// everything else is byte-identical to `fake_atv.opack_pack`.
pub fn pack(data: &Value) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    pack_inner(data, &mut objects)
}

fn pack_inner(data: &Value, objects: &mut Vec<Vec<u8>>) -> Vec<u8> {
    let packed: Vec<u8> = match data {
        Value::Null => vec![0x04],
        Value::Bool(b) => vec![if *b { 1 } else { 2 }],
        Value::Uuid(u) => {
            let mut out = vec![0x05];
            out.extend_from_slice(u);
            out
        }
        Value::Time(t) => {
            // Python raises for packing absolute time; keep the marker for
            // completeness so re-packing unpacked data works.
            let mut out = vec![0x06];
            out.extend_from_slice(&t.to_le_bytes());
            out
        }
        Value::Int(i) => {
            // Sized integers are signed (int8/16/32/64), as Apple's CoreUtils encodes and
            // decodes them: a value with the top bit of its size set reads back negative on
            // the iPhone. Its request ids (_x) are random over the whole u32 range, so an id
            // >= 2^31 must go out in 8 bytes; in 4 the iPhone does not match the reply.
            if *i < 0x28 {
                vec![(*i as u8) + 8]
            } else if *i <= 0x7F {
                vec![0x30, *i as u8]
            } else if *i <= 0x7FFF {
                let mut out = vec![0x31];
                out.extend_from_slice(&(*i as u16).to_le_bytes());
                out
            } else if *i <= 0x7FFF_FFFF {
                let mut out = vec![0x32];
                out.extend_from_slice(&(*i as u32).to_le_bytes());
                out
            } else {
                let mut out = vec![0x33];
                out.extend_from_slice(&i.to_le_bytes());
                out
            }
        }
        Value::Double(d) => {
            let mut out = vec![0x36];
            out.extend_from_slice(&d.to_le_bytes());
            out
        }
        Value::Str(s) => {
            let encoded = s.as_bytes();
            let len = encoded.len();
            let mut out = if len <= 0x20 {
                vec![0x40 + len as u8]
            } else if len <= 0xFF {
                vec![0x61, len as u8]
            } else if len <= 0xFFFF {
                let mut v = vec![0x62];
                v.extend_from_slice(&(len as u16).to_le_bytes());
                v
            } else {
                let mut v = vec![0x64];
                v.extend_from_slice(&(len as u32).to_le_bytes());
                v
            };
            out.extend_from_slice(encoded);
            out
        }
        Value::Bytes(b) => {
            let len = b.len();
            let mut out = if len <= 0x20 {
                vec![0x70 + len as u8]
            } else if len <= 0xFF {
                vec![0x91, len as u8]
            } else if len <= 0xFFFF {
                let mut v = vec![0x92];
                v.extend_from_slice(&(len as u16).to_le_bytes());
                v
            } else {
                let mut v = vec![0x93];
                v.extend_from_slice(&(len as u32).to_le_bytes());
                v
            };
            out.extend_from_slice(b);
            out
        }
        Value::Array(items) => {
            let mut out = vec![0xD0 + items.len().min(0xF) as u8];
            for item in items {
                out.extend_from_slice(&pack_inner(item, objects));
            }
            if items.len() >= 0xF {
                out.push(0x03);
            }
            out
        }
        Value::Dict(entries) => {
            let mut out = vec![0xE0 + entries.len().min(0xF) as u8];
            for (key, value) in entries {
                out.extend_from_slice(&pack_inner(key, objects));
                out.extend_from_slice(&pack_inner(value, objects));
            }
            if entries.len() >= 0xF {
                out.push(0x03);
            }
            out
        }
    };

    // Reuse via the object-reference table when possible.
    if packed.len() > 1 {
        if let Some(index) = objects.iter().position(|o| *o == packed) {
            if index < 0x21 {
                return vec![0xA0 + index as u8];
            }
            if index <= 0xFF {
                return vec![0xC1, index as u8];
            }
            let mut out = vec![0xC2];
            out.extend_from_slice(&(index as u16).to_le_bytes());
            return out;
        }
        objects.push(packed.clone());
    }
    packed
}

/// Unpack OPACK data, returning the value and the remaining bytes,
/// byte-identical to `fake_atv.opack_unpack`.
pub fn unpack(data: &[u8]) -> Result<(Value, &[u8]), OpackError> {
    let mut objects: Vec<Value> = Vec::new();
    unpack_inner(data, &mut objects)
}

fn unpack_inner<'a>(data: &'a [u8], objects: &mut Vec<Value>) -> Result<(Value, &'a [u8]), OpackError> {
    let err = |msg: &str| OpackError(msg.to_string());
    if data.is_empty() {
        return Err(err("empty input"));
    }
    let marker = data[0];
    let mut add_ref = true;

    let (value, remaining): (Value, &[u8]) = match marker {
        0x01 => {
            add_ref = false;
            (Value::Bool(true), &data[1..])
        }
        0x02 => {
            add_ref = false;
            (Value::Bool(false), &data[1..])
        }
        0x04 => {
            add_ref = false;
            (Value::Null, &data[1..])
        }
        0x05 => {
            if data.len() < 17 {
                return Err(err("truncated uuid"));
            }
            let mut u = [0u8; 16];
            u.copy_from_slice(&data[1..17]);
            (Value::Uuid(u), &data[17..])
        }
        0x06 => {
            if data.len() < 9 {
                return Err(err("truncated time"));
            }
            let t = u64::from_le_bytes(data[1..9].try_into().unwrap());
            (Value::Time(t), &data[9..])
        }
        0x08..=0x2F => {
            add_ref = false;
            (Value::Int((marker - 8) as u64), &data[1..])
        }
        0x35 => {
            if data.len() < 5 {
                return Err(err("truncated f32"));
            }
            let v = f32::from_le_bytes(data[1..5].try_into().unwrap());
            (Value::Double(v as f64), &data[5..])
        }
        0x36 => {
            if data.len() < 9 {
                return Err(err("truncated f64"));
            }
            let v = f64::from_le_bytes(data[1..9].try_into().unwrap());
            (Value::Double(v), &data[9..])
        }
        m if (m & 0xF0) == 0x30 => {
            let size = 1usize << (m & 0xF);
            if size > 8 {
                // 0x34+ would be a 16-byte integer; Python's bigint handles
                // it, but Companion never sends these and Value::Int is u64.
                return Err(err("sized int wider than 8 bytes"));
            }
            if data.len() < 1 + size {
                return Err(err("truncated sized int"));
            }
            let mut v: u64 = 0;
            for (i, b) in data[1..1 + size].iter().enumerate() {
                v |= (*b as u64) << (8 * i);
            }
            (Value::Int(v), &data[1 + size..])
        }
        0x40..=0x60 => {
            let size = (marker - 0x40) as usize;
            if data.len() < 1 + size {
                return Err(err("truncated string"));
            }
            let s = std::str::from_utf8(&data[1..1 + size])
                .map_err(|_| err("invalid utf-8 string"))?
                .to_string();
            (Value::Str(s), &data[1 + size..])
        }
        0x61..=0x64 => {
            let size_len = (marker & 0xF) as usize;
            if data.len() < 1 + size_len {
                return Err(err("truncated string length"));
            }
            let mut size: usize = 0;
            for (i, b) in data[1..1 + size_len].iter().enumerate() {
                size |= (*b as usize) << (8 * i);
            }
            let start = 1 + size_len;
            if data.len() < start + size {
                return Err(err("truncated string body"));
            }
            let s = std::str::from_utf8(&data[start..start + size])
                .map_err(|_| err("invalid utf-8 string"))?
                .to_string();
            (Value::Str(s), &data[start + size..])
        }
        0x70..=0x90 => {
            let size = (marker - 0x70) as usize;
            if data.len() < 1 + size {
                return Err(err("truncated bytes"));
            }
            (Value::Bytes(data[1..1 + size].to_vec()), &data[1 + size..])
        }
        0x91..=0x94 => {
            let size_len = 1usize << ((marker & 0xF) - 1);
            if data.len() < 1 + size_len {
                return Err(err("truncated bytes length"));
            }
            let mut size: usize = 0;
            for (i, b) in data[1..1 + size_len].iter().enumerate() {
                size |= (*b as usize) << (8 * i);
            }
            let start = 1 + size_len;
            if data.len() < start + size {
                return Err(err("truncated bytes body"));
            }
            (
                Value::Bytes(data[start..start + size].to_vec()),
                &data[start + size..],
            )
        }
        m if (m & 0xF0) == 0xD0 => {
            let count = (m & 0xF) as usize;
            let mut output = Vec::new();
            let mut ptr: &[u8] = &data[1..];
            if count == 0xF {
                loop {
                    if ptr.is_empty() {
                        return Err(err("unterminated endless list"));
                    }
                    if ptr[0] == 0x03 {
                        break;
                    }
                    let (item, rest) = unpack_inner(ptr, objects)?;
                    output.push(item);
                    ptr = rest;
                }
                ptr = &ptr[1..];
            } else {
                for _ in 0..count {
                    let (item, rest) = unpack_inner(ptr, objects)?;
                    output.push(item);
                    ptr = rest;
                }
            }
            add_ref = false;
            (Value::Array(output), ptr)
        }
        m if (m & 0xE0) == 0xE0 => {
            let count = (m & 0xF) as usize;
            let mut output: Vec<(Value, Value)> = Vec::new();
            let mut ptr: &[u8] = &data[1..];
            if count == 0xF {
                loop {
                    if ptr.is_empty() {
                        return Err(err("unterminated endless dict"));
                    }
                    if ptr[0] == 0x03 {
                        break;
                    }
                    let (key, rest) = unpack_inner(ptr, objects)?;
                    let (val, rest) = unpack_inner(rest, objects)?;
                    output.push((key, val));
                    ptr = rest;
                }
                ptr = &ptr[1..];
            } else {
                for _ in 0..count {
                    let (key, rest) = unpack_inner(ptr, objects)?;
                    let (val, rest) = unpack_inner(rest, objects)?;
                    output.push((key, val));
                    ptr = rest;
                }
            }
            add_ref = false;
            (Value::Dict(output), ptr)
        }
        0xA0..=0xC0 => {
            let index = (marker - 0xA0) as usize;
            let value = objects
                .get(index)
                .ok_or_else(|| err("object reference out of range"))?
                .clone();
            add_ref = false;
            (value, &data[1..])
        }
        0xC1..=0xC4 => {
            let size_len = (marker - 0xC0) as usize;
            if data.len() < 1 + size_len {
                return Err(err("truncated object reference"));
            }
            let mut index: usize = 0;
            for (i, b) in data[1..1 + size_len].iter().enumerate() {
                index |= (*b as usize) << (8 * i);
            }
            let value = objects
                .get(index)
                .ok_or_else(|| err("object reference out of range"))?
                .clone();
            add_ref = false;
            (value, &data[1 + size_len..])
        }
        m => return Err(err(&format!("unknown OPACK marker {m:#x}"))),
    };

    if add_ref && !objects.contains(&value) {
        objects.push(value.clone());
    }
    Ok((value, remaining))
}

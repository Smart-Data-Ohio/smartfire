//! The subset of Ruby's `Marshal.dump` (format 4.8) needed to digest variation transformations:
//! `ActiveStorage::Variation#digest` is `SHA1.base64digest(Marshal.dump(transformations))`.
//!
//! Symbols and strings are distinct: transformations built in Ruby carry symbols
//! (`format: :webp`), while a variation decoded from a signed URL key carries strings
//! (`format: "webp"`), and the two digest differently.

/// A Ruby value as it appears in a transformations hash (after `deep_symbolize_keys`).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Symbol(String),
    /// A UTF-8 string (the encoding of every string Active Storage puts in a transformation).
    Str(String),
    Array(Vec<Value>),
    /// A hash with symbol keys, in insertion order.
    Hash(Vec<(String, Value)>),
}

pub fn dump(value: &Value) -> Vec<u8> {
    let mut writer = Writer { out: vec![4, 8], symbols: Vec::new() };
    writer.value(value);
    writer.out
}

struct Writer {
    out: Vec<u8>,
    symbols: Vec<String>,
}

impl Writer {
    fn value(&mut self, value: &Value) {
        match value {
            Value::Nil => self.out.push(b'0'),
            Value::Bool(true) => self.out.push(b'T'),
            Value::Bool(false) => self.out.push(b'F'),
            Value::Int(n) => self.integer(*n),
            Value::Symbol(name) => self.symbol(name),
            Value::Str(s) => {
                // A string with an encoding is wrapped in an ivar list holding `E: true` (UTF-8).
                self.out.push(b'I');
                self.out.push(b'"');
                self.bytes(s.as_bytes());
                self.long(1);
                self.symbol("E");
                self.out.push(b'T');
            }
            Value::Array(items) => {
                self.out.push(b'[');
                self.long(items.len() as i64);
                for item in items {
                    self.value(item);
                }
            }
            Value::Hash(entries) => {
                self.out.push(b'{');
                self.long(entries.len() as i64);
                for (key, item) in entries {
                    self.symbol(key);
                    self.value(item);
                }
            }
        }
    }

    /// Integers whose tagged VALUE fits in 32 bits (31-bit signed) are written inline (`i`),
    /// larger ones as bignums (`l`).
    fn integer(&mut self, n: i64) {
        if (-(1i64 << 30)..(1i64 << 30)).contains(&n) {
            self.out.push(b'i');
            self.long(n);
        } else {
            self.out.push(b'l');
            self.out.push(if n < 0 { b'-' } else { b'+' });
            let mut magnitude = n.unsigned_abs();
            let mut digits = Vec::new();
            while magnitude > 0 {
                digits.push((magnitude & 0xff) as u8);
                magnitude >>= 8;
            }
            if digits.len() % 2 == 1 {
                digits.push(0);
            }
            self.long((digits.len() / 2) as i64);
            self.out.extend_from_slice(&digits);
        }
    }

    fn symbol(&mut self, name: &str) {
        if let Some(index) = self.symbols.iter().position(|s| s == name) {
            self.out.push(b';');
            self.long(index as i64);
        } else {
            self.symbols.push(name.to_string());
            self.out.push(b':');
            self.bytes(name.as_bytes());
        }
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.long(bytes.len() as i64);
        self.out.extend_from_slice(bytes);
    }

    /// `w_long` in marshal.c.
    fn long(&mut self, n: i64) {
        if n == 0 {
            self.out.push(0);
        } else if 0 < n && n < 123 {
            self.out.push((n + 5) as u8);
        } else if -124 < n && n < 0 {
            self.out.push(((n - 5) & 0xff) as u8);
        } else {
            let mut buf = [0u8; 9];
            let mut x = n;
            for i in 1..=8 {
                buf[i] = (x & 0xff) as u8;
                x >>= 8;
                if x == 0 {
                    buf[0] = i as u8;
                    self.out.extend_from_slice(&buf[..=i]);
                    return;
                }
                if x == -1 {
                    buf[0] = (-(i as i8)) as u8;
                    self.out.extend_from_slice(&buf[..=i]);
                    return;
                }
            }
        }
    }
}

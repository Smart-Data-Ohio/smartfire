//! Ruby JSON.parse + String/Hash/Array#[] and #to_s for API errors. Object order survives.
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::Number;
use std::fmt;
#[derive(Clone)]
enum RubyJson {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}
impl<'de> Deserialize<'de> for RubyJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = RubyJson;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(RubyJson::Null)
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
                Ok(RubyJson::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(RubyJson::Number(v.into()))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(RubyJson::Number(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Number::from_f64(v)
                    .map(RubyJson::Number)
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(RubyJson::String(v.into()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = seq.next_element()? {
                    values.push(v)
                }
                Ok(RubyJson::Array(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values: Vec<(String, RubyJson)> = Vec::new();
                while let Some((k, v)) = map.next_entry()? {
                    if let Some(entry) = values.iter_mut().find(|(key, _)| *key == k) {
                        entry.1 = v;
                    } else {
                        values.push((k, v));
                    }
                }
                Ok(RubyJson::Object(values))
            }
        }
        d.deserialize_any(JsonVisitor)
    }
}
impl RubyJson {
    fn truthy(&self) -> bool {
        !matches!(self, Self::Null | Self::Bool(false))
    }
    fn get(&self, key: &str) -> Result<Option<Self>, &'static str> {
        match self {
            Self::Object(entries) => Ok(entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())),
            Self::String(s) => Ok(s.contains(key).then(|| Self::String(key.into()))),
            Self::Array(_) => Err("Type Error"),
            Self::Number(n) if n.is_i64() || n.is_u64() => Err("Type Error"),
            _ => Err("No Method Error"),
        }
    }
    fn to_s(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::String(s) => s.clone(),
            _ => self.inspect(),
        }
    }
    fn inspect(&self) -> String {
        match self {
            Self::Null => "nil".into(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => serde_json::to_string(s).unwrap(),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::inspect).collect::<Vec<_>>().join(", ")
            ),
            Self::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!(
                        "{} => {}",
                        serde_json::to_string(k).unwrap(),
                        v.inspect()
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}
pub(super) fn message(body: &[u8]) -> Result<String, &'static str> {
    let parsed: RubyJson = if within_nesting_limit(body) {
        serde_json::from_slice(body).unwrap_or_else(|_| RubyJson::Object(vec![]))
    } else {
        RubyJson::Object(vec![])
    };
    let mut selected = None;
    for key in ["message", "error", "errors"] {
        if let Some(value) = parsed.get(key)?.filter(RubyJson::truthy) {
            selected = Some(value);
            break;
        }
    }
    let message = selected
        .map(|v| v.to_s())
        .unwrap_or_default()
        .replace("@[", "@\u{200b}[");
    let message = regex::Regex::new(r"[\r\n]+")
        .unwrap()
        .replace_all(&message, " ");
    let message = message.trim_matches(['\0', ' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    Ok(if message.is_empty() {
        "request was not allowed".into()
    } else {
        message.into()
    })
}

/// JSON.parse's default max_nesting is 100; braces inside strings do not contribute.
pub(super) fn within_nesting_limit(body: &[u8]) -> bool {
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for byte in body {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'[' | b'{' => {
                    depth += 1;
                    if depth > 100 {
                        return false;
                    }
                }
                b']' | b'}' => depth = depth.saturating_sub(1),
                _ => (),
            }
        }
    }
    true
}

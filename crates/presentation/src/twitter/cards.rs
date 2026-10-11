use crate::helpers as h;

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Card {
    pub post_id: String,
    pub view_url: String,
    pub display_name: String,
    pub display_handle: Option<String>,
    pub profile_url: Option<String>,
    pub author_avatar_url: Option<String>,
    pub text: Option<String>,
    pub posted_at: Option<jiff::Timestamp>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub likes: Option<i64>,
    #[serde(deserialize_with = "media_or_empty")]
    pub media: Vec<Value>,
    pub quote: Option<Value>,
    pub fetched_at: Option<jiff::Timestamp>,
    pub fetch_error: Option<String>,
    pub logo_url: Option<String>,
}
pub fn media_or_empty<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Value>, D::Error> {
    Ok(Option::<Vec<Value>>::deserialize(deserializer)?.unwrap_or_default())
}
pub fn present(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !h::is_blank(s))
}
pub fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

impl Card {
    pub fn error(&self) -> bool {
        present(self.fetch_error.as_deref()).is_some()
    }
    pub fn handle(&self) -> Option<&str> {
        present(self.display_handle.as_deref())
    }
    pub fn body(&self) -> Option<&str> {
        present(self.text.as_deref())
    }
    pub fn media(&self) -> &[Value] {
        &self.media[..self.media.len().min(4)]
    }
    pub fn photo(item: &Value) -> bool {
        item["type"] == "photo"
    }
    pub fn quote(&self) -> Option<&Value> {
        self.quote.as_ref().filter(|q| match q {
            Value::Null | Value::Bool(false) => false,
            Value::String(s) => !h::is_blank(s),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            _ => true,
        })
    }
}

//! `Fizzy::AgentCardAction`; shared by WS11's REST/MCP approval-request adapters.
use super::client::{Client, Error};
use campfire_db::Errors;
use campfire_richtext::ruby::{is_blank, json_value_to_s, strip, truncate};
use serde_json::{Value, json};
pub struct Action {
    fields: Value,
}
impl Action {
    /// Approval numbers are Ruby Integers and may exceed both i64 and u64. Retain
    /// their literal decimal digits rather than rounding them through an f64.
    pub fn from_stored(stored: &str) -> Option<Self> {
        if !super::error_body::within_nesting_limit(stored.as_bytes()) {
            return None;
        }
        let raw: std::collections::HashMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(stored).ok()?;
        let mut fields: Value = serde_json::from_str(stored).ok()?;
        if let Some(number) = raw.get("number").map(|v| v.get())
            && !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit())
        {
            fields["number"] = Value::String(number.into());
        }
        Some(Self::from_payload(fields))
    }
    pub fn from_payload(fields: Value) -> Self {
        Self { fields }
    }
    fn text(&self, key: &str) -> String {
        json_value_to_s(&self.fields[key])
    }
    fn normalized(&self, key: &str) -> Option<String> {
        let text = self.text(key);
        let text = strip(&text);
        (!is_blank(text)).then(|| text.to_owned())
    }
    fn number(&self) -> Option<String> {
        let number = self.text("number");
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let digits = number.trim_start_matches('0');
        Some(if digits.is_empty() { "0" } else { digits }.to_owned())
    }
    pub fn errors(&self) -> Errors {
        let mut errors = Errors::default();
        let kind = self.text("kind");
        if !["create", "comment", "move", "close", "reopen"].contains(&kind.as_str()) {
            errors.add(
                "kind",
                "must be one of: create, comment, move, close, reopen",
            );
        }
        if super::blank(&self.fields["account_id"]) {
            errors.add("account_id", "can't be blank");
        }
        for key in ["account_id", "board_id", "column_id"] {
            let value = self.text(key);
            if key != "account_id" && super::blank(&self.fields[key]) {
                continue;
            }
            if value.is_empty()
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                errors.add(key, "is invalid");
            }
        }
        match kind.as_str() {
            "create" => {
                if self.normalized("board_id").is_none() {
                    errors.add("board_id", "is required to create a card");
                }
                if self.normalized("title").is_none() {
                    errors.add("title", "is required to create a card");
                }
            }
            "comment" => {
                if self.number().is_none() {
                    errors.add("number", "is required");
                }
                if self.normalized("body").is_none() {
                    errors.add("body", "is required for a comment");
                }
            }
            "move" => {
                if self.number().is_none() {
                    errors.add("number", "is required");
                }
                if self.normalized("column_id").is_none() {
                    errors.add("column_id", "is required to move a card");
                }
            }
            "close" | "reopen" if self.number().is_none() => errors.add("number", "is required"),
            _ => {}
        }
        for (key, limit) in [("title", 500), ("description", 3500), ("body", 3500)] {
            if self
                .normalized(key)
                .is_some_and(|s| s.chars().count() > limit)
            {
                errors.add(key, format!("is too long (maximum is {limit} characters)"));
            }
        }
        // Presence checks above deliberately accept number zero, just like Ruby's Integer#blank?.
        errors
    }
    pub fn action_name(&self) -> String {
        format!("fizzy.{}", self.text("kind"))
    }
    pub fn summary(&self) -> Option<String> {
        let account = self.text("account_id");
        let number = self.number().unwrap_or_default();
        let normalized = |key| self.normalized(key).unwrap_or_default();
        Some(match self.text("kind").as_str() {
            "create" => truncate(
                &format!(
                    "Create Fizzy card in board {} (account {account}): {}",
                    normalized("board_id"),
                    normalized("title").chars().take(120).collect::<String>()
                ),
                500,
                "...",
            ),
            "comment" => format!(
                "Comment on Fizzy card #{number} (account {account}): {}",
                normalized("body").chars().take(120).collect::<String>()
            ),
            "move" => format!(
                "Move Fizzy card #{number} (account {account}) to column {}",
                normalized("column_id")
            ),
            "close" => format!("Close Fizzy card #{number} (account {account})"),
            "reopen" => format!("Reopen Fizzy card #{number} (account {account})"),
            _ => return None,
        })
    }
    /// All eight keys (including nulls), in the reference hash's insertion order.
    #[allow(
        dead_code,
        reason = "WS11's approval request adapters consume this serialization seam"
    )]
    pub fn payload_json(&self) -> String {
        let kind = self.text("kind");
        let fields = [
            ("account_id", json!(self.text("account_id"))),
            ("kind", json!(kind)),
            (
                "board_id",
                if kind == "create" {
                    json!(self.normalized("board_id"))
                } else {
                    Value::Null
                },
            ),
            (
                "number",
                if kind != "create" {
                    json!(self.number())
                } else {
                    Value::Null
                },
            ),
            (
                "column_id",
                if kind == "move" {
                    json!(self.normalized("column_id"))
                } else {
                    Value::Null
                },
            ),
            (
                "title",
                if kind == "create" {
                    json!(self.normalized("title"))
                } else {
                    Value::Null
                },
            ),
            (
                "description",
                if kind == "create" {
                    json!(self.normalized("description"))
                } else {
                    Value::Null
                },
            ),
            (
                "body",
                if kind == "comment" {
                    json!(self.normalized("body"))
                } else {
                    Value::Null
                },
            ),
        ];
        format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(k, v)| if *k == "number" && kind != "create" {
                    format!("\"{k}\":{}", self.number().unwrap_or("null".into()))
                } else {
                    format!("\"{k}\":{v}")
                })
                .collect::<Vec<_>>()
                .join(",")
        )
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
    }
    pub async fn perform(&self, client: &Client) -> Result<Value, Error> {
        let account = self.text("account_id");
        let number = self.number().unwrap_or_default();
        match self.text("kind").as_str() {
            "create" => {
                client
                    .create_card(
                        &account,
                        &self.normalized("board_id").unwrap_or_default(),
                        json!(self.normalized("title")),
                        Some(json!(self.normalized("description"))),
                    )
                    .await
            }
            "comment" => {
                client
                    .create_comment(&account, &number, json!(self.normalized("body")))
                    .await
            }
            "move" => client
                .move_to_column(&account, &number, json!(self.normalized("column_id")))
                .await
                .map(Value::Bool),
            "close" => client.close_card(&account, &number).await.map(Value::Bool),
            "reopen" => client.reopen_card(&account, &number).await.map(Value::Bool),
            _ => Ok(Value::Null),
        }
    }
}
#[cfg(test)]
mod tests;

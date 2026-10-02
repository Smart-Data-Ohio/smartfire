//! Human room membership and canonical recipients from `Rooms::DriveRecipientsController`.
use crate::{Connection, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recipient {
    pub id: i64,
    pub name: String,
    pub email: String,
}
// URI::MailTo::EMAIL_REGEXP from pinned Rails' uri gem. Test vectors validate
// this constant; release code must not depend on the parity fixture directory.
const EMAIL_PATTERN: &str = r"\A[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*\z";
pub fn eligible_email(email: &str) -> bool {
    static PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(EMAIL_PATTERN).expect("URI::MailTo::EMAIL_REGEXP")
    });
    PATTERN.is_match(email)
}
pub fn eligible(conn: &Connection, room_id: i64, requester_id: i64) -> Result<Vec<Recipient>> {
    let rows=conn.prepare("SELECT u.id,u.name,u.email_address FROM users u JOIN memberships m ON m.user_id=u.id WHERE m.room_id=? AND u.id!=? AND u.status=0 AND u.role!=2 AND NOT EXISTS(SELECT 1 FROM agents a WHERE a.user_id=u.id) AND u.email_address IS NOT NULL AND u.email_address!='' ORDER BY LOWER(u.name),u.id")?.query_map([room_id,requester_id],|r|Ok(Recipient{id:r.get(0)?,name:r.get(1)?,email:r.get(2)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .filter(|r| eligible_email(&r.email))
        .collect())
}
pub fn normalized_ids(raw: &Value) -> Option<Vec<u64>> {
    let mut ids = vec![];
    for value in raw.as_array()? {
        let text = campfire_richtext::ruby::json_value_to_s(value);
        let text = campfire_richtext::ruby::strip(&text);
        if text.is_empty() || !text.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let id = text.parse().ok()?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Some(ids)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rails_email_and_selection_vectors() {
        let v: Value =
            serde_json::from_str(include_str!("../../../../vectors/google_drive_policy.json"))
                .unwrap();
        assert_eq!(EMAIL_PATTERN, v["email_pattern"].as_str().unwrap().replace("\\#", "#"));
        for case in v["emails"].as_array().unwrap() {
            assert_eq!(
                eligible_email(case["input"].as_str().unwrap()),
                case["eligible"],
                "{case}"
            );
        }
        for case in v["selections"].as_array().unwrap() {
            assert_eq!(
                serde_json::to_value(normalized_ids(&case["input"])).unwrap(),
                case["ids"],
                "{case}"
            );
        }
    }
}

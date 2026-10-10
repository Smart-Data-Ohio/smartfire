// Huddle render models, independent of the database and the request/session.
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Participant {
    pub id: i64,
    pub name: String,
    pub avatar_path: String,
}

use crate::helpers as h;

use jiff::Timestamp;

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct GithubCard {
    pub id: i64,
    pub full_name: String,
    pub display_name: String,
    pub number: i64,
    pub state: String,
    pub title: String,
    pub author: String,
    pub avatar: String,
    pub base: String,
    pub head: String,
    pub review: String,
    pub checks: String,
    pub updated_at: Option<Timestamp>,
    pub url: String,
    pub error: String,
    pub thread_id: Option<i64>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct EmbedCard {
    pub url: String,
    pub title: String,
    pub description: String,
    pub site: String,
    pub image: String,
    pub player_url: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub enum GithubEntry {
    Public(Box<GithubCard>),
    Private(String),
}
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct EmbedEntry {
    pub linkedin: bool,
    pub card: EmbedCard,
}

pub fn present(value: &str) -> bool {
    !h::is_blank(value)
}

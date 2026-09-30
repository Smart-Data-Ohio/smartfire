//! Read-only provider facts for the bounded message page. Provider owners retain fetch/write APIs.
use crate::{Result, Timestamp};
use rusqlite::{Connection, Row};
use std::collections::HashMap;

#[derive(Clone)]
pub struct GithubCard {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    pub private: Option<bool>,
    pub state: Option<String>,
    pub title: Option<String>,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub base_branch: Option<String>,
    pub head_branch: Option<String>,
    pub review_decision: Option<String>,
    pub check_status: Option<String>,
    pub github_updated_at: Option<Timestamp>,
    pub html_url: Option<String>,
    pub fetch_error: Option<String>,
    pub payload: Option<String>,
    pub discussion_thread_id: Option<i64>,
}
impl GithubCard {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            owner: r.get("owner")?,
            repo: r.get("repo")?,
            number: r.get("number")?,
            private: r.get("private")?,
            state: r.get("state")?,
            title: r.get("title")?,
            author_login: r.get("author_login")?,
            author_avatar_url: r.get("author_avatar_url")?,
            base_branch: r.get("base_branch")?,
            head_branch: r.get("head_branch")?,
            review_decision: r.get("review_decision")?,
            check_status: r.get("check_status")?,
            github_updated_at: r.get("github_updated_at")?,
            html_url: r.get("html_url")?,
            fetch_error: r.get("fetch_error")?,
            payload: r.get("payload")?,
            discussion_thread_id: r.get("discussion_thread_id")?,
        })
    }
}
#[derive(Clone)]
pub struct EmbedCard {
    pub url: String,
    pub normalized_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_url: Option<String>,
}
#[derive(Default)]
pub struct Providers {
    pub github: HashMap<i64, Vec<GithubCard>>,
    pub embeds: HashMap<i64, Vec<EmbedCard>>,
}
impl Providers {
    pub(super) fn load(conn: &Connection, ids: &[i64]) -> Result<Self> {
        let mut data = Self::default();
        for (message,card) in super::rows(conn,
            "SELECT r.message_id,c.*,t.channel_thread_id AS discussion_thread_id FROM github_pull_request_references r
             JOIN github_pull_requests c ON c.id=r.github_pull_request_id JOIN messages m ON m.id=r.message_id
             LEFT JOIN github_pull_request_threads t ON t.github_pull_request_id=c.id AND t.room_id=m.room_id
             WHERE r.message_id IN ($ids) ORDER BY c.owner,c.repo,c.number",ids,
             |r| Ok((r.get::<_,i64>("message_id")?,GithubCard::from_row(r)?)))? {
            data.github.entry(message).or_default().push(card);
        }
        for (message,card) in super::rows(conn,
            "SELECT r.message_id,r.url,c.normalized_url,c.title,c.description,c.site_name,c.image_url FROM link_embed_references r
             JOIN link_embeds c ON c.id=r.link_embed_id WHERE r.message_id IN ($ids) ORDER BY r.position,r.id",ids,
             |r| {
                 let url:Option<String>=r.get("url")?;
                 let normalized_url:String=r.get("normalized_url")?;
                 let url=url.filter(|s| !campfire_richtext::ruby::is_blank(s)).unwrap_or_else(||normalized_url.clone());
                 Ok((r.get::<_,i64>("message_id")?,EmbedCard { url,normalized_url,title:r.get("title")?,description:r.get("description")?,site_name:r.get("site_name")?,image_url:r.get("image_url")? }))
             })? {
            data.embeds.entry(message).or_default().push(card);
        }
        Ok(data)
    }
}

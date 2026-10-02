//! `app/models/work_thread_link.rb`. Links have no inbox, history, audit or thread-touch hook.
use crate::sql::{query_all, query_one};
use crate::{ChannelThread, Errors, Result, Timestamp, Tx, User};
use campfire_richtext::ruby::{is_blank, truncate};
use rusqlite::{Connection, Row, params};

pub const KINDS: [&str; 3] = ["pull_request", "event", "drive_file"];
#[derive(Debug, Clone)]
pub struct WorkThreadLink {
    pub id: i64,
    pub channel_thread_id: i64,
    pub created_by_id: i64,
    pub kind: String,
    pub github_pull_request_id: Option<i64>,
    pub event_id: Option<i64>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone, Default)]
pub struct NewWorkThreadLink {
    pub channel_thread_id: i64,
    pub created_by_id: i64,
    pub kind: Option<String>,
    pub github_pull_request_id: Option<i64>,
    pub event_id: Option<i64>,
    pub url: Option<String>,
    pub title: Option<String>,
}
impl WorkThreadLink {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            channel_thread_id: row.get("channel_thread_id")?,
            created_by_id: row.get("created_by_id")?,
            kind: row.get("kind")?,
            github_pull_request_id: row.get("github_pull_request_id")?,
            event_id: row.get("event_id")?,
            url: row.get("url")?,
            title: row.get("title")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM work_thread_links WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn for_thread(conn: &Connection, id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM work_thread_links WHERE channel_thread_id=? ORDER BY id",
            [id],
            Self::from_row,
        )
    }
    pub fn validate(conn: &Connection, attributes: &NewWorkThreadLink) -> Result<Errors> {
        let mut errors = Errors::default();
        let thread = ChannelThread::find_by_id(conn, attributes.channel_thread_id)?;
        if thread.is_none() {
            errors.add("channel_thread", "must exist");
        }
        if User::find_by_id(conn, attributes.created_by_id)?.is_none() {
            errors.add("created_by", "must exist");
        }
        let kind = attributes.kind.as_deref().unwrap_or_default();
        if is_blank(kind) {
            errors.add("kind", "can't be blank");
        } else if !KINDS.contains(&kind) {
            return Err(crate::Error::Other(format!("'{kind}' is not a valid kind")));
        }
        let duplicate = match kind {
            "pull_request" => crate::sql::exists(
                conn,
                "SELECT 1 FROM work_thread_links WHERE channel_thread_id=? AND github_pull_request_id IS ?",
                params![
                    attributes.channel_thread_id,
                    attributes.github_pull_request_id
                ],
            )?,
            "event" => crate::sql::exists(
                conn,
                "SELECT 1 FROM work_thread_links WHERE channel_thread_id=? AND event_id IS ?",
                params![attributes.channel_thread_id, attributes.event_id],
            )?,
            "drive_file" => crate::sql::exists(
                conn,
                "SELECT 1 FROM work_thread_links WHERE channel_thread_id=? AND url IS ?",
                params![attributes.channel_thread_id, attributes.url],
            )?,
            _ => false,
        };
        if duplicate {
            errors.add(
                match kind {
                    "pull_request" => "github_pull_request_id",
                    "event" => "event_id",
                    _ => "url",
                },
                "has already been taken",
            );
        }
        match kind {
            "pull_request" => {
                if attributes.github_pull_request_id.is_none() {
                    errors.add("github_pull_request", "must be set for a pull request link");
                }
                if attributes.event_id.is_some() {
                    errors.add("event", "must be blank for a pull request link");
                }
                if attributes.url.as_deref().is_some_and(|v| !is_blank(v)) {
                    errors.add("url", "must be blank for a pull request link");
                }
                if attributes.title.as_deref().is_some_and(|v| !is_blank(v)) {
                    errors.add("title", "must be blank for a pull request link");
                }
            }
            "event" => {
                if attributes.event_id.is_none() {
                    errors.add("event", "must be set for an event link");
                }
                if attributes.github_pull_request_id.is_some() {
                    errors.add("github_pull_request", "must be blank for an event link");
                }
                if attributes.url.as_deref().is_some_and(|v| !is_blank(v)) {
                    errors.add("url", "must be blank for an event link");
                }
                if attributes.title.as_deref().is_some_and(|v| !is_blank(v)) {
                    errors.add("title", "must be blank for an event link");
                }
            }
            "drive_file" => {
                if attributes.url.as_deref().is_none_or(is_blank) {
                    errors.add("url", "must be set for a Drive file link");
                }
                if attributes.github_pull_request_id.is_some() {
                    errors.add("github_pull_request", "must be blank for a Drive file link");
                }
                if attributes.event_id.is_some() {
                    errors.add("event", "must be blank for a Drive file link");
                }
            }
            _ => (),
        }
        if let (Some(thread), Some(event)) = (
            thread,
            attributes
                .event_id
                .map(|id| {
                    query_one(conn, "SELECT room_id FROM events WHERE id=?", [id], |row| {
                        row.get::<_, i64>(0)
                    })
                })
                .transpose()?
                .flatten(),
        ) && thread.room_id != event
        {
            errors.add("event", "must belong to the thread's room");
        }
        Ok(errors)
    }
    pub fn create(tx: &mut Tx<'_>, mut attributes: NewWorkThreadLink) -> Result<Self> {
        if let Some(title) = attributes.title.as_ref().filter(|s| !is_blank(s)) {
            attributes.title = Some(truncate(title, 255, "..."));
        }
        Self::validate(tx.conn(), &attributes)?.into_result()?;
        let id=tx.conn().query_row("INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,github_pull_request_id,event_id,url,title,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?) RETURNING id",
            params![attributes.channel_thread_id,attributes.created_by_id,attributes.kind,attributes.github_pull_request_id,attributes.event_id,attributes.url,attributes.title,tx.now(),tx.now()],|row|row.get::<_,i64>(0))?;
        Self::find(tx.conn(), id)?.ok_or(crate::Error::RecordNotFound("WorkThreadLink"))
    }
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM work_thread_links WHERE id=?", [self.id])?;
        Ok(())
    }
}

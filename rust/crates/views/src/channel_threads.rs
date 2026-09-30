//! The standalone ChannelThreadsController templates. Conversation/composer integration and
//! populated work/board/PR sections are separate seams with their feature owners.
use askama::Template;
use crate::{ViewContext, messages::MessageItem};

pub struct ListRow {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub message_count: i64,
    pub work_label: Option<String>,
    pub owner_label: String,
    pub agent: bool,
}

#[derive(Template)]
#[template(path = "channel_threads/index.html")]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room_id: i64,
    pub room_name: &'a str,
    pub threads: &'a [ListRow],
}

#[derive(Template)]
#[template(path = "channel_threads/show.html")]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub name: &'a str,
    pub status: &'a str,
    pub count: i64,
    pub parent: Option<&'a MessageItem>,
    pub messages: &'a [MessageItem],
}

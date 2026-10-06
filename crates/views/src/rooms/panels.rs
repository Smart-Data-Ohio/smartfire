//! WS13's thread/poll composition factories, reused by the native room shell.
//! Templates are unchanged from WS13 cdb9b757; the message list/composer stay in WS8bm.
use super::RoomView;
use crate::{ViewContext, helpers as h};
use askama::Template;

#[derive(Template)]
#[template(path = "rooms/composition/_thread_panel.html")]
pub struct ThreadPanel<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room: &'a RoomView,
    pub neutral_name: Option<&'a str>,
}
impl ThreadPanel<'_> {
    pub fn channel_name(&self) -> &str {
        self.neutral_name.unwrap_or(&self.room.display_name)
    }
}

#[derive(Template)]
#[template(path = "rooms/composition/_poll_builder.html")]
pub struct PollBuilder<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room: &'a RoomView,
}
impl PollBuilder<'_> {
    pub fn form_action(&self, kind: &str) -> String {
        format!("/rooms/{}/{kind}", self.room.id)
    }
}

use super::{RoomView,shell,composition,navigation};
use crate::{ViewContext,helpers as h,layouts::Page,messages::{MessageItem,UserView}};
use crate::messages::support::epoch_ms;
use askama::Template;
use serde::Deserialize;
use jiff::Timestamp;

/// `rooms/show`.
#[derive(Template)]
#[template(path = "rooms/composition/show.html", blocks = ["head", "content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub show: &'a ShowView,
}

impl Page for Show<'_> {
    fn has_sidebar(&self) -> bool { true }
    fn page_title(&self) -> Option<String> {
        Some(self.show.room.display_name.clone())
    }

    fn body_class(&self) -> Option<&str> {
        Some("sidebar room-workspace")
    }
}

impl Show<'_> {
    fn composition(&self,partial:&str)->String {composition::request(self.ctx,&self.show.room,self.show.thread_panel_name.as_deref(),partial)}
    fn footer(&self)->String {let markup=self.composition("composer");format!("  {}",markup.strip_prefix('\n').unwrap_or(&markup))}
    fn panel(&self,partial:&str)->String {format!("  {}\n",self.composition(partial))}
    fn timeline(&self) -> String {
        let collection = |messages: &[MessageItem]| messages.iter()
            .map(|message| crate::messages::cached_message_item(self.ctx, message).0.into_owned())
            .collect::<String>();
        let prefix = if self.show.invitation { "      " } else { "    \n      " };
        let body = if let Some(index) = self.show.shell.unread_index {
            format!("{}\n      {}\n      {}", collection(&self.show.messages[..index]),
                shell::unread(self.show.shell.unread_count), collection(&self.show.messages[index..]))
        } else {
            collection(&self.show.messages)
        };
        format!("{prefix}{body}")
    }
    fn notices(&self) -> String {shell::Notices {ctx:self.ctx,notices:&self.show.shell.notices}.render().expect("notices render")}
    fn loaded_at(&self) -> i64 {
        epoch_ms(self.show.updated_at)
    }
}

mod filters { pub use crate::helpers::filters::*; }

use crate::rendering::*;
/// What `rooms/show` shows.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ShowView {
    pub room: RoomView,
    /// `room.updated_at`, the refresh controller's `loaded_at`.
    pub updated_at: Timestamp,
    /// `Current.user`, for the client-side message template.
    pub user: UserView,
    pub messages: Vec<MessageItem>,
    /// `@room == Room.original && !@room.messages.paged?` (`rooms/show/_invitation`).
    pub invitation: bool,
    /// `Current.account.join_code`, for the invitation's join link.
    #[serde(default)]
    pub join_code: String,
    /// `Turbo::StreamsChannel.signed_stream_name([room, :messages])`.
    pub messages_stream_name: String,
    #[serde(default)]
    pub navigation: Option<navigation::Navigation>,
    #[serde(default)]
    pub thread_panel_name: Option<String>,
    #[serde(default)]
    pub shell: shell::State,
}

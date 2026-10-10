pub mod board;
// The standalone and conversation templates share message composition with board posts.
use askama::Template;
use crate::{ViewContext, messages::MessageItem};
use crate::helpers as h;

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
    /// WS15g's `github::thread_header`, rendered before the starter as Rails does.
    pub pull_request_header: &'a h::Html,
    pub parent: Option<&'a MessageItem>,
    pub messages: &'a [MessageItem],
    pub work: Option<&'a Work>,
}
#[derive(Template)]
#[template(path="channel_threads/_work.html")]
struct WorkSection<'a> {ctx:&'a ViewContext<'a>,work:&'a Work}
impl Show<'_> {
    fn work_section(&self,work:&Work)->h::Html {
        h::raw(WorkSection {ctx:self.ctx,work}.render().expect("work header"))
    }
}
impl WorkSection<'_> {
    fn links(&self)->h::Html {
        h::raw(board::LinksBox {ctx:self.ctx,thread_id:self.work.id,links:&self.work.links,context:"header"}.render().expect("work header links"))
    }
    fn handoff(&self)->h::Html {
        h::link_to_text("Hand off to an agent",&format!("/threads/{}/work/handoff/new",self.work.id),h::attrs().class("btn"))
    }
}

/// The content endpoint's pane. Message items and the composer are also stable shell seams.
#[derive(Template)]
#[template(path = "channel_threads/_conversation.html")]
pub struct Conversation<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub thread_id: i64,
    pub room_updated_at: jiff::Timestamp,
    pub anchor: Option<i64>,
    pub messages: &'a [MessageItem],
    pub user: &'a crate::messages::UserView,
    pub steps: &'a [crate::messages::parts::AgentStep],
    pub composer: &'a crate::messages::composer::Facts,
    pub scheduled_control: &'a h::Html,
}
impl Conversation<'_> {
    fn area_open(&self) -> h::Html {
        let attrs = h::attrs().id(format!("message_area_channel_thread_{}", self.thread_id)).class("message-area").attr("contents", "true")
            .data("controller", "messages drop-target")
            .data("action", "turbo:before-stream-render@document->messages#beforeStreamRender keydown.up@document->messages#editMyLastMessage dragenter->drop-target#dragenter dragover->drop-target#dragover drop->drop-target#drop")
            .data("messages_first_of_day_class", "message--first-of-day").data("messages_formatted_class", "message--formatted")
            .data("messages_me_class", "message--me").data("messages_mentioned_class", "message--mentioned").data("messages_threaded_class", "message--threaded")
            .data("messages_page_url_value", self.ctx.url(&self.composer.message_path()))
            .attr_opt("data-messages-anchor-message-id-value", self.anchor);
        h::raw(format!("<div{}>", attrs.render()))
    }
    fn list_open(&self) -> h::Html {
        let attrs = h::attrs().id(format!("messages_channel_thread_{}", self.thread_id)).class("messages")
            .data("controller", "maintain-scroll message-list").data("action", "turbo:before-stream-render@document->maintain-scroll#beforeStreamRender")
            .data("messages_target", "messages").attr_opt("data-messages-anchor-message-id-value", self.anchor)
            .data("refresh_room_loaded_at_value", crate::messages::support::epoch_ms(self.room_updated_at));
        h::raw(format!("<div{}>", attrs.render()))
    }
    fn pending_template(&self) -> h::Html {
        h::raw(PendingTemplate {ctx: self.ctx, user: self.user}.render().expect("pending template renders"))
    }
    fn composer(&self) -> h::Html {
        h::raw(crate::messages::composer::Composer {ctx: self.ctx, facts: self.composer, scheduled_control: self.scheduled_control}.render().expect("composer renders"))
    }
    fn stream(&self) -> h::Html {
        let signed = (self.ctx.signed_stream_name)(&[&h::gid_param("ChannelThread", self.thread_id), "messages"]);
        h::content_tag("turbo-cable-stream-source", h::attrs().attr("channel", "RoomMessagesChannel").attr("signed-stream-name", signed), "")
    }
    fn jump(&self, ctx: &ViewContext) -> h::Html {
        let content = format!("{}<span class=\"for-screen-reader\">Jump to newest message</span>", h::image_tag(ctx, "arrow-down.svg", h::attrs().aria_hidden().size(20)));
        h::content_tag("button", h::attrs().class("message-area__return-to-latest btn").data("action", "messages#returnToLatest").data("messages_target", "latest").hidden(), &content)
    }
}

#[derive(Template)]
#[template(path = "messages/_pending.html")]
pub struct PendingTemplate<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: &'a crate::messages::UserView,
}
pub use campfire_presentation::channel_threads::*;

use crate::rendering::*;

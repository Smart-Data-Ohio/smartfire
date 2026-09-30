//! The message-owned composer. Feature owners supply the schedule control and Drive availability.
use super::{RoomKind, room_dom_id};
use crate::{ViewContext, helpers as h};
use askama::Template;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum DriveFlow {
    #[default]
    None,
    Share,
    Metadata,
}
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Thread {
    pub id: i64,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Facts {
    pub room_id: i64,
    pub room_kind: RoomKind,
    /// `Room.model_name.param_key`; the legacy room kind collapses Voice/Stage/Board.
    #[serde(default)]
    pub room_param_key: Option<String>,
    pub room_name: String,
    pub thread: Option<Thread>,
    /// The complete registry and ordered room agent-command names, supplied by the presenter.
    pub slash_commands: Vec<String>,
    pub drive: DriveFlow,
}
impl Facts {
    pub fn thread_value(&self) -> String {
        self.thread
            .as_ref()
            .map(|thread| thread.id.to_string())
            .unwrap_or_default()
    }
    pub fn scoped_id(&self, prefix: &str, room_default: &str) -> String {
        self.thread.as_ref().map_or_else(
            || room_default.into(),
            |thread| format!("{prefix}_channel_thread_{}", thread.id),
        )
    }
    pub fn composer_frame_id(&self) -> String {
        self.scoped_id("composer_frame", "composer-frame")
    }
    pub fn attach_menu_id(&self) -> String {
        self.scoped_id("attach_menu", "attach-menu")
    }
    pub fn reply_notify_id(&self) -> String {
        let room_id = self.room_param_key.as_ref().map_or_else(
            || room_dom_id(self.room_kind, self.room_id, "reply_notify"),
            |key| format!("reply_notify_{key}_{}", self.room_id),
        );
        self.scoped_id("reply_notify", &room_id)
    }
    pub fn drive_available(&self) -> bool {
        self.drive != DriveFlow::None
    }
    pub fn drive_share(&self) -> bool {
        self.drive == DriveFlow::Share
    }
    pub fn drive_metadata(&self) -> bool {
        self.drive == DriveFlow::Metadata
    }
    pub fn message_path(&self) -> String {
        self.thread.as_ref().map_or_else(
            || format!("/rooms/{}/messages", self.room_id),
            |thread| format!("/rooms/{}/threads/{}/messages", self.room_id, thread.id),
        )
    }
    pub fn slash_list_path(&self) -> String {
        format!(
            "/autocompletable/slash_commands?room_id={}{}",
            self.room_id,
            self.thread
                .as_ref()
                .map(|thread| format!("&thread_id={}", thread.id))
                .unwrap_or_default()
        )
    }
    pub fn form(&self) -> h::FormWith {
        let mut form = h::form_with(self.message_path()).model("message").id(self.scoped_id("composer", "composer"))
            .class("margin-block flex-item-grow contain")
            .data("controller", "composer drop-target")
            .data("action", "dragenter->drop-target#dragenter dragover->drop-target#dragover drop->drop-target#drop drop-target:drop@window->composer#dropFiles trix-file-accept->composer#preventAttachment refresh-room:online@window->composer#online submit->typing-notifications#stop paste->composer#pasteFiles turbo:submit-end->composer#submitEnd refresh-room:offline@window->composer#offline turbo:before-fetch-request->composer#prepareRequest messages:recover@window->composer#recover")
            .data("composer_messages_outlet", format!("#{}", self.scoped_id("message_area", "message-area")))
            .data("composer_room_id_value", self.room_id);
        if let Some(thread) = &self.thread {
            form = form
                .namespace(format!("thread_{}", thread.id))
                .data("composer_thread_id_value", thread.id);
        }
        form.data("composer_thread_mode_value", self.thread.is_some())
            .data(
                "composer_slash_commands_url_value",
                format!("/rooms/{}/slash_commands", self.room_id),
            )
            .data(
                "composer_slash_commands_value",
                serde_json::to_string(&self.slash_commands).expect("string array serializes"),
            )
            .data(
                "composer_slash_commands_list_url_value",
                self.slash_list_path(),
            )
    }
    pub fn text_area(&self) -> h::Html {
        self.form().text_area("markdown_source", None, h::attrs().rows(1).maxlength(50_000).class("composer__textarea input")
            .placeholder(self.thread.as_ref().map_or_else(|| format!("Message #{}", self.room_name), |thread| format!("Reply in {}", thread.name)))
            .role("combobox").aria("multiline", "true").aria("label", if self.thread.is_some() {"Write a thread reply"} else {"Write a message"})
            .aria("autocomplete", "list").aria("expanded", "false")
            .data("controller", "markdown-autocomplete")
            .data("action", "input->markdown-editor#resize input->markdown-autocomplete#search input->typing-notifications#start input->composer#saveDraft focus->markdown-autocomplete#focus blur->markdown-autocomplete#blur keydown->markdown-editor#shortcut keydown->composer#submitByKeyboard")
            .data("markdown_autocomplete_url_value", format!("/autocompletable/users?room_id={}", self.room_id))
            .data("markdown_autocomplete_icons_url_value", "/autocompletable/icons")
            .data("markdown_autocomplete_slash_commands_url_value", self.slash_list_path())
            .data("markdown_editor_target", "source").data("composer_target", "markdown").data("suggestion_results_placement", "above"))
    }
    pub fn send_button(&self, ctx: &ViewContext) -> h::Html {
        let content = format!(
            "\n                {}\n                <span class=\"for-screen-reader\">{}</span>\n",
            h::image_tag(ctx, "arrow-up.svg", h::attrs().size(17).aria_hidden()),
            if self.thread.is_some() {
                "Send Reply"
            } else {
                "Send Message"
            }
        );
        h::button_tag(
            h::attrs()
                .name("send")
                .type_("submit")
                .data("action", "composer#submit")
                .data("composer_target", "send")
                .class("composer__send btn btn--reversed flex-item-no-shrink"),
            &content,
        )
    }
}

#[derive(Template)]
#[template(path = "messages/_composer.html")]
pub struct Composer<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub facts: &'a Facts,
    /// WS8bm2 mounts its pinned schedule control in this owner-provided slot.
    pub scheduled_control: &'a h::Html,
}

//! The message-owned composer. Feature owners supply the schedule control and Drive availability.
use crate::{ViewContext, helpers as h};
use askama::Template;
pub trait FactsRendering {
    fn form(&self) -> h::FormWith;
    fn text_area(&self) -> h::Html;
    fn send_button(&self, ctx: &ViewContext) -> h::Html;
}
impl FactsRendering for Facts {
    fn form(&self) -> h::FormWith {
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
    fn text_area(&self) -> h::Html {
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
    fn send_button(&self, ctx: &ViewContext) -> h::Html {
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

/// The same composer captured by Rails's `content_for :footer` instead of `inline: true`.
/// This is an additive shell slot; the existing pane/inline entry point is unchanged.
#[derive(Template)]
#[template(path = "messages/_footer_composer.html")]
pub struct FooterComposer<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub facts: &'a Facts,
    pub scheduled_control: &'a h::Html,
}
pub use campfire_presentation::messages::composer::*;

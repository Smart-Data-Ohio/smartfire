//! Views for `reference/app/views/autocompletable`.

use askama::Template;

use crate::ViewContext;
use crate::helpers as h;
use crate::users::MentionUser;

/// `autocompletable/users/index.html.erb`: `<lexxy-prompt-item>`s for the mentions prompt,
/// rendered without a layout.
#[derive(Template)]
#[template(path = "autocompletable/users/index.html")]
pub struct UsersIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub users: Vec<MentionUser>,
}

/// `autocompletable/users/_prompt_item.html.erb` on its own.
#[derive(Template)]
#[template(path = "autocompletable/users/_prompt_item.html")]
pub struct PromptItem<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: MentionUser,
}
pub use campfire_presentation::autocompletable::*;

use crate::rendering::*;

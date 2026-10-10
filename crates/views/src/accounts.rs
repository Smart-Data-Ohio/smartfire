//! Views for `reference/app/views/accounts`.

pub mod audit_logs;
pub mod icons;

use askama::Template;

use crate::ViewContext;
use crate::helpers::{self as h, filters};
use crate::layouts::Page;
use crate::users::UserSummary;

pub mod bot_access;

pub use campfire_view_kit::HelpContact;

/// `accounts/_help_contact.html.erb` on its own.
#[derive(Template)]
#[template(path = "accounts/_help_contact.html")]
pub struct HelpContactPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub help_contact: Option<HelpContact>,
}

/// `accounts/edit.html.erb`.
#[derive(Template)]
#[template(path = "accounts/edit.html", blocks = ["head", "nav", "content", "footer"])]
pub struct Edit<'a> {
    pub ctx: &'a ViewContext<'a>,
    /// `Current.account.id`: `form_with model: @account` posts to `/account.<id>` because the
    /// account is a singular resource (a quirk the reference ships with).
    pub account_id: i64,
    pub join_code: String,
    pub restrict_room_creation_to_administrators: bool,
    pub administrators: Vec<UserSummary>,
    pub members: Vec<UserSummary>,
    /// `@page.next_param` unless `@page.last?`.
    pub next_page: Option<String>,
}

impl Edit<'_> {
    fn account_action(&self) -> String {
        format!("{}.{}", h::routes::account(), self.account_id)
    }
}

impl Page for Edit<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Account settings".into())
    }
}

/// `accounts/_invite.html.erb` on its own.
#[derive(Template)]
#[template(path = "accounts/_invite.html")]
pub struct Invite<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub join_code: String,
}

/// `accounts/users/_user.html.erb` on its own.
#[derive(Template)]
#[template(path = "accounts/users/_user.html")]
pub struct UserPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: UserSummary,
}

/// `accounts/users/_next_page_container.html.erb` on its own.
#[derive(Template)]
#[template(path = "accounts/users/_next_page_container.html")]
pub struct NextPageContainer {
    pub page: String,
}

/// `accounts/users/index.turbo_stream.erb`.
#[derive(Template)]
#[template(path = "accounts/users/index.turbo_stream.html")]
pub struct UsersIndexTurboStream<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub users: Vec<UserSummary>,
    pub next_page: Option<String>,
}
pub trait BotFormRendering {
    fn icon_field(&self, ctx: &ViewContext<'_>, form: &h::FormWith) -> askama::Result<h::Html>;
    fn webhook_options(&self, ctx: &ViewContext<'_>) -> h::Attrs;
    fn manageable(&self, ctx: &ViewContext<'_>) -> bool;
}
impl BotFormRendering for BotForm {
    fn icon_field(&self, ctx: &ViewContext<'_>, form: &h::FormWith) -> askama::Result<h::Html> {
        Ok(h::raw(
            crate::shared::IconField {
                ctx,
                form,
                scope: "user",
                icon_name: self.icon_name.as_deref(),
                icon: self.icon.as_ref(),
            }
            .render()?,
        ))
    }
    fn webhook_options(&self, ctx: &ViewContext<'_>) -> h::Attrs {
        h::attrs()
            .class("input")
            .placeholder("Webhook URL")
            .disabled(self.persisted && !ctx.can_administer())
            .attr_opt(
                "title",
                if ctx.can_administer() {
                    None
                } else {
                    Some("Only an administrator can change the webhook URL")
                },
            )
    }
    fn manageable(&self, ctx: &ViewContext<'_>) -> bool {
        ctx.can_administer()
            || self.agent.as_ref().is_some_and(|agent| {
                agent.owner_id == ctx.current_user.as_ref().map(|user| user.id)
            })
    }
}

#[derive(Template)]
#[template(path = "accounts/bots/_bot.html")]
pub struct BotPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot: Bot,
}

/// `accounts/bots/index.html.erb`.
#[derive(Template)]
#[template(path = "accounts/bots/index.html", blocks = ["head", "content"])]
pub struct BotsIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bots: Vec<Bot>,
}

impl Page for BotsIndex<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Chat bots".into())
    }
}

/// `accounts/bots/new.html.erb`.
#[derive(Template)]
#[template(path = "accounts/bots/new.html", blocks = ["head", "content"])]
pub struct BotsNew<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot: BotForm,
}

impl Page for BotsNew<'_> {
    fn page_title(&self) -> Option<String> {
        Some("New chat bot".into())
    }
}

/// `accounts/bots/edit.html.erb`.
#[derive(Template)]
#[template(path = "accounts/bots/edit.html", blocks = ["head", "content"])]
pub struct BotsEdit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_id: i64,
    pub bot: BotForm,
}

impl BotsEdit<'_> {
    /// WS15g owner fragment: pinned inline `accounts/bots/edit.html.erb`;
    /// the planned Rails partial name is `accounts/bots/_github_connection.html.erb`.
    fn ws15g_github_connection_fragment(&self) -> askama::Result<h::Html> {
        let account = self.bot.github.as_ref();
        let data = crate::github::connections::Connection {
            linked: account.is_some(),
            usable: account.is_some_and(|a| a.usable),
            login: account.map(|a| a.login.clone()).unwrap_or_default(),
            reason: account.and_then(|a| a.disconnected_reason.clone()),
            ..Default::default()
        };
        let html = crate::github::connections::bot(&data, self.bot_id, self.ctx.can_administer());
        // This inline call already contributes the partial's final newline.
        Ok(h::raw(html.strip_suffix('\n').unwrap_or(&html)))
    }

    /// WS15e plug-in boundary: Rails `accounts/bots/_fizzy_connection.html.erb`.
    /// This partial is not in the pinned edit page; WS15e supplies its HTML/data.
    fn ws15e_fizzy_connection_fragment(&self) -> h::Html {
        h::empty()
    }
}

impl Page for BotsEdit<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Edit bot".into())
    }
}

/// `accounts/custom_styles/edit.html.erb`.
#[derive(Template)]
#[template(path = "accounts/custom_styles/edit.html", blocks = ["head", "content"])]
pub struct CustomStylesEdit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub custom_styles: Option<String>,
}

impl Page for CustomStylesEdit<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Custom styles".into())
    }
}

/// `accounts/bots/keys/show.html.erb`: a just-rotated key, rendered once.
#[derive(Template)]
#[template(path = "accounts/bots/keys/show.html", blocks = ["head", "content"])]
pub struct BotKey<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot_name: &'a str,
    pub bot_key: &'a str,
}

impl Page for BotKey<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Bot key".into())
    }
}
pub use campfire_presentation::accounts::*;

use crate::rendering::*;

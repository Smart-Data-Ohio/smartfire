//! Views for `reference/app/views/accounts`.

use askama::Template;

use crate::ViewContext;
use crate::helpers::{self as h, filters};
use crate::layouts::Page;
use crate::users::UserSummary;

pub mod bot_access;

/// `User.administrator.first`, shown by `accounts/_help_contact`.
#[derive(Clone, Debug)]
pub struct HelpContact {
    pub name: String,
    pub email_address: String,
}

/// `accounts/_help_contact.html.erb` on its own.
#[derive(Template)]
#[template(path = "accounts/_help_contact.html")]
pub struct HelpContactPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub help_contact: Option<HelpContact>,
}

/// `accounts/edit.html.erb`.
#[derive(Template)]
#[template(path = "accounts/edit.html", blocks = ["head", "content"])]
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

/// A bot, as `accounts/bots/_bot` and `_form` see it.
#[derive(Clone, Debug, Default)]
pub struct Bot {
    pub user: UserSummary,
    pub kind: Option<String>,
    pub owner_name: Option<String>,
    pub icon: Option<h::AvatarIcon>,
    /// `bot.rooms.without_directs.ordered`.
    pub rooms: Vec<BotRoom>,
}

#[derive(Clone, Debug)]
pub struct BotRoom {
    pub id: i64,
    /// `room_display_name(room)`, the room's name for shared rooms.
    pub name: String,
}

/// The fields `accounts/bots/_form` fills in.
#[derive(Clone, Debug, Default)]
pub struct BotForm {
    pub name: Option<String>,
    pub webhook_url: Option<String>,
    /// `url_for(bot.avatar)` when attached.
    pub avatar_attachment_url: Option<String>,
    pub persisted: bool,
    pub icon_name: Option<String>,
    pub icon: Option<h::AvatarIcon>,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
    pub agent: Option<BotAgentForm>,
    pub budget_usage_line: String,
    pub signing_secret: Option<String>,
    pub github: Option<BotGithubAccount>,
}

#[derive(Clone, Debug, Default)]
pub struct BotAgentForm {
    pub id: i64,
    pub owner_id: Option<i64>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    pub suspended: bool,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
}

impl BotForm {
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
    fn github_usable(&self) -> bool {
        self.github.as_ref().is_some_and(|account| account.usable)
    }
    fn github_submit_label(&self) -> &'static str {
        if self.github.is_some() {
            "Reconnect GitHub"
        } else {
            "Connect GitHub"
        }
    }
}
impl BotAgentForm {
    fn cap_value(&self, name: &str) -> Option<String> {
        match name {
            "messages" => self.daily_message_cap,
            "board_posts" => self.daily_board_post_cap,
            _ => self.daily_external_action_cap,
        }
        .map(|value| value.to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub struct BotGithubAccount {
    pub usable: bool,
    pub login: String,
    pub disconnected_reason: Option<String>,
}

#[derive(Template)]
#[template(path = "accounts/bots/_bot.html")]
pub struct BotPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub bot: Bot,
}

impl Bot {
    fn ownership_line(&self) -> String {
        match &self.kind {
            Some(kind) => format!(
                "{} agent · {}",
                if kind == "workspace" {
                    "Workspace"
                } else {
                    "Personal"
                },
                self.owner_name
                    .as_ref()
                    .map(|name| format!("Owned by {name}"))
                    .unwrap_or_else(|| "no owner recorded".into())
            ),
            None => "no owner recorded".into(),
        }
    }
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
    /// WS15g plug-in boundary: Rails `accounts/bots/_github_connection.html.erb`.
    /// At d7c7de92 this section is inline in `accounts/bots/edit.html.erb`.
    /// Keep the already-ported rendering here until WS15g supplies its fragment;
    /// connection validation, writes and read-side effects remain with WS15g.
    fn ws15g_github_connection_fragment(&self) -> askama::Result<h::Html> {
        #[derive(Template)]
        #[template(path = "accounts/bots/_github_connection.html")]
        struct GithubConnection<'a> {
            ctx: &'a ViewContext<'a>,
            bot_id: i64,
            bot: &'a BotForm,
        }
        GithubConnection { ctx: self.ctx, bot_id: self.bot_id, bot: &self.bot }
            .render().map(h::raw)
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

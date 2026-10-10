//! Durable aliases for retired application pages and card fragments.
use crate::app::AppCtx;
use crate::concerns::{
    self, Before, MatchedRoute, before_actions, cast_integer, require_current_user,
};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Error, Result, format};

fn human_page(c: &mut Ctx) -> Result<()> {
    concerns::deny_bots(c)?;
    if require_current_user(c)?.is_bot() {
        return campfire_kit::halt(concerns::head(campfire_kit::StatusCode::FORBIDDEN));
    }
    Ok(())
}

pub async fn page(c: &mut Ctx) -> Result {
    let endpoint = c.current::<MatchedRoute>().ok_or(Error::NotFound)?.endpoint;
    if endpoint == "agents/directory#index" || endpoint.starts_with("rooms/events") {
        concerns::before_actions_with_authentication(c, Before::default(), None, Some(human_page))
            .await?;
    } else {
        before_actions(c, Before::default()).await?;
    }
    match endpoint {
        "messages#show" | "messages#edit" if c.param_str("room_id").is_some() => {
            let room = campfire_messages::controllers::messages::set_root_room(c).await?;
            let message = campfire_messages::controllers::messages::set_message(c, &room).await?;
            if endpoint == "messages#edit" {
                campfire_messages::controllers::messages::ensure_can_edit(c, &message)?;
            }
        }
        "messages/boosts#index" | "messages/boosts#new" => {
            campfire_messages::controllers::messages::boosts::set_message(c).await?;
        }
        "accounts#edit"
        | "accounts/custom_styles#edit"
        | "accounts/icons#index"
        | "accounts/integrations_health#show"
        | "accounts/users#index"
        | "accounts/bots#index"
        | "accounts/bots#new" => concerns::ensure_can_administer(c)?,
        "accounts/bots#edit" | "accounts/bots/credentials#index" | "accounts/bots/grants#index" => {
            let key = if endpoint == "accounts/bots#edit" {
                "id"
            } else {
                "bot_id"
            };
            let bot = campfire_people::controllers::accounts::bots::find_active_bot(c, key).await?;
            campfire_people::controllers::accounts::bots::ensure_can_manage_bot(c, &bot).await?;
        }
        "rooms/opens#new" | "rooms/closeds#new" | "rooms/boards#new" | "rooms/voices#new"
        | "rooms/stages#new" => {
            campfire_rooms::controllers::rooms::ensure_permission_to_create_rooms(c).await?;
        }
        "rooms/opens#edit" | "rooms/closeds#edit" | "rooms/boards#edit" | "rooms/voices#edit"
        | "rooms/stages#edit" | "rooms/directs#edit" => {
            use campfire_rooms::controllers::rooms::{self, Scope};
            let scope = match endpoint {
                "rooms/opens#edit" | "rooms/closeds#edit" => Scope::WithoutDirects,
                "rooms/boards#edit" => Scope::Boards,
                "rooms/voices#edit" => Scope::Voices,
                "rooms/stages#edit" => Scope::Stages,
                _ => Scope::Directs,
            };
            let room = rooms::set_room(c, scope).await?;
            if !matches!(scope, Scope::Directs) {
                rooms::ensure_can_administer(c, &room)?;
            }
        }
        "rooms#show" => {
            let (room, _) = campfire_rooms::controllers::rooms::set_room_for_show(
                c,
                campfire_rooms::controllers::rooms::Scope::All,
            )
            .await?;
            concerns::remember_last_room_visited(c, room.id);
        }
        "rooms/involvements#show" => {
            concerns::set_room(c).await?;
        }
        "rooms/events#show" | "rooms/events#edit" | "rooms/events/attendances#show" => {
            let event = campfire_rooms::controllers::rooms::events::set_event(c).await?;
            if endpoint == "rooms/events#edit" {
                campfire_rooms::controllers::rooms::events::ensure_manager(c, &event, false)?;
            }
        }
        "rooms/events#index" | "rooms/events#new" => {
            campfire_rooms::controllers::rooms::events::scheduled_room(c).await?;
        }
        "rooms/files#index" | "rooms/pins#index" => {
            concerns::set_room(c).await?;
        }
        "rooms/board_automations#show" => {
            campfire_rooms::controllers::rooms::board_automations::board(c).await?;
        }
        "channel_threads#content" => {
            super::channel_threads::scope(c).await?;
        }
        "channel_threads#new" => {
            let (_, room) = concerns::set_room(c).await?;
            if room.deleted_at.is_some() || !room.board() {
                return Ok(concerns::head(campfire_kit::StatusCode::NOT_FOUND));
            }
        }
        "work_threads#new_handoff" => {
            super::work_threads::scope(c, true).await?;
            c.no_store();
            c.set_header("pragma", "no-cache");
        }
        "accounts/slack_imports#show" | "accounts/slack_import_runs#index" => {
            concerns::ensure_can_administer(c)?;
        }
        "accounts/slack_import_runs#show"
        | "accounts/slack_import_runs#status"
        | "accounts/slack_import_runs#plan"
        | "slack/imports#show"
        | "slack/imports#status" => {
            let admin = endpoint.starts_with("accounts/");
            if admin {
                concerns::ensure_can_administer(c)?;
            }
            let user = require_current_user(c)?.clone();
            super::slack::runs::find(c, &user, admin).await?;
        }
        _ => {}
    }
    campfire_runtime::navigation::redirect(c).await
}

/// History aliases keep the human/owner gates used by the retired fragments.
pub async fn agent_history(c: &mut Ctx) -> Result {
    let endpoint = c.current::<MatchedRoute>().ok_or(Error::NotFound)?.endpoint;
    let approvals = endpoint == "agents/approvals#for_agent";
    let before = if approvals {
        Before::default().allow_bot_access().allow_agent_access()
    } else {
        Before::default()
    };
    before_actions(c, before).await?;
    let viewer = require_current_user(c)?.clone();
    let denied = || concerns::head(campfire_kit::StatusCode::NOT_FOUND);
    if approvals
        && (viewer.is_bot()
            || matches!(
                concerns::authenticated_by(c),
                concerns::AuthenticatedBy::BotKey | concerns::AuthenticatedBy::AgentToken
            ))
    {
        return Ok(denied());
    }
    let Some(id) = c.param_str("id").and_then(cast_integer) else {
        return Ok(denied());
    };
    let allowed = c
        .app()
        .db
        .read(move |conn| {
            let Some(agent) = campfire_db::Agent::find(conn, id)? else {
                return Ok(false);
            };
            let managed = viewer.is_administrator() || agent.owner_id == Some(viewer.id);
            if !approvals {
                return Ok(managed);
            }
            Ok(managed
                && viewer.is_active()
                && campfire_db::User::find_by_id(conn, agent.user_id)?
                    .is_some_and(|bot| bot.is_active()))
        })
        .await
        .map_err(db_error)?;
    if !allowed {
        return Ok(if approvals {
            denied()
        } else {
            concerns::head(campfire_kit::StatusCode::FORBIDDEN)
        });
    }
    campfire_runtime::navigation::redirect(c).await
}

pub async fn record(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let viewer = require_current_user(c)?.id;
    let endpoint = c.current::<MatchedRoute>().ok_or(Error::NotFound)?.endpoint;
    let message = c
        .param_str("message_id")
        .filter(|value| !crate::integrations::github::blank(value));
    let thread = c
        .param_str("thread_id")
        .filter(|value| !crate::integrations::github::blank(value));
    let (message_id, thread_id) = if let Some(message) = message {
        (Some(cast_integer(message).ok_or(Error::NotFound)?), None)
    } else if let Some(thread) = thread {
        (None, Some(cast_integer(thread).ok_or(Error::NotFound)?))
    } else {
        (None, None)
    };
    let location = c.app().db.read(move |conn| {
        let message = match endpoint {
            "rooms/message_links#show" => {
                let message = campfire_db::message_quote::source(conn, room.id, id)?;
                if !campfire_db::message_quote::visible(conn, &message, viewer)? {
                    return Err(campfire_db::Error::RecordNotFound("Message"));
                }
                Some(message)
            }
            "rooms/github/pull_request_cards#show" | "github/pull_request_write_actions#show" => {
                let thread_id = if endpoint == "github/pull_request_write_actions#show"
                    && message_id.is_none() && thread_id.is_none()
                {
                    crate::integrations::github::threads::PullRequestThread::for_room_pr(conn, room.id, id)?
                        .map(|mapping| mapping.channel_thread_id)
                } else {
                    thread_id
                };
                let context = crate::integrations::github::pull_requests::viewer_card_context(
                    conn, room.id, id, message_id, thread_id,
                )?;
                if let Some(thread) = context.thread {
                    return Ok(format!("/app/r/{}/t/{}", room.id, thread.id));
                }
                context.message
            }
            "rooms/fizzy/cards#show" => {
                let message = campfire_db::Message::find(conn, message_id.ok_or(campfire_db::Error::RecordNotFound("Message"))?)?;
                let referenced: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM fizzy_card_references WHERE message_id=? AND fizzy_card_id=?)",
                    (message.id, id), |row| row.get(0),
                )?;
                if message.room_id != room.id || !referenced {
                    return Err(campfire_db::Error::RecordNotFound("Fizzy::CardReference"));
                }
                Some(message)
            }
            _ => return Err(campfire_db::Error::RecordNotFound("Page")),
        };
        Ok(match message {
            Some(message) => message.thread_id.map_or_else(
                || format!("/app/r/{}/m/{}", message.room_id, message.id),
                |thread| format!("/app/r/{}/t/{thread}?m={}", message.room_id, message.id),
            ),
            None => format!("/app/r/{}", room.id),
        })
    }).await.map_err(|error| match error {
        campfire_db::Error::RecordNotFound(_) => Error::Halt(Box::new(concerns::head(campfire_kit::StatusCode::NOT_FOUND))),
        error => db_error(error),
    })?;
    c.respond_to(&[&format::HTML])?;
    concerns::keep_waiting_flash(c);
    c.redirect_to(&c.url_for(&location))
}

/// Typed room lists also served machine clients by redirecting to their last room.
pub async fn room_index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    if c.format()?
        .is_some_and(|kind| kind == &format::HTML || kind == &format::ALL)
    {
        concerns::keep_waiting_flash(c);
        return c.redirect_to(&c.url_for("/app/"));
    }
    let user = require_current_user(c)?.id;
    let room = c
        .app()
        .db
        .read(move |conn| campfire_db::Room::last_for_user(conn, user))
        .await
        .map_err(db_error)?
        .ok_or_else(|| Error::internal(anyhow::anyhow!("No route matches room_url(nil)")))?;
    c.redirect_to(&c.url_for(&campfire_routes::room(room.id)))
}

/// Refresh URLs have always redirected for every format, with the room membership gate.
pub async fn refresh(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    c.redirect_to(&c.url_for(&campfire_routes::room(room.id)))
}

/// Typed bookmarks retain their membership checks and hop through the generic room alias.
pub async fn typed_room(c: &mut Ctx) -> Result {
    use campfire_rooms::controllers::rooms::{Scope, set_room};
    before_actions(c, Before::default()).await?;
    let endpoint = c.current::<MatchedRoute>().ok_or(Error::NotFound)?.endpoint;
    let scope = match endpoint {
        "rooms/opens#show" | "rooms/closeds#show" => Scope::WithoutDirects,
        "rooms/directs#show" => Scope::Directs,
        "rooms/boards#show" => Scope::Boards,
        "rooms/voices#show" => Scope::Voices,
        "rooms/stages#show" => Scope::Stages,
        _ => return Err(Error::NotFound),
    };
    let room = if matches!(scope, Scope::WithoutDirects) {
        campfire_rooms::controllers::rooms::set_room_for_show(c, scope)
            .await?
            .0
    } else {
        set_room(c, scope).await?
    };
    concerns::remember_last_room_visited(c, room.id);
    c.redirect_to(&c.url_for(&campfire_routes::room(room.id)))
}

pub async fn retired(c: &mut Ctx) -> Result {
    Ok(c.head(campfire_kit::StatusCode::GONE))
}
